import { invoke } from '@tauri-apps/api/core'
import type { OfficialCredential, OfficialCredentialCreate, OfficialCredentialUpdate } from '@/types/models'

export const credentialsApi = {
  list: async (cliType: string): Promise<{ data: OfficialCredential[] }> => {
    const data = await invoke<OfficialCredential[]>('get_credentials', { cliType })
    return { data }
  },

  get: async (id: number): Promise<{ data: OfficialCredential }> => {
    const data = await invoke<OfficialCredential>('get_credential', { id })
    return { data }
  },

  create: async (data: OfficialCredentialCreate): Promise<{ data: OfficialCredential }> => {
    const result = await invoke<OfficialCredential>('create_credential', { input: data })
    return { data: result }
  },

  update: async (id: number, data: OfficialCredentialUpdate): Promise<{ data: OfficialCredential }> => {
    const result = await invoke<OfficialCredential>('update_credential', { id, input: data })
    return { data: result }
  },

  delete: async (id: number) => {
    await invoke('delete_credential', { id })
    return { data: null }
  },

  reorder: async (ids: number[]) => {
    await invoke('reorder_credentials', { ids })
    return { data: null }
  },

  readCliCredential: async (cliType: string): Promise<{ data: string }> => {
    const data = await invoke<string>('read_cli_credential', { cliType })
    return { data }
  },
}
