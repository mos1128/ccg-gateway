import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  ScheduledTask,
  ScheduledTaskCreate,
  ScheduledTaskRun,
  ScheduledTaskRunItem,
  ScheduledTaskRunListResponse,
  ScheduledTaskUpdate
} from '@/types/models'

export interface ScheduledTaskChangeEvent {
  task_id: number | null
  run_id: number | null
}

export const scheduledTasksApi = {
  list: async (): Promise<{ data: ScheduledTask[] }> => {
    const data = await invoke<ScheduledTask[]>('get_scheduled_tasks')
    return { data }
  },
  get: async (id: number): Promise<{ data: ScheduledTask }> => {
    const data = await invoke<ScheduledTask>('get_scheduled_task', { id })
    return { data }
  },
  create: async (input: ScheduledTaskCreate): Promise<{ data: ScheduledTask }> => {
    const data = await invoke<ScheduledTask>('create_scheduled_task', { input })
    return { data }
  },
  update: async (id: number, input: ScheduledTaskUpdate): Promise<{ data: ScheduledTask }> => {
    const data = await invoke<ScheduledTask>('update_scheduled_task', { id, input })
    return { data }
  },
  delete: async (id: number): Promise<{ data: null }> => {
    await invoke('delete_scheduled_task', { id })
    return { data: null }
  },
  runNow: async (id: number): Promise<{ data: ScheduledTaskRun }> => {
    const data = await invoke<ScheduledTaskRun>('run_scheduled_task_now', { id })
    return { data }
  },
  runs: async (params?: { task_id?: number; page?: number; page_size?: number }): Promise<{ data: ScheduledTaskRunListResponse }> => {
    const args: Record<string, number> = {}
    if (params?.task_id !== undefined) args.taskId = params.task_id
    if (params?.page !== undefined) args.page = params.page
    if (params?.page_size !== undefined) args.pageSize = params.page_size
    const data = await invoke<ScheduledTaskRunListResponse>('get_scheduled_task_runs', args)
    return { data }
  },
  runItems: async (runId: number): Promise<{ data: ScheduledTaskRunItem[] }> => {
    const data = await invoke<ScheduledTaskRunItem[]>('get_scheduled_task_run_items', { runId })
    return { data }
  },
  listenChanges: (callback: (event: ScheduledTaskChangeEvent) => void): Promise<UnlistenFn> => {
    return listen<ScheduledTaskChangeEvent>('scheduled-task-changed', (event) => {
      callback(event.payload)
    })
  }
}
