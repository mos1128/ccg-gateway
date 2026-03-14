use crate::config::get_data_dir;
use crate::db::models::{
    Provider, ProviderCreate, ProviderResponse, ProviderUpdate,
    GatewaySettings, TimeoutSettings, TimeoutSettingsUpdate,
    CliSettingsResponse, CliSettingsUpdate,
    RequestLogItem, RequestLogDetail, PaginatedLogs,
    SystemLogItem, SystemLogListResponse,
    DailyStats, ProviderStatsRow, ProviderStatsResponse,
    McpConfig, McpCliFlag, McpResponse, McpCreate, McpUpdate,
    PromptPreset, PromptCliFlag, PromptResponse, PromptCreate, PromptUpdate,
    SkillRepo, SkillRepoCreate,
    SkillConfig, SkillCliFlag, DiscoverableSkill, InstalledSkillResponse,
    WebdavSettings, WebdavSettingsUpdate, WebdavBackup,
    ProjectInfo, SessionInfo, PaginatedProjects, PaginatedSessions, SessionMessage,
    SystemStatus,
    OfficialCredential, OfficialCredentialCreate, OfficialCredentialUpdate, OfficialCredentialResponse,
};
use crate::LogDb;
use sqlx::SqlitePool;
use tauri::State;
use std::io::Read;

type Result<T> = std::result::Result<T, String>;

#[tauri::command]
pub async fn get_providers(
    db: State<'_, SqlitePool>,
    cli_type: Option<String>,
) -> Result<Vec<ProviderResponse>> {
    let providers = if let Some(ct) = cli_type {
        sqlx::query_as::<_, Provider>(
            "SELECT * FROM providers WHERE cli_type = ? ORDER BY sort_order, id",
        )
        .bind(&ct)
        .fetch_all(db.inner())
        .await
    } else {
        sqlx::query_as::<_, Provider>("SELECT * FROM providers ORDER BY sort_order, id")
            .fetch_all(db.inner())
            .await
    };

    let providers = providers.map_err(|e| e.to_string())?;
    let mut results = Vec::new();

    for provider in providers {
        let mut response = ProviderResponse::from(provider.clone());

        // Load model maps
        let maps: Vec<(i64, String, String, i64)> = sqlx::query_as(
            "SELECT id, source_model, target_model, enabled FROM provider_model_map WHERE provider_id = ? ORDER BY id",
        )
        .bind(provider.id)
        .fetch_all(db.inner())
        .await
        .map_err(|e| e.to_string())?;

        response.model_maps = maps
            .into_iter()
            .map(|(id, source_model, target_model, enabled)| crate::db::models::ModelMapResponse {
                id,
                source_model,
                target_model,
                enabled: enabled != 0,
            })
            .collect();

        // Load model blacklist
        let blacklist: Vec<(i64, String)> = sqlx::query_as(
            "SELECT id, model_pattern FROM provider_model_blacklist WHERE provider_id = ? ORDER BY id",
        )
        .bind(provider.id)
        .fetch_all(db.inner())
        .await
        .map_err(|e| e.to_string())?;

        response.model_blacklist = blacklist
            .into_iter()
            .map(|(id, model_pattern)| crate::db::models::ModelBlacklistResponse {
                id,
                model_pattern,
            })
            .collect();

        results.push(response);
    }

    Ok(results)
}

#[tauri::command]
pub async fn get_provider(db: State<'_, SqlitePool>, id: i64) -> Result<ProviderResponse> {
    let provider = sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Provider not found".to_string())?;

    let mut response = ProviderResponse::from(provider);

    // Load model maps
    let maps: Vec<(i64, String, String, i64)> = sqlx::query_as(
        "SELECT id, source_model, target_model, enabled FROM provider_model_map WHERE provider_id = ? ORDER BY id",
    )
    .bind(id)
    .fetch_all(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    response.model_maps = maps
        .into_iter()
        .map(|(id, source_model, target_model, enabled)| crate::db::models::ModelMapResponse {
            id,
            source_model,
            target_model,
            enabled: enabled != 0,
        })
        .collect();

    // Load model blacklist
    let blacklist: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, model_pattern FROM provider_model_blacklist WHERE provider_id = ? ORDER BY id",
    )
    .bind(id)
    .fetch_all(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    response.model_blacklist = blacklist
        .into_iter()
        .map(|(id, model_pattern)| crate::db::models::ModelBlacklistResponse {
            id,
            model_pattern,
        })
        .collect();

    Ok(response)
}

#[tauri::command]
pub async fn create_provider(
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    input: ProviderCreate,
) -> Result<ProviderResponse> {
    let now = chrono::Utc::now().timestamp();
    let cli_type = input.cli_type.unwrap_or_else(|| "claude_code".to_string());
    let provider_name = input.name.clone();

    // Normalize custom_useragent: treat empty string as None
    let custom_ua = input.custom_useragent
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let result = sqlx::query(
        r#"
        INSERT INTO providers (cli_type, name, base_url, api_key, enabled, failure_threshold, blacklist_minutes, consecutive_failures, sort_order, custom_useragent, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, 0, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM providers), ?, ?, ?)
        "#,
    )
    .bind(&cli_type)
    .bind(&input.name)
    .bind(&input.base_url)
    .bind(&input.api_key)
    .bind(input.enabled.unwrap_or(true) as i64)
    .bind(input.failure_threshold.unwrap_or(3))
    .bind(input.blacklist_minutes.unwrap_or(10))
    .bind(&custom_ua)
    .bind(now)
    .bind(now)
    .execute(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    let id = result.last_insert_rowid();

    // Insert model maps if provided
    if let Some(model_maps) = input.model_maps {
        for map in model_maps {
            sqlx::query(
                "INSERT INTO provider_model_map (provider_id, source_model, target_model, enabled) VALUES (?, ?, ?, ?)",
            )
            .bind(id)
            .bind(&map.source_model)
            .bind(&map.target_model)
            .bind(map.enabled as i64)
            .execute(db.inner())
            .await
            .map_err(|e| e.to_string())?;
        }
    }

    // Insert model blacklist if provided
    if let Some(model_blacklist) = input.model_blacklist {
        for item in model_blacklist {
            sqlx::query(
                "INSERT INTO provider_model_blacklist (provider_id, model_pattern) VALUES (?, ?)",
            )
            .bind(id)
            .bind(&item.model_pattern)
            .execute(db.inner())
            .await
            .map_err(|e| e.to_string())?;
        }
    }

    // Log system event
    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "provider_created",
        &format!("服务商 {} 已创建", provider_name),
    ).await;

    get_provider(db, id).await
}

#[tauri::command]
pub async fn update_provider(
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    id: i64,
    input: ProviderUpdate,
) -> Result<ProviderResponse> {
    let now = chrono::Utc::now().timestamp();

    // Get provider name for logging
    let provider_name: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM providers WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    let provider_name = provider_name.map(|(n,)| n).unwrap_or_else(|| format!("Provider#{}", id));

    // Check if model maps will be updated (before moving)
    let has_model_maps_update = input.model_maps.is_some();
    let has_model_blacklist_update = input.model_blacklist.is_some();

    // Build dynamic update query
    let mut updates = vec!["updated_at = ?".to_string()];
    let mut has_updates = false;

    if input.name.is_some() {
        updates.push("name = ?".to_string());
        has_updates = true;
    }
    if input.base_url.is_some() {
        updates.push("base_url = ?".to_string());
        has_updates = true;
    }
    if input.api_key.is_some() {
        updates.push("api_key = ?".to_string());
        has_updates = true;
    }
    if input.enabled.is_some() {
        updates.push("enabled = ?".to_string());
        has_updates = true;
    }
    if input.failure_threshold.is_some() {
        updates.push("failure_threshold = ?".to_string());
        has_updates = true;
    }
    if input.blacklist_minutes.is_some() {
        updates.push("blacklist_minutes = ?".to_string());
        has_updates = true;
    }
    if input.custom_useragent.is_some() {
        updates.push("custom_useragent = ?".to_string());
        has_updates = true;
    }

    if has_updates {
        let query = format!("UPDATE providers SET {} WHERE id = ?", updates.join(", "));
        let mut q = sqlx::query(&query).bind(now);

        if let Some(ref name) = input.name {
            q = q.bind(name);
        }
        if let Some(ref base_url) = input.base_url {
            q = q.bind(base_url);
        }
        if let Some(ref api_key) = input.api_key {
            q = q.bind(api_key);
        }
        if let Some(enabled) = input.enabled {
            q = q.bind(enabled as i64);
        }
        if let Some(failure_threshold) = input.failure_threshold {
            q = q.bind(failure_threshold);
        }
        if let Some(blacklist_minutes) = input.blacklist_minutes {
            q = q.bind(blacklist_minutes);
        }
        if let Some(ref custom_useragent) = input.custom_useragent {
            // Normalize: treat empty string as NULL
            let ua = custom_useragent.trim();
            if ua.is_empty() {
                q = q.bind(None::<String>);
            } else {
                q = q.bind(ua);
            }
        }

        q.bind(id)
            .execute(db.inner())
            .await
            .map_err(|e| e.to_string())?;
    }

    // Update model maps if provided
    if let Some(model_maps) = input.model_maps {
        // Delete existing maps
        sqlx::query("DELETE FROM provider_model_map WHERE provider_id = ?")
            .bind(id)
            .execute(db.inner())
            .await
            .map_err(|e| e.to_string())?;

        // Insert new maps
        for map in model_maps {
            sqlx::query(
                "INSERT INTO provider_model_map (provider_id, source_model, target_model, enabled) VALUES (?, ?, ?, ?)",
            )
            .bind(id)
            .bind(&map.source_model)
            .bind(&map.target_model)
            .bind(map.enabled as i64)
            .execute(db.inner())
            .await
            .map_err(|e| e.to_string())?;
        }
    }

    // Update model blacklist if provided
    if let Some(model_blacklist) = input.model_blacklist {
        // Delete existing blacklist
        sqlx::query("DELETE FROM provider_model_blacklist WHERE provider_id = ?")
            .bind(id)
            .execute(db.inner())
            .await
            .map_err(|e| e.to_string())?;

        // Insert new blacklist
        for item in model_blacklist {
            sqlx::query(
                "INSERT INTO provider_model_blacklist (provider_id, model_pattern) VALUES (?, ?)",
            )
            .bind(id)
            .bind(&item.model_pattern)
            .execute(db.inner())
            .await
            .map_err(|e| e.to_string())?;
        }
    }

    // Log system event (only if there were actual updates)
    if has_updates || has_model_maps_update || has_model_blacklist_update {
        let _ = crate::services::stats::record_system_log(
            &log_db.0,
            "provider_updated",
            &format!("服务商 {} 已更新", provider_name),
        ).await;
    }

    get_provider(db, id).await
}

#[tauri::command]
pub async fn delete_provider(
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    id: i64,
) -> Result<()> {
    // Get provider name before deletion
    let provider_name: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM providers WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    let provider_name = provider_name.map(|(n,)| n).unwrap_or_else(|| format!("Provider#{}", id));

    // Delete associated model maps first (cascade delete)
    sqlx::query("DELETE FROM provider_model_map WHERE provider_id = ?")
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    // Delete associated model blacklist
    sqlx::query("DELETE FROM provider_model_blacklist WHERE provider_id = ?")
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    // Then delete the provider
    sqlx::query("DELETE FROM providers WHERE id = ?")
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    // Log system event
    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "provider_deleted",
        &format!("服务商 {} 已删除", provider_name),
    ).await;

    Ok(())
}

#[tauri::command]
pub async fn reorder_providers(db: State<'_, SqlitePool>, ids: Vec<i64>) -> Result<()> {
    for (idx, id) in ids.iter().enumerate() {
        sqlx::query("UPDATE providers SET sort_order = ? WHERE id = ?")
            .bind(idx as i64)
            .bind(id)
            .execute(db.inner())
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn reset_provider_failures(
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    id: i64,
) -> Result<()> {
    // Get provider name for logging
    let provider_name: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM providers WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    let provider_name = provider_name.map(|(n,)| n).unwrap_or_else(|| format!("Provider#{}", id));

    sqlx::query("UPDATE providers SET consecutive_failures = 0, blacklisted_until = NULL WHERE id = ?")
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    // Log system event
    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "provider_reset",
        &format!("服务商 {} 状态已手动重置", provider_name),
    ).await;

    Ok(())
}

// Settings commands
#[tauri::command]
pub async fn get_gateway_settings(db: State<'_, SqlitePool>) -> Result<GatewaySettings> {
    sqlx::query_as::<_, GatewaySettings>("SELECT debug_log FROM gateway_settings WHERE id = 1")
        .fetch_one(db.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_gateway_settings(
    db: State<'_, SqlitePool>,
    debug_log: bool,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let debug_log_val = if debug_log { 1i64 } else { 0 };

    sqlx::query("UPDATE gateway_settings SET debug_log = ?, updated_at = ? WHERE id = 1")
        .bind(debug_log_val)
        .bind(now)
        .execute(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_timeout_settings(db: State<'_, SqlitePool>) -> Result<TimeoutSettings> {
    sqlx::query_as::<_, TimeoutSettings>(
        "SELECT stream_first_byte_timeout, stream_idle_timeout, non_stream_timeout FROM timeout_settings WHERE id = 1",
    )
    .fetch_one(db.inner())
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_timeout_settings(
    db: State<'_, SqlitePool>,
    input: TimeoutSettingsUpdate,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let current = get_timeout_settings(db.clone()).await?;

    sqlx::query(
        "UPDATE timeout_settings SET stream_first_byte_timeout = ?, stream_idle_timeout = ?, non_stream_timeout = ?, updated_at = ? WHERE id = 1",
    )
    .bind(input.stream_first_byte_timeout.unwrap_or(current.stream_first_byte_timeout))
    .bind(input.stream_idle_timeout.unwrap_or(current.stream_idle_timeout))
    .bind(input.non_stream_timeout.unwrap_or(current.non_stream_timeout))
    .bind(now)
    .execute(db.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_cli_settings(db: State<'_, SqlitePool>, cli_type: String) -> Result<CliSettingsResponse> {
    let row = sqlx::query_as::<_, CliSettingsRowWithoutConfigDir>(
        "SELECT cli_type, default_json_config, cli_mode, updated_at FROM cli_settings WHERE cli_type = ?",
    )
    .bind(&cli_type)
    .fetch_optional(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    // 从数据库获取 config_dir（可能为用户自定义值）
    let config_dir = get_cli_config_dir(db.inner(), &cli_type).await;
    // 默认 config_dir 始终是系统路径（硬编码）
    let default_config_dir = get_default_config_dir(&cli_type);

    if let Some(row) = row {
        let enabled = check_cli_enabled(db.inner(), &cli_type).await;

        Ok(CliSettingsResponse {
            cli_type: row.cli_type,
            enabled,
            default_json_config: row.default_json_config.unwrap_or_default(),
            cli_mode: row.cli_mode,
            config_dir,
            default_config_dir,
        })
    } else {
        Ok(CliSettingsResponse {
            cli_type: cli_type.clone(),
            enabled: false,
            default_json_config: String::new(),
            cli_mode: "proxy".to_string(),
            config_dir,
            default_config_dir,
        })
    }
}

/// CliSettingsRow without config_dir (for backward compatibility with old databases)
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct CliSettingsRowWithoutConfigDir {
    pub cli_type: String,
    pub default_json_config: Option<String>,
    pub cli_mode: String,
    pub updated_at: i64,
}

#[tauri::command]
pub async fn update_cli_settings(
    db: State<'_, SqlitePool>,
    cli_type: String,
    input: CliSettingsUpdate,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    // Validate and update database
    if let Some(ref config) = input.default_json_config {
        let config_trimmed = config.trim();

        // Validate format if config is not empty
        if !config_trimmed.is_empty() {
            match cli_type.as_str() {
                "claude_code" | "gemini" => {
                    // Validate JSON format
                    serde_json::from_str::<serde_json::Value>(config_trimmed)
                        .map_err(|e| format!("JSON 格式错误: {}", e))?;
                }
                "codex" => {
                    // Validate TOML format
                    config_trimmed.parse::<toml_edit::DocumentMut>()
                        .map_err(|e| format!("TOML 格式错误: {}", e))?;
                }
                _ => {}
            }
        }

        sqlx::query(
            "UPDATE cli_settings SET default_json_config = ?, updated_at = ? WHERE cli_type = ?",
        )
        .bind(config_trimmed)
        .bind(now)
        .bind(&cli_type)
        .execute(db.inner())
        .await
        .map_err(|e| e.to_string())?;
    }

    // Update config_dir if provided
    if let Some(ref config_dir) = input.config_dir {
        sqlx::query(
            "UPDATE cli_settings SET config_dir = ?, updated_at = ? WHERE cli_type = ?",
        )
        .bind(config_dir)
        .bind(now)
        .bind(&cli_type)
        .execute(db.inner())
        .await
        .map_err(|e| e.to_string())?;
    }

    // Update CLI config file if enabled flag is provided
    if let Some(enabled) = input.enabled {
        // 检查当前模式
        let current_mode: Option<(String,)> = sqlx::query_as(
            "SELECT cli_mode FROM cli_settings WHERE cli_type = ?",
        )
        .bind(&cli_type)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?;

        let mode = current_mode.map(|r| r.0).unwrap_or_else(|| "proxy".to_string());

        // 只有在中转模式下才处理 enabled 参数
        if mode == "proxy" {
            // 检查 CLI 当前是否已经处于目标状态
            let current_enabled = check_cli_enabled(db.inner(), &cli_type).await;
            
            if current_enabled == enabled {
                tracing::info!("{} CLI 已经处于目标状态（enabled={}），跳过操作", cli_type, enabled);
            } else {
                // Get default_json_config from database (without config_dir to avoid column errors)
                let row = sqlx::query_as::<_, CliSettingsRowWithoutConfigDir>(
                    "SELECT cli_type, default_json_config, cli_mode, updated_at FROM cli_settings WHERE cli_type = ?",
                )
                .bind(&cli_type)
                .fetch_optional(db.inner())
                .await
                .map_err(|e| e.to_string())?;

                let default_config = row.and_then(|r| r.default_json_config).unwrap_or_default();
                tracing::info!("{} 执行 CLI 状态切换：enabled={}", cli_type, enabled);
                sync_cli_config(db.inner(), &cli_type, enabled, &default_config).await?;
            }
        } else {
            tracing::info!("{} 处于直连模式，忽略 enabled 参数", cli_type);
        }
    }

    Ok(())
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
    let config_path = match cli_type {
        "claude_code" => {
            // Claude Code MCP 配置在 .claude.json（父目录）
            let config_dir = get_cli_config_dir_path(db, cli_type).await;
            config_dir.parent().map(|p| p.join(".claude.json"))
        }
        "gemini" => {
            let config_dir = get_cli_config_dir_path(db, cli_type).await;
            Some(config_dir.join("settings.json"))
        }
        "codex" => {
            let config_dir = get_cli_config_dir_path(db, cli_type).await;
            Some(config_dir.join("config.toml"))
        }
        _ => None,
    };

    let path = match config_path {
        Some(p) => p,
        None => return false,
    };

    if !path.exists() {
        return false;
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    match cli_type {
        "claude_code" | "gemini" => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(config) => {
                    config.get("mcpServers")
                        .and_then(|v| v.as_object())
                        .map(|servers| servers.contains_key(mcp_name))
                        .unwrap_or(false)
                }
                Err(_) => false,
            }
        }
        "codex" => {
            match content.parse::<toml_edit::DocumentMut>() {
                Ok(doc) => {
                    doc.get("mcp_servers")
                        .and_then(|v| v.as_table())
                        .map(|servers| servers.contains_key(mcp_name))
                        .unwrap_or(false)
                }
                Err(_) => false,
            }
        }
        _ => false,
    }
}


// Check if prompt content matches the file content - 异步版本，支持自定义配置目录
async fn prompt_enabled_in_file_async(db: &SqlitePool, cli_type: &str, prompt_content: &str) -> bool {
    let config_dir = get_cli_config_dir_path(db, cli_type).await;
    
    let prompt_path = match cli_type {
        "claude_code" => config_dir.join("CLAUDE.md"),
        "codex" => config_dir.join("AGENTS.md"),
        "gemini" => config_dir.join("GEMINI.md"),
        _ => return false,
    };

    if !prompt_path.exists() {
        return false;
    }

    let file_content = match std::fs::read_to_string(&prompt_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    // Normalize and compare
    normalize_text(prompt_content) == normalize_text(&file_content)
}

// ============================================================================
// 统一的配置目录获取方法
// ============================================================================

/// 从数据库获取CLI配置目录（异步版本）
/// 如果数据库中没有设置，返回默认配置目录
pub async fn get_cli_config_dir(db: &SqlitePool, cli_type: &str) -> String {
    let result: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT config_dir FROM cli_settings WHERE cli_type = ?",
    )
    .bind(cli_type)
    .fetch_optional(db)
    .await
    .ok()
    .flatten();

    result
        .and_then(|r| r.0)
        .unwrap_or_else(|| get_default_config_dir(cli_type))
}

/// 获取CLI默认配置目录
pub fn get_default_config_dir(cli_type: &str) -> String {
    let home = dirs::home_dir().unwrap_or_default();
    match cli_type {
        "claude_code" => home.join(".claude").to_string_lossy().to_string(),
        "codex" => home.join(".codex").to_string_lossy().to_string(),
        "gemini" => home.join(".gemini").to_string_lossy().to_string(),
        _ => home.to_string_lossy().to_string(),
    }
}

/// 获取CLI配置目录的PathBuf版本（异步）
pub async fn get_cli_config_dir_path(db: &SqlitePool, cli_type: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(get_cli_config_dir(db, cli_type).await)
}

// ============================================================================
// 以下是旧的函数，保留用于向后兼容，内部调用新的统一方法
// ============================================================================

// 从数据库获取配置目录，如果没有则返回默认值
async fn get_config_dir_from_db(db: &SqlitePool, cli_type: &str) -> String {
    get_cli_config_dir(db, cli_type).await
}

async fn check_cli_enabled(db: &SqlitePool, cli_type: &str) -> bool {
    match cli_type {
        "claude_code" => check_claude_uses_gateway(db, cli_type).await,
        "codex" => check_codex_uses_gateway(db, cli_type).await,
        "gemini" => check_gemini_uses_gateway(db, cli_type).await,
        _ => false,
    }
}

async fn check_claude_uses_gateway(db: &SqlitePool, cli_type: &str) -> bool {
    let config_dir = get_config_dir_from_db(db, cli_type).await;
    let config_path = std::path::PathBuf::from(config_dir).join("settings.json");

    if !config_path.exists() {
        return false;
    }

    let content = match std::fs::read_to_string(&config_path) {
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
                    return base_url.contains("127.0.0.1:7788") || base_url.contains("localhost:7788");
                }
            }
            false
        }
        Err(_) => false,
    }
}

async fn check_codex_uses_gateway(db: &SqlitePool, cli_type: &str) -> bool {
    let config_dir = get_config_dir_from_db(db, cli_type).await;
    let config_path = std::path::PathBuf::from(config_dir).join("config.toml");

    if !config_path.exists() {
        return false;
    }

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    if content.trim().is_empty() {
        return false;
    }

    match content.parse::<toml_edit::DocumentMut>() {
        Ok(doc) => {
            // Check if model_provider is "ccg-gateway"
            if let Some(provider) = doc.get("model_provider").and_then(|v| v.as_str()) {
                if provider == "ccg-gateway" {
                    return true;
                }
            }
            false
        }
        Err(_) => false,
    }
}

async fn check_gemini_uses_gateway(db: &SqlitePool, cli_type: &str) -> bool {
    let config_dir = get_config_dir_from_db(db, cli_type).await;
    let env_path = std::path::PathBuf::from(config_dir).join(".env");

    if !env_path.exists() {
        return false;
    }

    let content = match std::fs::read_to_string(&env_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    // Check if .env contains GOOGLE_GEMINI_BASE_URL pointing to gateway
    for line in content.lines() {
        if line.starts_with("GOOGLE_GEMINI_BASE_URL=") {
            let url = line.split('=').nth(1).unwrap_or("");
            return url.contains("127.0.0.1:7788") || url.contains("localhost:7788");
        }
    }
    false
}

// Get the config file path for MCP/prompts sync (different for Codex)
async fn get_mcp_config_path(db: &SqlitePool, cli_type: &str) -> Option<std::path::PathBuf> {
    let config_dir = get_config_dir_from_db(db, cli_type).await;
    let base_path = std::path::PathBuf::from(config_dir);
    
    match cli_type {
        "claude_code" => {
            // Claude Code MCP goes to ~/.claude.json (parent of config_dir)
            base_path.parent().map(|p| p.join(".claude.json"))
        },
        "codex" => Some(base_path.join("config.toml")),  // Codex MCP goes to config.toml
        "gemini" => Some(base_path.join("settings.json")),
        _ => None,
    }
}

async fn sync_cli_config(db: &SqlitePool, cli_type: &str, enabled: bool, default_config: &str) -> Result<()> {
    match cli_type {
        "claude_code" => sync_claude_code_config(db, enabled, default_config).await,
        "codex" => sync_codex_config(db, enabled, default_config).await,
        "gemini" => sync_gemini_config(db, enabled, default_config).await,
        _ => Err("Invalid CLI type".to_string()),
    }
}

fn get_backup_path(original_path: &std::path::Path) -> std::path::PathBuf {
    let file_name = original_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    let parent = original_path.parent().unwrap_or(original_path);
    parent.join(format!("{}.ccg-backup", file_name))
}

fn backup_file(path: &std::path::Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let backup_path = get_backup_path(path);
    std::fs::copy(path, &backup_path).map_err(|e| {
        tracing::error!("Failed to backup {}: {}", path.display(), e);
        e.to_string()
    })?;
    Ok(())
}

fn restore_backup(path: &std::path::Path) -> Result<bool> {
    let backup_path = get_backup_path(path);
    if !backup_path.exists() {
        return Ok(false);
    }
    std::fs::copy(&backup_path, path).map_err(|e| {
        tracing::error!("Failed to restore backup from {}: {}", backup_path.display(), e);
        e.to_string()
    })?;
    std::fs::remove_file(&backup_path).map_err(|e| {
        tracing::warn!("Failed to remove backup file {}: {}", backup_path.display(), e);
        e.to_string()
    })?;
    Ok(true)
}

fn has_backup(path: &std::path::Path) -> bool {
    get_backup_path(path).exists()
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

// Sync Claude Code configuration (settings.json)
async fn sync_claude_code_config(db: &SqlitePool, enabled: bool, default_config: &str) -> Result<()> {
    let config_dir = get_cli_config_dir_path(db, "claude_code").await;
    let config_path = config_dir.join("settings.json");

    if enabled {
        // Backup existing config if not already backed up
        if config_path.exists() && !has_backup(&config_path) {
            backup_file(&config_path)?;
        }

        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                tracing::error!("Failed to create directory: {}", e);
                e.to_string()
            })?;
        }

        // Build base config with gateway address
        let mut config = serde_json::json!({
            "env": {
                "ANTHROPIC_BASE_URL": "http://127.0.0.1:7788",
                "ANTHROPIC_AUTH_TOKEN": "ccg-gateway"
            }
        });

        // Merge user's custom config if provided
        if !default_config.is_empty() {
            match serde_json::from_str::<serde_json::Value>(default_config) {
                Ok(custom_config) => {
                    deep_merge(&mut config, &custom_config);
                }
                Err(e) => {
                    tracing::warn!("Failed to parse custom config (invalid JSON): {}", e);
                }
            }
        }

        // Write config file
        let config_str = serde_json::to_string_pretty(&config).map_err(|e| {
            tracing::error!("Failed to serialize config: {}", e);
            e.to_string()
        })?;
        std::fs::write(&config_path, config_str).map_err(|e| {
            tracing::error!("Failed to write config file: {}", e);
            e.to_string()
        })?;
    } else {
        // When disabling, restore backup or remove config file
        if restore_backup(&config_path)? {
            tracing::info!("已恢复 Claude Code settings.json 备份");
        } else if config_path.exists() {
            tracing::info!("删除 Claude Code settings.json（无备份）");
            // No backup, remove the config file
            std::fs::remove_file(&config_path).map_err(|e| {
                tracing::error!("Failed to remove config file: {}", e);
                e.to_string()
            })?;
        }
    }

    Ok(())
}

// Sync Codex configuration (auth.json + config.toml)
async fn sync_codex_config(db: &SqlitePool, enabled: bool, default_config: &str) -> Result<()> {
    let codex_dir = get_cli_config_dir_path(db, "codex").await;
    let auth_path = codex_dir.join("auth.json");
    let config_path = codex_dir.join("config.toml");

    if enabled {
        // Backup existing configs if not already backed up
        if auth_path.exists() && !has_backup(&auth_path) {
            backup_file(&auth_path)?;
        }
        if config_path.exists() && !has_backup(&config_path) {
            backup_file(&config_path)?;
        }

        // Create config directory if it doesn't exist
        std::fs::create_dir_all(&codex_dir).map_err(|e| {
            tracing::error!("Failed to create Codex directory: {}", e);
            e.to_string()
        })?;

        // Write auth.json with gateway API key
        let auth = serde_json::json!({
            "OPENAI_API_KEY": "ccg-gateway"
        });
        let auth_str = serde_json::to_string_pretty(&auth).map_err(|e| {
            tracing::error!("Failed to serialize auth.json: {}", e);
            e.to_string()
        })?;
        std::fs::write(&auth_path, auth_str).map_err(|e| {
            tracing::error!("Failed to write auth.json: {}", e);
            e.to_string()
        })?;

        // Build base config.toml pointing to gateway
        let mut doc = toml_edit::DocumentMut::new();
        doc["model_provider"] = toml_edit::value("ccg-gateway");

        if !doc.contains_table("model_providers") {
            doc["model_providers"] = toml_edit::table();
        }

        let mut gateway_table = toml_edit::Table::new();
        gateway_table.insert("name", toml_edit::value("ccg-gateway"));
        gateway_table.insert("base_url", toml_edit::value("http://127.0.0.1:7788"));
        gateway_table.insert("wire_api", toml_edit::value("responses"));
        gateway_table.insert("requires_openai_auth", toml_edit::value(false));

        doc["model_providers"]["ccg-gateway"] = toml_edit::Item::Table(gateway_table);

        // Merge user's custom config if provided (TOML format)
        if !default_config.is_empty() {
            match default_config.parse::<toml_edit::DocumentMut>() {
                Ok(custom_doc) => {
                    // Merge custom config into base config
                    for (key, value) in custom_doc.iter() {
                        if key != "model_provider" && key != "model_providers" {
                            doc[key] = value.clone();
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse custom config (invalid TOML): {}", e);
                }
            }
        }

        std::fs::write(&config_path, doc.to_string()).map_err(|e| {
            tracing::error!("Failed to write config.toml: {}", e);
            e.to_string()
        })?;
    } else {
        // When disabling, restore backups or remove config files
        let auth_restored = restore_backup(&auth_path)?;
        let config_restored = restore_backup(&config_path)?;

        if auth_restored {
            tracing::info!("已恢复 Codex auth.json 备份");
        } else if auth_path.exists() {
            tracing::info!("删除 Codex auth.json（无备份）");
            std::fs::remove_file(&auth_path).map_err(|e| {
                tracing::error!("Failed to remove auth.json: {}", e);
                e.to_string()
            })?;
        }

        if config_restored {
            tracing::info!("已恢复 Codex config.toml 备份");
        } else if config_path.exists() {
            tracing::info!("删除 Codex config.toml（无备份）");
            std::fs::remove_file(&config_path).map_err(|e| {
                tracing::error!("Failed to remove config.toml: {}", e);
                e.to_string()
            })?;
        }
    }

    Ok(())
}

// Sync Gemini configuration (settings.json + .env)
async fn sync_gemini_config(db: &SqlitePool, enabled: bool, default_config: &str) -> Result<()> {
    let gemini_dir = get_cli_config_dir_path(db, "gemini").await;
    let config_path = gemini_dir.join("settings.json");
    let env_path = gemini_dir.join(".env");

    if enabled {
        // Backup existing configs if not already backed up
        if config_path.exists() && !has_backup(&config_path) {
            backup_file(&config_path)?;
        }
        if env_path.exists() && !has_backup(&env_path) {
            backup_file(&env_path)?;
        }

        // Create config directory if it doesn't exist
        std::fs::create_dir_all(&gemini_dir).map_err(|e| {
            tracing::error!("Failed to create Gemini directory: {}", e);
            e.to_string()
        })?;

        // Write .env file with gateway address
        let env_content = "GEMINI_API_KEY=ccg-gateway\nGOOGLE_GEMINI_BASE_URL=http://127.0.0.1:7788\n".to_string();
        std::fs::write(&env_path, env_content).map_err(|e| {
            tracing::error!("Failed to write .env file: {}", e);
            e.to_string()
        })?;

        // Build base config with security.auth.selectedType
        let mut config = serde_json::json!({
            "security": {
                "auth": {
                    "selectedType": "gemini-api-key"
                }
            }
        });

        // Merge user's custom config if provided
        if !default_config.is_empty() {
            match serde_json::from_str::<serde_json::Value>(default_config) {
                Ok(custom_config) => {
                    deep_merge(&mut config, &custom_config);
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
        std::fs::write(&config_path, config_str).map_err(|e| {
            tracing::error!("Failed to write config.json: {}", e);
            e.to_string()
        })?;
    } else {
        // When disabling, restore backups or remove config files
        let env_restored = restore_backup(&env_path)?;
        let config_restored = restore_backup(&config_path)?;

        if env_restored {
            tracing::info!("已恢复 Gemini .env 备份");
        } else if env_path.exists() {
            tracing::info!("删除 Gemini .env（无备份）");
            std::fs::remove_file(&env_path).map_err(|e| {
                tracing::error!("Failed to remove .env file: {}", e);
                e.to_string()
            })?;
        }

        if config_restored {
            tracing::info!("已恢复 Gemini settings.json 备份");
        } else if config_path.exists() {
            tracing::info!("删除 Gemini settings.json（无备份）");
            std::fs::remove_file(&config_path).map_err(|e| {
                tracing::error!("Failed to remove config.json: {}", e);
                e.to_string()
            })?;
        }
    }

    Ok(())
}

// Log commands
#[tauri::command]
pub async fn get_request_logs(
    log_db: State<'_, crate::LogDb>,
    page: Option<i64>,
    page_size: Option<i64>,
    cli_type: Option<String>,
    provider_name: Option<String>,
) -> Result<PaginatedLogs> {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;
    let pool = &log_db.0;

    let mut sql = "SELECT id, created_at, cli_type, provider_name, model_id, status_code, elapsed_ms, input_tokens, output_tokens, client_method, client_path, source_model, target_model FROM request_logs WHERE 1=1".to_string();
    let mut count_sql = "SELECT COUNT(*) FROM request_logs WHERE 1=1".to_string();

    if cli_type.is_some() {
        sql.push_str(" AND cli_type = ?");
        count_sql.push_str(" AND cli_type = ?");
    }
    if provider_name.is_some() {
        sql.push_str(" AND provider_name = ?");
        count_sql.push_str(" AND provider_name = ?");
    }

    sql.push_str(" ORDER BY id DESC LIMIT ? OFFSET ?");

    let mut q = sqlx::query_as::<_, RequestLogItem>(&sql);

    if let Some(ct) = &cli_type {
        q = q.bind(ct);
    }
    if let Some(pn) = &provider_name {
        q = q.bind(pn);
    }

    q = q.bind(page_size).bind(offset);

    let items = q.fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    let mut count_q = sqlx::query_as::<_, (i64,)>(&count_sql);
    if let Some(ct) = &cli_type {
        count_q = count_q.bind(ct);
    }
    if let Some(pn) = &provider_name {
        count_q = count_q.bind(pn);
    }

    let (total,) = count_q.fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(PaginatedLogs {
        items,
        total,
        page,
        page_size,
    })
}

#[tauri::command]
pub async fn clear_request_logs(log_db: State<'_, crate::LogDb>) -> Result<()> {
    sqlx::query("DELETE FROM request_logs")
        .execute(&log_db.0)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_request_log_detail(
    log_db: State<'_, crate::LogDb>,
    id: i64,
) -> Result<RequestLogDetail> {
    sqlx::query_as::<_, RequestLogDetail>(
        "SELECT id, created_at, cli_type, provider_name, model_id, status_code, elapsed_ms, input_tokens, output_tokens, client_method, client_path, client_headers, client_body, forward_url, forward_headers, forward_body, provider_headers, provider_body, error_message, source_model, target_model FROM request_logs WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&log_db.0)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Log not found".to_string())
}

// System logs commands
#[tauri::command]
pub async fn get_system_logs(
    log_db: State<'_, crate::LogDb>,
    page: Option<i64>,
    page_size: Option<i64>,
    event_type: Option<String>,
) -> Result<SystemLogListResponse> {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    // Build query
    let mut sql = "SELECT id, created_at, event_type, message FROM system_logs WHERE 1=1".to_string();
    let mut count_sql = "SELECT COUNT(*) FROM system_logs WHERE 1=1".to_string();

    if event_type.is_some() {
        sql.push_str(" AND event_type = ?");
        count_sql.push_str(" AND event_type = ?");
    }

    sql.push_str(" ORDER BY id DESC LIMIT ? OFFSET ?");
    let mut q = sqlx::query_as::<_, SystemLogItem>(&sql);

    if let Some(et) = &event_type {
        q = q.bind(et);
    }

    q = q.bind(page_size).bind(offset);

    let items = q.fetch_all(&log_db.0)
        .await
        .map_err(|e| e.to_string())?;

    // Get total count
    let mut count_q = sqlx::query_as::<_, (i64,)>(&count_sql);
    if let Some(et) = &event_type {
        count_q = count_q.bind(et);
    }
    let (total,) = count_q.fetch_one(&log_db.0)
        .await
        .map_err(|e| e.to_string())?;

    Ok(SystemLogListResponse {
        items,
        total,
        page,
        page_size,
    })
}

#[tauri::command]
pub async fn clear_system_logs(log_db: State<'_, crate::LogDb>) -> Result<()> {
    sqlx::query("DELETE FROM system_logs")
        .execute(&log_db.0)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// System status
#[tauri::command]
pub async fn get_system_status() -> Result<SystemStatus> {
    Ok(SystemStatus {
        status: "running".to_string(),
        port: 7788,
        uptime: 0,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

// MCP commands
#[tauri::command]
pub async fn get_mcps(db: State<'_, SqlitePool>) -> Result<Vec<McpResponse>> {
    let mcps = sqlx::query_as::<_, McpConfig>("SELECT * FROM mcp_configs ORDER BY id")
        .fetch_all(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    let cli_types = vec!["claude_code", "codex", "gemini"];

    let mut results = Vec::new();
    for mcp in mcps {
        // Read real status from config files
        let mut cli_flags = Vec::new();
        for cli_type in &cli_types {
            let enabled = mcp_enabled_in_file_async(db.inner(), cli_type, &mcp.name).await;
            cli_flags.push(McpCliFlag {
                cli_type: cli_type.to_string(),
                enabled,
            });
        }

        results.push(McpResponse {
            id: mcp.id,
            name: mcp.name,
            config_json: mcp.config_json,
            cli_flags,
        });
    }
    Ok(results)
}

#[tauri::command]
pub async fn get_mcp(db: State<'_, SqlitePool>, id: i64) -> Result<McpResponse> {
    let mcp = sqlx::query_as::<_, McpConfig>("SELECT * FROM mcp_configs WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "MCP not found".to_string())?;

    // Read real status from config files
    let cli_types = vec!["claude_code", "codex", "gemini"];
    let mut cli_flags = Vec::new();
    for cli_type in &cli_types {
        let enabled = mcp_enabled_in_file_async(db.inner(), cli_type, &mcp.name).await;
        cli_flags.push(McpCliFlag {
            cli_type: cli_type.to_string(),
            enabled,
        });
    }

    Ok(McpResponse {
        id: mcp.id,
        name: mcp.name,
        config_json: mcp.config_json,
        cli_flags,
    })
}

#[tauri::command]
pub async fn create_mcp(db: State<'_, SqlitePool>, input: McpCreate) -> Result<McpResponse> {
    let now = chrono::Utc::now().timestamp();

    let result = sqlx::query(
        "INSERT INTO mcp_configs (name, config_json, updated_at) VALUES (?, ?, ?)",
    )
    .bind(&input.name)
    .bind(&input.config_json)
    .bind(now)
    .execute(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    let id = result.last_insert_rowid();

    // Sync to CLI files if cli_flags provided
    let cli_flags = input.cli_flags.unwrap_or_default();
    if !cli_flags.is_empty() {
        sync_single_mcp_to_cli(db.inner(), id, &input.name, &input.config_json, &cli_flags).await?;
    }

    get_mcp(db, id).await
}

#[tauri::command]
pub async fn update_mcp(db: State<'_, SqlitePool>, id: i64, input: McpUpdate) -> Result<McpResponse> {
    let now = chrono::Utc::now().timestamp();

    let (name, config_json) = if input.name.is_some() || input.config_json.is_some() {
        let current = sqlx::query_as::<_, McpConfig>("SELECT * FROM mcp_configs WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "MCP not found".to_string())?;

        let new_name = input.name.unwrap_or(current.name.clone());
        let new_config = input.config_json.unwrap_or(current.config_json.clone());

        sqlx::query(
            "UPDATE mcp_configs SET name = ?, config_json = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&new_name)
        .bind(&new_config)
        .bind(now)
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(|e| e.to_string())?;

        (new_name, new_config)
    } else {
        // Get current values if not updating
        let current = sqlx::query_as::<_, McpConfig>("SELECT * FROM mcp_configs WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "MCP not found".to_string())?;
        (current.name, current.config_json)
    };

    // Sync to CLI files if cli_flags provided
    if let Some(cli_flags) = input.cli_flags {
        sync_single_mcp_to_cli(db.inner(), id, &name, &config_json, &cli_flags).await?;
    }

    get_mcp(db, id).await
}

#[tauri::command]
pub async fn delete_mcp(db: State<'_, SqlitePool>, id: i64) -> Result<()> {
    // Get MCP name before deletion
    let mcp = sqlx::query_as::<_, McpConfig>("SELECT * FROM mcp_configs WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "MCP not found".to_string())?;

    let mcp_name = mcp.name.clone();

    // Delete from database
    sqlx::query("DELETE FROM mcp_configs WHERE id = ?")
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    // Remove from all CLI configs
    delete_mcp_from_cli(db.inner(), &mcp_name).await?;

    Ok(())
}

// Sync a single MCP to CLI files based on enabled flags
async fn sync_single_mcp_to_cli(
    db: &SqlitePool,
    _mcp_id: i64,
    mcp_name: &str,
    mcp_config_json: &str,
    cli_flags: &[McpCliFlag],
) -> Result<()> {
    let cli_types = vec!["claude_code", "codex", "gemini"];

    for cli_type in cli_types {
        // Check if this MCP is enabled for this CLI
        let is_enabled = cli_flags.iter()
            .any(|f| f.cli_type == cli_type && f.enabled);

        let config_path = get_mcp_config_path(db, cli_type).await;
        if let Some(path) = config_path {
            // Handle Codex separately (TOML format)
            if cli_type == "codex" {
                sync_single_codex_mcp(path, mcp_name, mcp_config_json, is_enabled)?;
                continue;
            }

            // For ClaudeCode and Gemini (JSON format)
            // Read existing config or create new one
            let mut config = if path.exists() {
                let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                serde_json::from_str::<serde_json::Value>(&content).unwrap_or_else(|_| serde_json::json!({}))
            } else {
                serde_json::json!({})
            };

            // Update MCP section
            if is_enabled {
                // Add or update this MCP
                if let Ok(mcp_json) = serde_json::from_str::<serde_json::Value>(mcp_config_json) {
                    if let Some(obj) = config.as_object_mut() {
                        if !obj.contains_key("mcpServers") {
                            obj.insert("mcpServers".to_string(), serde_json::json!({}));
                        }
                        if let Some(servers) = obj.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
                            servers.insert(mcp_name.to_string(), mcp_json);
                        }
                    }
                }
            } else {
                // Remove this MCP by name
                if let Some(obj) = config.as_object_mut() {
                    if let Some(servers) = obj.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
                        servers.remove(mcp_name);
                    }
                }
            }

            // Write config file
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let config_str = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
            std::fs::write(&path, config_str).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

// Helper function to sync a single MCP to Codex config.toml
fn sync_single_codex_mcp(
    config_path: std::path::PathBuf,
    mcp_name: &str,
    mcp_config_json: &str,
    is_enabled: bool,
) -> Result<()> {
    // Read existing TOML or create new one
    let mut doc = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).map_err(|e| {
            tracing::error!("Failed to read config.toml: {}", e);
            e.to_string()
        })?;
        content.parse::<toml_edit::DocumentMut>().unwrap_or_else(|e| {
            tracing::warn!("Failed to parse config.toml, creating new: {}", e);
            toml_edit::DocumentMut::new()
        })
    } else {
        toml_edit::DocumentMut::new()
    };

    // Ensure mcp_servers table exists
    if !doc.contains_table("mcp_servers") {
        doc["mcp_servers"] = toml_edit::table();
    }

    if is_enabled {
        // Add or update this MCP
        if let Ok(mcp_config) = serde_json::from_str::<serde_json::Value>(mcp_config_json) {
            let mcp_type = mcp_config.get("type").and_then(|v| v.as_str()).unwrap_or("stdio");

            // Create MCP server table
            let mut server_table = toml_edit::Table::new();

            // Handle STDIO type servers
            if let Some(command) = mcp_config.get("command").and_then(|v| v.as_str()) {
                server_table.insert("command", toml_edit::value(command));
            }
            if let Some(args) = mcp_config.get("args").and_then(|v| v.as_array()) {
                let args_array: toml_edit::Array = args.iter()
                    .filter_map(|v| v.as_str())
                    .map(toml_edit::Value::from)
                    .collect();
                server_table.insert("args", toml_edit::Item::Value(args_array.into()));
            }
            if let Some(env) = mcp_config.get("env").and_then(|v| v.as_object()) {
                let mut env_table = toml_edit::Table::new();
                for (k, v) in env.iter() {
                    if let Some(v_str) = v.as_str() {
                        env_table.insert(k, toml_edit::value(v_str));
                    }
                }
                server_table.insert("env", toml_edit::Item::Table(env_table));
            }
            if let Some(cwd) = mcp_config.get("cwd").and_then(|v| v.as_str()) {
                server_table.insert("cwd", toml_edit::value(cwd));
            }

            // Handle HTTP/SSE type servers
            if mcp_type == "sse" || mcp_type == "http" {
                if let Some(url) = mcp_config.get("url").and_then(|v| v.as_str()) {
                    server_table.insert("url", toml_edit::value(url));
                }
            }

            // Optional fields
            if let Some(timeout) = mcp_config.get("startup_timeout_sec").and_then(|v| v.as_i64()) {
                server_table.insert("startup_timeout_sec", toml_edit::value(timeout));
            }
            if let Some(timeout) = mcp_config.get("tool_timeout_sec").and_then(|v| v.as_i64()) {
                server_table.insert("tool_timeout_sec", toml_edit::value(timeout));
            }

            doc["mcp_servers"][mcp_name] = toml_edit::Item::Table(server_table);
        }
    } else {
        // Remove this MCP by name
        if let Some(table) = doc.get_mut("mcp_servers").and_then(|v| v.as_table_mut()) {
            table.remove(mcp_name);
        }
    }

    // Write config file
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            tracing::error!("Failed to create directory: {}", e);
            e.to_string()
        })?;
    }
    std::fs::write(&config_path, doc.to_string()).map_err(|e| {
        tracing::error!("Failed to write config.toml: {}", e);
        e.to_string()
    })?;

    Ok(())
}

// Delete a single MCP from all CLI configs
async fn delete_mcp_from_cli(db: &SqlitePool, mcp_name: &str) -> Result<()> {
    let cli_types = vec!["claude_code", "codex", "gemini"];

    for cli_type in cli_types {
        let config_path = get_mcp_config_path(db, cli_type).await;
        if let Some(path) = config_path {
            if !path.exists() {
                continue;
            }

            if cli_type == "codex" {
                // Handle Codex TOML format
                let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                let mut doc = content.parse::<toml_edit::DocumentMut>().unwrap_or_else(|_| toml_edit::DocumentMut::new());

                if let Some(table) = doc["mcp_servers"].as_table_mut() {
                    table.remove(mcp_name);
                }

                std::fs::write(&path, doc.to_string()).map_err(|e| e.to_string())?;
            } else {
                // Handle Claude/Gemini JSON format
                let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                let mut config: serde_json::Value = serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}));

                if let Some(mcp_servers) = config.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
                    mcp_servers.remove(mcp_name);
                }

                let config_str = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
                std::fs::write(&path, config_str).map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(())
}

// Prompt commands
#[tauri::command]
pub async fn get_prompts(db: State<'_, SqlitePool>) -> Result<Vec<PromptResponse>> {
    let prompts = sqlx::query_as::<_, PromptPreset>("SELECT * FROM prompt_presets ORDER BY id")
        .fetch_all(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    let cli_types = vec!["claude_code", "codex", "gemini"];

    let mut results = Vec::new();
    for prompt in prompts {
        // Read real status from prompt files
        let mut cli_flags = Vec::new();
        for cli_type in &cli_types {
            let enabled = prompt_enabled_in_file_async(db.inner(), cli_type, &prompt.content).await;
            cli_flags.push(PromptCliFlag {
                cli_type: cli_type.to_string(),
                enabled,
            });
        }

        results.push(PromptResponse {
            id: prompt.id,
            name: prompt.name,
            content: prompt.content,
            cli_flags,
        });
    }
    Ok(results)
}

#[tauri::command]
pub async fn get_prompt(db: State<'_, SqlitePool>, id: i64) -> Result<PromptResponse> {
    let prompt = sqlx::query_as::<_, PromptPreset>("SELECT * FROM prompt_presets WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Prompt not found".to_string())?;

    // Read real status from prompt files
    let cli_types = vec!["claude_code", "codex", "gemini"];
    let mut cli_flags = Vec::new();
    for cli_type in &cli_types {
        let enabled = prompt_enabled_in_file_async(db.inner(), cli_type, &prompt.content).await;
        cli_flags.push(PromptCliFlag {
            cli_type: cli_type.to_string(),
            enabled,
        });
    }

    Ok(PromptResponse {
        id: prompt.id,
        name: prompt.name,
        content: prompt.content,
        cli_flags,
    })
}

#[tauri::command]
pub async fn create_prompt(db: State<'_, SqlitePool>, input: PromptCreate) -> Result<PromptResponse> {
    let now = chrono::Utc::now().timestamp();

    let result = sqlx::query(
        "INSERT INTO prompt_presets (name, content, updated_at) VALUES (?, ?, ?)",
    )
    .bind(&input.name)
    .bind(&input.content)
    .bind(now)
    .execute(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    let id = result.last_insert_rowid();

    // Sync to CLI files if cli_flags provided
    let cli_flags = input.cli_flags.unwrap_or_default();
    if !cli_flags.is_empty() {
        sync_single_prompt_to_cli(db.inner(), &input.content, &cli_flags).await?;
    }

    get_prompt(db, id).await
}

#[tauri::command]
pub async fn update_prompt(db: State<'_, SqlitePool>, id: i64, input: PromptUpdate) -> Result<PromptResponse> {
    let now = chrono::Utc::now().timestamp();

    let content = if input.name.is_some() || input.content.is_some() {
        let current = sqlx::query_as::<_, PromptPreset>("SELECT * FROM prompt_presets WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Prompt not found".to_string())?;

        let new_name = input.name.unwrap_or(current.name.clone());
        let new_content = input.content.unwrap_or(current.content.clone());

        sqlx::query(
            "UPDATE prompt_presets SET name = ?, content = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&new_name)
        .bind(&new_content)
        .bind(now)
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(|e| e.to_string())?;

        new_content
    } else {
        // Get current values if not updating
        let current = sqlx::query_as::<_, PromptPreset>("SELECT * FROM prompt_presets WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Prompt not found".to_string())?;
        current.content
    };

    // Sync to CLI files if cli_flags provided
    if let Some(cli_flags) = input.cli_flags {
        sync_single_prompt_to_cli(db.inner(), &content, &cli_flags).await?;
    }

    get_prompt(db, id).await
}

#[tauri::command]
pub async fn delete_prompt(db: State<'_, SqlitePool>, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM prompt_presets WHERE id = ?")
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    // Sync prompt configs to CLI files
    sync_prompt_configs_to_cli(db).await?;

    Ok(())
}

// Sync a single prompt to CLI files based on enabled flags
async fn sync_single_prompt_to_cli(
    db: &SqlitePool,
    prompt_content: &str,
    cli_flags: &[PromptCliFlag],
) -> Result<()> {
    let cli_types = vec!["claude_code", "codex", "gemini"];

    for cli_type in cli_types {
        // Check if this prompt is enabled for this CLI
        let is_enabled = cli_flags.iter()
            .any(|f| f.cli_type == cli_type && f.enabled);

        // Get the prompt file path for this CLI
        let prompt_path = get_prompt_file_path(db, cli_type).await;
        if let Some(path) = prompt_path {
            // Check if CLI directory exists (skip if CLI not installed)
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    continue;
                }

                if is_enabled {
                    // Write prompt content to file
                    std::fs::write(&path, prompt_content).map_err(|e| {
                        tracing::error!("Failed to write prompt file: {}", e);
                        e.to_string()
                    })?;
                } else {
                    // Check if this prompt was previously in the file
                    if path.exists() {
                        let file_content = std::fs::read_to_string(&path).unwrap_or_default();
                        if normalize_text(prompt_content) == normalize_text(&file_content) {
                            // This prompt was in the file, clear it
                            std::fs::write(&path, "").map_err(|e| {
                                tracing::error!("Failed to clear prompt file: {}", e);
                                e.to_string()
                            })?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

async fn sync_prompt_configs_to_cli(_db: State<'_, SqlitePool>) -> Result<()> {
    // This function is no longer used, keeping for compatibility
    Ok(())
}

async fn get_prompt_file_path(db: &SqlitePool, cli_type: &str) -> Option<std::path::PathBuf> {
    let config_dir = get_config_dir_from_db(db, cli_type).await;
    let base_path = std::path::PathBuf::from(config_dir);
    
    match cli_type {
        "claude_code" => Some(base_path.join("CLAUDE.md")),
        "codex" => Some(base_path.join("AGENTS.md")),
        "gemini" => Some(base_path.join("GEMINI.md")),
        _ => None,
    }
}

// Stats commands
#[tauri::command]
pub async fn get_daily_stats(
    log_db: State<'_, crate::LogDb>,
    start_date: Option<String>,
    end_date: Option<String>,
    cli_type: Option<String>,
) -> Result<Vec<DailyStats>> {
    let pool = &log_db.0;

    let mut query = "SELECT * FROM usage_daily WHERE 1=1".to_string();
    if start_date.is_some() {
        query.push_str(" AND usage_date >= ?");
    }
    if end_date.is_some() {
        query.push_str(" AND usage_date <= ?");
    }
    if cli_type.is_some() {
        query.push_str(" AND cli_type = ?");
    }
    query.push_str(" ORDER BY usage_date DESC");

    let mut q = sqlx::query_as::<_, DailyStats>(&query);
    if let Some(ref sd) = start_date {
        q = q.bind(sd);
    }
    if let Some(ref ed) = end_date {
        q = q.bind(ed);
    }
    if let Some(ref ct) = cli_type {
        q = q.bind(ct);
    }

    q.fetch_all(pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_provider_stats(
    log_db: State<'_, crate::LogDb>,
    start_date: Option<String>,
    end_date: Option<String>,
    cli_type: Option<String>,
    provider_name: Option<String>,
) -> Result<Vec<ProviderStatsResponse>> {
    let pool = &log_db.0;

    let mut query = r#"
        SELECT
            cli_type,
            provider_name,
            model_id,
            COUNT(*) as total_requests,
            SUM(CASE WHEN status_code >= 200 AND status_code < 300 THEN 1 ELSE 0 END) as total_success,
            SUM(input_tokens + output_tokens) as total_tokens,
            SUM(elapsed_ms) as total_elapsed_ms
        FROM request_logs
        WHERE 1=1
    "#.to_string();

    if start_date.is_some() {
        query.push_str(" AND datetime(created_at, 'unixepoch', 'localtime') >= ?");
    }
    if end_date.is_some() {
        query.push_str(" AND datetime(created_at, 'unixepoch', 'localtime') <= ?");
    }
    if cli_type.is_some() {
        query.push_str(" AND cli_type = ?");
    }
    if provider_name.is_some() {
        query.push_str(" AND provider_name = ?");
    }
    query.push_str(" GROUP BY cli_type, provider_name, model_id ORDER BY total_requests DESC");

    let mut q = sqlx::query_as::<_, ProviderStatsRow>(&query);
    if let Some(ref sd) = start_date {
        q = q.bind(sd);
    }
    if let Some(ref ed) = end_date {
        q = q.bind(ed);
    }
    if let Some(ref ct) = cli_type {
        q = q.bind(ct);
    }
    if let Some(ref pn) = provider_name {
        q = q.bind(pn);
    }

    let rows = q.fetch_all(pool).await.map_err(|e| e.to_string())?;

    let results = rows.into_iter().map(|row| ProviderStatsResponse {
        cli_type: row.cli_type,
        provider_name: row.provider_name,
        model_id: row.model_id,
        total_requests: row.total_requests,
        total_success: row.total_success,
        total_tokens: row.total_tokens,
        total_elapsed_ms: row.total_elapsed_ms,
        success_rate: if row.total_requests > 0 {
            (row.total_success as f64 / row.total_requests as f64) * 100.0
        } else {
            0.0
        },
    }).collect();

    Ok(results)
}

// Session helpers

/// 获取CLI基础目录（异步版本，支持自定义配置目录）
async fn get_cli_base_dir_async(db: &SqlitePool, cli_type: &str) -> std::path::PathBuf {
    get_cli_config_dir_path(db, cli_type).await
}


/// Parse Claude Code session file to extract info (first_message, git_branch, summary)
/// Returns (first_message, git_branch, summary)
fn parse_claude_session_info(file_path: &std::path::Path) -> (String, String, String) {
    use std::io::{BufRead, BufReader};
    
    let mut first_message = String::new();
    let mut git_branch = String::new();
    let mut summary = String::new();
    
    // Check file size to avoid reading very large files entirely
    let file_size = file_path.metadata().map(|m| m.len()).unwrap_or(0);
    let should_limit_read = file_size > 10 * 1024 * 1024; // 10MB
    
    let file = match std::fs::File::open(file_path) {
        Ok(f) => f,
        Err(_) => return (first_message, git_branch, summary),
    };
    
    let reader = BufReader::new(file);
    let mut lines_read = 0;
    let max_lines = if should_limit_read { 50 } else { 200 };
    
    for line in reader.lines() {
        if lines_read >= max_lines {
            break;
        }
        lines_read += 1;
        
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        
        let data: serde_json::Value = match serde_json::from_str(line) {
            Ok(d) => d,
            Err(_) => continue,
        };
        
        // Extract summary
        if data.get("type").and_then(|t| t.as_str()) == Some("summary") {
            if let Some(s) = data.get("summary").and_then(|s| s.as_str()) {
                summary = s.to_string();
            }
        }
        
        // Extract git branch
        if git_branch.is_empty() {
            if let Some(branch) = data.get("gitBranch").and_then(|b| b.as_str()) {
                git_branch = branch.to_string();
            }
        }
        
        // Extract first message from user type
        if first_message.is_empty() && data.get("type").and_then(|t| t.as_str()) == Some("user") {
            if let Some(message) = data.get("message") {
                if let Some(content) = message.get("content") {
                    let text = if let Some(content_str) = content.as_str() {
                        // content is a string
                        if content_str != "Warmup" {
                            content_str.chars().take(200).collect::<String>()
                        } else {
                            String::new()
                        }
                    } else if let Some(content_arr) = content.as_array() {
                        // content is an array of items
                        let mut text_parts = Vec::new();
                        for item in content_arr {
                            if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                                if let Some(t) = item.get("text").and_then(|t| t.as_str()) {
                                    text_parts.push(t);
                                }
                            }
                        }
                        let joined = text_parts.join("\n");
                        if !joined.is_empty() && joined != "Warmup" {
                            joined.chars().take(200).collect::<String>()
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };
                    
                    if !text.is_empty() {
                        first_message = text;
                    }
                }
            }
        }
    }
    
    (first_message, git_branch, summary)
}

/// Decode Claude Code project name to (display_name, full_path)
/// Format: D--my-develop-project-other -> ("other", "D:\\my-develop\\project\\other")
fn decode_claude_project_name(encoded_name: &str) -> (String, String) {
    #[cfg(target_os = "windows")]
    {
        // Windows format: D--path-parts (drive letter + double dash + path with single dashes)
        if encoded_name.len() >= 3 && encoded_name.chars().nth(1) == Some('-') && encoded_name.chars().nth(2) == Some('-') {
            let drive = encoded_name.chars().next().unwrap().to_uppercase().to_string();
            let path_part = &encoded_name[3..]; // Skip "D--"
            let path_parts: Vec<&str> = path_part.split('-').collect();
            let full_path = format!("{}:\\{}", drive, path_parts.join("\\"));
            let display_name = path_parts.last().unwrap_or(&encoded_name).to_string();
            return (display_name, full_path);
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Unix format: starts with - then path parts separated by -
        if encoded_name.starts_with("-") {
            let parts: Vec<&str> = encoded_name[1..].split('-').collect();
            let full_path = format!("/{}", parts.join("/"));
            let display_name = parts.last().unwrap_or(&encoded_name).to_string();
            return (display_name, full_path);
        }
    }
    (encoded_name.to_string(), encoded_name.to_string())
}

// Extract cwd from Codex session file
fn extract_codex_cwd(file_path: &std::path::Path) -> Option<String> {
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(file_path).ok()?;
    let reader = BufReader::new(file);
    
    for line in reader.lines().flatten() {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&line) {
            if data.get("type").and_then(|t| t.as_str()) == Some("session_meta") {
                if let Some(cwd) = data.get("payload")
                    .and_then(|p| p.get("cwd"))
                    .and_then(|c| c.as_str()) {
                    return Some(cwd.to_string());
                }
            }
        }
    }
    None
}

// Handle Codex projects (group sessions by cwd)
fn get_codex_projects(sessions_dir: std::path::PathBuf, page: i64, page_size: i64) -> Result<PaginatedProjects> {
    use std::collections::HashMap;
    use walkdir::WalkDir;
    
    if !sessions_dir.exists() {
        return Ok(PaginatedProjects {
            items: vec![],
            total: 0,
            page,
            page_size,
        });
    }
    
    // Group sessions by cwd (search recursively in date subdirectories)
    let mut project_map: HashMap<String, Vec<(std::path::PathBuf, std::fs::Metadata)>> = HashMap::new();
    
    // Use WalkDir to recursively search all subdirectories
    for entry in WalkDir::new(&sessions_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            
            if filename.starts_with("rollout-") && filename.ends_with(".jsonl") {
                if let Some(cwd) = extract_codex_cwd(path) {
                    if let Ok(meta) = path.metadata() {
                        project_map.entry(cwd).or_insert_with(Vec::new).push((path.to_path_buf(), meta));
                    }
                }
            }
        }
    }
    
    // Build project list
    let mut projects_data: Vec<(String, String, usize, i64, f64)> = Vec::new();
    for (cwd, files) in project_map {
        let total_size: i64 = files.iter().map(|(_, m)| m.len() as i64).sum();
        let last_modified = files.iter()
            .filter_map(|(_, m)| m.modified().ok())
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0))
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);
        
        let display_name = std::path::Path::new(&cwd)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();
        
        projects_data.push((cwd.clone(), display_name, files.len(), total_size, last_modified));
    }
    
    // Sort by last_modified descending
    projects_data.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap_or(std::cmp::Ordering::Equal));
    
    let total = projects_data.len() as i64;
    let start = ((page - 1) * page_size) as usize;
    let items: Vec<_> = projects_data.into_iter()
        .skip(start)
        .take(page_size as usize)
        .map(|(cwd, display_name, session_count, total_size, last_modified)| ProjectInfo {
            name: cwd.clone(),
            display_name,
            full_path: cwd,
            session_count: session_count as i64,
            total_size,
            last_modified,
        })
        .collect();
    
    Ok(PaginatedProjects {
        items,
        total,
        page,
        page_size,
    })
}

/// Calculate SHA256 hash of a path (same as Gemini CLI)
fn get_path_hash(path: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Build hash -> path mapping for Gemini projects using rainbow table method
fn build_gemini_path_mapping(target_hashes: &std::collections::HashSet<String>) -> std::collections::HashMap<String, String> {
    use std::collections::HashMap;
    
    let mut results: HashMap<String, String> = HashMap::new();
    let home = dirs::home_dir().unwrap_or_default();
    
    // Define search paths with max depth
    let mut search_paths: Vec<(std::path::PathBuf, usize)> = vec![
        (home.clone(), 0),
        (home.join("Desktop"), 4),
        (home.join("Documents"), 4),
        (home.join("Downloads"), 3),
        (home.join("Projects"), 4),
        (home.join("Code"), 4),
        (home.join("workspace"), 4),
        (home.join("dev"), 4),
        (home.join("src"), 4),
        (home.join("work"), 4),
        (home.join("repos"), 4),
        (home.join("github"), 4),
    ];
    
    // Windows specific paths
    #[cfg(target_os = "windows")]
    {
        for drive in ["C:", "D:", "E:", "F:"] {
            let drive_path = std::path::PathBuf::from(format!("{}\\" , drive));
            if drive_path.exists() {
                search_paths.extend(vec![
                    (drive_path.join("Projects"), 4),
                    (drive_path.join("Code"), 4),
                    (drive_path.join("workspace"), 4),
                    (drive_path.join("dev"), 4),
                    (drive_path.join("my-develop"), 5),
                ]);
            }
        }
    }
    
    fn scan_dir(
        dir_path: &std::path::Path,
        max_depth: usize,
        current_depth: usize,
        target_hashes: &std::collections::HashSet<String>,
        results: &mut std::collections::HashMap<String, String>,
    ) {
        if current_depth > max_depth || results.len() >= target_hashes.len() {
            return;
        }
        
        // Calculate hash for current directory
        let path_str = dir_path.to_string_lossy().to_string();
        let path_hash = get_path_hash(&path_str);
        if target_hashes.contains(&path_hash) && !results.contains_key(&path_hash) {
            results.insert(path_hash, path_str);
        }
        
        if results.len() >= target_hashes.len() {
            return;
        }
        
        // Scan subdirectories
        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let item_path = entry.path();
                if !item_path.is_dir() {
                    continue;
                }
                
                let name = item_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                // Skip hidden and common irrelevant directories
                if name.starts_with('.') || 
                   name == "node_modules" || 
                   name == "venv" || 
                   name == "__pycache__" ||
                   name == "Library" ||
                   name == "Applications" ||
                   name == "target" ||
                   name == "dist" ||
                   name == "build" {
                    continue;
                }
                
                scan_dir(&item_path, max_depth, current_depth + 1, target_hashes, results);
                if results.len() >= target_hashes.len() {
                    return;
                }
            }
        }
    }
    
    for (search_path, depth) in search_paths {
        if search_path.exists() {
            scan_dir(&search_path, depth, 0, target_hashes, &mut results);
        }
        if results.len() >= target_hashes.len() {
            break;
        }
    }
    
    results
}

// Handle Gemini projects (from hash directories with chats subfolder)
fn get_gemini_projects(tmp_dir: std::path::PathBuf, page: i64, page_size: i64) -> Result<PaginatedProjects> {
    use std::collections::HashSet;
    
    if !tmp_dir.exists() {
        return Ok(PaginatedProjects {
            items: vec![],
            total: 0,
            page,
            page_size,
        });
    }
    
    let mut project_dirs: Vec<(std::path::PathBuf, f64)> = Vec::new();
    let mut all_hashes: HashSet<String> = HashSet::new();
    
    if let Ok(entries) = std::fs::read_dir(&tmp_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            
            // Check if it's a valid 64-char hex hash
            if name.len() == 64 && name.chars().all(|c| c.is_ascii_hexdigit()) {
                let chats_dir = path.join("chats");
                if chats_dir.exists() {
                    if let Ok(meta) = path.metadata() {
                        if let Ok(mtime) = meta.modified() {
                            let secs = mtime.duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs_f64())
                                .unwrap_or(0.0);
                            all_hashes.insert(name.clone());
                            project_dirs.push((path, secs));
                        }
                    }
                }
            }
        }
    }
    
    // Sort by last_modified descending
    project_dirs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    
    let total = project_dirs.len() as i64;
    let start = ((page - 1) * page_size) as usize;
    let page_dirs: Vec<_> = project_dirs.into_iter().skip(start).take(page_size as usize).collect();
    
    // Build path mapping using rainbow table method
    let path_mapping = build_gemini_path_mapping(&all_hashes);
    
    let mut projects = Vec::new();
    for (path, _) in page_dirs {
        let hash_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        
        let chats_dir = path.join("chats");
        let mut session_count = 0i64;
        let mut total_size = 0i64;
        let mut last_modified = 0f64;
        
        if let Ok(entries) = std::fs::read_dir(&chats_dir) {
            for entry in entries.flatten() {
                let session_path = entry.path();
                if session_path.is_file() {
                    let filename = session_path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    
                    if filename.starts_with("session-") && filename.ends_with(".json") {
                        session_count += 1;
                        if let Ok(meta) = session_path.metadata() {
                            total_size += meta.len() as i64;
                            if let Ok(mtime) = meta.modified() {
                                let secs = mtime.duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_secs_f64())
                                    .unwrap_or(0.0);
                                if secs > last_modified {
                                    last_modified = secs;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        if session_count > 0 {
            // Try to get project path from rainbow table
            let (display_name, full_path) = if let Some(real_path) = path_mapping.get(hash_name) {
                let name = std::path::Path::new(real_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&format!("Project {}", &hash_name[..8]))
                    .to_string();
                (name, real_path.clone())
            } else {
                (format!("Project {}", &hash_name[..8]), hash_name.to_string())
            };
            
            projects.push(ProjectInfo {
                name: hash_name.to_string(),
                display_name,
                full_path,
                session_count,
                total_size,
                last_modified,
            });
        }
    }
    
    Ok(PaginatedProjects {
        items: projects,
        total,
        page,
        page_size,
    })
}

// Handle Codex sessions (find by cwd) - 异步版本，支持自定义配置目录
async fn get_codex_sessions_async(db: &SqlitePool, project_name: &str, page: i64, page_size: i64) -> Result<PaginatedSessions> {
    use std::io::{BufRead, BufReader};
    use walkdir::WalkDir;
    
    let config_dir = get_cli_config_dir_path(db, "codex").await;
    let sessions_dir = config_dir.join("sessions");
    
    if !sessions_dir.exists() {
        return Ok(PaginatedSessions {
            items: vec![],
            total: 0,
            page,
            page_size,
        });
    }
    
    let mut session_files: Vec<(std::path::PathBuf, std::fs::Metadata)> = Vec::new();
    
    // Use WalkDir to recursively search all subdirectories
    for entry in WalkDir::new(&sessions_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            
            if filename.starts_with("rollout-") && filename.ends_with(".jsonl") {
                if let Some(cwd) = extract_codex_cwd(path) {
                    if cwd == project_name {
                        if let Ok(meta) = path.metadata() {
                            session_files.push((path.to_path_buf(), meta));
                        }
                    }
                }
            }
        }
    }
    
    // Sort by mtime descending
    session_files.sort_by(|a, b| {
        let a_mtime = a.1.modified().ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        let b_mtime = b.1.modified().ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        b_mtime.partial_cmp(&a_mtime).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    let total = session_files.len() as i64;
    let start = ((page - 1) * page_size) as usize;
    let page_files: Vec<_> = session_files.into_iter().skip(start).take(page_size as usize).collect();
    
    let mut sessions = Vec::new();
    for (path, meta) in page_files {
        let session_id = path.file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        
        let size = meta.len() as i64;
        let mtime = meta.modified().ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        
        // Try to extract first message
        let mut first_message = String::new();
        if let Ok(file) = std::fs::File::open(&path) {
            let reader = BufReader::new(file);
            for line in reader.lines().flatten() {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&line) {
                    if data.get("type").and_then(|t| t.as_str()) == Some("event_msg") {
                        if let Some(payload) = data.get("payload") {
                            if payload.get("type").and_then(|t| t.as_str()) == Some("user_message") {
                                if let Some(msg) = payload.get("message").and_then(|m| m.as_str()) {
                                    first_message = msg.chars().take(200).collect();
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        sessions.push(SessionInfo {
            session_id,
            size,
            mtime,
            first_message,
            git_branch: String::new(),
            summary: String::new(),
        });
    }
    
    Ok(PaginatedSessions {
        items: sessions,
        total,
        page,
        page_size,
    })
}

// Handle Gemini sessions - 异步版本，支持自定义配置目录
async fn get_gemini_sessions_async(db: &SqlitePool, project_name: &str, page: i64, page_size: i64) -> Result<PaginatedSessions> {
    let config_dir = get_cli_config_dir_path(db, "gemini").await;
    let chats_dir = config_dir.join("tmp").join(project_name).join("chats");
    
    if !chats_dir.exists() {
        return Ok(PaginatedSessions {
            items: vec![],
            total: 0,
            page,
            page_size,
        });
    }
    
    let mut session_files: Vec<(std::path::PathBuf, std::fs::Metadata)> = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(&chats_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let filename = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                if filename.starts_with("session-") && filename.ends_with(".json") {
                    if let Ok(meta) = path.metadata() {
                        session_files.push((path, meta));
                    }
                }
            }
        }
    }
    
    // Sort by mtime descending
    session_files.sort_by(|a, b| {
        let a_mtime = a.1.modified().ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        let b_mtime = b.1.modified().ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        b_mtime.partial_cmp(&a_mtime).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    let total = session_files.len() as i64;
    let start = ((page - 1) * page_size) as usize;
    let page_files: Vec<_> = session_files.into_iter().skip(start).take(page_size as usize).collect();
    
    let mut sessions = Vec::new();
    for (path, meta) in page_files {
        let session_id = path.file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        
        let size = meta.len() as i64;
        let mtime = meta.modified().ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        
        // Try to extract first message
        let mut first_message = String::new();
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(messages) = json.get("messages").and_then(|m| m.as_array()) {
                    for msg in messages {
                        if msg.get("type").and_then(|t| t.as_str()) == Some("user") {
                            if let Some(text) = msg.get("content").and_then(|c| c.as_str()) {
                                first_message = text.chars().take(200).collect();
                                break;
                            }
                        }
                    }
                }
            }
        }
        
        sessions.push(SessionInfo {
            session_id,
            size,
            mtime,
            first_message,
            git_branch: String::new(),
            summary: String::new(),
        });
    }
    
    Ok(PaginatedSessions {
        items: sessions,
        total,
        page,
        page_size,
    })
}

// Parse Codex messages from JSONL file - 异步版本，支持自定义配置目录
async fn get_codex_messages_async(db: &SqlitePool, session_id: &str) -> Result<Vec<SessionMessage>> {
    use std::io::{BufRead, BufReader};
    use walkdir::WalkDir;
    
    let config_dir = get_cli_config_dir_path(db, "codex").await;
    let sessions_dir = config_dir.join("sessions");
    
    // Find the session file by searching recursively
    let mut session_file_path: Option<std::path::PathBuf> = None;
    for entry in WalkDir::new(&sessions_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            // Match session_id which is the stem (filename without extension)
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if stem == session_id {
                    session_file_path = Some(path.to_path_buf());
                    break;
                }
            }
        }
    }
    
    let session_file = session_file_path.ok_or_else(|| format!("Session file not found: {}", session_id))?;
    
    let file = std::fs::File::open(&session_file)
        .map_err(|e| format!("Failed to open session file: {}", e))?;
    let reader = BufReader::new(file);
    
    let mut messages = Vec::new();
    
    for line in reader.lines().flatten() {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&line) {
            let msg_type = data.get("type").and_then(|t| t.as_str());
            
            // Only process response_item for structured messages
            if msg_type == Some("response_item") {
                if let Some(payload) = data.get("payload") {
                    let item_type = payload.get("type").and_then(|t| t.as_str());
                    let role = payload.get("role").and_then(|r| r.as_str());
                    let timestamp = data.get("timestamp").and_then(|t| t.as_i64());
                    
                    // User messages
                    if role == Some("user") && item_type == Some("message") {
                        if let Some(content_list) = payload.get("content").and_then(|c| c.as_array()) {
                            let text_parts: Vec<String> = content_list.iter()
                                .filter_map(|item| {
                                    if item.get("type").and_then(|t| t.as_str()) == Some("input_text") {
                                        item.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            if !text_parts.is_empty() {
                                messages.push(SessionMessage {
                                    role: "user".to_string(),
                                    content: text_parts.join("\n\n"),
                                    timestamp,
                                });
                            }
                        }
                    }
                    // Assistant messages
                    else if role == Some("assistant") && item_type == Some("message") {
                        if let Some(content_list) = payload.get("content").and_then(|c| c.as_array()) {
                            let text_parts: Vec<String> = content_list.iter()
                                .filter_map(|item| {
                                    let item_type = item.get("type").and_then(|t| t.as_str());
                                    if item_type == Some("output_text") || item_type == Some("text") {
                                        item.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            if !text_parts.is_empty() {
                                messages.push(SessionMessage {
                                    role: "assistant".to_string(),
                                    content: text_parts.join("\n\n"),
                                    timestamp,
                                });
                            }
                        }
                    }
                    // Reasoning summary
                    else if item_type == Some("reasoning") {
                        let summary = payload.get("summary").and_then(|s| s.as_array());
                        if let Some(summary_arr) = summary {
                            let text_parts: Vec<String> = summary_arr.iter()
                                .filter_map(|item| {
                                    if item.get("type").and_then(|t| t.as_str()) == Some("summary_text") {
                                        item.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            if !text_parts.is_empty() {
                                messages.push(SessionMessage {
                                    role: "assistant".to_string(),
                                    content: format!("**[推理]**\n{}", text_parts.join("\n")),
                                    timestamp,
                                });
                            }
                        }
                    }
                    // Function call (tool use)
                    else if item_type == Some("function_call") {
                        let name = payload.get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("unknown");
                        let arguments = payload.get("arguments")
                            .and_then(|a| a.as_str())
                            .unwrap_or("{}");
                        let args_str = match serde_json::from_str::<serde_json::Value>(arguments) {
                            Ok(args_obj) => serde_json::to_string_pretty(&args_obj).unwrap_or_else(|_| arguments.to_string()),
                            Err(_) => arguments.to_string(),
                        };
                        messages.push(SessionMessage {
                            role: "assistant".to_string(),
                            content: format!("**[调用工具: {}]**\n```json\n{}\n```", name, args_str),
                            timestamp,
                        });
                    }
                    // Function call output (tool result)
                    else if item_type == Some("function_call_output") {
                        let output = payload.get("output")
                            .and_then(|o| o.as_str())
                            .unwrap_or("");
                        if !output.is_empty() {
                            messages.push(SessionMessage {
                                role: "user".to_string(),
                                content: format!("**[工具结果]**\n```\n{}\n```", output),
                                timestamp,
                            });
                        }
                    }
                }
            }
        }
    }
    
    Ok(messages)
}

// Parse Claude Code messages from JSONL content
fn parse_claude_jsonl(content: &str) -> Result<Vec<SessionMessage>> {
    use std::io::{BufRead, BufReader};
    
    let mut messages = Vec::new();
    let reader = BufReader::new(content.as_bytes());
    
    for line in reader.lines().flatten() {
        if line.trim().is_empty() {
            continue;
        }
        
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&line) {
            let msg_type = data.get("type").and_then(|t| t.as_str());
            
            if msg_type == Some("user") || msg_type == Some("assistant") {
                let role = msg_type.unwrap();
                let timestamp = data.get("timestamp").and_then(|t| t.as_i64());
                
                if let Some(message) = data.get("message") {
                    let content_val = message.get("content");
                    
                    let content = if let Some(arr) = content_val.and_then(|c| c.as_array()) {
                        let mut text_parts = Vec::new();
                        for item in arr {
                            if let Some(item_type) = item.get("type").and_then(|t| t.as_str()) {
                                match item_type {
                                    "text" => {
                                        if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                                            text_parts.push(text.to_string());
                                        }
                                    }
                                    "tool_use" if role == "assistant" => {
                                        // Tool call from assistant
                                        let tool_name = item.get("name")
                                            .and_then(|n| n.as_str())
                                            .unwrap_or("unknown");
                                        let tool_input = item.get("input");
                                        let input_str = if let Some(input) = tool_input {
                                            serde_json::to_string_pretty(input).unwrap_or_else(|_| "{}".to_string())
                                        } else {
                                            "{}".to_string()
                                        };
                                        text_parts.push(format!("**[调用工具: {}]**\n```json\n{}\n```", tool_name, input_str));
                                    }
                                    "tool_result" if role == "user" => {
                                        // Tool result from user
                                        let result_content = item.get("content");
                                        let result_str = if let Some(content) = result_content {
                                            if let Some(s) = content.as_str() {
                                                s.to_string()
                                            } else {
                                                serde_json::to_string_pretty(content).unwrap_or_else(|_| "".to_string())
                                            }
                                        } else {
                                            String::new()
                                        };
                                        if !result_str.is_empty() {
                                            text_parts.push(format!("**[工具结果]**\n```\n{}\n```", result_str));
                                        }
                                    }
                                    "thinking" if role == "assistant" => {
                                        // Thinking from assistant
                                        if let Some(thinking) = item.get("thinking").and_then(|t| t.as_str()) {
                                            if !thinking.is_empty() {
                                                text_parts.push(format!("**[思考]**\n{}", thinking));
                                            }
                                        }
                                    }
                                    "image" => {
                                        text_parts.push("[图片]".to_string());
                                    }
                                    _ => {}
                                }
                            }
                        }
                        text_parts.join("\n\n")
                    } else if let Some(text) = content_val.and_then(|c| c.as_str()) {
                        text.to_string()
                    } else {
                        continue;
                    };
                    
                    if !content.is_empty() && content != "Warmup" {
                        messages.push(SessionMessage {
                            role: role.to_string(),
                            content,
                            timestamp,
                        });
                    }
                }
            }
        }
    }
    
    Ok(messages)
}

// Session commands
#[tauri::command]
pub async fn get_session_projects(
    db: State<'_, SqlitePool>,
    cli_type: String,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<PaginatedProjects> {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).clamp(1, 100);

    let base_dir = get_cli_base_dir_async(db.inner(), &cli_type).await;
    let projects_dir = match cli_type.as_str() {
        "codex" => base_dir.join("sessions"),
        "gemini" => base_dir.join("tmp"),
        _ => base_dir.join("projects"),
    };

    // For Codex, we need special handling since sessions are not in project folders
    if cli_type == "codex" {
        return get_codex_projects(projects_dir, page, page_size);
    }

    // For Gemini, check if sessions are in hash directories with chats subfolder
    if cli_type == "gemini" {
        return get_gemini_projects(projects_dir, page, page_size);
    }

    let mut projects = Vec::new();

    if projects_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();

                    if name.is_empty() || name.starts_with('.') {
                        continue;
                    }

                    // Count sessions and calculate size
                    let mut session_count = 0i64;
                    let mut total_size = 0i64;
                    let mut last_modified = 0f64;

                    if let Ok(sessions) = std::fs::read_dir(&path) {
                        for session in sessions.flatten() {
                            let session_path = session.path();
                            if session_path.is_file() {
                                // Only count .jsonl files, exclude index and agent files
                                let ext = session_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                                if ext != "jsonl" {
                                    continue;
                                }
                                let stem = session_path.file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("");
                                if stem == "sessions-index" || stem.starts_with("agent-") {
                                    continue;
                                }
                                
                                session_count += 1;
                                if let Ok(meta) = session_path.metadata() {
                                    total_size += meta.len() as i64;
                                    if let Ok(mtime) = meta.modified() {
                                        let secs = mtime.duration_since(std::time::UNIX_EPOCH)
                                            .map(|d| d.as_secs_f64())
                                            .unwrap_or(0.0);
                                        if secs > last_modified {
                                            last_modified = secs;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let (display_name, full_path) = if cli_type == "claude_code" {
                        // Decode path from project name (format: -D-my-develop-project-other)
                        decode_claude_project_name(&name)
                    } else {
                        (name.clone(), path.to_string_lossy().to_string())
                    };

                    projects.push(ProjectInfo {
                        name: name.clone(),
                        display_name,
                        full_path,
                        session_count,
                        total_size,
                        last_modified,
                    });
                }
            }
        }
    }

    // Sort by last_modified descending
    projects.sort_by(|a, b| b.last_modified.partial_cmp(&a.last_modified).unwrap_or(std::cmp::Ordering::Equal));

    let total = projects.len() as i64;
    let start = ((page - 1) * page_size) as usize;
    let items: Vec<_> = projects.into_iter().skip(start).take(page_size as usize).collect();

    Ok(PaginatedProjects {
        items,
        total,
        page,
        page_size,
    })
}

#[tauri::command]
pub async fn get_project_sessions(
    db: State<'_, SqlitePool>,
    cli_type: String,
    project_name: String,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<PaginatedSessions> {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).clamp(1, 100);

    // Special handling for Codex
    if cli_type == "codex" {
        return get_codex_sessions_async(db.inner(), &project_name, page, page_size).await;
    }

    // Special handling for Gemini
    if cli_type == "gemini" {
        return get_gemini_sessions_async(db.inner(), &project_name, page, page_size).await;
    }

    // Claude Code default handling
    let base_dir = get_cli_base_dir_async(db.inner(), &cli_type).await;
    let project_dir = base_dir.join("projects").join(&project_name);

    let mut sessions = Vec::new();

    if project_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&project_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    // Only process .jsonl files
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if ext != "jsonl" {
                        continue;
                    }
                    
                    let session_id = path.file_stem()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();

                    // Skip empty, index files, and agent files
                    if session_id.is_empty() 
                        || session_id == "sessions-index" 
                        || session_id.starts_with("agent-") {
                        continue;
                    }

                    let mut size = 0i64;
                    let mut mtime = 0f64;

                    if let Ok(meta) = path.metadata() {
                        size = meta.len() as i64;
                        if let Ok(mt) = meta.modified() {
                            mtime = mt.duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs_f64())
                                .unwrap_or(0.0);
                        }
                    }

                    // Try to read first message from JSONL (Claude Code uses JSONL format)
                    let (first_message, git_branch, _) = parse_claude_session_info(&path);

                    sessions.push(SessionInfo {
                        session_id,
                        size,
                        mtime,
                        first_message,
                        git_branch,
                        summary: String::new(),
                    });
                }
            }
        }
    }

    // Sort by mtime descending
    sessions.sort_by(|a, b| b.mtime.partial_cmp(&a.mtime).unwrap_or(std::cmp::Ordering::Equal));

    let total = sessions.len() as i64;
    let start = ((page - 1) * page_size) as usize;
    let items: Vec<_> = sessions.into_iter().skip(start).take(page_size as usize).collect();

    Ok(PaginatedSessions {
        items,
        total,
        page,
        page_size,
    })
}

#[tauri::command]
pub async fn get_session_messages(
    db: State<'_, SqlitePool>,
    cli_type: String,
    project_name: String,
    session_id: String,
) -> Result<Vec<SessionMessage>> {
    // Special handling for Codex JSONL format
    if cli_type == "codex" {
        return get_codex_messages_async(db.inner(), &session_id).await;
    }
    
    let base_dir = get_cli_base_dir_async(db.inner(), &cli_type).await;
    let session_file = match cli_type.as_str() {
        "gemini" => base_dir.join("tmp").join(&project_name).join("chats").join(format!("{}.json", session_id)),
        _ => base_dir.join("projects").join(&project_name).join(format!("{}.jsonl", session_id)),
    };

    let content = std::fs::read_to_string(&session_file)
        .map_err(|e| format!("Failed to read session file: {}", e))?;

    // For Claude Code JSONL format
    if cli_type == "claude_code" {
        return parse_claude_jsonl(&content);
    }
    
    // For Gemini JSON format
    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse session JSON: {}", e))?;

    let mut messages = Vec::new();

    // Try to parse messages in different formats
    if let Some(msgs) = json.get("messages").and_then(|m| m.as_array()) {
        // Standard format with messages array
        for msg in msgs {
            let msg_type = msg.get("type").and_then(|t| t.as_str()).unwrap_or("");
            
            let timestamp = msg.get("timestamp").and_then(|t| t.as_str()).map(|s| {
                chrono::DateTime::parse_from_rfc3339(s)
                    .ok()
                    .map(|dt| dt.timestamp())
            }).flatten();
            
            if msg_type == "user" {
                // User message
                let content = if let Some(content_val) = msg.get("content") {
                    if let Some(text) = content_val.as_str() {
                        text.to_string()
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };
                
                if !content.is_empty() {
                    messages.push(SessionMessage {
                        role: "user".to_string(),
                        content,
                        timestamp,
                    });
                }
            } else if msg_type == "gemini" || msg_type == "assistant" || msg_type == "ai" {
                // Gemini/Assistant message - may contain content, thoughts, and toolCalls
                let mut text_parts = Vec::new();
                
                // Get main content
                if let Some(content_val) = msg.get("content") {
                    if let Some(text) = content_val.as_str() {
                        if !text.is_empty() {
                            text_parts.push(text.to_string());
                        }
                    }
                }
                
                // Handle thoughts
                if let Some(thoughts) = msg.get("thoughts").and_then(|t| t.as_array()) {
                    for thought in thoughts {
                        if let Some(desc) = thought.get("description").and_then(|d| d.as_str()) {
                            if !desc.is_empty() {
                                text_parts.push(format!("**[思考]**\n{}", desc));
                            }
                        }
                    }
                }
                
                // Handle tool calls
                if let Some(tool_calls) = msg.get("toolCalls").and_then(|t| t.as_array()) {
                    for tool_call in tool_calls {
                        let tool_name = tool_call.get("displayName")
                            .or_else(|| tool_call.get("name"))
                            .and_then(|n| n.as_str())
                            .unwrap_or("unknown");
                        let result_display = tool_call.get("resultDisplay")
                            .and_then(|r| r.as_str())
                            .unwrap_or("");
                        if !result_display.is_empty() {
                            text_parts.push(format!("**[工具: {}]**\n{}", tool_name, result_display));
                        }
                    }
                }
                
                let final_content = text_parts.join("\n\n");
                if !final_content.is_empty() {
                    messages.push(SessionMessage {
                        role: "assistant".to_string(),
                        content: final_content,
                        timestamp,
                    });
                }
            }
        }
    } else if let Some(conversation) = json.as_object() {
        // Try to parse as flat object with role-based keys
        for (key, value) in conversation {
            if key == "id" || key == "title" || key == "created_at" || key == "updated_at" {
                continue;
            }
            let role = if key.starts_with("user") || key.starts_with("human") {
                "user"
            } else if key.starts_with("assistant") || key.starts_with("ai") {
                "assistant"
            } else {
                continue;
            };

            if let Some(text) = value.as_str() {
                messages.push(SessionMessage {
                    role: role.to_string(),
                    content: text.to_string(),
                    timestamp: None,
                });
            }
        }
    }

    Ok(messages)
}

#[tauri::command]
pub async fn delete_session(
    db: State<'_, SqlitePool>,
    cli_type: String,
    project_name: String,
    session_id: String,
) -> Result<()> {
    let base_dir = get_cli_base_dir_async(db.inner(), &cli_type).await;
    
    // Special handling for Codex - need to search recursively
    if cli_type == "codex" {
        use walkdir::WalkDir;
        let sessions_dir = base_dir.join("sessions");
        for entry in WalkDir::new(&sessions_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if stem == session_id {
                        // Verify the cwd matches project_name
                        if let Some(cwd) = extract_codex_cwd(path) {
                            if cwd == project_name {
                                std::fs::remove_file(path)
                                    .map_err(|e| format!("Failed to delete session: {}", e))?;
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }
        return Err("Session file not found".to_string());
    }
    
    let session_file = match cli_type.as_str() {
        "gemini" => base_dir.join("tmp").join(&project_name).join("chats").join(format!("{}.json", session_id)),
        _ => base_dir.join("projects").join(&project_name).join(format!("{}.jsonl", session_id)),
    };

    if !session_file.exists() {
        return Err(format!("Session file not found: {}", session_file.display()));
    }

    std::fs::remove_file(&session_file)
        .map_err(|e| format!("Failed to delete session '{}': {}", session_file.display(), e))?;

    Ok(())
}

#[tauri::command]
pub async fn delete_project(
    db: State<'_, SqlitePool>,
    cli_type: String,
    project_name: String,
) -> Result<()> {
    let base_dir = get_cli_base_dir_async(db.inner(), &cli_type).await;
    
    if cli_type == "codex" {
        // For Codex, delete all session files matching the project cwd
        use walkdir::WalkDir;
        let sessions_dir = base_dir.join("sessions");
        if sessions_dir.exists() {
            // Use WalkDir to recursively search all subdirectories
            for entry in WalkDir::new(&sessions_dir)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.is_file() {
                    let filename = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    if filename.starts_with("rollout-") && filename.ends_with(".jsonl") {
                        if let Some(cwd) = extract_codex_cwd(path) {
                            if cwd == project_name {
                                let _ = std::fs::remove_file(path);
                            }
                        }
                    }
                }
            }
        }
        return Ok(());
    }
    
    // For Claude Code and Gemini, delete the project directory
    let project_dir = match cli_type.as_str() {
        "gemini" => base_dir.join("tmp").join(&project_name),
        _ => base_dir.join("projects").join(&project_name),
    };

    std::fs::remove_dir_all(&project_dir)
        .map_err(|e| format!("Failed to delete project: {}", e))?;

    Ok(())
}

/// 退出应用程序（导入后需要手动重启）
async fn exit_application() -> Result<()> {
    tokio::spawn(async {
        // 延迟 3 秒，等待响应返回前端并给用户时间看提示
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        std::process::exit(0);
    });

    Ok(())
}

// Backup commands
#[tauri::command]
pub async fn get_webdav_settings(db: State<'_, SqlitePool>) -> Result<WebdavSettings> {
    // Try to get existing settings
    let settings = sqlx::query_as::<_, WebdavSettings>(
        "SELECT url, username, password FROM webdav_settings WHERE id = 1"
    )
    .fetch_optional(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    match settings {
        Some(s) => Ok(s),
        None => {
            // Create default settings
            let now = chrono::Utc::now().timestamp();
            sqlx::query(
                "INSERT INTO webdav_settings (id, url, username, password, updated_at) VALUES (1, '', '', '', ?)"
            )
            .bind(now)
            .execute(db.inner())
            .await
            .map_err(|e| e.to_string())?;

            Ok(WebdavSettings {
                url: String::new(),
                username: String::new(),
                password: String::new(),
            })
        }
    }
}

#[tauri::command]
pub async fn update_webdav_settings(
    db: State<'_, SqlitePool>,
    input: WebdavSettingsUpdate,
) -> Result<WebdavSettings> {
    let now = chrono::Utc::now().timestamp();
    let current = get_webdav_settings(db.clone()).await?;

    sqlx::query(
        "UPDATE webdav_settings SET url = ?, username = ?, password = ?, updated_at = ? WHERE id = 1"
    )
    .bind(input.url.unwrap_or(current.url))
    .bind(input.username.unwrap_or(current.username))
    .bind(input.password.unwrap_or(current.password))
    .bind(now)
    .execute(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    get_webdav_settings(db).await
}

#[tauri::command]
pub async fn test_webdav_connection(
    url: String,
    username: String,
    password: String,
) -> Result<bool> {
    use reqwest::Client;

    let client = Client::new();
    let response = client
        .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
        .basic_auth(&username, Some(&password))
        .header("Depth", "0")
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    Ok(response.status().is_success() || response.status().as_u16() == 207)
}

#[tauri::command]
pub async fn export_to_local() -> Result<Vec<u8>> {
    // Get the database path from config
    let db_path = get_data_dir().join("ccg_gateway.db");

    // Read the database file
    let content = std::fs::read(&db_path)
        .map_err(|e| format!("Failed to read database: {}", e))?;

    Ok(content)
}

#[tauri::command]
pub async fn import_from_local(data: Vec<u8>) -> Result<()> {
    let db_path = get_data_dir().join("ccg_gateway.db");

    // Write the database file
    std::fs::write(&db_path, &data)
        .map_err(|e| format!("Failed to write database: {}", e))?;

    // 退出应用，用户需手动重启
    exit_application().await?;

    Ok(())
}

#[tauri::command]
pub async fn export_to_webdav(db: State<'_, SqlitePool>) -> Result<String> {
    use reqwest::Client;

    let settings = get_webdav_settings(db.clone()).await?;
    if settings.url.is_empty() {
        return Err("WebDAV URL not configured".to_string());
    }

    // Read database file
    let db_path = get_data_dir().join("ccg_gateway.db");
    let content = std::fs::read(&db_path)
        .map_err(|e| format!("Failed to read database: {}", e))?;

    // Generate filename
    let filename = format!(
        "ccg_gateway_{}.db",
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    );

    // Ensure remote directory exists
    let client = Client::new();
    let remote_dir = format!("{}/ccg-gateway-backup", settings.url.trim_end_matches('/'));

    // Try to create directory (ignore error if exists)
    let _ = client
        .request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), &remote_dir)
        .basic_auth(&settings.username, Some(&settings.password))
        .send()
        .await;

    // Upload file
    let remote_file = format!("{}/{}", remote_dir, filename);
    let response = client
        .put(&remote_file)
        .basic_auth(&settings.username, Some(&settings.password))
        .body(content)
        .send()
        .await
        .map_err(|e| format!("Upload failed: {}", e))?;

    if !response.status().is_success() && response.status().as_u16() != 201 {
        return Err(format!("Upload failed with status: {}", response.status()));
    }

    Ok(filename)
}

#[tauri::command]
pub async fn list_webdav_backups(db: State<'_, SqlitePool>) -> Result<Vec<WebdavBackup>> {
    use reqwest::Client;

    let settings = get_webdav_settings(db).await?;
    if settings.url.is_empty() {
        return Err("WebDAV URL not configured".to_string());
    }

    let client = Client::new();
    let remote_dir = format!("{}/ccg-gateway-backup", settings.url.trim_end_matches('/'));

    let response = client
        .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &remote_dir)
        .basic_auth(&settings.username, Some(&settings.password))
        .header("Depth", "1")
        .header("Content-Type", "application/xml")
        .body(r#"<?xml version="1.0" encoding="utf-8"?>
            <propfind xmlns="DAV:">
                <prop>
                    <getcontentlength/>
                    <getlastmodified/>
                </prop>
            </propfind>"#)
        .send()
        .await
        .map_err(|e| format!("Failed to list backups: {}", e))?;

    if !response.status().is_success() && response.status().as_u16() != 207 {
        return Ok(Vec::new());
    }

    let body = response.text().await.map_err(|e| e.to_string())?;

    // Parse XML response using quick-xml
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(&body);
    reader.config_mut().trim_text(true);

    let mut backups = Vec::new();
    let mut current_href = String::new();
    let mut current_size: i64 = 0;
    let mut current_modified = String::new();
    let mut in_response = false;
    let mut current_tag = String::new();

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name.ends_with(":response") || name == "response" {
                    in_response = true;
                    current_href.clear();
                    current_size = 0;
                    current_modified.clear();
                }
                current_tag = name;
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default().trim().to_string();
                if in_response && !text.is_empty() {
                    if current_tag.ends_with(":href") || current_tag == "href" {
                        current_href = text;
                    } else if current_tag.ends_with(":getcontentlength") || current_tag == "getcontentlength" {
                        current_size = text.parse::<i64>().unwrap_or(0);
                    } else if current_tag.ends_with(":getlastmodified") || current_tag == "getlastmodified" {
                        current_modified = text;
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name.ends_with(":response") || name == "response" {
                    in_response = false;
                    
                    // Check if this is a .db file we care about
                    if current_href.contains("ccg_gateway_") && current_href.ends_with(".db") {
                        // Extract filename from href
                        if let Some(start) = current_href.rfind('/') {
                            let filename = current_href[start + 1..].to_string();
                            if filename.starts_with("ccg_gateway_") {
                                backups.push(WebdavBackup {
                                    filename,
                                    size: current_size,
                                    modified: current_modified.clone(),
                                });
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error at position {}: {}", reader.buffer_position(), e)),
            _ => {}
        }
        buf.clear();
    }

    // Sort by filename descending (newest first based on timestamp in name)
    backups.sort_by(|a, b| b.filename.cmp(&a.filename));

    Ok(backups)
}

#[tauri::command]
pub async fn import_from_webdav(
    db: State<'_, SqlitePool>,
    filename: String,
) -> Result<()> {
    use reqwest::Client;

    let settings = get_webdav_settings(db).await?;
    if settings.url.is_empty() {
        return Err("WebDAV URL not configured".to_string());
    }

    let client = Client::new();
    let remote_file = format!(
        "{}/ccg-gateway-backup/{}",
        settings.url.trim_end_matches('/'),
        filename
    );

    let response = client
        .get(&remote_file)
        .basic_auth(&settings.username, Some(&settings.password))
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    let content = response.bytes().await.map_err(|e| e.to_string())?;

    // Write to database file
    let db_path = get_data_dir().join("ccg_gateway.db");

    std::fs::write(&db_path, &content)
        .map_err(|e| format!("Failed to write database: {}", e))?;

    // 退出应用，用户需手动重启
    exit_application().await?;

    Ok(())
}

#[tauri::command]
pub async fn delete_webdav_backup(
    db: State<'_, SqlitePool>,
    filename: String,
) -> Result<()> {
    use reqwest::Client;

    let settings = get_webdav_settings(db).await?;
    if settings.url.is_empty() {
        return Err("WebDAV URL not configured".to_string());
    }

    let client = Client::new();
    let remote_file = format!(
        "{}/ccg-gateway-backup/{}",
        settings.url.trim_end_matches('/'),
        filename
    );

    let response = client
        .delete(&remote_file)
        .basic_auth(&settings.username, Some(&settings.password))
        .send()
        .await
        .map_err(|e| format!("Delete failed: {}", e))?;

    if !response.status().is_success() && response.status().as_u16() != 204 {
        return Err(format!("Delete failed with status: {}", response.status()));
    }

    Ok(())
}

// ==================== Skill 相关命令 ====================

// 获取 SSOT 目录 (ccg-gateway 数据目录下的 skills/)
fn get_ssot_dir() -> std::path::PathBuf {
    let dir = get_data_dir().join("skills");
    std::fs::create_dir_all(&dir).ok();
    dir
}

// 获取 Skill 仓库缓存目录 (ccg-gateway 数据目录下的 skill_repo/)
fn get_skill_cache_dir() -> std::path::PathBuf {
    let dir = get_data_dir().join("skill_repo");
    std::fs::create_dir_all(&dir).ok();
    dir
}

// 获取缓存的仓库 ZIP 文件路径
fn get_cached_repo_zip(owner: &str, name: &str, branch: &str) -> std::path::PathBuf {
    get_skill_cache_dir().join(format!("{}_{}__{}.zip", owner, name, branch))
}

// 读取缓存的 ZIP 文件（如果存在）
fn read_cached_zip(owner: &str, name: &str, branch: &str) -> Option<Vec<u8>> {
    let path = get_cached_repo_zip(owner, name, branch);
    if path.exists() {
        std::fs::read(&path).ok()
    } else {
        None
    }
}

// 保存 ZIP 到缓存
fn save_zip_to_cache(owner: &str, name: &str, branch: &str, bytes: &[u8]) -> Result<()> {
    let path = get_cached_repo_zip(owner, name, branch);
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    tracing::info!("Saved repo ZIP to cache: {}", path.display());
    Ok(())
}

// 删除缓存的仓库 ZIP
fn delete_cached_repo_zip(owner: &str, name: &str) {
    let cache_dir = get_skill_cache_dir();
    // 删除所有该仓库的缓存（不同分支）
    if let Ok(entries) = std::fs::read_dir(&cache_dir) {
        let prefix = format!("{}_{}_", owner, name);
        for entry in entries.flatten() {
            if let Some(filename) = entry.file_name().to_str() {
                if filename.starts_with(&prefix) && filename.ends_with(".zip") {
                    let _ = std::fs::remove_file(entry.path());
                    tracing::info!("Deleted cached ZIP: {}", filename);
                }
            }
        }
    }
}

// 获取 CLI 的 skills 目录（异步版本，支持自定义配置目录）
async fn get_skill_cli_dir_async(db: &SqlitePool, cli_type: &str) -> Option<std::path::PathBuf> {
    let config_dir = get_cli_config_dir_path(db, cli_type).await;
    Some(config_dir.join("skills"))
}

// 检查 skill 是否在 CLI 目录中启用（异步版本）
async fn skill_enabled_in_cli_async(db: &SqlitePool, cli_type: &str, directory: &str) -> bool {
    let cli_dir = match get_skill_cli_dir_async(db, cli_type).await {
        Some(d) => d,
        None => return false,
    };
    
    let skill_path = cli_dir.join(directory);
    skill_path.exists()
}

// 解析 SKILL.md frontmatter
fn parse_skill_metadata(content: &str) -> (Option<String>, Option<String>) {
    let content = content.trim_start_matches('\u{feff}');
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return (None, None);
    }
    let front_matter = parts[1].trim();
    let mut name = None;
    let mut description = None;
    for line in front_matter.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("name:") {
            name = Some(val.trim().to_string());
        } else if let Some(val) = line.strip_prefix("description:") {
            description = Some(val.trim().to_string());
        }
    }
    (name, description)
}

// 递归复制目录
fn copy_dir_recursive(src: &std::path::Path, dest: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            std::fs::copy(&path, &dest_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

// 同步 skill 到 CLI 目录（异步版本）
async fn sync_skill_to_cli_async(db: &SqlitePool, directory: &str, cli_type: &str) -> Result<()> {
    let ssot_dir = get_ssot_dir();
    let source = ssot_dir.join(directory);
    if !source.exists() {
        return Err(format!("Skill directory not found: {}", source.display()));
    }
    let cli_dir = match get_skill_cli_dir_async(db, cli_type).await {
        Some(d) => d,
        None => return Err(format!("Unsupported CLI type: {}", cli_type)),
    };

    let dest = cli_dir.join(directory);
    copy_dir_recursive(&source, &dest)?;
    tracing::info!("Synced skill {} to {}", directory, cli_type);
    Ok(())
}

// 从 CLI 目录移除 skill（异步版本）
async fn remove_skill_from_cli_async(db: &SqlitePool, directory: &str, cli_type: &str) -> Result<()> {
    let cli_dir = match get_skill_cli_dir_async(db, cli_type).await {
        Some(d) => d,
        None => return Ok(()),
    };
    let skill_folder = cli_dir.join(directory);
    if skill_folder.exists() {
        std::fs::remove_dir_all(&skill_folder).map_err(|e| e.to_string())?;
        tracing::info!("Removed skill {} from {}", directory, cli_type);
    }
    Ok(())
}

// 从所有 CLI 目录移除 skill（异步版本）
async fn remove_skill_from_all_cli_async(db: &SqlitePool, directory: &str) -> Result<()> {
    for cli_type in ["claude_code", "codex", "gemini"] {
        remove_skill_from_cli_async(db, directory, cli_type).await?;
    }
    Ok(())
}

// ==================== 仓库管理命令 ====================

// 解析 GitHub 仓库 URL，返回 (owner, name)
// 支持格式:
//   https://github.com/owner/name
//   https://github.com/owner/name.git
//   github.com/owner/name
//   owner/name
fn parse_github_url(url: &str) -> Result<(String, String)> {
    let url = url.trim();
    
    // 移除 .git 后缀
    let url = url.strip_suffix(".git").unwrap_or(url);
    
    // 尝试解析不同格式
    let parts: Vec<&str> = if url.contains("github.com") {
        // https://github.com/owner/name 或 github.com/owner/name
        url.split("github.com/")
            .last()
            .unwrap_or("")
            .split('/')
            .collect()
    } else if url.contains('/') {
        // owner/name 格式
        url.split('/').collect()
    } else {
        return Err("Invalid GitHub URL format".to_string());
    };
    
    if parts.len() >= 2 && !parts[0].is_empty() && !parts[1].is_empty() {
        Ok((parts[0].to_string(), parts[1].to_string()))
    } else {
        Err("Invalid GitHub URL: cannot extract owner/name".to_string())
    }
}

#[tauri::command]
pub async fn get_skill_repos(db: State<'_, SqlitePool>) -> Result<Vec<SkillRepo>> {
    let repos = sqlx::query_as::<_, SkillRepo>("SELECT * FROM skill_repos ORDER BY owner, name")
        .fetch_all(db.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(repos)
}

#[tauri::command]
pub async fn add_skill_repo(db: State<'_, SqlitePool>, input: SkillRepoCreate) -> Result<SkillRepo> {
    // 解析 URL 获取 owner/name
    let (owner, name) = parse_github_url(&input.url)?;
    let user_branch = input.branch.unwrap_or_else(|| "main".to_string());
    
    // 检测实际分支
    let client = reqwest::Client::new();
    let actual_branch = detect_repo_branch(&client, &owner, &name, &user_branch).await?;
    
    // 如果用户指定的分支不存在，返回错误提示
    if actual_branch != user_branch {
        return Err(format!(
            "分支 '{}' 不存在，该仓库使用的是 '{}' 分支",
            user_branch, actual_branch
        ));
    }
    
    sqlx::query("INSERT OR REPLACE INTO skill_repos (owner, name, branch) VALUES (?, ?, ?)")
        .bind(&owner)
        .bind(&name)
        .bind(&actual_branch)
        .execute(db.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(SkillRepo {
        owner,
        name,
        branch: actual_branch,
    })
}

// 检测仓库实际分支
async fn detect_repo_branch(
    client: &reqwest::Client,
    owner: &str,
    name: &str,
    preferred_branch: &str,
) -> Result<String> {
    // 尝试的分支顺序
    let branches = if preferred_branch.is_empty() {
        vec!["main", "master"]
    } else {
        vec![preferred_branch, "main", "master"]
    };
    
    for br in branches {
        let url = format!("https://github.com/{}/{}/archive/refs/heads/{}.zip", owner, name, br);
        match client.head(&url).send().await {
            Ok(response) if response.status().is_success() => {
                return Ok(br.to_string());
            }
            _ => continue,
        }
    }
    Err(format!("无法访问仓库 {}/{}，请检查仓库地址是否正确", owner, name))
}

#[tauri::command]
pub async fn remove_skill_repo(db: State<'_, SqlitePool>, owner: String, name: String) -> Result<()> {
    sqlx::query("DELETE FROM skill_repos WHERE owner = ? AND name = ?")
        .bind(&owner)
        .bind(&name)
        .execute(db.inner())
        .await
        .map_err(|e| e.to_string())?;
    
    // 删除缓存的仓库 ZIP
    delete_cached_repo_zip(&owner, &name);
    
    Ok(())
}

#[tauri::command]
pub async fn update_skill_repo(
    db: State<'_, SqlitePool>,
    old_owner: String,
    old_name: String,
    new_url: String,
    new_branch: String,
) -> Result<SkillRepo> {
    // 解析新 URL
    let (new_owner, new_name) = parse_github_url(&new_url)?;
    let user_branch = if new_branch.is_empty() { "main".to_string() } else { new_branch };
    
    // 检测实际分支
    let client = reqwest::Client::new();
    let actual_branch = detect_repo_branch(&client, &new_owner, &new_name, &user_branch).await?;
    
    // 如果用户指定的分支不存在，返回错误提示
    if actual_branch != user_branch {
        return Err(format!(
            "分支 '{}' 不存在，该仓库使用的是 '{}' 分支",
            user_branch, actual_branch
        ));
    }
    
    // 检查旧记录是否存在
    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM skill_repos WHERE owner = ? AND name = ?")
        .bind(&old_owner)
        .bind(&old_name)
        .fetch_one(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    if exists == 0 {
        return Err("Repo not found".to_string());
    }

    // 删除旧记录
    sqlx::query("DELETE FROM skill_repos WHERE owner = ? AND name = ?")
        .bind(&old_owner)
        .bind(&old_name)
        .execute(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    // 插入新记录
    sqlx::query("INSERT OR REPLACE INTO skill_repos (owner, name, branch) VALUES (?, ?, ?)")
        .bind(&new_owner)
        .bind(&new_name)
        .bind(&actual_branch)
        .execute(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(SkillRepo {
        owner: new_owner,
        name: new_name,
        branch: actual_branch,
    })
}

// ==================== Skill 发现命令 ====================

#[tauri::command]
pub async fn discover_repo_skills(owner: String, name: String, branch: String) -> Result<Vec<DiscoverableSkill>> {
    let branch_to_use = if branch.is_empty() { "main" } else { &branch };
    
    // 优先使用缓存
    if let Some(bytes) = read_cached_zip(&owner, &name, branch_to_use) {
        tracing::info!("Using cached ZIP for {}/{}", owner, name);
        let mut skills = scan_zip_for_skills(&bytes, &owner, &name, branch_to_use)?;
        skills.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        return Ok(skills);
    }
    
    // 没有缓存则下载
    let client = reqwest::Client::new();
    let bytes = download_repo_zip(&client, &owner, &name, branch_to_use).await?;
    
    // 保存到缓存
    let _ = save_zip_to_cache(&owner, &name, branch_to_use, &bytes);
    
    let mut skills = scan_zip_for_skills(&bytes, &owner, &name, branch_to_use)?;
    skills.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(skills)
}

// 强制刷新仓库 skills（删除缓存后重新下载）
#[tauri::command]
pub async fn refresh_repo_skills(owner: String, name: String, branch: String) -> Result<Vec<DiscoverableSkill>> {
    let branch_to_use = if branch.is_empty() { "main" } else { &branch };
    
    // 删除旧缓存
    delete_cached_repo_zip(&owner, &name);
    
    // 重新下载
    let client = reqwest::Client::new();
    let bytes = download_repo_zip(&client, &owner, &name, branch_to_use).await?;
    
    // 保存到缓存
    let _ = save_zip_to_cache(&owner, &name, branch_to_use, &bytes);
    
    let mut skills = scan_zip_for_skills(&bytes, &owner, &name, branch_to_use)?;
    skills.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(skills)
}

// 下载仓库 ZIP
async fn download_repo_zip(
    client: &reqwest::Client,
    owner: &str,
    name: &str,
    branch: &str,
) -> Result<Vec<u8>> {
    let url = format!("https://github.com/{}/{}/archive/refs/heads/{}.zip", owner, name, branch);
    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        return Err(format!("下载失败: HTTP {}", response.status()));
    }
    
    response.bytes().await
        .map(|b| b.to_vec())
        .map_err(|e| e.to_string())
}

// 扫描 ZIP 中的 skills
fn scan_zip_for_skills(
    bytes: &[u8],
    owner: &str,
    repo_name: &str,
    branch: &str,
) -> Result<Vec<DiscoverableSkill>> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| e.to_string())?;

    // 获取根目录名
    let root_name = if !archive.is_empty() {
        let first = archive.by_index(0).map_err(|e| e.to_string())?;
        first.name().split('/').next().unwrap_or("").to_string()
    } else {
        return Ok(vec![]);
    };

    let mut skills = Vec::new();
    let mut skill_dirs: std::collections::HashSet<String> = std::collections::HashSet::new();

    // 查找所有 SKILL.md 文件
    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(|e| e.to_string())?;
        let file_path = file.name().to_string();

        if file_path.ends_with("SKILL.md") {
            // 获取相对路径
            let relative_path = file_path
                .strip_prefix(&format!("{}/", root_name))
                .unwrap_or(&file_path);

            // 获取目录路径 (移除 SKILL.md)
            let dir_path = std::path::Path::new(relative_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            if !dir_path.is_empty() {
                skill_dirs.insert(dir_path);
            } else {
                // SKILL.md 在根目录
                skill_dirs.insert(repo_name.to_string());
            }
        }
    }

    // 为每个 skill 目录读取元数据
    for dir in skill_dirs {
        let skill_md_path = if dir == repo_name {
            format!("{}/SKILL.md", root_name)
        } else {
            format!("{}/{}/SKILL.md", root_name, dir)
        };

        let mut content = String::new();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
            if file.name() == skill_md_path {
                file.read_to_string(&mut content).map_err(|e| e.to_string())?;
                break;
            }
        }

        let (name, description) = parse_skill_metadata(&content);
        let directory_name = std::path::Path::new(&dir)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| dir.clone());

        skills.push(DiscoverableSkill {
            key: format!("{}/{}:{}", owner, repo_name, dir),
            name: name.unwrap_or_else(|| directory_name.clone()),
            description: description.unwrap_or_default(),
            directory: dir.clone(),
            readme_url: Some(format!("https://github.com/{}/{}/tree/{}/{}", owner, repo_name, branch, dir)),
            repo_owner: owner.to_string(),
            repo_name: repo_name.to_string(),
            repo_branch: branch.to_string(),
        });
    }

    Ok(skills)
}

// ==================== Skill 安装/卸载命令 ====================

#[tauri::command]
pub async fn install_skill(db: State<'_, SqlitePool>, skill: DiscoverableSkill, reinstall: Option<bool>) -> Result<InstalledSkillResponse> {
    let ssot_dir = get_ssot_dir();
    let directory_name = std::path::Path::new(&skill.directory)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| skill.directory.clone());

    let is_reinstall = reinstall.unwrap_or(false);

    // 检查是否已安装
    let existing = sqlx::query_as::<_, SkillConfig>("SELECT * FROM skill_configs WHERE directory = ?")
        .bind(&directory_name)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    if existing.is_some() && !is_reinstall {
        return Err(format!("Skill '{}' is already installed", directory_name));
    }

    // 如果是重装，先删除旧的 SSOT 目录
    if is_reinstall {
        let old_skill_path = ssot_dir.join(&directory_name);
        if old_skill_path.exists() {
            let _ = std::fs::remove_dir_all(&old_skill_path);
        }
    }

    // 优先使用缓存的 ZIP
    let branch_to_use = if skill.repo_branch.is_empty() { "main" } else { &skill.repo_branch };
    let bytes = if let Some(cached) = read_cached_zip(&skill.repo_owner, &skill.repo_name, branch_to_use) {
        tracing::info!("Using cached ZIP for install: {}/{}", skill.repo_owner, skill.repo_name);
        cached
    } else {
        // 没有缓存则下载
        let client = reqwest::Client::new();
        let downloaded = download_repo_zip(&client, &skill.repo_owner, &skill.repo_name, branch_to_use).await?;
        // 保存到缓存
        let _ = save_zip_to_cache(&skill.repo_owner, &skill.repo_name, branch_to_use, &downloaded);
        downloaded
    };

    // 提取 skill 到 SSOT
    extract_skill_from_zip(&bytes, &skill.directory, &ssot_dir, &directory_name)?;

    // 保存到数据库（如果是重装则更新）
    let now = chrono::Utc::now().timestamp();
    let id = if is_reinstall && existing.is_some() {
        let old = existing.unwrap();
        sqlx::query(
            "UPDATE skill_configs SET name = ?, description = ?, repo_owner = ?, repo_name = ?, repo_branch = ?, readme_url = ?, installed_at = ? WHERE id = ?"
        )
        .bind(&skill.name)
        .bind(&skill.description)
        .bind(&skill.repo_owner)
        .bind(&skill.repo_name)
        .bind(&skill.repo_branch)
        .bind(&skill.readme_url)
        .bind(now)
        .bind(old.id)
        .execute(db.inner())
        .await
        .map_err(|e| e.to_string())?;
        tracing::info!("Reinstalled skill: {} ({})", skill.name, directory_name);
        old.id
    } else {
        let result = sqlx::query(
            "INSERT INTO skill_configs (name, description, directory, repo_owner, repo_name, repo_branch, readme_url, installed_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&skill.name)
        .bind(&skill.description)
        .bind(&directory_name)
        .bind(&skill.repo_owner)
        .bind(&skill.repo_name)
        .bind(&skill.repo_branch)
        .bind(&skill.readme_url)
        .bind(now)
        .execute(db.inner())
        .await
        .map_err(|e| e.to_string())?;
        tracing::info!("Installed skill: {} ({})", skill.name, directory_name);
        result.last_insert_rowid()
    };

    // 返回安装结果（默认三个端都未启用）
    let cli_flags = vec![
        SkillCliFlag { cli_type: "claude_code".to_string(), enabled: false },
        SkillCliFlag { cli_type: "codex".to_string(), enabled: false },
        SkillCliFlag { cli_type: "gemini".to_string(), enabled: false },
    ];

    Ok(InstalledSkillResponse {
        id,
        name: skill.name,
        description: if skill.description.is_empty() { None } else { Some(skill.description) },
        directory: directory_name,
        repo_owner: Some(skill.repo_owner),
        repo_name: Some(skill.repo_name),
        repo_branch: Some(skill.repo_branch),
        readme_url: skill.readme_url,
        installed_at: now,
        cli_flags,
        exists_on_disk: true, // 刚安装完肯定存在
    })
}

// 从 ZIP 中提取 skill 到 SSOT
fn extract_skill_from_zip(
    bytes: &[u8],
    skill_dir: &str,
    ssot_dir: &std::path::Path,
    directory_name: &str,
) -> Result<()> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| e.to_string())?;

    // 获取根目录名
    let root_name = if !archive.is_empty() {
        let first = archive.by_index(0).map_err(|e| e.to_string())?;
        first.name().split('/').next().unwrap_or("").to_string()
    } else {
        return Err("Empty archive".to_string());
    };

    let skill_prefix = format!("{}/{}/", root_name, skill_dir);
    let dest_dir = ssot_dir.join(directory_name);

    // 创建目标目录
    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let file_path = file.name().to_string();

        if let Some(relative) = file_path.strip_prefix(&skill_prefix) {
            if relative.is_empty() {
                continue;
            }

            let out_path = dest_dir.join(relative);

            if file.is_dir() {
                std::fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
            } else {
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut out_file = std::fs::File::create(&out_path).map_err(|e| e.to_string())?;
                std::io::copy(&mut file, &mut out_file).map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn uninstall_skill(db: State<'_, SqlitePool>, id: i64) -> Result<()> {
    // 获取 skill 信息
    let skill = sqlx::query_as::<_, SkillConfig>("SELECT * FROM skill_configs WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Skill not found".to_string())?;

    // 从所有 CLI 目录移除
    remove_skill_from_all_cli_async(db.inner(), &skill.directory).await?;

    // 从 SSOT 移除
    let ssot_dir = get_ssot_dir();
    let skill_path = ssot_dir.join(&skill.directory);
    if skill_path.exists() {
        std::fs::remove_dir_all(&skill_path).map_err(|e| e.to_string())?;
    }

    // 从数据库删除
    sqlx::query("DELETE FROM skill_configs WHERE id = ?")
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    tracing::info!("Uninstalled skill: {}", skill.directory);
    Ok(())
}

// ==================== 已安装 Skill 管理命令 ====================

#[tauri::command]
pub async fn get_installed_skills(db: State<'_, SqlitePool>) -> Result<Vec<InstalledSkillResponse>> {
    let skills = sqlx::query_as::<_, SkillConfig>("SELECT * FROM skill_configs ORDER BY name")
        .fetch_all(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    let ssot_dir = get_ssot_dir();
    let mut results = Vec::new();
    for skill in skills {
        let cli_flags = vec![
            SkillCliFlag {
                cli_type: "claude_code".to_string(),
                enabled: skill_enabled_in_cli_async(db.inner(), "claude_code", &skill.directory).await,
            },
            SkillCliFlag {
                cli_type: "codex".to_string(),
                enabled: skill_enabled_in_cli_async(db.inner(), "codex", &skill.directory).await,
            },
            SkillCliFlag {
                cli_type: "gemini".to_string(),
                enabled: skill_enabled_in_cli_async(db.inner(), "gemini", &skill.directory).await,
            },
        ];

        // 检查 skill 目录是否存在于 SSOT 目录
        let exists_on_disk = ssot_dir.join(&skill.directory).exists();

        results.push(InstalledSkillResponse {
            id: skill.id,
            name: skill.name,
            description: skill.description,
            directory: skill.directory,
            repo_owner: skill.repo_owner,
            repo_name: skill.repo_name,
            repo_branch: skill.repo_branch,
            readme_url: skill.readme_url,
            installed_at: skill.installed_at,
            cli_flags,
            exists_on_disk,
        });
    }
    Ok(results)
}

#[tauri::command]
pub async fn toggle_skill_cli(db: State<'_, SqlitePool>, id: i64, cli_type: String, enabled: bool) -> Result<()> {
    let skill = sqlx::query_as::<_, SkillConfig>("SELECT * FROM skill_configs WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Skill not found".to_string())?;

    if enabled {
        sync_skill_to_cli_async(db.inner(), &skill.directory, &cli_type).await?;
    } else {
        remove_skill_from_cli_async(db.inner(), &skill.directory, &cli_type).await?;
    }

    Ok(())
}

// ==================== 检查更新命令 ====================

const GITHUB_OWNER: &str = "mos1128";
const GITHUB_REPO: &str = "ccg-gateway";

#[derive(serde::Serialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub body: Option<String>,
    pub html_url: String,
    pub published_at: Option<String>,
}

#[tauri::command]
pub async fn check_for_updates() -> Result<Option<GitHubRelease>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        GITHUB_OWNER, GITHUB_REPO
    );

    let response = client
        .get(&url)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "ccg-gateway")
        .send()
        .await
        .map_err(|e| format!("网络请求失败: {}", e))?;

    if response.status() == 404 {
        return Ok(None);
    }

    if !response.status().is_success() {
        return Err(format!("GitHub API 错误: {}", response.status()));
    }

    let release: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    Ok(Some(GitHubRelease {
        tag_name: release["tag_name"].as_str().unwrap_or("").to_string(),
        name: release["name"].as_str().map(|s| s.to_string()),
        body: release["body"].as_str().map(|s| s.to_string()),
        html_url: release["html_url"].as_str().unwrap_or("").to_string(),
        published_at: release["published_at"].as_str().map(|s| s.to_string()),
    }))
}

// ==================== Official Credential 相关命令 ====================

/// 解析凭证 JSON 生成显示信息
fn parse_display_info(cli_type: &str, credential_json: &str) -> String {
    // 尝试解析为文件列表格式
    if let Ok(files) = serde_json::from_str::<Vec<serde_json::Value>>(credential_json) {
        match cli_type {
            "claude_code" => {
                // 查找 settings.json 文件
                if let Some(file) = files.iter().find(|f| {
                    f.get("path").and_then(|p| p.as_str()).map(|p| p.contains("settings.json")).unwrap_or(false)
                }) {
                    if let Some(content) = file.get("content").and_then(|c| c.as_str()) {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(content) {
                            return data.get("ANTHROPIC_API_KEY")
                                .and_then(|v| v.as_str())
                                .map(|key| {
                                    if key.len() > 12 {
                                        format!("sk-ant-...{}", &key[key.len()-8..])
                                    } else {
                                        "已配置".to_string()
                                    }
                                })
                                .unwrap_or_else(|| "未知".to_string());
                        }
                    }
                }
                "未配置".to_string()
            }
            "codex" => {
                // 查找 auth.json 文件
                if let Some(file) = files.iter().find(|f| {
                    f.get("path").and_then(|p| p.as_str()).map(|p| p.contains("auth.json")).unwrap_or(false)
                }) {
                    if let Some(content) = file.get("content").and_then(|c| c.as_str()) {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(content) {
                            return data.get("tokens")
                                .and_then(|t| t.get("access_token"))
                                .and_then(|v| v.as_str())
                                .map(|_| "已配置".to_string())
                                .unwrap_or_else(|| "未知".to_string());
                        }
                    }
                }
                "未配置".to_string()
            }
            "gemini" => {
                // 查找 google_accounts.json 文件
                if let Some(file) = files.iter().find(|f| {
                    f.get("path").and_then(|p| p.as_str()).map(|p| p.contains("google_accounts.json")).unwrap_or(false)
                }) {
                    if let Some(content) = file.get("content").and_then(|c| c.as_str()) {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(content) {
                            return data.get("active")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "已配置".to_string());
                        }
                    }
                }
                "未配置".to_string()
            }
            _ => "未知".to_string()
        }
    } else {
        // 兼容旧格式：直接解析为 JSON 对象
        match serde_json::from_str::<serde_json::Value>(credential_json) {
            Ok(creds) => {
                match cli_type {
                    "claude_code" => {
                        creds.get("ANTHROPIC_API_KEY")
                            .and_then(|v| v.as_str())
                            .map(|key| {
                                if key.len() > 12 {
                                    format!("sk-ant-...{}", &key[key.len()-8..])
                                } else {
                                    "已配置".to_string()
                                }
                            })
                            .unwrap_or_else(|| "未知".to_string())
                    }
                    "codex" => {
                        creds.get("tokens")
                            .and_then(|t| t.get("active_email"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "已配置".to_string())
                    }
                    "gemini" => {
                        creds.get("google_accounts")
                            .and_then(|g| g.get("active"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "已配置".to_string())
                    }
                    _ => "未知".to_string()
                }
            }
            Err(_) => "无效 JSON".to_string()
        }
    }
}

/// 读取 CLI 当前凭证（异步版本，支持自定义配置目录）
async fn read_cli_credential_impl_async(db: &SqlitePool, cli_type: &str) -> Result<String> {
    let config_dir = get_cli_config_dir_path(db, cli_type).await;

    match cli_type {
        "claude_code" => {
            let config_path = config_dir.join("settings.json");
            
            // 如果文件不存在，返回空内容（而不是报错）
            if !config_path.exists() {
                let files = vec![
                    serde_json::json!({
                        "path": format!("{}/settings.json", config_dir.display()),
                        "content": ""
                    })
                ];
                return Ok(serde_json::to_string(&files).unwrap());
            }
            
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| format!("读取失败: {}", e))?;
            
            let files = vec![
                serde_json::json!({
                    "path": format!("{}/settings.json", config_dir.display()),
                    "content": content
                })
            ];
            Ok(serde_json::to_string(&files).unwrap())
        }
        "codex" => {
            let auth_path = config_dir.join("auth.json");
            
            // 如果文件不存在，返回空的文件列表（而不是报错）
            if !auth_path.exists() {
                let files = vec![
                    serde_json::json!({
                        "path": format!("{}/auth.json", config_dir.display()),
                        "content": ""
                    })
                ];
                return Ok(serde_json::to_string(&files).unwrap());
            }
            
            let content = std::fs::read_to_string(&auth_path)
                .map_err(|e| format!("读取失败: {}", e))?;
            
            let files = vec![
                serde_json::json!({
                    "path": format!("{}/auth.json", config_dir.display()),
                    "content": content
                })
            ];
            Ok(serde_json::to_string(&files).unwrap())
        }
        "gemini" => {
            let oauth_path = config_dir.join("oauth_creds.json");
            let accounts_path = config_dir.join("google_accounts.json");
            let settings_path = config_dir.join("settings.json");

            let mut files = vec![];

            // 即使文件不存在，也添加空内容的占位符
            if oauth_path.exists() {
                let content = std::fs::read_to_string(&oauth_path)
                    .map_err(|e| format!("读取 oauth_creds.json 失败: {}", e))?;
                files.push(serde_json::json!({
                    "path": format!("{}/oauth_creds.json", config_dir.display()),
                    "content": content
                }));
            } else {
                files.push(serde_json::json!({
                    "path": format!("{}/oauth_creds.json", config_dir.display()),
                    "content": ""
                }));
            }
            
            if accounts_path.exists() {
                let content = std::fs::read_to_string(&accounts_path)
                    .map_err(|e| format!("读取 google_accounts.json 失败: {}", e))?;
                files.push(serde_json::json!({
                    "path": format!("{}/google_accounts.json", config_dir.display()),
                    "content": content
                }));
            } else {
                files.push(serde_json::json!({
                    "path": format!("{}/google_accounts.json", config_dir.display()),
                    "content": ""
                }));
            }
            
            if settings_path.exists() {
                let content = std::fs::read_to_string(&settings_path)
                    .map_err(|e| format!("读取 settings.json 失败: {}", e))?;
                files.push(serde_json::json!({
                    "path": format!("{}/settings.json", config_dir.display()),
                    "content": content
                }));
            } else {
                files.push(serde_json::json!({
                    "path": format!("{}/settings.json", config_dir.display()),
                    "content": ""
                }));
            }

            Ok(serde_json::to_string(&files).unwrap())
        }
        _ => Err("Unsupported CLI type".to_string()),
    }
}

/// 同步凭证到 CLI 配置文件（异步版本，支持自定义配置目录）
async fn sync_credential_to_cli_async(db: &SqlitePool, cli_type: &str, credential_json: &str, default_config: &str) -> Result<()> {
    // 解析文件列表
    let files: Vec<serde_json::Value> = serde_json::from_str(credential_json)
        .map_err(|e| format!("解析凭证文件列表失败: {}", e))?;
    
    let config_dir = get_cli_config_dir_path(db, cli_type).await;

    match cli_type {
        "claude_code" => {
            // TODO: Claude Code 的具体实现待完善
            tracing::warn!("Claude Code 的直连模式配置写入功能尚未实现");
        }
        "codex" => {
            let auth_path = config_dir.join("auth.json");
            let config_path = config_dir.join("config.toml");

            // 直连模式不备份
            std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;

            // 查找 auth.json 文件
            let auth_file = files.iter()
                .find(|f| f.get("path")
                    .and_then(|p| p.as_str())
                    .map(|p| p.contains("auth.json"))
                    .unwrap_or(false));
            
            if let Some(file) = auth_file {
                let content = file.get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or("");
                
                // 只有当内容不为空时才写入
                if !content.is_empty() {
                    tracing::info!("写入 Codex auth.json，内容长度: {}，路径: {:?}", content.len(), auth_path);
                    std::fs::write(&auth_path, content).map_err(|e| {
                        tracing::error!("写入 auth.json 失败: {}", e);
                        e.to_string()
                    })?;
                    tracing::info!("Codex auth.json 写入成功");
                } else {
                    tracing::warn!("Codex auth.json 内容为空，跳过写入");
                }
            } else {
                tracing::warn!("未找到 Codex auth.json 文件配置");
            }

            // 处理 config.toml：直接使用全局配置（如果有）
            if !default_config.is_empty() {
                let doc = default_config.parse::<toml_edit::DocumentMut>()
                    .unwrap_or_else(|_| toml_edit::DocumentMut::new());
                tracing::info!("写入 Codex config.toml，路径: {:?}", config_path);
                std::fs::write(&config_path, doc.to_string()).map_err(|e| {
                    tracing::error!("写入 config.toml 失败: {}", e);
                    e.to_string()
                })?;
                tracing::info!("Codex config.toml 写入成功");
            } else {
                tracing::info!("Codex 全局配置为空，跳过写入 config.toml");
            }
        }
        "gemini" => {
            let oauth_path = config_dir.join("oauth_creds.json");
            let accounts_path = config_dir.join("google_accounts.json");
            let settings_path = config_dir.join("settings.json");
            let env_path = config_dir.join(".env");

            // 直连模式不备份
            std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;

            // 用于存储 settings.json 的内容
            let mut settings_content: Option<String> = None;

            // 写入各个文件
            for file in files.iter() {
                let path_str = file.get("path")
                    .and_then(|p| p.as_str())
                    .unwrap_or("");
                let content = file.get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or("");
                
                if path_str.contains("oauth_creds.json") && !content.is_empty() {
                    tracing::info!("写入 Gemini oauth_creds.json，内容长度: {}，路径: {:?}", content.len(), oauth_path);
                    std::fs::write(&oauth_path, content).map_err(|e| {
                        tracing::error!("写入 oauth_creds.json 失败: {}", e);
                        e.to_string()
                    })?;
                    tracing::info!("Gemini oauth_creds.json 写入成功");
                } else if path_str.contains("google_accounts.json") && !content.is_empty() {
                    tracing::info!("写入 Gemini google_accounts.json，内容长度: {}，路径: {:?}", content.len(), accounts_path);
                    std::fs::write(&accounts_path, content).map_err(|e| {
                        tracing::error!("写入 google_accounts.json 失败: {}", e);
                        e.to_string()
                    })?;
                    tracing::info!("Gemini google_accounts.json 写入成功");
                } else if path_str.contains("settings.json") && !content.is_empty() {
                    settings_content = Some(content.to_string());
                }
            }

            // 删除 .env 文件（如果存在）
            if env_path.exists() { 
                let _ = std::fs::remove_file(&env_path); 
            }

            // 处理 settings.json：合并凭证配置和全局配置（不读取现有文件）
            // 1. 从凭证中的 settings.json 开始
            let mut config = if let Some(ref user_settings) = settings_content {
                tracing::info!("Gemini 凭证中包含 settings.json，内容长度: {}", user_settings.len());
                serde_json::from_str::<serde_json::Value>(user_settings)
                    .unwrap_or_else(|_| serde_json::json!({}))
            } else {
                tracing::info!("Gemini 凭证中不包含 settings.json");
                serde_json::json!({})
            };

            // 2. 合并全局配置（全局配置优先）
            if !default_config.is_empty() {
                tracing::info!("Gemini 全局配置不为空，长度: {}", default_config.len());
                if let Ok(default_val) = serde_json::from_str::<serde_json::Value>(default_config) {
                    deep_merge(&mut config, &default_val);
                    tracing::info!("Gemini 全局配置合并成功");
                }
            } else {
                tracing::info!("Gemini 全局配置为空");
            }

            // 检查最终配置
            let is_empty = config.as_object().map(|o| o.is_empty()).unwrap_or(true);
            tracing::info!("Gemini settings.json 最终配置是否为空: {}", is_empty);

            // 只有当配置不为空对象时才写入
            if !is_empty {
                tracing::info!("写入 Gemini settings.json，路径: {:?}", settings_path);
                std::fs::write(&settings_path, serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?)
                    .map_err(|e| {
                        tracing::error!("写入 settings.json 失败: {}", e);
                        e.to_string()
                    })?;
                tracing::info!("Gemini settings.json 写入成功");
            } else {
                tracing::warn!("Gemini settings.json 配置为空对象，跳过写入");
            }
        }
        _ => return Err("不支持的 CLI 类型".to_string())
    }

    Ok(())
}

/// 在直连模式下，自动同步第一个凭证到 CLI 配置文件
async fn auto_sync_credential_in_direct_mode(db: &SqlitePool, cli_type: &str) -> Result<()> {
    tracing::info!("auto_sync_credential_in_direct_mode 被调用，cli_type: {}", cli_type);
    
    // 检查当前是否为直连模式
    let current_mode: Option<(String,)> = sqlx::query_as(
        "SELECT cli_mode FROM cli_settings WHERE cli_type = ?",
    )
    .bind(cli_type)
    .fetch_optional(db)
    .await
    .map_err(|e| e.to_string())?;

    let mode = current_mode.map(|r| r.0).unwrap_or_else(|| "proxy".to_string());
    tracing::info!("{} 当前模式: {}", cli_type, mode);
    
    if mode != "direct" {
        tracing::debug!("{} 当前不是直连模式，跳过自动同步", cli_type);
        return Ok(());
    }

    // 获取第一个凭证（sort_order = 0）
    let cred: Option<OfficialCredential> = sqlx::query_as(
        "SELECT * FROM official_credentials WHERE cli_type = ? AND sort_order = 0 LIMIT 1",
    )
    .bind(cli_type)
    .fetch_optional(db)
    .await
    .map_err(|e| e.to_string())?;

    if let Some(cred) = cred {
        tracing::info!("{} 找到凭证 ID: {}, 名称: {}", cli_type, cred.id, cred.name);
        
        // 获取全局配置
        let default_config = sqlx::query_as::<_, (Option<String>,)>(
            "SELECT default_json_config FROM cli_settings WHERE cli_type = ?",
        )
        .bind(cli_type)
        .fetch_optional(db)
        .await
        .map_err(|e| e.to_string())?
        .and_then(|r| r.0)
        .unwrap_or_default();

        tracing::info!("{} 全局配置长度: {}", cli_type, default_config.len());
        tracing::info!("{} 开始同步凭证到文件", cli_type);
        
        match sync_credential_to_cli_async(db, cli_type, &cred.credential_json, &default_config).await {
            Ok(_) => {
                tracing::info!("{} 凭证同步成功", cli_type);
                Ok(())
            }
            Err(e) => {
                tracing::error!("{} 凭证同步失败: {}", cli_type, e);
                Err(e)
            }
        }
    } else {
        tracing::warn!("{} 没有可用的凭证，跳过同步", cli_type);
        Ok(())
    }
}

/// 删除直连模式写入的所有文件（异步版本，支持自定义配置目录）
async fn remove_direct_mode_files_async(db: &SqlitePool, cli_type: &str) -> Result<()> {
    let config_dir = get_cli_config_dir_path(db, cli_type).await;
    
    match cli_type {
        "claude_code" => {
            // TODO: Claude Code 的具体实现待完善
            tracing::warn!("Claude Code 的直连模式文件删除功能尚未实现");
            Ok(())
        }
        "codex" => {
            let auth_path = config_dir.join("auth.json");
            let config_path = config_dir.join("config.toml");
            
            if auth_path.exists() {
                tracing::info!("删除直连模式文件: {:?}", auth_path);
                std::fs::remove_file(&auth_path).map_err(|e| {
                    tracing::error!("删除 auth.json 失败: {}", e);
                    e.to_string()
                })?;
            }
            if config_path.exists() {
                tracing::info!("删除直连模式文件: {:?}", config_path);
                std::fs::remove_file(&config_path).map_err(|e| {
                    tracing::error!("删除 config.toml 失败: {}", e);
                    e.to_string()
                })?;
            }
            Ok(())
        }
        "gemini" => {
            let oauth_path = config_dir.join("oauth_creds.json");
            let accounts_path = config_dir.join("google_accounts.json");
            let settings_path = config_dir.join("settings.json");
            
            if oauth_path.exists() {
                tracing::info!("删除直连模式文件: {:?}", oauth_path);
                std::fs::remove_file(&oauth_path).map_err(|e| {
                    tracing::error!("删除 oauth_creds.json 失败: {}", e);
                    e.to_string()
                })?;
            }
            if accounts_path.exists() {
                tracing::info!("删除直连模式文件: {:?}", accounts_path);
                std::fs::remove_file(&accounts_path).map_err(|e| {
                    tracing::error!("删除 google_accounts.json 失败: {}", e);
                    e.to_string()
                })?;
            }
            if settings_path.exists() {
                tracing::info!("删除直连模式文件: {:?}", settings_path);
                std::fs::remove_file(&settings_path).map_err(|e| {
                    tracing::error!("删除 settings.json 失败: {}", e);
                    e.to_string()
                })?;
            }
            Ok(())
        }
        _ => Err("不支持的 CLI 类型".to_string())
    }
}

#[tauri::command]
pub async fn get_credentials(
    db: State<'_, SqlitePool>,
    cli_type: String,
) -> Result<Vec<OfficialCredentialResponse>> {
    let creds: Vec<OfficialCredential> = sqlx::query_as(
        "SELECT * FROM official_credentials WHERE cli_type = ? ORDER BY sort_order, id",
    )
    .bind(&cli_type)
    .fetch_all(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    let results = creds.into_iter().enumerate().map(|(i, c)| {
        let display_info = parse_display_info(&c.cli_type, &c.credential_json);
        OfficialCredentialResponse {
            is_active: i == 0,
            id: c.id,
            cli_type: c.cli_type,
            name: c.name,
            credential_json: c.credential_json,
            sort_order: c.sort_order,
            display_info,
        }
    }).collect();

    Ok(results)
}

#[tauri::command]
pub async fn create_credential(
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    input: OfficialCredentialCreate,
) -> Result<OfficialCredentialResponse> {
    let now = chrono::Utc::now().timestamp();

    // Check if this is the first credential for this cli_type
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM official_credentials WHERE cli_type = ?",
    )
    .bind(&input.cli_type)
    .fetch_one(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    let sort_order = if count.0 == 0 { 0i64 } else { count.0 };

    let result = sqlx::query(
        "INSERT INTO official_credentials (cli_type, name, credential_json, sort_order, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&input.cli_type)
    .bind(&input.name)
    .bind(&input.credential_json)
    .bind(sort_order)
    .bind(now)
    .bind(now)
    .execute(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    let id = result.last_insert_rowid();

    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "credential_created",
        &format!("凭证 {} 已创建", input.name),
    ).await;

    // 如果是直连模式，自动同步到文件
    if let Err(e) = auto_sync_credential_in_direct_mode(db.inner(), &input.cli_type).await {
        tracing::error!("自动同步凭证失败: {}", e);
    }

    get_credential(db, id).await
}

#[tauri::command]
pub async fn get_credential(
    db: State<'_, SqlitePool>,
    id: i64,
) -> Result<OfficialCredentialResponse> {
    let cred = sqlx::query_as::<_, OfficialCredential>("SELECT * FROM official_credentials WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "凭证不存在".to_string())?;

    Ok(OfficialCredentialResponse {
        is_active: cred.sort_order == 0,
        id: cred.id,
        display_info: parse_display_info(&cred.cli_type, &cred.credential_json),
        cli_type: cred.cli_type,
        name: cred.name,
        credential_json: cred.credential_json,
        sort_order: cred.sort_order,
    })
}

#[tauri::command]
pub async fn update_credential(
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    id: i64,
    input: OfficialCredentialUpdate,
) -> Result<OfficialCredentialResponse> {
    let now = chrono::Utc::now().timestamp();

    let cred_name: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM official_credentials WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    let cred_name = cred_name.ok_or_else(|| "凭证不存在".to_string())?.0;

    let mut updates = vec!["updated_at = ?".to_string()];
    if input.name.is_some() { updates.push("name = ?".to_string()); }
    if input.credential_json.is_some() { updates.push("credential_json = ?".to_string()); }

    let query = format!("UPDATE official_credentials SET {} WHERE id = ?", updates.join(", "));
    let mut q = sqlx::query(&query).bind(now);
    if let Some(ref name) = input.name { q = q.bind(name); }
    if let Some(ref json) = input.credential_json { q = q.bind(json); }
    q.bind(id).execute(db.inner()).await.map_err(|e| e.to_string())?;

    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "credential_updated",
        &format!("凭证 {} 已更新", cred_name),
    ).await;

    // 获取更新后的凭证信息
    let updated_cred: Option<OfficialCredential> = sqlx::query_as(
        "SELECT * FROM official_credentials WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    // 如果是直连模式，自动同步到文件
    if let Some(cred) = updated_cred {
        if let Err(e) = auto_sync_credential_in_direct_mode(db.inner(), &cred.cli_type).await {
            tracing::error!("自动同步凭证失败: {}", e);
        }
    }

    get_credential(db, id).await
}

#[tauri::command]
pub async fn delete_credential(
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    id: i64,
) -> Result<()> {
    let cred_info: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM official_credentials WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    if let Some((name,)) = cred_info {
        let _ = crate::services::stats::record_system_log(
            &log_db.0,
            "credential_deleted",
            &format!("凭证 {} 已删除", name),
        ).await;
    }

    sqlx::query("DELETE FROM official_credentials WHERE id = ?")
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn reorder_credentials(
    db: State<'_, SqlitePool>,
    ids: Vec<i64>,
) -> Result<()> {
    // 获取第一个凭证的 cli_type（用于后续同步）
    let cli_type: Option<(String,)> = if !ids.is_empty() {
        sqlx::query_as("SELECT cli_type FROM official_credentials WHERE id = ?")
            .bind(ids[0])
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?
    } else {
        None
    };

    for (index, id) in ids.iter().enumerate() {
        sqlx::query("UPDATE official_credentials SET sort_order = ? WHERE id = ?")
            .bind(index as i64)
            .bind(id)
            .execute(db.inner())
            .await
            .map_err(|e| e.to_string())?;
    }

    // 如果是直连模式，自动同步到文件
    if let Some((cli_type_str,)) = cli_type {
        if let Err(e) = auto_sync_credential_in_direct_mode(db.inner(), &cli_type_str).await {
            tracing::error!("自动同步凭证失败: {}", e);
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn read_cli_credential(db: State<'_, SqlitePool>, cli_type: String) -> Result<String> {
    read_cli_credential_impl_async(db.inner(), &cli_type).await
}

#[tauri::command]
pub async fn get_cli_mode(db: State<'_, SqlitePool>, cli_type: String) -> Result<String> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT cli_mode FROM cli_settings WHERE cli_type = ?",
    )
    .bind(&cli_type)
    .fetch_optional(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(row.map(|r| r.0).unwrap_or_else(|| "proxy".to_string()))
}

#[tauri::command]
pub async fn set_cli_mode(
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    cli_type: String,
    mode: String,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    let current_mode: Option<(String,)> = sqlx::query_as(
        "SELECT cli_mode FROM cli_settings WHERE cli_type = ?",
    )
    .bind(&cli_type)
    .fetch_optional(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    let current_mode = current_mode.map(|r| r.0).unwrap_or_else(|| "proxy".to_string());

    if current_mode == mode {
        return Ok(());
    }

    sqlx::query("UPDATE cli_settings SET cli_mode = ?, updated_at = ? WHERE cli_type = ?")
        .bind(&mode)
        .bind(now)
        .bind(&cli_type)
        .execute(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    if mode == "direct" {
        // 步骤1: 如果从中转模式切换过来，先关闭 CLI
        if current_mode == "proxy" {
            // 检查是否真的有中转配置（通过检查配置文件）
            let has_gateway_config = check_cli_enabled(db.inner(), &cli_type).await;
            
            if has_gateway_config {
                let default_config = sqlx::query_as::<_, (Option<String>,)>(
                    "SELECT default_json_config FROM cli_settings WHERE cli_type = ?",
                )
                .bind(&cli_type)
                .fetch_optional(db.inner())
                .await
                .map_err(|e| e.to_string())?
                .and_then(|r| r.0)
                .unwrap_or_default();

                tracing::info!("{} 从中转模式切换到直连模式，先关闭 CLI", cli_type);
                // 关闭中转模式（会自动处理备份恢复）
                sync_cli_config(db.inner(), &cli_type, false, &default_config).await?;
            } else {
                tracing::info!("{} 当前没有中转配置，跳过关闭 CLI 步骤", cli_type);
            }
        }

        // 步骤2: 应用直连模式配置
        if let Ok(Some(cred)) = sqlx::query_as::<_, OfficialCredential>(
            "SELECT * FROM official_credentials WHERE cli_type = ? AND sort_order = 0 LIMIT 1",
        )
        .bind(&cli_type)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())
        {
            let default_config = sqlx::query_as::<_, (Option<String>,)>(
                "SELECT default_json_config FROM cli_settings WHERE cli_type = ?",
            )
            .bind(&cli_type)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?
            .and_then(|r| r.0)
            .unwrap_or_default();

            tracing::info!("开始同步 {} 凭证到文件", cli_type);
            match sync_credential_to_cli_async(db.inner(), &cli_type, &cred.credential_json, &default_config).await {
                Ok(_) => {
                    tracing::info!("{} 凭证同步成功", cli_type);
                }
                Err(e) => {
                    tracing::error!("{} 凭证同步失败: {}", cli_type, e);
                    return Err(format!("同步凭证失败: {}", e));
                }
            }
        } else {
            tracing::warn!("{} 没有可用的凭证", cli_type);
        }

        let _ = crate::services::stats::record_system_log(
            &log_db.0,
            "cli_mode_changed",
            &format!("{} 已切换到直连模式", cli_type),
        ).await;
    } else {
        // 切换到中转模式
        
        // 步骤1: 如果从直连模式切换过来，先删除直连模式的文件
        if current_mode == "direct" {
            tracing::info!("{} 从直连模式切换到中转模式，先删除直连模式文件", cli_type);
            remove_direct_mode_files_async(db.inner(), &cli_type).await?;
        }
        
        // 步骤2: 开启中转模式
        let default_config = sqlx::query_as::<_, (Option<String>,)>(
            "SELECT default_json_config FROM cli_settings WHERE cli_type = ?",
        )
        .bind(&cli_type)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .and_then(|r| r.0)
        .unwrap_or_default();

        sync_cli_config(db.inner(), &cli_type, true, &default_config).await?;

        let _ = crate::services::stats::record_system_log(
            &log_db.0,
            "cli_mode_changed",
            &format!("{} 已切换到中转模式", cli_type),
        ).await;
    }

    Ok(())
}
