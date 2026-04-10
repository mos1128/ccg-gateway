<template>
  <div class="app-shell">
    <div class="sidebar">
      <div class="logo">CCG Gateway</div>
      
      <div class="sidebar-scrollable">
        <div class="nav-group">
          <div class="nav-group-title">总览</div>
          <div class="nav-item" :class="{ active: route.path === '/' }" @click="router.push('/')">仪表盘</div>
          <div class="nav-item" :class="{ active: route.path === '/config' }" @click="router.push('/config')">全局设置</div>
          <div class="nav-item" :class="{ active: route.path === '/logs' }" @click="router.push('/logs')">日志记录</div>
          <div class="nav-item" :class="{ active: route.path === '/sessions' }" @click="router.push('/sessions')">会话记录</div>
        </div>
        
        <div class="nav-group">
          <div class="nav-group-title">核心资源</div>
          <!-- Note: Made the original menu paths consistent with old code, keeping '服务商管理' instead of '服务商' to match perfectly if desired, but spec says '服务商'. I'll stick to simple '服务商' -->
          <div class="nav-item" :class="{ active: route.path === '/providers' }" @click="router.push('/providers')">服务商</div>
          <div class="nav-item" :class="{ active: route.path === '/mcp' }" @click="router.push('/mcp')">MCP</div>
          <div class="nav-item" :class="{ active: route.path === '/prompts' }" @click="router.push('/prompts')">提示词</div>
          <div class="nav-item" :class="{ active: route.path === '/skills' }" @click="router.push('/skills')">Skill</div>
          <div class="nav-item" :class="{ active: route.path === '/plugins' }" @click="router.push('/plugins')">Plugin</div>
        </div>
      </div>
      
      <div class="sidebar-footer">
        <div class="footer-actions">
          <button class="footer-btn" @click="handleCheckUpdate" :disabled="checkingUpdate" title="检查更新">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" :class="{ 'spin': checkingUpdate }">
              <path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/>
            </svg>
          </button>
          <button class="footer-btn" @click="toggleDevtools" title="开发者工具">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/>
            </svg>
          </button>
          <button class="footer-btn" @click="openGithubRepo" title="GitHub 仓库">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"/>
            </svg>
          </button>
        </div>
        <div class="version-tag mono">
          <span>v</span>{{ appVersion }}
        </div>
      </div>
    </div>

    <div class="view-container">
      <div class="view-content">
        <router-view />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { getVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'
import { checkForUpdates } from '@/utils/updater'
import { open } from '@tauri-apps/plugin-shell'

const route = useRoute()
const router = useRouter()

const appVersion = ref('0.0.0')
const checkingUpdate = ref(false)

async function handleCheckUpdate() {
  checkingUpdate.value = true
  try {
    await checkForUpdates(false)
  } finally {
    checkingUpdate.value = false
  }
}

async function openGithubRepo() {
  await open('https://github.com/mos1128/ccg-gateway')
}

async function toggleDevtools() {
  await invoke('toggle_devtools')
}

onMounted(async () => {
  appVersion.value = await getVersion()
  checkForUpdates(true)
})
</script>

<style>
/* Global styles for our frost theme added to MainLayout avoiding strict scoped limits on deep elements if needed, though most is local */
body {
  background: var(--color-bg-page);
  margin: 0;
  padding: 20px;
  color: var(--color-text);
}

/* Ethereal Frost ElMessageBox Global Overrides to mimic custom modals */
.el-overlay.is-message-box {
  background: var(--color-scrim) !important;
  backdrop-filter: blur(3px) !important;
}

.el-message-box {
  background: var(--color-bg-95) !important;
  backdrop-filter: blur(20px) !important;
  border-radius: 12px !important;
  border: 1px solid var(--color-bg-80) !important;
  box-shadow: 0 20px 40px -10px var(--color-shadow-lg) !important;
  padding-bottom: 0 !important;
  width: 400px !important;
  max-width: 90vw !important;
}

.el-message-box__header {
  padding: 20px 24px 0 24px !important;
  border-bottom: none !important;
  background: transparent !important;
}

.el-message-box__title {
  font-size: var(--fs-16) !important;
  font-weight: var(--fw-600) !important;
  color: var(--color-text) !important;
}

.el-message-box__headerbtn {
  top: 18px !important;
  right: 20px !important;
}

.el-message-box__content {
  padding: 16px 24px 24px 24px !important;
  font-size: var(--fs-14) !important;
  color: var(--color-text-secondary) !important;
}

.el-message-box__btns {
  padding: 16px 24px !important;
  background: var(--color-bg-page) !important;
  border-top: 1px dashed var(--color-border-medium) !important;
  border-radius: 0 0 12px 12px !important;
  display: flex !important;
  justify-content: flex-end !important;
  gap: 12px;
}

.el-message-box__btns .el-button {
  margin-left: 0 !important;
  padding: 8px 16px !important;
  border-radius: 8px !important;
  font-weight: var(--fw-500) !important;
  font-size: var(--fs-14) !important;
  transition: all 0.2s !important;
  outline: none !important;
  min-height: auto !important;
}

.el-message-box__btns .el-button--default {
  background: var(--color-bg) !important;
  border: 1px solid var(--color-border) !important;
  color: var(--color-text-secondary) !important;
}
.el-message-box__btns .el-button--default:hover {
  background: var(--color-bg-subtle) !important;
  color: var(--color-text) !important;
}

.el-message-box__btns .el-button--primary {
  background: var(--color-primary) !important;
  border: none !important;
  color: var(--color-bg) !important;
}
.el-message-box__btns .el-button--primary:hover {
  background: var(--color-primary-hover) !important;
}
</style>

<style scoped>
* { box-sizing: border-box; }

.app-shell { 
  display: flex; gap: 32px; height: calc(100vh - 40px); width: 100%;
}

/* Sidebar Navigation */
.sidebar {
  width: 160px;
  padding-top: 12px;  display: flex; 
  flex-direction: column;
  position: relative;
  height: 100%;
}

.logo {
  font-size: var(--fs-20);
  font-weight: var(--fw-700);
  margin-bottom: 24px;
  color: var(--color-primary);
  padding-left: 16px;
  letter-spacing: -0.5px;
  flex-shrink: 0;
}

.sidebar-scrollable {
  flex: 1;
  overflow-y: auto;
  scrollbar-width: none;
}
.sidebar-scrollable::-webkit-scrollbar {
  display: none;
}

.nav-group { margin-bottom: 24px; }

.nav-group-title {
  font-size: var(--fs-12);
  font-weight: var(--fw-600);
  color: var(--color-text-weak);
  margin-bottom: 12px;
  letter-spacing: 1px;
  padding-left: 16px;
}

.nav-item {
  padding: 10px 16px;
  border-radius: 8px;
  margin-bottom: 4px;
  cursor: pointer;
  font-size: var(--fs-14);
  font-weight: var(--fw-500);
  color: var(--color-text);
  transition: all 0.2s;
}

.nav-item:hover {
  background: var(--color-border);
}

.nav-item.active {
  background: var(--color-bg);
  color: var(--color-primary);
  box-shadow: 0 2px 8px var(--color-shadow);
  font-weight: var(--fw-600);
}

/* Footer stats */
.sidebar-footer {
  margin-top: auto;
  padding: 20px 16px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
}

.footer-actions {
  display: flex;
  gap: 8px;
  background: var(--color-overlay);
  padding: 4px;
  border-radius: 10px;
}

.footer-btn {
  background: transparent;
  border: none;
  color: var(--color-text-weak);
  padding: 6px;
  border-radius: 7px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}
.footer-btn:hover {
  background: var(--color-bg);
  color: var(--color-primary);
}
.footer-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.version-tag {
  font-size: var(--fs-12);
  font-weight: var(--fw-500);
  color: var(--color-text-muted);
  letter-spacing: 0.5px;
}

.spin { animation: fa-spin 2s infinite linear; }
@keyframes fa-spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(359deg); }
}

/* View container */
.view-container {
  flex: 1; 
  background: var(--color-bg-subtle); 
  border-radius: 24px; 
  box-shadow: inset 0 0 0 1px var(--color-border); 
  padding: 40px 0; 
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.view-content {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
</style>
