use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::db::models::{Provider, ProviderModelBlacklist, ProviderModelMap};
use crate::services::proxy::wildcard_match;
use crate::time::now_timestamp;

pub const DEFAULT_PROFILE: &str = "default";
pub const PROFILE1: &str = "profile1";
pub const PROFILE2: &str = "profile2";
pub const PROFILE3: &str = "profile3";

pub const PROVIDER_PROFILES: [&str; 4] = [DEFAULT_PROFILE, PROFILE1, PROFILE2, PROFILE3];
pub const GATEWAY_PROFILE_PATH_ROOT: &str = "/__ccg";

pub fn normalize_profile(profile: Option<&str>) -> Option<&'static str> {
    let profile = profile.unwrap_or(DEFAULT_PROFILE).trim();
    match profile {
        "" | DEFAULT_PROFILE => Some(DEFAULT_PROFILE),
        PROFILE1 => Some(PROFILE1),
        PROFILE2 => Some(PROFILE2),
        PROFILE3 => Some(PROFILE3),
        _ => None,
    }
}

pub fn gateway_token_for_profile(profile: &str) -> Option<&'static str> {
    match normalize_profile(Some(profile))? {
        DEFAULT_PROFILE => Some("ccg-gateway"),
        PROFILE1 => Some("ccg-gateway-1"),
        PROFILE2 => Some("ccg-gateway-2"),
        PROFILE3 => Some("ccg-gateway-3"),
        _ => None,
    }
}

pub fn profile_from_gateway_token(token: &str) -> Option<&'static str> {
    match token.trim() {
        "ccg-gateway" => Some(DEFAULT_PROFILE),
        "ccg-gateway-1" => Some(PROFILE1),
        "ccg-gateway-2" => Some(PROFILE2),
        "ccg-gateway-3" => Some(PROFILE3),
        _ => None,
    }
}

pub fn gateway_path_prefix_for_profile(profile: &str) -> Option<&'static str> {
    match normalize_profile(Some(profile))? {
        DEFAULT_PROFILE => Some(""),
        PROFILE1 => Some("/__ccg/profile1"),
        PROFILE2 => Some("/__ccg/profile2"),
        PROFILE3 => Some("/__ccg/profile3"),
        _ => None,
    }
}

pub fn split_gateway_profile_path(full_path: &str) -> Option<(&'static str, String)> {
    let rest = full_path.strip_prefix("/__ccg/")?;
    let (profile_segment, suffix) = match rest.find(|c| c == '/' || c == '?') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, ""),
    };
    let profile = normalize_profile(Some(profile_segment))?;

    let stripped_path = if suffix.is_empty() {
        "/".to_string()
    } else if suffix.starts_with('/') {
        suffix.to_string()
    } else {
        format!("/{}", suffix)
    };

    Some((profile, stripped_path))
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
    model: Option<&str>,
) -> Result<Option<ProviderWithMaps>, sqlx::Error> {
    let now = now_timestamp();
    let profile = normalize_profile(Some(profile)).unwrap_or(DEFAULT_PROFILE);

    // Query enabled providers ordered by sort_order, excluding blacklisted ones
    let providers = sqlx::query_as::<_, Provider>(
        r#"
        SELECT * FROM providers
        WHERE cli_type = ?
          AND profile = ?
          AND enabled = 1
          AND (blacklisted_until IS NULL OR blacklisted_until <= ?)
        ORDER BY sort_order, id
        "#,
    )
    .bind(cli_type)
    .bind(profile)
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
    model: Option<&str>,
) -> Result<Vec<ProviderWithMaps>, sqlx::Error> {
    let now = now_timestamp();
    let profile = normalize_profile(Some(profile)).unwrap_or(DEFAULT_PROFILE);

    let providers = sqlx::query_as::<_, Provider>(
        r#"
        SELECT * FROM providers
        WHERE cli_type = ?
          AND profile = ?
          AND enabled = 1
          AND (blacklisted_until IS NULL OR blacklisted_until <= ?)
        ORDER BY sort_order, id
        "#,
    )
    .bind(cli_type)
    .bind(profile)
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
