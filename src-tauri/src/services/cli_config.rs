use crate::config::{expand_home_path, get_default_cli_config_dir};
use crate::services::routing::{normalize_profile, DEFAULT_PROFILE};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};

pub async fn get_cli_config_dir_path(db: &SqlitePool, cli_type: &str) -> PathBuf {
    let result: Option<(Option<String>,)> =
        sqlx::query_as("SELECT config_dir FROM cli_settings WHERE cli_type = ?")
            .bind(cli_type)
            .fetch_optional(db)
            .await
            .ok()
            .flatten();

    match result.and_then(|r| r.0) {
        Some(path) => PathBuf::from(expand_home_path(&path)),
        None => get_default_cli_config_dir(cli_type),
    }
}

pub fn claude_settings_filename(profile: &str) -> String {
    let profile = normalize_profile(Some(profile)).unwrap_or_else(|| DEFAULT_PROFILE.to_string());
    if profile == DEFAULT_PROFILE {
        "settings.json".to_string()
    } else {
        format!("settings-ccg-{}.json", profile)
    }
}

pub fn codex_profile_config_filename(profile: &str) -> String {
    if profile == DEFAULT_PROFILE {
        "config.toml".to_string()
    } else {
        format!("{}.config.toml", profile)
    }
}

pub fn codex_profile_config_path(config_dir: &Path, profile: &str) -> PathBuf {
    config_dir.join(codex_profile_config_filename(profile))
}
