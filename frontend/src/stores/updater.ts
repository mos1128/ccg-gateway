import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { translate } from '@/locales'
import { updaterApi, type UpdateSettingsPatch } from '@/api/updater'
import { getErrorMessage } from '@/utils/error'
import { notify } from '@/utils/notification'
import type {
  UpdateCheckResult,
  UpdateInstallOutcome,
  UpdateProgressPayload,
  UpdateSettingsResponse,
} from '@/types/models'

export type UpdatePhase =
  | 'idle'
  | 'checking'
  | 'latest'
  | 'available'
  | 'downloading'
  | 'ready'
  | 'installing'
  | 'error'

const AUTO_CHECK_INTERVAL_MS = 8 * 60 * 60 * 1000

export const useUpdaterStore = defineStore('updater', () => {
  const visible = ref(false)
  const settings = ref<UpdateSettingsResponse | null>(null)
  const result = ref<UpdateCheckResult | null>(null)
  const phase = ref<UpdatePhase>('idle')
  const error = ref('')
  const downloaded = ref(0)
  const total = ref<number | null>(null)
  const localPath = ref('')
  const installOutcome = ref<UpdateInstallOutcome | null>(null)
  const cancelling = ref(false)

  const currentVersion = computed(() => settings.value?.current_version || result.value?.current_version || '')
  const latest = computed(() => result.value?.latest ?? null)
  const hasUpdate = computed(() => result.value?.has_update === true)
  const progress = computed(() => {
    if (total.value === null || total.value <= 0) return null
    return Math.min(100, Math.max(0, Math.floor((downloaded.value * 100) / total.value)))
  })

  let unlistenProgress: (() => void) | null = null
  let initPromise: Promise<void> | null = null
  let autoCheckTimer: ReturnType<typeof setInterval> | null = null
  let promptedVersion = ''
  let installFailure = false

  function handleProgress(payload: UpdateProgressPayload) {
    if (phase.value !== 'downloading') return
    const target = latest.value
    if (target && payload.version !== target.version) return
    downloaded.value = payload.downloaded
    total.value = payload.total
  }

  async function loadSettings() {
    try {
      settings.value = await updaterApi.getSettings()
    } catch (e) {
      error.value = getErrorMessage(e, translate('updater.loadFailed'))
    }
  }

  async function saveSettings(patch: UpdateSettingsPatch) {
    const cleaned = Object.fromEntries(
      Object.entries(patch).filter(([, value]) => value !== undefined)
    ) as UpdateSettingsPatch
    const previous = settings.value ? { ...settings.value } : null
    if (settings.value) settings.value = { ...settings.value, ...cleaned }
    try {
      await updaterApi.updateSettings(cleaned)
      await loadSettings()
    } catch (e) {
      if (previous) settings.value = previous
      notify(getErrorMessage(e, translate('updater.saveFailed')), 'error')
      throw e
    }
  }

  async function refreshInstallResult() {
    try {
      const outcome = await updaterApi.takeResult()
      if (!outcome) return
      if (outcome.ok) {
        notify(translate('updater.updatedTo', { version: outcome.version }))
      } else {
        installFailure = true
        phase.value = 'error'
        error.value = outcome.message || translate('updater.installFailed')
        visible.value = true
        notify(error.value, 'error')
      }
    } catch {
      // 结果文件损坏不应影响启动
    }
  }

  function startAutoCheck() {
    if (autoCheckTimer) return
    autoCheckTimer = setInterval(() => {
      if (settings.value?.auto_check) void check(true)
    }, AUTO_CHECK_INTERVAL_MS)
  }

  async function init() {
    if (initPromise) return initPromise
    initPromise = (async () => {
      try {
        unlistenProgress = await updaterApi.onProgress(handleProgress)
      } catch {
        unlistenProgress = null
      }
      await loadSettings()
      await refreshInstallResult()
      if (!installFailure && settings.value?.auto_check) {
        await check(true)
      }
      startAutoCheck()
    })()
    return initPromise
  }

  async function check(silent = false) {
    if (['checking', 'downloading', 'installing', 'ready'].includes(phase.value)) return
    phase.value = 'checking'
    error.value = ''
    try {
      const response = await updaterApi.check()
      if (response.latest?.version !== result.value?.latest?.version) {
        localPath.value = ''
        installOutcome.value = null
      }
      result.value = response
      if (response.has_update) {
        phase.value = 'available'
        const version = `v${response.latest?.version ?? ''}`
        const shouldPrompt = silent && !visible.value && promptedVersion !== version
        promptedVersion = version
        if (shouldPrompt) visible.value = true
      } else {
        phase.value = 'latest'
      }
    } catch (e) {
      phase.value = 'error'
      error.value = getErrorMessage(e, translate('updater.fetchFailed'))
    }
  }

  async function download(): Promise<boolean> {
    const target = latest.value
    if (!target) return false
    phase.value = 'downloading'
    error.value = ''
    installOutcome.value = null
    localPath.value = ''
    downloaded.value = 0
    total.value = null
    cancelling.value = false
    try {
      const response = await updaterApi.download(target.version)
      localPath.value = response.path
      phase.value = 'ready'
      return true
    } catch (e) {
      const message = getErrorMessage(e, translate('updater.downloadFailed'))
      if (message === 'update_download_cancelled') {
        phase.value = 'available'
        downloaded.value = 0
        total.value = null
        return false
      }
      phase.value = 'error'
      error.value = message
      return false
    } finally {
      cancelling.value = false
    }
  }

  async function install() {
    const target = latest.value
    if (!target) return
    phase.value = 'installing'
    error.value = ''
    try {
      const outcome = await updaterApi.install(target.version)
      installOutcome.value = outcome
      if (outcome.kind === 'restart_scheduled') {
        notify(translate('updater.restarting'))
        return
      }
      phase.value = 'ready'
      if (outcome.kind === 'opened_installer') {
        notify(translate('updater.installerOpened'))
      } else {
        notify(outcome.message || translate('updater.manualHint'), 'warning')
      }
    } catch (e) {
      phase.value = 'error'
      error.value = getErrorMessage(e, translate('updater.installFailed'))
    }
  }

  async function startUpdate() {
    if (phase.value === 'ready') {
      await install()
      return
    }
    const ok = await download()
    if (ok) await install()
  }

  async function cancelDownload() {
    if (phase.value !== 'downloading' || cancelling.value) return
    cancelling.value = true
    try {
      await updaterApi.cancel()
    } catch (e) {
      cancelling.value = false
      notify(getErrorMessage(e, translate('updater.cancelFailed')), 'error')
    }
  }

  function openWithCheck() {
    visible.value = true
    void check(false)
  }

  function close() {
    if (phase.value === 'installing') return
    visible.value = false
  }

  function dispose() {
    if (autoCheckTimer) {
      clearInterval(autoCheckTimer)
      autoCheckTimer = null
    }
    unlistenProgress?.()
    unlistenProgress = null
  }

  return {
    visible,
    settings,
    result,
    phase,
    error,
    downloaded,
    total,
    localPath,
    installOutcome,
    cancelling,
    currentVersion,
    latest,
    hasUpdate,
    progress,
    init,
    loadSettings,
    saveSettings,
    check,
    download,
    install,
    startUpdate,
    cancelDownload,
    openWithCheck,
    close,
    dispose,
  }
})
