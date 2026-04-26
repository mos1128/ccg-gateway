use crate::db::models::RequestLogInfo;
use sqlx::SqlitePool;

/// Record a request in the daily usage statistics
pub async fn record_request(
    log_db: &SqlitePool,
    provider_name: &str,
    cli_type: &str,
    success: bool,
    input_tokens: i64,
    cache_read_input_tokens: i64,
    cache_creation_input_tokens: i64,
    output_tokens: i64,
) -> Result<(), sqlx::Error> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // Upsert into usage_daily table
    sqlx::query(
        r#"
        INSERT INTO usage_daily (usage_date, provider_name, cli_type, request_count, success_count, failure_count, input_tokens, cache_read_input_tokens, cache_creation_input_tokens, output_tokens)
        VALUES (?, ?, ?, 1, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(usage_date, provider_name, cli_type) DO UPDATE SET
            request_count = request_count + 1,
            success_count = success_count + excluded.success_count,
            failure_count = failure_count + excluded.failure_count,
            input_tokens = input_tokens + excluded.input_tokens,
            cache_read_input_tokens = cache_read_input_tokens + excluded.cache_read_input_tokens,
            cache_creation_input_tokens = cache_creation_input_tokens + excluded.cache_creation_input_tokens,
            output_tokens = output_tokens + excluded.output_tokens
        "#,
    )
    .bind(&today)
    .bind(provider_name)
    .bind(cli_type)
    .bind(if success { 1 } else { 0 })
    .bind(if success { 0 } else { 1 })
    .bind(input_tokens)
    .bind(cache_read_input_tokens)
    .bind(cache_creation_input_tokens)
    .bind(output_tokens)
    .execute(log_db)
    .await?;

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
    let now = chrono::Utc::now().timestamp();
    let info = info.unwrap_or_default();

    let result = sqlx::query(
        r#"
        INSERT INTO request_logs (created_at, cli_type, provider_name, model_id, status_code, elapsed_ms, input_tokens, cache_read_input_tokens, cache_creation_input_tokens, output_tokens, client_method, client_path, client_headers, client_body, forward_url, forward_headers, forward_body, provider_headers, provider_body, error_message, source_model, target_model)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
    .bind(&info.client_headers)
    .bind(&info.client_body)
    .bind(&info.forward_url)
    .bind(&info.forward_headers)
    .bind(&info.forward_body)
    .bind(&info.provider_headers)
    .bind(&info.provider_body)
    .bind(&info.error_message)
    .bind(source_model)
    .bind(target_model)
    .execute(log_db)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Record a system log entry
pub async fn record_system_log(
    log_db: &SqlitePool,
    event_type: &str,
    message: &str,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().timestamp();

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
