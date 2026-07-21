use crate::config::{
    expand_home_path, get_data_dir, get_default_cli_config_dir, shrink_home_path, Config,
};
use crate::db::models::{
    AdvancedStatsRow, CliSettingsResponse, CliSettingsUpdate, DiscoverableSkill, GatewaySettings,
    InstalledSkillResponse, MarketplaceInfo, McpCliFlag, McpConfig, McpCreate, McpResponse,
    McpUpdate, OfficialCredential, OfficialCredentialCreate, OfficialCredentialResponse,
    OfficialCredentialUpdate, PaginatedLogs, PaginatedProjects, PaginatedSessions,
    PluginFavoriteItem, PluginItem, ProjectInfo, PromptCliFlag, PromptCreate, PromptPreset,
    PromptResponse, PromptUpdate, Provider, ProviderStatsResponse, ProviderStatsRow,
    RequestLogDetail, RequestLogItem, ScheduledTaskCreate, ScheduledTaskResponse, ScheduledTaskRun,
    ScheduledTaskRunItem, ScheduledTaskRunListResponse, ScheduledTaskUpdate, SessionInfo,
    SessionMessage, SkillCliFlag, SkillFavorite, SkillFavoriteItem, SkillRepo, SkillRepoCreate,
    SystemLogItem, SystemLogListResponse, SystemStatus, TimeoutSettings, TimeoutSettingsUpdate,
    WebdavBackup, WebdavSettings, WebdavSettingsUpdate,
};
use crate::services::routing::DEFAULT_PROFILE;
use crate::services::skill::{self, is_local_repo_source, InstalledSkillManifestEntry};
use crate::time::{local_compact_datetime, now_timestamp};
use crate::{LogDb, StatsDb};
use serde::Serialize;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use tauri::{Manager, State};

pub mod agent_commands;
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

const CLI_MODE_PROXY_ROUTE: &str = "proxy_route";
const CLI_MODE_PROVIDER_DIRECT: &str = "provider_direct";
const CLI_MODE_OFFICIAL_DIRECT: &str = "official_direct";
const CLI_MODE_DISABLED: &str = "disabled";

fn normalize_cli_mode(mode: &str) -> Option<&'static str> {
    match mode.trim() {
        "proxy" | CLI_MODE_PROXY_ROUTE => Some(CLI_MODE_PROXY_ROUTE),
        "direct" | CLI_MODE_OFFICIAL_DIRECT => Some(CLI_MODE_OFFICIAL_DIRECT),
        CLI_MODE_PROVIDER_DIRECT => Some(CLI_MODE_PROVIDER_DIRECT),
        CLI_MODE_DISABLED => Some(CLI_MODE_DISABLED),
        _ => None,
    }
}

async fn remember_default_provider_direct_provider_id(
    db: &SqlitePool,
    cli_type: &str,
    provider_id: i64,
    now: i64,
) -> Result<()> {
    sqlx::query(
        "UPDATE cli_settings SET last_provider_direct_provider_id = ?, updated_at = ? WHERE cli_type = ?",
    )
    .bind(provider_id)
    .bind(now)
    .bind(cli_type)
    .execute(db)
    .await
    .map_err(map_db_error)?;

    Ok(())
}

async fn remember_default_provider_direct_provider(
    db: &SqlitePool,
    provider: &Provider,
    now: i64,
) -> Result<()> {
    if provider.profile != DEFAULT_PROFILE {
        return Ok(());
    }

    remember_default_provider_direct_provider_id(db, &provider.cli_type, provider.id, now).await
}

async fn remember_last_official_credential_id(
    db: &SqlitePool,
    cli_type: &str,
    credential_id: i64,
    now: i64,
) -> Result<()> {
    sqlx::query(
        "UPDATE cli_settings SET last_official_credential_id = ?, updated_at = ? WHERE cli_type = ?",
    )
    .bind(credential_id)
    .bind(now)
    .bind(cli_type)
    .execute(db)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

async fn matched_official_credential_id(db: &SqlitePool, cli_type: &str) -> Result<Option<i64>> {
    let creds: Vec<OfficialCredential> = sqlx::query_as(
        "SELECT * FROM official_credentials WHERE cli_type = ? ORDER BY sort_order, id",
    )
    .bind(cli_type)
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())?;

    for cred in &creds {
        if credential_commands::credential_matches_cli_files(db, cred)
            .await
            .unwrap_or(false)
        {
            return Ok(Some(cred.id));
        }
    }

    Ok(None)
}

/// 通过读取配置文件动态检测 CLI 当前的路由模式
pub async fn detect_cli_mode_from_url(
    db: &SqlitePool,
    gateway_url: &str,
    cli_type: &str,
) -> &'static str {
    if crate::services::agent_config::is_provider_config_applied(
        db,
        cli_type,
        gateway_url,
        DEFAULT_PROFILE,
    )
    .await
    {
        return CLI_MODE_PROXY_ROUTE;
    }

    if crate::services::agent_config::provider_direct_active_provider_id(
        db,
        cli_type,
        DEFAULT_PROFILE,
    )
    .await
    .unwrap_or(None)
    .is_some()
    {
        return CLI_MODE_PROVIDER_DIRECT;
    }

    // 4. 匹配官方凭证
    if matched_official_credential_id(db, cli_type)
        .await
        .unwrap_or(None)
        .is_some()
    {
        return CLI_MODE_OFFICIAL_DIRECT;
    }

    // 5. 无匹配
    CLI_MODE_DISABLED
}

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

fn parse_mcp_toml_table(mcp_config_json: &str) -> Result<toml_edit::Table> {
    let value = serde_json::from_str::<serde_json::Value>(mcp_config_json)
        .map_err(|e| format!("MCP JSON 格式错误: {}", e))?;

    if !value.is_object() {
        return Err("TOML MCP 配置必须是 JSON object".to_string());
    }

    validate_toml_compatible_json(&value)?;
    serialize_toml_table(&value, "MCP config")
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

// ============================================================================
// CLI 配置目录获取（统一入口）
// ============================================================================

/// 获取 CLI 配置目录
/// 优先级：数据库配置 > 默认路径
pub async fn get_cli_config_dir_path(db: &SqlitePool, cli_type: &str) -> std::path::PathBuf {
    crate::services::cli_config::get_cli_config_dir_path(db, cli_type).await
}

// ============================================================================
// 内部辅助函数
// ============================================================================

async fn check_cli_enabled(db: &SqlitePool, cli_type: &str, gateway_url: &str) -> bool {
    crate::services::agent_config::is_provider_config_applied(
        db,
        cli_type,
        gateway_url,
        DEFAULT_PROFILE,
    )
    .await
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
    crate::services::agent_config::sync_proxy_route_config(
        db,
        cli_type,
        enabled,
        gateway_url,
        DEFAULT_PROFILE,
        default_config,
        previous_default_config,
        &write_mode,
    )
    .await
    .map(|_| ())
}

async fn sync_cli_config_with_log(
    db: &SqlitePool,
    log_db: &SqlitePool,
    cli_type: &str,
    enabled: bool,
    default_config: &str,
    previous_default_config: Option<&str>,
    gateway_url: &str,
) -> Result<()> {
    match sync_cli_config(
        db,
        cli_type,
        enabled,
        default_config,
        previous_default_config,
        gateway_url,
    )
    .await
    {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = crate::services::stats::record_system_log(
                log_db,
                "config_patch_failed",
                &format!("Agent {} 配置写入失败: {}", cli_type, error),
            )
            .await;
            Err(error)
        }
    }
}

async fn get_cli_default_config(db: &SqlitePool, cli_type: &str) -> Result<String> {
    let global_preset_enabled = crate::services::agent::get_definition(cli_type)
        .is_some_and(|definition| definition.features.global_preset.enabled);
    if !global_preset_enabled {
        return Ok(String::new());
    }
    Ok(sqlx::query_as::<_, (Option<String>,)>(
        "SELECT default_json_config FROM cli_settings WHERE cli_type = ?",
    )
    .bind(cli_type)
    .fetch_optional(db)
    .await
    .map_err(|e| e.to_string())?
    .and_then(|r| r.0)
    .unwrap_or_default())
}

// Session helpers

/// 获取CLI基础目录（异步版本，支持自定义配置目录）
async fn get_cli_base_dir_async(db: &SqlitePool, cli_type: &str) -> std::path::PathBuf {
    get_cli_config_dir_path(db, cli_type).await
}
