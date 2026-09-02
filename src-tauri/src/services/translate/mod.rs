//! Anthropic Messages / OpenAI Chat / OpenAI Responses 三方互转。
//!
//! 客户端协议与服务商端点类型不一致时，请求走「客户端格式 -> IR -> 上游格式」，
//! 响应反向再走一遍。Gemini 不参与转换，仍然只能原样透传。

pub mod anthropic;
pub mod ir;
pub mod openai_chat;
pub mod openai_responses;

use serde_json::Value;

use crate::db::models::Protocol;
use ir::{ChatRequest, ChatResponse, StreamEvent};
pub use ir::has_envelope;
pub use ir::DEFAULT_MAX_TOKENS;
pub use openai_responses::ToolShapes;

/// 关掉 Opaque 回放用的上游标识：任何信封都配不上它。
pub const NO_REPLAY: &str = "\0";

/// 缓冲上限。这块缓冲同时兼作「请求写了 stream、上游却回一整块 JSON」时的容器，
/// 所以比透传那条路的单帧上限（`MAX_PENDING_EVENT_BYTES`，2MB）再宽一档。超限说明
/// 上游发的不是能解析的流，放弃转换并报错——绝不能把半截内容当成完整帧吐出去。
const MAX_BUFFER_BYTES: usize = 4 * 1024 * 1024;

/// 通用保活帧。三家协议都把 `:` 开头的注释行当成无内容跳过，所以它在任何目标协议、
/// 任何时点（包括还没发过首个事件时）都能安全发出去。
const KEEPALIVE_COMMENT: &str = ": ccg-keepalive\n\n";

/// 一条 SSE 帧。`event` 为 None 时只输出 data 行（OpenAI Chat 的风格）。
pub struct SseFrame {
    pub event: Option<String>,
    pub data: String,
}

impl SseFrame {
    pub fn render(&self) -> String {
        match &self.event {
            Some(name) => format!("event: {}\ndata: {}\n\n", name, self.data),
            None => format!("data: {}\n\n", self.data),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Anthropic,
    Chat,
    Responses,
}

impl Format {
    pub fn from_protocol(protocol: Protocol) -> Option<Format> {
        match protocol {
            Protocol::AnthropicMessages => Some(Format::Anthropic),
            Protocol::OpenaiChat => Some(Format::Chat),
            Protocol::OpenaiResponses => Some(Format::Responses),
            Protocol::GeminiGenerateContent => None,
        }
    }

    pub fn path(self) -> &'static str {
        match self {
            Format::Anthropic => "/v1/messages",
            Format::Chat => "/v1/chat/completions",
            Format::Responses => "/v1/responses",
        }
    }

    /// Opaque 信封里记录来源协议用的短名。
    pub fn name(self) -> &'static str {
        match self {
            Format::Anthropic => "anthropic",
            Format::Chat => "chat",
            Format::Responses => "responses",
        }
    }

    pub fn from_name(name: &str) -> Option<Format> {
        match name {
            "anthropic" => Some(Format::Anthropic),
            "chat" => Some(Format::Chat),
            "responses" => Some(Format::Responses),
            _ => None,
        }
    }
}

pub fn is_convertible(protocol: Protocol) -> bool {
    Format::from_protocol(protocol).is_some()
}

/// 两边都在可转换集合里、且确实不同，才需要转换。
pub fn can_translate(from: Protocol, to: Protocol) -> bool {
    from != to && is_convertible(from) && is_convertible(to)
}

/// 转换时上游路径按目标协议改写，服务商配的 base_url 不变。
pub fn upstream_path(protocol: Protocol) -> Option<&'static str> {
    Format::from_protocol(protocol).map(Format::path)
}

fn parse_request(format: Format, body: &Value) -> ChatRequest {
    match format {
        Format::Anthropic => anthropic::parse_request(body),
        Format::Chat => openai_chat::parse_request(body),
        Format::Responses => openai_responses::parse_request(body),
    }
}

fn build_request(
    format: Format,
    request: &ChatRequest,
    key: &str,
    default_max_tokens: i64,
) -> Value {
    match format {
        Format::Anthropic => anthropic::build_request(request, key, default_max_tokens),
        Format::Chat => openai_chat::build_request(request),
        Format::Responses => openai_responses::build_request(request, key),
    }
}

fn parse_response(format: Format, body: &Value, key: &str) -> ChatResponse {
    match format {
        Format::Anthropic => anthropic::parse_response(body, key),
        Format::Chat => openai_chat::parse_response(body),
        Format::Responses => openai_responses::parse_response(body, key),
    }
}

fn build_response(format: Format, response: &ChatResponse, shapes: &ToolShapes) -> Value {
    match format {
        Format::Anthropic => anthropic::build_response(response),
        Format::Chat => openai_chat::build_response(response),
        Format::Responses => openai_responses::build_response(response, shapes),
    }
}

/// 客户端请求里带的私有工具形状。只有 Responses 客户端可能带，其余协议不必解析。
pub fn tool_shapes(protocol: Protocol, body: &[u8]) -> ToolShapes {
    if !matches!(Format::from_protocol(protocol), Some(Format::Responses)) {
        return ToolShapes::new();
    }
    serde_json::from_slice::<Value>(body)
        .map(|body| openai_responses::tool_shapes(&body))
        .unwrap_or_default()
}

/// 客户端请求体 -> 上游请求体。`key` 是本次上游的标识（服务商 id），只有客户端
/// 回传的 Opaque 载荷确实来自同一个上游才会被回放。`default_max_tokens` 用于源请求
/// 没给 max_tokens、目标协议又必须要（Anthropic）的场合。
pub fn convert_request(
    from: Protocol,
    to: Protocol,
    body: &Value,
    key: &str,
    default_max_tokens: i64,
) -> Option<Value> {
    let (from, to) = (Format::from_protocol(from)?, Format::from_protocol(to)?);
    Some(build_request(
        to,
        &parse_request(from, body),
        key,
        default_max_tokens,
    ))
}

/// 上游响应体 -> 客户端响应体。`shapes` 来自客户端请求，用于还原私有工具调用。
pub fn convert_response(
    from: Protocol,
    to: Protocol,
    body: &Value,
    key: &str,
    shapes: &ToolShapes,
) -> Option<Value> {
    let (from, to) = (Format::from_protocol(from)?, Format::from_protocol(to)?);
    Some(build_response(to, &parse_response(from, body, key), shapes))
}

/// 直通路径（客户端与上游同协议、不走转换）上清掉历史里失效的思考内容。两种触发：
/// 事后整流要把全部思考块剥掉重试；平时也得剥掉网关自己写进去的 Opaque 信封——那是
/// 上一轮走过转换留下的，原样发给原生上游一律验不过。返回是否真的改动过。
pub fn strip_thinking(protocol: Protocol, body: &mut Value, drop_all: bool) -> bool {
    match Format::from_protocol(protocol) {
        Some(Format::Anthropic) => anthropic::strip_thinking(body, drop_all),
        Some(Format::Chat) => openai_chat::strip_thinking(body, drop_all),
        Some(Format::Responses) => openai_responses::strip_thinking(body, drop_all),
        None => false,
    }
}

enum Decoder {
    Anthropic(anthropic::Decoder),
    Chat(openai_chat::Decoder),
    Responses(openai_responses::Decoder),
}

impl Decoder {
    fn new(format: Format, key: &str) -> Decoder {
        match format {
            Format::Anthropic => Decoder::Anthropic(anthropic::Decoder::new(key)),
            Format::Chat => Decoder::Chat(openai_chat::Decoder::default()),
            Format::Responses => Decoder::Responses(openai_responses::Decoder::new(key)),
        }
    }

    fn push(&mut self, name: &str, data: &Value) -> Vec<StreamEvent> {
        match self {
            Decoder::Anthropic(inner) => inner.push(name, data),
            Decoder::Chat(inner) => inner.push(name, data),
            Decoder::Responses(inner) => inner.push(name, data),
        }
    }

    fn finish(&mut self) -> Vec<StreamEvent> {
        match self {
            Decoder::Anthropic(inner) => inner.finish(),
            Decoder::Chat(inner) => inner.finish(),
            Decoder::Responses(inner) => inner.finish(),
        }
    }

    /// 读到 `data: [DONE]`。只有 Chat 把它当终止标记——另外两家各有自己的终止事件，
    /// 收到 `[DONE]` 不代表协议层正常收尾。
    fn mark_done(&mut self) {
        if let Decoder::Chat(inner) = self {
            inner.mark_done();
        }
    }
}

enum Encoder {
    Anthropic(anthropic::Encoder),
    Chat(openai_chat::Encoder),
    Responses(openai_responses::Encoder),
}

impl Encoder {
    fn new(format: Format, shapes: &ToolShapes) -> Encoder {
        match format {
            Format::Anthropic => Encoder::Anthropic(anthropic::Encoder::default()),
            Format::Chat => Encoder::Chat(openai_chat::Encoder::default()),
            Format::Responses => Encoder::Responses(openai_responses::Encoder::new(shapes)),
        }
    }

    fn encode(&mut self, event: &StreamEvent) -> Vec<SseFrame> {
        match self {
            Encoder::Anthropic(inner) => inner.encode(event),
            Encoder::Chat(inner) => inner.encode(event),
            Encoder::Responses(inner) => inner.encode(event),
        }
    }

    /// 上游发来的帧翻译不出任何客户端事件时补一帧保活，免得客户端或网关自己把
    /// 等待判成空闲超时。Anthropic 有标准的 ping 事件，但它必须排在 message_start
    /// 之后；还没开始（以及其余两家）就用 SSE 注释行顶上。
    fn keepalive(&self) -> String {
        match self {
            Encoder::Anthropic(inner) => inner
                .keepalive()
                .map(|frame| frame.render())
                .unwrap_or_else(|| KEEPALIVE_COMMENT.to_string()),
            Encoder::Chat(_) | Encoder::Responses(_) => KEEPALIVE_COMMENT.to_string(),
        }
    }
}

/// 找一帧的结束位置，返回 (帧内容长度, 下一帧起点)。
fn frame_boundary(buffer: &[u8]) -> Option<(usize, usize)> {
    (0..buffer.len()).find_map(|start| {
        let rest = &buffer[start..];
        if rest.starts_with(b"\r\n\r\n") {
            Some((start, start + 4))
        } else if rest.starts_with(b"\n\n") {
            Some((start, start + 2))
        } else {
            None
        }
    })
}

/// 取出一帧里的 event 名与拼好的 data。注释行（以 `:` 开头）自然被跳过。
fn decode_frame(block: &str) -> (String, String) {
    let mut event = String::new();
    let mut data = String::new();
    for line in block.lines() {
        if let Some(rest) = line.strip_prefix("event:") {
            event = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest.trim());
        }
    }
    (event, data)
}

/// 上游 SSE 字节流 -> 客户端协议 SSE 字节流。逐帧解码成 IR 事件再编码，帧不完整
/// 就留在缓冲区等下一个 chunk。
pub struct StreamTranslator {
    source: Format,
    key: String,
    decoder: Decoder,
    encoder: Encoder,
    buffer: Vec<u8>,
    decoded: bool,
    done: bool,
}

impl StreamTranslator {
    pub fn new(
        from: Protocol,
        to: Protocol,
        key: &str,
        shapes: &ToolShapes,
    ) -> Option<StreamTranslator> {
        let source = Format::from_protocol(from)?;
        let target = Format::from_protocol(to)?;
        Some(StreamTranslator {
            source,
            key: key.to_string(),
            decoder: Decoder::new(source, key),
            encoder: Encoder::new(target, shapes),
            buffer: Vec::new(),
            decoded: false,
            done: false,
        })
    }

    pub fn push(&mut self, chunk: &[u8]) -> String {
        if self.done {
            return String::new();
        }
        self.buffer.extend_from_slice(chunk);
        let mut out = String::new();
        while let Some((end, next)) = frame_boundary(&self.buffer) {
            let block = String::from_utf8_lossy(&self.buffer[..end]).into_owned();
            self.buffer.drain(..next);
            out.push_str(&self.handle(&block));
        }
        if self.buffer.len() > MAX_BUFFER_BYTES {
            self.buffer.clear();
            out.push_str(&self.error("上游流超过缓冲上限且没有可解析的帧边界"));
        }
        out
    }

    /// 上游一帧能解码的 SSE 都没发过、翻译器也还没收尾时，缓冲里剩的字节就是上游当成
    /// 响应体发来的那一整块 JSON。调用方要在 [`Self::finish`] 之前拿它按**上游**协议
    /// 判错：`finish` 会把它摊成事件，错误体也一样会被摊成「空内容 + 正常结束」的假
    /// 成功。判错只能看这一段，不能看整条流的字节——上游先发过保活注释的话，拼起来
    /// 就不是一份合法 JSON 了。
    pub fn pending_body(&self) -> Option<&[u8]> {
        (!self.decoded && !self.done).then_some(self.buffer.as_slice())
    }

    /// 直接向客户端发一个错误事件并终止转换。判错本身由调用方按上游协议做。
    pub fn error(&mut self, message: &str) -> String {
        if self.done {
            return String::new();
        }
        self.done = true;
        self.render(&[StreamEvent::Error {
            message: message.to_string(),
        }])
    }

    /// 上游流结束：处理残留的最后一帧，补齐解码器还没吐完的事件。
    pub fn finish(&mut self) -> String {
        let tail = String::from_utf8_lossy(&std::mem::take(&mut self.buffer)).into_owned();
        let mut out = String::new();
        if !tail.trim().is_empty() {
            out.push_str(&self.handle(&tail));
        }
        // 请求写了 stream 但上游回了完整 JSON，摊成事件再编码。
        if !self.decoded && !self.done {
            if let Ok(body) = serde_json::from_str::<Value>(tail.trim()) {
                let response = parse_response(self.source, &body, &self.key);
                // 一个内容块都没有：错误体或空壳。摊出去就是一次「空内容 + 正常结束」，
                // 客户端和网关双双当成功。宁可让客户端看到错误，也不能把失败记成成功。
                if response.blocks.is_empty() {
                    out.push_str(&self.error("上游在流式请求下返回了没有内容的响应体"));
                    return out;
                }
                let events = ir::response_to_events(&response);
                out.push_str(&self.render(&events));
                self.done = true;
                return out;
            }
        }
        out.push_str(&self.flush());
        out
    }

    fn handle(&mut self, block: &str) -> String {
        if self.done {
            return String::new();
        }
        let (name, data) = decode_frame(block);
        if data.is_empty() {
            return self.keepalive();
        }
        if data == "[DONE]" {
            self.decoder.mark_done();
            return self.flush();
        }
        let Ok(value) = serde_json::from_str::<Value>(&data) else {
            return self.keepalive();
        };
        self.decoded = true;
        let events = self.decoder.push(&name, &value);
        let out = self.render(&events);
        if out.is_empty() {
            return self.keepalive();
        }
        out
    }

    fn keepalive(&self) -> String {
        if self.done {
            return String::new();
        }
        self.encoder.keepalive()
    }

    fn flush(&mut self) -> String {
        if self.done {
            return String::new();
        }
        self.done = true;
        let events = self.decoder.finish();
        self.render(&events)
    }

    fn render(&mut self, events: &[StreamEvent]) -> String {
        let mut out = String::new();
        for event in events {
            for frame in self.encoder.encode(event) {
                out.push_str(&frame.render());
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::ir::{array, str_field, DEFAULT_MAX_TOKENS};
    use super::*;
    use crate::services::proxy::{parse_token_usage, TokenUsage};
    use serde_json::json;

    const ANTHROPIC: Protocol = Protocol::AnthropicMessages;
    const CHAT: Protocol = Protocol::OpenaiChat;
    const RESPONSES: Protocol = Protocol::OpenaiResponses;

    /// Anthropic 的硬性结构要求，违反任何一条都是 400。
    fn assert_anthropic_shape(body: &Value) {
        let messages = body["messages"].as_array().expect("messages 必须是数组");
        assert!(!messages.is_empty(), "messages 不能为空");
        assert_eq!(messages[0]["role"], json!("user"), "首条必须是 user");
        let declared: Vec<String> = array(body.get("tools"))
            .iter()
            .map(|tool| str_field(tool, "name"))
            .collect();
        let mut previous_calls: Vec<String> = Vec::new();
        for (index, message) in messages.iter().enumerate() {
            let role = str_field(message, "role");
            if index > 0 {
                assert_ne!(
                    role,
                    str_field(&messages[index - 1], "role"),
                    "第 {index} 条与上一条同角色，Anthropic 要求交替"
                );
            }
            let blocks = array(message.get("content"));
            assert!(!blocks.is_empty(), "第 {index} 条 content 为空");
            let mut calls = Vec::new();
            for block in blocks {
                match str_field(block, "type").as_str() {
                    "tool_use" => {
                        let name = str_field(block, "name");
                        assert!(declared.contains(&name), "tool_use {name} 没在 tools 里声明");
                        calls.push(str_field(block, "id"));
                    }
                    "tool_result" => {
                        let id = str_field(block, "tool_use_id");
                        assert!(previous_calls.contains(&id), "tool_result {id} 找不到配对的调用");
                    }
                    _ => {}
                }
            }
            previous_calls = calls;
        }
    }
    /// Codex 的真实形状照抄成最小样本：工具声明埋在 input 的 `additional_tools` 里、按
    /// namespace 分组，历史是 reasoning + custom_tool_call + output 的循环。不直接放语料
    /// 里的请求体——那里面有用户的源码和凭证。
    fn codex_request() -> Value {
        json!({
            "model": "gpt-5-codex",
            "instructions": "You are Codex.",
            "stream": true,
            "reasoning": {"effort": "max"},
            "input": [
                {"type": "additional_tools", "role": "developer", "tools": [
                    {"type": "namespace", "name": "functions", "tools": [
                        {"type": "custom", "name": "exec", "description": "run js"},
                        {"type": "function", "name": "wait", "parameters": {"type": "object"}}
                    ]}
                ]},
                {"type": "message", "role": "user",
                 "content": [{"type": "input_text", "text": "跑一下"}]},
                {"type": "reasoning", "id": "rs_1", "summary": [],
                 "encrypted_content": "gAAAAABnative"},
                {"type": "custom_tool_call", "call_id": "call_1", "name": "exec",
                 "namespace": "functions", "input": "console.log(1)"},
                {"type": "custom_tool_call_output", "call_id": "call_1", "output": "1"}
            ]
        })
    }

    #[test]
    fn codex_to_anthropic_keeps_structure() {
        let body = convert_request(RESPONSES, ANTHROPIC, &codex_request(), "7", DEFAULT_MAX_TOKENS)
            .expect("转换不该失败");
        assert_anthropic_shape(&body);
        // 上游原生密文不是我们的信封，回放不成立，思考块必须被丢掉而不是原样带走。
        let thinking = body["messages"].as_array().unwrap().iter().any(|message| {
            array(message.get("content"))
                .iter()
                .any(|block| str_field(block, "type").starts_with("thinking"))
        });
        assert!(!thinking, "原生密文不能当成可回放的思考块");
        // 末条 assistant 带 tool_use 又没有可验签的思考块，思考闸门整体关掉（A-1），
        // 而且必须显式写 disabled，不能省掉字段。
        assert_eq!(body["thinking"], json!({"type": "disabled"}));
        assert!(body.pointer("/output_config/effort").is_none());
        // 工具名照原样登记，namespace 只在回程补。
        let names: Vec<String> = array(body.get("tools"))
            .iter()
            .map(|tool| str_field(tool, "name"))
            .collect();
        assert_eq!(names, vec!["exec".to_string(), "wait".to_string()]);
    }
    /// C-1 的回归：清单里少了一个工具时，只摊平对不上的那个，别的照原样留着。
    #[test]
    fn undeclared_tool_is_flattened_not_rejected() {
        let mut body = codex_request();
        body["input"].as_array_mut().unwrap().extend([
            json!({"type": "function_call", "call_id": "call_2", "name": "ghost",
                   "arguments": "{\"a\":1}"}),
            json!({"type": "function_call_output", "call_id": "call_2", "output": "gone"}),
        ]);
        let out = convert_request(RESPONSES, ANTHROPIC, &body, "7", DEFAULT_MAX_TOKENS).unwrap();
        assert_anthropic_shape(&out);
        assert!(
            out.to_string().contains("调用工具 ghost"),
            "对不上的调用要变成文本，不能原样发出去换一个 No such tool"
        );
        let kept = out["messages"].as_array().unwrap().iter().any(|message| {
            array(message.get("content")).iter().any(|block| {
                str_field(block, "type") == "tool_use" && str_field(block, "name") == "exec"
            })
        });
        assert!(kept, "已声明的工具调用不能被顺手一起摊平");
    }

    /// C-14 的回归：`functions/exec` 同时是 custom 工具和 namespace 子工具，两件事都要还原。
    #[test]
    fn namespaced_custom_tool_is_restored() {
        let shapes = tool_shapes(RESPONSES, codex_request().to_string().as_bytes());
        let upstream = json!({
            "id": "msg_1", "model": "m", "stop_reason": "tool_use",
            "content": [
                {"type": "tool_use", "id": "toolu_1", "name": "exec",
                 "input": {"input": "console.log(1)"}},
                {"type": "tool_use", "id": "toolu_2", "name": "wait", "input": {}}
            ]
        });
        let out = convert_response(ANTHROPIC, RESPONSES, &upstream, "7", &shapes).unwrap();
        let items = out["output"].as_array().unwrap();
        assert_eq!(items[0]["type"], json!("custom_tool_call"));
        assert_eq!(items[0]["name"], json!("exec"));
        assert_eq!(items[0]["namespace"], json!("functions"), "custom 工具的 namespace 不能丢");
        assert_eq!(items[0]["input"], json!("console.log(1)"), "裸文本要从 input 字段剥回来");
        assert_eq!(items[1]["type"], json!("function_call"));
        assert_eq!(items[1]["namespace"], json!("functions"));
    }
    /// Claude Code 的真实形状：system 是内容块数组，思考走 adaptive + output_config.effort，
    /// 工具 id 45 字符（实测上限），schema 里带 OpenAI 不认的 `format: "uri"`。
    fn claude_code_request() -> Value {
        json!({
            "model": "claude-sonnet-4-5",
            "max_tokens": 64000,
            "stream": true,
            "system": [{"type": "text", "text": "You are Claude Code."}],
            "thinking": {"type": "adaptive"},
            "output_config": {"effort": "xhigh"},
            "context_management": {"edits": [{"type": "clear_thinking_20251015", "keep": "all"}]},
            "tools": [{"name": "Read", "description": "read a file", "input_schema": {
                "type": "object",
                "properties": {"file_path": {"type": "string", "format": "uri"}},
                "required": ["file_path"]
            }}],
            "messages": [
                {"role": "user", "content": "看一下 a.rs"},
                {"role": "assistant", "content": [
                    {"type": "thinking", "thinking": "先读文件", "signature": "native-sig"},
                    {"type": "tool_use", "id": "call-550e8400-e29b-41d4-a716-446655440000-0",
                     "name": "Read", "input": {"file_path": "a.rs"}}
                ]},
                {"role": "user", "content": [
                    {"type": "tool_result", "tool_use_id": "call-550e8400-e29b-41d4-a716-446655440000-0",
                     "content": [{"type": "text", "text": "fn main() {}"}]}
                ]}
            ]
        })
    }

    #[test]
    fn claude_code_to_responses_keeps_tool_pairing() {
        let out =
            convert_request(ANTHROPIC, RESPONSES, &claude_code_request(), "7", DEFAULT_MAX_TOKENS)
                .unwrap();
        let input = out["input"].as_array().unwrap();
        let call = input
            .iter()
            .position(|item| str_field(item, "type") == "function_call")
            .expect("要有 function_call");
        let result = input
            .iter()
            .position(|item| str_field(item, "type") == "function_call_output")
            .expect("要有 function_call_output");
        assert!(call < result, "调用必须排在结果之前");
        assert_eq!(str_field(&input[call], "call_id"), str_field(&input[result], "call_id"));
        assert!(
            !input.iter().any(|item| str_field(item, "type") == "reasoning"),
            "原生签名不是我们的信封，不能当成可回放的 reasoning"
        );
        // A-5：xhigh 原样透传，实测真实 Responses 端点接受这个档位，别加白名单压档。
        assert_eq!(out["reasoning"]["effort"], json!("xhigh"));
        // C-7：只剥字符串形式的 format: "uri"。
        assert!(!out["tools"][0]["parameters"].to_string().contains("\"uri\""));
    }
    /// C-3 的回归：超过 40 字符要哈希，且同一个请求里两处算出同一个值。截断会让
    /// `call-<uuid>-0` 和 `call-<uuid>-10` 撞成一个 id，工具结果配错调用。
    #[test]
    fn long_tool_ids_are_hashed_consistently() {
        let out =
            convert_request(ANTHROPIC, CHAT, &claude_code_request(), "7", DEFAULT_MAX_TOKENS)
                .unwrap();
        let messages = out["messages"].as_array().unwrap();
        let call = messages
            .iter()
            .find_map(|message| message.pointer("/tool_calls/0/id").and_then(Value::as_str))
            .expect("assistant 要带 tool_calls");
        let result = messages
            .iter()
            .filter(|message| str_field(message, "role") == "tool")
            .map(|message| str_field(message, "tool_call_id"))
            .next()
            .expect("要有 tool 消息");
        assert!(call.len() <= 40, "Chat 的 tool_call_id 上限 40 字符，实到 {}", call.len());
        assert_eq!(call, result, "调用与结果必须算出同一个 id");
    }

    /// 只有 Chat 需要封顶：它的 `reasoning_effort` 只有三档。
    #[test]
    fn only_chat_caps_the_effort_tiers() {
        let effort_to_chat = |effort: &str| {
            let body = json!({
                "model": "m",
                "reasoning": {"effort": effort},
                "input": [{"type": "message", "role": "user",
                           "content": [{"type": "input_text", "text": "hi"}]}]
            });
            convert_request(RESPONSES, CHAT, &body, "7", DEFAULT_MAX_TOKENS).unwrap()
                ["reasoning_effort"]
                .clone()
        };
        for effort in ["xhigh", "max"] {
            assert_eq!(effort_to_chat(effort), json!("high"), "{effort} 要压到 high");
        }
        for effort in ["low", "medium", "high"] {
            assert_eq!(effort_to_chat(effort), json!(effort));
        }
    }

    fn thinking_for(effort: &str) -> Value {
        let body = json!({
            "model": "m",
            "reasoning": {"effort": effort},
            "input": [{"type": "message", "role": "user",
                       "content": [{"type": "input_text", "text": "hi"}]}]
        });
        convert_request(RESPONSES, ANTHROPIC, &body, "7", DEFAULT_MAX_TOKENS).unwrap()
    }

    /// A-5：档位原样透传，且开关和强度两个旋钮一起写。
    #[test]
    fn thinking_writes_both_knobs_and_passes_effort_through() {
        for effort in ["low", "medium", "high", "xhigh", "max"] {
            let out = thinking_for(effort);
            assert_eq!(out["thinking"], json!({"type": "adaptive"}), "{effort} 应该开思考");
            assert_eq!(
                out["output_config"]["effort"],
                json!(effort),
                "{effort} 必须原样透传，Anthropic 和 Codex 是同一套档位"
            );
            assert!(
                out.pointer("/thinking/budget_tokens").is_none(),
                "不再换算成 token 预算"
            );
        }
        // OpenAI 侧的「不要思考」档，Anthropic 的 effort 里没有对应值。
        for effort in ["none", "minimal"] {
            let out = thinking_for(effort);
            assert_eq!(out["thinking"], json!({"type": "disabled"}));
            assert!(out.pointer("/output_config/effort").is_none());
        }
    }

    /// 老写法 `thinking.budget_tokens` 不认：走互转的只有 Claude Code 和 Codex，两家都只发
    /// 档位词。带着它进来时既不换算也不猜，思考按「没给档位」处理。
    #[test]
    fn legacy_budget_tokens_is_ignored() {
        let body = json!({
            "model": "m",
            "max_tokens": 48000,
            "thinking": {"type": "enabled", "budget_tokens": 16000},
            "messages": [{"role": "user", "content": "hi"}]
        });
        let out = convert_request(ANTHROPIC, RESPONSES, &body, "7", DEFAULT_MAX_TOKENS).unwrap();
        assert!(out.get("reasoning").is_none(), "不从数字预算里猜档位");
        assert!(out.get("include").is_none(), "没开思考就不该要密文");
    }

    /// 关思考必须显式写 `disabled`，不能省掉字段——省掉等于交给上游默认，而默认是开的。
    #[test]
    fn forced_tool_choice_disables_thinking_explicitly() {
        let body = json!({
            "model": "m",
            "reasoning": {"effort": "max"},
            "tool_choice": "required",
            "tools": [{"type": "function", "name": "Read", "parameters": {"type": "object"}}],
            "input": [{"type": "message", "role": "user",
                       "content": [{"type": "input_text", "text": "hi"}]}]
        });
        let out = convert_request(RESPONSES, ANTHROPIC, &body, "7", DEFAULT_MAX_TOKENS).unwrap();
        assert_eq!(
            out["thinking"],
            json!({"type": "disabled"}),
            "强制工具调用与思考互斥，且必须显式关掉"
        );
        assert!(out.pointer("/output_config/effort").is_none());
    }
    /// 上游 → IR → 客户端协议 → 网关统计，四个数字一个都不能少。C-13 的回归。
    fn usage_after_round_trip(from: Protocol, to: Protocol, upstream: Value) -> TokenUsage {
        let converted = convert_response(from, to, &upstream, "7", &ToolShapes::new()).unwrap();
        let mut usage = TokenUsage::default();
        parse_token_usage(converted.to_string().as_bytes(), to, &mut usage);
        usage
    }

    #[test]
    fn usage_round_trip_keeps_cache_write() {
        // Anthropic 上游 → Codex 客户端。
        let usage = usage_after_round_trip(
            ANTHROPIC,
            RESPONSES,
            json!({
                "id": "msg_1", "model": "m", "stop_reason": "end_turn",
                "content": [{"type": "text", "text": "hi"}],
                "usage": {"input_tokens": 19, "cache_read_input_tokens": 440,
                          "cache_creation_input_tokens": 196, "output_tokens": 361}
            }),
        );
        assert_eq!(usage.input_tokens, 19);
        assert_eq!(usage.cache_read_input_tokens, 440);
        assert_eq!(usage.cache_creation_input_tokens, 196, "缓存写入不能被并进普通输入");
        assert_eq!(usage.output_tokens, 361);

        // Codex 上游 → Claude Code 客户端。实测形状：cache_write 已经算在 input_tokens 里，
        // 所以要减掉；余下的 499 才是没命中缓存的新增内容。
        let usage = usage_after_round_trip(
            RESPONSES,
            ANTHROPIC,
            json!({
                "id": "resp_1", "model": "m", "status": "completed",
                "output": [{"type": "message",
                            "content": [{"type": "output_text", "text": "hi"}]}],
                "usage": {"input_tokens": 16243, "output_tokens": 495,
                          "input_tokens_details": {"cached_tokens": 13312,
                                                   "cache_write_tokens": 2432}}
            }),
        );
        assert_eq!(usage.input_tokens, 499);
        assert_eq!(usage.cache_read_input_tokens, 13312);
        assert_eq!(usage.cache_creation_input_tokens, 2432);
        assert_eq!(usage.output_tokens, 495);
    }
    /// C-2 的回归：有工具块时把 stop 规整成 tool_use，但截断和拦截不动。
    #[test]
    fn tool_calls_upgrade_stop_but_not_length() {
        let upstream = json!({"id": "c1", "model": "m", "choices": [{"index": 0,
            "finish_reason": "stop",
            "message": {"role": "assistant", "tool_calls": [{"id": "call_1", "type": "function",
                "function": {"name": "Read", "arguments": "{}"}}]}}]});
        let out = convert_response(CHAT, ANTHROPIC, &upstream, "7", &ToolShapes::new()).unwrap();
        assert_eq!(out["stop_reason"], json!("tool_use"));

        let mut truncated = upstream;
        truncated["choices"][0]["finish_reason"] = json!("length");
        let out = convert_response(CHAT, ANTHROPIC, &truncated, "7", &ToolShapes::new()).unwrap();
        assert_eq!(
            out["stop_reason"],
            json!("max_tokens"),
            "工具调用被截断时客户端必须知道参数 JSON 是残缺的"
        );
    }

    /// C-4 的回归：上游没发终止标记就断了，不能替它补一个成功结尾。
    #[test]
    fn truncated_stream_never_becomes_success() {
        let mut translator =
            StreamTranslator::new(ANTHROPIC, CHAT, "7", &ToolShapes::new()).unwrap();
        let mut out = translator.push(
            concat!(
                "event: message_start\n",
                r#"data: {"type":"message_start","message":{"id":"msg_1","model":"m"}}"#,
                "\n\n",
                "event: content_block_delta\n",
                r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"半截"}}"#,
                "\n\n",
            )
            .as_bytes(),
        );
        out.push_str(&translator.finish());
        assert!(out.contains("半截"), "已经收到的内容要交给客户端");
        assert!(out.contains("error"), "截断必须变成错误事件");
        assert!(!out.contains("finish_reason\":\"stop"), "不能补一个成功结尾");
        assert!(!out.contains("[DONE]"), "不能补 [DONE]");
    }
    /// C-9 + C-10 的回归：先保活、再把错误体当整块 JSON 甩过来（实测形状）。保活要转出去，
    /// 判错只能喂还没被当成帧吃掉的那一段，摊开时没有内容块也必须发错误。
    #[test]
    fn keepalive_then_whole_json_error_body() {
        let mut translator =
            StreamTranslator::new(RESPONSES, ANTHROPIC, "7", &ToolShapes::new()).unwrap();
        let out = translator.push(b": PING\n\n{\"error\":{\"message\":\"boom\"}}");
        assert!(
            out.starts_with(':'),
            "保活必须转成客户端协议的保活，丢掉会让转换路径比直通更容易超时"
        );
        let pending = translator.pending_body().expect("整块 JSON 还留在缓冲里");
        assert_eq!(
            std::str::from_utf8(pending).unwrap(),
            "{\"error\":{\"message\":\"boom\"}}",
            "判错要拿到干净的那一段：整条流拼起来带着保活注释，不是合法 JSON"
        );
        let tail = translator.finish();
        assert!(tail.contains("error"), "没有内容块就得发错误");
        assert!(!tail.contains("message_stop"), "不能发正常结尾");
    }
}

