import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { CliType } from '@/types/models'

export const useUiStore = defineStore('ui', () => {
  // 服务商管理页面的 CLI 类型 tab
  const providersActiveCliType = ref<CliType>('claude_code')

  // 会话管理页面的 CLI 类型 tab
  const sessionsActiveCliType = ref<CliType>('claude_code')

  // 日志页面的 tab 状态
  const logsActiveTab = ref<'request' | 'system'>('request')

  // 全局配置页面的 tab 状态
  const configActiveCliTab = ref<'claude_code' | 'codex' | 'gemini'>('claude_code')
  const configActiveBackupTab = ref<'local' | 'webdav'>('local')

  function setProvidersActiveCliType(cliType: CliType) {
    providersActiveCliType.value = cliType
  }

  function setSessionsActiveCliType(cliType: CliType) {
    sessionsActiveCliType.value = cliType
  }

  function setLogsActiveTab(tab: 'request' | 'system') {
    logsActiveTab.value = tab
  }

  function setConfigActiveCliTab(tab: 'claude_code' | 'codex' | 'gemini') {
    configActiveCliTab.value = tab
  }

  function setConfigActiveBackupTab(tab: 'local' | 'webdav') {
    configActiveBackupTab.value = tab
  }

  return {
    providersActiveCliType,
    sessionsActiveCliType,
    logsActiveTab,
    configActiveCliTab,
    configActiveBackupTab,
    setProvidersActiveCliType,
    setSessionsActiveCliType,
    setLogsActiveTab,
    setConfigActiveCliTab,
    setConfigActiveBackupTab,
  }
})
