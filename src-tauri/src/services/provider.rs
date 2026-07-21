use crate::db::models::TestProviderResult;
use crate::time::now_timestamp;
use sqlx::SqlitePool;

/// Default timeout (seconds) used when the DB setting is unavailable.
const DEFAULT_STREAM_FIRST_BYTE_TIMEOUT: u64 = 30;

/// Read `stream_first_byte_timeout` from timeout_settings.
/// Falls back to 30s if the row is missing or the query fails.
pub async fn get_stream_first_byte_timeout(db: &SqlitePool) -> u64 {
    sqlx::query_as::<_, (i64,)>(
        "SELECT stream_first_byte_timeout FROM timeout_settings WHERE id = 1",
    )
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .map(|(v,)| v as u64)
    .unwrap_or(DEFAULT_STREAM_FIRST_BYTE_TIMEOUT)
}

/// Record a successful request for a provider
/// Resets consecutive_failures to 0
/// Returns (had_previous_failures) to indicate if the provider was recovering
pub async fn record_success(db: &SqlitePool, provider_id: i64) -> Result<bool, sqlx::Error> {
    let now = now_timestamp();

    // Check if provider had previous failures
    let had_failures: Option<(i64,)> =
        sqlx::query_as("SELECT consecutive_failures FROM providers WHERE id = ?")
            .bind(provider_id)
            .fetch_optional(db)
            .await?;

    let had_previous_failures = had_failures.map(|(cf,)| cf > 0).unwrap_or(false);

    sqlx::query(
        r#"
        UPDATE providers
        SET consecutive_failures = 0,
            blacklisted_until = NULL,
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(now)
    .bind(provider_id)
    .execute(db)
    .await?;

    Ok(had_previous_failures)
}

/// Record a failed request for a provider
/// Increments consecutive_failures and blacklists if threshold is reached
/// If the provider was blacklisted but blacklist has expired, resets count before incrementing
/// Uses a single write transaction to avoid lost updates under concurrent failures
/// Returns (was_blacklisted, provider_name) tuple
pub async fn record_failure(
    db: &SqlitePool,
    provider_id: i64,
) -> Result<(bool, String), sqlx::Error> {
    let now = now_timestamp();
    let mut tx = db.begin().await?;

    let result = sqlx::query(
        r#"
        UPDATE providers
        SET consecutive_failures = CASE
                WHEN blacklisted_until IS NOT NULL AND blacklisted_until <= ? THEN 1
                ELSE consecutive_failures + 1
            END,
            blacklisted_until = CASE
                WHEN (
                    CASE
                        WHEN blacklisted_until IS NOT NULL AND blacklisted_until <= ? THEN 1
                        ELSE consecutive_failures + 1
                    END
                ) >= failure_threshold THEN ? + blacklist_minutes * 60
                WHEN blacklisted_until IS NOT NULL AND blacklisted_until <= ? THEN NULL
                ELSE blacklisted_until
            END,
            updated_at = ?
        WHERE id = ?
          AND (blacklisted_until IS NULL OR blacklisted_until <= ?)
        "#,
    )
    .bind(now)
    .bind(now)
    .bind(now)
    .bind(now)
    .bind(now)
    .bind(provider_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    let provider: Option<(String, i64, i64)> = sqlx::query_as(
        "SELECT name, consecutive_failures, failure_threshold FROM providers WHERE id = ?",
    )
    .bind(provider_id)
    .fetch_optional(&mut *tx)
    .await?;

    tx.commit().await?;

    let Some((provider_name, failures, threshold)) = provider else {
        return Ok((false, String::new()));
    };

    let should_blacklist = result.rows_affected() > 0 && failures >= threshold;
    if should_blacklist {
        tracing::warn!(
            provider_id = provider_id,
            failures = failures,
            "Provider blacklisted due to consecutive failures"
        );
    }

    Ok((should_blacklist, provider_name))
}

/// Reset provider failures and remove blacklist
pub async fn reset_failures(db: &SqlitePool, provider_id: i64) -> Result<(), sqlx::Error> {
    let now = now_timestamp();

    sqlx::query(
        r#"
        UPDATE providers
        SET consecutive_failures = 0,
            blacklisted_until = NULL,
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(now)
    .bind(provider_id)
    .execute(db)
    .await?;

    Ok(())
}

/// Test a single provider's model availability using streaming.
/// Sends a lightweight request with stream=true, succeeds on first chunk received.
pub async fn test_provider_model(
    db: &SqlitePool,
    provider_id: i64,
    model_name: &str,
    test_text: Option<&str>,
    timeout_secs: u64,
) -> TestProviderResult {
    // 1. Load provider
    let provider = match sqlx::query_as::<_, crate::db::models::Provider>(
        "SELECT * FROM providers WHERE id = ?",
    )
    .bind(provider_id)
    .fetch_optional(db)
    .await
    {
        Ok(Some(p)) => p,
        Ok(None) => {
            return TestProviderResult {
                provider_id,
                provider_name: "Unknown".to_string(),
                actual_model: model_name.to_string(),
                status_code: None,
                elapsed_ms: 0,
                response_text: "Provider not found".to_string(),
                request_url: String::new(),
                request_headers: String::new(),
                request_body: String::new(),
                response_headers: String::new(),
                response_body: String::new(),
            };
        }
        Err(e) => {
            return TestProviderResult {
                provider_id,
                provider_name: "Unknown".to_string(),
                actual_model: model_name.to_string(),
                status_code: None,
                elapsed_ms: 0,
                response_text: format!("DB error: {}", e),
                request_url: String::new(),
                request_headers: String::new(),
                request_body: String::new(),
                response_headers: String::new(),
                response_body: String::new(),
            };
        }
    };

    let provider_name = provider.name.clone();
    let cli_type = provider.cli_type.clone();
    let base_url = provider.base_url.trim_end_matches('/').to_string();
    let protocol = match provider.protocol.parse::<crate::db::models::Protocol>() {
        Ok(protocol) => protocol,
        Err(error) => {
            return TestProviderResult {
                provider_id,
                provider_name,
                actual_model: model_name.to_string(),
                status_code: None,
                elapsed_ms: 0,
                response_text: error,
                request_url: String::new(),
                request_headers: String::new(),
                request_body: String::new(),
                response_headers: String::new(),
                response_body: String::new(),
            };
        }
    };

    // 2. Resolve model mapping
    let model_maps = sqlx::query_as::<_, crate::db::models::ProviderModelMap>(
        "SELECT * FROM provider_model_map WHERE provider_id = ? AND enabled = 1 ORDER BY id",
    )
    .bind(provider_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let mut actual_model = model_name.to_string();
    for map in &model_maps {
        if crate::services::proxy::wildcard_match(&map.source_model, model_name) {
            actual_model = map.target_model.clone();
            break;
        }
    }

    // 3. Build a synthetic Agent request from the probe template, then route it
    //    through the same upstream-request constructor used by real forwarding.
    //    This keeps auth injection, hop-by-hop filtering, and UA override in one
    //    place; the only difference from a real request is that the body and
    //    Agent-specific headers come from the template (refined by captured
    //    headers) instead of a live CLI.
    let client = {
        static TEST_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
        TEST_CLIENT
            .get_or_init(|| reqwest::Client::builder().build().unwrap_or_default())
            .clone()
    };

    let probe =
        crate::services::proxy::build_probe_request(&cli_type, protocol, &actual_model, test_text);

    // Assemble the synthetic client header map: start from the template's
    // Agent-specific headers, then let captured headers (if any) refine the UA
    // / anthropic-beta so the probe stays close to the currently installed CLI.
    let mut client_headers = axum::http::HeaderMap::new();
    if let Ok(v) = reqwest::header::HeaderValue::from_str("application/json") {
        client_headers.insert(reqwest::header::CONTENT_TYPE, v);
    }
    for (name, value) in &probe.extra_headers {
        if let (Ok(n), Ok(v)) = (
            reqwest::header::HeaderName::from_bytes(name.as_bytes()),
            reqwest::header::HeaderValue::from_str(value),
        ) {
            client_headers.insert(n, v);
        }
    }
    apply_captured_header_override(&mut client_headers, &cli_type, protocol);

    let url = crate::services::proxy::join_upstream_url(&base_url, &probe.path);

    let request = match crate::services::proxy::build_upstream_request(
        &client,
        &provider,
        protocol,
        &url,
        &client_headers,
        probe.body.clone(),
        probe.method.clone(),
    ) {
        Ok(req) => req,
        Err(e) => {
            return TestProviderResult {
                provider_id,
                provider_name,
                actual_model,
                status_code: None,
                elapsed_ms: 0,
                response_text: format!("Failed to build request: {}", e),
                request_url: url,
                request_headers: headers_to_json(&client_headers),
                request_body: String::from_utf8_lossy(&probe.body).into_owned(),
                response_headers: String::new(),
                response_body: String::new(),
            };
        }
    };

    let request_body = String::from_utf8_lossy(&probe.body).into_owned();
    let request_headers = headers_to_json(request.headers());

    // Log request details
    tracing::info!(
        "[模型测试] {}\nURL: {}\nHeaders: {}\nBody: {}",
        provider_name,
        url,
        request_headers,
        request_body
    );

    // 4. Send request, measure time to first chunk
    let start = std::time::Instant::now();
    let response = match tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        client.execute(request),
    )
    .await
    {
        Ok(r) => r,
        Err(_) => {
            return TestProviderResult {
                provider_id,
                provider_name,
                actual_model,
                status_code: None,
                elapsed_ms: start.elapsed().as_millis() as u64,
                response_text: "Request timeout".to_string(),
                request_url: url,
                request_headers,
                request_body,
                response_headers: String::new(),
                response_body: String::new(),
            };
        }
    };
    let elapsed_ms = start.elapsed().as_millis() as u64;

    match response {
        Ok(resp) => {
            let status_code = resp.status().as_u16();
            let response_headers = headers_to_json(resp.headers());
            if status_code >= 200 && status_code < 300 && probe.streaming {
                // Stream mode: wait for first chunk only
                use futures_util::StreamExt;
                let mut stream = resp.bytes_stream();
                let first_chunk = tokio::time::timeout(
                    std::time::Duration::from_secs(timeout_secs),
                    stream.next(),
                )
                .await;
                let first_chunk_ms = start.elapsed().as_millis() as u64;

                let (response_text, raw_chunk) = match first_chunk {
                    Ok(Some(Ok(bytes))) => {
                        let text = String::from_utf8_lossy(&bytes).to_string();
                        ("请求成功".to_string(), text)
                    }
                    Ok(Some(Err(e))) => (format!("Stream error: {}", e), String::new()),
                    Ok(None) => ("Empty stream".to_string(), String::new()),
                    Err(_) => ("Stream timeout".to_string(), String::new()),
                };

                TestProviderResult {
                    provider_id,
                    provider_name,
                    actual_model,
                    status_code: Some(status_code),
                    elapsed_ms: first_chunk_ms,
                    response_text,
                    request_url: url,
                    request_headers,
                    request_body,
                    response_headers,
                    response_body: raw_chunk,
                }
            } else {
                let body_text = resp.text().await.unwrap_or_default();
                let success = status_code >= 200 && status_code < 300;
                TestProviderResult {
                    provider_id,
                    provider_name,
                    actual_model,
                    status_code: Some(status_code),
                    elapsed_ms,
                    response_text: if success {
                        "请求成功".to_string()
                    } else {
                        body_text.clone()
                    },
                    request_url: url,
                    request_headers,
                    request_body,
                    response_headers,
                    response_body: body_text,
                }
            }
        }
        Err(e) => TestProviderResult {
            provider_id,
            provider_name,
            actual_model,
            status_code: None,
            elapsed_ms,
            response_text: format!("{}", e),
            request_url: url,
            request_headers,
            request_body,
            response_headers: String::new(),
            response_body: String::new(),
        },
    }
}

/// Convert headers to formatted JSON string
fn headers_to_json(headers: &reqwest::header::HeaderMap) -> String {
    let mut map = serde_json::Map::new();
    for (name, value) in headers.iter() {
        let key = name.as_str().to_string();
        let val = value.to_str().unwrap_or("<binary>").to_string();
        map.insert(key, serde_json::Value::String(val));
    }
    serde_json::to_string_pretty(&map).unwrap_or_default()
}

/// Refine the synthetic probe headers with values learned from real CLI
/// traffic, when available. Claude reuses the captured `anthropic-beta`;
/// Codex/Gemini reuse the captured `user-agent`. Falls back silently to the
/// template defaults when no capture exists (cold start).
fn apply_captured_header_override(
    headers: &mut axum::http::HeaderMap,
    agent_id: &str,
    protocol: crate::db::models::Protocol,
) {
    for (key, value) in crate::services::proxy::get_captured_headers(agent_id, protocol) {
        if let (Ok(name), Ok(value)) = (
            axum::http::HeaderName::from_bytes(key.as_bytes()),
            axum::http::HeaderValue::from_str(&value),
        ) {
            headers.insert(name, value);
        }
    }
}
