use axum::{
    body::Body,
    extract::State,
    http::{Response, StatusCode},
};
use bytes::Bytes;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use std::io::Read;
use std::sync::Arc;
use std::time::Instant;
use tauri::Emitter;
use tokio::sync::{mpsc, Mutex};

use super::AppState;
use crate::db::models::{RequestLogInfo, RequestLogItem};
use crate::services::proxy::{
    apply_body_model_mapping, apply_url_model_mapping, apply_useragent_override, detect_cli_type,
    extract_model_from_body, extract_model_from_path, filter_headers, is_streaming,
    parse_token_usage, set_auth_header, CliType, TimeoutConfig, TokenUsage,
};
use crate::services::routing::select_provider;
use crate::services::{provider as provider_service, stats as stats_service};

// Catch-all proxy handler - forwards any non-API request to the appropriate provider
pub async fn proxy_handler_catchall(
    State(state): State<Arc<AppState>>,
    req: axum::http::Request<Body>,
) -> Result<Response<Body>, StatusCode> {
    let start_time = Instant::now();
    let method = req.method().clone();
    let headers = req.headers().clone();
    let uri = req.uri().clone();

    // Get the full path including query string
    let full_path = if let Some(query) = uri.query() {
        format!("{}?{}", uri.path(), query)
    } else {
        uri.path().to_string()
    };

    // Detect CLI type from User-Agent
    let cli_type = detect_cli_type(&headers);

    // Serialize client headers for logging
    let client_headers_json = serialize_headers(&headers);

    // Read request body
    let body_bytes = match axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024).await {
        Ok(bytes) => bytes.to_vec(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to read request body");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Store client body for logging
    let client_body_str = truncate_body(&body_bytes);

    // Check if streaming
    let streaming = is_streaming(&body_bytes, &full_path, cli_type);

    // Only learn from streaming requests since our test is streaming
    if streaming {
        match cli_type {
            CliType::ClaudeCode => crate::services::proxy::update_captured_claude_headers(&headers),
            CliType::Codex => crate::services::proxy::update_captured_codex_headers(&headers),
            CliType::Gemini => crate::services::proxy::update_captured_gemini_headers(&headers),
        }
    }

    // Extract model name before selecting provider (for blacklist filtering)
    let extracted_model = match cli_type {
        CliType::Gemini => extract_model_from_path(&full_path),
        _ => extract_model_from_body(&body_bytes),
    };

    // Select provider based on CLI type and model
    let provider_with_maps =
        match select_provider(&state.db, cli_type.as_str(), extracted_model.as_deref()).await {
            Ok(Some(p)) => p,
            Ok(None) => {
                tracing::warn!(cli_type = %cli_type, "No available provider");
                // Log system event
                let _ = stats_service::record_system_log(
                    &state.log_db,
                    "no_provider_available",
                    &format!("CLI 类型 {} 没有可用的服务商", cli_type),
                )
                .await;
                return Ok(Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"error": "No available provider configured"}"#,
                    ))
                    .unwrap());
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to select provider");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };

    let provider = &provider_with_maps.provider;
    let provider_id = provider.id;
    let provider_name = provider.name.clone();

    // Get timeout settings
    let timeouts = match sqlx::query_as::<_, (i64, i64, i64)>(
        "SELECT stream_first_byte_timeout, stream_idle_timeout, non_stream_timeout FROM timeout_settings WHERE id = 1",
    )
    .fetch_one(&state.db)
    .await
    {
        Ok((first, idle, non_stream)) => TimeoutConfig::from_db(first, idle, non_stream),
        Err(_) => TimeoutConfig::default(),
    };

    // Check if streaming
    // (streaming flag already determined above)

    // Apply model mapping and extract model info
    let (final_body, final_path, source_model, target_model) = match cli_type {
        CliType::Gemini => {
            let mapping = apply_url_model_mapping(
                &provider_with_maps,
                &full_path,
                &provider_with_maps.model_maps,
            );
            (
                body_bytes.clone(),
                mapping.path,
                mapping.source_model,
                mapping.target_model,
            )
        }
        _ => {
            let mapping = apply_body_model_mapping(&provider_with_maps, &body_bytes, &full_path);
            (
                mapping.body,
                mapping.path,
                mapping.source_model,
                mapping.target_model,
            )
        }
    };

    // Use target model if mapped, otherwise use source model
    let model_id = target_model.clone().or(source_model.clone());

    // Build upstream URL: base_url + original_path
    // e.g., base_url="https://api.example.com/v1", path="/responses" -> "https://api.example.com/v1/responses"
    let base_url = provider.base_url.trim_end_matches('/');
    let upstream_url = format!("{}{}", base_url, final_path);

    // Prepare headers - filter hop-by-hop headers and set auth
    let mut req_headers = filter_headers(&headers);
    set_auth_header(&mut req_headers, &provider.api_key, cli_type);

    // Apply User-Agent override (per-provider)
    let _original_ua =
        apply_useragent_override(&mut req_headers, provider.custom_useragent.as_deref());

    // Set content-type if not present
    if !req_headers.contains_key(reqwest::header::CONTENT_TYPE) {
        req_headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
    }

    // Explicitly set Content-Length to ensure correct body transmission
    // This is critical because we filtered out the original content-length header
    if !final_body.is_empty() {
        req_headers.insert(
            reqwest::header::CONTENT_LENGTH,
            final_body.len().to_string().parse().unwrap(),
        );
    }

    // Create HTTP client request
    let client = reqwest::Client::new();
    let request_builder = match method.as_str() {
        "GET" => client.get(&upstream_url),
        "POST" => client.post(&upstream_url),
        "PUT" => client.put(&upstream_url),
        "DELETE" => client.delete(&upstream_url),
        "PATCH" => client.patch(&upstream_url),
        _ => client.request(
            reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap_or(reqwest::Method::GET),
            &upstream_url,
        ),
    };

    let request_builder = request_builder.headers(req_headers);

    let request_builder = if !final_body.is_empty() {
        request_builder.body(final_body)
    } else {
        request_builder
    };

    // Build the request to inspect actual headers and body that will be sent
    let request = match request_builder.build() {
        Ok(req) => req,
        Err(e) => {
            tracing::error!(error = %e, "Failed to build request");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Log the actual request that will be sent
    let actual_forward_headers = serialize_reqwest_headers(request.headers());
    let actual_forward_body = request
        .body()
        .and_then(|b| b.as_bytes())
        .map(|bytes| truncate_body(bytes))
        .unwrap_or_default();

    // Build log info with actual request data
    let log_info = RequestLogInfo {
        client_headers: Some(client_headers_json),
        client_body: Some(client_body_str),
        forward_url: Some(upstream_url.clone()),
        forward_headers: Some(actual_forward_headers),
        forward_body: Some(actual_forward_body),
        ..Default::default()
    };

    // Execute request
    if streaming {
        handle_streaming_request(
            request,
            &client,
            &state,
            provider_id,
            &provider_name,
            cli_type,
            model_id.as_deref(),
            method.as_ref(),
            &full_path,
            start_time,
            timeouts,
            source_model.as_deref(),
            target_model.as_deref(),
            log_info,
        )
        .await
    } else {
        handle_non_streaming_request(
            request,
            &client,
            &state,
            provider_id,
            &provider_name,
            cli_type,
            model_id.as_deref(),
            method.as_ref(),
            &full_path,
            start_time,
            timeouts,
            source_model.as_deref(),
            target_model.as_deref(),
            log_info,
        )
        .await
    }
}

fn serialize_headers(headers: &axum::http::HeaderMap) -> String {
    let map: std::collections::HashMap<String, String> = headers
        .iter()
        .filter_map(|(k, v)| {
            let key = k.as_str().to_lowercase();
            v.to_str().ok().map(|v| (key, v.to_string()))
        })
        .collect();
    serde_json::to_string(&map).unwrap_or_default()
}

fn serialize_reqwest_headers(headers: &reqwest::header::HeaderMap) -> String {
    let map: std::collections::HashMap<String, String> = headers
        .iter()
        .filter_map(|(k, v)| {
            let key = k.as_str().to_lowercase();
            v.to_str().ok().map(|v| (key, v.to_string()))
        })
        .collect();
    serde_json::to_string(&map).unwrap_or_default()
}

fn truncate_body(body: &[u8]) -> String {
    String::from_utf8_lossy(body).into_owned()
}

/// Decompress gzip data if needed
fn maybe_decompress(body: &[u8], content_encoding: Option<&str>) -> Vec<u8> {
    if let Some(encoding) = content_encoding {
        if encoding.to_lowercase().contains("gzip") {
            let mut decoder = GzDecoder::new(body);
            let mut decompressed = Vec::new();
            if decoder.read_to_end(&mut decompressed).is_ok() {
                return decompressed;
            }
        }
    }
    body.to_vec()
}

async fn handle_streaming_request(
    request: reqwest::Request,
    client: &reqwest::Client,
    state: &Arc<AppState>,
    provider_id: i64,
    provider_name: &str,
    cli_type: CliType,
    model_id: Option<&str>,
    client_method: &str,
    client_path: &str,
    start_time: Instant,
    timeouts: TimeoutConfig,
    source_model: Option<&str>,
    target_model: Option<&str>,
    mut log_info: RequestLogInfo,
) -> Result<Response<Body>, StatusCode> {
    // Send request with timeout for first byte
    let response =
        match tokio::time::timeout(timeouts.first_byte_timeout, client.execute(request)).await {
            Ok(Ok(resp)) => resp,
            Ok(Err(e)) => {
                tracing::error!(error = %e, "Upstream request failed");
                if let Ok((was_blacklisted, prov_name)) =
                    provider_service::record_failure(&state.db, provider_id).await
                {
                    if was_blacklisted {
                        let _ = stats_service::record_system_log(
                            &state.log_db,
                            "provider_blacklisted",
                            &format!("服务商 {} 因连续失败已被加入黑名单", prov_name),
                        )
                        .await;
                    }
                }
                log_info.error_message = Some(format!("Upstream error: {}", e));
                record_request_stats(
                    state,
                    cli_type,
                    provider_name,
                    model_id,
                    None,
                    start_time.elapsed().as_millis() as i64,
                    0,
                    0,
                    client_method,
                    client_path,
                    source_model,
                    target_model,
                    Some(log_info),
                )
                .await;
                return Ok(Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"error": "Upstream error: {}"}}"#,
                        e
                    )))
                    .unwrap());
            }
            Err(_) => {
                tracing::error!("First byte timeout");
                if let Ok((was_blacklisted, prov_name)) =
                    provider_service::record_failure(&state.db, provider_id).await
                {
                    if was_blacklisted {
                        let _ = stats_service::record_system_log(
                            &state.log_db,
                            "provider_blacklisted",
                            &format!("服务商 {} 因连续失败已被加入黑名单", prov_name),
                        )
                        .await;
                    }
                }
                log_info.error_message = Some("First byte timeout".to_string());
                record_request_stats(
                    state,
                    cli_type,
                    provider_name,
                    model_id,
                    None,
                    start_time.elapsed().as_millis() as i64,
                    0,
                    0,
                    client_method,
                    client_path,
                    source_model,
                    target_model,
                    Some(log_info),
                )
                .await;
                return Ok(Response::builder()
                    .status(StatusCode::GATEWAY_TIMEOUT)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"error": "First byte timeout"}"#))
                    .unwrap());
            }
        };

    let status = response.status();
    let resp_headers = response.headers().clone();

    // Store provider response info
    log_info.provider_headers = Some(serialize_reqwest_headers(&resp_headers));

    // Build response headers
    let mut builder =
        Response::builder().status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK));

    for (name, value) in resp_headers.iter() {
        if let Ok(header_name) = axum::http::HeaderName::from_bytes(name.as_str().as_bytes()) {
            if let Ok(header_value) = axum::http::HeaderValue::from_bytes(value.as_bytes()) {
                builder = builder.header(header_name, header_value);
            }
        }
    }
    builder = builder.header("X-CCG-Provider", provider_name);

    // Create streaming body
    let is_success = status.is_success();

    // 使用共享状态收集chunks，确保即使stream被提前终止也能记录日志
    // 优化：只存储原始chunks，后台任务再解析（避免重复解析）
    let collected_chunks = Arc::new(Mutex::new(Vec::<Bytes>::new()));
    let collected_chunks_for_stream = collected_chunks.clone();

    // 创建channel用于通知stream结束
    let (stream_end_tx, mut stream_end_rx) = mpsc::channel::<()>(1);

    // 收集完整内容的上限（10MB），用于解析token
    const MAX_COLLECT_SIZE: usize = 10 * 1024 * 1024;

    let stream = async_stream::stream! {
        let mut byte_stream = response.bytes_stream();
        let idle_timeout = timeouts.idle_timeout;
        let mut chunk_count = 0usize;
        let mut total_bytes = 0usize;
        let mut collected_bytes = 0usize;

        loop {
            match tokio::time::timeout(idle_timeout, byte_stream.next()).await {
                Ok(Some(Ok(chunk))) => {
                    chunk_count += 1;
                    let chunk_size = chunk.len();
                    total_bytes += chunk_size;

                    // 收集chunk用于解析token（限制10MB防止极端情况）
                    if collected_bytes < MAX_COLLECT_SIZE {
                        let mut chunks = collected_chunks_for_stream.lock().await;
                        chunks.push(chunk.clone());
                        collected_bytes += chunk_size;
                        drop(chunks);  // 立即释放锁
                    }

                    tracing::debug!(
                        "[{}] Chunk #{}: size={} bytes, total={} bytes",
                        cli_type, chunk_count, chunk_size, total_bytes
                    );

                    yield Ok::<Bytes, std::io::Error>(chunk);
                }
                Ok(Some(Err(e))) => {
                    tracing::error!(
                        "[{}] Stream error after {} chunks, {} bytes: {}",
                        cli_type, chunk_count, total_bytes, e
                    );
                    break;
                }
                Ok(None) => {
                    // Stream completed normally
                    tracing::info!(
                        "[{}] Stream completed normally: {} chunks, {} bytes",
                        cli_type, chunk_count, total_bytes
                    );
                    break;
                }
                Err(_) => {
                    // Idle timeout
                    tracing::warn!(
                        "[{}] Stream idle timeout after {} chunks, {} bytes",
                        cli_type, chunk_count, total_bytes
                    );
                    // Send SSE error event
                    let error_event = "event: error\ndata: {\"error\": \"Stream idle timeout\"}\n\n".to_string();
                    yield Ok::<Bytes, std::io::Error>(Bytes::from(error_event));
                    break;
                }
            }
        }

        // Stream loop正常结束（无论是completed、error还是timeout）
        tracing::debug!("[{}] Stream loop ended naturally", cli_type);

        // 通知后台任务stream已结束
        let _ = stream_end_tx.send(()).await;
    };

    // Spawn后台任务记录日志 - 等待stream结束通知或超时
    let log_state = state.clone();
    let log_provider_name = provider_name.to_string();
    let log_model_id = model_id.map(|s| s.to_string());
    let log_client_method = client_method.to_string();
    let log_client_path = client_path.to_string();
    let log_provider_id = provider_id;
    let log_status = status;
    let log_resp_headers = resp_headers.clone();
    let log_is_success = is_success;
    let log_source_model = source_model.map(|s| s.to_string());
    let log_target_model = target_model.map(|s| s.to_string());

    tokio::spawn(async move {
        // 等待stream结束通知（已验证可靠，无需超时兜底）
        let _ = stream_end_rx.recv().await;
        tracing::debug!("[{}] Received stream end notification", cli_type);

        // 读取收集的chunks
        let chunks = collected_chunks.lock().await.clone();
        drop(collected_chunks); // 立即释放Arc引用

        // 一次性解析（避免重复解析，提升性能）
        let full_body: Vec<u8> = chunks.iter().flat_map(|c| c.iter()).copied().collect();
        let chunk_count = chunks.len();

        tracing::info!(
            "[{}] Processing stream log: {} chunks, {} bytes",
            cli_type,
            chunk_count,
            full_body.len()
        );

        // 解析token usage
        let mut usage = TokenUsage::default();
        if !full_body.is_empty() {
            // SSE 格式需要逐行解析，不能直接解析整个body
            // 注意：流式响应可能有多个usage更新，应该使用最后一个值
            let body_str = String::from_utf8_lossy(&full_body);
            for line in body_str.lines() {
                if line.starts_with("data:") {
                    // 提取 data: 后面的 JSON
                    let data = line.strip_prefix("data:").unwrap_or("").trim();
                    if data == "[DONE]" || data.is_empty() {
                        continue;
                    }
                    // 解析这一行的 JSON（如果有usage，会覆盖旧值）
                    parse_token_usage(data.as_bytes(), cli_type, &mut usage);
                    // 继续遍历所有行，使用最后一个值
                }
            }
        }

        tracing::debug!(
            "[{}] Parsed tokens: input={}, output={}",
            cli_type,
            usage.input_tokens,
            usage.output_tokens
        );

        // Update log info with response body
        let content_encoding = log_resp_headers
            .get("content-encoding")
            .and_then(|v| v.to_str().ok());
        let decompressed_body = maybe_decompress(&full_body, content_encoding);
        let mut final_log_info = log_info;
        final_log_info.provider_body = Some(truncate_body(&decompressed_body));

        // Record stats
        let elapsed = start_time.elapsed().as_millis() as i64;
        if log_is_success {
            if let Ok(had_failures) =
                provider_service::record_success(&log_state.db, log_provider_id).await
            {
                if had_failures {
                    let _ = stats_service::record_system_log(
                        &log_state.log_db,
                        "provider_recovered",
                        &format!("服务商 {} 已恢复正常", log_provider_name),
                    )
                    .await;
                }
            }
        } else if let Ok((was_blacklisted, prov_name)) =
            provider_service::record_failure(&log_state.db, log_provider_id).await
        {
            if was_blacklisted {
                let _ = stats_service::record_system_log(
                    &log_state.log_db,
                    "provider_blacklisted",
                    &format!("服务商 {} 因连续失败已被加入黑名单", prov_name),
                )
                .await;
            }
        }

        record_request_stats(
            &log_state,
            cli_type,
            &log_provider_name,
            log_model_id.as_deref(),
            Some(log_status.as_u16()),
            elapsed,
            usage.input_tokens,
            usage.output_tokens,
            &log_client_method,
            &log_client_path,
            log_source_model.as_deref(),
            log_target_model.as_deref(),
            Some(final_log_info),
        )
        .await;

        tracing::info!("[{}] Delayed log recording completed", cli_type);
    });

    Ok(builder.body(Body::from_stream(stream)).unwrap())
}

async fn handle_non_streaming_request(
    request: reqwest::Request,
    client: &reqwest::Client,
    state: &Arc<AppState>,
    provider_id: i64,
    provider_name: &str,
    cli_type: CliType,
    model_id: Option<&str>,
    client_method: &str,
    client_path: &str,
    start_time: Instant,
    timeouts: TimeoutConfig,
    source_model: Option<&str>,
    target_model: Option<&str>,
    mut log_info: RequestLogInfo,
) -> Result<Response<Body>, StatusCode> {
    // Send request with timeout
    let response =
        match tokio::time::timeout(timeouts.non_stream_timeout, client.execute(request)).await {
            Ok(Ok(resp)) => resp,
            Ok(Err(e)) => {
                tracing::error!(error = %e, "Upstream request failed");
                if let Ok((was_blacklisted, prov_name)) =
                    provider_service::record_failure(&state.db, provider_id).await
                {
                    if was_blacklisted {
                        let _ = stats_service::record_system_log(
                            &state.log_db,
                            "provider_blacklisted",
                            &format!("服务商 {} 因连续失败已被加入黑名单", prov_name),
                        )
                        .await;
                    }
                }
                log_info.error_message = Some(format!("Upstream error: {}", e));
                record_request_stats(
                    state,
                    cli_type,
                    provider_name,
                    model_id,
                    None,
                    start_time.elapsed().as_millis() as i64,
                    0,
                    0,
                    client_method,
                    client_path,
                    source_model,
                    target_model,
                    Some(log_info),
                )
                .await;
                return Ok(Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"error": "Upstream error: {}"}}"#,
                        e
                    )))
                    .unwrap());
            }
            Err(_) => {
                tracing::error!("Request timeout");
                if let Ok((was_blacklisted, prov_name)) =
                    provider_service::record_failure(&state.db, provider_id).await
                {
                    if was_blacklisted {
                        let _ = stats_service::record_system_log(
                            &state.log_db,
                            "provider_blacklisted",
                            &format!("服务商 {} 因连续失败已被加入黑名单", prov_name),
                        )
                        .await;
                    }
                }
                log_info.error_message = Some("Request timeout".to_string());
                record_request_stats(
                    state,
                    cli_type,
                    provider_name,
                    model_id,
                    None,
                    start_time.elapsed().as_millis() as i64,
                    0,
                    0,
                    client_method,
                    client_path,
                    source_model,
                    target_model,
                    Some(log_info),
                )
                .await;
                return Ok(Response::builder()
                    .status(StatusCode::GATEWAY_TIMEOUT)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"error": "Request timeout"}"#))
                    .unwrap());
            }
        };

    let status = response.status();
    let resp_headers = response.headers().clone();
    let is_success = status.is_success();

    // Store provider response info
    log_info.provider_headers = Some(serialize_reqwest_headers(&resp_headers));

    // Read response body
    let body_bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!(error = %e, "Failed to read response body");
            if let Ok((was_blacklisted, prov_name)) =
                provider_service::record_failure(&state.db, provider_id).await
            {
                if was_blacklisted {
                    let _ = stats_service::record_system_log(
                        &state.log_db,
                        "provider_blacklisted",
                        &format!("服务商 {} 因连续失败已被加入黑名单", prov_name),
                    )
                    .await;
                }
            }
            log_info.error_message = Some(format!("Failed to read response body: {}", e));
            record_request_stats(
                state,
                cli_type,
                provider_name,
                model_id,
                Some(status.as_u16()),
                start_time.elapsed().as_millis() as i64,
                0,
                0,
                client_method,
                client_path,
                source_model,
                target_model,
                Some(log_info),
            )
            .await;
            return Err(StatusCode::BAD_GATEWAY);
        }
    };

    // Decompress if needed for logging and token parsing
    let content_encoding = resp_headers
        .get("content-encoding")
        .and_then(|v| v.to_str().ok());
    let decompressed_body = maybe_decompress(&body_bytes, content_encoding);

    // Store response body for logging (use decompressed version)
    log_info.provider_body = Some(truncate_body(&decompressed_body));

    // Parse token usage (use decompressed body)
    let mut usage = TokenUsage::default();
    parse_token_usage(&decompressed_body, cli_type, &mut usage);

    // Record success/failure
    if is_success {
        if let Ok(had_failures) = provider_service::record_success(&state.db, provider_id).await {
            if had_failures {
                let _ = stats_service::record_system_log(
                    &state.log_db,
                    "provider_recovered",
                    &format!("服务商 {} 已恢复正常", provider_name),
                )
                .await;
            }
        }
    } else if let Ok((was_blacklisted, prov_name)) =
        provider_service::record_failure(&state.db, provider_id).await
    {
        if was_blacklisted {
            let _ = stats_service::record_system_log(
                &state.log_db,
                "provider_blacklisted",
                &format!("服务商 {} 因连续失败已被加入黑名单", prov_name),
            )
            .await;
        }
    }

    // Record stats
    let elapsed = start_time.elapsed().as_millis() as i64;
    record_request_stats(
        state,
        cli_type,
        provider_name,
        model_id,
        Some(status.as_u16()),
        elapsed,
        usage.input_tokens,
        usage.output_tokens,
        client_method,
        client_path,
        source_model,
        target_model,
        Some(log_info),
    )
    .await;

    // Build response
    let mut builder =
        Response::builder().status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK));

    for (name, value) in resp_headers.iter() {
        if let Ok(header_name) = axum::http::HeaderName::from_bytes(name.as_str().as_bytes()) {
            if let Ok(header_value) = axum::http::HeaderValue::from_bytes(value.as_bytes()) {
                builder = builder.header(header_name, header_value);
            }
        }
    }
    builder = builder.header("X-CCG-Provider", provider_name);

    Ok(builder.body(Body::from(body_bytes)).unwrap())
}

async fn record_request_stats(
    state: &Arc<AppState>,
    cli_type: CliType,
    provider_name: &str,
    model_id: Option<&str>,
    status_code: Option<u16>,
    elapsed_ms: i64,
    input_tokens: i64,
    output_tokens: i64,
    client_method: &str,
    client_path: &str,
    source_model: Option<&str>,
    target_model: Option<&str>,
    log_info: Option<RequestLogInfo>,
) {
    // Derive success from status_code (200-299 = success)
    let success = status_code
        .map(|code| (200..300).contains(&code))
        .unwrap_or(false);

    // Record to request_logs and get the inserted ID
    let log_id = match stats_service::record_request_log(
        &state.log_db,
        cli_type.as_str(),
        provider_name,
        model_id,
        status_code,
        elapsed_ms,
        input_tokens,
        output_tokens,
        client_method,
        client_path,
        source_model,
        target_model,
        log_info,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::error!(error = %e, "Failed to record request log");
            return;
        }
    };

    // Query the inserted log item
    let log_item = sqlx::query_as::<_, RequestLogItem>(
        "SELECT id, created_at, cli_type, provider_name, model_id, status_code, elapsed_ms, input_tokens, output_tokens, client_method, client_path, source_model, target_model FROM request_logs WHERE id = ?",
    )
    .bind(log_id)
    .fetch_one(&state.log_db)
    .await;

    // Emit event to frontend
    if let Ok(item) = log_item {
        if let Err(e) = state.app_handle.emit("request-log-new", item) {
            tracing::error!(error = %e, "Failed to emit request log event");
        }
    }

    // Record to usage_daily
    let _ = stats_service::record_request(
        &state.log_db,
        provider_name,
        cli_type.as_str(),
        success,
        input_tokens,
        output_tokens,
    )
    .await;
}
