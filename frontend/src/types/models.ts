// Agent / Protocol
export type CliType = string
export type Protocol = 'anthropic_messages' | 'openai_chat' | 'openai_responses' | 'gemini_generate_content'
export const PROTOCOL_LABELS: Record<Protocol, string> = {
  anthropic_messages: 'Anthropic Messages',
  openai_chat: 'OpenAI Chat Completions',
  openai_responses: 'OpenAI Responses',
  gemini_generate_content: 'Gemini GenerateContent',
}
export type AgentFeatureName = 'provider_config' | 'global_preset' | 'profiles' | 'official_login' | 'model_mapping' | 'token_usage' | 'skills' | 'mcp' | 'sessions' | 'plugins' | 'prompts'

export interface ToggleFeature {
  enabled: boolean
}

export interface AdapterFeature extends ToggleFeature {
  adapter?: string | null
}

export interface ProfileLaunch {
  default: string[]
  non_default: string[]
}

export interface ProfileFeature extends ToggleFeature {
  profile_file?: string | null
  operations: ProviderConfigOperation[]
  launch?: ProfileLaunch | null
}

export type ConfigFormat = 'json' | 'jsonc' | 'toml' | 'env'

export interface ProviderConfigOperation {
  id: string
  op: 'set' | 'remove'
  file: string
  format: ConfigFormat
  path: string[]
  value?: unknown
}

export interface ProviderConfigFeature extends ToggleFeature {
  operations: ProviderConfigOperation[]
}

export interface GlobalPresetFeature extends ToggleFeature {
  file?: string | null
  format?: 'json' | 'toml' | null
}

export interface CredentialSource {
  file_id: string
  path?: string[]
}

export interface OfficialLoginOperation {
  id: string
  op: 'replace_file' | 'set_field'
  file: string
  format?: 'json' | null
  path?: string[]
  content_from?: CredentialSource | null
  value?: unknown
  value_from?: CredentialSource | null
}

export interface OfficialLoginFeature extends ToggleFeature {
  operations?: OfficialLoginOperation[]
}

export interface McpFeature extends ToggleFeature {
  file?: string | null
  format?: 'json' | 'toml' | null
  servers_path?: string[]
}

export interface FileFeature extends ToggleFeature {
  file?: string | null
}

export interface SkillFeature extends ToggleFeature {
  directory?: string | null
}

export interface AgentFeatures {
  provider_config: ProviderConfigFeature
  global_preset: GlobalPresetFeature
  profiles: ProfileFeature
  official_login: OfficialLoginFeature
  model_mapping: ToggleFeature
  token_usage: ToggleFeature
  skills: SkillFeature
  mcp: McpFeature
  sessions: AdapterFeature
  plugins: AdapterFeature
  prompts: FileFeature
}

export interface AgentInfo {
  schema_version: number
  id: CliType
  name: string
  config_dir: string
  user_agent: string[]
  protocols: Protocol[]
  features: AgentFeatures
}

export interface AgentDefinitionLoadError {
  source: string
  message: string
}

export interface AgentDiagnostic {
  id: number
  kind: string
  key: string
  payload_json: string
  first_seen: number
  last_seen: number
  occurrence_count: number
}
export type ProviderProfile = string
export type CliMode = 'proxy_route' | 'provider_direct' | 'official_direct' | 'disabled'

export interface ProviderProfileItem {
  cli_type: CliType
  name: ProviderProfile
  label: string
  is_default: boolean
  sort_order: number
}

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
  profile: ProviderProfile
  protocol: Protocol
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
  input_price_per_m: number
  output_price_per_m: number
  cache_read_price_per_m: number
  cache_creation_price_per_m: number
  model_maps: ModelMap[]
  model_blacklist: ModelBlacklist[]
  is_blacklisted: boolean
  is_direct_active: boolean
}

export interface ProviderCreate {
  cli_type: CliType
  profile?: ProviderProfile
  protocol?: Protocol
  name: string
  base_url: string
  api_key: string
  enabled?: boolean
  failure_threshold?: number
  blacklist_minutes?: number
  custom_useragent?: string
  input_price_per_m?: number
  output_price_per_m?: number
  cache_read_price_per_m?: number
  cache_creation_price_per_m?: number
  model_maps?: ModelMap[]
  model_blacklist?: ModelBlacklist[]
}

export interface ProviderUpdate {
  profile?: ProviderProfile
  protocol?: Protocol
  name?: string
  base_url?: string
  api_key?: string
  enabled?: boolean
  failure_threshold?: number
  blacklist_minutes?: number
  custom_useragent?: string
  input_price_per_m?: number
  output_price_per_m?: number
  cache_read_price_per_m?: number
  cache_creation_price_per_m?: number
  model_maps?: ModelMap[]
  model_blacklist?: ModelBlacklist[]
}

// Model Detection types
export interface TestProviderResult {
  provider_id: number
  provider_name: string
  actual_model: string
  status_code: number | null
  elapsed_ms: number
  response_text: string
  request_url: string
  request_headers: string
  request_body: string
  response_headers: string
  response_body: string
}

// Scheduled task types
export type ScheduledTaskType = 'provider_keepalive'
export type ScheduledTaskStatus = 'pending' | 'running' | 'success' | 'partial_failed' | 'failed' | 'retrying' | 'skipped'
export type ScheduledTaskTrigger = 'scheduled' | 'manual'
export type ScheduledTaskScheduleType = 'interval' | 'daily'

export interface ProviderKeepalivePayload {
  target_mode: 'all' | 'selected'
  cli_type?: CliType
  profile?: ProviderProfile
  provider_ids?: number[]
  model_name: string
  test_text?: string
}

export interface ScheduledTask {
  id: number
  name: string
  task_type: ScheduledTaskType
  enabled: boolean
  schedule_type: ScheduledTaskScheduleType
  schedule_expr: string
  payload_json: string
  retry_limit: number
  retry_interval_minutes: number
  retry_count: number
  last_run_at: number | null
  next_run_at: number
  last_status: ScheduledTaskStatus
  last_error: string | null
  created_at: number
  updated_at: number
}

export interface ScheduledTaskCreate {
  name: string
  task_type: ScheduledTaskType
  enabled?: boolean
  schedule_type: ScheduledTaskScheduleType
  schedule_expr: string
  payload_json: string
  retry_limit?: number
  retry_interval_minutes?: number
}

export interface ScheduledTaskUpdate {
  name?: string
  enabled?: boolean
  schedule_type?: ScheduledTaskScheduleType
  schedule_expr?: string
  payload_json?: string
  retry_limit?: number
  retry_interval_minutes?: number
}

export interface ScheduledTaskRun {
  id: number
  task_id: number
  task_name: string
  task_type: ScheduledTaskType
  trigger_type: ScheduledTaskTrigger
  status: ScheduledTaskStatus
  started_at: number
  finished_at: number | null
  elapsed_ms: number
  total_count: number
  success_count: number
  failure_count: number
  skipped_count: number
  error_message: string | null
}

export interface ScheduledTaskRunItem {
  id: number
  run_id: number
  provider_id: number | null
  provider_name: string
  model_name: string
  status: 'success' | 'failed' | 'skipped'
  status_code: number | null
  elapsed_ms: number
  error_message: string | null
  created_at: number
}

export interface ScheduledTaskRunListResponse {
  items: ScheduledTaskRun[]
  total: number
}

// Settings types
export interface GatewaySettings {
  debug_log: boolean
  log_detail_mode: 'full' | 'failure_only'
  launch_on_startup: boolean
  silent_startup: boolean
  minimize_to_tray_on_close: boolean
}

export interface GatewaySettingsRaw {
  debug_log: number
  log_detail_mode: string
  launch_on_startup: number
  silent_startup: number
  minimize_to_tray_on_close: number
}

export interface TimeoutSettings {
  stream_first_byte_timeout: number
  stream_idle_timeout: number
  non_stream_timeout: number
}

export interface CliSettings {
  cli_type: CliType
  enabled: boolean
  default_json_config: string
  cli_mode: CliMode
  config_dir: string
  default_config_dir: string
  config_write_mode: 'overwrite' | 'merge'
  last_official_credential_id: number | null
}

export interface CliProfileSettingsStatus {
  profile: ProviderProfile
  filename: string
  path: string
  launch_command: string
  exists: boolean
  uses_gateway: boolean
}

export interface AllSettings {
  gateway: GatewaySettings
  timeouts: TimeoutSettings
  cli_settings: Record<string, CliSettings>
  status: SystemStatus
}

export interface GatewaySettingsUpdate {
  debug_log?: boolean
  log_detail_mode?: 'full' | 'failure_only'
  launch_on_startup?: boolean
  silent_startup?: boolean
  minimize_to_tray_on_close?: boolean
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
  is_written: boolean
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

export interface OfficialCredentialPayload {
  schema_version: 1
  files: Record<string, { format: 'json'; content: unknown }>
}

export interface CredentialFileDefinition {
  key: string
  name: string
  placeholder?: string
  compact?: boolean
}

export interface SystemStatus {
  status: 'running' | 'stopped'
  host: string
  port: number
  gateway_url: string
  uptime: number
  version: string
}

// MCP types
export type CliFlags = Record<string, boolean>

export interface CliFlagItem {
  cli_type: CliType
  enabled: boolean
}

export interface Mcp {
  id: number
  name: string
  config_json: string
  enabled: boolean
  cli_flags: CliFlags
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
  cli_flags: CliFlags
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
  cli_flags: CliFlags
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
export interface ProviderStats {
  provider_name: string
  total_requests: number
  total_success: number
  success_rate: number
  total_tokens: number
  total_cache_read_tokens: number
  total_cache_creation_tokens: number
  total_elapsed_ms: number
  total_cost: number
}

export interface AdvancedStatsRow {
  date: string
  cli_type: CliType
  provider_name: string
  model_id: string
  total_requests: number
  total_success: number
  total_tokens: number
  total_input_tokens: number
  total_output_tokens: number
  total_cache_read_tokens: number
  total_cache_creation_tokens: number
  total_cost: number
}

// Log types
export interface RequestLogListItem {
  id: number
  created_at: number
  finished_at: number | null
  cli_type: CliType
  protocol: Protocol | null
  provider_id: number | null
  profile: string | null
  provider_name: string
  model_id: string | null
  status_code: number | null
  elapsed_ms: number
  first_byte_ms: number
  input_tokens: number
  cache_read_input_tokens: number
  cache_creation_input_tokens: number
  output_tokens: number
  total_cost: number
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
  is_installed: boolean | null
  is_enabled: boolean | null
  is_favorited: boolean | null
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
}

// 市场操作返回结果
export interface MarketplaceActionResult {
  cli_output: string
}
