import { invoke } from '@tauri-apps/api/core'
import { transformCliFlags } from '@/utils/cliFlags'
import type { CliFlagItem, CliType, Prompt, PromptCreate, PromptUpdate } from '@/types/models'

// 后端返回的 cli_flags 格式
type PromptCliFlagBackend = CliFlagItem
type PromptBackend = Omit<Prompt, 'cli_flags'> & { cli_flags: PromptCliFlagBackend[] }

function transformPrompt(prompt: PromptBackend): Prompt {
  return {
    ...prompt,
    cli_flags: transformCliFlags(prompt.cli_flags)
  }
}

export const promptsApi = {
  list: async (): Promise<{ data: Prompt[] }> => {
    const data = await invoke<PromptBackend[]>('get_prompts')
    return { data: data.map(transformPrompt) }
  },
  get: async (id: number): Promise<{ data: Prompt }> => {
    const data = await invoke<PromptBackend>('get_prompt', { id })
    return { data: transformPrompt(data) }
  },
  create: async (data: PromptCreate): Promise<{ data: Prompt }> => {
    const result = await invoke<PromptBackend>('create_prompt', { input: data })
    return { data: transformPrompt(result) }
  },
  update: async (id: number, data: PromptUpdate): Promise<{ data: Prompt }> => {
    const result = await invoke<PromptBackend>('update_prompt', { id, input: data })
    return { data: transformPrompt(result) }
  },
  toggleCli: async (id: number, cliType: CliType, enabled: boolean): Promise<{ data: Prompt }> => {
    const result = await invoke<PromptBackend>('toggle_prompt_cli', { id, cliType, enabled })
    return { data: transformPrompt(result) }
  },
  delete: async (id: number) => {
    await invoke('delete_prompt', { id })
    return { data: null }
  }
}
