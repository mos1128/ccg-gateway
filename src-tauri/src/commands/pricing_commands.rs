use super::*;
use crate::db::models::{ModelPriceCatalogEntry, PriceSyncState};

#[tauri::command]
pub async fn get_price_sync_status(db: State<'_, SqlitePool>) -> Result<PriceSyncState> {
    crate::services::pricing::get_status(db.inner()).await
}

#[tauri::command]
pub async fn sync_model_prices(db: State<'_, SqlitePool>) -> Result<PriceSyncState> {
    crate::services::pricing::sync_now(db.inner()).await
}

#[tauri::command]
pub async fn get_model_price_catalog(
    db: State<'_, SqlitePool>,
    query: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<ModelPriceCatalogEntry>> {
    let limit = limit.unwrap_or(200).clamp(1, 5000);
    let query = query.unwrap_or_default().trim().to_ascii_lowercase();
    if query.is_empty() {
        return sqlx::query_as::<_, ModelPriceCatalogEntry>(
            "SELECT * FROM model_price_catalog ORDER BY model_name, model_key LIMIT ?",
        )
        .bind(limit)
        .fetch_all(db.inner())
        .await
        .map_err(|error| error.to_string());
    }

    let pattern = format!("%{}%", query);
    sqlx::query_as::<_, ModelPriceCatalogEntry>(
        "SELECT * FROM model_price_catalog WHERE lower(model_key) LIKE ? OR lower(model_name) LIKE ? ORDER BY model_name, model_key LIMIT ?",
    )
    .bind(&pattern)
    .bind(&pattern)
    .bind(limit)
    .fetch_all(db.inner())
    .await
    .map_err(|error| error.to_string())
}
