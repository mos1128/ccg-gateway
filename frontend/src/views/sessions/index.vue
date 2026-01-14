<template>
  <div class="sessions-page">
    <el-tabs v-model="activeCliType" @tab-change="handleCliTypeChange">
      <el-tab-pane label="Claude Code" name="claude_code" />
      <el-tab-pane label="Codex" name="codex" />
      <el-tab-pane label="Gemini" name="gemini" />
    </el-tabs>

    <!-- Project List View -->
    <div v-if="!currentProject" class="project-list">
      <div class="page-header">
        <span class="header-title">项目列表</span>
        <el-input
          v-model="searchQuery"
          placeholder="搜索项目..."
          clearable
          style="width: 300px"
        >
          <template #prefix>
            <el-icon><Search /></el-icon>
          </template>
        </el-input>
      </div>

      <el-card v-loading="sessionStore.loading">
        <template v-if="filteredProjects.length === 0">
          <el-empty description="暂无项目" />
        </template>
        <div v-else class="projects-grid">
          <div
            v-for="project in filteredProjects"
            :key="project.name"
            class="project-card"
            @click="handleProjectClick(project)"
          >
            <div class="project-icon">
              <el-icon :size="32"><Folder /></el-icon>
            </div>
            <div class="project-info">
              <div class="project-name">{{ project.display_name }}</div>
              <div class="project-path">{{ project.full_path }}</div>
              <div class="project-meta">
                <el-tag size="small">{{ project.session_count }} 个会话</el-tag>
                <span class="project-size">{{ formatSize(project.total_size) }}</span>
              </div>
            </div>
            <el-button
              class="delete-btn"
              type="danger"
              :icon="Delete"
              circle
              size="small"
              @click.stop="handleDeleteProject(project)"
            />
          </div>
        </div>
        <div v-if="sessionStore.projectTotal > sessionStore.pageSize" class="pagination-wrapper">
          <el-pagination
            v-model:current-page="sessionStore.projectPage"
            :page-size="sessionStore.pageSize"
            :total="sessionStore.projectTotal"
            layout="total, prev, pager, next"
            @current-change="handleProjectPageChange"
          />
        </div>
      </el-card>
    </div>

    <!-- Session List View -->
    <div v-else class="session-list">
      <div class="page-header">
        <div class="header-left">
          <el-button :icon="ArrowLeft" @click="handleBackToProjects">返回</el-button>
          <div class="project-title">
            <span class="header-title">{{ sessionStore.currentProjectInfo?.display_name }}</span>
            <el-tag size="small" type="info">{{ sessionStore.sessionTotal }} 个会话</el-tag>
          </div>
        </div>
        <el-input
          v-model="sessionSearchQuery"
          placeholder="搜索会话..."
          clearable
          style="width: 300px"
        >
          <template #prefix>
            <el-icon><Search /></el-icon>
          </template>
        </el-input>
      </div>

      <el-card v-loading="sessionStore.loading">
        <template v-if="filteredSessions.length === 0">
          <el-empty description="暂无会话" />
        </template>
        <div v-else class="sessions-list">
          <div
            v-for="session in filteredSessions"
            :key="session.session_id"
            class="session-item"
            @click="handleSessionClick(session)"
          >
            <div class="session-icon">
              <el-icon :size="24" color="#409EFF"><ChatDotRound /></el-icon>
            </div>
            <div class="session-info">
              <div class="session-header">
                <span class="session-id">{{ session.session_id }}</span>
                <el-tag v-if="session.git_branch" size="small" type="info" class="branch-tag">
                  <el-icon><Connection /></el-icon>
                  {{ session.git_branch }}
                </el-tag>
              </div>
              <div class="session-message" v-if="session.first_message">
                {{ truncateText(session.first_message, 100) }}
              </div>
              <div class="session-meta">
                <span>{{ formatTime(session.mtime) }}</span>
                <span>{{ formatSize(session.size) }}</span>
              </div>
            </div>
            <el-button
              class="delete-btn"
              type="danger"
              :icon="Delete"
              circle
              size="small"
              @click.stop="handleDeleteSession(session)"
            />
          </div>
        </div>
        <div v-if="sessionStore.sessionTotal > sessionStore.pageSize" class="pagination-wrapper">
          <el-pagination
            v-model:current-page="sessionStore.sessionPage"
            :page-size="sessionStore.pageSize"
            :total="sessionStore.sessionTotal"
            layout="total, prev, pager, next"
            @current-change="handleSessionPageChange"
          />
        </div>
      </el-card>
    </div>

    <!-- Session Detail Drawer -->
    <el-drawer
      v-model="showSessionDrawer"
      :title="'会话详情 - ' + currentSessionId.substring(0, 8)"
      size="80%"
      direction="rtl"
    >
      <div v-loading="sessionStore.loading" class="chat-container">
        <div
          v-for="(msg, index) in sessionStore.messages"
          :key="index"
          :class="['chat-message', msg.role]"
        >
          <div class="message-header">
            <el-tag :type="msg.role === 'user' ? 'primary' : 'success'" size="small">
              {{ msg.role === 'user' ? '用户' : '助手' }}
            </el-tag>
            <el-button
              :icon="CopyDocument"
              size="small"
              text
              @click="handleCopyMessage(msg.content)"
            />
          </div>
          <div class="message-content">{{ getDisplayContent(msg.content, index) }}</div>
          <div v-if="isLongMessage(msg.content)" class="expand-btn">
            <el-button type="primary" link size="small" @click="toggleExpand(index)">
              {{ expandedMessages.has(index) ? '收起' : '展开全部' }}
            </el-button>
          </div>
        </div>
        <el-empty v-if="sessionStore.messages.length === 0" description="暂无消息" />
      </div>
    </el-drawer>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Search, Folder, Delete, ArrowLeft, ChatDotRound, Connection, CopyDocument } from '@element-plus/icons-vue'
import { useSessionStore } from '@/stores/sessions'
import type { CliType } from '@/types/models'
import type { ProjectInfo, SessionInfo } from '@/api/sessions'

const sessionStore = useSessionStore()

const activeCliType = computed({
  get: () => sessionStore.currentCliType,
  set: (val) => sessionStore.setCliType(val as CliType)
})

const currentProject = computed(() => sessionStore.currentProject)
const searchQuery = ref('')
const sessionSearchQuery = ref('')
const showSessionDrawer = ref(false)
const currentSessionId = ref('')
const expandedMessages = ref(new Set<number>())

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

function handleCliTypeChange(cliType: string) {
  sessionStore.setCliType(cliType as CliType)
  sessionStore.clearSessions()
  sessionStore.fetchProjects(1)
}

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
    ElMessage.success('项目已删除')
  } catch {
    // cancelled
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
    ElMessage.success('会话已删除')
  } catch {
    // cancelled
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

function truncateText(text: string, maxLength: number): string {
  if (!text) return ''
  if (text.length > maxLength) {
    return text.substring(0, maxLength) + '...'
  }
  return text
}

async function handleCopyMessage(content: string) {
  try {
    await navigator.clipboard.writeText(normalizeContent(content))
    ElMessage.success('已复制')
  } catch {
    ElMessage.error('复制失败')
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
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.project-title {
  display: flex;
  align-items: center;
  gap: 8px;
}

.header-title {
  font-size: 18px;
  font-weight: 600;
}

.projects-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 16px;
}

.project-card {
  display: flex;
  align-items: center;
  padding: 16px;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
  position: relative;
}

.project-card:hover {
  border-color: var(--el-color-primary);
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.1);
}

.project-card:hover .delete-btn {
  opacity: 1;
}

.project-icon {
  flex-shrink: 0;
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--el-color-primary-light-9);
  border-radius: 8px;
  color: var(--el-color-primary);
  margin-right: 16px;
}

.project-info {
  flex: 1;
  min-width: 0;
}

.project-name {
  font-weight: 600;
  font-size: 15px;
  margin-bottom: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project-path {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  margin-bottom: 8px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project-meta {
  display: flex;
  align-items: center;
  gap: 12px;
}

.project-size {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.delete-btn {
  position: absolute;
  top: 8px;
  right: 8px;
  opacity: 0;
  transition: opacity 0.2s;
}

.sessions-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.session-item {
  display: flex;
  align-items: flex-start;
  padding: 16px;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
  position: relative;
}

.session-item:hover {
  border-color: var(--el-color-primary);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

.session-item:hover .delete-btn {
  opacity: 1;
}

.session-icon {
  flex-shrink: 0;
  margin-right: 12px;
  margin-top: 2px;
}

.session-info {
  flex: 1;
  min-width: 0;
}

.session-header {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin-bottom: 6px;
}

.session-id {
  font-weight: 600;
  font-family: monospace;
  word-break: break-all;
  flex: 1;
  min-width: 0;
}

.branch-tag {
  flex-shrink: 0;
  white-space: nowrap;
  max-width: none !important;
  display: inline-flex !important;
  align-items: center;
}

.branch-tag :deep(.el-tag__content) {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  overflow: visible;
}

.session-message {
  font-size: 13px;
  color: var(--el-text-color-regular);
  margin-bottom: 8px;
  line-height: 1.5;
}

.session-meta {
  display: flex;
  gap: 16px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.pagination-wrapper {
  display: flex;
  justify-content: center;
  margin-top: 20px;
  padding-top: 20px;
  border-top: 1px solid var(--el-border-color-lighter);
}

.chat-container {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 16px;
}

.chat-message {
  padding: 12px 16px;
  border-radius: 8px;
}

.chat-message.user {
  background: var(--el-color-primary-light-9);
}

.chat-message.assistant {
  background: var(--el-fill-color-light);
}

.message-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.message-content {
  white-space: pre-wrap;
  word-break: break-word;
  line-height: 1.6;
  font-size: 14px;
}

.expand-btn {
  margin-top: 8px;
  text-align: right;
}
</style>
