import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { agentsApi } from '@/api/agents'
import type {
  AgentDefinitionLoadError,
  AgentDiagnostic,
  AgentFeatureName,
  AgentInfo,
} from '@/types/models'

export const useAgentStore = defineStore('agents', () => {
  const agents = ref<AgentInfo[]>([])
  const definitionErrors = ref<AgentDefinitionLoadError[]>([])
  const diagnostics = ref<AgentDiagnostic[]>([])
  const loading = ref(false)

  const tabs = computed(() => agents.value.map((agent) => ({ id: agent.id, label: agent.name })))
  const ids = computed(() => agents.value.map((agent) => agent.id))
  const byId = computed(() => new Map(agents.value.map((agent) => [agent.id, agent])))

  function get(agentId: string) {
    return byId.value.get(agentId)
  }

  function supports(agentId: string, feature: AgentFeatureName) {
    return get(agentId)?.features[feature].enabled === true
  }

  function agentsFor(feature: AgentFeatureName) {
    return agents.value.filter((agent) => agent.features[feature].enabled)
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
    definitionErrors,
    diagnostics,
    loading,
    tabs,
    ids,
    get,
    supports,
    agentsFor,
    fetchAgents,
    fetchDiagnostics,
  }
})
