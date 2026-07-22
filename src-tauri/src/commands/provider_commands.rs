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
    cli_type: String,
    profile: String,
) -> Result<()> {
    provider_profile::delete_profile(db.inner(), &config.gateway_base_url(), &cli_type, &profile)
        .await
}

fn normalize_price_per_m(value: Option<f64>, field: &str) -> Result<f64> {
    let value = value.unwrap_or(0.0);
    if !value.is_finite() || value < 0.0 {
        return Err(format!("{} 必须是大于等于 0 的数字", field));
    }
    Ok(value)
}

fn normalize_provider_prices(input: &ProviderCreate) -> Result<(f64, f64, f64, f64)> {
    Ok((
        normalize_price_per_m(input.input_price_per_m, "输入单价")?,
        normalize_price_per_m(input.output_price_per_m, "输出单价")?,
        normalize_price_per_m(input.cache_read_price_per_m, "缓存读取单价")?,
        normalize_price_per_m(input.cache_creation_price_per_m, "缓存创建单价")?,
    ))
}

fn normalize_provider_update_prices(
    input: &ProviderUpdate,
) -> Result<(Option<f64>, Option<f64>, Option<f64>, Option<f64>)> {
    Ok((
        input
            .input_price_per_m
            .map(|value| normalize_price_per_m(Some(value), "输入单价"))
            .transpose()?,
        input
            .output_price_per_m
            .map(|value| normalize_price_per_m(Some(value), "输出单价"))
            .transpose()?,
        input
            .cache_read_price_per_m
            .map(|value| normalize_price_per_m(Some(value), "缓存读取单价"))
            .transpose()?,
        input
            .cache_creation_price_per_m
            .map(|value| normalize_price_per_m(Some(value), "缓存创建单价"))
            .transpose()?,
    ))
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
        return Err(format!("Agent {} 未声明 Protocol {}", agent_id, protocol));
    }
    Ok(protocol.as_str().to_string())
}

struct ProviderInsert<'a> {
    cli_type: &'a str,
    profile: &'a str,
    protocol: &'a str,
    input: &'a ProviderCreate,
    custom_useragent: Option<&'a str>,
    prices: (f64, f64, f64, f64),
    now: i64,
}

async fn insert_provider_record(pool: &SqlitePool, values: ProviderInsert<'_>) -> Result<i64> {
    let result = sqlx::query(
        r#"
        INSERT INTO providers (cli_type, profile, protocol, name, base_url, api_key, enabled, failure_threshold, blacklist_minutes, consecutive_failures, sort_order, custom_useragent, created_at, updated_at, input_price_per_m, output_price_per_m, cache_read_price_per_m, cache_creation_price_per_m)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM providers WHERE cli_type = ? AND profile = ?), ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(values.cli_type)
    .bind(values.profile)
    .bind(values.protocol)
    .bind(&values.input.name)
    .bind(&values.input.base_url)
    .bind(&values.input.api_key)
    .bind(values.input.enabled.unwrap_or(true) as i64)
    .bind(values.input.failure_threshold.unwrap_or(5))
    .bind(values.input.blacklist_minutes.unwrap_or(10))
    .bind(values.cli_type)
    .bind(values.profile)
    .bind(values.custom_useragent)
    .bind(values.now)
    .bind(values.now)
    .bind(values.prices.0)
    .bind(values.prices.1)
    .bind(values.prices.2)
    .bind(values.prices.3)
    .execute(pool)
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
    let active_provider_id = match (&cli_type, &profile) {
        (Some(ct), Some(profile)) => {
            crate::services::agent_config::provider_direct_active_provider_id(
                db.inner(),
                ct,
                profile,
            )
            .await
            .unwrap_or(None)
        }
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

    let active_provider_id = crate::services::agent_config::provider_direct_active_provider_id(
        db.inner(),
        &provider.cli_type,
        &provider.profile,
    )
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
    let (input_price_per_m, output_price_per_m, cache_read_price_per_m, cache_creation_price_per_m) =
        normalize_provider_prices(&input)?;
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

    let id = insert_provider_record(
        db.inner(),
        ProviderInsert {
            cli_type: &cli_type,
            profile: &profile,
            protocol: &protocol,
            input: &input,
            custom_useragent: custom_ua.as_deref(),
            prices: (
                input_price_per_m,
                output_price_per_m,
                cache_read_price_per_m,
                cache_creation_price_per_m,
            ),
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
    let (input_price_per_m, output_price_per_m, cache_read_price_per_m, cache_creation_price_per_m) =
        normalize_provider_update_prices(&input)?;

    let provider_before: Provider = sqlx::query_as("SELECT * FROM providers WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "服务商不存在".to_string())?;
    let provider_name = provider_before.name.clone();
    let provider_cli_type = provider_before.cli_type.clone();
    let was_direct_active = crate::services::agent_config::provider_direct_active_provider_id(
        db.inner(),
        &provider_before.cli_type,
        &provider_before.profile,
    )
    .await?
        == Some(id);

    // Check if model maps will be updated (before moving)
    let has_model_maps_update = input.model_maps.is_some();
    let has_model_blacklist_update = input.model_blacklist.is_some();
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
    let provider_config_changed = profile_changed
        || normalized_protocol
            .as_ref()
            .is_some_and(|protocol| protocol != &provider_before.protocol)
        || input.base_url.as_ref().is_some_and(|base_url| {
            base_url.trim().trim_end_matches('/')
                != provider_before.base_url.trim().trim_end_matches('/')
        })
        || input
            .api_key
            .as_ref()
            .is_some_and(|api_key| api_key.trim() != provider_before.api_key.trim());
    if was_direct_active && provider_config_changed {
        let base_url = input
            .base_url
            .as_deref()
            .unwrap_or(&provider_before.base_url);
        let api_key = input.api_key.as_deref().unwrap_or(&provider_before.api_key);
        if base_url.trim().is_empty() || api_key.trim().is_empty() {
            return Err("当前直连服务商的 Base URL 或 API Key 不能为空".to_string());
        }
    }

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
    if input_price_per_m.is_some() {
        updates.push("input_price_per_m = ?".to_string());
        has_updates = true;
    }
    if output_price_per_m.is_some() {
        updates.push("output_price_per_m = ?".to_string());
        has_updates = true;
    }
    if cache_read_price_per_m.is_some() {
        updates.push("cache_read_price_per_m = ?".to_string());
        has_updates = true;
    }
    if cache_creation_price_per_m.is_some() {
        updates.push("cache_creation_price_per_m = ?".to_string());
        has_updates = true;
    }

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
        if let Some(value) = input_price_per_m {
            q = q.bind(value);
        }
        if let Some(value) = output_price_per_m {
            q = q.bind(value);
        }
        if let Some(value) = cache_read_price_per_m {
            q = q.bind(value);
        }
        if let Some(value) = cache_creation_price_per_m {
            q = q.bind(value);
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

    if was_direct_active && provider_config_changed {
        let provider_after: Provider = sqlx::query_as("SELECT * FROM providers WHERE id = ?")
            .bind(id)
            .fetch_one(db.inner())
            .await
            .map_err(|e| e.to_string())?;
        if profile_changed {
            crate::services::agent_config::remove_provider_direct_config_for_provider(
                db.inner(),
                &provider_before,
            )
            .await?;
        }
        crate::services::agent_config::write_provider_direct_config(db.inner(), &provider_after)
            .await?;
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
    let was_default_provider_direct = if let Some((cli_type, profile)) = &scope {
        profile == DEFAULT_PROFILE
            && crate::services::agent_config::provider_direct_active_provider_id(
                db,
                cli_type,
                DEFAULT_PROFILE,
            )
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
        "UPDATE providers SET sort_order = CASE id {} END WHERE id IN ({})",
        case_clauses.join(" "),
        id_list.join(", ")
    );

    sqlx::query(&sql).execute(db).await.map_err(map_db_error)?;

    if was_default_provider_direct {
        let (cli_type, profile) = scope.expect("provider scope should exist");
        let provider: Provider = sqlx::query_as(
            "SELECT * FROM providers WHERE cli_type = ? AND profile = ? ORDER BY sort_order, id LIMIT 1",
        )
        .bind(&cli_type)
        .bind(&profile)
        .fetch_one(db)
        .await
        .map_err(|e| e.to_string())?;
        crate::services::agent_config::write_provider_direct_config(db, &provider).await?;
        remember_default_provider_direct_provider(db, &provider, now_timestamp()).await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema_definition::DatabaseSchema;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::path::Path;

    async fn test_pool(config_dir: &Path) -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("test database");
        for sql in DatabaseSchema::current().to_create_all_sql() {
            sqlx::query(&sql).execute(&pool).await.expect("test schema");
        }
        sqlx::query(
            "INSERT INTO cli_settings (cli_type, default_json_config, config_dir, updated_at) VALUES (?, '', ?, 0)",
        )
        .bind("claude_code")
        .bind(config_dir.to_string_lossy().as_ref())
        .execute(&pool)
        .await
        .expect("test settings");
        pool
    }

    async fn insert_provider(db: &SqlitePool, name: &str, sort_order: i64) -> i64 {
        sqlx::query(
            "INSERT INTO providers (cli_type, profile, protocol, name, base_url, api_key, sort_order, created_at, updated_at) VALUES ('claude_code', 'default', 'anthropic_messages', ?, ?, ?, ?, 0, 0)",
        )
        .bind(name)
        .bind(format!("https://{}.example.com", name))
        .bind(format!("{}-key", name))
        .bind(sort_order)
        .execute(db)
        .await
        .expect("test provider")
        .last_insert_rowid()
    }

    #[tokio::test]
    async fn reorder_rewrites_direct_config_with_new_first_provider() {
        let config_dir = std::env::temp_dir().join(format!(
            "ccg-gateway-provider-reorder-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&config_dir).expect("test config directory");
        let db = test_pool(&config_dir).await;
        let first_id = insert_provider(&db, "first", 0).await;
        let second_id = insert_provider(&db, "second", 1).await;
        let first: Provider = sqlx::query_as("SELECT * FROM providers WHERE id = ?")
            .bind(first_id)
            .fetch_one(&db)
            .await
            .expect("first provider");
        crate::services::agent_config::write_provider_direct_config(&db, &first)
            .await
            .expect("initial direct config");

        reorder_providers_impl(&db, vec![second_id, first_id])
            .await
            .expect("provider reorder");

        let active_id = crate::services::agent_config::provider_direct_active_provider_id(
            &db,
            "claude_code",
            DEFAULT_PROFILE,
        )
        .await
        .expect("active provider");
        assert_eq!(active_id, Some(second_id));
        let remembered_id: Option<i64> = sqlx::query_as::<_, (Option<i64>,)>(
            "SELECT last_provider_direct_provider_id FROM cli_settings WHERE cli_type = 'claude_code'",
        )
        .fetch_one(&db)
        .await
        .expect("remembered provider")
        .0;
        assert_eq!(remembered_id, Some(second_id));

        db.close().await;
        std::fs::remove_dir_all(config_dir).expect("remove test config directory");
    }
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
