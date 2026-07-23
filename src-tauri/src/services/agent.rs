use crate::db::models::{
    AdapterFeature, AgentDefinition, AgentDefinitionLoadError, AgentFeatures, AgentInfo,
    FileFeature, McpAdapter, McpFeature, Protocol,
};
use crate::time::now_timestamp;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

include!(concat!(env!("OUT_DIR"), "/agent_definitions.rs"));

const USER_AGENT_DEFINITIONS_DIR: &str = "agent-definitions";
const AGENT_DEFINITION_SCHEMA_FILE: &str = "agent-definition.schema.json";

struct AgentRegistry {
    definitions: Vec<AgentDefinition>,
    errors: Vec<AgentDefinitionLoadError>,
}

#[derive(Debug, Clone)]
pub struct AgentMatch {
    pub selected: AgentInfo,
    pub matched_agents: Vec<String>,
}

fn validate_config_operations(definition: &AgentDefinition) -> Result<(), String> {
    let feature = &definition.features.provider_config;
    if !feature.enabled {
        if definition.features.profiles.enabled {
            return Err("enabled profiles 需要启用 provider_config".to_string());
        }
        return Ok(());
    }
    if feature.operations.is_empty() {
        return Err("enabled provider_config 必须提供 operations".to_string());
    }

    fn validate_operations(
        operations: &[crate::db::models::ConfigOperation],
        label: &str,
    ) -> Result<(), String> {
        let mut operation_ids = HashSet::new();
        let mut file_formats = HashMap::new();
        for operation in operations {
            if operation.id.trim().is_empty() || !operation_ids.insert(operation.id.as_str()) {
                return Err(format!(
                    "{} operation id `{}` 为空或重复",
                    label, operation.id
                ));
            }
            if operation.file.trim().is_empty() {
                return Err(format!(
                    "{} operation `{}` 的 file 不能为空",
                    label, operation.id
                ));
            }
            if operation.path.is_empty() || operation.path.iter().any(|part| part.trim().is_empty())
            {
                return Err(format!(
                    "{} operation `{}` 的 path 不能为空",
                    label, operation.id
                ));
            }
            if let Some(existing) = file_formats.insert(operation.file.as_str(), operation.format) {
                if existing != operation.format {
                    return Err(format!(
                        "{} 文件 `{}` 不能同时使用不同 format",
                        label, operation.file
                    ));
                }
            }

            use crate::db::models::{ConfigFormat, ConfigOperationKind};
            let is_env = operation.format == ConfigFormat::Env;
            if is_env && operation.path.len() != 1 {
                return Err(format!(
                    "{} operation `{}` 的 ENV path 必须只有一个 key",
                    label, operation.id
                ));
            }
            let writes_value = operation.op == ConfigOperationKind::Set;
            if writes_value && operation.value.is_none() {
                return Err(format!("{} operation `{}` 缺少 value", label, operation.id));
            }
            if !writes_value && operation.value.is_some() {
                return Err(format!(
                    "{} operation `{}` 不接受 value",
                    label, operation.id
                ));
            }

            let path_text = operation.path.join("\n");
            if operation.file.contains("{target.")
                || operation.file.contains("{agent.")
                || path_text.contains("{target.")
                || path_text.contains("{agent.")
            {
                return Err(format!(
                    "{} operation `{}` 的 target/agent 模板变量只能用于 value",
                    label, operation.id
                ));
            }
            let value_text = operation
                .value
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default();
            if label == "provider_config"
                && (operation.file.contains("{profile")
                    || path_text.contains("{profile")
                    || value_text.contains("{profile"))
            {
                return Err(format!(
                    "{} operation `{}` 不能使用 Profile 模板变量",
                    label, operation.id
                ));
            }
            if operation.file.contains("{profile.absolute_path}")
                || path_text.contains("{profile.absolute_path}")
                || value_text.contains("{profile.relative_path}")
                || value_text.contains("{profile.absolute_path}")
            {
                return Err(format!(
                    "{} operation `{}` 的 Profile 路径模板变量位置无效",
                    label, operation.id
                ));
            }
        }
        Ok(())
    }

    validate_operations(&feature.operations, "provider_config")?;
    if definition.features.profiles.enabled {
        validate_operations(&definition.features.profiles.operations, "profiles")?;
    }
    Ok(())
}

fn validate_global_preset(definition: &AgentDefinition) -> Result<(), String> {
    let feature = &definition.features.global_preset;
    if !feature.enabled {
        return Ok(());
    }
    if feature
        .file
        .as_deref()
        .map(str::trim)
        .filter(|file| !file.is_empty())
        .is_none()
    {
        return Err("enabled global_preset 必须提供 file".to_string());
    }
    if !matches!(
        feature.format,
        Some(crate::db::models::ConfigFormat::Json | crate::db::models::ConfigFormat::Toml)
    ) {
        return Err("enabled global_preset 只支持 json 或 toml".to_string());
    }
    Ok(())
}

fn validate_official_login(definition: &AgentDefinition) -> Result<(), String> {
    let feature = &definition.features.official_login;
    if !feature.enabled {
        return Ok(());
    }
    if feature.operations.is_empty() {
        return Err("enabled official_login 必须提供 operations".to_string());
    }
    let mut source_formats = HashMap::new();
    for operation in &feature.operations {
        use crate::db::models::{ConfigFormat, OfficialLoginOperationKind};
        let format = operation.format.unwrap_or(ConfigFormat::Json);
        let source = match operation.op {
            OfficialLoginOperationKind::ReplaceFile => {
                let source = operation.content_from.as_ref().ok_or_else(|| {
                    format!(
                        "official_login operation `{}` 缺少 content_from",
                        operation.id
                    )
                })?;
                if format != ConfigFormat::Json && !source.path.is_empty() {
                    return Err(format!(
                        "official_login operation `{}` 的非 JSON 来源不能配置 path",
                        operation.id
                    ));
                }
                Some(source)
            }
            OfficialLoginOperationKind::SetField => {
                if operation.path.is_empty()
                    || operation.value.is_some() == operation.value_from.is_some()
                    || operation.format != Some(ConfigFormat::Json)
                {
                    return Err(format!(
                        "official_login operation `{}` 必须使用 JSON format，并正确配置 path 和 value",
                        operation.id
                    ));
                }
                operation.value_from.as_ref()
            }
        };
        if let Some(source) = source {
            if let Some(existing) = source_formats.insert(source.file_id.as_str(), format) {
                if existing != format {
                    return Err(format!(
                        "official_login 逻辑文件 `{}` 不能使用不同 format",
                        source.file_id
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_icon(definition: &AgentDefinition) -> Result<(), String> {
    let Some(icon) = &definition.icon else {
        return Ok(());
    };
    if icon.view_box.trim().is_empty()
        || icon.paths.is_empty()
        || icon.paths.iter().any(|path| path.d.trim().is_empty())
    {
        return Err("Agent icon 必须提供 view_box 和非空 paths".to_string());
    }
    if icon.color.as_ref().is_some_and(|color| {
        color.len() != 7
            || !color.starts_with('#')
            || !color[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
    }) {
        return Err("Agent icon color 必须是六位十六进制颜色".to_string());
    }
    for path in &icon.paths {
        if path
            .opacity
            .is_some_and(|opacity| !(0.0..=1.0).contains(&opacity))
        {
            return Err("Agent icon path opacity 必须在 0 到 1 之间".to_string());
        }
        for rule in [&path.fill_rule, &path.clip_rule].into_iter().flatten() {
            if rule != "nonzero" && rule != "evenodd" {
                return Err("Agent icon path rule 只支持 nonzero 或 evenodd".to_string());
            }
        }
    }
    Ok(())
}

fn validate_skills(definition: &AgentDefinition) -> Result<(), String> {
    let feature = &definition.features.skills;
    if feature.enabled
        && feature
            .directory
            .as_deref()
            .map(str::trim)
            .filter(|directory| !directory.is_empty())
            .is_none()
    {
        return Err("enabled skills 必须提供 directory".to_string());
    }
    Ok(())
}

fn validate_mcp(feature: &McpFeature) -> Result<(), String> {
    if !feature.enabled {
        return Ok(());
    }
    if feature
        .file
        .as_deref()
        .map(str::trim)
        .filter(|file| !file.is_empty())
        .is_none()
    {
        return Err("enabled mcp 必须提供 file".to_string());
    }
    if !matches!(
        feature.format,
        Some(crate::db::models::ConfigFormat::Json | crate::db::models::ConfigFormat::Toml)
    ) {
        return Err("enabled mcp 只支持 json 或 toml".to_string());
    }
    if feature.adapter == Some(McpAdapter::Opencode)
        && feature.format != Some(crate::db::models::ConfigFormat::Json)
    {
        return Err("mcp adapter opencode 只支持 json format".to_string());
    }
    if feature.servers_path.is_empty()
        || feature
            .servers_path
            .iter()
            .any(|part| part.trim().is_empty())
    {
        return Err("enabled mcp 必须提供非空 servers_path".to_string());
    }
    Ok(())
}

fn validate_adapter(feature: &AdapterFeature, label: &str) -> Result<(), String> {
    if feature.enabled
        && feature
            .adapter
            .as_deref()
            .map(str::trim)
            .filter(|adapter| !adapter.is_empty())
            .is_none()
    {
        return Err(format!("enabled {label} 必须提供 adapter"));
    }
    Ok(())
}

fn validate_file_feature(feature: &FileFeature, label: &str) -> Result<(), String> {
    if feature.enabled
        && feature
            .file
            .as_deref()
            .map(str::trim)
            .filter(|file| !file.is_empty())
            .is_none()
    {
        return Err(format!("enabled {label} 必须提供 file"));
    }
    Ok(())
}

fn validate_profiles(definition: &AgentDefinition) -> Result<(), String> {
    let feature = &definition.features.profiles;
    if !feature.enabled {
        return Ok(());
    }

    let profile_file = feature
        .profile_file
        .as_deref()
        .filter(|file| !file.trim().is_empty())
        .ok_or_else(|| "enabled profiles 必须提供 profile_file".to_string())?;
    if !profile_file.contains("{profile}") {
        return Err("profiles.profile_file 必须包含 {profile}".to_string());
    }
    if feature.operations.is_empty() {
        return Err("enabled profiles 必须提供 operations".to_string());
    }

    if let Some(launch) = &feature.launch {
        for (label, args) in [
            ("default", &launch.default),
            ("non_default", &launch.non_default),
        ] {
            if args.iter().any(|arg| arg.trim().is_empty()) {
                return Err(format!("profiles.launch.{} 不能包含空参数", label));
            }
            if args
                .iter()
                .any(|arg| arg.contains("{target.") || arg.contains("{agent."))
            {
                return Err(format!(
                    "profiles.launch.{} 不能使用 target/agent 模板变量",
                    label
                ));
            }
            if args
                .iter()
                .any(|arg| arg.contains("{profile.relative_path}"))
            {
                return Err(format!(
                    "profiles.launch.{} 不能使用 {{profile.relative_path}}",
                    label
                ));
            }
            if label == "default"
                && args
                    .iter()
                    .any(|arg| arg.contains("{profile.absolute_path}"))
            {
                return Err("profiles.launch.default 不能使用 {profile.absolute_path}".to_string());
            }
        }
    }

    Ok(())
}

fn validate_definition(definition: &AgentDefinition) -> Result<(), String> {
    if definition.schema_version != 1 {
        return Err(format!(
            "不支持 schema_version {}",
            definition.schema_version
        ));
    }
    if definition.id.is_empty()
        || !definition.id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        })
    {
        return Err("Agent id 格式无效".to_string());
    }
    if definition.name.trim().is_empty() {
        return Err("Agent name 不能为空".to_string());
    }
    if definition
        .remark
        .as_deref()
        .is_some_and(|remark| remark.trim().is_empty())
    {
        return Err("Agent remark 不能为空".to_string());
    }
    validate_icon(definition)?;
    if definition.config_dir.trim().is_empty() {
        return Err("Agent config_dir 不能为空".to_string());
    }
    if definition.sort_order < 0 {
        return Err("Agent sort_order 不能小于 0".to_string());
    }
    if definition.user_agent.is_empty()
        || definition
            .user_agent
            .iter()
            .any(|pattern| pattern.trim().is_empty())
    {
        return Err("user_agent 必须包含非空匹配项".to_string());
    }
    if definition.protocols.is_empty() {
        return Err("protocols 不能为空".to_string());
    }
    let mut protocols = HashSet::new();
    if definition
        .protocols
        .iter()
        .any(|protocol| !protocols.insert(*protocol))
    {
        return Err("protocols 不能重复".to_string());
    }
    validate_config_operations(definition)?;
    validate_global_preset(definition)?;
    validate_profiles(definition)?;
    validate_official_login(definition)?;
    validate_skills(definition)?;
    validate_mcp(&definition.features.mcp)?;
    validate_adapter(&definition.features.sessions, "sessions")?;
    validate_adapter(&definition.features.plugins, "plugins")?;
    validate_file_feature(&definition.features.prompts, "prompts")?;
    Ok(())
}

fn parse_definition(
    source: &str,
    json: &str,
    expected_id: &str,
) -> Result<AgentDefinition, AgentDefinitionLoadError> {
    let definition = serde_json::from_str::<AgentDefinition>(json).map_err(|error| {
        AgentDefinitionLoadError {
            source: source.to_string(),
            message: error.to_string(),
        }
    })?;
    validate_definition(&definition)
        .and_then(|_| {
            if definition.id == expected_id {
                Ok(())
            } else {
                Err(format!("文件名必须与 Agent id `{}` 一致", definition.id))
            }
        })
        .map_err(|message| AgentDefinitionLoadError {
            source: source.to_string(),
            message,
        })?;
    Ok(definition)
}

fn load_user_definitions(
    directory: &Path,
    definitions: &mut HashMap<String, AgentDefinition>,
    errors: &mut Vec<AgentDefinitionLoadError>,
) {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            errors.push(AgentDefinitionLoadError {
                source: directory.display().to_string(),
                message: format!("读取用户模板目录失败: {error}"),
            });
            return;
        }
    };

    let mut files = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                let is_json = path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
                let is_schema = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.eq_ignore_ascii_case(AGENT_DEFINITION_SCHEMA_FILE));
                if path.is_file() && is_json && !is_schema {
                    files.push(path);
                }
            }
            Err(error) => errors.push(AgentDefinitionLoadError {
                source: directory.display().to_string(),
                message: format!("读取用户模板目录项失败: {error}"),
            }),
        }
    }
    files.sort();

    for file in files {
        let source = file.display().to_string();
        let Some(expected_id) = file.file_stem().and_then(|stem| stem.to_str()) else {
            errors.push(AgentDefinitionLoadError {
                source,
                message: "模板文件名不是有效 UTF-8".to_string(),
            });
            continue;
        };
        let json = match fs::read_to_string(&file) {
            Ok(json) => json,
            Err(error) => {
                errors.push(AgentDefinitionLoadError {
                    source,
                    message: format!("读取用户模板失败: {error}"),
                });
                continue;
            }
        };
        match parse_definition(&source, &json, expected_id) {
            Ok(definition) => {
                definitions.insert(definition.id.clone(), definition);
            }
            Err(error) => errors.push(error),
        }
    }
}

fn load_registry_from_user_dir(user_definitions_dir: Option<&Path>) -> AgentRegistry {
    let mut definitions = HashMap::new();
    let mut errors = Vec::new();

    for (source, json) in BUILTIN_AGENT_DEFINITION_JSON {
        match parse_definition(source, json, source) {
            Ok(definition) => {
                if definitions
                    .insert(definition.id.clone(), definition)
                    .is_some()
                {
                    errors.push(AgentDefinitionLoadError {
                        source: (*source).to_string(),
                        message: format!("Agent id `{source}` 重复"),
                    });
                }
            }
            Err(error) => errors.push(error),
        }
    }

    if let Some(directory) = user_definitions_dir {
        load_user_definitions(directory, &mut definitions, &mut errors);
    }

    let mut definitions: Vec<_> = definitions.into_values().collect();
    definitions.sort_by(|left, right| {
        left.sort_order
            .cmp(&right.sort_order)
            .then_with(|| left.id.cmp(&right.id))
    });

    AgentRegistry {
        definitions,
        errors,
    }
}

fn user_agent_definitions_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(USER_AGENT_DEFINITIONS_DIR)
}

fn load_registry() -> AgentRegistry {
    let directory = user_agent_definitions_dir(&crate::config::get_data_dir());
    load_registry_from_user_dir(Some(&directory))
}

fn registry() -> &'static AgentRegistry {
    static REGISTRY: OnceLock<AgentRegistry> = OnceLock::new();
    REGISTRY.get_or_init(load_registry)
}

pub fn definitions() -> &'static [AgentDefinition] {
    &registry().definitions
}

pub fn definition_load_errors() -> &'static [AgentDefinitionLoadError] {
    &registry().errors
}

pub fn get_definition(agent_id: &str) -> Option<&'static AgentDefinition> {
    definitions()
        .iter()
        .find(|definition| definition.id == agent_id)
}

pub fn validate_agent_id(agent_id: &str) -> Result<String, String> {
    let agent_id = agent_id.trim();
    get_definition(agent_id)
        .map(|_| agent_id.to_string())
        .ok_or_else(|| format!("未知 Agent: {}", agent_id))
}

pub fn default_config_directory(agent_id: &str) -> Option<&'static str> {
    get_definition(agent_id).map(|definition| definition.config_dir.as_str())
}

fn feature_exists(features: &AgentFeatures, feature: &str) -> bool {
    match feature {
        "provider_config" => features.provider_config.enabled,
        "global_preset" => features.global_preset.enabled,
        "profiles" => features.profiles.enabled,
        "official_login" => features.official_login.enabled,
        "model_mapping" => features.model_mapping.enabled,
        "token_usage" => features.token_usage.enabled,
        "skills" => features.skills.enabled,
        "mcp" => features.mcp.enabled,
        "sessions" => features.sessions.enabled,
        "plugins" => features.plugins.enabled,
        "prompts" => features.prompts.enabled,
        _ => false,
    }
}

pub fn agent_ids_for_feature(feature: &str) -> Vec<&'static str> {
    definitions()
        .iter()
        .filter(|definition| feature_exists(&definition.features, feature))
        .map(|definition| definition.id.as_str())
        .collect()
}

fn resolve_definition(definition: &AgentDefinition) -> AgentInfo {
    AgentInfo {
        schema_version: definition.schema_version,
        id: definition.id.clone(),
        name: definition.name.clone(),
        remark: definition.remark.clone(),
        icon: definition.icon.clone(),
        config_dir: definition.config_dir.clone(),
        user_agent: definition.user_agent.clone(),
        protocols: definition.protocols.clone(),
        features: definition.features.clone(),
    }
}

pub async fn ordered_agents(_db: &SqlitePool) -> Result<Vec<AgentInfo>, sqlx::Error> {
    Ok(definitions().iter().map(resolve_definition).collect())
}

pub async fn get_agent(db: &SqlitePool, agent_id: &str) -> Result<Option<AgentInfo>, sqlx::Error> {
    Ok(ordered_agents(db)
        .await?
        .into_iter()
        .find(|agent| agent.id == agent_id))
}

pub async fn match_user_agent(
    db: &SqlitePool,
    user_agent: &str,
) -> Result<Option<AgentMatch>, sqlx::Error> {
    let user_agent = user_agent.to_lowercase();
    let matches: Vec<AgentInfo> = ordered_agents(db)
        .await?
        .into_iter()
        .filter(|agent| {
            agent
                .user_agent
                .iter()
                .any(|pattern| user_agent.contains(&pattern.to_lowercase()))
        })
        .collect();

    let Some(selected) = matches.first().cloned() else {
        return Ok(None);
    };
    Ok(Some(AgentMatch {
        selected,
        matched_agents: matches.into_iter().map(|agent| agent.id).collect(),
    }))
}

pub fn protocol_allowed(agent: &AgentInfo, protocol: Protocol) -> bool {
    agent.protocols.contains(&protocol)
}

pub async fn record_diagnostic(
    log_db: &SqlitePool,
    kind: &str,
    key: &str,
    payload: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    let now = now_timestamp();
    sqlx::query(
        r#"
        INSERT INTO agent_diagnostics
            (kind, key, payload_json, first_seen, last_seen, occurrence_count)
        VALUES (?, ?, ?, ?, ?, 1)
        ON CONFLICT(kind, key) DO UPDATE SET
            payload_json = excluded.payload_json,
            last_seen = excluded.last_seen,
            occurrence_count = occurrence_count + 1
        "#,
    )
    .bind(kind)
    .bind(key)
    .bind(payload.to_string())
    .bind(now)
    .bind(now)
    .execute(log_db)
    .await?;
    Ok(())
}
