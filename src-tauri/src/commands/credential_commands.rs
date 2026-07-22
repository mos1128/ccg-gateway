use super::*;

// ==================== Official Credential 相关命令 ====================

pub(super) async fn credential_matches_cli_files(
    db: &SqlitePool,
    credential: &OfficialCredential,
) -> Result<bool> {
    crate::services::official_credential::payload_matches(
        db,
        &credential.cli_type,
        &credential.credential_json,
    )
    .await
}

async fn official_direct_written_credential_id(
    db: &SqlitePool,
    gateway_url: &str,
    cli_type: &str,
) -> Result<Option<i64>> {
    if detect_cli_mode_from_url(db, gateway_url, cli_type).await != CLI_MODE_OFFICIAL_DIRECT {
        return Ok(None);
    }

    matched_official_credential_id(db, cli_type).await
}

fn official_credential_response(
    c: OfficialCredential,
    is_active: bool,
    is_written: bool,
) -> OfficialCredentialResponse {
    let display_info = crate::services::official_credential::display_info(&c.credential_json);
    OfficialCredentialResponse {
        is_active,
        is_written,
        id: c.id,
        cli_type: c.cli_type,
        name: c.name,
        credential_json: c.credential_json,
        sort_order: c.sort_order,
        display_info,
    }
}

/// 读取 CLI 当前凭证（异步版本，支持自定义配置目录）
async fn read_cli_credential_impl_async(db: &SqlitePool, cli_type: &str) -> Result<String> {
    crate::services::agent::validate_agent_id(cli_type)?;
    crate::services::official_credential::read_current_payload(db, cli_type).await
}

/// 同步凭证到 CLI 配置文件（异步版本，支持自定义配置目录）
async fn sync_credential_to_cli_async(
    db: &SqlitePool,
    cli_type: &str,
    credential_json: &str,
    default_config: &str,
    previous_default_config: Option<&str>,
) -> Result<()> {
    let write_mode = get_config_write_mode(db, cli_type).await;
    crate::services::agent_config::sync_global_preset(
        db,
        cli_type,
        true,
        default_config,
        previous_default_config,
        &write_mode,
    )
    .await?;
    crate::services::official_credential::apply_payload(db, cli_type, credential_json).await?;
    Ok(())
}

async fn sync_official_credential_to_cli(
    db: &SqlitePool,
    cred: &OfficialCredential,
    previous_default_config: Option<&str>,
) -> Result<()> {
    let default_config = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT default_json_config FROM cli_settings WHERE cli_type = ?",
    )
    .bind(&cred.cli_type)
    .fetch_optional(db)
    .await
    .map_err(|e| e.to_string())?
    .and_then(|r| r.0)
    .unwrap_or_default();

    sync_credential_to_cli_async(
        db,
        &cred.cli_type,
        &cred.credential_json,
        &default_config,
        previous_default_config,
    )
    .await?;
    remember_last_official_credential_id(db, &cred.cli_type, cred.id, now_timestamp()).await
}

async fn sync_first_official_credential_or_clear(
    db: &SqlitePool,
    cli_type: &str,
    previous_default_config: Option<&str>,
    removed_payload: Option<&str>,
) -> Result<()> {
    let cred: Option<OfficialCredential> = sqlx::query_as(
        "SELECT * FROM official_credentials WHERE cli_type = ? ORDER BY sort_order, id LIMIT 1",
    )
    .bind(cli_type)
    .fetch_optional(db)
    .await
    .map_err(|e| e.to_string())?;

    if let Some(cred) = cred {
        tracing::info!("{} 找到凭证 ID: {}, 名称: {}", cli_type, cred.id, cred.name);
        sync_official_credential_to_cli(db, &cred, previous_default_config).await
    } else {
        tracing::warn!("{} 没有可用的凭证，清理官方直连文件", cli_type);
        remove_official_mode_config(db, cli_type, removed_payload).await
    }
}

/// 在直连模式下，自动同步第一个凭证到 CLI 配置文件
pub(super) async fn auto_sync_credential_in_direct_mode(
    db: &SqlitePool,
    gateway_url: &str,
    cli_type: &str,
    previous_default_config: Option<&str>,
) -> Result<()> {
    tracing::info!(
        "auto_sync_credential_in_direct_mode 被调用，cli_type: {}",
        cli_type
    );

    // 检查当前是否为直连模式
    let mode = detect_cli_mode_from_url(db, gateway_url, cli_type).await;
    tracing::info!("{} 当前模式: {}", cli_type, mode);

    if mode != CLI_MODE_OFFICIAL_DIRECT {
        tracing::debug!("{} 当前不是直连模式，跳过自动同步", cli_type);
        return Ok(());
    }

    tracing::info!("{} 开始同步凭证到文件", cli_type);
    sync_first_official_credential_or_clear(db, cli_type, previous_default_config, None).await
}

pub(super) async fn rewrite_official_credential_in_current_mode(
    db: &SqlitePool,
    cli_type: &str,
    previous_default_config: Option<&str>,
) -> Result<()> {
    sync_first_official_credential_or_clear(db, cli_type, previous_default_config, None).await
}

async fn remove_official_mode_config(
    db: &SqlitePool,
    cli_type: &str,
    payload: Option<&str>,
) -> Result<()> {
    let owned_payload = if let Some(payload) = payload {
        Some(payload.to_string())
    } else {
        let credentials: Vec<OfficialCredential> = sqlx::query_as(
            "SELECT * FROM official_credentials WHERE cli_type = ? ORDER BY sort_order, id",
        )
        .bind(cli_type)
        .fetch_all(db)
        .await
        .map_err(|error| error.to_string())?;
        let mut matched = None;
        for credential in credentials {
            if crate::services::official_credential::payload_matches(
                db,
                cli_type,
                &credential.credential_json,
            )
            .await
            .unwrap_or(false)
            {
                matched = Some(credential.credential_json);
                break;
            }
        }
        matched
    };

    if let Some(payload) = owned_payload {
        crate::services::official_credential::remove_payload(db, cli_type, &payload).await?;
    }

    let default_config = get_cli_default_config(db, cli_type).await?;
    let write_mode = get_config_write_mode(db, cli_type).await;
    crate::services::agent_config::sync_global_preset(
        db,
        cli_type,
        false,
        &default_config,
        None,
        &write_mode,
    )
    .await?;
    Ok(())
}

async fn remove_direct_mode_files_async(db: &SqlitePool, cli_type: &str) -> Result<()> {
    remove_official_mode_config(db, cli_type, None).await
}

#[tauri::command]
pub async fn get_credentials(
    db: State<'_, SqlitePool>,
    config: State<'_, Config>,
    cli_type: String,
) -> Result<Vec<OfficialCredentialResponse>> {
    let creds: Vec<OfficialCredential> = sqlx::query_as(
        "SELECT * FROM official_credentials WHERE cli_type = ? ORDER BY sort_order, id",
    )
    .bind(&cli_type)
    .fetch_all(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    let gateway_url = config.gateway_base_url();
    let written_id = official_direct_written_credential_id(db.inner(), &gateway_url, &cli_type)
        .await
        .unwrap_or(None);
    let results = creds
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            let is_written = written_id == Some(c.id);
            official_credential_response(c, i == 0, is_written)
        })
        .collect();

    Ok(results)
}

#[tauri::command]
pub async fn create_credential(
    db: State<'_, SqlitePool>,
    config: State<'_, Config>,
    log_db: State<'_, LogDb>,
    input: OfficialCredentialCreate,
) -> Result<OfficialCredentialResponse> {
    let now = now_timestamp();
    crate::services::official_credential::validate_payload(
        &input.cli_type,
        &input.credential_json,
    )?;

    // Check if this is the first credential for this cli_type
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM official_credentials WHERE cli_type = ?")
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
    .map_err(map_db_error)?;

    let id = result.last_insert_rowid();

    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "credential_created",
        &format!("凭证 {} 已创建", input.name),
    )
    .await;

    // 如果是直连模式，自动同步到文件
    let gateway_url = config.gateway_base_url();
    if let Err(e) =
        auto_sync_credential_in_direct_mode(db.inner(), &gateway_url, &input.cli_type, None).await
    {
        tracing::error!("自动同步凭证失败: {}", e);
    }

    get_credential(db, config, id).await
}

#[tauri::command]
pub async fn get_credential(
    db: State<'_, SqlitePool>,
    config: State<'_, Config>,
    id: i64,
) -> Result<OfficialCredentialResponse> {
    let cred =
        sqlx::query_as::<_, OfficialCredential>("SELECT * FROM official_credentials WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "凭证不存在".to_string())?;

    let gateway_url = config.gateway_base_url();
    let is_written = detect_cli_mode_from_url(db.inner(), &gateway_url, &cred.cli_type).await
        == CLI_MODE_OFFICIAL_DIRECT
        && credential_matches_cli_files(db.inner(), &cred)
            .await
            .unwrap_or(false);

    let is_active = cred.sort_order == 0;
    Ok(official_credential_response(cred, is_active, is_written))
}

#[tauri::command]
pub async fn update_credential(
    db: State<'_, SqlitePool>,
    config: State<'_, Config>,
    log_db: State<'_, LogDb>,
    id: i64,
    input: OfficialCredentialUpdate,
) -> Result<OfficialCredentialResponse> {
    let now = now_timestamp();

    let old_cred: OfficialCredential =
        sqlx::query_as("SELECT * FROM official_credentials WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "凭证不存在".to_string())?;

    let cred_name = old_cred.name.clone();
    let gateway_url = config.gateway_base_url();
    let was_written =
        official_direct_written_credential_id(db.inner(), &gateway_url, &old_cred.cli_type).await?
            == Some(old_cred.id);

    if let Some(credential_json) = input.credential_json.as_deref() {
        crate::services::official_credential::validate_payload(
            &old_cred.cli_type,
            credential_json,
        )?;
    }

    let mut updates = vec!["updated_at = ?".to_string()];
    if input.name.is_some() {
        updates.push("name = ?".to_string());
    }
    if input.credential_json.is_some() {
        updates.push("credential_json = ?".to_string());
    }

    let query = format!(
        "UPDATE official_credentials SET {} WHERE id = ?",
        updates.join(", ")
    );
    let mut q = sqlx::query(&query).bind(now);
    if let Some(ref name) = input.name {
        q = q.bind(name);
    }
    if let Some(ref json) = input.credential_json {
        q = q.bind(json);
    }
    q.bind(id).execute(db.inner()).await.map_err(map_db_error)?;

    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "credential_updated",
        &format!("凭证 {} 已更新", cred_name),
    )
    .await;

    // 获取更新后的凭证信息
    let updated_cred: Option<OfficialCredential> =
        sqlx::query_as("SELECT * FROM official_credentials WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?;

    // 如果是直连模式，自动同步到文件
    if let Some(cred) = updated_cred {
        let sync_result = if was_written {
            sync_official_credential_to_cli(db.inner(), &cred, None).await
        } else {
            auto_sync_credential_in_direct_mode(db.inner(), &gateway_url, &cred.cli_type, None)
                .await
        };
        if let Err(e) = sync_result {
            tracing::error!("自动同步凭证失败: {}", e);
        }
    }

    get_credential(db, config, id).await
}

#[tauri::command]
pub async fn delete_credential(
    db: State<'_, SqlitePool>,
    config: State<'_, Config>,
    log_db: State<'_, LogDb>,
    id: i64,
) -> Result<()> {
    let old_cred: OfficialCredential =
        sqlx::query_as("SELECT * FROM official_credentials WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "凭证不存在".to_string())?;

    let active_cli_type = (old_cred.sort_order == 0).then(|| old_cred.cli_type.clone());
    let gateway_url = config.gateway_base_url();
    let was_written =
        official_direct_written_credential_id(db.inner(), &gateway_url, &old_cred.cli_type).await?
            == Some(old_cred.id);

    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "credential_deleted",
        &format!("凭证 {} 已删除", old_cred.name),
    )
    .await;

    sqlx::query("DELETE FROM official_credentials WHERE id = ?")
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

    if was_written {
        if let Err(e) = sync_first_official_credential_or_clear(
            db.inner(),
            &old_cred.cli_type,
            None,
            Some(&old_cred.credential_json),
        )
        .await
        {
            tracing::error!("自动同步凭证失败: {}", e);
        }
    } else if let Some(cli_type) = active_cli_type {
        if let Err(e) =
            auto_sync_credential_in_direct_mode(db.inner(), &gateway_url, &cli_type, None).await
        {
            tracing::error!("自动同步凭证失败: {}", e);
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn reorder_credentials(
    db: State<'_, SqlitePool>,
    config: State<'_, Config>,
    ids: Vec<i64>,
) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }

    // 获取第一个凭证的 cli_type（用于后续同步）
    let cli_type: Option<String> =
        sqlx::query_as("SELECT cli_type FROM official_credentials WHERE id = ?")
            .bind(ids[0])
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?
            .map(|row: (String,)| row.0);
    let gateway_url = config.gateway_base_url();
    let was_official_direct = if let Some(ref cli_type_str) = cli_type {
        official_direct_written_credential_id(db.inner(), &gateway_url, cli_type_str)
            .await?
            .is_some()
    } else {
        false
    };

    // 使用 CASE WHEN 批量更新（避免 N 次单独更新）
    let case_clauses: Vec<String> = ids
        .iter()
        .enumerate()
        .map(|(idx, id)| format!("WHEN {} THEN {}", id, idx))
        .collect();

    let id_list: Vec<String> = ids.iter().map(|id| id.to_string()).collect();

    let sql = format!(
        "UPDATE official_credentials SET sort_order = CASE id {} END WHERE id IN ({})",
        case_clauses.join(" "),
        id_list.join(", ")
    );

    sqlx::query(&sql)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

    // 如果是直连模式，自动同步到文件
    if was_official_direct {
        if let Some(cli_type_str) = cli_type {
            if let Err(e) =
                sync_first_official_credential_or_clear(db.inner(), &cli_type_str, None, None).await
            {
                tracing::error!("自动同步凭证失败: {}", e);
            }
        }
    } else if let Some(cli_type_str) = cli_type {
        if let Err(e) =
            auto_sync_credential_in_direct_mode(db.inner(), &gateway_url, &cli_type_str, None).await
        {
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
pub async fn write_credential_config(
    db: State<'_, SqlitePool>,
    config: State<'_, Config>,
    log_db: State<'_, LogDb>,
    id: i64,
) -> Result<OfficialCredentialResponse> {
    let cred: OfficialCredential =
        sqlx::query_as("SELECT * FROM official_credentials WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "凭证不存在".to_string())?;

    let gateway_url = config.gateway_base_url();
    let current_mode = detect_cli_mode_from_url(db.inner(), &gateway_url, &cred.cli_type).await;
    let default_config = get_cli_default_config(db.inner(), &cred.cli_type).await?;
    if current_mode == CLI_MODE_PROVIDER_DIRECT {
        crate::services::agent_config::remove_default_provider_direct_config(
            db.inner(),
            &cred.cli_type,
        )
        .await?;
    } else if current_mode == CLI_MODE_PROXY_ROUTE {
        let has_gateway_config = check_cli_enabled(db.inner(), &cred.cli_type, &gateway_url).await;
        if has_gateway_config {
            sync_cli_config_with_log(
                db.inner(),
                &log_db.0,
                &cred.cli_type,
                false,
                &default_config,
                None,
                &gateway_url,
            )
            .await?;
        }
    }

    if let Err(error) = sync_credential_to_cli_async(
        db.inner(),
        &cred.cli_type,
        &cred.credential_json,
        &default_config,
        None,
    )
    .await
    {
        let _ = crate::services::stats::record_system_log(
            &log_db.0,
            "official_credential_write_failed",
            &format!("Agent {} 官方凭证写入失败: {}", cred.cli_type, error),
        )
        .await;
        return Err(error);
    }

    remember_last_official_credential_id(db.inner(), &cred.cli_type, cred.id, now_timestamp())
        .await?;

    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "credential_written",
        &format!("凭证 {} 已写入 CLI 配置", cred.name),
    )
    .await;

    get_credential(db, config, id).await
}

async fn dashboard_provider_direct_provider(db: &SqlitePool, cli_type: &str) -> Result<Provider> {
    let provider: Provider = sqlx::query_as(
        "SELECT * FROM providers WHERE cli_type = ? AND profile = ? ORDER BY sort_order, id LIMIT 1",
    )
    .bind(cli_type)
    .bind(DEFAULT_PROFILE)
    .fetch_optional(db)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "default Profile 下没有可用服务商，请先添加服务商".to_string())?;

    if provider.base_url.trim().is_empty() || provider.api_key.trim().is_empty() {
        return Err(format!(
            "服务商 {} 的 Base URL 或 API Key 为空",
            provider.name
        ));
    }

    Ok(provider)
}

async fn first_official_credential(db: &SqlitePool, cli_type: &str) -> Result<OfficialCredential> {
    sqlx::query_as(
        "SELECT * FROM official_credentials WHERE cli_type = ? ORDER BY sort_order, id LIMIT 1",
    )
    .bind(cli_type)
    .fetch_optional(db)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "没有可用官方凭证，请先添加凭证".to_string())
}

async fn preferred_default_credential(
    db: &SqlitePool,
    cli_type: &str,
) -> Result<OfficialCredential> {
    let last_cred_id: Option<i64> = sqlx::query_as::<_, (Option<i64>,)>(
        "SELECT last_official_credential_id FROM cli_settings WHERE cli_type = ?",
    )
    .bind(cli_type)
    .fetch_optional(db)
    .await
    .map_err(|e| e.to_string())?
    .and_then(|row| row.0);

    if let Some(id) = last_cred_id {
        if let Some(cred) = sqlx::query_as::<_, OfficialCredential>(
            "SELECT * FROM official_credentials WHERE id = ? AND cli_type = ?",
        )
        .bind(id)
        .bind(cli_type)
        .fetch_optional(db)
        .await
        .map_err(|e| e.to_string())?
        {
            return Ok(cred);
        }
    }

    first_official_credential(db, cli_type).await
}

async fn remember_current_default_provider_direct_provider(
    db: &SqlitePool,
    cli_type: &str,
) -> Result<()> {
    if let Some(id) = crate::services::agent_config::provider_direct_active_provider_id(
        db,
        cli_type,
        DEFAULT_PROFILE,
    )
    .await?
    {
        remember_default_provider_direct_provider_id(db, cli_type, id, now_timestamp()).await?;
    }
    Ok(())
}

async fn write_dashboard_proxy_route(
    db: &SqlitePool,
    config: &Config,
    log_db: &LogDb,
    cli_type: &str,
    current_mode: &str,
) -> Result<()> {
    let gateway_url = config.gateway_base_url();
    if current_mode == CLI_MODE_OFFICIAL_DIRECT {
        remove_direct_mode_files_async(db, cli_type).await?;
    } else if current_mode == CLI_MODE_PROVIDER_DIRECT {
        remember_current_default_provider_direct_provider(db, cli_type).await?;
        crate::services::agent_config::remove_default_provider_direct_config(db, cli_type).await?;
    }

    let default_config = get_cli_default_config(db, cli_type).await?;
    sync_cli_config_with_log(
        db,
        &log_db.0,
        cli_type,
        true,
        &default_config,
        None,
        &gateway_url,
    )
    .await?;

    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "cli_mode_changed",
        &format!("{} 已切换到中转路由", cli_type),
    )
    .await;

    Ok(())
}

async fn write_dashboard_provider_direct(
    db: &SqlitePool,
    config: &Config,
    log_db: &LogDb,
    cli_type: &str,
    current_mode: &str,
) -> Result<()> {
    if current_mode == CLI_MODE_OFFICIAL_DIRECT {
        remove_direct_mode_files_async(db, cli_type).await?;
    } else if current_mode == CLI_MODE_PROXY_ROUTE {
        let gateway_url = config.gateway_base_url();
        let default_config = get_cli_default_config(db, cli_type).await?;
        sync_cli_config_with_log(
            db,
            &log_db.0,
            cli_type,
            false,
            &default_config,
            None,
            &gateway_url,
        )
        .await?;
    }

    let provider = dashboard_provider_direct_provider(db, cli_type).await?;
    crate::services::agent_config::write_provider_direct_config(db, &provider).await?;
    let now = now_timestamp();
    remember_default_provider_direct_provider(db, &provider, now).await?;

    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "cli_mode_changed",
        &format!("{} 已切换到中转直连：{}", cli_type, provider.name),
    )
    .await;

    Ok(())
}

async fn write_dashboard_official_direct(
    db: &SqlitePool,
    config: &Config,
    log_db: &LogDb,
    cli_type: &str,
    current_mode: &str,
) -> Result<()> {
    let cred = preferred_default_credential(db, cli_type).await?;
    let gateway_url = config.gateway_base_url();
    let default_config = get_cli_default_config(db, cli_type).await?;

    if current_mode == CLI_MODE_PROVIDER_DIRECT {
        remember_current_default_provider_direct_provider(db, cli_type).await?;
        crate::services::agent_config::remove_default_provider_direct_config(db, cli_type).await?;
    } else if current_mode == CLI_MODE_PROXY_ROUTE
        && check_cli_enabled(db, cli_type, &gateway_url).await
    {
        sync_cli_config_with_log(
            db,
            &log_db.0,
            cli_type,
            false,
            &default_config,
            None,
            &gateway_url,
        )
        .await?;
    }

    if let Err(error) =
        sync_credential_to_cli_async(db, cli_type, &cred.credential_json, &default_config, None)
            .await
    {
        let _ = crate::services::stats::record_system_log(
            &log_db.0,
            "official_credential_write_failed",
            &format!("Agent {} 官方凭证写入失败: {}", cli_type, error),
        )
        .await;
        return Err(error);
    }
    remember_last_official_credential_id(db, cli_type, cred.id, now_timestamp()).await?;

    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "cli_mode_changed",
        &format!("{} 已切换到官方直连：{}", cli_type, cred.name),
    )
    .await;

    Ok(())
}

async fn write_dashboard_disabled(
    db: &SqlitePool,
    config: &Config,
    log_db: &LogDb,
    cli_type: &str,
    current_mode: &str,
) -> Result<()> {
    match current_mode {
        // 路由 → 停用：删除网关配置 + 全局预设配置
        CLI_MODE_PROXY_ROUTE => {
            let gateway_url = config.gateway_base_url();
            let default_config = get_cli_default_config(db, cli_type).await?;
            sync_cli_config_with_log(
                db,
                &log_db.0,
                cli_type,
                false,
                &default_config,
                None,
                &gateway_url,
            )
            .await?;
        }
        // 直连 → 停用：删除服务商配置 + 全局预设配置
        CLI_MODE_PROVIDER_DIRECT => {
            let default_config = get_cli_default_config(db, cli_type).await?;
            remember_current_default_provider_direct_provider(db, cli_type).await?;
            crate::services::agent_config::remove_default_provider_direct_config(db, cli_type)
                .await?;
            sync_cli_config_with_log(
                db,
                &log_db.0,
                cli_type,
                false,
                &default_config,
                None,
                &config.gateway_base_url(),
            )
            .await?;
        }
        // 官方 → 停用：删除直连模式写入的文件
        CLI_MODE_OFFICIAL_DIRECT => {
            remove_direct_mode_files_async(db, cli_type).await?;
        }
        _ => {}
    }

    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "cli_mode_changed",
        &format!("{} 已停用路由", cli_type),
    )
    .await;

    Ok(())
}

async fn apply_dashboard_cli_mode(
    db: &SqlitePool,
    config: &Config,
    log_db: &LogDb,
    cli_type: &str,
    mode: &str,
) -> Result<()> {
    let agent = crate::services::agent::get_agent(db, cli_type)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("未知 Agent: {}", cli_type))?;
    validate_dashboard_mode_capability(&agent, mode)?;

    let gateway_url = config.gateway_base_url();
    let current_mode = detect_cli_mode_from_url(db, &gateway_url, cli_type).await;
    if current_mode == mode {
        return Ok(());
    }

    match mode {
        CLI_MODE_PROXY_ROUTE => {
            write_dashboard_proxy_route(db, config, log_db, cli_type, current_mode).await
        }
        CLI_MODE_PROVIDER_DIRECT => {
            write_dashboard_provider_direct(db, config, log_db, cli_type, current_mode).await
        }
        CLI_MODE_OFFICIAL_DIRECT => {
            write_dashboard_official_direct(db, config, log_db, cli_type, current_mode).await
        }
        CLI_MODE_DISABLED => {
            write_dashboard_disabled(db, config, log_db, cli_type, current_mode).await
        }
        _ => Err("不支持的 CLI 模式".to_string()),
    }
}

fn validate_dashboard_mode_capability(
    agent: &crate::db::models::AgentInfo,
    mode: &str,
) -> Result<()> {
    match mode {
        CLI_MODE_PROXY_ROUTE => {
            let feature = &agent.features.provider_config;
            if !feature.enabled || feature.operations.is_empty() {
                return Err(format!("Agent {} 不支持自动写入网关配置", agent.name));
            }
        }
        CLI_MODE_PROVIDER_DIRECT => {
            let feature = &agent.features.provider_config;
            if !feature.enabled || feature.operations.is_empty() {
                return Err(format!("Agent {} 不支持服务商直连模式", agent.name));
            }
        }
        CLI_MODE_OFFICIAL_DIRECT => {
            let feature = &agent.features.official_login;
            let supported = feature.enabled && !feature.operations.is_empty();
            if !supported {
                return Err(format!("Agent {} 不支持托管官方凭证", agent.name));
            }
        }
        CLI_MODE_DISABLED => {}
        _ => return Err("不支持的 CLI 模式".to_string()),
    }
    Ok(())
}

#[tauri::command]
pub async fn set_dashboard_cli_mode(
    db: State<'_, SqlitePool>,
    config: State<'_, Config>,
    log_db: State<'_, LogDb>,
    cli_type: String,
    mode: String,
) -> Result<()> {
    let mode = normalize_cli_mode(&mode).ok_or_else(|| {
        "cli_mode 只能是 proxy_route / provider_direct / official_direct / disabled".to_string()
    })?;

    apply_dashboard_cli_mode(db.inner(), config.inner(), log_db.inner(), &cli_type, mode).await
}

#[tauri::command]
pub async fn get_cli_mode(
    db: State<'_, SqlitePool>,
    config: State<'_, Config>,
    cli_type: String,
) -> Result<String> {
    let gateway_url = config.gateway_base_url();
    Ok(
        detect_cli_mode_from_url(db.inner(), &gateway_url, &cli_type)
            .await
            .to_string(),
    )
}

#[tauri::command]
pub async fn set_cli_mode(
    db: State<'_, SqlitePool>,
    config: State<'_, Config>,
    log_db: State<'_, LogDb>,
    cli_type: String,
    mode: String,
) -> Result<()> {
    let mode = normalize_cli_mode(&mode).ok_or_else(|| {
        "cli_mode 只能是 proxy_route / provider_direct / official_direct / disabled".to_string()
    })?;

    apply_dashboard_cli_mode(db.inner(), config.inner(), log_db.inner(), &cli_type, mode).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema_definition::DatabaseSchema;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn dashboard_direct_uses_first_provider_instead_of_remembered_provider() {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("test database");
        for sql in DatabaseSchema::current().to_create_all_sql() {
            sqlx::query(&sql).execute(&db).await.expect("test schema");
        }
        sqlx::query(
            "INSERT INTO cli_settings (cli_type, default_json_config, updated_at) VALUES ('claude_code', '', 0)",
        )
        .execute(&db)
        .await
        .expect("test settings");
        let first_id = sqlx::query(
            "INSERT INTO providers (cli_type, profile, protocol, name, base_url, api_key, sort_order, created_at, updated_at) VALUES ('claude_code', 'default', 'anthropic_messages', 'first', 'https://first.example.com', 'first-key', 0, 0, 0)",
        )
        .execute(&db)
        .await
        .expect("first provider")
        .last_insert_rowid();
        let remembered_id = sqlx::query(
            "INSERT INTO providers (cli_type, profile, protocol, name, base_url, api_key, sort_order, created_at, updated_at) VALUES ('claude_code', 'default', 'anthropic_messages', 'remembered', 'https://remembered.example.com', 'remembered-key', 1, 0, 0)",
        )
        .execute(&db)
        .await
        .expect("remembered provider")
        .last_insert_rowid();
        sqlx::query(
            "UPDATE cli_settings SET last_provider_direct_provider_id = ? WHERE cli_type = 'claude_code'",
        )
        .bind(remembered_id)
        .execute(&db)
        .await
        .expect("remembered provider setting");

        let selected = dashboard_provider_direct_provider(&db, "claude_code")
            .await
            .expect("dashboard direct provider");
        assert_eq!(selected.id, first_id);
    }
}
