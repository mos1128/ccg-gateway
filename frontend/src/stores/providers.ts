import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { providersApi } from '@/api/providers'
import type { Provider, ProviderCreate, ProviderProfile, ProviderUpdate } from '@/types/models'
import { useUiStore } from './ui'

export const useProviderStore = defineStore('providers', () => {
  const uiStore = useUiStore()
  
  // Cache map: key is `${cliType}_${profile}`
  const providersMap = ref<Record<string, Provider[]>>({})
  const loading = ref(false)

  const activeCacheKey = computed(() => {
    const type = uiStore.providersActiveCliType
    const profile = type === 'claude_code' || type === 'codex' 
      ? uiStore.providersActiveProfile 
      : 'default'
    return `${type}_${profile}`
  })

  const providers = computed({
    get: () => providersMap.value[activeCacheKey.value] || [],
    set: (val) => {
      providersMap.value[activeCacheKey.value] = val
    }
  })

  function getCacheKey(cliType?: string, profile?: ProviderProfile) {
    const type = cliType || uiStore.providersActiveCliType
    const targetProfile = type === 'claude_code' || type === 'codex'
      ? (profile || uiStore.providersActiveProfile)
      : 'default'
    return `${type}_${targetProfile}`
  }

  async function fetchProviders(cliType?: string, profile?: ProviderProfile) {
    loading.value = true
    try {
      const type = cliType || uiStore.providersActiveCliType
      const targetProfile = type === 'claude_code' || type === 'codex'
        ? (profile || uiStore.providersActiveProfile)
        : 'default'
      const key = `${type}_${targetProfile}`
      
      const { data } = await providersApi.list(type, targetProfile)
      providersMap.value[key] = data
    } finally {
      loading.value = false
    }
  }

  async function createProvider(data: ProviderCreate) {
    const { data: provider } = await providersApi.create(data)
    const key = activeCacheKey.value
    if (!providersMap.value[key]) {
      providersMap.value[key] = []
    }
    providersMap.value[key].push(provider)
    return provider
  }

  async function updateProvider(id: number, data: ProviderUpdate) {
    const { data: provider } = await providersApi.update(id, data)
    const key = activeCacheKey.value
    if (providersMap.value[key]) {
      const index = providersMap.value[key].findIndex(p => p.id === id)
      if (index !== -1) {
        providersMap.value[key][index] = provider
      }
    }
    return provider
  }

  async function deleteProvider(id: number) {
    await providersApi.delete(id)
    const key = activeCacheKey.value
    if (providersMap.value[key]) {
      providersMap.value[key] = providersMap.value[key].filter(p => p.id !== id)
    }
  }

  async function reorderProviders(ids: number[]) {
    await providersApi.reorder(ids)
    await fetchProviders()
  }

  async function resetFailures(id: number) {
    await providersApi.resetFailures(id)
    await fetchProviders()
  }

  async function unblacklist(id: number) {
    await providersApi.unblacklist(id)
    await fetchProviders()
  }

  return {
    providers,
    providersMap,
    loading,
    getCacheKey,
    fetchProviders,
    createProvider,
    updateProvider,
    deleteProvider,
    reorderProviders,
    resetFailures,
    unblacklist
  }
})
