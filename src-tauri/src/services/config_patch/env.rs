use super::ResolvedOperation;
use crate::db::models::ConfigOperationKind;
use serde_json::Value;

fn line_key(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let assignment = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    let (key, _) = assignment.split_once('=')?;
    let key = key.trim();
    (!key.is_empty()
        && key.bytes().enumerate().all(|(index, byte)| {
            byte == b'_' || byte.is_ascii_alphabetic() || (index > 0 && byte.is_ascii_digit())
        }))
    .then_some(key)
}

fn env_value(value: &Value) -> Result<String, String> {
    let raw = match value {
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Null => String::new(),
        Value::Array(_) | Value::Object(_) => {
            serde_json::to_string(value).map_err(|error| error.to_string())?
        }
    };
    if raw.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/' | b':' | b'@')
    }) {
        Ok(raw)
    } else {
        Ok(format!(
            "\"{}\"",
            raw.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
        ))
    }
}

pub(super) fn patch(content: &str, operations: &[ResolvedOperation]) -> Result<String, String> {
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let had_trailing_newline = content.ends_with('\n');
    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();

    for operation in operations {
        if operation.path.len() != 1 {
            return Err(format!(
                "operation `{}` 的 ENV path 必须只有一个 key",
                operation.id
            ));
        }
        let key = &operation.path[0];
        match operation.kind {
            ConfigOperationKind::Set => {
                let value = env_value(operation.value.as_ref().expect("resolved env value"))?;
                let mut replaced = false;
                lines.retain_mut(|line| {
                    if line_key(line) == Some(key.as_str()) {
                        if replaced {
                            return false;
                        }
                        let export = line.trim_start().starts_with("export ");
                        *line = format!("{}{}={}", if export { "export " } else { "" }, key, value);
                        replaced = true;
                    }
                    true
                });
                if !replaced {
                    lines.push(format!("{}={}", key, value));
                }
            }
            ConfigOperationKind::Remove => {
                lines.retain(|line| line_key(line) != Some(key.as_str()));
            }
        }
    }

    let mut output = lines.join(newline);
    if had_trailing_newline || (!output.is_empty() && content.is_empty()) {
        output.push_str(newline);
    }
    Ok(output)
}
