use crate::config::get_data_dir;
use crate::db::models::RequestLogInfo;
use crate::time::{local_date_from_timestamp, now_timestamp, today_local_date};
use chrono::{Duration, Local, NaiveDate};
use sqlx::SqlitePool;
use std::path::PathBuf;

const REQUEST_DETAIL_DIR: &str = "request-bodies";

/// Record a request in the daily usage statistics
pub async fn record_request(
    stats_db: &SqlitePool,
    provider_name: &str,
    cli_type: &str,
    model_id: Option<&str>,
    source_model: Option<&str>,
    success: bool,
    elapsed_ms: i64,
    input_tokens: i64,
    cache_read_input_tokens: i64,
    cache_creation_input_tokens: i64,
    output_tokens: i64,
) -> Result<(), sqlx::Error> {
    let today = today_local_date();
    let stat_model_id = model_id.or(source_model).unwrap_or("未知模型");

    sqlx::query(
        r#"
        INSERT INTO usage_daily_model (usage_date, cli_type, provider_name, model_id, request_count, success_count, failure_count, input_tokens, cache_read_input_tokens, cache_creation_input_tokens, output_tokens, elapsed_ms)
        VALUES (?, ?, ?, ?, 1, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(usage_date, cli_type, provider_name, model_id) DO UPDATE SET
            request_count = request_count + 1,
            success_count = success_count + excluded.success_count,
            failure_count = failure_count + excluded.failure_count,
            input_tokens = input_tokens + excluded.input_tokens,
            cache_read_input_tokens = cache_read_input_tokens + excluded.cache_read_input_tokens,
            cache_creation_input_tokens = cache_creation_input_tokens + excluded.cache_creation_input_tokens,
            output_tokens = output_tokens + excluded.output_tokens,
            elapsed_ms = elapsed_ms + excluded.elapsed_ms
        "#,
    )
    .bind(&today)
    .bind(cli_type)
    .bind(provider_name)
    .bind(stat_model_id)
    .bind(if success { 1 } else { 0 })
    .bind(if success { 0 } else { 1 })
    .bind(input_tokens)
    .bind(cache_read_input_tokens)
    .bind(cache_creation_input_tokens)
    .bind(output_tokens)
    .bind(elapsed_ms)
    .execute(stats_db)
    .await?;

    Ok(())
}

pub async fn clear_usage_stats(stats_db: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM usage_daily_model")
        .execute(stats_db)
        .await?;
    sqlx::query("VACUUM").execute(stats_db).await?;
    Ok(())
}

fn request_log_detail_day(created_at: i64) -> String {
    local_date_from_timestamp(created_at)
}

fn request_log_detail_path_for_day(log_id: i64, day: &str, name: &str) -> Option<PathBuf> {
    match name {
        "client.body" | "forward.body" | "provider.body" | "client.headers" | "forward.headers"
        | "provider.headers" => Some(
            get_data_dir()
                .join(REQUEST_DETAIL_DIR)
                .join(day)
                .join(format!("{}-{}", log_id, name)),
        ),
        _ => None,
    }
}

fn request_log_detail_path(log_id: i64, created_at: i64, name: &str) -> Option<PathBuf> {
    request_log_detail_path_for_day(log_id, &request_log_detail_day(created_at), name)
}

async fn write_request_log_detail(
    log_id: i64,
    created_at: i64,
    name: &str,
    content: Option<&String>,
) {
    let Some(content) = content else {
        return;
    };
    let Some(path) = request_log_detail_path(log_id, created_at, name) else {
        return;
    };
    let Some(parent) = path.parent() else {
        return;
    };

    if let Err(e) = tokio::fs::create_dir_all(parent).await {
        tracing::error!(error = %e, path = %parent.display(), "Failed to create request body directory");
        return;
    }

    let tmp_path = path.with_file_name(format!(
        "{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("body"),
        uuid::Uuid::new_v4()
    ));

    if let Err(e) = tokio::fs::write(&tmp_path, content.as_bytes()).await {
        tracing::error!(error = %e, path = %tmp_path.display(), "Failed to write request detail file");
        return;
    }

    let _ = tokio::fs::remove_file(&path).await;
    if let Err(e) = tokio::fs::rename(&tmp_path, &path).await {
        let _ = tokio::fs::remove_file(&tmp_path).await;
        tracing::error!(error = %e, path = %path.display(), "Failed to finalize request detail file");
    }
}

async fn write_request_log_details(log_id: i64, created_at: i64, info: &RequestLogInfo) {
    write_request_log_detail(
        log_id,
        created_at,
        "client.headers",
        info.client_headers.as_ref(),
    )
    .await;
    write_request_log_detail(log_id, created_at, "client.body", info.client_body.as_ref()).await;
    write_request_log_detail(
        log_id,
        created_at,
        "forward.headers",
        info.forward_headers.as_ref(),
    )
    .await;
    write_request_log_detail(
        log_id,
        created_at,
        "forward.body",
        info.forward_body.as_ref(),
    )
    .await;
    write_request_log_detail(
        log_id,
        created_at,
        "provider.headers",
        info.provider_headers.as_ref(),
    )
    .await;
    write_request_log_detail(
        log_id,
        created_at,
        "provider.body",
        info.provider_body.as_ref(),
    )
    .await;
}

pub async fn read_request_log_detail(log_id: i64, created_at: i64, name: &str) -> Option<String> {
    let path = request_log_detail_path(log_id, created_at, name)?;
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => Some(content),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            tracing::warn!(error = %e, path = %path.display(), "Failed to read request detail file");
            None
        }
    }
}

pub async fn clear_request_log_detail_files() -> std::io::Result<()> {
    let dir = get_data_dir().join(REQUEST_DETAIL_DIR);
    match tokio::fs::remove_dir_all(&dir).await {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

pub async fn cleanup_request_log_detail_files(retention_days: i64) -> std::io::Result<()> {
    if retention_days <= 0 {
        return Ok(());
    }

    let dir = get_data_dir().join(REQUEST_DETAIL_DIR);
    let mut entries = match tokio::fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };

    let cutoff = Local::now().date_naive() - Duration::days(retention_days);
    while let Some(entry) = entries.next_entry().await? {
        let file_type = entry.file_type().await?;
        if !file_type.is_dir() {
            continue;
        }

        let Some(day) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let Ok(day) = NaiveDate::parse_from_str(&day, "%Y-%m-%d") else {
            continue;
        };

        if day < cutoff {
            let path = entry.path();
            if let Err(e) = tokio::fs::remove_dir_all(&path).await {
                tracing::warn!(
                    error = %e,
                    path = %path.display(),
                    "Failed to remove expired request detail directory"
                );
            }
        }
    }

    Ok(())
}

/// Record a request log entry, returns the inserted log ID
pub async fn record_request_log(
    log_db: &SqlitePool,
    cli_type: &str,
    provider_name: &str,
    model_id: Option<&str>,
    status_code: Option<u16>,
    elapsed_ms: i64,
    input_tokens: i64,
    cache_read_input_tokens: i64,
    cache_creation_input_tokens: i64,
    output_tokens: i64,
    client_method: &str,
    client_path: &str,
    source_model: Option<&str>,
    target_model: Option<&str>,
    info: Option<RequestLogInfo>,
) -> Result<i64, sqlx::Error> {
    let now = now_timestamp();
    let info = info.unwrap_or_default();

    let result = sqlx::query(
        r#"
        INSERT INTO request_logs (created_at, cli_type, provider_name, model_id, status_code, elapsed_ms, input_tokens, cache_read_input_tokens, cache_creation_input_tokens, output_tokens, client_method, client_path, forward_url, error_message, source_model, target_model)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(now)
    .bind(cli_type)
    .bind(provider_name)
    .bind(model_id)
    .bind(status_code.map(|c| c as i64))
    .bind(elapsed_ms)
    .bind(input_tokens)
    .bind(cache_read_input_tokens)
    .bind(cache_creation_input_tokens)
    .bind(output_tokens)
    .bind(client_method)
    .bind(client_path)
    .bind(&info.forward_url)
    .bind(&info.error_message)
    .bind(source_model)
    .bind(target_model)
    .execute(log_db)
    .await?;

    let log_id = result.last_insert_rowid();
    write_request_log_details(log_id, now, &info).await;
    Ok(log_id)
}

/// Record a system log entry
pub async fn record_system_log(
    log_db: &SqlitePool,
    event_type: &str,
    message: &str,
) -> Result<(), sqlx::Error> {
    let now = now_timestamp();

    sqlx::query(
        r#"
        INSERT INTO system_logs (created_at, event_type, message)
        VALUES (?, ?, ?)
        "#,
    )
    .bind(now)
    .bind(event_type)
    .bind(message)
    .execute(log_db)
    .await?;

    Ok(())
}
