use crate::config::shrink_home_path;
use crate::db::models::{AgentInfo, ConfigFormat, ConfigOperation, Provider};
use crate::services::agent;
use crate::services::cli_config::{get_cli_config_dir_path, resolve_cli_config_file_from_dir};
use crate::services::config_patch::{self, PatchContext};
use crate::services::provider_profile::validate_provider_profile;
use crate::services::routing::{gateway_token_for_profile, DEFAULT_PROFILE};
use serde::Serialize;
use serde_json::Value;
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct OperationGroup {
    path: PathBuf,
    format: ConfigFormat,
    operations: Vec<ConfigOperation>,
}

#[derive(Debug)]
struct PresetTarget {
    path: PathBuf,
    format: ConfigFormat,
    current: String,
    previous: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ProfileTemplateContext {
    pub profile: String,
    relative_path: Option<String>,
    absolute_path: Option<String>,
}

fn resolve_profile_pattern(pattern: &str, profile: &str) -> String {
    pattern.replace("{profile}", profile)
}

pub fn profile_template_context(
    resolved: &AgentInfo,
    profile: &str,
) -> Result<ProfileTemplateContext, String> {
    let profile = validate_provider_profile(Some(profile))?;
    if !resolved.features.profiles.enabled && profile != DEFAULT_PROFILE {
        return Err(format!("Agent {} 不支持 Profile", resolved.name));
    }

    let relative_path = (profile != DEFAULT_PROFILE)
        .then(|| resolved.features.profiles.profile_file.as_deref())
        .flatten()
        .map(|pattern| resolve_profile_pattern(pattern, &profile));

    Ok(ProfileTemplateContext {
        profile,
        relative_path,
        absolute_path: None,
    })
}

fn replace_profile_placeholders(
    input: &str,
    context: &ProfileTemplateContext,
) -> Result<String, String> {
    let mut output = input.to_string();
    output = output.replace("{profile}", &context.profile);
    if output.contains("{profile.relative_path}") {
        let value = context
            .relative_path
            .as_deref()
            .ok_or_else(|| "当前 Profile 模板未提供 relative_path".to_string())?;
        output = output.replace("{profile.relative_path}", value);
    }
    if output.contains("{profile.absolute_path}") {
        let value = context
            .absolute_path
            .as_deref()
            .ok_or_else(|| "当前 Profile 未提供 absolute_path".to_string())?;
        output = output.replace("{profile.absolute_path}", value);
    }
    if output.contains("{profile") {
        return Err(format!("未知 Profile 模板变量: {}", output));
    }
    Ok(output)
}

fn resolve_template_value(
    value: &Value,
    context: &ProfileTemplateContext,
) -> Result<Value, String> {
    match value {
        Value::String(value) => replace_profile_placeholders(value, context).map(Value::String),
        Value::Array(values) => values
            .iter()
            .map(|value| resolve_template_value(value, context))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(values) => values
            .iter()
            .map(|(key, value)| {
                Ok((
                    replace_profile_placeholders(key, context)?,
                    resolve_template_value(value, context)?,
                ))
            })
            .collect::<Result<serde_json::Map<_, _>, String>>()
            .map(Value::Object),
        _ => Ok(value.clone()),
    }
}

fn resolve_operations(
    resolved: &AgentInfo,
    profile: &str,
    operations: &[ConfigOperation],
) -> Result<Vec<ConfigOperation>, String> {
    let context = profile_template_context(resolved, profile)?;
    operations
        .iter()
        .map(|operation| {
            Ok(ConfigOperation {
                id: operation.id.clone(),
                op: operation.op,
                file: replace_profile_placeholders(&operation.file, &context)?,
                format: operation.format,
                path: operation
                    .path
                    .iter()
                    .map(|part| replace_profile_placeholders(part, &context))
                    .collect::<Result<Vec<_>, _>>()?,
                value: operation
                    .value
                    .as_ref()
                    .map(|value| resolve_template_value(value, &context))
                    .transpose()?,
            })
        })
        .collect()
}

fn operations_for_profile<'a>(
    resolved: &'a AgentInfo,
    profile: &str,
) -> Result<&'a [ConfigOperation], String> {
    let profile = validate_provider_profile(Some(profile))?;
    if profile == DEFAULT_PROFILE {
        return Ok(&resolved.features.provider_config.operations);
    }
    if !resolved.features.profiles.enabled {
        return Err(format!("Agent {} 不支持 Profile", resolved.name));
    }
    Ok(&resolved.features.profiles.operations)
}

fn resolve_profile_operations(
    resolved: &AgentInfo,
    profile: &str,
) -> Result<Vec<ConfigOperation>, String> {
    resolve_operations(
        resolved,
        profile,
        operations_for_profile(resolved, profile)?,
    )
}

pub async fn profile_operation_paths(
    db: &SqlitePool,
    agent_id: &str,
    profile: &str,
) -> Result<Vec<PathBuf>, String> {
    let resolved = resolved_agent(db, agent_id).await?;
    let operations = resolve_profile_operations(&resolved, profile)?;
    let config_dir = get_cli_config_dir_path(db, agent_id).await;
    Ok(group_operations(&config_dir, &operations)?
        .into_iter()
        .map(|group| group.path)
        .collect())
}

fn group_operations(
    config_dir: &Path,
    operations: &[ConfigOperation],
) -> Result<Vec<OperationGroup>, String> {
    let mut groups: Vec<OperationGroup> = Vec::new();
    for operation in operations {
        let path = resolve_cli_config_file_from_dir(config_dir, &operation.file);
        if let Some(group) = groups.iter_mut().find(|group| group.path == path) {
            if group.format != operation.format {
                return Err(format!(
                    "Provider 配置文件 `{}` 不能同时使用不同 format",
                    operation.file
                ));
            }
            group.operations.push(operation.clone());
        } else {
            groups.push(OperationGroup {
                path,
                format: operation.format,
                operations: vec![operation.clone()],
            });
        }
    }
    Ok(groups)
}

fn ensure_group(
    groups: &mut Vec<OperationGroup>,
    path: &Path,
    format: ConfigFormat,
) -> Result<(), String> {
    if let Some(group) = groups.iter().find(|group| group.path == path) {
        if group.format != format {
            return Err(format!(
                "配置文件 `{}` 不能同时使用不同 format",
                path.display()
            ));
        }
    } else {
        groups.push(OperationGroup {
            path: path.to_path_buf(),
            format,
            operations: Vec::new(),
        });
    }
    Ok(())
}

fn gateway_context(agent_id: &str, profile: &str, gateway_endpoint: &str) -> PatchContext {
    PatchContext {
        target_endpoint: gateway_endpoint.trim().trim_end_matches('/').to_string(),
        target_token: gateway_token_for_profile(profile)
            .unwrap_or_else(|| "ccg-gateway".to_string()),
        agent_id: agent_id.to_string(),
    }
}

fn provider_context(provider: &Provider) -> PatchContext {
    PatchContext {
        target_endpoint: provider.base_url.trim().trim_end_matches('/').to_string(),
        target_token: provider.api_key.trim().to_string(),
        agent_id: provider.cli_type.clone(),
    }
}

async fn resolved_agent(db: &SqlitePool, agent_id: &str) -> Result<AgentInfo, String> {
    agent::get_agent(db, agent_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("未知 Agent: {}", agent_id))
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileSettingsStatus {
    pub profile: String,
    pub filename: String,
    pub path: String,
    pub launch_command: String,
    pub exists: bool,
    pub uses_gateway: bool,
}

pub async fn profile_file(
    db: &SqlitePool,
    agent_id: &str,
    profile: &str,
) -> Result<Option<(String, PathBuf)>, String> {
    let resolved = resolved_agent(db, agent_id).await?;
    let context = profile_template_context(&resolved, profile)?;
    let file = if context.profile == DEFAULT_PROFILE {
        resolved
            .features
            .provider_config
            .operations
            .first()
            .map(|operation| operation.file.clone())
    } else {
        context.relative_path
    };
    let Some(file) = file else {
        return Ok(None);
    };
    let config_dir = get_cli_config_dir_path(db, agent_id).await;
    let path = resolve_cli_config_file_from_dir(&config_dir, &file);
    Ok(Some((file, path)))
}

fn shell_quote_command_arg(value: &str) -> String {
    #[cfg(windows)]
    {
        if value.is_empty()
            || !value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_@%+=:,./\\-".contains(c))
        {
            return format!("\"{}\"", value.replace('"', "\\\""));
        }
        value.to_string()
    }

    #[cfg(not(windows))]
    {
        if value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c))
        {
            value.to_string()
        } else {
            format!("'{}'", value.replace('\'', "'\\''"))
        }
    }
}

pub async fn profile_launch_command(
    db: &SqlitePool,
    agent_id: &str,
    profile: &str,
) -> Result<String, String> {
    let resolved = resolved_agent(db, agent_id).await?;
    let profile = validate_provider_profile(Some(profile))?;
    let mut context = profile_template_context(&resolved, &profile)?;
    if let Some(file) = context.relative_path.as_deref() {
        let config_dir = get_cli_config_dir_path(db, agent_id).await;
        let absolute_path = resolve_cli_config_file_from_dir(&config_dir, file);
        context.absolute_path = Some(absolute_path.to_string_lossy().to_string());
    }
    let Some(launch) = resolved.features.profiles.launch.as_ref() else {
        return Ok(String::new());
    };
    let args = if profile == DEFAULT_PROFILE {
        &launch.default
    } else {
        &launch.non_default
    };
    args.iter()
        .map(|arg| {
            replace_profile_placeholders(arg, &context).map(|value| shell_quote_command_arg(&value))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|args| args.join(" "))
}

pub async fn profile_settings_status(
    db: &SqlitePool,
    agent_id: &str,
    gateway_url: &str,
    profile: &str,
) -> Result<ProfileSettingsStatus, String> {
    let profile = validate_provider_profile(Some(profile))?;
    let Some((filename, path)) = profile_file(db, agent_id, &profile).await? else {
        return Err(format!("Agent {} 未声明 Profile 配置文件", agent_id));
    };
    let exists = tokio::fs::try_exists(&path).await.unwrap_or(false);
    let launch_command = profile_launch_command(db, agent_id, &profile).await?;
    let uses_gateway =
        exists && is_provider_config_applied(db, agent_id, gateway_url, &profile).await;
    Ok(ProfileSettingsStatus {
        profile,
        filename,
        path: shrink_home_path(&path.to_string_lossy()),
        launch_command,
        exists,
        uses_gateway,
    })
}

pub async fn ensure_profile_settings(
    db: &SqlitePool,
    agent_id: &str,
    gateway_url: &str,
    profile: &str,
) -> Result<ProfileSettingsStatus, String> {
    let profile = validate_provider_profile(Some(profile))?;
    if profile != DEFAULT_PROFILE {
        let write_mode = sqlx::query_as::<_, (String,)>(
            "SELECT config_write_mode FROM cli_settings WHERE cli_type = ?",
        )
        .bind(agent_id)
        .fetch_optional(db)
        .await
        .map_err(|error| error.to_string())?
        .map(|row| row.0)
        .unwrap_or_else(|| "merge".to_string());
        sync_proxy_route_config(
            db,
            agent_id,
            true,
            gateway_url,
            &profile,
            "",
            None,
            &write_mode,
        )
        .await?;
    }
    profile_settings_status(db, agent_id, gateway_url, &profile).await
}

fn preset_target(
    resolved: &AgentInfo,
    config_dir: &Path,
    profile: &str,
    current: &str,
    previous: Option<&str>,
) -> Result<Option<PresetTarget>, String> {
    let feature = &resolved.features.global_preset;
    if !feature.enabled || profile != DEFAULT_PROFILE {
        return Ok(None);
    }
    let template = profile_template_context(resolved, profile)?;
    let file = replace_profile_placeholders(
        feature
            .file
            .as_deref()
            .ok_or_else(|| "global_preset 缺少 file".to_string())?,
        &template,
    )?;
    let format = feature
        .format
        .ok_or_else(|| "global_preset 缺少 format".to_string())?;
    Ok(Some(PresetTarget {
        path: resolve_cli_config_file_from_dir(config_dir, &file),
        format,
        current: current.to_string(),
        previous: previous
            .map(str::trim)
            .filter(|value| !value.is_empty() && *value != current.trim())
            .map(str::to_string),
    }))
}

async fn read_optional(path: &Path) -> Result<Option<String>, String> {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("读取 {} 失败: {}", path.display(), error)),
    }
}

async fn apply_enabled_groups(
    groups: &[OperationGroup],
    context: &PatchContext,
    preset: Option<&PresetTarget>,
    write_mode: &str,
) -> Result<Vec<PathBuf>, String> {
    if !matches!(write_mode, "merge" | "overwrite") {
        return Err("config_write_mode 只能是 'overwrite' 或 'merge'".to_string());
    }

    let mut prepared = Vec::with_capacity(groups.len());
    for group in groups {
        let existing = read_optional(&group.path).await?.unwrap_or_default();
        let mut content = if write_mode == "overwrite" {
            String::new()
        } else {
            existing
        };
        if let Some(preset) = preset.filter(|preset| preset.path == group.path) {
            if write_mode == "merge" {
                if let Some(previous) = preset.previous.as_deref() {
                    let previous = config_patch::sanitize_preset(
                        group.format,
                        previous,
                        &group.operations,
                        context,
                    )?;
                    content =
                        config_patch::safely_remove_preset(group.format, &content, &previous)?;
                }
            }
            let current = config_patch::sanitize_preset(
                group.format,
                &preset.current,
                &group.operations,
                context,
            )?;
            content = config_patch::apply_preset(group.format, &content, &current)?;
        }
        if !group.operations.is_empty() {
            content =
                config_patch::patch_content(group.format, &content, &group.operations, context)?;
        }
        prepared.push((group.path.clone(), content));
    }

    let mut paths = Vec::with_capacity(prepared.len());
    for (path, content) in prepared {
        config_patch::write_atomic_text(&path, &content).await?;
        paths.push(path);
    }
    Ok(paths)
}

async fn safely_remove_groups(
    groups: &[OperationGroup],
    context: &PatchContext,
    preset: Option<&PresetTarget>,
) -> Result<Vec<PathBuf>, String> {
    let mut prepared = Vec::new();
    for group in groups {
        let Some(mut content) = read_optional(&group.path).await? else {
            continue;
        };
        if let Some(preset) = preset.filter(|preset| preset.path == group.path) {
            match config_patch::sanitize_preset(
                group.format,
                &preset.current,
                &group.operations,
                context,
            ) {
                Ok(current) => {
                    match config_patch::safely_remove_preset(group.format, &content, &current) {
                        Ok(next) => content = next,
                        Err(error) => tracing::warn!(
                            "无法解析 {} 的全局预设清理目标，保留原文件: {}",
                            group.path.display(),
                            error
                        ),
                    }
                }
                Err(error) => tracing::warn!(
                    "无法解析 {} 的全局预设内容，保留原文件: {}",
                    group.path.display(),
                    error
                ),
            }
        }
        if !group.operations.is_empty() {
            content = config_patch::safely_remove_operations(
                group.format,
                &content,
                &group.operations,
                context,
            )?;
        }
        prepared.push((group.path.clone(), content));
    }

    let mut paths = Vec::with_capacity(prepared.len());
    for (path, content) in prepared {
        config_patch::write_atomic_text(&path, &content).await?;
        paths.push(path);
    }
    Ok(paths)
}

async fn operation_groups_applied(groups: &[OperationGroup], context: &PatchContext) -> bool {
    for group in groups {
        let Ok(content) = tokio::fs::read_to_string(&group.path).await else {
            return false;
        };
        if !config_patch::operations_applied(group.format, &content, &group.operations, context)
            .unwrap_or(false)
        {
            return false;
        }
    }
    true
}

async fn cli_preset_settings(db: &SqlitePool, agent_id: &str) -> Result<(String, String), String> {
    Ok(sqlx::query_as::<_, (Option<String>, String)>(
        "SELECT default_json_config, config_write_mode FROM cli_settings WHERE cli_type = ?",
    )
    .bind(agent_id)
    .fetch_optional(db)
    .await
    .map_err(|error| error.to_string())?
    .map(|(preset, mode)| (preset.unwrap_or_default(), mode))
    .unwrap_or_else(|| (String::new(), "merge".to_string())))
}

pub async fn sync_global_preset(
    db: &SqlitePool,
    agent_id: &str,
    enabled: bool,
    current_preset: &str,
    previous_preset: Option<&str>,
    write_mode: &str,
) -> Result<Vec<PathBuf>, String> {
    let resolved = resolved_agent(db, agent_id).await?;
    if !resolved.features.global_preset.enabled {
        return if enabled && !current_preset.trim().is_empty() {
            Err(format!("Agent {} 未启用全局预设", resolved.name))
        } else {
            Ok(Vec::new())
        };
    }

    let config_dir = get_cli_config_dir_path(db, agent_id).await;
    let Some(preset) = preset_target(
        &resolved,
        &config_dir,
        DEFAULT_PROFILE,
        current_preset,
        previous_preset,
    )?
    else {
        return Ok(Vec::new());
    };

    if enabled
        && write_mode == "merge"
        && current_preset.trim().is_empty()
        && preset.previous.is_none()
    {
        return Ok(Vec::new());
    }
    if !enabled && current_preset.trim().is_empty() {
        return Ok(Vec::new());
    }

    let groups = vec![OperationGroup {
        path: preset.path.clone(),
        format: preset.format,
        operations: Vec::new(),
    }];
    let context = gateway_context(agent_id, DEFAULT_PROFILE, "");
    if enabled {
        apply_enabled_groups(&groups, &context, Some(&preset), write_mode).await
    } else {
        safely_remove_groups(&groups, &context, Some(&preset)).await
    }
}

pub async fn sync_proxy_route_config(
    db: &SqlitePool,
    agent_id: &str,
    enabled: bool,
    gateway_endpoint: &str,
    profile: &str,
    current_preset: &str,
    previous_preset: Option<&str>,
    write_mode: &str,
) -> Result<Vec<PathBuf>, String> {
    let resolved = resolved_agent(db, agent_id).await?;
    let feature = &resolved.features.provider_config;
    if enabled && !feature.enabled {
        return Err("Provider 配置功能未启用".to_string());
    }

    let operations = resolve_profile_operations(&resolved, profile)?;
    let config_dir = get_cli_config_dir_path(db, agent_id).await;
    let mut groups = group_operations(&config_dir, &operations)?;
    let preset = preset_target(
        &resolved,
        &config_dir,
        profile,
        current_preset,
        previous_preset,
    )?;
    if let Some(preset) = &preset {
        ensure_group(&mut groups, &preset.path, preset.format)?;
    }
    let context = gateway_context(agent_id, profile, gateway_endpoint);
    if enabled {
        apply_enabled_groups(&groups, &context, preset.as_ref(), write_mode).await
    } else {
        safely_remove_groups(&groups, &context, preset.as_ref()).await
    }
}

pub async fn is_provider_config_applied(
    db: &SqlitePool,
    agent_id: &str,
    gateway_endpoint: &str,
    profile: &str,
) -> bool {
    let Ok(resolved) = resolved_agent(db, agent_id).await else {
        return false;
    };
    let feature = &resolved.features.provider_config;
    if !feature.enabled {
        return false;
    }
    let Ok(operations) = resolve_profile_operations(&resolved, profile) else {
        return false;
    };
    let config_dir = get_cli_config_dir_path(db, agent_id).await;
    let Ok(groups) = group_operations(&config_dir, &operations) else {
        return false;
    };
    operation_groups_applied(
        &groups,
        &gateway_context(agent_id, profile, gateway_endpoint),
    )
    .await
}

async fn write_provider_direct_config_impl(
    db: &SqlitePool,
    provider: &Provider,
    previous_preset: Option<&str>,
    write_mode_override: Option<&str>,
) -> Result<Vec<PathBuf>, String> {
    if provider.base_url.trim().is_empty() || provider.api_key.trim().is_empty() {
        return Err(format!(
            "服务商 {} 的 Base URL 或 API Key 为空",
            provider.name
        ));
    }
    let resolved = resolved_agent(db, &provider.cli_type).await?;
    if !resolved
        .protocols
        .iter()
        .any(|protocol| protocol.as_str() == provider.protocol.trim())
    {
        return Err(format!(
            "Agent {} 未声明 Provider Protocol {}",
            resolved.name, provider.protocol
        ));
    }
    let feature = &resolved.features.provider_config;
    if !feature.enabled {
        return Err(format!("Agent {} 不支持服务商直连模式", resolved.name));
    }
    let operations = resolve_profile_operations(&resolved, &provider.profile)?;
    let config_dir = get_cli_config_dir_path(db, &provider.cli_type).await;
    let mut groups = group_operations(&config_dir, &operations)?;
    let (current_preset, stored_write_mode) = cli_preset_settings(db, &provider.cli_type).await?;
    let write_mode = write_mode_override.unwrap_or(&stored_write_mode);
    let preset = preset_target(
        &resolved,
        &config_dir,
        &provider.profile,
        &current_preset,
        previous_preset,
    )?;
    if let Some(preset) = &preset {
        ensure_group(&mut groups, &preset.path, preset.format)?;
    }
    apply_enabled_groups(
        &groups,
        &provider_context(provider),
        preset.as_ref(),
        write_mode,
    )
    .await
}

pub async fn write_provider_direct_config_with_previous(
    db: &SqlitePool,
    provider: &Provider,
    previous_preset: Option<&str>,
) -> Result<Vec<PathBuf>, String> {
    write_provider_direct_config_impl(db, provider, previous_preset, None).await
}

pub async fn write_provider_direct_config(
    db: &SqlitePool,
    provider: &Provider,
) -> Result<Vec<PathBuf>, String> {
    write_provider_direct_config_with_previous(db, provider, None).await
}

pub async fn write_provider_direct_config_for_profile_rename(
    db: &SqlitePool,
    provider: &Provider,
) -> Result<Vec<PathBuf>, String> {
    write_provider_direct_config_impl(db, provider, None, Some("merge")).await
}

pub async fn remove_provider_direct_config_for_provider(
    db: &SqlitePool,
    provider: &Provider,
) -> Result<Vec<PathBuf>, String> {
    let resolved = resolved_agent(db, &provider.cli_type).await?;
    let feature = &resolved.features.provider_config;
    if !feature.enabled {
        return Ok(Vec::new());
    }

    let operations = resolve_profile_operations(&resolved, &provider.profile)?;
    let config_dir = get_cli_config_dir_path(db, &provider.cli_type).await;
    let groups = group_operations(&config_dir, &operations)?;
    safely_remove_groups(&groups, &provider_context(provider), None).await
}

async fn providers_for_profile(
    db: &SqlitePool,
    agent_id: &str,
    profile: &str,
) -> Result<Vec<Provider>, String> {
    sqlx::query_as::<_, Provider>(
        "SELECT * FROM providers WHERE cli_type = ? AND profile = ? ORDER BY sort_order, id",
    )
    .bind(agent_id)
    .bind(profile)
    .fetch_all(db)
    .await
    .map_err(|error| error.to_string())
}

pub async fn provider_direct_active_provider_id(
    db: &SqlitePool,
    agent_id: &str,
    profile: &str,
) -> Result<Option<i64>, String> {
    let resolved = resolved_agent(db, agent_id).await?;
    let feature = &resolved.features.provider_config;
    if !feature.enabled {
        return Ok(None);
    }
    let operations = resolve_profile_operations(&resolved, profile)?;
    let config_dir = get_cli_config_dir_path(db, agent_id).await;
    let groups = group_operations(&config_dir, &operations)?;
    for provider in providers_for_profile(db, agent_id, profile).await? {
        if operation_groups_applied(&groups, &provider_context(&provider)).await {
            return Ok(Some(provider.id));
        }
    }
    Ok(None)
}

async fn any_target_value_applied(groups: &[OperationGroup], context: &PatchContext) -> bool {
    for group in groups {
        let Ok(content) = tokio::fs::read_to_string(&group.path).await else {
            continue;
        };
        for operation in &group.operations {
            if !operation
                .value
                .as_ref()
                .is_some_and(|value| value.to_string().contains("{target."))
            {
                continue;
            }
            if config_patch::operations_applied(
                group.format,
                &content,
                std::slice::from_ref(operation),
                context,
            )
            .unwrap_or(false)
            {
                return true;
            }
        }
    }
    false
}

async fn remembered_default_provider(
    db: &SqlitePool,
    agent_id: &str,
) -> Result<Option<Provider>, String> {
    sqlx::query_as::<_, Provider>(
        r#"
        SELECT p.*
        FROM providers p
        JOIN cli_settings c ON c.cli_type = p.cli_type
        WHERE p.id = c.last_provider_direct_provider_id
          AND p.cli_type = ?
          AND p.profile = ?
        "#,
    )
    .bind(agent_id)
    .bind(DEFAULT_PROFILE)
    .fetch_optional(db)
    .await
    .map_err(|error| error.to_string())
}

pub async fn remove_provider_direct_config(
    db: &SqlitePool,
    agent_id: &str,
) -> Result<Vec<PathBuf>, String> {
    let resolved = resolved_agent(db, agent_id).await?;
    let feature = &resolved.features.provider_config;
    if !feature.enabled {
        return Ok(Vec::new());
    }

    let mut profiles: Vec<String> = sqlx::query_as::<_, (String,)>(
        "SELECT DISTINCT profile FROM providers WHERE cli_type = ? ORDER BY profile",
    )
    .bind(agent_id)
    .fetch_all(db)
    .await
    .map_err(|error| error.to_string())?
    .into_iter()
    .map(|(profile,)| profile)
    .collect();
    if !resolved.features.profiles.enabled {
        profiles.retain(|profile| profile == DEFAULT_PROFILE);
    }

    let config_dir = get_cli_config_dir_path(db, agent_id).await;
    let mut touched = HashSet::new();
    for profile in profiles {
        let operations = resolve_profile_operations(&resolved, &profile)?;
        let groups = group_operations(&config_dir, &operations)?;
        let providers = providers_for_profile(db, agent_id, &profile).await?;
        let mut selected = None;
        for provider in providers {
            let context = provider_context(&provider);
            if operation_groups_applied(&groups, &context).await
                || any_target_value_applied(&groups, &context).await
            {
                selected = Some(provider);
                break;
            }
        }
        if selected.is_none() && profile == DEFAULT_PROFILE {
            selected = remembered_default_provider(db, agent_id).await?;
        }
        let Some(provider) = selected else {
            continue;
        };
        for path in safely_remove_groups(&groups, &provider_context(&provider), None).await? {
            touched.insert(path);
        }
    }
    let mut paths: Vec<_> = touched.into_iter().collect();
    paths.sort();
    Ok(paths)
}
