import { invoke } from '@tauri-apps/api/core'
import type { ProviderStats, AdvancedStatsRow } from '@/types/models'

export const statsApi = {
  clearStatsData: async (): Promise<{ data: null }> => {
    await invoke('clear_stats_data')
    return { data: null }
  },
  getProviders: async (params?: { start_date?: string; end_date?: string }): Promise<{ data: ProviderStats[] }> => {
    const data = await invoke<ProviderStats[]>('get_provider_stats', {
      startDate: params?.start_date,
      endDate: params?.end_date
    })
    return { data }
  },
  getAdvanced: async (params?: { start_date?: string; end_date?: string; cli_type?: string; provider_name?: string; model_id?: string }): Promise<{ data: AdvancedStatsRow[] }> => {
    const data = await invoke<AdvancedStatsRow[]>('get_advanced_stats', {
      startDate: params?.start_date,
      endDate: params?.end_date,
      cliType: params?.cli_type,
      providerName: params?.provider_name,
      modelId: params?.model_id
    })
    return { data }
  }
}
