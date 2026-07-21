use super::ResolvedOperation;
use crate::db::models::ConfigOperationKind;
use serde_json::Value;
use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, Value as TomlValue};

fn to_toml_value(value: &Value) -> Result<TomlValue, String> {
    match value {
        Value::Null => Err("TOML 不支持 null".to_string()),
        Value::Bool(value) => Ok(TomlValue::from(*value)),
        Value::Number(value) => value
            .as_i64()
            .map(TomlValue::from)
            .or_else(|| value.as_f64().map(TomlValue::from))
            .ok_or_else(|| "TOML 数字超出支持范围".to_string()),
        Value::String(value) => Ok(TomlValue::from(value.as_str())),
        Value::Array(values) => {
            let mut array = Array::new();
            for value in values {
                array.push(to_toml_value(value)?);
            }
            Ok(TomlValue::Array(array))
        }
        Value::Object(values) => {
            let mut table = InlineTable::new();
            for (key, value) in values {
                table.insert(key, to_toml_value(value)?);
            }
            Ok(TomlValue::InlineTable(table))
        }
    }
}

fn to_item(value: &Value) -> Result<Item, String> {
    if let Value::Object(values) = value {
        let mut table = Table::new();
        for (key, value) in values {
            table.insert(key, to_item(value)?);
        }
        Ok(Item::Table(table))
    } else {
        Ok(Item::Value(to_toml_value(value)?))
    }
}

fn table_mut<'a>(
    table: &'a mut Table,
    path: &[String],
    create: bool,
    operation_id: &str,
) -> Result<Option<&'a mut Table>, String> {
    let mut current = table;
    for key in path {
        if !current.contains_key(key) {
            if !create {
                return Ok(None);
            }
            current.insert(key, Item::Table(Table::new()));
        }
        current = current
            .get_mut(key)
            .and_then(Item::as_table_mut)
            .ok_or_else(|| format!("operation `{}` 路径结构冲突", operation_id))?;
    }
    Ok(Some(current))
}

fn set(
    document: &mut DocumentMut,
    operation: &ResolvedOperation,
    value: &Value,
) -> Result<(), String> {
    let (key, parent) = operation
        .path
        .split_last()
        .ok_or_else(|| format!("operation `{}` path 不能为空", operation.id))?;
    let table = table_mut(document.as_table_mut(), parent, true, &operation.id)?
        .expect("created TOML path");
    table.insert(key, to_item(value)?);
    Ok(())
}

fn remove_path(table: &mut Table, path: &[String], operation_id: &str) -> Result<(), String> {
    let Some((key, remaining)) = path.split_first() else {
        return Err(format!("operation `{}` path 不能为空", operation_id));
    };
    if remaining.is_empty() {
        table.remove(key);
        return Ok(());
    }
    let Some(child) = table.get_mut(key) else {
        return Ok(());
    };
    let child_table = child
        .as_table_mut()
        .ok_or_else(|| format!("operation `{}` 路径结构冲突", operation_id))?;
    remove_path(child_table, remaining, operation_id)?;
    if child_table.is_empty() {
        table.remove(key);
    }
    Ok(())
}

pub(super) fn patch(content: &str, operations: &[ResolvedOperation]) -> Result<String, String> {
    let mut document = if content.trim().is_empty() {
        DocumentMut::new()
    } else {
        content
            .parse::<DocumentMut>()
            .map_err(|error| format!("TOML 解析失败: {}", error))?
    };
    for operation in operations {
        match operation.kind {
            ConfigOperationKind::Set => set(
                &mut document,
                operation,
                operation.value.as_ref().expect("resolved set value"),
            )?,
            ConfigOperationKind::Remove => {
                remove_path(document.as_table_mut(), &operation.path, &operation.id)?
            }
        }
    }
    Ok(document.to_string())
}
