import { defineStore } from 'pinia'
import { ref } from 'vue'
import { settingsApi } from '@/api/settings'

export const useDashboardStore = defineStore('dashboard', () => {
  const status = ref<'running' | 'stopped'>('stopped')
  const port = ref(0)
  const gatewayUrl = ref('')
  const version = ref('')

  async function fetchStatus() {
    try {
      const { data } = await settingsApi.getStatus()
      status.value = data.status
      port.value = data.port
      gatewayUrl.value = data.gateway_url
      version.value = data.version
    } catch {
      status.value = 'stopped'
    }
  }

  return { status, port, gatewayUrl, version, fetchStatus }
})
