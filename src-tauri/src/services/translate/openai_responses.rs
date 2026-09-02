//! OpenAI Responses 与 IR 互转。

use serde_json::{json, Map, Value};

use super::ir::*;
use super::{Format, SseFrame};

const KIND_TEXT: u8 = 0;
const KIND_REASONING: u8 = 1;
const KIND_TOOL: u8 = 2;

/// Codex 私有工具转成普通函数后用的名字。
const TOOL_SEARCH: &str = "tool_search";
const LOCAL_SHELL: &str = "local_shell";

fn parse_parts(value: Option<&Value>) -> Vec<Block> {
    match value {
        Some(Value::String(text)) if !text.is_empty() => vec![Block::Text(text.clone())],
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| match str_field(item, "type").as_str() {
                "input_text" | "output_text" | "text" => Some(Block::Text(str_field(item, "text"))),
                "input_image" => match item.get("image_url") {
                    Some(Value::String(url)) if !url.is_empty() => {
                        Some(Block::Image(url.clone()))
                    }
                    _ => None,
                },
                "input_file" => {
                    let url = str_field(item, "file_data");
                    (!url.is_empty()).then(|| Block::Document {
                        url,
                        filename: str_field(item, "filename"),
                    })
                }
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// function_call_output 的 output 可能是字符串、块数组或对象，图片单独收进 media。
fn output_content(value: Option<&Value>) -> (String, Vec<String>) {
    match value {
        Some(Value::Array(_)) => {
            let mut texts = Vec::new();
            let mut media = Vec::new();
            for block in parse_parts(value) {
                match block {
                    Block::Text(text) => texts.push(text),
                    Block::Image(url) => media.push(url),
                    _ => {}
                }
            }
            (texts.join("\n"), media)
        }
        Some(Value::String(text)) => (text.clone(), Vec::new()),
        Some(value) => (str_field(value, "text"), Vec::new()),
        None => (String::new(), Vec::new()),
    }
}

fn output_text(value: Option<&Value>) -> String {
    output_content(value).0
}

fn summary_text(item: &Value) -> String {
    let mut parts: Vec<String> = array(item.get("summary"))
        .iter()
        .map(|part| str_field(part, "text"))
        .filter(|text| !text.is_empty())
        .collect();
    if parts.is_empty() {
        parts = array(item.get("content"))
            .iter()
            .map(|part| str_field(part, "text"))
            .filter(|text| !text.is_empty())
            .collect();
    }
    parts.join("\n")
}

/// Codex 私有的工具形状。别家协议只有普通函数，转出去时一律摊平成普通函数；响应
/// 回来要照这份记录还原，否则 Codex 认不出自己发的调用。
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// 子工具所属的 namespace。工具名照原样登记，还原时把 namespace 补回去。
    Namespace(String),
    /// 原始工具名。入参是裸字符串，不是 JSON。custom 工具同样可能挂在 namespace 下
    /// （Codex 的 `functions/exec`），所以这里也要记住它——两件事只能记一件的话，
    /// 还原出来的调用会少一个 namespace，客户端对不上。
    Custom {
        name: String,
        namespace: Option<String>,
    },
    ToolSearch,
    LocalShell,
}

/// 工具名 -> 原始形状。普通函数不进这张表。
pub type ToolShapes = Vec<(String, Shape)>;

fn shape_of<'a>(shapes: &'a ToolShapes, name: &str) -> Option<&'a Shape> {
    shapes
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, shape)| shape)
}

/// custom 工具收裸文本，装进 `input` 字段变成普通函数的入参。
fn custom_schema() -> Value {
    json!({
        "type": "object",
        "properties": {"input": {"type": "string", "description": "工具的完整输入文本"}},
        "required": ["input"],
    })
}

fn tool_search_schema() -> Value {
    json!({
        "type": "object",
        "properties": {"query": {"type": "string", "description": "要查找的工具关键词"}},
        "required": ["query"],
    })
}

fn local_shell_schema() -> Value {
    json!({
        "type": "object",
        "properties": {"action": {
            "type": "object",
            "properties": {
                "command": {"type": "array", "items": {"type": "string"}},
                "timeout_ms": {"type": "integer"},
                "working_directory": {"type": "string"},
                "env": {"type": "object"},
            },
            "required": ["command"],
        }},
        "required": ["action"],
    })
}

/// 收集工具并记下私有形状。除了标准的 `tools`，还有两处藏在 `input` 里：Codex 把
/// 自己的工具声明塞在 `additional_tools` 里，动态加载的工具只出现在
/// `tool_search_output` 里。不一起扫，模型就一个工具都拿不到。
fn collect_tools(body: &Value) -> (Vec<Tool>, ToolShapes) {
    let mut tools = Vec::new();
    let mut shapes = ToolShapes::new();
    for tool in array(body.get("tools")) {
        push_tool(tool, None, &mut tools, &mut shapes);
    }
    for item in array(body.get("input")) {
        if matches!(
            str_field(item, "type").as_str(),
            "additional_tools" | "tool_search_output"
        ) {
            for tool in array(item.get("tools")) {
                push_tool(tool, None, &mut tools, &mut shapes);
            }
        }
    }
    (tools, shapes)
}

fn push_tool(tool: &Value, namespace: Option<&str>, tools: &mut Vec<Tool>, shapes: &mut ToolShapes) {
    let kind = str_field(tool, "type");
    // Chat 形状的工具把字段装在 function 子对象里，一并容忍。
    let source = tool.get("function").unwrap_or(tool);
    let raw = str_field(source, "name");
    if kind == "namespace" {
        // 子工具挂在 tools 或 children 上。
        let children = array(source.get("tools"))
            .iter()
            .chain(array(source.get("children")).iter());
        for child in children {
            push_tool(child, Some(&raw), tools, shapes);
        }
        return;
    }
    let (name, schema, shape) = match kind.as_str() {
        "custom" if !raw.is_empty() => (
            raw.clone(),
            custom_schema(),
            Some(Shape::Custom {
                name: raw,
                namespace: namespace.map(str::to_string),
            }),
        ),
        "tool_search" => (
            TOOL_SEARCH.to_string(),
            tool_search_schema(),
            Some(Shape::ToolSearch),
        ),
        "local_shell" => (
            LOCAL_SHELL.to_string(),
            local_shell_schema(),
            Some(Shape::LocalShell),
        ),
        "function" | "" if !raw.is_empty() => {
            let schema = source
                .get("parameters")
                .cloned()
                .unwrap_or_else(|| json!({"type": "object", "properties": {}}));
            // 名字照原样登记：历史里的调用绝大多数不带 namespace，改名就对不上了。
            match namespace {
                Some(space) => (raw, schema, Some(Shape::Namespace(space.to_string()))),
                None => (raw, schema, None),
            }
        }
        // web_search 这类内置工具没有 schema，别家协议也没有对应物。
        _ => return,
    };
    if tools.iter().any(|tool| tool.name == name) {
        return;
    }
    if let Some(shape) = shape {
        shapes.push((name.clone(), shape));
    }
    tools.push(Tool {
        name,
        description: source
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_string),
        schema,
    });
}

/// 客户端请求里的私有工具形状，响应还原时要用。
pub fn tool_shapes(body: &Value) -> ToolShapes {
    collect_tools(body).1
}

/// call_id 才是客户端下一轮回传的那个，item id 只是兜底。
fn call_id(item: &Value) -> String {
    let id = str_field(item, "call_id");
    if id.is_empty() {
        str_field(item, "id")
    } else {
        id
    }
}

/// 从函数入参里剥回 custom 工具的裸文本。模型直接给了裸串就原样用。
fn custom_input(arguments: &str) -> String {
    serde_json::from_str::<Value>(arguments)
        .ok()
        .and_then(|value| {
            value.get("input").map(|input| match input {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            })
        })
        .unwrap_or_else(|| arguments.to_string())
}

fn arguments_object(arguments: &str) -> Value {
    serde_json::from_str::<Value>(arguments)
        .ok()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}))
}

fn tool_item_prefix(shapes: &ToolShapes, name: &str) -> &'static str {
    match shape_of(shapes, name) {
        Some(Shape::Custom { .. }) => "ctc_",
        Some(Shape::LocalShell) => "lsh_",
        _ => "fc_",
    }
}

/// 按记下的形状还原成 Codex 认得的 item。表里没有就是普通函数调用。
fn tool_call_item(
    shapes: &ToolShapes,
    item: &str,
    status: &str,
    id: &str,
    name: &str,
    arguments: &str,
) -> Value {
    match shape_of(shapes, name) {
        Some(Shape::Custom {
            name: real,
            namespace,
        }) => {
            let mut call = json!({
                "type": "custom_tool_call", "id": item, "call_id": id,
                "name": real, "input": custom_input(arguments), "status": status,
            });
            if let Some(space) = namespace {
                call["namespace"] = json!(space);
            }
            call
        }
        // tool_search_call 由客户端自己执行，没有 item id。
        Some(Shape::ToolSearch) => json!({
            "type": "tool_search_call", "call_id": id, "status": status,
            "execution": "client", "arguments": arguments_object(arguments),
        }),
        Some(Shape::LocalShell) => json!({
            "type": "local_shell_call", "id": item, "call_id": id, "status": status,
            "action": arguments_object(arguments).get("action").cloned().unwrap_or(Value::Null),
        }),
        Some(Shape::Namespace(space)) => json!({
            "type": "function_call", "id": item, "call_id": id, "name": name,
            "namespace": space, "arguments": arguments, "status": status,
        }),
        None => json!({
            "type": "function_call", "id": item, "call_id": id, "name": name,
            "arguments": arguments, "status": status,
        }),
    }
}

/// Responses 的结构化输出已经是扁平形状，直接搬。
fn parse_text_format(value: Option<&Value>) -> Option<Value> {
    let value = value?;
    match str_field(value, "type").as_str() {
        "json_object" => Some(json!({"type": "json_object"})),
        "json_schema" => Some(json!({
            "type": "json_schema",
            "name": str_field(value, "name"),
            "schema": value.get("schema").cloned().unwrap_or_else(|| json!({})),
            "strict": value.get("strict").and_then(Value::as_bool).unwrap_or(false),
        })),
        _ => None,
    }
}

fn build_text_format(format: &Value) -> Option<Value> {
    match str_field(format, "type").as_str() {
        "json_object" => Some(json!({"type": "json_object"})),
        "json_schema" => {
            let name = str_field(format, "name");
            Some(json!({
                "type": "json_schema",
                "name": if name.is_empty() { "response".to_string() } else { name },
                "schema": clean_schema(format.get("schema").unwrap_or(&Value::Null)),
                "strict": format.get("strict").and_then(Value::as_bool).unwrap_or(false),
            }))
        }
        _ => None,
    }
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
            let name = str_field(value, "name");
            (!name.is_empty()).then_some(ToolChoice::Tool(name))
        }
    }
}

fn parse_usage(value: Option<&Value>) -> Usage {
    let Some(value) = value else {
        return Usage::default();
    };
    let detail = |key: &str| {
        value
            .get("input_tokens_details")
            .and_then(|details| details.get(key))
            .and_then(Value::as_i64)
            .unwrap_or(0)
    };
    let cached = detail("cached_tokens");
    // 官方用量没有缓存写入这一格，兼容端点普遍会给，实测 cache_write_tokens 与
    // cached_creation_tokens 两种写法都有。它和 cached_tokens 一样已经算进
    // input_tokens，所以要一起减掉才是 Anthropic 语义的「不含缓存的输入」。
    let written = match detail("cache_write_tokens") {
        0 => detail("cached_creation_tokens"),
        value => value,
    };
    let input = value.get("input_tokens").and_then(Value::as_i64).unwrap_or(0);
    Usage {
        input: (input - cached - written).max(0),
        cache_read: cached,
        cache_write: written,
        output: value
            .get("output_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(0),
        reasoning: value
            .pointer("/output_tokens_details/reasoning_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(0),
    }
}

fn build_usage(usage: &Usage) -> Value {
    json!({
        "input_tokens": usage.prompt_total(),
        // cache_write_tokens 不是官方字段，但真实上游就是这么发给 Codex 的（直通路径
        // 上客户端一直在收），不写这一格网关自己的用量统计就丢掉缓存写入。
        "input_tokens_details": {
            "cached_tokens": usage.cache_read,
            "cache_write_tokens": usage.cache_write,
        },
        "output_tokens": usage.output,
        "output_tokens_details": {"reasoning_tokens": usage.reasoning},
        "total_tokens": usage.prompt_total() + usage.output,
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

fn item_id(prefix: &str, seq: usize) -> String {
    format!("{}{}{}", prefix, crate::time::now_timestamp(), seq)
}

fn parse_item(item: &Value, request: &mut ChatRequest) {
    match str_field(item, "type").as_str() {
        // 工具表按原名登记，历史里的调用名直接用；namespace 只在还原时补回去。
        "function_call" => push_block(
            &mut request.messages,
            Role::Assistant,
            Block::ToolUse {
                id: call_id(item),
                name: str_field(item, "name"),
                input: tool_input(item.get("arguments")),
            },
        ),
        // custom 工具的入参是裸文本，包成展平后 schema 认的那个字段。
        "custom_tool_call" => push_block(
            &mut request.messages,
            Role::Assistant,
            Block::ToolUse {
                id: call_id(item),
                name: str_field(item, "name"),
                input: json!({"input": str_field(item, "input")}),
            },
        ),
        "tool_search_call" => push_block(
            &mut request.messages,
            Role::Assistant,
            Block::ToolUse {
                id: call_id(item),
                name: TOOL_SEARCH.to_string(),
                input: tool_input(item.get("arguments")),
            },
        ),
        "local_shell_call" => push_block(
            &mut request.messages,
            Role::Assistant,
            Block::ToolUse {
                id: call_id(item),
                name: LOCAL_SHELL.to_string(),
                input: json!({"action": item.get("action").cloned().unwrap_or(Value::Null)}),
            },
        ),
        "function_call_output" | "custom_tool_call_output" | "local_shell_call_output" => {
            let (text, media) = output_content(item.get("output"));
            push_block(
                &mut request.messages,
                Role::User,
                Block::ToolResult {
                    id: call_id(item),
                    text,
                    is_error: false,
                    media,
                },
            )
        }
        // 加载到的工具 schema 已经并进 tools 了，结果里只把名字回给模型。
        "tool_search_output" => {
            let mut loaded = Vec::new();
            let mut ignored = ToolShapes::new();
            for tool in array(item.get("tools")) {
                push_tool(tool, None, &mut loaded, &mut ignored);
            }
            let names: Vec<String> = loaded.into_iter().map(|tool| tool.name).collect();
            push_block(
                &mut request.messages,
                Role::User,
                Block::ToolResult {
                    id: call_id(item),
                    text: names.join("\n"),
                    is_error: false,
                    media: Vec::new(),
                },
            )
        }
        // 密文只有原路回到同一个上游才解得开，靠信封里的标记判定。
        "reasoning" => {
            let opaque = Opaque::decode(&str_field(item, "encrypted_content"));
            let text = summary_text(item);
            if opaque.is_some() || !text.is_empty() {
                push_block(
                    &mut request.messages,
                    Role::Assistant,
                    Block::Thinking { text, opaque },
                );
            }
        }
        // 引用的是上游存的 item，换协议后无从还原。
        "item_reference" => {}
        // 工具声明，collect_tools 已经收过；它的 role 是 developer，别落到下面被
        // 当成一条系统提示。
        "additional_tools" => {}
        _ => match str_field(item, "role").as_str() {
            "system" | "developer" => {
                let text = parse_parts(item.get("content"))
                    .into_iter()
                    .filter_map(|block| match block {
                        Block::Text(text) => Some(text),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                if !text.is_empty() {
                    if !request.system.is_empty() {
                        request.system.push_str("\n\n");
                    }
                    request.system.push_str(&text);
                }
            }
            role => {
                let role = if role == "assistant" {
                    Role::Assistant
                } else {
                    Role::User
                };
                for block in parse_parts(item.get("content")) {
                    push_block(&mut request.messages, role, block);
                }
            }
        },
    }
}

pub fn parse_request(body: &Value) -> ChatRequest {
    let mut request = ChatRequest {
        model: str_field(body, "model"),
        system: str_field(body, "instructions"),
        max_tokens: body.get("max_output_tokens").and_then(Value::as_i64),
        temperature: body.get("temperature").and_then(Value::as_f64),
        top_p: body.get("top_p").and_then(Value::as_f64),
        stream: body.get("stream").and_then(Value::as_bool).unwrap_or(false),
        effort: body
            .pointer("/reasoning/effort")
            .and_then(Value::as_str)
            .map(str::to_string),
        parallel_tool_calls: body.get("parallel_tool_calls").and_then(Value::as_bool),
        response_format: parse_text_format(body.pointer("/text/format")),
        cache_key: body
            .get("prompt_cache_key")
            .and_then(Value::as_str)
            .map(str::to_string),
        ..Default::default()
    };
    match body.get("input") {
        Some(Value::String(text)) if !text.is_empty() => request.messages.push(Message {
            role: Role::User,
            blocks: vec![Block::Text(text.clone())],
        }),
        Some(Value::Array(items)) => {
            for item in items {
                parse_item(item, &mut request);
            }
        }
        _ => {}
    }
    request.tools = collect_tools(body).0;
    request.tool_choice = parse_tool_choice(body.get("tool_choice"));
    request
}

fn flush_message(parts: &mut Vec<Value>, input: &mut Vec<Value>) {
    if parts.is_empty() {
        return;
    }
    input.push(json!({
        "type": "message",
        "role": "assistant",
        "content": std::mem::take(parts),
    }));
}

fn build_user(blocks: &[Block], input: &mut Vec<Value>) {
    let mut parts = Vec::new();
    let mut media = Vec::new();
    for block in blocks {
        match block {
            Block::ToolResult {
                id,
                text,
                media: images,
                ..
            } => {
                input.push(json!({
                    "type": "function_call_output",
                    "call_id": id,
                    "output": if text.trim().is_empty() && images.is_empty() {
                        TOOL_NO_OUTPUT
                    } else {
                        text.as_str()
                    },
                }));
                media.extend(images.iter().cloned());
            }
            Block::Text(text) if !text.is_empty() => {
                parts.push(json!({"type": "input_text", "text": text}))
            }
            Block::Image(url) => parts.push(json!({"type": "input_image", "image_url": url})),
            // input_file 只收 data URL，http URL 无法降级只能丢。
            Block::Document { url, filename } if url.starts_with("data:") => parts.push(
                json!({"type": "input_file", "filename": filename, "file_data": url}),
            ),
            _ => {}
        }
    }
    // function_call_output 只收文本，图片挪到紧随其后的这条 user 消息里。
    if !media.is_empty() {
        let mut moved = vec![json!({"type": "input_text", "text": TOOL_MEDIA_MOVED})];
        moved.extend(
            media
                .iter()
                .map(|url| json!({"type": "input_image", "image_url": url})),
        );
        moved.append(&mut parts);
        parts = moved;
    }
    if !parts.is_empty() {
        input.push(json!({"type": "message", "role": "user", "content": parts}));
    }
}

fn build_assistant(blocks: &[Block], input: &mut Vec<Value>, key: &str) {
    let mut parts = Vec::new();
    for block in blocks {
        match block {
            Block::Text(text) if !text.is_empty() => {
                parts.push(json!({"type": "output_text", "text": text}))
            }
            // 只回放确实来自这个上游的 reasoning item，密文才解得开。
            Block::Thinking {
                opaque: Some(opaque),
                ..
            } => {
                if let Some(payload) = opaque.replay(Format::Responses, key) {
                    // reasoning 是独立 item，前面攒的文本要先落一条 message。
                    flush_message(&mut parts, input);
                    input.push(payload.clone());
                }
            }
            Block::ToolUse {
                id,
                name,
                input: arguments,
            } => {
                // function_call 是独立 item，前面攒的文本要先落一条 message。
                flush_message(&mut parts, input);
                input.push(json!({
                    "type": "function_call",
                    "call_id": id,
                    "name": name,
                    "arguments": tool_input_string(arguments),
                }));
            }
            _ => {}
        }
    }
    flush_message(&mut parts, input);
}

pub fn build_request(request: &ChatRequest, key: &str) -> Value {
    let mut input = Vec::new();
    for message in &request.messages {
        match message.role {
            Role::User => build_user(&message.blocks, &mut input),
            Role::Assistant => build_assistant(&message.blocks, &mut input, key),
        }
    }

    let mut body = Map::new();
    body.insert("model".to_string(), json!(request.model));
    if !request.system.is_empty() {
        body.insert("instructions".to_string(), json!(request.system));
    }
    body.insert("input".to_string(), Value::Array(input));
    if let Some(value) = request.max_tokens {
        body.insert("max_output_tokens".to_string(), json!(value));
    }
    if let Some(value) = request.temperature {
        body.insert("temperature".to_string(), json!(value));
    }
    if let Some(value) = request.top_p {
        body.insert("top_p".to_string(), json!(value));
    }
    if !request.tools.is_empty() {
        let tools = request
            .tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "name": tool.name,
                    "description": tool.description.clone().unwrap_or_default(),
                    "parameters": clean_schema(&tool.schema),
                })
            })
            .collect();
        body.insert("tools".to_string(), Value::Array(tools));
    }
    // 没有 tools 时带 tool_choice / parallel_tool_calls 会被拒。
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
                ToolChoice::Tool(name) => json!({"type": "function", "name": name}),
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
        // summary 要 auto 才会回摘要，否则客户端只能看到一个空思考块；密文得显式
        // include 才会给，是下一轮回放的唯一依据。档位原样透传，取值集合和客户端同一套。
        body.insert(
            "reasoning".to_string(),
            json!({"effort": effort, "summary": "auto"}),
        );
        body.insert(
            "include".to_string(),
            json!(["reasoning.encrypted_content"]),
        );
    }
    // 历史每轮都由我们自己拼全，不用上游存状态，也就不需要 previous_response_id。
    body.insert("store".to_string(), json!(false));
    if let Some(cache_key) = &request.cache_key {
        body.insert("prompt_cache_key".to_string(), json!(cache_key));
    }
    if let Some(format) = request.response_format.as_ref().and_then(build_text_format) {
        body.insert("text".to_string(), json!({"format": format}));
    }
    if request.stream {
        body.insert("stream".to_string(), json!(true));
    }
    Value::Object(body)
}

/// 直通路径上剥掉 input 里失效的 reasoning item。`drop_all` 为真是事后整流，全部
/// 剥掉重试；否则只剥 encrypted_content 是网关信封的那些——信封是上一轮走过转换留
/// 下的，原生上游解不开。返回是否真的改动过。
pub fn strip_thinking(body: &mut Value, drop_all: bool) -> bool {
    let Some(input) = body.get_mut("input").and_then(Value::as_array_mut) else {
        return false;
    };
    let before = input.len();
    input.retain(|item| {
        if str_field(item, "type") != "reasoning" {
            return true;
        }
        !drop_all && !Opaque::is_envelope(&str_field(item, "encrypted_content"))
    });
    before != input.len()
}

pub fn parse_response(body: &Value, key: &str) -> ChatResponse {
    let response = body.get("response").unwrap_or(body);
    let mut blocks = Vec::new();
    let mut tool_use = false;
    for item in array(response.get("output")) {
        match str_field(item, "type").as_str() {
            "message" => blocks.extend(parse_parts(item.get("content"))),
            "function_call" => {
                tool_use = true;
                blocks.push(Block::ToolUse {
                    id: str_field(item, "call_id"),
                    name: str_field(item, "name"),
                    input: tool_input(item.get("arguments")),
                });
            }
            "reasoning" => {
                let text = summary_text(item);
                // 整块留着，下一轮原路回放才能续上思考链。
                let opaque = (!str_field(item, "encrypted_content").is_empty())
                    .then(|| Opaque::new(Format::Responses, key, item.clone()));
                if opaque.is_some() || !text.is_empty() {
                    blocks.push(Block::Thinking { text, opaque });
                }
            }
            _ => {}
        }
    }
    let finish = match str_field(response, "status").as_str() {
        "incomplete" => Some(Finish::Length),
        _ if tool_use => Some(Finish::ToolUse),
        "completed" => Some(Finish::Stop),
        _ => None,
    };
    ChatResponse {
        id: str_field(response, "id"),
        model: str_field(response, "model"),
        blocks,
        finish,
        usage: parse_usage(response.get("usage")),
    }
}

fn flush_output(parts: &mut Vec<Value>, output: &mut Vec<Value>) {
    if parts.is_empty() {
        return;
    }
    let id = item_id("msg_", output.len());
    output.push(json!({
        "type": "message",
        "id": id,
        "role": "assistant",
        "status": "completed",
        "content": std::mem::take(parts),
    }));
}

pub fn build_response(response: &ChatResponse, shapes: &ToolShapes) -> Value {
    let mut output = Vec::new();
    let mut parts = Vec::new();
    for block in &response.blocks {
        match block {
            Block::Thinking { text, opaque } => {
                if text.is_empty() && opaque.is_none() {
                    continue;
                }
                flush_output(&mut parts, &mut output);
                let id = item_id("rs_", output.len());
                let mut item = json!({
                    "type": "reasoning",
                    "id": id,
                    "summary": [{"type": "summary_text", "text": text}],
                });
                // 信封塞进密文字段带给客户端，下一轮回传时能原样还原。
                if let Some(opaque) = opaque {
                    item["encrypted_content"] = json!(opaque.encode());
                }
                output.push(item);
            }
            Block::Text(text) if !text.is_empty() => {
                parts.push(json!({"type": "output_text", "text": text, "annotations": []}))
            }
            Block::ToolUse { id, name, input } => {
                flush_output(&mut parts, &mut output);
                let item = item_id(tool_item_prefix(shapes, name), output.len());
                output.push(tool_call_item(
                    shapes,
                    &item,
                    "completed",
                    id,
                    name,
                    &tool_input_string(input),
                ));
            }
            _ => {}
        }
    }
    flush_output(&mut parts, &mut output);
    build_envelope(response.finish, &response.id, &response.model, output, Some(&response.usage))
}

fn build_envelope(
    finish: Option<Finish>,
    id: &str,
    model: &str,
    output: Vec<Value>,
    usage: Option<&Usage>,
) -> Value {
    let incomplete = finish == Some(Finish::Length);
    let mut response = json!({
        "id": ensure_id("resp_", id),
        "object": "response",
        "created_at": crate::time::now_timestamp(),
        "model": model,
        "status": if incomplete { "incomplete" } else { "completed" },
        "incomplete_details": if incomplete {
            json!({"reason": "max_output_tokens"})
        } else {
            Value::Null
        },
        "output": output,
        "parallel_tool_calls": true,
        "tool_choice": "auto",
        "tools": [],
    });
    if let Some(usage) = usage {
        response["usage"] = build_usage(usage);
    }
    response
}

fn output_index(data: &Value) -> usize {
    data.get("output_index").and_then(Value::as_u64).unwrap_or(0) as usize
}

fn error_message(data: &Value) -> String {
    data.pointer("/error/message")
        .and_then(Value::as_str)
        .or_else(|| data.get("message").and_then(Value::as_str))
        .map(str::to_string)
        .unwrap_or_else(|| data.to_string())
}

/// Responses 的块序号是 (output_index, content_index/summary_index) 两级，映射成
/// IR 的一维序号。
#[derive(Default)]
pub struct Decoder {
    key: String,
    id: String,
    model: String,
    started: bool,
    stopped: bool,
    /// 见过 `response.completed` / `response.incomplete`。只有传输层 EOF 不算，否则
    /// 截断的流会被补成一次成功的空回答。
    received_complete: bool,
    slots: Vec<((usize, usize), usize)>,
    open: Vec<usize>,
    deltas: Vec<usize>,
    next: usize,
    usage: Usage,
    finish: Option<Finish>,
    tool_use: bool,
}

impl Decoder {
    pub fn new(key: &str) -> Decoder {
        Decoder {
            key: key.to_string(),
            ..Default::default()
        }
    }

    pub fn push(&mut self, name: &str, data: &Value) -> Vec<StreamEvent> {
        let kind = if name.is_empty() {
            str_field(data, "type")
        } else {
            name.to_string()
        };
        if kind == "error" || kind == "response.error" {
            self.stopped = true;
            return vec![StreamEvent::Error {
                message: error_message(data),
            }];
        }
        if let Some(response) = data.get("response") {
            if self.id.is_empty() {
                self.id = str_field(response, "id");
            }
            if self.model.is_empty() {
                self.model = str_field(response, "model");
            }
        }
        let mut events = self.start();
        match kind.as_str() {
            "response.output_item.added" => events.extend(self.item_added(data)),
            "response.output_text.delta" => events.extend(self.text_delta(data, false)),
            "response.reasoning_summary_text.delta" | "response.reasoning_text.delta" => {
                events.extend(self.text_delta(data, true))
            }
            "response.function_call_arguments.delta" => {
                let index = self.slot(output_index(data), 0);
                self.mark(index);
                events.push(StreamEvent::ToolDelta {
                    index,
                    json: str_field(data, "delta"),
                });
            }
            "response.output_item.done" => {
                events.extend(self.item_done(data));
                events.extend(self.close(output_index(data)));
            }
            "response.completed" | "response.incomplete" | "response.failed" => {
                events.extend(self.complete(&kind, data))
            }
            _ => {}
        }
        events
    }

    pub fn finish(&mut self) -> Vec<StreamEvent> {
        if self.stopped {
            return Vec::new();
        }
        self.stopped = true;
        // 没见过 response.completed / response.incomplete 就是流被截断了，不能替上游
        // 补一次成功结束。
        if !self.received_complete {
            return vec![StreamEvent::Error {
                message: if self.started {
                    "Stream ended without response.completed".to_string()
                } else {
                    "Empty upstream response".to_string()
                },
            }];
        }
        let mut events = self.start();
        for index in std::mem::take(&mut self.open) {
            events.push(StreamEvent::BlockStop { index });
        }
        if self.finish.is_none() {
            self.finish = Some(if self.tool_use {
                Finish::ToolUse
            } else {
                Finish::Stop
            });
        }
        events.push(StreamEvent::Finish {
            finish: self.finish,
            usage: self.usage.clone(),
        });
        events
    }

    fn start(&mut self) -> Vec<StreamEvent> {
        if self.started {
            return Vec::new();
        }
        self.started = true;
        vec![StreamEvent::Start {
            id: self.id.clone(),
            model: self.model.clone(),
        }]
    }

    fn has_slot(&self, output: usize, part: usize) -> bool {
        self.slots.iter().any(|(key, _)| *key == (output, part))
    }

    /// 这个 output item 最后分配的槽。`slots` 按分配顺序追加，取最后一个匹配的即可。
    fn last_slot(&self, output: usize) -> Option<usize> {
        self.slots
            .iter()
            .filter(|(key, _)| key.0 == output)
            .map(|(_, index)| *index)
            .last()
    }

    fn slot(&mut self, output: usize, part: usize) -> usize {
        if let Some((_, index)) = self.slots.iter().find(|(key, _)| *key == (output, part)) {
            return *index;
        }
        let index = self.next;
        self.next += 1;
        self.slots.push(((output, part), index));
        self.open.push(index);
        index
    }

    fn mark(&mut self, index: usize) {
        if !self.deltas.contains(&index) {
            self.deltas.push(index);
        }
    }

    fn close(&mut self, output: usize) -> Vec<StreamEvent> {
        let indexes: Vec<usize> = self
            .slots
            .iter()
            .filter(|(key, _)| key.0 == output)
            .map(|(_, index)| *index)
            .collect();
        let mut events = Vec::new();
        for index in indexes {
            if let Some(position) = self.open.iter().position(|item| *item == index) {
                self.open.remove(position);
                events.push(StreamEvent::BlockStop { index });
            }
        }
        events
    }

    fn item_added(&mut self, data: &Value) -> Vec<StreamEvent> {
        let Some(item) = data.get("item") else {
            return Vec::new();
        };
        if str_field(item, "type") != "function_call" {
            return Vec::new();
        }
        self.tool_use = true;
        let index = self.slot(output_index(data), 0);
        vec![StreamEvent::ToolStart {
            index,
            id: str_field(item, "call_id"),
            name: str_field(item, "name"),
        }]
    }

    fn text_delta(&mut self, data: &Value, thinking: bool) -> Vec<StreamEvent> {
        let part = data
            .get("content_index")
            .or_else(|| data.get("summary_index"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize;
        let index = self.slot(output_index(data), part);
        self.mark(index);
        let text = str_field(data, "delta");
        if text.is_empty() {
            Vec::new()
        } else if thinking {
            vec![StreamEvent::ThinkingDelta { index, text }]
        } else {
            vec![StreamEvent::TextDelta { index, text }]
        }
    }

    fn complete(&mut self, kind: &str, data: &Value) -> Vec<StreamEvent> {
        let response = data.get("response");
        self.usage
            .merge(&parse_usage(response.and_then(|value| value.get("usage"))));
        if kind == "response.failed" {
            self.stopped = true;
            return vec![StreamEvent::Error {
                message: response
                    .map(error_message)
                    .unwrap_or_else(|| "上游响应失败".to_string()),
            }];
        }
        if kind == "response.incomplete" {
            self.finish = Some(Finish::Length);
        }
        self.received_complete = true;
        self.finish()
    }

    /// 有些上游只发终态 item、不发增量。已经收到过增量的块跳过，否则按整块补发。
    fn item_done(&mut self, data: &Value) -> Vec<StreamEvent> {
        let Some(item) = data.get("item") else {
            return Vec::new();
        };
        let output = output_index(data);
        let mut events = Vec::new();
        match str_field(item, "type").as_str() {
            "function_call" => {
                self.tool_use = true;
                if !self.has_slot(output, 0) {
                    let index = self.slot(output, 0);
                    events.push(StreamEvent::ToolStart {
                        index,
                        id: str_field(item, "call_id"),
                        name: str_field(item, "name"),
                    });
                }
                let index = self.slot(output, 0);
                if !self.deltas.contains(&index) {
                    self.mark(index);
                    events.push(StreamEvent::ToolDelta {
                        index,
                        json: tool_input_string(item.get("arguments").unwrap_or(&Value::Null)),
                    });
                }
            }
            "message" => {
                for (part, content) in array(item.get("content")).iter().enumerate() {
                    let index = self.slot(output, part);
                    if self.deltas.contains(&index) {
                        continue;
                    }
                    let text = output_text(Some(content));
                    if text.is_empty() {
                        continue;
                    }
                    self.mark(index);
                    events.push(StreamEvent::TextDelta { index, text });
                }
            }
            "reasoning" => {
                let index = self.slot(output, 0);
                if !self.deltas.contains(&index) {
                    let text = summary_text(item);
                    if !text.is_empty() {
                        self.mark(index);
                        events.push(StreamEvent::ThinkingDelta { index, text });
                    }
                }
                // 密文整块带走，下一轮原路回放才能续上思考链。挂在这个 output 最后
                // 分配的槽上：摘要分成多段时前面的槽已经被目标编码器关掉了（Anthropic
                // 同一时刻只允许一个内容块打开），往关掉的块上补 delta 是非法序列。
                if !str_field(item, "encrypted_content").is_empty() {
                    events.push(StreamEvent::ThinkingOpaque {
                        index: self.last_slot(output).unwrap_or(index),
                        opaque: Opaque::new(Format::Responses, &self.key, item.clone()),
                    });
                }
            }
            _ => {}
        }
        events
    }
}
/// IR 块序号 -> 一个独立的 output item。Responses 允许 output 里有多个 message，
/// 所以每个块各占一个 item、content_index 固定为 0，省掉两级序号的簿记。
#[derive(Clone, Default)]
struct Slot {
    kind: u8,
    output: usize,
    item: String,
    id: String,
    name: String,
    text: String,
    /// 上游给的原始载荷编成的信封，reasoning item 关闭时写进 encrypted_content。
    opaque: String,
    closed: bool,
}

#[derive(Default)]
pub struct Encoder {
    id: String,
    model: String,
    created: bool,
    sequence: u64,
    slots: Vec<(usize, Slot)>,
    output_next: usize,
    done: Vec<Value>,
    finished: bool,
    /// 客户端请求里记下的私有工具形状，收尾时按它还原调用 item。
    shapes: ToolShapes,
}

impl Encoder {
    pub fn new(shapes: &ToolShapes) -> Encoder {
        Encoder {
            shapes: shapes.clone(),
            ..Default::default()
        }
    }

    pub fn encode(&mut self, event: &StreamEvent) -> Vec<SseFrame> {
        match event {
            StreamEvent::Start { id, model } => {
                self.id = ensure_id("resp_", id);
                self.model = model.clone();
                // 同 Anthropic：包裹事件立刻发，不等第一个内容增量。
                self.created_frames()
            }
            StreamEvent::TextDelta { index, text } => self.delta(*index, KIND_TEXT, text),
            StreamEvent::ThinkingDelta { index, text } => {
                self.delta(*index, KIND_REASONING, text)
            }
            // 信封先记在槽位上，跟着 reasoning item 的收尾事件一起发出去。
            StreamEvent::ThinkingOpaque { index, opaque } => {
                let (position, frames) = self.ensure(*index, KIND_REASONING, "", "");
                self.slots[position].1.opaque = opaque.encode();
                frames
            }
            // 工具 id 原样透传：客户端下一轮会带着它回来，改写会对不上上游。
            StreamEvent::ToolStart { index, id, name } => self.ensure(*index, KIND_TOOL, id, name).1,
            StreamEvent::ToolDelta { index, json: partial } => {
                self.delta(*index, KIND_TOOL, partial)
            }
            StreamEvent::BlockStop { index } => self.close(*index),
            StreamEvent::Finish { finish, usage } => self.finish(*finish, usage),
            StreamEvent::Error { message } => {
                let data = json!({"code": Value::Null, "message": message, "param": Value::Null});
                vec![self.frame("error", data)]
            }
        }
    }
    /// Responses 每个事件都带自增 sequence_number，客户端按它判断丢包。
    fn frame(&mut self, name: &str, mut data: Value) -> SseFrame {
        if let Some(object) = data.as_object_mut() {
            object.insert("type".to_string(), Value::String(name.to_string()));
            object.insert("sequence_number".to_string(), Value::from(self.sequence));
        }
        self.sequence += 1;
        SseFrame {
            event: Some(name.to_string()),
            data: data.to_string(),
        }
    }

    fn created_frames(&mut self) -> Vec<SseFrame> {
        if self.created {
            return Vec::new();
        }
        self.created = true;
        if self.id.is_empty() {
            self.id = ensure_id("resp_", "");
        }
        let mut response = build_envelope(None, &self.id, &self.model, Vec::new(), None);
        response["status"] = Value::String("in_progress".to_string());
        vec![
            self.frame("response.created", json!({"response": response.clone()})),
            self.frame("response.in_progress", json!({"response": response})),
        ]
    }

    /// 返回槽位在 slots 里的下标，顺带补齐 created / output_item.added / part.added。
    fn ensure(&mut self, index: usize, kind: u8, id: &str, name: &str) -> (usize, Vec<SseFrame>) {
        let mut frames = self.created_frames();
        if let Some(position) = self.slots.iter().position(|(key, _)| *key == index) {
            return (position, frames);
        }
        // 一个 output item 要先 done 再开下一个。源协议不一定按块发 stop（Chat 的文本
        // 块要等流结束才关），开新 item 前把还没收尾的先收掉，否则事件时序不合法。
        let pending: Vec<usize> = self
            .slots
            .iter()
            .filter(|(_, slot)| !slot.closed)
            .map(|(key, _)| *key)
            .collect();
        for open in pending {
            frames.extend(self.close(open));
        }
        let output = self.output_next;
        self.output_next += 1;
        let prefix = match kind {
            KIND_REASONING => "rs_",
            KIND_TOOL => tool_item_prefix(&self.shapes, name),
            _ => "msg_",
        };
        let slot = Slot {
            kind,
            output,
            item: item_id(prefix, output),
            id: id.to_string(),
            name: name.to_string(),
            text: String::new(),
            opaque: String::new(),
            closed: false,
        };
        let item = match kind {
            KIND_TOOL => tool_call_item(
                &self.shapes,
                &slot.item,
                "in_progress",
                &slot.id,
                &slot.name,
                "",
            ),
            KIND_REASONING => json!({"id": slot.item, "type": "reasoning", "summary": []}),
            _ => json!({
                "id": slot.item, "type": "message", "status": "in_progress",
                "role": "assistant", "content": [],
            }),
        };
        let part = match kind {
            KIND_TOOL => None,
            KIND_REASONING => Some((
                "response.reasoning_summary_part.added",
                json!({
                    "item_id": slot.item, "output_index": output, "summary_index": 0,
                    "part": {"type": "summary_text", "text": ""},
                }),
            )),
            _ => Some((
                "response.content_part.added",
                json!({
                    "item_id": slot.item, "output_index": output, "content_index": 0,
                    "part": {"type": "output_text", "text": "", "annotations": []},
                }),
            )),
        };
        self.slots.push((index, slot));
        frames.push(self.frame(
            "response.output_item.added",
            json!({"output_index": output, "item": item}),
        ));
        if let Some((name, data)) = part {
            frames.push(self.frame(name, data));
        }
        (self.slots.len() - 1, frames)
    }
    fn delta(&mut self, index: usize, kind: u8, text: &str) -> Vec<SseFrame> {
        let (position, mut frames) = self.ensure(index, kind, "", "");
        if text.is_empty() || self.slots[position].1.closed {
            return frames;
        }
        self.slots[position].1.text.push_str(text);
        let slot = self.slots[position].1.clone();
        let (name, data) = match slot.kind {
            KIND_TOOL => {
                // custom 工具的入参是裸文本，得等参数攒全才剥得出来，中途发不了增量。
                if matches!(shape_of(&self.shapes, &slot.name), Some(Shape::Custom { .. })) {
                    return frames;
                }
                (
                    "response.function_call_arguments.delta",
                    json!({
                        "item_id": slot.item, "output_index": slot.output, "delta": text,
                    }),
                )
            }
            KIND_REASONING => (
                "response.reasoning_summary_text.delta",
                json!({
                    "item_id": slot.item, "output_index": slot.output, "summary_index": 0,
                    "delta": text,
                }),
            ),
            _ => (
                "response.output_text.delta",
                json!({
                    "item_id": slot.item, "output_index": slot.output, "content_index": 0,
                    "delta": text,
                }),
            ),
        };
        frames.push(self.frame(name, data));
        frames
    }
    fn close(&mut self, index: usize) -> Vec<SseFrame> {
        let Some(position) = self.slots.iter().position(|(key, _)| *key == index) else {
            return Vec::new();
        };
        if self.slots[position].1.closed {
            return Vec::new();
        }
        self.slots[position].1.closed = true;
        let slot = self.slots[position].1.clone();
        let mut frames = Vec::new();
        let item = match slot.kind {
            KIND_TOOL => {
                let custom =
                    matches!(shape_of(&self.shapes, &slot.name), Some(Shape::Custom { .. }));
                let arguments = if slot.text.is_empty() && !custom {
                    "{}".to_string()
                } else {
                    slot.text.clone()
                };
                let item = tool_call_item(
                    &self.shapes,
                    &slot.item,
                    "completed",
                    &slot.id,
                    &slot.name,
                    &arguments,
                );
                if custom {
                    // 裸文本一次性发完，客户端按 custom 工具自己的事件名收。
                    let input = str_field(&item, "input");
                    frames.push(self.frame(
                        "response.custom_tool_call_input.delta",
                        json!({
                            "item_id": slot.item, "output_index": slot.output, "delta": input,
                        }),
                    ));
                    frames.push(self.frame(
                        "response.custom_tool_call_input.done",
                        json!({
                            "item_id": slot.item, "output_index": slot.output, "input": input,
                        }),
                    ));
                } else {
                    frames.push(self.frame(
                        "response.function_call_arguments.done",
                        json!({
                            "item_id": slot.item, "output_index": slot.output,
                            "arguments": arguments,
                        }),
                    ));
                }
                item
            }
            KIND_REASONING => {
                frames.push(self.frame(
                    "response.reasoning_summary_text.done",
                    json!({
                        "item_id": slot.item, "output_index": slot.output, "summary_index": 0,
                        "text": slot.text,
                    }),
                ));
                frames.push(self.frame(
                    "response.reasoning_summary_part.done",
                    json!({
                        "item_id": slot.item, "output_index": slot.output, "summary_index": 0,
                        "part": {"type": "summary_text", "text": slot.text},
                    }),
                ));
                json!({
                    "id": slot.item, "type": "reasoning",
                    "summary": [{"type": "summary_text", "text": slot.text}],
                    "encrypted_content": if slot.opaque.is_empty() {
                        Value::Null
                    } else {
                        json!(slot.opaque)
                    },
                })
            }
            _ => {
                frames.push(self.frame(
                    "response.output_text.done",
                    json!({
                        "item_id": slot.item, "output_index": slot.output, "content_index": 0,
                        "text": slot.text,
                    }),
                ));
                frames.push(self.frame(
                    "response.content_part.done",
                    json!({
                        "item_id": slot.item, "output_index": slot.output, "content_index": 0,
                        "part": {"type": "output_text", "text": slot.text, "annotations": []},
                    }),
                ));
                json!({
                    "id": slot.item, "type": "message", "status": "completed",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": slot.text, "annotations": []}],
                })
            }
        };
        frames.push(self.frame(
            "response.output_item.done",
            json!({"output_index": slot.output, "item": item.clone()}),
        ));
        self.done.push(item);
        frames
    }
    fn finish(&mut self, finish: Option<Finish>, usage: &Usage) -> Vec<SseFrame> {
        if self.finished {
            return Vec::new();
        }
        self.finished = true;
        let mut frames = self.created_frames();
        let pending: Vec<usize> = self
            .slots
            .iter()
            .filter(|(_, slot)| !slot.closed)
            .map(|(key, _)| *key)
            .collect();
        for index in pending {
            frames.extend(self.close(index));
        }
        let output = std::mem::take(&mut self.done);
        let response = build_envelope(finish, &self.id, &self.model, output, Some(usage));
        let name = if finish == Some(Finish::Length) {
            "response.incomplete"
        } else {
            "response.completed"
        };
        frames.push(self.frame(name, json!({"response": response})));
        frames
    }
}

