import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { agentsApi } from '@/api/agents'
import type {
  AgentDefinitionLoadError,
  AgentDiagnostic,
  AgentFeatureName,
  AgentInfo,
} from '@/types/models'

const HIDDEN_AGENTS_STORAGE_KEY = 'hidden-agents'

function loadHiddenAgentIds() {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(HIDDEN_AGENTS_STORAGE_KEY) || '[]')
    return new Set<string>(Array.isArray(value) ? value.filter((id: unknown): id is string => typeof id === 'string') : [])
  } catch {
    return new Set<string>()
  }
}

export const useAgentStore = defineStore('agents', () => {
  const agents = ref<AgentInfo[]>([])
  const hiddenAgentIds = ref(loadHiddenAgentIds())
  const definitionErrors = ref<AgentDefinitionLoadError[]>([])
  const diagnostics = ref<AgentDiagnostic[]>([])
  const loading = ref(false)

  const visibleAgents = computed(() => {
    const visible = agents.value.filter((agent) => !hiddenAgentIds.value.has(agent.id))
    return visible.length ? visible : agents.value.slice(0, 1)
  })
  const tabs = computed(() => visibleAgents.value.map((agent) => ({ id: agent.id, label: agent.name })))
  const ids = computed(() => visibleAgents.value.map((agent) => agent.id))
  const byId = computed(() => new Map(agents.value.map((agent) => [agent.id, agent])))

  function get(agentId: string) {
    return byId.value.get(agentId)
  }

  function supports(agentId: string, feature: AgentFeatureName) {
    return get(agentId)?.features[feature].enabled === true
  }

  function agentsFor(feature: AgentFeatureName) {
    return visibleAgents.value.filter((agent) => agent.features[feature].enabled)
  }

  function isVisible(agentId: string) {
    return visibleAgents.value.some((agent) => agent.id === agentId)
  }

  function setVisible(agentId: string, visible: boolean) {
    if (!visible && isVisible(agentId) && visibleAgents.value.length === 1) return false

    const next = new Set(hiddenAgentIds.value)
    if (visible) next.delete(agentId)
    else next.add(agentId)
    hiddenAgentIds.value = next
    localStorage.setItem(HIDDEN_AGENTS_STORAGE_KEY, JSON.stringify([...next]))
    return true
  }

  async function fetchAgents() {
    loading.value = true
    try {
      const [{ data }, { data: errors }] = await Promise.all([
        agentsApi.list(),
        agentsApi.definitionErrors(),
      ])
      agents.value = data
      definitionErrors.value = errors
    } finally {
      loading.value = false
    }
  }

  async function fetchDiagnostics(kind?: string) {
    const { data } = await agentsApi.diagnostics(kind)
    diagnostics.value = data
  }

  return {
    agents,
    visibleAgents,
    definitionErrors,
    diagnostics,
    loading,
    tabs,
    ids,
    get,
    supports,
    agentsFor,
    isVisible,
    setVisible,
    fetchAgents,
    fetchDiagnostics,
  }
})
