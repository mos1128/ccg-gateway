use super::*;
use crate::db::models::ProviderModelsResponse;

#[tauri::command]
pub async fn get_provider_models(
    db: State<'_, SqlitePool>,
    provider_id: i64,
) -> Result<ProviderModelsResponse> {
    crate::services::model_sync::list_provider_models(db.inner(), provider_id).await
}

#[tauri::command]
pub async fn sync_provider_models(
    db: State<'_, SqlitePool>,
    provider_id: i64,
) -> Result<ProviderModelsResponse> {
    crate::services::model_sync::sync_provider(db.inner(), provider_id).await
}

#[tauri::command]
pub async fn sync_all_provider_models(db: State<'_, SqlitePool>) -> Result<()> {
    crate::services::model_sync::sync_all(db.inner()).await
}

#[tauri::command]
pub async fn add_provider_manual_model(
    db: State<'_, SqlitePool>,
    provider_id: i64,
    model_name: String,
) -> Result<ProviderModelsResponse> {
    crate::services::model_sync::add_manual_model(db.inner(), provider_id, &model_name).await
}

#[tauri::command]
pub async fn delete_provider_model(
    db: State<'_, SqlitePool>,
    provider_id: i64,
    model_id: i64,
) -> Result<ProviderModelsResponse> {
    crate::services::model_sync::delete_model(db.inner(), provider_id, model_id).await
}
