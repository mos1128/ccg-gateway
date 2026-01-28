use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{Response, StatusCode},
    Json,
};
use bytes::Bytes;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Mutex};
use flate2::read::GzDecoder;
use std::io::Read;

use super::AppState;
use crate::db::models::{
    Provider, ProviderCreate, ProviderResponse, ProviderUpdate,
    GatewaySettings, TimeoutSettings, TimeoutSettingsUpdate,
    RequestLogItem, RequestLogDetail, PaginatedLogs,
    SystemLogItem, SystemLogListResponse,
    DailyStats,
    SystemStatus,
};
use crate::services::proxy::{
    apply_body_model_mapping, apply_url_model_mapping, detect_cli_type,
    filter_headers, is_streaming, parse_token_usage, set_auth_header,
    CliType, TimeoutConfig, TokenUsage,
};
use crate::services::routing::select_provider;
use crate::services::{provider as provider_service, stats as stats_service};
use crate::services::stats::RequestLogInfo;

// Common query params
#[derive(Debug, Deserialize)]
pub struct PaginatedQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

#[derive(Debug, Deserialize)]
pub struct ProviderQuery {
    pub cli_type: Option<String>,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    20
}

// Error response
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

fn error_response(msg: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: msg.into(),
        }),
    )
}

fn db_error(e: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    error_response(e.to_string())
}

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

    // Store client body for logging (truncate if too large)
    let client_body_str = truncate_body(&body_bytes);

    // Select provider based on CLI type
    let provider_with_maps = match select_provider(&state.db, cli_type.as_str()).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            tracing::warn!(cli_type = %cli_type, "No available provider");
            // Log system event
            let _ = stats_service::record_system_log(
                &state.log_db,
                "warn",
                "no_provider_available",
                &format!("No available provider for CLI type: {}", cli_type),
                None,
                None,
            ).await;
            return Ok(Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"error": "No available provider configured"}"#))
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
    let streaming = is_streaming(&body_bytes, &full_path, cli_type);

    // Apply model mapping and extract model info
    let (final_body, final_path, source_model, target_model) = match cli_type {
        CliType::Gemini => {
            let mapping = apply_url_model_mapping(&provider_with_maps, &full_path, &provider_with_maps.model_maps);
            (body_bytes.clone(), mapping.path, mapping.source_model, mapping.target_model)
        }
        _ => {
            let mapping = apply_body_model_mapping(&provider_with_maps, &body_bytes, &full_path);
            (mapping.body, mapping.path, mapping.source_model, mapping.target_model)
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

    // Set content-type if not present
    if !req_headers.contains_key(reqwest::header::CONTENT_TYPE) {
        req_headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
    }

    // Serialize forward headers for logging (mask sensitive headers)
    let forward_headers_json = serialize_reqwest_headers(&req_headers);
    let forward_body_str = truncate_body(&final_body);

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

    // Build log info
    let log_info = RequestLogInfo {
        client_headers: Some(client_headers_json),
        client_body: Some(client_body_str),
        forward_url: Some(upstream_url.clone()),
        forward_headers: Some(forward_headers_json),
        forward_body: Some(forward_body_str),
        ..Default::default()
    };

    // Execute request
    if streaming {
        handle_streaming_request(
            request_builder,
            &state,
            provider_id,
            &provider_name,
            cli_type,
            model_id.as_deref(),
            method.as_ref(),
            &full_path,
            start_time,
            timeouts,
            log_info,
        )
        .await
    } else {
        handle_non_streaming_request(
            request_builder,
            &state,
            provider_id,
            &provider_name,
            cli_type,
            model_id.as_deref(),
            method.as_ref(),
            &full_path,
            start_time,
            timeouts,
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
    const MAX_SIZE: usize = 100 * 1024; // 100KB
    let s = String::from_utf8_lossy(body);
    if s.len() > MAX_SIZE {
        format!("{}...[truncated]", &s[..MAX_SIZE])
    } else {
        s.to_string()
    }
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
    request_builder: reqwest::RequestBuilder,
    state: &Arc<AppState>,
    provider_id: i64,
    provider_name: &str,
    cli_type: CliType,
    model_id: Option<&str>,
    client_method: &str,
    client_path: &str,
    start_time: Instant,
    timeouts: TimeoutConfig,
    mut log_info: RequestLogInfo,
) -> Result<Response<Body>, StatusCode> {
    // Send request with timeout for first byte
    let response = match tokio::time::timeout(
        timeouts.first_byte_timeout,
        request_builder.send(),
    )
    .await
    {
        Ok(Ok(resp)) => resp,
        Ok(Err(e)) => {
            tracing::error!(error = %e, "Upstream request failed");
            if let Ok((was_blacklisted, prov_name)) = provider_service::record_failure(&state.db, provider_id).await {
                if was_blacklisted {
                    let _ = stats_service::record_system_log(
                        &state.log_db,
                        "warn",
                        "provider_blacklisted",
                        &format!("Provider {} blacklisted due to consecutive failures", prov_name),
                        Some(&prov_name),
                        Some(&format!("{{\"error\": \"{}\"}}", e)),
                    ).await;
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
                Some(log_info),
            )
            .await;
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"error": "Upstream error: {}"}}"#, e)))
                .unwrap());
        }
        Err(_) => {
            tracing::error!("First byte timeout");
            if let Ok((was_blacklisted, prov_name)) = provider_service::record_failure(&state.db, provider_id).await {
                if was_blacklisted {
                    let _ = stats_service::record_system_log(
                        &state.log_db,
                        "warn",
                        "provider_blacklisted",
                        &format!("Provider {} blacklisted due to consecutive failures", prov_name),
                        Some(&prov_name),
                        Some("{\"error\": \"First byte timeout\"}"),
                    ).await;
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
    log_info.response_headers = Some(serialize_reqwest_headers(&resp_headers));

    // Build response headers
    let mut builder = Response::builder()
        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK));

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

    let stream = async_stream::stream! {
        let mut byte_stream = response.bytes_stream();
        let idle_timeout = timeouts.idle_timeout;
        let mut chunk_count = 0usize;
        let mut total_bytes = 0usize;

        loop {
            match tokio::time::timeout(idle_timeout, byte_stream.next()).await {
                Ok(Some(Ok(chunk))) => {
                    chunk_count += 1;
                    let chunk_size = chunk.len();
                    total_bytes += chunk_size;
                    
                    // 只收集chunk到共享状态（快速操作，减少锁持有时间）
                    // 限制总大小避免内存占用过大
                    if total_bytes <= 100 * 1024 {
                        let mut chunks = collected_chunks_for_stream.lock().await;
                        chunks.push(chunk.clone());
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
    
    tokio::spawn(async move {
        // 等待stream结束通知（已验证可靠，无需超时兜底）
        let _ = stream_end_rx.recv().await;
        tracing::debug!("[{}] Received stream end notification", cli_type);
        
        // 读取收集的chunks
        let chunks = collected_chunks.lock().await.clone();
        drop(collected_chunks);  // 立即释放Arc引用
        
        // 一次性解析（避免重复解析，提升性能）
        let full_body: Vec<u8> = chunks.iter().flat_map(|c| c.iter()).copied().collect();
        let chunk_count = chunks.len();
        
        tracing::info!(
            "[{}] Processing stream log: {} chunks, {} bytes",
            cli_type, chunk_count, full_body.len()
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
            cli_type, usage.input_tokens, usage.output_tokens
        );
        
        // Update log info with response body
        let content_encoding = log_resp_headers.get("content-encoding")
            .and_then(|v| v.to_str().ok());
        let decompressed_body = maybe_decompress(&full_body, content_encoding);
        let mut final_log_info = log_info;
        final_log_info.provider_body = Some(truncate_body(&decompressed_body));
        final_log_info.response_body = final_log_info.provider_body.clone();
        
        // Record stats
        let elapsed = start_time.elapsed().as_millis() as i64;
        if log_is_success {
            if let Ok(had_failures) = provider_service::record_success(&log_state.db, log_provider_id).await {
                if had_failures {
                    let _ = stats_service::record_system_log(
                        &log_state.log_db,
                        "info",
                        "provider_recovered",
                        &format!("Provider {} recovered successfully", log_provider_name),
                        Some(&log_provider_name),
                        None,
                    ).await;
                }
            }
        } else if let Ok((was_blacklisted, prov_name)) = provider_service::record_failure(&log_state.db, log_provider_id).await {
            if was_blacklisted {
                let _ = stats_service::record_system_log(
                    &log_state.log_db,
                    "warn",
                    "provider_blacklisted",
                    &format!("Provider {} blacklisted due to consecutive failures", prov_name),
                    Some(&prov_name),
                    final_log_info.error_message.as_deref(),
                ).await;
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
            Some(final_log_info),
        ).await;
        
        tracing::info!("[{}] Delayed log recording completed", cli_type);
    });

    Ok(builder
        .body(Body::from_stream(stream))
        .unwrap())
}

async fn handle_non_streaming_request(
    request_builder: reqwest::RequestBuilder,
    state: &Arc<AppState>,
    provider_id: i64,
    provider_name: &str,
    cli_type: CliType,
    model_id: Option<&str>,
    client_method: &str,
    client_path: &str,
    start_time: Instant,
    timeouts: TimeoutConfig,
    mut log_info: RequestLogInfo,
) -> Result<Response<Body>, StatusCode> {
    // Send request with timeout
    let response = match tokio::time::timeout(
        timeouts.non_stream_timeout,
        request_builder.send(),
    )
    .await
    {
        Ok(Ok(resp)) => resp,
        Ok(Err(e)) => {
            tracing::error!(error = %e, "Upstream request failed");
            if let Ok((was_blacklisted, prov_name)) = provider_service::record_failure(&state.db, provider_id).await {
                if was_blacklisted {
                    let _ = stats_service::record_system_log(
                        &state.log_db,
                        "warn",
                        "provider_blacklisted",
                        &format!("Provider {} blacklisted due to consecutive failures", prov_name),
                        Some(&prov_name),
                        Some(&format!("{{\"error\": \"{}\"}}", e)),
                    ).await;
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
                Some(log_info),
            )
            .await;
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"error": "Upstream error: {}"}}"#, e)))
                .unwrap());
        }
        Err(_) => {
            tracing::error!("Request timeout");
            if let Ok((was_blacklisted, prov_name)) = provider_service::record_failure(&state.db, provider_id).await {
                if was_blacklisted {
                    let _ = stats_service::record_system_log(
                        &state.log_db,
                        "warn",
                        "provider_blacklisted",
                        &format!("Provider {} blacklisted due to consecutive failures", prov_name),
                        Some(&prov_name),
                        Some("{\"error\": \"Request timeout\"}"),
                    ).await;
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
    log_info.response_headers = Some(serialize_reqwest_headers(&resp_headers));

    // Read response body
    let body_bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!(error = %e, "Failed to read response body");
            if let Ok((was_blacklisted, prov_name)) = provider_service::record_failure(&state.db, provider_id).await {
                if was_blacklisted {
                    let _ = stats_service::record_system_log(
                        &state.log_db,
                        "warn",
                        "provider_blacklisted",
                        &format!("Provider {} blacklisted due to consecutive failures", prov_name),
                        Some(&prov_name),
                        Some(&format!("{{\"error\": \"{}\"}}", e)),
                    ).await;
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
                Some(log_info),
            )
            .await;
            return Err(StatusCode::BAD_GATEWAY);
        }
    };

    // Decompress if needed for logging and token parsing
    let content_encoding = resp_headers.get("content-encoding")
        .and_then(|v| v.to_str().ok());
    let decompressed_body = maybe_decompress(&body_bytes, content_encoding);

    // Store response body for logging (use decompressed version)
    log_info.provider_body = Some(truncate_body(&decompressed_body));
    log_info.response_body = log_info.provider_body.clone();

    // Parse token usage (use decompressed body)
    let mut usage = TokenUsage::default();
    parse_token_usage(&decompressed_body, cli_type, &mut usage);

    // Record success/failure
    if is_success {
        if let Ok(had_failures) = provider_service::record_success(&state.db, provider_id).await {
            if had_failures {
                let _ = stats_service::record_system_log(
                    &state.log_db,
                    "info",
                    "provider_recovered",
                    &format!("Provider {} recovered successfully", provider_name),
                    Some(provider_name),
                    None,
                ).await;
            }
        }
    } else if let Ok((was_blacklisted, prov_name)) = provider_service::record_failure(&state.db, provider_id).await {
        if was_blacklisted {
            let _ = stats_service::record_system_log(
                &state.log_db,
                "warn",
                "provider_blacklisted",
                &format!("Provider {} blacklisted due to consecutive failures", prov_name),
                Some(&prov_name),
                log_info.error_message.as_deref(),
            ).await;
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
        Some(log_info),
    )
    .await;

    // Build response
    let mut builder = Response::builder()
        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK));

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
    log_info: Option<RequestLogInfo>,
) {
    // Derive success from status_code (200-299 = success)
    let success = status_code.map(|code| (200..300).contains(&code)).unwrap_or(false);

    // Record to request_logs
    let _ = stats_service::record_request_log(
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
        log_info,
    )
    .await;

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

// Providers
pub async fn list_providers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ProviderQuery>,
) -> Result<Json<Vec<ProviderResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let providers = if let Some(ct) = query.cli_type {
        sqlx::query_as::<_, Provider>(
            "SELECT * FROM providers WHERE cli_type = ? ORDER BY sort_order, id",
        )
        .bind(&ct)
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as::<_, Provider>("SELECT * FROM providers ORDER BY sort_order, id")
            .fetch_all(&state.db)
            .await
    };

    providers
        .map(|ps| Json(ps.into_iter().map(ProviderResponse::from).collect()))
        .map_err(db_error)
}

pub async fn get_provider_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ProviderResponse>, (StatusCode, Json<ErrorResponse>)> {
    sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(db_error)?
        .map(ProviderResponse::from)
        .map(Json)
        .ok_or_else(|| error_response("Provider not found"))
}

pub async fn create_provider_handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<ProviderCreate>,
) -> Result<Json<ProviderResponse>, (StatusCode, Json<ErrorResponse>)> {
    let now = chrono::Utc::now().timestamp();
    let cli_type = input.cli_type.unwrap_or_else(|| "claude_code".to_string());

    let result = sqlx::query(
        r#"
        INSERT INTO providers (cli_type, name, base_url, api_key, enabled, failure_threshold, blacklist_minutes, consecutive_failures, sort_order, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, 0, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM providers), ?, ?)
        "#,
    )
    .bind(&cli_type)
    .bind(&input.name)
    .bind(&input.base_url)
    .bind(&input.api_key)
    .bind(input.enabled.unwrap_or(true) as i64)
    .bind(input.failure_threshold.unwrap_or(3))
    .bind(input.blacklist_minutes.unwrap_or(10))
    .bind(now)
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(db_error)?;

    let id = result.last_insert_rowid();
    get_provider_handler(State(state), Path(id)).await
}

pub async fn update_provider_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(input): Json<ProviderUpdate>,
) -> Result<Json<ProviderResponse>, (StatusCode, Json<ErrorResponse>)> {
    let now = chrono::Utc::now().timestamp();
    let mut updates = vec!["updated_at = ?".to_string()];
    let mut has_updates = false;

    if input.name.is_some() {
        updates.push("name = ?".to_string());
        has_updates = true;
    }
    if input.base_url.is_some() {
        updates.push("base_url = ?".to_string());
        has_updates = true;
    }
    if input.api_key.is_some() {
        updates.push("api_key = ?".to_string());
        has_updates = true;
    }
    if input.enabled.is_some() {
        updates.push("enabled = ?".to_string());
        has_updates = true;
    }
    if input.failure_threshold.is_some() {
        updates.push("failure_threshold = ?".to_string());
        has_updates = true;
    }
    if input.blacklist_minutes.is_some() {
        updates.push("blacklist_minutes = ?".to_string());
        has_updates = true;
    }

    if !has_updates {
        return get_provider_handler(State(state), Path(id)).await;
    }

    let query = format!("UPDATE providers SET {} WHERE id = ?", updates.join(", "));
    let mut q = sqlx::query(&query).bind(now);

    if let Some(ref name) = input.name {
        q = q.bind(name);
    }
    if let Some(ref base_url) = input.base_url {
        q = q.bind(base_url);
    }
    if let Some(ref api_key) = input.api_key {
        q = q.bind(api_key);
    }
    if let Some(enabled) = input.enabled {
        q = q.bind(enabled as i64);
    }
    if let Some(failure_threshold) = input.failure_threshold {
        q = q.bind(failure_threshold);
    }
    if let Some(blacklist_minutes) = input.blacklist_minutes {
        q = q.bind(blacklist_minutes);
    }

    q.bind(id)
        .execute(&state.db)
        .await
        .map_err(db_error)?;

    get_provider_handler(State(state), Path(id)).await
}

pub async fn delete_provider_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    sqlx::query("DELETE FROM providers WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(db_error)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn reorder_providers_handler(
    State(state): State<Arc<AppState>>,
    Json(ids): Json<Vec<i64>>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    for (idx, id) in ids.iter().enumerate() {
        sqlx::query("UPDATE providers SET sort_order = ? WHERE id = ?")
            .bind(idx as i64)
            .bind(id)
            .execute(&state.db)
            .await
            .map_err(db_error)?;
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn reset_provider_failures_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    sqlx::query("UPDATE providers SET consecutive_failures = 0, blacklisted_until = NULL WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(db_error)?;
    Ok(StatusCode::NO_CONTENT)
}

// Settings
#[derive(Debug, Deserialize)]
pub struct GatewaySettingsUpdate {
    pub debug_log: bool,
}

#[derive(Debug, Serialize)]
pub struct GatewaySettingsResponse {
    pub debug_log: bool,
}

pub async fn get_gateway_settings(
    State(state): State<Arc<AppState>>,
) -> Result<Json<GatewaySettingsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let settings = sqlx::query_as::<_, GatewaySettings>("SELECT debug_log FROM gateway_settings WHERE id = 1")
        .fetch_one(&state.db)
        .await
        .map_err(db_error)?;

    Ok(Json(GatewaySettingsResponse {
        debug_log: settings.debug_log != 0,
    }))
}

pub async fn update_gateway_settings_handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<GatewaySettingsUpdate>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query("UPDATE gateway_settings SET debug_log = ?, updated_at = ? WHERE id = 1")
        .bind(input.debug_log as i64)
        .bind(now)
        .execute(&state.db)
        .await
        .map_err(db_error)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_timeout_settings(
    State(state): State<Arc<AppState>>,
) -> Result<Json<TimeoutSettings>, (StatusCode, Json<ErrorResponse>)> {
    sqlx::query_as::<_, TimeoutSettings>(
        "SELECT stream_first_byte_timeout, stream_idle_timeout, non_stream_timeout FROM timeout_settings WHERE id = 1",
    )
    .fetch_one(&state.db)
    .await
    .map(Json)
    .map_err(db_error)
}

pub async fn update_timeout_settings_handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<TimeoutSettingsUpdate>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let now = chrono::Utc::now().timestamp();
    let current = get_timeout_settings(State(state.clone())).await?;

    sqlx::query(
        "UPDATE timeout_settings SET stream_first_byte_timeout = ?, stream_idle_timeout = ?, non_stream_timeout = ?, updated_at = ? WHERE id = 1",
    )
    .bind(input.stream_first_byte_timeout.unwrap_or(current.stream_first_byte_timeout))
    .bind(input.stream_idle_timeout.unwrap_or(current.stream_idle_timeout))
    .bind(input.non_stream_timeout.unwrap_or(current.non_stream_timeout))
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(db_error)?;
    Ok(StatusCode::NO_CONTENT)
}

// Logs
#[derive(Debug, Deserialize)]
pub struct LogQuery {
    #[serde(default = "default_page")]
    page: i64,
    #[serde(default = "default_page_size")]
    page_size: i64,
    cli_type: Option<String>,
}

pub async fn get_request_logs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LogQuery>,
) -> Result<Json<PaginatedLogs>, (StatusCode, Json<ErrorResponse>)> {
    let page = query.page.max(1);
    let page_size = query.page_size.clamp(1, 100);
    let offset = (page - 1) * page_size;
    let pool = &state.log_db;

    let (items, total) = if let Some(ct) = query.cli_type {
        let items = sqlx::query_as::<_, RequestLogItem>(
            "SELECT id, created_at, cli_type, provider_name, model_id, status_code, elapsed_ms, input_tokens, output_tokens, client_method, client_path FROM request_logs WHERE cli_type = ? ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(&ct)
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(db_error)?;

        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM request_logs WHERE cli_type = ?")
            .bind(&ct)
            .fetch_one(pool)
            .await
            .map_err(db_error)?;

        (items, total.0)
    } else {
        let items = sqlx::query_as::<_, RequestLogItem>(
            "SELECT id, created_at, cli_type, provider_name, model_id, status_code, elapsed_ms, input_tokens, output_tokens, client_method, client_path FROM request_logs ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(db_error)?;

        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM request_logs")
            .fetch_one(pool)
            .await
            .map_err(db_error)?;

        (items, total.0)
    };

    Ok(Json(PaginatedLogs {
        items,
        total,
        page,
        page_size,
    }))
}

pub async fn clear_request_logs(
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    sqlx::query("DELETE FROM request_logs")
        .execute(&state.log_db)
        .await
        .map_err(db_error)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_request_log_detail(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<RequestLogDetail>, (StatusCode, Json<ErrorResponse>)> {
    sqlx::query_as::<_, RequestLogDetail>(
        "SELECT id, created_at, cli_type, provider_name, model_id, status_code, elapsed_ms, input_tokens, output_tokens, client_method, client_path, client_headers, client_body, forward_url, forward_headers, forward_body, provider_headers, provider_body, response_headers, response_body, error_message FROM request_logs WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.log_db)
    .await
    .map_err(db_error)?
    .map(Json)
    .ok_or_else(|| error_response("Log not found"))
}

// System logs
#[derive(Debug, Deserialize)]
pub struct SystemLogQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
    pub level: Option<String>,
    pub event_type: Option<String>,
    pub provider_name: Option<String>,
}

pub async fn get_system_logs_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SystemLogQuery>,
) -> Result<Json<SystemLogListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let page = query.page.max(1);
    let page_size = query.page_size.clamp(1, 100);
    let offset = (page - 1) * page_size;
    let pool = &state.log_db;

    // Build query
    let mut sql = "SELECT * FROM system_logs WHERE 1=1".to_string();
    let mut count_sql = "SELECT COUNT(*) FROM system_logs WHERE 1=1".to_string();

    if query.level.is_some() {
        sql.push_str(" AND level = ?");
        count_sql.push_str(" AND level = ?");
    }
    if query.event_type.is_some() {
        sql.push_str(" AND event_type = ?");
        count_sql.push_str(" AND event_type = ?");
    }
    if query.provider_name.is_some() {
        sql.push_str(" AND provider_name = ?");
        count_sql.push_str(" AND provider_name = ?");
    }

    sql.push_str(" ORDER BY id DESC LIMIT ? OFFSET ?");
    let mut q = sqlx::query_as::<_, SystemLogItem>(&sql)
        .bind(page_size)
        .bind(offset);

    if let Some(ref lvl) = query.level {
        q = q.bind(lvl);
    }
    if let Some(ref et) = query.event_type {
        q = q.bind(et);
    }
    if let Some(ref pn) = query.provider_name {
        q = q.bind(pn);
    }

    let items = q.fetch_all(pool).await.map_err(db_error)?;

    // Get total count
    let mut count_q = sqlx::query_as::<_, (i64,)>(&count_sql);
    if let Some(ref lvl) = query.level {
        count_q = count_q.bind(lvl);
    }
    if let Some(ref et) = query.event_type {
        count_q = count_q.bind(et);
    }
    if let Some(ref pn) = query.provider_name {
        count_q = count_q.bind(pn);
    }
    let (total,) = count_q.fetch_one(pool).await.map_err(db_error)?;

    Ok(Json(SystemLogListResponse {
        items,
        total,
        page,
        page_size,
    }))
}

pub async fn clear_system_logs_handler(
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    sqlx::query("DELETE FROM system_logs")
        .execute(&state.log_db)
        .await
        .map_err(db_error)?;
    Ok(StatusCode::NO_CONTENT)
}

// Stats
#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub cli_type: Option<String>,
}

pub async fn get_daily_stats(
    State(state): State<Arc<AppState>>,
    Query(query): Query<StatsQuery>,
) -> Result<Json<Vec<DailyStats>>, (StatusCode, Json<ErrorResponse>)> {
    let pool = &state.log_db;

    let mut sql = "SELECT * FROM usage_daily WHERE 1=1".to_string();
    if query.start_date.is_some() {
        sql.push_str(" AND usage_date >= ?");
    }
    if query.end_date.is_some() {
        sql.push_str(" AND usage_date <= ?");
    }
    if query.cli_type.is_some() {
        sql.push_str(" AND cli_type = ?");
    }
    sql.push_str(" ORDER BY usage_date DESC");

    let mut q = sqlx::query_as::<_, DailyStats>(&sql);
    if let Some(ref sd) = query.start_date {
        q = q.bind(sd);
    }
    if let Some(ref ed) = query.end_date {
        q = q.bind(ed);
    }
    if let Some(ref ct) = query.cli_type {
        q = q.bind(ct);
    }

    q.fetch_all(pool)
        .await
        .map(Json)
        .map_err(db_error)
}

pub async fn get_system_status_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<SystemStatus>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(SystemStatus {
        status: "running".to_string(),
        port: 7788,
        uptime: 0,
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

// Get all settings (for dashboard)
#[derive(Debug, Serialize)]
pub struct AllSettingsResponse {
    pub gateway: GatewaySettingsResponse,
    pub timeouts: TimeoutSettings,
    pub cli_settings: std::collections::HashMap<String, crate::db::models::CliSettingsResponse>,
}

pub async fn get_all_settings(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AllSettingsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Get gateway settings
    let gateway_settings = sqlx::query_as::<_, GatewaySettings>("SELECT debug_log FROM gateway_settings WHERE id = 1")
        .fetch_one(&state.db)
        .await
        .map_err(db_error)?;

    // Get timeout settings
    let timeout_settings = sqlx::query_as::<_, TimeoutSettings>("SELECT stream_first_byte_timeout, stream_idle_timeout, non_stream_timeout FROM timeout_settings WHERE id = 1")
        .fetch_one(&state.db)
        .await
        .map_err(db_error)?;

    // Get CLI settings
    let mut cli_settings = std::collections::HashMap::new();
    for cli_type in &["claude_code", "codex", "gemini"] {
        cli_settings.insert(
            cli_type.to_string(),
            crate::db::models::CliSettingsResponse {
                cli_type: cli_type.to_string(),
                enabled: false, // TODO: Check if config file exists
                default_json_config: String::new(),
            },
        );
    }

    Ok(Json(AllSettingsResponse {
        gateway: GatewaySettingsResponse {
            debug_log: gateway_settings.debug_log != 0,
        },
        timeouts: timeout_settings,
        cli_settings,
    }))
}

// Get provider stats
#[derive(Debug, Serialize)]
pub struct ProviderStatsResponse {
    pub provider_name: String,
    pub cli_type: String,
    pub total_requests: i64,
    pub total_success: i64,
    pub total_failure: i64,
    pub success_rate: f64,
    pub total_tokens: i64,
}

pub async fn get_provider_stats(
    State(state): State<Arc<AppState>>,
    Query(query): Query<StatsQuery>,
) -> Result<Json<Vec<ProviderStatsResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let pool = &state.log_db;

    let mut sql = r#"
        SELECT
            provider_name,
            cli_type,
            COUNT(*) as total_requests,
            SUM(CASE WHEN status_code >= 200 AND status_code < 300 THEN 1 ELSE 0 END) as total_success,
            SUM(CASE WHEN status_code IS NULL OR status_code < 200 OR status_code >= 300 THEN 1 ELSE 0 END) as total_failure,
            SUM(input_tokens + output_tokens) as total_tokens
        FROM request_logs
        WHERE 1=1
    "#.to_string();

    if query.start_date.is_some() {
        sql.push_str(" AND DATE(created_at, 'unixepoch') >= ?");
    }
    if query.end_date.is_some() {
        sql.push_str(" AND DATE(created_at, 'unixepoch') <= ?");
    }
    if query.cli_type.is_some() {
        sql.push_str(" AND cli_type = ?");
    }

    sql.push_str(" GROUP BY provider_name, cli_type ORDER BY total_requests DESC");

    let mut q = sqlx::query_as::<_, (String, String, i64, i64, i64, i64)>(&sql);
    if let Some(ref sd) = query.start_date {
        q = q.bind(sd);
    }
    if let Some(ref ed) = query.end_date {
        q = q.bind(ed);
    }
    if let Some(ref ct) = query.cli_type {
        q = q.bind(ct);
    }

    let results = q.fetch_all(pool).await.map_err(db_error)?;

    let stats = results
        .into_iter()
        .map(|(provider_name, cli_type, total_requests, total_success, total_failure, total_tokens)| {
            let success_rate = if total_requests > 0 {
                (total_success as f64 / total_requests as f64) * 100.0
            } else {
                0.0
            };

            ProviderStatsResponse {
                provider_name,
                cli_type,
                total_requests,
                total_success,
                total_failure,
                success_rate,
                total_tokens,
            }
        })
        .collect();

    Ok(Json(stats))
}

// MCP, Prompts, Sessions, Backup - placeholder implementations
// For brevity, returning empty responses for now

pub async fn list_mcps(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(vec![]))
}

pub async fn get_mcp_handler(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(serde_json::json!({})))
}

pub async fn create_mcp_handler(
    State(_state): State<Arc<AppState>>,
    Json(_input): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(serde_json::json!({})))
}

pub async fn update_mcp_handler(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<i64>,
    Json(_input): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(serde_json::json!({})))
}

pub async fn delete_mcp_handler(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_prompts(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(vec![]))
}

pub async fn get_prompt_handler(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(serde_json::json!({})))
}

pub async fn create_prompt_handler(
    State(_state): State<Arc<AppState>>,
    Json(_input): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(serde_json::json!({})))
}

pub async fn update_prompt_handler(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<i64>,
    Json(_input): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(serde_json::json!({})))
}

pub async fn delete_prompt_handler(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_projects(
    Query(_query): Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(serde_json::json!({ "items": [], "total": 0, "page": 1, "page_size": 20 })))
}

pub async fn delete_project_handler(
    Query(_query): Query<serde_json::Value>,
    Path(_name): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_sessions(
    Query(_query): Query<serde_json::Value>,
    Path(_name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(serde_json::json!({ "items": [], "total": 0, "page": 1, "page_size": 20 })))
}

pub async fn delete_session_handler(
    Query(_query): Query<serde_json::Value>,
    Path((_name, _id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_session_messages_handler(
    Query(_query): Query<serde_json::Value>,
    Path((_name, _id)): Path<(String, String)>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(vec![]))
}

pub async fn get_webdav_settings_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(serde_json::json!({ "url": "", "username": "", "password": "" })))
}

pub async fn update_webdav_settings_handler(
    State(_state): State<Arc<AppState>>,
    Json(_input): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(serde_json::json!({ "url": "", "username": "", "password": "" })))
}

pub async fn test_webdav_connection_handler(
    Json(_input): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(serde_json::json!({ "success": false })))
}

pub async fn export_to_local_handler() -> Result<Response<Body>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/octet-stream")
        .body(Body::empty())
        .unwrap())
}

pub async fn import_from_local_handler(
    _bytes: axum::body::Bytes,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(serde_json::json!({ "success": true, "message": "Not implemented" })))
}

pub async fn export_to_webdav_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(serde_json::json!({ "success": false, "message": "Not implemented" })))
}

pub async fn list_webdav_backups_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(serde_json::json!({ "backups": [] })))
}

pub async fn import_from_webdav_handler(
    State(_state): State<Arc<AppState>>,
    Query(_query): Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(serde_json::json!({ "success": true, "message": "Not implemented" })))
}
