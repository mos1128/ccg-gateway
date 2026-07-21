use crate::db::models::{AgentDefinitionLoadError, AgentDiagnostic, AgentInfo};
use crate::services::agent;
use crate::LogDb;
use sqlx::SqlitePool;
use tauri::State;

#[tauri::command]
pub async fn get_agents(db: State<'_, SqlitePool>) -> Result<Vec<AgentInfo>, String> {
    agent::ordered_agents(db.inner())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_agent_definition_errors() -> Vec<AgentDefinitionLoadError> {
    agent::definition_load_errors().to_vec()
}

#[tauri::command]
pub async fn get_agent_diagnostics(
    log_db: State<'_, LogDb>,
    kind: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<AgentDiagnostic>, String> {
    let limit = limit.unwrap_or(100).clamp(1, 500);
    match kind.filter(|kind| !kind.trim().is_empty()) {
        Some(kind) => sqlx::query_as::<_, AgentDiagnostic>(
            "SELECT * FROM agent_diagnostics WHERE kind = ? ORDER BY last_seen DESC LIMIT ?",
        )
        .bind(kind)
        .bind(limit)
        .fetch_all(&log_db.0)
        .await
        .map_err(|error| error.to_string()),
        None => sqlx::query_as::<_, AgentDiagnostic>(
            "SELECT * FROM agent_diagnostics ORDER BY last_seen DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&log_db.0)
        .await
        .map_err(|error| error.to_string()),
    }
}
