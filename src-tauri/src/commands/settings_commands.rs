use super::*;

#[tauri::command]
pub async fn get_gateway_settings(db: State<'_, SqlitePool>) -> Result<GatewaySettings> {
    sqlx::query_as::<_, GatewaySettings>(
        "SELECT debug_log, log_detail_mode, launch_on_startup, silent_startup, minimize_to_tray_on_close FROM gateway_settings WHERE id = 1",
    )
    .fetch_one(db.inner())
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_gateway_settings(
    app: tauri::AppHandle,
    db: State<'_, SqlitePool>,
    debug_log: Option<bool>,
    log_detail_mode: Option<String>,
    launch_on_startup: Option<bool>,
    silent_startup: Option<bool>,
    minimize_to_tray_on_close: Option<bool>,
) -> Result<()> {
    let now = now_timestamp();

    let mut updates = Vec::new();
    if debug_log.is_some() {
        updates.push("debug_log = ?");
    }
    if log_detail_mode.is_some() {
        updates.push("log_detail_mode = ?");
    }
    if launch_on_startup.is_some() {
        updates.push("launch_on_startup = ?");
    }
    if silent_startup.is_some() {
        updates.push("silent_startup = ?");
    }
    if minimize_to_tray_on_close.is_some() {
        updates.push("minimize_to_tray_on_close = ?");
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
    if let Some(launch_on_startup) = launch_on_startup {
        if launch_on_startup {
            crate::auto_launch::enable_auto_launch()?;
        } else {
            crate::auto_launch::disable_auto_launch()?;
        }
        query = query.bind(if launch_on_startup { 1i64 } else { 0 });
    }
    if let Some(silent_startup) = silent_startup {
        query = query.bind(if silent_startup { 1i64 } else { 0 });
    }
    if let Some(minimize_to_tray_on_close) = minimize_to_tray_on_close {
        crate::set_minimize_to_tray_on_close(&app, minimize_to_tray_on_close);
        query = query.bind(if minimize_to_tray_on_close { 1i64 } else { 0 });
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
            cli_mode: normalize_cli_mode(&row.cli_mode)
                .unwrap_or(CLI_MODE_PROXY_ROUTE)
                .to_string(),
            config_dir,
            default_config_dir,
            config_write_mode: row.config_write_mode,
        })
    } else {
        Ok(CliSettingsResponse {
            cli_type: cli_type.clone(),
            enabled: false,
            default_json_config: String::new(),
            cli_mode: CLI_MODE_PROXY_ROUTE.to_string(),
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

async fn collect_provider_direct_rewrite_ids(db: &SqlitePool, cli_type: &str) -> Result<Vec<i64>> {
    let mut ids = Vec::new();
    for profile in PROVIDER_PROFILES {
        if cli_type == "gemini" && profile != DEFAULT_PROFILE {
            continue;
        }

        if let Some(id) = provider_direct_active_provider_id(db, cli_type, profile).await? {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    Ok(ids)
}

fn validate_provider_direct_provider(provider: &Provider) -> Result<()> {
    if provider.base_url.trim().is_empty() || provider.api_key.trim().is_empty() {
        return Err(format!(
            "服务商 {} 的 Base URL 或 API Key 为空",
            provider.name
        ));
    }
    Ok(())
}

async fn provider_direct_rewrite_providers(
    db: &SqlitePool,
    cli_type: &str,
    preferred_ids: &[i64],
) -> Result<Vec<Provider>> {
    let mut providers = Vec::new();
    for id in preferred_ids {
        if let Some(provider) =
            sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE id = ? AND cli_type = ?")
                .bind(id)
                .bind(cli_type)
                .fetch_optional(db)
                .await
                .map_err(|e| e.to_string())?
        {
            validate_provider_direct_provider(&provider)?;
            providers.push(provider);
        }
    }

    if !providers.is_empty() {
        return Ok(providers);
    }

    let provider = sqlx::query_as::<_, Provider>(
        "SELECT * FROM providers WHERE cli_type = ? AND profile = ? ORDER BY sort_order, id LIMIT 1",
    )
    .bind(cli_type)
    .bind(DEFAULT_PROFILE)
    .fetch_optional(db)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "default Profile 下没有可用服务商，请先添加服务商".to_string())?;

    validate_provider_direct_provider(&provider)?;
    Ok(vec![provider])
}

async fn rewrite_cli_config_for_current_mode(
    db: &SqlitePool,
    cli_type: &str,
    mode: &str,
    default_config: &str,
    previous_default_config: Option<&str>,
    gateway_url: &str,
    provider_direct_ids: &[i64],
    proxy_was_enabled: bool,
) -> Result<()> {
    match mode {
        CLI_MODE_PROXY_ROUTE => {
            if proxy_was_enabled || check_cli_enabled(db, cli_type, gateway_url).await {
                sync_cli_config(
                    db,
                    cli_type,
                    true,
                    default_config,
                    previous_default_config,
                    gateway_url,
                )
                .await?;
            }
        }
        CLI_MODE_PROVIDER_DIRECT => {
            let providers =
                provider_direct_rewrite_providers(db, cli_type, provider_direct_ids).await?;
            for provider in providers {
                write_provider_direct_config_with_previous(db, &provider, previous_default_config)
                    .await?;
            }
        }
        CLI_MODE_OFFICIAL_DIRECT => {
            credential_commands::auto_sync_credential_in_direct_mode(
                db,
                cli_type,
                previous_default_config,
            )
            .await?;
        }
        _ => {}
    }
    Ok(())
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

    let current_settings =
        sqlx::query_as::<_, (Option<String>, Option<String>, String, String)>(
            "SELECT default_json_config, config_dir, config_write_mode, cli_mode FROM cli_settings WHERE cli_type = ?",
        )
        .bind(&cli_type)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    let previous_default_config = current_settings
        .as_ref()
        .and_then(|row| row.0.clone())
        .unwrap_or_default();
    let previous_write_mode = current_settings
        .as_ref()
        .map(|row| row.2.clone())
        .unwrap_or_else(|| "merge".to_string());
    let previous_effective_config_dir = current_settings
        .as_ref()
        .and_then(|row| row.1.clone())
        .map(|path| expand_home_path(&path))
        .unwrap_or_else(|| {
            get_default_cli_config_dir(&cli_type)
                .to_string_lossy()
                .to_string()
        });
    let mode_before = current_settings
        .as_ref()
        .and_then(|row| normalize_cli_mode(&row.3))
        .unwrap_or(CLI_MODE_PROXY_ROUTE);

    let default_config_changed = config_trimmed
        .as_ref()
        .map(|config| config != &previous_default_config)
        .unwrap_or(false);
    let write_mode_changed = input
        .config_write_mode
        .as_ref()
        .map(|write_mode| write_mode != &previous_write_mode)
        .unwrap_or(false);
    let config_dir_changed = input
        .config_dir
        .as_ref()
        .map(|config_dir| {
            std::path::PathBuf::from(expand_home_path(config_dir))
                != std::path::PathBuf::from(previous_effective_config_dir.clone())
        })
        .unwrap_or(false);

    let provider_direct_ids_before = if mode_before == CLI_MODE_PROVIDER_DIRECT {
        collect_provider_direct_rewrite_ids(db.inner(), &cli_type)
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let proxy_was_enabled = mode_before == CLI_MODE_PROXY_ROUTE
        && check_cli_enabled(db.inner(), &cli_type, &gateway_url).await;

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
        sqlx::query(
            "UPDATE cli_settings SET default_json_config = ?, updated_at = ? WHERE cli_type = ?",
        )
        .bind(config_trimmed)
        .bind(now)
        .bind(&cli_type)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;
    }

    if default_config_changed || write_mode_changed || config_dir_changed {
        let mode = get_normalized_cli_mode(db.inner(), &cli_type).await?;
        let current_default_config = config_trimmed
            .clone()
            .unwrap_or_else(|| previous_default_config.clone());
        let previous_for_sync = if default_config_changed {
            Some(previous_default_config.as_str())
        } else {
            None
        };

        rewrite_cli_config_for_current_mode(
            db.inner(),
            &cli_type,
            mode,
            &current_default_config,
            previous_for_sync,
            &gateway_url,
            &provider_direct_ids_before,
            proxy_was_enabled,
        )
        .await?;
    }

    if let Some(enabled) = input.enabled {
        let mode = get_normalized_cli_mode(db.inner(), &cli_type).await?;

        if mode == CLI_MODE_PROXY_ROUTE {
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
            tracing::info!("{} 处于非中转路由模式，忽略 enabled 参数", cli_type);
        }
    }

    Ok(())
}
