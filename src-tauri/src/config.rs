use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_host")]
    pub host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_path")]
    pub path: PathBuf,
    #[serde(default = "default_log_db_path")]
    pub log_path: PathBuf,
}

fn default_port() -> u16 {
    std::env::var("CCG_GATEWAY_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(7788)
}

fn default_host() -> String {
    std::env::var("CCG_GATEWAY_HOST").unwrap_or_else(|_| "127.0.0.1".into())
}

fn default_db_path() -> PathBuf {
    get_data_dir().join("ccg_gateway.db")
}

fn default_log_db_path() -> PathBuf {
    get_data_dir().join("ccg_logs.db")
}

pub fn get_data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CCG_DATA_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(home) = dirs::home_dir() {
        return home.join(".ccg-gateway");
    }
    PathBuf::from(".").join(".ccg-gateway")
}

pub fn get_log_dir() -> PathBuf {
    get_data_dir().join("logs")
}

pub fn is_file_log_enabled() -> bool {
    std::env::var("CCG_LOG_FILE")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false)
}

/// 获取 CLI 默认配置目录（不涉及数据库）
pub fn get_default_cli_config_dir(cli_type: &str) -> PathBuf {
    let home = dirs::home_dir().unwrap_or_default();
    match cli_type {
        "claude_code" => home.join(".claude"),
        "codex" => home.join(".codex"),
        "gemini" => home.join(".gemini"),
        _ => home,
    }
}

/// 展开 ~ 为用户目录
pub fn expand_home_path(path: &str) -> String {
    if path.starts_with('~') {
        let home = dirs::home_dir().unwrap_or_default();
        let remaining = &path[1..];
        let remaining = remaining
            .strip_prefix('/')
            .or_else(|| remaining.strip_prefix('\\'))
            .unwrap_or(remaining);
        home.join(remaining).to_string_lossy().to_string()
    } else {
        path.to_string()
    }
}

/// 将绝对路径收缩为 ~ 开头的相对路径
pub fn shrink_home_path(path: &str) -> String {
    let home = dirs::home_dir().unwrap_or_default();
    let home_str = home.to_string_lossy();
    let path_normalized = path.replace('\\', "/");
    let home_normalized = home_str.replace('\\', "/");

    if path_normalized.starts_with(&home_normalized) {
        let remaining = &path_normalized[home_normalized.len()..];
        if remaining.is_empty() {
            "~".to_string()
        } else {
            format!("~{}", remaining)
        }
    } else {
        path.to_string()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                port: default_port(),
                host: default_host(),
            },
            database: DatabaseConfig {
                path: default_db_path(),
                log_path: default_log_db_path(),
            },
        }
    }
}

impl Config {
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    pub fn gateway_base_url(&self) -> String {
        format!(
            "http://{}:{}",
            host_for_url(&self.server.host),
            self.server.port
        )
    }
}

fn host_for_url(host: &str) -> String {
    match host {
        "0.0.0.0" => "127.0.0.1".to_string(),
        "::" => "[::1]".to_string(),
        h if h.contains(':') && !h.starts_with('[') => format!("[{}]", h),
        h => h.to_string(),
    }
}
