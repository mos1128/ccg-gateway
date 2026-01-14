import api from './instance'

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
  listProjects: (cliType: string, page = 1, pageSize = 20) =>
    api.get<PaginatedResponse<ProjectInfo>>('/sessions/projects', {
      params: { cli_type: cliType, page, page_size: pageSize }
    }),

  listSessions: (cliType: string, projectName: string, page = 1, pageSize = 20) =>
    api.get<PaginatedResponse<SessionInfo>>(
      `/sessions/projects/${encodeURIComponent(projectName)}/sessions`,
      { params: { cli_type: cliType, page, page_size: pageSize } }
    ),

  getSessionMessages: (cliType: string, projectName: string, sessionId: string) =>
    api.get<SessionMessage[]>(
      `/sessions/projects/${encodeURIComponent(projectName)}/sessions/${sessionId}/messages`,
      { params: { cli_type: cliType } }
    ),

  deleteSession: (cliType: string, projectName: string, sessionId: string) =>
    api.delete(`/sessions/projects/${encodeURIComponent(projectName)}/sessions/${sessionId}`, {
      params: { cli_type: cliType }
    }),

  deleteProject: (cliType: string, projectName: string) =>
    api.delete(`/sessions/projects/${encodeURIComponent(projectName)}`, {
      params: { cli_type: cliType }
    })
}
