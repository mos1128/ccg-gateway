use crate::db::models::TestProviderResult;
use sqlx::SqlitePool;

/// Record a successful request for a provider
/// Resets consecutive_failures to 0
/// Returns (had_previous_failures) to indicate if the provider was recovering
pub async fn record_success(db: &SqlitePool, provider_id: i64) -> Result<bool, sqlx::Error> {
    let now = chrono::Utc::now().timestamp();

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
    let now = chrono::Utc::now().timestamp();
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
    let now = chrono::Utc::now().timestamp();

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
    let api_key = provider.api_key.clone();
    let custom_ua = provider.custom_useragent.clone();

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

    // 3. Build request per CLI type (all use stream mode)
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default();

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert("accept", "text/event-stream".parse().unwrap());
    headers.insert("accept-encoding", "identity".parse().unwrap());

    let (url, body_json) = match cli_type.as_str() {
        "claude_code" => {
            // Anthropic native: POST /v1/messages with stream
            let url = format!("{}/v1/messages", base_url);
            let body = serde_json::json!({
                "model": actual_model,
                "messages": [{"role": "user", "content": [{"type": "text", "text": "今天天气不错"}]}],
                "system": [{"type": "text", "text": "You are Claude Code, Anthropic's official CLI for Claude."}],
                "max_tokens": 1024,
                "thinking": {"type": "adaptive"},
                "stream": true
            });

            headers.insert("accept", "application/json".parse().unwrap());
            headers.insert(
                "accept-encoding",
                "gzip, deflate, br, zstd".parse().unwrap(),
            );
            headers.insert("x-app", "cli".parse().unwrap());

            if let Ok(v) = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", api_key)) {
                headers.insert(reqwest::header::AUTHORIZATION, v);
            }

            headers.insert(
                reqwest::header::USER_AGENT,
                "claude-cli/2.1.121 (external, cli)".parse().unwrap(),
            );

            // Dynamically learn anthropic-beta from captured headers if available, otherwise default
            let mut beta_value = "claude-code-20250219,context-1m-2025-08-07,interleaved-thinking-2025-05-14,redact-thinking-2026-02-12,context-management-2025-06-27,prompt-caching-scope-2026-01-05,advanced-tool-use-2025-11-20,effort-2025-11-24".to_string();
            let captured_headers = crate::services::proxy::get_captured_claude_headers();
            for (k, v) in &captured_headers.headers {
                if k.to_lowercase() == "anthropic-beta" {
                    beta_value = v.clone();
                    break;
                }
            }
            if let Ok(v) = reqwest::header::HeaderValue::from_str(&beta_value) {
                headers.insert("anthropic-beta", v);
            }

            (url, body)
        }
        "codex" => {
            // Codex format: base_url already includes /v1
            let url = format!("{}/responses", base_url);
            let body = serde_json::json!({
                "model": actual_model,
                "instructions": "You are Codex, a coding agent based on GPT-5.",
                "input": [{"type": "message", "role": "user", "content": [{"type": "input_text", "text": "今天天气不错"}]}],
                "reasoning": {"effort": "high"},
                "stream": true
            });
            if let Ok(v) = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", api_key)) {
                headers.insert(reqwest::header::AUTHORIZATION, v);
            }

            headers.insert("accept", "text/event-stream".parse().unwrap());
            headers.insert("originator", "codex-tui".parse().unwrap());

            let mut user_agent =
                "codex-tui/0.125.0 (Windows 10.0.22631; x86_64) unknown (codex-tui; 0.125.0)"
                    .to_string();
            let captured_headers = crate::services::proxy::get_captured_codex_headers();
            for (k, v) in &captured_headers.headers {
                if k.to_lowercase() == "user-agent" {
                    user_agent = v.clone();
                    break;
                }
            }
            if let Ok(v) = reqwest::header::HeaderValue::from_str(&user_agent) {
                headers.insert(reqwest::header::USER_AGENT, v);
            }

            (url, body)
        }
        "gemini" => {
            // Gemini streaming: streamGenerateContent with alt=sse
            let url = format!(
                "{}/v1beta/models/{}:streamGenerateContent?alt=sse",
                base_url, actual_model
            );
            let body = serde_json::json!({
                "contents": [{"role": "user", "parts": [{"text": "今天天气不错"}]}],
                "systemInstruction": {"parts": [{"text": "You are Gemini CLI, an interactive CLI agent specializing in software engineering tasks."}]},
                "generationConfig": {"temperature": 1.0, "topP": 0.95, "topK": 64, "thinkingConfig": {"includeThoughts": true}}
            });

            headers.insert("accept", "*/*".parse().unwrap());
            headers.insert("content-type", "application/json".parse().unwrap());
            headers.insert("accept-encoding", "gzip, deflate".parse().unwrap());

            if let Ok(v) = reqwest::header::HeaderValue::from_str(&api_key) {
                headers.insert("x-goog-api-key", v);
            }

            // Apply captured headers or defaults
            let mut user_agent =
                "GeminiCLI/0.39.1/gemini-3.1-pro-preview (win32; x64; terminal)".to_string();

            let captured_headers = crate::services::proxy::get_captured_gemini_headers();
            for (k, v) in &captured_headers.headers {
                if k.to_lowercase() == "user-agent" {
                    user_agent = v.clone();
                    break;
                }
            }

            if let Ok(v) = reqwest::header::HeaderValue::from_str(&user_agent) {
                headers.insert(reqwest::header::USER_AGENT, v);
            }

            (url, body)
        }
        _ => {
            // Fallback: OpenAI compatible
            let url = format!("{}/v1/chat/completions", base_url);
            let body = serde_json::json!({
                "model": actual_model,
                "messages": [{"role": "user", "content": "今天天气不错"}],
                "stream": true,
                "max_tokens": 1024
            });
            if let Ok(v) = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", api_key)) {
                headers.insert(reqwest::header::AUTHORIZATION, v);
            }
            (url, body)
        }
    };

    // Apply custom UA if configured
    if let Some(ref ua) = custom_ua {
        if !ua.is_empty() {
            if let Ok(v) = reqwest::header::HeaderValue::from_str(ua) {
                headers.insert(reqwest::header::USER_AGENT, v);
            }
        }
    }

    let request_body = serde_json::to_string_pretty(&body_json).unwrap_or_default();
    let request_headers = headers_to_json(&headers);

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
    let response = client
        .post(&url)
        .headers(headers)
        .json(&body_json)
        .send()
        .await;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    match response {
        Ok(resp) => {
            let status_code = resp.status().as_u16();
            let response_headers = headers_to_json(resp.headers());
            if status_code >= 200 && status_code < 300 {
                // Stream mode: wait for first chunk only
                use futures_util::StreamExt;
                let mut stream = resp.bytes_stream();
                let first_chunk =
                    tokio::time::timeout(std::time::Duration::from_secs(30), stream.next()).await;
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
                // Non-2xx: return raw response body for debugging
                let body_text = resp.text().await.unwrap_or_default();
                TestProviderResult {
                    provider_id,
                    provider_name,
                    actual_model,
                    status_code: Some(status_code),
                    elapsed_ms,
                    response_text: body_text.clone(),
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
