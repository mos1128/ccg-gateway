import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from './tauri-bridge'
import type {
  UpdateApplyResult,
  UpdateCheckResult,
  UpdateDownloadResult,
  UpdateInstallOutcome,
  UpdateProgressPayload,
  UpdateSettingsResponse,
} from '@/types/models'

export interface UpdateSettingsPatch {
  auto_check?: boolean
  accel_enabled?: boolean
  accel_prefix?: string
}

export const updaterApi = {
  getSettings: () => invoke<UpdateSettingsResponse>('get_update_settings'),

  updateSettings: (patch: UpdateSettingsPatch) =>
    invoke<void>('update_update_settings', {
      autoCheck: patch.auto_check,
      accelEnabled: patch.accel_enabled,
      accelPrefix: patch.accel_prefix,
    }),

  check: () => invoke<UpdateCheckResult>('check_for_updates'),

  download: (version: string) =>
    invoke<UpdateDownloadResult>('download_update', { version }),

  cancel: () => invoke<void>('cancel_update_download'),

  install: (version: string) =>
    invoke<UpdateInstallOutcome>('install_update', { version }),

  takeResult: () => invoke<UpdateApplyResult | null>('take_update_result'),

  onProgress: (handler: (payload: UpdateProgressPayload) => void): Promise<UnlistenFn> =>
    listen<UpdateProgressPayload>('update-download-progress', (event) => handler(event.payload)),
}
