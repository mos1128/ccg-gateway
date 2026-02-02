pub mod api;
pub mod commands;
pub mod config;
pub mod db;
pub mod services;

use config::Config;
use db::init_db;
use sqlx::SqlitePool;
use tauri::Manager;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};

// Type wrappers for Tauri state
pub struct LogDb(pub SqlitePool);

impl std::ops::Deref for LogDb {
    type Target = SqlitePool;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = Config::load();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            let config = config.clone();

            // Initialize database
            let db_path = config.database.path.clone();
            let log_db_path = config.database.log_path.clone();

            tauri::async_runtime::block_on(async {
                // Ensure data directory exists
                if let Some(parent) = db_path.parent() {
                    std::fs::create_dir_all(parent).ok();
                }

                let db = init_db(&db_path).await.expect("Failed to init database");
                let log_db = init_db(&log_db_path)
                    .await
                    .expect("Failed to init log database");

                app.manage(db.clone());
                app.manage(LogDb(log_db.clone()));

                // Start HTTP server for proxy
                let state = api::AppState {
                    db: db.clone(),
                    log_db: log_db.clone(),
                };

                let router = api::create_router(state);
                let addr = format!("{}:{}", config.server.host, config.server.port);

            tokio::spawn(async move {
                // Bind listener with better error handling
                let listener = match tokio::net::TcpListener::bind(&addr).await {
                    Ok(listener) => {
                        tracing::info!("Gateway HTTP server listening on {}", addr);
                        listener
                    }
                    Err(e) => {
                        tracing::error!("Failed to bind to {}: {}", addr, e);
                        panic!("Cannot bind to address {}: {}", addr, e);
                    }
                };

                if let Err(e) = axum::serve(listener, router).await {
                    tracing::error!("Gateway server error: {}", e);
                }
            });
            });

            // Setup tray icon with menu
            let show_item = MenuItemBuilder::with_id("show", "显示窗口").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "退出").build(app)?;
            let menu = MenuBuilder::new(app)
                .items(&[&show_item, &quit_item])
                .build()?;

            // Get default app icon for tray
            let icon = app.default_window_icon().cloned().unwrap();
            
            let _tray = TrayIconBuilder::new()
                .icon(icon)
                .tooltip("CCG Gateway")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.unminimize();
                        }
                    }
                    "quit" => {
                        std::process::exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    match event {
                        TrayIconEvent::Click {
                            button: tauri::tray::MouseButton::Left,
                            button_state: tauri::tray::MouseButtonState::Up,
                            ..  
                        } => {
                            if let Some(window) = tray.app_handle().get_webview_window("main") {
                                match (window.is_visible(), window.is_minimized()) {
                                    (Ok(true), Ok(false)) => {
                                        let _ = window.hide();
                                    }
                                    _ => {
                                        let _ = window.show();
                                        let _ = window.unminimize();
                                        let _ = window.set_focus();
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // Handle window close event - always minimize to tray
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        let _ = window_clone.hide();
                        api.prevent_close();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_providers,
            commands::get_provider,
            commands::create_provider,
            commands::update_provider,
            commands::delete_provider,
            commands::reorder_providers,
            commands::reset_provider_failures,
            commands::get_gateway_settings,
            commands::update_gateway_settings,
            commands::get_timeout_settings,
            commands::update_timeout_settings,
            commands::get_cli_settings,
            commands::update_cli_settings,
            commands::get_request_logs,
            commands::get_request_log_detail,
            commands::clear_request_logs,
            commands::get_system_logs,
            commands::clear_system_logs,
            commands::get_system_status,
            commands::get_mcps,
            commands::get_mcp,
            commands::create_mcp,
            commands::update_mcp,
            commands::delete_mcp,
            commands::get_prompts,
            commands::get_prompt,
            commands::create_prompt,
            commands::update_prompt,
            commands::delete_prompt,
            commands::get_skill_repos,
            commands::add_skill_repo,
            commands::remove_skill_repo,
            commands::update_skill_repo,
            commands::discover_repo_skills,
            commands::refresh_repo_skills,
            commands::install_skill,
            commands::uninstall_skill,
            commands::get_installed_skills,
            commands::toggle_skill_cli,
            commands::get_daily_stats,
            commands::get_provider_stats,
            commands::get_session_projects,
            commands::get_project_sessions,
            commands::get_session_messages,
            commands::delete_session,
            commands::delete_project,
            commands::get_webdav_settings,
            commands::update_webdav_settings,
            commands::test_webdav_connection,
            commands::export_to_local,
            commands::import_from_local,
            commands::export_to_webdav,
            commands::list_webdav_backups,
            commands::import_from_webdav,
            commands::delete_webdav_backup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
