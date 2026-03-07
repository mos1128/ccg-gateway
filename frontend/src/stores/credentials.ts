import { defineStore } from 'pinia'
import { ref } from 'vue'
import { credentialsApi } from '@/api/credentials'
import type { OfficialCredential, OfficialCredentialCreate, OfficialCredentialUpdate, CliType } from '@/types/models'

export const useCredentialStore = defineStore('credentials', () => {
  const credentials = ref<OfficialCredential[]>([])
  const loading = ref(false)

  async function fetchCredentials(cliType: CliType) {
    loading.value = true
    try {
      const { data } = await credentialsApi.list(cliType)
      credentials.value = data
    } finally {
      loading.value = false
    }
  }

  async function createCredential(data: OfficialCredentialCreate) {
    const { data: credential } = await credentialsApi.create(data)
    credentials.value.push(credential)
    return credential
  }

  async function updateCredential(id: number, data: OfficialCredentialUpdate) {
    const { data: credential } = await credentialsApi.update(id, data)
    const index = credentials.value.findIndex(c => c.id === id)
    if (index !== -1) {
      credentials.value[index] = credential
    }
    return credential
  }

  async function deleteCredential(id: number) {
    await credentialsApi.delete(id)
    credentials.value = credentials.value.filter(c => c.id !== id)
  }

  async function reorderCredentials(ids: number[]) {
    await credentialsApi.reorder(ids)
    await fetchCredentials(credentials.value[0]?.cli_type || 'claude_code')
  }

  return {
    credentials,
    loading,
    fetchCredentials,
    createCredential,
    updateCredential,
    deleteCredential,
    reorderCredentials
  }
})
