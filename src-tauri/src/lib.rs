pub mod api;
pub mod commands;
pub mod config;
pub mod db;
pub mod services;
pub mod time;

use config::Config;
use db::{init_db, init_stats_db};
use sqlx::SqlitePool;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

// Type wrappers for Tauri state
pub struct LogDb(pub SqlitePool);
pub struct StatsDb(pub SqlitePool);

impl std::ops::Deref for LogDb {
    type Target = SqlitePool;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::Deref for StatsDb {
    type Target = SqlitePool;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = Config::default();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let config = config.clone();
            app.manage(config.clone());

            // Initialize database
            let db_path = config.database.path.clone();
            let log_db_path = config.database.log_path.clone();
            let stats_db_path = config.database.stats_path.clone();

            tauri::async_runtime::block_on(async {
                // Ensure data directory exists
                if let Some(parent) = db_path.parent() {
                    std::fs::create_dir_all(parent).ok();
                }

                let db = match init_db(&db_path).await {
                    Ok(db) => db,
                    Err(e) => {
                        tracing::error!("Failed to init database: {}", e);
                        std::process::exit(1);
                    }
                };
                let log_db = match init_db(&log_db_path).await {
                    Ok(db) => db,
                    Err(e) => {
                        tracing::error!("Failed to init log database: {}", e);
                        std::process::exit(1);
                    }
                };
                let stats_db = match init_stats_db(&stats_db_path).await {
                    Ok(db) => db,
                    Err(e) => {
                        tracing::error!("Failed to init stats database: {}", e);
                        std::process::exit(1);
                    }
                };

                app.manage(db.clone());
                app.manage(LogDb(log_db.clone()));
                app.manage(StatsDb(stats_db.clone()));
                let app_handle = app.handle().clone();
                services::scheduler::start_scheduler(
                    db.clone(),
                    log_db.clone(),
                    app_handle.clone(),
                );

                let addr = config.bind_addr();

                tokio::spawn(async move {
                    if let Err(e) = ensure_historical_stats_ready(&log_db, &stats_db).await {
                        tracing::error!("Historical stats backfill failed: {}", e);
                        return;
                    }

                    let http_client = reqwest::Client::builder()
                        .pool_max_idle_per_host(10)
                        .pool_idle_timeout(std::time::Duration::from_secs(90))
                        .build()
                        .unwrap_or_default();

                    let state = api::AppState {
                        db,
                        log_db,
                        stats_db,
                        app_handle,
                        http_client,
                    };

                    let router = api::create_router(state);

                    let listener = match tokio::net::TcpListener::bind(&addr).await {
                        Ok(listener) => {
                            tracing::info!("Gateway HTTP server listening on {}", addr);
                            listener
                        }
                        Err(e) => {
                            tracing::error!("Failed to bind to {}: {}", addr, e);
                            std::process::exit(1);
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
            let icon = match app.default_window_icon().cloned() {
                Some(icon) => icon,
                None => {
                    tracing::error!("Failed to get default window icon");
                    std::process::exit(1);
                }
            };

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
                .on_tray_icon_event(|tray, event| match event {
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
            commands::provider_commands::get_providers,
            commands::provider_commands::get_provider,
            commands::provider_commands::create_provider,
            commands::provider_commands::update_provider,
            commands::provider_commands::delete_provider,
            commands::provider_commands::reorder_providers,
            commands::provider_commands::reset_provider_failures,
            commands::provider_commands::test_provider_models,
            commands::scheduled_task_commands::get_scheduled_tasks,
            commands::scheduled_task_commands::get_scheduled_task,
            commands::scheduled_task_commands::create_scheduled_task,
            commands::scheduled_task_commands::update_scheduled_task,
            commands::scheduled_task_commands::delete_scheduled_task,
            commands::scheduled_task_commands::run_scheduled_task_now,
            commands::scheduled_task_commands::get_scheduled_task_runs,
            commands::scheduled_task_commands::get_scheduled_task_run_items,
            commands::settings_commands::get_gateway_settings,
            commands::settings_commands::update_gateway_settings,
            commands::settings_commands::get_timeout_settings,
            commands::settings_commands::update_timeout_settings,
            commands::settings_commands::get_cli_settings,
            commands::settings_commands::get_claude_profile_settings_status,
            commands::settings_commands::ensure_claude_profile_settings,
            commands::settings_commands::get_codex_profile_settings_status,
            commands::settings_commands::ensure_codex_profile_settings,
            commands::settings_commands::update_cli_settings,
            commands::log_commands::get_request_logs,
            commands::log_commands::get_request_log_detail,
            commands::log_commands::clear_request_logs,
            commands::log_commands::clear_request_detail_files,
            commands::log_commands::clear_old_request_logs,
            commands::log_commands::clear_old_request_detail_files,
            commands::log_commands::get_system_logs,
            commands::log_commands::clear_system_logs,
            commands::system_commands::get_system_status,
            commands::system_commands::toggle_devtools,
            commands::mcp_commands::get_mcps,
            commands::mcp_commands::get_mcp,
            commands::mcp_commands::create_mcp,
            commands::mcp_commands::update_mcp,
            commands::mcp_commands::toggle_mcp_cli,
            commands::mcp_commands::delete_mcp,
            commands::prompt_commands::get_prompts,
            commands::prompt_commands::get_prompt,
            commands::prompt_commands::create_prompt,
            commands::prompt_commands::update_prompt,
            commands::prompt_commands::toggle_prompt_cli,
            commands::prompt_commands::delete_prompt,
            commands::skill_commands::get_skill_repos,
            commands::skill_commands::add_skill_repo,
            commands::skill_commands::remove_skill_repo,
            commands::skill_commands::discover_repo_skills,
            commands::skill_commands::reinstall_skill_repo,
            commands::skill_commands::install_skill,
            commands::skill_commands::reinstall_skill,
            commands::skill_commands::uninstall_skill,
            commands::skill_commands::get_installed_skills,
            commands::skill_commands::toggle_skill_cli,
            commands::skill_commands::get_skill_favorites,
            commands::skill_commands::add_skill_favorite,
            commands::skill_commands::toggle_installed_skill_favorite,
            commands::skill_commands::remove_skill_favorite,
            commands::skill_commands::install_favorite_skill,
            commands::skill_commands::reinstall_favorite_skill,
            commands::stats_commands::get_provider_stats,
            commands::stats_commands::get_advanced_stats,
            commands::session_commands::get_session_projects,
            commands::session_commands::get_project_sessions,
            commands::session_commands::get_session_messages,
            commands::session_commands::delete_session,
            commands::session_commands::delete_project,
            commands::backup_commands::get_webdav_settings,
            commands::backup_commands::update_webdav_settings,
            commands::backup_commands::test_webdav_connection,
            commands::backup_commands::export_to_local_path,
            commands::backup_commands::import_from_local,
            commands::backup_commands::export_to_webdav,
            commands::backup_commands::list_webdav_backups,
            commands::backup_commands::import_from_webdav,
            commands::backup_commands::delete_webdav_backup,
            commands::update_commands::check_for_updates,
            commands::credential_commands::get_credentials,
            commands::credential_commands::get_credential,
            commands::credential_commands::create_credential,
            commands::credential_commands::update_credential,
            commands::credential_commands::delete_credential,
            commands::credential_commands::reorder_credentials,
            commands::credential_commands::read_cli_credential,
            commands::credential_commands::get_cli_mode,
            commands::credential_commands::set_cli_mode,
            commands::plugin_commands::get_installed_plugins,
            commands::plugin_commands::get_marketplace_plugins,
            commands::plugin_commands::get_marketplaces,
            commands::plugin_commands::plugin_action,
            commands::plugin_commands::get_plugin_favorites,
            commands::plugin_commands::add_plugin_favorite,
            commands::plugin_commands::remove_plugin_favorite,
            commands::plugin_commands::marketplace_action,
            commands::plugin_commands::install_favorite_plugin,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            tracing::error!("Failed to run tauri application: {}", e);
            std::process::exit(1);
        });
}

async fn ensure_historical_stats_ready(
    log_db: &SqlitePool,
    stats_db: &SqlitePool,
) -> Result<(), sqlx::Error> {
    if services::stats::is_historical_backfill_done(stats_db).await? {
        return Ok(());
    }

    let max_log_id = services::stats::request_log_max_id(log_db).await?;
    tracing::info!(
        "Starting historical stats backfill before gateway startup up to log id {}",
        max_log_id
    );
    services::stats::backfill_historical_stats(log_db, stats_db, max_log_id).await
}
