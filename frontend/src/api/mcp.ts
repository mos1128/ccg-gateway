import { invoke } from '@tauri-apps/api/core'
import type { CliType, Mcp, McpCreate, McpUpdate } from '@/types/models'

// 后端返回的 cli_flags 格式
type McpCliFlagBackend = { cli_type: string; enabled: boolean }
type McpBackend = Omit<Mcp, 'cli_flags'> & { cli_flags: McpCliFlagBackend[] }

// 将后端数组格式转换为前端对象格式
function transformCliFlags(cliFlags: McpCliFlagBackend[]): Record<string, boolean> {
  const result: Record<string, boolean> = {}
  for (const flag of cliFlags) {
    result[flag.cli_type] = flag.enabled
  }
  return result
}

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
