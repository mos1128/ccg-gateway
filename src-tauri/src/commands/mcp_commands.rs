use super::*;

struct McpTarget {
    path: std::path::PathBuf,
    format: crate::db::models::ConfigFormat,
    servers_path: Vec<String>,
}

async fn resolve_mcp_target(db: &SqlitePool, agent_id: &str) -> Result<McpTarget> {
    let agent = crate::services::agent::get_agent(db, agent_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("未知 Agent: {}", agent_id))?;
    let feature = agent.features.mcp;
    if !feature.enabled {
        return Err(format!("Agent {} 的 MCP 功能不可用", agent_id));
    }
    let file = feature
        .file
        .as_deref()
        .ok_or_else(|| format!("Agent {} 的 MCP 缺少 file", agent_id))?;
    let format = feature
        .format
        .ok_or_else(|| format!("Agent {} 的 MCP 缺少 format", agent_id))?;
    if !matches!(
        format,
        crate::db::models::ConfigFormat::Json | crate::db::models::ConfigFormat::Toml
    ) {
        return Err(format!(
            "Agent {} 的 MCP format 只支持 json 或 toml",
            agent_id
        ));
    }
    if feature.servers_path.is_empty() {
        return Err(format!("Agent {} 的 MCP 缺少 servers_path", agent_id));
    }
    Ok(McpTarget {
        path: crate::services::cli_config::resolve_cli_config_file(db, agent_id, file).await,
        format,
        servers_path: feature.servers_path,
    })
}

fn json_value_at<'a>(
    value: &'a serde_json::Value,
    path: &[String],
) -> Option<&'a serde_json::Value> {
    path.iter().try_fold(value, |current, key| current.get(key))
}

fn json_object_at_mut<'a>(
    value: &'a mut serde_json::Value,
    path: &[String],
) -> Option<&'a mut serde_json::Map<String, serde_json::Value>> {
    let current = path
        .iter()
        .try_fold(value, |current, key| current.get_mut(key))?;
    current.as_object_mut()
}

fn ensure_json_object<'a>(
    value: &'a mut serde_json::Value,
    path: &[String],
) -> Result<&'a mut serde_json::Map<String, serde_json::Value>> {
    let mut current = value;
    for key in path {
        let object = current
            .as_object_mut()
            .ok_or_else(|| format!("MCP 路径 `{}` 不是对象", path.join(".")))?;
        current = object
            .entry(key.clone())
            .or_insert_with(|| serde_json::json!({}));
    }
    current
        .as_object_mut()
        .ok_or_else(|| format!("MCP 路径 `{}` 不是对象", path.join(".")))
}

fn toml_table_at<'a>(table: &'a toml_edit::Table, path: &[String]) -> Option<&'a toml_edit::Table> {
    path.iter().try_fold(table, |current, key| {
        current.get(key).and_then(toml_edit::Item::as_table)
    })
}

fn toml_table_at_mut<'a>(
    table: &'a mut toml_edit::Table,
    path: &[String],
) -> Option<&'a mut toml_edit::Table> {
    let mut current = table;
    for key in path {
        current = current.get_mut(key)?.as_table_mut()?;
    }
    Some(current)
}

fn ensure_toml_table<'a>(
    table: &'a mut toml_edit::Table,
    path: &[String],
) -> Result<&'a mut toml_edit::Table> {
    let mut current = table;
    for key in path {
        if !current.contains_key(key) {
            current.insert(key, toml_edit::Item::Table(toml_edit::Table::new()));
        }
        current = current
            .get_mut(key)
            .and_then(toml_edit::Item::as_table_mut)
            .ok_or_else(|| format!("MCP 路径 `{}` 不是 TOML table", path.join(".")))?;
    }
    Ok(current)
}

async fn mcp_enabled_in_file_async(db: &SqlitePool, agent_id: &str, mcp_name: &str) -> bool {
    let Ok(target) = resolve_mcp_target(db, agent_id).await else {
        return false;
    };
    let Ok(content) = tokio::fs::read_to_string(&target.path).await else {
        return false;
    };
    match target.format {
        crate::db::models::ConfigFormat::Json => {
            serde_json::from_str::<serde_json::Value>(&content)
                .ok()
                .and_then(|config| {
                    json_value_at(&config, &target.servers_path)
                        .and_then(serde_json::Value::as_object)
                        .map(|servers| servers.contains_key(mcp_name))
                })
                .unwrap_or(false)
        }
        crate::db::models::ConfigFormat::Toml => content
            .parse::<toml_edit::DocumentMut>()
            .ok()
            .and_then(|document| {
                toml_table_at(document.as_table(), &target.servers_path)
                    .map(|servers| servers.contains_key(mcp_name))
            })
            .unwrap_or(false),
        _ => false,
    }
}

async fn validate_mcp_config_for_flags(
    db: &SqlitePool,
    config_json: &str,
    cli_flags: &[McpCliFlag],
) -> Result<()> {
    for flag in cli_flags.iter().filter(|flag| flag.enabled) {
        let target = resolve_mcp_target(db, &flag.cli_type).await?;
        if target.format == crate::db::models::ConfigFormat::Toml {
            parse_mcp_toml_table(config_json)?;
        }
    }
    Ok(())
}

// MCP commands
#[tauri::command]
pub async fn get_mcps(db: State<'_, SqlitePool>) -> Result<Vec<McpResponse>> {
    let mcps = sqlx::query_as::<_, McpConfig>("SELECT * FROM mcp_configs ORDER BY id")
        .fetch_all(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for mcp in mcps {
        let mut cli_flags = Vec::new();
        for cli_type in crate::services::agent::agent_ids_for_feature("mcp") {
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

    let mut cli_flags = Vec::new();
    for cli_type in crate::services::agent::agent_ids_for_feature("mcp") {
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
    let now = now_timestamp();

    // Validate JSON format if config_json is not empty
    let config_trimmed = input.config_json.trim();
    if !config_trimmed.is_empty() {
        serde_json::from_str::<serde_json::Value>(config_trimmed)
            .map_err(|e| format!("JSON 格式错误: {}", e))?;
    }
    let cli_flags = input.cli_flags.unwrap_or_default();
    validate_mcp_config_for_flags(db.inner(), config_trimmed, &cli_flags).await?;

    let result =
        sqlx::query("INSERT INTO mcp_configs (name, config_json, updated_at) VALUES (?, ?, ?)")
            .bind(&input.name)
            .bind(config_trimmed)
            .bind(now)
            .execute(db.inner())
            .await
            .map_err(map_db_error)?;

    let id = result.last_insert_rowid();

    // Sync to CLI files if cli_flags provided
    if !cli_flags.is_empty() {
        sync_single_mcp_to_cli(db.inner(), id, &input.name, config_trimmed, &cli_flags).await?;
    }

    get_mcp(db, id).await
}

#[tauri::command]
pub async fn update_mcp(
    db: State<'_, SqlitePool>,
    id: i64,
    input: McpUpdate,
) -> Result<McpResponse> {
    let now = now_timestamp();

    let current = sqlx::query_as::<_, McpConfig>("SELECT * FROM mcp_configs WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "MCP not found".to_string())?;

    // Validate JSON format if config_json is provided and not empty
    if let Some(ref config) = input.config_json {
        let config_trimmed = config.trim();
        if !config_trimmed.is_empty() {
            serde_json::from_str::<serde_json::Value>(config_trimmed)
                .map_err(|e| format!("JSON 格式错误: {}", e))?;
        }
    }

    let current_cli_flags = {
        let mut flags = Vec::new();
        for cli_type in crate::services::agent::agent_ids_for_feature("mcp") {
            flags.push(McpCliFlag {
                cli_type: cli_type.to_string(),
                enabled: mcp_enabled_in_file_async(db.inner(), cli_type, &current.name).await,
            });
        }
        flags
    };

    let has_explicit_cli_flags = input.cli_flags.is_some();
    let new_name = input.name.unwrap_or_else(|| current.name.clone());
    let new_config = input
        .config_json
        .map(|c| c.trim().to_string())
        .unwrap_or_else(|| current.config_json.clone());
    let cli_flags = input.cli_flags.unwrap_or(current_cli_flags);

    validate_mcp_config_for_flags(db.inner(), &new_config, &cli_flags).await?;

    if new_name != current.name || new_config != current.config_json {
        sqlx::query(
            "UPDATE mcp_configs SET name = ?, config_json = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&new_name)
        .bind(&new_config)
        .bind(now)
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;
    }

    if new_name != current.name {
        delete_mcp_from_cli(db.inner(), &current.name).await?;
    }

    if has_explicit_cli_flags {
        sync_single_mcp_to_cli(db.inner(), id, &new_name, &new_config, &cli_flags).await?;
    } else {
        sync_enabled_mcp_to_cli(db.inner(), &new_name, &new_config, &cli_flags).await?;
    }

    get_mcp(db, id).await
}

#[tauri::command]
pub async fn toggle_mcp_cli(
    db: State<'_, SqlitePool>,
    id: i64,
    cli_type: String,
    enabled: bool,
) -> Result<McpResponse> {
    let mcp = sqlx::query_as::<_, McpConfig>("SELECT * FROM mcp_configs WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "MCP not found".to_string())?;

    sync_mcp_to_cli_async(db.inner(), &mcp.name, &mcp.config_json, &cli_type, enabled).await?;

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
        .map_err(map_db_error)?;

    // Remove from all CLI configs
    delete_mcp_from_cli(db.inner(), &mcp_name).await?;

    Ok(())
}

async fn sync_mcp_to_cli_async(
    db: &SqlitePool,
    mcp_name: &str,
    mcp_config_json: &str,
    cli_type: &str,
    is_enabled: bool,
) -> Result<()> {
    let target = resolve_mcp_target(db, cli_type).await?;
    if !is_enabled && !tokio::fs::try_exists(&target.path).await.unwrap_or(false) {
        return Ok(());
    }
    match target.format {
        crate::db::models::ConfigFormat::Json => {
            sync_json_mcp(&target, mcp_name, mcp_config_json, is_enabled).await
        }
        crate::db::models::ConfigFormat::Toml => {
            sync_toml_mcp(&target, mcp_name, mcp_config_json, is_enabled).await
        }
        _ => Err("MCP format 只支持 json 或 toml".to_string()),
    }
}

async fn sync_json_mcp(
    target: &McpTarget,
    mcp_name: &str,
    mcp_config_json: &str,
    is_enabled: bool,
) -> Result<()> {
    let mut config = if tokio::fs::try_exists(&target.path).await.unwrap_or(false) {
        let content = tokio::fs::read_to_string(&target.path)
            .await
            .map_err(|error| error.to_string())?;
        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(config) => config,
            Err(error) if is_enabled => {
                return Err(format!(
                    "{} 解析失败，未写入: {}",
                    target.path.display(),
                    error
                ));
            }
            Err(error) => {
                tracing::warn!(
                    "Failed to parse {}, leaving file untouched: {}",
                    target.path.display(),
                    error
                );
                return Ok(());
            }
        }
    } else {
        serde_json::json!({})
    };

    if is_enabled {
        let mcp_config = serde_json::from_str::<serde_json::Value>(mcp_config_json)
            .map_err(|error| format!("MCP JSON 格式错误: {}", error))?;
        ensure_json_object(&mut config, &target.servers_path)?
            .insert(mcp_name.to_string(), mcp_config);
    } else if let Some(servers) = json_object_at_mut(&mut config, &target.servers_path) {
        servers.remove(mcp_name);
    } else {
        return Ok(());
    }

    if let Some(parent) = target.path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| error.to_string())?;
    }
    let content = serde_json::to_string_pretty(&config).map_err(|error| error.to_string())?;
    tokio::fs::write(&target.path, content)
        .await
        .map_err(|error| error.to_string())
}

async fn sync_toml_mcp(
    target: &McpTarget,
    mcp_name: &str,
    mcp_config_json: &str,
    is_enabled: bool,
) -> Result<()> {
    let mut document = if tokio::fs::try_exists(&target.path).await.unwrap_or(false) {
        let content = tokio::fs::read_to_string(&target.path)
            .await
            .map_err(|error| error.to_string())?;
        match content.parse::<toml_edit::DocumentMut>() {
            Ok(document) => document,
            Err(error) if is_enabled => {
                return Err(format!(
                    "{} 解析失败，未写入: {}",
                    target.path.display(),
                    error
                ));
            }
            Err(error) => {
                tracing::warn!(
                    "Failed to parse {}, leaving file untouched: {}",
                    target.path.display(),
                    error
                );
                return Ok(());
            }
        }
    } else {
        toml_edit::DocumentMut::new()
    };

    if is_enabled {
        let server = parse_mcp_toml_table(mcp_config_json)?;
        ensure_toml_table(document.as_table_mut(), &target.servers_path)?
            .insert(mcp_name, toml_edit::Item::Table(server));
    } else if let Some(servers) = toml_table_at_mut(document.as_table_mut(), &target.servers_path) {
        servers.remove(mcp_name);
    } else {
        return Ok(());
    }

    if let Some(parent) = target.path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| error.to_string())?;
    }
    tokio::fs::write(&target.path, document.to_string())
        .await
        .map_err(|error| error.to_string())
}

async fn sync_enabled_mcp_to_cli(
    db: &SqlitePool,
    mcp_name: &str,
    mcp_config_json: &str,
    cli_flags: &[McpCliFlag],
) -> Result<()> {
    for flag in cli_flags.iter().filter(|f| f.enabled) {
        sync_mcp_to_cli_async(db, mcp_name, mcp_config_json, &flag.cli_type, true).await?;
    }

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
    for cli_type in crate::services::agent::agent_ids_for_feature("mcp") {
        let is_enabled = cli_flags
            .iter()
            .any(|f| f.cli_type == cli_type && f.enabled);

        sync_mcp_to_cli_async(db, mcp_name, mcp_config_json, cli_type, is_enabled).await?;
    }

    Ok(())
}

// Delete a single MCP from all CLI configs
async fn delete_mcp_from_cli(db: &SqlitePool, mcp_name: &str) -> Result<()> {
    for cli_type in crate::services::agent::agent_ids_for_feature("mcp") {
        sync_mcp_to_cli_async(db, mcp_name, "{}", cli_type, false).await?;
    }

    Ok(())
}
