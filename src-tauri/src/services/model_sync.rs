use crate::db::models::{
    Protocol, Provider, ProviderModel, ProviderModelSyncState, ProviderModelsResponse,
};
use crate::services::proxy::join_upstream_url;
use crate::time::now_timestamp;
use serde_json::Value;
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::time::Duration;

const MODEL_REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_PAGES: usize = 50;
const MAX_ERROR_BODY: usize = 2000;

pub async fn sync_all(db: &SqlitePool) -> Result<(), String> {
    let providers = sqlx::query_as::<_, Provider>(
        "SELECT * FROM providers WHERE enabled = 1 ORDER BY sort_order, id",
    )
    .fetch_all(db)
    .await
    .map_err(|error| error.to_string())?;
    let client = reqwest::Client::builder()
        .timeout(MODEL_REQUEST_TIMEOUT)
        .build()
        .map_err(|error| error.to_string())?;
    let mut first_error = None;
    for provider in providers {
        if let Err(error) = sync_provider_with_client(db, &provider, &client).await {
            tracing::warn!(provider_id = provider.id, provider = %provider.name, error = %error, "Provider model sync failed");
            if first_error.is_none() {
                first_error = Some(format!("{}: {}", provider.name, error));
            }
        }
    }
    first_error.map_or(Ok(()), Err)
}

pub async fn sync_provider(
    db: &SqlitePool,
    provider_id: i64,
) -> Result<ProviderModelsResponse, String> {
    let provider = sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE id = ?")
        .bind(provider_id)
        .fetch_optional(db)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "服务商不存在".to_string())?;
    let client = reqwest::Client::builder()
        .timeout(MODEL_REQUEST_TIMEOUT)
        .build()
        .map_err(|error| error.to_string())?;
    sync_provider_with_client(db, &provider, &client).await
}

pub async fn list_provider_models(
    db: &SqlitePool,
    provider_id: i64,
) -> Result<ProviderModelsResponse, String> {
    let models = sqlx::query_as::<_, ProviderModel>(
        "SELECT * FROM provider_models WHERE provider_id = ? ORDER BY source, model_name, id",
    )
    .bind(provider_id)
    .fetch_all(db)
    .await
    .map_err(|error| error.to_string())?;
    let sync_state = sqlx::query_as::<_, ProviderModelSyncState>(
        "SELECT * FROM provider_model_sync_state WHERE provider_id = ?",
    )
    .bind(provider_id)
    .fetch_optional(db)
    .await
    .map_err(|error| error.to_string())?;
    Ok(ProviderModelsResponse {
        provider_id,
        models,
        sync_state,
    })
}

/// Invalidate the authoritative snapshot after endpoint credentials or protocol
/// settings change. The rows are kept for inspection, but routing must not use
/// them until a sync succeeds against the new configuration.
pub async fn invalidate_sync_state(db: &SqlitePool, provider_id: i64) -> Result<(), String> {
    ensure_state_row(db, provider_id).await?;
    let now = now_timestamp();
    sqlx::query(
        "UPDATE provider_model_sync_state
         SET last_success_at = NULL, last_error = NULL, model_count = 0, updated_at = ?
         WHERE provider_id = ?",
    )
    .bind(now)
    .bind(provider_id)
    .execute(db)
    .await
    .map(|_| ())
    .map_err(|error| error.to_string())
}

/// Manual models are never dropped by `sync_provider`, so a provider whose
/// model endpoint is unusable can still offer mapping candidates.
pub async fn add_manual_model(
    db: &SqlitePool,
    provider_id: i64,
    model_name: &str,
) -> Result<ProviderModelsResponse, String> {
    let model_name = model_name.trim();
    if model_name.is_empty() {
        return Err("模型名称不能为空".to_string());
    }
    sqlx::query_as::<_, (i64,)>("SELECT id FROM providers WHERE id = ?")
        .bind(provider_id)
        .fetch_optional(db)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "服务商不存在".to_string())?;

    let now = now_timestamp();
    // The unique index is case sensitive while sync dedupes case insensitively,
    // so pin the existing row instead of inserting a second casing of one name.
    let existing = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM provider_models WHERE provider_id = ? AND lower(model_name) = lower(?)",
    )
    .bind(provider_id)
    .bind(model_name)
    .fetch_optional(db)
    .await
    .map_err(|error| error.to_string())?;
    match existing {
        Some((id,)) => {
            sqlx::query(
                "UPDATE provider_models SET model_name = ?, source = 'manual', enabled = 1, last_seen_at = ? WHERE id = ?",
            )
            .bind(model_name)
            .bind(now)
            .bind(id)
        }
        None => sqlx::query(
            "INSERT INTO provider_models (provider_id, model_name, source, enabled, first_seen_at, last_seen_at) VALUES (?, ?, 'manual', 1, ?, ?)",
        )
        .bind(provider_id)
        .bind(model_name)
        .bind(now)
        .bind(now),
    }
    .execute(db)
    .await
    .map_err(|error| error.to_string())?;

    refresh_model_count(db, provider_id).await?;
    list_provider_models(db, provider_id).await
}

pub async fn delete_model(
    db: &SqlitePool,
    provider_id: i64,
    model_id: i64,
) -> Result<ProviderModelsResponse, String> {
    let deleted = sqlx::query("DELETE FROM provider_models WHERE id = ? AND provider_id = ?")
        .bind(model_id)
        .bind(provider_id)
        .execute(db)
        .await
        .map_err(|error| error.to_string())?;
    if deleted.rows_affected() == 0 {
        return Err("模型不存在".to_string());
    }
    refresh_model_count(db, provider_id).await?;
    list_provider_models(db, provider_id).await
}

async fn refresh_model_count(db: &SqlitePool, provider_id: i64) -> Result<(), String> {
    ensure_state_row(db, provider_id).await?;
    sqlx::query(
        "UPDATE provider_model_sync_state
         SET model_count = (SELECT COUNT(*) FROM provider_models WHERE provider_id = ? AND enabled = 1),
             updated_at = ?
         WHERE provider_id = ?",
    )
    .bind(provider_id)
    .bind(now_timestamp())
    .bind(provider_id)
    .execute(db)
    .await
    .map(|_| ())
    .map_err(|error| error.to_string())
}

async fn sync_provider_with_client(
    db: &SqlitePool,
    provider: &Provider,
    client: &reqwest::Client,
) -> Result<ProviderModelsResponse, String> {
    ensure_state_row(db, provider.id).await?;
    let attempt_at = now_timestamp();
    sqlx::query(
        "UPDATE provider_model_sync_state SET last_attempt_at = ?, last_error = NULL, updated_at = ? WHERE provider_id = ?",
    )
    .bind(attempt_at)
    .bind(attempt_at)
    .bind(provider.id)
    .execute(db)
    .await
    .map_err(|error| error.to_string())?;

    let protocol = match provider.protocol.parse::<Protocol>() {
        Ok(protocol) => protocol,
        Err(error) => {
            return mark_failed(db, provider, format!("协议无效: {}", error)).await;
        }
    };
    let fetched = match fetch_models(client, provider, protocol).await {
        Ok(models) if !models.is_empty() => models,
        Ok(_) => return mark_failed(db, provider, "上游返回空模型列表".to_string()).await,
        Err(error) => return mark_failed(db, provider, error).await,
    };

    let now = now_timestamp();
    let mut tx = db.begin().await.map_err(|error| error.to_string())?;
    let manual_names: Vec<(String,)> = sqlx::query_as(
        "SELECT model_name FROM provider_models WHERE provider_id = ? AND source = 'manual'",
    )
    .bind(provider.id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|error| error.to_string())?;
    let manual_names: HashSet<String> = manual_names
        .into_iter()
        .map(|(name,): (String,)| name.to_ascii_lowercase())
        .collect();

    sqlx::query("DELETE FROM provider_models WHERE provider_id = ? AND source = 'auto'")
        .bind(provider.id)
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;

    let mut seen = HashSet::new();
    for model_name in fetched {
        let model_name = model_name.trim().to_string();
        let key = model_name.to_ascii_lowercase();
        if model_name.is_empty() || manual_names.contains(&key) || !seen.insert(key) {
            continue;
        }
        sqlx::query(
            "INSERT INTO provider_models (provider_id, model_name, source, enabled, first_seen_at, last_seen_at) VALUES (?, ?, 'auto', 1, ?, ?)",
        )
        .bind(provider.id)
        .bind(model_name)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
    }

    let (model_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM provider_models WHERE provider_id = ? AND enabled = 1",
    )
    .bind(provider.id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| error.to_string())?;
    // Commit the snapshot only while the provider still has the configuration
    // used for this request. A settings edit can overlap a slow model fetch;
    // the conditional update makes the old transaction roll back instead of
    // publishing stale models.
    let state_update = sqlx::query(
        "UPDATE provider_model_sync_state
         SET last_success_at = ?, last_error = NULL, model_count = ?, updated_at = ?
         WHERE provider_id = ?
           AND EXISTS (
               SELECT 1 FROM providers
               WHERE id = ?
                 AND profile = ?
                 AND protocol = ?
                 AND rtrim(trim(base_url), '/') = rtrim(trim(?), '/')
                 AND trim(api_key) = trim(?)
                 AND coalesce(trim(custom_useragent), '') = coalesce(trim(?), '')
                 AND enabled = ?
           )",
    )
    .bind(now)
    .bind(model_count)
    .bind(now)
    .bind(provider.id)
    .bind(provider.id)
    .bind(&provider.profile)
    .bind(&provider.protocol)
    .bind(&provider.base_url)
    .bind(&provider.api_key)
    .bind(&provider.custom_useragent)
    .bind(provider.enabled)
    .execute(&mut *tx)
    .await
    .map_err(|error| error.to_string())?;
    if state_update.rows_affected() == 0 {
        return Err("服务商配置已变更，已丢弃过期模型列表".to_string());
    }
    tx.commit().await.map_err(|error| error.to_string())?;

    tracing::info!(
        provider_id = provider.id,
        model_count,
        "Provider model list updated"
    );
    list_provider_models(db, provider.id).await
}

async fn fetch_models(
    client: &reqwest::Client,
    provider: &Provider,
    protocol: Protocol,
) -> Result<Vec<String>, String> {
    match protocol {
        Protocol::AnthropicMessages => fetch_anthropic_models(client, provider).await,
        Protocol::GeminiGenerateContent => fetch_gemini_models(client, provider).await,
        Protocol::OpenaiChat | Protocol::OpenaiResponses => {
            fetch_openai_models(client, provider).await
        }
    }
}

async fn fetch_openai_models(
    client: &reqwest::Client,
    provider: &Provider,
) -> Result<Vec<String>, String> {
    let base = provider.base_url.trim_end_matches('/');
    let url = join_upstream_url(base, "/v1/models");
    let mut models = Vec::new();
    let mut after = None::<String>;
    for _ in 0..MAX_PAGES {
        let mut request = build_model_request(client, provider, Protocol::OpenaiChat, &url);
        if let Some(after) = after.as_deref() {
            request = request.query(&[("after", after)]);
        }
        let response = request
            .send()
            .await
            .map_err(|error| format!("请求模型列表失败: {}", error))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| format!("读取模型列表失败: {}", error))?;
        if !status.is_success() {
            return Err(format!("{}: {}", status, truncate_error(&body)));
        }

        let value: Value =
            serde_json::from_str(&body).map_err(|error| format!("解析模型列表失败: {}", error))?;
        models.extend(read_ids(&value, "data"));
        let has_more = value
            .get("has_more")
            .or_else(|| value.get("hasMore"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !has_more {
            return normalize_models(models);
        }

        let next = value
            .get("last_id")
            .or_else(|| value.get("lastId"))
            .or_else(|| value.get("nextPageToken"))
            .or_else(|| value.get("next_page_token"))
            .or_else(|| value.get("next"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .filter(|value| !value.is_empty());
        if next.is_none() || next == after {
            return Err("OpenAI 分页响应缺少有效游标".to_string());
        }
        after = next;
    }
    Err("OpenAI 模型列表分页超过上限".to_string())
}

async fn fetch_anthropic_models(
    client: &reqwest::Client,
    provider: &Provider,
) -> Result<Vec<String>, String> {
    let base = provider.base_url.trim_end_matches('/');
    let url = join_upstream_url(base, "/v1/models");
    let mut result = Vec::new();
    let mut after_id = None::<String>;
    for _ in 0..MAX_PAGES {
        let mut request = build_model_request(client, provider, Protocol::AnthropicMessages, &url);
        if let Some(after_id) = after_id.as_deref() {
            request = request.query(&[("after_id", after_id)]);
        }
        let response = request
            .send()
            .await
            .map_err(|error| format!("请求 Anthropic 模型列表失败: {}", error))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| format!("读取 Anthropic 模型列表失败: {}", error))?;
        if !status.is_success() {
            return Err(format!("{}: {}", status, truncate_error(&body)));
        }
        let value: Value = serde_json::from_str(&body)
            .map_err(|error| format!("解析 Anthropic 模型列表失败: {}", error))?;
        result.extend(read_ids(&value, "data"));
        let has_more = value
            .get("has_more")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !has_more {
            return normalize_models(result);
        }
        let next = value
            .get("last_id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .filter(|value| !value.is_empty());
        if next.is_none() || next == after_id {
            return Err("Anthropic 分页响应缺少有效 last_id".to_string());
        }
        after_id = next;
    }
    Err("Anthropic 模型列表分页超过上限".to_string())
}

async fn fetch_gemini_models(
    client: &reqwest::Client,
    provider: &Provider,
) -> Result<Vec<String>, String> {
    let base = provider.base_url.trim_end_matches('/');
    let url = join_upstream_url(base, "/v1beta/models");
    let mut result = Vec::new();
    let mut page_token = None::<String>;
    for _ in 0..MAX_PAGES {
        let mut request =
            build_model_request(client, provider, Protocol::GeminiGenerateContent, &url);
        if let Some(page_token) = page_token.as_deref() {
            request = request.query(&[("pageToken", page_token)]);
        }
        let response = request
            .send()
            .await
            .map_err(|error| format!("请求 Gemini 模型列表失败: {}", error))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| format!("读取 Gemini 模型列表失败: {}", error))?;
        if !status.is_success() {
            return Err(format!("{}: {}", status, truncate_error(&body)));
        }
        let value: Value = serde_json::from_str(&body)
            .map_err(|error| format!("解析 Gemini 模型列表失败: {}", error))?;
        result.extend(read_gemini_model_names(&value));
        let next = value
            .get("nextPageToken")
            .and_then(Value::as_str)
            .map(str::to_string)
            .filter(|value| !value.is_empty());
        if next.is_none() {
            return normalize_models(result);
        }
        if next == page_token {
            return Err("Gemini 分页响应包含重复 page token".to_string());
        }
        page_token = next;
    }
    Err("Gemini 模型列表分页超过上限".to_string())
}

fn build_model_request<'a>(
    client: &'a reqwest::Client,
    provider: &'a Provider,
    protocol: Protocol,
    url: &'a str,
) -> reqwest::RequestBuilder {
    let mut request = client.get(url).header(
        reqwest::header::USER_AGENT,
        provider
            .custom_useragent
            .as_deref()
            .unwrap_or("ccg-gateway/model-sync"),
    );
    match protocol {
        Protocol::AnthropicMessages => {
            request = request
                .header("x-api-key", &provider.api_key)
                .header("anthropic-version", "2023-06-01");
        }
        Protocol::GeminiGenerateContent => {
            request = request.header("x-goog-api-key", &provider.api_key);
        }
        Protocol::OpenaiChat | Protocol::OpenaiResponses => {
            request = request.header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", provider.api_key),
            );
        }
    }
    request
}

fn read_ids(value: &Value, key: &str) -> Vec<String> {
    let items = value
        .get(key)
        .and_then(Value::as_array)
        .or_else(|| {
            (key == "data")
                .then(|| value.get("models"))
                .flatten()
                .and_then(Value::as_array)
        })
        .or_else(|| {
            (value.is_array() && key == "data")
                .then_some(value)
                .and_then(Value::as_array)
        });
    items
        .into_iter()
        .flatten()
        .filter_map(|item| {
            item.as_str().map(str::to_string).or_else(|| {
                item.get("id")
                    .or_else(|| item.get("name"))
                    .or_else(|| item.get("model"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
        })
        .collect()
}

fn read_gemini_model_names(value: &Value) -> Vec<String> {
    value
        .get("models")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|model| {
            model
                .get("supportedGenerationMethods")
                .and_then(Value::as_array)
                .map(|methods| {
                    methods.iter().any(|method| {
                        method
                            .as_str()
                            .is_some_and(|name| name.eq_ignore_ascii_case("generateContent"))
                    })
                })
                .unwrap_or(true)
        })
        .filter_map(|model| model.get("name").and_then(Value::as_str))
        .map(|name| name.strip_prefix("models/").unwrap_or(name).to_string())
        .collect()
}

fn normalize_models(models: Vec<String>) -> Result<Vec<String>, String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for model in models {
        let model = model.trim().to_string();
        if model.is_empty() || !seen.insert(model.to_ascii_lowercase()) {
            continue;
        }
        result.push(model);
    }
    if result.is_empty() {
        return Err("模型列表为空".to_string());
    }
    Ok(result)
}

async fn ensure_state_row(db: &SqlitePool, provider_id: i64) -> Result<(), String> {
    let now = now_timestamp();
    sqlx::query(
        "INSERT OR IGNORE INTO provider_model_sync_state (provider_id, last_attempt_at, last_success_at, last_error, model_count, updated_at) VALUES (?, NULL, NULL, NULL, 0, ?)",
    )
    .bind(provider_id)
    .bind(now)
    .execute(db)
    .await
    .map(|_| ())
    .map_err(|error| error.to_string())
}

async fn mark_failed(
    db: &SqlitePool,
    provider: &Provider,
    message: String,
) -> Result<ProviderModelsResponse, String> {
    let now = now_timestamp();
    let message = truncate_error(&message);
    sqlx::query(
        "UPDATE provider_model_sync_state
         SET last_error = ?, updated_at = ?
         WHERE provider_id = ?
           AND EXISTS (
               SELECT 1 FROM providers
               WHERE id = ?
                 AND profile = ?
                 AND protocol = ?
                 AND rtrim(trim(base_url), '/') = rtrim(trim(?), '/')
                 AND trim(api_key) = trim(?)
                 AND coalesce(trim(custom_useragent), '') = coalesce(trim(?), '')
                 AND enabled = ?
           )",
    )
    .bind(&message)
    .bind(now)
    .bind(provider.id)
    .bind(provider.id)
    .bind(&provider.profile)
    .bind(&provider.protocol)
    .bind(&provider.base_url)
    .bind(&provider.api_key)
    .bind(&provider.custom_useragent)
    .bind(provider.enabled)
    .execute(db)
    .await
    .map_err(|error| format!("{}; 写入同步状态失败: {}", message, error))?;
    Err(message)
}

fn truncate_error(value: &str) -> String {
    let value = value.trim();
    if value.chars().count() <= MAX_ERROR_BODY {
        value.to_string()
    } else {
        value.chars().take(MAX_ERROR_BODY).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gemini_sync_keeps_generation_models_only() {
        let value = serde_json::json!({
            "models": [
                {"name": "models/gemini-pro", "supportedGenerationMethods": ["generateContent"]},
                {"name": "models/text-embedding", "supportedGenerationMethods": ["embedContent"]},
                {"name": "models/legacy"}
            ]
        });

        assert_eq!(
            read_gemini_model_names(&value),
            vec!["gemini-pro".to_string(), "legacy".to_string()]
        );
    }

    #[test]
    fn normalize_models_deduplicates_case_insensitively() {
        assert_eq!(
            normalize_models(vec!["gpt-4".into(), " GPT-4 ".into(), "claude".into()]).unwrap(),
            vec!["gpt-4".to_string(), "claude".to_string()]
        );
    }
}
