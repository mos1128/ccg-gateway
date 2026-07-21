use super::ResolvedOperation;
use crate::db::models::ConfigOperationKind;
use edikt_core::{Document, Step, Value as EdiktValue};
use serde_json::Value;

fn convert(value: &Value) -> Result<EdiktValue, String> {
    match value {
        Value::Null => Ok(EdiktValue::Null),
        Value::Bool(value) => Ok(EdiktValue::Bool(*value)),
        Value::Number(value) => value
            .as_i64()
            .map(EdiktValue::Int)
            .or_else(|| value.as_f64().map(EdiktValue::Float))
            .ok_or_else(|| "JSON 数字超出支持范围".to_string()),
        Value::String(value) => Ok(EdiktValue::Str(value.clone())),
        Value::Array(values) => values
            .iter()
            .map(convert)
            .collect::<Result<Vec<_>, _>>()
            .map(EdiktValue::Array),
        Value::Object(values) => values
            .iter()
            .map(|(key, value)| Ok((key.clone(), convert(value)?)))
            .collect::<Result<Vec<_>, String>>()
            .map(EdiktValue::Object),
    }
}

fn steps(path: &[String]) -> Vec<Step> {
    path.iter().cloned().map(Step::Field).collect()
}

pub(super) fn patch(content: &str, operations: &[ResolvedOperation]) -> Result<String, String> {
    let source = if content.trim().is_empty() {
        "{}"
    } else {
        content
    };
    let mut document = edikt_jsonc::parse(source).map_err(|error| error.to_string())?;
    for operation in operations {
        match operation.kind {
            ConfigOperationKind::Set => document
                .set(
                    &steps(&operation.path),
                    &convert(operation.value.as_ref().expect("resolved set value"))?,
                )
                .map_err(|error| format!("operation `{}`: {}", operation.id, error))?,
            ConfigOperationKind::Remove => document
                .delete(&steps(&operation.path))
                .map_err(|error| format!("operation `{}`: {}", operation.id, error))?,
        }
    }
    Ok(document.to_source())
}

pub(super) fn equivalent(left: &str, right: &str) -> Result<bool, String> {
    let left = edikt_jsonc::parse(left).map_err(|error| error.to_string())?;
    let right = edikt_jsonc::parse(right).map_err(|error| error.to_string())?;
    Ok(left.to_value() == right.to_value())
}
