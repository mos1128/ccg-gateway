<template>
  <div class="app-shell">
    <div class="sidebar">
      <div class="logo">CCG Gateway</div>
      
      <div class="nav-group">
        <div class="nav-group-title">总览</div>
        <div class="nav-item" :class="{ active: route.path === '/' }" @click="router.push('/')">仪表盘</div>
        <div class="nav-item" :class="{ active: route.path === '/sessions' }" @click="router.push('/sessions')">会话记录</div>
        <div class="nav-item" :class="{ active: route.path === '/logs' }" @click="router.push('/logs')">日志记录</div>
      </div>
      
      <div class="nav-group">
        <div class="nav-group-title">核心资源</div>
        <!-- Note: Made the original menu paths consistent with old code, keeping '服务商管理' instead of '服务商' to match perfectly if desired, but spec says '服务商'. I'll stick to simple '服务商' -->
        <div class="nav-item" :class="{ active: route.path === '/providers' }" @click="router.push('/providers')">服务商</div>
        <div class="nav-item" :class="{ active: route.path === '/mcp' }" @click="router.push('/mcp')">MCP 工具</div>
        <div class="nav-item" :class="{ active: route.path === '/prompts' }" @click="router.push('/prompts')">提示词</div>
        <div class="nav-item" :class="{ active: route.path === '/skills' }" @click="router.push('/skills')">扩展技能</div>
        <div class="nav-item" :class="{ active: route.path === '/plugins' }" @click="router.push('/plugins')">插件应用</div>
      </div>
      
      <div class="nav-group">
        <div class="nav-group-title">系统管理</div>
        <div class="nav-item" :class="{ active: route.path === '/config' }" @click="router.push('/config')">全局设置</div>
      </div>

      <div class="sidebar-footer">
        <div class="version">v{{ appVersion }}</div>
        <div style="margin-top: 12px; display: flex; gap: 8px; justify-content: center;">
          <el-button size="small" :icon="Refresh" @click="handleCheckUpdate" :loading="checkingUpdate" circle />
          <el-button size="small" :icon="Link" @click="openGithubRepo" circle />
        </div>
      </div>
    </div>

    <div class="view-container">
      <router-view />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { getVersion } from '@tauri-apps/api/app'
import { Refresh, Link } from '@element-plus/icons-vue'
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

onMounted(async () => {
  appVersion.value = await getVersion()
  checkForUpdates(true)
})
</script>

<style>
/* Global styles for our frost theme added to MainLayout avoiding strict scoped limits on deep elements if needed, though most is local */
body {
  background: #f8fafc;
  margin: 0;
  padding: 20px;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  color: #0f172a;
}

/* Ethereal Frost ElMessageBox Global Overrides to mimic custom modals */
.el-overlay.is-message-box {
  background: rgba(15, 23, 42, 0.25) !important;
  backdrop-filter: blur(3px) !important;
}

.el-message-box {
  background: rgba(255, 255, 255, 0.95) !important;
  backdrop-filter: blur(20px) !important;
  border-radius: 12px !important;
  border: 1px solid rgba(255, 255, 255, 0.8) !important;
  box-shadow: 0 20px 40px -10px rgba(0, 0, 0, 0.1) !important;
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
  font-size: 16px !important;
  font-weight: 600 !important;
  color: #0f172a !important;
}

.el-message-box__headerbtn {
  top: 18px !important;
  right: 20px !important;
}

.el-message-box__content {
  padding: 16px 24px 24px 24px !important;
  font-size: 14px !important;
  color: #475569 !important;
}

.el-message-box__btns {
  padding: 16px 24px !important;
  background: #f8fafc !important;
  border-top: 1px dashed rgba(226, 232, 240, 0.8) !important;
  border-radius: 0 0 12px 12px !important;
  display: flex !important;
  justify-content: flex-end !important;
  gap: 12px;
}

.el-message-box__btns .el-button {
  margin-left: 0 !important;
  padding: 8px 16px !important;
  border-radius: 8px !important;
  font-weight: 500 !important;
  font-size: 13px !important;
  transition: all 0.2s !important;
  outline: none !important;
  min-height: auto !important;
}

.el-message-box__btns .el-button--default {
  background: #ffffff !important;
  border: 1px solid #e2e8f0 !important;
  color: #475569 !important;
}
.el-message-box__btns .el-button--default:hover {
  background: #f1f5f9 !important;
  color: #0f172a !important;
}

.el-message-box__btns .el-button--primary {
  background: #0ea5e9 !important;
  border: none !important;
  color: #ffffff !important;
  box-shadow: 0 2px 8px rgba(14, 165, 233, 0.2) !important;
}
.el-message-box__btns .el-button--primary:hover {
  background: #0284c7 !important;
  transform: translateY(-1px) !important;
  box-shadow: 0 4px 12px rgba(14, 165, 233, 0.3) !important;
}
</style>

<style scoped>
* { box-sizing: border-box; }

.app-shell { 
  display: flex; gap: 32px; height: calc(100vh - 40px); width: 100%;
}

/* Sidebar Navigation */
.sidebar { 
  width: 220px; 
  padding-top: 12px; 
  display: flex; 
  flex-direction: column;
  position: relative;
}

.logo { 
  font-size: 22px; font-weight: 700; margin-bottom: 40px; color: #0ea5e9; padding-left: 16px; letter-spacing: -0.5px; 
}

.nav-group { margin-bottom: 32px; }

.nav-group-title { 
  font-size: 12px; font-weight: 700; color: #94a3b8; margin-bottom: 12px; letter-spacing: 1px; padding-left: 16px; 
}

.nav-item { 
  padding: 10px 16px; border-radius: 8px; margin-bottom: 4px; cursor: pointer; font-size: 14px; font-weight: 500; color: #475569; transition: all 0.2s; 
}

.nav-item:hover { 
  background: #e2e8f0; color: #0f172a; 
}

.nav-item.active { 
  background: #ffffff; color: #0ea5e9; box-shadow: 0 2px 8px rgba(0,0,0,0.03); font-weight: 600; 
}

/* Footer stats */
.sidebar-footer {
  margin-top: auto;
  text-align: center;
  padding-bottom: 12px;
}
.version {
  font-size: 13px;
  color: #94a3b8;
  font-family: monospace;
}

/* View container */
.view-container {
  flex: 1; 
  background: #f4f7fe; 
  border-radius: 24px; 
  box-shadow: inset 0 0 0 1px #e2e8f0; 
  padding: 40px; 
  min-width: 0;
  overflow-y: auto;
}
</style>
