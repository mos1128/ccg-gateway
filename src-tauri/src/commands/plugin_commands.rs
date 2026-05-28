use super::*;

// ==================== 插件管理命令 ====================

use crate::services::plugin::{MarketplaceActionResult, PluginActionResult};

/// 获取收藏列表
async fn get_favorites_raw(
    db: &SqlitePool,
) -> Result<Vec<(String, String, String, Option<String>)>> {
    let favorites: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT plugin_id, plugin_name, marketplace_name, marketplace_source FROM plugin_favorites",
    )
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(favorites)
}

#[tauri::command]
pub async fn get_installed_plugins(db: State<'_, SqlitePool>) -> Result<Vec<PluginItem>> {
    let config_dir = get_cli_config_dir_path(db.inner(), "claude_code").await;
    crate::services::plugin::get_installed_plugins(&config_dir).await
}

#[tauri::command]
pub async fn get_marketplace_plugins(
    db: State<'_, SqlitePool>,
    market_name: String,
) -> Result<Vec<PluginItem>> {
    let config_dir = get_cli_config_dir_path(db.inner(), "claude_code").await;
    crate::services::plugin::get_marketplace_plugins(&market_name, &config_dir).await
}

#[tauri::command]
pub async fn get_plugin_favorites(db: State<'_, SqlitePool>) -> Result<Vec<PluginFavoriteItem>> {
    let favorites = get_favorites_raw(db.inner()).await?;
    crate::services::plugin::get_favorites(favorites).await
}

#[tauri::command]
pub async fn get_marketplaces(db: State<'_, SqlitePool>) -> Result<Vec<MarketplaceInfo>> {
    let config_dir = get_cli_config_dir_path(db.inner(), "claude_code").await;
    crate::services::plugin::get_marketplaces(&config_dir)
}

#[tauri::command]
pub async fn plugin_action(action: String, plugin_id: String) -> Result<PluginActionResult> {
    crate::services::plugin::plugin_action(&action, &plugin_id).await
}

#[tauri::command]
pub async fn add_plugin_favorite(
    db: State<'_, SqlitePool>,
    plugin_id: String,
    plugin_name: String,
    marketplace_name: String,
) -> Result<String> {
    let config_dir = get_cli_config_dir_path(db.inner(), "claude_code").await;
    let marketplace_source =
        crate::services::plugin::get_marketplace_source_info(&config_dir, &marketplace_name);
    let source_type =
        crate::services::plugin::get_marketplace_source_type(&config_dir, &marketplace_name);

    let now = now_timestamp();

    sqlx::query(
        "INSERT OR REPLACE INTO plugin_favorites (plugin_id, plugin_name, marketplace_name, created_at, marketplace_source) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&plugin_id)
    .bind(&plugin_name)
    .bind(&marketplace_name)
    .bind(now)
    .bind(&marketplace_source)
    .execute(db.inner())
    .await
    .map_err(map_db_error)?;

    if source_type.as_deref() == Some("directory") {
        Ok("该插件来自本地市场，可能不支持跨设备恢复".to_string())
    } else {
        Ok(String::new())
    }
}

#[tauri::command]
pub async fn remove_plugin_favorite(db: State<'_, SqlitePool>, plugin_id: String) -> Result<()> {
    sqlx::query("DELETE FROM plugin_favorites WHERE plugin_id = ?")
        .bind(&plugin_id)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

    Ok(())
}

#[tauri::command]
pub async fn marketplace_action(action: String, param: String) -> Result<MarketplaceActionResult> {
    crate::services::plugin::marketplace_action(&action, &param).await
}

#[tauri::command]
pub async fn install_favorite_plugin(
    db: State<'_, SqlitePool>,
    plugin_id: String,
    marketplace_name: String,
    marketplace_source: Option<String>,
) -> Result<crate::services::plugin::FavoriteInstallResult> {
    let config_dir = get_cli_config_dir_path(db.inner(), "claude_code").await;
    crate::services::plugin::install_favorite_plugin(
        &plugin_id,
        &marketplace_name,
        marketplace_source.as_deref(),
        &config_dir,
    )
    .await
}
