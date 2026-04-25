import { invoke } from '@tauri-apps/api/core'
import type {
  AllSettings,
  GatewaySettingsUpdate,
  TimeoutSettingsUpdate,
  CliSettingsUpdate,
  CliSettings,
  SystemStatus,
  ProviderProfile,
  ClaudeProfileSettingsStatus
} from '@/types/models'

export const settingsApi = {
  getAll: async () => {
    const [gateway, timeouts, claudeCode, codex, gemini, status] = await Promise.all([
      invoke<{ debug_log: number }>('get_gateway_settings'),
      invoke<{ stream_first_byte_timeout: number; stream_idle_timeout: number; non_stream_timeout: number }>('get_timeout_settings'),
      invoke<CliSettings>('get_cli_settings', { cliType: 'claude_code' }),
      invoke<CliSettings>('get_cli_settings', { cliType: 'codex' }),
      invoke<CliSettings>('get_cli_settings', { cliType: 'gemini' }),
      invoke<SystemStatus>('get_system_status'),
    ])
    return {
      data: {
        gateway: { debug_log: !!gateway.debug_log },
        timeouts,
        cli_settings: {
          claude_code: claudeCode,
          codex: codex,
          gemini: gemini
        },
        status
      } as AllSettings
    }
  },
  updateGateway: async (data: GatewaySettingsUpdate) => {
    await invoke('update_gateway_settings', { debugLog: data.debug_log })
    return { data: null }
  },
  updateTimeouts: async (data: TimeoutSettingsUpdate) => {
    await invoke('update_timeout_settings', { input: data })
    return { data: null }
  },
  updateCli: async (cliType: string, data: CliSettingsUpdate) => {
    await invoke('update_cli_settings', { cliType, input: data })
    return { data: null }
  },
  setCliMode: async (cliType: string, mode: 'proxy' | 'direct') => {
    await invoke('set_cli_mode', { cliType, mode })
    return { data: null }
  },
  getClaudeProfileSettingsStatus: async (profile: ProviderProfile) => {
    const data = await invoke<ClaudeProfileSettingsStatus>('get_claude_profile_settings_status', { profile })
    return { data }
  },
  ensureClaudeProfileSettings: async (profile: ProviderProfile) => {
    const data = await invoke<ClaudeProfileSettingsStatus>('ensure_claude_profile_settings', { profile })
    return { data }
  },
  getStatus: async () => {
    const data = await invoke<SystemStatus>('get_system_status')
    return { data }
  }
}
