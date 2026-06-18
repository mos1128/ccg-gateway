use axum::http::HeaderMap;
use regex::Regex;
use serde_json::Value;
use std::time::Duration;

use crate::db::models::{Provider, ProviderModelMap};
use crate::services::routing::{profile_from_gateway_token, ProviderWithMaps, DEFAULT_PROFILE};

pub struct CapturedHeaders {
    pub headers: Vec<(String, String)>,
}

static CLAUDE_HEADERS: std::sync::OnceLock<std::sync::RwLock<CapturedHeaders>> =
    std::sync::OnceLock::new();
static CODEX_HEADERS: std::sync::OnceLock<std::sync::RwLock<CapturedHeaders>> =
    std::sync::OnceLock::new();
static GEMINI_HEADERS: std::sync::OnceLock<std::sync::RwLock<CapturedHeaders>> =
    std::sync::OnceLock::new();

fn extract_safe_headers(headers: &HeaderMap) -> Vec<(String, String)> {
    let mut safe = Vec::new();
    let skip_keys = [
        "host",
        "content-length",
        "content-type",
        "accept",
        "accept-encoding",
        "authorization",
        "x-api-key",
        "x-goog-api-key",
        "connection",
        "keep-alive",
    ];
    for (k, v) in headers.iter() {
        if !skip_keys.iter().any(|h| k.as_str().eq_ignore_ascii_case(h)) {
            if let Ok(val_str) = v.to_str() {
                safe.push((k.as_str().to_owned(), val_str.to_string()));
            }
        }
    }
    safe
}

pub fn get_captured_claude_headers() -> std::sync::RwLockReadGuard<'static, CapturedHeaders> {
    CLAUDE_HEADERS
        .get_or_init(|| {
            std::sync::RwLock::new(CapturedHeaders {
                headers: Vec::new(),
            })
        })
        .read()
        .unwrap()
}

pub fn update_captured_claude_headers(headers: &HeaderMap) {
    if let Some(mut guard) = CLAUDE_HEADERS
        .get_or_init(|| {
            std::sync::RwLock::new(CapturedHeaders {
                headers: Vec::new(),
            })
        })
        .write()
        .ok()
    {
        guard.headers = extract_safe_headers(headers);
    }
}

pub fn get_captured_codex_headers() -> std::sync::RwLockReadGuard<'static, CapturedHeaders> {
    CODEX_HEADERS
        .get_or_init(|| {
            std::sync::RwLock::new(CapturedHeaders {
                headers: Vec::new(),
            })
        })
        .read()
        .unwrap()
}

pub fn update_captured_codex_headers(headers: &HeaderMap) {
    if let Some(mut guard) = CODEX_HEADERS
        .get_or_init(|| {
            std::sync::RwLock::new(CapturedHeaders {
                headers: Vec::new(),
            })
        })
        .write()
        .ok()
    {
        guard.headers = extract_safe_headers(headers);
    }
}

pub fn get_captured_gemini_headers() -> std::sync::RwLockReadGuard<'static, CapturedHeaders> {
    GEMINI_HEADERS
        .get_or_init(|| {
            std::sync::RwLock::new(CapturedHeaders {
                headers: Vec::new(),
            })
        })
        .read()
        .unwrap()
}

pub fn update_captured_gemini_headers(headers: &HeaderMap) {
    if let Some(mut guard) = GEMINI_HEADERS
        .get_or_init(|| {
            std::sync::RwLock::new(CapturedHeaders {
                headers: Vec::new(),
            })
        })
        .write()
        .ok()
    {
        guard.headers = extract_safe_headers(headers);
    }
}

/// Wildcard pattern matching: * matches any characters, ? matches single character
pub fn wildcard_match(pattern: &str, value: &str) -> bool {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let value_chars: Vec<char> = value.chars().collect();

    let mut p_idx = 0usize;
    let mut v_idx = 0usize;
    let mut star_idx: Option<usize> = None;
    let mut match_idx = 0usize;

    while v_idx < value_chars.len() {
        if p_idx < pattern_chars.len()
            && (pattern_chars[p_idx] == value_chars[v_idx] || pattern_chars[p_idx] == '?')
        {
            p_idx += 1;
            v_idx += 1;
        } else if p_idx < pattern_chars.len() && pattern_chars[p_idx] == '*' {
            star_idx = Some(p_idx);
            match_idx = v_idx;
            p_idx += 1;
        } else if let Some(si) = star_idx {
            p_idx = si + 1;
            match_idx += 1;
            v_idx = match_idx;
        } else {
            return false;
        }
    }

    while p_idx < pattern_chars.len() && pattern_chars[p_idx] == '*' {
        p_idx += 1;
    }

    p_idx == pattern_chars.len()
}

/// Extract model name from request body (Claude/Codex)
pub fn extract_model_from_body(body: &[u8]) -> Option<String> {
    let json = serde_json::from_slice::<Value>(body).ok()?;
    json.get("model")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Extract model name from URL path (Gemini)
pub fn extract_model_from_path(path: &str) -> Option<String> {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"/models/([^/:]+)").unwrap());
    let caps = re.captures(path)?;
    caps.get(1).map(|m| m.as_str().to_string())
}

/// CLI type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliType {
    ClaudeCode,
    Codex,
    Gemini,
}

impl CliType {
    pub const ALL: [CliType; 3] = [CliType::ClaudeCode, CliType::Codex, CliType::Gemini];

    pub fn as_str(&self) -> &'static str {
        match self {
            CliType::ClaudeCode => "claude_code",
            CliType::Codex => "codex",
            CliType::Gemini => "gemini",
        }
    }
}

impl std::fmt::Display for CliType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Parse a stored `cli_type` string into a [`CliType`].
///
/// Unknown values fall back to [`CliType::ClaudeCode`]. Most third-party /
/// OpenAI-compatible coding agents also speak the Anthropic Messages protocol,
/// and Anthropic-only ones (e.g. ZCode) require it — so defaulting to the
/// Anthropic family covers the broadest set of providers.
pub fn cli_type_from_str(s: &str) -> CliType {
    match s {
        "claude_code" => CliType::ClaudeCode,
        "codex" => CliType::Codex,
        "gemini" => CliType::Gemini,
        _ => CliType::ClaudeCode,
    }
}

/// Token usage tracking
#[derive(Debug, Default, Clone)]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub cache_read_input_tokens: i64,
    pub cache_creation_input_tokens: i64,
    pub output_tokens: i64,
    gemini_prompt_tokens: i64,
    gemini_tool_use_prompt_tokens: i64,
    gemini_candidates_token_count: i64,
    gemini_thoughts_token_count: i64,
}

/// Detect CLI type from User-Agent header
pub fn detect_cli_type(headers: &HeaderMap) -> CliType {
    let ua = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    if ua.contains("codex") || ua.contains("openai") {
        CliType::Codex
    } else if ua.contains("gemini") || ua.contains("google") {
        CliType::Gemini
    } else {
        CliType::ClaudeCode
    }
}

fn bearer_token(value: &str) -> &str {
    let value = value.trim();
    if let Some((scheme, token)) = value.split_once(' ') {
        if scheme.eq_ignore_ascii_case("bearer") {
            return token.trim();
        }
    }
    value
}

/// Detect provider profile from the gateway token sent by the CLI.
pub fn detect_gateway_profile(headers: &HeaderMap) -> String {
    let header_names = ["authorization", "x-goog-api-key"];

    for name in header_names {
        if let Some(value) = headers.get(name).and_then(|v| v.to_str().ok()) {
            if let Some(profile) = profile_from_gateway_token(bearer_token(value)) {
                return profile;
            }
        }
    }

    DEFAULT_PROFILE.to_string()
}

/// Check if request is streaming based on body content
pub fn is_streaming(body: &[u8], path: &str, cli_type: CliType) -> bool {
    match cli_type {
        CliType::ClaudeCode => {
            // Claude uses "stream": true in body
            if let Ok(json) = serde_json::from_slice::<Value>(body) {
                json.get("stream")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            } else {
                false
            }
        }
        CliType::Codex => {
            // Codex uses "stream": true in body
            if let Ok(json) = serde_json::from_slice::<Value>(body) {
                json.get("stream")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            } else {
                false
            }
        }
        CliType::Gemini => {
            // Gemini uses streamGenerateContent in path
            path.contains("streamGenerateContent")
        }
    }
}

/// Model mapping result
pub struct ModelMappingResult {
    pub body: Vec<u8>,
    pub path: String,
    pub source_model: Option<String>,
    pub target_model: Option<String>,
}

/// Apply model mapping for body-based APIs (Claude, Codex)
pub fn apply_body_model_mapping(
    provider: &ProviderWithMaps,
    body: &[u8],
    path: &str,
) -> ModelMappingResult {
    let Some(model) = extract_model_from_body(body) else {
        return ModelMappingResult {
            body: body.to_vec(),
            path: path.to_string(),
            source_model: None,
            target_model: None,
        };
    };

    if provider.model_maps.is_empty() {
        return ModelMappingResult {
            body: body.to_vec(),
            path: path.to_string(),
            source_model: Some(model),
            target_model: None,
        };
    }

    // Find matching model map (supports wildcard: * matches any, ? matches single char)
    for map in &provider.model_maps {
        if wildcard_match(&map.source_model, &model) {
            // Only clone and modify body when a mapping is found
            let mapped_body = if let Ok(mut json) = serde_json::from_slice::<Value>(body) {
                if let Some(obj) = json.as_object_mut() {
                    obj.insert("model".to_string(), Value::String(map.target_model.clone()));
                }
                serde_json::to_vec(&json).unwrap_or_else(|_| body.to_vec())
            } else {
                body.to_vec()
            };

            return ModelMappingResult {
                body: mapped_body,
                path: path.to_string(),
                source_model: Some(model),
                target_model: Some(map.target_model.clone()),
            };
        }
    }

    // No mapping matched
    ModelMappingResult {
        body: body.to_vec(),
        path: path.to_string(),
        source_model: Some(model),
        target_model: None,
    }
}

/// Apply model mapping for URL-based APIs (Gemini)
pub fn apply_url_model_mapping(
    _provider: &ProviderWithMaps,
    path: &str,
    model_maps: &[ProviderModelMap],
) -> ModelMappingResult {
    let mut result = ModelMappingResult {
        body: vec![],
        path: path.to_string(),
        source_model: None,
        target_model: None,
    };

    let Some(source_model) = extract_model_from_path(path) else {
        return result;
    };

    // Always record the source model
    result.source_model = Some(source_model.clone());

    if model_maps.is_empty() {
        return result;
    }

    // Find matching model map (supports wildcard: * matches any, ? matches single char)
    for map in model_maps {
        if wildcard_match(&map.source_model, &source_model) {
            result.target_model = Some(map.target_model.clone());

            // Replace model in path
            result.path = path.replace(
                &format!("/models/{}", source_model),
                &format!("/models/{}", map.target_model),
            );

            break;
        }
    }

    result
}

/// Parse token usage from response data
pub fn parse_token_usage(data: &[u8], cli_type: CliType, usage: &mut TokenUsage) {
    let Ok(json) = serde_json::from_slice::<Value>(data) else {
        return;
    };

    match cli_type {
        CliType::ClaudeCode => {
            // Claude format: message.usage or usage at root
            if let Some(msg_usage) = json.get("message").and_then(|m| m.get("usage")) {
                apply_claude_usage(msg_usage, usage);
            } else if let Some(root_usage) = json.get("usage") {
                apply_claude_usage(root_usage, usage);
            }
        }
        CliType::Codex => {
            // Codex format: response.usage in response.completed event
            // Or usage at root for non-streaming
            if let Some(response) = json.get("response") {
                if let Some(resp_usage) = response.get("usage") {
                    apply_openai_usage(resp_usage, usage);
                }
            } else if let Some(root_usage) = json.get("usage") {
                apply_openai_usage(root_usage, usage);
            }
        }
        CliType::Gemini => {
            // Gemini format: usageMetadata
            if let Some(metadata) = json.get("usageMetadata") {
                apply_gemini_usage(metadata, usage);
            }
        }
    }
}

fn apply_i64(value: &Value, key: &str, target: &mut i64) -> bool {
    if let Some(next) = value.get(key).and_then(|v| v.as_i64()) {
        *target = next;
        return true;
    }
    false
}

fn apply_claude_usage(value: &Value, usage: &mut TokenUsage) {
    apply_i64(value, "input_tokens", &mut usage.input_tokens);
    apply_i64(
        value,
        "cache_read_input_tokens",
        &mut usage.cache_read_input_tokens,
    );
    apply_i64(
        value,
        "cache_creation_input_tokens",
        &mut usage.cache_creation_input_tokens,
    );
    apply_i64(value, "output_tokens", &mut usage.output_tokens);
}

fn apply_openai_usage(value: &Value, usage: &mut TokenUsage) {
    let cached_updated = value
        .get("input_tokens_details")
        .or_else(|| value.get("prompt_tokens_details"))
        .and_then(|details| details.get("cached_tokens"))
        .and_then(|v| v.as_i64())
        .map(|cached| {
            usage.cache_read_input_tokens = cached;
        })
        .is_some();

    if let Some(input) = value
        .get("input_tokens")
        .or_else(|| value.get("prompt_tokens"))
        .and_then(|v| v.as_i64())
    {
        usage.input_tokens = (input - usage.cache_read_input_tokens).max(0);
    } else if cached_updated {
        usage.input_tokens = usage.input_tokens.max(0);
    }

    if let Some(output) = value
        .get("output_tokens")
        .or_else(|| value.get("completion_tokens"))
        .and_then(|v| v.as_i64())
    {
        usage.output_tokens = output;
    }
}

fn apply_gemini_usage(value: &Value, usage: &mut TokenUsage) {
    let prompt_updated = apply_i64(value, "promptTokenCount", &mut usage.gemini_prompt_tokens);
    let cached_updated = apply_i64(
        value,
        "cachedContentTokenCount",
        &mut usage.cache_read_input_tokens,
    );
    let tool_updated = apply_i64(
        value,
        "toolUsePromptTokenCount",
        &mut usage.gemini_tool_use_prompt_tokens,
    );

    if prompt_updated || cached_updated || tool_updated {
        usage.input_tokens = (usage.gemini_prompt_tokens - usage.cache_read_input_tokens
            + usage.gemini_tool_use_prompt_tokens)
            .max(0);
    }

    let candidates_updated = apply_i64(
        value,
        "candidatesTokenCount",
        &mut usage.gemini_candidates_token_count,
    );
    let thoughts_updated = apply_i64(
        value,
        "thoughtsTokenCount",
        &mut usage.gemini_thoughts_token_count,
    );

    if candidates_updated || thoughts_updated {
        usage.output_tokens =
            usage.gemini_candidates_token_count + usage.gemini_thoughts_token_count;
    }
}

/// Parse token usage from SSE streaming data
pub fn parse_streaming_token_usage(line: &str, cli_type: CliType, usage: &mut TokenUsage) {
    // SSE format: data: {...}
    let data = if let Some(stripped) = line.strip_prefix("data: ") {
        stripped
    } else if let Some(stripped) = line.strip_prefix("data:") {
        stripped
    } else {
        return;
    };

    if data.trim() == "[DONE]" {
        return;
    }

    parse_token_usage(data.as_bytes(), cli_type, usage);
}

/// Headers to filter out when forwarding requests
const FILTERED_HEADERS: &[&str] = &[
    "host",
    "connection",
    "keep-alive",
    "transfer-encoding",
    "te",
    "trailer",
    "upgrade",
    "content-length",
    "proxy-connection",
    "proxy-authenticate",
    "proxy-authorization",
];

/// Filter headers for forwarding
pub fn filter_headers(headers: &HeaderMap) -> reqwest::header::HeaderMap {
    let mut filtered = reqwest::header::HeaderMap::new();

    for (name, value) in headers.iter() {
        if !FILTERED_HEADERS.iter().any(|h| name.as_str().eq_ignore_ascii_case(h)) {
            if let Ok(header_name) =
                reqwest::header::HeaderName::from_bytes(name.as_str().as_bytes())
            {
                if let Ok(header_value) = reqwest::header::HeaderValue::from_bytes(value.as_bytes())
                {
                    filtered.insert(header_name, header_value);
                }
            }
        }
    }

    filtered
}

/// Set authentication header based on CLI type
pub fn set_auth_header(headers: &mut reqwest::header::HeaderMap, api_key: &str, cli_type: CliType) {
    match cli_type {
        CliType::ClaudeCode => {
            // Claude uses Authorization: Bearer
            if let Ok(value) =
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", api_key))
            {
                headers.insert(reqwest::header::AUTHORIZATION, value);
            }
        }
        CliType::Codex => {
            // Codex uses Authorization: Bearer
            if let Ok(value) =
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", api_key))
            {
                headers.insert(reqwest::header::AUTHORIZATION, value);
            }
        }
        CliType::Gemini => {
            // Gemini uses x-goog-api-key
            if let Ok(value) = reqwest::header::HeaderValue::from_str(api_key) {
                headers.insert("x-goog-api-key", value);
            }
        }
    }
}

/// Apply User-Agent override to headers
/// If custom_ua is provided, replaces the User-Agent header with it.
pub fn apply_useragent_override(headers: &mut reqwest::header::HeaderMap, custom_ua: Option<&str>) {
    let Some(target_ua) = custom_ua.filter(|s| !s.is_empty()) else {
        return;
    };

    if let Ok(new_value) = reqwest::header::HeaderValue::from_str(target_ua) {
        headers.insert(reqwest::header::USER_AGENT, new_value);
    }
}

// ---------------------------------------------------------------------------
// Agent probe templates
//
// Used by the "test provider" flow to emulate a real CLI request when no real
// Agent traffic is available (cold start). These are best-effort defaults: the
// captured-header mechanism (update_captured_*_headers) can override the UA /
// anthropic-beta at runtime to stay closer to the currently installed CLI, but
// these defaults must remain self-sufficient so testing works before any real
// request has passed through the gateway.
//
// Centralized here so version bumps are a single, visible change.
// ---------------------------------------------------------------------------

/// Default User-Agent strings per CLI type.
pub fn default_user_agent(cli_type: CliType) -> &'static str {
    match cli_type {
        CliType::ClaudeCode => "claude-cli/2.1.121 (external, cli)",
        CliType::Codex => "codex-tui/0.125.0 (Windows 10.0.22631; x86_64) unknown (codex-tui; 0.125.0)",
        CliType::Gemini => "GeminiCLI/0.39.1/gemini-3.1-pro-preview (win32; x64; terminal)",
    }
}

/// Default `anthropic-beta` value for Claude Code probes.
pub const DEFAULT_ANTHROPIC_BETA: &str = "claude-code-20250219,context-1m-2025-08-07,interleaved-thinking-2025-05-14,redact-thinking-2026-02-12,context-management-2025-06-27,prompt-caching-scope-2026-01-05,advanced-tool-use-2025-11-20,effort-2025-11-24";

/// Probe request parts emitted by the template for a given CLI type + model.
///
/// `path` is the suffix appended to the provider base_url.
/// `body` is the JSON body bytes.
/// `extra_headers` are Agent-specific headers (UA, anthropic-beta, x-app, ...)
/// the caller should merge into its client header map before invoking
/// [`build_upstream_request`]. UA set here is the default and may be overridden
/// by captured headers; `custom_useragent` is applied later by
/// `build_upstream_request` and always wins.
pub struct ProbeRequest {
    pub path: String,
    pub body: Vec<u8>,
    pub extra_headers: Vec<(&'static str, String)>,
}

/// Build a probe request emulating the given CLI type.
pub fn build_probe_request(cli_type: CliType, model: &str) -> ProbeRequest {
    match cli_type {
        CliType::ClaudeCode => {
            let body = serde_json::json!({
                "model": model,
                "messages": [{"role": "user", "content": [{"type": "text", "text": "今天天气不错"}]}],
                "system": [{"type": "text", "text": "You are Claude Code, Anthropic's official CLI for Claude."}],
                "max_tokens": 1024,
                "thinking": {"type": "adaptive"},
                "stream": true
            });
            ProbeRequest {
                path: "/v1/messages".to_string(),
                body: serde_json::to_vec(&body).unwrap_or_default(),
                extra_headers: vec![
                    ("user-agent", default_user_agent(cli_type).to_string()),
                    ("anthropic-beta", DEFAULT_ANTHROPIC_BETA.to_string()),
                    ("x-app", "cli".to_string()),
                    ("accept", "application/json".to_string()),
                    ("accept-encoding", "gzip, deflate, br, zstd".to_string()),
                ],
            }
        }
        CliType::Codex => {
            let body = serde_json::json!({
                "model": model,
                "instructions": "You are Codex, a coding agent based on GPT-5.",
                "input": [{"type": "message", "role": "user", "content": [{"type": "input_text", "text": "今天天气不错"}]}],
                "reasoning": {"effort": "high"},
                "stream": true
            });
            ProbeRequest {
                path: "/responses".to_string(),
                body: serde_json::to_vec(&body).unwrap_or_default(),
                extra_headers: vec![
                    ("user-agent", default_user_agent(cli_type).to_string()),
                    ("originator", "codex-tui".to_string()),
                    ("accept", "text/event-stream".to_string()),
                    ("accept-encoding", "identity".to_string()),
                ],
            }
        }
        CliType::Gemini => {
            let body = serde_json::json!({
                "contents": [{"role": "user", "parts": [{"text": "今天天气不错"}]}],
                "systemInstruction": {"parts": [{"text": "You are Gemini CLI, an interactive CLI agent specializing in software engineering tasks."}]},
                "generationConfig": {"temperature": 1.0, "topP": 0.95, "topK": 64, "thinkingConfig": {"includeThoughts": true}}
            });
            ProbeRequest {
                path: format!("/v1beta/models/{}:streamGenerateContent?alt=sse", model),
                body: serde_json::to_vec(&body).unwrap_or_default(),
                extra_headers: vec![
                    ("user-agent", default_user_agent(cli_type).to_string()),
                    ("accept", "*/*".to_string()),
                    ("accept-encoding", "gzip, deflate".to_string()),
                ],
            }
        }
    }
}

/// Build the upstream `reqwest::Request` shared by the real proxy path and the
/// test-provider path.
///
/// Single source of truth for: hop-by-hop filtering, auth injection, and
/// per-provider User-Agent override. Real forwarding passes the Agent's
/// original headers; testing passes a synthetic header map built from the
/// probe template (optionally refined by captured headers).
pub fn build_upstream_request(
    client: &reqwest::Client,
    provider: &Provider,
    cli_type: CliType,
    upstream_url: &str,
    client_headers: &HeaderMap,
    body: Vec<u8>,
    method: reqwest::Method,
) -> Result<reqwest::Request, reqwest::Error> {
    let mut req_headers = filter_headers(client_headers);
    set_auth_header(&mut req_headers, &provider.api_key, cli_type);
    apply_useragent_override(&mut req_headers, provider.custom_useragent.as_deref());

    // Content-Length is intentionally not set: filter_headers stripped the
    // original value, and reqwest recomputes it from the body length on build.
    let mut builder = client.request(method, upstream_url);
    builder = builder.headers(req_headers);
    if !body.is_empty() {
        builder = builder.body(body);
    }
    builder.build()
}

/// Timeout configuration
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    pub first_byte_timeout: Duration,
    pub idle_timeout: Duration,
    pub non_stream_timeout: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            first_byte_timeout: Duration::from_secs(60),
            idle_timeout: Duration::from_secs(30),
            non_stream_timeout: Duration::from_secs(120),
        }
    }
}

impl TimeoutConfig {
    pub fn from_db(
        stream_first_byte_timeout: i64,
        stream_idle_timeout: i64,
        non_stream_timeout: i64,
    ) -> Self {
        Self {
            first_byte_timeout: Duration::from_secs(stream_first_byte_timeout as u64),
            idle_timeout: Duration::from_secs(stream_idle_timeout as u64),
            non_stream_timeout: Duration::from_secs(non_stream_timeout as u64),
        }
    }
}
