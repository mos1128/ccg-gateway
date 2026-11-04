mod cordis;
mod env;
mod json;
mod jsonc;
mod preset;
mod toml;
mod writer;
mod yaml;

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
                return Err(crate::tr!(
                    "err.config_patch.missing_target_endpoint",
                    operation.id
                ));
            }
            ("{target.endpoint}", context.target_endpoint.as_str())
        } else if candidate.starts_with("{target.token}") {
            if context.target_token.is_empty() {
                return Err(crate::tr!(
                    "err.config_patch.missing_target_token",
                    operation.id
                ));
            }
            ("{target.token}", context.target_token.as_str())
        } else if candidate.starts_with("{agent.id}") {
            ("{agent.id}", context.agent_id.as_str())
        } else if candidate.starts_with("{target.")
            || candidate.starts_with("{agent.")
            || candidate.starts_with("{profile")
        {
            return Err(crate::tr!(
                "err.config_patch.unknown_template_variable",
                operation.id
            ));
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
        .ok_or_else(|| crate::tr!("err.config_patch.missing_value", operation.id))?;
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
                return Err(crate::tr!("err.config_patch.operation_id_required"));
            }
            if operation.path.is_empty() || operation.path.iter().any(|part| part.is_empty()) {
                return Err(crate::tr!("err.config_patch.path_required", operation.id));
            }
            let writes_value = operation.op == ConfigOperationKind::Set;
            let value = if writes_value {
                Some(resolve_value(operation, context)?)
            } else {
                if operation.value.is_some() {
                    return Err(crate::tr!(
                        "err.config_patch.value_not_accepted",
                        operation.id
                    ));
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
        ConfigFormat::Yaml => yaml::patch(content, &operations),
        ConfigFormat::CordisYaml => cordis::patch(content, &operations),
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
        ConfigFormat::Yaml => {
            let current: Value =
                serde_yaml::from_str(content).map_err(|error| error.to_string())?;
            let next: Value = serde_yaml::from_str(&patched).map_err(|error| error.to_string())?;
            Ok(current == next)
        }
        ConfigFormat::Env => Ok(content == patched),
        ConfigFormat::CordisYaml => Ok(cordis::configs(content)? == cordis::configs(&patched)?
            && cordis::applied(content, &resolve_operations(operations, context)?)?),
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
                return Err(crate::tr!(
                    "err.config_patch.read_file_failed",
                    task_path.display(),
                    error
                ));
            }
        };
        let next = patch_content(format, &content, &operations, &context)?;
        writer::write_atomic(&task_path, next.as_bytes())?;
        Ok(())
    })
    .await
    .map_err(|error| crate::tr!("err.config_patch.write_task_failed", error))??;
    Ok(path)
}

pub async fn write_atomic_json(path: &Path, value: &Value) -> Result<(), String> {
    let path = path.to_path_buf();
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    tokio::task::spawn_blocking(move || writer::write_atomic(&path, &bytes))
        .await
        .map_err(|error| crate::tr!("err.config_patch.write_task_failed", error))?
}

pub async fn write_atomic_text(path: &Path, content: &str) -> Result<(), String> {
    write_atomic_text_with_privacy(path, content, false).await
}

pub async fn write_atomic_text_with_privacy(
    path: &Path,
    content: &str,
    private: bool,
) -> Result<(), String> {
    let path = path.to_path_buf();
    let bytes = content.as_bytes().to_vec();
    tokio::task::spawn_blocking(move || writer::write_atomic_with_privacy(&path, &bytes, private))
        .await
        .map_err(|error| crate::tr!("err.config_patch.write_task_failed", error))?
}

fn inverse_operation(operation: &ConfigOperation) -> Option<ConfigOperation> {
    if operation.preserve_on_remove {
        return None;
    }
    let op = match operation.op {
        ConfigOperationKind::Set => ConfigOperationKind::Remove,
        ConfigOperationKind::Remove => return None,
    };
    Some(ConfigOperation {
        id: format!("disable-{}", operation.id),
        op,
        file: operation.file.clone(),
        format: operation.format,
        private: operation.private,
        preserve_on_remove: false,
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
        let matches = if format == ConfigFormat::CordisYaml {
            let owned = yaml::to_string(&cordis::configs(&current)?)?;
            operations_applied(
                ConfigFormat::Yaml,
                &owned,
                std::slice::from_ref(operation),
                context,
            )
        } else {
            operations_applied(format, &current, std::slice::from_ref(operation), context)
        };
        if !matches.unwrap_or(false) {
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
    if format == ConfigFormat::CordisYaml {
        return cordis::preset(content, preset_content, false);
    }
    preset::apply(format, content, preset_content)
}

pub fn safely_remove_preset(
    format: ConfigFormat,
    content: &str,
    preset_content: &str,
) -> Result<String, String> {
    if format == ConfigFormat::CordisYaml {
        return cordis::preset(content, preset_content, true);
    }
    preset::remove(format, content, preset_content)
}

pub fn reset_content(format: ConfigFormat, content: &str) -> Result<String, String> {
    if format == ConfigFormat::CordisYaml {
        cordis::reset(content)
    } else {
        Ok(String::new())
    }
}

pub fn has_cordis_block(content: &str) -> bool {
    cordis::has_block(content)
}

pub fn validate_preset(format: ConfigFormat, content: &str) -> Result<(), String> {
    apply_preset(format, "", content).map(|_| ())
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
                private: operation.private,
                preserve_on_remove: false,
                path: operation.path.clone(),
                value: None,
            })
        })
        .collect();
    if removals.is_empty() {
        return Ok(content.to_string());
    }
    patch_content(
        if format == ConfigFormat::CordisYaml {
            ConfigFormat::Yaml
        } else {
            format
        },
        content,
        &removals,
        context,
    )
}

#[cfg(test)]
mod template_tests {
    use super::*;
    use crate::db::models::AgentDefinition;
    use serde_json::Value;

    #[test]
    fn zcode_template_targets_desktop_only() {
        let definition: AgentDefinition =
            serde_json::from_str(include_str!("../../../agent-definitions/zcode.json"))
                .expect("zcode template");
        let operations = definition.features.provider_config.operations;
        assert!(!operations.is_empty());
        assert!(operations
            .iter()
            .all(|operation| operation.file == "v2/config.json"));
    }

    #[test]
    fn opencode_template_uses_v2_provider_keys() {
        let definition: AgentDefinition =
            serde_json::from_str(include_str!("../../../agent-definitions/opencode.json"))
                .expect("opencode template");
        let operations = definition.features.provider_config.operations;
        let context = PatchContext {
            target_endpoint: "https://gateway.test".to_string(),
            target_token: "token-123".to_string(),
            agent_id: "opencode".to_string(),
        };
        let output = patch_content(ConfigFormat::Json, "{}", &operations, &context).unwrap();
        let value: Value = serde_json::from_str(&output).unwrap();

        assert_eq!(
            value["providers"]["anthropic"]["settings"]["baseURL"],
            "https://gateway.test/v1"
        );
        assert_eq!(
            value["providers"]["openai"]["settings"]["apiKey"],
            "token-123"
        );
        assert!(value.get("provider").is_none());
        assert_eq!(
            definition.features.mcp.servers_path,
            vec!["mcp".to_string(), "servers".to_string()]
        );
    }
}

#[cfg(test)]
mod kimi_template_tests {
    use super::*;
    use crate::db::models::AgentDefinition;
    use serde_json::Value;

    #[test]
    fn kimi_template_sets_provider_protocols_and_paths() {
        let definition: AgentDefinition =
            serde_json::from_str(include_str!("../../../agent-definitions/kimi_code.json"))
                .expect("kimi template");
        let operations = definition.features.provider_config.operations;
        let context = PatchContext {
            target_endpoint: "https://gateway.test".to_string(),
            target_token: "token-123".to_string(),
            agent_id: "kimi_code".to_string(),
        };
        let output = patch_content(ConfigFormat::Toml, "", &operations, &context).unwrap();
        let value: ::toml::Value = ::toml::from_str(&output).unwrap();

        assert_eq!(
            value["providers"]["anthropic"]["type"].as_str(),
            Some("anthropic")
        );
        assert_eq!(
            value["providers"]["anthropic"]["base_url"].as_str(),
            Some("https://gateway.test")
        );
        assert_eq!(
            value["providers"]["anthropic"]["api_key"].as_str(),
            Some("token-123")
        );
        assert_eq!(
            value["providers"]["openai"]["type"].as_str(),
            Some("openai")
        );
        assert_eq!(
            value["providers"]["openai"]["base_url"].as_str(),
            Some("https://gateway.test")
        );

        let raw: Value =
            serde_json::from_str(include_str!("../../../agent-definitions/kimi_code.json"))
                .expect("kimi template json");
        assert_eq!(raw["config_dir_env"], "KIMI_CODE_HOME");
        assert_eq!(
            raw["features"]["official_login"]["operations"][0]["file"],
            "credentials/kimi-code.json"
        );
        assert_eq!(
            raw["features"]["official_login"]["managed_config"]["provider"],
            "managed:kimi-code"
        );
        assert_eq!(
            raw["features"]["official_login"]["managed_config"]["oauth_key"],
            "oauth/kimi-code"
        );
        assert_eq!(
            raw["features"]["official_login"]["managed_config"]["oauth_storage"],
            "file"
        );
    }
}

#[cfg(test)]
mod agent_template_path_tests {
    use crate::db::models::AgentDefinition;

    fn definition(source: &str) -> AgentDefinition {
        serde_json::from_str(source).expect("agent template")
    }

    fn first_operation_file(definition: &AgentDefinition) -> Option<&str> {
        definition
            .features
            .provider_config
            .operations
            .first()
            .map(|operation| operation.file.as_str())
    }

    #[test]
    fn pi_template_uses_settings_and_auth_files() {
        let definition = definition(include_str!("../../../agent-definitions/pi.json"));
        assert_eq!(
            definition.features.global_preset.file.as_deref(),
            Some("agent/settings.json")
        );
        assert_eq!(first_operation_file(&definition), Some("agent/models.json"));

        assert!(definition.features.official_login.enabled);
        let operation = definition
            .features
            .official_login
            .operations
            .first()
            .expect("pi auth operation");
        assert_eq!(operation.file, "agent/auth.json");
        assert_eq!(
            operation
                .content_from
                .as_ref()
                .map(|source| source.file_id.as_str()),
            Some("pi_auth")
        );
    }

    #[test]
    fn omp_template_uses_config_yml_for_global_preset() {
        let definition = definition(include_str!("../../../agent-definitions/omp.json"));
        assert_eq!(
            definition.features.global_preset.file.as_deref(),
            Some("agent/config.yml")
        );
        assert_eq!(first_operation_file(&definition), Some("agent/models.yml"));
    }

    #[test]
    fn grok_template_identifies_cli_and_shell() {
        let definition = definition(include_str!("../../../agent-definitions/grok.json"));
        assert!(definition
            .user_agent
            .iter()
            .any(|value| value == "grok-cli"));
        assert!(definition
            .user_agent
            .iter()
            .any(|value| value == "grok-shell"));
    }

    #[test]
    fn dsh_template_enables_readonly_sessions() {
        let definition = definition(include_str!(
            "../../../agent-definitions/deepseek_harness.json"
        ));
        assert!(definition.features.sessions.enabled);
        assert_eq!(definition.features.sessions.adapter.as_deref(), Some("dsh"));
    }
}
