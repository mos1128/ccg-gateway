import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { providersApi } from '@/api/providers'
import type { CliType, Provider, ProviderCreate, ProviderHealthEvent, ProviderProfile, ProviderUpdate } from '@/types/models'
import { useUiStore } from './ui'
import { useAgentStore } from './agents'

export const useProviderStore = defineStore('providers', () => {
  const uiStore = useUiStore()
  const agentStore = useAgentStore()

  function supportsProfiles(cliType: CliType) {
    const feature = agentStore.get(cliType)?.features.profiles
    return feature?.enabled === true
  }
  
  // Cache map: key is `${cliType}_${profile}`
  const providersMap = ref<Record<string, Provider[]>>({})
  const loading = ref(false)

  const activeCacheKey = computed(() => {
    const type = uiStore.providersActiveCliType
    const profile = supportsProfiles(type)
      ? uiStore.getProvidersActiveProfile(type)
      : 'default'
    return `${type}_${profile}`
  })

  const providers = computed({
    get: () => providersMap.value[activeCacheKey.value] || [],
    set: (val) => {
      providersMap.value[activeCacheKey.value] = val
    }
  })

  function getCacheKey(cliType?: CliType, profile?: ProviderProfile) {
    const type = cliType || uiStore.providersActiveCliType
    const targetProfile = supportsProfiles(type)
      ? (profile || uiStore.getProvidersActiveProfile(type))
      : 'default'
    return `${type}_${targetProfile}`
  }

  async function fetchProviders(cliType?: CliType, profile?: ProviderProfile) {
    loading.value = true
    try {
      const type = cliType || uiStore.providersActiveCliType
      const targetProfile = supportsProfiles(type)
        ? (profile || uiStore.getProvidersActiveProfile(type))
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
    // 按 id 找回所在分组，而不是认定它一定在当前分组：请求期间用户可能已经切了
    // tab，那时写当前分组会落空，界面就会停在旧值
    for (const list of Object.values(providersMap.value)) {
      const index = list.findIndex((p) => p.id === id)
      if (index !== -1) list[index] = provider
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

  // 熔断状态由后端推送，直接就地更新所有缓存分组，避免切换页面才刷新
  function applyHealthEvent(health: ProviderHealthEvent) {
    for (const list of Object.values(providersMap.value)) {
      const provider = list.find((p) => p.id === health.provider_id)
      if (!provider) continue
      provider.consecutive_failures = health.consecutive_failures
      provider.blacklisted_until = health.blacklisted_until
      provider.is_blacklisted = health.is_blacklisted
    }
  }

  // 解除时刻随列表一起下发，到期不必回头问后端：按后端 ProviderResponse 的口径
  // 本地算一遍即可（熔断已过期则失败计数显示为 0，blacklisted_until 保持原值）
  function expireBlacklists(nowSeconds: number) {
    for (const list of Object.values(providersMap.value)) {
      for (const provider of list) {
        if (!provider.is_blacklisted || !provider.blacklisted_until) continue
        if (provider.blacklisted_until > nowSeconds) continue
        provider.is_blacklisted = false
        provider.consecutive_failures = 0
      }
    }
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
    applyHealthEvent,
    expireBlacklists
  }
})
