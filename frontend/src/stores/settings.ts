import { defineStore } from 'pinia'
import { ref } from 'vue'
import { settingsApi } from '@/api/settings'
import { useUiStore } from '@/stores/ui'
import type { AllSettings, CliType, CliMode, GatewaySettingsUpdate, TimeoutSettingsUpdate, CliSettingsUpdate } from '@/types/models'

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<AllSettings | null>(null)
  const loading = ref(false)

  async function fetchSettings() {
    loading.value = true
    try {
      const { data } = await settingsApi.getAll()
      settings.value = data
      useUiStore().applyPersistedTabs({
        config_active_cli_type: data.gateway.config_active_cli_type,
        providers_active_cli_type: data.gateway.providers_active_cli_type,
        sessions_active_cli_type: data.gateway.sessions_active_cli_type
      })
    } finally {
      loading.value = false
    }
  }

  async function updateGateway(data: GatewaySettingsUpdate) {
    await settingsApi.updateGateway(data)
    await fetchSettings()
  }

  async function updateTimeouts(data: TimeoutSettingsUpdate) {
    await settingsApi.updateTimeouts(data)
    await fetchSettings()
  }

  async function updateCli(cliType: CliType, data: CliSettingsUpdate) {
    await settingsApi.updateCli(cliType, data)
    await fetchSettings()
  }

  async function setCliMode(cliType: CliType, mode: CliMode) {
    await settingsApi.setCliMode(cliType, mode)
    await fetchSettings()
  }

  async function setDashboardCliMode(cliType: CliType, mode: CliMode) {
    await settingsApi.setDashboardCliMode(cliType, mode)
    await fetchSettings()
  }

  return { settings, loading, fetchSettings, updateGateway, updateTimeouts, updateCli, setCliMode, setDashboardCliMode }
})
