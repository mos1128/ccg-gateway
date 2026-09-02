//! OpenAI Chat Completions 与 IR 互转。

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use serde_json::{json, Map, Value};

use super::ir::*;
use super::SseFrame;

fn parse_parts(value: Option<&Value>) -> Vec<Block> {
    match value {
        Some(Value::String(text)) if !text.is_empty() => vec![Block::Text(text.clone())],
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| match str_field(item, "type").as_str() {
                "text" => Some(Block::Text(str_field(item, "text"))),
                "image_url" => item
                    .pointer("/image_url/url")
                    .and_then(Value::as_str)
                    .map(|url| Block::Image(url.to_string())),
                "file" => {
                    let file = item.get("file")?;
                    let url = str_field(file, "file_data");
                    (!url.is_empty()).then(|| Block::Document {
                        url,
                        filename: str_field(file, "filename"),
                    })
                }
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn plain_text(value: Option<&Value>) -> String {
    parse_parts(value)
        .into_iter()
        .filter_map(|block| match block {
            Block::Text(text) => Some(text),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_tool_calls(value: Option<&Value>) -> Vec<Block> {
    array(value)
        .iter()
        .map(|call| Block::ToolUse {
            id: str_field(call, "id"),
            name: call
                .pointer("/function/name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            input: tool_input(call.pointer("/function/arguments")),
        })
        .collect()
}

fn reasoning_text(message: &Value) -> Option<String> {
    ["reasoning_content", "reasoning"]
        .iter()
        .filter_map(|key| message.get(*key).and_then(Value::as_str))
        .find(|text| !text.is_empty())
        .map(str::to_string)
}

fn parse_tool(tool: &Value) -> Option<Tool> {
    let function = tool.get("function").unwrap_or(tool);
    let name = str_field(function, "name");
    if name.is_empty() {
        return None;
    }
    Some(Tool {
        name,
        description: function
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_string),
        schema: function
            .get("parameters")
            .cloned()
            .unwrap_or_else(|| json!({"type": "object", "properties": {}})),
    })
}

fn parse_tool_choice(value: Option<&Value>) -> Option<ToolChoice> {
    match value? {
        Value::String(text) => match text.as_str() {
            "auto" => Some(ToolChoice::Auto),
            "none" => Some(ToolChoice::None),
            "required" | "any" => Some(ToolChoice::Required),
            _ => None,
        },
        value => {
            let name = value
                .pointer("/function/name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            (!name.is_empty()).then(|| ToolChoice::Tool(name.to_string()))
        }
    }
}

/// Chat 的 json_schema 嵌在 `json_schema` 子对象里，摊平成 IR 的扁平形状。
fn parse_response_format(value: Option<&Value>) -> Option<Value> {
    let value = value?;
    match str_field(value, "type").as_str() {
        "json_object" => Some(json!({"type": "json_object"})),
        "json_schema" => {
            let inner = value.get("json_schema").unwrap_or(value);
            Some(json!({
                "type": "json_schema",
                "name": str_field(inner, "name"),
                "schema": inner.get("schema").cloned().unwrap_or_else(|| json!({})),
                "strict": inner.get("strict").and_then(Value::as_bool).unwrap_or(false),
            }))
        }
        _ => None,
    }
}

fn build_response_format(format: &Value) -> Option<Value> {
    match str_field(format, "type").as_str() {
        "json_object" => Some(json!({"type": "json_object"})),
        "json_schema" => {
            let name = str_field(format, "name");
            Some(json!({"type": "json_schema", "json_schema": {
                "name": if name.is_empty() { "response".to_string() } else { name },
                "schema": clean_schema(format.get("schema").unwrap_or(&Value::Null)),
                "strict": format.get("strict").and_then(Value::as_bool).unwrap_or(false),
            }}))
        }
        _ => None,
    }
}

fn parse_finish(reason: &str) -> Option<Finish> {
    match reason {
        "stop" => Some(Finish::Stop),
        "length" => Some(Finish::Length),
        "tool_calls" | "function_call" => Some(Finish::ToolUse),
        "content_filter" => Some(Finish::ContentFilter),
        _ => None,
    }
}

fn finish_name(finish: Option<Finish>) -> &'static str {
    match finish {
        Some(Finish::Length) => "length",
        Some(Finish::ToolUse) => "tool_calls",
        Some(Finish::ContentFilter) => "content_filter",
        _ => "stop",
    }
}

/// 上游给了 tool_calls 却回 `finish_reason: stop`（或压根没给、给了自造值）时，按结束
/// 标记决定要不要执行工具的客户端会当成本轮结束，工具不执行、会话空转。以实际产出为准
/// 规整成工具调用。length / content_filter 不动——截断和拦截比「有工具调用」更该让客户端
/// 知道，Anthropic 自己也是这么回的。反过来也不能做：没有工具块时凭空写 tool_use，
/// Anthropic 会要求 content 里必须有对应的块。
fn finish_with_tools(finish: Option<Finish>, has_tool_use: bool) -> Option<Finish> {
    match (finish, has_tool_use) {
        (None | Some(Finish::Stop), true) => Some(Finish::ToolUse),
        _ => finish,
    }
}

fn parse_usage(value: Option<&Value>) -> Usage {
    let Some(value) = value else {
        return Usage::default();
    };
    let detail = |key: &str| {
        value
            .get("prompt_tokens_details")
            .and_then(|details| details.get(key))
            .and_then(Value::as_i64)
            .unwrap_or(0)
    };
    let cached = detail("cached_tokens");
    // 同 Responses 侧：缓存写入不是官方字段，兼容端点会给，且已经算进 prompt_tokens。
    let written = match detail("cache_write_tokens") {
        0 => detail("cached_creation_tokens"),
        value => value,
    };
    let prompt = value
        .get("prompt_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    Usage {
        input: (prompt - cached - written).max(0),
        cache_read: cached,
        cache_write: written,
        output: value
            .get("completion_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(0),
        reasoning: value
            .pointer("/completion_tokens_details/reasoning_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(0),
    }
}

fn build_usage(usage: &Usage) -> Value {
    json!({
        "prompt_tokens": usage.prompt_total(),
        "completion_tokens": usage.output,
        "total_tokens": usage.prompt_total() + usage.output,
        "prompt_tokens_details": {
            "cached_tokens": usage.cache_read,
            "cache_write_tokens": usage.cache_write,
        },
        "completion_tokens_details": {"reasoning_tokens": usage.reasoning},
    })
}

fn push_block(messages: &mut Vec<Message>, role: Role, block: Block) {
    match messages.last_mut() {
        Some(message) if message.role == role => message.blocks.push(block),
        _ => messages.push(Message {
            role,
            blocks: vec![block],
        }),
    }
}

pub fn parse_request(body: &Value) -> ChatRequest {
    let mut request = ChatRequest {
        model: str_field(body, "model"),
        max_tokens: body
            .get("max_tokens")
            .or_else(|| body.get("max_completion_tokens"))
            .and_then(Value::as_i64),
        temperature: body.get("temperature").and_then(Value::as_f64),
        top_p: body.get("top_p").and_then(Value::as_f64),
        stop: string_list(body.get("stop")),
        stream: body.get("stream").and_then(Value::as_bool).unwrap_or(false),
        effort: body
            .get("reasoning_effort")
            .and_then(Value::as_str)
            .map(str::to_string),
        parallel_tool_calls: body.get("parallel_tool_calls").and_then(Value::as_bool),
        response_format: parse_response_format(body.get("response_format")),
        cache_key: body
            .get("prompt_cache_key")
            .and_then(Value::as_str)
            .map(str::to_string),
        ..Default::default()
    };
    let mut system = Vec::new();
    for message in array(body.get("messages")) {
        match str_field(message, "role").as_str() {
            "system" | "developer" => {
                let text = plain_text(message.get("content"));
                if !text.is_empty() {
                    system.push(text);
                }
            }
            // 平行的多个 tool 消息要并进同一条 user 消息，Anthropic 才收。
            "tool" => push_block(
                &mut request.messages,
                Role::User,
                Block::ToolResult {
                    id: str_field(message, "tool_call_id"),
                    text: plain_text(message.get("content")),
                    is_error: false,
                    media: Vec::new(),
                },
            ),
            "assistant" => {
                let mut blocks = parse_parts(message.get("content"));
                blocks.extend(parse_tool_calls(message.get("tool_calls")));
                if !blocks.is_empty() {
                    request.messages.push(Message {
                        role: Role::Assistant,
                        blocks,
                    });
                }
            }
            _ => {
                for block in parse_parts(message.get("content")) {
                    push_block(&mut request.messages, Role::User, block);
                }
            }
        }
    }
    request.system = system.join("\n\n");
    request.tools = array(body.get("tools")).iter().filter_map(parse_tool).collect();
    request.tool_choice = parse_tool_choice(body.get("tool_choice"));
    request
}

/// 官方 Chat 的 tool_call_id 有 40 字符上限，兼容端点造的 `call-<uuid>-<n>` 是 43~45
/// 字符，原样带过去整个请求 400。超限的换成纯函数哈希：同一次请求里 assistant 的
/// tool_calls[].id 和 tool 消息的 tool_call_id 算出同一个值，上游新造的 id 本来就不
/// 超限、原样透传，所以不需要反向映射表。不能截断——`call-<uuid>-0` 和
/// `call-<uuid>-10` 前 40 位相同，截断会把两次调用合成一个。
fn short_tool_id(id: &str) -> String {
    if id.len() <= 40 {
        return id.to_string();
    }
    let mut hasher = DefaultHasher::new();
    id.hash(&mut hasher);
    format!("call_{:016x}", hasher.finish())
}

fn build_user(blocks: &[Block], messages: &mut Vec<Value>) {
    let mut parts = Vec::new();
    let mut media = Vec::new();
    for block in blocks {
        match block {
            // tool_result 在 Chat 里是独立的 tool 消息，必须排在 user 内容之前。
            Block::ToolResult {
                id,
                text,
                media: images,
                ..
            } => {
                let text = if text.trim().is_empty() && images.is_empty() {
                    TOOL_NO_OUTPUT
                } else {
                    text.as_str()
                };
                messages.push(
                    json!({"role": "tool", "tool_call_id": short_tool_id(id), "content": text}),
                );
                media.extend(images.iter().cloned());
            }
            Block::Text(text) if !text.is_empty() => {
                parts.push(json!({"type": "text", "text": text}))
            }
            Block::Image(url) => parts.push(json!({"type": "image_url", "image_url": {"url": url}})),
            // Chat 的 file 部件只收 data URL，http URL 无法降级只能丢。
            Block::Document { url, filename } if url.starts_with("data:") => parts.push(
                json!({"type": "file", "file": {"filename": filename, "file_data": url}}),
            ),
            _ => {}
        }
    }
    // tool 消息只收文本，图片挪到紧随其后的这条 user 消息里，并说明来处。
    if !media.is_empty() {
        let mut moved = vec![json!({"type": "text", "text": TOOL_MEDIA_MOVED})];
        moved.extend(
            media
                .iter()
                .map(|url| json!({"type": "image_url", "image_url": {"url": url}})),
        );
        moved.append(&mut parts);
        parts = moved;
    }
    match parts.len() {
        0 => {}
        1 if parts[0]["type"] == json!("text") => {
            messages.push(json!({"role": "user", "content": parts[0]["text"]}))
        }
        _ => messages.push(json!({"role": "user", "content": parts})),
    }
}

fn build_assistant(blocks: &[Block], messages: &mut Vec<Value>) {
    let mut text = String::new();
    let mut calls = Vec::new();
    for block in blocks {
        match block {
            Block::Text(part) => text.push_str(part),
            Block::ToolUse { id, name, input } => calls.push(json!({
                "id": short_tool_id(id),
                "type": "function",
                "function": {"name": name, "arguments": tool_input_string(input)},
            })),
            _ => {}
        }
    }
    if text.is_empty() && calls.is_empty() {
        return;
    }
    let mut message = json!({"role": "assistant"});
    message["content"] = if text.is_empty() {
        Value::Null
    } else {
        json!(text)
    };
    if !calls.is_empty() {
        message["tool_calls"] = Value::Array(calls);
    }
    messages.push(message);
}

/// Chat 的 `reasoning_effort` 只有 low / medium / high 三档，是三家里唯一需要封顶的：
/// Anthropic 与 Responses 的档位集合（low / medium / high / xhigh / max）和客户端是同一套，
/// 那两边原样透传，只有到 Chat 才压到 high。`none` / `minimal` 是 Chat 自己有的档位，
/// 原样留着。
fn chat_effort(effort: &str) -> &str {
    if matches!(
        effort.trim().to_ascii_lowercase().as_str(),
        "xhigh" | "max" | "ultra"
    ) {
        return "high";
    }
    effort
}

pub fn build_request(request: &ChatRequest) -> Value {
    let mut messages = Vec::new();
    if !request.system.is_empty() {
        messages.push(json!({"role": "system", "content": request.system}));
    }
    for message in &request.messages {
        match message.role {
            Role::User => build_user(&message.blocks, &mut messages),
            Role::Assistant => build_assistant(&message.blocks, &mut messages),
        }
    }

    let mut body = Map::new();
    body.insert("model".to_string(), json!(request.model));
    body.insert("messages".to_string(), Value::Array(messages));
    if let Some(value) = request.max_tokens {
        // 推理模型只认 max_completion_tokens，官方对普通模型也接受这个名字。
        body.insert("max_completion_tokens".to_string(), json!(value));
    }
    if let Some(value) = request.temperature {
        body.insert("temperature".to_string(), json!(value));
    }
    if let Some(value) = request.top_p {
        body.insert("top_p".to_string(), json!(value));
    }
    if !request.stop.is_empty() {
        body.insert("stop".to_string(), json!(request.stop));
    }
    if !request.tools.is_empty() {
        let tools = request
            .tools
            .iter()
            .map(|tool| {
                json!({"type": "function", "function": {
                    "name": tool.name,
                    "description": tool.description.clone().unwrap_or_default(),
                    "parameters": clean_schema(&tool.schema),
                }})
            })
            .collect();
        body.insert("tools".to_string(), Value::Array(tools));
    }
    // tool_choice / parallel_tool_calls 都要求同时给 tools，否则 OpenAI 400。
    if let Some(choice) = request
        .tool_choice
        .as_ref()
        .filter(|_| !request.tools.is_empty())
    {
        body.insert(
            "tool_choice".to_string(),
            match choice {
                ToolChoice::Auto => json!("auto"),
                ToolChoice::None => json!("none"),
                ToolChoice::Required => json!("required"),
                ToolChoice::Tool(name) => json!({"type": "function", "function": {"name": name}}),
            },
        );
    }
    if let Some(value) = request
        .parallel_tool_calls
        .filter(|_| !request.tools.is_empty())
    {
        body.insert("parallel_tool_calls".to_string(), json!(value));
    }
    if let Some(effort) = &request.effort {
        body.insert("reasoning_effort".to_string(), json!(chat_effort(effort)));
    }
    if let Some(cache_key) = &request.cache_key {
        body.insert("prompt_cache_key".to_string(), json!(cache_key));
    }
    if let Some(format) = request.response_format.as_ref().and_then(build_response_format) {
        body.insert("response_format".to_string(), format);
    }
    if request.stream {
        body.insert("stream".to_string(), json!(true));
        // 用量只在最后一个 chunk 给，不显式打开就统计不到。
        body.insert(
            "stream_options".to_string(),
            json!({"include_usage": true}),
        );
    }
    Value::Object(body)
}

/// 直通路径上剥掉历史里的思考痕迹。Chat 没有承载签名的字段，网关也不会往里写
/// Opaque 信封，所以只有事后整流那一轮需要清。返回是否真的改动过。
pub fn strip_thinking(body: &mut Value, drop_all: bool) -> bool {
    if !drop_all {
        return false;
    }
    let Some(messages) = body.get_mut("messages").and_then(Value::as_array_mut) else {
        return false;
    };
    let mut changed = false;
    for message in messages {
        if let Some(object) = message.as_object_mut() {
            for key in ["reasoning_content", "reasoning"] {
                changed |= object.remove(key).is_some();
            }
        }
    }
    changed
}

pub fn parse_response(body: &Value) -> ChatResponse {
    let choice = array(body.get("choices")).first();
    let message = choice.and_then(|choice| choice.get("message"));
    let mut blocks = Vec::new();
    if let Some(text) = message.and_then(reasoning_text) {
        // Chat 没有承载签名/密文的字段，思考内容只能当纯文本转出去。
        blocks.push(Block::Thinking { text, opaque: None });
    }
    blocks.extend(parse_parts(message.and_then(|message| message.get("content"))));
    blocks.extend(parse_tool_calls(
        message.and_then(|message| message.get("tool_calls")),
    ));
    let finish = parse_finish(
        &choice
            .map(|choice| str_field(choice, "finish_reason"))
            .unwrap_or_default(),
    );
    ChatResponse {
        id: str_field(body, "id"),
        model: str_field(body, "model"),
        finish: finish_with_tools(
            finish,
            blocks.iter().any(|block| matches!(block, Block::ToolUse { .. })),
        ),
        blocks,
        usage: parse_usage(body.get("usage")),
    }
}

pub fn build_response(response: &ChatResponse) -> Value {
    let mut text = String::new();
    let mut reasoning = String::new();
    let mut calls = Vec::new();
    for block in &response.blocks {
        match block {
            Block::Text(part) => text.push_str(part),
            Block::Thinking { text: part, .. } => reasoning.push_str(part),
            Block::ToolUse { id, name, input } => calls.push(json!({
                "id": id,
                "type": "function",
                "function": {"name": name, "arguments": tool_input_string(input)},
            })),
            _ => {}
        }
    }
    let mut message = json!({"role": "assistant", "content": if text.is_empty() {
        Value::Null
    } else {
        json!(text)
    }});
    if !reasoning.is_empty() {
        message["reasoning_content"] = json!(reasoning);
    }
    if !calls.is_empty() {
        message["tool_calls"] = Value::Array(calls);
    }
    json!({
        "id": ensure_id("chatcmpl-", &response.id),
        "object": "chat.completion",
        "created": crate::time::now_timestamp(),
        "model": response.model,
        "choices": [{"index": 0, "message": message, "finish_reason": finish_name(response.finish)}],
        "usage": build_usage(&response.usage),
    })
}

#[derive(Default)]
pub struct Decoder {
    id: String,
    model: String,
    started: bool,
    stopped: bool,
    received_finish: bool,
    /// 上游发过 `data: [DONE]`。Chat 的协议终止标记有两个，finish_reason 和 [DONE]，
    /// 部分兼容端点只给后者，只要见过一个就算协议层正常收尾。
    saw_done: bool,
    text: Option<usize>,
    reasoning: Option<usize>,
    tools: HashMap<usize, usize>,
    next: usize,
    usage: Usage,
    finish: Option<Finish>,
}

impl Decoder {
    /// 由 [`super::StreamTranslator`] 在读到 `data: [DONE]` 时调用。
    pub fn mark_done(&mut self) {
        self.saw_done = true;
    }

    pub fn push(&mut self, _name: &str, data: &Value) -> Vec<StreamEvent> {
        if let Some(message) = data.pointer("/error/message").and_then(Value::as_str) {
            self.stopped = true;
            return vec![StreamEvent::Error {
                message: message.to_string(),
            }];
        }
        let mut events = Vec::new();
        if !self.started {
            self.started = true;
            self.id = str_field(data, "id");
            self.model = str_field(data, "model");
            events.push(StreamEvent::Start {
                id: self.id.clone(),
                model: self.model.clone(),
            });
        }
        self.usage.merge(&parse_usage(data.get("usage")));
        let Some(choice) = array(data.get("choices")).first() else {
            return events;
        };
        let delta = choice.get("delta").or_else(|| choice.get("message"));
        if let Some(text) = delta.and_then(reasoning_text) {
            let index = match self.reasoning {
                Some(index) => index,
                None => {
                    let index = self.alloc();
                    self.reasoning = Some(index);
                    index
                }
            };
            events.push(StreamEvent::ThinkingDelta { index, text });
        }
        // reasoning 块首次出现时立即发 BlockStop，免得和后续 text/tool 嵌套。
        if self.reasoning.is_some() && delta.and_then(reasoning_text).is_none() {
            if let Some(index) = self.reasoning.take() {
                events.push(StreamEvent::BlockStop { index });
            }
        }
        if let Some(text) = delta
            .and_then(|delta| delta.get("content"))
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
        {
            let index = match self.text {
                Some(index) => index,
                None => {
                    let index = self.alloc();
                    self.text = Some(index);
                    index
                }
            };
            events.push(StreamEvent::TextDelta {
                index,
                text: text.to_string(),
            });
        }
        for call in array(delta.and_then(|delta| delta.get("tool_calls"))) {
            let slot = call.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
            let index = match self.tools.get(&slot).copied() {
                Some(index) => index,
                None => {
                    let index = self.alloc();
                    self.tools.insert(slot, index);
                    events.push(StreamEvent::ToolStart {
                        index,
                        id: str_field(call, "id"),
                        name: call
                            .pointer("/function/name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                    });
                    index
                }
            };
            if let Some(arguments) = call
                .pointer("/function/arguments")
                .and_then(Value::as_str)
                .filter(|arguments| !arguments.is_empty())
            {
                events.push(StreamEvent::ToolDelta {
                    index,
                    json: arguments.to_string(),
                });
            }
        }
        if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
            // 用量 chunk 排在 finish_reason 之后，Finish 必须等到 [DONE] 或流结束。
            self.finish = parse_finish(reason);
            self.received_finish = true;
        }
        events
    }

    pub fn finish(&mut self) -> Vec<StreamEvent> {
        if self.stopped {
            return Vec::new();
        }
        self.stopped = true;

        // 区分「协议层正常收尾」和「传输层刚好断在这里」：finish_reason 和 [DONE]
        // 任一出现过就算收到过终止标记，两个都没有说明流是断的，不能补出成功结束。
        if !self.received_finish && !self.saw_done {
            return vec![StreamEvent::Error {
                message: if self.started {
                    // 半截流：收到了部分内容但没有任何终止标记
                    "Stream ended without completion event".to_string()
                } else {
                    // 空流：完全没有内容
                    "Empty upstream response".to_string()
                },
            }];
        }

        let mut events = Vec::new();
        if !self.started {
            self.started = true;
            events.push(StreamEvent::Start {
                id: self.id.clone(),
                model: self.model.clone(),
            });
        }
        // 补 BlockStop 前先记下哪些槽位已经发过 Start，批量关闭它们。
        let mut started_slots = Vec::new();
        if let Some(index) = self.text {
            started_slots.push(index);
        }
        if let Some(index) = self.reasoning {
            started_slots.push(index);
        }
        started_slots.extend(self.tools.values().copied());
        started_slots.sort_unstable();
        started_slots.dedup();
        for slot in started_slots {
            events.push(StreamEvent::BlockStop { index: slot });
        }
        events.push(StreamEvent::Finish {
            finish: finish_with_tools(self.finish, !self.tools.is_empty()),
            usage: self.usage.clone(),
        });
        events
    }

    fn alloc(&mut self) -> usize {
        let index = self.next;
        self.next += 1;
        index
    }
}

fn data_frame(data: Value) -> SseFrame {
    SseFrame {
        event: None,
        data: data.to_string(),
    }
}

#[derive(Default)]
pub struct Encoder {
    id: String,
    model: String,
    started: bool,
    done: bool,
    slots: HashMap<usize, usize>,
    next: usize,
}

impl Encoder {
    pub fn encode(&mut self, event: &StreamEvent) -> Vec<SseFrame> {
        match event {
            StreamEvent::Start { id, model } => {
                self.id = ensure_id("chatcmpl-", id);
                self.model = model.clone();
                Vec::new()
            }
            StreamEvent::TextDelta { text, .. } => self.chunk(json!({"content": text})),
            StreamEvent::ThinkingDelta { text, .. } => {
                self.chunk(json!({"reasoning_content": text}))
            }
            // Chat 没有能带回签名/密文的字段，只能丢。
            StreamEvent::ThinkingOpaque { .. } => Vec::new(),
            StreamEvent::ToolStart { index, id, name } => {
                let slot = self.slot(*index);
                self.chunk(json!({"tool_calls": [{
                    "index": slot, "id": id, "type": "function",
                    "function": {"name": name, "arguments": ""},
                }]}))
            }
            StreamEvent::ToolDelta { index, json: partial } => {
                let slot = self.slot(*index);
                self.chunk(json!({"tool_calls": [{
                    "index": slot, "function": {"arguments": partial},
                }]}))
            }
            StreamEvent::BlockStop { .. } => Vec::new(),
            StreamEvent::Finish { finish, usage } => self.finish(*finish, usage),
            StreamEvent::Error { message } => vec![data_frame(
                json!({"error": {"type": "api_error", "message": message}}),
            )],
        }
    }
    fn slot(&mut self, index: usize) -> usize {
        if let Some(slot) = self.slots.get(&index) {
            return *slot;
        }
        let slot = self.next;
        self.next += 1;
        self.slots.insert(index, slot);
        slot
    }

    fn base(&mut self) -> Value {
        if self.id.is_empty() {
            self.id = ensure_id("chatcmpl-", "");
        }
        json!({
            "id": self.id,
            "object": "chat.completion.chunk",
            "created": crate::time::now_timestamp(),
            "model": self.model,
        })
    }

    fn chunk(&mut self, delta: Value) -> Vec<SseFrame> {
        let mut delta = delta;
        if !self.started {
            self.started = true;
            if let Some(object) = delta.as_object_mut() {
                object.insert("role".to_string(), json!("assistant"));
            }
        }
        let mut chunk = self.base();
        chunk["choices"] = json!([{"index": 0, "delta": delta, "finish_reason": Value::Null}]);
        vec![data_frame(chunk)]
    }

    fn finish(&mut self, finish: Option<Finish>, usage: &Usage) -> Vec<SseFrame> {
        if self.done {
            return Vec::new();
        }
        self.done = true;
        let mut stop = self.base();
        stop["choices"] = json!([{"index": 0, "delta": {}, "finish_reason": finish_name(finish)}]);
        let mut tail = self.base();
        tail["choices"] = json!([]);
        tail["usage"] = build_usage(usage);
        vec![
            data_frame(stop),
            data_frame(tail),
            SseFrame {
                event: None,
                data: "[DONE]".to_string(),
            },
        ]
    }
}

