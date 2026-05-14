use crate::db::models::RequestLogInfo;
use chrono::{Local, TimeZone};
use sqlx::{FromRow, Sqlite, SqlitePool, Transaction};
use std::collections::HashMap;

const BACKFILL_BATCH_SIZE: i64 = 500;

#[derive(Debug, FromRow)]
struct UsageAggregateRow {
    usage_date: String,
    cli_type: String,
    provider_name: String,
    model_id: String,
    request_count: i64,
    success_count: i64,
    failure_count: i64,
    input_tokens: i64,
    cache_read_input_tokens: i64,
    cache_creation_input_tokens: i64,
    output_tokens: i64,
    elapsed_ms: i64,
}

#[derive(Debug, FromRow)]
struct RequestLogStatsRow {
    created_at: i64,
    cli_type: String,
    provider_name: String,
    model_id: Option<String>,
    source_model: Option<String>,
    status_code: Option<i64>,
    input_tokens: i64,
    cache_read_input_tokens: i64,
    cache_creation_input_tokens: i64,
    output_tokens: i64,
    elapsed_ms: i64,
}

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
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
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

pub async fn is_historical_backfill_done(stats_db: &SqlitePool) -> Result<bool, sqlx::Error> {
    let done: Option<(String,)> =
        sqlx::query_as("SELECT value FROM stats_meta WHERE key = 'historical_backfill_done'")
            .fetch_optional(stats_db)
            .await?;

    Ok(done.map(|row| row.0 == "1").unwrap_or(false))
}

pub async fn request_log_max_id(log_db: &SqlitePool) -> Result<i64, sqlx::Error> {
    let (max_id,): (i64,) = sqlx::query_as("SELECT COALESCE(MAX(id), 0) FROM request_logs")
        .fetch_one(log_db)
        .await?;
    Ok(max_id)
}

pub async fn backfill_historical_stats(
    log_db: &SqlitePool,
    stats_db: &SqlitePool,
    startup_max_log_id: i64,
) -> Result<(), sqlx::Error> {
    if is_historical_backfill_done(stats_db).await? {
        return Ok(());
    }

    let max_log_id = match get_meta_i64(stats_db, "historical_backfill_max_log_id").await? {
        Some(max_log_id) => max_log_id,
        None => {
            set_meta_value(
                stats_db,
                "historical_backfill_max_log_id",
                &startup_max_log_id.to_string(),
            )
            .await?;
            startup_max_log_id
        }
    };

    let mut last_id = get_meta_i64(stats_db, "historical_backfill_last_log_id")
        .await?
        .unwrap_or(0);

    while last_id < max_log_id {
        let batch_end = (last_id + BACKFILL_BATCH_SIZE).min(max_log_id);
        let rows = fetch_usage_aggregate(log_db, last_id, batch_end).await?;

        let mut tx = stats_db.begin().await?;
        for row in rows {
            upsert_usage_aggregate(&mut tx, &row).await?;
        }
        set_meta_value_tx(
            &mut tx,
            "historical_backfill_last_log_id",
            &batch_end.to_string(),
        )
        .await?;
        tx.commit().await?;

        last_id = batch_end;
        tokio::task::yield_now().await;
    }

    let mut tx = stats_db.begin().await?;
    set_meta_value_tx(&mut tx, "historical_backfill_done", "1").await?;
    set_meta_value_tx(
        &mut tx,
        "historical_backfill_last_log_id",
        &max_log_id.to_string(),
    )
    .await?;
    tx.commit().await?;

    tracing::info!(
        "Historical stats backfill completed up to log id {}",
        max_log_id
    );
    Ok(())
}

async fn fetch_usage_aggregate(
    log_db: &SqlitePool,
    start_id: i64,
    end_id: i64,
) -> Result<Vec<UsageAggregateRow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, RequestLogStatsRow>(
        r#"
        SELECT
            created_at,
            cli_type,
            provider_name,
            model_id,
            source_model,
            status_code,
            COALESCE(input_tokens, 0) as input_tokens,
            COALESCE(cache_read_input_tokens, 0) as cache_read_input_tokens,
            COALESCE(cache_creation_input_tokens, 0) as cache_creation_input_tokens,
            COALESCE(output_tokens, 0) as output_tokens,
            COALESCE(elapsed_ms, 0) as elapsed_ms
        FROM request_logs
        WHERE id > ? AND id <= ?
        ORDER BY id
        "#,
    )
    .bind(start_id)
    .bind(end_id)
    .fetch_all(log_db)
    .await?;

    let mut aggregates = HashMap::<(String, String, String, String), UsageAggregateRow>::new();

    for row in rows {
        let usage_date = Local
            .timestamp_opt(row.created_at, 0)
            .single()
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
        let model_id = row
            .model_id
            .as_deref()
            .or(row.source_model.as_deref())
            .unwrap_or("未知模型")
            .to_string();
        let key = (
            usage_date.clone(),
            row.cli_type.clone(),
            row.provider_name.clone(),
            model_id.clone(),
        );
        let entry = aggregates.entry(key).or_insert_with(|| UsageAggregateRow {
            usage_date,
            cli_type: row.cli_type.clone(),
            provider_name: row.provider_name.clone(),
            model_id,
            request_count: 0,
            success_count: 0,
            failure_count: 0,
            input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
            output_tokens: 0,
            elapsed_ms: 0,
        });

        entry.request_count += 1;
        if row
            .status_code
            .map(|code| (200..300).contains(&code))
            .unwrap_or(false)
        {
            entry.success_count += 1;
        } else {
            entry.failure_count += 1;
        }
        entry.input_tokens += row.input_tokens;
        entry.cache_read_input_tokens += row.cache_read_input_tokens;
        entry.cache_creation_input_tokens += row.cache_creation_input_tokens;
        entry.output_tokens += row.output_tokens;
        entry.elapsed_ms += row.elapsed_ms;
    }

    Ok(aggregates.into_values().collect())
}

async fn upsert_usage_aggregate(
    tx: &mut Transaction<'_, Sqlite>,
    row: &UsageAggregateRow,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO usage_daily_model (usage_date, cli_type, provider_name, model_id, request_count, success_count, failure_count, input_tokens, cache_read_input_tokens, cache_creation_input_tokens, output_tokens, elapsed_ms)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(usage_date, cli_type, provider_name, model_id) DO UPDATE SET
            request_count = request_count + excluded.request_count,
            success_count = success_count + excluded.success_count,
            failure_count = failure_count + excluded.failure_count,
            input_tokens = input_tokens + excluded.input_tokens,
            cache_read_input_tokens = cache_read_input_tokens + excluded.cache_read_input_tokens,
            cache_creation_input_tokens = cache_creation_input_tokens + excluded.cache_creation_input_tokens,
            output_tokens = output_tokens + excluded.output_tokens,
            elapsed_ms = elapsed_ms + excluded.elapsed_ms
        "#,
    )
    .bind(&row.usage_date)
    .bind(&row.cli_type)
    .bind(&row.provider_name)
    .bind(&row.model_id)
    .bind(row.request_count)
    .bind(row.success_count)
    .bind(row.failure_count)
    .bind(row.input_tokens)
    .bind(row.cache_read_input_tokens)
    .bind(row.cache_creation_input_tokens)
    .bind(row.output_tokens)
    .bind(row.elapsed_ms)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn get_meta_i64(stats_db: &SqlitePool, key: &str) -> Result<Option<i64>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM stats_meta WHERE key = ?")
        .bind(key)
        .fetch_optional(stats_db)
        .await?;

    Ok(row.and_then(|row| row.0.parse::<i64>().ok()))
}

async fn set_meta_value(stats_db: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO stats_meta (key, value, updated_at)
        VALUES (?, ?, strftime('%s', 'now'))
        ON CONFLICT(key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(key)
    .bind(value)
    .execute(stats_db)
    .await?;

    Ok(())
}

async fn set_meta_value_tx(
    tx: &mut Transaction<'_, Sqlite>,
    key: &str,
    value: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO stats_meta (key, value, updated_at)
        VALUES (?, ?, strftime('%s', 'now'))
        ON CONFLICT(key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(key)
    .bind(value)
    .execute(&mut **tx)
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
