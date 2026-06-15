use super::*;

// ==================== Official Credential 相关命令 ====================

/// 解析凭证 JSON 生成显示信息
fn parse_display_info(cli_type: &str, credential_json: &str) -> String {
    // 尝试解析为文件列表格式
    if let Ok(files) = serde_json::from_str::<Vec<serde_json::Value>>(credential_json) {
        match cli_type {
            "claude_code" => {
                // 查找 settings.json 文件
                if let Some(file) = files.iter().find(|f| {
                    f.get("path")
                        .and_then(|p| p.as_str())
                        .map(|p| p.contains("settings.json"))
                        .unwrap_or(false)
                }) {
                    if let Some(content) = file.get("content").and_then(|c| c.as_str()) {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(content) {
                            return data
                                .get("ANTHROPIC_API_KEY")
                                .and_then(|v| v.as_str())
                                .map(|key| {
                                    if key.len() > 12 {
                                        format!("sk-ant-...{}", &key[key.len() - 8..])
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
                    f.get("path")
                        .and_then(|p| p.as_str())
                        .map(|p| p.contains("auth.json"))
                        .unwrap_or(false)
                }) {
                    if let Some(content) = file.get("content").and_then(|c| c.as_str()) {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(content) {
                            return data
                                .get("tokens")
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
                    f.get("path")
                        .and_then(|p| p.as_str())
                        .map(|p| p.contains("google_accounts.json"))
                        .unwrap_or(false)
                }) {
                    if let Some(content) = file.get("content").and_then(|c| c.as_str()) {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(content) {
                            return data
                                .get("active")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "已配置".to_string());
                        }
                    }
                }
                "未配置".to_string()
            }
            _ => "未知".to_string(),
        }
    } else {
        // 兼容旧格式：直接解析为 JSON 对象
        match serde_json::from_str::<serde_json::Value>(credential_json) {
            Ok(creds) => match cli_type {
                "claude_code" => creds
                    .get("ANTHROPIC_API_KEY")
                    .and_then(|v| v.as_str())
                    .map(|key| {
                        if key.len() > 12 {
                            format!("sk-ant-...{}", &key[key.len() - 8..])
                        } else {
                            "已配置".to_string()
                        }
                    })
                    .unwrap_or_else(|| "未知".to_string()),
                "codex" => creds
                    .get("tokens")
                    .and_then(|t| t.get("active_email"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "已配置".to_string()),
                "gemini" => creds
                    .get("google_accounts")
                    .and_then(|g| g.get("active"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "已配置".to_string()),
                _ => "未知".to_string(),
            },
            Err(_) => "无效 JSON".to_string(),
        }
    }
}

fn credential_file_target<'a>(
    config_dir: &'a std::path::Path,
    cli_type: &str,
    path: &str,
) -> Option<std::path::PathBuf> {
    match cli_type {
        "claude_code" if path.contains("settings.json") => Some(config_dir.join("settings.json")),
        "codex" if path.contains("auth.json") => Some(config_dir.join("auth.json")),
        "gemini" if path.contains("oauth_creds.json") => Some(config_dir.join("oauth_creds.json")),
        "gemini" if path.contains("google_accounts.json") => {
            Some(config_dir.join("google_accounts.json"))
        }
        _ => None,
    }
}

pub(super) async fn credential_matches_cli_files(
    db: &SqlitePool,
    credential: &OfficialCredential,
) -> Result<bool> {
    let files: Vec<serde_json::Value> = serde_json::from_str(&credential.credential_json)
        .map_err(|e| format!("解析凭证文件列表失败: {}", e))?;
    let config_dir = get_cli_config_dir_path(db, &credential.cli_type).await;
    let mut matched_any = false;

    for file in files {
        let path = file.get("path").and_then(|p| p.as_str()).unwrap_or("");
        let expected = file.get("content").and_then(|c| c.as_str()).unwrap_or("");
        let Some(target_path) = credential_file_target(&config_dir, &credential.cli_type, path)
        else {
            continue;
        };

        matched_any = true;
        let actual = if tokio::fs::try_exists(&target_path).await.unwrap_or(false) {
            tokio::fs::read_to_string(&target_path)
                .await
                .map_err(|e| e.to_string())?
        } else {
            String::new()
        };

        if actual != expected {
            return Ok(false);
        }
    }

    Ok(matched_any)
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
    let display_info = parse_display_info(&c.cli_type, &c.credential_json);
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
    let config_dir = get_cli_config_dir_path(db, cli_type).await;

    match cli_type {
        "claude_code" => {
            let config_path = config_dir.join("settings.json");

            // 如果文件不存在，返回空内容（而不是报错）
            if !tokio::fs::try_exists(&config_path).await.unwrap_or(false) {
                let files = vec![serde_json::json!({
                    "path": format!("{}/settings.json", config_dir.display()),
                    "content": ""
                })];
                return Ok(serde_json::to_string(&files).unwrap());
            }

            let content = tokio::fs::read_to_string(&config_path)
                .await
                .map_err(|e| format!("读取失败: {}", e))?;

            let files = vec![serde_json::json!({
                "path": format!("{}/settings.json", config_dir.display()),
                "content": content
            })];
            Ok(serde_json::to_string(&files).unwrap())
        }
        "codex" => {
            let auth_path = config_dir.join("auth.json");

            // 如果文件不存在，返回空的文件列表（而不是报错）
            if !tokio::fs::try_exists(&auth_path).await.unwrap_or(false) {
                let files = vec![serde_json::json!({
                    "path": format!("{}/auth.json", config_dir.display()),
                    "content": ""
                })];
                return Ok(serde_json::to_string(&files).unwrap());
            }

            let content = tokio::fs::read_to_string(&auth_path)
                .await
                .map_err(|e| format!("读取失败: {}", e))?;

            let files = vec![serde_json::json!({
                "path": format!("{}/auth.json", config_dir.display()),
                "content": content
            })];
            Ok(serde_json::to_string(&files).unwrap())
        }
        "gemini" => {
            let oauth_path = config_dir.join("oauth_creds.json");
            let accounts_path = config_dir.join("google_accounts.json");

            let mut files = vec![];

            // 即使文件不存在，也添加空内容的占位符
            if tokio::fs::try_exists(&oauth_path).await.unwrap_or(false) {
                let content = tokio::fs::read_to_string(&oauth_path)
                    .await
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

            if tokio::fs::try_exists(&accounts_path).await.unwrap_or(false) {
                let content = tokio::fs::read_to_string(&accounts_path)
                    .await
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

            Ok(serde_json::to_string(&files).unwrap())
        }
        _ => Err("Unsupported CLI type".to_string()),
    }
}

/// 同步凭证到 CLI 配置文件（异步版本，支持自定义配置目录）
async fn sync_credential_to_cli_async(
    db: &SqlitePool,
    cli_type: &str,
    credential_json: &str,
    default_config: &str,
    previous_default_config: Option<&str>,
) -> Result<()> {
    // 解析文件列表
    let files: Vec<serde_json::Value> = serde_json::from_str(credential_json)
        .map_err(|e| format!("解析凭证文件列表失败: {}", e))?;

    let config_dir = get_cli_config_dir_path(db, cli_type).await;
    let write_mode = get_config_write_mode(db, cli_type).await;
    let use_merge = write_mode == "merge";

    match cli_type {
        "claude_code" => {
            // TODO: Claude Code 的具体实现待完善
            tracing::warn!("Claude Code 的直连模式配置写入功能尚未实现");
        }
        "codex" => {
            let auth_path = config_dir.join("auth.json");
            let config_path = config_dir.join("config.toml");

            // 直连模式不备份
            tokio::fs::create_dir_all(&config_dir)
                .await
                .map_err(|e| e.to_string())?;

            // 查找 auth.json 文件
            let auth_file = files.iter().find(|f| {
                f.get("path")
                    .and_then(|p| p.as_str())
                    .map(|p| p.contains("auth.json"))
                    .unwrap_or(false)
            });

            if let Some(file) = auth_file {
                let content = file.get("content").and_then(|c| c.as_str()).unwrap_or("");

                // 只有当内容不为空时才写入
                if !content.is_empty() {
                    tracing::info!(
                        "写入 Codex auth.json，内容长度: {}，路径: {:?}",
                        content.len(),
                        auth_path
                    );
                    tokio::fs::write(&auth_path, content).await.map_err(|e| {
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

            // 处理 config.toml
            let should_clean_previous = use_merge
                && previous_config_to_remove(previous_default_config, default_config).is_some();
            let config_exists = tokio::fs::try_exists(&config_path).await.unwrap_or(false);
            if !default_config.is_empty() || (should_clean_previous && config_exists) {
                let existing_content = if use_merge && config_exists {
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

                if use_merge {
                    remove_previous_codex_preset(
                        &mut final_doc,
                        previous_default_config,
                        default_config,
                        false,
                    );
                }

                if !default_config.is_empty() {
                    match default_config.parse::<toml_edit::DocumentMut>() {
                        Ok(custom_doc) => {
                            for (k, v) in custom_doc.iter() {
                                final_doc.insert(&k, v.clone());
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to parse Codex default_config (invalid TOML): {}",
                                e
                            );
                        }
                    }
                }

                let final_content = final_doc.to_string();

                tracing::info!(
                    "写入 Codex config.toml（{}模式），路径: {:?}",
                    write_mode,
                    config_path
                );
                tokio::fs::write(&config_path, final_content)
                    .await
                    .map_err(|e| {
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
            tokio::fs::create_dir_all(&config_dir)
                .await
                .map_err(|e| e.to_string())?;

            // 写入各个文件
            for file in files.iter() {
                let path_str = file.get("path").and_then(|p| p.as_str()).unwrap_or("");
                let content = file.get("content").and_then(|c| c.as_str()).unwrap_or("");

                if path_str.contains("oauth_creds.json") && !content.is_empty() {
                    tracing::info!(
                        "写入 Gemini oauth_creds.json，内容长度: {}，路径: {:?}",
                        content.len(),
                        oauth_path
                    );
                    tokio::fs::write(&oauth_path, content).await.map_err(|e| {
                        tracing::error!("写入 oauth_creds.json 失败: {}", e);
                        e.to_string()
                    })?;
                    tracing::info!("Gemini oauth_creds.json 写入成功");
                } else if path_str.contains("google_accounts.json") && !content.is_empty() {
                    tracing::info!(
                        "写入 Gemini google_accounts.json，内容长度: {}，路径: {:?}",
                        content.len(),
                        accounts_path
                    );
                    tokio::fs::write(&accounts_path, content)
                        .await
                        .map_err(|e| {
                            tracing::error!("写入 google_accounts.json 失败: {}", e);
                            e.to_string()
                        })?;
                    tracing::info!("Gemini google_accounts.json 写入成功");
                }
            }

            // 删除 .env 文件（如果存在）
            if tokio::fs::try_exists(&env_path).await.unwrap_or(false) {
                let _ = tokio::fs::remove_file(&env_path).await;
            }

            // 处理 settings.json
            // 1. 根据写入模式决定是否读取现有文件作为基础
            let mut config =
                if use_merge && tokio::fs::try_exists(&settings_path).await.unwrap_or(false) {
                    tokio::fs::read_to_string(&settings_path)
                        .await
                        .ok()
                        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                        .unwrap_or_else(|| serde_json::json!({}))
                } else {
                    serde_json::json!({})
                };

            let protected = gemini_gateway_json_template();
            if use_merge {
                remove_previous_json_preset(
                    &mut config,
                    previous_default_config,
                    default_config,
                    &protected,
                    "Gemini",
                );
            }

            // 2. 注入直连模式强制配置 (OAuth Personal)
            let direct_mode_auth = serde_json::json!({
                "security": {
                    "auth": {
                        "selectedType": "oauth-personal"
                    }
                }
            });
            deep_merge(&mut config, &direct_mode_auth);
            tracing::info!("Gemini 直连模式强制配置注入成功");

            // 3. 合并全局配置（全局配置优先级最高，但过滤受保护字段）
            if !default_config.is_empty() {
                tracing::info!("Gemini 全局配置不为空，长度: {}", default_config.len());
                if let Ok(default_val) = serde_json::from_str::<serde_json::Value>(default_config) {
                    let sanitized = sanitize_json_config(default_val, &protected);
                    deep_merge(&mut config, &sanitized);
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
                tracing::info!(
                    "写入 Gemini settings.json（{}模式），路径: {:?}",
                    write_mode,
                    settings_path
                );
                tokio::fs::write(
                    &settings_path,
                    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?,
                )
                .await
                .map_err(|e| {
                    tracing::error!("写入 settings.json 失败: {}", e);
                    e.to_string()
                })?;
                tracing::info!("Gemini settings.json 写入成功");
            } else {
                tracing::warn!("Gemini settings.json 配置为空对象，跳过写入");
            }
        }
        _ => return Err("不支持的 CLI 类型".to_string()),
    }

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
        remove_direct_mode_files_async(db, cli_type).await
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
    sync_first_official_credential_or_clear(db, cli_type, previous_default_config).await
}

/// 删除直连模式写入的所有文件（异步版本，支持自定义配置目录）
async fn remove_direct_mode_files_async(db: &SqlitePool, cli_type: &str) -> Result<()> {
    let config_dir = get_cli_config_dir_path(db, cli_type).await;
    let use_merge = get_config_write_mode(db, cli_type).await == "merge";

    match cli_type {
        "claude_code" => {
            // TODO: Claude Code 的具体实现待完善
            tracing::warn!("Claude Code 的直连模式文件删除功能尚未实现");
            Ok(())
        }
        "codex" => remove_codex_direct_mode_files(&config_dir, use_merge).await,
        "gemini" => remove_gemini_direct_mode_files(&config_dir, use_merge).await,
        _ => Err("不支持的 CLI 类型".to_string()),
    }
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
    if let Err(e) = auto_sync_credential_in_direct_mode(db.inner(), &gateway_url, &input.cli_type, None).await {
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
        if let Err(e) =
            sync_first_official_credential_or_clear(db.inner(), &old_cred.cli_type, None).await
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
pub async fn reorder_credentials(db: State<'_, SqlitePool>, config: State<'_, Config>, ids: Vec<i64>) -> Result<()> {
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
                sync_first_official_credential_or_clear(db.inner(), &cli_type_str, None).await
            {
                tracing::error!("自动同步凭证失败: {}", e);
            }
        }
    } else if let Some(cli_type_str) = cli_type {
        if let Err(e) =
            auto_sync_credential_in_direct_mode(db.inner(), &gateway_url, &cli_type_str, None)
                .await
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

    if cred.cli_type == "claude_code" {
        return Err("Claude Code 暂不支持官方直连写入".to_string());
    }

    let gateway_url = config.gateway_base_url();
    let current_mode = detect_cli_mode_from_url(db.inner(), &gateway_url, &cred.cli_type).await;
    let default_config = get_cli_default_config(db.inner(), &cred.cli_type).await?;
    if current_mode == CLI_MODE_PROVIDER_DIRECT {
        remove_provider_direct_config_async(db.inner(), &cred.cli_type).await?;
    } else if current_mode == CLI_MODE_PROXY_ROUTE {
        let has_gateway_config = check_cli_enabled(db.inner(), &cred.cli_type, &gateway_url).await;
        if has_gateway_config {
            sync_cli_config(
                db.inner(),
                &cred.cli_type,
                false,
                &default_config,
                None,
                &gateway_url,
            )
            .await?;
        }
    }

    sync_credential_to_cli_async(
        db.inner(),
        &cred.cli_type,
        &cred.credential_json,
        &default_config,
        None,
    )
    .await?;

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

async fn first_default_provider(db: &SqlitePool, cli_type: &str) -> Result<Provider> {
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

async fn preferred_default_provider(db: &SqlitePool, cli_type: &str) -> Result<Provider> {
    let last_provider_id: Option<i64> = sqlx::query_as::<_, (Option<i64>,)>(
        "SELECT last_provider_direct_provider_id FROM cli_settings WHERE cli_type = ?",
    )
    .bind(cli_type)
    .fetch_optional(db)
    .await
    .map_err(|e| e.to_string())?
    .and_then(|row| row.0);

    if let Some(id) = last_provider_id {
        if let Some(provider) = sqlx::query_as::<_, Provider>(
            "SELECT * FROM providers WHERE id = ? AND cli_type = ? AND profile = ?",
        )
        .bind(id)
        .bind(cli_type)
        .bind(DEFAULT_PROFILE)
        .fetch_optional(db)
        .await
        .map_err(|e| e.to_string())?
        {
            if provider.base_url.trim().is_empty() || provider.api_key.trim().is_empty() {
                return Err(format!(
                    "服务商 {} 的 Base URL 或 API Key 为空",
                    provider.name
                ));
            }
            return Ok(provider);
        }
    }

    first_default_provider(db, cli_type).await
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

async fn preferred_default_credential(db: &SqlitePool, cli_type: &str) -> Result<OfficialCredential> {
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

async fn remove_dashboard_provider_direct_config(db: &SqlitePool, cli_type: &str) -> Result<()> {
    match cli_type {
        "claude_code" => {
            let config_dir = get_cli_config_dir_path(db, "claude_code").await;
            let config_path =
                config_dir.join(cli_helpers::claude_settings_filename(DEFAULT_PROFILE));
            let gateway_config = claude_gateway_json_template();
            remove_json_config_content(&config_path, &gateway_config, "", &gateway_config).await
        }
        "codex" => {
            let config_dir = get_cli_config_dir_path(db, "codex").await;
            let config_path = codex_profile_config_path(&config_dir, DEFAULT_PROFILE);
            if !tokio::fs::try_exists(&config_path).await.unwrap_or(false) {
                return Ok(());
            }

            let content = tokio::fs::read_to_string(&config_path)
                .await
                .map_err(|e| e.to_string())?;
            let mut doc = match content.parse::<toml_edit::DocumentMut>() {
                Ok(doc) => doc,
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse Codex config {}, leaving file untouched: {}",
                        config_path.display(),
                        e
                    );
                    return Ok(());
                }
            };

            remove_codex_provider_direct_entry(&mut doc, DEFAULT_PROFILE)?;
            tokio::fs::write(&config_path, doc.to_string())
                .await
                .map_err(|e| e.to_string())
        }
        "gemini" => {
            let config_dir = get_cli_config_dir_path(db, "gemini").await;
            let env_path = config_dir.join(".env");
            if tokio::fs::try_exists(&env_path).await.unwrap_or(false) {
                tokio::fs::remove_file(&env_path)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        _ => Err("不支持的 CLI 类型".to_string()),
    }
}

async fn remember_current_default_provider_direct_provider(
    db: &SqlitePool,
    cli_type: &str,
) -> Result<()> {
    if let Some(id) = provider_direct_active_provider_id(db, cli_type, DEFAULT_PROFILE).await? {
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
        remove_dashboard_provider_direct_config(db, cli_type).await?;
    }

    let default_config = get_cli_default_config(db, cli_type).await?;
    sync_cli_config(db, cli_type, true, &default_config, None, &gateway_url).await?;

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
    log_db: &LogDb,
    cli_type: &str,
    current_mode: &str,
) -> Result<()> {
    if current_mode == CLI_MODE_OFFICIAL_DIRECT {
        remove_direct_mode_files_async(db, cli_type).await?;
    }

    let provider = preferred_default_provider(db, cli_type).await?;
    write_provider_direct_config(db, &provider).await?;
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
    if cli_type == "claude_code" {
        return Err("Claude Code 暂不支持官方直连".to_string());
    }

    let cred = preferred_default_credential(db, cli_type).await?;
    let gateway_url = config.gateway_base_url();
    let default_config = get_cli_default_config(db, cli_type).await?;

    if current_mode == CLI_MODE_PROVIDER_DIRECT {
        remember_current_default_provider_direct_provider(db, cli_type).await?;
        remove_dashboard_provider_direct_config(db, cli_type).await?;
    } else if current_mode == CLI_MODE_PROXY_ROUTE
        && check_cli_enabled(db, cli_type, &gateway_url).await
    {
        sync_cli_config(db, cli_type, false, &default_config, None, &gateway_url).await?;
    }

    sync_credential_to_cli_async(db, cli_type, &cred.credential_json, &default_config, None)
        .await?;
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
            sync_cli_config(db, cli_type, false, &default_config, None, &gateway_url).await?;
        }
        // 直连 → 停用：删除服务商配置 + 全局预设配置
        CLI_MODE_PROVIDER_DIRECT => {
            let default_config = get_cli_default_config(db, cli_type).await?;
            match cli_type {
                "claude_code" => {
                    let config_dir = get_cli_config_dir_path(db, "claude_code").await;
                    let config_path =
                        config_dir.join(cli_helpers::claude_settings_filename(DEFAULT_PROFILE));
                    let gateway_config = claude_gateway_json_template();
                    remove_json_config_content(
                        &config_path,
                        &gateway_config,
                        &default_config,
                        &gateway_config,
                    )
                    .await?;
                }
                "codex" => {
                    let config_dir = get_cli_config_dir_path(db, "codex").await;
                    let config_path = codex_profile_config_path(&config_dir, DEFAULT_PROFILE);
                    if tokio::fs::try_exists(&config_path).await.unwrap_or(false) {
                        let content = tokio::fs::read_to_string(&config_path)
                            .await
                            .map_err(|e| e.to_string())?;
                        if let Ok(mut doc) = content.parse::<toml_edit::DocumentMut>() {
                            remove_codex_provider_direct_entry(&mut doc, DEFAULT_PROFILE)?;
                            // 移除全局预设配置的 key
                            if !default_config.is_empty() {
                                if let Ok(preset_doc) =
                                    default_config.parse::<toml_edit::DocumentMut>()
                                {
                                    for (key, _) in preset_doc.iter() {
                                        doc.remove(key);
                                    }
                                }
                            }
                            tokio::fs::write(&config_path, doc.to_string())
                                .await
                                .map_err(|e| e.to_string())?;
                        }
                    }
                }
                "gemini" => {
                    let config_dir = get_cli_config_dir_path(db, "gemini").await;
                    // 清理 .env：移除 API key 和 base URL
                    let env_path = config_dir.join(".env");
                    if tokio::fs::try_exists(&env_path).await.unwrap_or(false) {
                        let content = tokio::fs::read_to_string(&env_path)
                            .await
                            .map_err(|e| e.to_string())?;
                        let filtered: Vec<&str> = content
                            .lines()
                            .filter(|line| {
                                let t = line.trim();
                                !t.starts_with("GEMINI_API_KEY=")
                                    && !t.starts_with("GOOGLE_GEMINI_BASE_URL=")
                            })
                            .collect();
                        if filtered.is_empty() {
                            tokio::fs::remove_file(&env_path)
                                .await
                                .map_err(|e| e.to_string())?;
                        } else {
                            tokio::fs::write(&env_path, filtered.join("\n") + "\n")
                                .await
                                .map_err(|e| e.to_string())?;
                        }
                    }
                    // 清理 settings.json：移除 auth 配置和全局预设
                    let settings_path = config_dir.join("settings.json");
                    let gateway_config = gemini_gateway_json_template();
                    remove_json_config_content(
                        &settings_path,
                        &gateway_config,
                        &default_config,
                        &gateway_config,
                    )
                    .await?;
                }
                _ => {}
            }
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
            write_dashboard_provider_direct(db, log_db, cli_type, current_mode).await
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
pub async fn get_cli_mode(db: State<'_, SqlitePool>, config: State<'_, Config>, cli_type: String) -> Result<String> {
    let gateway_url = config.gateway_base_url();
    Ok(detect_cli_mode_from_url(db.inner(), &gateway_url, &cli_type)
        .await
        .to_string())
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
