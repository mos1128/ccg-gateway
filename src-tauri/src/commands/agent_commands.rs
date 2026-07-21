use crate::db::models::{AgentDefinitionLoadError, AgentDiagnostic, AgentInfo, ConfigFormat};
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
pub fn validate_config_content(format: ConfigFormat, content: String) -> Result<(), String> {
    match format {
        ConfigFormat::Json => serde_json::from_str::<serde_json::Value>(&content)
            .map(|_| ())
            .map_err(|error| format!("JSON 格式错误: {error}")),
        ConfigFormat::Toml => toml::from_str::<toml::Value>(&content)
            .map(|_| ())
            .map_err(|error| format!("TOML 格式错误: {error}")),
        ConfigFormat::Jsonc | ConfigFormat::Env => Ok(()),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_json_and_toml_with_their_real_parsers() {
        assert!(validate_config_content(ConfigFormat::Json, r#"{"ok":true}"#.into()).is_ok());
        assert!(validate_config_content(ConfigFormat::Json, "{]".into()).is_err());
        assert!(validate_config_content(ConfigFormat::Toml, "model = \"gpt\"".into()).is_ok());
        assert!(validate_config_content(ConfigFormat::Toml, "model = [".into()).is_err());
        assert!(validate_config_content(ConfigFormat::Env, "NOT TOML = [".into()).is_ok());
    }
}
