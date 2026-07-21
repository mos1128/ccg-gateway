import { invoke } from './tauri-bridge'
import type {
  AgentDefinitionLoadError,
  AgentDiagnostic,
  AgentInfo,
} from '@/types/models'

export const agentsApi = {
  list: async () => ({ data: await invoke<AgentInfo[]>('get_agents') }),
  definitionErrors: async () => ({
    data: await invoke<AgentDefinitionLoadError[]>('get_agent_definition_errors'),
  }),
  diagnostics: async (kind?: string, limit = 100) => ({
    data: await invoke<AgentDiagnostic[]>('get_agent_diagnostics', { kind, limit }),
  }),
}
