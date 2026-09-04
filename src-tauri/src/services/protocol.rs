use crate::db::models::{AgentInfo, Protocol};
use axum::http::Method;
use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct ProtocolMatch {
    pub selected: Protocol,
    pub matched_protocols: Vec<Protocol>,
}

fn request_path(path_and_query: &str) -> &str {
    path_and_query
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(path_and_query)
}

pub fn matches_request(protocol: Protocol, method: &Method, path_and_query: &str) -> bool {
    if method != Method::POST {
        return false;
    }
    let path = request_path(path_and_query).trim_end_matches('/');
    match protocol {
        Protocol::AnthropicMessages => matches!(path, "/v1/messages" | "/messages"),
        Protocol::OpenaiChat => {
            matches!(path, "/v1/chat/completions" | "/chat/completions")
        }
        Protocol::OpenaiResponses => matches!(path, "/v1/responses" | "/responses"),
        Protocol::GeminiGenerateContent => {
            static GEMINI_PATH: OnceLock<Regex> = OnceLock::new();
            GEMINI_PATH
                .get_or_init(|| {
                    Regex::new(r"^/(?:v1(?:beta)?/)?models/[^/:]+:(?:stream)?generateContent$")
                        .expect("valid Gemini protocol path")
                })
                .is_match(path)
        }
    }
}

/// 反推一个不匹配任何端点类型的路径该归给哪种上游。
///
/// 辅助端点都挂在主端点下面（`/v1/messages/count_tokens` 是 Anthropic 的，
/// `models/x:countTokens` 是 Gemini 的），按主端点的路径前缀归属就够了。各家都有的
/// 路径（`/models` 列表）归不出来，返回 None 表示随便哪家上游都行。
pub fn infer_protocol(path_and_query: &str) -> Option<Protocol> {
    let path = request_path(path_and_query).trim_end_matches('/');
    // 版本前缀由 join_upstream_url 负责对齐，归属只看后面的业务路径。
    let rest = ["/v1beta", "/v1"]
        .iter()
        .find_map(|prefix| path.strip_prefix(prefix))
        .unwrap_or(path);
    match rest.split('/').nth(1)? {
        "messages" => Some(Protocol::AnthropicMessages),
        "chat" => Some(Protocol::OpenaiChat),
        "responses" => Some(Protocol::OpenaiResponses),
        // Gemini 把方法名写在模型后面（`models/gemini-x:countTokens`），带冒号才是它的
        // 调用形式；光秃秃的 `models` 各家都有，不归属。
        "models" if rest.contains(':') => Some(Protocol::GeminiGenerateContent),
        _ => None,
    }
}

pub fn detect_protocol(
    agent: &AgentInfo,
    method: &Method,
    path_and_query: &str,
) -> Option<ProtocolMatch> {
    let matched_protocols: Vec<_> = agent
        .protocols
        .iter()
        .copied()
        .filter(|protocol| matches_request(*protocol, method, path_and_query))
        .collect();

    let selected = matched_protocols.first().copied()?;
    Some(ProtocolMatch {
        selected,
        matched_protocols,
    })
}
