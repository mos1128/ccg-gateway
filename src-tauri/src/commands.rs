use crate::config::{
    expand_home_path, get_data_dir, get_default_cli_config_dir, shrink_home_path, Config,
};
use crate::db::models::{
    AdvancedStatsRow, CliSettingsResponse, CliSettingsUpdate, DiscoverableSkill, GatewaySettings,
    InstalledSkillResponse, MarketplaceInfo, McpCliFlag, McpConfig, McpCreate, McpResponse,
    McpUpdate, OfficialCredential, OfficialCredentialCreate, OfficialCredentialResponse,
    OfficialCredentialUpdate, PaginatedLogs, PaginatedProjects, PaginatedSessions,
    PluginFavoriteItem, PluginItem, ProjectInfo, PromptCliFlag, PromptCreate, PromptPreset,
    PromptResponse, PromptUpdate, ProviderStatsResponse, ProviderStatsRow, RequestLogDetail,
    RequestLogItem, ScheduledTaskCreate, ScheduledTaskResponse, ScheduledTaskRun,
    ScheduledTaskRunItem, ScheduledTaskRunListResponse, ScheduledTaskUpdate, SessionInfo,
    SessionMessage, SkillCliFlag, SkillFavorite, SkillFavoriteItem, SkillRepo, SkillRepoCreate,
    SystemLogItem, SystemLogListResponse, SystemStatus, TimeoutSettings, TimeoutSettingsUpdate,
    WebdavBackup, WebdavSettings, WebdavSettingsUpdate,
};
use crate::services::proxy::CliType;
use crate::services::routing::{
    gateway_token_for_profile, normalize_profile, DEFAULT_PROFILE, PROFILE1, PROFILE2, PROFILE3,
    PROVIDER_PROFILES,
};
use crate::services::skill::{self, is_local_repo_source, InstalledSkillManifestEntry};
use crate::time::{local_compact_datetime, now_timestamp};
use crate::{LogDb, StatsDb};
use serde::Serialize;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use tauri::{Manager, State};

pub mod backup_commands;
pub mod cli_helpers;
pub mod credential_commands;
pub mod log_commands;
pub mod mcp_commands;
pub mod plugin_commands;
pub mod prompt_commands;
pub mod provider_commands;
pub mod scheduled_task_commands;
pub mod session_commands;
pub mod settings_commands;
pub mod skill_commands;
pub mod stats_commands;
pub mod system_commands;
pub mod update_commands;

type Result<T> = std::result::Result<T, String>;

fn serialize_toml_document<T: Serialize>(
    value: &T,
    context: &str,
) -> Result<toml_edit::DocumentMut> {
    toml_edit::ser::to_document(value)
        .map_err(|e| format!("Failed to serialize {}: {}", context, e))
}

fn serialize_toml_table<T: Serialize>(value: &T, context: &str) -> Result<toml_edit::Table> {
    Ok(serialize_toml_document(value, context)?.as_table().clone())
}

fn codex_gateway_provider_name(profile: &str) -> Option<&'static str> {
    match normalize_profile(Some(profile))? {
        DEFAULT_PROFILE => Some("ccg-gateway"),
        PROFILE1 => Some("ccg-gateway-profile1"),
        PROFILE2 => Some("ccg-gateway-profile2"),
        PROFILE3 => Some("ccg-gateway-profile3"),
        _ => None,
    }
}

fn ensure_toml_table<'a>(
    doc: &'a mut toml_edit::DocumentMut,
    key: &str,
) -> &'a mut toml_edit::Table {
    if doc.get(key).and_then(|item| item.as_table()).is_none() {
        let table = doc
            .remove(key)
            .and_then(|item| item.into_table().ok())
            .unwrap_or_else(toml_edit::Table::new);
        doc[key] = toml_edit::Item::Table(table);
    }

    doc.get_mut(key)
        .and_then(|item| item.as_table_mut())
        .expect("table should exist after normalization")
}

fn ensure_child_toml_table<'a>(
    table: &'a mut toml_edit::Table,
    key: &str,
) -> &'a mut toml_edit::Table {
    if table.get(key).and_then(|item| item.as_table()).is_none() {
        let child = table
            .remove(key)
            .and_then(|item| item.into_table().ok())
            .unwrap_or_else(toml_edit::Table::new);
        table[key] = toml_edit::Item::Table(child);
    }

    table
        .get_mut(key)
        .and_then(|item| item.as_table_mut())
        .expect("child table should exist after normalization")
}

fn apply_codex_gateway_provider_config(
    doc: &mut toml_edit::DocumentMut,
    gateway_url: &str,
    profile: &str,
) -> Result<&'static str> {
    let profile = normalize_profile(Some(profile))
        .ok_or_else(|| format!("profile 只能是 {}", PROVIDER_PROFILES.join(" / ")))?;
    let provider_name = codex_gateway_provider_name(profile)
        .ok_or_else(|| format!("profile 只能是 {}", PROVIDER_PROFILES.join(" / ")))?;
    let gateway_token = gateway_token_for_profile(profile)
        .ok_or_else(|| format!("profile 只能是 {}", PROVIDER_PROFILES.join(" / ")))?;

    let mut provider_table = toml_edit::Table::new();
    provider_table["name"] = toml_edit::value(provider_name);
    provider_table["base_url"] = toml_edit::value(gateway_url.trim_end_matches('/'));
    provider_table["wire_api"] = toml_edit::value("responses");
    provider_table["requires_openai_auth"] = toml_edit::value(false);
    provider_table["experimental_bearer_token"] = toml_edit::value(gateway_token);

    let model_providers = ensure_toml_table(doc, "model_providers");
    model_providers[provider_name] = toml_edit::Item::Table(provider_table);

    Ok(provider_name)
}

fn apply_codex_gateway_default_config(
    doc: &mut toml_edit::DocumentMut,
    gateway_url: &str,
) -> Result<()> {
    let provider_name = apply_codex_gateway_provider_config(doc, gateway_url, DEFAULT_PROFILE)?;
    doc["model_provider"] = toml_edit::value(provider_name);

    Ok(())
}

fn apply_codex_gateway_named_profile_config(
    doc: &mut toml_edit::DocumentMut,
    gateway_url: &str,
    profile: &str,
) -> Result<()> {
    let profile = normalize_profile(Some(profile))
        .ok_or_else(|| format!("profile 只能是 {}", PROVIDER_PROFILES.join(" / ")))?;
    if profile == DEFAULT_PROFILE {
        return Ok(());
    }

    let provider_name = apply_codex_gateway_provider_config(doc, gateway_url, profile)?;
    let profiles = ensure_toml_table(doc, "profiles");
    let profile_table = ensure_child_toml_table(profiles, profile);
    profile_table["model_provider"] = toml_edit::value(provider_name);

    Ok(())
}

fn remove_codex_default_gateway_entry(doc: &mut toml_edit::DocumentMut) {
    let Some(default_provider_name) = codex_gateway_provider_name(DEFAULT_PROFILE) else {
        return;
    };

    if doc
        .get("model_provider")
        .and_then(|item| item.as_str())
        .map(|provider| provider == default_provider_name)
        .unwrap_or(false)
    {
        doc.remove("model_provider");
    }

    if doc.get("model_providers").is_some() {
        let model_providers = ensure_toml_table(doc, "model_providers");
        model_providers.remove(default_provider_name);
        if model_providers.is_empty() {
            doc.remove("model_providers");
        }
    }
}

async fn remove_file_if_exists(path: &std::path::Path, label: &str) -> Result<()> {
    if tokio::fs::try_exists(path).await.unwrap_or(false) {
        tracing::info!("删除直连模式文件: {:?}", path);
        tokio::fs::remove_file(path).await.map_err(|e| {
            tracing::error!("删除 {} 失败: {}", label, e);
            e.to_string()
        })?;
    }
    Ok(())
}

async fn remove_codex_direct_mode_files(
    config_dir: &std::path::Path,
    use_merge: bool,
) -> Result<()> {
    let auth_path = config_dir.join("auth.json");
    let config_path = config_dir.join("config.toml");

    remove_file_if_exists(&auth_path, "auth.json").await?;

    if use_merge {
        tracing::info!("Codex 增量模式切换到中转时保留 config.toml，供后续合并");
    } else {
        remove_file_if_exists(&config_path, "config.toml").await?;
    }

    Ok(())
}

async fn remove_gemini_direct_mode_files(
    config_dir: &std::path::Path,
    use_merge: bool,
) -> Result<()> {
    let oauth_path = config_dir.join("oauth_creds.json");
    let accounts_path = config_dir.join("google_accounts.json");
    let settings_path = config_dir.join("settings.json");

    remove_file_if_exists(&oauth_path, "oauth_creds.json").await?;
    remove_file_if_exists(&accounts_path, "google_accounts.json").await?;

    if use_merge {
        tracing::info!("Gemini 增量模式切换到中转时保留 settings.json，供后续合并");
    } else {
        remove_file_if_exists(&settings_path, "settings.json").await?;
    }

    Ok(())
}

fn parse_codex_mcp_toml_table(mcp_config_json: &str) -> Result<toml_edit::Table> {
    let value = serde_json::from_str::<serde_json::Value>(mcp_config_json)
        .map_err(|e| format!("Codex MCP JSON 格式错误: {}", e))?;

    if !value.is_object() {
        return Err("Codex MCP 配置必须是 JSON object".to_string());
    }

    validate_toml_compatible_json(&value)?;
    serialize_toml_table(&value, "Codex MCP config")
}

fn validate_toml_compatible_json(value: &serde_json::Value) -> Result<()> {
    match value {
        serde_json::Value::Null => Err("Codex MCP 配置不能包含 null，TOML 不支持 null".to_string()),
        serde_json::Value::Array(items) => {
            for item in items {
                validate_toml_compatible_json(item)?;
            }
            Ok(())
        }
        serde_json::Value::Object(map) => {
            for value in map.values() {
                validate_toml_compatible_json(value)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn map_db_error(e: sqlx::Error) -> String {
    let err_str = e.to_string();
    if err_str.contains("code: 2067") || err_str.contains("UNIQUE constraint failed") {
        if err_str.contains("providers.cli_type") && err_str.contains("providers.name") {
            return "同类型同 Profile 的服务商名称已存在".to_string();
        }
        if err_str.contains("provider_model_map.provider_id")
            && err_str.contains("provider_model_map.source_model")
        {
            return "该服务商已存在相同的模型映射".to_string();
        }
        if err_str.contains("provider_model_blacklist.provider_id")
            && err_str.contains("provider_model_blacklist.model_pattern")
        {
            return "该服务商已存在相同的黑名单模式".to_string();
        }
        if err_str.contains("mcp_configs.name") {
            return "MCP 配置名称已存在".to_string();
        }
        if err_str.contains("prompt_presets.name") {
            return "提示词预设名称已存在".to_string();
        }
        if err_str.contains("skill_configs.directory") {
            return "该目录已安装过 Skill".to_string();
        }
        if err_str.contains("official_credentials.cli_type")
            && err_str.contains("official_credentials.name")
        {
            return "同类型的凭证名称已存在".to_string();
        }
        if err_str.contains("plugin_favorites.plugin_id") {
            return "该插件已收藏".to_string();
        }
        if err_str.contains("skill_favorites.skill_key") {
            return "该技能已收藏".to_string();
        }
        return "数据已存在，请勿重复添加".to_string();
    }
    err_str
}

fn validate_provider_profile(profile: Option<&str>) -> Result<&'static str> {
    normalize_profile(profile)
        .ok_or_else(|| format!("profile 只能是 {}", PROVIDER_PROFILES.join(" / ")))
}

#[derive(Debug, Serialize)]
pub struct ClaudeProfileSettingsStatus {
    pub profile: String,
    pub filename: String,
    pub path: String,
    pub launch_command: String,
    pub exists: bool,
    pub uses_gateway: bool,
}

#[derive(Debug, Serialize)]
pub struct CodexProfileSettingsStatus {
    pub profile: String,
    pub filename: String,
    pub path: String,
    pub launch_command: String,
    pub exists: bool,
    pub uses_gateway: bool,
}

// Normalize text for comparison: trim, normalize whitespace, remove extra blank lines
fn normalize_text(text: &str) -> String {
    text.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<&str>>()
        .join("\n")
}

// Check if MCP config exists in the CLI config file - 异步版本，支持自定义配置目录
async fn mcp_enabled_in_file_async(db: &SqlitePool, cli_type: &str, mcp_name: &str) -> bool {
    let config_dir = get_cli_config_dir_path(db, cli_type).await;

    let config_path = cli_helpers::mcp_config_path(&config_dir, cli_type);

    let path = match config_path {
        Some(p) => p,
        None => return false,
    };

    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return false;
    }

    let content = match tokio::fs::read_to_string(&path).await {
        Ok(c) => c,
        Err(_) => return false,
    };

    match cli_type {
        "claude_code" | "gemini" => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(config) => config
                .get("mcpServers")
                .and_then(|v| v.as_object())
                .map(|servers| servers.contains_key(mcp_name))
                .unwrap_or(false),
            Err(_) => false,
        },
        "codex" => match content.parse::<toml_edit::DocumentMut>() {
            Ok(doc) => doc
                .get("mcp_servers")
                .and_then(|v| v.as_table())
                .map(|servers| servers.contains_key(mcp_name))
                .unwrap_or(false),
            Err(_) => false,
        },
        _ => false,
    }
}

// Check if prompt content matches the file content - 异步版本，支持自定义配置目录
async fn prompt_enabled_in_file_async(
    db: &SqlitePool,
    cli_type: &str,
    prompt_content: &str,
) -> bool {
    let config_dir = get_cli_config_dir_path(db, cli_type).await;

    let prompt_path = match cli_helpers::prompt_file_path(&config_dir, cli_type) {
        Some(path) => path,
        None => return false,
    };

    if !tokio::fs::try_exists(&prompt_path).await.unwrap_or(false) {
        return false;
    }

    let file_content = match tokio::fs::read_to_string(&prompt_path).await {
        Ok(c) => c,
        Err(_) => return false,
    };

    // Normalize and compare
    normalize_text(prompt_content) == normalize_text(&file_content)
}

// ============================================================================
// CLI 配置目录获取（统一入口）
// ============================================================================

/// 获取 CLI 配置目录
/// 优先级：数据库配置 > 默认路径
pub async fn get_cli_config_dir_path(db: &SqlitePool, cli_type: &str) -> std::path::PathBuf {
    // 1. 查询数据库
    let result: Option<(Option<String>,)> =
        sqlx::query_as("SELECT config_dir FROM cli_settings WHERE cli_type = ?")
            .bind(cli_type)
            .fetch_optional(db)
            .await
            .ok()
            .flatten();

    // 2. 有配置则展开路径，否则使用默认
    match result.and_then(|r| r.0) {
        Some(path) => std::path::PathBuf::from(expand_home_path(&path)),
        None => get_default_cli_config_dir(cli_type),
    }
}

// ============================================================================
// 内部辅助函数
// ============================================================================

async fn check_cli_enabled(db: &SqlitePool, cli_type: &str, gateway_url: &str) -> bool {
    match cli_type {
        "claude_code" => check_claude_uses_gateway(db, cli_type, gateway_url).await,
        "codex" => check_codex_uses_gateway(db, cli_type, gateway_url).await,
        "gemini" => check_gemini_uses_gateway(db, cli_type, gateway_url).await,
        _ => false,
    }
}

fn normalize_gateway_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_ascii_lowercase()
}

fn gateway_url_matches(value: &str, gateway_url: &str) -> bool {
    normalize_gateway_url(value) == normalize_gateway_url(gateway_url)
}

async fn check_claude_uses_gateway(db: &SqlitePool, cli_type: &str, gateway_url: &str) -> bool {
    let config_dir = get_cli_config_dir_path(db, cli_type).await;
    let config_path = config_dir.join("settings.json");

    if !tokio::fs::try_exists(&config_path).await.unwrap_or(false) {
        return false;
    }

    let content = match tokio::fs::read_to_string(&config_path).await {
        Ok(c) => c,
        Err(_) => return false,
    };

    let content_trimmed = content.trim();
    if content_trimmed.is_empty() || content_trimmed == "{}" {
        return false;
    }

    match serde_json::from_str::<serde_json::Value>(content_trimmed) {
        Ok(data) => {
            if let Some(env) = data.get("env") {
                if let Some(base_url) = env.get("ANTHROPIC_BASE_URL").and_then(|v| v.as_str()) {
                    return gateway_url_matches(base_url, gateway_url);
                }
            }
            false
        }
        Err(_) => false,
    }
}

async fn check_codex_uses_gateway(db: &SqlitePool, cli_type: &str, gateway_url: &str) -> bool {
    let config_dir = get_cli_config_dir_path(db, cli_type).await;
    let config_path = config_dir.join("config.toml");

    if !tokio::fs::try_exists(&config_path).await.unwrap_or(false) {
        return false;
    }

    let content = match tokio::fs::read_to_string(&config_path).await {
        Ok(c) => c,
        Err(_) => return false,
    };

    if content.trim().is_empty() {
        return false;
    }

    match content.parse::<toml_edit::DocumentMut>() {
        Ok(doc) => codex_default_provider_uses_gateway(&doc, gateway_url),
        Err(_) => false,
    }
}

async fn check_gemini_uses_gateway(db: &SqlitePool, cli_type: &str, gateway_url: &str) -> bool {
    let config_dir = get_cli_config_dir_path(db, cli_type).await;
    let env_path = config_dir.join(".env");

    if !tokio::fs::try_exists(&env_path).await.unwrap_or(false) {
        return false;
    }

    let content = match tokio::fs::read_to_string(&env_path).await {
        Ok(c) => c,
        Err(_) => return false,
    };

    // Check if .env contains GOOGLE_GEMINI_BASE_URL pointing to gateway
    for line in content.lines() {
        if let Some(url) = line.trim().strip_prefix("GOOGLE_GEMINI_BASE_URL=") {
            return gateway_url_matches(url, gateway_url);
        }
    }
    false
}

// Get the config file path for MCP/prompts sync (different for Codex)
async fn get_mcp_config_path(db: &SqlitePool, cli_type: &str) -> Option<std::path::PathBuf> {
    let base_path = get_cli_config_dir_path(db, cli_type).await;
    cli_helpers::mcp_config_path(&base_path, cli_type)
}

async fn get_config_write_mode(db: &SqlitePool, cli_type: &str) -> String {
    sqlx::query_as::<_, (String,)>("SELECT config_write_mode FROM cli_settings WHERE cli_type = ?")
        .bind(cli_type)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .map(|r| r.0)
        .unwrap_or_else(|| "merge".to_string())
}

async fn sync_cli_config(
    db: &SqlitePool,
    cli_type: &str,
    enabled: bool,
    default_config: &str,
    previous_default_config: Option<&str>,
    gateway_url: &str,
) -> Result<()> {
    let write_mode = get_config_write_mode(db, cli_type).await;
    match cli_type {
        "claude_code" => {
            sync_claude_code_config(
                db,
                enabled,
                default_config,
                previous_default_config,
                &write_mode,
                gateway_url,
                ClaudeProfileSyncScope::DefaultOnly,
            )
            .await
        }
        "codex" => {
            sync_codex_config(
                db,
                enabled,
                default_config,
                previous_default_config,
                &write_mode,
                gateway_url,
            )
            .await
        }
        "gemini" => {
            sync_gemini_config(
                db,
                enabled,
                default_config,
                previous_default_config,
                &write_mode,
                gateway_url,
            )
            .await
        }
        _ => Err("Invalid CLI type".to_string()),
    }
}

#[derive(Clone, Copy)]
enum ClaudeProfileSyncScope {
    DefaultOnly,
}

fn claude_profile_in_scope(scope: ClaudeProfileSyncScope, profile: &str) -> bool {
    match scope {
        ClaudeProfileSyncScope::DefaultOnly => profile == DEFAULT_PROFILE,
    }
}

fn claude_gateway_json_template() -> serde_json::Value {
    serde_json::json!({
        "env": {
            "ANTHROPIC_BASE_URL": "",
            "ANTHROPIC_AUTH_TOKEN": ""
        }
    })
}

fn gemini_gateway_json_template() -> serde_json::Value {
    serde_json::json!({
        "security": {
            "auth": {
                "selectedType": ""
            }
        }
    })
}

fn sanitize_json_config(
    config: serde_json::Value,
    protected_template: &serde_json::Value,
) -> serde_json::Value {
    let mut sanitized = config;
    deep_remove(&mut sanitized, protected_template);
    sanitized
}

async fn remove_json_config_content(
    config_path: &std::path::Path,
    gateway_template: &serde_json::Value,
    default_config: &str,
    protected_template: &serde_json::Value,
) -> Result<()> {
    if !tokio::fs::try_exists(config_path).await.unwrap_or(false) {
        return Ok(());
    }

    let content = tokio::fs::read_to_string(config_path).await.map_err(|e| {
        tracing::error!("Failed to read {}: {}", config_path.display(), e);
        e.to_string()
    })?;

    let mut config = match serde_json::from_str::<serde_json::Value>(&content) {
        Ok(config) => config,
        Err(e) => {
            tracing::warn!(
                "Failed to parse JSON config {}, leaving file untouched: {}",
                config_path.display(),
                e
            );
            return Ok(());
        }
    };

    deep_remove(&mut config, gateway_template);

    if !default_config.is_empty() {
        if let Ok(preset) = serde_json::from_str::<serde_json::Value>(default_config) {
            let sanitized_preset = sanitize_json_config(preset, protected_template);
            deep_remove(&mut config, &sanitized_preset);
        }
    }

    let config_str = serde_json::to_string_pretty(&config).map_err(|e| {
        tracing::error!(
            "Failed to serialize config {}: {}",
            config_path.display(),
            e
        );
        e.to_string()
    })?;
    tokio::fs::write(config_path, config_str)
        .await
        .map_err(|e| {
            tracing::error!("Failed to write {}: {}", config_path.display(), e);
            e.to_string()
        })?;
    Ok(())
}

async fn remove_codex_gateway_auth_content(auth_path: &std::path::Path) -> Result<()> {
    if !tokio::fs::try_exists(auth_path).await.unwrap_or(false) {
        return Ok(());
    }

    let content = tokio::fs::read_to_string(auth_path).await.map_err(|e| {
        tracing::error!("Failed to read auth.json: {}", e);
        e.to_string()
    })?;

    let mut auth = match serde_json::from_str::<serde_json::Value>(&content) {
        Ok(auth) => auth,
        Err(e) => {
            tracing::warn!("Failed to parse auth.json, leaving file untouched: {}", e);
            return Ok(());
        }
    };

    if let Some(object) = auth.as_object_mut() {
        let should_remove = object
            .get("OPENAI_API_KEY")
            .and_then(|value| value.as_str())
            .map(|value| value == "ccg-gateway")
            .unwrap_or(false);
        if should_remove {
            object.remove("OPENAI_API_KEY");
        }
    }

    let auth_str = serde_json::to_string_pretty(&auth).map_err(|e| {
        tracing::error!("Failed to serialize auth.json: {}", e);
        e.to_string()
    })?;
    tokio::fs::write(auth_path, auth_str).await.map_err(|e| {
        tracing::error!("Failed to write auth.json: {}", e);
        e.to_string()
    })?;
    Ok(())
}

async fn write_codex_gateway_auth(auth_path: &std::path::Path) -> Result<()> {
    let auth = serde_json::json!({
        "OPENAI_API_KEY": "ccg-gateway"
    });
    let auth_str = serde_json::to_string_pretty(&auth).map_err(|e| {
        tracing::error!("Failed to serialize auth.json: {}", e);
        e.to_string()
    })?;
    tokio::fs::write(auth_path, auth_str).await.map_err(|e| {
        tracing::error!("Failed to write auth.json: {}", e);
        e.to_string()
    })
}

async fn remove_gemini_gateway_env_content(
    env_path: &std::path::Path,
    gateway_url: &str,
) -> Result<()> {
    if !tokio::fs::try_exists(env_path).await.unwrap_or(false) {
        return Ok(());
    }

    let content = tokio::fs::read_to_string(env_path).await.map_err(|e| {
        tracing::error!("Failed to read .env file: {}", e);
        e.to_string()
    })?;

    let filtered_lines = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed == "GEMINI_API_KEY=ccg-gateway" {
                return false;
            }
            if let Some(url) = trimmed.strip_prefix("GOOGLE_GEMINI_BASE_URL=") {
                if gateway_url_matches(url, gateway_url) {
                    return false;
                }
            }
            true
        })
        .collect::<Vec<_>>();

    let new_content = if filtered_lines.is_empty() {
        String::new()
    } else {
        filtered_lines.join("\n") + "\n"
    };

    tokio::fs::write(env_path, new_content).await.map_err(|e| {
        tracing::error!("Failed to write .env file: {}", e);
        e.to_string()
    })?;
    Ok(())
}

async fn remove_codex_gateway_config_content(
    config_path: &std::path::Path,
    default_config: &str,
) -> Result<()> {
    if !tokio::fs::try_exists(config_path).await.unwrap_or(false) {
        return Ok(());
    }

    let content = tokio::fs::read_to_string(config_path).await.map_err(|e| {
        tracing::error!("Failed to read config.toml: {}", e);
        e.to_string()
    })?;

    let mut doc = match content.parse::<toml_edit::DocumentMut>() {
        Ok(doc) => doc,
        Err(e) => {
            tracing::warn!("Failed to parse config.toml, leaving file untouched: {}", e);
            return Ok(());
        }
    };

    remove_codex_default_gateway_entry(&mut doc);

    if !default_config.is_empty() {
        if let Ok(mut preset_doc) = default_config.parse::<toml_edit::DocumentMut>() {
            remove_codex_default_gateway_entry(&mut preset_doc);
            for (key, _) in preset_doc.iter() {
                doc.remove(key);
            }
        }
    }

    tokio::fs::write(config_path, doc.to_string())
        .await
        .map_err(|e| {
            tracing::error!("Failed to write config.toml: {}", e);
            e.to_string()
        })?;
    Ok(())
}

fn deep_merge(base: &mut serde_json::Value, override_val: &serde_json::Value) {
    if let (Some(base_obj), Some(override_obj)) = (base.as_object_mut(), override_val.as_object()) {
        for (key, value) in override_obj {
            if let Some(base_value) = base_obj.get_mut(key) {
                if base_value.is_object() && value.is_object() {
                    deep_merge(base_value, value);
                } else {
                    *base_value = value.clone();
                }
            } else {
                base_obj.insert(key.clone(), value.clone());
            }
        }
    }
}

/// 从 base 中移除 template 中出现的所有叶子节点 key，自底向上清理空的中间对象
fn deep_remove(base: &mut serde_json::Value, template: &serde_json::Value) {
    if let (Some(base_obj), Some(tmpl_obj)) = (base.as_object_mut(), template.as_object()) {
        for (key, tmpl_val) in tmpl_obj {
            if tmpl_val.is_object() {
                // 递归移除子节点
                if let Some(base_val) = base_obj.get_mut(key) {
                    deep_remove(base_val, tmpl_val);
                    // 如果子对象变空了，移除这个键
                    if base_val.as_object().map(|o| o.is_empty()).unwrap_or(false) {
                        base_obj.remove(key);
                    }
                }
            } else {
                // 叶子节点：直接移除
                base_obj.remove(key);
            }
        }
    }
}

fn previous_config_to_remove<'a>(
    previous_default_config: Option<&'a str>,
    default_config: &str,
) -> Option<&'a str> {
    previous_default_config
        .map(str::trim)
        .filter(|config| !config.is_empty() && *config != default_config.trim())
}

fn remove_previous_json_preset(
    config: &mut serde_json::Value,
    previous_default_config: Option<&str>,
    default_config: &str,
    protected_template: &serde_json::Value,
    label: &str,
) {
    if let Some(previous_default_config) =
        previous_config_to_remove(previous_default_config, default_config)
    {
        match serde_json::from_str::<serde_json::Value>(previous_default_config) {
            Ok(previous_config) => {
                let sanitized_config = sanitize_json_config(previous_config, protected_template);
                deep_remove(config, &sanitized_config);
            }
            Err(e) => {
                tracing::warn!("Failed to parse previous {} preset: {}", label, e);
            }
        }
    }
}

fn remove_previous_codex_preset(
    doc: &mut toml_edit::DocumentMut,
    previous_default_config: Option<&str>,
    default_config: &str,
    strip_gateway_fields: bool,
) {
    if let Some(previous_default_config) =
        previous_config_to_remove(previous_default_config, default_config)
    {
        match previous_default_config.parse::<toml_edit::DocumentMut>() {
            Ok(mut preset_doc) => {
                if strip_gateway_fields {
                    preset_doc.remove("model_provider");
                    preset_doc.remove("model_providers");
                }
                for (key, _) in preset_doc.iter() {
                    doc.remove(key);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to parse previous Codex preset: {}", e);
            }
        }
    }
}

#[cfg(windows)]
fn shell_quote_path(path: &str) -> String {
    if path.chars().any(char::is_whitespace) {
        format!("\"{}\"", path)
    } else {
        path.to_string()
    }
}

#[cfg(not(windows))]
fn shell_quote_path(path: &str) -> String {
    if path
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c))
    {
        path.to_string()
    } else {
        format!("'{}'", path.replace('\'', "'\\''"))
    }
}

fn claude_settings_launch_command(config_path: &std::path::Path) -> String {
    format!(
        "claude --settings {}",
        shell_quote_path(&config_path.to_string_lossy())
    )
}

fn claude_profile_launch_command(profile: &str, config_path: &std::path::Path) -> String {
    if profile == DEFAULT_PROFILE {
        "claude".to_string()
    } else {
        claude_settings_launch_command(config_path)
    }
}

fn codex_profile_launch_command(profile: &str) -> String {
    if profile == DEFAULT_PROFILE {
        "codex".to_string()
    } else {
        format!("codex --profile {}", profile)
    }
}

async fn claude_settings_uses_gateway(
    config_path: &std::path::Path,
    gateway_url: &str,
    gateway_token: &str,
) -> bool {
    if !tokio::fs::try_exists(config_path).await.unwrap_or(false) {
        return false;
    }

    let content = match tokio::fs::read_to_string(config_path).await {
        Ok(content) => content,
        Err(_) => return false,
    };

    let content_trimmed = content.trim();
    if content_trimmed.is_empty() || content_trimmed == "{}" {
        return false;
    }

    let data = match serde_json::from_str::<serde_json::Value>(content_trimmed) {
        Ok(data) => data,
        Err(_) => return false,
    };

    let env = match data.get("env") {
        Some(env) => env,
        None => return false,
    };

    let base_url = env
        .get("ANTHROPIC_BASE_URL")
        .and_then(|value| value.as_str());
    let auth_token = env
        .get("ANTHROPIC_AUTH_TOKEN")
        .and_then(|value| value.as_str());

    matches!(
        (base_url, auth_token),
        (Some(base_url), Some(auth_token))
            if gateway_url_matches(base_url, gateway_url) && auth_token == gateway_token
    )
}

async fn claude_profile_settings_status(
    db: &SqlitePool,
    profile: &str,
    gateway_url: &str,
) -> Result<ClaudeProfileSettingsStatus> {
    let profile = normalize_profile(Some(profile))
        .ok_or_else(|| format!("profile 只能是 {}", PROVIDER_PROFILES.join(" / ")))?;
    let filename = cli_helpers::claude_settings_filename(profile);
    let config_dir = get_cli_config_dir_path(db, "claude_code").await;
    let config_path = config_dir.join(filename);
    let gateway_token = gateway_token_for_profile(profile).unwrap_or("ccg-gateway");
    let path = shrink_home_path(&config_path.to_string_lossy());
    let launch_command = claude_profile_launch_command(profile, &config_path);
    let exists = tokio::fs::try_exists(&config_path).await.unwrap_or(false);
    let uses_gateway = if profile == DEFAULT_PROFILE {
        true
    } else {
        claude_settings_uses_gateway(&config_path, gateway_url, gateway_token).await
    };

    Ok(ClaudeProfileSettingsStatus {
        profile: profile.to_string(),
        filename: filename.to_string(),
        path,
        launch_command,
        exists,
        uses_gateway,
    })
}

async fn codex_profile_uses_gateway(
    config_path: &std::path::Path,
    profile: &str,
    gateway_url: &str,
) -> bool {
    if profile == DEFAULT_PROFILE {
        return true;
    }

    if !tokio::fs::try_exists(config_path).await.unwrap_or(false) {
        return false;
    }

    let content = match tokio::fs::read_to_string(config_path).await {
        Ok(content) => content,
        Err(_) => return false,
    };

    let doc = match content.parse::<toml_edit::DocumentMut>() {
        Ok(doc) => doc,
        Err(_) => return false,
    };

    let provider_name = doc
        .get("profiles")
        .and_then(|item| item.get(profile))
        .and_then(|item| item.get("model_provider"))
        .and_then(|item| item.as_str())
        .map(|value| value.to_string());

    let Some(provider_name) = provider_name else {
        return false;
    };

    let Some(expected_token) = gateway_token_for_profile(profile) else {
        return false;
    };
    let expected_url = gateway_url.trim_end_matches('/');

    doc.get("model_providers")
        .and_then(|item| item.get(&provider_name))
        .and_then(|item| item.as_table())
        .map(|provider| {
            let Some(base_url) = provider.get("base_url").and_then(|item| item.as_str()) else {
                return false;
            };

            let base_url_matches = gateway_url_matches(base_url, expected_url);
            let token = provider
                .get("experimental_bearer_token")
                .and_then(|item| item.as_str())
                .map(str::trim);
            let token_matches = token.map(|token| token == expected_token).unwrap_or(false);

            base_url_matches && token_matches
        })
        .unwrap_or(false)
}

fn codex_default_provider_uses_gateway(doc: &toml_edit::DocumentMut, gateway_url: &str) -> bool {
    if doc
        .get("model_provider")
        .and_then(|v| v.as_str())
        .map(|provider| provider != "ccg-gateway")
        .unwrap_or(true)
    {
        return false;
    }

    doc.get("model_providers")
        .and_then(|providers| providers.get("ccg-gateway"))
        .and_then(|provider| provider.as_table())
        .map(|provider| {
            let Some(base_url) = provider.get("base_url").and_then(|v| v.as_str()) else {
                return false;
            };

            gateway_url_matches(base_url, gateway_url)
        })
        .unwrap_or(false)
}

async fn codex_profile_settings_status(
    db: &SqlitePool,
    profile: &str,
    gateway_url: &str,
) -> Result<CodexProfileSettingsStatus> {
    let profile = normalize_profile(Some(profile))
        .ok_or_else(|| format!("profile 只能是 {}", PROVIDER_PROFILES.join(" / ")))?;
    let config_dir = get_cli_config_dir_path(db, "codex").await;
    let config_path = config_dir.join("config.toml");
    let path = shrink_home_path(&config_path.to_string_lossy());
    let exists = tokio::fs::try_exists(&config_path).await.unwrap_or(false);

    Ok(CodexProfileSettingsStatus {
        profile: profile.to_string(),
        filename: "config.toml".to_string(),
        path,
        launch_command: codex_profile_launch_command(profile),
        exists,
        uses_gateway: codex_profile_uses_gateway(&config_path, profile, gateway_url).await,
    })
}

fn claude_gateway_config(gateway_url: &str, gateway_token: &str) -> serde_json::Value {
    serde_json::json!({
        "env": {
            "ANTHROPIC_BASE_URL": gateway_url,
            "ANTHROPIC_AUTH_TOKEN": gateway_token
        }
    })
}

async fn write_claude_gateway_settings(
    config_path: &std::path::Path,
    default_config: &str,
    previous_default_config: Option<&str>,
    use_merge: bool,
    gateway_url: &str,
    gateway_token: &str,
) -> Result<()> {
    if let Some(parent) = config_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            tracing::error!("Failed to create directory: {}", e);
            e.to_string()
        })?;
    }

    let gateway_config = claude_gateway_config(gateway_url, gateway_token);
    let protected_gateway_fields = claude_gateway_json_template();

    let mut config = if use_merge {
        if tokio::fs::try_exists(config_path).await.unwrap_or(false) {
            tokio::fs::read_to_string(config_path)
                .await
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .unwrap_or_else(|| serde_json::json!({}))
        } else {
            serde_json::json!({})
        }
    } else {
        serde_json::json!({})
    };

    if use_merge {
        remove_previous_json_preset(
            &mut config,
            previous_default_config,
            default_config,
            &protected_gateway_fields,
            "Claude Code",
        );
    }

    deep_merge(&mut config, &gateway_config);

    if !default_config.is_empty() {
        match serde_json::from_str::<serde_json::Value>(default_config) {
            Ok(custom_config) => {
                let sanitized_config =
                    sanitize_json_config(custom_config, &protected_gateway_fields);
                deep_merge(&mut config, &sanitized_config);
            }
            Err(e) => {
                tracing::warn!("Failed to parse custom config (invalid JSON): {}", e);
            }
        }
    }

    let config_str = serde_json::to_string_pretty(&config).map_err(|e| {
        tracing::error!("Failed to serialize config: {}", e);
        e.to_string()
    })?;
    tokio::fs::write(config_path, config_str)
        .await
        .map_err(|e| {
            tracing::error!("Failed to write config file: {}", e);
            e.to_string()
        })?;

    Ok(())
}

// Sync Claude Code configuration (settings.json)
async fn sync_claude_code_config(
    db: &SqlitePool,
    enabled: bool,
    default_config: &str,
    previous_default_config: Option<&str>,
    write_mode: &str,
    gateway_url: &str,
    scope: ClaudeProfileSyncScope,
) -> Result<()> {
    let config_dir = get_cli_config_dir_path(db, "claude_code").await;
    let use_merge = write_mode == "merge";

    if enabled {
        for profile in PROVIDER_PROFILES {
            if !claude_profile_in_scope(scope, profile) {
                continue;
            }

            let gateway_token = gateway_token_for_profile(profile).unwrap_or("ccg-gateway");
            let config_path = config_dir.join(cli_helpers::claude_settings_filename(profile));
            // 非 default profile 只写网关必要字段，不合并用户预设配置
            let profile_default_config = if profile == DEFAULT_PROFILE {
                default_config
            } else {
                ""
            };
            let profile_previous_config = if profile == DEFAULT_PROFILE {
                previous_default_config
            } else {
                None
            };
            write_claude_gateway_settings(
                &config_path,
                profile_default_config,
                profile_previous_config,
                use_merge,
                gateway_url,
                gateway_token,
            )
            .await?;
        }
    } else {
        let gateway_config = claude_gateway_json_template();
        for profile in PROVIDER_PROFILES {
            if !claude_profile_in_scope(scope, profile) {
                continue;
            }

            let config_path = config_dir.join(cli_helpers::claude_settings_filename(profile));
            remove_json_config_content(
                &config_path,
                &gateway_config,
                default_config,
                &gateway_config,
            )
            .await?;
        }
        tracing::info!("已从 Claude Code 目标配置中移除 gateway 及预设配置");
    }

    Ok(())
}

async fn sync_claude_code_preset_update(
    db: &SqlitePool,
    default_config: &str,
    previous_default_config: &str,
    gateway_url: &str,
) -> Result<()> {
    let write_mode = get_config_write_mode(db, "claude_code").await;

    if check_cli_enabled(db, "claude_code", gateway_url).await {
        sync_claude_code_config(
            db,
            true,
            default_config,
            Some(previous_default_config),
            &write_mode,
            gateway_url,
            ClaudeProfileSyncScope::DefaultOnly,
        )
        .await?;
    }

    Ok(())
}

// Sync Codex configuration (auth.json + config.toml)
async fn sync_codex_config(
    db: &SqlitePool,
    enabled: bool,
    default_config: &str,
    previous_default_config: Option<&str>,
    write_mode: &str,
    gateway_url: &str,
) -> Result<()> {
    let codex_dir = get_cli_config_dir_path(db, "codex").await;
    let auth_path = codex_dir.join("auth.json");
    let config_path = codex_dir.join("config.toml");

    let use_merge = write_mode == "merge";

    if enabled {
        // Create config directory if it doesn't exist
        tokio::fs::create_dir_all(&codex_dir).await.map_err(|e| {
            tracing::error!("Failed to create Codex directory: {}", e);
            e.to_string()
        })?;

        write_codex_gateway_auth(&auth_path).await?;

        // merge 模式下保留现有文件中未被 gateway / 预设覆盖的顶层 key。
        let existing_content =
            if use_merge && tokio::fs::try_exists(&config_path).await.unwrap_or(false) {
                tokio::fs::read_to_string(&config_path).await.ok()
            } else {
                None
            };

        let mut final_doc = if let Some(ref content) = existing_content {
            content
                .parse::<toml_edit::DocumentMut>()
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to parse existing Codex config.toml: {}", e);
                    toml_edit::DocumentMut::new()
                })
        } else {
            toml_edit::DocumentMut::new()
        };

        remove_codex_default_gateway_entry(&mut final_doc);
        if use_merge {
            remove_previous_codex_preset(
                &mut final_doc,
                previous_default_config,
                default_config,
                true,
            );
        }

        if !default_config.is_empty() {
            match default_config.parse::<toml_edit::DocumentMut>() {
                Ok(mut custom_doc) => {
                    custom_doc.remove("model_provider");
                    custom_doc.remove("model_providers");

                    for (k, v) in custom_doc.iter() {
                        final_doc.insert(&k, v.clone());
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse custom config (invalid TOML): {}", e);
                }
            }
        }

        apply_codex_gateway_default_config(&mut final_doc, gateway_url)?;

        let final_content = final_doc.to_string();

        tokio::fs::write(&config_path, final_content)
            .await
            .map_err(|e| {
                tracing::error!("Failed to write config.toml: {}", e);
                e.to_string()
            })?;
    } else {
        remove_codex_gateway_auth_content(&auth_path).await?;
        remove_codex_gateway_config_content(&config_path, default_config).await?;
        tracing::info!("已从 Codex 配置中移除 gateway 及预设配置");
    }

    Ok(())
}

// Sync Gemini configuration (settings.json + .env)
async fn sync_gemini_config(
    db: &SqlitePool,
    enabled: bool,
    default_config: &str,
    previous_default_config: Option<&str>,
    write_mode: &str,
    gateway_url: &str,
) -> Result<()> {
    let gemini_dir = get_cli_config_dir_path(db, "gemini").await;
    let config_path = gemini_dir.join("settings.json");
    let env_path = gemini_dir.join(".env");

    let use_merge = write_mode == "merge";

    if enabled {
        // Create config directory if it doesn't exist
        tokio::fs::create_dir_all(&gemini_dir).await.map_err(|e| {
            tracing::error!("Failed to create Gemini directory: {}", e);
            e.to_string()
        })?;

        // Write .env file with gateway address
        let env_content = format!(
            "GEMINI_API_KEY=ccg-gateway\nGOOGLE_GEMINI_BASE_URL={}\n",
            gateway_url
        );
        tokio::fs::write(&env_path, env_content)
            .await
            .map_err(|e| {
                tracing::error!("Failed to write .env file: {}", e);
                e.to_string()
            })?;

        // Build gateway config
        let gateway_config = serde_json::json!({
            "security": {
                "auth": {
                    "selectedType": "gemini-api-key"
                }
            }
        });
        let protected_gateway_fields = gemini_gateway_json_template();

        let mut config = if use_merge {
            // merge 模式：先读取现有文件作为基础
            if tokio::fs::try_exists(&config_path).await.unwrap_or(false) {
                tokio::fs::read_to_string(&config_path)
                    .await
                    .ok()
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                    .unwrap_or_else(|| serde_json::json!({}))
            } else {
                serde_json::json!({})
            }
        } else {
            serde_json::json!({})
        };

        if use_merge {
            remove_previous_json_preset(
                &mut config,
                previous_default_config,
                default_config,
                &protected_gateway_fields,
                "Gemini",
            );
        }

        deep_merge(&mut config, &gateway_config);

        if !default_config.is_empty() {
            match serde_json::from_str::<serde_json::Value>(default_config) {
                Ok(custom_config) => {
                    let sanitized_config =
                        sanitize_json_config(custom_config, &protected_gateway_fields);
                    deep_merge(&mut config, &sanitized_config);
                }
                Err(e) => {
                    tracing::warn!("Failed to parse custom config (invalid JSON): {}", e);
                }
            }
        }

        // Write config file
        let config_str = serde_json::to_string_pretty(&config).map_err(|e| {
            tracing::error!("Failed to serialize config.json: {}", e);
            e.to_string()
        })?;
        tokio::fs::write(&config_path, config_str)
            .await
            .map_err(|e| {
                tracing::error!("Failed to write config.json: {}", e);
                e.to_string()
            })?;
    } else {
        let gateway_config = gemini_gateway_json_template();
        remove_gemini_gateway_env_content(&env_path, gateway_url).await?;
        remove_json_config_content(
            &config_path,
            &gateway_config,
            default_config,
            &gateway_config,
        )
        .await?;
        tracing::info!("已从 Gemini 配置中移除 gateway 及预设配置");
    }

    Ok(())
}

// Session helpers

/// 获取CLI基础目录（异步版本，支持自定义配置目录）
async fn get_cli_base_dir_async(db: &SqlitePool, cli_type: &str) -> std::path::PathBuf {
    get_cli_config_dir_path(db, cli_type).await
}
