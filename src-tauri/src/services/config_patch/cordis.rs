use super::{yaml, ResolvedOperation};
use serde_json::{Map, Value};
use std::ops::Range;

const START: &str = "# >>> CCG Gateway Cordis Config >>>";
const END: &str = "# <<< CCG Gateway Cordis Config <<<";

pub(super) fn has_block(content: &str) -> bool {
    content.contains(START)
}

fn block(content: &str) -> Result<Option<(Range<usize>, &str)>, String> {
    let starts: Vec<_> = content.match_indices(START).collect();
    let ends: Vec<_> = content.match_indices(END).collect();
    match (starts.as_slice(), ends.as_slice()) {
        ([], []) => Ok(None),
        ([(start, _)], [(end, _)]) if start < end => {
            let body = start + START.len();
            Ok(Some((*start..end + END.len(), &content[body..*end])))
        }
        _ => Err(crate::tr!("err.cordis.block_invalid")),
    }
}

fn document(content: &str) -> Result<serde_yaml::Value, String> {
    let root: serde_yaml::Value = serde_yaml::from_str(content)
        .map_err(|error| crate::tr!("err.dsh_mcp.patch_parse_failed", error))?;
    if !root.is_null() && !root.is_sequence() {
        return Err(crate::tr!("err.dsh_mcp.patch_must_be_yaml_array"));
    }
    Ok(root)
}

pub(super) fn configs(content: &str) -> Result<Value, String> {
    document(content)?;
    let mut configs = Map::new();
    if let Some((_, body)) = block(content)? {
        let rows = match serde_yaml::from_str::<Value>(body)
            .map_err(|_| crate::tr!("err.cordis.block_invalid"))?
        {
            Value::Null => Vec::new(),
            Value::Array(rows) => rows,
            _ => return Err(crate::tr!("err.cordis.block_invalid")),
        };
        for row in rows {
            let id = row
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| crate::tr!("err.cordis.block_invalid"))?;
            let config = row
                .get("config")
                .filter(|value| value.is_object())
                .ok_or_else(|| crate::tr!("err.cordis.block_invalid"))?;
            if configs.insert(id.to_string(), config.clone()).is_some() {
                return Err(crate::tr!("err.cordis.block_invalid"));
            }
        }
    }
    Ok(Value::Object(configs))
}

fn row_extras(content: &str) -> Result<Map<String, Value>, String> {
    let Some((_, body)) = block(content)? else {
        return Ok(Map::new());
    };
    let rows = match serde_yaml::from_str::<Value>(body)
        .map_err(|_| crate::tr!("err.cordis.block_invalid"))?
    {
        Value::Null => Vec::new(),
        Value::Array(rows) => rows,
        _ => return Err(crate::tr!("err.cordis.block_invalid")),
    };
    let mut extras = Map::new();
    for row in rows {
        let Some(object) = row.as_object() else {
            continue;
        };
        let Some(id) = object.get("id").and_then(Value::as_str) else {
            continue;
        };
        let mut extra = Map::new();
        for (key, value) in object {
            if key != "id" && key != "config" {
                extra.insert(key.clone(), value.clone());
            }
        }
        if !extra.is_empty() {
            extras.insert(id.to_string(), Value::Object(extra));
        }
    }
    Ok(extras)
}

fn write(content: &str, configs: &Value) -> Result<String, String> {
    let root = document(content)?;
    let existing = block(content)?;
    let extras = row_extras(content)?;
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let rows: Vec<_> = configs
        .as_object()
        .expect("config mapping")
        .iter()
        .filter(|(_, config)| !config.as_object().is_some_and(Map::is_empty))
        .map(|(id, config)| {
            let mut row = Map::new();
            row.insert("id".to_string(), Value::String(id.clone()));
            if let Some(Value::Object(extra)) = extras.get(id) {
                for (key, value) in extra {
                    row.insert(key.clone(), value.clone());
                }
            }
            row.insert("config".to_string(), config.clone());
            Value::Object(row)
        })
        .collect();
    let rendered = if rows.is_empty() {
        String::new()
    } else {
        let yaml = serde_yaml::to_string(&rows).map_err(|error| error.to_string())?;
        format!(
            "{START}{newline}{}{newline}{END}",
            yaml.trim_end().replace('\n', newline)
        )
    };
    let next = if let Some((range, _)) = existing {
        let tail = &content[range.end..];
        if !rendered.is_empty() && tail.lines().any(|line| line.starts_with("- ")) {
            return write(&format!("{}{}", &content[..range.start], tail), configs);
        }
        format!("{}{}{}", &content[..range.start], rendered, tail)
    } else if rendered.is_empty() {
        return Ok(content.to_string());
    } else if root.as_sequence().is_some_and(Vec::is_empty) {
        // Keep comments around the conventional empty patch document.
        let offset = content
            .split_inclusive('\n')
            .scan(0, |offset, line| {
                let start = *offset;
                *offset += line.len();
                Some((start, line))
            })
            .find_map(|(offset, line)| {
                let trimmed = line.trim_start();
                (trimmed.starts_with("[]") && trimmed[2..].trim_start().starts_with('#')
                    || trimmed.trim_end() == "[]")
                    .then_some(offset + line.len() - trimmed.len())
            })
            .ok_or_else(|| crate::tr!("err.dsh_mcp.empty_patch_single_brackets"))?;
        format!(
            "{}{}{}",
            &content[..offset],
            rendered,
            &content[offset + 2..]
        )
    } else {
        let separator = if content.is_empty() || content.ends_with('\n') {
            ""
        } else {
            newline
        };
        format!("{content}{separator}{rendered}{newline}")
    };
    if document(&next)?.is_null() {
        Ok(format!("{}{newline}[]{newline}", next.trim_end()))
    } else {
        Ok(next)
    }
}

pub(super) fn applied(content: &str, operations: &[ResolvedOperation]) -> Result<bool, String> {
    let root = document(content)?;
    let Some(rows) = root.as_sequence() else {
        return Ok(false);
    };
    for operation in operations {
        let Some((id, path)) = operation.path.split_first() else {
            return Ok(false);
        };
        let mut current = rows
            .iter()
            .rev()
            .find(|row| {
                row.get("id").and_then(serde_yaml::Value::as_str) == Some(id.as_str())
                    && row.get("config").is_some()
            })
            .and_then(|row| row.get("config"));
        for key in path {
            current = current.and_then(|value| value.get(key.as_str()));
        }
        let expected = operation
            .value
            .as_ref()
            .map(serde_yaml::to_value)
            .transpose()
            .map_err(|error| error.to_string())?;
        if current != expected.as_ref() {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) fn patch(content: &str, operations: &[ResolvedOperation]) -> Result<String, String> {
    let current = yaml::to_string(&configs(content)?)?;
    let patched = yaml::patch(&current, operations)?;
    let configs = yaml::parse_object(&patched, "Cordis config")?;
    validate(&configs)?;
    write(content, &configs)
}

fn validate(configs: &Value) -> Result<(), String> {
    if configs.as_object().is_none_or(|configs| {
        configs
            .iter()
            .any(|(id, config)| id.is_empty() || !config.is_object())
    }) {
        return Err(crate::tr!("err.cordis.preset_invalid"));
    }
    Ok(())
}

pub(super) fn preset(content: &str, preset: &str, remove: bool) -> Result<String, String> {
    let value = yaml::parse_object(preset, "Cordis preset")?;
    validate(&value)?;
    let current = yaml::to_string(&configs(content)?)?;
    let next = if remove {
        super::preset::remove(crate::db::models::ConfigFormat::Yaml, &current, preset)?
    } else {
        super::preset::apply(crate::db::models::ConfigFormat::Yaml, &current, preset)?
    };
    write(content, &yaml::parse_object(&next, "Cordis config")?)
}

pub(super) fn reset(content: &str) -> Result<String, String> {
    write(content, &Value::Object(Map::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{AgentDefinition, ConfigFormat};
    use crate::services::config_patch::{self, PatchContext};

    fn fixture() -> (AgentDefinition, PatchContext) {
        (
            serde_json::from_str(include_str!(
                "../../../agent-definitions/deepseek_harness.json"
            ))
            .unwrap(),
            PatchContext {
                target_endpoint: "http://127.0.0.1:7788".into(),
                target_token: "test-gateway-token".into(),
                agent_id: "deepseek_harness".into(),
            },
        )
    }

    #[test]
    fn empty_managed_block_reads_as_empty_configs() {
        let content = "# >>> CCG Gateway Cordis Config >>>\n# <<< CCG Gateway Cordis Config <<<\n";
        assert_eq!(configs(content).unwrap(), serde_json::json!({}));
    }

    #[test]
    fn managed_block_preserves_row_metadata() {
        let content = "# >>> CCG Gateway Cordis Config >>>\n- id: ui-chat\n  name: \"@deepseek-ai/dsh-client-ui-chat\"\n  config:\n    transcriptView: standard\n# <<< CCG Gateway Cordis Config <<<\n";
        let parsed = configs(content).unwrap();
        assert_eq!(parsed["ui-chat"]["transcriptView"], "standard");

        let written = write(content, &parsed).unwrap();
        assert!(written.contains("name:"));
        assert_eq!(configs(&written).unwrap(), parsed);
    }

    #[test]
    fn enable_disable_preserves_mcp_user_patches_and_js_tags() {
        let (agent, context) = fixture();
        let operations: Vec<_> = agent
            .features
            .provider_config
            .operations
            .into_iter()
            .filter(|op| op.format == ConfigFormat::CordisYaml)
            .collect();
        let original = "# User settings\n- id: ui-settings\n  config:\n    enabled: !!js 'true'\n# >>> CCG Gateway DeepSeek Harness MCP >>>\n- insert:\n  - id: ccg-mcp-example\n    name: '@deepseek-ai/dsh-mcp-client'\n    config:\n      serverName: example\n      transport: stdio\n      command: test-command\n# <<< CCG Gateway DeepSeek Harness MCP <<<\n";
        let enabled =
            config_patch::patch_content(ConfigFormat::CordisYaml, original, &operations, &context)
                .unwrap();
        assert!(enabled.starts_with(original));
        assert!(config_patch::operations_applied(
            ConfigFormat::CordisYaml,
            &enabled,
            &operations,
            &context
        )
        .unwrap());
        let rows: serde_yaml::Value = serde_yaml::from_str(&enabled).unwrap();
        let deepseek = rows
            .as_sequence()
            .unwrap()
            .iter()
            .find(|row| row.get("id").and_then(serde_yaml::Value::as_str) == Some("llm-deepseek"))
            .unwrap();
        assert_eq!(
            deepseek["config"]["baseURL"].as_str(),
            Some(context.target_endpoint.as_str())
        );
        assert_eq!(
            deepseek["config"]["apiKeyEnv"].as_str(),
            Some("CCG_GATEWAY_API_KEY")
        );
        let pi_ai = rows
            .as_sequence()
            .unwrap()
            .iter()
            .find(|row| row.get("id").and_then(serde_yaml::Value::as_str) == Some("llm-pi-ai"))
            .unwrap();
        assert_eq!(
            pi_ai["config"]["providers"]["openai"]["baseURL"].as_str(),
            Some(context.target_endpoint.as_str())
        );
        assert_eq!(
            pi_ai["config"]["providers"]["anthropic"]["baseURL"].as_str(),
            Some(context.target_endpoint.as_str())
        );
        assert!(pi_ai["config"]["providers"]["openai"].get("api").is_none());
        assert!(pi_ai["config"]["providers"]["anthropic"]
            .get("api")
            .is_none());
        let disabled = config_patch::safely_remove_operations(
            ConfigFormat::CordisYaml,
            &enabled,
            &operations,
            &context,
        )
        .unwrap();
        assert_eq!(disabled.trim_end(), original.trim_end());
    }

    #[test]
    fn enabling_removes_legacy_api_overrides_from_managed_routes() {
        let (agent, context) = fixture();
        let operations: Vec<_> = agent
            .features
            .provider_config
            .operations
            .into_iter()
            .filter(|op| op.format == ConfigFormat::CordisYaml)
            .collect();
        let original = "# >>> CCG Gateway Cordis Config >>>\n- id: llm-pi-ai\n  config:\n    providers:\n      openai:\n        baseURL: https://old-gateway.example\n        apiKeyEnv: CCG_GATEWAY_API_KEY\n        api: openai-responses\n      anthropic:\n        baseURL: https://old-gateway.example\n        apiKeyEnv: CCG_GATEWAY_API_KEY\n        api: anthropic-messages\n# <<< CCG Gateway Cordis Config <<<\n";

        let enabled =
            config_patch::patch_content(ConfigFormat::CordisYaml, original, &operations, &context)
                .unwrap();
        let configs = configs(&enabled).unwrap();
        assert!(configs["llm-pi-ai"]["providers"]["openai"]
            .get("api")
            .is_none());
        assert!(configs["llm-pi-ai"]["providers"]["anthropic"]
            .get("api")
            .is_none());
        assert!(config_patch::operations_applied(
            ConfigFormat::CordisYaml,
            &enabled,
            &operations,
            &context
        )
        .unwrap());
    }

    #[test]
    fn overwrite_only_resets_owned_block_and_preset_cannot_override_gateway() {
        let (agent, context) = fixture();
        let operations: Vec<_> = agent
            .features
            .provider_config
            .operations
            .into_iter()
            .filter(|op| op.format == ConfigFormat::CordisYaml)
            .collect();
        let original = "# Preserved\r\n[] # Empty patch\r\n";
        let enabled =
            config_patch::patch_content(ConfigFormat::CordisYaml, original, &operations, &context)
                .unwrap();
        let preset = "llm-pi-ai:\n  providers:\n    openai:\n      baseURL: https://ignored.example\n      maxTokens: 1234\nui-settings:\n  enabled: true\n";
        let sanitized =
            config_patch::sanitize_preset(ConfigFormat::CordisYaml, preset, &operations, &context)
                .unwrap();
        let next =
            config_patch::apply_preset(ConfigFormat::CordisYaml, &enabled, &sanitized).unwrap();
        assert!(config_patch::operations_applied(
            ConfigFormat::CordisYaml,
            &next,
            &operations,
            &context
        )
        .unwrap());
        assert_eq!(
            configs(&next).unwrap()["llm-pi-ai"]["providers"]["openai"]["maxTokens"],
            1234
        );
        let reset = config_patch::reset_content(ConfigFormat::CordisYaml, &next).unwrap();
        assert!(reset.starts_with("# Preserved\r\n"));
        assert!(reset.contains("# Empty patch"));
        assert!(document(&reset).unwrap().as_sequence().unwrap().is_empty());
    }

    #[test]
    fn later_overrides_change_status_and_reapply_restores_priority() {
        let (agent, context) = fixture();
        let operations: Vec<_> = agent
            .features
            .provider_config
            .operations
            .into_iter()
            .filter(|op| op.format == ConfigFormat::CordisYaml)
            .collect();
        let enabled =
            config_patch::patch_content(ConfigFormat::CordisYaml, "", &operations, &context)
                .unwrap();
        let edited = format!(
            "{enabled}\n- id: llm-pi-ai\n  config:\n    providers:\n      openai:\n        baseURL: https://user.example\n"
        );
        assert!(!config_patch::operations_applied(
            ConfigFormat::CordisYaml,
            &edited,
            &operations,
            &context
        )
        .unwrap());
        let reapplied =
            config_patch::patch_content(ConfigFormat::CordisYaml, &edited, &operations, &context)
                .unwrap();
        assert!(config_patch::operations_applied(
            ConfigFormat::CordisYaml,
            &reapplied,
            &operations,
            &context
        )
        .unwrap());
        let disabled = config_patch::safely_remove_operations(
            ConfigFormat::CordisYaml,
            &edited,
            &operations,
            &context,
        )
        .unwrap();
        assert!(!disabled.contains(START));
        assert!(disabled.contains("https://user.example"));
    }

    #[test]
    fn gateway_credentials_keep_account_device_and_version_on_disable() {
        let (agent, context) = fixture();
        let operations: Vec<_> = agent
            .features
            .provider_config
            .operations
            .into_iter()
            .filter(|op| op.format == ConfigFormat::Yaml)
            .collect();
        let original = "version: 1\nrefs:\n  USER_KEY: keep\nrecords:\n  deepseek-account-platform/default:\n    kind: grant\n    payload:\n      version: 1\n      token: account-token\n      issuer: https://platform.deepseek.com\n  deepseek-account-platform/device:\n    kind: grant\n    payload: device\n";
        let enabled =
            config_patch::patch_content(ConfigFormat::Yaml, original, &operations, &context)
                .unwrap();
        let value: Value = serde_yaml::from_str(&enabled).unwrap();
        assert_eq!(value["refs"]["CCG_GATEWAY_API_KEY"], context.target_token);
        assert!(value.get("CCG_GATEWAY_API_KEY").is_none());
        let disabled = config_patch::safely_remove_operations(
            ConfigFormat::Yaml,
            &enabled,
            &operations,
            &context,
        )
        .unwrap();
        assert_eq!(
            serde_yaml::from_str::<Value>(&disabled).unwrap(),
            serde_yaml::from_str::<Value>(original).unwrap()
        );
    }
}
