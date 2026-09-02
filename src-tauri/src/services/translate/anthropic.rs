//! Anthropic Messages 与 IR 互转。

use std::collections::{HashMap, HashSet};

use serde_json::{json, Map, Value};

use super::ir::*;
use super::{Format, SseFrame, NO_REPLAY};

fn role_name(role: Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

fn parse_system(value: Option<&Value>) -> String {
    let text = match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .filter(|item| str_field(item, "type") == "text")
            .map(|item| str_field(item, "text"))
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n"),
        _ => String::new(),
    };
    strip_billing_header(&text).to_string()
}

/// Claude Code 会在 system 开头塞一行 `x-anthropic-billing-header:`，里面的 cch=
/// 每次请求都在变。原样带给别家上游会让稳定前缀每轮都不同，前缀缓存整轮失效，
/// 所以只剥开头这一行——后面出现的同名文本是用户自己写的，留着。
const BILLING_HEADER_PREFIX: &str = "x-anthropic-billing-header:";

fn strip_billing_header(text: &str) -> &str {
    if !text.starts_with(BILLING_HEADER_PREFIX) {
        return text;
    }
    let rest = text.split_once('\n').map(|(_, rest)| rest).unwrap_or("");
    rest.strip_prefix("\r\n")
        .or_else(|| rest.strip_prefix('\n'))
        .unwrap_or(rest)
}

fn image_from_source(source: Option<&Value>) -> Option<String> {
    let source = source?;
    match str_field(source, "type").as_str() {
        "base64" => Some(format!(
            "data:{};base64,{}",
            str_field(source, "media_type"),
            str_field(source, "data")
        )),
        "url" => Some(str_field(source, "url")),
        _ => None,
    }
}

fn document_from_block(item: &Value) -> Option<Block> {
    let url = image_from_source(item.get("source"))?;
    let filename = [
        str_field(item, "title"),
        str_field(item, "filename"),
        "document.pdf".to_string(),
    ]
    .into_iter()
    .find(|name| !name.is_empty())
    .unwrap_or_default();
    Some(Block::Document { url, filename })
}

/// tool_result 的 content 可以是字符串或块数组，图片单独收进 media。
fn tool_result_content(value: Option<&Value>) -> (String, Vec<String>) {
    match value {
        Some(Value::String(text)) => (text.clone(), Vec::new()),
        Some(Value::Array(items)) => {
            let mut texts = Vec::new();
            let mut media = Vec::new();
            for item in items {
                match str_field(item, "type").as_str() {
                    "text" => texts.push(str_field(item, "text")),
                    "image" => media.extend(image_from_source(item.get("source"))),
                    // 工具检索的结果，整块内容就只有工具名（没有 text 部件）。丢掉
                    // 会让模型以为检索什么都没返回，和 Responses 侧 tool_search_output
                    // 一样只把名字回给模型。
                    "tool_reference" => texts.push(str_field(item, "tool_name")),
                    _ => {}
                }
            }
            (texts.join("\n"), media)
        }
        _ => (String::new(), Vec::new()),
    }
}

/// `key` 为 None 表示在解析客户端请求：签名只可能是我们自己编进去的信封。为
/// Some 表示在解析上游响应：原生签名连同整块存成 Opaque，供下一轮原路回放。
fn parse_content(value: Option<&Value>, key: Option<&str>) -> Vec<Block> {
    match value {
        Some(Value::String(text)) if !text.is_empty() => vec![Block::Text(text.clone())],
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| parse_block(item, key))
            .collect(),
        _ => Vec::new(),
    }
}

fn parse_thinking(item: &Value, key: Option<&str>) -> Block {
    let text = str_field(item, "thinking");
    let signature = match str_field(item, "type").as_str() {
        "redacted_thinking" => str_field(item, "data"),
        _ => str_field(item, "signature"),
    };
    let opaque = match key {
        // 上游响应：整块原样留着，下一轮回到同一个上游时连签名一起送回去。
        Some(key) if !signature.is_empty() => {
            Some(Opaque::new(Format::Anthropic, key, item.clone()))
        }
        Some(_) => None,
        // 客户端请求：只认我们自己编的信封，原生签名换了上游必被拒，直接丢。
        None => Opaque::decode(&signature),
    };
    Block::Thinking { text, opaque }
}

fn parse_block(item: &Value, key: Option<&str>) -> Option<Block> {
    match str_field(item, "type").as_str() {
        "text" => Some(Block::Text(str_field(item, "text"))),
        "image" => image_from_source(item.get("source")).map(Block::Image),
        "document" => document_from_block(item),
        "thinking" | "redacted_thinking" => Some(parse_thinking(item, key)),
        "tool_use" => Some(Block::ToolUse {
            id: str_field(item, "id"),
            name: str_field(item, "name"),
            input: tool_input(item.get("input")),
        }),
        "tool_result" => {
            let (text, media) = tool_result_content(item.get("content"));
            Some(Block::ToolResult {
                id: str_field(item, "tool_use_id"),
                text,
                is_error: item
                    .get("is_error")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                media,
            })
        }
        // server_tool_use / web_search_tool_result 等服务端工具别家协议没有对应物。
        _ => None,
    }
}


fn parse_tool_choice(value: Option<&Value>) -> Option<ToolChoice> {
    match str_field(value?, "type").as_str() {
        "auto" => Some(ToolChoice::Auto),
        "none" => Some(ToolChoice::None),
        "any" => Some(ToolChoice::Required),
        "tool" => Some(ToolChoice::Tool(str_field(value?, "name"))),
        _ => None,
    }
}

fn parse_finish(reason: &str) -> Option<Finish> {
    match reason {
        "end_turn" | "stop_sequence" | "pause_turn" => Some(Finish::Stop),
        "max_tokens" => Some(Finish::Length),
        "tool_use" => Some(Finish::ToolUse),
        "refusal" => Some(Finish::ContentFilter),
        _ => None,
    }
}

fn finish_name(finish: Option<Finish>) -> &'static str {
    match finish {
        Some(Finish::Length) => "max_tokens",
        Some(Finish::ToolUse) => "tool_use",
        Some(Finish::ContentFilter) => "refusal",
        _ => "end_turn",
    }
}

fn parse_usage(value: Option<&Value>) -> Usage {
    let Some(value) = value else {
        return Usage::default();
    };
    let field = |key: &str| value.get(key).and_then(Value::as_i64).unwrap_or(0);
    Usage {
        input: field("input_tokens"),
        cache_read: field("cache_read_input_tokens"),
        cache_write: field("cache_creation_input_tokens"),
        output: field("output_tokens"),
        // 思考消耗。和 OpenAI 的 reasoning_tokens 是同一件事：都已经算在 output_tokens
        // 里，只是明细。不读的话思考量转到 OpenAI 侧客户端就永远是 0。
        reasoning: value
            .pointer("/output_tokens_details/thinking_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(0),
    }
}

fn build_usage(usage: &Usage) -> Value {
    json!({
        "input_tokens": usage.input,
        "cache_read_input_tokens": usage.cache_read,
        "cache_creation_input_tokens": usage.cache_write,
        "output_tokens": usage.output,
    })
}

fn error_message(data: &Value) -> String {
    data.pointer("/error/message")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| data.to_string())
}

pub fn parse_request(body: &Value) -> ChatRequest {
    let mut request = ChatRequest {
        model: str_field(body, "model"),
        system: parse_system(body.get("system")),
        max_tokens: body.get("max_tokens").and_then(Value::as_i64),
        temperature: body.get("temperature").and_then(Value::as_f64),
        top_p: body.get("top_p").and_then(Value::as_f64),
        stop: string_list(body.get("stop_sequences")),
        stream: body.get("stream").and_then(Value::as_bool).unwrap_or(false),
        ..Default::default()
    };
    // thinking.type 是 disabled 时客户端明确不要思考——只看 effort 会把用户关掉的思考在
    // 下游重新打开，多花钱也多花时间。老写法 `thinking.budget_tokens` 不认：走互转的只有
    // Claude Code 和 Codex，两家都只发档位词（实测最近一周 3707 条请求里 budget_tokens
    // 0 条）。多协议客户端（opencode 这类）本来就不需要互转。
    let thinking_disabled =
        body.pointer("/thinking/type").and_then(Value::as_str) == Some("disabled");
    request.effort = (!thinking_disabled)
        .then(|| {
            body.pointer("/output_config/effort")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .flatten();
    request.response_format = parse_output_format(body.pointer("/output_config/format"));
    if body
        .pointer("/tool_choice/disable_parallel_tool_use")
        .and_then(Value::as_bool)
        == Some(true)
    {
        request.parallel_tool_calls = Some(false);
    }
    for message in array(body.get("messages")) {
        let role = match str_field(message, "role").as_str() {
            "assistant" => Role::Assistant,
            _ => Role::User,
        };
        let blocks = parse_content(message.get("content"), None);
        if !blocks.is_empty() {
            request.messages.push(Message { role, blocks });
        }
    }
    for tool in array(body.get("tools")) {
        let name = str_field(tool, "name");
        // 服务端工具（web_search 等）没有 input_schema，别家协议也没有对应物。
        let Some(schema) = tool.get("input_schema").filter(|value| value.is_object()) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        request.tools.push(Tool {
            name,
            description: tool
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_string),
            schema: schema.clone(),
        });
    }
    request.tool_choice = parse_tool_choice(body.get("tool_choice"));
    request
}

/// Anthropic 的 `output_config.format` 和 OpenAI 两家的结构化输出说的是同一件事，
/// 形状也已经是 IR 的扁平形状，直接搬。认不出的类型丢掉，不猜。
fn parse_output_format(value: Option<&Value>) -> Option<Value> {
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

fn image_source(url: &str) -> Value {
    match split_data_url(url) {
        Some((mime, data)) => json!({"type": "base64", "media_type": mime, "data": data}),
        None => json!({"type": "url", "url": url}),
    }
}

fn build_block(block: &Block, key: &str) -> Option<Value> {
    match block {
        Block::Text(text) if !text.trim().is_empty() => {
            Some(json!({"type": "text", "text": text}))
        }
        Block::Image(url) => Some(json!({"type": "image", "source": image_source(url)})),
        Block::Document { url, filename } => Some(json!({
            "type": "document",
            "source": image_source(url),
            "title": filename,
        })),
        // 只回放确实来自这个上游的原始 thinking 块，签名才验得过。
        Block::Thinking { opaque, .. } => opaque
            .as_ref()
            .and_then(|opaque| opaque.replay(Format::Anthropic, key))
            .cloned(),
        Block::ToolUse { id, name, input } => Some(json!({
            "type": "tool_use",
            "id": id,
            "name": name,
            "input": tool_input_object(input),
        })),
        Block::ToolResult {
            id,
            text,
            is_error,
            media,
        } => {
            let mut content = Vec::new();
            if !text.trim().is_empty() {
                content.push(json!({"type": "text", "text": text}));
            } else if media.is_empty() {
                // 空文本块会被 Anthropic 拒，工具确实没输出时（写文件、静默的命令）
                // 补个占位，不能留空。
                content.push(json!({"type": "text", "text": TOOL_NO_OUTPUT}));
            }
            for url in media {
                content.push(json!({"type": "image", "source": image_source(url)}));
            }
            let mut item = json!({"type": "tool_result", "tool_use_id": id, "content": content});
            if *is_error {
                item["is_error"] = json!(true);
            }
            Some(item)
        }
        // 空文本会被 Anthropic 拒绝。
        Block::Text(_) => None,
    }
}

/// 响应侧的 thinking 带上信封：客户端下一轮回传时 parse 能还原出 Opaque，原路
/// 回到同一个上游就能继续验签，换了上游会被丢掉。上游压根没给签名的，写占位信封，
/// 好让下一轮认出这是网关造的块。
fn build_output_block(block: &Block) -> Option<Value> {
    match block {
        Block::Thinking { text, opaque } => {
            if text.is_empty() {
                // 只有密文没有摘要，对应 Anthropic 的 redacted_thinking；连密文都没有
                // 就没什么可带回去的了。
                return opaque
                    .as_ref()
                    .map(|opaque| json!({"type": "redacted_thinking", "data": opaque.encode()}));
            }
            let signature = opaque
                .as_ref()
                .map(Opaque::encode)
                .unwrap_or_else(placeholder_signature);
            Some(json!({"type": "thinking", "thinking": text, "signature": signature}))
        }
        _ => build_block(block, ""),
    }
}

fn build_messages(messages: &[Message], key: &str) -> Vec<Value> {
    let mut merged: Vec<(Role, Vec<Value>)> = Vec::new();
    for message in messages {
        let blocks: Vec<Value> = message
            .blocks
            .iter()
            .filter_map(|block| build_block(block, key))
            .collect();
        if blocks.is_empty() {
            continue;
        }
        match merged.last_mut() {
            // Anthropic 要求 user / assistant 交替，同角色必须合并成一条。
            Some((role, existing)) if *role == message.role => existing.extend(blocks),
            _ => merged.push((message.role, blocks)),
        }
    }
    // 末条是 assistant 就是 prefill，Anthropic 要求它的正文不能以空白结尾。
    if let Some((Role::Assistant, blocks)) = merged.last_mut() {
        trim_prefill(blocks);
    }
    if merged.last().is_some_and(|(_, blocks)| blocks.is_empty()) {
        merged.pop();
    }
    // 首条必须是 user，也不能一条都没有，否则 Anthropic 直接 400。
    if !matches!(merged.first(), Some((Role::User, _))) {
        merged.insert(0, (Role::User, vec![json!({"type": "text", "text": "."})]));
    }
    merged
        .into_iter()
        .map(|(role, blocks)| json!({"role": role_name(role), "content": blocks}))
        .collect()
}

/// 去掉 prefill 末尾文本块的尾随空白。整块空了就丢掉——空文本块同样会被拒。
fn trim_prefill(blocks: &mut Vec<Value>) {
    let Some(last) = blocks.last_mut() else {
        return;
    };
    if last.get("type").and_then(Value::as_str) != Some("text") {
        return;
    }
    let trimmed = str_field(last, "text").trim_end().to_string();
    if trimmed.is_empty() {
        blocks.pop();
    } else {
        last["text"] = json!(trimmed);
    }
}

/// 开启 extended thinking 后，最后一轮 assistant 若带 tool_use，Anthropic 会要求
/// 同一条消息里有验得过的 thinking 块，缺了就 400。拿不到就干脆不开。
fn thinking_replay_ok(messages: &[Message], key: &str) -> bool {
    let Some(message) = messages
        .iter()
        .rev()
        .find(|message| message.role == Role::Assistant)
    else {
        return true;
    };
    if !message
        .blocks
        .iter()
        .any(|block| matches!(block, Block::ToolUse { .. }))
    {
        return true;
    }
    message.blocks.iter().any(|block| {
        matches!(block, Block::Thinking { opaque: Some(opaque), .. }
            if opaque.replay(Format::Anthropic, key).is_some())
    })
}

/// 直通路径（客户端与上游同协议、不走转换）上清掉历史里失效的思考块。`drop_all`
/// 为真是事后整流，全部剥掉重试；否则只剥签名是网关信封的那些——信封是上一轮走过
/// 转换留下的，原生 Anthropic 上游一律验不过。返回是否真的改动过。
pub fn strip_thinking(body: &mut Value, drop_all: bool) -> bool {
    let mut changed = false;
    if let Some(messages) = body.get_mut("messages").and_then(Value::as_array_mut) {
        for message in messages.iter_mut() {
            if let Some(content) = message.get_mut("content").and_then(Value::as_array_mut) {
                let before = content.len();
                content.retain(|block| !stale_thinking(block, drop_all));
                changed |= content.len() != before;
            }
        }
        // 内容被剥空的消息也得丢掉，Anthropic 拒绝空 content。
        messages.retain(|message| {
            message
                .get("content")
                .and_then(Value::as_array)
                .map_or(true, |content| !content.is_empty())
        });
    }
    // 剥完最后一条 assistant 仍带 tool_use 却没剩下思考块，就得一并关掉思考：开着的
    // 话 Anthropic 会要求同一条消息里有验得过的思考块，缺了直接 400。这一步和上面有没
    // 有剥到东西无关——「必须以思考块开头」这类拒绝往往本来就没有块可剥，真正要做的
    // 就是关开关，提前收工等于整流做了一半。
    let orphan_tool_use = body
        .get("messages")
        .and_then(Value::as_array)
        .and_then(|messages| {
            messages
                .iter()
                .rev()
                .find(|message| str_field(message, "role") == "assistant")
        })
        .is_some_and(|message| {
            let blocks = array(message.get("content"));
            blocks
                .iter()
                .any(|block| str_field(block, "type") == "tool_use")
                && !blocks.iter().any(|block| {
                    matches!(
                        str_field(block, "type").as_str(),
                        "thinking" | "redacted_thinking"
                    )
                })
        });
    if orphan_tool_use {
        changed |= disable_thinking(body);
    }
    changed
}

/// 关掉思考。开关有两个：老写法顶层 `thinking`，新写法 `output_config.effort`（新版
/// Claude Code 两个一起发），只关一个等于没关。`output_config` 里别的字段（format 等）
/// 要留着，清空了才把它整个删掉。
fn disable_thinking(body: &mut Value) -> bool {
    // 客户端已经明确写了 disabled 就别删那个字段：删掉等于把「不要思考」这句话也一起
    // 删了，默认开思考的上游会把它重新打开。
    let explicit_off = body.pointer("/thinking/type").and_then(Value::as_str) == Some("disabled");
    let Some(object) = body.as_object_mut() else {
        return false;
    };
    let mut changed = !explicit_off && object.remove("thinking").is_some();
    if let Some(config) = object.get_mut("output_config").and_then(Value::as_object_mut) {
        changed |= config.remove("effort").is_some();
        if config.is_empty() {
            object.remove("output_config");
        }
    }
    changed
}

/// 这个块该不该剥。整流之外只认网关信封：`thinking` 装在 signature 里，
/// `redacted_thinking` 装在 data 里。上游没给签名时网关写的是
/// [`placeholder_signature`]，也是信封，所以这里一并认出来。真的空签名不剥——那是
/// 兼容端点自己回的思考块，剥了等于白丢一段历史。
fn stale_thinking(block: &Value, drop_all: bool) -> bool {
    let field = match str_field(block, "type").as_str() {
        "thinking" => "signature",
        "redacted_thinking" => "data",
        _ => return false,
    };
    drop_all || Opaque::is_envelope(&str_field(block, field))
}

/// 目标是 Anthropic 时源协议不会带 cache_control，自己补断点：tools 尾部和 system
/// 是稳定前缀，最后一条消息让下一轮命中缓存。Anthropic 最多允许 4 个。
fn inject_cache_control(body: &mut Map<String, Value>) {
    let mark = |value: &mut Value| {
        if let Some(object) = value.as_object_mut() {
            object.insert("cache_control".to_string(), json!({"type": "ephemeral"}));
        }
    };
    if let Some(last) = body
        .get_mut("tools")
        .and_then(Value::as_array_mut)
        .and_then(|items| items.last_mut())
    {
        mark(last);
    }
    if let Some(system) = body.get("system").and_then(Value::as_str) {
        body.insert(
            "system".to_string(),
            json!([{
                "type": "text",
                "text": system,
                "cache_control": {"type": "ephemeral"},
            }]),
        );
    }
    // cache_control 不能挂在 thinking / redacted_thinking 上，往前找第一个能挂的块。
    if let Some(last) = body
        .get_mut("messages")
        .and_then(Value::as_array_mut)
        .and_then(|items| items.last_mut())
        .and_then(|message| message.get_mut("content"))
        .and_then(Value::as_array_mut)
        .and_then(|items| {
            items.iter_mut().rev().find(|item| {
                !matches!(
                    item.get("type").and_then(Value::as_str),
                    Some("thinking" | "redacted_thinking")
                )
            })
        })
    {
        mark(last);
    }
}

/// 能写成 Anthropic 原生 `output_config.format` 的形状。只认实测见过的
/// `{"type": "json_schema", "schema": …}`：`name` / `strict` 是 OpenAI 侧的字段，
/// Anthropic 的真实请求里没出现过，多塞一个猜错就是整个请求 400。认不出的类型
/// （`json_object` 没见过原生形状）返回 None，由调用方退回 [`schema_instruction`]。
fn native_output_format(format: &Value) -> Option<Value> {
    let schema = format
        .get("schema")
        .filter(|_| str_field(format, "type") == "json_schema")?;
    Some(json!({"type": "json_schema", "schema": object_schema(schema)}))
}

/// 写不成原生 `output_config.format` 的类型，约束只能写进 system，否则会被静默丢掉。
fn schema_instruction(format: &Value) -> Option<String> {
    match str_field(format, "type").as_str() {
        "json_object" => Some("只输出一个 JSON 对象，不要带解释文字或代码块围栏。".to_string()),
        "json_schema" => Some(format!(
            "只输出符合以下 JSON Schema 的 JSON，不要带解释文字或代码块围栏：\n{}",
            format.get("schema").unwrap_or(&Value::Null)
        )),
        _ => None,
    }
}

/// Anthropic 要求 `messages` 里出现的每个 tool_use 都能在 `tools` 里找到定义，否则 400
/// No such tool；落单的 tool_result（找不到对应调用）同样会被拒。两种情况都把工具块摊平
/// 成文本：历史内容一点不丢，也不用凭空编出工具声明——编出来的假工具模型下一轮会真的去调。
///
/// 触发场景两类：本轮压根不声明工具（Codex 的上下文压缩请求就是这样，把几十轮历史发过来
/// 只为让模型做交接总结），以及清单中途变了（MCP 服务器掉线、用户关掉某个工具），历史里
/// 留着已经不存在的那个调用。**只摊平对不上的那些**，别的工具块照原样留着——全表摊平会
/// 让本来能用的工具在这一轮集体失效。
///
/// 没有对不上的块时返回 None，调用方直接用原消息，不白拷一遍。
fn flatten_tool_blocks(messages: &[Message], tools: &[Tool]) -> Option<Vec<Message>> {
    let blocks = || messages.iter().flat_map(|message| message.blocks.iter());
    let calls: HashSet<&str> = blocks()
        .filter_map(|block| match block {
            Block::ToolUse { id, .. } => Some(id.as_str()),
            _ => None,
        })
        .collect();
    // 配对的另一半必须跟着走，只摊平一半照样过不去。
    let stale: HashSet<&str> = blocks()
        .filter_map(|block| match block {
            Block::ToolUse { id, name, .. } => {
                (!tools.iter().any(|tool| tool.name == *name)).then_some(id.as_str())
            }
            Block::ToolResult { id, .. } => (!calls.contains(id.as_str())).then_some(id.as_str()),
            _ => None,
        })
        .collect();
    if stale.is_empty() {
        return None;
    }
    Some(
        messages
            .iter()
            .map(|message| Message {
                role: message.role,
                blocks: message
                    .blocks
                    .iter()
                    .flat_map(|block| match block {
                        Block::ToolUse { id, name, input } if stale.contains(id.as_str()) => {
                            vec![Block::Text(format!(
                                "[ccg: 调用工具 {name}]\n{}",
                                tool_input_string(input)
                            ))]
                        }
                        Block::ToolResult {
                            id,
                            text,
                            is_error,
                            media,
                        } if stale.contains(id.as_str()) => {
                            let label = if *is_error { "工具报错" } else { "工具结果" };
                            let body = if text.trim().is_empty() {
                                TOOL_NO_OUTPUT
                            } else {
                                text.as_str()
                            };
                            let mut blocks = vec![Block::Text(format!("[ccg: {label}]\n{body}"))];
                            blocks.extend(media.iter().cloned().map(Block::Image));
                            blocks
                        }
                        other => vec![other.clone()],
                    })
                    .collect(),
            })
            .collect(),
    )
}

/// 客户端说的「不要思考」。OpenAI 两家有 `none` / `minimal` 这两个档，Anthropic 的
/// `output_config.effort` 里没有对应取值，原样透传就是 400，只能落到 `thinking.disabled`。
/// 其余取值一律原样透传：Anthropic 与 Codex 的档位集合（low / medium / high / xhigh /
/// max）是同一套，不猜、也不压档。
fn no_thinking(effort: &str) -> bool {
    matches!(
        effort.to_ascii_lowercase().as_str(),
        "none" | "minimal" | "off"
    )
}

pub fn build_request(request: &ChatRequest, key: &str, default_max_tokens: i64) -> Value {
    let flattened = flatten_tool_blocks(&request.messages, &request.tools);
    let messages = flattened.as_deref().unwrap_or(&request.messages);
    // Anthropic 的 max_tokens 是必填，源协议没给就用网关设置里的默认值。
    let max_tokens = request.max_tokens.unwrap_or(default_max_tokens);
    let forced_tool = matches!(
        request.tool_choice,
        Some(ToolChoice::Required | ToolChoice::Tool(_))
    );
    // 思考在 Anthropic 侧有两个旋钮，语义不同：`thinking.type` 管开关，
    // `output_config.effort` 管用多大力（实测 284 条真实请求同时发 `disabled` +
    // `effort: high`，两者独立）。要写就两个一起写：只写 effort 等于没控制开关，模型按
    // 默认走（现在的模型默认是开的）；只写开关又丢掉了客户端说的强度。
    //
    // 强制工具调用与 extended thinking 互斥，Anthropic 会直接拒绝；签名回放不成立时也
    // 不能开（见 `thinking_replay_ok`）。这些情况一律显式写 `disabled`，不是省掉
    // `thinking` 字段——省掉等于交给上游默认，而默认是开的，历史里又没有可验签的思考块，
    // 照样 400。
    let effort = request
        .effort
        .as_deref()
        .map(str::trim)
        .filter(|effort| !effort.is_empty() && !no_thinking(effort))
        .filter(|_| !forced_tool)
        .filter(|_| thinking_replay_ok(messages, key));
    // thinking 没开就不能回放历史思考块：带着签名块又没开思考，请求自相矛盾。
    let message_key = if effort.is_some() { key } else { NO_REPLAY };

    // 结构化输出优先写原生 output_config.format；写不了的类型退回系统提示，不然约束会
    // 静默消失。两条路只走一条，别同时发——同一个要求说两遍是噪音。
    let output_format = request.response_format.as_ref().and_then(native_output_format);

    let mut system = request.system.clone();
    if output_format.is_none() {
        if let Some(instruction) = request.response_format.as_ref().and_then(schema_instruction) {
            if !system.is_empty() {
                system.push_str("\n\n");
            }
            system.push_str(&instruction);
        }
    }

    let mut body = Map::new();
    body.insert("model".to_string(), json!(request.model));
    body.insert("max_tokens".to_string(), json!(max_tokens));
    if !system.is_empty() {
        body.insert("system".to_string(), json!(system));
    }
    body.insert(
        "messages".to_string(),
        Value::Array(build_messages(messages, message_key)),
    );
    if !request.tools.is_empty() {
        let tools = request
            .tools
            .iter()
            .map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description.clone().unwrap_or_default(),
                    "input_schema": object_schema(&tool.schema),
                })
            })
            .collect();
        body.insert("tools".to_string(), Value::Array(tools));
    }
    // 没有 tools 时带 tool_choice 会被 Anthropic 拒绝。关掉并行调用只能挂在
    // tool_choice 上，所以客户端只关并行、没给 tool_choice 时补一个 auto——auto
    // 本来就是 Anthropic 的默认值，多写一个不改变行为。
    if !request.tools.is_empty()
        && (request.tool_choice.is_some() || request.parallel_tool_calls == Some(false))
    {
        let mut value = match &request.tool_choice {
            Some(ToolChoice::None) => json!({"type": "none"}),
            Some(ToolChoice::Required) => json!({"type": "any"}),
            Some(ToolChoice::Tool(name)) => json!({"type": "tool", "name": name}),
            Some(ToolChoice::Auto) | None => json!({"type": "auto"}),
        };
        if request.parallel_tool_calls == Some(false) {
            value["disable_parallel_tool_use"] = json!(true);
        }
        body.insert("tool_choice".to_string(), value);
    }
    // 原生结构化输出（形状在 native_output_format 里限定）和思考档位共用 output_config。
    let mut output_config = Map::new();
    if let Some(format) = output_format {
        output_config.insert("format".to_string(), format);
    }
    match effort {
        Some(effort) => {
            // 档位原样透传：Anthropic 的取值范围（low / medium / high / xhigh / max）和
            // Codex 发的是同一套，不用换算也不用白名单。
            output_config.insert("effort".to_string(), json!(effort));
            body.insert("thinking".to_string(), json!({"type": "adaptive"}));
        }
        None => {
            body.insert("thinking".to_string(), json!({"type": "disabled"}));
            // 关了思考才能调 temperature / top_p。
            if let Some(value) = request.temperature {
                body.insert("temperature".to_string(), json!(value));
            }
            if let Some(value) = request.top_p {
                body.insert("top_p".to_string(), json!(value));
            }
        }
    }
    if !output_config.is_empty() {
        body.insert("output_config".to_string(), Value::Object(output_config));
    }
    if !request.stop.is_empty() {
        body.insert("stop_sequences".to_string(), json!(request.stop));
    }
    if request.stream {
        body.insert("stream".to_string(), json!(true));
    }
    inject_cache_control(&mut body);
    Value::Object(body)
}

pub fn parse_response(body: &Value, key: &str) -> ChatResponse {
    ChatResponse {
        id: str_field(body, "id"),
        model: str_field(body, "model"),
        blocks: parse_content(body.get("content"), Some(key)),
        finish: parse_finish(&str_field(body, "stop_reason")),
        usage: parse_usage(body.get("usage")),
    }
}

pub fn build_response(response: &ChatResponse) -> Value {
    let content: Vec<Value> = response
        .blocks
        .iter()
        .filter_map(build_output_block)
        .collect();
    json!({
        "id": ensure_id("msg_", &response.id),
        "type": "message",
        "role": "assistant",
        "model": response.model,
        "content": content,
        "stop_reason": finish_name(response.finish),
        "stop_sequence": Value::Null,
        "usage": build_usage(&response.usage),
    })
}

fn index_of(data: &Value) -> usize {
    data.get("index").and_then(Value::as_u64).unwrap_or(0) as usize
}

fn text_event(index: usize, text: String, thinking: bool) -> Vec<StreamEvent> {
    if text.is_empty() {
        Vec::new()
    } else if thinking {
        vec![StreamEvent::ThinkingDelta { index, text }]
    } else {
        vec![StreamEvent::TextDelta { index, text }]
    }
}

/// 流式 thinking 块的累积状态：签名分片攒齐后在块结束时打包成 Opaque。
#[derive(Default)]
struct ThinkingState {
    redacted: bool,
    text: String,
    signature: String,
}

#[derive(Default)]
pub struct Decoder {
    key: String,
    id: String,
    model: String,
    started: bool,
    stopped: bool,
    /// 见过协议层的终止标记（message_stop，或带 stop_reason 的 message_delta）。
    /// 只有传输层 EOF 不算，否则截断的流会被补成一次成功的空回答。
    received_stop: bool,
    usage: Usage,
    finish: Option<Finish>,
    thinking: HashMap<usize, ThinkingState>,
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
        match kind.as_str() {
            "message_start" => {
                let message = data.get("message");
                self.id = message.map(|value| str_field(value, "id")).unwrap_or_default();
                self.model = message
                    .map(|value| str_field(value, "model"))
                    .unwrap_or_default();
                self.usage
                    .merge(&parse_usage(message.and_then(|value| value.get("usage"))));
                self.started = true;
                vec![StreamEvent::Start {
                    id: self.id.clone(),
                    model: self.model.clone(),
                }]
            }
            "content_block_start" => {
                let index = index_of(data);
                let block = data.get("content_block");
                let field = |key: &str| block.map(|value| str_field(value, key)).unwrap_or_default();
                match field("type").as_str() {
                    "tool_use" => vec![StreamEvent::ToolStart {
                        index,
                        id: field("id"),
                        name: field("name"),
                    }],
                    "text" => text_event(index, field("text"), false),
                    "thinking" => {
                        let text = field("thinking");
                        self.thinking.insert(
                            index,
                            ThinkingState {
                                redacted: false,
                                text: text.clone(),
                                signature: field("signature"),
                            },
                        );
                        text_event(index, text, true)
                    }
                    // 密文思考只有 data，没有可展示的摘要。
                    "redacted_thinking" => {
                        self.thinking.insert(
                            index,
                            ThinkingState {
                                redacted: true,
                                signature: field("data"),
                                ..Default::default()
                            },
                        );
                        Vec::new()
                    }
                    _ => Vec::new(),
                }
            }
            "content_block_delta" => {
                let index = index_of(data);
                let delta = data.get("delta");
                let field = |key: &str| delta.map(|value| str_field(value, key)).unwrap_or_default();
                match field("type").as_str() {
                    "text_delta" => text_event(index, field("text"), false),
                    "thinking_delta" => {
                        let text = field("thinking");
                        if let Some(state) = self.thinking.get_mut(&index) {
                            state.text.push_str(&text);
                        }
                        text_event(index, text, true)
                    }
                    "signature_delta" => {
                        self.thinking
                            .entry(index)
                            .or_default()
                            .signature
                            .push_str(&field("signature"));
                        Vec::new()
                    }
                    "input_json_delta" => vec![StreamEvent::ToolDelta {
                        index,
                        json: field("partial_json"),
                    }],
                    _ => Vec::new(),
                }
            }
            "content_block_stop" => {
                let index = index_of(data);
                let mut events = Vec::new();
                // 攒齐的思考块原样打包，客户端下一轮回传时才能还原出可验签的载荷。
                if let Some(state) = self
                    .thinking
                    .remove(&index)
                    .filter(|state| !state.signature.is_empty())
                {
                    let payload = if state.redacted {
                        json!({"type": "redacted_thinking", "data": state.signature})
                    } else {
                        json!({
                            "type": "thinking",
                            "thinking": state.text,
                            "signature": state.signature,
                        })
                    };
                    events.push(StreamEvent::ThinkingOpaque {
                        index,
                        opaque: Opaque::new(Format::Anthropic, &self.key, payload),
                    });
                }
                events.push(StreamEvent::BlockStop { index });
                events
            }
            "message_delta" => {
                if let Some(reason) = data.pointer("/delta/stop_reason").and_then(Value::as_str) {
                    self.finish = parse_finish(reason);
                    self.received_stop = true;
                }
                self.usage.merge(&parse_usage(data.get("usage")));
                Vec::new()
            }
            "message_stop" => {
                self.received_stop = true;
                self.finish()
            }
            "error" => {
                self.stopped = true;
                vec![StreamEvent::Error {
                    message: error_message(data),
                }]
            }
            _ => Vec::new(),
        }
    }

    pub fn finish(&mut self) -> Vec<StreamEvent> {
        if self.stopped {
            return Vec::new();
        }
        self.stopped = true;
        // 没见过终止标记就是流被截断了，不能替上游补一次成功结束。
        if !self.received_stop {
            return vec![StreamEvent::Error {
                message: if self.started {
                    "Stream ended without message_stop".to_string()
                } else {
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
        events.push(StreamEvent::Finish {
            finish: self.finish,
            usage: self.usage.clone(),
        });
        events
    }
}

fn frame(name: &str, data: Value) -> SseFrame {
    SseFrame {
        event: Some(name.to_string()),
        data: data.to_string(),
    }
}

/// IR 事件 -> Anthropic SSE。IR 的块序号按首次出现顺序重新映射成 Anthropic 的
/// 连续 index，并补齐 message_start / content_block_start 这些包裹事件。
#[derive(Default)]
pub struct Encoder {
    id: String,
    model: String,
    started: bool,
    done: bool,
    slots: HashMap<usize, usize>,
    open_slots: Vec<usize>,
    next: usize,
    /// 还没拿到签名的思考块。关块前补一个占位信封，见 [`placeholder_signature`]。
    unsigned: HashSet<usize>,
}

impl Encoder {
    pub fn encode(&mut self, event: &StreamEvent) -> Vec<SseFrame> {
        match event {
            StreamEvent::Start { id, model } => {
                self.id = ensure_id("msg_", id);
                self.model = model.clone();
                // 立刻发 message_start，别等第一个内容增量：客户端和网关的首字节 /
                // 空闲计时都盯着这条流，上游光是思考就够把超时耗完。
                self.start()
            }
            StreamEvent::TextDelta { index, text } => {
                self.delta(*index, "text", json!({"type": "text_delta", "text": text}))
            }
            StreamEvent::ThinkingDelta { index, text } => {
                self.unsigned.insert(*index);
                self.delta(
                    *index,
                    "thinking",
                    json!({"type": "thinking_delta", "thinking": text}),
                )
            }
            // 信封塞进签名字段带给客户端，下一轮回传时能原样还原。
            StreamEvent::ThinkingOpaque { index, opaque } => {
                self.unsigned.remove(index);
                self.delta(
                    *index,
                    "thinking",
                    json!({"type": "signature_delta", "signature": opaque.encode()}),
                )
            }
            StreamEvent::ToolStart { index, id, name } => {
                // 工具 id 原样透传：客户端下一轮会带着它回来，改写会对不上上游。
                self.open(
                    *index,
                    json!({"type": "tool_use", "id": id, "name": name, "input": {}}),
                )
            }
            StreamEvent::ToolDelta { index, json: partial } => self.delta(
                *index,
                "tool_use",
                json!({"type": "input_json_delta", "partial_json": partial}),
            ),
            StreamEvent::BlockStop { index } => self.close(*index),
            StreamEvent::Finish { finish, usage } => self.finish(*finish, usage),
            StreamEvent::Error { message } => vec![frame(
                "error",
                json!({"type": "error", "error": {"type": "api_error", "message": message}}),
            )],
        }
    }

    /// message_start 之前和 message_stop 之后都不能插 ping，客户端会当成协议错误。
    pub fn keepalive(&self) -> Option<SseFrame> {
        (self.started && !self.done).then(|| frame("ping", json!({"type": "ping"})))
    }

    fn start(&mut self) -> Vec<SseFrame> {
        if self.started {
            return Vec::new();
        }
        self.started = true;
        if self.id.is_empty() {
            self.id = ensure_id("msg_", "");
        }
        vec![frame(
            "message_start",
            json!({"type": "message_start", "message": {
                "id": self.id, "type": "message", "role": "assistant", "model": self.model,
                "content": [], "stop_reason": Value::Null, "stop_sequence": Value::Null,
                "usage": {"input_tokens": 0, "output_tokens": 0},
            }}),
        )]
    }

    fn open(&mut self, index: usize, block: Value) -> Vec<SseFrame> {
        let mut frames = self.start();
        if self.slots.contains_key(&index) {
            return frames;
        }
        // Anthropic 同一时刻只允许一个块打开。源协议不一定按块发 stop（Chat 的文本
        // 块要等流结束才关），开新块前把还开着的先关掉，否则事件时序不合法。
        frames.extend(self.close_open());
        let slot = self.next;
        self.next += 1;
        self.slots.insert(index, slot);
        self.open_slots.push(slot);
        frames.push(frame(
            "content_block_start",
            json!({"type": "content_block_start", "index": slot, "content_block": block}),
        ));
        frames
    }

    fn delta(&mut self, index: usize, kind: &str, delta: Value) -> Vec<SseFrame> {
        let block = match kind {
            "thinking" => json!({"type": "thinking", "thinking": "", "signature": ""}),
            "tool_use" => json!({"type": "tool_use", "id": "", "name": "", "input": {}}),
            _ => json!({"type": "text", "text": ""}),
        };
        let mut frames = self.open(index, block);
        let slot = self.slots.get(&index).copied().unwrap_or(0);
        frames.push(frame(
            "content_block_delta",
            json!({"type": "content_block_delta", "index": slot, "delta": delta}),
        ));
        frames
    }

    fn close(&mut self, index: usize) -> Vec<SseFrame> {
        let Some(slot) = self.slots.get(&index).copied() else {
            return Vec::new();
        };
        let Some(position) = self.open_slots.iter().position(|item| *item == slot) else {
            return Vec::new();
        };
        self.open_slots.remove(position);
        let mut frames = self.sign(index, slot);
        frames.push(frame(
            "content_block_stop",
            json!({"type": "content_block_stop", "index": slot}),
        ));
        frames
    }

    /// 关掉所有还开着的块。
    fn close_open(&mut self) -> Vec<SseFrame> {
        let mut frames = Vec::new();
        for slot in std::mem::take(&mut self.open_slots) {
            let index = self
                .slots
                .iter()
                .find(|(_, value)| **value == slot)
                .map(|(key, _)| *key);
            if let Some(index) = index {
                frames.extend(self.sign(index, slot));
            }
            frames.push(frame(
                "content_block_stop",
                json!({"type": "content_block_stop", "index": slot}),
            ));
        }
        frames
    }

    /// 上游没给签名的思考块，关块前补一个占位签名，好让下一轮的直通路径认出这是网关
    /// 造的块并剥掉——原生 Anthropic 不收验不过签的思考块。
    fn sign(&mut self, index: usize, slot: usize) -> Vec<SseFrame> {
        if !self.unsigned.remove(&index) {
            return Vec::new();
        }
        vec![frame(
            "content_block_delta",
            json!({"type": "content_block_delta", "index": slot,
                "delta": {"type": "signature_delta", "signature": placeholder_signature()}}),
        )]
    }

    fn finish(&mut self, finish: Option<Finish>, usage: &Usage) -> Vec<SseFrame> {
        if self.done {
            return Vec::new();
        }
        self.done = true;
        let mut frames = self.start();
        frames.extend(self.close_open());
        // 真实 Anthropic 只在这里给 output_tokens；把整份用量都放进来，网关按客户端
        // 协议统计用量时才拿得到输入和缓存部分。
        frames.push(frame(
            "message_delta",
            json!({"type": "message_delta",
                "delta": {"stop_reason": finish_name(finish), "stop_sequence": Value::Null},
                "usage": build_usage(usage)}),
        ));
        frames.push(frame("message_stop", json!({"type": "message_stop"})));
        frames
    }
}

