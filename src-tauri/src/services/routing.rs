use sqlx::SqlitePool;

use crate::db::models::{Provider, ProviderModelMap, ProviderModelBlacklist};
use crate::services::proxy::wildcard_match;

/// Provider with its model mappings and blacklist
#[derive(Debug, Clone)]
pub struct ProviderWithMaps {
    pub provider: Provider,
    pub model_maps: Vec<ProviderModelMap>,
    pub model_blacklist: Vec<ProviderModelBlacklist>,
}

/// Check if model matches any blacklist pattern
fn is_model_blacklisted(model: &str, blacklist: &[ProviderModelBlacklist]) -> bool {
    blacklist.iter().any(|item| wildcard_match(&item.model_pattern, model))
}

/// Select an available provider for the given CLI type
/// Returns None if all providers are blacklisted or none are configured
pub async fn select_provider(
    db: &SqlitePool,
    cli_type: &str,
    model: Option<&str>,
) -> Result<Option<ProviderWithMaps>, sqlx::Error> {
    let now = chrono::Utc::now().timestamp();

    // Query enabled providers ordered by sort_order, excluding blacklisted ones
    let providers = sqlx::query_as::<_, Provider>(
        r#"
        SELECT * FROM providers
        WHERE cli_type = ?
          AND enabled = 1
          AND (blacklisted_until IS NULL OR blacklisted_until <= ?)
        ORDER BY sort_order, id
        "#,
    )
    .bind(cli_type)
    .bind(now)
    .fetch_all(db)
    .await?;

    // Return the first available provider that doesn't blacklist the model
    for provider in providers {
        let model_maps = sqlx::query_as::<_, ProviderModelMap>(
            "SELECT * FROM provider_model_map WHERE provider_id = ? AND enabled = 1 ORDER BY id",
        )
        .bind(provider.id)
        .fetch_all(db)
        .await?;

        let model_blacklist = sqlx::query_as::<_, ProviderModelBlacklist>(
            "SELECT * FROM provider_model_blacklist WHERE provider_id = ? ORDER BY id",
        )
        .bind(provider.id)
        .fetch_all(db)
        .await?;

        // Check if model is blacklisted
        if let Some(m) = model {
            if is_model_blacklisted(m, &model_blacklist) {
                continue;
            }
        }

        return Ok(Some(ProviderWithMaps { provider, model_maps, model_blacklist }));
    }

    Ok(None)
}

/// Get all available providers for a CLI type (for fallback scenarios)
pub async fn get_available_providers(
    db: &SqlitePool,
    cli_type: &str,
    model: Option<&str>,
) -> Result<Vec<ProviderWithMaps>, sqlx::Error> {
    let now = chrono::Utc::now().timestamp();

    let providers = sqlx::query_as::<_, Provider>(
        r#"
        SELECT * FROM providers
        WHERE cli_type = ?
          AND enabled = 1
          AND (blacklisted_until IS NULL OR blacklisted_until <= ?)
        ORDER BY sort_order, id
        "#,
    )
    .bind(cli_type)
    .bind(now)
    .fetch_all(db)
    .await?;

    let mut result = Vec::new();
    for provider in providers {
        let model_maps = sqlx::query_as::<_, ProviderModelMap>(
            "SELECT * FROM provider_model_map WHERE provider_id = ? AND enabled = 1 ORDER BY id",
        )
        .bind(provider.id)
        .fetch_all(db)
        .await?;

        let model_blacklist = sqlx::query_as::<_, ProviderModelBlacklist>(
            "SELECT * FROM provider_model_blacklist WHERE provider_id = ? ORDER BY id",
        )
        .bind(provider.id)
        .fetch_all(db)
        .await?;

        // Check if model is blacklisted
        if let Some(m) = model {
            if is_model_blacklisted(m, &model_blacklist) {
                continue;
            }
        }

        result.push(ProviderWithMaps { provider, model_maps, model_blacklist });
    }

    Ok(result)
}
