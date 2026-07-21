use crate::db::models::ConfigFormat;
use serde_json::{Map, Value};
use toml_edit::{DocumentMut, Item, Table};

fn parse_json(content: &str, label: &str) -> Result<Value, String> {
    if content.trim().is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    let value = serde_json::from_str::<Value>(content)
        .map_err(|error| format!("{} JSON 解析失败: {}", label, error))?;
    value
        .is_object()
        .then_some(value)
        .ok_or_else(|| format!("{} JSON 顶层必须是对象", label))
}

fn merge_json(base: &mut Value, incoming: &Value) {
    let (Some(base), Some(incoming)) = (base.as_object_mut(), incoming.as_object()) else {
        return;
    };
    for (key, value) in incoming {
        match base.get_mut(key) {
            Some(current) if current.is_object() && value.is_object() => merge_json(current, value),
            Some(current) => *current = value.clone(),
            None => {
                base.insert(key.clone(), value.clone());
            }
        }
    }
}

fn remove_matching_json(base: &mut Value, owned: &Value) {
    let (Some(base), Some(owned)) = (base.as_object_mut(), owned.as_object()) else {
        return;
    };
    for (key, value) in owned {
        let remove = match base.get_mut(key) {
            Some(current) if current.is_object() && value.is_object() => {
                remove_matching_json(current, value);
                current.as_object().is_some_and(Map::is_empty)
            }
            Some(current) => current == value,
            None => false,
        };
        if remove {
            base.remove(key);
        }
    }
}

fn parse_toml(content: &str, label: &str) -> Result<DocumentMut, String> {
    if content.trim().is_empty() {
        Ok(DocumentMut::new())
    } else {
        content
            .parse::<DocumentMut>()
            .map_err(|error| format!("{} TOML 解析失败: {}", label, error))
    }
}

fn merge_toml(base: &mut Table, incoming: &Table) {
    for (key, value) in incoming.iter() {
        match (base.get_mut(key), value.as_table()) {
            (Some(current), Some(incoming_table)) if current.is_table() => {
                merge_toml(
                    current.as_table_mut().expect("checked TOML table"),
                    incoming_table,
                );
            }
            _ => {
                base.insert(key, value.clone());
            }
        }
    }
}

fn remove_matching_toml(base: &mut Table, owned: &Table) {
    for (key, value) in owned.iter() {
        let remove = match (base.get_mut(key), value.as_table()) {
            (Some(current), Some(owned_table)) if current.is_table() => {
                let current = current.as_table_mut().expect("checked TOML table");
                remove_matching_toml(current, owned_table);
                current.is_empty()
            }
            (Some(current), None) => items_equal(current, value),
            _ => false,
        };
        if remove {
            base.remove(key);
        }
    }
}

fn items_equal(left: &Item, right: &Item) -> bool {
    fn semantic_value(item: &Item) -> Option<::toml::Value> {
        let mut document = DocumentMut::new();
        document.insert("value", item.clone());
        ::toml::from_str::<::toml::Value>(&document.to_string())
            .ok()?
            .get("value")
            .cloned()
    }

    semantic_value(left)
        .zip(semantic_value(right))
        .is_some_and(|(left, right)| left == right)
}

pub(super) fn apply(
    format: ConfigFormat,
    content: &str,
    preset_content: &str,
) -> Result<String, String> {
    match format {
        ConfigFormat::Json => {
            let mut current = parse_json(content, "目标配置")?;
            let preset = parse_json(preset_content, "全局预设")?;
            merge_json(&mut current, &preset);
            serde_json::to_string_pretty(&current).map_err(|error| error.to_string())
        }
        ConfigFormat::Toml => {
            let mut current = parse_toml(content, "目标配置")?;
            let preset = parse_toml(preset_content, "全局预设")?;
            merge_toml(current.as_table_mut(), preset.as_table());
            Ok(current.to_string())
        }
        ConfigFormat::Jsonc | ConfigFormat::Env => {
            Err("global_preset 只支持 JSON 或 TOML".to_string())
        }
    }
}

pub(super) fn remove(
    format: ConfigFormat,
    content: &str,
    preset_content: &str,
) -> Result<String, String> {
    if preset_content.trim().is_empty() || content.trim().is_empty() {
        return Ok(content.to_string());
    }
    match format {
        ConfigFormat::Json => {
            let mut current = parse_json(content, "目标配置")?;
            let preset = parse_json(preset_content, "全局预设")?;
            remove_matching_json(&mut current, &preset);
            serde_json::to_string_pretty(&current).map_err(|error| error.to_string())
        }
        ConfigFormat::Toml => {
            let mut current = parse_toml(content, "目标配置")?;
            let preset = parse_toml(preset_content, "全局预设")?;
            remove_matching_toml(current.as_table_mut(), preset.as_table());
            Ok(current.to_string())
        }
        ConfigFormat::Jsonc | ConfigFormat::Env => {
            Err("global_preset 只支持 JSON 或 TOML".to_string())
        }
    }
}
