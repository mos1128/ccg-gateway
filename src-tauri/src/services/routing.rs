use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::db::models::{Protocol, Provider, ProviderModelBlacklist, ProviderModelMap};
use crate::services::proxy::wildcard_match;
use crate::services::translate;
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

/// 服务商的端点类型能否服务这次请求：协议相同，或两边都在可转换集合里。
fn provider_matches(protocol: Protocol, provider_protocol: &str) -> bool {
    match provider_protocol.parse::<Protocol>() {
        Ok(upstream) => upstream == protocol || translate::can_translate(protocol, upstream),
        Err(_) => false,
    }
}

/// 透传用的服务商：不匹配任何端点类型的路径要原样转给上游，优先挑端点类型和路径归属
/// 一致的那一家（`/v1/messages/count_tokens` 只有 Anthropic 上游认），归不出归属或没有
/// 同类的，就用顺序第一家，由上游自己回答认不认。透传不做协议转换，所以不能复用
/// `provider_matches` 的可转换判定。
pub async fn get_passthrough_provider(
    db: &SqlitePool,
    cli_type: &str,
    profile: &str,
    preferred: Option<Protocol>,
) -> Result<Option<(Provider, Protocol)>, sqlx::Error> {
    let now = now_timestamp();
    let profile = normalize_profile(Some(profile)).unwrap_or_else(|| DEFAULT_PROFILE.to_string());

    let candidates: Vec<(Provider, Protocol)> = sqlx::query_as::<_, Provider>(
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
    .bind(&profile)
    .bind(now)
    .fetch_all(db)
    .await?
    .into_iter()
    .filter_map(|provider| {
        let protocol = provider.protocol.parse::<Protocol>().ok()?;
        Some((provider, protocol))
    })
    .collect();

    let index = preferred
        .and_then(|preferred| {
            candidates
                .iter()
                .position(|(_, protocol)| *protocol == preferred)
        })
        .unwrap_or(0);
    Ok(candidates.into_iter().nth(index))
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

/// Get all providers eligible for a request, in failover order.
///
/// 端点类型与请求协议不同的服务商也参与竞争，转发时由 translate 模块转换协议；
/// Gemini 不参与转换，只能协议完全一致才用。
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
          AND enabled = 1
          AND (blacklisted_until IS NULL OR blacklisted_until <= ?)
        ORDER BY sort_order, id
        "#,
    )
    .bind(cli_type)
    .bind(&profile)
    .bind(now)
    .fetch_all(db)
    .await?;

    // 顺序严格按用户配置的 sort_order，不因协议是否需要转换而调整。
    let providers: Vec<Provider> = providers
        .into_iter()
        .filter(|provider| provider_matches(protocol, &provider.protocol))
        .collect();

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
