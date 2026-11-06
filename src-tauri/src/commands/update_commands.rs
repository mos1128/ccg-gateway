use super::*;
use crate::services::updater::{self, UpdateDownloadState};

#[tauri::command]
pub async fn get_update_settings(
    app: tauri::AppHandle,
    db: State<'_, SqlitePool>,
) -> Result<updater::UpdateSettingsResponse> {
    updater::get_update_settings(&app, db.inner()).await
}

#[tauri::command]
pub async fn update_update_settings(
    db: State<'_, SqlitePool>,
    auto_check: Option<bool>,
    accel_enabled: Option<bool>,
    accel_prefix: Option<String>,
) -> Result<()> {
    updater::save_update_settings(db.inner(), auto_check, accel_enabled, accel_prefix).await
}

#[tauri::command]
pub async fn check_for_updates(
    app: tauri::AppHandle,
    db: State<'_, SqlitePool>,
) -> Result<updater::UpdateCheckResult> {
    updater::check_for_updates(&app, db.inner()).await
}

#[tauri::command]
pub async fn download_update(
    app: tauri::AppHandle,
    db: State<'_, SqlitePool>,
    state: State<'_, UpdateDownloadState>,
    version: String,
) -> Result<updater::DownloadResult> {
    updater::download_update(&app, db.inner(), state.inner(), &version).await
}

#[tauri::command]
pub fn cancel_update_download(state: State<'_, UpdateDownloadState>) {
    updater::cancel_download(state.inner());
}

#[tauri::command]
pub async fn install_update(
    app: tauri::AppHandle,
    db: State<'_, SqlitePool>,
    version: String,
) -> Result<updater::InstallOutcome> {
    updater::install_update(&app, db.inner(), &version).await
}

#[tauri::command]
pub fn take_update_result() -> Result<Option<updater::UpdateApplyResult>> {
    updater::take_install_result()
}
