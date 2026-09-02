//! Anthropic Messages / OpenAI Chat / OpenAI Responses 三方共用的中间表示。
//!
//! 转换一律走「源协议 -> IR -> 目标协议」，避免 3x3 两两适配。IR 只保留三方都
//! 能表达的内容；各家私有的不可跨协议状态（thinking 签名、reasoning 密文）装进
//! [`Opaque`] 原样携带，并记下产生它的协议与上游标识，只有原路回到同一个上游才
//! 回放，其余情况一律丢弃——换一家上游原样回传会被判为非法签名。

use serde_json::{json, Value};

use super::Format;

/// 目标协议要求 max_tokens 但源请求没给时使用的兜底值，也是网关设置项
/// `translate_max_tokens` 的默认值。Anthropic 的 `max_tokens` 是必填，而 Chat 的
/// `max_tokens` / Responses 的 `max_output_tokens` 都是选填——实测 9397 条 Codex 请求里
/// 9372 条压根不带，所以这个兜底是每次转换都要用到的，不能省。32000 是 Claude 4 系列都
/// 接受的上限，只有 3.5 及更早的老模型上限更低（8192 / 4096），那些模型要靠客户端自己
/// 传 max_tokens。
pub const DEFAULT_MAX_TOKENS: i64 = 32000;

/// 工具结果里的图片挪到后续 user 消息时留的说明，免得模型以为图片凭空出现。
pub const TOOL_MEDIA_MOVED: &str = "[ccg: 上一条工具结果里的图片附在本条消息中]";

/// 工具确实没有输出时（写文件、静默的命令）的占位。空字符串会让模型以为工具挂了，
/// Anthropic 更是直接拒空文本块。
pub const TOOL_NO_OUTPUT: &str = "(no output)";

const ENVELOPE_PREFIX: &str = "ccg-opaque-v1:";

/// 上游私有、跨协议无意义的原始载荷：Anthropic 的 thinking 块（含签名）或 OpenAI
/// 的 reasoning item（含 encrypted_content）。`format` + `key` 记住是谁给的。
#[derive(Debug, Clone)]
pub struct Opaque {
    pub format: Format,
    /// 产生它的上游标识（服务商 id）。故障转移换了服务商就对不上，不会误回放。
    pub key: String,
    pub payload: Value,
}

impl Opaque {
    pub fn new(format: Format, key: &str, payload: Value) -> Opaque {
        Opaque {
            format,
            key: key.to_string(),
            payload,
        }
    }

    /// 协议和上游都对得上才把原始载荷交回去。
    pub fn replay(&self, target: Format, key: &str) -> Option<&Value> {
        (self.format == target && self.key == key).then_some(&self.payload)
    }

    /// 编进目标协议的字符串字段（thinking.signature / reasoning.encrypted_content）
    /// 带给客户端，下一轮客户端回传时由 [`Opaque::decode`] 还原。
    pub fn encode(&self) -> String {
        let envelope = json!({"f": self.format.name(), "k": self.key, "p": self.payload});
        format!("{}{}", ENVELOPE_PREFIX, envelope)
    }

    pub fn decode(text: &str) -> Option<Opaque> {
        let envelope: Value = serde_json::from_str(text.strip_prefix(ENVELOPE_PREFIX)?).ok()?;
        Some(Opaque {
            format: Format::from_name(envelope.get("f").and_then(Value::as_str)?)?,
            key: str_field(&envelope, "k"),
            payload: envelope.get("p")?.clone(),
        })
    }

    /// 判断文本是否包含网关内部信封。
    pub fn is_envelope(text: &str) -> bool {
        text.starts_with(ENVELOPE_PREFIX)
    }
}

/// 原始请求体里是否带网关信封。直通路径靠它跳过绝大多数请求的 JSON 解析。
pub fn has_envelope(body: &[u8]) -> bool {
    body.windows(ENVELOPE_PREFIX.len())
        .any(|window| window == ENVELOPE_PREFIX.as_bytes())
}

/// 网关自己造的思考块签名。上游没给签名（Chat 压根没有签名这个概念，兼容端点也可能
/// 不给）时用它占位：客户端原样回传，下一轮直通路径按信封认出来剥掉。不能用空签名
/// 当标记——兼容端点自己回的思考块也是空签名，按空签名剥会白丢别人家的思考历史。
/// key 留空，任何上游都配不上，所以它永远不会被当成可回放的载荷。
pub fn placeholder_signature() -> String {
    Opaque::new(Format::Anthropic, "", Value::Null).encode()
}

/// 用量统一按 Anthropic 语义存放：input 不含缓存部分。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Usage {
    pub input: i64,
    pub cache_read: i64,
    pub cache_write: i64,
    pub output: i64,
    pub reasoning: i64,
}

impl Usage {
    /// 流式下用量分散在多个事件里，只用非零字段覆盖。
    pub fn merge(&mut self, other: &Usage) {
        for (target, next) in [
            (&mut self.input, other.input),
            (&mut self.cache_read, other.cache_read),
            (&mut self.cache_write, other.cache_write),
            (&mut self.output, other.output),
            (&mut self.reasoning, other.reasoning),
        ] {
            if next > 0 {
                *target = next;
            }
        }
    }

    /// OpenAI 的 prompt_tokens 含缓存部分。
    pub fn prompt_total(&self) -> i64 {
        self.input + self.cache_read + self.cache_write
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    Stop,
    Length,
    ToolUse,
    ContentFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub enum Block {
    Text(String),
    Thinking {
        text: String,
        /// 能原路回放时带着上游给的原始载荷，否则历史里的这一块会被丢掉。
        opaque: Option<Opaque>,
    },
    /// data URL 或 http(s) URL。
    Image(String),
    /// PDF 等文档附件。OpenAI 两家只收 data URL，http URL 只有 Anthropic 支持。
    Document {
        url: String,
        filename: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        id: String,
        text: String,
        is_error: bool,
        /// 工具结果里的图片。Anthropic 能直接放进 tool_result，OpenAI 两家的工具
        /// 消息只收文本，转换时挪到紧随其后的 user 消息里。
        media: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone)]
pub struct Tool {
    pub name: String,
    pub description: Option<String>,
    pub schema: Value,
}

#[derive(Debug, Clone)]
pub enum ToolChoice {
    Auto,
    None,
    Required,
    Tool(String),
}

#[derive(Debug, Clone, Default)]
pub struct ChatRequest {
    pub model: String,
    pub system: String,
    pub messages: Vec<Message>,
    pub tools: Vec<Tool>,
    pub tool_choice: Option<ToolChoice>,
    pub max_tokens: Option<i64>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub stop: Vec<String>,
    pub stream: bool,
    /// low / medium / high
    pub effort: Option<String>,
    pub parallel_tool_calls: Option<bool>,
    /// 结构化输出要求，扁平存放：`{"type": "json_schema", "name":…, "schema":…,
    /// "strict":…}` 或 `{"type": "json_object"}`。Chat 的 response_format 和
    /// Responses 的 text.format 都能对上，Anthropic 没有对应字段。
    pub response_format: Option<Value>,
    /// OpenAI 两家的 prompt_cache_key：同一个值的请求会被路由到同一台机器，前缀缓存
    /// 才命中。Anthropic 靠 cache_control 断点，没有对应字段。
    pub cache_key: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ChatResponse {
    pub id: String,
    pub model: String,
    pub blocks: Vec<Block>,
    pub finish: Option<Finish>,
    pub usage: Usage,
}

/// 流式增量。index 是解码器按首次出现顺序分配的内容块序号，编码器再映射到自己
/// 协议的编号体系。
#[derive(Debug, Clone)]
pub enum StreamEvent {
    Start {
        id: String,
        model: String,
    },
    TextDelta {
        index: usize,
        text: String,
    },
    ThinkingDelta {
        index: usize,
        text: String,
    },
    /// 思考块结束前补上上游的原始载荷，编码器塞进自己协议的字段带给客户端。
    ThinkingOpaque {
        index: usize,
        opaque: Opaque,
    },
    ToolStart {
        index: usize,
        id: String,
        name: String,
    },
    ToolDelta {
        index: usize,
        json: String,
    },
    BlockStop {
        index: usize,
    },
    Finish {
        finish: Option<Finish>,
        usage: Usage,
    },
    Error {
        message: String,
    },
}

/// 上游在流式请求下返回了完整 JSON 时，把它摊成一串流式事件交给编码器。
pub fn response_to_events(response: &ChatResponse) -> Vec<StreamEvent> {
    let mut events = vec![StreamEvent::Start {
        id: response.id.clone(),
        model: response.model.clone(),
    }];
    for (index, block) in response.blocks.iter().enumerate() {
        match block {
            Block::Text(text) => events.push(StreamEvent::TextDelta {
                index,
                text: text.clone(),
            }),
            Block::Thinking { text, opaque } => {
                if !text.is_empty() {
                    events.push(StreamEvent::ThinkingDelta {
                        index,
                        text: text.clone(),
                    });
                }
                if let Some(opaque) = opaque {
                    events.push(StreamEvent::ThinkingOpaque {
                        index,
                        opaque: opaque.clone(),
                    });
                }
            }
            Block::ToolUse { id, name, input } => {
                events.push(StreamEvent::ToolStart {
                    index,
                    id: id.clone(),
                    name: name.clone(),
                });
                events.push(StreamEvent::ToolDelta {
                    index,
                    json: input.to_string(),
                });
            }
            Block::Image(_) | Block::Document { .. } | Block::ToolResult { .. } => continue,
        }
        events.push(StreamEvent::BlockStop { index });
    }
    events.push(StreamEvent::Finish {
        finish: response.finish,
        usage: response.usage.clone(),
    });
    events
}

pub fn str_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub fn array(value: Option<&Value>) -> &[Value] {
    const EMPTY: &[Value] = &[];
    value
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(EMPTY)
}

pub fn string_list(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(text)) => vec![text.clone()],
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

/// 工具 schema 里 OpenAI 两家不接受的写法：`format: "uri"` 会被拒（Claude Code 的
/// WebFetch 就带这个），根 schema 缺 type 也会被拒。Anthropic 收完整 JSON Schema，
/// 只在目标是 OpenAI 时调用。
pub fn clean_schema(schema: &Value) -> Value {
    object_schema(&strip_unsupported(schema))
}

/// 只保证根节点是带 `type` 的对象 schema，不裁剪任何字段。无参工具常把
/// `parameters` 写成 `{}` 或 `null`，直接透传会被 Anthropic 以
/// `input_schema.type: Field required` 拒掉整个请求（不是只丢那个工具）。
pub fn object_schema(schema: &Value) -> Value {
    let mut normalized = schema.clone();
    let Some(object) = normalized.as_object_mut() else {
        return json!({"type": "object", "properties": {}});
    };
    if !object.contains_key("type") {
        object.insert("type".to_string(), json!("object"));
        object.entry("properties").or_insert_with(|| json!({}));
    }
    normalized
}

fn strip_unsupported(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(key, value)| {
                    !(key.as_str() == "format" && value.as_str() == Some("uri"))
                })
                .map(|(key, value)| (key.clone(), strip_unsupported(value)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(strip_unsupported).collect()),
        other => other.clone(),
    }
}

/// 拆 `data:image/png;base64,xxx`。
pub fn split_data_url(url: &str) -> Option<(&str, &str)> {
    let rest = url.strip_prefix("data:")?;
    let (mime, data) = rest.split_once(',')?;
    Some((mime.trim_end_matches(";base64"), data))
}

/// 工具入参在 OpenAI 两家是 JSON 字符串，在 Anthropic 是对象。能解析就转成对象，
/// 解析不了保留原字符串，让目标协议自己决定怎么降级。
pub fn tool_input(value: Option<&Value>) -> Value {
    match value {
        Some(Value::String(text)) => {
            serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.clone()))
        }
        Some(other) => other.clone(),
        None => Value::Object(serde_json::Map::new()),
    }
}

/// Anthropic 的 `tool_use.input` 必须是对象。
pub fn tool_input_object(value: &Value) -> Value {
    if value.is_object() {
        value.clone()
    } else {
        Value::Object(serde_json::Map::new())
    }
}

/// OpenAI 两家的 `arguments` 必须是字符串。
pub fn tool_input_string(value: &Value) -> String {
    match value {
        Value::String(text) if text.is_empty() => "{}".to_string(),
        Value::String(text) => text.clone(),
        Value::Null => "{}".to_string(),
        other => other.to_string(),
    }
}

/// 响应 id 客户端不会回传，直接补上目标协议的前缀即可。工具调用 id 会回传，
/// 必须原样透传，不能走这里。
pub fn ensure_id(prefix: &str, id: &str) -> String {
    if id.starts_with(prefix) {
        id.to_string()
    } else if id.is_empty() {
        format!("{}{}", prefix, crate::time::now_timestamp())
    } else {
        format!("{}{}", prefix, id)
    }
}
