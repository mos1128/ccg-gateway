use axum::{
    body::Body,
    extract::State,
    http::{Response, StatusCode},
};
use bytes::Bytes;
use flate2::read::{DeflateDecoder, GzDecoder, ZlibDecoder};
use futures_util::StreamExt;
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::Emitter;
use tokio::sync::{mpsc, Mutex};

use super::AppState;
use crate::db::models::{Protocol, RequestLogInfo, RequestLogItem};
use crate::services::proxy::{
    apply_body_model_mapping, apply_url_model_mapping, detect_gateway_profile,
    extract_model_from_body, extract_model_from_path, is_streaming, parse_streaming_token_usage,
    parse_token_usage, TimeoutConfig, TokenUsage,
};
use crate::services::routing::select_provider;
use crate::services::{
    agent as agent_service, protocol as protocol_service, provider as provider_service,
    stats as stats_service,
};

const RESPONSE_FILTERED_HEADERS: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "content-length",
];
const CLIENT_CLOSED_REQUEST_STATUS: u16 = 499;
const CLIENT_CLOSED_REQUEST_MESSAGE: &str = "Client closed request before completion";

#[derive(Clone)]
struct RequestIdentity {
    agent_id: String,
    profile: String,
    protocol: Protocol,
    provider_id: i64,
    token_usage_enabled: bool,
}

struct RequestLogCancelGuard {
    state: Arc<AppState>,
    log_id: i64,
    start_time: Instant,
    first_byte_ms: Arc<AtomicI64>,
    completed: AtomicBool,
}

impl RequestLogCancelGuard {
    fn new(state: &Arc<AppState>, log_id: Option<i64>, start_time: Instant) -> Option<Self> {
        log_id.map(|log_id| Self {
            state: state.clone(),
            log_id,
            start_time,
            first_byte_ms: Arc::new(AtomicI64::new(0)),
            completed: AtomicBool::new(false),
        })
    }

    fn set_first_byte_ms(&self, first_byte_ms: i64) {
        self.first_byte_ms.store(first_byte_ms, Ordering::Relaxed);
    }

    fn disarm(&self) {
        self.completed.store(true, Ordering::Relaxed);
    }
}

impl Drop for RequestLogCancelGuard {
    fn drop(&mut self) {
        if self.completed.load(Ordering::Relaxed) {
            return;
        }

        let state = self.state.clone();
        let log_id = self.log_id;
        let elapsed_ms = self.start_time.elapsed().as_millis() as i64;
        let recorded_first_byte_ms = self.first_byte_ms.load(Ordering::Relaxed);
        let first_byte_ms = if recorded_first_byte_ms > 0 {
            recorded_first_byte_ms
        } else {
            elapsed_ms
        };

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                match stats_service::cancel_request_log(
                    &state.log_db,
                    log_id,
                    CLIENT_CLOSED_REQUEST_STATUS,
                    elapsed_ms,
                    first_byte_ms,
                    CLIENT_CLOSED_REQUEST_MESSAGE,
                )
                .await
                {
                    Ok(true) => emit_request_log_event(&state, "request-log-updated", log_id).await,
                    Ok(false) => {}
                    Err(e) => tracing::error!(error = %e, "Failed to cancel request log"),
                }
            });
        }
    }
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
    let raw_full_path = if let Some(query) = uri.query() {
        format!("{}?{}", uri.path(), query)
    } else {
        uri.path().to_string()
    };

    let full_path = raw_full_path.clone();

    let user_agent = headers
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let agent_match = match agent_service::match_user_agent(&state.db, &user_agent).await {
        Ok(Some(agent_match)) => agent_match,
        Ok(None) => {
            let payload = serde_json::json!({
                "type": "unknown_agent",
                "user_agent": user_agent,
            });
            let key = user_agent.to_lowercase();
            let _ =
                agent_service::record_diagnostic(&state.log_db, "unknown_agent", &key, &payload)
                    .await;
            let _ = stats_service::record_system_log_dedup(
                &state.log_db,
                "unknown_agent",
                &payload.to_string(),
                600,
            )
            .await;
            return Ok(json_error_response(
                StatusCode::BAD_REQUEST,
                "unknown_agent",
                "User-Agent does not match any built-in Agent",
            ));
        }
        Err(error) => {
            tracing::error!(error = %error, "Failed to load Agent definitions");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    let agent = agent_match.selected;
    if agent_match.matched_agents.len() > 1 {
        let payload = serde_json::json!({
            "type": "config_conflict",
            "user_agent": user_agent,
            "matched_agents": agent_match.matched_agents,
            "selected_agent": agent.id,
        });
        let key = format!(
            "{}|{}|{}",
            user_agent.to_lowercase(),
            payload["matched_agents"],
            agent.id
        );
        let _ = agent_service::record_diagnostic(&state.log_db, "config_conflict", &key, &payload)
            .await;
        let _ = stats_service::record_system_log_dedup(
            &state.log_db,
            "config_conflict",
            &payload.to_string(),
            600,
        )
        .await;
    }
    let requested_profile = detect_gateway_profile(&headers);
    let profiles_enabled = agent.features.profiles.enabled;
    let provider_profile = if profiles_enabled {
        requested_profile
    } else {
        crate::services::routing::DEFAULT_PROFILE.to_string()
    };

    let protocol_match = match protocol_service::detect_protocol(&agent, &method, &full_path) {
        Some(protocol_match) => protocol_match,
        None => {
            let payload = serde_json::json!({
                "type": "protocol_not_matched",
                "agent_id": agent.id,
                "method": method.as_str(),
                "path": full_path,
            });
            let key = format!("{}|{}|{}", agent.id, method, full_path);
            let _ = agent_service::record_diagnostic(
                &state.log_db,
                "protocol_not_matched",
                &key,
                &payload,
            )
            .await;
            let _ = stats_service::record_system_log_dedup(
                &state.log_db,
                "protocol_not_matched",
                &payload.to_string(),
                600,
            )
            .await;
            return Ok(json_error_response(
                StatusCode::NOT_FOUND,
                "protocol_not_matched",
                "Request path does not match any protocol declared by this Agent",
            ));
        }
    };
    let protocol = protocol_match.selected;
    if protocol_match.matched_protocols.len() > 1 {
        let matched: Vec<_> = protocol_match
            .matched_protocols
            .iter()
            .map(|protocol| protocol.as_str())
            .collect();
        let payload = serde_json::json!({
            "type": "protocol_conflict",
            "agent_id": agent.id,
            "path": full_path,
            "matched_protocols": matched,
            "selected_protocol": protocol.as_str(),
        });
        let key = format!(
            "{}|{}|{}",
            agent.id, full_path, payload["matched_protocols"]
        );
        let _ =
            agent_service::record_diagnostic(&state.log_db, "protocol_conflict", &key, &payload)
                .await;
        let _ = stats_service::record_system_log_dedup(
            &state.log_db,
            "protocol_conflict",
            &payload.to_string(),
            600,
        )
        .await;
    }

    // Serialize client headers for logging
    let client_headers_json = serialize_headers(&headers);

    // Read request body
    let body_bytes = match axum::body::to_bytes(req.into_body(), 20 * 1024 * 1024).await {
        Ok(bytes) => bytes.to_vec(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to read request body");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Store client body for logging
    let client_body_str = truncate_body(&body_bytes);

    // Check if streaming
    let streaming = is_streaming(&body_bytes, &full_path, protocol);

    // Only learn from streaming requests since our test is streaming
    if streaming {
        crate::services::proxy::update_captured_headers(&agent.id, protocol, &headers);
    }

    // Extract model name before selecting provider (for blacklist filtering)
    let extracted_model = match protocol {
        Protocol::GeminiGenerateContent => extract_model_from_path(&full_path),
        _ => extract_model_from_body(&body_bytes),
    };

    // Select provider based on CLI type and model
    let provider_with_maps = match select_provider(
        &state.db,
        &agent.id,
        &provider_profile,
        protocol,
        extracted_model.as_deref(),
    )
    .await
    {
        Ok(Some(p)) => p,
        Ok(None) => {
            tracing::warn!(
                agent_id = agent.id,
                protocol = %protocol,
                profile = provider_profile,
                "No available provider"
            );
            // Log system event
            let _ = stats_service::record_system_log(
                &state.log_db,
                "no_provider_available",
                &format!(
                    "Agent {} / profile {} / protocol {} 没有可用的服务商",
                    agent.id, provider_profile, protocol
                ),
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
    let model_mapping_enabled = agent.features.model_mapping.enabled;
    let (final_body, final_path, source_model, target_model) = match protocol {
        Protocol::GeminiGenerateContent if model_mapping_enabled => {
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
        Protocol::GeminiGenerateContent => (
            body_bytes.clone(),
            full_path.clone(),
            extract_model_from_path(&full_path),
            None,
        ),
        _ if model_mapping_enabled => {
            let mapping = apply_body_model_mapping(&provider_with_maps, &body_bytes, &full_path);
            (
                mapping.body,
                mapping.path,
                mapping.source_model,
                mapping.target_model,
            )
        }
        _ => (
            body_bytes.clone(),
            full_path.clone(),
            extract_model_from_body(&body_bytes),
            None,
        ),
    };

    // Use target model if mapped, otherwise use source model
    let model_id = target_model.clone().or(source_model.clone());

    // Build upstream URL: base_url + original_path
    // e.g., base_url="https://api.example.com/v1", path="/responses" -> "https://api.example.com/v1/responses"
    let upstream_url = crate::services::proxy::join_upstream_url(&provider.base_url, &final_path);

    // Build the upstream request via the shared constructor (single source of
    // truth for hop-by-hop filtering, auth injection, and UA override).
    let request = match crate::services::proxy::build_upstream_request(
        &state.http_client,
        provider,
        protocol,
        &upstream_url,
        &headers,
        final_body,
        reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap_or(reqwest::Method::GET),
    ) {
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

    let identity = RequestIdentity {
        agent_id: agent.id.clone(),
        profile: provider_profile.clone(),
        protocol,
        provider_id,
        token_usage_enabled: agent.features.token_usage.enabled,
    };

    let request_log_id = start_request_log(
        &state,
        &identity,
        &provider_name,
        model_id.as_deref(),
        method.as_ref(),
        &raw_full_path,
        Some(&upstream_url),
        source_model.as_deref(),
        target_model.as_deref(),
    )
    .await;

    // Execute request
    if streaming {
        handle_streaming_request(
            request,
            &state.http_client,
            &state,
            provider_id,
            &provider_name,
            identity,
            model_id.as_deref(),
            method.as_ref(),
            &raw_full_path,
            start_time,
            timeouts,
            source_model.as_deref(),
            target_model.as_deref(),
            log_info,
            request_log_id,
        )
        .await
    } else {
        handle_non_streaming_request(
            request,
            &state.http_client,
            &state,
            provider_id,
            &provider_name,
            identity,
            model_id.as_deref(),
            method.as_ref(),
            &raw_full_path,
            start_time,
            timeouts,
            source_model.as_deref(),
            target_model.as_deref(),
            log_info,
            request_log_id,
        )
        .await
    }
}

fn json_error_response(status: StatusCode, error_type: &str, message: &str) -> Response<Body> {
    let body = serde_json::json!({
        "error": {
            "type": error_type,
            "message": message,
        }
    });
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("valid JSON error response")
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

fn copy_response_headers(
    mut builder: axum::http::response::Builder,
    headers: &reqwest::header::HeaderMap,
) -> axum::http::response::Builder {
    for (name, value) in headers.iter() {
        if RESPONSE_FILTERED_HEADERS
            .iter()
            .any(|h| name.as_str().eq_ignore_ascii_case(h))
        {
            continue;
        }

        if let Ok(header_name) = axum::http::HeaderName::from_bytes(name.as_str().as_bytes()) {
            if let Ok(header_value) = axum::http::HeaderValue::from_bytes(value.as_bytes()) {
                builder = builder.header(header_name, header_value);
            }
        }
    }

    builder
}

/// Maximum body size to store in logs (2MB)
const MAX_LOG_BODY_SIZE: usize = 2 * 1024 * 1024;

fn truncate_body(body: &[u8]) -> String {
    if body.len() <= MAX_LOG_BODY_SIZE {
        String::from_utf8_lossy(body).into_owned()
    } else {
        let mut s = String::from_utf8_lossy(&body[..MAX_LOG_BODY_SIZE]).into_owned();
        s.push_str("\n\n[body truncated at 2MB]");
        s
    }
}

fn maybe_decompress(body: &[u8], content_encoding: Option<&str>) -> Vec<u8> {
    try_decompress(body, content_encoding).unwrap_or_else(|| body.to_vec())
}

fn try_decompress(body: &[u8], content_encoding: Option<&str>) -> Option<Vec<u8>> {
    let Some(content_encoding) = content_encoding else {
        return Some(body.to_vec());
    };

    let encodings: Vec<String> = content_encoding
        .split(',')
        .map(|encoding| encoding.trim().to_lowercase())
        .filter(|encoding| !encoding.is_empty() && encoding != "identity")
        .collect();
    if encodings.is_empty() {
        return Some(body.to_vec());
    }

    let mut current = body.to_vec();
    for encoding in encodings.iter().rev() {
        current = decode_body(&current, encoding)?;
    }

    Some(current)
}

fn has_body_encoding(content_encoding: Option<&str>) -> bool {
    content_encoding
        .map(|value| {
            value.split(',').any(|encoding| {
                let encoding = encoding.trim();
                !encoding.is_empty() && !encoding.eq_ignore_ascii_case("identity")
            })
        })
        .unwrap_or(false)
}

fn decode_body(body: &[u8], encoding: &str) -> Option<Vec<u8>> {
    match encoding {
        "gzip" | "x-gzip" => read_all(GzDecoder::new(body)),
        "deflate" => {
            read_all(ZlibDecoder::new(body)).or_else(|| read_all(DeflateDecoder::new(body)))
        }
        "br" => read_all(brotli::Decompressor::new(body, 4096)),
        "zstd" | "zst" => zstd::decode_all(body).ok(),
        _ => None,
    }
}

fn read_all<R: Read>(mut reader: R) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    reader.read_to_end(&mut out).ok().map(|_| out)
}

fn parse_streaming_usage_chunk(
    buffer: &mut String,
    chunk: &[u8],
    protocol: Protocol,
    usage: &mut TokenUsage,
) {
    const MAX_SSE_LINE_BUFFER: usize = 1024 * 1024;

    buffer.push_str(&String::from_utf8_lossy(chunk));
    while let Some(pos) = buffer.find('\n') {
        let mut line: String = buffer.drain(..=pos).collect();
        if line.ends_with('\n') {
            line.pop();
        }
        if line.ends_with('\r') {
            line.pop();
        }
        parse_streaming_token_usage(&line, protocol, usage);
    }

    if buffer.len() > MAX_SSE_LINE_BUFFER {
        tracing::warn!(
            "[{}] SSE line exceeded {} bytes before newline; dropping buffered line, token usage in this event may be missed",
            protocol,
            MAX_SSE_LINE_BUFFER
        );
        buffer.clear();
    }
}

fn parse_streaming_usage_body(body: &[u8], protocol: Protocol) -> TokenUsage {
    let mut usage = TokenUsage::default();
    let mut buffer = String::new();
    parse_streaming_usage_chunk(&mut buffer, body, protocol, &mut usage);
    if !buffer.is_empty() {
        parse_streaming_token_usage(buffer.trim_end_matches('\r'), protocol, &mut usage);
    }
    usage
}

fn streaming_body_log_text(body: &[u8], limit: usize, truncated: bool) -> String {
    let log_len = body.len().min(limit);
    let mut body_str = String::from_utf8_lossy(&body[..log_len]).into_owned();
    if truncated || body.len() > limit {
        body_str.push_str("\n\n[response body truncated at 10MB]");
    }
    body_str
}

async fn handle_streaming_request(
    request: reqwest::Request,
    client: &reqwest::Client,
    state: &Arc<AppState>,
    provider_id: i64,
    provider_name: &str,
    identity: RequestIdentity,
    model_id: Option<&str>,
    client_method: &str,
    client_path: &str,
    start_time: Instant,
    timeouts: TimeoutConfig,
    source_model: Option<&str>,
    target_model: Option<&str>,
    mut log_info: RequestLogInfo,
    request_log_id: Option<i64>,
) -> Result<Response<Body>, StatusCode> {
    let protocol = identity.protocol;
    let cancel_guard = RequestLogCancelGuard::new(state, request_log_id, start_time);

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
                let elapsed = start_time.elapsed().as_millis() as i64;
                record_request_stats(
                    state,
                    &identity,
                    provider_name,
                    model_id,
                    None,
                    elapsed,
                    elapsed,
                    TokenUsage::default(),
                    client_method,
                    client_path,
                    source_model,
                    target_model,
                    Some(log_info),
                    request_log_id,
                )
                .await;
                if let Some(guard) = &cancel_guard {
                    guard.disarm();
                }
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
                let elapsed = start_time.elapsed().as_millis() as i64;
                record_request_stats(
                    state,
                    &identity,
                    provider_name,
                    model_id,
                    None,
                    elapsed,
                    elapsed,
                    TokenUsage::default(),
                    client_method,
                    client_path,
                    source_model,
                    target_model,
                    Some(log_info),
                    request_log_id,
                )
                .await;
                if let Some(guard) = &cancel_guard {
                    guard.disarm();
                }
                return Ok(Response::builder()
                    .status(StatusCode::GATEWAY_TIMEOUT)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"error": "First byte timeout"}"#))
                    .unwrap());
            }
        };

    let first_byte_fallback_ms = start_time.elapsed().as_millis() as i64;
    if let Some(guard) = &cancel_guard {
        guard.set_first_byte_ms(first_byte_fallback_ms);
    }
    let status = response.status();
    let resp_headers = response.headers().clone();

    // Store provider response info
    log_info.provider_headers = Some(serialize_reqwest_headers(&resp_headers));

    // Build response headers
    let mut builder =
        Response::builder().status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK));

    builder = copy_response_headers(builder, &resp_headers);

    // Create streaming body
    let is_success = status.is_success();

    // Collect body chunks for logging, capped at 10MB.
    const MAX_BODY_LOG: usize = 10 * 1024 * 1024;
    let collected_chunks = Arc::new(Mutex::new(Vec::<Bytes>::new()));
    let collected_chunks_for_stream = collected_chunks.clone();
    let stream_usage = Arc::new(Mutex::new(TokenUsage::default()));
    let stream_usage_for_stream = stream_usage.clone();
    let body_truncated_flag = Arc::new(Mutex::new(false));
    let body_truncated_flag_for_stream = body_truncated_flag.clone();
    let idle_timeout_flag = Arc::new(Mutex::new(false));
    let idle_timeout_flag_for_stream = idle_timeout_flag.clone();
    let first_chunk_ms = Arc::new(Mutex::new(None::<i64>));
    let first_chunk_ms_for_stream = first_chunk_ms.clone();
    let first_byte_log_state = state.clone();
    let first_byte_log_id = request_log_id;
    let response_encoded = has_body_encoding(
        resp_headers
            .get("content-encoding")
            .and_then(|v| v.to_str().ok()),
    );

    // 创建channel用于通知stream结束
    let (stream_end_tx, mut stream_end_rx) = mpsc::channel::<()>(1);

    let stream = async_stream::stream! {
        let mut byte_stream = response.bytes_stream();
        let idle_timeout = timeouts.idle_timeout;
        let mut chunk_count = 0usize;
        let mut total_bytes = 0usize;
        let mut collected_bytes = 0usize;
        let mut sse_buffer = String::new();

        loop {
            match tokio::time::timeout(idle_timeout, byte_stream.next()).await {
                Ok(Some(Ok(chunk))) => {
                    chunk_count += 1;
                    if chunk_count == 1 {
                        let first_byte_ms = start_time.elapsed().as_millis() as i64;
                        *first_chunk_ms_for_stream.lock().await = Some(first_byte_ms);
                        if let Some(log_id) = first_byte_log_id {
                            match stats_service::update_request_log_first_byte(
                                &first_byte_log_state.log_db,
                                log_id,
                                first_byte_ms,
                            )
                            .await
                            {
                                Ok(_) => emit_request_log_event(
                                    &first_byte_log_state,
                                    "request-log-updated",
                                    log_id,
                                )
                                .await,
                                Err(e) => {
                                    tracing::error!(error = %e, "Failed to update request first byte time");
                                }
                            }
                        }
                    }
                    let chunk_size = chunk.len();
                    total_bytes += chunk_size;

                    if !response_encoded {
                        let mut usage = stream_usage_for_stream.lock().await;
                        parse_streaming_usage_chunk(
                            &mut sse_buffer,
                            chunk.as_ref(),
                            protocol,
                            &mut usage,
                        );
                    }

                    // Collect chunk for body logging.
                    if collected_bytes < MAX_BODY_LOG {
                        let mut chunks = collected_chunks_for_stream.lock().await;
                        let to_collect = chunk.len().min(MAX_BODY_LOG - collected_bytes);
                        if to_collect > 0 {
                            chunks.push(chunk.slice(..to_collect));
                            collected_bytes += to_collect;
                        }
                        if to_collect < chunk.len() {
                            *body_truncated_flag_for_stream.lock().await = true;
                        }
                    } else {
                        *body_truncated_flag_for_stream.lock().await = true;
                    }

                    tracing::debug!(
                        "[{}] Chunk #{}: size={} bytes, total={} bytes",
                        protocol, chunk_count, chunk_size, total_bytes
                    );

                    yield Ok::<Bytes, std::io::Error>(chunk);
                }
                Ok(Some(Err(e))) => {
                    tracing::error!(
                        "[{}] Stream error after {} chunks, {} bytes: {}",
                        protocol, chunk_count, total_bytes, e
                    );
                    break;
                }
                Ok(None) => {
                    tracing::info!(
                        "[{}] Stream completed normally: {} chunks, {} bytes",
                        protocol, chunk_count, total_bytes
                    );
                    break;
                }
                Err(_) => {
                    tracing::warn!(
                        "[{}] Stream idle timeout after {} chunks, {} bytes",
                        protocol, chunk_count, total_bytes
                    );
                    *idle_timeout_flag_for_stream.lock().await = true;
                    break;
                }
            }
        }

        if !response_encoded && !sse_buffer.is_empty() {
            let mut usage = stream_usage_for_stream.lock().await;
            parse_streaming_token_usage(
                sse_buffer.trim_end_matches('\r'),
                protocol,
                &mut usage,
            );
        }

        tracing::debug!("[{}] Stream loop ended naturally", protocol);
        let _ = stream_end_tx.send(()).await;
    };

    // Spawn后台任务记录日志
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
    let log_request_log_id = request_log_id;
    let log_identity = identity.clone();

    tokio::spawn(async move {
        let client_cancelled = stream_end_rx.recv().await.is_none();
        tracing::debug!(
            "[{}] Received stream end notification, client_cancelled={}",
            protocol,
            client_cancelled
        );

        // Reconstruct body from collected chunks (up to 10MB)
        let chunks = collected_chunks.lock().await.clone();
        drop(collected_chunks);
        let full_body: Vec<u8> = chunks.iter().flat_map(|c| c.iter()).copied().collect();
        let body_truncated = *body_truncated_flag.lock().await;

        tracing::info!(
            "[{}] Processing stream log: {} bytes collected",
            protocol,
            full_body.len()
        );

        let content_encoding = log_resp_headers
            .get("content-encoding")
            .and_then(|v| v.to_str().ok());
        let response_encoded = has_body_encoding(content_encoding);
        let (usage, body_str) = if response_encoded {
            match try_decompress(&full_body, content_encoding) {
                Some(body) => (
                    if log_identity.token_usage_enabled {
                        parse_streaming_usage_body(&body, protocol)
                    } else {
                        TokenUsage::default()
                    },
                    streaming_body_log_text(&body, MAX_BODY_LOG, body_truncated),
                ),
                None => {
                    tracing::warn!(
                        "[{}] Failed to decompress streaming response body",
                        protocol
                    );
                    (
                        TokenUsage::default(),
                        streaming_body_log_text(
                            b"[response body decompression failed]",
                            MAX_BODY_LOG,
                            body_truncated,
                        ),
                    )
                }
            }
        } else {
            (
                if log_identity.token_usage_enabled {
                    stream_usage.lock().await.clone()
                } else {
                    TokenUsage::default()
                },
                streaming_body_log_text(&full_body, MAX_BODY_LOG, body_truncated),
            )
        };

        tracing::debug!(
            "[{}] Parsed tokens: input={}, cache_read={}, cache_creation={}, output={}",
            protocol,
            usage.input_tokens,
            usage.cache_read_input_tokens,
            usage.cache_creation_input_tokens,
            usage.output_tokens
        );

        let mut final_log_info = log_info;
        final_log_info.provider_body = Some(body_str);

        // Check idle timeout flag
        let is_idle_timeout = *idle_timeout_flag.lock().await;
        if client_cancelled {
            final_log_info.error_message = Some(CLIENT_CLOSED_REQUEST_MESSAGE.to_string());
        } else if is_idle_timeout {
            final_log_info.error_message = Some("Stream idle timeout".to_string());
        }

        // Record stats
        let elapsed = start_time.elapsed().as_millis() as i64;
        let first_byte_ms = (*first_chunk_ms.lock().await).unwrap_or(first_byte_fallback_ms);
        if !client_cancelled && log_is_success {
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
        } else if !client_cancelled {
            if let Ok((was_blacklisted, prov_name)) =
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
        }

        record_request_stats(
            &log_state,
            &log_identity,
            &log_provider_name,
            log_model_id.as_deref(),
            Some(if client_cancelled {
                CLIENT_CLOSED_REQUEST_STATUS
            } else {
                log_status.as_u16()
            }),
            elapsed,
            first_byte_ms,
            usage,
            &log_client_method,
            &log_client_path,
            log_source_model.as_deref(),
            log_target_model.as_deref(),
            Some(final_log_info),
            log_request_log_id,
        )
        .await;

        tracing::info!("[{}] Delayed log recording completed", protocol);
    });

    if let Some(guard) = &cancel_guard {
        guard.disarm();
    }
    Ok(builder.body(Body::from_stream(stream)).unwrap())
}

async fn handle_non_streaming_request(
    request: reqwest::Request,
    client: &reqwest::Client,
    state: &Arc<AppState>,
    provider_id: i64,
    provider_name: &str,
    identity: RequestIdentity,
    model_id: Option<&str>,
    client_method: &str,
    client_path: &str,
    start_time: Instant,
    timeouts: TimeoutConfig,
    source_model: Option<&str>,
    target_model: Option<&str>,
    mut log_info: RequestLogInfo,
    request_log_id: Option<i64>,
) -> Result<Response<Body>, StatusCode> {
    let protocol = identity.protocol;
    let cancel_guard = RequestLogCancelGuard::new(state, request_log_id, start_time);

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
                let elapsed = start_time.elapsed().as_millis() as i64;
                record_request_stats(
                    state,
                    &identity,
                    provider_name,
                    model_id,
                    None,
                    elapsed,
                    elapsed,
                    TokenUsage::default(),
                    client_method,
                    client_path,
                    source_model,
                    target_model,
                    Some(log_info),
                    request_log_id,
                )
                .await;
                if let Some(guard) = &cancel_guard {
                    guard.disarm();
                }
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
                let elapsed = start_time.elapsed().as_millis() as i64;
                record_request_stats(
                    state,
                    &identity,
                    provider_name,
                    model_id,
                    None,
                    elapsed,
                    elapsed,
                    TokenUsage::default(),
                    client_method,
                    client_path,
                    source_model,
                    target_model,
                    Some(log_info),
                    request_log_id,
                )
                .await;
                if let Some(guard) = &cancel_guard {
                    guard.disarm();
                }
                return Ok(Response::builder()
                    .status(StatusCode::GATEWAY_TIMEOUT)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"error": "Request timeout"}"#))
                    .unwrap());
            }
        };

    let first_byte_ms = start_time.elapsed().as_millis() as i64;
    if let Some(guard) = &cancel_guard {
        guard.set_first_byte_ms(first_byte_ms);
    }
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
            let elapsed = start_time.elapsed().as_millis() as i64;
            record_request_stats(
                state,
                &identity,
                provider_name,
                model_id,
                Some(status.as_u16()),
                elapsed,
                first_byte_ms,
                TokenUsage::default(),
                client_method,
                client_path,
                source_model,
                target_model,
                Some(log_info),
                request_log_id,
            )
            .await;
            if let Some(guard) = &cancel_guard {
                guard.disarm();
            }
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
    if identity.token_usage_enabled {
        parse_token_usage(&decompressed_body, protocol, &mut usage);
    }

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
        &identity,
        provider_name,
        model_id,
        Some(status.as_u16()),
        elapsed,
        first_byte_ms,
        usage,
        client_method,
        client_path,
        source_model,
        target_model,
        Some(log_info),
        request_log_id,
    )
    .await;
    if let Some(guard) = &cancel_guard {
        guard.disarm();
    }

    // Build response
    let mut builder =
        Response::builder().status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK));

    builder = copy_response_headers(builder, &resp_headers);

    Ok(builder.body(Body::from(body_bytes)).unwrap())
}

/// 根据设置过滤日志详情字段
fn filter_log_detail(log_info: &mut RequestLogInfo, mode: &str, is_success: bool) {
    if mode == "failure_only" && is_success {
        log_info.client_headers = None;
        log_info.client_body = None;
        log_info.forward_headers = None;
        log_info.forward_body = None;
        log_info.provider_headers = None;
        log_info.provider_body = None;
    }
}

async fn emit_request_log_event(state: &Arc<AppState>, event: &str, log_id: i64) {
    let log_item = sqlx::query_as::<_, RequestLogItem>(
        "SELECT id, created_at, finished_at, cli_type, protocol, provider_id, profile, provider_name, model_id, status_code, elapsed_ms, first_byte_ms, input_tokens, cache_read_input_tokens, cache_creation_input_tokens, output_tokens, 0.0 as total_cost, client_method, client_path, source_model, target_model FROM request_logs WHERE id = ?",
    )
    .bind(log_id)
    .fetch_one(&state.log_db)
    .await;

    if let Ok(mut item) = log_item {
        let pricing =
            crate::services::cost::provider_pricing(&state.db, &item.cli_type, &item.provider_name)
                .await
                .unwrap_or_default();
        item.total_cost = crate::services::cost::calculate_token_cost(
            pricing,
            item.input_tokens,
            item.cache_read_input_tokens,
            item.cache_creation_input_tokens,
            item.output_tokens,
        );
        if let Err(e) = state.app_handle.emit(event, item) {
            tracing::error!(error = %e, event, "Failed to emit request log event");
        }
    }
}

async fn start_request_log(
    state: &Arc<AppState>,
    identity: &RequestIdentity,
    provider_name: &str,
    model_id: Option<&str>,
    client_method: &str,
    client_path: &str,
    forward_url: Option<&str>,
    source_model: Option<&str>,
    target_model: Option<&str>,
) -> Option<i64> {
    let (debug_log,) =
        sqlx::query_as::<_, (i64,)>("SELECT debug_log FROM gateway_settings WHERE id = 1")
            .fetch_one(&state.db)
            .await
            .unwrap_or((0,));

    if debug_log == 0 {
        return None;
    }

    let log_id = match stats_service::start_request_log(
        &state.log_db,
        &identity.agent_id,
        identity.protocol.as_str(),
        identity.provider_id,
        &identity.profile,
        provider_name,
        model_id,
        client_method,
        client_path,
        forward_url,
        source_model,
        target_model,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::error!(error = %e, "Failed to start request log");
            return None;
        }
    };

    emit_request_log_event(state, "request-log-new", log_id).await;
    Some(log_id)
}

async fn record_request_stats(
    state: &Arc<AppState>,
    identity: &RequestIdentity,
    provider_name: &str,
    model_id: Option<&str>,
    status_code: Option<u16>,
    elapsed_ms: i64,
    first_byte_ms: i64,
    usage: TokenUsage,
    client_method: &str,
    client_path: &str,
    source_model: Option<&str>,
    target_model: Option<&str>,
    log_info: Option<RequestLogInfo>,
    request_log_id: Option<i64>,
) {
    // 读取 gateway 设置
    let settings: (i64, String) = sqlx::query_as::<_, (i64, String)>(
        "SELECT debug_log, log_detail_mode FROM gateway_settings WHERE id = 1",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or((0, "failure_only".to_string()));

    // Derive success from status_code (200-299 = success)
    let success = status_code
        .map(|code| (200..300).contains(&code))
        .unwrap_or(false);

    if let Err(e) = stats_service::record_request(
        &state.stats_db,
        provider_name,
        &identity.agent_id,
        model_id,
        source_model,
        success,
        elapsed_ms,
        usage.input_tokens,
        usage.cache_read_input_tokens,
        usage.cache_creation_input_tokens,
        usage.output_tokens,
    )
    .await
    {
        tracing::error!(error = %e, "Failed to record usage stats");
    }

    // debug_log = 0 时跳过新日志；已开始的日志仍要完成，避免页面一直显示进行中。
    if settings.0 == 0 && request_log_id.is_none() {
        return;
    }

    // 过滤详情字段
    let mut filtered_log_info = log_info;
    if let Some(ref mut info) = filtered_log_info {
        filter_log_detail(info, &settings.1, success);
    }

    let (log_id, event) = if let Some(log_id) = request_log_id {
        if let Err(e) = stats_service::finish_request_log(
            &state.log_db,
            log_id,
            &identity.agent_id,
            identity.protocol.as_str(),
            identity.provider_id,
            &identity.profile,
            provider_name,
            model_id,
            status_code,
            elapsed_ms,
            first_byte_ms,
            usage.input_tokens,
            usage.cache_read_input_tokens,
            usage.cache_creation_input_tokens,
            usage.output_tokens,
            client_method,
            client_path,
            source_model,
            target_model,
            filtered_log_info,
        )
        .await
        {
            tracing::error!(error = %e, "Failed to finish request log");
            return;
        }
        (log_id, "request-log-updated")
    } else {
        let log_id = match stats_service::record_request_log(
            &state.log_db,
            &identity.agent_id,
            identity.protocol.as_str(),
            identity.provider_id,
            &identity.profile,
            provider_name,
            model_id,
            status_code,
            elapsed_ms,
            first_byte_ms,
            usage.input_tokens,
            usage.cache_read_input_tokens,
            usage.cache_creation_input_tokens,
            usage.output_tokens,
            client_method,
            client_path,
            source_model,
            target_model,
            filtered_log_info,
        )
        .await
        {
            Ok(id) => id,
            Err(e) => {
                tracing::error!(error = %e, "Failed to record request log");
                return;
            }
        };
        (log_id, "request-log-new")
    };

    emit_request_log_event(state, event, log_id).await;
}
