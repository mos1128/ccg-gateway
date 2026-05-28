use super::*;

#[tauri::command]
pub async fn get_system_status(config: State<'_, Config>) -> Result<SystemStatus> {
    Ok(SystemStatus {
        status: "running".to_string(),
        host: config.server.host.clone(),
        port: config.server.port,
        gateway_url: config.gateway_base_url(),
        uptime: 0,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
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
