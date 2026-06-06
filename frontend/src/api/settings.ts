import { invoke } from '@tauri-apps/api/core'
import { CLI_TYPES } from '@/types/models'
import type {
  AllSettings,
  CliType,
  GatewaySettingsUpdate,
  TimeoutSettingsUpdate,
  CliSettingsUpdate,
  CliSettings,
  GatewaySettingsRaw,
  SystemStatus,
  ProviderProfile,
  CliProfileSettingsStatus,
  CliMode
} from '@/types/models'

export const settingsApi = {
  getAll: async () => {
    const [gateway, timeouts, cliSettingsList, status] = await Promise.all([
      invoke<GatewaySettingsRaw>('get_gateway_settings'),
      invoke<{ stream_first_byte_timeout: number; stream_idle_timeout: number; non_stream_timeout: number }>('get_timeout_settings'),
      Promise.all(CLI_TYPES.map((cliType) => invoke<CliSettings>('get_cli_settings', { cliType }))),
      invoke<SystemStatus>('get_system_status'),
    ])
    const cliSettings = {} as Record<CliType, CliSettings>
    for (const [index, cliType] of CLI_TYPES.entries()) {
      cliSettings[cliType] = cliSettingsList[index]
    }

    return {
      data: {
        gateway: {
          debug_log: !!gateway.debug_log,
          log_detail_mode: gateway.log_detail_mode as 'full' | 'failure_only',
          launch_on_startup: !!gateway.launch_on_startup,
          silent_startup: !!gateway.silent_startup,
          minimize_to_tray_on_close: !!gateway.minimize_to_tray_on_close,
        },
        timeouts,
        cli_settings: cliSettings,
        status
      } as AllSettings
    }
  },
  updateGateway: async (data: GatewaySettingsUpdate) => {
    await invoke('update_gateway_settings', {
      debugLog: data.debug_log,
      logDetailMode: data.log_detail_mode,
      launchOnStartup: data.launch_on_startup,
      silentStartup: data.silent_startup,
      minimizeToTrayOnClose: data.minimize_to_tray_on_close,
    })
    return { data: null }
  },
  updateTimeouts: async (data: TimeoutSettingsUpdate) => {
    await invoke('update_timeout_settings', { input: data })
    return { data: null }
  },
  updateCli: async (cliType: CliType, data: CliSettingsUpdate) => {
    await invoke('update_cli_settings', { cliType, input: data })
    return { data: null }
  },
  setCliMode: async (cliType: CliType, mode: CliMode) => {
    await invoke('set_cli_mode', { cliType, mode })
    return { data: null }
  },
  setDashboardCliMode: async (cliType: CliType, mode: CliMode) => {
    await invoke('set_dashboard_cli_mode', { cliType, mode })
    return { data: null }
  },
  getClaudeProfileSettingsStatus: async (profile: ProviderProfile) => {
    const data = await invoke<CliProfileSettingsStatus>('get_claude_profile_settings_status', { profile })
    return { data }
  },
  ensureClaudeProfileSettings: async (profile: ProviderProfile) => {
    const data = await invoke<CliProfileSettingsStatus>('ensure_claude_profile_settings', { profile })
    return { data }
  },
  getCodexProfileSettingsStatus: async (profile: ProviderProfile) => {
    const data = await invoke<CliProfileSettingsStatus>('get_codex_profile_settings_status', { profile })
    return { data }
  },
  ensureCodexProfileSettings: async (profile: ProviderProfile) => {
    const data = await invoke<CliProfileSettingsStatus>('ensure_codex_profile_settings', { profile })
    return { data }
  },
  getStatus: async () => {
    const data = await invoke<SystemStatus>('get_system_status')
    return { data }
  }
}
