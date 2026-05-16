import { invoke } from '@tauri-apps/api/core'
import type { CliType } from '@/types/models'

export interface ProjectInfo {
  name: string
  display_name: string
  full_path: string
  session_count: number
  total_size: number
  last_modified: number
}

export interface SessionInfo {
  session_id: string
  size: number
  mtime: number
  first_message: string
  git_branch: string
  summary: string
}

export interface SessionMessage {
  role: 'user' | 'assistant'
  content: string
  timestamp?: number
}

export interface PaginatedResponse<T> {
  items: T[]
  total: number
  page: number
  page_size: number
}

export const sessionsApi = {
  listProjects: async (cliType: CliType, page = 1, pageSize = 20): Promise<{ data: PaginatedResponse<ProjectInfo> }> => {
    const data = await invoke<PaginatedResponse<ProjectInfo>>('get_session_projects', {
      cliType,
      page,
      pageSize
    })
    return { data }
  },

  listSessions: async (cliType: CliType, projectName: string, page = 1, pageSize = 20): Promise<{ data: PaginatedResponse<SessionInfo> }> => {
    const data = await invoke<PaginatedResponse<SessionInfo>>('get_project_sessions', {
      cliType,
      projectName,
      page,
      pageSize
    })
    return { data }
  },

  getSessionMessages: async (cliType: CliType, projectName: string, sessionId: string): Promise<{ data: SessionMessage[] }> => {
    const data = await invoke<SessionMessage[]>('get_session_messages', {
      cliType,
      projectName,
      sessionId
    })
    return { data }
  },

  deleteSession: async (cliType: CliType, projectName: string, sessionId: string) => {
    await invoke('delete_session', { cliType, projectName, sessionId })
    return { data: null }
  },

  deleteProject: async (cliType: CliType, projectName: string) => {
    await invoke('delete_project', { cliType, projectName })
    return { data: null }
  }
}
