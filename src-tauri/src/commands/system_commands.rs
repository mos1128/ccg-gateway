use super::*;

#[tauri::command]
pub async fn get_system_status(
    config: State<'_, Config>,
    runtime: State<'_, crate::GatewayRuntimeState>,
) -> Result<SystemStatus> {
    let snapshot = runtime.snapshot();
    Ok(SystemStatus {
        status: snapshot.status.to_string(),
        host: config.server.host.clone(),
        port: config.server.port,
        gateway_url: config.gateway_base_url(),
        error_message: snapshot.error_message,
        host_env_override: crate::config::gateway_host_env_override().is_some(),
        port_env_override: crate::config::gateway_port_env_override().is_some(),
        data_dir: crate::config::get_data_dir().to_string_lossy().to_string(),
        default_data_dir: crate::config::get_default_data_dir()
            .to_string_lossy()
            .to_string(),
        data_dir_env_override: crate::config::data_dir_env_override().is_some(),
        log_file: crate::config::is_file_log_enabled(),
        log_file_env_override: crate::config::log_file_env_override().is_some(),
        log_level: crate::config::effective_log_level(crate::config::DEFAULT_LOG_LEVEL),
        log_level_env_override: crate::config::log_level_env_override().is_some(),
        uptime: snapshot
            .started_at
            .map(|started_at| started_at.elapsed().as_secs() as i64)
            .unwrap_or(0),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[tauri::command]
pub fn update_bootstrap_settings(
    data_dir: Option<String>,
    log_file: Option<bool>,
    log_level: Option<String>,
) -> Result<()> {
    crate::config::update_bootstrap_config(data_dir, log_file, log_level)
}

#[tauri::command]
pub fn close_app(app: tauri::AppHandle) -> Result<()> {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        app.exit(0);
    });
    Ok(())
}

#[tauri::command]
pub async fn toggle_devtools(app: tauri::AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_devtools_open() {
            window.close_devtools();
        } else {
            window.open_devtools();
        }
    }
    Ok(())
}
