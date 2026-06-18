<template>
  <div class="ses-page">
    <div class="v2-tabs ses-tabs">
      <div v-for="cli in CLI_TABS" :key="cli.id" class="v2-tab" :class="{ active: activeCliType === cli.id }" @click="handleCliChange(cli.id)">
        <span class="tab-label-text">{{ cli.label }}</span>
      </div>
    </div>

    <!-- 项目列表 -->
    <template v-if="!currentProject">
      <div v-loading="sessionStore.loading" class="ses-body">
        <V2Empty class="v2-card" v-if="sessionStore.projects.length === 0" title="暂无项目" description="使用 Agent 产生会话后，会按项目分组显示">
          <template #icon><svg width="40" height="40" viewBox="0 0 24 24"><path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/></svg></template>
        </V2Empty>
        <div v-else class="v2-card ses-plist">
          <div v-for="project in sessionStore.projects" :key="project.name" class="ses-prow" @click="handleProjectClick(project)">
            <div class="ses-picon"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/></svg></div>
            <div class="ses-pinfo">
              <div class="ses-pname">{{ getProjectTitle(project) }}</div>
              <div class="ses-ppath mono" :title="project.full_path">{{ project.full_path }}</div>
            </div>
            <div class="ses-pmeta"><span>{{ project.session_count }} 个会话</span><span class="mono">{{ formatSize(project.total_size) }}</span></div>
            <el-tooltip content="删除项目" placement="top" effect="light" :show-after="250">
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
          <el-tooltip content="返回" placement="top" effect="light" :show-after="250">
            <button class="v2-row-act" @click="handleBackToProjects"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 19-7-7 7-7"/><path d="M19 12H5"/></svg></button>
          </el-tooltip>
          <div class="ses-listhead-main">
            <div class="ses-listhead-t">{{ getProjectTitle(sessionStore.currentProjectInfo) }}</div>
            <div class="ses-listhead-sub">
              <span class="mono" :title="sessionStore.currentProjectInfo?.full_path">{{ sessionStore.currentProjectInfo?.full_path }}</span>
              <span class="ses-sdot">•</span>
              <span>{{ sessionStore.sessionTotal }} 个会话</span>
            </div>
          </div>
        </div>
        <input v-model="sessionSearchQuery" class="v2-input v2-input-surface ses-search" placeholder="搜索会话…">
      </div>

      <div v-loading="sessionStore.loading" class="ses-body">
        <V2Empty class="v2-card" v-if="filteredSessions.length === 0" title="暂无会话">
          <template #icon><svg width="40" height="40" viewBox="0 0 24 24"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg></template>
        </V2Empty>
        <div v-else class="v2-card ses-slist">
          <div v-for="session in filteredSessions" :key="session.session_id" class="ses-srow" @click="handleSessionClick(session)">
            <div class="ses-sicon"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg></div>
            <div class="ses-sinfo">
              <div class="ses-stitle">{{ session.first_message ? truncateText(session.first_message, 160) : '无消息内容' }}</div>
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
            <el-tooltip content="删除会话" placement="top" effect="light" :show-after="250">
              <button class="v2-row-act danger" @click.stop="handleDeleteSession(session)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg></button>
            </el-tooltip>
          </div>
        </div>
      </div>
    </template>

    <V2Drawer v-model="showSessionDrawer" :title="`会话 ${currentSessionId.slice(0, 8)}`" :show-footer="false" width="80%">
      <div v-loading="sessionStore.loading" class="ses-chat">
        <div v-if="sessionStore.messages.length === 0" class="v2-hint" style="text-align:center;padding:40px">暂无消息</div>
        <div v-for="(msg, index) in sessionStore.messages" :key="index" class="ses-bubble" :class="msg.role === 'user' ? 'is-user' : 'is-bot'">
          <div class="ses-role">{{ msg.role === 'user' ? 'USER' : 'ASSISTANT' }}</div>
          <div class="ses-bubble-c">
            <el-tooltip content="复制" placement="top" effect="light" :show-after="250">
              <button class="ses-copy" @click="handleCopyMessage(msg.content)"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="14" x="8" y="8" rx="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg></button>
            </el-tooltip>
            {{ getDisplayContent(msg.content, index) }}
            <div v-if="isLongMessage(msg.content)" class="ses-expand" @click="toggleExpand(index)">{{ expandedMessages.has(index) ? '收起' : '展开全部' }}</div>
          </div>
        </div>
      </div>
    </V2Drawer>
  </div>
</template>

<script setup lang="ts">
import V2Drawer from '@/components/V2Drawer.vue'
import V2Empty from '@/components/V2Empty.vue'
import { confirm } from '@/utils/confirm'
import { notify } from '@/utils/notification'
import { useSessionStore } from '@/stores/sessions'
import { useUiStore } from '@/stores/ui'
import { CLI_TABS } from '@/types/models'
import type { CliType } from '@/types/models'
import type { ProjectInfo, SessionInfo } from '@/api/sessions'

const sessionStore = useSessionStore()
const uiStore = useUiStore()

const activeCliType = computed({
  get: () => uiStore.sessionsActiveCliType,
  set: (val) => uiStore.setSessionsActiveCliType(val)
})
const currentProject = computed(() => sessionStore.currentProject)
const sessionSearchQuery = ref('')
const showSessionDrawer = ref(false)
const currentSessionId = ref('')
const expandedMessages = ref(new Set<number>())

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
  sessionStore.fetchMessages(sessionStore.currentProject, session.session_id)
}
async function handleDeleteProject(project: ProjectInfo) {
  try {
    await confirm(`确定删除项目 "${project.display_name}" 及其所有会话吗？此操作不可恢复！`, '确认删除')
    await sessionStore.deleteProject(project.name)
    notify('项目已删除')
  } catch (e: any) {
    if (e !== 'cancel' && e?.toString() !== 'cancel') notify(e?.message || e?.toString() || '删除失败', 'error')
  }
}
async function handleDeleteSession(session: SessionInfo) {
  try {
    await confirm(`确定删除会话 "${session.session_id.substring(0, 8)}..." 吗？此操作不可恢复！`, '确认删除')
    await sessionStore.deleteSession(sessionStore.currentProject, session.session_id)
    notify('会话已删除')
  } catch (e: any) {
    if (e !== 'cancel' && e?.toString() !== 'cancel') notify(e?.message || e?.toString() || '删除失败', 'error')
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
  return new Date(timestamp * 1000).toLocaleString('zh-CN')
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
    notify('已复制')
  } catch {
    notify('复制失败', 'error')
  }
}
const MAX_LINES = 10
function normalizeContent(content: string): string {
  return content ? content.replace(/\\n/g, '\n') : ''
}
function isLongMessage(content: string): boolean {
  return content ? normalizeContent(content).split('\n').length > MAX_LINES : false
}
function getDisplayContent(content: string, index: number): string {
  const normalized = normalizeContent(content)
  if (expandedMessages.value.has(index) || !isLongMessage(content)) return normalized
  return normalized.split('\n').slice(0, MAX_LINES).join('\n')
}
function toggleExpand(index: number) {
  if (expandedMessages.value.has(index)) expandedMessages.value.delete(index)
  else expandedMessages.value.add(index)
}

onMounted(() => {
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

.ses-chat { display: flex; flex-direction: column; gap: 20px; }
.ses-bubble { max-width: 92%; display: flex; flex-direction: column; gap: 5px; }
.ses-bubble.is-user { align-self: flex-end; }
.ses-bubble.is-bot { align-self: flex-start; }
.ses-role { font-size: var(--v2-fs-xs); font-weight: var(--v2-fw-semibold); color: var(--v2-text-3); }
.ses-bubble.is-user .ses-role { text-align: right; }
.ses-bubble-c { position: relative; padding: 12px 16px; padding-right: 38px; border: 1px solid transparent; border-radius: var(--v2-r); font-size: var(--v2-fs-sm); line-height: 1.6; white-space: pre-wrap; word-break: break-word; }
.ses-bubble.is-user .ses-bubble-c { background: var(--v2-selected-bg); color: var(--v2-text); border-bottom-right-radius: 3px; }
.ses-bubble.is-bot .ses-bubble-c { background: var(--v2-surface-2); border-color: var(--v2-surface-3); color: var(--v2-text); border-bottom-left-radius: 3px; }
.ses-copy { position: absolute; top: 8px; right: 8px; width: 26px; height: 26px; display: flex; align-items: center; justify-content: center; border: none; background: transparent; color: var(--v2-text-3); border-radius: 6px; cursor: pointer; opacity: 0; transition: opacity 0.15s; }
.ses-bubble-c:hover .ses-copy { opacity: 1; }
.ses-copy:hover { background: var(--v2-surface-3); color: var(--v2-text); }
.ses-expand { margin: 10px -16px 0; border-top: 1px solid var(--v2-surface-3); padding: 8px 16px 0; color: var(--v2-text-3); font-size: var(--v2-fs-xs); font-weight: var(--v2-fw-medium); text-align: center; cursor: pointer; }
.ses-expand:hover { color: var(--v2-text); }
</style>
