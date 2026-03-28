<template>
  <div class="config-page">
    <!-- Icon Symbols -->
    <svg style="display:none">
      <defs>
        <symbol id="icon-settings" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.38a2 2 0 0 0-.73-2.73l-.15-.1a2 2 0 0 1-1-1.72v-.51a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/>
        </symbol>
        <symbol id="icon-cloud" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M17.5 19a5.5 5.5 0 0 0 2.5-10.5 8.5 8.5 0 1 0-14 10h11.5Z"/>
        </symbol>
        <symbol id="icon-terminal" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/>
        </symbol>
        <symbol id="icon-save" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/><polyline points="17 21 17 13 7 13 7 21"/><polyline points="7 3 7 8 15 8"/>
        </symbol>
        <symbol id="icon-download" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
        </symbol>
        <symbol id="icon-upload" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/>
        </symbol>
        <symbol id="icon-activity" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/>
        </symbol>
      </defs>
    </svg>

    <div class="config-layout">
      <!-- Left Column: Core & Backup -->
      <div class="config-column">
        
        <!-- Timeout Card -->
        <div class="frost-card">
          <div class="card-header-simple">
            <svg width="20" height="20" class="header-icon"><use href="#icon-activity"/></svg>
            <span class="card-label">基础配置</span>
          </div>
          <div class="card-body">
            <div class="input-item">
              <label class="item-label">流式首字节超时</label>
              <div class="input-with-unit">
                <input type="number" v-model.number="timeoutForm.stream_first_byte_timeout" class="f-input">
                <span class="unit">秒</span>
              </div>
            </div>
            <div class="input-item">
              <label class="item-label">流式空闲超时</label>
              <div class="input-with-unit">
                <input type="number" v-model.number="timeoutForm.stream_idle_timeout" class="f-input">
                <span class="unit">秒</span>
              </div>
            </div>
            <div class="input-item">
              <label class="item-label">非流式超时</label>
              <div class="input-with-unit">
                <input type="number" v-model.number="timeoutForm.non_stream_timeout" class="f-input">
                <span class="unit">秒</span>
              </div>
            </div>
            <div class="card-footer-right">
              <button class="f-button" @click="saveTimeouts">
                <svg width="14" height="14" style="margin-right: 6px;"><use href="#icon-save"/></svg>
                保存配置
              </button>
            </div>
          </div>
        </div>

        <!-- Backup Card -->
        <div class="frost-card">
          <div class="card-header-simple">
            <svg width="20" height="20" class="header-icon"><use href="#icon-cloud"/></svg>
            <span class="card-label">备份与同步</span>
          </div>
          <div class="card-body">
            <div class="frost-segmented">
              <div class="seg-item" :class="{ active: activeBackupTab === 'local' }" @click="activeBackupTab = 'local'">本地备份</div>
              <div class="seg-item" :class="{ active: activeBackupTab === 'webdav' }" @click="activeBackupTab = 'webdav'">WebDAV 备份</div>
            </div>

            <div v-if="activeBackupTab === 'local'" class="tab-content">
              <p class="desc-text">定期导出数据库以保护您的服务商、MCP、提示词等配置数据不丢失。</p>
              <div class="action-row-end">
                <button class="f-button ghost" @click="handleExportLocal" :disabled="exportingLocal">
                  <svg width="14" height="14" style="margin-right: 6px;"><use href="#icon-download"/></svg>
                  导出
                </button>
                <el-upload :show-file-list="false" :before-upload="handleImportLocal" accept=".db">
                   <button class="f-button secondary" :disabled="importingLocal">
                     <svg width="14" height="14" style="margin-right: 6px;"><use href="#icon-upload"/></svg>
                     导入
                   </button>
                </el-upload>
              </div>
            </div>

            <div v-if="activeBackupTab === 'webdav'" class="tab-content">
              <div class="input-item">
                <label class="item-label">服务器地址</label>
                <input type="text" v-model="webdavForm.url" placeholder="https://dav.jianguoyun.com/dav/" class="f-input">
              </div>
              <div class="input-row">
                <div class="input-item" style="flex: 1;">
                  <label class="item-label">用户名</label>
                  <input type="text" v-model="webdavForm.username" class="f-input">
                </div>
                <div class="input-item" style="flex: 1;">
                  <label class="item-label">应用密码</label>
                  <input type="password" v-model="webdavForm.password" class="f-input">
                </div>
              </div>
              <div class="action-row-end" style="margin-top: 12px; gap: 8px;">
                <button class="f-button ghost-plain" @click="handleTestWebdav" :disabled="testingWebdav">测试链接</button>
                <button class="f-button ghost-plain" @click="handleSaveWebdav">保存账号</button>
                <button class="f-button ghost" @click="handleExportWebdav" :disabled="exportingWebdav">
                  <svg width="14" height="14" style="margin-right: 6px;"><use href="#icon-download"/></svg>
                  导出
                </button>
                <button class="f-button secondary" @click="handleShowWebdavList">
                  <svg width="14" height="14" style="margin-right: 6px;"><use href="#icon-upload"/></svg>
                  导入
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Right Column: CLI Settings -->
      <div class="config-column">
        <div class="frost-card cli-settings-card">
          <div class="card-header-simple">
            <svg width="20" height="20" class="header-icon"><use href="#icon-terminal"/></svg>
            <span class="card-label">CLI 运行环境配置</span>
          </div>
          <div class="card-body" style="flex: 1; display: flex; flex-direction: column;">
            <div class="frost-segmented" style="margin-bottom: 24px;">
              <div class="seg-item" :class="{ active: activeCliTab === 'claude_code' }" @click="activeCliTab = 'claude_code'">Claude Code</div>
              <div class="seg-item" :class="{ active: activeCliTab === 'codex' }" @click="activeCliTab = 'codex'">Codex</div>
              <div class="seg-item" :class="{ active: activeCliTab === 'gemini' }" @click="activeCliTab = 'gemini'">Gemini</div>
            </div>

            <div class="cli-form-container">
              <CliSettingsForm 
                :key="activeCliTab"
                :cli-type="activeCliTab" 
                :settings="settingsStore.settings?.cli_settings?.[activeCliTab]" 
                @save="saveCli" 
              />
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- WebDAV Backup List Dialog -->
    <div class="modal-overlay" :class="{ active: webdavListVisible }" @click.self="webdavListVisible = false">
      <div class="modal-content">
        <div style="padding: 24px 32px; border-bottom: 1px solid #f1f5f9; display: flex; justify-content: space-between; align-items: center;">
          <div style="font-size: 20px; font-weight: 500; color: #0f172a;">管理 WebDAV 备份</div>
          <div style="cursor: pointer; color: #94a3b8; font-size: 20px; font-weight: bold;" @click="webdavListVisible = false">×</div>
        </div>
        <div style="padding: 0; max-height: 70vh; overflow-y: auto;">
          <el-table :data="webdavBackups" v-loading="loadingWebdavList" style="width: 100%; border-radius: 0 0 20px 20px; overflow: hidden;">
            <el-table-column prop="filename" label="文件名" min-width="240">
              <template #default="{ row }">
                <div style="padding-left: 20px;">{{ row.filename }}</div>
              </template>
            </el-table-column>
            <el-table-column prop="size" label="大小" width="100">
              <template #default="{ row }">{{ formatSize(row.size) }}</template>
            </el-table-column>
            <el-table-column label="操作" width="160" align="right">
              <template #default="{ row }">
                <div style="display: flex; gap: 8px; justify-content: flex-end; padding-right: 20px;">
                  <el-button type="primary" link @click="handleImportWebdav(row.filename)" :loading="importingWebdav">恢复</el-button>
                  <el-button type="danger" link @click="handleDeleteWebdav(row.filename)" :loading="deletingWebdav">删除</el-button>
                </div>
              </template>
            </el-table-column>
          </el-table>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, computed } from 'vue'
import { ElMessageBox } from 'element-plus'
import { notify } from '@/utils/notification'
import { useSettingsStore } from '@/stores/settings'
import { useUiStore } from '@/stores/ui'
import CliSettingsForm from './components/CliSettingsForm.vue'
import * as backupApi from '@/api/backup'
import type { WebdavSettings, WebdavBackup } from '@/api/backup'

const settingsStore = useSettingsStore()
const uiStore = useUiStore()

const activeCliTab = computed({
  get: () => uiStore.configActiveCliTab,
  set: (val) => uiStore.setConfigActiveCliTab(val as 'claude_code' | 'codex' | 'gemini')
})
const activeBackupTab = computed({
  get: () => uiStore.configActiveBackupTab,
  set: (val) => uiStore.setConfigActiveBackupTab(val as 'local' | 'webdav')
})

const timeoutForm = ref({
  stream_first_byte_timeout: 30,
  stream_idle_timeout: 60,
  non_stream_timeout: 120
})

watch(() => settingsStore.settings, (settings) => {
  if (settings) {
    timeoutForm.value = { ...settings.timeouts }
  }
}, { immediate: true })

async function saveTimeouts() {
  await settingsStore.updateTimeouts(timeoutForm.value)
  notify('超时配置已保存')
}

async function saveCli(cliType: string, data: any) {
  await settingsStore.updateCli(cliType, data)
  notify('CLI 配置已保存')
}

// Backup related
const webdavForm = ref<WebdavSettings>({ url: '', username: '', password: '' })
const exportingLocal = ref(false)
const importingLocal = ref(false)
const testingWebdav = ref(false)
const savingWebdav = ref(false)
const exportingWebdav = ref(false)
const loadingWebdavList = ref(false)
const importingWebdav = ref(false)
const deletingWebdav = ref(false)
const webdavListVisible = ref(false)
const webdavBackups = ref<WebdavBackup[]>([])

async function loadWebdavSettings() {
  try {
    const { data } = await backupApi.getWebdavSettings()
    webdavForm.value = data
  } catch {}
}

async function handleExportLocal() {
  exportingLocal.value = true
  try {
    const { data } = await backupApi.exportToLocal()
    const url = window.URL.createObjectURL(new Blob([data]))
    const link = document.createElement('a')
    link.href = url
    link.download = `ccg_gateway_${new Date().toISOString().slice(0, 10)}.db`
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
    window.URL.revokeObjectURL(url)
    notify('导出成功')
  } catch (error: any) {
    notify(error?.message || '导出失败', 'error')
  } finally {
    exportingLocal.value = false
  }
}

async function handleImportLocal(file: File) {
  try {
    await ElMessageBox.confirm('导入将覆盖当前所有数据，确定继续？', '警告')
    importingLocal.value = true
    await backupApi.importFromLocal(file)
    notify('导入成功，应用将自动退出，请重新打开应用')
  } catch (e) {} finally {
    importingLocal.value = false
  }
  return false
}

async function handleTestWebdav() {
  testingWebdav.value = true
  try {
    const { data } = await backupApi.testWebdavConnection(webdavForm.value)
    if (data.success) {
      notify('连接成功')
    } else {
      notify('连接失败', 'error')
    }
  } catch (error: any) {
    notify(error?.message || '连接失败', 'error')
  } finally {
    testingWebdav.value = false
  }
}

async function handleSaveWebdav() {
  try {
    await backupApi.updateWebdavSettings(webdavForm.value)
    notify('WebDAV 配置已保存')
  } catch (error: any) {
    notify(error?.message || '保存失败', 'error')
  }
}

async function handleExportWebdav() {
  exportingWebdav.value = true
  try {
    const { data } = await backupApi.exportToWebdav()
    notify(`同步成功: ${data.filename}`)
  } catch (error: any) {
    notify(error?.message || '同步失败', 'error')
  } finally {
    exportingWebdav.value = false
  }
}

async function handleShowWebdavList() {
  webdavListVisible.value = true
  loadingWebdavList.value = true
  try {
    const { data } = await backupApi.listWebdavBackups()
    webdavBackups.value = data.backups
  } finally {
    loadingWebdavList.value = false
  }
}

async function handleImportWebdav(filename: string) {
  try {
    await ElMessageBox.confirm('导入将覆盖当前所有数据，确定继续？', '警告')
    importingWebdav.value = true
    await backupApi.importFromWebdav(filename)
    notify('导入成功，应用将自动退出，请重新打开应用')
    webdavListVisible.value = false
  } catch (error: any) {
    if (error !== 'cancel') notify(error?.message || '导入失败', 'error')
  } finally {
    importingWebdav.value = false
  }
}

async function handleDeleteWebdav(filename: string) {
  try {
    await ElMessageBox.confirm(`确定要删除远程备份 ${filename} 吗？`, '警告')
    deletingWebdav.value = true
    await backupApi.deleteWebdavBackup(filename)
    notify('已删除')
    await handleShowWebdavList()
  } catch (error: any) {
    if (error !== 'cancel') notify(error?.message || '删除失败', 'error')
  } finally {
    deletingWebdav.value = false
  }
}

function formatSize(bytes: number) {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / 1024 / 1024).toFixed(1) + ' MB'
}

onMounted(() => {
  settingsStore.fetchSettings()
  loadWebdavSettings()
})
</script>

<style scoped>
.config-page {
  font-family: "Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
}

/* Header */
.page-header { margin-bottom: 32px; }
.page-title { font-size: 28px; font-weight: 800; color: #0f172a; margin: 0 0 8px 0; letter-spacing: -0.8px; }
.page-subtitle { font-size: 14px; color: #64748b; margin: 0; }

/* Layout */
.config-layout { display: flex; gap: 32px; align-items: flex-start; }
.config-column { flex: 1; display: flex; flex-direction: column; gap: 32px; min-width: 0; }

/* Frost Card */
.frost-card { 
  background: #ffffff; border-radius: 20px; border: 1px solid rgba(226, 232, 240, 0.8); 
  padding: 32px; box-shadow: 0 4px 12px rgba(0,0,0,0.03); transition: all 0.2s; 
  display: flex; flex-direction: column;
}
.frost-card:hover { border-color: #0ea5e9; box-shadow: 0 10px 20px -5px rgba(0,0,0,0.05); }

.card-header-simple { display: flex; align-items: center; gap: 12px; margin-bottom: 24px; color: #0f172a; }
.header-icon { color: #64748b; opacity: 0.8; }
.card-label { font-size: 16px; font-weight: 700; letter-spacing: -0.3px; }

/* Form Items */
.input-item { margin-bottom: 20px; }
.input-row { display: flex; gap: 16px; }
.item-label { display: block; font-size: 13px; font-weight: 600; color: #475569; margin-bottom: 8px; }

.f-input { 
  width: 100%; padding: 10px 14px; background: #ffffff; border: 1px solid #e2e8f0; 
  border-radius: 10px; font-size: 14px; color: #0f172a; outline: none; transition: all 0.2s; 
}
.f-input:focus { border-color: #0ea5e9; box-shadow: 0 0 0 3px rgba(14, 165, 233, 0.1); }

.input-with-unit { display: flex; align-items: center; gap: 12px; }
.unit { font-size: 13px; color: #94a3b8; font-weight: 500; }

/* Segmented Control */
.frost-segmented { 
  display: flex; background: rgba(148, 163, 184, 0.08); padding: 4px; border-radius: 12px; margin-bottom: 20px;
}
.seg-item { 
  flex: 1; text-align: center; padding: 8px 12px; font-size: 13px; font-weight: 600; 
  color: #64748b; cursor: pointer; border-radius: 9px; transition: all 0.2s; 
}
.seg-item.active { background: #ffffff; color: #0f172a; box-shadow: 0 2px 8px rgba(0,0,0,0.06); }

/* Buttons */
.f-button { 
  background: #0ea5e9; color: #ffffff; border: none; padding: 10px 18px; border-radius: 10px; 
  font-size: 14px; font-weight: 600; cursor: pointer; display: flex; align-items: center; 
  transition: background 0.2s; 
}
.f-button:hover:not(:disabled) { background: #0284c7; }
.f-button:disabled { background: #94a3b8; box-shadow: none; cursor: not-allowed; opacity: 0.7; }

.f-button.secondary { background: #f8fafc; color: #475569; border: 1px solid #e2e8f0; box-shadow: none; }
.f-button.secondary:hover:not(:disabled) { background: #f1f5f9; color: #0f172a; }

.f-button.ghost { background: #f0f9ff; color: #0369a1; box-shadow: none; border: 1px solid #bae6fd; }
.f-button.ghost:hover:not(:disabled) { background: #e0f2fe; }

.f-button.ghost-plain { background: transparent; color: #64748b; box-shadow: none; border: 1px solid transparent; padding: 6px 12px; font-size: 13px; font-weight: 500; }
.f-button.ghost-plain:hover { color: #0f172a; background: #f8fafc; }

.action-row-end { display: flex; justify-content: flex-end; gap: 12px; align-items: center; }
.card-footer-right { margin-top: 8px; display: flex; justify-content: flex-end; }

.desc-text { font-size: 13px; color: #94a3b8; line-height: 1.6; margin: 0 0 20px 0; }

/* CLI Column adjustment */
.cli-settings-card { flex: 1; }
.cli-form-container { flex: 1; min-height: 400px; display: flex; flex-direction: column; }

/* Modal Styles */
.modal-overlay { position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(15, 23, 42, 0.4); display: flex; align-items: center; justify-content: center; opacity: 0; pointer-events: none; transition: opacity 0.2s; z-index: 1000; }
.modal-overlay.active { opacity: 1; pointer-events: auto; }
.modal-content { background: white; border-radius: 20px; width: 720px; max-width: 95vw; box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.15); display: flex; flex-direction: column; }
</style>
