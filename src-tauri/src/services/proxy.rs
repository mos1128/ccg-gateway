use axum::http::HeaderMap;
use regex::Regex;
use serde_json::Value;
use std::time::Duration;

use crate::db::models::ProviderModelMap;
use crate::services::routing::ProviderWithMaps;

/// Wildcard pattern matching: * matches any characters, ? matches single character
fn wildcard_match(pattern: &str, value: &str) -> bool {
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

/// CLI type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliType {
    ClaudeCode,
    Codex,
    Gemini,
}

impl CliType {
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

/// Token usage tracking
#[derive(Debug, Default, Clone)]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
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

/// Check if request is streaming based on body content
pub fn is_streaming(body: &[u8], path: &str, cli_type: CliType) -> bool {
    match cli_type {
        CliType::ClaudeCode => {
            // Claude uses "stream": true in body
            if let Ok(json) = serde_json::from_slice::<Value>(body) {
                json.get("stream").and_then(|v| v.as_bool()).unwrap_or(false)
            } else {
                false
            }
        }
        CliType::Codex => {
            // Codex uses "stream": true in body
            if let Ok(json) = serde_json::from_slice::<Value>(body) {
                json.get("stream").and_then(|v| v.as_bool()).unwrap_or(false)
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
    let mut result = ModelMappingResult {
        body: body.to_vec(),
        path: path.to_string(),
        source_model: None,
        target_model: None,
    };

    let Ok(mut json) = serde_json::from_slice::<Value>(body) else {
        return result;
    };

    let Some(model) = json.get("model").and_then(|v| v.as_str()).map(|s| s.to_string()) else {
        return result;
    };

    // Always record the source model
    result.source_model = Some(model.clone());

    if provider.model_maps.is_empty() {
        return result;
    }

    // Find matching model map (supports wildcard: * matches any, ? matches single char)
    for map in &provider.model_maps {
        if wildcard_match(&map.source_model, &model) {
            result.target_model = Some(map.target_model.clone());

            // Replace model in body
            if let Some(obj) = json.as_object_mut() {
                obj.insert("model".to_string(), Value::String(map.target_model.clone()));
            }

            if let Ok(new_body) = serde_json::to_vec(&json) {
                result.body = new_body;
            }

            break;
        }
    }

    result
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

    // Extract model from Gemini path: /v1beta/models/{model}:generateContent
    let re = Regex::new(r"/models/([^/:]+)").unwrap();
    let Some(caps) = re.captures(path) else {
        return result;
    };

    let source_model = caps.get(1).map(|m| m.as_str()).unwrap_or("");
    if source_model.is_empty() {
        return result;
    }

    // Always record the source model
    result.source_model = Some(source_model.to_string());

    if model_maps.is_empty() {
        return result;
    }

    // Find matching model map (supports wildcard: * matches any, ? matches single char)
    for map in model_maps {
        if wildcard_match(&map.source_model, source_model) {
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
                if let Some(input) = msg_usage.get("input_tokens").and_then(|v| v.as_i64()) {
                    usage.input_tokens = input;
                }
                if let Some(output) = msg_usage.get("output_tokens").and_then(|v| v.as_i64()) {
                    usage.output_tokens = output;
                }
            } else if let Some(root_usage) = json.get("usage") {
                if let Some(input) = root_usage.get("input_tokens").and_then(|v| v.as_i64()) {
                    usage.input_tokens = input;
                }
                if let Some(output) = root_usage.get("output_tokens").and_then(|v| v.as_i64()) {
                    usage.output_tokens = output;
                }
            }
        }
        CliType::Codex => {
            // Codex format: response.usage in response.completed event
            // Or usage at root for non-streaming
            if let Some(response) = json.get("response") {
                if let Some(resp_usage) = response.get("usage") {
                    if let Some(input) = resp_usage.get("input_tokens").and_then(|v| v.as_i64()) {
                        usage.input_tokens = input;
                    }
                    if let Some(output) = resp_usage.get("output_tokens").and_then(|v| v.as_i64()) {
                        usage.output_tokens = output;
                    }
                }
            } else if let Some(root_usage) = json.get("usage") {
                if let Some(input) = root_usage
                    .get("prompt_tokens")
                    .or_else(|| root_usage.get("input_tokens"))
                    .and_then(|v| v.as_i64())
                {
                    usage.input_tokens = input;
                }
                if let Some(output) = root_usage
                    .get("completion_tokens")
                    .or_else(|| root_usage.get("output_tokens"))
                    .and_then(|v| v.as_i64())
                {
                    usage.output_tokens = output;
                }
            }
        }
        CliType::Gemini => {
            // Gemini format: usageMetadata
            if let Some(metadata) = json.get("usageMetadata") {
                if let Some(prompt) = metadata.get("promptTokenCount").and_then(|v| v.as_i64()) {
                    usage.input_tokens = prompt;
                }
                let candidates = metadata
                    .get("candidatesTokenCount")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let thoughts = metadata
                    .get("thoughtsTokenCount")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                usage.output_tokens = candidates + thoughts;
            }
        }
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
        let name_str = name.as_str().to_lowercase();
        if !FILTERED_HEADERS.contains(&name_str.as_str()) {
            if let Ok(header_name) = reqwest::header::HeaderName::from_bytes(name.as_str().as_bytes())
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
pub fn set_auth_header(
    headers: &mut reqwest::header::HeaderMap,
    api_key: &str,
    cli_type: CliType,
) {
    match cli_type {
        CliType::ClaudeCode => {
            // Claude uses Authorization: Bearer
            if let Ok(value) = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", api_key))
            {
                headers.insert(reqwest::header::AUTHORIZATION, value);
            }
        }
        CliType::Codex => {
            // Codex uses Authorization: Bearer
            if let Ok(value) = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", api_key))
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

/// User-Agent 映射规则
#[derive(Debug, Clone)]
pub struct UseragentMapRule {
    pub source_pattern: String,
    pub target_value: String,
}

/// Apply User-Agent mapping to headers
/// Returns the original User-Agent if mapped for logging purposes
pub fn apply_useragent_mapping(
    headers: &mut reqwest::header::HeaderMap,
    rules: &[UseragentMapRule],
) -> Option<String> {
    if rules.is_empty() {
        return None;
    }

    // Get current User-Agent
    let current_ua = headers
        .get(reqwest::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let Some(ua) = current_ua.as_ref() else {
        return None;
    };

    // Find matching rule using wildcard_match
    for rule in rules {
        if wildcard_match(&rule.source_pattern, ua) {
            // Replace User-Agent with target value
            if let Ok(new_value) = reqwest::header::HeaderValue::from_str(&rule.target_value) {
                headers.insert(reqwest::header::USER_AGENT, new_value);
                return current_ua; // Return original UA for logging
            }
        }
    }

    None
}

/// Build upstream URL from provider base URL and request path
pub fn build_upstream_url(base_url: &str, path: &str, cli_type: CliType) -> String {
    let base = base_url.trim_end_matches('/');

    match cli_type {
        CliType::ClaudeCode => {
            // Claude: base_url + path (path already includes /v1)
            format!("{}{}", base, path)
        }
        CliType::Codex => {
            // Codex: base_url + path
            format!("{}{}", base, path)
        }
        CliType::Gemini => {
            // Gemini: base_url + path
            format!("{}{}", base, path)
        }
    }
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
