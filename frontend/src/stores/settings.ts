import { defineStore } from 'pinia'
import { ref } from 'vue'
import { settingsApi } from '@/api/settings'
import type { AllSettings, BootstrapSettingsUpdate, CliType, CliMode, GatewaySettingsUpdate, SystemStatus, TimeoutSettingsUpdate, CliSettingsUpdate } from '@/types/models'

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<AllSettings | null>(null)
  const gatewayStatus = ref<SystemStatus | null>(null)
  const loading = ref(false)

  async function fetchSettings() {
    loading.value = true
    try {
      const { data } = await settingsApi.getAll()
      settings.value = data
      gatewayStatus.value = data.status
    } finally {
      loading.value = false
    }
  }

  async function updateGateway(data: GatewaySettingsUpdate) {
    await settingsApi.updateGateway(data)
    if (settings.value) Object.assign(settings.value.gateway, data)
  }

  async function fetchGatewayStatus() {
    const { data } = await settingsApi.getStatus()
    gatewayStatus.value = data
    if (settings.value) settings.value.status = data
    return data
  }

  async function updateBootstrap(data: BootstrapSettingsUpdate) {
    await settingsApi.updateBootstrap(data)
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

  return { settings, gatewayStatus, loading, fetchSettings, fetchGatewayStatus, updateGateway, updateBootstrap, updateTimeouts, updateCli, setCliMode, setDashboardCliMode }
})
