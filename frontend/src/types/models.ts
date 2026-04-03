// CLI Type
export type CliType = 'claude_code' | 'codex' | 'gemini'

// Provider types
export interface ModelMap {
  id?: number
  source_model: string
  target_model: string
  enabled: boolean
}

export interface ModelBlacklist {
  id?: number
  model_pattern: string
}

export interface Provider {
  id: number
  cli_type: CliType
  name: string
  base_url: string
  api_key: string
  enabled: boolean
  failure_threshold: number
  blacklist_minutes: number
  consecutive_failures: number
  blacklisted_until: number | null
  sort_order: number
  custom_useragent: string | null
  model_maps: ModelMap[]
  model_blacklist: ModelBlacklist[]
  is_blacklisted: boolean
}

export interface ProviderCreate {
  cli_type?: CliType
  name: string
  base_url: string
  api_key: string
  enabled?: boolean
  failure_threshold?: number
  blacklist_minutes?: number
  custom_useragent?: string
  model_maps?: ModelMap[]
  model_blacklist?: ModelBlacklist[]
}

export interface ProviderUpdate {
  name?: string
  base_url?: string
  api_key?: string
  enabled?: boolean
  failure_threshold?: number
  blacklist_minutes?: number
  custom_useragent?: string
  model_maps?: ModelMap[]
  model_blacklist?: ModelBlacklist[]
}

// Settings types
export interface GatewaySettings {
  debug_log: boolean
}

export interface TimeoutSettings {
  stream_first_byte_timeout: number
  stream_idle_timeout: number
  non_stream_timeout: number
}

export interface CliSettings {
  cli_type: string
  enabled: boolean
  default_json_config: string
  cli_mode: 'proxy' | 'direct'
  config_dir: string
  default_config_dir: string
  config_write_mode: 'overwrite' | 'merge'
}

export interface AllSettings {
  gateway: GatewaySettings
  timeouts: TimeoutSettings
  cli_settings: Record<string, CliSettings>
}

export interface GatewaySettingsUpdate {
  debug_log?: boolean
}

export interface TimeoutSettingsUpdate {
  stream_first_byte_timeout?: number
  stream_idle_timeout?: number
  non_stream_timeout?: number
}

export interface CliSettingsUpdate {
  enabled?: boolean
  default_json_config?: string
  config_dir?: string
  config_write_mode?: 'overwrite' | 'merge'
}

// Official Credential types
export interface OfficialCredential {
  id: number
  cli_type: CliType
  name: string
  credential_json: string
  sort_order: number
  is_active: boolean
  display_info: string
}

export interface OfficialCredentialCreate {
  cli_type: CliType
  name: string
  credential_json: string
}

export interface OfficialCredentialUpdate {
  name?: string
  credential_json?: string
}

export interface SystemStatus {
  status: 'running' | 'stopped'
  port: number
  uptime: number
  version: string
}

// MCP types
export interface CliFlags {
  claude_code: boolean
  codex: boolean
  gemini: boolean
}

export interface CliFlagItem {
  cli_type: CliType
  enabled: boolean
}

export interface Mcp {
  id: number
  name: string
  config_json: string
  enabled: boolean
  cli_flags: Record<string, boolean>
}

export interface McpCreate {
  name: string
  config_json: string
  enabled?: boolean
  cli_flags?: CliFlagItem[]
}

export interface McpUpdate {
  name?: string
  config_json?: string
  enabled?: boolean
  cli_flags?: CliFlagItem[]
}

// Prompt types
export interface Prompt {
  id: number
  name: string
  content: string
  enabled: boolean
  cli_flags: Record<string, boolean>
}

export interface PromptCreate {
  name: string
  content: string
  enabled?: boolean
  cli_flags?: CliFlagItem[]
}

export interface PromptUpdate {
  name?: string
  content?: string
  enabled?: boolean
  cli_flags?: CliFlagItem[]
}

// Skill Repo (仓库配置)
export interface SkillRepo {
  name: string    // 显示名称
  source: string  // 来源（URL/repo/local path）
}

export interface SkillRepoCreate {
  url: string
}

export interface DiscoverableSkill {
  key: string
  name: string
  description: string
  directory: string
  install_directory: string
  readme_url: string | null
  repo: SkillRepo
  is_favorited: boolean
  is_installed: boolean
}

export interface InstalledSkill {
  id: string
  name: string
  description: string | null
  directory: string
  repo: SkillRepo | null
  readme_url: string | null
  installed_at: number
  cli_flags: Record<string, boolean>
  exists_on_disk: boolean
  is_favorited: boolean
  can_favorite: boolean
  favorite_key: string | null
  market_display: string
}

export interface SkillFavoriteItem {
  key: string
  name: string
  description: string | null
  directory: string
  readme_url: string | null
  repo: SkillRepo
  is_installed: boolean
}

// Stats types
export interface DailyStats {
  usage_date: string
  provider_name: string
  cli_type: string
  request_count: number
  success_count: number
  failure_count: number
  prompt_tokens: number
  completion_tokens: number
}

export interface ProviderStats {
  provider_name: string
  cli_type: string
  total_requests: number
  total_success: number
  total_failure: number
  success_rate: number
  total_tokens: number
}

// Log types
export interface RequestLogListItem {
  id: number
  created_at: number
  cli_type: string
  provider_name: string
  model_id: string | null
  status_code: number | null
  elapsed_ms: number
  input_tokens: number
  output_tokens: number
  client_method: string
  client_path: string
  source_model: string | null
  target_model: string | null
}

export interface RequestLogDetail extends RequestLogListItem {
  client_headers: string
  client_body: string
  forward_url: string
  forward_headers: string
  forward_body: string
  provider_headers: string | null
  provider_body: string | null
  error_message: string | null
}

export interface RequestLogListResponse {
  items: RequestLogListItem[]
  total: number
  page: number
  page_size: number
}

export interface SystemLogItem {
  id: number
  created_at: number
  event_type: string
  message: string
}

export interface SystemLogListResponse {
  items: SystemLogItem[]
  total: number
  page: number
  page_size: number
}

// Plugin types
export interface InstalledPlugin {
  name: string
  version: string | null
  description: string | null
  marketplace_name: string | null
  is_enabled: boolean
}

export interface MarketplaceInfo {
  name: string
  marketplace_source: string | null
}

export interface MarketplacePlugin {
  name: string
  version: string | null
  description: string | null
  marketplace_name: string
}

export interface PluginItem {
  name: string
  version: string | null
  description: string | null
  marketplace_name: string
  is_installed: boolean
  is_enabled: boolean | null
  is_favorited: boolean
}

export interface PluginFavoriteItem {
  plugin_id: string
  plugin_name: string
  marketplace_name: string
  is_installed: boolean
  marketplace_source: string | null
}

// 插件操作返回结果
export interface PluginActionResult {
  cli_output: string
  plugins: PluginItem[]
}

// 市场操作返回结果
export interface MarketplaceActionResult {
  cli_output: string
  plugins: PluginItem[]
  marketplaces: MarketplaceInfo[]
}
