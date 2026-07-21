use std::path::{Path, PathBuf};

/// Returns the projects listing directory for a given CLI type.
pub fn projects_dir(base_dir: &Path, cli_type: &str) -> PathBuf {
    match cli_type {
        "codex" => base_dir.join("sessions"),
        "gemini" => base_dir.join("tmp"),
        _ => base_dir.join("projects"),
    }
}

/// Returns the full path to a specific session file.
pub fn session_file_path(
    base_dir: &Path,
    cli_type: &str,
    project: &str,
    session_id: &str,
) -> PathBuf {
    match cli_type {
        "gemini" => base_dir
            .join("tmp")
            .join(project)
            .join("chats")
            .join(format!("{}.json", session_id)),
        _ => base_dir
            .join("projects")
            .join(project)
            .join(format!("{}.jsonl", session_id)),
    }
}

/// Returns the directory for a specific project.
pub fn project_dir(base_dir: &Path, cli_type: &str, project: &str) -> PathBuf {
    match cli_type {
        "gemini" => base_dir.join("tmp").join(project),
        _ => base_dir.join("projects").join(project),
    }
}

/// Validates that the given config content matches the expected format for the CLI type.
pub fn validate_config_format(cli_type: &str, content: &str) -> std::result::Result<(), String> {
    let feature = &crate::services::agent::get_definition(cli_type)
        .ok_or_else(|| format!("未知 Agent: {}", cli_type))?
        .features
        .global_preset;
    match feature.format {
        Some(crate::db::models::ConfigFormat::Json) => {
            serde_json::from_str::<serde_json::Value>(content)
                .map_err(|e| format!("JSON 格式错误: {}", e))?;
        }
        Some(crate::db::models::ConfigFormat::Toml) => {
            content
                .parse::<toml_edit::DocumentMut>()
                .map_err(|e| format!("TOML 格式错误: {}", e))?;
        }
        _ => return Err(format!("Agent {} 未声明可用的全局预设格式", cli_type)),
    }
    Ok(())
}
