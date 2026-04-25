import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { Provider, ProviderCreate, ProviderUpdate, TestProviderResult } from '@/types/models'

export const providersApi = {
  list: async (cliType?: string, profile?: string): Promise<{ data: Provider[] }> => {
    const args: Record<string, string> = {}
    if (cliType) args.cliType = cliType
    if (profile) args.profile = profile
    const data = await invoke<Provider[]>('get_providers', args)
    return { data }
  },
  get: async (id: number): Promise<{ data: Provider }> => {
    const data = await invoke<Provider>('get_provider', { id })
    return { data }
  },
  create: async (data: ProviderCreate): Promise<{ data: Provider }> => {
    const result = await invoke<Provider>('create_provider', { input: data })
    return { data: result }
  },
  update: async (id: number, data: ProviderUpdate): Promise<{ data: Provider }> => {
    const result = await invoke<Provider>('update_provider', { id, input: data })
    return { data: result }
  },
  delete: async (id: number) => {
    await invoke('delete_provider', { id })
    return { data: null }
  },
  reorder: async (ids: number[]) => {
    await invoke('reorder_providers', { ids })
    return { data: null }
  },
  resetFailures: async (id: number) => {
    await invoke('reset_provider_failures', { id })
    return { data: null }
  },
  unblacklist: async (id: number) => {
    await invoke('reset_provider_failures', { id })
    return { data: null }
  },
  startTestModels: async (modelName: string, providerIds: number[]) => {
    await invoke('test_provider_models', {
      input: { model_name: modelName, provider_ids: providerIds }
    })
    return { data: null }
  },
  listenTestResults: (callback: (result: TestProviderResult) => void): Promise<UnlistenFn> => {
    return listen<TestProviderResult>('provider-test-result', (event) => {
      callback(event.payload)
    })
  }
}
