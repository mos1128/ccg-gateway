<template>
  <div class="sessions-page">
    <svg style="display:none">
      <defs>
        <symbol id="icon-folder" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/>
        </symbol>
        <symbol id="icon-chat" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
        </symbol>
        <symbol id="icon-trash" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/><line x1="10" x2="10" y1="11" y2="17"/><line x1="14" x2="14" y1="11" y2="17"/>
        </symbol>
        <symbol id="icon-search" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>
        </symbol>
        <symbol id="icon-branch" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="6" x2="6" y1="3" y2="15"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M18 9a9 9 0 0 1-9 9"/>
        </symbol>
        <symbol id="icon-back" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="m12 19-7-7 7-7"/><path d="M19 12H5"/>
        </symbol>
        <symbol id="icon-copy" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/>
        </symbol>
        <symbol id="icon-close" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line>
        </symbol>
      </defs>
    </svg>

    <!-- Top Level Tabs -->
    <div class="top-tabs">
      <div 
        v-for="cli in [{label: 'Claude Code', name: 'claude_code'}, {label: 'Codex', name: 'codex'}, {label: 'Gemini', name: 'gemini'}]" 
        :key="cli.name"
        :class="['tab-item', { active: activeCliType === cli.name }]"
        @click="handleCliChange(cli.name)"
      >
        {{ cli.label }}
      </div>
    </div>

    <!-- Project List View -->
    <div v-if="!currentProject" class="project-list">
      <div v-loading="sessionStore.loading" class="list-container">
        <template v-if="sessionStore.projects.length === 0">
          <el-empty description="暂无项目" />
        </template>
        <div v-else class="scroll-area">
          <div class="project-grid">
            <div
              v-for="project in sessionStore.projects"
              :key="project.name"
              class="project-card"
              @click="handleProjectClick(project)"
            >
              <div class="project-icon-box">
                <svg width="24" height="24"><use href="#icon-folder"/></svg>
              </div>
              <div class="project-info">
                <div class="project-path-title">{{ project.full_path }}</div>
                <div class="project-meta">
                  <div class="text-12 text-muted">{{ project.session_count }} 个会话</div>
                  <span class="text-12 text-muted">{{ formatSize(project.total_size) }}</span>
                </div>
              </div>
              <div class="ghost-delete" @click.stop="handleDeleteProject(project)">
                <svg width="16" height="16"><use href="#icon-trash"/></svg>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Session List View -->
    <div v-else class="session-list">
      <div class="page-header">
        <div style="display: flex; align-items: center; gap: 16px;">
          <button class="b-button-outline" style="border: none; background: transparent; box-shadow: none; padding: 6px; color: #64748b;" @click="handleBackToProjects">
            <svg width="20" height="20"><use href="#icon-back"/></svg>
          </button>
          <div>
            <div class="page-title">{{ sessionStore.currentProjectInfo?.full_path }}</div>
            <div class="text-14 text-muted">{{ sessionStore.sessionTotal }} 个会话</div>
          </div>
        </div>
        <div class="search-box" style="width: 260px; position: relative;">
          <svg class="search-icon" width="16" height="16" style="position: absolute; left: 12px; top: 50%; transform: translateY(-50%); color: #94a3b8; pointer-events: none; z-index: 1;"><use href="#icon-search"/></svg>
          <input type="text" v-model="sessionSearchQuery" class="c-input" placeholder="搜索..." style="height: 38px; padding: 0 12px 0 36px; margin: 0; box-shadow: none;">
        </div>
      </div>

      <div v-loading="sessionStore.loading" class="list-container">
        <template v-if="filteredSessions.length === 0">
          <el-empty description="暂无会话" />
        </template>
        <div v-else class="scroll-area">
          <div style="display: flex; flex-direction: column;">
            <div
              v-for="session in filteredSessions"
              :key="session.session_id"
              class="session-card"
              @click="handleSessionClick(session)"
            >
              <div class="session-icon">
                <svg width="20" height="20"><use href="#icon-chat"/></svg>
              </div>
              
              <div style="flex: 1; min-width: 0; display: flex; flex-direction: column; justify-content: center; gap: 8px;">
                <div style="display: flex; align-items: center; gap: 12px; margin-top: 2px;">
                  <span class="fw-medium text-md text-primary">{{ session.session_id }}</span>
                  <div v-if="session.git_branch" class="pill pill-blue">
                    <svg width="12" height="12"><use href="#icon-branch"/></svg> {{ session.git_branch }}
                  </div>
                </div>

                <div v-if="session.first_message" class="text-14 text-secondary" style="white-space: nowrap; overflow: hidden; text-overflow: ellipsis; padding-right: 32px;">
                  "{{ truncateText(session.first_message, 200) }}"
                </div>

                <div class="text-12 text-muted" style="display: flex; gap: 20px;">
                  <span>{{ formatTime(session.mtime) }}</span>
                  <span>{{ formatSize(session.size) }}</span>
                </div>
              </div>
              
              <div class="ghost-delete" @click.stop="handleDeleteSession(session)">
                <svg width="16" height="16"><use href="#icon-trash"/></svg>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Elegantly Designed Right Drawer For Session Details -->
    <div :class="['scrim', { active: showSessionDrawer }]" @click="closeDrawer"></div>
    <div :class="['drawer', { active: showSessionDrawer }]">
      <div class="drawer-header">
        <div>
          <div class="text-12 fw-bold text-muted text-upper">会话详情</div>
          <div class="text-2xl fw-bold text-primary">{{ currentSessionId }}</div>
        </div>
        <div class="drawer-close" @click="closeDrawer">
          <svg width="20" height="20"><use href="#icon-close"/></svg>
        </div>
      </div>
      <div class="drawer-body" v-loading="sessionStore.loading">
        <template v-if="sessionStore.messages.length === 0">
          <el-empty description="暂无消息" />
        </template>
        <template v-else>
          <div
            v-for="(msg, index) in sessionStore.messages"
            :key="index"
            :class="['bubble', msg.role === 'user' ? 'bubble-user' : 'bubble-bot']"
          >
            <div class="bubble-role">
              {{ msg.role === 'user' ? 'USER' : 'ASSISTANT' }}
            </div>
            <div class="bubble-content">
              <svg class="copy-btn" width="24" height="24" @click="handleCopyMessage(msg.content)"><use href="#icon-copy"/></svg>
              {{ getDisplayContent(msg.content, index) }}
              <div v-if="isLongMessage(msg.content)" class="expand-btn" @click="toggleExpand(index)">
                {{ expandedMessages.has(index) ? '收起' : '展开全部' }}
              </div>
            </div>
          </div>
        </template>
      </div>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { ElMessageBox } from 'element-plus'
import { notify } from '@/utils/notification'
import { useSessionStore } from '@/stores/sessions'
import { useUiStore } from '@/stores/ui'
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

function handleCliChange(name: string) {
  activeCliType.value = name
  sessionStore.clearSessions()
  sessionStore.fetchProjects(1)
}

const filteredSessions = computed(() => {
  if (!sessionSearchQuery.value) return sessionStore.sessions
  const query = sessionSearchQuery.value.toLowerCase()
  return sessionStore.sessions.filter(s =>
    s.session_id.toLowerCase().includes(query) ||
    s.first_message?.toLowerCase().includes(query) ||
    s.git_branch?.toLowerCase().includes(query)
  )
})

function handleProjectClick(project: ProjectInfo) {
  sessionStore.fetchSessions(project.name, 1, project)
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

function closeDrawer() {
  showSessionDrawer.value = false
}

async function handleDeleteProject(project: ProjectInfo) {
  try {
    await ElMessageBox.confirm(
      `确定删除项目 "${project.display_name}" 及其所有会话吗？此操作不可恢复！`,
      '确认删除'
    )
    await sessionStore.deleteProject(project.name)
    notify('项目已删除')
  } catch (e: any) {
    if (e !== 'cancel' && e?.toString() !== 'cancel') {
      console.error('Delete project error:', e)
      notify(e?.message || e?.toString() || '删除失败', 'error')
    }
  }
}

async function handleDeleteSession(session: SessionInfo) {
  try {
    await ElMessageBox.confirm(
      `确定删除会话 "${session.session_id.substring(0, 8)}..." 吗？此操作不可恢复！`,
      '确认删除'
    )
    await sessionStore.deleteSession(sessionStore.currentProject, session.session_id)
    notify('会话已删除')
  } catch (e: any) {
    if (e !== 'cancel' && e?.toString() !== 'cancel') {
      console.error('Delete session error:', e)
      notify(e?.message || e?.toString() || '删除失败', 'error')
    }
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
  const date = new Date(timestamp * 1000)
  return date.toLocaleString('zh-CN')
}

// Ensure first message does not look broken, CSS nowrap will handle rest visually 
function truncateText(text: string, maxLength: number): string {
  if (!text) return ''
  if (text.length > maxLength) {
    return text.substring(0, maxLength)
  }
  return text
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
  if (!content) return ''
  return content.replace(/\\n/g, '\n')
}

function isLongMessage(content: string): boolean {
  if (!content) return false
  return normalizeContent(content).split('\n').length > MAX_LINES
}

function getCollapsedContent(content: string): string {
  if (!content) return ''
  return normalizeContent(content).split('\n').slice(0, MAX_LINES).join('\n')
}

function getDisplayContent(content: string, index: number): string {
  const normalized = normalizeContent(content)
  if (expandedMessages.value.has(index) || !isLongMessage(content)) {
    return normalized
  }
  return getCollapsedContent(content)
}

function toggleExpand(index: number) {
  if (expandedMessages.value.has(index)) {
    expandedMessages.value.delete(index)
  } else {
    expandedMessages.value.add(index)
  }
}

onMounted(() => {
  sessionStore.fetchProjects(1)
})
</script>

<style scoped>
/* Scoped overrides to not depend completely on global, but using Ethereal Frost */
.sessions-page {
  color: #0f172a;
  height: 100%;
  display: flex;
  flex-direction: column;
}

/* Tab Underlines */
.top-tabs { display: flex; gap: 32px; border-bottom: 1px solid rgba(226, 232, 240, 0.6); margin: 0 40px 24px 40px; padding-top: 8px; flex-shrink: 0; }
.tab-item { padding-bottom: 12px; color: #94a3b8; font-weight: var(--fw-400); font-size: var(--fs-14); cursor: pointer; position: relative; transition: color 0.2s; }
.tab-item:hover { color: #475569; }
.tab-item.active { color: #0f172a; font-weight: var(--fw-600); border-bottom: 2px solid #0f172a; }

.project-list, .session-list {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.list-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  margin: 0 40px;
}

.scroll-area {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}

/* Headers & Inputs */
.page-header { display: flex; justify-content: space-between; align-items: center; margin: 0 40px 32px 40px; flex-shrink: 0; }
.page-title { font-size: var(--fs-14); font-weight: var(--fw-500); margin: 0; color: #0f172a; }

.search-box { position: relative; width: 320px; }
.search-box input { width: 100%; padding-left: 36px; border-radius: 10px; background: #ffffff; box-shadow: 0 2px 10px rgba(0,0,0,0.02); }
.search-icon { position: absolute; left: 12px; top: 50%; transform: translateY(-50%); color: #94a3b8; pointer-events: none; }

.c-input { padding: 8px 14px; border: 1px solid #e2e8f0; border-radius: 8px; font-size: var(--fs-14); outline: none; background: rgba(255,255,255,0.8); color: #0f172a; transition: all 0.2s; }
.c-input:focus { border-color: #0ea5e9; box-shadow: 0 0 0 3px rgba(14, 165, 233, 0.1); background: #ffffff; }

.b-button-outline { background: white; color: #0f172a; border: 1px solid #e2e8f0; padding: 8px 16px; border-radius: 8px; font-size: var(--fs-14); font-weight: var(--fw-400); cursor: pointer; display: inline-flex; align-items: center; justify-content: center; gap: 6px; box-shadow: 0 2px 4px rgba(0,0,0,0.02); transition: all 0.2s;}
.b-button-outline:hover { background: #f8fafc; border-color: #cbd5e1; }

.pill { padding: 4px 10px; border-radius: 999px; font-size: var(--fs-12); font-weight: var(--fw-600); display: inline-flex; align-items: center; gap: 4px; letter-spacing: 0.3px; }
.pill-grey { background: #f1f5f9; color: #64748b; }
.pill-blue { background: #f0f9ff; color: #0ea5e9; }

/* Grid & Cards */
.project-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 24px; }
.project-card { display: flex; align-items: center; padding: 16px 20px; border: 1px solid rgba(226, 232, 240, 0.8); border-radius: 16px; background: #ffffff; cursor: pointer; position: relative; box-shadow: 0 4px 12px rgba(0,0,0,0.03); transition: border-color 0.2s, background-color 0.2s; transform: none !important; }
.project-card:hover { border-color: #0ea5e9; background: #f8fafc; transform: none !important; box-shadow: 0 4px 12px rgba(0,0,0,0.03); }

.project-icon-box { width: 48px; height: 48px; border-radius: 12px; background: #f1f5f9; color: #0284c7; display: flex; align-items: center; justify-content: center; margin-right: 16px; flex-shrink: 0; }
.project-info { flex: 1; min-width: 0; padding-right: 28px; }
.project-path-title { font-weight: var(--fw-500); font-size: var(--fs-14); color: #0f172a; margin-bottom: 8px; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; text-overflow: ellipsis; }
.project-meta { display: flex; align-items: center; gap: 12px; }

.ghost-delete { position: absolute; right: 16px; top: 50%; transform: translateY(-50%) !important; color: #cbd5e1; padding: 6px; border-radius: 6px; z-index: 10; cursor: pointer; display: flex; align-items: center; justify-content: center; transition: opacity 0.2s, background-color 0.2s, color 0.2s; }
.ghost-delete:hover { background: #fee2e2; color: #ef4444; }

/* Sessions List Detail */
.session-card { display: flex; align-items: center; padding: 20px 24px; border-radius: 16px; background: #ffffff; cursor: pointer; margin-bottom: 12px; position: relative; border: 1px solid #f1f5f9; box-shadow: 0 2px 8px rgba(0,0,0,0.02); gap: 16px; transition: border-color 0.2s, background-color 0.2s; transform: none !important; }
.session-card:hover { border-color: #e0f2fe; background: #f8fafc; transform: none !important; box-shadow: 0 2px 8px rgba(0,0,0,0.02); }
.session-icon { width: 40px; height: 40px; border-radius: 50%; background: #f1f5f9; color: #0ea5e9; display: flex; align-items: center; justify-content: center; flex-shrink: 0; }

/* Custom Drawer */
.scrim { position: fixed; inset: 0; background: rgba(15, 23, 42, 0.1); z-index: 2000; opacity: 0; pointer-events: none; transition: opacity 0.3s; }
.drawer { position: fixed; right: 0; top: 0; bottom: 0; width: 80%; max-width: 100vw; background: #ffffff; z-index: 2001; transform: translateX(100%); transition: transform 0.3s ease; box-shadow: -10px 0 30px rgba(0,0,0,0.05); display: flex; flex-direction: column; border-left: 1px solid #f1f5f9; }
.scrim.active { opacity: 1; pointer-events: auto; }
.drawer.active { transform: translateX(0); }

.drawer-header { padding: 24px 32px; border-bottom: 1px solid #f1f5f9; display: flex; justify-content: space-between; align-items: center; background: #ffffff; z-index: 1; }
.drawer-close { cursor: pointer; padding: 8px; border-radius: 50%; color: #64748b; display: flex; align-items: center; justify-content: center; }
.drawer-close:hover { background: #f1f5f9; color: #0f172a; }
.drawer-body { flex: 1; overflow-y: auto; padding: 32px; display: flex; flex-direction: column; gap: 24px; background: #ffffff; }

/* Chat Bubbles */
.bubble { max-width: 92%; line-height: 1.6; font-size: var(--fs-14); position: relative; display: flex; flex-direction: column; gap: 6px; }
.bubble-role { font-size: var(--fs-12); font-weight: var(--fw-600); color: #94a3b8; display: flex; align-items: center; gap: 8px; margin-bottom: 2px; }
.bubble-user { align-self: flex-end; }
.bubble-user .bubble-content { background: #f1f5f9; color: #0f172a; padding: 14px 20px; padding-right: 44px; border-radius: 12px; border-bottom-right-radius: 2px; word-break: break-word; white-space: pre-wrap; position: relative; }
.bubble-user .bubble-role { justify-content: flex-end; }
.bubble-bot { align-self: flex-start; }
.bubble-bot .bubble-content { background: #f8fafc; color: #334155; padding: 14px 20px; padding-right: 44px; border-radius: 12px; border-bottom-left-radius: 2px; border: 1px solid #f1f5f9; word-break: break-word; white-space: pre-wrap; position: relative; }

.expand-btn { margin-top: 12px; border-top: 1px dashed #e2e8f0; padding-top: 8px; color: #64748b; font-size: var(--fs-12); font-weight: var(--fw-600); text-align: center; cursor: pointer; }
.expand-btn:hover { color: #0f172a; }

.copy-btn { 
  position: absolute; 
  top: 8px; 
  right: 8px; 
  opacity: 0.3; 
  color: #0f172a; 
  cursor: pointer; 
  padding: 6px; 
  z-index: 10; 
  display: flex;
  align-items: center;
  justify-content: center;
}
.bubble:hover .copy-btn { opacity: 0.8; }
.bubble-user .copy-btn { color: #0f172a; }
</style>
