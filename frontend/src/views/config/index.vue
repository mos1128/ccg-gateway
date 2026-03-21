<template>
  <div class="config-page">
    <div class="two-column-layout">
      <div class="column">
        <!-- Timeout Settings -->
        <el-card class="config-card">
          <template #header>基础配置</template>
          <el-form :model="timeoutForm" label-width="140px">
            <el-form-item label="流式首字节超时">
              <el-input-number v-model="timeoutForm.stream_first_byte_timeout" :min="1" />
              <span class="unit">秒</span>
            </el-form-item>
            <el-form-item label="流式空闲超时">
              <el-input-number v-model="timeoutForm.stream_idle_timeout" :min="1" />
              <span class="unit">秒</span>
            </el-form-item>
            <el-form-item label="非流式超时">
              <el-input-number v-model="timeoutForm.non_stream_timeout" :min="1" />
              <span class="unit">秒</span>
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="saveTimeouts">保存</el-button>
            </el-form-item>
          </el-form>
        </el-card>

        <!-- Backup Settings -->
        <el-card class="config-card">
          <template #header>备份与恢复</template>
          <el-tabs v-model="activeBackupTab">
            <el-tab-pane label="本地备份" name="local">
              <p class="backup-desc">将数据库文件导出到本地，或从本地文件恢复</p>
              <div class="backup-actions">
                <el-button type="primary" @click="handleExportLocal" :loading="exportingLocal">导出到本地</el-button>
                <el-upload :show-file-list="false" :before-upload="handleImportLocal" accept=".db">
                  <el-button type="warning" :loading="importingLocal">从本地导入</el-button>
                </el-upload>
              </div>
            </el-tab-pane>
            <el-tab-pane label="WebDAV" name="webdav">
              <el-form :model="webdavForm" label-width="90px">
                <el-form-item label="服务器地址">
                  <el-input v-model="webdavForm.url" placeholder="https://dav.example.com" />
                </el-form-item>
                <el-form-item label="用户名">
                  <el-input v-model="webdavForm.username" />
                </el-form-item>
                <el-form-item label="密码">
                  <el-input v-model="webdavForm.password" type="password" show-password />
                </el-form-item>
              </el-form>
              <div class="backup-actions">
                <el-button @click="handleTestWebdav" :loading="testingWebdav">测试连接</el-button>
                <el-button @click="handleSaveWebdav" :loading="savingWebdav">保存配置</el-button>
                <el-button type="primary" @click="handleExportWebdav" :loading="exportingWebdav">导出到WebDAV</el-button>
                <el-button type="warning" @click="handleShowWebdavList" :loading="loadingWebdavList">从WebDAV导入</el-button>
              </div>
            </el-tab-pane>
          </el-tabs>
        </el-card>
      </div>

      <div class="column">
        <!-- CLI Settings -->
        <el-card class="config-card">
          <template #header>CLI全局配置</template>
          <el-tabs v-model="activeCliTab">
            <el-tab-pane label="ClaudeCode" name="claude_code">
              <CliSettingsForm cli-type="claude_code" :settings="settingsStore.settings?.cli_settings?.claude_code" @save="saveCli" />
            </el-tab-pane>
            <el-tab-pane label="Codex" name="codex">
              <CliSettingsForm cli-type="codex" :settings="settingsStore.settings?.cli_settings?.codex" @save="saveCli" />
            </el-tab-pane>
            <el-tab-pane label="Gemini" name="gemini">
              <CliSettingsForm cli-type="gemini" :settings="settingsStore.settings?.cli_settings?.gemini" @save="saveCli" />
            </el-tab-pane>
          </el-tabs>
        </el-card>

      </div>
    </div>

    <!-- WebDAV Backup List Dialog -->
    <el-dialog v-model="webdavListVisible" title="选择备份文件" width="700px">
      <el-table :data="webdavBackups" v-loading="loadingWebdavList">
        <el-table-column prop="filename" label="文件名" min-width="280" />
        <el-table-column prop="size" label="大小" width="100">
          <template #default="{ row }">{{ formatSize(row.size) }}</template>
        </el-table-column>
        <el-table-column label="操作" width="160">
          <template #default="{ row }">
            <el-button type="primary" size="small" @click="handleImportWebdav(row.filename)" :loading="importingWebdav">导入</el-button>
            <el-button type="danger" size="small" @click="handleDeleteWebdav(row.filename)" :loading="deletingWebdav">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-dialog>
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
    notify('导出成功（默认保存至下载文件夹）')
  } catch (error: any) {
    notify(error?.message || '导出失败', 'error')
  } finally {
    exportingLocal.value = false
  }
}

async function handleImportLocal(file: File) {
  await ElMessageBox.confirm('导入将覆盖当前所有数据，确定继续？', '警告', { type: 'warning' })
  importingLocal.value = true
  try {
    await backupApi.importFromLocal(file)
    notify('导入成功，应用将自动退出，请重新打开应用')
  } finally {
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
  savingWebdav.value = true
  try {
    await backupApi.updateWebdavSettings(webdavForm.value)
    notify('WebDAV 配置已保存')
  } catch (error: any) {
    notify(error?.message || '保存失败', 'error')
  } finally {
    savingWebdav.value = false
  }
}

async function handleExportWebdav() {
  exportingWebdav.value = true
  try {
    const { data } = await backupApi.exportToWebdav()
    notify(`导出成功: ${data.filename}`)
  } catch (error: any) {
    notify(error?.message || '导出失败', 'error')
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
  await ElMessageBox.confirm('导入将覆盖当前所有数据，确定继续？', '警告', { type: 'warning' })
  importingWebdav.value = true
  try {
    await backupApi.importFromWebdav(filename)
    notify('导入成功，应用将自动退出，请重新打开应用')
    webdavListVisible.value = false
  } catch (error: any) {
    notify(error?.message || '导入失败', 'error')
  } finally {
    importingWebdav.value = false
  }
}

async function handleDeleteWebdav(filename: string) {
  await ElMessageBox.confirm(`确定要删除远程备份 ${filename} 吗？`, '警告', { type: 'warning' })
  deletingWebdav.value = true
  try {
    await backupApi.deleteWebdavBackup(filename)
    notify('删除成功')
    // 重新加载备份列表
    await handleShowWebdavList()
  } catch (error: any) {
    notify(error?.message || '删除失败', 'error')
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
.two-column-layout {
  display: flex;
  gap: 20px;
  align-items: flex-start;
}
.column {
  flex: 1;
  flex-basis: 0;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 20px;
}
.unit {
  margin-left: 10px;
  color: #999;
}
.backup-desc {
  color: #909399;
  font-size: 13px;
  margin: 0 0 15px 0;
}
.backup-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-top: 12px;
}
</style>
