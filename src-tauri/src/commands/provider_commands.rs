use super::*;
use crate::db::models::{
    Provider, ProviderCreate, ProviderProfileCreate, ProviderProfileRename,
    ProviderProfileResponse, ProviderResponse, ProviderUpdate, TestProviderModelsInput,
};
use crate::services::provider_profile;
use crate::services::provider_profile::{
    provider_profile_exists_if_supported, validate_cli_type, validate_provider_profile,
};
use crate::time::now_timestamp;
use crate::LogDb;
use sqlx::SqlitePool;
use std::collections::HashMap;
use tauri::{Emitter, State};

#[tauri::command]
pub async fn get_provider_profiles(
    db: State<'_, SqlitePool>,
    cli_type: String,
) -> Result<Vec<ProviderProfileResponse>> {
    provider_profile::list_profiles(db.inner(), &cli_type).await
}

#[tauri::command]
pub async fn create_provider_profile(
    db: State<'_, SqlitePool>,
    input: ProviderProfileCreate,
) -> Result<ProviderProfileResponse> {
    provider_profile::create_profile(db.inner(), input).await
}

#[tauri::command]
pub async fn rename_provider_profile(
    db: State<'_, SqlitePool>,
    config: State<'_, Config>,
    profile: String,
    input: ProviderProfileRename,
) -> Result<ProviderProfileResponse> {
    provider_profile::rename_profile(db.inner(), &config.gateway_base_url(), &profile, input).await
}

#[tauri::command]
pub async fn delete_provider_profile(
    db: State<'_, SqlitePool>,
    config: State<'_, Config>,
    log_db: State<'_, LogDb>,
    cli_type: String,
    profile: String,
) -> Result<()> {
    provider_profile::delete_profile(
        db.inner(),
        &log_db.0,
        &config.gateway_base_url(),
        &cli_type,
        &profile,
    )
    .await
}

/// A multiplier of 1 is the official catalog price and the only safe fallback:
/// any non-positive value would silently zero out every cost for the channel.
fn normalize_price_multiplier(value: Option<f64>) -> f64 {
    value
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(1.0)
}

fn normalize_provider_update_multiplier(input: &ProviderUpdate) -> Option<f64> {
    input
        .price_multiplier
        .map(|value| normalize_price_multiplier(Some(value)))
}

/// 小于 1024 时 Anthropic 侧连思考预算都摆不下，直接钳住，免得存进去一个废值。
fn normalize_translate_max_tokens(value: Option<i64>) -> i64 {
    value
        .unwrap_or(crate::services::translate::DEFAULT_MAX_TOKENS)
        .max(1024)
}

/// 阶梯档位归一化：次数 1-50、时长 1-10080 分钟（最长一周），同次数去重。
/// 空列表回退默认一档（5 次拉黑 10 分钟），保证熔断判定永远有档可用。
fn normalize_blacklist_tiers(
    tiers: Option<&Vec<crate::db::models::BlacklistTierInput>>,
) -> Vec<(i64, i64)> {
    let mut normalized: Vec<(i64, i64)> = tiers
        .map(|tiers| {
            tiers
                .iter()
                .map(|tier| {
                    (
                        tier.failure_count.clamp(1, 50),
                        tier.blacklist_minutes.clamp(1, 10080),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    normalized.sort();
    normalized.dedup_by(|a, b| a.0 == b.0);
    if normalized.is_empty() {
        normalized.push((5, 10));
    }
    normalized
}

async fn insert_blacklist_tiers_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    provider_id: i64,
    tiers: &[(i64, i64)],
    now: i64,
) -> Result<()> {
    for (failure_count, blacklist_minutes) in tiers {
        sqlx::query(
            "INSERT INTO provider_blacklist_tier (provider_id, failure_count, blacklist_minutes, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(provider_id)
        .bind(failure_count)
        .bind(blacklist_minutes)
        .bind(now)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    Ok(())
}

fn validate_provider_protocol(agent_id: &str, protocol: Option<&str>) -> Result<String> {
    let definition = crate::services::agent::get_definition(agent_id)
        .ok_or_else(|| format!("未知 Agent: {}", agent_id))?;
    let declared = &definition.protocols;
    let protocol = match protocol.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => value
            .parse::<crate::db::models::Protocol>()
            .map_err(|_| format!("无效 Protocol: {}", value))?,
        None if declared.len() == 1 => declared[0],
        None => return Err("该 Agent 支持多个 Protocol，请明确选择".to_string()),
    };
    if !declared.contains(&protocol) {
        // 端点类型可以和 Agent 声明的协议不同，网关转发时自动转换；Gemini 不参与
        // 转换，所以两边都必须落在可转换集合里。
        let convertible = crate::services::translate::is_convertible(protocol)
            && declared
                .iter()
                .copied()
                .any(crate::services::translate::is_convertible);
        if !convertible {
            return Err(format!("Agent {} 无法使用 Protocol {}", agent_id, protocol));
        }
    }
    Ok(protocol.as_str().to_string())
}

struct ProviderInsert<'a> {
    cli_type: &'a str,
    profile: &'a str,
    protocol: &'a str,
    input: &'a ProviderCreate,
    custom_useragent: Option<&'a str>,
    price_multiplier: f64,
    now: i64,
}

async fn insert_provider_record(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    values: ProviderInsert<'_>,
) -> Result<i64> {
    let result = sqlx::query(
        r#"
        INSERT INTO providers (cli_type, profile, protocol, name, base_url, api_key, enabled, consecutive_failures, sort_order, custom_useragent, created_at, updated_at, price_multiplier, translate_max_tokens)
        VALUES (?, ?, ?, ?, ?, ?, ?, 0, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM providers WHERE cli_type = ? AND profile = ?), ?, ?, ?, ?, ?)
        "#,
    )
    .bind(values.cli_type)
    .bind(values.profile)
    .bind(values.protocol)
    .bind(&values.input.name)
    .bind(&values.input.base_url)
    .bind(&values.input.api_key)
    .bind(values.input.enabled.unwrap_or(true) as i64)
    .bind(values.cli_type)
    .bind(values.profile)
    .bind(values.custom_useragent)
    .bind(values.now)
    .bind(values.now)
    .bind(values.price_multiplier)
    .bind(normalize_translate_max_tokens(
        values.input.translate_max_tokens,
    ))
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(result.last_insert_rowid())
}

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

    let tiers_sql = format!(
        "SELECT provider_id, failure_count, blacklist_minutes FROM provider_blacklist_tier WHERE provider_id IN ({}) ORDER BY provider_id, failure_count",
        placeholders
    );
    let mut tiers_query = sqlx::query_as::<_, (i64, i64, i64)>(&tiers_sql);
    for provider_id in &provider_ids {
        tiers_query = tiers_query.bind(*provider_id);
    }
    let all_tiers = tiers_query
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
            acc.entry(provider_id).or_insert_with(Vec::new).push((id, model_pattern));
            acc
        },
    );

    let tiers_by_provider: HashMap<i64, Vec<(i64, i64)>> = all_tiers.into_iter().fold(
        HashMap::new(),
        |mut acc, (provider_id, failure_count, blacklist_minutes)| {
            acc.entry(provider_id).or_insert_with(Vec::new).push((failure_count, blacklist_minutes));
            acc
        },
    );

    // 组装结果
    let results: Vec<ProviderResponse> = providers
        .into_iter()
        .map(|provider| {
            let mut response = ProviderResponse::from(provider.clone());

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

            // 从分组数据中获取 blacklist_tiers（已按 failure_count 升序）
            response.blacklist_tiers = tiers_by_provider
                .get(&provider.id)
                .map(|tiers| {
                    tiers.iter()
                        .enumerate()
                        .map(|(index, (failure_count, blacklist_minutes))| {
                            crate::db::models::BlacklistTierResponse {
                                id: index as i64,
                                failure_count: *failure_count,
                                blacklist_minutes: *blacklist_minutes,
                            }
                        })
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

    // Load blacklist tiers（按失败次数升序，即从最小档到最高档）
    let tiers: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT failure_count, blacklist_minutes FROM provider_blacklist_tier WHERE provider_id = ? ORDER BY failure_count",
    )
    .bind(id)
    .fetch_all(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    response.blacklist_tiers = tiers
        .into_iter()
        .enumerate()
        .map(|(index, (failure_count, blacklist_minutes))| {
            crate::db::models::BlacklistTierResponse {
                id: index as i64,
                failure_count,
                blacklist_minutes,
            }
        })
        .collect();

    Ok(response)
}

async fn apply_first_enabled_provider_direct(
    db: &SqlitePool,
    cli_type: &str,
    profile: &str,
) -> Result<()> {
    let provider = crate::services::routing::get_first_enabled_provider_for_direct(
        db, cli_type, profile,
    )
    .await
    .map_err(|e| e.to_string())?;

    if let Some(provider) = provider {
        crate::services::agent_config::write_provider_direct_config(db, &provider).await?;
        remember_default_provider_direct_provider(db, &provider, now_timestamp()).await?;
    } else if profile == DEFAULT_PROFILE {
        sqlx::query(
            "UPDATE cli_settings SET last_provider_direct_provider_id = NULL, updated_at = ? WHERE cli_type = ?",
        )
        .bind(now_timestamp())
        .bind(cli_type)
        .execute(db)
        .await
        .map_err(map_db_error)?;
    }

    Ok(())
}

async fn refresh_provider_direct_config(db: &SqlitePool, active: &Provider) -> Result<()> {
    crate::services::agent_config::remove_provider_direct_config_for_provider(db, active).await?;
    apply_first_enabled_provider_direct(db, &active.cli_type, &active.profile).await
}

#[tauri::command]
pub async fn write_provider_direct_config_command(
    db: State<'_, SqlitePool>,
    config: State<'_, Config>,
    log_db: State<'_, LogDb>,
    id: i64,
) -> Result<ProviderResponse> {
    let provider = sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "服务商不存在".to_string())?;
    if provider.enabled == 0 {
        return Err("已停用的服务商不能用于中转直连".to_string());
    }

    let agent = crate::services::agent::get_agent(db.inner(), &provider.cli_type)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("未知 Agent: {}", provider.cli_type))?;
    if !agent.features.provider_config.enabled
        || agent.features.provider_config.operations.is_empty()
    {
        return Err(format!("Agent {} 不支持服务商直连模式", agent.name));
    }

    let gateway_url = config.gateway_base_url();
    if crate::services::agent_config::is_provider_config_applied(
        db.inner(),
        &provider.cli_type,
        &gateway_url,
        &provider.profile,
    )
    .await
    {
        let default_config = get_cli_default_config(db.inner(), &provider.cli_type).await?;
        let write_mode = get_config_write_mode(db.inner(), &provider.cli_type).await;
        crate::services::agent_config::sync_proxy_route_config(
            db.inner(),
            &provider.cli_type,
            false,
            &gateway_url,
            &provider.profile,
            &default_config,
            None,
            &write_mode,
        )
        .await?;
    }

    crate::services::agent_config::write_provider_direct_config(db.inner(), &provider).await?;
    let now = now_timestamp();
    remember_default_provider_direct_provider(db.inner(), &provider, now).await?;

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
    let price_multiplier = normalize_price_multiplier(input.price_multiplier);
    let cli_type = validate_cli_type(
        input
            .cli_type
            .as_deref()
            .ok_or_else(|| "必须指定 Agent".to_string())?,
    )?;
    let protocol = validate_provider_protocol(&cli_type, input.protocol.as_deref())?;
    let profile = validate_provider_profile(input.profile.as_deref())?.to_string();
    if !provider_profile_exists_if_supported(db.inner(), &cli_type, &profile).await? {
        return Err("Profile 不存在".to_string());
    }
    let provider_name = input.name.clone();

    // Normalize custom_useragent: treat empty string as None
    let custom_ua = input
        .custom_useragent
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let mut tx = db.begin().await.map_err(|e| e.to_string())?;
    let id = insert_provider_record(
        &mut tx,
        ProviderInsert {
            cli_type: &cli_type,
            profile: &profile,
            protocol: &protocol,
            input: &input,
            custom_useragent: custom_ua.as_deref(),
            price_multiplier,
            now,
        },
    )
    .await?;

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
            .execute(&mut *tx)
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
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
    }

    // Insert blacklist tiers（空列表归一化时已回退默认一档）
    let tiers = normalize_blacklist_tiers(input.blacklist_tiers.as_ref());
    insert_blacklist_tiers_tx(&mut tx, id, &tiers, now).await?;

    tx.commit().await.map_err(|e| e.to_string())?;

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
    let price_multiplier = normalize_provider_update_multiplier(&input);

    let provider_before: Provider = sqlx::query_as("SELECT * FROM providers WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "服务商不存在".to_string())?;
    let provider_name = provider_before.name.clone();
    let provider_cli_type = provider_before.cli_type.clone();
    let active_direct_provider_id = crate::services::agent_config::provider_direct_active_provider_id(
        db.inner(),
        &provider_before.cli_type,
        &provider_before.profile,
    )
    .await?;
    let active_direct_provider = match active_direct_provider_id {
        Some(active_id) if active_id == id => Some(provider_before.clone()),
        Some(active_id) => sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE id = ?")
            .bind(active_id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?,
        None => None,
    };
    let was_direct_active = active_direct_provider_id == Some(id);

    // Check if model maps will be updated (before moving)
    let has_model_maps_update = input.model_maps.is_some();
    let has_model_blacklist_update = input.model_blacklist.is_some();
    let has_blacklist_tiers_update = input.blacklist_tiers.is_some();
    let normalized_profile = if let Some(ref profile) = input.profile {
        Some(validate_provider_profile(Some(profile.as_str()))?.to_string())
    } else {
        None
    };
    let normalized_protocol = input
        .protocol
        .as_deref()
        .map(|protocol| validate_provider_protocol(&provider_cli_type, Some(protocol)))
        .transpose()?;
    if let Some(ref profile) = normalized_profile {
        if !provider_profile_exists_if_supported(db.inner(), &provider_cli_type, profile).await? {
            return Err("Profile 不存在".to_string());
        }
    }
    let profile_changed = normalized_profile
        .as_ref()
        .is_some_and(|profile| profile != &provider_before.profile);
    let protocol_changed = normalized_protocol
        .as_ref()
        .is_some_and(|protocol| protocol != &provider_before.protocol);
    let base_url_changed = input.base_url.as_ref().is_some_and(|base_url| {
        base_url.trim().trim_end_matches('/')
            != provider_before.base_url.trim().trim_end_matches('/')
    });
    let api_key_changed = input
        .api_key
        .as_ref()
        .is_some_and(|api_key| api_key.trim() != provider_before.api_key.trim());
    let enabled_changed = input
        .enabled
        .is_some_and(|enabled| enabled != (provider_before.enabled != 0));
    let custom_useragent_changed = input.custom_useragent.as_ref().is_some_and(|useragent| {
        useragent.trim()
            != provider_before
                .custom_useragent
                .as_deref()
                .unwrap_or_default()
                .trim()
    });
    let provider_config_changed =
        profile_changed || protocol_changed || base_url_changed || api_key_changed;
    let model_sync_config_changed =
        protocol_changed || base_url_changed || api_key_changed || custom_useragent_changed;

    // Build dynamic update query
    let mut updates = vec!["updated_at = ?".to_string()];
    let mut has_updates = false;

    if normalized_profile.is_some() {
        updates.push("profile = ?".to_string());
        has_updates = true;
    }
    if normalized_protocol.is_some() {
        updates.push("protocol = ?".to_string());
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
    if input.custom_useragent.is_some() {
        updates.push("custom_useragent = ?".to_string());
        has_updates = true;
    }
    if price_multiplier.is_some() {
        updates.push("price_multiplier = ?".to_string());
        has_updates = true;
    }
    if input.translate_max_tokens.is_some() {
        updates.push("translate_max_tokens = ?".to_string());
        has_updates = true;
    }

    let mut tx = db.begin().await.map_err(|e| e.to_string())?;
    if has_updates {
        let query = format!("UPDATE providers SET {} WHERE id = ?", updates.join(", "));
        let mut q = sqlx::query(&query).bind(now);

        if let Some(ref profile) = normalized_profile {
            q = q.bind(profile);
        }
        if let Some(ref protocol) = normalized_protocol {
            q = q.bind(protocol);
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
        if let Some(ref custom_useragent) = input.custom_useragent {
            // Normalize: treat empty string as NULL
            let ua = custom_useragent.trim();
            if ua.is_empty() {
                q = q.bind(None::<String>);
            } else {
                q = q.bind(ua);
            }
        }
        if let Some(value) = price_multiplier {
            q = q.bind(value);
        }
        if input.translate_max_tokens.is_some() {
            q = q.bind(normalize_translate_max_tokens(input.translate_max_tokens));
        }

        q.bind(id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;

        // The old model snapshot no longer describes the saved endpoint, so it
        // stops counting as synced until the user syncs against the new one.
        if model_sync_config_changed {
            crate::services::model_sync::invalidate_sync_state_tx(&mut tx, id).await?;
        }
    }

    // Update model maps if provided
    if let Some(model_maps) = input.model_maps {
        // Delete existing maps
        sqlx::query("DELETE FROM provider_model_map WHERE provider_id = ?")
            .bind(id)
            .execute(&mut *tx)
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
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
    }

    // Update model blacklist if provided
    if let Some(model_blacklist) = input.model_blacklist {
        // Delete existing blacklist
        sqlx::query("DELETE FROM provider_model_blacklist WHERE provider_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;

        // Insert new blacklist
        for item in model_blacklist {
            sqlx::query(
                "INSERT INTO provider_model_blacklist (provider_id, model_pattern) VALUES (?, ?)",
            )
            .bind(id)
            .bind(&item.model_pattern)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
    }

    // Update blacklist tiers if provided（整体替换，空列表归一化时已回退默认一档）
    if input.blacklist_tiers.is_some() {
        sqlx::query("DELETE FROM provider_blacklist_tier WHERE provider_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;

        let tiers = normalize_blacklist_tiers(input.blacklist_tiers.as_ref());
        insert_blacklist_tiers_tx(&mut tx, id, &tiers, now).await?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    if let Some(active) = active_direct_provider
        .as_ref()
        .filter(|_| enabled_changed || (was_direct_active && provider_config_changed))
    {
        refresh_provider_direct_config(db.inner(), active).await?;
    }

    // Log system event (only if there were actual updates)
    if has_updates
        || has_model_maps_update
        || has_model_blacklist_update
        || has_blacklist_tiers_update
    {
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
    let Some(provider) = sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };
    let provider_name = provider.name.clone();
    let was_direct_active = crate::services::agent_config::provider_direct_active_provider_id(
        db.inner(),
        &provider.cli_type,
        &provider.profile,
    )
    .await?
        == Some(id);
    if was_direct_active {
        crate::services::agent_config::remove_provider_direct_config_for_provider(
            db.inner(),
            &provider,
        )
        .await?;
    }

    let mut tx = db.begin().await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM provider_model_map WHERE provider_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

    sqlx::query("DELETE FROM provider_model_blacklist WHERE provider_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

    sqlx::query("DELETE FROM provider_blacklist_tier WHERE provider_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

    sqlx::query("DELETE FROM provider_models WHERE provider_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

    sqlx::query("DELETE FROM provider_model_sync_state WHERE provider_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

    sqlx::query("DELETE FROM providers WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
    sqlx::query(
        "UPDATE cli_settings SET last_provider_direct_provider_id = NULL WHERE last_provider_direct_provider_id = ?",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(map_db_error)?;
    tx.commit().await.map_err(|e| e.to_string())?;

    if was_direct_active {
        apply_first_enabled_provider_direct(db.inner(), &provider.cli_type, &provider.profile)
            .await?;
    }

    // Log system event
    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "provider_deleted",
        &format!("服务商 {} 已删除", provider_name),
    )
    .await;

    Ok(())
}

async fn reorder_providers_impl(db: &SqlitePool, ids: Vec<i64>) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }

    let scope = sqlx::query_as::<_, (String, String)>(
        "SELECT cli_type, profile FROM providers WHERE id = ?",
    )
    .bind(ids[0])
    .fetch_optional(db)
    .await
    .map_err(|e| e.to_string())?;
    let active_direct_provider = if let Some((cli_type, profile)) = &scope {
        let active_id = crate::services::agent_config::provider_direct_active_provider_id(
            db, cli_type, profile,
        )
        .await?;
        if let Some(active_id) = active_id {
            sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE id = ?")
                .bind(active_id)
                .fetch_optional(db)
                .await
                .map_err(|e| e.to_string())?
        } else {
            None
        }
    } else {
        None
    };

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

    sqlx::query(&sql).execute(db).await.map_err(map_db_error)?;

    if let Some(active) = &active_direct_provider {
        refresh_provider_direct_config(db, active).await?;
    }

    Ok(())
}

#[tauri::command]
pub async fn reorder_providers(db: State<'_, SqlitePool>, ids: Vec<i64>) -> Result<()> {
    reorder_providers_impl(db.inner(), ids).await
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
    let test_text = input.test_text.clone();

    let timeout_secs = provider_service::get_stream_first_byte_timeout(db.inner()).await;

    for provider_id in input.provider_ids {
        let pool = db_pool.clone();
        let model = model_name.clone();
        let test_text = test_text.clone();
        let app_handle = app.clone();

        tokio::spawn(async move {
            let result = provider_service::test_provider_model(
                &pool,
                provider_id,
                &model,
                test_text.as_deref(),
                timeout_secs,
            )
            .await;
            if let Err(e) = app_handle.emit("provider-test-result", result) {
                tracing::error!(error = %e, "Failed to emit test result");
            }
        });
    }

    Ok(())
}
