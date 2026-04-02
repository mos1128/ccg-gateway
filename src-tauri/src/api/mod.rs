pub mod handlers;

use axum::{routing::get, Router};
use sqlx::SqlitePool;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub log_db: SqlitePool,
}

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Desktop-only mode: No /api routes needed
    // Frontend uses Tauri IPC instead of HTTP
    // Only CLI proxy is required
    Router::new()
        .route("/health", get(|| async { "ok" }))
        // Catch-all proxy route for CLI tools (Claude Code, Codex, Gemini)
        .fallback(handlers::proxy_handler_catchall)
        .layer(cors)
        .with_state(Arc::new(state))
}
