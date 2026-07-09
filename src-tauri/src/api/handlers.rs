use axum::{
    body::Body,
    extract::State,
    http::{Response, StatusCode},
};
use bytes::Bytes;
use flate2::read::{DeflateDecoder, GzDecoder, ZlibDecoder};
use futures_util::StreamExt;
use std::io::Read;
use std::sync::Arc;
use std::time::Instant;
use tauri::Emitter;
use tokio::sync::{mpsc, Mutex};

use super::AppState;
use crate::db::models::{RequestLogInfo, RequestLogItem};
use crate::services::proxy::{
    apply_body_model_mapping, apply_url_model_mapping, detect_cli_type,
    detect_gateway_profile, extract_model_from_body, extract_model_from_path,
    is_streaming, parse_streaming_token_usage, parse_token_usage, CliType,
    TimeoutConfig, TokenUsage,
};
use crate::services::routing::{select_provider, split_gateway_profile_path};
use crate::services::{provider as provider_service, stats as stats_service};

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

    let (path_profile, full_path) =
        if let Some((profile, stripped_path)) = split_gateway_profile_path(&raw_full_path) {
            (Some(profile), stripped_path)
        } else {
            (None, raw_full_path.clone())
        };

    // Detect CLI type from User-Agent
    let cli_type = detect_cli_type(&headers);
    let provider_profile = path_profile.unwrap_or_else(|| detect_gateway_profile(&headers));

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
    let provider_with_maps = match select_provider(
        &state.db,
        cli_type.as_str(),
        &provider_profile,
        extracted_model.as_deref(),
    )
    .await
    {
        Ok(Some(p)) => p,
        Ok(None) => {
            tracing::warn!(
                cli_type = %cli_type,
                profile = provider_profile,
                "No available provider"
            );
            // Log system event
            let _ = stats_service::record_system_log(
                &state.log_db,
                "no_provider_available",
                &format!(
                    "CLI 类型 {} / profile {} 没有可用的服务商",
                    cli_type, provider_profile
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

    // Build the upstream request via the shared constructor (single source of
    // truth for hop-by-hop filtering, auth injection, and UA override).
    let request = match crate::services::proxy::build_upstream_request(
        &state.http_client,
        provider,
        cli_type,
        &upstream_url,
        &headers,
        final_body,
        reqwest::Method::from_bytes(method.as_str().as_bytes())
            .unwrap_or(reqwest::Method::GET),
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

    // Execute request
    if streaming {
        handle_streaming_request(
            request,
            &state.http_client,
            &state,
            provider_id,
            &provider_name,
            cli_type,
            model_id.as_deref(),
            method.as_ref(),
            &raw_full_path,
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
            &state.http_client,
            &state,
            provider_id,
            &provider_name,
            cli_type,
            model_id.as_deref(),
            method.as_ref(),
            &raw_full_path,
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

fn copy_response_headers(
    mut builder: axum::http::response::Builder,
    headers: &reqwest::header::HeaderMap,
) -> axum::http::response::Builder {
    for (name, value) in headers.iter() {
        if RESPONSE_FILTERED_HEADERS.iter().any(|h| name.as_str().eq_ignore_ascii_case(h)) {
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
    cli_type: CliType,
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
        parse_streaming_token_usage(&line, cli_type, usage);
    }

    if buffer.len() > MAX_SSE_LINE_BUFFER {
        tracing::warn!(
            "[{}] SSE line exceeded {} bytes before newline; dropping buffered line, token usage in this event may be missed",
            cli_type,
            MAX_SSE_LINE_BUFFER
        );
        buffer.clear();
    }
}

fn parse_streaming_usage_body(body: &[u8], cli_type: CliType) -> TokenUsage {
    let mut usage = TokenUsage::default();
    let mut buffer = String::new();
    parse_streaming_usage_chunk(&mut buffer, body, cli_type, &mut usage);
    if !buffer.is_empty() {
        parse_streaming_token_usage(buffer.trim_end_matches('\r'), cli_type, &mut usage);
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
                let elapsed = start_time.elapsed().as_millis() as i64;
                record_request_stats(
                    state,
                    cli_type,
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
                let elapsed = start_time.elapsed().as_millis() as i64;
                record_request_stats(
                    state,
                    cli_type,
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
                )
                .await;
                return Ok(Response::builder()
                    .status(StatusCode::GATEWAY_TIMEOUT)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"error": "First byte timeout"}"#))
                    .unwrap());
            }
        };

    let first_byte_fallback_ms = start_time.elapsed().as_millis() as i64;
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
                        *first_chunk_ms_for_stream.lock().await =
                            Some(start_time.elapsed().as_millis() as i64);
                    }
                    let chunk_size = chunk.len();
                    total_bytes += chunk_size;

                    if !response_encoded {
                        let mut usage = stream_usage_for_stream.lock().await;
                        parse_streaming_usage_chunk(
                            &mut sse_buffer,
                            chunk.as_ref(),
                            cli_type,
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
                    tracing::info!(
                        "[{}] Stream completed normally: {} chunks, {} bytes",
                        cli_type, chunk_count, total_bytes
                    );
                    break;
                }
                Err(_) => {
                    tracing::warn!(
                        "[{}] Stream idle timeout after {} chunks, {} bytes",
                        cli_type, chunk_count, total_bytes
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
                cli_type,
                &mut usage,
            );
        }

        tracing::debug!("[{}] Stream loop ended naturally", cli_type);
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

    tokio::spawn(async move {
        let _ = stream_end_rx.recv().await;
        tracing::debug!("[{}] Received stream end notification", cli_type);

        // Reconstruct body from collected chunks (up to 10MB)
        let chunks = collected_chunks.lock().await.clone();
        drop(collected_chunks);
        let full_body: Vec<u8> = chunks.iter().flat_map(|c| c.iter()).copied().collect();
        let body_truncated = *body_truncated_flag.lock().await;

        tracing::info!(
            "[{}] Processing stream log: {} bytes collected",
            cli_type,
            full_body.len()
        );

        let content_encoding = log_resp_headers
            .get("content-encoding")
            .and_then(|v| v.to_str().ok());
        let response_encoded = has_body_encoding(content_encoding);
        let (usage, body_str) = if response_encoded {
            match try_decompress(&full_body, content_encoding) {
                Some(body) => (
                    parse_streaming_usage_body(&body, cli_type),
                    streaming_body_log_text(&body, MAX_BODY_LOG, body_truncated),
                ),
                None => {
                    tracing::warn!(
                        "[{}] Failed to decompress streaming response body",
                        cli_type
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
                stream_usage.lock().await.clone(),
                streaming_body_log_text(&full_body, MAX_BODY_LOG, body_truncated),
            )
        };

        tracing::debug!(
            "[{}] Parsed tokens: input={}, cache_read={}, cache_creation={}, output={}",
            cli_type,
            usage.input_tokens,
            usage.cache_read_input_tokens,
            usage.cache_creation_input_tokens,
            usage.output_tokens
        );

        let mut final_log_info = log_info;
        final_log_info.provider_body = Some(body_str);

        // Check idle timeout flag
        let is_idle_timeout = *idle_timeout_flag.lock().await;
        if is_idle_timeout {
            final_log_info.error_message = Some("Stream idle timeout".to_string());
        }

        // Record stats
        let elapsed = start_time.elapsed().as_millis() as i64;
        let first_byte_ms = (*first_chunk_ms.lock().await).unwrap_or(first_byte_fallback_ms);
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
            first_byte_ms,
            usage,
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
                let elapsed = start_time.elapsed().as_millis() as i64;
                record_request_stats(
                    state,
                    cli_type,
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
                let elapsed = start_time.elapsed().as_millis() as i64;
                record_request_stats(
                    state,
                    cli_type,
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
                )
                .await;
                return Ok(Response::builder()
                    .status(StatusCode::GATEWAY_TIMEOUT)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"error": "Request timeout"}"#))
                    .unwrap());
            }
        };

    let first_byte_ms = start_time.elapsed().as_millis() as i64;
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
                cli_type,
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
        first_byte_ms,
        usage,
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

async fn record_request_stats(
    state: &Arc<AppState>,
    cli_type: CliType,
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
        cli_type.as_str(),
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

    // debug_log = 0 时跳过日志记录
    if settings.0 == 0 {
        return;
    }

    // 过滤详情字段
    let mut filtered_log_info = log_info;
    if let Some(ref mut info) = filtered_log_info {
        filter_log_detail(info, &settings.1, success);
    }

    // Record to request_logs and get the inserted ID
    let log_id = match stats_service::record_request_log(
        &state.log_db,
        cli_type.as_str(),
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

    // Query the inserted log item
    let log_item = sqlx::query_as::<_, RequestLogItem>(
        "SELECT id, created_at, cli_type, provider_name, model_id, status_code, elapsed_ms, first_byte_ms, input_tokens, cache_read_input_tokens, cache_creation_input_tokens, output_tokens, 0.0 as total_cost, client_method, client_path, source_model, target_model FROM request_logs WHERE id = ?",
    )
    .bind(log_id)
    .fetch_one(&state.log_db)
    .await;

    // Emit event to frontend
    if let Ok(mut item) = log_item {
        let pricing =
            crate::services::cost::provider_pricing(&state.db, cli_type.as_str(), provider_name)
                .await
                .unwrap_or_default();
        item.total_cost = crate::services::cost::calculate_token_cost(
            pricing,
            usage.input_tokens,
            usage.cache_read_input_tokens,
            usage.cache_creation_input_tokens,
            usage.output_tokens,
        );
        if let Err(e) = state.app_handle.emit("request-log-new", item) {
            tracing::error!(error = %e, "Failed to emit request log event");
        }
    }
}
