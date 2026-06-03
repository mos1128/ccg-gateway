use super::*;
use crate::db::models::{
    Provider, ProviderCreate, ProviderResponse, ProviderUpdate, TestProviderModelsInput,
};
use crate::time::now_timestamp;
use crate::LogDb;
use sqlx::SqlitePool;
use std::collections::HashMap;
use tauri::{Emitter, State};
#[tauri::command]
pub async fn get_providers(
    db: State<'_, SqlitePool>,
    cli_type: Option<String>,
    profile: Option<String>,
) -> Result<Vec<ProviderResponse>> {
    let profile = match profile {
        Some(value) => Some(validate_provider_profile(Some(&value))?.to_string()),
        None => None,
    };
    let active_provider_id = match (&cli_type, &profile) {
        (Some(ct), Some(profile)) => provider_direct_active_provider_id(db.inner(), ct, profile)
            .await
            .unwrap_or(None),
        _ => None,
    };

    let providers = match (cli_type, profile) {
        (Some(ct), Some(profile)) => sqlx::query_as::<_, Provider>(
            "SELECT * FROM providers WHERE cli_type = ? AND profile = ? ORDER BY sort_order, id",
        )
        .bind(&ct)
        .bind(&profile)
        .fetch_all(db.inner())
        .await,
        (Some(ct), None) => {
            sqlx::query_as::<_, Provider>(
                "SELECT * FROM providers WHERE cli_type = ? ORDER BY sort_order, id",
            )
            .bind(&ct)
            .fetch_all(db.inner())
            .await
        }
        (None, Some(profile)) => {
            sqlx::query_as::<_, Provider>(
                "SELECT * FROM providers WHERE profile = ? ORDER BY cli_type, sort_order, id",
            )
            .bind(&profile)
            .fetch_all(db.inner())
            .await
        }
        (None, None) => {
            sqlx::query_as::<_, Provider>(
                "SELECT * FROM providers ORDER BY cli_type, profile, sort_order, id",
            )
            .fetch_all(db.inner())
            .await
        }
    };

    let providers = providers.map_err(|e| e.to_string())?;
    if providers.is_empty() {
        return Ok(Vec::new());
    }

    let provider_ids: Vec<i64> = providers.iter().map(|provider| provider.id).collect();
    let placeholders = vec!["?"; provider_ids.len()].join(", ");

    let map_sql = format!(
        "SELECT id, provider_id, source_model, target_model, enabled FROM provider_model_map WHERE provider_id IN ({}) ORDER BY provider_id, id",
        placeholders
    );
    let mut map_query = sqlx::query_as::<_, (i64, i64, String, String, i64)>(&map_sql);
    for provider_id in &provider_ids {
        map_query = map_query.bind(*provider_id);
    }
    let all_maps = map_query
        .fetch_all(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    let blacklist_sql = format!(
        "SELECT id, provider_id, model_pattern FROM provider_model_blacklist WHERE provider_id IN ({}) ORDER BY provider_id, id",
        placeholders
    );
    let mut blacklist_query = sqlx::query_as::<_, (i64, i64, String)>(&blacklist_sql);
    for provider_id in &provider_ids {
        blacklist_query = blacklist_query.bind(*provider_id);
    }
    let all_blacklist = blacklist_query
        .fetch_all(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    let maps_by_provider: HashMap<i64, Vec<_>> = all_maps.into_iter().fold(
        HashMap::new(),
        |mut acc, (id, provider_id, source_model, target_model, enabled)| {
            acc.entry(provider_id).or_insert_with(Vec::new).push((
                id,
                source_model,
                target_model,
                enabled,
            ));
            acc
        },
    );

    let blacklist_by_provider: HashMap<i64, Vec<_>> = all_blacklist.into_iter().fold(
        HashMap::new(),
        |mut acc, (id, provider_id, model_pattern)| {
            acc.entry(provider_id)
                .or_insert_with(Vec::new)
                .push((id, model_pattern));
            acc
        },
    );

    // 组装结果
    let results: Vec<ProviderResponse> = providers
        .into_iter()
        .map(|provider| {
            let mut response = ProviderResponse::from(provider.clone());
            response.is_direct_active = active_provider_id == Some(provider.id);

            // 从分组数据中获取 model_maps
            response.model_maps = maps_by_provider
                .get(&provider.id)
                .map(|maps| {
                    maps.iter()
                        .map(|(id, source_model, target_model, enabled)| {
                            crate::db::models::ModelMapResponse {
                                id: *id,
                                source_model: source_model.clone(),
                                target_model: target_model.clone(),
                                enabled: *enabled != 0,
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            // 从分组数据中获取 model_blacklist
            response.model_blacklist = blacklist_by_provider
                .get(&provider.id)
                .map(|blacklist| {
                    blacklist
                        .iter()
                        .map(
                            |(id, model_pattern)| crate::db::models::ModelBlacklistResponse {
                                id: *id,
                                model_pattern: model_pattern.clone(),
                            },
                        )
                        .collect()
                })
                .unwrap_or_default();

            response
        })
        .collect();

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

    let active_provider_id =
        provider_direct_active_provider_id(db.inner(), &provider.cli_type, &provider.profile)
            .await
            .unwrap_or(None);
    let mut response = ProviderResponse::from(provider);
    response.is_direct_active = active_provider_id == Some(response.id);

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
        .map(
            |(id, source_model, target_model, enabled)| crate::db::models::ModelMapResponse {
                id,
                source_model,
                target_model,
                enabled: enabled != 0,
            },
        )
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
        .map(|(id, model_pattern)| crate::db::models::ModelBlacklistResponse { id, model_pattern })
        .collect();

    Ok(response)
}

#[tauri::command]
pub async fn write_provider_direct_config_command(
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    id: i64,
) -> Result<ProviderResponse> {
    let provider = sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "服务商不存在".to_string())?;

    write_provider_direct_config(db.inner(), &provider).await?;
    set_normalized_cli_mode(
        db.inner(),
        &provider.cli_type,
        CLI_MODE_PROVIDER_DIRECT,
        now_timestamp(),
    )
    .await?;

    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "provider_direct_written",
        &format!("服务商 {} 已写入 CLI 配置", provider.name),
    )
    .await;

    get_provider(db, id).await
}

#[tauri::command]
pub async fn create_provider(
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    input: ProviderCreate,
) -> Result<ProviderResponse> {
    let now = now_timestamp();
    let cli_type = input.cli_type.unwrap_or_else(|| "claude_code".to_string());
    let profile = validate_provider_profile(input.profile.as_deref())?.to_string();
    let provider_name = input.name.clone();

    // Normalize custom_useragent: treat empty string as None
    let custom_ua = input
        .custom_useragent
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let result = sqlx::query(
        r#"
        INSERT INTO providers (cli_type, profile, name, base_url, api_key, enabled, failure_threshold, blacklist_minutes, consecutive_failures, sort_order, custom_useragent, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM providers WHERE cli_type = ? AND profile = ?), ?, ?, ?)
        "#,
    )
    .bind(&cli_type)
    .bind(&profile)
    .bind(&input.name)
    .bind(&input.base_url)
    .bind(&input.api_key)
    .bind(input.enabled.unwrap_or(true) as i64)
    .bind(input.failure_threshold.unwrap_or(5))
    .bind(input.blacklist_minutes.unwrap_or(10))
    .bind(&cli_type)
    .bind(&profile)
    .bind(&custom_ua)
    .bind(now)
    .bind(now)
    .execute(db.inner())
    .await
    .map_err(map_db_error)?;

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
            .map_err(map_db_error)?;
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
            .map_err(map_db_error)?;
        }
    }

    // Log system event
    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "provider_created",
        &format!("服务商 {} 已创建", provider_name),
    )
    .await;

    get_provider(db, id).await
}

#[tauri::command]
pub async fn update_provider(
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    id: i64,
    input: ProviderUpdate,
) -> Result<ProviderResponse> {
    let now = now_timestamp();

    // Get provider name for logging
    let provider_name: Option<(String,)> =
        sqlx::query_as("SELECT name FROM providers WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?;

    let provider_name = provider_name
        .map(|(n,)| n)
        .unwrap_or_else(|| format!("Provider#{}", id));

    // Check if model maps will be updated (before moving)
    let has_model_maps_update = input.model_maps.is_some();
    let has_model_blacklist_update = input.model_blacklist.is_some();
    let normalized_profile = if let Some(ref profile) = input.profile {
        Some(validate_provider_profile(Some(profile.as_str()))?.to_string())
    } else {
        None
    };

    // Build dynamic update query
    let mut updates = vec!["updated_at = ?".to_string()];
    let mut has_updates = false;

    if normalized_profile.is_some() {
        updates.push("profile = ?".to_string());
        has_updates = true;
    }
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

        if let Some(ref profile) = normalized_profile {
            q = q.bind(profile);
        }
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

        q.bind(id).execute(db.inner()).await.map_err(map_db_error)?;
    }

    // Update model maps if provided
    if let Some(model_maps) = input.model_maps {
        // Delete existing maps
        sqlx::query("DELETE FROM provider_model_map WHERE provider_id = ?")
            .bind(id)
            .execute(db.inner())
            .await
            .map_err(map_db_error)?;

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
            .map_err(map_db_error)?;
        }
    }

    // Update model blacklist if provided
    if let Some(model_blacklist) = input.model_blacklist {
        // Delete existing blacklist
        sqlx::query("DELETE FROM provider_model_blacklist WHERE provider_id = ?")
            .bind(id)
            .execute(db.inner())
            .await
            .map_err(map_db_error)?;

        // Insert new blacklist
        for item in model_blacklist {
            sqlx::query(
                "INSERT INTO provider_model_blacklist (provider_id, model_pattern) VALUES (?, ?)",
            )
            .bind(id)
            .bind(&item.model_pattern)
            .execute(db.inner())
            .await
            .map_err(map_db_error)?;
        }
    }

    // Log system event (only if there were actual updates)
    if has_updates || has_model_maps_update || has_model_blacklist_update {
        let _ = crate::services::stats::record_system_log(
            &log_db.0,
            "provider_updated",
            &format!("服务商 {} 已更新", provider_name),
        )
        .await;
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
    let provider_name: Option<(String,)> =
        sqlx::query_as("SELECT name FROM providers WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?;

    let provider_name = provider_name
        .map(|(n,)| n)
        .unwrap_or_else(|| format!("Provider#{}", id));

    // Delete associated model maps first (cascade delete)
    sqlx::query("DELETE FROM provider_model_map WHERE provider_id = ?")
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

    // Delete associated model blacklist
    sqlx::query("DELETE FROM provider_model_blacklist WHERE provider_id = ?")
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

    // Then delete the provider
    sqlx::query("DELETE FROM providers WHERE id = ?")
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

    // Log system event
    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "provider_deleted",
        &format!("服务商 {} 已删除", provider_name),
    )
    .await;

    Ok(())
}

#[tauri::command]
pub async fn reorder_providers(db: State<'_, SqlitePool>, ids: Vec<i64>) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }

    // 使用 CASE WHEN 批量更新（避免 N 次单独更新）
    let case_clauses: Vec<String> = ids
        .iter()
        .enumerate()
        .map(|(idx, id)| format!("WHEN {} THEN {}", id, idx))
        .collect();

    let id_list: Vec<String> = ids.iter().map(|id| id.to_string()).collect();

    let sql = format!(
        "UPDATE providers SET sort_order = CASE id {} END WHERE id IN ({})",
        case_clauses.join(" "),
        id_list.join(", ")
    );

    sqlx::query(&sql)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

    Ok(())
}

#[tauri::command]
pub async fn reset_provider_failures(
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    id: i64,
) -> Result<()> {
    // Get provider name for logging
    let provider_name: Option<(String,)> =
        sqlx::query_as("SELECT name FROM providers WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?;

    let provider_name = provider_name
        .map(|(n,)| n)
        .unwrap_or_else(|| format!("Provider#{}", id));

    sqlx::query(
        "UPDATE providers SET consecutive_failures = 0, blacklisted_until = NULL WHERE id = ?",
    )
    .bind(id)
    .execute(db.inner())
    .await
    .map_err(map_db_error)?;

    // Log system event
    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "provider_reset",
        &format!("服务商 {} 状态已手动重置", provider_name),
    )
    .await;

    Ok(())
}

#[tauri::command]
pub async fn test_provider_models(
    app: tauri::AppHandle,
    db: State<'_, SqlitePool>,
    input: TestProviderModelsInput,
) -> Result<()> {
    use crate::services::provider as provider_service;

    let db_pool = db.inner().clone();
    let model_name = input.model_name.clone();

    let timeout_secs = provider_service::get_stream_first_byte_timeout(db.inner()).await;

    for provider_id in input.provider_ids {
        let pool = db_pool.clone();
        let model = model_name.clone();
        let app_handle = app.clone();

        tokio::spawn(async move {
            let result =
                provider_service::test_provider_model(&pool, provider_id, &model, timeout_secs)
                    .await;
            if let Err(e) = app_handle.emit("provider-test-result", result) {
                tracing::error!(error = %e, "Failed to emit test result");
            }
        });
    }

    Ok(())
}
