use crate::db::models::PriceSyncState;
use crate::time::now_timestamp;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::Mutex;

const PRICE_SOURCE_URL: &str = "https://models.dev/api.json";
const PRICE_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const PRICE_SYNC_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const PRICE_SYNC_RETRY_MIN: Duration = Duration::from_secs(60);
const PRICE_SYNC_RETRY_MAX: Duration = Duration::from_secs(60 * 60);
const MAX_ERROR_BODY: usize = 2000;

/// models.dev is keyed by channel, so a single model is published once per
/// reseller, each with its own price. The catalog only keeps first-party vendor
/// channels: it is the official reference price, and a reseller's real cost is
/// expressed through that provider's multiplier. Subscription plan channels
/// (`*-coding-plan`, `*-token-plan`) are excluded because they price by plan,
/// not by token. Order matters: vendor clouds that also resell other vendors'
/// models (Alibaba Bailian, Volcengine) come last so a model is priced by its
/// own vendor, and a global endpoint precedes its China counterpart.
const OFFICIAL_PROVIDERS: &[&str] = &[
    "openai",
    "anthropic",
    "google",
    "xai",
    "deepseek",
    "moonshotai",
    "moonshotai-cn",
    "zai",
    "zhipuai",
    "minimax",
    "minimax-cn",
    "mistral",
    "cohere",
    "meta",
    "llama",
    "perplexity",
    "upstage",
    "inception",
    "stepfun-ai",
    "stepfun",
    "sensenova",
    "longcat",
    "bailing",
    "xiaomi",
    "volcengine",
    "alibaba",
    "alibaba-cn",
];

static PRICE_SYNC_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// 启动时刷新一次，之后每天一次。失败时按指数退避重试（1 分钟起、最长 1 小时），
/// 避免开机瞬间没网就要等满 24 小时才有价目表。
pub fn start_background_sync(db: SqlitePool) {
    tokio::spawn(async move {
        let mut retry_delay = PRICE_SYNC_RETRY_MIN;
        loop {
            match sync_now(&db).await {
                Ok(_) => {
                    retry_delay = PRICE_SYNC_RETRY_MIN;
                    tokio::time::sleep(PRICE_SYNC_INTERVAL).await;
                }
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        retry_in_secs = retry_delay.as_secs(),
                        "Scheduled model price catalog sync failed"
                    );
                    tokio::time::sleep(retry_delay).await;
                    retry_delay = (retry_delay * 2).min(PRICE_SYNC_RETRY_MAX);
                }
            }
        }
    });
}

/// 分层价：models.dev 的 `cost.tiers`，上下文超过阈值后整组价格替换为该档。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PriceTier {
    pub threshold_tokens: i64,
    pub input_price_per_m: f64,
    pub output_price_per_m: f64,
    pub cache_read_price_per_m: f64,
    pub cache_creation_price_per_m: f64,
}

/// 目录查价结果：选中的价格加上它的来源渠道（models.dev 的厂商 id），
/// 来源会随单价一起固化进日志，供费用明细展示"按哪家厂商的价目计费"。
#[derive(Debug, Clone)]
pub struct ResolvedPrice {
    pub price: CatalogPrice,
    pub source_provider: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatalogPrice {
    pub input_price_per_m: f64,
    pub output_price_per_m: f64,
    pub cache_read_price_per_m: f64,
    pub cache_creation_price_per_m: f64,
    /// 阈值降序，取第一个命中的档位。
    pub tiers: Vec<PriceTier>,
}

#[derive(Debug, Deserialize, Default)]
struct SourceVendor {
    #[serde(default)]
    models: HashMap<String, SourceModel>,
}

#[derive(Debug, Deserialize, Default)]
struct SourceModel {
    id: Option<String>,
    family: Option<String>,
    modalities: Option<SourceModalities>,
    cost: Option<SourceCost>,
}

#[derive(Debug, Deserialize, Default)]
struct SourceModalities {
    #[serde(default)]
    output: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
struct SourceCost {
    input: Option<f64>,
    output: Option<f64>,
    cache_read: Option<f64>,
    cache_write: Option<f64>,
    /// 长上下文分层价。`context_over_200k` 是它的旧写法，凡是有 `context_over_200k`
    /// 的模型都同时给了 `tiers`，所以只解析 `tiers`。
    #[serde(default)]
    tiers: Vec<SourceTier>,
}

#[derive(Debug, Deserialize)]
struct SourceTier {
    tier: Option<SourceTierBound>,
    input: Option<f64>,
    output: Option<f64>,
    cache_read: Option<f64>,
    cache_write: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct SourceTierBound {
    #[serde(rename = "type")]
    kind: Option<String>,
    size: Option<i64>,
}

/// 价格查询结果行，`tiers` 是入库时序列化的 JSON。
#[derive(Debug, FromRow)]
struct PriceRow {
    model_key: String,
    source_provider: String,
    input_price_per_m: f64,
    output_price_per_m: f64,
    cache_read_price_per_m: f64,
    cache_creation_price_per_m: f64,
    tiers: Option<String>,
}

impl PriceRow {
    fn into_price(&self) -> CatalogPrice {
        let tiers = self
            .tiers
            .as_deref()
            .and_then(|raw| serde_json::from_str::<Vec<PriceTier>>(raw).ok())
            .unwrap_or_default();
        CatalogPrice {
            input_price_per_m: self.input_price_per_m,
            output_price_per_m: self.output_price_per_m,
            cache_read_price_per_m: self.cache_read_price_per_m,
            cache_creation_price_per_m: self.cache_creation_price_per_m,
            tiers,
        }
    }
}

#[derive(Debug, Clone)]
struct ParsedPrice {
    model_key: String,
    model_name: String,
    source_provider: String,
    price: CatalogPrice,
}

/// A failed refresh keeps the previous snapshot: the catalog table is only
/// rewritten once the new payload parses into at least one billable model.
pub async fn sync_now(db: &SqlitePool) -> Result<PriceSyncState, String> {
    let lock = PRICE_SYNC_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock.lock().await;
    let attempt_at = now_timestamp();
    ensure_state_row(db).await?;
    sqlx::query(
        "UPDATE price_sync_state SET last_attempt_at = ?, last_error = NULL, updated_at = ? WHERE id = 1",
    )
    .bind(attempt_at)
    .bind(attempt_at)
    .execute(db)
    .await
    .map_err(|error| error.to_string())?;

    let client = reqwest::Client::builder()
        .timeout(PRICE_REQUEST_TIMEOUT)
        .build()
        .map_err(|error| error.to_string())?;
    let response = match client
        .get(PRICE_SOURCE_URL)
        .header(reqwest::header::USER_AGENT, "ccg-gateway/model-price-sync")
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => return mark_failed(db, format!("请求价格源失败: {}", error)).await,
    };

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let detail = truncate_error(&body);
        let message = if detail.is_empty() {
            format!("价格源返回 HTTP {}", status)
        } else {
            format!("价格源返回 HTTP {}: {}", status, detail)
        };
        return mark_failed(db, message).await;
    }

    let body = match response.text().await {
        Ok(body) => body,
        Err(error) => return mark_failed(db, format!("读取价格源失败: {}", error)).await,
    };
    let raw: HashMap<String, SourceVendor> = match serde_json::from_str(&body) {
        Ok(raw) => raw,
        Err(error) => return mark_failed(db, format!("解析价格源失败: {}", error)).await,
    };
    let mut entries = parse_catalog(raw);
    if entries.is_empty() {
        return mark_failed(db, "价格源未返回可计费文本模型".to_string()).await;
    }
    entries.sort_by(|left, right| left.model_key.cmp(&right.model_key));

    let fetched_at = now_timestamp();
    let mut tx = db.begin().await.map_err(|error| error.to_string())?;
    sqlx::query("DELETE FROM model_price_catalog")
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
    for entry in &entries {
        let tiers = if entry.price.tiers.is_empty() {
            None
        } else {
            serde_json::to_string(&entry.price.tiers).ok()
        };
        sqlx::query(
            "INSERT INTO model_price_catalog (model_key, model_name, source_provider, input_price_per_m, output_price_per_m, cache_read_price_per_m, cache_creation_price_per_m, tiers, fetched_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&entry.model_key)
        .bind(&entry.model_name)
        .bind(&entry.source_provider)
        .bind(entry.price.input_price_per_m)
        .bind(entry.price.output_price_per_m)
        .bind(entry.price.cache_read_price_per_m)
        .bind(entry.price.cache_creation_price_per_m)
        .bind(tiers)
        .bind(fetched_at)
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
    }
    sqlx::query(
        "UPDATE price_sync_state SET last_success_at = ?, last_error = NULL, model_count = ?, updated_at = ? WHERE id = 1",
    )
    .bind(fetched_at)
    .bind(entries.len() as i64)
    .bind(fetched_at)
    .execute(&mut *tx)
    .await
    .map_err(|error| error.to_string())?;
    tx.commit().await.map_err(|error| error.to_string())?;

    tracing::info!(model_count = entries.len(), "Model price catalog updated");
    get_status(db).await
}

pub async fn get_status(db: &SqlitePool) -> Result<PriceSyncState, String> {
    ensure_state_row(db).await?;
    sqlx::query_as::<_, PriceSyncState>("SELECT * FROM price_sync_state WHERE id = 1")
        .fetch_one(db)
        .await
        .map_err(|error| error.to_string())
}

pub async fn lookup(db: &SqlitePool, model_name: &str) -> Result<Option<ResolvedPrice>, String> {
    let normalized_value = model_name.trim().to_ascii_lowercase();
    let normalized = normalized_value
        .strip_prefix("models/")
        .unwrap_or(&normalized_value)
        .to_string();
    if normalized.is_empty() {
        return Ok(None);
    }
    let bare = normalized
        .rsplit('/')
        .next()
        .unwrap_or(&normalized)
        .to_string();
    // `_` 在 LIKE 里是"任意一个字符"，模型名里的下划线必须转义成字面量，
    // 否则 `gpt_4` 会误匹配 `gpt-4` 这类只差一个字符的 key。
    let suffix = format!("%/{}", escape_like(&normalized));

    // 请求的模型名可能带厂商前缀（`anthropic/claude-...`）也可能不带，所以裸名、
    // 全名、`%/全名` 三种形式一起查。目录只有几百行，单条 OR 查询足够快。
    // 精确命中也一并参与排序：带前缀的精确命中可能是转售条目，不一定优于原厂条目。
    let rows = sqlx::query_as::<_, PriceRow>(
        "SELECT model_key, source_provider, input_price_per_m, output_price_per_m, cache_read_price_per_m, cache_creation_price_per_m, tiers FROM model_price_catalog WHERE model_key = ? OR model_name IN (?, ?) OR model_key LIKE ? ESCAPE '\\'",
    )
    .bind(&normalized)
    .bind(&normalized)
    .bind(&bare)
    .bind(&suffix)
    .fetch_all(db)
    .await
    .map_err(|error| error.to_string())?;

    // 开源模型可能被多家厂商部署且价格不同（如 GLM 同时在智谱、阿里、字节上架），
    // 所以不能要求所有候选价格一致，而是按确定性规则选"模型原厂"的条目，
    // 服务商间的实际差价由倍率表达。
    let best = rows.iter().fold(None::<&PriceRow>, |best, row| match best {
        None => Some(row),
        Some(current) if alias_better(row, current) => Some(row),
        Some(current) => Some(current),
    });
    Ok(best.map(|row| ResolvedPrice {
        price: row.into_price(),
        source_provider: Some(row.source_provider.clone()),
    }))
}

/// 候选排序规则：① 有价的条目优先于全零条目；② 裸 model_key（原厂用自家渠道上架，
/// 不带前缀）优先于带厂商前缀的条目（云厂商转售别家模型时的写法）；③ 最后按
/// `OFFICIAL_PROVIDERS` 的渠道顺序。保证同一个模型名永远解析到同一个价格。
fn alias_better(candidate: &PriceRow, incumbent: &PriceRow) -> bool {
    match (row_is_zero(candidate), row_is_zero(incumbent)) {
        (false, true) => return true,
        (true, false) => return false,
        _ => {}
    }
    let candidate_bare = !candidate.model_key.contains('/');
    let incumbent_bare = !incumbent.model_key.contains('/');
    if candidate_bare != incumbent_bare {
        return candidate_bare;
    }
    provider_rank(&candidate.source_provider) < provider_rank(&incumbent.source_provider)
}

fn row_is_zero(row: &PriceRow) -> bool {
    row.input_price_per_m == 0.0
        && row.output_price_per_m == 0.0
        && row.cache_read_price_per_m == 0.0
        && row.cache_creation_price_per_m == 0.0
}

/// 转义 LIKE 模式里的通配符（`\`、`%`、`_`），配合 `ESCAPE '\'` 按字面量比较。
fn escape_like(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

fn parse_catalog(raw: HashMap<String, SourceVendor>) -> Vec<ParsedPrice> {
    let mut entries_by_key: HashMap<String, ParsedPrice> = HashMap::new();
    for (source_provider, vendor) in raw {
        if !OFFICIAL_PROVIDERS.contains(&source_provider.as_str()) {
            continue;
        }
        for (fallback_id, model) in vendor.models {
            let source_id = model
                .id
                .as_deref()
                .unwrap_or(&fallback_id)
                .trim()
                .to_ascii_lowercase();
            if source_id.is_empty() {
                continue;
            }
            let output_is_text = model
                .modalities
                .as_ref()
                .map(|modalities| {
                    modalities
                        .output
                        .iter()
                        .any(|value| value.eq_ignore_ascii_case("text"))
                })
                .unwrap_or(true);
            let family = model
                .family
                .as_deref()
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !output_is_text || source_id.contains("embed") || family.contains("embed") {
                continue;
            }
            let cost = model.cost.unwrap_or_default();
            let price = build_price(&cost);
            let model_name = source_id
                .rsplit('/')
                .next()
                .unwrap_or(&source_id)
                .to_string();
            let parsed = ParsedPrice {
                model_key: source_id,
                model_name,
                source_provider: source_provider.clone(),
                price,
            };
            entries_by_key
                .entry(parsed.model_key.clone())
                .and_modify(|existing| {
                    if should_replace(existing, &parsed) {
                        *existing = parsed.clone();
                    }
                })
                .or_insert(parsed);
        }
    }
    entries_by_key.into_values().collect()
}

/// 只收录 `tier.type == "context"` 的档位，按阈值降序排列，计费时取第一个命中的档。
/// 档位里没写的字段回落到基准价（个别模型的分层只给了 input/output）。
fn build_price(cost: &SourceCost) -> CatalogPrice {
    let mut tiers: Vec<PriceTier> = cost
        .tiers
        .iter()
        .filter_map(|tier| {
            let bound = tier.tier.as_ref()?;
            if !bound
                .kind
                .as_deref()
                .unwrap_or_default()
                .eq_ignore_ascii_case("context")
            {
                return None;
            }
            let threshold_tokens = bound.size.filter(|size| *size > 0)?;
            Some(PriceTier {
                threshold_tokens,
                input_price_per_m: non_negative(tier.input.or(cost.input)),
                output_price_per_m: non_negative(tier.output.or(cost.output)),
                cache_read_price_per_m: non_negative(tier.cache_read.or(cost.cache_read)),
                cache_creation_price_per_m: non_negative(tier.cache_write.or(cost.cache_write)),
            })
        })
        .collect();
    tiers.sort_by(|left, right| right.threshold_tokens.cmp(&left.threshold_tokens));
    CatalogPrice {
        input_price_per_m: non_negative(cost.input),
        output_price_per_m: non_negative(cost.output),
        cache_read_price_per_m: non_negative(cost.cache_read),
        cache_creation_price_per_m: non_negative(cost.cache_write),
        tiers,
    }
}

/// Vendor channels overlap: a vendor may publish a global and a China endpoint,
/// and a vendor cloud may resell another vendor's model. Prefer a priced entry,
/// then the earlier channel in `OFFICIAL_PROVIDERS`, so the stored price never
/// depends on map iteration order.
fn should_replace(existing: &ParsedPrice, candidate: &ParsedPrice) -> bool {
    match (
        is_zero_price(&existing.price),
        is_zero_price(&candidate.price),
    ) {
        (true, false) => true,
        (false, true) => false,
        _ => provider_rank(&candidate.source_provider) < provider_rank(&existing.source_provider),
    }
}

fn provider_rank(source_provider: &str) -> usize {
    OFFICIAL_PROVIDERS
        .iter()
        .position(|candidate| *candidate == source_provider)
        .unwrap_or(usize::MAX)
}

fn is_zero_price(price: &CatalogPrice) -> bool {
    price.input_price_per_m == 0.0
        && price.output_price_per_m == 0.0
        && price.cache_read_price_per_m == 0.0
        && price.cache_creation_price_per_m == 0.0
}

fn non_negative(value: Option<f64>) -> f64 {
    value
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(0.0)
}

async fn ensure_state_row(db: &SqlitePool) -> Result<(), String> {
    let now = now_timestamp();
    sqlx::query(
        "INSERT OR IGNORE INTO price_sync_state (id, last_attempt_at, last_success_at, last_error, model_count, updated_at) VALUES (1, NULL, NULL, NULL, 0, ?)",
    )
    .bind(now)
    .execute(db)
    .await
    .map(|_| ())
    .map_err(|error| error.to_string())
}

async fn mark_failed(db: &SqlitePool, message: String) -> Result<PriceSyncState, String> {
    let now = now_timestamp();
    let update_result =
        sqlx::query("UPDATE price_sync_state SET last_error = ?, updated_at = ? WHERE id = 1")
            .bind(truncate_error(&message))
            .bind(now)
            .execute(db)
            .await;
    if let Err(error) = update_result {
        return Err(format!("{}; 写入同步状态失败: {}", message, error));
    }
    tracing::warn!(error = %message, "Model price catalog sync failed");
    Err(message)
}

fn truncate_error(value: &str) -> String {
    let value = value.trim();
    if value.len() <= MAX_ERROR_BODY {
        return value.to_string();
    }
    value.chars().take(MAX_ERROR_BODY).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(key: &str, provider: &str, input: f64) -> PriceRow {
        PriceRow {
            model_key: key.to_string(),
            source_provider: provider.to_string(),
            input_price_per_m: input,
            output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0,
            cache_creation_price_per_m: 0.0,
            tiers: None,
        }
    }

    #[test]
    fn alias_prefers_original_vendor_bare_key() {
        // GLM 同时在智谱（原厂，裸 key）和阿里（转售，带前缀）上架且价格不同
        let original = row("glm-4.6", "zai", 0.6);
        let resold = row("zai/glm-4.6", "alibaba", 0.8);
        assert!(alias_better(&original, &resold));
        assert!(!alias_better(&resold, &original));
    }

    #[test]
    fn alias_prefers_priced_entry_over_zero() {
        let zero = row("glm-4.6", "zai", 0.0);
        let priced = row("zai/glm-4.6", "alibaba", 0.8);
        assert!(alias_better(&priced, &zero));
        assert!(!alias_better(&zero, &priced));
    }

    #[test]
    fn alias_same_shape_falls_back_to_provider_rank() {
        // 同为裸 key 时按 OFFICIAL_PROVIDERS 顺序，zai 排在 zhipuai 前面
        let first = row("glm-4.6", "zai", 0.6);
        let second = row("glm-4.6", "zhipuai", 0.7);
        assert!(alias_better(&first, &second));
        assert!(!alias_better(&second, &first));
    }
}
