use axum::{
    body::Body,
    extract::State,
    http::{Response, StatusCode},
};
use bytes::Bytes;
use flate2::read::{DeflateDecoder, GzDecoder, ZlibDecoder};
use futures_util::{Stream, StreamExt};
use serde_json::Value;
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::Emitter;
use tokio::sync::{mpsc, Mutex};

use super::AppState;
use crate::db::models::{Protocol, ProviderHealthEvent, RequestLogInfo, RequestLogItem};
use crate::services::proxy::{
    apply_body_model_mapping, apply_url_model_mapping, detect_gateway_profile,
    extract_model_from_body, extract_model_from_path, is_stream_completion_line, is_streaming,
    parse_streaming_token_usage, parse_token_usage, stream_body_has_completion, TimeoutConfig,
    TokenUsage,
};
use crate::services::routing::get_available_providers;
use crate::services::{
    agent as agent_service, protocol as protocol_service, provider as provider_service,
    stats as stats_service, translate,
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
const CLIENT_CLOSED_REQUEST_MESSAGE: &str = "客户端在完成前断开了连接";
/// 流式响应体收集进日志的上限，10MB。
const MAX_BODY_LOG: usize = 10 * 1024 * 1024;
/// 整流判定要把错误体整个读进内存，4xx 的错误体最多几 KB，1MB 足够。
const MAX_ERROR_BODY: usize = 1024 * 1024;
/// 服务商熔断状态变化的事件名，前端服务商页监听它做增量更新。
const PROVIDER_HEALTH_EVENT: &str = "provider-health-changed";

#[derive(Clone)]
struct RequestIdentity {
    agent_id: String,
    profile: String,
    protocol: Protocol,
    /// 真的走了协议转换时才有值：请求最终发给上游用的协议。
    upstream_protocol: Option<Protocol>,
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
    let mut headers = req.headers().clone();
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
            // 不匹配任何端点类型的路径（count_tokens、/models 这类辅助端点）不再拒绝：
            // 网关本来就是要把流量转给上游的，挑一家原样透传就好。路径归属决定挑哪家，
            // 一家上游都没有时才报错。
            match crate::services::routing::get_passthrough_provider(
                &state.db,
                &agent.id,
                &provider_profile,
                protocol_service::infer_protocol(&full_path),
            )
            .await
            {
                Ok(Some((provider, upstream_protocol))) => {
                    return handle_passthrough_request(
                        &state,
                        &agent.id,
                        provider,
                        upstream_protocol,
                        &method,
                        &headers,
                        req.into_body(),
                        &full_path,
                    )
                    .await;
                }
                // 路径没命中协议不再算错误，唯一的失败是没有上游可转——这和命中协议时
                // 挑不到服务商是同一件事，报同一个错。
                Ok(None) => {
                    let payload = serde_json::json!({
                        "type": "no_provider_available",
                        "agent_id": agent.id,
                        "profile": provider_profile,
                        "method": method.as_str(),
                        "path": full_path,
                    });
                    let _ = stats_service::record_system_log_dedup(
                        &state.log_db,
                        "no_provider_available",
                        &payload.to_string(),
                        600,
                    )
                    .await;
                    return Ok(json_error_response(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "no_provider_available",
                        "没有可用的服务商，无法透传该请求",
                    ));
                }
                Err(error) => {
                    tracing::error!(error = %error, "Failed to select passthrough provider");
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
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

    // Serialize client headers for logging before normalization, so the log
    // records what the client actually sent (e.g. x-api-key from OpenCode).
    let client_headers_json = serialize_headers(&headers);

    // Normalize x-api-key -> Authorization for Anthropic clients (e.g. OpenCode
    // overriding the built-in anthropic provider) so upstream forwarding sees a
    // single auth source. Profile detection already handles x-api-key directly.
    crate::services::proxy::normalize_anthropic_auth_headers(&mut headers, protocol);

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

    // Codex 私有工具在转换时被展平成普通函数，响应还原要照请求里的形状来。
    let tool_shapes = translate::tool_shapes(protocol, &body_bytes);

    // Only learn from streaming requests since our test is streaming
    if streaming {
        crate::services::proxy::update_captured_headers(&agent.id, protocol, &headers);
    }

    // Extract model name before selecting provider (for blacklist filtering)
    let extracted_model = match protocol {
        Protocol::GeminiGenerateContent => extract_model_from_path(&full_path),
        _ => extract_model_from_body(&body_bytes),
    };

    // Load all candidates once. Each attempt below rebuilds the request so a
    // failed upstream can be replaced before anything is committed to the
    // client.
    let providers = match get_available_providers(
        &state.db,
        &agent.id,
        &provider_profile,
        protocol,
        extracted_model.as_deref(),
    )
    .await
    {
        Ok(providers) if !providers.is_empty() => providers,
        Ok(_) => {
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
                    r#"{"error": "没有可用的服务商"}"#,
                ))
                .unwrap());
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to select provider");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

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

    let model_mapping_enabled = agent.features.model_mapping.enabled;
    // Rotating failover: every failed attempt counts once toward the channel
    // breaker. A channel gets up to its `retry_limit` consecutive attempts
    // before the next channel takes over; once every channel has spent its
    // round budget, the rotation starts over until one channel succeeds or
    // all channels are blacklisted. The last upstream error is what the
    // client sees when every channel is exhausted.
    let mut last_failure: Option<Response<Body>> = None;
    // Channels whose own answer rules out any retry (bad credentials, unknown
    // model): rotating back to them would only repeat the same error.
    let mut skipped: std::collections::HashSet<i64> = std::collections::HashSet::new();

    loop {
    for provider_with_maps in &providers {
        let provider = &provider_with_maps.provider;
        let provider_id = provider.id;
        let provider_name = provider.name.clone();
        // 端点类型与客户端协议不同时转换协议；解析失败按同协议处理（路由已经筛过）。
        let upstream_protocol = provider
            .protocol
            .parse::<Protocol>()
            .unwrap_or(protocol);
        let translate_path = if translate::can_translate(protocol, upstream_protocol) {
            translate::upstream_path(upstream_protocol)
        } else {
            None
        };
        if skipped.contains(&provider_id) {
            continue;
        }
        // Re-check live state: the snapshot is from request start, so a
        // channel disabled or blacklisted mid-request must be skipped.
        if provider_unavailable(&state.db, provider_id).await {
            continue;
        }
        // At least 1 so a stored 0 cannot silence the channel entirely.
        let retry_limit = provider.retry_limit.clamp(1, 20) as usize;
        // 转成 Anthropic 时源请求没给 max_tokens 的兜底值（Anthropic 必填），按上游
        // 模型的上限由服务商自己配；低于 1024 连思考预算都摆不下。
        let translate_max_tokens = provider.translate_max_tokens.max(1024);
        // 事后整流：这一轮已经剥掉过历史里的思考块，不再重复剥。
        let mut rectified = false;

        let mut attempt = 0usize;
        while attempt < retry_limit {
            if provider_unavailable(&state.db, provider_id).await {
                break;
            }

            // Backoff delay after first failure (200ms, 400ms, 800ms, capped at 2s)
            if attempt > 0 {
                let delay_ms = (200u64 * (1 << (attempt - 1))).min(2000);
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
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
                    let mapping =
                        apply_body_model_mapping(&provider_with_maps, &body_bytes, &full_path);
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
            let model_id = target_model.clone().or(source_model.clone());
            // 整流过的这一轮不再回放客户端带回来的私有载荷。
            let replay_key = if rectified {
                translate::NO_REPLAY.to_string()
            } else {
                provider_id.to_string()
            };
            // 转换在模型映射之后做：映射改的是 model 字段，转换才换协议外壳。
            let converted_body = translate_path.and_then(|_| {
                serde_json::from_slice::<Value>(&final_body).ok().and_then(|body| {
                    translate::convert_request(
                        protocol,
                        upstream_protocol,
                        &body,
                        &replay_key,
                        translate_max_tokens,
                    )
                })
                .and_then(|body| serde_json::to_vec(&body).ok())
            });
            let (final_body, final_path) = match (translate_path, converted_body) {
                (Some(path), Some(body)) => (body, path.to_string()),
                (Some(_), None) => {
                    tracing::warn!(
                        provider_id,
                        provider = %provider_name,
                        "请求体无法转成上游协议，跳过该服务商"
                    );
                    skipped.insert(provider_id);
                    break;
                }
                (None, _) => {
                    // 直通路径：同协议不必转换，但历史里的思考块可能已经失效——整流
                    // 这一轮要把它们全剥掉重试，平时也得剥掉网关自己写进去的 Opaque
                    // 信封，那是上一轮走过转换留下的，原生上游一律验不过。
                    let stale = rectified || translate::has_envelope(&final_body);
                    let mut parsed = stale
                        .then(|| serde_json::from_slice::<Value>(&final_body).ok())
                        .flatten();
                    let mut stripped = None;
                    if let Some(body) = parsed.as_mut() {
                        if translate::strip_thinking(protocol, body, rectified) {
                            stripped = serde_json::to_vec(body).ok();
                        }
                    }
                    (stripped.unwrap_or(final_body), final_path)
                }
            };
            let adapted_headers = translate_path.map(|_| {
                crate::services::proxy::adapt_headers_for_protocol(
                    &headers,
                    upstream_protocol,
                    streaming,
                )
            });
            let upstream_url =
                crate::services::proxy::join_upstream_url(&provider.base_url, &final_path);
            let request = match crate::services::proxy::build_upstream_request(
                &state.http_client,
                provider,
                upstream_protocol,
                &upstream_url,
                adapted_headers.as_ref().unwrap_or(&headers),
                final_body,
                reqwest::Method::from_bytes(method.as_str().as_bytes())
                    .unwrap_or(reqwest::Method::GET),
            ) {
                Ok(req) => req,
                Err(e) => {
                    tracing::error!(provider_id, error = %e, "Failed to build request");
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            };
            // Streaming requests no longer force identity encoding to allow
            // full pass-through. Compressed streams skip prefix inspection.

            let log_info = RequestLogInfo {
                client_headers: Some(client_headers_json.clone()),
                client_body: Some(client_body_str.clone()),
                forward_url: Some(upstream_url.clone()),
                forward_headers: Some(serialize_reqwest_headers(request.headers())),
                forward_body: Some(
                    request
                        .body()
                        .and_then(|body| body.as_bytes())
                        .map(truncate_body)
                        .unwrap_or_default(),
                ),
                ..Default::default()
            };
            let identity = RequestIdentity {
                agent_id: agent.id.clone(),
                profile: provider_profile.clone(),
                protocol,
                upstream_protocol: translate_path.map(|_| upstream_protocol),
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

            let result = if streaming {
                handle_streaming_request(
                    request,
                    &state.http_client,
                    &state,
                    provider_id,
                    &provider_name,
                    identity,
                    upstream_protocol,
                    &tool_shapes,
                    model_id.as_deref(),
                    method.as_ref(),
                    &raw_full_path,
                    start_time,
                    timeouts.clone(),
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
                    upstream_protocol,
                    &tool_shapes,
                    model_id.as_deref(),
                    method.as_ref(),
                    &raw_full_path,
                    start_time,
                    timeouts.clone(),
                    source_model.as_deref(),
                    target_model.as_deref(),
                    log_info,
                    request_log_id,
                )
                .await
            };

            let mut response = match result {
                Ok(response) => response,
                Err(status) => json_error_response(
                    status,
                    "upstream_error",
                    "上游请求失败，未收到响应",
                ),
            };

            let status_code = response.status().as_u16();
            // 上游拒了历史里的思考块：这些签名/密文只可能是跨上游回放留下的，剥掉
            // 回放再发一次，其余错误照常走重试与轮转。透传路径也检查，避免信封/空
            // 签名块被原生上游拒后一直卡在 400 无法故障转移。
            if !rectified && is_client_request_error(status_code) {
                let (rejected, buffered) = thinking_rejected(response).await;
                response = buffered;
                if rejected {
                    rectified = true;
                    tracing::warn!(
                        provider_id,
                        provider = %provider_name,
                        "上游拒绝历史思考块，剥掉私有载荷回放后重试"
                    );
                    drain_response_body(response).await;
                    continue;
                }
            }
            if !is_channel_failure(status_code) {
                // 跨协议转换时，不同服务商收到的请求可能不同（参数转换、模型映射、工具展平）
                // 服务商 A 的 400 不代表服务商 B 也会 400，应该尝试轮转一次
                if translate_path.is_some() && is_client_request_error(status_code) {
                    tracing::warn!(
                        provider_id,
                        provider = %provider_name,
                        status = %status_code,
                        "Client error in translation mode, trying next provider"
                    );
                    if let Some(previous) = last_failure.replace(response) {
                        drain_response_body(previous).await;
                    }
                    skipped.insert(provider_id);
                    // 不记熔断：400 大概率是我们转出来的请求这家不认，渠道本身没坏，
                    // 记进熔断会把好渠道拉黑。
                    break;  // 继续外层循环，尝试下一个服务商
                }
                // 同协议或非 400/413/422 的错误，直接返回
                return Ok(response);
            }

            // 401/403/404 don't retry same provider, move to next immediately
            if matches!(status_code, 401 | 403 | 404) {
                tracing::warn!(
                    provider_id,
                    provider = %provider_name,
                    status = %status_code,
                    "Provider authentication/not-found error, skipping retries"
                );
                if let Some(previous) = last_failure.replace(response) {
                    drain_response_body(previous).await;
                }
                skipped.insert(provider_id);
                // The failed attempt still counts toward the breaker.
                record_provider_failure(&state, provider_id).await;
                break;
            }

            tracing::warn!(
                provider_id,
                provider = %provider_name,
                status = %status_code,
                attempt = attempt + 1,
                "Provider attempt failed"
            );
            if let Some(previous) = last_failure.replace(response) {
                drain_response_body(previous).await;
            }

            // Every failed attempt counts once toward the channel breaker;
            // reaching the threshold blacklists it for the cooldown.
            record_provider_failure(&state, provider_id).await;

            if provider_unavailable(&state.db, provider_id).await {
                break;
            }
            attempt += 1;
        }
    }

    // The round is over. Keep rotating while at least one channel is neither
    // blacklisted nor ruled out by its own answer; otherwise give up and let
    // the last upstream error reach the client.
    let mut any_available = false;
    for provider_with_maps in &providers {
        if skipped.contains(&provider_with_maps.provider.id) {
            continue;
        }
        if !provider_unavailable(&state.db, provider_with_maps.provider.id).await {
            any_available = true;
            break;
        }
    }
    if !any_available {
        break;
    }
    }

    Ok(last_failure.unwrap_or_else(|| {
        json_error_response(
            StatusCode::BAD_GATEWAY,
            "upstream_unavailable",
            "所有服务商均未能成功响应",
        )
    }))
}

/// 透传一个不匹配任何协议的请求。
///
/// 只做凭证替换和地址重写：不转换协议、不做模型映射、不解析 token 用量、不写请求日志
/// 和统计，单次尝试不轮转，成功失败都不计入熔断——辅助端点在三方中转上本来就常常没
/// 实现，它答的 404 说明不了这个渠道跑正经请求时的健康状况。
#[allow(clippy::too_many_arguments)]
async fn handle_passthrough_request(
    state: &Arc<AppState>,
    agent_id: &str,
    provider: crate::db::models::Provider,
    upstream_protocol: Protocol,
    method: &axum::http::Method,
    client_headers: &axum::http::HeaderMap,
    body: Body,
    full_path: &str,
) -> Result<Response<Body>, StatusCode> {
    let body_bytes = match axum::body::to_bytes(body, 20 * 1024 * 1024).await {
        Ok(bytes) => bytes.to_vec(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to read passthrough request body");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // 客户端带上来的网关令牌要先归一，否则 x-api-key 会盖掉下面写进去的真实凭证。
    let mut headers = client_headers.clone();
    crate::services::proxy::normalize_anthropic_auth_headers(&mut headers, upstream_protocol);
    let upstream_url = crate::services::proxy::join_upstream_url(&provider.base_url, full_path);
    let request = match crate::services::proxy::build_upstream_request(
        &state.http_client,
        &provider,
        upstream_protocol,
        &upstream_url,
        &headers,
        body_bytes,
        reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap_or(reqwest::Method::GET),
    ) {
        Ok(request) => request,
        Err(e) => {
            tracing::error!(error = %e, "Failed to build passthrough request");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let timeout = std::time::Duration::from_secs(
        sqlx::query_scalar::<_, i64>("SELECT non_stream_timeout FROM timeout_settings WHERE id = 1")
            .fetch_one(&state.db)
            .await
            .unwrap_or(120)
            .clamp(1, 600) as u64,
    );
    // 辅助端点答的都是一次性 JSON，整读进来原样带回；真流式的路径走不到这里。
    let response = match tokio::time::timeout(timeout, state.http_client.execute(request)).await {
        Ok(Ok(response)) => response,
        Ok(Err(e)) => {
            let reason = format!("上游请求失败: {}", e);
            record_passthrough_failure(state, agent_id, &provider.name, method, full_path, &reason)
                .await;
            return Ok(json_error_response(
                StatusCode::BAD_GATEWAY,
                "passthrough_failed",
                &reason,
            ));
        }
        Err(_) => {
            record_passthrough_failure(
                state,
                agent_id,
                &provider.name,
                method,
                full_path,
                "请求超时",
            )
            .await;
            return Ok(json_error_response(
                StatusCode::GATEWAY_TIMEOUT,
                "passthrough_timeout",
                "透传请求超时",
            ));
        }
    };

    let status =
        StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let resp_headers = response.headers().clone();
    let body_bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(e) => {
            let reason = format!("读取上游响应体失败: {}", e);
            record_passthrough_failure(state, agent_id, &provider.name, method, full_path, &reason)
                .await;
            return Ok(json_error_response(
                StatusCode::BAD_GATEWAY,
                "passthrough_failed",
                &reason,
            ));
        }
    };

    if !status.is_success() {
        // 上游明确答了就原样带回客户端，只留一条去重日志，方便看出它没实现这个端点。
        record_passthrough_failure(
            state,
            agent_id,
            &provider.name,
            method,
            full_path,
            &format!("上游返回 {}", status.as_u16()),
        )
        .await;
    }

    let builder = copy_response_headers(Response::builder().status(status), &resp_headers);
    Ok(builder
        .body(Body::from(body_bytes))
        .unwrap_or_else(|_| {
            json_error_response(
                StatusCode::BAD_GATEWAY,
                "passthrough_failed",
                "透传响应构造失败",
            )
        }))
}

/// 透传失败留一条 1 小时去重的系统日志：成功的透传完全静默，失败的看得见又不刷屏。
/// 窗口比别的诊断长，因为"这家上游没实现这个端点"是个静态事实，知道一次就够了。
async fn record_passthrough_failure(
    state: &Arc<AppState>,
    agent_id: &str,
    provider_name: &str,
    method: &axum::http::Method,
    full_path: &str,
    reason: &str,
) {
    let payload = serde_json::json!({
        "type": "passthrough_failed",
        "agent_id": agent_id,
        "provider": provider_name,
        "method": method.as_str(),
        "path": full_path,
        "reason": reason,
    });
    let _ = stats_service::record_system_log_dedup(
        &state.log_db,
        "passthrough_failed",
        &payload.to_string(),
        3600,
    )
    .await;
}

/// The upstream rejected the request itself. Another channel would reject it
/// the same way, so the error goes straight back to the client and must not
/// count against the channel.
fn is_client_request_error(status: u16) -> bool {
    matches!(status, 400 | 413 | 422)
}

fn is_channel_failure(status: u16) -> bool {
    status >= 400 && !is_client_request_error(status)
}

/// Whether this channel should no longer receive retries. Disabled, deleted,
/// or currently blacklisted channels are skipped so the next channel can take
/// over without waiting for the original snapshot to exhaust. A read error is
/// treated as unavailable so a failing request always makes progress.
async fn provider_unavailable(db: &sqlx::SqlitePool, provider_id: i64) -> bool {
    let now = crate::time::now_timestamp();
    sqlx::query_as::<_, (i64,)>(
        r#"
        SELECT COUNT(*) FROM providers
        WHERE id = ?
          AND enabled = 1
          AND (blacklisted_until IS NULL OR blacklisted_until <= ?)
        "#,
    )
    .bind(provider_id)
    .bind(now)
    .fetch_one(db)
    .await
    .map_or(true, |(count,)| count == 0)
}

async fn drain_response_body(response: Response<Body>) {
    let _ = axum::body::to_bytes(response.into_body(), 2 * 1024 * 1024).await;
}

/// 上游是不是在抱怨历史里的思考块。只认签名/思考块本身被拒这几种说法，别的 4xx
/// 交给正常的重试与轮转，免得把普通参数错误也当成整流机会。
fn rejects_thinking(message: &str) -> bool {
    const SCENARIOS: [&[&str]; 9] = [
        &["signature", "thinking"],
        &["thought signature"],
        &["must start with a thinking block"],
        &["expected", "thinking", "found", "tool_use"],
        &["signature", "field required"],
        &["signature", "extra inputs are not permitted"],
        &["thinking", "cannot be modified"],
        &["reasoning", "without its required"],
        &["encrypted_content"],
    ];
    SCENARIOS
        .iter()
        .any(|keywords| keywords.iter().all(|keyword| message.contains(keyword)))
}

/// 4xx 的错误体上游一定已经发全，读出来判定完原样装回去，好继续往下走。
async fn thinking_rejected(response: Response<Body>) -> (bool, Response<Body>) {
    let (parts, body) = response.into_parts();
    let bytes = match axum::body::to_bytes(body, MAX_ERROR_BODY).await {
        Ok(bytes) => bytes,
        // 读不出来（超限或连接中断）就没法原样装回去了。补一个同状态码的说明体，
        // 不能把空 body 甩给客户端。
        Err(error) => {
            tracing::warn!(status = %parts.status, %error, "读取上游错误响应体失败");
            return (
                false,
                json_error_response(parts.status, "upstream_error", "上游错误响应体读取失败"),
            );
        }
    };
    let rejected = rejects_thinking(&String::from_utf8_lossy(&bytes).to_lowercase());
    (rejected, Response::from_parts(parts, Body::from(bytes)))
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

/// Pass through upstream error response with original body and headers.
/// Used when we want to preserve the upstream's exact error format for Agent compatibility.
fn build_passthrough_error_response(
    status: StatusCode,
    body: &[u8],
    upstream_headers: &reqwest::header::HeaderMap,
) -> Response<Body> {
    let mut builder = Response::builder().status(status);

    // Copy relevant headers from upstream (content-type, etc.)
    for (name, value) in upstream_headers.iter() {
        let name_str = name.as_str();
        if name_str.eq_ignore_ascii_case("content-type")
            || name_str.eq_ignore_ascii_case("content-language")
        {
            if let Ok(header_name) = axum::http::HeaderName::from_bytes(name.as_str().as_bytes()) {
                if let Ok(header_value) = axum::http::HeaderValue::from_bytes(value.as_bytes()) {
                    builder = builder.header(header_name, header_value);
                }
            }
        }
    }

    builder
        .body(Body::from(body.to_vec()))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap()
        })
}

enum StreamStartFailure {
    Timeout,
    Empty,
    Body(String),
    Protocol(String),
}

impl StreamStartFailure {
    fn status(&self) -> StatusCode {
        match self {
            Self::Timeout => StatusCode::GATEWAY_TIMEOUT,
            Self::Empty | Self::Body(_) | Self::Protocol(_) => StatusCode::BAD_GATEWAY,
        }
    }

    fn log_message(&self) -> String {
        match self {
            Self::Timeout => "首字节超时".to_string(),
            Self::Empty => "上游返回了空流".to_string(),
            Self::Body(error) => format!("上游流错误: {}", error),
            Self::Protocol(error) => format!("上游流协议错误: {}", error),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamResponseKind {
    Sse,
    Json,
}

struct StreamPrefix {
    chunks: Vec<Bytes>,
    kind: StreamResponseKind,
}

enum StreamPrefixState {
    NeedMore,
    Ready(StreamResponseKind),
    Error(String),
}

fn inspect_stream_prefix(
    protocol: Protocol,
    prefix: &mut Vec<u8>,
    end_of_stream: bool,
    hinted_kind: Option<StreamResponseKind>,
) -> StreamPrefixState {
    while let Some((event_len, delimiter_len)) = find_sse_boundary(prefix) {
        let event: Vec<u8> = prefix.drain(..event_len + delimiter_len).collect();
        let event = &event[..event_len];
        if event.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        if let Some(error) = stream_event_error(protocol, event) {
            return StreamPrefixState::Error(error);
        }
        if stream_event_has_data(event) || looks_like_sse(event) {
            return StreamPrefixState::Ready(StreamResponseKind::Sse);
        }
    }

    let trimmed = prefix
        .iter()
        .copied()
        .skip_while(u8::is_ascii_whitespace)
        .collect::<Vec<_>>();
    if trimmed.is_empty() {
        return StreamPrefixState::NeedMore;
    }

    // Some upstreams return a JSON error despite accepting a streaming request.
    // Detect complete JSON without waiting for an SSE delimiter.
    if !looks_like_sse(&trimmed) {
        if let Ok(value) = serde_json::from_slice::<Value>(&trimmed) {
            if let Some(error) = json_stream_error(protocol, None, &value) {
                return StreamPrefixState::Error(error);
            }
            return StreamPrefixState::Ready(StreamResponseKind::Json);
        }
    }

    if looks_like_sse(&trimmed) {
        if let Some(error) = stream_event_error(protocol, &trimmed) {
            return StreamPrefixState::Error(error);
        }
        return StreamPrefixState::Ready(StreamResponseKind::Sse);
    }

    if end_of_stream {
        if hinted_kind == Some(StreamResponseKind::Json) {
            return StreamPrefixState::Error(
                "上游返回了无效的 JSON 响应".to_string(),
            );
        }
        if hinted_kind == Some(StreamResponseKind::Sse) && stream_event_has_data(&trimmed) {
            return StreamPrefixState::Ready(StreamResponseKind::Sse);
        }
    }

    StreamPrefixState::NeedMore
}

fn stream_kind_from_content_type(content_type: Option<&str>) -> Option<StreamResponseKind> {
    let content_type = content_type?.to_ascii_lowercase();
    if content_type.contains("text/event-stream") {
        Some(StreamResponseKind::Sse)
    } else if content_type.contains("json") {
        Some(StreamResponseKind::Json)
    } else {
        None
    }
}

fn find_sse_boundary(bytes: &[u8]) -> Option<(usize, usize)> {
    for index in 0..bytes.len().saturating_sub(1) {
        if bytes[index] == b'\n' && bytes[index + 1] == b'\n' {
            return Some((index, 2));
        }
        if index + 3 < bytes.len() && &bytes[index..index + 4] == b"\r\n\r\n" {
            return Some((index, 4));
        }
    }
    None
}

fn looks_like_sse(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    text.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with("data:")
            || line.starts_with("event:")
            || line.starts_with("id:")
            || line.starts_with("retry:")
            || line.starts_with(':')
    })
}

fn stream_event_has_data(event: &[u8]) -> bool {
    String::from_utf8_lossy(event).lines().any(|line| {
        line.trim_start()
            .strip_prefix("data:")
            .is_some_and(|data| !data.trim().is_empty())
    })
}

fn stream_event_error(protocol: Protocol, event: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(event);
    let event_name = text.lines().find_map(|line| {
        line.trim_start()
            .strip_prefix("event:")
            .map(str::trim)
            .filter(|name| !name.is_empty())
    });
    let data = text
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("data:"))
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if let Ok(value) = serde_json::from_str::<Value>(&data) {
        if let Some(error) = json_stream_error(protocol, event_name, &value) {
            return Some(error);
        }
    }
    if event_name.is_some_and(is_error_event_name) {
        Some(if data.is_empty() {
            event_name.unwrap_or("error").to_string()
        } else {
            truncate_stream_error(&data)
        })
    } else {
        None
    }
}

fn json_stream_error(
    protocol: Protocol,
    event_name: Option<&str>,
    value: &Value,
) -> Option<String> {
    let explicit_event_error = event_name.is_some_and(is_error_event_name);
    let value_type = value
        .get("type")
        .and_then(Value::as_str)
        .map(str::to_ascii_lowercase);
    let value_status = value
        .get("status")
        .and_then(Value::as_str)
        .map(str::to_ascii_lowercase);
    let response_status = value
        .get("response")
        .and_then(|response| response.get("status"))
        .and_then(Value::as_str)
        .map(str::to_ascii_lowercase);
    let structured_error = value.get("error").is_some_and(|error| !error.is_null())
        || value_type.as_deref().is_some_and(|kind| {
            kind == "error"
                || kind.ends_with(".error")
                || kind.ends_with(".failed")
                || kind == "failed"
        })
        || (matches!(protocol, Protocol::OpenaiResponses)
            && (value_status.as_deref() == Some("failed")
                || response_status.as_deref() == Some("failed")));
    if !explicit_event_error && !structured_error {
        return None;
    }

    let message = value
        .get("error")
        .and_then(|error| {
            error
                .get("message")
                .and_then(Value::as_str)
                .or_else(|| error.as_str())
        })
        .or_else(|| {
            value
                .get("response")
                .and_then(|response| response.get("error"))
                .and_then(|error| {
                    error
                        .get("message")
                        .and_then(Value::as_str)
                        .or_else(|| error.as_str())
                })
        })
        .or_else(|| value.get("message").and_then(Value::as_str))
        .or_else(|| {
            value
                .get("response")
                .and_then(|response| response.get("message"))
                .and_then(Value::as_str)
        })
        .or_else(|| value.get("detail").and_then(Value::as_str))
        .or_else(|| event_name.filter(|name| !name.is_empty()))
        .unwrap_or("upstream returned a streaming error");
    Some(truncate_stream_error(message))
}

fn is_error_event_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name == "error" || name.contains("error") || name.ends_with(".failed")
}

fn truncate_stream_error(value: &str) -> String {
    value.chars().take(2048).collect()
}

fn response_body_error(protocol: Protocol, body: &[u8]) -> Option<String> {
    let value = serde_json::from_slice::<Value>(body).ok()?;
    json_stream_error(protocol, None, &value)
}

fn upstream_error_message(protocol: Protocol, body: &[u8]) -> String {
    if let Some(message) = response_body_error(protocol, body) {
        return message;
    }
    if let Ok(value) = serde_json::from_slice::<Value>(body) {
        for key in ["message", "detail", "error", "description"] {
            if let Some(message) = value.get(key).and_then(Value::as_str) {
                if !message.trim().is_empty() {
                    return truncate_stream_error(message);
                }
            }
        }
    }
    let text = String::from_utf8_lossy(body);
    let text = text.trim();
    if text.is_empty() {
        "上游返回了 HTTP 错误".to_string()
    } else {
        truncate_stream_error(text)
    }
}

fn stream_error_event(protocol: Protocol) -> Bytes {
    stream_error_event_with_message(protocol, "上游流中断，未完成")
}

fn stream_error_event_with_message(protocol: Protocol, message: &str) -> Bytes {
    let payload = serde_json::json!({
        "type": "error",
        "error": {
            "type": "upstream_stream_error",
            "message": message,
        }
    });
    let body = match protocol {
        Protocol::AnthropicMessages | Protocol::OpenaiResponses => {
            format!("event: error\ndata: {}\n\n", payload)
        }
        Protocol::OpenaiChat | Protocol::GeminiGenerateContent => {
            format!("data: {}\n\n", payload)
        }
    };
    Bytes::from(body)
}

/// Drain complete SSE events from a rolling buffer. Incomplete events stay in
/// the buffer so an error split across network chunks is still recognized.
fn drain_sse_events(buffer: &mut Vec<u8>, protocol: Protocol) -> (Bytes, Option<String>) {
    const MAX_PENDING_EVENT_BYTES: usize = 2 * 1024 * 1024;
    let mut output = Vec::new();
    let mut error_message = None;

    while let Some((event_len, delimiter_len)) = find_sse_boundary(buffer) {
        let event_end = event_len + delimiter_len;
        let event: Vec<u8> = buffer.drain(..event_end).collect();
        if let Some(message) = stream_event_error(protocol, &event[..event_len]) {
            error_message = Some(message.clone());
            output.extend_from_slice(&stream_error_event_with_message(protocol, &message));
            buffer.clear();
            break;
        }
        output.extend_from_slice(&event);
    }

    // A malformed or unusually large event must not hold the stream forever.
    if error_message.is_none() && buffer.len() > MAX_PENDING_EVENT_BYTES {
        output.extend_from_slice(buffer);
        buffer.clear();
    }

    (Bytes::from(output), error_message)
}

async fn record_provider_failure(state: &Arc<AppState>, provider_id: i64) {
    if let Ok((was_blacklisted, provider_name)) =
        provider_service::record_failure(&state.db, provider_id).await
    {
        if was_blacklisted {
            let _ = stats_service::record_system_log(
                &state.log_db,
                "provider_blacklisted",
                &format!("服务商 {} 因连续失败已被加入黑名单", provider_name),
            )
            .await;
        }
        emit_provider_health_event(state, provider_id).await;
    }
}

async fn record_provider_success(state: &Arc<AppState>, provider_id: i64, provider_name: &str) {
    if let Ok(had_failures) = provider_service::record_success(&state.db, provider_id).await {
        if had_failures {
            let _ = stats_service::record_system_log(
                &state.log_db,
                "provider_recovered",
                &format!("服务商 {} 已恢复正常", provider_name),
            )
            .await;
            emit_provider_health_event(state, provider_id).await;
        }
    }
}

/// 把服务商当前的熔断状态推给前端，让服务商页无需切页就能刷新状态。
async fn emit_provider_health_event(state: &Arc<AppState>, provider_id: i64) {
    let row = sqlx::query_as::<_, (i64, Option<i64>)>(
        "SELECT consecutive_failures, blacklisted_until FROM providers WHERE id = ?",
    )
    .bind(provider_id)
    .fetch_optional(&state.db)
    .await;

    if let Ok(Some((failures, blacklisted_until))) = row {
        let event = ProviderHealthEvent::new(provider_id, failures, blacklisted_until);
        if let Err(e) = state.app_handle.emit(PROVIDER_HEALTH_EVENT, event) {
            tracing::error!(error = %e, "Failed to emit provider health event");
        }
    }
}

async fn read_stream_prefix<S>(
    byte_stream: &mut S,
    protocol: Protocol,
    response_encoded: bool,
    content_type: Option<&str>,
    timeout: std::time::Duration,
) -> Result<StreamPrefix, StreamStartFailure>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    const MAX_PREFIX_BYTES: usize = 1024 * 1024;
    let deadline = Instant::now() + timeout;
    let mut chunks = Vec::new();
    let mut prefix = Vec::new();
    let hinted_kind = stream_kind_from_content_type(content_type);

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(StreamStartFailure::Timeout);
        }
        let chunk = match tokio::time::timeout(remaining, byte_stream.next()).await {
            Ok(Some(Ok(chunk))) => chunk,
            Ok(Some(Err(error))) => {
                return Err(StreamStartFailure::Body(error.to_string()));
            }
            Ok(None) => {
                return match inspect_stream_prefix(protocol, &mut prefix, true, hinted_kind) {
                    StreamPrefixState::Ready(kind) => Ok(StreamPrefix { chunks, kind }),
                    StreamPrefixState::Error(error) => Err(StreamStartFailure::Protocol(error)),
                    StreamPrefixState::NeedMore if prefix.is_empty() => {
                        Err(StreamStartFailure::Empty)
                    }
                    StreamPrefixState::NeedMore => Err(StreamStartFailure::Protocol(
                        "上游流在有效事件前结束".to_string(),
                    )),
                };
            }
            Err(_) => return Err(StreamStartFailure::Timeout),
        };
        if chunk.is_empty() {
            continue;
        }

        prefix.extend_from_slice(&chunk);
        chunks.push(chunk);
        if response_encoded || prefix.len() >= MAX_PREFIX_BYTES {
            return Ok(StreamPrefix {
                chunks,
                kind: hinted_kind.unwrap_or(StreamResponseKind::Sse),
            });
        }

        match inspect_stream_prefix(protocol, &mut prefix, false, hinted_kind) {
            StreamPrefixState::Ready(kind) => return Ok(StreamPrefix { chunks, kind }),
            StreamPrefixState::Error(error) => return Err(StreamStartFailure::Protocol(error)),
            StreamPrefixState::NeedMore => {}
        }
    }
}

/// 上游 SSE -> 客户端协议 SSE。原始上游字节顺带收进 `collected`，日志里的「服务商
/// 响应体」记的是上游真正发的内容，不是转换后的。`completed` 记录上游是否正常 EOF，
/// 非正常结束（传输错误/超时）不补完成事件，避免把截断流伪造成完整响应。
fn translated_stream<S>(
    upstream: S,
    upstream_protocol: Protocol,
    mut translator: translate::StreamTranslator,
    collected: Arc<Mutex<Vec<Bytes>>>,
    truncated: Arc<Mutex<bool>>,
    completed: Arc<Mutex<bool>>,
) -> impl Stream<Item = Result<Bytes, reqwest::Error>>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
{
    async_stream::stream! {
        let mut upstream = Box::pin(upstream);
        let mut collected_bytes = 0usize;
        while let Some(item) = upstream.next().await {
            let chunk = match item {
                Ok(chunk) => chunk,
                Err(error) => {
                    yield Err(error);
                    return;
                }
            };
            if collected_bytes < MAX_BODY_LOG {
                let to_collect = chunk.len().min(MAX_BODY_LOG - collected_bytes);
                if to_collect > 0 {
                    collected.lock().await.push(chunk.slice(..to_collect));
                    collected_bytes += to_collect;
                }
                if to_collect < chunk.len() {
                    *truncated.lock().await = true;
                }
            } else {
                *truncated.lock().await = true;
            }
            let output = translator.push(&chunk);
            if !output.is_empty() {
                yield Ok(Bytes::from(output));
            }
        }
        // 请求写了 stream，上游却把一整块 JSON 当响应回来（兼容端点很常见）。转换器
        // 会把它摊成流式事件，错误体也一样会被摊成「空内容 + 正常结束」，客户端和
        // 网关双双当成功。所以先按**上游**协议判错，口径与非流式、透传两条路径共用
        // 同一个 response_body_error。判出错误就发错误事件，首包检查会据此判失败并
        // 切下一个服务商。判错只喂还没被当成 SSE 帧吃掉的那一段：上游先发过保活注释
        // 的话，整条流的字节拼起来不是合法 JSON，判错会整个漏掉。
        let pending_error = translator
            .pending_body()
            .and_then(|body| response_body_error(upstream_protocol, body));
        if let Some(message) = pending_error {
            let message = format!("上游响应错误: {}", message);
            tracing::warn!(error = %message, "Upstream returned a structured error body on a streaming request");
            let frame = translator.error(&message);
            if !frame.is_empty() {
                yield Ok(Bytes::from(frame));
            }
            return;
        }
        // 上游流干净结束才补完成事件并标记 completed，中途传输错误已经走 Err 分支
        // 提前返回了，不会走到这里。
        let tail = translator.finish();
        if !tail.is_empty() {
            yield Ok(Bytes::from(tail));
        }
        *completed.lock().await = true;
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

/// 转换模式下上游的 content-type / content-length / content-encoding 都已经不成立，
/// 不能整份照抄；但限流与追踪类的头对客户端有用（Claude Code 靠 retry-after 决定
/// 退避），按前缀挑出来带上。
fn copy_metadata_headers(
    mut builder: axum::http::response::Builder,
    headers: &reqwest::header::HeaderMap,
) -> axum::http::response::Builder {
    const KEPT: [&str; 5] = [
        "retry-after",
        "x-ratelimit-",
        "anthropic-ratelimit-",
        "x-request-id",
        "request-id",
    ];
    for (name, value) in headers.iter() {
        let name = name.as_str();
        if !KEPT.iter().any(|kept| {
            name.len() >= kept.len() && name[..kept.len()].eq_ignore_ascii_case(kept)
        }) {
            continue;
        }
        if let Ok(header_value) = axum::http::HeaderValue::from_bytes(value.as_bytes()) {
            if let Ok(header_name) = axum::http::HeaderName::from_bytes(name.as_bytes()) {
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
) -> bool {
    const MAX_SSE_LINE_BUFFER: usize = 1024 * 1024;
    let mut stream_completed = false;

    buffer.push_str(&String::from_utf8_lossy(chunk));
    while let Some(pos) = buffer.find('\n') {
        let mut line: String = buffer.drain(..=pos).collect();
        if line.ends_with('\n') {
            line.pop();
        }
        if line.ends_with('\r') {
            line.pop();
        }
        stream_completed |= is_stream_completion_line(&line, protocol);
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

    stream_completed
}

fn parse_streaming_usage_body(body: &[u8], protocol: Protocol) -> TokenUsage {
    let mut usage = TokenUsage::default();
    let mut buffer = String::new();
    let _ = parse_streaming_usage_chunk(&mut buffer, body, protocol, &mut usage);
    if !buffer.is_empty() {
        let line = buffer.trim_end_matches('\r');
        parse_streaming_token_usage(line, protocol, &mut usage);
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
    upstream_protocol: Protocol,
    tool_shapes: &translate::ToolShapes,
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
    let translating = translate::can_translate(protocol, upstream_protocol);
    let cancel_guard = RequestLogCancelGuard::new(state, request_log_id, start_time);

    // Send request with timeout for first byte
    let response =
        match tokio::time::timeout(timeouts.first_byte_timeout, client.execute(request)).await {
            Ok(Ok(resp)) => resp,
            Ok(Err(e)) => {
                tracing::error!(error = %e, "Upstream request failed");
                log_info.error_message = Some(format!("上游请求失败: {}", e));
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
                return Ok(json_error_response(
                    StatusCode::BAD_GATEWAY,
                    "upstream_error",
                    &format!("上游请求失败: {}", e),
                ));
            }
            Err(_) => {
                tracing::error!("First byte timeout");
                log_info.error_message = Some("首字节超时".to_string());
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
                return Ok(json_error_response(
                    StatusCode::GATEWAY_TIMEOUT,
                    "upstream_timeout",
                    "首字节超时",
                ));
            }
        };

    let status = response.status();
    let resp_headers = response.headers().clone();

    // Store provider response info
    log_info.provider_headers = Some(serialize_reqwest_headers(&resp_headers));

    if !status.is_success() {
        let error_body =
            match tokio::time::timeout(timeouts.first_byte_timeout, response.bytes()).await {
                Ok(Ok(body)) => maybe_decompress(
                    &body,
                    resp_headers
                        .get("content-encoding")
                        .and_then(|value| value.to_str().ok()),
                ),
                Ok(Err(error)) => format!("读取上游错误响应失败: {}", error).into_bytes(),
                Err(_) => "读取上游错误响应超时".as_bytes().to_vec(),
            };
        let message = upstream_error_message(upstream_protocol, &error_body);
        tracing::warn!(
            provider_id,
            provider = provider_name,
            status = %status,
            error = %message,
            "Upstream returned an HTTP error before streaming started"
        );
        log_info.provider_body = Some(truncate_body(&error_body));
        log_info.error_message = Some(message.clone());
        let elapsed = start_time.elapsed().as_millis() as i64;
        record_request_stats(
            state,
            &identity,
            provider_name,
            model_id,
            Some(status.as_u16()),
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
        // Pass through real HTTP errors with original status and body
        return Ok(build_passthrough_error_response(status, &error_body, &resp_headers));
    }

    // 响应体收集缓冲：转换模式下由 translated_stream 填原始上游字节，其余情况在下面
    // 的流循环里填规整后的输出。
    let collected_chunks = Arc::new(Mutex::new(Vec::<Bytes>::new()));
    let body_truncated_flag = Arc::new(Mutex::new(false));

    let upstream_encoded = has_body_encoding(
        resp_headers
            .get("content-encoding")
            .and_then(|value| value.to_str().ok()),
    );
    // 转换必须读明文。请求已经要过 identity，上游还压缩就交给下一个服务商。
    if translating && upstream_encoded {
        let message = "上游压缩了流式响应，无法转换协议".to_string();
        tracing::warn!(provider_id, provider = provider_name, error = %message, "Compressed upstream stream cannot be translated");
        log_info.error_message = Some(message.clone());
        let elapsed = start_time.elapsed().as_millis() as i64;
        record_request_stats(
            state,
            &identity,
            provider_name,
            model_id,
            Some(StatusCode::BAD_GATEWAY.as_u16()),
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
        return Ok(json_error_response(
            StatusCode::BAD_GATEWAY,
            "translate_failed",
            &message,
        ));
    }
    // 转换后到客户端的一定是 SSE，后续判定全部按客户端协议走。
    let response_encoded = upstream_encoded && !translating;
    let content_type = if translating {
        Some("text/event-stream")
    } else {
        resp_headers
            .get("content-type")
            .and_then(|value| value.to_str().ok())
    };
    let translator = if translating {
        // 上游标识：客户端回传的私有载荷只在原路回到同一个服务商时才回放。
        translate::StreamTranslator::new(
            upstream_protocol,
            protocol,
            &provider_id.to_string(),
            tool_shapes,
        )
    } else {
        None
    };
    // 转换流需要跟踪上游是否真正完成，免得截断时伪造完成事件。
    let stream_really_completed = Arc::new(Mutex::new(false));
    let stream_really_completed_for_check = stream_really_completed.clone();
    let mut byte_stream: std::pin::Pin<
        Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>,
    > = match translator {
        Some(translator) => Box::pin(translated_stream(
            response.bytes_stream(),
            upstream_protocol,
            translator,
            collected_chunks.clone(),
            body_truncated_flag.clone(),
            stream_really_completed.clone(),
        )),
        None => Box::pin(response.bytes_stream()),
    };
    let prefetched = if status.is_success() {
        match read_stream_prefix(
            &mut byte_stream,
            protocol,
            response_encoded,
            content_type,
            timeouts.first_byte_timeout,
        )
        .await
        {
            Ok(chunks) => chunks,
            Err(failure) => {
                let failure_status = failure.status();
                let message = failure.log_message();
                tracing::warn!(provider_id, provider = provider_name, error = %message, "Upstream stream failed before the client response started");
                log_info.error_message = Some(message.clone());
                let elapsed = start_time.elapsed().as_millis() as i64;
                record_request_stats(
                    state,
                    &identity,
                    provider_name,
                    model_id,
                    Some(failure_status.as_u16()),
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
                // Stream prefix error: gateway-generated JSON with 502/504
                return Ok(json_error_response(
                    failure_status,
                    "upstream_stream_error",
                    &message,
                ));
            }
        }
    } else {
        StreamPrefix {
            chunks: Vec::new(),
            kind: stream_kind_from_content_type(content_type).unwrap_or(StreamResponseKind::Sse),
        }
    };
    let StreamPrefix {
        chunks: prefetched_chunks,
        kind: stream_kind,
    } = prefetched;

    let first_byte_fallback_ms = start_time.elapsed().as_millis() as i64;
    if let Some(guard) = &cancel_guard {
        guard.set_first_byte_ms(first_byte_fallback_ms);
    }

    // Build response headers
    let mut builder =
        Response::builder().status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK));

    // 转换后上游的 content-type / content-length 已经不成立，只给必要的头，另外把
    // 限流与追踪类的头挑出来带上。
    builder = if translating {
        copy_metadata_headers(builder, &resp_headers)
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
    } else {
        copy_response_headers(builder, &resp_headers)
    };

    // Create streaming body
    let is_success = status.is_success();

    let collected_chunks_for_stream = collected_chunks.clone();
    let stream_usage = Arc::new(Mutex::new(TokenUsage::default()));
    let stream_usage_for_stream = stream_usage.clone();
    let body_truncated_flag_for_stream = body_truncated_flag.clone();
    let idle_timeout_flag = Arc::new(Mutex::new(false));
    let idle_timeout_flag_for_stream = idle_timeout_flag.clone();
    let stream_failure = Arc::new(Mutex::new(None::<String>));
    let stream_failure_for_stream = stream_failure.clone();
    let stream_completed = Arc::new(AtomicBool::new(false));
    let stream_completed_for_stream = stream_completed.clone();
    let first_chunk_ms = Arc::new(Mutex::new(None::<i64>));
    let first_chunk_ms_for_stream = first_chunk_ms.clone();
    let first_byte_log_state = state.clone();
    let first_byte_log_id = request_log_id;
    // 创建channel用于通知stream结束
    let (stream_end_tx, mut stream_end_rx) = mpsc::channel::<()>(1);

    let stream = async_stream::stream! {
        let idle_timeout = timeouts.idle_timeout;
        let mut chunk_count = 0usize;
        let mut total_bytes = 0usize;
        let mut collected_bytes = 0usize;
        let mut sse_buffer = String::new();
        let mut pending_sse = Vec::new();
        let mut upstream_error_event = false;
        let mut prefetched_chunks = prefetched_chunks.into_iter();

        loop {
            let next = if let Some(chunk) = prefetched_chunks.next() {
                Ok(Some(Ok(chunk)))
            } else {
                tokio::time::timeout(idle_timeout, byte_stream.next()).await
            };
            match next {
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

                    if !response_encoded && stream_kind == StreamResponseKind::Sse {
                        let mut usage = stream_usage_for_stream.lock().await;
                        if parse_streaming_usage_chunk(
                            &mut sse_buffer,
                            chunk.as_ref(),
                            protocol,
                            &mut usage,
                        ) {
                            stream_completed_for_stream.store(true, Ordering::Relaxed);
                        }
                    }

                    let (output_chunk, error_message) =
                        if !response_encoded && stream_kind == StreamResponseKind::Sse {
                            pending_sse.extend_from_slice(&chunk);
                            drain_sse_events(&mut pending_sse, protocol)
                        } else {
                            (chunk.clone(), None)
                        };
                    if let Some(message) = error_message {
                        upstream_error_event = true;
                        *stream_failure_for_stream.lock().await =
                            Some(format!("上游流错误: {}", message));
                    }

                    // Collect the normalized chunk for body logging.
                    // 转换模式下日志记的是原始上游响应，已经由 translated_stream 收好。
                    if !translating {
                        if collected_bytes < MAX_BODY_LOG {
                            let mut chunks = collected_chunks_for_stream.lock().await;
                            let to_collect = output_chunk.len().min(MAX_BODY_LOG - collected_bytes);
                            if to_collect > 0 {
                                chunks.push(output_chunk.slice(..to_collect));
                                collected_bytes += to_collect;
                            }
                            if to_collect < output_chunk.len() {
                                *body_truncated_flag_for_stream.lock().await = true;
                            }
                        } else {
                            *body_truncated_flag_for_stream.lock().await = true;
                        }
                    }

                    tracing::debug!(
                        "[{}] Chunk #{}: size={} bytes, total={} bytes",
                        protocol, chunk_count, chunk_size, total_bytes
                    );

                    if !output_chunk.is_empty() {
                        yield Ok::<Bytes, std::io::Error>(output_chunk);
                    }
                    if upstream_error_event {
                        break;
                    }
                }
                Ok(Some(Err(e))) => {
                    tracing::error!(
                        "[{}] Stream error after {} chunks, {} bytes: {}",
                        protocol, chunk_count, total_bytes, e
                    );
                    if !stream_completed_for_stream.load(Ordering::Relaxed) {
                        *stream_failure_for_stream.lock().await =
                            Some(format!("上游流错误: {}", e));
                    }
                    break;
                }
                Ok(None) => {
                    tracing::info!(
                        "[{}] Stream completed normally: {} chunks, {} bytes",
                        protocol, chunk_count, total_bytes
                    );
                    if stream_kind == StreamResponseKind::Json {
                        stream_completed_for_stream.store(true, Ordering::Relaxed);
                    }
                    break;
                }
                Err(_) => {
                    tracing::warn!(
                        "[{}] Stream idle timeout after {} chunks, {} bytes",
                        protocol, chunk_count, total_bytes
                    );
                    if !stream_completed_for_stream.load(Ordering::Relaxed) {
                        *idle_timeout_flag_for_stream.lock().await = true;
                        *stream_failure_for_stream.lock().await =
                            Some("流空闲超时".to_string());
                    }
                    break;
                }
            }
        }

        if stream_kind == StreamResponseKind::Sse
            && !response_encoded
            && !pending_sse.is_empty()
            && !upstream_error_event
        {
            let pending = if let Some(message) = stream_event_error(protocol, &pending_sse) {
                upstream_error_event = true;
                *stream_failure_for_stream.lock().await =
                    Some(format!("上游流错误: {}", message));
                stream_error_event_with_message(protocol, &message)
            } else {
                Bytes::from(std::mem::take(&mut pending_sse))
            };
            if !translating {
                if collected_bytes < MAX_BODY_LOG {
                    let mut chunks = collected_chunks_for_stream.lock().await;
                    let to_collect = pending.len().min(MAX_BODY_LOG - collected_bytes);
                    if to_collect > 0 {
                        chunks.push(pending.slice(..to_collect));
                    }
                    if to_collect < pending.len() {
                        *body_truncated_flag_for_stream.lock().await = true;
                    }
                } else {
                    *body_truncated_flag_for_stream.lock().await = true;
                }
            }
            yield Ok::<Bytes, std::io::Error>(pending);
        }

        if !response_encoded && stream_kind == StreamResponseKind::Sse && !sse_buffer.is_empty() {
            let line = sse_buffer.trim_end_matches('\r');
            if is_stream_completion_line(line, protocol) {
                stream_completed_for_stream.store(true, Ordering::Relaxed);
            }
            let mut usage = stream_usage_for_stream.lock().await;
            parse_streaming_token_usage(line, protocol, &mut usage);
        }

        if stream_kind == StreamResponseKind::Sse
            && !response_encoded
            && !upstream_error_event
            && !stream_completed_for_stream.load(Ordering::Relaxed)
        {
            // 转换流的完成判定在 translated_stream 里已经记录，这里只补错误事件。
            // 防止截断时 translator.finish() 伪造的完成事件被误判。
            let really_done = if translating {
                *stream_really_completed_for_check.lock().await
            } else {
                false
            };
            if !really_done {
                let mut failure = stream_failure_for_stream.lock().await;
                if failure.is_none() {
                    *failure = Some("上游流在完成事件前结束".to_string());
                }
                drop(failure);

                let event = stream_error_event(protocol);
                if !translating && collected_bytes < MAX_BODY_LOG {
                    let mut chunks = collected_chunks_for_stream.lock().await;
                    let to_collect = event.len().min(MAX_BODY_LOG - collected_bytes);
                    if to_collect > 0 {
                        chunks.push(event.slice(..to_collect));
                    }
                    if to_collect < event.len() {
                        *body_truncated_flag_for_stream.lock().await = true;
                    }
                }
                yield Ok::<Bytes, std::io::Error>(event);
            }
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
    let stream_completed_for_log = stream_completed.clone();
    let log_stream_kind = stream_kind;

    tokio::spawn(async move {
        let client_disconnected = stream_end_rx.recv().await.is_none();
        tracing::debug!(
            "[{}] Received stream end notification, client_disconnected={}",
            protocol,
            client_disconnected
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
        let mut stream_completed = stream_completed_for_log.load(Ordering::Relaxed);
        let (usage, body_str) = if response_encoded {
            match try_decompress(&full_body, content_encoding) {
                Some(body) => {
                    stream_completed |= stream_body_has_completion(&body, protocol);
                    let usage = if log_identity.token_usage_enabled {
                        if log_stream_kind == StreamResponseKind::Json {
                            let mut usage = TokenUsage::default();
                            parse_token_usage(&body, protocol, &mut usage);
                            usage
                        } else {
                            parse_streaming_usage_body(&body, protocol)
                        }
                    } else {
                        TokenUsage::default()
                    };
                    (
                        usage,
                        streaming_body_log_text(&body, MAX_BODY_LOG, body_truncated),
                    )
                }
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
        } else if log_stream_kind == StreamResponseKind::Json {
            let mut usage = TokenUsage::default();
            if log_identity.token_usage_enabled {
                parse_token_usage(&full_body, protocol, &mut usage);
            }
            (
                usage,
                streaming_body_log_text(&full_body, MAX_BODY_LOG, body_truncated),
            )
        } else {
            // 转换模式下收集的是原始上游响应，完成判定按上游协议来，但用量要从原始
            // 上游字节解析才能保留 cache_creation。
            stream_completed |= stream_body_has_completion(&full_body, upstream_protocol);
            let usage = if log_identity.token_usage_enabled {
                if translating {
                    parse_streaming_usage_body(&full_body, upstream_protocol)
                } else {
                    stream_usage.lock().await.clone()
                }
            } else {
                TokenUsage::default()
            };
            (
                usage,
                streaming_body_log_text(&full_body, MAX_BODY_LOG, body_truncated),
            )
        };

        let client_cancelled = client_disconnected && !stream_completed;
        tracing::debug!(
            "[{}] Stream completion state: stream_completed={}, client_cancelled={}",
            protocol,
            stream_completed,
            client_cancelled
        );

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
        let stream_failure = stream_failure.lock().await.clone();
        if client_cancelled {
            final_log_info.error_message = Some(CLIENT_CLOSED_REQUEST_MESSAGE.to_string());
        } else if let Some(message) = stream_failure.as_ref() {
            final_log_info.error_message = Some(message.clone());
        } else if is_idle_timeout {
            final_log_info.error_message = Some("流空闲超时".to_string());
        }

        // Record stats
        let elapsed = start_time.elapsed().as_millis() as i64;
        let first_byte_ms = (*first_chunk_ms.lock().await).unwrap_or(first_byte_fallback_ms);
        if !client_cancelled && stream_failure.is_none() && log_is_success {
            record_provider_success(&log_state, log_provider_id, &log_provider_name).await;
        } else if !client_cancelled {
            record_provider_failure(&log_state, log_provider_id).await;
        }

        record_request_stats(
            &log_state,
            &log_identity,
            &log_provider_name,
            log_model_id.as_deref(),
            Some(if client_cancelled {
                CLIENT_CLOSED_REQUEST_STATUS
            } else if stream_failure.is_some() {
                StatusCode::BAD_GATEWAY.as_u16()
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
    upstream_protocol: Protocol,
    tool_shapes: &translate::ToolShapes,
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
    let translating = translate::can_translate(protocol, upstream_protocol);
    let cancel_guard = RequestLogCancelGuard::new(state, request_log_id, start_time);

    // Send request with timeout
    let response =
        match tokio::time::timeout(timeouts.non_stream_timeout, client.execute(request)).await {
            Ok(Ok(resp)) => resp,
            Ok(Err(e)) => {
                tracing::error!(error = %e, "Upstream request failed");
                log_info.error_message = Some(format!("上游请求失败: {}", e));
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
                return Ok(json_error_response(
                    StatusCode::BAD_GATEWAY,
                    "upstream_error",
                    &format!("上游请求失败: {}", e),
                ));
            }
            Err(_) => {
                tracing::error!("Request timeout");
                log_info.error_message = Some("请求超时".to_string());
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
                return Ok(json_error_response(
                    StatusCode::GATEWAY_TIMEOUT,
                    "upstream_timeout",
                    "Request timeout",
                ));
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
    let body_bytes = match tokio::time::timeout(timeouts.non_stream_timeout, response.bytes()).await
    {
        Err(_) => {
            let message = "响应体读取超时".to_string();
            tracing::error!(provider_id, provider = provider_name, error = %message, "Timed out while reading upstream response body");
            log_info.error_message = Some(message);
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
            return Ok(json_error_response(
                StatusCode::GATEWAY_TIMEOUT,
                "upstream_timeout",
                "响应体读取超时",
            ));
        }
        Ok(result) => match result {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::error!(error = %e, "Failed to read response body");
                log_info.error_message = Some(format!("读取响应体失败: {}", e));
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
                return Ok(json_error_response(
                    StatusCode::BAD_GATEWAY,
                    "upstream_body_error",
                    "读取上游响应体失败",
                ));
            }
        },
    };

    // Decompress if needed for logging and token parsing
    let content_encoding = resp_headers
        .get("content-encoding")
        .and_then(|v| v.to_str().ok());
    let decompressed_body = maybe_decompress(&body_bytes, content_encoding);

    // Store response body for logging (use decompressed version)
    log_info.provider_body = Some(truncate_body(&decompressed_body));

    if !is_success {
        let message = upstream_error_message(upstream_protocol, &decompressed_body);
        tracing::warn!(
            provider_id,
            provider = provider_name,
            status = %status,
            error = %message,
            "Upstream returned an HTTP error"
        );
        log_info.error_message = Some(message.clone());
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
        // Pass through real HTTP errors with original status and body
        return Ok(build_passthrough_error_response(status, &decompressed_body, &resp_headers));
    }

    if is_success {
        if let Some(message) = response_body_error(upstream_protocol, &decompressed_body) {
            let message = format!("上游响应错误: {}", message);
            tracing::warn!(
                provider_id,
                provider = provider_name,
                error = %message,
                "Upstream returned a structured error with a successful HTTP status"
            );
            log_info.error_message = Some(message.clone());
            let elapsed = start_time.elapsed().as_millis() as i64;
            record_request_stats(
                state,
                &identity,
                provider_name,
                model_id,
                Some(StatusCode::BAD_GATEWAY.as_u16()),
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
            // 200 with error body: rewrite status to 502 for failover, but pass through original body
            return Ok(build_passthrough_error_response(StatusCode::BAD_GATEWAY, &decompressed_body, &resp_headers));
        }
    }

    // 只转换成功响应；转不动就报 502，交给上层继续故障转移。
    let converted_body = if translating {
        let converted = serde_json::from_slice::<Value>(&decompressed_body)
            .ok()
            .and_then(|value| {
                translate::convert_response(
                    upstream_protocol,
                    protocol,
                    &value,
                    &provider_id.to_string(),
                    tool_shapes,
                )
            })
            .and_then(|value| serde_json::to_vec(&value).ok());
        match converted {
            Some(body) => Some(body),
            None => {
                let message = "上游响应无法转成客户端协议".to_string();
                tracing::warn!(provider_id, provider = provider_name, error = %message, "Upstream response cannot be translated");
                log_info.error_message = Some(message.clone());
                let elapsed = start_time.elapsed().as_millis() as i64;
                record_request_stats(
                    state,
                    &identity,
                    provider_name,
                    model_id,
                    Some(StatusCode::BAD_GATEWAY.as_u16()),
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
                return Ok(json_error_response(
                    StatusCode::BAD_GATEWAY,
                    "translate_failed",
                    &message,
                ));
            }
        }
    } else {
        None
    };

    // Parse token usage (use decompressed body)
    let mut usage = TokenUsage::default();
    if identity.token_usage_enabled {
        parse_token_usage(&decompressed_body, upstream_protocol, &mut usage);
    }

    // Record success. Failure paths have all returned above with an error
    // response, so only the success path reaches here; the failover loop in
    // the caller records failures once per provider per request.
    record_provider_success(state, provider_id, provider_name).await;

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

    builder = if translating {
        // 转换后上游的 content-type / content-length 已经不成立，限流与追踪类的头
        // 照旧带给客户端。
        copy_metadata_headers(builder, &resp_headers).header("content-type", "application/json")
    } else {
        copy_response_headers(builder, &resp_headers)
    };

    let body = match converted_body {
        Some(body) => Body::from(body),
        None => Body::from(body_bytes),
    };

    Ok(builder.body(body).unwrap())
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
        "SELECT id, created_at, finished_at, cli_type, protocol, upstream_protocol, provider_id, profile, provider_name, model_id, status_code, elapsed_ms, first_byte_ms, input_tokens, cache_read_input_tokens, cache_creation_input_tokens, output_tokens, 0.0 as total_cost, price_input_per_m, price_output_per_m, price_cache_read_per_m, price_cache_creation_per_m, price_multiplier, price_tier_threshold, price_source, client_method, client_path, source_model, target_model FROM request_logs WHERE id = ?",
    )
    .bind(log_id)
    .fetch_one(&state.log_db)
    .await;

    if let Ok(mut item) = log_item {
        let (total_cost, breakdown) = crate::services::cost::cost_from_snapshot(
            &item.price,
            item.input_tokens,
            item.cache_read_input_tokens,
            item.cache_creation_input_tokens,
            item.output_tokens,
        );
        item.total_cost = total_cost;
        item.cost = breakdown;
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
        identity.upstream_protocol.map(Protocol::as_str),
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

    // 单价在这里定下来就不再变：日志和按天统计共用同一份快照，之后改倍率或目录价
    // 都只影响新请求，历史费用保持原样。
    let pricing = crate::services::cost::provider_pricing_for_model(
        &state.db,
        Some(identity.provider_id),
        &identity.agent_id,
        provider_name,
        model_id.or(source_model),
    )
    .await
    .unwrap_or_default();
    let price = crate::services::cost::resolve_price_snapshot(
        &pricing,
        usage.input_tokens,
        usage.cache_read_input_tokens,
        usage.cache_creation_input_tokens,
        1,
    );
    let (total_cost, _) = crate::services::cost::cost_from_snapshot(
        &price,
        usage.input_tokens,
        usage.cache_read_input_tokens,
        usage.cache_creation_input_tokens,
        usage.output_tokens,
    );

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
        total_cost,
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
            identity.upstream_protocol.map(Protocol::as_str),
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
            &price,
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
            identity.upstream_protocol.map(Protocol::as_str),
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
            &price,
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
