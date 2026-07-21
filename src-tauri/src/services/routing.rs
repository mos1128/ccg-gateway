use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::db::models::{Protocol, Provider, ProviderModelBlacklist, ProviderModelMap};
use crate::services::proxy::wildcard_match;
use crate::time::now_timestamp;

pub const DEFAULT_PROFILE: &str = "default";

pub fn normalize_profile(profile: Option<&str>) -> Option<String> {
    let profile = profile.unwrap_or(DEFAULT_PROFILE).trim();
    if profile.is_empty() || profile.eq_ignore_ascii_case(DEFAULT_PROFILE) {
        return Some(DEFAULT_PROFILE.to_string());
    }

    normalize_profile_name(profile)
}

pub fn normalize_profile_name(profile: &str) -> Option<String> {
    let profile = profile.trim();
    if profile.is_empty() {
        return None;
    }

    let profile = profile
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
        .to_ascii_lowercase();
    is_valid_profile_name(&profile).then_some(profile)
}

pub fn is_valid_profile_name(profile: &str) -> bool {
    let profile = profile.trim();
    !profile.is_empty()
        && profile.len() <= 64
        && profile != "."
        && profile != ".."
        && profile
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

pub fn gateway_token_for_profile(profile: &str) -> Option<String> {
    let profile = normalize_profile(Some(profile))?;
    if profile == DEFAULT_PROFILE {
        Some("ccg-gateway".to_string())
    } else {
        Some(format!("ccg-gateway-{}", profile))
    }
}

pub fn profile_from_gateway_token(token: &str) -> Option<String> {
    let token = token.trim();
    if token == "ccg-gateway" {
        return Some(DEFAULT_PROFILE.to_string());
    }

    let profile = token.strip_prefix("ccg-gateway-")?;
    normalize_profile(Some(profile))
}

/// Provider with its model mappings and blacklist
#[derive(Debug, Clone)]
pub struct ProviderWithMaps {
    pub provider: Provider,
    pub model_maps: Vec<ProviderModelMap>,
    pub model_blacklist: Vec<ProviderModelBlacklist>,
}

/// Check if model matches any blacklist pattern
fn is_model_blacklisted(model: &str, blacklist: &[ProviderModelBlacklist]) -> bool {
    blacklist
        .iter()
        .any(|item| wildcard_match(&item.model_pattern, model))
}

async fn load_provider_maps(
    db: &SqlitePool,
    provider_ids: &[i64],
) -> Result<
    (
        HashMap<i64, Vec<ProviderModelMap>>,
        HashMap<i64, Vec<ProviderModelBlacklist>>,
    ),
    sqlx::Error,
> {
    if provider_ids.is_empty() {
        return Ok((HashMap::new(), HashMap::new()));
    }

    let placeholders = vec!["?"; provider_ids.len()].join(", ");

    let map_sql = format!(
        "SELECT * FROM provider_model_map WHERE enabled = 1 AND provider_id IN ({}) ORDER BY provider_id, id",
        placeholders
    );
    let mut map_query = sqlx::query_as::<_, ProviderModelMap>(&map_sql);
    for id in provider_ids {
        map_query = map_query.bind(*id);
    }
    let model_maps = map_query.fetch_all(db).await?;

    let blacklist_sql = format!(
        "SELECT * FROM provider_model_blacklist WHERE provider_id IN ({}) ORDER BY provider_id, id",
        placeholders
    );
    let mut blacklist_query = sqlx::query_as::<_, ProviderModelBlacklist>(&blacklist_sql);
    for id in provider_ids {
        blacklist_query = blacklist_query.bind(*id);
    }
    let model_blacklist = blacklist_query.fetch_all(db).await?;

    let mut maps_by_provider: HashMap<i64, Vec<ProviderModelMap>> = HashMap::new();
    for item in model_maps {
        maps_by_provider
            .entry(item.provider_id)
            .or_default()
            .push(item);
    }

    let mut blacklist_by_provider: HashMap<i64, Vec<ProviderModelBlacklist>> = HashMap::new();
    for item in model_blacklist {
        blacklist_by_provider
            .entry(item.provider_id)
            .or_default()
            .push(item);
    }

    Ok((maps_by_provider, blacklist_by_provider))
}

/// Select an available provider for the given CLI type
/// Returns None if all providers are blacklisted or none are configured
pub async fn select_provider(
    db: &SqlitePool,
    cli_type: &str,
    profile: &str,
    protocol: Protocol,
    model: Option<&str>,
) -> Result<Option<ProviderWithMaps>, sqlx::Error> {
    let now = now_timestamp();
    let profile = normalize_profile(Some(profile)).unwrap_or_else(|| DEFAULT_PROFILE.to_string());

    // Query enabled providers ordered by sort_order, excluding blacklisted ones
    let providers = sqlx::query_as::<_, Provider>(
        r#"
        SELECT * FROM providers
        WHERE cli_type = ?
          AND profile = ?
          AND protocol = ?
          AND enabled = 1
          AND (blacklisted_until IS NULL OR blacklisted_until <= ?)
        ORDER BY sort_order, id
        "#,
    )
    .bind(cli_type)
    .bind(&profile)
    .bind(protocol.as_str())
    .bind(now)
    .fetch_all(db)
    .await?;

    let provider_ids: Vec<i64> = providers.iter().map(|provider| provider.id).collect();
    let (mut maps_by_provider, mut blacklist_by_provider) =
        load_provider_maps(db, &provider_ids).await?;

    // Return the first available provider that doesn't blacklist the model
    for provider in providers {
        let model_maps = maps_by_provider.remove(&provider.id).unwrap_or_default();
        let model_blacklist = blacklist_by_provider
            .remove(&provider.id)
            .unwrap_or_default();

        // Check if model is blacklisted
        if let Some(m) = model {
            if is_model_blacklisted(m, &model_blacklist) {
                continue;
            }
        }

        return Ok(Some(ProviderWithMaps {
            provider,
            model_maps,
            model_blacklist,
        }));
    }

    Ok(None)
}

/// Get all available providers for a CLI type (for fallback scenarios)
pub async fn get_available_providers(
    db: &SqlitePool,
    cli_type: &str,
    profile: &str,
    protocol: Protocol,
    model: Option<&str>,
) -> Result<Vec<ProviderWithMaps>, sqlx::Error> {
    let now = now_timestamp();
    let profile = normalize_profile(Some(profile)).unwrap_or_else(|| DEFAULT_PROFILE.to_string());

    let providers = sqlx::query_as::<_, Provider>(
        r#"
        SELECT * FROM providers
        WHERE cli_type = ?
          AND profile = ?
          AND protocol = ?
          AND enabled = 1
          AND (blacklisted_until IS NULL OR blacklisted_until <= ?)
        ORDER BY sort_order, id
        "#,
    )
    .bind(cli_type)
    .bind(&profile)
    .bind(protocol.as_str())
    .bind(now)
    .fetch_all(db)
    .await?;

    let provider_ids: Vec<i64> = providers.iter().map(|provider| provider.id).collect();
    let (mut maps_by_provider, mut blacklist_by_provider) =
        load_provider_maps(db, &provider_ids).await?;

    let mut result = Vec::new();
    for provider in providers {
        let model_maps = maps_by_provider.remove(&provider.id).unwrap_or_default();
        let model_blacklist = blacklist_by_provider
            .remove(&provider.id)
            .unwrap_or_default();

        // Check if model is blacklisted
        if let Some(m) = model {
            if is_model_blacklisted(m, &model_blacklist) {
                continue;
            }
        }

        result.push(ProviderWithMaps {
            provider,
            model_maps,
            model_blacklist,
        });
    }

    Ok(result)
}
