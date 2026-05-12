import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { sessionsApi, type ProjectInfo, type SessionInfo, type SessionMessage } from '@/api/sessions'
import type { CliType } from '@/types/models'
import { useUiStore } from './ui'

export const useSessionStore = defineStore('sessions', () => {
  const uiStore = useUiStore()

  // State maps by CliType
  const projectsMap = ref<Record<string, ProjectInfo[]>>({})
  const sessionsMap = ref<Record<string, SessionInfo[]>>({})
  const currentProjectMap = ref<Record<string, string>>({})
  const currentProjectInfoMap = ref<Record<string, ProjectInfo | null>>({})

  // Shared state
  const messages = ref<SessionMessage[]>([])
  const loading = ref(false)
  const currentSession = ref<string>('')

  // Pagination state maps by CliType
  const projectPageMap = ref<Record<string, number>>({})
  const projectTotalMap = ref<Record<string, number>>({})
  const sessionPageMap = ref<Record<string, number>>({})
  const sessionTotalMap = ref<Record<string, number>>({})
  const pageSize = ref(1000)

  const activeCliType = computed(() => uiStore.sessionsActiveCliType)

  const projects = computed(() => projectsMap.value[activeCliType.value] || [])
  const sessions = computed(() => sessionsMap.value[activeCliType.value] || [])
  const currentProject = computed(() => currentProjectMap.value[activeCliType.value] || '')
  const currentProjectInfo = computed(() => currentProjectInfoMap.value[activeCliType.value] || null)

  const projectPage = computed({
    get: () => projectPageMap.value[activeCliType.value] || 1,
    set: (val) => projectPageMap.value[activeCliType.value] = val
  })
  const projectTotal = computed(() => projectTotalMap.value[activeCliType.value] || 0)

  const sessionPage = computed({
    get: () => sessionPageMap.value[activeCliType.value] || 1,
    set: (val) => sessionPageMap.value[activeCliType.value] = val
  })
  const sessionTotal = computed(() => sessionTotalMap.value[activeCliType.value] || 0)

  async function fetchProjects(page?: number, cliType?: CliType) {
    loading.value = true
    const type = cliType || activeCliType.value
    if (page !== undefined) {
      projectPageMap.value[type] = page
    } else if (!projectPageMap.value[type]) {
      projectPageMap.value[type] = 1
    }
    
    try {
      const { data } = await sessionsApi.listProjects(type, projectPageMap.value[type], pageSize.value)
      projectsMap.value[type] = data.items
      projectTotalMap.value[type] = data.total
    } catch (error: any) {
      console.error('Failed to fetch projects:', error)
      projectsMap.value[type] = []
      projectTotalMap.value[type] = 0
    } finally {
      loading.value = false
    }
  }

  async function fetchSessions(projectName: string, page?: number, projectInfo?: ProjectInfo, cliType?: CliType) {
    loading.value = true
    const type = cliType || activeCliType.value
    
    currentProjectMap.value[type] = projectName
    if (projectInfo) {
      currentProjectInfoMap.value[type] = projectInfo
    }
    if (page !== undefined) {
      sessionPageMap.value[type] = page
    } else if (!sessionPageMap.value[type]) {
      sessionPageMap.value[type] = 1
    }
    
    try {
      const { data } = await sessionsApi.listSessions(type, projectName, sessionPageMap.value[type], pageSize.value)
      sessionsMap.value[type] = data.items
      sessionTotalMap.value[type] = data.total
    } catch (error: any) {
      console.error('Failed to fetch sessions:', error)
      sessionsMap.value[type] = []
      sessionTotalMap.value[type] = 0
    } finally {
      loading.value = false
    }
  }

  async function fetchMessages(projectName: string, sessionId: string, cliType?: CliType) {
    loading.value = true
    const type = cliType || activeCliType.value
    currentSession.value = sessionId
    try {
      const { data } = await sessionsApi.getSessionMessages(type, projectName, sessionId)
      messages.value = data
    } catch (error: any) {
      console.error('Failed to fetch messages:', error)
      messages.value = []
    } finally {
      loading.value = false
    }
  }

  async function deleteSession(projectName: string, sessionId: string, cliType?: CliType) {
    const type = cliType || activeCliType.value
    await sessionsApi.deleteSession(type, projectName, sessionId)
    if (sessionsMap.value[type]) {
      sessionsMap.value[type] = sessionsMap.value[type].filter(s => s.session_id !== sessionId)
      sessionTotalMap.value[type] = Math.max(0, (sessionTotalMap.value[type] || 0) - 1)
    }
  }

  async function deleteProject(projectName: string, cliType?: CliType) {
    const type = cliType || activeCliType.value
    await sessionsApi.deleteProject(type, projectName)
    if (projectsMap.value[type]) {
      projectsMap.value[type] = projectsMap.value[type].filter(p => p.name !== projectName)
      projectTotalMap.value[type] = Math.max(0, (projectTotalMap.value[type] || 0) - 1)
    }
  }

  function clearMessages() {
    messages.value = []
    currentSession.value = ''
  }

  function clearSessions(cliType?: CliType) {
    const type = cliType || activeCliType.value
    sessionsMap.value[type] = []
    currentProjectMap.value[type] = ''
    currentProjectInfoMap.value[type] = null
    sessionPageMap.value[type] = 1
    sessionTotalMap.value[type] = 0
  }

  return {
    projects,
    sessions,
    messages,
    loading,
    currentProject,
    currentProjectInfo,
    currentSession,
    projectPage,
    projectTotal,
    sessionPage,
    sessionTotal,
    pageSize,
    fetchProjects,
    fetchSessions,
    fetchMessages,
    deleteSession,
    deleteProject,
    clearMessages,
    clearSessions
  }
})
