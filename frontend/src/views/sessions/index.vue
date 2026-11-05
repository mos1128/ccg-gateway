<template>
  <div class="ses-page">
    <V2Tabs class="ses-tabs">
      <div v-for="cli in sessionTabs" :key="cli.id" class="v2-tab" :class="{ active: activeCliType === cli.id }" @click="handleCliChange(cli.id)">
        <span class="tab-label-text">{{ cli.label }}</span>
      </div>
    </V2Tabs>

    <!-- 项目列表 -->
    <template v-if="!currentProject">
      <div v-loading="sessionStore.loading" class="ses-body">
        <V2Empty class="v2-card" v-if="sessionStore.projects.length === 0" :title="t('sessions.emptyProjects')" :description="t('sessions.emptyProjectsDesc')">
          <template #icon><svg width="40" height="40" viewBox="0 0 24 24"><path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/></svg></template>
        </V2Empty>
        <div v-else class="v2-card ses-plist">
          <div v-for="project in sessionStore.projects" :key="project.name" class="ses-prow" @click="handleProjectClick(project)">
            <div class="ses-picon"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/></svg></div>
            <div class="ses-pinfo">
              <div class="ses-pname">{{ getProjectTitle(project) }}</div>
              <div class="ses-ppath mono"><OverflowText :text="project.full_path" /></div>
            </div>
            <div class="ses-pmeta"><span>{{ t('sessions.sessionCount', { count: project.session_count }) }}</span><span class="mono">{{ formatSize(project.total_size) }}</span></div>
            <el-tooltip v-if="!sessionReadOnly" :content="t('sessions.deleteProject')" placement="top" effect="light" :show-after="250">
              <button class="v2-row-act danger ses-pdel" @click.stop="handleDeleteProject(project)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg></button>
            </el-tooltip>
          </div>
        </div>
      </div>
    </template>

    <!-- 会话列表 -->
    <template v-else>
      <div class="ses-listhead">
        <div class="ses-listhead-l">
          <el-tooltip :content="t('sessions.back')" placement="top" effect="light" :show-after="250">
            <button class="v2-row-act" @click="handleBackToProjects"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 19-7-7 7-7"/><path d="M19 12H5"/></svg></button>
          </el-tooltip>
          <div class="ses-listhead-main">
            <div class="ses-listhead-t">{{ getProjectTitle(sessionStore.currentProjectInfo) }}</div>
            <div class="ses-listhead-sub">
              <span class="mono"><OverflowText :text="sessionStore.currentProjectInfo?.full_path || ''" /></span>
              <span class="ses-sdot">•</span>
              <span>{{ t('sessions.sessionCount', { count: sessionStore.sessionTotal }) }}</span>
            </div>
          </div>
        </div>
        <input v-model="sessionSearchQuery" class="v2-input v2-input-surface ses-search" :placeholder="t('sessions.searchPlaceholder')">
      </div>

      <div v-loading="sessionStore.loading" class="ses-body">
        <V2Empty class="v2-card" v-if="filteredSessions.length === 0" :title="t('sessions.emptySessions')">
          <template #icon><svg width="40" height="40" viewBox="0 0 24 24"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg></template>
        </V2Empty>
        <div v-else class="v2-card ses-slist">
          <div v-for="session in filteredSessions" :key="session.session_id" class="ses-srow" @click="handleSessionClick(session)">
            <div class="ses-sicon"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg></div>
            <div class="ses-sinfo">
              <div class="ses-stitle">{{ session.first_message ? truncateText(session.first_message, 160) : t('sessions.noMessageContent') }}</div>
              <div class="ses-smeta">
                <el-tooltip :content="session.session_id" placement="top" effect="light" :show-after="250">
                  <span class="mono ses-sid">{{ formatSessionId(session.session_id) }}</span>
                </el-tooltip>
                <span v-if="session.git_branch" class="v2-pill v2-pill-neutral">
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" style="opacity: 0.8; flex-shrink: 0;"><line x1="6" y1="3" x2="6" y2="15"></line><circle cx="18" cy="6" r="3"></circle><circle cx="6" cy="18" r="3"></circle><path d="M18 9a9 9 0 0 1-9 9"></path></svg>
                  {{ session.git_branch }}
                </span>
              </div>
            </div>
            <div class="ses-smeta-r">
              <span>{{ formatTime(session.mtime) }}</span>
              <span class="mono">{{ formatSize(session.size) }}</span>
            </div>
            <el-tooltip v-if="!sessionReadOnly" :content="t('sessions.deleteSession')" placement="top" effect="light" :show-after="250">
              <button class="v2-row-act danger" @click.stop="handleDeleteSession(session)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg></button>
            </el-tooltip>
          </div>
        </div>
      </div>
    </template>

    <V2Drawer v-model="showSessionDrawer" :title="currentSessionTitle" :show-footer="false" width="80%">
      <template #actions>
        <button
          class="ses-injected-toggle"
          :class="{ active: showInjected }"
          type="button"
          role="switch"
          :aria-checked="showInjected"
          :aria-label="t('sessions.injectedContext')"
          :title="showInjected ? t('sessions.hideInjected') : t('sessions.showInjected')"
          @click="showInjected = !showInjected"
        >
          <svg v-if="showInjected" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0"/><circle cx="12" cy="12" r="3"/></svg>
          <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49"/><path d="M14.084 14.158a3 3 0 0 1-4.242-4.242"/><path d="M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143"/><path d="m2 2 20 20"/></svg>
          <span class="ses-injected-label">{{ t('sessions.injectedContext') }}</span>
          <span v-if="injectedCount" class="ses-injected-count">{{ injectedCount }}</span>
          <span class="ses-switch" :class="{ on: showInjected }"><span class="ses-switch-knob"></span></span>
        </button>
      </template>
      <div v-loading="sessionStore.loading" class="ses-chat">
        <div v-if="visibleMessages.length === 0" class="v2-hint" style="text-align:center;padding:40px">
          {{ sessionStore.messages.length === 0 ? t('sessions.emptyMessages') : t('sessions.noVisibleMessages') }}
        </div>
        <div v-for="(msg, index) in visibleMessages" :key="index" class="ses-bubble" :class="msg.role === 'user' ? 'is-user' : 'is-bot'">
          <div class="ses-role">
            <span class="ses-avatar">{{ msg.role === 'user' ? 'U' : 'AI' }}</span>
            <span>{{ msg.role === 'user' ? t('sessions.roleUser') : t('sessions.roleAssistant') }}</span>
          </div>
          <div class="ses-bubble-c">
            <el-tooltip :content="t('common.copy')" placement="top" effect="light" :show-after="250">
              <button class="ses-copy" @click="handleCopyMessage(msg.content)"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="14" x="8" y="8" rx="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg></button>
            </el-tooltip>
            <template v-for="(segment, segmentIndex) in messageSegments(msg.content, index)" :key="segmentIndex">
              <template v-if="segment.type === 'image'">
                <div v-if="failedImages.has(imageKey(index, segmentIndex))" class="ses-image-error">{{ t('sessions.imageLoadFailed') }}</div>
                <img v-else :src="segment.src || ''" :alt="segment.alt || ''" class="ses-image" loading="lazy" @error="failedImages.add(imageKey(index, segmentIndex))" />
              </template>
              <div v-else-if="segment.type === 'image_failed'" class="ses-image-error">{{ t('sessions.imageLoadFailed') }}</div>
              <V2Markdown v-else :content="segment.text || ''" />
            </template>
            <div v-if="isLongMessage(msg.content)" class="ses-expand" @click="toggleExpand(index)">{{ expandedMessages.has(index) ? t('sessions.collapse') : t('sessions.expandAll') }}</div>
          </div>
        </div>
      </div>
    </V2Drawer>
  </div>
</template>

<script setup lang="ts">
import { convertFileSrc } from '@tauri-apps/api/core'
import V2Drawer from '@/components/V2Drawer.vue'
import V2Markdown from '@/components/V2Markdown.vue'
import V2Empty from '@/components/V2Empty.vue'
import V2Tabs from '@/components/V2Tabs.vue'
import OverflowText from '@/components/OverflowText.vue'
import { confirm } from '@/utils/confirm'
import { notify } from '@/utils/notification'
import { useSessionStore } from '@/stores/sessions'
import { useUiStore } from '@/stores/ui'
import { useAgentStore } from '@/stores/agents'
import { useI18n } from 'vue-i18n'
import type { CliType } from '@/types/models'
import type { ProjectInfo, SessionInfo } from '@/api/sessions'

const sessionStore = useSessionStore()
const uiStore = useUiStore()
const agentStore = useAgentStore()
const { t, locale } = useI18n()
const sessionTabs = computed(() => agentStore.visibleAgents
  .filter((agent) => agent.features.sessions.enabled)
  .map((agent) => ({ id: agent.id, label: agent.name })))

const activeCliType = computed({
  get: () => uiStore.sessionsActiveCliType,
  set: (val) => uiStore.setSessionsActiveCliType(val)
})
const sessionReadOnly = computed(() => {
  const adapter = agentStore.get(activeCliType.value)?.features.sessions.adapter
  return ['opencode', 'zcode', 'dsh'].includes(adapter || '')
})
const currentProject = computed(() => sessionStore.currentProject)
const sessionSearchQuery = ref('')
const showSessionDrawer = ref(false)
const currentSessionId = ref('')
const currentSessionTitle = computed(() => {
  const session = sessionStore.sessions.find((item) => item.session_id === currentSessionId.value)
  const title = session?.first_message?.trim()
  return title || currentSessionId.value.slice(0, 8)
})
const expandedMessages = ref(new Set<number>())
const showInjected = ref(localStorage.getItem('ccg.sessions.showInjected') === 'true')
const failedImages = reactive(new Set<string>())
const visibleMessages = computed(() => sessionStore.messages.filter((message) => showInjected.value || message.kind !== 'injected'))
const injectedCount = computed(() => sessionStore.messages.filter((message) => message.kind === 'injected').length)

watch(showInjected, (value) => {
  localStorage.setItem('ccg.sessions.showInjected', String(value))
  expandedMessages.value.clear()
  failedImages.clear()
})

function handleCliChange(cliType: CliType) {
  activeCliType.value = cliType
  if (!sessionStore.currentProject && sessionStore.projects.length === 0) sessionStore.fetchProjects(1)
}

const filteredSessions = computed(() => {
  if (!sessionSearchQuery.value) return sessionStore.sessions
  const q = sessionSearchQuery.value.toLowerCase()
  return sessionStore.sessions.filter(s =>
    s.session_id.toLowerCase().includes(q) || s.first_message?.toLowerCase().includes(q) || s.git_branch?.toLowerCase().includes(q))
})

function handleProjectClick(project: ProjectInfo) {
  sessionStore.fetchSessions(project.name, 1, project)
}
function getProjectTitle(project?: ProjectInfo | null): string {
  if (!project) return ''
  const path = project.full_path?.replace(/[\\/]+$/, '')
  const name = path?.split(/[\\/]/).pop()
  return name || project.display_name || project.name
}
function handleBackToProjects() {
  sessionStore.clearSessions()
}
function handleSessionClick(session: SessionInfo) {
  currentSessionId.value = session.session_id
  showSessionDrawer.value = true
  expandedMessages.value.clear()
  failedImages.clear()
  sessionStore.fetchMessages(sessionStore.currentProject, session.session_id)
}
async function handleDeleteProject(project: ProjectInfo) {
  try {
    await confirm(t('sessions.deleteProjectConfirm', { name: project.display_name }), t('sessions.confirmDeleteTitle'))
    await sessionStore.deleteProject(project.name)
    notify(t('sessions.projectDeleted'))
  } catch (e: any) {
    if (e !== 'cancel' && e?.toString() !== 'cancel') notify(e?.message || e?.toString() || t('sessions.deleteFailed'), 'error')
  }
}
async function handleDeleteSession(session: SessionInfo) {
  try {
    await confirm(t('sessions.deleteSessionConfirm', { id: session.session_id.substring(0, 8) }), t('sessions.confirmDeleteTitle'))
    await sessionStore.deleteSession(sessionStore.currentProject, session.session_id)
    notify(t('sessions.sessionDeleted'))
  } catch (e: any) {
    if (e !== 'cancel' && e?.toString() !== 'cancel') notify(e?.message || e?.toString() || t('sessions.deleteFailed'), 'error')
  }
}
function formatSize(bytes: number): string {
  if (!bytes) return '0 B'
  const k = 1024
  if (bytes < k) return bytes + ' B'
  if (bytes < k * k) return (bytes / k).toFixed(1) + ' KB'
  return (bytes / k / k).toFixed(1) + ' MB'
}
function formatTime(timestamp: number): string {
  if (!timestamp) return ''
  return new Date(timestamp * 1000).toLocaleString(locale.value)
}
function truncateText(text: string, maxLength: number): string {
  if (!text) return ''
  return text.length > maxLength ? text.substring(0, maxLength) + '...' : text
}
function formatSessionId(id: string): string {
  if (!id) return ''
  if (id.length > 30) {
    return id.substring(0, 24) + '...' + id.substring(id.length - 6)
  }
  return id
}
async function handleCopyMessage(content: string) {
  try {
    await navigator.clipboard.writeText(normalizeContent(content))
    notify(t('sessions.copied'))
  } catch {
    notify(t('sessions.copyFailed'), 'error')
  }
}
const MAX_LINES = 10
interface MessageSegment {
  type: 'text' | 'image' | 'image_failed'
  text?: string
  src?: string
  alt?: string
}
function imageKey(index: number, segmentIndex: number): string {
  return `${index}:${segmentIndex}`
}
function pushTextSegments(text: string, segments: MessageSegment[]) {
  const placeholder = /\[图片\]|\[Image\]|\[image\]/g
  let lastIndex = 0
  for (const match of text.matchAll(placeholder)) {
    const start = match.index ?? 0
    if (start > lastIndex) {
      segments.push({ type: 'text', text: text.slice(lastIndex, start) })
    }
    segments.push({ type: 'image_failed' })
    lastIndex = start + match[0].length
  }
  if (lastIndex < text.length) {
    segments.push({ type: 'text', text: text.slice(lastIndex) })
  }
}
function normalizeContent(content: string): string {
  return content ? content.replace(/\\n/g, '\n') : ''
}
function hasImageTag(content: string): boolean {
  return /<image\b/i.test(normalizeContent(content))
}
function isLongMessage(content: string): boolean {
  if (!content || hasImageTag(content)) return false
  return normalizeContent(content).split('\n').length > MAX_LINES
}
function getDisplayContent(content: string, index: number): string {
  const normalized = normalizeContent(content)
  if (expandedMessages.value.has(index) || !isLongMessage(content)) return normalized
  return normalized.split('\n').slice(0, MAX_LINES).join('\n')
}
function messageSegments(content: string, index: number): MessageSegment[] {
  const display = hasImageTag(content) ? normalizeContent(content) : getDisplayContent(content, index)
  const imageTag = /<image\b([^>]*?)(?:\/>|>([\s\S]*?)<\/image>)/gi
  const segments: MessageSegment[] = []
  let lastIndex = 0

  for (const match of display.matchAll(imageTag)) {
    const start = match.index ?? 0
    if (start > lastIndex) {
      pushTextSegments(display.slice(lastIndex, start), segments)
    }
    const attrs = match[1] || ''
    const pathMatch = attrs.match(/\bpath\s*=\s*(?:"([^"]+)"|'([^']+)')/i)
    const path = pathMatch?.[1] || pathMatch?.[2] || ''
    if (path && /\.(png|jpe?g|gif|webp|bmp|svg)$/i.test(path)) {
      const nameMatch = attrs.match(/\bname\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s]+))/i)
      const alt = nameMatch?.[1] || nameMatch?.[2] || nameMatch?.[3] || 'image'
      try {
        segments.push({ type: 'image', src: convertFileSrc(path.replace(/\\/g, '/')), alt })
      } catch {
        segments.push({ type: 'image_failed' })
      }
    } else {
      segments.push({ type: 'image_failed' })
    }
    lastIndex = start + match[0].length
  }

  if (lastIndex < display.length) {
    pushTextSegments(display.slice(lastIndex), segments)
  }
  return segments.length ? segments : [{ type: 'text', text: display }]
}
function toggleExpand(index: number) {
  if (expandedMessages.value.has(index)) expandedMessages.value.delete(index)
  else expandedMessages.value.add(index)
}

onMounted(async () => {
  if (!agentStore.agents.length) await agentStore.fetchAgents()
  if (!sessionTabs.value.some((tab) => tab.id === activeCliType.value) && sessionTabs.value.length) {
    activeCliType.value = sessionTabs.value[0].id
  }
  sessionStore.fetchProjects(1)
})
</script>

<style scoped>
.ses-page { flex: 1; min-height: 0; display: flex; flex-direction: column; margin-top: -16px; }
.ses-tabs { margin-bottom: 16px; flex-shrink: 0; }
.ses-body { flex: 1; overflow-y: auto; min-height: 0; }
.ses-empty { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 9px; padding: 60px; color: var(--v2-text-3); }
.ses-empty-t { font-size: var(--v2-fs-base); font-weight: var(--v2-fw-medium); color: var(--v2-text-2); }

.ses-plist { overflow: hidden; }
.ses-prow { display: grid; grid-template-columns: auto minmax(0, 1fr) auto auto; align-items: center; gap: 14px; padding: 14px 16px; border-bottom: 1px solid var(--v2-surface-2); cursor: pointer; transition: background 0.15s; }
.ses-prow:last-child { border-bottom: none; }
.ses-prow:hover { background: var(--v2-surface-2); }
.ses-picon { width: 36px; height: 36px; border-radius: 9px; background: var(--v2-surface-2); color: var(--v2-text-2); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.ses-picon svg { fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; }
.ses-pinfo { flex: 1; min-width: 0; }
.ses-pname { font-size: var(--v2-fs-base); font-weight: var(--v2-fw-medium); color: var(--v2-text); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-bottom: 4px; }
.ses-ppath { font-size: var(--v2-fs-xs); color: var(--v2-text-3); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.ses-pmeta { display: flex; align-items: center; gap: 14px; font-size: var(--v2-fs-xs); color: var(--v2-text-2); white-space: nowrap; }
.ses-pdel { flex-shrink: 0; opacity: 0; transition: opacity 0.15s; }
.ses-prow:hover .ses-pdel { opacity: 1; }

.ses-listhead { display: flex; align-items: center; justify-content: space-between; gap: 16px; margin-bottom: 14px; flex-shrink: 0; }
.ses-listhead-l { display: flex; align-items: center; gap: 12px; min-width: 0; }
.ses-listhead-main { min-width: 0; }
.ses-listhead-t { font-size: var(--v2-fs-base); font-weight: var(--v2-fw-medium); color: var(--v2-text); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 620px; }
.ses-listhead-sub { display: flex; align-items: center; gap: 8px; margin-top: 4px; font-size: var(--v2-fs-xs); color: var(--v2-text-3); min-width: 0; }
.ses-listhead-sub .mono { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.ses-search { width: 240px; flex-shrink: 0; }

.ses-slist { padding: 4px 0; }
.ses-srow { display: grid; grid-template-columns: auto minmax(0, 1fr) auto auto; align-items: center; gap: 14px; padding: 15px 16px; border-bottom: 1px solid var(--v2-surface-2); cursor: pointer; transition: background 0.15s; }
.ses-srow:last-child { border-bottom: none; }
.ses-srow:hover { background: var(--v2-surface-2); }
.ses-sicon { width: 36px; height: 36px; border-radius: 50%; background: var(--v2-surface-2); color: var(--v2-text-2); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.ses-sicon svg { fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; }
.ses-sinfo { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
.ses-stitle { font-size: var(--v2-fs-sm); font-weight: var(--v2-fw-medium); color: var(--v2-text); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.ses-smeta { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; font-size: var(--v2-fs-xs); color: var(--v2-text-2); margin-top: 2px; }
.ses-smeta-r { display: flex; flex-direction: column; align-items: flex-end; gap: 4px; color: var(--v2-text-3); font-size: var(--v2-fs-xs); white-space: nowrap; }
.ses-sid { color: var(--v2-text-3); }
.ses-sdot { color: var(--v2-text-3); opacity: 0.5; }
.ses-srow .v2-row-act.danger { opacity: 0; transition: opacity 0.15s; }
.ses-srow:hover .v2-row-act.danger { opacity: 1; }

.ses-injected-toggle { display: inline-flex; align-items: center; gap: 7px; height: 30px; padding: 0 10px; border: 1px solid var(--v2-surface-3); border-radius: 999px; background: transparent; color: var(--v2-text-2); font: inherit; font-size: var(--v2-fs-xs); cursor: pointer; transition: background 0.15s, border-color 0.15s, color 0.15s; }
.ses-injected-toggle:hover { background: var(--v2-surface-2); color: var(--v2-text); }
.ses-injected-toggle.active { background: var(--v2-selected-bg); border-color: color-mix(in srgb, var(--v2-accent) 42%, transparent); color: var(--v2-accent); }
.ses-injected-toggle svg { flex-shrink: 0; opacity: 0.9; }
.ses-injected-label { white-space: nowrap; }
.ses-injected-count { display: inline-flex; align-items: center; justify-content: center; min-width: 16px; height: 16px; padding: 0 5px; border-radius: 999px; background: var(--v2-surface-3); color: var(--v2-text-2); font-size: 10px; line-height: 1; }
.ses-injected-toggle.active .ses-injected-count { background: color-mix(in srgb, var(--v2-accent) 18%, transparent); color: var(--v2-accent); }
.ses-switch { position: relative; width: 28px; height: 16px; border-radius: 999px; background: var(--v2-surface-3); flex-shrink: 0; transition: background 0.15s; }
.ses-switch.on { background: var(--v2-accent); }
.ses-switch-knob { position: absolute; top: 2px; left: 2px; width: 12px; height: 12px; border-radius: 50%; background: var(--v2-text-3); transition: transform 0.15s, background 0.15s; }
.ses-switch.on .ses-switch-knob { transform: translateX(12px); background: #fff; }
.ses-chat { display: flex; flex-direction: column; gap: 24px; padding-bottom: 24px; }
.ses-bubble { max-width: min(780px, 88%); display: flex; flex-direction: column; gap: 7px; }
.ses-bubble.is-user { align-self: flex-end; }
.ses-bubble.is-bot { align-self: flex-start; }
.ses-role { display: flex; align-items: center; gap: 7px; color: var(--v2-text-3); font-size: var(--v2-fs-xs); font-weight: var(--v2-fw-medium); }
.ses-bubble.is-user .ses-role { flex-direction: row-reverse; }
.ses-avatar { display: inline-flex; align-items: center; justify-content: center; width: 18px; height: 18px; border: 1px solid var(--v2-surface-3); border-radius: 50%; background: var(--v2-surface-2); color: var(--v2-text-2); font-size: 9px; font-weight: 700; line-height: 1; flex-shrink: 0; }
.ses-bubble.is-user .ses-avatar { background: var(--v2-selected-bg); border-color: color-mix(in srgb, var(--v2-accent) 35%, transparent); color: var(--v2-accent); }
.ses-bubble-c { position: relative; padding: 13px 17px; padding-right: 40px; border: 1px solid transparent; border-radius: 14px; font-size: var(--v2-fs-sm); line-height: 1.6; white-space: normal; word-break: break-word; box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05); }
.ses-bubble.is-user .ses-bubble-c { background: var(--v2-selected-bg); color: var(--v2-text); border-color: color-mix(in srgb, var(--v2-accent) 16%, var(--v2-surface-3)); border-bottom-right-radius: 4px; }
.ses-bubble.is-bot .ses-bubble-c { background: var(--v2-surface-2); border-color: var(--v2-surface-3); color: var(--v2-text); border-bottom-left-radius: 4px; }
.ses-image { display: block; max-width: 100%; max-height: 480px; margin: 8px 0; border-radius: 8px; object-fit: contain; background: var(--v2-surface-3); }
.ses-image-error { display: inline-flex; align-items: center; margin: 8px 0; padding: 8px 10px; border: 1px dashed var(--v2-surface-3); border-radius: 6px; color: var(--v2-text-3); font-size: var(--v2-fs-xs); }
.ses-copy { position: absolute; top: 9px; right: 9px; width: 26px; height: 26px; display: flex; align-items: center; justify-content: center; border: none; background: transparent; color: var(--v2-text-3); border-radius: 6px; cursor: pointer; opacity: 0; transition: opacity 0.15s; }
.ses-bubble-c:hover .ses-copy { opacity: 1; }
.ses-copy:hover { background: var(--v2-surface-3); color: var(--v2-text); }
.ses-expand { margin: 10px -17px 0; border-top: 1px solid var(--v2-surface-3); padding: 8px 17px 0; color: var(--v2-text-3); font-size: var(--v2-fs-xs); font-weight: var(--v2-fw-medium); text-align: center; cursor: pointer; }
.ses-expand:hover { color: var(--v2-text); }
@media (max-width: 720px) {
  .ses-injected-label { display: none; }
}
</style>
