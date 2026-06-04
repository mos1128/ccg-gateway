use super::*;

#[tauri::command]
pub async fn clear_stats_data(stats_db: State<'_, StatsDb>) -> Result<()> {
    crate::services::stats::clear_usage_stats(&stats_db.0)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_provider_stats(
    db: State<'_, SqlitePool>,
    stats_db: State<'_, StatsDb>,
    start_date: Option<String>,
    end_date: Option<String>,
    cli_type: Option<String>,
    provider_name: Option<String>,
) -> Result<Vec<ProviderStatsResponse>> {
    let pool = &stats_db.0;

    let mut query = r#"
        SELECT
            cli_type,
            provider_name,
            SUM(request_count) as total_requests,
            SUM(success_count) as total_success,
            SUM(input_tokens) as total_input_tokens,
            SUM(output_tokens) as total_output_tokens,
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
    query.push_str(" GROUP BY cli_type, provider_name ORDER BY total_requests DESC");

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
    let pricing_map = crate::services::cost::provider_pricing_map(db.inner())
        .await
        .map_err(|e| e.to_string())?;
    let mut grouped: HashMap<String, ProviderStatsResponse> = HashMap::new();

    for row in rows {
        let pricing = pricing_map
            .get(&(row.cli_type.clone(), row.provider_name.clone()))
            .copied()
            .unwrap_or_default();
        let total_cost = crate::services::cost::calculate_token_cost(
            pricing,
            row.total_input_tokens,
            row.total_cache_read_tokens,
            row.total_cache_creation_tokens,
            row.total_output_tokens,
        );
        let entry =
            grouped
                .entry(row.provider_name.clone())
                .or_insert_with(|| ProviderStatsResponse {
                    provider_name: row.provider_name.clone(),
                    total_requests: 0,
                    total_success: 0,
                    total_tokens: 0,
                    total_cache_read_tokens: 0,
                    total_cache_creation_tokens: 0,
                    total_elapsed_ms: 0,
                    total_cost: 0.0,
                    success_rate: 0.0,
                });
        entry.total_requests += row.total_requests;
        entry.total_success += row.total_success;
        entry.total_tokens += row.total_tokens;
        entry.total_cache_read_tokens += row.total_cache_read_tokens;
        entry.total_cache_creation_tokens += row.total_cache_creation_tokens;
        entry.total_elapsed_ms += row.total_elapsed_ms;
        entry.total_cost += total_cost;
    }

    let mut results: Vec<ProviderStatsResponse> = grouped.into_values().collect();
    for item in &mut results {
        item.success_rate = if item.total_requests > 0 {
            (item.total_success as f64 / item.total_requests as f64) * 100.0
        } else {
            0.0
        };
    }
    results.sort_by(|a, b| b.total_requests.cmp(&a.total_requests));

    Ok(results)
}

#[tauri::command]
pub async fn get_advanced_stats(
    db: State<'_, SqlitePool>,
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
            cli_type,
            provider_name,
            model_id,
            SUM(request_count) as total_requests,
            SUM(success_count) as total_success,
            SUM(input_tokens + cache_read_input_tokens + cache_creation_input_tokens + output_tokens) as total_tokens,
            SUM(input_tokens) as total_input_tokens,
            SUM(output_tokens) as total_output_tokens,
            SUM(cache_read_input_tokens) as total_cache_read_tokens,
            SUM(cache_creation_input_tokens) as total_cache_creation_tokens,
            0.0 as total_cost
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

    query.push_str(" GROUP BY usage_date, cli_type, provider_name, model_id ORDER BY usage_date DESC, provider_name, model_id");

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

    let mut rows = q.fetch_all(pool).await.map_err(|e| e.to_string())?;
    let pricing_map = crate::services::cost::provider_pricing_map(db.inner())
        .await
        .map_err(|e| e.to_string())?;
    for row in &mut rows {
        let pricing = pricing_map
            .get(&(row.cli_type.clone(), row.provider_name.clone()))
            .copied()
            .unwrap_or_default();
        row.total_cost = crate::services::cost::calculate_token_cost(
            pricing,
            row.total_input_tokens,
            row.total_cache_read_tokens,
            row.total_cache_creation_tokens,
            row.total_output_tokens,
        );
    }

    Ok(rows)
}
