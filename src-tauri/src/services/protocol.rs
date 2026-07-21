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
