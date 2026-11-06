<template>
  <Teleport to="body">
    <div class="update-overlay v2-scope" :class="{ active: store.visible }" @click.self="store.close()">
      <div class="update-dialog">
        <div class="update-header">
          <div class="update-title">
            <span>{{ t('updater.title') }}</span>
            <span class="update-current mono">v{{ store.currentVersion || '—' }}</span>
          </div>
          <div class="update-header-actions">
            <button
              class="update-icon-btn"
              type="button"
              :disabled="busy"
              :title="t('updater.checkAgain')"
              @click="checkAgain"
            >
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/></svg>
            </button>
            <button
              class="update-icon-btn"
              type="button"
              :aria-label="t('common.close')"
              :disabled="store.phase === 'installing'"
              @click="store.close()"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 6l12 12M18 6L6 18"/></svg>
            </button>
          </div>
        </div>

        <div class="update-body">
          <div v-if="store.phase === 'error'" class="update-error">
            <div class="update-error-text">{{ store.error }}</div>
          </div>

          <div v-else-if="store.phase === 'checking'" class="update-status">
            <span class="update-spinner"></span>
            <span>{{ t('updater.checking') }}</span>
          </div>

          <div v-else-if="store.phase === 'latest'" class="update-status update-status-success">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>
            <span>{{ t('updater.latest') }}</span>
          </div>

          <template v-else-if="store.phase === 'available' && store.latest">
            <div class="update-found">
              <div class="update-found-title">{{ t('updater.foundVersion', { version: `v${store.latest.version}` }) }}</div>
              <div class="update-found-meta">
                {{ t('updater.currentVersion', { version: `v${store.currentVersion}` }) }}
                <span v-if="sourceLabel"> · {{ sourceLabel }}</span>
              </div>
            </div>
            <div class="update-notes">
              <div class="update-notes-title">{{ t('updater.releaseNotes') }}</div>
              <div class="update-notes-body">
                <V2Markdown v-if="store.latest.notes" :content="store.latest.notes" />
                <span v-else class="update-muted">{{ t('updater.noNotes') }}</span>
              </div>
            </div>
            <div class="update-hint">{{ platformHint }}</div>
            <div v-if="!store.latest.asset" class="update-hint update-hint-warning">{{ t('updater.noAsset') }}</div>
          </template>

          <template v-else-if="store.phase === 'downloading'">
            <div class="update-found">
              <div class="update-found-title">{{ t('updater.downloading') }}</div>
              <div class="update-found-meta">{{ t('updater.downloadingHint') }}</div>
            </div>
            <div class="update-progress-row">
              <div class="update-progress-track" :class="{ indeterminate: store.progress === null }">
                <span :style="store.progress === null ? undefined : { width: `${store.progress}%` }"></span>
              </div>
              <span class="update-progress-text mono">{{ progressText }}</span>
            </div>
          </template>

          <template v-else-if="store.phase === 'ready'">
            <div class="update-status update-status-success">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>
              <span>{{ t('updater.downloadReady') }}</span>
            </div>
            <div v-if="store.installOutcome?.message" class="update-hint update-hint-warning">
              {{ store.installOutcome.message }}
            </div>
            <div v-if="store.localPath" class="update-path mono">{{ store.localPath }}</div>
          </template>

          <div v-else-if="store.phase === 'installing'" class="update-status">
            <span class="update-spinner"></span>
            <span>{{ t('updater.installing') }}</span>
          </div>

          <div v-else class="update-status update-muted">{{ t('updater.idle') }}</div>
        </div>

        <div class="update-footer">
          <button
            v-if="releaseUrl && store.phase !== 'installing'"
            class="v2-btn v2-btn-sm v2-btn-ghost"
            type="button"
            @click="openRelease"
          >{{ t('updater.releasePage') }}</button>
          <span class="update-footer-gap"></span>

          <template v-if="store.phase === 'available'">
            <button class="v2-btn v2-btn-sm v2-btn-outline" type="button" @click="store.close()">{{ t('updater.later') }}</button>
            <button
              class="v2-btn v2-btn-sm v2-btn-primary"
              type="button"
              :disabled="!store.latest?.asset"
              @click="startUpdate"
            >{{ t('updater.install') }}</button>
          </template>
          <template v-else-if="store.phase === 'downloading'">
            <button
              class="v2-btn v2-btn-sm v2-btn-outline"
              type="button"
              :disabled="store.cancelling"
              @click="store.cancelDownload()"
            >{{ store.cancelling ? t('updater.cancelling') : t('updater.cancelDownload') }}</button>
          </template>
          <template v-else-if="store.phase === 'ready'">
            <button class="v2-btn v2-btn-sm v2-btn-ghost" type="button" @click="openFolder">{{ t('updater.openFolder') }}</button>
            <button class="v2-btn v2-btn-sm v2-btn-outline" type="button" @click="store.close()">{{ t('updater.later') }}</button>
            <button class="v2-btn v2-btn-sm v2-btn-primary" type="button" @click="store.install()">{{ installButtonText }}</button>
          </template>
          <template v-else-if="store.phase === 'installing'">
            <button class="v2-btn v2-btn-sm v2-btn-primary" type="button" disabled>{{ t('updater.installing') }}</button>
          </template>
          <template v-else-if="store.phase === 'error'">
            <button class="v2-btn v2-btn-sm v2-btn-outline" type="button" @click="store.close()">{{ t('common.close') }}</button>
            <button v-if="store.localPath" class="v2-btn v2-btn-sm v2-btn-ghost" type="button" @click="openFolder">{{ t('updater.openFolder') }}</button>
            <button
              class="v2-btn v2-btn-sm"
              :class="store.localPath ? 'v2-btn-outline' : 'v2-btn-primary'"
              type="button"
              @click="checkAgain"
            >{{ t('updater.checkAgain') }}</button>
            <button v-if="store.localPath" class="v2-btn v2-btn-sm v2-btn-primary" type="button" @click="store.install()">{{ t('updater.installNow') }}</button>
          </template>
          <template v-else>
            <button class="v2-btn v2-btn-sm v2-btn-outline" type="button" @click="store.close()">{{ t('common.close') }}</button>
            <button class="v2-btn v2-btn-sm v2-btn-primary" type="button" :disabled="store.phase === 'checking'" @click="checkAgain">{{ t('updater.checkAgain') }}</button>
          </template>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, onUnmounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { open } from '@tauri-apps/plugin-shell'
import V2Markdown from '@/components/V2Markdown.vue'
import { useUpdaterStore } from '@/stores/updater'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'

const store = useUpdaterStore()
const { t } = useI18n()

const RELEASES_PAGE = 'https://github.com/mos1128/ccg-gateway/releases'

const busy = computed(() => ['checking', 'downloading', 'installing'].includes(store.phase))
const releaseUrl = computed(() => store.latest?.html_url || RELEASES_PAGE)
const sourceLabel = computed(() => {
  const source = store.result?.source ?? ''
  if (source.startsWith('api')) return t('updater.sourceApi')
  if (source.startsWith('redirect')) return t('updater.sourceRedirect')
  if (source === 'mirror') return t('updater.sourceMirror')
  return t('updater.sourceGithub')
})
const platformHint = computed(() => {
  const platform = store.settings?.platform
  if (platform === 'windows') return t('updater.platformWindows')
  if (platform === 'macos') return t('updater.platformMacos')
  return t('updater.platformLinux')
})
const progressText = computed(() => {
  if (store.total !== null && store.total > 0) {
    return `${formatBytes(store.downloaded)} / ${formatBytes(store.total)}`
  }
  return t('updater.downloadedBytes', { size: formatBytes(store.downloaded) })
})
const installButtonText = computed(() =>
  store.installOutcome?.kind === 'opened_installer'
    ? t('updater.openInstallerAgain')
    : t('updater.installNow')
)

watch(
  () => store.visible,
  (visible) => {
    if (visible) window.addEventListener('keydown', onKey)
    else window.removeEventListener('keydown', onKey)
  },
  { immediate: true }
)

function onKey(event: KeyboardEvent) {
  if (event.key === 'Escape' && store.visible) store.close()
}

onUnmounted(() => {
  window.removeEventListener('keydown', onKey)
})

function formatBytes(value: number): string {
  const units = ['B', 'KB', 'MB', 'GB']
  let size = Math.max(0, value)
  let unit = 0
  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024
    unit += 1
  }
  return `${size.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`
}

async function checkAgain() {
  await store.check(false)
}

async function startUpdate() {
  await store.startUpdate()
}

async function openRelease() {
  try {
    await open(releaseUrl.value)
  } catch (error) {
    notify(getErrorMessage(error, t('updater.releasePage')), 'error')
  }
}

async function openFolder() {
  const path = store.localPath
  if (!path) return
  const directory = path.replace(/[\\/][^\\/]*$/, '')
  try {
    await open(directory || path)
  } catch (error) {
    notify(getErrorMessage(error, t('updater.openFolder')), 'error')
  }
}
</script>

<style scoped>
.update-overlay {
  position: fixed;
  inset: 0;
  background: var(--v2-mask-bg);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2001;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.22s ease;
}
.update-overlay.active {
  opacity: 1;
  pointer-events: auto;
}
.update-dialog {
  width: 600px;
  max-width: 95vw;
  max-height: 86vh;
  background: var(--v2-surface);
  border: 1px solid var(--v2-surface-3);
  border-radius: var(--v2-r-lg);
  box-shadow: var(--v2-shadow-pop);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  transform: scale(0.94);
  opacity: 0;
  transition: transform 0.28s cubic-bezier(0.34, 1.56, 0.64, 1), opacity 0.22s;
}
.update-overlay.active .update-dialog {
  transform: scale(1);
  opacity: 1;
}
.update-header {
  padding: 18px 22px 12px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.update-title {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: var(--v2-fs-md);
  font-weight: var(--v2-fw-medium);
  color: var(--v2-text);
}
.update-current {
  font-size: var(--v2-fs-xs);
  color: var(--v2-text-3);
  font-weight: var(--v2-fw-regular);
}
.update-header-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}
.update-icon-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--v2-text-3);
  border-radius: var(--v2-r-sm);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.update-icon-btn:hover:not(:disabled) {
  background: var(--v2-surface-2);
  color: var(--v2-text);
}
.update-icon-btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.update-body {
  padding: 0 22px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.update-status {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 14px 16px;
  border-radius: var(--v2-r);
  background: var(--v2-surface-2);
  font-size: var(--v2-fs-sm);
  color: var(--v2-text-2);
}
.update-status-success {
  color: var(--v2-success);
  background: color-mix(in srgb, var(--v2-success) 9%, transparent);
}
.update-spinner {
  width: 13px;
  height: 13px;
  border: 2px solid var(--v2-surface-3);
  border-top-color: var(--v2-accent);
  border-radius: 50%;
  animation: update-spin 0.8s linear infinite;
  flex-shrink: 0;
}
@keyframes update-spin {
  to { transform: rotate(360deg); }
}
.update-error {
  padding: 12px 14px;
  border-radius: var(--v2-r);
  background: var(--v2-danger-bg);
  color: var(--v2-danger);
  font-size: var(--v2-fs-sm);
  line-height: 1.55;
  overflow-wrap: anywhere;
}
.update-error-text {
  white-space: pre-wrap;
}
.update-found {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.update-found-title {
  font-size: var(--v2-fs-md);
  font-weight: var(--v2-fw-semibold);
  color: var(--v2-text);
}
.update-found-meta {
  font-size: var(--v2-fs-xs);
  color: var(--v2-text-3);
}
.update-notes {
  border: 1px solid var(--v2-surface-3);
  border-radius: var(--v2-r);
  overflow: hidden;
}
.update-notes-title {
  padding: 9px 14px;
  background: var(--v2-surface-2);
  border-bottom: 1px solid var(--v2-surface-3);
  font-size: var(--v2-fs-xs);
  font-weight: var(--v2-fw-medium);
  color: var(--v2-text-2);
}
.update-notes-body {
  padding: 12px 14px;
  max-height: 250px;
  overflow-y: auto;
  font-size: var(--v2-fs-sm);
  color: var(--v2-text-2);
}
.update-hint {
  font-size: var(--v2-fs-xs);
  line-height: 1.55;
  color: var(--v2-text-3);
}
.update-hint-warning {
  color: var(--v2-warning);
}
.update-path {
  padding: 9px 12px;
  border-radius: var(--v2-r-sm);
  background: var(--v2-bg-base);
  font-size: var(--v2-fs-xs);
  color: var(--v2-text-2);
  overflow-wrap: anywhere;
}
.update-progress-row {
  display: flex;
  align-items: center;
  gap: 12px;
}
.update-progress-track {
  flex: 1;
  height: 6px;
  border-radius: 999px;
  background: var(--v2-surface-2);
  overflow: hidden;
  position: relative;
}
.update-progress-track span {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--v2-accent);
  transition: width 0.2s ease;
}
.update-progress-track.indeterminate span {
  width: 35%;
  animation: update-indeterminate 1.1s ease-in-out infinite;
}
@keyframes update-indeterminate {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(320%); }
}
.update-progress-text {
  min-width: 132px;
  text-align: right;
  font-size: var(--v2-fs-xs);
  color: var(--v2-text-3);
}
.update-muted {
  color: var(--v2-text-3);
}
.update-footer {
  padding: 16px 22px 20px;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
  row-gap: 8px;
}
.update-footer-gap {
  flex: 1;
}
.update-footer .v2-btn {
  white-space: nowrap;
}
</style>
