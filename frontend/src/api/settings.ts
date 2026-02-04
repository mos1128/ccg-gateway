import { invoke } from '@tauri-apps/api/core'
import type { AllSettings, GatewaySettingsUpdate, TimeoutSettingsUpdate, CliSettingsUpdate, SystemStatus, UseragentMapInput, UseragentMapResponse } from '@/types/models'

export const settingsApi = {
  getAll: async () => {
    const [gateway, timeouts, claudeCode, codex, gemini, status] = await Promise.all([
      invoke<{ debug_log: number }>('get_gateway_settings'),
      invoke<{ stream_first_byte_timeout: number; stream_idle_timeout: number; non_stream_timeout: number }>('get_timeout_settings'),
      invoke<{ cli_type: string; enabled: boolean; default_json_config: string }>('get_cli_settings', { cliType: 'claude_code' }),
      invoke<{ cli_type: string; enabled: boolean; default_json_config: string }>('get_cli_settings', { cliType: 'codex' }),
      invoke<{ cli_type: string; enabled: boolean; default_json_config: string }>('get_cli_settings', { cliType: 'gemini' }),
      invoke<SystemStatus>('get_system_status')
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
  getStatus: async () => {
    const data = await invoke<SystemStatus>('get_system_status')
    return { data }
  },
  getUseragentMaps: async () => {
    const data = await invoke<UseragentMapResponse[]>('get_useragent_maps')
    return { data }
  },
  updateUseragentMaps: async (maps: UseragentMapInput[]) => {
    const data = await invoke<UseragentMapResponse[]>('update_useragent_maps', { maps })
    return { data }
  }
}
