import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  RequestLogListResponse,
  RequestLogDetail,
  RequestLogListItem,
  SystemLogListResponse,
  GatewaySettings,
  GatewaySettingsUpdate
} from '@/types/models'

export interface RequestLogQuery {
  page?: number
  page_size?: number
  cli_type?: string
  provider_name?: string
}

export interface SystemLogQuery {
  page?: number
  page_size?: number
  event_type?: string
}

export const logsApi = {
  getSettings: async () => {
    const data = await invoke<{ debug_log: number }>('get_gateway_settings')
    return { data: { debug_log: !!data.debug_log } as GatewaySettings }
  },
  updateSettings: async (data: GatewaySettingsUpdate) => {
    await invoke('update_gateway_settings', { debugLog: data.debug_log })
    return { data: null }
  },

  listRequestLogs: async (params: RequestLogQuery) => {
    const data = await invoke<RequestLogListResponse>('get_request_logs', {
      page: params.page,
      pageSize: params.page_size,
      cliType: params.cli_type,
      providerName: params.provider_name
    })
    return { data }
  },
  getRequestLog: async (id: number) => {
    const data = await invoke<RequestLogDetail>('get_request_log_detail', { id })
    return { data }
  },
  clearRequestLogs: async (before_timestamp?: number) => {
    await invoke('clear_request_logs')
    return { data: null }
  },
  listenRequestLogs: (callback: (log: RequestLogListItem) => void): Promise<UnlistenFn> => {
    return listen<RequestLogListItem>('request-log-new', (event) => {
      callback(event.payload)
    })
  },

  listSystemLogs: async (params: SystemLogQuery) => {
    const data = await invoke<SystemLogListResponse>('get_system_logs', {
      page: params.page,
      pageSize: params.page_size,
      eventType: params.event_type
    })
    return { data }
  },
  clearSystemLogs: async (before_timestamp?: number) => {
    await invoke('clear_system_logs')
    return { data: null }
  }
}
