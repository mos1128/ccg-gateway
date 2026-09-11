// Agent / Protocol
export type CliType = string
export type Protocol = 'anthropic_messages' | 'openai_chat' | 'openai_responses' | 'gemini_generate_content'
export const PROTOCOL_LABELS: Record<Protocol, string> = {
  anthropic_messages: 'Anthropic Messages',
  openai_chat: 'OpenAI Chat Completions',
  openai_responses: 'OpenAI Responses',
  gemini_generate_content: 'Gemini GenerateContent',
}
// 可互相转换的协议，Gemini 不参与转换
export const CONVERTIBLE_PROTOCOLS: Protocol[] = ['anthropic_messages', 'openai_chat', 'openai_responses']
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

export type ConfigFormat = 'json' | 'jsonc' | 'toml' | 'yaml' | 'env'

export interface ProviderConfigOperation {
  id: string
  op: 'set' | 'remove'
  file: string
  format: ConfigFormat
  private?: boolean
  path: string[]
  value?: unknown
}

export interface ProviderConfigFeature extends ToggleFeature {
  operations: ProviderConfigOperation[]
}

export interface GlobalPresetFeature extends ToggleFeature {
  file?: string | null
  format?: ConfigFormat | null
}

export interface AgentIconPath {
  d: string
  fill?: string | null
  opacity?: number
  fill_rule?: 'nonzero' | 'evenodd'
  clip_rule?: 'nonzero' | 'evenodd'
}

export interface AgentIconGradientStop {
  offset: number
  color: string
}

export interface AgentIcon {
  view_box: string
  linear_gradient?: AgentIconGradientStop[] | null
  paths: AgentIconPath[]
}

export interface CredentialSource {
  file_id: string
  path?: string[]
}

export interface OfficialLoginOperation {
  id: string
  op: 'replace_file' | 'set_field'
  file: string
  format?: ConfigFormat | null
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
  format?: 'json' | 'toml' | 'yaml' | null
  adapter?: 'opencode' | 'dsh' | null
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
  remark?: string | null
  icon?: AgentIcon | null
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

export interface ProviderModel {
  id: number
  provider_id: number
  model_name: string
  source: 'auto' | 'manual' | string
  enabled: boolean | number
  first_seen_at: number
  last_seen_at: number
}

export interface ProviderModelSyncState {
  provider_id: number
  last_attempt_at: number | null
  last_success_at: number | null
  last_error: string | null
  model_count: number
  updated_at: number
}

export interface ProviderModelsResponse {
  provider_id: number
  models: ProviderModel[]
  sync_state: ProviderModelSyncState | null
}

export interface PriceSyncState {
  id: number
  last_attempt_at: number | null
  last_success_at: number | null
  last_error: string | null
  model_count: number
  updated_at: number
}

export interface ModelPriceTier {
  threshold_tokens: number
  input_price_per_m: number
  output_price_per_m: number
  cache_read_price_per_m: number
  cache_creation_price_per_m: number
}

export interface ModelPriceCatalogEntry {
  model_key: string
  model_name: string
  source_provider: string | null
  input_price_per_m: number
  output_price_per_m: number
  cache_read_price_per_m: number
  cache_creation_price_per_m: number
  tiers: string | null
  fetched_at: number
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
  /** 单个服务商在一轮里连续尝试的上限，达到后切下一个服务商。 */
  retry_limit: number
  blacklist_minutes: number
  consecutive_failures: number
  blacklisted_until: number | null
  sort_order: number
  custom_useragent: string | null
  price_multiplier: number
  /** 转成 Anthropic 协议时源请求没给 max_tokens 的兜底值，按上游模型的上限设。 */
  translate_max_tokens: number
  model_maps: ModelMap[]
  model_blacklist: ModelBlacklist[]
  is_blacklisted: boolean
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
  retry_limit?: number
  blacklist_minutes?: number
  custom_useragent?: string
  price_multiplier?: number
  translate_max_tokens?: number
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
  retry_limit?: number
  blacklist_minutes?: number
  custom_useragent?: string
  price_multiplier?: number
  translate_max_tokens?: number
  model_maps?: ModelMap[]
  model_blacklist?: ModelBlacklist[]
}

/** 后端在熔断/恢复时推送的增量状态，用于服务商页免刷新更新。 */
export interface ProviderHealthEvent {
  provider_id: number
  consecutive_failures: number
  blacklisted_until: number | null
  is_blacklisted: boolean
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
  gateway_host: string
  gateway_port: number
}

export interface GatewaySettingsRaw {
  debug_log: number
  log_detail_mode: string
  launch_on_startup: number
  silent_startup: number
  minimize_to_tray_on_close: number
  gateway_host: string
  gateway_port: number
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
  gateway_host?: string
  gateway_port?: number
}

export interface BootstrapSettingsUpdate {
  data_dir?: string
  log_file?: boolean
  log_level?: string
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
  files: Record<string, { format: ConfigFormat; content: unknown }>
}

export interface CredentialFileDefinition {
  key: string
  name: string
  format: ConfigFormat
  placeholder?: string
  compact?: boolean
}

export interface SystemStatus {
  status: 'starting' | 'running' | 'error'
  host: string
  port: number
  gateway_url: string
  error_message: string | null
  host_env_override: boolean
  port_env_override: boolean
  data_dir: string
  default_data_dir: string
  data_dir_env_override: boolean
  log_file: boolean
  log_file_env_override: boolean
  log_level: string
  log_level_env_override: boolean
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
/** 实际用于计费的单价，后端读取日志时算出来，不落库。价格都已乘过服务商倍率。 */
export interface CostBreakdown {
  matched: boolean
  multiplier: number
  input_price_per_m: number
  output_price_per_m: number
  cache_read_price_per_m: number
  cache_creation_price_per_m: number
  /** 命中的分层档阈值，走基准价时为 null。 */
  tier_threshold_tokens: number | null
  /** 价格来源（models.dev 的厂商渠道 id），未命中目录时为 null。 */
  source: string | null
}

export interface RequestLogListItem {
  id: number
  created_at: number
  finished_at: number | null
  cli_type: CliType
  protocol: Protocol | null
  /** 真的走了协议转换时才有值：请求最终发给上游用的协议。 */
  upstream_protocol: Protocol | null
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
  cost: CostBreakdown
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
export interface PluginItem {
  profile: string
  name: string
  version: string | null
  description: string | null
}

// 插件操作返回结果
export interface PluginActionResult {
  cli_output: string
}
