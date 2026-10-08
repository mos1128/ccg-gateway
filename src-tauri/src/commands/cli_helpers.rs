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
        .ok_or_else(|| crate::tr!("err.cli_helpers.unknown_agent", cli_type))?
        .features
        .global_preset;
    match feature.format {
        Some(crate::db::models::ConfigFormat::Json) => {
            serde_json::from_str::<serde_json::Value>(content)
                .map_err(|e| crate::tr!("err.cli_helpers.json_invalid", e))?;
        }
        Some(crate::db::models::ConfigFormat::Toml) => {
            content
                .parse::<toml_edit::DocumentMut>()
                .map_err(|e| crate::tr!("err.cli_helpers.toml_invalid", e))?;
        }
        Some(crate::db::models::ConfigFormat::Yaml) => {
            serde_yaml::from_str::<serde_json::Value>(content)
                .map_err(|e| crate::tr!("err.cli_helpers.yaml_invalid", e))?;
        }
        _ => return Err(crate::tr!("err.cli_helpers.preset_format_unsupported", cli_type)),
    }
    Ok(())
}
