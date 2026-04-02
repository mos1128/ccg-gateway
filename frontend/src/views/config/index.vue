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

    <div class="scroll-area">
      <div class="config-layout">
        <!-- Left Column: CLI Settings -->
        <div class="config-column">
          <div class="frost-card cli-settings-card">
            <div class="card-header-simple">
              <svg width="20" height="20" class="header-icon"><use href="#icon-terminal"/></svg>
              <span class="card-label">CLI 运行配置</span>
              <div style="flex: 1;"></div>
              <button class="b-button" style="padding: 6px 14px;" @click="cliFormRef?.handleSave()">
                <svg width="16" height="16" style="margin-right: 6px;"><use href="#icon-save"/></svg>
                保存
              </button>
            </div>
            <div class="card-body" style="flex: 1; display: flex; flex-direction: column;">
              <div class="frost-segmented" style="margin-bottom: 24px;">
                <div class="seg-item" :class="{ active: activeCliTab === 'claude_code' }" @click="activeCliTab = 'claude_code'">Claude Code</div>
                <div class="seg-item" :class="{ active: activeCliTab === 'codex' }" @click="activeCliTab = 'codex'">Codex</div>
                <div class="seg-item" :class="{ active: activeCliTab === 'gemini' }" @click="activeCliTab = 'gemini'">Gemini</div>
              </div>

              <div class="cli-form-container">
                <CliSettingsForm
                  ref="cliFormRef"
                  :key="activeCliTab"
                  :cli-type="activeCliTab"
                  :settings="settingsStore.settings?.cli_settings?.[activeCliTab]"
                  @save="saveCli"
                />
              </div>
            </div>
          </div>
        </div>

        <!-- Right Column: Core & Backup -->
        <div class="config-column">

          <!-- Timeout Card -->
          <div class="frost-card">
            <div class="card-header-simple">
              <svg width="20" height="20" class="header-icon"><use href="#icon-activity"/></svg>
              <span class="card-label">基础配置</span>
              <div style="flex: 1;"></div>
              <button class="b-button" style="padding: 6px 14px;" @click="saveTimeouts">
                <svg width="16" height="16" style="margin-right: 6px;"><use href="#icon-save"/></svg>
                保存
              </button>
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
            </div>
          </div>

          <!-- Backup Card -->
          <div class="frost-card">
            <div class="card-header-simple">
              <svg width="20" height="20" class="header-icon"><use href="#icon-cloud"/></svg>
              <span class="card-label">备份与同步</span>
            </div>
            <div class="card-body">
              <div class="backup-row">
                <div class="frost-segmented backup-segmented">
                  <div class="seg-item" :class="{ active: activeBackupTab === 'local' }" @click="activeBackupTab = 'local'">本地备份</div>
                  <div class="seg-item" :class="{ active: activeBackupTab === 'webdav' }" @click="activeBackupTab = 'webdav'">WebDAV</div>
                </div>
                <div class="backup-actions">
                  <template v-if="activeBackupTab === 'local'">
                    <div class="action-icon" @click="!exportingLocal && handleExportLocal()" title="导出" :class="{ disabled: exportingLocal }">
                      <svg width="18" height="18"><use href="#icon-upload"/></svg>
                    </div>
                    <el-upload :show-file-list="false" :before-upload="handleImportLocal" accept=".db" :disabled="importingLocal">
                      <div class="action-icon" title="导入" :class="{ disabled: importingLocal }">
                        <svg width="18" height="18"><use href="#icon-download"/></svg>
                      </div>
                    </el-upload>
                  </template>
                  <template v-else>
                    <div class="action-icon" @click="showWebdavSettings" title="WebDAV设置">
                      <svg width="18" height="18"><use href="#icon-settings"/></svg>
                    </div>
                    <div class="action-icon" @click="!exportingWebdav && handleExportWebdav()" title="导出" :class="{ disabled: exportingWebdav }">
                      <svg width="18" height="18"><use href="#icon-upload"/></svg>
                    </div>
                    <div class="action-icon" @click="handleShowWebdavList" title="导入">
                      <svg width="18" height="18"><use href="#icon-download"/></svg>
                    </div>
                  </template>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- WebDAV Backup List Dialog -->
    <AppModal v-model="webdavListVisible" title="管理 WebDAV 备份" width="720px" :show-footer="false">
        <div v-loading="loadingWebdavList" style="max-height: 60vh; display: flex; flex-direction: column; margin: -32px;">
            <div class="scroll-area" style="flex: 1; overflow-y: auto; padding: 32px;">
              <table class="flat-table" style="width: 100%;">
                <tbody>
                  <tr v-for="backup in webdavBackups" :key="backup.filename">
                    <td class="mono" style="padding: 16px 0; border-bottom: 1px solid #f1f5f9;">{{ backup.filename }}</td>
                    <td class="mono" style="width: 120px; padding: 16px 20px; border-bottom: 1px solid #f1f5f9;">{{ formatSize(backup.size) }}</td>
                    <td style="width: 140px; text-align: right; padding: 16px 0; border-bottom: 1px solid #f1f5f9;">
                      <div style="display: inline-flex; gap: 16px;">
                        <a class="table-link" @click="handleImportWebdav(backup.filename)">恢复</a>
                        <a class="table-link danger" @click="handleDeleteWebdav(backup.filename)">删除</a>
                      </div>
                    </td>
                  </tr>
                  <tr v-if="webdavBackups.length === 0">
                    <td colspan="3" style="text-align: center; color: #94a3b8; padding: 60px; font-size: 13px;">暂无备份</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
    </AppModal>

    <!-- WebDAV Settings Dialog -->
    <AppModal v-model="webdavSettingsVisible" title="WebDAV 设置" width="480px" :show-footer="false">
      <div class="webdav-settings-form">
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
            <label class="item-label">密码</label>
            <input type="password" v-model="webdavForm.password" class="f-input">
          </div>
        </div>
        <div class="webdav-settings-footer">
          <button class="f-button ghost-plain" @click="handleTestWebdav" :disabled="testingWebdav">
            {{ testingWebdav ? '测试中...' : '测试链接' }}
          </button>
          <button class="b-button" @click="handleSaveWebdav" :disabled="savingWebdav">
            {{ savingWebdav ? '保存中...' : '保存' }}
          </button>
        </div>
      </div>
    </AppModal>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, computed } from 'vue'
import { ElMessageBox } from 'element-plus'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'
import { useSettingsStore } from '@/stores/settings'
import { useUiStore } from '@/stores/ui'
import AppModal from '@/components/AppModal.vue'
import CliSettingsForm from './components/CliSettingsForm.vue'
import * as backupApi from '@/api/backup'
import type { WebdavSettings, WebdavBackup } from '@/api/backup'

const settingsStore = useSettingsStore()
const uiStore = useUiStore()
const cliFormRef = ref<InstanceType<typeof CliSettingsForm> | null>(null)

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
  try {
    await settingsStore.updateTimeouts(timeoutForm.value)
    notify('超时配置已保存')
  } catch (e: any) {
    notify(getErrorMessage(e, '保存失败'), 'error')
  }
}

async function saveCli(cliType: string, data: any) {
  try {
    await settingsStore.updateCli(cliType, data)
    notify('CLI 配置已保存')
  } catch (e: any) {
    notify(getErrorMessage(e, '保存失败'), 'error')
  }
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
const webdavSettingsVisible = ref(false)
const webdavBackups = ref<WebdavBackup[]>([])

async function loadWebdavSettings() {
  try {
    const { data } = await backupApi.getWebdavSettings()
    webdavForm.value = data
  } catch {}
}

function showWebdavSettings() {
  webdavSettingsVisible.value = true
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
    notify(getErrorMessage(error, '导出失败'), 'error')
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
    notify(getErrorMessage(error, '连接失败'), 'error')
  } finally {
    testingWebdav.value = false
  }
}

async function handleSaveWebdav() {
  savingWebdav.value = true
  try {
    await backupApi.updateWebdavSettings(webdavForm.value)
    notify('WebDAV 配置已保存')
    webdavSettingsVisible.value = false
  } catch (error: any) {
    notify(getErrorMessage(error, '保存失败'), 'error')
  } finally {
    savingWebdav.value = false
  }
}

async function handleExportWebdav() {
  exportingWebdav.value = true
  try {
    const { data } = await backupApi.exportToWebdav()
    notify(`同步成功: ${data.filename}`)
  } catch (error: any) {
    notify(getErrorMessage(error, '同步失败'), 'error')
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
    if (error !== 'cancel') notify(getErrorMessage(error, '导入失败'), 'error')
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
    if (error !== 'cancel') notify(getErrorMessage(error, '删除失败'), 'error')
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
  height: 100%;
  display: flex;
  flex-direction: column;
}

.scroll-area {
  flex: 1;
  overflow-y: auto;
  padding: 0 40px 16px 40px;
}

/* Header */
.page-header { margin-bottom: 32px; flex-shrink: 0; }
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

/* Action Icon Buttons */
.action-icon {
  width: 34px;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  color: #64748b;
  cursor: pointer;
  transition: all 0.2s;
  background: transparent;
  flex-shrink: 0;
}
.action-icon:hover {
  background: #f1f5f9;
  color: #0f172a;
}

.desc-text { font-size: 13px; color: #94a3b8; line-height: 1.6; margin: 0 0 20px 0; }

/* CLI Column adjustment */
.cli-settings-card { flex: 1; }
.cli-form-container { flex: 1; min-height: 400px; display: flex; flex-direction: column; }

/* Flat Table (matching logs page style) */
.table-container { background: #ffffff; border-radius: 12px; padding: 0; border: 1px solid #e2e8f0; box-shadow: 0 4px 15px rgba(0,0,0,0.02); overflow: hidden; }
.flat-table { width: 100%; border-collapse: collapse; text-align: left; }
.flat-table th, .flat-table td { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.flat-table th { padding: 12px 20px; font-size: 12px; font-weight: 600; color: #64748b; text-transform: uppercase; border-bottom: 1px solid #e2e8f0; background: #f8fafc; }
.flat-table td { padding: 12px 20px; font-size: 13px; color: #0f172a; border-bottom: 1px solid #f1f5f9; }
.flat-table tr:last-child td { border-bottom: none; }
.flat-table tr:hover td { background: #f8fafc; }
.mono { font-family: "JetBrains Mono", monospace; color: #64748b; font-size: 12px; }
.table-link { color: #0ea5e9; cursor: pointer; text-decoration: none; font-weight: 500; font-size: 13px; }
.table-link:hover { text-decoration: underline; }
.table-link.danger { color: #ef4444; }
.table-link.danger:hover { color: #dc2626; }

/* Custom Scrollbar for Modal */
.scroll-area::-webkit-scrollbar {
  width: 6px;
}
.scroll-area::-webkit-scrollbar-track {
  background: transparent;
}
.scroll-area::-webkit-scrollbar-thumb {
  background: #e2e8f0;
  border-radius: 10px;
}
.scroll-area::-webkit-scrollbar-thumb:hover {
  background: #cbd5e1;
}

/* Backup Row */
.backup-row { display: flex; align-items: center; justify-content: space-between; gap: 24px; }
.backup-segmented { margin-bottom: 0; flex-shrink: 0; }
.backup-actions { display: flex; gap: 8px; }
.backup-actions .action-icon.disabled { opacity: 0.5; cursor: not-allowed; }

/* WebDAV Settings Form */
.webdav-settings-form { padding: 4px 0; }
.webdav-settings-footer { display: flex; justify-content: flex-end; gap: 12px; margin-top: 24px; }

.b-button {
  background: #0ea5e9; color: #ffffff; border: none; padding: 10px 20px; border-radius: 10px;
  font-size: 14px; font-weight: 600; cursor: pointer; display: flex; align-items: center;
  transition: all 0.2s;
}
.b-button:hover { background: #0284c7; }
</style>
