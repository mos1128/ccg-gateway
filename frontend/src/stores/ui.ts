import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { settingsApi } from '@/api/settings'
import type { CliType, ProviderProfile } from '@/types/models'

export const useUiStore = defineStore('ui', () => {
  // 服务商管理页面的 CLI 类型 tab
  const providersActiveCliType = ref<CliType>('claude_code')
  const providersActiveProfiles = ref<Record<CliType, ProviderProfile>>({
    claude_code: 'default',
    codex: 'default',
    gemini: 'default'
  })
  const providersActiveProfile = computed(() => getProvidersActiveProfile(providersActiveCliType.value))

  // 会话管理页面的 CLI 类型 tab
  const sessionsActiveCliType = ref<CliType>('claude_code')

  // 日志页面的 tab 状态
  const logsActiveTab = ref<'request' | 'system'>('request')

  // 全局配置页面的 tab 状态
  const configActiveCliTab = ref<CliType>('claude_code')
  const configActiveBackupTab = ref<'local' | 'webdav'>('local')

  function setProvidersActiveCliType(cliType: CliType) {
    providersActiveCliType.value = cliType
    void settingsApi.updateUiTabs({ providers_active_cli_type: cliType }).catch((e) => {
      console.error('Failed to persist providers active CLI tab:', e)
    })
  }

  function getProvidersActiveProfile(cliType: CliType) {
    return providersActiveProfiles.value[cliType] ?? 'default'
  }

  function setProvidersActiveProfile(profile: ProviderProfile, cliType = providersActiveCliType.value) {
    providersActiveProfiles.value = { ...providersActiveProfiles.value, [cliType]: profile }
  }

  function setSessionsActiveCliType(cliType: CliType) {
    sessionsActiveCliType.value = cliType
    void settingsApi.updateUiTabs({ sessions_active_cli_type: cliType }).catch((e) => {
      console.error('Failed to persist sessions active CLI tab:', e)
    })
  }

  function setLogsActiveTab(tab: 'request' | 'system') {
    logsActiveTab.value = tab
  }

  function setConfigActiveCliTab(tab: CliType) {
    configActiveCliTab.value = tab
    void settingsApi.updateUiTabs({ config_active_cli_type: tab }).catch((e) => {
      console.error('Failed to persist config active CLI tab:', e)
    })
  }

  function setConfigActiveBackupTab(tab: 'local' | 'webdav') {
    configActiveBackupTab.value = tab
  }

  function applyPersistedTabs(tabs: {
    config_active_cli_type?: CliType
    providers_active_cli_type?: CliType
    sessions_active_cli_type?: CliType
  }) {
    if (tabs.config_active_cli_type) {
      configActiveCliTab.value = tabs.config_active_cli_type
    }
    if (tabs.providers_active_cli_type) {
      providersActiveCliType.value = tabs.providers_active_cli_type
    }
    if (tabs.sessions_active_cli_type) {
      sessionsActiveCliType.value = tabs.sessions_active_cli_type
    }
  }

  return {
    providersActiveCliType,
    providersActiveProfile,
    sessionsActiveCliType,
    logsActiveTab,
    configActiveCliTab,
    configActiveBackupTab,
    setProvidersActiveCliType,
    getProvidersActiveProfile,
    setProvidersActiveProfile,
    setSessionsActiveCliType,
    setLogsActiveTab,
    setConfigActiveCliTab,
    setConfigActiveBackupTab,
    applyPersistedTabs,
  }
})
