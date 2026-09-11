use atomic_write_file::AtomicWriteFile;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::OnceLock;

#[cfg(debug_assertions)]
pub const DEFAULT_LOG_LEVEL: &str = "info,ccg_gateway=debug,ccg_gateway_lib=debug";

#[cfg(not(debug_assertions))]
pub const DEFAULT_LOG_LEVEL: &str = "info";

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
    #[serde(default = "default_stats_db_path")]
    pub stats_path: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BootstrapConfig {
    #[serde(default)]
    pub data_dir: Option<String>,
    #[serde(default)]
    pub log_file: Option<bool>,
    #[serde(default)]
    pub log_level: Option<String>,
}

fn default_port() -> u16 {
    gateway_port_env_override().unwrap_or(7788)
}

fn default_host() -> String {
    gateway_host_env_override().unwrap_or_else(|| "127.0.0.1".into())
}

pub fn gateway_port_env_override() -> Option<u16> {
    std::env::var("CCG_GATEWAY_PORT")
        .ok()
        .and_then(|port| port.parse().ok())
        .filter(|port| *port > 0)
}

pub fn gateway_host_env_override() -> Option<String> {
    std::env::var("CCG_GATEWAY_HOST")
        .ok()
        .and_then(|host| validate_gateway_host(&host).ok())
}

pub fn validate_gateway_host(host: &str) -> Result<String, String> {
    let host = host.trim();
    if host.is_empty() {
        return Err("监听地址不能为空".to_string());
    }
    if host.parse::<IpAddr>().is_ok() || host.eq_ignore_ascii_case("localhost") {
        return Ok(host.to_string());
    }
    if host.len() > 253
        || !host.is_ascii()
        || host.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
    {
        return Err("监听地址必须是有效的 IP 地址或主机名".to_string());
    }
    Ok(host.to_string())
}

pub(crate) fn bind_addr_for(host: &str, port: u16) -> String {
    format!("{}:{}", host_with_ipv6_brackets(host), port)
}

fn default_db_path() -> PathBuf {
    get_data_dir().join("ccg_gateway.db")
}

fn default_log_db_path() -> PathBuf {
    get_data_dir().join("ccg_logs.db")
}

fn default_stats_db_path() -> PathBuf {
    get_data_dir().join("ccg_stats.db")
}

pub fn data_dir_env_override() -> Option<PathBuf> {
    std::env::var("CCG_DATA_DIR")
        .ok()
        .map(|dir| dir.trim().to_string())
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
}

pub fn log_file_env_override() -> Option<bool> {
    std::env::var("CCG_LOG_FILE").ok().and_then(|value| {
        match value.trim().to_ascii_lowercase().as_str() {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        }
    })
}

pub fn log_level_env_override() -> Option<String> {
    std::env::var("CCG_LOG_LEVEL")
        .ok()
        .map(|level| level.trim().to_string())
        .filter(|level| !level.is_empty() && tracing_subscriber::EnvFilter::try_new(level).is_ok())
}

pub fn effective_log_level(default: &str) -> String {
    log_level_env_override()
        .or_else(|| {
            bootstrap_config()
                .log_level
                .as_ref()
                .map(|level| level.trim().to_string())
                .filter(|level| {
                    !level.is_empty() && tracing_subscriber::EnvFilter::try_new(level).is_ok()
                })
        })
        .unwrap_or_else(|| default.to_string())
}

pub fn bootstrap_config_path() -> PathBuf {
    dirs::config_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ccg-gateway")
        .join("bootstrap.json")
}

fn read_bootstrap_config() -> BootstrapConfig {
    fs::read_to_string(bootstrap_config_path())
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

fn bootstrap_config() -> &'static BootstrapConfig {
    static CONFIG: OnceLock<BootstrapConfig> = OnceLock::new();
    CONFIG.get_or_init(read_bootstrap_config)
}

pub fn update_bootstrap_config(
    data_dir: Option<String>,
    log_file: Option<bool>,
    log_level: Option<String>,
) -> Result<(), String> {
    // 以磁盘当前值为基准，避免进程内连续部分更新互相覆盖。
    let mut config = read_bootstrap_config();
    if let Some(data_dir) = data_dir {
        config.data_dir = Some(validate_data_dir(&data_dir)?);
    }
    if let Some(log_file) = log_file {
        config.log_file = Some(log_file);
    }
    if let Some(log_level) = log_level {
        config.log_level = Some(validate_log_level(&log_level)?);
    }

    let path = bootstrap_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("无法创建配置目录 {}：{}", parent.display(), error))?;
    }
    let content = serde_json::to_string_pretty(&config)
        .map_err(|error| format!("无法序列化启动配置：{}", error))?;
    let mut file = AtomicWriteFile::options()
        .open(&path)
        .map_err(|error| format!("无法创建启动配置 {}：{}", path.display(), error))?;
    file.write_all(content.as_bytes())
        .map_err(|error| format!("无法写入启动配置 {}：{}", path.display(), error))?;
    file.commit()
        .map_err(|error| format!("无法替换启动配置 {}：{}", path.display(), error))
}

pub fn validate_log_level(log_level: &str) -> Result<String, String> {
    let log_level = log_level.trim();
    if log_level.is_empty() {
        return Err("日志级别不能为空".to_string());
    }
    tracing_subscriber::EnvFilter::try_new(log_level)
        .map_err(|error| format!("日志级别格式无效：{}", error))?;
    Ok(log_level.to_string())
}

fn validate_data_dir(data_dir: &str) -> Result<String, String> {
    let data_dir = data_dir.trim();
    if data_dir.is_empty() {
        return Err("数据目录不能为空".to_string());
    }

    let path = PathBuf::from(expand_home_path(data_dir));
    if !path.is_absolute() {
        return Err("数据目录必须使用绝对路径".to_string());
    }
    fs::create_dir_all(&path)
        .map_err(|error| format!("无法创建数据目录 {}：{}", path.display(), error))?;

    let probe_path = path.join(format!(".ccg-gateway-write-test-{}", std::process::id()));
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe_path)
        .map_err(|error| format!("数据目录不可写 {}：{}", path.display(), error))?;
    let _ = fs::remove_file(&probe_path);

    Ok(path.to_string_lossy().to_string())
}

pub fn get_data_dir() -> PathBuf {
    if let Some(dir) = data_dir_env_override() {
        return dir;
    }
    if let Some(dir) = bootstrap_config()
        .data_dir
        .as_deref()
        .map(str::trim)
        .filter(|dir| !dir.is_empty())
    {
        return PathBuf::from(expand_home_path(dir));
    }
    get_default_data_dir()
}

pub fn get_default_data_dir() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        return home.join(".ccg-gateway");
    }
    PathBuf::from(".").join(".ccg-gateway")
}

pub fn get_log_dir() -> PathBuf {
    get_data_dir().join("logs")
}

pub fn is_file_log_enabled() -> bool {
    log_file_env_override()
        .or(bootstrap_config().log_file)
        .unwrap_or(false)
}

/// 获取 CLI 默认配置目录（不涉及数据库）
pub fn get_default_cli_config_dir(cli_type: &str) -> PathBuf {
    crate::services::agent::default_config_directory(cli_type)
        .map(expand_home_path)
        .map(PathBuf::from)
        .unwrap_or_else(|| get_data_dir().join("unmanaged-agents").join(cli_type))
}

/// 展开 ~ 为用户目录
pub fn expand_home_path(path: &str) -> String {
    if path == "~" || path.starts_with("~/") || path.starts_with("~\\") {
        let mut expanded = dirs::home_dir().unwrap_or_default();
        for component in path[1..].split(['/', '\\']).filter(|part| !part.is_empty()) {
            expanded.push(component);
        }
        expanded.to_string_lossy().to_string()
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
                stats_path: default_stats_db_path(),
            },
        }
    }
}

impl Config {
    pub fn with_stored_server(mut self, host: &str, port: i64) -> Self {
        if gateway_host_env_override().is_none() {
            self.server.host = validate_gateway_host(host).unwrap_or_else(|_| "127.0.0.1".into());
        }
        if gateway_port_env_override().is_none() {
            self.server.port = u16::try_from(port)
                .ok()
                .filter(|port| *port > 0)
                .unwrap_or(7788);
        }
        self
    }

    pub fn bind_addr(&self) -> String {
        bind_addr_for(&self.server.host, self.server.port)
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
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(address)) if address.is_unspecified() => "127.0.0.1".to_string(),
        Ok(IpAddr::V6(address)) if address.is_unspecified() => "[::1]".to_string(),
        _ => host_with_ipv6_brackets(host),
    }
}

fn host_with_ipv6_brackets(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{}]", host)
    } else {
        host.to_string()
    }
}
