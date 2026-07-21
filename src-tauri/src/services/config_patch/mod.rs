mod env;
mod json;
mod jsonc;
mod preset;
mod toml;
mod writer;

use crate::db::models::{ConfigFormat, ConfigOperation, ConfigOperationKind};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PatchContext {
    pub target_endpoint: String,
    pub target_token: String,
    pub agent_id: String,
}

#[derive(Debug, Clone)]
pub(super) struct ResolvedOperation {
    pub id: String,
    pub kind: ConfigOperationKind,
    pub path: Vec<String>,
    pub value: Option<Value>,
}

fn resolve_runtime_string(
    operation: &ConfigOperation,
    input: &str,
    context: &PatchContext,
) -> Result<String, String> {
    let mut output = String::with_capacity(input.len());
    let mut remaining = input;
    while let Some(start) = remaining.find('{') {
        output.push_str(&remaining[..start]);
        let candidate = &remaining[start..];
        let (placeholder, replacement) = if candidate.starts_with("{target.endpoint}") {
            if context.target_endpoint.is_empty() {
                return Err(format!("operation `{}` 缺少 target.endpoint", operation.id));
            }
            ("{target.endpoint}", context.target_endpoint.as_str())
        } else if candidate.starts_with("{target.token}") {
            if context.target_token.is_empty() {
                return Err(format!("operation `{}` 缺少 target.token", operation.id));
            }
            ("{target.token}", context.target_token.as_str())
        } else if candidate.starts_with("{agent.id}") {
            ("{agent.id}", context.agent_id.as_str())
        } else if candidate.starts_with("{target.")
            || candidate.starts_with("{agent.")
            || candidate.starts_with("{profile")
        {
            return Err(format!("operation `{}` 使用了未知模板变量", operation.id));
        } else {
            output.push('{');
            remaining = &candidate[1..];
            continue;
        };
        output.push_str(replacement);
        remaining = &candidate[placeholder.len()..];
    }
    output.push_str(remaining);
    Ok(output)
}

fn resolve_runtime_value(
    operation: &ConfigOperation,
    value: &Value,
    context: &PatchContext,
) -> Result<Value, String> {
    match value {
        Value::String(value) => {
            resolve_runtime_string(operation, value, context).map(Value::String)
        }
        Value::Array(values) => values
            .iter()
            .map(|value| resolve_runtime_value(operation, value, context))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(values) => values
            .iter()
            .map(|(key, value)| {
                Ok((
                    resolve_runtime_string(operation, key, context)?,
                    resolve_runtime_value(operation, value, context)?,
                ))
            })
            .collect::<Result<serde_json::Map<_, _>, String>>()
            .map(Value::Object),
        _ => Ok(value.clone()),
    }
}

fn resolve_value(operation: &ConfigOperation, context: &PatchContext) -> Result<Value, String> {
    let value = operation
        .value
        .as_ref()
        .ok_or_else(|| format!("operation `{}` 缺少 value", operation.id))?;
    resolve_runtime_value(operation, value, context)
}

fn resolve_operations(
    operations: &[ConfigOperation],
    context: &PatchContext,
) -> Result<Vec<ResolvedOperation>, String> {
    operations
        .iter()
        .map(|operation| {
            if operation.id.trim().is_empty() {
                return Err("operation id 不能为空".to_string());
            }
            if operation.path.is_empty() || operation.path.iter().any(|part| part.is_empty()) {
                return Err(format!("operation `{}` 的 path 不能为空", operation.id));
            }
            let writes_value = operation.op == ConfigOperationKind::Set;
            let value = if writes_value {
                Some(resolve_value(operation, context)?)
            } else {
                if operation.value.is_some() {
                    return Err(format!("operation `{}` 不接受 value", operation.id));
                }
                None
            };
            Ok(ResolvedOperation {
                id: operation.id.clone(),
                kind: operation.op,
                path: operation.path.clone(),
                value,
            })
        })
        .collect()
}

pub fn patch_content(
    format: ConfigFormat,
    content: &str,
    operations: &[ConfigOperation],
    context: &PatchContext,
) -> Result<String, String> {
    let operations = resolve_operations(operations, context)?;
    match format {
        ConfigFormat::Json => json::patch(content, &operations),
        ConfigFormat::Jsonc => jsonc::patch(content, &operations),
        ConfigFormat::Toml => toml::patch(content, &operations),
        ConfigFormat::Env => env::patch(content, &operations),
    }
}

pub fn operations_applied(
    format: ConfigFormat,
    content: &str,
    operations: &[ConfigOperation],
    context: &PatchContext,
) -> Result<bool, String> {
    let patched = patch_content(format, content, operations, context)?;
    match format {
        ConfigFormat::Json => {
            let current: Value =
                serde_json::from_str(content).map_err(|error| error.to_string())?;
            let next: Value = serde_json::from_str(&patched).map_err(|error| error.to_string())?;
            Ok(current == next)
        }
        ConfigFormat::Jsonc => jsonc::equivalent(content, &patched),
        ConfigFormat::Toml => {
            let current: ::toml::Value =
                ::toml::from_str(content).map_err(|error| error.to_string())?;
            let next: ::toml::Value =
                ::toml::from_str(&patched).map_err(|error| error.to_string())?;
            Ok(current == next)
        }
        ConfigFormat::Env => Ok(content == patched),
    }
}

pub async fn patch_file(
    path: &Path,
    format: ConfigFormat,
    operations: &[ConfigOperation],
    context: &PatchContext,
) -> Result<PathBuf, String> {
    let path = path.to_path_buf();
    let operations = operations.to_vec();
    let context = context.clone();
    let task_path = path.clone();
    tokio::task::spawn_blocking(move || {
        let content = match std::fs::read_to_string(&task_path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => {
                return Err(format!("读取 {} 失败: {}", task_path.display(), error));
            }
        };
        let next = patch_content(format, &content, &operations, &context)?;
        writer::write_atomic(&task_path, next.as_bytes())?;
        Ok(())
    })
    .await
    .map_err(|error| format!("配置写入任务失败: {}", error))??;
    Ok(path)
}

pub async fn write_atomic_json(path: &Path, value: &Value) -> Result<(), String> {
    let path = path.to_path_buf();
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    tokio::task::spawn_blocking(move || writer::write_atomic(&path, &bytes))
        .await
        .map_err(|error| format!("配置写入任务失败: {}", error))?
}

pub async fn write_atomic_text(path: &Path, content: &str) -> Result<(), String> {
    let path = path.to_path_buf();
    let bytes = content.as_bytes().to_vec();
    tokio::task::spawn_blocking(move || writer::write_atomic(&path, &bytes))
        .await
        .map_err(|error| format!("配置写入任务失败: {}", error))?
}

fn inverse_operation(operation: &ConfigOperation) -> Option<ConfigOperation> {
    let op = match operation.op {
        ConfigOperationKind::Set => ConfigOperationKind::Remove,
        ConfigOperationKind::Remove => return None,
    };
    Some(ConfigOperation {
        id: format!("disable-{}", operation.id),
        op,
        file: operation.file.clone(),
        format: operation.format,
        path: operation.path.clone(),
        value: None,
    })
}

pub fn safely_remove_operations(
    format: ConfigFormat,
    content: &str,
    operations: &[ConfigOperation],
    context: &PatchContext,
) -> Result<String, String> {
    if content.trim().is_empty() {
        return Ok(content.to_string());
    }

    let mut current = content.to_string();
    for operation in operations {
        let Some(inverse) = inverse_operation(operation) else {
            continue;
        };
        if !operations_applied(format, &current, std::slice::from_ref(operation), context)
            .unwrap_or(false)
        {
            continue;
        }
        current = patch_content(format, &current, &[inverse], context)?;
    }
    Ok(current)
}

pub fn apply_preset(
    format: ConfigFormat,
    content: &str,
    preset_content: &str,
) -> Result<String, String> {
    preset::apply(format, content, preset_content)
}

pub fn safely_remove_preset(
    format: ConfigFormat,
    content: &str,
    preset_content: &str,
) -> Result<String, String> {
    preset::remove(format, content, preset_content)
}

/// Remove fields owned by a mode before merging a global preset.
pub fn sanitize_preset(
    format: ConfigFormat,
    content: &str,
    operations: &[ConfigOperation],
    context: &PatchContext,
) -> Result<String, String> {
    if content.trim().is_empty() {
        return Ok(content.to_string());
    }
    let removals: Vec<ConfigOperation> = operations
        .iter()
        .filter_map(|operation| {
            let kind = match operation.op {
                ConfigOperationKind::Set => ConfigOperationKind::Remove,
                ConfigOperationKind::Remove => return None,
            };
            Some(ConfigOperation {
                id: format!("protect-{}", operation.id),
                op: kind,
                file: operation.file.clone(),
                format: operation.format,
                path: operation.path.clone(),
                value: None,
            })
        })
        .collect();
    if removals.is_empty() {
        return Ok(content.to_string());
    }
    patch_content(format, content, &removals, context)
}
