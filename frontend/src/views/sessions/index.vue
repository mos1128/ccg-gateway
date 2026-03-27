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
      <div class="page-header">
        <h2 class="page-title">项目列表</h2>
        <div class="search-box">
          <svg class="search-icon" width="16" height="16"><use href="#icon-search"/></svg>
          <input type="text" v-model="searchQuery" class="c-input" placeholder="搜索项目...">
        </div>
      </div>

      <div v-loading="sessionStore.loading">
        <template v-if="filteredProjects.length === 0">
          <el-empty description="暂无项目" />
        </template>
        <div v-else class="project-grid">
          <div
            v-for="project in filteredProjects"
            :key="project.name"
            class="project-card"
            @click="handleProjectClick(project)"
          >
            <div class="project-icon-box">
              <svg width="24" height="24"><use href="#icon-folder"/></svg>
            </div>
            <div class="project-info">
              <div class="project-name">{{ project.display_name }}</div>
              <div class="project-path">{{ project.full_path }}</div>
              <div class="project-meta">
                <div class="pill pill-grey">{{ project.session_count }} 个会话</div>
                <span style="font-size: 11px; color: #94a3b8; font-family: monospace;">{{ formatSize(project.total_size) }}</span>
              </div>
            </div>
            <div class="ghost-delete" @click.stop="handleDeleteProject(project)">
              <svg width="16" height="16"><use href="#icon-trash"/></svg>
            </div>
          </div>
        </div>
        
        <div v-if="sessionStore.projectTotal > sessionStore.pageSize" style="display: flex; justify-content: center; margin-top: 24px;">
          <el-pagination
            v-model:current-page="sessionStore.projectPage"
            :page-size="sessionStore.pageSize"
            :total="sessionStore.projectTotal"
            layout="prev, pager, next"
            @current-change="handleProjectPageChange"
          />
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
          <h2 class="page-title" style="font-size: 20px;">{{ sessionStore.currentProjectInfo?.display_name }}</h2>
          <div class="pill pill-grey">{{ sessionStore.sessionTotal }} 个会话</div>
        </div>
        <div class="search-box" style="width: 260px;">
          <svg class="search-icon" width="16" height="16"><use href="#icon-search"/></svg>
          <input type="text" v-model="sessionSearchQuery" class="c-input" placeholder="搜索会话...">
        </div>
      </div>

      <div v-loading="sessionStore.loading">
        <template v-if="filteredSessions.length === 0">
          <el-empty description="暂无会话" />
        </template>
        <div v-else style="display: flex; flex-direction: column;">
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
                <span style="font-family: 'JetBrains Mono', monospace; font-weight: 600; font-size: 14px; color: #0f172a;">{{ session.session_id }}</span>
                <div v-if="session.git_branch" class="pill pill-blue">
                  <svg width="12" height="12"><use href="#icon-branch"/></svg> {{ session.git_branch }}
                </div>
              </div>
              
              <div v-if="session.first_message" style="font-size: 13px; color: #475569; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; padding-right: 32px;">
                "{{ truncateText(session.first_message, 200) }}"
              </div>

              <div style="display: flex; gap: 20px; font-size: 12px; color: #94a3b8; font-family: monospace;">
                <span>{{ formatTime(session.mtime) }}</span>
                <span>{{ formatSize(session.size) }}</span>
              </div>
            </div>
            
            <div class="ghost-delete" @click.stop="handleDeleteSession(session)">
              <svg width="16" height="16"><use href="#icon-trash"/></svg>
            </div>
          </div>
        </div>
        
        <div v-if="sessionStore.sessionTotal > sessionStore.pageSize" style="display: flex; justify-content: center; margin-top: 24px;">
          <el-pagination
            v-model:current-page="sessionStore.sessionPage"
            :page-size="sessionStore.pageSize"
            :total="sessionStore.sessionTotal"
            layout="prev, pager, next"
            @current-change="handleSessionPageChange"
          />
        </div>
      </div>
    </div>

    <!-- Elegantly Designed Right Drawer For Session Details -->
    <div :class="['scrim', { active: showSessionDrawer }]" @click="closeDrawer"></div>
    <div :class="['drawer', { active: showSessionDrawer }]">
      <div class="drawer-header">
        <div>
          <div style="font-size: 12px; font-weight: 700; color: #94a3b8; text-transform: uppercase; margin-bottom: 4px; letter-spacing: 0.5px;">会话审查</div>
          <div style="font-size: 18px; font-weight: 700; color: #0f172a; font-family: 'JetBrains Mono', monospace;">{{ currentSessionId }}</div>
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
              <svg class="copy-btn" width="12" height="12" @click="handleCopyMessage(msg.content)"><use href="#icon-copy"/></svg>
            </div>
            <div class="bubble-content">
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
const searchQuery = ref('')
const sessionSearchQuery = ref('')
const showSessionDrawer = ref(false)
const currentSessionId = ref('')
const expandedMessages = ref(new Set<number>())

function handleCliChange(name: string) {
  activeCliType.value = name
  sessionStore.clearSessions()
  sessionStore.fetchProjects(1)
}

const filteredProjects = computed(() => {
  if (!searchQuery.value) return sessionStore.projects
  const query = searchQuery.value.toLowerCase()
  return sessionStore.projects.filter(p =>
    p.display_name.toLowerCase().includes(query) ||
    p.full_path.toLowerCase().includes(query)
  )
})

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

function handleProjectPageChange(page: number) {
  sessionStore.fetchProjects(page)
}

function handleSessionPageChange(page: number) {
  sessionStore.fetchSessions(sessionStore.currentProject, page)
}

async function handleDeleteProject(project: ProjectInfo) {
  try {
    await ElMessageBox.confirm(
      `确定删除项目 "${project.display_name}" 及其所有会话吗？此操作不可恢复！`,
      '确认删除',
      { type: 'warning' }
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
      '确认删除',
      { type: 'warning' }
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
  font-family: "Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  color: #0f172a;
}

/* Tab Underlines */
.top-tabs { display: flex; gap: 32px; border-bottom: 1px solid rgba(226, 232, 240, 0.6); margin-bottom: 24px; padding-top: 8px; }
.tab-item { padding-bottom: 12px; color: #94a3b8; font-weight: 500; font-size: 15px; cursor: pointer; position: relative; transition: color 0.2s; }
.tab-item:hover { color: #475569; }
.tab-item.active { color: #0f172a; font-weight: 600; border-bottom: 2px solid #0f172a; }

/* Headers & Inputs */
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 32px; }
.page-title { font-size: 24px; font-weight: 800; margin: 0; letter-spacing: -0.5px; color: #0f172a; }

.search-box { position: relative; width: 320px; }
.search-box input { width: 100%; padding-left: 36px; border-radius: 10px; background: #ffffff; box-shadow: 0 2px 10px rgba(0,0,0,0.02); }
.search-icon { position: absolute; left: 12px; top: 50%; transform: translateY(-50%); color: #94a3b8; pointer-events: none; }

.c-input { padding: 8px 14px; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 13px; outline: none; background: rgba(255,255,255,0.8); color: #0f172a; transition: all 0.2s; }
.c-input:focus { border-color: #0ea5e9; box-shadow: 0 0 0 3px rgba(14, 165, 233, 0.1); background: #ffffff; }

.b-button-outline { background: white; color: #0f172a; border: 1px solid #e2e8f0; padding: 8px 16px; border-radius: 8px; font-size: 13px; font-weight: 500; cursor: pointer; display: inline-flex; align-items: center; justify-content: center; gap: 6px; box-shadow: 0 2px 4px rgba(0,0,0,0.02); transition: all 0.2s;}
.b-button-outline:hover { background: #f8fafc; border-color: #cbd5e1; }

.pill { padding: 4px 10px; border-radius: 999px; font-size: 11px; font-weight: 600; display: inline-flex; align-items: center; gap: 4px; letter-spacing: 0.3px; }
.pill-grey { background: #f1f5f9; color: #64748b; }
.pill-blue { background: #f0f9ff; color: #0ea5e9; }

/* Grid & Cards */
.project-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 24px; }
.project-card { display: flex; align-items: center; padding: 16px 20px; border: 1px solid rgba(255,255,255,0.6); border-radius: 16px; background: rgba(255, 255, 255, 0.7); backdrop-filter: blur(10px); cursor: pointer; transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1); position: relative; box-shadow: 0 4px 15px rgba(0,0,0,0.02); }
.project-card:hover { border-color: rgba(14, 165, 233, 0.3); box-shadow: 0 12px 30px rgba(14, 165, 233, 0.08); transform: translateY(-2px); background: #ffffff; }

.project-icon-box { width: 48px; height: 48px; border-radius: 12px; background: linear-gradient(135deg, #e0f2fe 0%, #bae6fd 100%); color: #0284c7; display: flex; align-items: center; justify-content: center; box-shadow: 0 4px 10px rgba(186, 230, 253, 0.5); margin-right: 16px; flex-shrink: 0; }
.project-info { flex: 1; min-width: 0; padding-right: 28px; }
.project-name { font-weight: 700; font-size: 15px; color: #0f172a; margin-bottom: 2px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.project-path { font-size: 12px; color: #64748b; font-family: "JetBrains Mono", monospace; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; margin-bottom: 8px; }
.project-meta { display: flex; align-items: center; gap: 12px; }

.ghost-delete { position: absolute; right: 16px; top: 50%; transform: translateY(-50%); color: #cbd5e1; opacity: 0; transition: all 0.2s; padding: 6px; border-radius: 6px; z-index: 10; cursor: pointer; display: flex; align-items: center; justify-content: center; }
.project-card:hover .ghost-delete, .session-card:hover .ghost-delete { opacity: 1; }
.ghost-delete:hover { background: #fee2e2; color: #ef4444; }

/* Sessions List Detail */
.session-card { display: flex; align-items: flex-start; padding: 20px 24px; border-radius: 16px; background: rgba(255,255,255,0.8); cursor: pointer; transition: all 0.2s; margin-bottom: 12px; position: relative; border: 1px solid transparent; box-shadow: 0 2px 8px rgba(0,0,0,0.02); gap: 16px; backdrop-filter: blur(10px); }
.session-card:hover { background: #ffffff; border-color: #e0f2fe; box-shadow: 0 10px 30px rgba(14, 165, 233, 0.06); transform: scale(1.002); }
.session-icon { width: 40px; height: 40px; border-radius: 50%; background: #f0f9ff; color: #0ea5e9; display: flex; align-items: center; justify-content: center; flex-shrink: 0; margin-top: 2px; }

/* Custom Drawer */
.scrim { position: fixed; inset: 0; background: rgba(15, 23, 42, 0.15); backdrop-filter: blur(2px); z-index: 2000; opacity: 0; pointer-events: none; transition: opacity 0.3s; }
.drawer { position: fixed; right: 0; top: 0; bottom: 0; width: 680px; max-width: 100vw; background: #ffffff; z-index: 2001; transform: translateX(100%); transition: transform 0.4s cubic-bezier(0.16, 1, 0.3, 1); box-shadow: -20px 0 50px rgba(0,0,0,0.05); display: flex; flex-direction: column; border-top-left-radius: 24px; border-bottom-left-radius: 24px; }
.scrim.active { opacity: 1; pointer-events: auto; }
.drawer.active { transform: translateX(0); }

.drawer-header { padding: 24px 32px; border-bottom: 1px solid #f1f5f9; display: flex; justify-content: space-between; align-items: center; background: rgba(255,255,255,0.95); backdrop-filter: blur(10px); z-index: 1; }
.drawer-close { cursor: pointer; padding: 8px; border-radius: 50%; background: #f1f5f9; color: #64748b; transition: all 0.2s; display: flex; align-items: center; justify-content: center; }
.drawer-close:hover { background: #e2e8f0; color: #0f172a; }
.drawer-body { flex: 1; overflow-y: auto; padding: 32px; display: flex; flex-direction: column; gap: 24px; background: #f8fafc; }

/* Chat Bubbles */
.bubble { max-width: 85%; line-height: 1.6; font-size: 14px; position: relative; display: flex; flex-direction: column; gap: 6px; }
.bubble-role { font-size: 12px; font-weight: 600; color: #94a3b8; display: flex; align-items: center; gap: 8px; margin-bottom: 2px; }
.bubble-user { align-self: flex-end; }
.bubble-user .bubble-content { background: #0ea5e9; color: white; padding: 14px 20px; border-radius: 20px; border-bottom-right-radius: 4px; box-shadow: 0 4px 15px rgba(14, 165, 233, 0.15); word-break: break-word; white-space: pre-wrap; }
.bubble-user .bubble-role { justify-content: flex-end; }
.bubble-bot { align-self: flex-start; }
.bubble-bot .bubble-content { background: #ffffff; color: #334155; padding: 14px 20px; border-radius: 20px; border-bottom-left-radius: 4px; border: 1px solid #f1f5f9; box-shadow: 0 4px 15px rgba(0,0,0,0.03); word-break: break-word; white-space: pre-wrap; }

.expand-btn { margin-top: 12px; border-top: 1px dashed rgba(0,0,0,0.1); padding-top: 8px; color: currentcolor; opacity: 0.8; font-size: 12px; font-weight: 600; text-align: center; cursor: pointer; transition: opacity 0.2s; }
.expand-btn:hover { opacity: 1; }
.bubble-bot .expand-btn { border-top-color: #e2e8f0; }

.copy-btn { opacity: 0; transition: opacity 0.2s; color: #94a3b8; cursor: pointer; padding: 4px; border-radius: 4px; }
.bubble:hover .copy-btn { opacity: 1; }
.copy-btn:hover { color: #0f172a; background: #f1f5f9; }
.bubble-user .copy-btn { color: rgba(255,255,255,0.7); }
.bubble-user .copy-btn:hover { color: white; background: rgba(255,255,255,0.1); }
</style>
