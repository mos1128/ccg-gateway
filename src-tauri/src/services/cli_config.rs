use crate::config::{expand_home_path, get_default_cli_config_dir};
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

    match result
        .and_then(|r| r.0)
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty())
    {
        Some(path) => PathBuf::from(expand_home_path(&path)),
        None => get_default_cli_config_dir(cli_type),
    }
}

pub async fn resolve_cli_config_file(db: &SqlitePool, cli_type: &str, file: &str) -> PathBuf {
    let config_dir = get_cli_config_dir_path(db, cli_type).await;
    resolve_cli_config_file_from_dir(&config_dir, file)
}

pub fn resolve_cli_config_file_from_dir(config_dir: &Path, file: &str) -> PathBuf {
    let expanded = PathBuf::from(expand_home_path(file));
    if expanded.is_absolute() || file == "~" || file.starts_with("~/") || file.starts_with("~\\") {
        expanded
    } else {
        config_dir.join(expanded)
    }
}
