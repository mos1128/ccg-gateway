use sqlx::{FromRow, SqlitePool};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Default, FromRow)]
pub struct TokenPricing {
    pub input_price_per_m: f64,
    pub output_price_per_m: f64,
    pub cache_read_price_per_m: f64,
    pub cache_creation_price_per_m: f64,
}

#[derive(Debug, FromRow)]
struct ProviderPricingRow {
    cli_type: String,
    name: String,
    input_price_per_m: f64,
    output_price_per_m: f64,
    cache_read_price_per_m: f64,
    cache_creation_price_per_m: f64,
}

impl From<ProviderPricingRow> for TokenPricing {
    fn from(row: ProviderPricingRow) -> Self {
        Self {
            input_price_per_m: row.input_price_per_m,
            output_price_per_m: row.output_price_per_m,
            cache_read_price_per_m: row.cache_read_price_per_m,
            cache_creation_price_per_m: row.cache_creation_price_per_m,
        }
    }
}

pub fn calculate_token_cost(
    pricing: TokenPricing,
    input_tokens: i64,
    cache_read_input_tokens: i64,
    cache_creation_input_tokens: i64,
    output_tokens: i64,
) -> f64 {
    let cost = input_tokens.max(0) as f64 * pricing.input_price_per_m
        + cache_read_input_tokens.max(0) as f64 * pricing.cache_read_price_per_m
        + cache_creation_input_tokens.max(0) as f64 * pricing.cache_creation_price_per_m
        + output_tokens.max(0) as f64 * pricing.output_price_per_m;
    cost / 1_000_000.0
}

pub async fn provider_pricing_map(
    db: &SqlitePool,
) -> Result<HashMap<(String, String), TokenPricing>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ProviderPricingRow>(
        r#"
        SELECT cli_type, name, input_price_per_m, output_price_per_m, cache_read_price_per_m, cache_creation_price_per_m
        FROM providers
        ORDER BY id
        "#,
    )
    .fetch_all(db)
    .await?;

    let mut map = HashMap::new();
    for row in rows {
        let key = (row.cli_type.clone(), row.name.clone());
        map.insert(key, row.into());
    }
    Ok(map)
}

pub async fn provider_pricing(
    db: &SqlitePool,
    cli_type: &str,
    provider_name: &str,
) -> Result<TokenPricing, sqlx::Error> {
    let pricing = sqlx::query_as::<_, TokenPricing>(
        r#"
        SELECT input_price_per_m, output_price_per_m, cache_read_price_per_m, cache_creation_price_per_m
        FROM providers
        WHERE cli_type = ? AND name = ?
        ORDER BY id DESC
        LIMIT 1
        "#,
    )
    .bind(cli_type)
    .bind(provider_name)
    .fetch_optional(db)
    .await?;

    Ok(pricing.unwrap_or_default())
}
