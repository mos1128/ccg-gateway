import { defineStore } from 'pinia'
import { ref } from 'vue'
import { sessionsApi, type ProjectInfo, type SessionInfo, type SessionMessage } from '@/api/sessions'
import type { CliType } from '@/types/models'

export const useSessionStore = defineStore('sessions', () => {
  const projects = ref<ProjectInfo[]>([])
  const sessions = ref<SessionInfo[]>([])
  const messages = ref<SessionMessage[]>([])
  const loading = ref(false)
  const currentCliType = ref<CliType>('claude_code')
  const currentProject = ref<string>('')
  const currentProjectInfo = ref<ProjectInfo | null>(null)
  const currentSession = ref<string>('')

  // Pagination state
  const projectPage = ref(1)
  const projectTotal = ref(0)
  const sessionPage = ref(1)
  const sessionTotal = ref(0)
  const pageSize = ref(20)

  function setCliType(cliType: CliType) {
    currentCliType.value = cliType
    projectPage.value = 1
  }

  async function fetchProjects(page?: number) {
    loading.value = true
    if (page !== undefined) {
      projectPage.value = page
    }
    try {
      const { data } = await sessionsApi.listProjects(currentCliType.value, projectPage.value, pageSize.value)
      projects.value = data.items
      projectTotal.value = data.total
    } finally {
      loading.value = false
    }
  }

  async function fetchSessions(projectName: string, page?: number, projectInfo?: ProjectInfo) {
    loading.value = true
    currentProject.value = projectName
    if (projectInfo) {
      currentProjectInfo.value = projectInfo
    }
    if (page !== undefined) {
      sessionPage.value = page
    }
    try {
      const { data } = await sessionsApi.listSessions(currentCliType.value, projectName, sessionPage.value, pageSize.value)
      sessions.value = data.items
      sessionTotal.value = data.total
    } finally {
      loading.value = false
    }
  }

  async function fetchMessages(projectName: string, sessionId: string) {
    loading.value = true
    currentSession.value = sessionId
    try {
      const { data } = await sessionsApi.getSessionMessages(currentCliType.value, projectName, sessionId)
      messages.value = data
    } finally {
      loading.value = false
    }
  }

  async function deleteSession(projectName: string, sessionId: string) {
    await sessionsApi.deleteSession(currentCliType.value, projectName, sessionId)
    sessions.value = sessions.value.filter(s => s.session_id !== sessionId)
    sessionTotal.value = Math.max(0, sessionTotal.value - 1)
  }

  async function deleteProject(projectName: string) {
    await sessionsApi.deleteProject(currentCliType.value, projectName)
    projects.value = projects.value.filter(p => p.name !== projectName)
    projectTotal.value = Math.max(0, projectTotal.value - 1)
  }

  function clearMessages() {
    messages.value = []
    currentSession.value = ''
  }

  function clearSessions() {
    sessions.value = []
    currentProject.value = ''
    currentProjectInfo.value = null
    sessionPage.value = 1
    sessionTotal.value = 0
  }

  return {
    projects,
    sessions,
    messages,
    loading,
    currentCliType,
    currentProject,
    currentProjectInfo,
    currentSession,
    projectPage,
    projectTotal,
    sessionPage,
    sessionTotal,
    pageSize,
    setCliType,
    fetchProjects,
    fetchSessions,
    fetchMessages,
    deleteSession,
    deleteProject,
    clearMessages,
    clearSessions
  }
})
