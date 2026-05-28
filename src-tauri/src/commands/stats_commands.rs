use super::*;

#[tauri::command]
pub async fn get_provider_stats(
    stats_db: State<'_, StatsDb>,
    start_date: Option<String>,
    end_date: Option<String>,
    cli_type: Option<String>,
    provider_name: Option<String>,
) -> Result<Vec<ProviderStatsResponse>> {
    let pool = &stats_db.0;

    let mut query = r#"
        SELECT
            provider_name,
            SUM(request_count) as total_requests,
            SUM(success_count) as total_success,
            SUM(input_tokens + cache_read_input_tokens + cache_creation_input_tokens + output_tokens) as total_tokens,
            SUM(cache_read_input_tokens) as total_cache_read_tokens,
            SUM(cache_creation_input_tokens) as total_cache_creation_tokens,
            SUM(elapsed_ms) as total_elapsed_ms
        FROM usage_daily_model
        WHERE 1=1
    "#.to_string();

    if start_date.is_some() {
        query.push_str(" AND usage_date >= ?");
    }
    if end_date.is_some() {
        query.push_str(" AND usage_date <= ?");
    }
    if cli_type.is_some() {
        query.push_str(" AND cli_type = ?");
    }
    if provider_name.is_some() {
        query.push_str(" AND provider_name = ?");
    }
    query.push_str(" GROUP BY provider_name ORDER BY total_requests DESC");

    let mut q = sqlx::query_as::<_, ProviderStatsRow>(&query);
    if let Some(ref sd) = start_date {
        q = q.bind(sd);
    }
    if let Some(ref ed) = end_date {
        q = q.bind(ed);
    }
    if let Some(ref ct) = cli_type {
        q = q.bind(ct);
    }
    if let Some(ref pn) = provider_name {
        q = q.bind(pn);
    }

    let rows = q.fetch_all(pool).await.map_err(|e| e.to_string())?;

    let results = rows
        .into_iter()
        .map(|row| ProviderStatsResponse {
            provider_name: row.provider_name,
            total_requests: row.total_requests,
            total_success: row.total_success,
            total_tokens: row.total_tokens,
            total_cache_read_tokens: row.total_cache_read_tokens,
            total_cache_creation_tokens: row.total_cache_creation_tokens,
            total_elapsed_ms: row.total_elapsed_ms,
            success_rate: if row.total_requests > 0 {
                (row.total_success as f64 / row.total_requests as f64) * 100.0
            } else {
                0.0
            },
        })
        .collect();

    Ok(results)
}

#[tauri::command]
pub async fn get_advanced_stats(
    stats_db: State<'_, StatsDb>,
    start_date: Option<String>,
    end_date: Option<String>,
    cli_type: Option<String>,
    provider_name: Option<String>,
    model_id: Option<String>,
) -> Result<Vec<AdvancedStatsRow>> {
    let pool = &stats_db.0;

    let mut query = r#"
        SELECT
            usage_date as date,
            provider_name,
            model_id,
            SUM(request_count) as total_requests,
            SUM(success_count) as total_success,
            SUM(input_tokens + cache_read_input_tokens + cache_creation_input_tokens + output_tokens) as total_tokens,
            SUM(input_tokens) as total_input_tokens,
            SUM(output_tokens) as total_output_tokens,
            SUM(cache_read_input_tokens) as total_cache_read_tokens,
            SUM(cache_creation_input_tokens) as total_cache_creation_tokens
        FROM usage_daily_model
        WHERE 1=1
    "#.to_string();

    if start_date.is_some() {
        query.push_str(" AND usage_date >= ?");
    }
    if end_date.is_some() {
        query.push_str(" AND usage_date <= ?");
    }
    if cli_type.is_some() {
        query.push_str(" AND cli_type = ?");
    }
    if provider_name.is_some() {
        query.push_str(" AND provider_name = ?");
    }
    if model_id.is_some() {
        query.push_str(" AND model_id = ?");
    }

    query.push_str(" GROUP BY usage_date, provider_name, model_id ORDER BY usage_date DESC, provider_name, model_id");

    let mut q = sqlx::query_as::<_, AdvancedStatsRow>(&query);
    if let Some(ref sd) = start_date {
        q = q.bind(sd);
    }
    if let Some(ref ed) = end_date {
        q = q.bind(ed);
    }
    if let Some(ref ct) = cli_type {
        q = q.bind(ct);
    }
    if let Some(ref pn) = provider_name {
        q = q.bind(pn);
    }
    if let Some(ref mid) = model_id {
        q = q.bind(mid);
    }

    q.fetch_all(pool).await.map_err(|e| e.to_string())
}
