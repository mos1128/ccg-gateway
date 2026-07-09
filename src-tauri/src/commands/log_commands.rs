use super::*;
use chrono::Local;

#[tauri::command]
pub async fn get_request_logs(
    db: State<'_, SqlitePool>,
    log_db: State<'_, crate::LogDb>,
    page: Option<i64>,
    page_size: Option<i64>,
    cli_type: Option<String>,
    provider_name: Option<String>,
) -> Result<PaginatedLogs> {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;
    let pool = &log_db.0;

    let mut sql = "SELECT id, created_at, cli_type, provider_name, model_id, status_code, elapsed_ms, first_byte_ms, input_tokens, cache_read_input_tokens, cache_creation_input_tokens, output_tokens, 0.0 as total_cost, client_method, client_path, source_model, target_model FROM request_logs WHERE 1=1".to_string();
    let mut count_sql = "SELECT COUNT(*) FROM request_logs WHERE 1=1".to_string();

    if cli_type.is_some() {
        sql.push_str(" AND cli_type = ?");
        count_sql.push_str(" AND cli_type = ?");
    }
    if provider_name.is_some() {
        sql.push_str(" AND provider_name = ?");
        count_sql.push_str(" AND provider_name = ?");
    }

    sql.push_str(" ORDER BY created_at DESC, id DESC LIMIT ? OFFSET ?");

    let mut q = sqlx::query_as::<_, RequestLogItem>(&sql);

    if let Some(ct) = &cli_type {
        q = q.bind(ct);
    }
    if let Some(pn) = &provider_name {
        q = q.bind(pn);
    }

    q = q.bind(page_size).bind(offset);

    let mut items = q.fetch_all(pool).await.map_err(|e| e.to_string())?;
    let pricing_map = crate::services::cost::provider_pricing_map(db.inner())
        .await
        .map_err(|e| e.to_string())?;
    for item in &mut items {
        let pricing = pricing_map
            .get(&(item.cli_type.clone(), item.provider_name.clone()))
            .copied()
            .unwrap_or_default();
        item.total_cost = crate::services::cost::calculate_token_cost(
            pricing,
            item.input_tokens,
            item.cache_read_input_tokens,
            item.cache_creation_input_tokens,
            item.output_tokens,
        );
    }

    let mut count_q = sqlx::query_as::<_, (i64,)>(&count_sql);
    if let Some(ct) = &cli_type {
        count_q = count_q.bind(ct);
    }
    if let Some(pn) = &provider_name {
        count_q = count_q.bind(pn);
    }

    let (total,) = count_q.fetch_one(pool).await.map_err(|e| e.to_string())?;

    Ok(PaginatedLogs {
        items,
        total,
        page,
        page_size,
    })
}

#[tauri::command]
pub async fn clear_request_logs(log_db: State<'_, crate::LogDb>) -> Result<()> {
    sqlx::query("DELETE FROM request_logs")
        .execute(&log_db.0)
        .await
        .map_err(|e| e.to_string())?;
    crate::services::stats::clear_request_log_detail_files()
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("VACUUM")
        .execute(&log_db.0)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn clear_request_detail_files() -> Result<()> {
    crate::services::stats::clear_request_log_detail_files()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn clear_old_request_logs(log_db: State<'_, crate::LogDb>, days: i64) -> Result<()> {
    let cutoff = Local::now().timestamp() - days * 86400;
    sqlx::query("DELETE FROM request_logs WHERE created_at < ?")
        .bind(cutoff)
        .execute(&log_db.0)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("VACUUM")
        .execute(&log_db.0)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn clear_old_request_detail_files(days: i64) -> Result<()> {
    crate::services::stats::cleanup_request_log_detail_files(days)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_request_log_detail(
    db: State<'_, SqlitePool>,
    log_db: State<'_, crate::LogDb>,
    id: i64,
) -> Result<RequestLogDetail> {
    let mut detail = sqlx::query_as::<_, RequestLogDetail>(
        "SELECT id, created_at, cli_type, provider_name, model_id, status_code, elapsed_ms, first_byte_ms, input_tokens, cache_read_input_tokens, cache_creation_input_tokens, output_tokens, 0.0 as total_cost, client_method, client_path, NULL as client_headers, NULL as client_body, forward_url, NULL as forward_headers, NULL as forward_body, NULL as provider_headers, NULL as provider_body, error_message, source_model, target_model FROM request_logs WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&log_db.0)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Log not found".to_string())?;

    let pricing = crate::services::cost::provider_pricing(
        db.inner(),
        &detail.cli_type,
        &detail.provider_name,
    )
    .await
    .map_err(|e| e.to_string())?;
    detail.total_cost = crate::services::cost::calculate_token_cost(
        pricing,
        detail.input_tokens,
        detail.cache_read_input_tokens,
        detail.cache_creation_input_tokens,
        detail.output_tokens,
    );

    detail.client_headers = crate::services::stats::read_request_log_detail(
        detail.id,
        detail.created_at,
        "client.headers",
    )
    .await;
    detail.client_body = crate::services::stats::read_request_log_detail(
        detail.id,
        detail.created_at,
        "client.body",
    )
    .await
    .or_else(|| Some(String::new()));
    detail.forward_headers = crate::services::stats::read_request_log_detail(
        detail.id,
        detail.created_at,
        "forward.headers",
    )
    .await;
    detail.forward_body = crate::services::stats::read_request_log_detail(
        detail.id,
        detail.created_at,
        "forward.body",
    )
    .await
    .or_else(|| Some(String::new()));
    detail.provider_headers = crate::services::stats::read_request_log_detail(
        detail.id,
        detail.created_at,
        "provider.headers",
    )
    .await;
    detail.provider_body = crate::services::stats::read_request_log_detail(
        detail.id,
        detail.created_at,
        "provider.body",
    )
    .await;

    Ok(detail)
}

#[tauri::command]
pub async fn get_system_logs(
    log_db: State<'_, crate::LogDb>,
    page: Option<i64>,
    page_size: Option<i64>,
    event_type: Option<String>,
) -> Result<SystemLogListResponse> {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let mut sql =
        "SELECT id, created_at, event_type, message FROM system_logs WHERE 1=1".to_string();
    let mut count_sql = "SELECT COUNT(*) FROM system_logs WHERE 1=1".to_string();

    if event_type.is_some() {
        sql.push_str(" AND event_type = ?");
        count_sql.push_str(" AND event_type = ?");
    }

    sql.push_str(" ORDER BY id DESC LIMIT ? OFFSET ?");
    let mut q = sqlx::query_as::<_, SystemLogItem>(&sql);

    if let Some(et) = &event_type {
        q = q.bind(et);
    }

    q = q.bind(page_size).bind(offset);

    let items = q.fetch_all(&log_db.0).await.map_err(|e| e.to_string())?;

    let mut count_q = sqlx::query_as::<_, (i64,)>(&count_sql);
    if let Some(et) = &event_type {
        count_q = count_q.bind(et);
    }
    let (total,) = count_q
        .fetch_one(&log_db.0)
        .await
        .map_err(|e| e.to_string())?;

    Ok(SystemLogListResponse {
        items,
        total,
        page,
        page_size,
    })
}

#[tauri::command]
pub async fn clear_system_logs(log_db: State<'_, crate::LogDb>) -> Result<()> {
    sqlx::query("DELETE FROM system_logs")
        .execute(&log_db.0)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("VACUUM")
        .execute(&log_db.0)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
