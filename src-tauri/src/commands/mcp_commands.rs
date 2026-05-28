use super::*;

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
        for cli_type in CliType::ALL.iter().map(CliType::as_str) {
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
    for cli_type in CliType::ALL.iter().map(CliType::as_str) {
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
    let cli_flags = input.cli_flags.unwrap_or_default();
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
        for cli_type in CliType::ALL.iter().map(CliType::as_str) {
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

    if cli_flags.iter().any(|f| f.cli_type == "codex" && f.enabled) {
        parse_codex_mcp_toml_table(&new_config)?;
    }

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
    let path = get_mcp_config_path(db, cli_type)
        .await
        .ok_or_else(|| format!("Invalid CLI type: {}", cli_type))?;

    if !is_enabled && !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Ok(());
    }

    if cli_type == "codex" {
        sync_single_codex_mcp(path, mcp_name, mcp_config_json, is_enabled).await?;
        return Ok(());
    }

    let mut config = if tokio::fs::try_exists(&path).await.unwrap_or(false) {
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| e.to_string())?;
        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(config) => config,
            Err(e) => {
                if is_enabled {
                    return Err(format!("{} 解析失败，未写入: {}", path.display(), e));
                }
                tracing::warn!(
                    "Failed to parse {}, leaving file untouched: {}",
                    path.display(),
                    e
                );
                return Ok(());
            }
        }
    } else {
        serde_json::json!({})
    };

    if is_enabled {
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
    } else if let Some(obj) = config.as_object_mut() {
        if let Some(servers) = obj.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
            servers.remove(mcp_name);
        }
    }

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| e.to_string())?;
    }
    let config_str = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    tokio::fs::write(&path, config_str)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
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
    for cli_type in CliType::ALL.iter().map(CliType::as_str) {
        let is_enabled = cli_flags
            .iter()
            .any(|f| f.cli_type == cli_type && f.enabled);

        sync_mcp_to_cli_async(db, mcp_name, mcp_config_json, cli_type, is_enabled).await?;
    }

    Ok(())
}

// Helper function to sync a single MCP to Codex config.toml
async fn sync_single_codex_mcp(
    config_path: std::path::PathBuf,
    mcp_name: &str,
    mcp_config_json: &str,
    is_enabled: bool,
) -> Result<()> {
    // Read existing TOML or create new one
    let mut doc = if tokio::fs::try_exists(&config_path).await.unwrap_or(false) {
        let content = tokio::fs::read_to_string(&config_path).await.map_err(|e| {
            tracing::error!("Failed to read config.toml: {}", e);
            e.to_string()
        })?;
        match content.parse::<toml_edit::DocumentMut>() {
            Ok(doc) => doc,
            Err(e) => {
                if is_enabled {
                    return Err(format!("{} 解析失败，未写入: {}", config_path.display(), e));
                }
                tracing::warn!(
                    "Failed to parse {}, leaving file untouched: {}",
                    config_path.display(),
                    e
                );
                return Ok(());
            }
        }
    } else {
        toml_edit::DocumentMut::new()
    };

    // Ensure mcp_servers table exists
    if !doc.contains_table("mcp_servers") {
        doc["mcp_servers"] = toml_edit::table();
    }

    if is_enabled {
        let server_table = parse_codex_mcp_toml_table(mcp_config_json)?;
        doc["mcp_servers"][mcp_name] = toml_edit::Item::Table(server_table);
    } else {
        // Remove this MCP by name
        if let Some(table) = doc.get_mut("mcp_servers").and_then(|v| v.as_table_mut()) {
            table.remove(mcp_name);
        }
    }

    // Write config file
    if let Some(parent) = config_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            tracing::error!("Failed to create directory: {}", e);
            e.to_string()
        })?;
    }
    tokio::fs::write(&config_path, doc.to_string())
        .await
        .map_err(|e| {
            tracing::error!("Failed to write config.toml: {}", e);
            e.to_string()
        })?;

    Ok(())
}

// Delete a single MCP from all CLI configs
async fn delete_mcp_from_cli(db: &SqlitePool, mcp_name: &str) -> Result<()> {
    for cli_type in CliType::ALL.iter().map(CliType::as_str) {
        let config_path = get_mcp_config_path(db, cli_type).await;
        if let Some(path) = config_path {
            if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
                continue;
            }

            if cli_type == "codex" {
                // Handle Codex TOML format
                let content = tokio::fs::read_to_string(&path)
                    .await
                    .map_err(|e| e.to_string())?;
                let mut doc = match content.parse::<toml_edit::DocumentMut>() {
                    Ok(doc) => doc,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to parse {}, leaving file untouched: {}",
                            path.display(),
                            e
                        );
                        continue;
                    }
                };

                if let Some(table) = doc["mcp_servers"].as_table_mut() {
                    table.remove(mcp_name);
                }

                tokio::fs::write(&path, doc.to_string())
                    .await
                    .map_err(|e| e.to_string())?;
            } else {
                // Handle Claude/Gemini JSON format
                let content = tokio::fs::read_to_string(&path)
                    .await
                    .map_err(|e| e.to_string())?;
                let mut config: serde_json::Value = match serde_json::from_str(&content) {
                    Ok(config) => config,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to parse {}, leaving file untouched: {}",
                            path.display(),
                            e
                        );
                        continue;
                    }
                };

                if let Some(mcp_servers) =
                    config.get_mut("mcpServers").and_then(|v| v.as_object_mut())
                {
                    mcp_servers.remove(mcp_name);
                }

                let config_str =
                    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
                tokio::fs::write(&path, config_str)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(())
}
