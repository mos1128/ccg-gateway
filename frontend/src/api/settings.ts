import { invoke } from './tauri-bridge'
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
  CliMode,
  AgentInfo,
} from '@/types/models'

export const settingsApi = {
  getAll: async () => {
    const agents = await invoke<AgentInfo[]>('get_agents')
    const [gateway, timeouts, cliSettingsList, status] = await Promise.all([
      invoke<GatewaySettingsRaw>('get_gateway_settings'),
      invoke<{ stream_first_byte_timeout: number; stream_idle_timeout: number; non_stream_timeout: number }>('get_timeout_settings'),
      Promise.all(agents.map((agent) => invoke<CliSettings>('get_cli_settings', { cliType: agent.id }))),
      invoke<SystemStatus>('get_system_status'),
    ])
    const cliSettings: Record<string, CliSettings> = {}
    for (const [index, agent] of agents.entries()) {
      cliSettings[agent.id] = cliSettingsList[index]
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
  getProfileSettingsStatus: async (cliType: CliType, profile: ProviderProfile) => {
    const data = await invoke<CliProfileSettingsStatus>('get_profile_settings_status', { cliType, profile })
    return { data }
  },
  ensureProfileSettings: async (cliType: CliType, profile: ProviderProfile) => {
    const data = await invoke<CliProfileSettingsStatus>('ensure_profile_settings', { cliType, profile })
    return { data }
  },
  getStatus: async () => {
    const data = await invoke<SystemStatus>('get_system_status')
    return { data }
  }
}
