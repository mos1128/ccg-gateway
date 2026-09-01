use crate::services::pricing::PriceTier;
use serde::Serialize;
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Default)]
pub struct TokenPricing {
    /// 目录原价，未乘倍率。
    pub input_price_per_m: f64,
    pub output_price_per_m: f64,
    pub cache_read_price_per_m: f64,
    pub cache_creation_price_per_m: f64,
    /// 阈值降序，上下文超过阈值时整组价格换成该档。
    pub tiers: Vec<PriceTier>,
    /// 目录里有没有这个模型。没有时四项价格都是 0，费用只能按 0 算。
    pub matched: bool,
    pub multiplier: f64,
    /// 价格来源（models.dev 的厂商渠道 id），未命中目录时为 None。
    pub source: Option<String>,
}

/// 落库的计费单价快照，字段名对应 request_logs 的 price_* 列。四项单价是选完分层档
/// 后的目录原价（未乘倍率），为 NULL 表示请求发生时目录里没有这个模型，费用按 0 计。
/// 中转站的倍率和目录价都会变，但已经发生的请求扣的钱不会变，所以单价在请求完成时
/// 就固化下来，之后读取只做乘法。
#[derive(Debug, Clone, Default, FromRow)]
pub struct PriceSnapshot {
    pub price_input_per_m: Option<f64>,
    pub price_output_per_m: Option<f64>,
    pub price_cache_read_per_m: Option<f64>,
    pub price_cache_creation_per_m: Option<f64>,
    /// 请求完成时的服务商倍率。为 NULL 表示这条记录还没有快照（进行中或旧数据）。
    pub price_multiplier: Option<f64>,
    pub price_tier_threshold: Option<i64>,
    /// 价格来源（models.dev 的厂商渠道 id），未命中目录时为 NULL。
    pub price_source: Option<String>,
}

impl PriceSnapshot {
    fn matched(&self) -> bool {
        self.price_input_per_m.is_some()
    }

    /// 倍率缺失或非正数时回落到 1，免得服务商被算成零成本。
    fn multiplier(&self) -> f64 {
        self.price_multiplier
            .filter(|multiplier| multiplier.is_finite() && *multiplier > 0.0)
            .unwrap_or(1.0)
    }
}

/// 实际用于计费的单价，供界面展示计算过程。价格都已乘过服务商倍率。
#[derive(Debug, Clone, Default, Serialize)]
pub struct CostBreakdown {
    pub matched: bool,
    pub multiplier: f64,
    pub input_price_per_m: f64,
    pub output_price_per_m: f64,
    pub cache_read_price_per_m: f64,
    pub cache_creation_price_per_m: f64,
    /// 命中的分层档阈值，走基准价时为 None。
    pub tier_threshold_tokens: Option<i64>,
    /// 价格来源（models.dev 的厂商渠道 id），未命中目录时为 None。
    pub source: Option<String>,
}

/// 选定分层档并固化目录价。`request_count` 是这批 token 覆盖的请求数：单条请求传 1，
/// 按天聚合的历史数据传当天的请求数。分层价按单次请求的上下文大小选档，聚合行只能
/// 用平均值近似，否则一天的 token 累加起来必然落到最高档。
pub fn resolve_price_snapshot(
    pricing: &TokenPricing,
    input_tokens: i64,
    cache_read_input_tokens: i64,
    cache_creation_input_tokens: i64,
    request_count: i64,
) -> PriceSnapshot {
    if !pricing.matched {
        return PriceSnapshot {
            price_multiplier: Some(pricing.multiplier),
            ..Default::default()
        };
    }
    // input_tokens 已扣掉缓存命中部分，模型实际看到的上下文要把缓存加回来。
    let context_tokens = (input_tokens.max(0)
        + cache_read_input_tokens.max(0)
        + cache_creation_input_tokens.max(0))
        / request_count.max(1);
    let tier = pricing
        .tiers
        .iter()
        .find(|tier| context_tokens > tier.threshold_tokens);
    PriceSnapshot {
        price_input_per_m: Some(
            tier.map_or(pricing.input_price_per_m, |tier| tier.input_price_per_m),
        ),
        price_output_per_m: Some(
            tier.map_or(pricing.output_price_per_m, |tier| tier.output_price_per_m),
        ),
        price_cache_read_per_m: Some(tier.map_or(pricing.cache_read_price_per_m, |tier| {
            tier.cache_read_price_per_m
        })),
        price_cache_creation_per_m: Some(tier.map_or(pricing.cache_creation_price_per_m, |tier| {
            tier.cache_creation_price_per_m
        })),
        price_multiplier: Some(pricing.multiplier),
        price_tier_threshold: tier.map(|tier| tier.threshold_tokens),
        price_source: pricing.source.clone(),
    }
}

/// 单价全部来自快照，只做乘法，不再查目录价，所以改倍率或目录价不会改动历史费用。
pub fn cost_from_snapshot(
    snapshot: &PriceSnapshot,
    input_tokens: i64,
    cache_read_input_tokens: i64,
    cache_creation_input_tokens: i64,
    output_tokens: i64,
) -> (f64, CostBreakdown) {
    let multiplier = snapshot.multiplier();
    let breakdown = CostBreakdown {
        matched: snapshot.matched(),
        multiplier,
        input_price_per_m: snapshot.price_input_per_m.unwrap_or(0.0) * multiplier,
        output_price_per_m: snapshot.price_output_per_m.unwrap_or(0.0) * multiplier,
        cache_read_price_per_m: snapshot.price_cache_read_per_m.unwrap_or(0.0) * multiplier,
        cache_creation_price_per_m: snapshot.price_cache_creation_per_m.unwrap_or(0.0) * multiplier,
        tier_threshold_tokens: snapshot.price_tier_threshold,
        source: snapshot.price_source.clone(),
    };
    let cost = input_tokens.max(0) as f64 * breakdown.input_price_per_m
        + cache_read_input_tokens.max(0) as f64 * breakdown.cache_read_price_per_m
        + cache_creation_input_tokens.max(0) as f64 * breakdown.cache_creation_price_per_m
        + output_tokens.max(0) as f64 * breakdown.output_price_per_m;
    (cost / 1_000_000.0, breakdown)
}

/// Resolve effective pricing for a request: the global catalog price of the
/// model that was actually used, plus the provider's multiplier. Model prices
/// are global; only the multiplier belongs to a channel.
pub async fn provider_pricing_for_model(
    db: &SqlitePool,
    provider_id: Option<i64>,
    cli_type: &str,
    provider_name: &str,
    model_name: Option<&str>,
) -> Result<TokenPricing, sqlx::Error> {
    // 倍率在没命中目录价时也要落进快照，它是"这条记录已经算过价"的标记。
    let multiplier = provider_multiplier(db, provider_id, cli_type, provider_name).await?;
    let unmatched = TokenPricing {
        multiplier,
        ..Default::default()
    };
    let Some(model_name) = model_name else {
        return Ok(unmatched);
    };
    let catalog = match crate::services::pricing::lookup(db, model_name).await {
        Ok(Some(resolved)) => resolved,
        Ok(None) => return Ok(unmatched),
        Err(error) => {
            tracing::warn!(error = %error, model = model_name, "Failed to read model price catalog");
            return Ok(unmatched);
        }
    };

    Ok(TokenPricing {
        input_price_per_m: catalog.price.input_price_per_m,
        output_price_per_m: catalog.price.output_price_per_m,
        cache_read_price_per_m: catalog.price.cache_read_price_per_m,
        cache_creation_price_per_m: catalog.price.cache_creation_price_per_m,
        tiers: catalog.price.tiers,
        matched: true,
        multiplier,
        source: catalog.source_provider,
    })
}

/// Falls back to 1 (the official catalog price) whenever the stored value is
/// missing or non-positive, so a channel can never silently report zero cost.
async fn provider_multiplier(
    db: &SqlitePool,
    provider_id: Option<i64>,
    cli_type: &str,
    provider_name: &str,
) -> Result<f64, sqlx::Error> {
    let row: Option<(f64,)> = if let Some(provider_id) = provider_id {
        sqlx::query_as("SELECT price_multiplier FROM providers WHERE id = ?")
            .bind(provider_id)
            .fetch_optional(db)
            .await?
    } else {
        sqlx::query_as(
            "SELECT price_multiplier FROM providers WHERE cli_type = ? AND name = ? ORDER BY id DESC LIMIT 1",
        )
        .bind(cli_type)
        .bind(provider_name)
        .fetch_optional(db)
        .await?
    };

    Ok(row
        .map(|(multiplier,)| multiplier)
        .filter(|multiplier| multiplier.is_finite() && *multiplier > 0.0)
        .unwrap_or(1.0))
}
