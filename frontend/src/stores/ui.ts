import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import type { CliType } from '@/types/models'

export type Theme = 'light' | 'dark'

export const useUiStore = defineStore('ui', () => {
  // 主题状态
  const theme = ref<Theme>(localStorage.getItem('theme') as Theme || 'light')

  // 服务商管理页面的 CLI 类型 tab
  const providersActiveCliType = ref<CliType>('claude_code')

  // 会话管理页面的 CLI 类型 tab
  const sessionsActiveCliType = ref<CliType>('claude_code')

  // 日志页面的 tab 状态
  const logsActiveTab = ref<'request' | 'system'>('request')

  // 全局配置页面的 tab 状态
  const configActiveCliTab = ref<'claude_code' | 'codex' | 'gemini'>('claude_code')
  const configActiveBackupTab = ref<'local' | 'webdav'>('local')

  // 切换主题
  function toggleTheme() {
    theme.value = theme.value === 'light' ? 'dark' : 'light'
  }

  function setTheme(newTheme: Theme) {
    theme.value = newTheme
  }

  // 监听主题变化，持久化到 localStorage 并应用到文档
  watch(theme, (newTheme) => {
    localStorage.setItem('theme', newTheme)
    document.documentElement.setAttribute('data-theme', newTheme)
  }, { immediate: true })

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
    theme,
    toggleTheme,
    setTheme,
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
