use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ==================== Provider 相关实体 ====================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Provider {
    pub id: i64,
    pub cli_type: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub enabled: i64,
    pub failure_threshold: i64,
    pub blacklist_minutes: i64,
    pub consecutive_failures: i64,
    pub blacklisted_until: Option<i64>,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProviderModelMap {
    pub id: i64,
    pub provider_id: i64,
    pub source_model: String,
    pub target_model: String,
    pub enabled: i64,
}

// Input DTOs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMapInput {
    pub source_model: String,
    pub target_model: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCreate {
    pub cli_type: Option<String>,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub enabled: Option<bool>,
    pub failure_threshold: Option<i64>,
    pub blacklist_minutes: Option<i64>,
    pub model_maps: Option<Vec<ModelMapInput>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderUpdate {
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub enabled: Option<bool>,
    pub failure_threshold: Option<i64>,
    pub blacklist_minutes: Option<i64>,
    pub model_maps: Option<Vec<ModelMapInput>>,
}

// Response DTOs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMapResponse {
    pub id: i64,
    pub source_model: String,
    pub target_model: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub id: i64,
    pub cli_type: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub enabled: bool,
    pub failure_threshold: i64,
    pub blacklist_minutes: i64,
    pub consecutive_failures: i64,
    pub blacklisted_until: Option<i64>,
    pub sort_order: i64,
    pub is_blacklisted: bool,
    pub model_maps: Vec<ModelMapResponse>,
}

impl From<Provider> for ProviderResponse {
    fn from(p: Provider) -> Self {
        let now = chrono::Utc::now().timestamp();
        let is_blacklisted = p.blacklisted_until.map(|t| t > now).unwrap_or(false);
        Self {
            id: p.id,
            cli_type: p.cli_type,
            name: p.name,
            base_url: p.base_url,
            api_key: p.api_key,
            enabled: p.enabled != 0,
            failure_threshold: p.failure_threshold,
            blacklist_minutes: p.blacklist_minutes,
            consecutive_failures: p.consecutive_failures,
            blacklisted_until: p.blacklisted_until,
            sort_order: p.sort_order,
            is_blacklisted,
            model_maps: vec![], // Will be populated by the caller
        }
    }
}

// ==================== Settings 相关实体 ====================

// Gateway Settings (完整版 - 对应数据库表)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GatewaySettingsRow {
    pub id: i64,
    pub debug_log: i64,
    pub updated_at: i64,
}

// Gateway Settings (简化版 - 用于API响应)
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct GatewaySettings {
    pub debug_log: i64,
}

// Timeout Settings (完整版 - 对应数据库表)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TimeoutSettingsRow {
    pub id: i64,
    pub stream_first_byte_timeout: i64,
    pub stream_idle_timeout: i64,
    pub non_stream_timeout: i64,
    pub updated_at: i64,
}

// Timeout Settings (简化版 - 用于API响应)
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct TimeoutSettings {
    pub stream_first_byte_timeout: i64,
    pub stream_idle_timeout: i64,
    pub non_stream_timeout: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeoutSettingsUpdate {
    pub stream_first_byte_timeout: Option<i64>,
    pub stream_idle_timeout: Option<i64>,
    pub non_stream_timeout: Option<i64>,
}

// CLI Settings
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct CliSettingsRow {
    pub cli_type: String,
    pub default_json_config: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
pub struct CliSettingsResponse {
    pub cli_type: String,
    pub enabled: bool,
    pub default_json_config: String,
}

#[derive(Debug, Deserialize)]
pub struct CliSettingsUpdate {
    pub enabled: Option<bool>,
    pub default_json_config: Option<String>,
}

// WebDAV Settings
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WebdavSettingsRow {
    pub id: i64,
    pub url: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub path: Option<String>,
    pub enabled: i64,
    pub updated_at: i64,
}

// WebDAV Settings (简化版 - 用于API响应)
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct WebdavSettings {
    pub url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct WebdavSettingsUpdate {
    pub url: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WebdavBackup {
    pub filename: String,
    pub size: i64,
    pub modified: String,
}

// ==================== User-Agent 映射相关实体 ====================

// User-Agent Map (对应数据库表)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UseragentMap {
    pub id: i64,
    pub source_pattern: String,
    pub target_value: String,
    pub enabled: i64,
    pub sort_order: i64,
}

// User-Agent Map Input (用于创建/更新)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseragentMapInput {
    pub source_pattern: String,
    pub target_value: String,
    pub enabled: bool,
}

// User-Agent Map Response (用于API响应)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseragentMapResponse {
    pub id: i64,
    pub source_pattern: String,
    pub target_value: String,
    pub enabled: bool,
}

impl From<UseragentMap> for UseragentMapResponse {
    fn from(m: UseragentMap) -> Self {
        Self {
            id: m.id,
            source_pattern: m.source_pattern,
            target_value: m.target_value,
            enabled: m.enabled != 0,
        }
    }
}

// ==================== MCP 相关实体 ====================

// MCP Config (对应数据库表)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct McpConfig {
    pub id: i64,
    pub name: String,
    pub config_json: String,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpCliFlag {
    pub cli_type: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct McpResponse {
    pub id: i64,
    pub name: String,
    pub config_json: String,
    pub cli_flags: Vec<McpCliFlag>,
}

#[derive(Debug, Deserialize)]
pub struct McpCreate {
    pub name: String,
    pub config_json: String,
    pub enabled: Option<bool>,
    pub cli_flags: Option<Vec<McpCliFlag>>,
}

#[derive(Debug, Deserialize)]
pub struct McpUpdate {
    pub name: Option<String>,
    pub config_json: Option<String>,
    pub enabled: Option<bool>,
    pub cli_flags: Option<Vec<McpCliFlag>>,
}

// ==================== Prompt 相关实体 ====================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PromptPreset {
    pub id: i64,
    pub name: String,
    pub content: String,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptCliFlag {
    pub cli_type: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct PromptResponse {
    pub id: i64,
    pub name: String,
    pub content: String,
    pub cli_flags: Vec<PromptCliFlag>,
}

#[derive(Debug, Deserialize)]
pub struct PromptCreate {
    pub name: String,
    pub content: String,
    pub enabled: Option<bool>,
    pub cli_flags: Option<Vec<PromptCliFlag>>,
}

#[derive(Debug, Deserialize)]
pub struct PromptUpdate {
    pub name: Option<String>,
    pub content: Option<String>,
    pub enabled: Option<bool>,
    pub cli_flags: Option<Vec<PromptCliFlag>>,
}

// ==================== Skill 相关实体 ====================

// Skill Repo (仓库配置 - 对应数据库表)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillRepo {
    pub owner: String,
    pub name: String,
    pub branch: String,
}

#[derive(Debug, Deserialize)]
pub struct SkillRepoCreate {
    pub url: String,           // GitHub 仓库 URL
    pub branch: Option<String>,
}

// Skill Config (已安装的 Skill - 对应数据库表)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillConfig {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub directory: String,
    pub repo_owner: Option<String>,
    pub repo_name: Option<String>,
    pub repo_branch: Option<String>,
    pub readme_url: Option<String>,
    pub installed_at: i64,
}

// 可发现的 Skill (来自仓库，未安装)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverableSkill {
    pub key: String,           // 唯一标识: "owner/repo:directory"
    pub name: String,
    pub description: String,
    pub directory: String,     // 目录路径
    pub readme_url: Option<String>,
    pub repo_owner: String,
    pub repo_name: String,
    pub repo_branch: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SkillCliFlag {
    pub cli_type: String,
    pub enabled: bool,
}

// 已安装 Skill 响应
#[derive(Debug, Serialize)]
pub struct InstalledSkillResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub directory: String,
    pub repo_owner: Option<String>,
    pub repo_name: Option<String>,
    pub repo_branch: Option<String>,
    pub readme_url: Option<String>,
    pub installed_at: i64,
    pub cli_flags: Vec<SkillCliFlag>,
    pub exists_on_disk: bool, // skill 文件是否存在于本地
}

// ==================== Request Logs 相关实体 ====================

/// Request log detail info (用于写入日志)
#[derive(Default)]
pub struct RequestLogInfo {
    pub client_headers: Option<String>,
    pub client_body: Option<String>,
    pub forward_url: Option<String>,
    pub forward_headers: Option<String>,
    pub forward_body: Option<String>,
    pub provider_headers: Option<String>,
    pub provider_body: Option<String>,
    pub error_message: Option<String>,
}

// Request Log Item (列表视图)
#[derive(Debug, Serialize, FromRow)]
pub struct RequestLogItem {
    pub id: i64,
    pub created_at: i64,
    pub cli_type: String,
    pub provider_name: String,
    pub model_id: Option<String>,
    pub status_code: Option<i64>,
    pub elapsed_ms: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub client_method: String,
    pub client_path: String,
}

// Request Log Detail (详情视图)
#[derive(Debug, Serialize, FromRow)]
pub struct RequestLogDetail {
    pub id: i64,
    pub created_at: i64,
    pub cli_type: String,
    pub provider_name: String,
    pub model_id: Option<String>,
    pub status_code: Option<i64>,
    pub elapsed_ms: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub client_method: String,
    pub client_path: String,
    pub client_headers: Option<String>,
    pub client_body: Option<String>,
    pub forward_url: Option<String>,
    pub forward_headers: Option<String>,
    pub forward_body: Option<String>,
    pub provider_headers: Option<String>,
    pub provider_body: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedLogs {
    pub items: Vec<RequestLogItem>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

// ==================== System Logs 相关实体 ====================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SystemLog {
    pub id: i64,
    pub created_at: i64,
    pub event_type: String,
    pub message: String,
}

// System Log Item (用于列表视图)
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SystemLogItem {
    pub id: i64,
    pub created_at: i64,
    pub event_type: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SystemLogListResponse {
    pub items: Vec<SystemLogItem>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

// ==================== Usage Stats 相关实体 ====================

// Daily Usage Stats (对应 usage_daily 表)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UsageDaily {
    pub usage_date: String,
    pub provider_name: String,
    pub cli_type: String,
    pub request_count: i64,
    pub success_count: i64,
    pub failure_count: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

// Daily Stats (别名，用于向后兼容)
pub type DailyStats = UsageDaily;

// Provider Stats (从 request_logs 聚合)
#[derive(Debug, Serialize, FromRow)]
pub struct ProviderStatsRow {
    pub cli_type: String,
    pub provider_name: String,
    pub model_id: String,
    pub total_requests: i64,
    pub total_success: i64,
    pub total_tokens: i64,
    pub total_elapsed_ms: i64,
}

#[derive(Debug, Serialize)]
pub struct ProviderStatsResponse {
    pub cli_type: String,
    pub provider_name: String,
    pub model_id: String,
    pub total_requests: i64,
    pub total_success: i64,
    pub total_tokens: i64,
    pub total_elapsed_ms: i64,
    pub success_rate: f64,
}

// ==================== Session 相关实体 (非数据库) ====================

// Project Info (从文件系统读取)
#[derive(Debug, Serialize)]
pub struct ProjectInfo {
    pub name: String,
    pub display_name: String,
    pub full_path: String,
    pub session_count: i64,
    pub total_size: i64,
    pub last_modified: f64,
}

// Session Info (从文件系统读取)
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub size: i64,
    pub mtime: f64,
    pub first_message: String,
    pub git_branch: String,
    pub summary: String,
}

#[derive(Debug, Serialize)]
pub struct PaginatedProjects {
    pub items: Vec<ProjectInfo>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Serialize)]
pub struct PaginatedSessions {
    pub items: Vec<SessionInfo>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

// Session Message (从会话文件解析)
#[derive(Debug, Serialize)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
    pub timestamp: Option<i64>,
}

// ==================== System Status (非数据库) ====================

#[derive(Debug, Serialize)]
pub struct SystemStatus {
    pub status: String,
    pub port: u16,
    pub uptime: i64,
    pub version: String,
}
