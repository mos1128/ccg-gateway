// CLI Type
export type CliType = 'claude_code' | 'codex' | 'gemini'

// Provider types
export interface ModelMap {
  id?: number
  source_model: string
  target_model: string
  enabled: boolean
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
  model_maps: ModelMap[]
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
  model_maps?: ModelMap[]
}

export interface ProviderUpdate {
  name?: string
  base_url?: string
  api_key?: string
  enabled?: boolean
  failure_threshold?: number
  blacklist_minutes?: number
  model_maps?: ModelMap[]
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
  cli_flags?: CliFlags
}

export interface McpUpdate {
  name?: string
  config_json?: string
  enabled?: boolean
  cli_flags?: CliFlags
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
  cli_flags?: CliFlags
}

export interface PromptUpdate {
  name?: string
  content?: string
  enabled?: boolean
  cli_flags?: CliFlags
}

// Stats types
export interface DailyStats {
  usage_date: string
  provider_id: number
  provider_name: string
  cli_type: string
  request_count: number
  success_count: number
  failure_count: number
  prompt_tokens: number
  completion_tokens: number
}

export interface ProviderStats {
  provider_id: number
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
  success: boolean
  status_code: number | null
  elapsed_ms: number
  client_method: string
  client_path: string
}

export interface RequestLogDetail extends RequestLogListItem {
  client_headers: string
  client_body: string
  forward_url: string
  forward_headers: string
  forward_body: string
  provider_status: number | null
  provider_headers: string | null
  provider_body: string | null
  response_status: number | null
  response_headers: string | null
  response_body: string | null
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
  level: string
  event_type: string
  provider_name: string | null
  message: string
  details: string | null
}

export interface SystemLogListResponse {
  items: SystemLogItem[]
  total: number
  page: number
  page_size: number
}
