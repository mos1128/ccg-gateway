use super::*;

#[tauri::command]
pub async fn get_gateway_settings(db: State<'_, SqlitePool>) -> Result<GatewaySettings> {
    sqlx::query_as::<_, GatewaySettings>(
        "SELECT debug_log, log_detail_mode FROM gateway_settings WHERE id = 1",
    )
    .fetch_one(db.inner())
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_gateway_settings(
    db: State<'_, SqlitePool>,
    debug_log: Option<bool>,
    log_detail_mode: Option<String>,
) -> Result<()> {
    let now = now_timestamp();

    let mut updates = Vec::new();
    if debug_log.is_some() {
        updates.push("debug_log = ?");
    }
    if log_detail_mode.is_some() {
        updates.push("log_detail_mode = ?");
    }
    updates.push("updated_at = ?");

    let sql = format!(
        "UPDATE gateway_settings SET {} WHERE id = 1",
        updates.join(", ")
    );
    let mut query = sqlx::query(&sql);

    if let Some(debug_log) = debug_log {
        query = query.bind(if debug_log { 1i64 } else { 0 });
    }
    if let Some(mode) = log_detail_mode {
        query = query.bind(mode);
    }

    query
        .bind(now)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

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
    let now = now_timestamp();
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
    .map_err(map_db_error)?;
    Ok(())
}

#[tauri::command]
pub async fn get_cli_settings(
    db: State<'_, SqlitePool>,
    config: State<'_, Config>,
    cli_type: String,
) -> Result<CliSettingsResponse> {
    let gateway_url = config.gateway_base_url();
    let row = sqlx::query_as::<_, CliSettingsRowWithoutConfigDir>(
        "SELECT cli_type, default_json_config, cli_mode, config_write_mode, updated_at FROM cli_settings WHERE cli_type = ?",
    )
    .bind(&cli_type)
    .fetch_optional(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    let config_dir = get_cli_config_dir_path(db.inner(), &cli_type)
        .await
        .to_string_lossy()
        .to_string();
    let default_config_dir = get_default_cli_config_dir(&cli_type)
        .to_string_lossy()
        .to_string();

    if let Some(row) = row {
        let enabled = check_cli_enabled(db.inner(), &cli_type, &gateway_url).await;

        Ok(CliSettingsResponse {
            cli_type: row.cli_type,
            enabled,
            default_json_config: row.default_json_config.unwrap_or_default(),
            cli_mode: row.cli_mode,
            config_dir,
            default_config_dir,
            config_write_mode: row.config_write_mode,
        })
    } else {
        Ok(CliSettingsResponse {
            cli_type: cli_type.clone(),
            enabled: false,
            default_json_config: String::new(),
            cli_mode: "proxy".to_string(),
            config_dir,
            default_config_dir,
            config_write_mode: "merge".to_string(),
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct CliSettingsRowWithoutConfigDir {
    pub cli_type: String,
    pub default_json_config: Option<String>,
    pub cli_mode: String,
    pub config_write_mode: String,
    pub updated_at: i64,
}

#[tauri::command]
pub async fn get_claude_profile_settings_status(
    db: State<'_, SqlitePool>,
    config: State<'_, Config>,
    profile: String,
) -> Result<ClaudeProfileSettingsStatus> {
    let profile = validate_provider_profile(Some(&profile))?.to_string();
    claude_profile_settings_status(db.inner(), &profile, &config.gateway_base_url()).await
}

#[tauri::command]
pub async fn ensure_claude_profile_settings(
    db: State<'_, SqlitePool>,
    config: State<'_, Config>,
    profile: String,
) -> Result<ClaudeProfileSettingsStatus> {
    let profile = validate_provider_profile(Some(&profile))?.to_string();
    let gateway_url = config.gateway_base_url();
    if profile == DEFAULT_PROFILE {
        return claude_profile_settings_status(db.inner(), &profile, &gateway_url).await;
    }

    let config_dir = get_cli_config_dir_path(db.inner(), "claude_code").await;
    let config_path = config_dir.join(cli_helpers::claude_settings_filename(&profile));
    let gateway_token = gateway_token_for_profile(&profile).unwrap_or("ccg-gateway");
    let use_merge = get_config_write_mode(db.inner(), "claude_code").await == "merge";

    write_claude_gateway_settings(
        &config_path,
        "",
        None,
        use_merge,
        &gateway_url,
        gateway_token,
    )
    .await?;

    claude_profile_settings_status(db.inner(), &profile, &gateway_url).await
}

#[tauri::command]
pub async fn get_codex_profile_settings_status(
    db: State<'_, SqlitePool>,
    config: State<'_, Config>,
    profile: String,
) -> Result<CodexProfileSettingsStatus> {
    let profile = validate_provider_profile(Some(&profile))?.to_string();
    codex_profile_settings_status(db.inner(), &profile, &config.gateway_base_url()).await
}

#[tauri::command]
pub async fn ensure_codex_profile_settings(
    db: State<'_, SqlitePool>,
    config: State<'_, Config>,
    profile: String,
) -> Result<CodexProfileSettingsStatus> {
    let profile = validate_provider_profile(Some(&profile))?.to_string();
    let gateway_url = config.gateway_base_url();

    if profile == DEFAULT_PROFILE {
        return codex_profile_settings_status(db.inner(), &profile, &gateway_url).await;
    }

    let codex_dir = get_cli_config_dir_path(db.inner(), "codex").await;
    let config_path = codex_dir.join("config.toml");
    let profile_path = codex_profile_config_path(&codex_dir, &profile);
    let profile_filename = codex_profile_config_filename(&profile);

    tokio::fs::create_dir_all(&codex_dir).await.map_err(|e| {
        tracing::error!("Failed to create Codex directory: {}", e);
        e.to_string()
    })?;

    let mut profile_doc = if tokio::fs::try_exists(&profile_path).await.unwrap_or(false) {
        let content = tokio::fs::read_to_string(&profile_path)
            .await
            .map_err(|e| {
                tracing::error!("Failed to read {}: {}", profile_filename, e);
                e.to_string()
            })?;
        content.parse::<toml_edit::DocumentMut>().map_err(|e| {
            format!(
                "Codex {} TOML 格式错误，未写入 Profile 配置: {}",
                profile_filename, e
            )
        })?
    } else {
        toml_edit::DocumentMut::new()
    };

    let mut base_doc = if tokio::fs::try_exists(&config_path).await.unwrap_or(false) {
        let content = tokio::fs::read_to_string(&config_path).await.map_err(|e| {
            tracing::error!("Failed to read config.toml: {}", e);
            e.to_string()
        })?;
        Some(content.parse::<toml_edit::DocumentMut>().map_err(|e| {
            format!(
                "Codex config.toml TOML 格式错误，未迁移旧 Profile 配置: {}",
                e
            )
        })?)
    } else {
        None
    };

    let base_changed = base_doc
        .as_mut()
        .map(|doc| migrate_codex_legacy_profile_config(doc, &mut profile_doc, &profile))
        .unwrap_or(false);

    apply_codex_gateway_named_profile_config(&mut profile_doc, &gateway_url, &profile)?;
    tokio::fs::write(&profile_path, profile_doc.to_string())
        .await
        .map_err(|e| {
            tracing::error!("Failed to write {}: {}", profile_filename, e);
            e.to_string()
        })?;

    if base_changed {
        if let Some(base_doc) = base_doc {
            tokio::fs::write(&config_path, base_doc.to_string())
                .await
                .map_err(|e| {
                    tracing::error!("Failed to write config.toml: {}", e);
                    e.to_string()
                })?;
        }
    }

    codex_profile_settings_status(db.inner(), &profile, &gateway_url).await
}

#[tauri::command]
pub async fn update_cli_settings(
    db: State<'_, SqlitePool>,
    config: State<'_, Config>,
    cli_type: String,
    input: CliSettingsUpdate,
) -> Result<()> {
    let now = now_timestamp();
    let gateway_url = config.gateway_base_url();
    let config_trimmed = input
        .default_json_config
        .as_ref()
        .map(|config| config.trim().to_string());

    if let Some(ref config_trimmed) = config_trimmed {
        if !config_trimmed.is_empty() {
            cli_helpers::validate_config_format(&cli_type, config_trimmed)?;
        }
    }

    if let Some(ref write_mode) = input.config_write_mode {
        if write_mode != "overwrite" && write_mode != "merge" {
            return Err("config_write_mode 只能是 'overwrite' 或 'merge'".to_string());
        }
    }

    if let Some(ref write_mode) = input.config_write_mode {
        sqlx::query(
            "UPDATE cli_settings SET config_write_mode = ?, updated_at = ? WHERE cli_type = ?",
        )
        .bind(write_mode)
        .bind(now)
        .bind(&cli_type)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;
    }

    if let Some(ref config_dir) = input.config_dir {
        let shrunk_path = shrink_home_path(config_dir);
        sqlx::query("UPDATE cli_settings SET config_dir = ?, updated_at = ? WHERE cli_type = ?")
            .bind(&shrunk_path)
            .bind(now)
            .bind(&cli_type)
            .execute(db.inner())
            .await
            .map_err(map_db_error)?;
    }

    if let Some(ref config_trimmed) = config_trimmed {
        let previous_default_config = sqlx::query_as::<_, (Option<String>,)>(
            "SELECT default_json_config FROM cli_settings WHERE cli_type = ?",
        )
        .bind(&cli_type)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .and_then(|r| r.0)
        .unwrap_or_default();

        sqlx::query(
            "UPDATE cli_settings SET default_json_config = ?, updated_at = ? WHERE cli_type = ?",
        )
        .bind(config_trimmed)
        .bind(now)
        .bind(&cli_type)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

        let mode: String =
            sqlx::query_as::<_, (String,)>("SELECT cli_mode FROM cli_settings WHERE cli_type = ?")
                .bind(&cli_type)
                .fetch_optional(db.inner())
                .await
                .map_err(|e| e.to_string())?
                .map(|r| r.0)
                .unwrap_or_else(|| "proxy".to_string());

        if cli_type == "claude_code" {
            sync_claude_code_preset_update(
                db.inner(),
                config_trimmed,
                &previous_default_config,
                &gateway_url,
            )
            .await?;

            if mode == "direct" {
                tracing::info!("{} 直连模式，配置变更后自动同步凭证配置", cli_type);
                credential_commands::auto_sync_credential_in_direct_mode(
                    db.inner(),
                    &cli_type,
                    Some(&previous_default_config),
                )
                .await?;
            }
        } else if mode == "proxy" {
            let enabled = check_cli_enabled(db.inner(), &cli_type, &gateway_url).await;
            if enabled {
                tracing::info!("{} CLI 已启用，配置变更后自动同步配置文件", cli_type);
                sync_cli_config(
                    db.inner(),
                    &cli_type,
                    true,
                    config_trimmed,
                    Some(&previous_default_config),
                    &gateway_url,
                )
                .await?;
            }
        } else {
            tracing::info!("{} 直连模式，配置变更后自动同步凭证配置", cli_type);
            credential_commands::auto_sync_credential_in_direct_mode(
                db.inner(),
                &cli_type,
                Some(&previous_default_config),
            )
            .await?;
        }
    }

    if let Some(enabled) = input.enabled {
        let current_mode: Option<(String,)> =
            sqlx::query_as("SELECT cli_mode FROM cli_settings WHERE cli_type = ?")
                .bind(&cli_type)
                .fetch_optional(db.inner())
                .await
                .map_err(|e| e.to_string())?;

        let mode = current_mode
            .map(|r| r.0)
            .unwrap_or_else(|| "proxy".to_string());

        if mode == "proxy" {
            let current_enabled = check_cli_enabled(db.inner(), &cli_type, &gateway_url).await;

            if current_enabled == enabled {
                tracing::info!(
                    "{} CLI 已经处于目标状态（enabled={}），跳过操作",
                    cli_type,
                    enabled
                );
            } else {
                let row = sqlx::query_as::<_, CliSettingsRowWithoutConfigDir>(
                    "SELECT cli_type, default_json_config, cli_mode, config_write_mode, updated_at FROM cli_settings WHERE cli_type = ?",
                )
                .bind(&cli_type)
                .fetch_optional(db.inner())
                .await
                .map_err(|e| e.to_string())?;

                let default_config = row
                    .as_ref()
                    .and_then(|r| r.default_json_config.clone())
                    .unwrap_or_default();
                tracing::info!("{} 执行 CLI 状态切换：enabled={}", cli_type, enabled);
                sync_cli_config(
                    db.inner(),
                    &cli_type,
                    enabled,
                    &default_config,
                    None,
                    &gateway_url,
                )
                .await?;
            }
        } else {
            tracing::info!("{} 处于直连模式，忽略 enabled 参数", cli_type);
        }
    }

    Ok(())
}
