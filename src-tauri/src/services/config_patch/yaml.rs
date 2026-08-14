use super::ResolvedOperation;
use crate::db::models::ConfigOperationKind;
use serde_json::{Map, Value};

fn parent_object_mut<'a>(
    root: &'a mut Value,
    path: &[String],
    create: bool,
    operation_id: &str,
) -> Result<Option<&'a mut Map<String, Value>>, String> {
    let mut current = root;
    for part in path {
        if current.get(part).is_none() {
            if !create {
                return Ok(None);
            }
            current
                .as_object_mut()
                .ok_or_else(|| format!("operation `{}` 路径结构冲突", operation_id))?
                .insert(part.clone(), Value::Object(Map::new()));
        }
        current = current
            .get_mut(part)
            .ok_or_else(|| format!("operation `{}` 路径不存在", operation_id))?;
        if !current.is_object() {
            return Err(format!("operation `{}` 路径结构冲突", operation_id));
        }
    }
    current
        .as_object_mut()
        .map(Some)
        .ok_or_else(|| format!("operation `{}` 目标不是对象", operation_id))
}

fn set_value(root: &mut Value, operation: &ResolvedOperation, value: Value) -> Result<(), String> {
    let (key, parent) = operation
        .path
        .split_last()
        .ok_or_else(|| format!("operation `{}` path 不能为空", operation.id))?;
    let target = parent_object_mut(root, parent, true, &operation.id)?
        .ok_or_else(|| format!("operation `{}` 路径不存在", operation.id))?;
    target.insert(key.clone(), value);
    Ok(())
}

fn remove_path(current: &mut Value, path: &[String], operation_id: &str) -> Result<(), String> {
    let Some((key, remaining)) = path.split_first() else {
        return Err(format!("operation `{}` path 不能为空", operation_id));
    };
    let object = current
        .as_object_mut()
        .ok_or_else(|| format!("operation `{}` 路径结构冲突", operation_id))?;
    if remaining.is_empty() {
        object.remove(key);
        return Ok(());
    }
    let Some(child) = object.get_mut(key) else {
        return Ok(());
    };
    remove_path(child, remaining, operation_id)?;
    if child.as_object().is_some_and(Map::is_empty) {
        object.remove(key);
    }
    Ok(())
}

fn remove_value(root: &mut Value, operation: &ResolvedOperation) -> Result<(), String> {
    remove_path(root, &operation.path, &operation.id)
}

fn parse_yaml_object(content: &str, label: &str) -> Result<Value, String> {
    if content.trim().is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    let value: Value = serde_yaml::from_str(content)
        .map_err(|error| format!("{} YAML 解析失败: {}", label, error))?;
    if !value.is_object() {
        return Err(format!("{} YAML 顶层必须是对象", label));
    }
    Ok(value)
}

pub(super) fn patch(content: &str, operations: &[ResolvedOperation]) -> Result<String, String> {
    let mut root = parse_yaml_object(content, "目标配置")?;
    for operation in operations {
        match operation.kind {
            ConfigOperationKind::Set => set_value(
                &mut root,
                operation,
                operation.value.clone().expect("resolved set value"),
            )?,
            ConfigOperationKind::Remove => remove_value(&mut root, operation)?,
        }
    }
    serde_yaml::to_string(&root).map_err(|error| error.to_string())
}

pub(super) fn parse_object(content: &str, label: &str) -> Result<Value, String> {
    parse_yaml_object(content, label)
}

pub(super) fn to_string(value: &Value) -> Result<String, String> {
    serde_yaml::to_string(value).map_err(|error| error.to_string())
}
