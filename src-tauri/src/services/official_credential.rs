use crate::db::models::{
    ConfigFormat, OfficialCredentialFile, OfficialCredentialPayload, OfficialLoginOperation,
    OfficialLoginOperationKind,
};
use crate::services::cli_config::{get_cli_config_dir_path, resolve_cli_config_file_from_dir};
use crate::services::{agent, config_patch};
use serde_json::{Map, Value};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug)]
enum PreparedFile {
    Json(Value),
    Text(String),
}

fn operation_format(operation: &OfficialLoginOperation) -> ConfigFormat {
    operation.format.unwrap_or(ConfigFormat::Json)
}

pub fn parse_payload(json: &str) -> Result<OfficialCredentialPayload, String> {
    let payload: OfficialCredentialPayload =
        serde_json::from_str(json).map_err(|error| format!("凭证 JSON 格式无效: {}", error))?;
    if payload.schema_version != 1 {
        return Err(format!(
            "不支持凭证 schema_version {}",
            payload.schema_version
        ));
    }
    if payload.files.is_empty() {
        return Err("凭证 files 不能为空".to_string());
    }
    Ok(payload)
}

pub fn validate_payload(agent_id: &str, json: &str) -> Result<(), String> {
    agent::validate_agent_id(agent_id)?;
    let payload = parse_payload(json)?;
    let definition = agent::get_definition(agent_id).expect("validated Agent");
    let feature = &definition.features.official_login;
    if !feature.enabled {
        return Err(format!("Agent {} 的官方凭证功能未启用", agent_id));
    }
    for operation in &feature.operations {
        let source = match operation.op {
            OfficialLoginOperationKind::ReplaceFile => operation.content_from.as_ref(),
            OfficialLoginOperationKind::SetField => operation.value_from.as_ref(),
        };
        if let Some(source) = source {
            let file = payload
                .files
                .get(&source.file_id)
                .ok_or_else(|| format!("凭证缺少逻辑文件 `{}`", source.file_id))?;
            let expected_format = operation_format(operation);
            if file.format != expected_format {
                return Err(format!(
                    "逻辑文件 `{}` 应为 {:?} 格式",
                    source.file_id, expected_format
                ));
            }
            if file.format == ConfigFormat::Json && value_at(&file.content, &source.path).is_none()
            {
                return Err(format!(
                    "逻辑文件 `{}` 缺少来源路径 `{}`",
                    source.file_id,
                    source.path.join(".")
                ));
            }
            if file.format != ConfigFormat::Json
                && (!source.path.is_empty() || !file.content.is_string())
            {
                return Err(format!(
                    "非 JSON 逻辑文件 `{}` 必须使用原始文本且不能配置来源路径",
                    source.file_id
                ));
            }
        }
    }
    Ok(())
}

fn value_at<'a>(value: &'a Value, path: &[String]) -> Option<&'a Value> {
    path.iter().try_fold(value, |current, key| current.get(key))
}

fn set_value_at(
    value: &mut Value,
    path: &[String],
    next: Value,
    context: &str,
) -> Result<(), String> {
    if path.is_empty() {
        *value = next;
        return Ok(());
    }
    let mut current = value;
    for key in &path[..path.len() - 1] {
        if current.get(key).is_none() {
            current
                .as_object_mut()
                .ok_or_else(|| format!("{} 路径结构冲突", context))?
                .insert(key.clone(), Value::Object(Map::new()));
        }
        current = current
            .get_mut(key)
            .ok_or_else(|| format!("{} 路径不存在", context))?;
        if !current.is_object() {
            return Err(format!("{} 路径结构冲突", context));
        }
    }
    current
        .as_object_mut()
        .ok_or_else(|| format!("{} 目标不是对象", context))?
        .insert(path[path.len() - 1].clone(), next);
    Ok(())
}

fn source_value(
    payload: &OfficialCredentialPayload,
    operation: &OfficialLoginOperation,
) -> Result<Value, String> {
    let source = match operation.op {
        OfficialLoginOperationKind::ReplaceFile => operation.content_from.as_ref(),
        OfficialLoginOperationKind::SetField => operation.value_from.as_ref(),
    }
    .ok_or_else(|| format!("operation `{}` 缺少凭证来源", operation.id))?;
    let file = payload
        .files
        .get(&source.file_id)
        .ok_or_else(|| format!("逻辑文件 `{}` 不存在", source.file_id))?;
    if file.format != operation_format(operation) {
        return Err(format!("逻辑文件 `{}` 格式与模板不一致", source.file_id));
    }
    if file.format != ConfigFormat::Json && !source.path.is_empty() {
        return Err(format!(
            "非 JSON 逻辑文件 `{}` 不支持来源路径",
            source.file_id
        ));
    }
    value_at(&file.content, &source.path)
        .cloned()
        .ok_or_else(|| format!("逻辑文件 `{}` 中的来源路径不存在", source.file_id))
}

fn prepared_replacement(
    payload: &OfficialCredentialPayload,
    operation: &OfficialLoginOperation,
) -> Result<PreparedFile, String> {
    let value = source_value(payload, operation)?;
    match operation_format(operation) {
        ConfigFormat::Json => Ok(PreparedFile::Json(value)),
        ConfigFormat::Jsonc | ConfigFormat::Toml | ConfigFormat::Env => value
            .as_str()
            .map(|content| PreparedFile::Text(content.to_string()))
            .ok_or_else(|| format!("operation `{}` 的凭证内容必须是文本", operation.id)),
    }
}

fn operation_value(
    payload: &OfficialCredentialPayload,
    operation: &OfficialLoginOperation,
) -> Result<Value, String> {
    match (&operation.value, &operation.value_from) {
        (Some(value), None) => Ok(value.clone()),
        (None, Some(_)) => source_value(payload, operation),
        _ => Err(format!(
            "operation `{}` 的 value 和 value_from 必须二选一",
            operation.id
        )),
    }
}

fn remove_matching_value(
    current: &mut Value,
    path: &[String],
    expected: &Value,
    context: &str,
) -> Result<bool, String> {
    let Some((key, remaining)) = path.split_first() else {
        return Err(format!("{} path 不能为空", context));
    };
    let Some(object) = current.as_object_mut() else {
        return Ok(false);
    };
    if remaining.is_empty() {
        if object.get(key) == Some(expected) {
            object.remove(key);
            return Ok(true);
        }
        return Ok(false);
    }
    let Some(child) = object.get_mut(key) else {
        return Ok(false);
    };
    let removed = remove_matching_value(child, remaining, expected, context)?;
    if removed && child.as_object().is_some_and(Map::is_empty) {
        object.remove(key);
    }
    Ok(removed)
}

async fn read_target_json(path: &Path) -> Result<Value, String> {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => serde_json::from_str(&content)
            .map_err(|error| format!("目标文件 {} 不是合法 JSON: {}", path.display(), error)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Value::Object(Map::new())),
        Err(error) => Err(format!("读取目标文件 {} 失败: {}", path.display(), error)),
    }
}

async fn resolved_operations(
    db: &SqlitePool,
    agent_id: &str,
) -> Result<Vec<OfficialLoginOperation>, String> {
    let resolved = agent::get_agent(db, agent_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("未知 Agent: {}", agent_id))?;
    let feature = resolved.features.official_login;
    if feature.enabled {
        Ok(feature.operations)
    } else {
        Err("该 Agent 的官方凭证功能未启用".to_string())
    }
}

fn builtin_operations(agent_id: &str) -> Result<Vec<OfficialLoginOperation>, String> {
    let definition =
        agent::get_definition(agent_id).ok_or_else(|| format!("未知 Agent: {}", agent_id))?;
    let feature = &definition.features.official_login;
    if feature.operations.is_empty() {
        return Err(format!("Agent {} 没有官方凭证写入规则", agent_id));
    }
    Ok(feature.operations.clone())
}

pub async fn apply_payload(
    db: &SqlitePool,
    agent_id: &str,
    json: &str,
) -> Result<Vec<PathBuf>, String> {
    let payload = parse_payload(json)?;
    let operations = resolved_operations(db, agent_id).await?;
    let config_dir = get_cli_config_dir_path(db, agent_id).await;
    let mut prepared: HashMap<PathBuf, PreparedFile> = HashMap::new();

    for operation in &operations {
        let path = resolve_cli_config_file_from_dir(&config_dir, &operation.file);
        match operation.op {
            OfficialLoginOperationKind::ReplaceFile => {
                prepared.insert(path, prepared_replacement(&payload, operation)?);
            }
            OfficialLoginOperationKind::SetField => {
                if operation.format != Some(ConfigFormat::Json) {
                    return Err(format!(
                        "operation `{}` 的 set_field 只支持 JSON",
                        operation.id
                    ));
                }
                let next = operation_value(&payload, operation)?;
                if !prepared.contains_key(&path) {
                    prepared.insert(
                        path.clone(),
                        PreparedFile::Json(read_target_json(&path).await?),
                    );
                }
                let PreparedFile::Json(target) = prepared.get_mut(&path).expect("prepared target")
                else {
                    return Err(format!(
                        "operation `{}` 与同一目标文件的原文替换规则冲突",
                        operation.id
                    ));
                };
                set_value_at(target, &operation.path, next, &operation.id)?;
            }
        }
    }

    let mut written = Vec::with_capacity(prepared.len());
    for (path, content) in prepared {
        match content {
            PreparedFile::Json(value) => config_patch::write_atomic_json(&path, &value).await?,
            PreparedFile::Text(value) => config_patch::write_atomic_text(&path, &value).await?,
        }
        written.push(path);
    }
    Ok(written)
}

pub async fn remove_payload(
    db: &SqlitePool,
    agent_id: &str,
    json: &str,
) -> Result<Vec<PathBuf>, String> {
    let payload = parse_payload(json)?;
    let operations = builtin_operations(agent_id)?;
    let config_dir = get_cli_config_dir_path(db, agent_id).await;
    let mut prepared: HashMap<PathBuf, Value> = HashMap::new();
    let mut remove_files = HashSet::new();

    for operation in &operations {
        let path = resolve_cli_config_file_from_dir(&config_dir, &operation.file);
        match operation.op {
            OfficialLoginOperationKind::ReplaceFile => {
                let actual = match tokio::fs::read_to_string(&path).await {
                    Ok(content) => content,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(error) => return Err(error.to_string()),
                };
                let matches = match prepared_replacement(&payload, operation)? {
                    PreparedFile::Json(expected) => serde_json::from_str::<Value>(&actual)
                        .map(|value| value == expected)
                        .unwrap_or(false),
                    PreparedFile::Text(expected) => actual == expected,
                };
                if matches {
                    remove_files.insert(path);
                }
            }
            OfficialLoginOperationKind::SetField => {
                if operation.format != Some(ConfigFormat::Json) {
                    return Err(format!(
                        "operation `{}` 的 set_field 只支持 JSON",
                        operation.id
                    ));
                }
                let expected = operation_value(&payload, operation)?;
                if let Some(target) = prepared.get_mut(&path) {
                    remove_matching_value(target, &operation.path, &expected, &operation.id)?;
                } else {
                    let mut target = read_target_json(&path).await?;
                    if remove_matching_value(
                        &mut target,
                        &operation.path,
                        &expected,
                        &operation.id,
                    )? {
                        prepared.insert(path, target);
                    }
                }
            }
        }
    }

    let mut touched = Vec::new();
    for (path, value) in prepared {
        if !remove_files.contains(&path) {
            config_patch::write_atomic_json(&path, &value).await?;
            touched.push(path);
        }
    }
    for path in remove_files {
        match tokio::fs::remove_file(&path).await {
            Ok(()) => touched.push(path),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("删除 {} 失败: {}", path.display(), error)),
        }
    }
    Ok(touched)
}

pub async fn payload_matches(db: &SqlitePool, agent_id: &str, json: &str) -> Result<bool, String> {
    let payload = parse_payload(json)?;
    // Detection must keep working after a feature is disabled so
    // the dashboard can still remove credentials previously written by CCG.
    let operations = builtin_operations(agent_id)?;
    let config_dir = get_cli_config_dir_path(db, agent_id).await;
    if operations.is_empty() {
        return Ok(false);
    }
    for operation in &operations {
        let path = resolve_cli_config_file_from_dir(&config_dir, &operation.file);
        match operation.op {
            OfficialLoginOperationKind::ReplaceFile => {
                let actual = match tokio::fs::read_to_string(&path).await {
                    Ok(content) => content,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
                    Err(error) => return Err(error.to_string()),
                };
                let matches = match prepared_replacement(&payload, operation)? {
                    PreparedFile::Json(expected) => serde_json::from_str::<Value>(&actual)
                        .map(|value| value == expected)
                        .unwrap_or(false),
                    PreparedFile::Text(expected) => actual == expected,
                };
                if !matches {
                    return Ok(false);
                }
            }
            OfficialLoginOperationKind::SetField => {
                let actual = match tokio::fs::read_to_string(&path).await {
                    Ok(content) => serde_json::from_str::<Value>(&content).map_err(|error| {
                        format!("目标文件 {} 不是合法 JSON: {}", path.display(), error)
                    })?,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
                    Err(error) => return Err(error.to_string()),
                };
                let expected = operation_value(&payload, operation)?;
                if value_at(&actual, &operation.path) != Some(&expected) {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}

pub async fn read_current_payload(db: &SqlitePool, agent_id: &str) -> Result<String, String> {
    let operations = resolved_operations(db, agent_id).await?;
    let config_dir = get_cli_config_dir_path(db, agent_id).await;
    let mut files: HashMap<String, OfficialCredentialFile> = HashMap::new();
    for operation in &operations {
        let path = resolve_cli_config_file_from_dir(&config_dir, &operation.file);
        match operation.op {
            OfficialLoginOperationKind::ReplaceFile => {
                let source = operation
                    .content_from
                    .as_ref()
                    .ok_or_else(|| format!("operation `{}` 缺少 content_from", operation.id))?;
                let format = operation_format(operation);
                let raw = match tokio::fs::read_to_string(&path).await {
                    Ok(content) => content,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
                    Err(error) => {
                        return Err(format!("读取目标文件 {} 失败: {}", path.display(), error))
                    }
                };
                let content = if format == ConfigFormat::Json {
                    if raw.trim().is_empty() {
                        Value::Object(Map::new())
                    } else {
                        serde_json::from_str(&raw).map_err(|error| {
                            format!("目标文件 {} 不是合法 JSON: {}", path.display(), error)
                        })?
                    }
                } else {
                    Value::String(raw)
                };
                files.insert(
                    source.file_id.clone(),
                    OfficialCredentialFile { format, content },
                );
            }
            OfficialLoginOperationKind::SetField => {
                let target = read_target_json(&path).await?;
                let Some(source) = &operation.value_from else {
                    continue;
                };
                let Some(current) = value_at(&target, &operation.path).cloned() else {
                    continue;
                };
                let file =
                    files
                        .entry(source.file_id.clone())
                        .or_insert_with(|| OfficialCredentialFile {
                            format: ConfigFormat::Json,
                            content: Value::Object(Map::new()),
                        });
                set_value_at(&mut file.content, &source.path, current, &operation.id)?;
            }
        }
    }
    serde_json::to_string(&OfficialCredentialPayload {
        schema_version: 1,
        files,
    })
    .map_err(|error| error.to_string())
}

fn find_display_value(value: &Value) -> Option<String> {
    const DISPLAY_KEYS: &[&str] = &["active_email", "email", "active", "account"];
    match value {
        Value::Object(object) => {
            for key in DISPLAY_KEYS {
                if let Some(value) = object.get(*key).and_then(Value::as_str) {
                    return Some(value.to_string());
                }
            }
            object.values().find_map(find_display_value)
        }
        Value::Array(values) => values.iter().find_map(find_display_value),
        _ => None,
    }
}

pub fn display_info(json: &str) -> String {
    match parse_payload(json) {
        Ok(payload) => payload
            .files
            .values()
            .find_map(|file| find_display_value(&file.content))
            .unwrap_or_else(|| format!("已保存 {} 个文件", payload.files.len())),
        Err(_) => "无效凭证格式".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_raw_text_credential_files() {
        let payload = parse_payload(
            r#"{"schema_version":1,"files":{"credentials":{"format":"toml","content":"token = 'demo'"}}}"#,
        )
        .expect("TOML credentials should be accepted");

        let file = payload.files.get("credentials").expect("credential file");
        assert_eq!(file.format, ConfigFormat::Toml);
        assert_eq!(file.content.as_str(), Some("token = 'demo'"));
    }
}
