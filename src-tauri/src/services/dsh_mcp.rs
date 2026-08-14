use serde_json::{Map, Value};
use std::{ops::Range, path::Path};

const BLOCK_START: &str = "# >>> CCG Gateway DeepSeek Harness MCP >>>";
const BLOCK_END: &str = "# <<< CCG Gateway DeepSeek Harness MCP <<<";
const MCP_PLUGIN: &str = "@deepseek-ai/dsh-mcp-client";

pub fn adapt(name: &str, config: Value) -> Result<Value, String> {
    validate_name(name)?;
    let server = config
        .as_object()
        .ok_or_else(|| "DeepSeek Harness MCP 配置必须是 JSON object".to_string())?;
    let transport = transport(server)?;
    let mut adapted = Map::new();
    adapted.insert("serverName".to_string(), serde_json::json!(name));
    adapted.insert("transport".to_string(), serde_json::json!(transport));

    if transport == "stdio" {
        let command = server
            .get("command")
            .and_then(Value::as_str)
            .filter(|command| !command.trim().is_empty())
            .ok_or_else(|| "stdio MCP 必须提供非空 command".to_string())?;
        adapted.insert("command".to_string(), serde_json::json!(command));
        if let Some(args) = server.get("args") {
            if !args
                .as_array()
                .is_some_and(|args| args.iter().all(Value::is_string))
            {
                return Err("MCP args 必须是字符串数组".to_string());
            }
            adapted.insert("args".to_string(), args.clone());
        }
        copy_string_map(server, &mut adapted, "env")?;
        if let Some(cwd) = server.get("cwd") {
            if !cwd.is_string() {
                return Err("MCP cwd 必须是字符串".to_string());
            }
            adapted.insert("cwd".to_string(), cwd.clone());
        }
    } else {
        let url = server
            .get("url")
            .and_then(Value::as_str)
            .filter(|url| !url.trim().is_empty())
            .ok_or_else(|| "streamable-http MCP 必须提供非空 url".to_string())?;
        adapted.insert("url".to_string(), serde_json::json!(url));
        copy_string_map(server, &mut adapted, "headers")?;
    }
    copy_common_fields(server, &mut adapted)?;
    Ok(Value::Object(adapted))
}

pub fn contains(content: &str, name: &str) -> bool {
    entries(content).ok().is_some_and(|entries| {
        entries
            .iter()
            .any(|entry| entry_name(entry).ok() == Some(name))
    })
}

pub async fn sync(path: &Path, name: &str, adapted_config: Option<&str>) -> Result<(), String> {
    let content = match tokio::fs::read_to_string(path).await {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(format!("读取 {} 失败: {}", path.display(), error)),
    };
    let config = adapted_config
        .map(|config| {
            serde_json::from_str(config)
                .map_err(|error| format!("DeepSeek Harness MCP JSON 格式错误: {}", error))
        })
        .transpose()?;
    let next = patch_content(&content, name, config)?;
    if next != content {
        super::config_patch::write_atomic_text(path, &next).await?;
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), String> {
    if (1..=32).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Ok(());
    }
    Err("DeepSeek Harness MCP 名称必须匹配 [A-Za-z0-9_-]{1,32}".to_string())
}

fn normalize_transport(value: &Value) -> Result<&'static str, String> {
    match value
        .as_str()
        .ok_or_else(|| "MCP transport/type 必须是字符串".to_string())?
    {
        "stdio" => Ok("stdio"),
        "http" | "streamable-http" => Ok("streamable-http"),
        "sse" => Err("DeepSeek Harness 不支持旧式 SSE MCP，请使用 streamable-http".to_string()),
        value => Err(format!(
            "DeepSeek Harness MCP 不支持 transport/type: {}",
            value
        )),
    }
}

fn transport(server: &Map<String, Value>) -> Result<&'static str, String> {
    let transport = server
        .get("transport")
        .map(normalize_transport)
        .transpose()?;
    let server_type = server.get("type").map(normalize_transport).transpose()?;
    match (transport, server_type) {
        (Some(left), Some(right)) if left != right => Err("MCP transport 与 type 冲突".to_string()),
        (Some(value), _) | (_, Some(value)) => Ok(value),
        (None, None) => match (server.contains_key("command"), server.contains_key("url")) {
            (true, false) => Ok("stdio"),
            (false, true) => Ok("streamable-http"),
            _ => Err("MCP 配置必须通过 command 或 url 明确传输类型".to_string()),
        },
    }
}

fn copy_string_map(
    source: &Map<String, Value>,
    target: &mut Map<String, Value>,
    key: &str,
) -> Result<(), String> {
    let Some(value) = source.get(key) else {
        return Ok(());
    };
    if !value
        .as_object()
        .is_some_and(|map| map.values().all(Value::is_string))
    {
        return Err(format!("MCP {} 必须是字符串值 JSON object", key));
    }
    target.insert(key.to_string(), value.clone());
    Ok(())
}

fn copy_common_fields(
    source: &Map<String, Value>,
    target: &mut Map<String, Value>,
) -> Result<(), String> {
    if let Some(value) = source.get("toolCallTimeoutMs") {
        if !value.is_number() {
            return Err("MCP toolCallTimeoutMs 必须是数字".to_string());
        }
        target.insert("toolCallTimeoutMs".to_string(), value.clone());
    }
    if let Some(value) = source.get("failOnStartupError") {
        if !value.is_boolean() {
            return Err("MCP failOnStartupError 必须是布尔值".to_string());
        }
        target.insert("failOnStartupError".to_string(), value.clone());
    }
    if let Some(value) = source.get("reconnect") {
        if !value.is_object() {
            return Err("MCP reconnect 必须是 JSON object".to_string());
        }
        target.insert("reconnect".to_string(), value.clone());
    }
    Ok(())
}

fn managed_block(content: &str) -> Result<Option<(Range<usize>, &str)>, String> {
    let starts = content
        .match_indices(BLOCK_START)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let ends = content
        .match_indices(BLOCK_END)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    match (starts.as_slice(), ends.as_slice()) {
        ([], []) => Ok(None),
        ([start], [end]) if start < end => {
            let marker_on_own_line = |index: usize, marker: &str| {
                (index == 0 || content.as_bytes().get(index.wrapping_sub(1)) == Some(&b'\n'))
                    && matches!(
                        content.as_bytes().get(index + marker.len()),
                        None | Some(b'\r' | b'\n')
                    )
            };
            if !marker_on_own_line(*start, BLOCK_START) || !marker_on_own_line(*end, BLOCK_END) {
                return Err("DeepSeek Harness MCP 管理标记必须独占一行".to_string());
            }
            let body_start = start + BLOCK_START.len();
            Ok(Some((
                *start..end + BLOCK_END.len(),
                &content[body_start..*end],
            )))
        }
        _ => Err("DeepSeek Harness MCP 管理标记缺失、重复或顺序错误".to_string()),
    }
}

fn entry_name(entry: &Value) -> Result<&str, String> {
    if entry.get("name").and_then(Value::as_str) != Some(MCP_PLUGIN) {
        return Err("DeepSeek Harness MCP 管理块包含未知插件".to_string());
    }
    entry
        .get("config")
        .and_then(|config| config.get("serverName"))
        .and_then(Value::as_str)
        .ok_or_else(|| "DeepSeek Harness MCP 管理项缺少 serverName".to_string())
}

fn entries(content: &str) -> Result<Vec<Value>, String> {
    let Some((_, body)) = managed_block(content)? else {
        return Ok(Vec::new());
    };
    let patches = serde_yaml::from_str::<Value>(body)
        .map_err(|error| format!("DeepSeek Harness MCP 管理块解析失败: {}", error))?;
    let patches = patches
        .as_array()
        .ok_or_else(|| "DeepSeek Harness MCP 管理块必须是 YAML 数组".to_string())?;
    let mut entries = Vec::new();
    for patch in patches {
        let insert = patch
            .get("insert")
            .and_then(Value::as_array)
            .ok_or_else(|| "DeepSeek Harness MCP 管理块只能包含 insert".to_string())?;
        for entry in insert {
            let name = entry_name(entry)?;
            validate_name(name)?;
            if entries
                .iter()
                .any(|current| entry_name(current).ok() == Some(name))
            {
                return Err(format!("DeepSeek Harness MCP 管理块包含重复项: {}", name));
            }
            entries.push(entry.clone());
        }
    }
    Ok(entries)
}

#[derive(PartialEq, Eq)]
enum PatchState {
    Blank,
    Empty,
    Populated,
}

fn patch_state(content: &str) -> Result<PatchState, String> {
    if content.lines().all(|line| {
        let line = line.trim();
        line.is_empty() || line.starts_with('#')
    }) {
        return Ok(PatchState::Blank);
    }
    let root = serde_yaml::from_str::<serde_yaml::Value>(content)
        .map_err(|error| format!("DeepSeek Harness cordis.patch.yml 解析失败: {}", error))?;
    let sequence = root
        .as_sequence()
        .ok_or_else(|| "DeepSeek Harness cordis.patch.yml 顶层必须是 YAML 数组".to_string())?;
    Ok(if sequence.is_empty() {
        PatchState::Empty
    } else {
        PatchState::Populated
    })
}

fn replace_empty_sequence(content: &str, replacement: &str) -> Result<String, String> {
    let mut matches = content.match_indices("[]").filter_map(|(index, _)| {
        let starts_line = index == 0 || content.as_bytes().get(index - 1) == Some(&b'\n');
        let ends_line = matches!(
            content.as_bytes().get(index + 2),
            None | Some(b'\r' | b'\n')
        );
        (starts_line && ends_line).then_some(index)
    });
    let Some(index) = matches.next() else {
        return Err("DeepSeek Harness 空 MCP patch 必须使用独占一行的 []".to_string());
    };
    if matches.next().is_some() {
        return Err("DeepSeek Harness 空 MCP patch 包含多个 []".to_string());
    }
    Ok(format!(
        "{}{}{}",
        &content[..index],
        replacement,
        &content[index + 2..]
    ))
}

fn render_block(entries: Vec<Value>, newline: &str) -> Result<String, String> {
    let patch = serde_json::json!([{ "insert": entries }]);
    let yaml = serde_yaml::to_string(&patch)
        .map_err(|error| format!("DeepSeek Harness MCP 配置序列化失败: {}", error))?;
    Ok(format!(
        "{}{}{}{}{}",
        BLOCK_START,
        newline,
        yaml.trim_end().replace('\n', newline),
        newline,
        BLOCK_END
    ))
}

fn patch_content(content: &str, name: &str, config: Option<Value>) -> Result<String, String> {
    let block = managed_block(content)?;
    let mut managed_entries = entries(content)?;
    managed_entries.retain(|entry| entry_name(entry).ok() != Some(name));
    if let Some(config) = config {
        managed_entries.push(serde_json::json!({
            "id": format!("ccg-mcp-{}", name),
            "name": MCP_PLUGIN,
            "config": config,
        }));
    }
    managed_entries.sort_by(|left, right| {
        entry_name(left)
            .unwrap_or_default()
            .cmp(entry_name(right).unwrap_or_default())
    });

    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut next = match (block, managed_entries.is_empty()) {
        (Some((range, _)), true) => format!("{}{}", &content[..range.start], &content[range.end..]),
        (Some((range, _)), false) => format!(
            "{}{}{}",
            &content[..range.start],
            render_block(managed_entries, newline)?,
            &content[range.end..]
        ),
        (None, true) => return Ok(content.to_string()),
        (None, false) => {
            let rendered = render_block(managed_entries, newline)?;
            if patch_state(content)? == PatchState::Empty {
                replace_empty_sequence(content, &rendered)?
            } else {
                let separator = if content.is_empty() || content.ends_with('\n') {
                    ""
                } else {
                    newline
                };
                format!("{}{}{}{}", content, separator, rendered, newline)
            }
        }
    };
    if patch_state(&next)? == PatchState::Blank {
        let separator = if next.is_empty() || next.ends_with('\n') {
            ""
        } else {
            newline
        };
        next = format!("{}{}[]{}", next, separator, newline);
    }
    patch_state(&next)?;
    Ok(next)
}
