import { invoke } from '@tauri-apps/api/core'
import { transformCliFlags } from '@/utils/cliFlags'
import type { CliFlagItem, CliType, Mcp, McpCreate, McpUpdate } from '@/types/models'

// 后端返回的 cli_flags 格式
type McpCliFlagBackend = CliFlagItem
type McpBackend = Omit<Mcp, 'cli_flags'> & { cli_flags: McpCliFlagBackend[] }

function transformMcp(mcp: McpBackend): Mcp {
  return {
    ...mcp,
    cli_flags: transformCliFlags(mcp.cli_flags)
  }
}

export const mcpApi = {
  list: async (): Promise<{ data: Mcp[] }> => {
    const data = await invoke<McpBackend[]>('get_mcps')
    return { data: data.map(transformMcp) }
  },
  get: async (id: number): Promise<{ data: Mcp }> => {
    const data = await invoke<McpBackend>('get_mcp', { id })
    return { data: transformMcp(data) }
  },
  create: async (data: McpCreate): Promise<{ data: Mcp }> => {
    const result = await invoke<McpBackend>('create_mcp', { input: data })
    return { data: transformMcp(result) }
  },
  update: async (id: number, data: McpUpdate): Promise<{ data: Mcp }> => {
    const result = await invoke<McpBackend>('update_mcp', { id, input: data })
    return { data: transformMcp(result) }
  },
  toggleCli: async (id: number, cliType: CliType, enabled: boolean): Promise<{ data: Mcp }> => {
    const result = await invoke<McpBackend>('toggle_mcp_cli', { id, cliType, enabled })
    return { data: transformMcp(result) }
  },
  delete: async (id: number) => {
    await invoke('delete_mcp', { id })
    return { data: null }
  }
}
