use std::path::{Path, PathBuf};

pub fn claude_settings_filename(profile: &str) -> String {
    crate::services::cli_config::claude_settings_filename(profile)
}

pub fn mcp_config_path(config_dir: &Path, cli_type: &str) -> Option<PathBuf> {
    match cli_type {
        "claude_code" => config_dir.parent().map(|p| p.join(".claude.json")),
        "codex" => Some(config_dir.join("config.toml")),
        "gemini" => Some(config_dir.join("settings.json")),
        _ => None,
    }
}

pub fn prompt_file_path(config_dir: &Path, cli_type: &str) -> Option<PathBuf> {
    match cli_type {
        "claude_code" => Some(config_dir.join("CLAUDE.md")),
        "codex" => Some(config_dir.join("AGENTS.md")),
        "gemini" => Some(config_dir.join("GEMINI.md")),
        _ => None,
    }
}

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
    match cli_type {
        "claude_code" | "gemini" => {
            serde_json::from_str::<serde_json::Value>(content)
                .map_err(|e| format!("JSON 格式错误: {}", e))?;
        }
        "codex" => {
            content
                .parse::<toml_edit::DocumentMut>()
                .map_err(|e| format!("TOML 格式错误: {}", e))?;
        }
        _ => {}
    }
    Ok(())
}
