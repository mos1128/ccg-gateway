import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from './tauri-bridge'
import type { Provider, ProviderCreate, ProviderProfileItem, ProviderUpdate, TestProviderResult } from '@/types/models'

export const providersApi = {
  listProfiles: async (cliType: string): Promise<{ data: ProviderProfileItem[] }> => {
    const data = await invoke<ProviderProfileItem[]>('get_provider_profiles', { cliType })
    return { data }
  },
  createProfile: async (cliType: string, name: string): Promise<{ data: ProviderProfileItem }> => {
    const data = await invoke<ProviderProfileItem>('create_provider_profile', { input: { cli_type: cliType, name } })
    return { data }
  },
  renameProfile: async (cliType: string, profile: string, name: string): Promise<{ data: ProviderProfileItem }> => {
    const data = await invoke<ProviderProfileItem>('rename_provider_profile', { profile, input: { cli_type: cliType, name } })
    return { data }
  },
  deleteProfile: async (cliType: string, profile: string) => {
    await invoke('delete_provider_profile', { cliType, profile })
    return { data: null }
  },
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
  startTestModels: async (modelName: string, providerIds: number[], testText: string) => {
    await invoke('test_provider_models', {
      input: { model_name: modelName, provider_ids: providerIds, test_text: testText }
    })
    return { data: null }
  },
  listenTestResults: (callback: (result: TestProviderResult) => void): Promise<UnlistenFn> => {
    return listen<TestProviderResult>('provider-test-result', (event) => {
      callback(event.payload)
    })
  }
}
