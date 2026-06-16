<template>
  <div v-loading="!settingsStore.settings">
    <div class="cfg-grid">
      <!-- 左列 -->
      <div class="cfg-col">
        <!-- CLI 运行配置 -->
        <div class="v2-card v2-card-pad cfg-cli">
          <div class="cfg-head">Agent 运行配置</div>
          <div class="v2-tabs cfg-tabs">
            <div v-for="cli in CLI_TABS" :key="cli.id" class="v2-tab" :class="{ active: activeCliTab === cli.id }" @click="activeCliTab = cli.id">
              <span class="tab-label-text">{{ cli.label }}</span>
            </div>
          </div>

          <div class="v2-field">
            <label class="v2-label">Agent 目录</label>
            <div class="cfg-dir">
              <input v-model="cliForm.config_dir" class="v2-input mono" placeholder="Agent 配置目录" @blur="saveCliConfigDir">
              <el-tooltip content="恢复默认" placement="top" effect="light" :show-after="250">
                <button class="v2-row-act" @click="handleRestoreDefault"><svg :class="{ 'spin-once': isRestoring }" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/></svg></button>
              </el-tooltip>
            </div>
          </div>

          <div class="v2-field">
            <label class="v2-label">全局预设</label>
            <div class="cfg-preset-card" :class="{ empty: !cliForm.default_json_config }" @click="openPresetDrawer">
              <div class="cfg-preset-body">
                <template v-if="cliForm.default_json_config">
                  <pre class="cfg-preset-code mono"><code>{{ getPresetPreviewText(cliForm.default_json_config) }}</code></pre>
                  <div class="cfg-preset-overlay">
                    <span class="overlay-text">
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                        <polygon points="5 3 19 12 5 21 5 3"/>
                      </svg>
                      点击编辑配置
                    </span>
                  </div>
                </template>
                <div v-else class="cfg-preset-empty-state">
                  <span class="empty-hint">未配置预设，将使用全局默认设置</span>
                </div>
              </div>
            </div>
          </div>

        </div>

        <!-- 备份与同步 -->
        <div class="v2-card v2-card-pad">
          <div class="cfg-head">备份与同步</div>
          <div class="cfg-backup">
            <div class="v2-seg">
              <div class="v2-seg-slider" :style="{ transform: `translateX(${activeBackupTab === 'local' ? 0 : 1}00%)`, width: 'calc((100% - 8px) / 2)' }"></div>
              <button class="v2-seg-btn" :class="{ active: activeBackupTab === 'local' }" @click="activeBackupTab = 'local'">本地备份</button>
              <button class="v2-seg-btn" :class="{ active: activeBackupTab === 'webdav' }" @click="activeBackupTab = 'webdav'">WebDAV</button>
            </div>
            <div class="cfg-backup-acts">
              <template v-if="activeBackupTab === 'local'">
                <el-tooltip content="导出" placement="top" effect="light" :show-after="250">
                  <button class="v2-row-act" :class="{ off: exportingLocal }" @click="!exportingLocal && handleExportLocal()"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg></button>
                </el-tooltip>
                <el-upload :show-file-list="false" :before-upload="handleImportLocal" accept=".db" :disabled="importingLocal">
                  <el-tooltip content="导入" placement="top" effect="light" :show-after="250">
                    <button class="v2-row-act" :class="{ off: importingLocal }"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg></button>
                  </el-tooltip>
                </el-upload>
              </template>
              <template v-else>
                <el-tooltip content="WebDAV 设置" placement="top" effect="light" :show-after="250">
                  <button class="v2-row-act" @click="webdavSettingsVisible = true"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg></button>
                </el-tooltip>
                <el-tooltip content="导出到 WebDAV" placement="top" effect="light" :show-after="250">
                  <button class="v2-row-act" :class="{ off: exportingWebdav }" @click="!exportingWebdav && handleExportWebdav()"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg></button>
                </el-tooltip>
                <el-tooltip content="从 WebDAV 导入" placement="top" effect="light" :show-after="250">
                  <button class="v2-row-act" @click="handleShowWebdavList"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg></button>
                </el-tooltip>
              </template>
            </div>
          </div>
        </div>
      </div>

      <!-- 右列 -->
      <div class="cfg-col">
        <!-- 请求超时 -->
        <div class="v2-card v2-card-pad">
          <div class="cfg-head">请求超时</div>
          <div class="cfg-trow">
            <span class="cfg-trow-l">流式首字节超时</span>
            <div class="cfg-input-wrapper">
              <input v-model.number="timeoutForm.stream_first_byte_timeout" type="number" class="v2-input cfg-tnum" @change="saveTimeouts">
              <span class="cfg-input-unit">秒</span>
            </div>
          </div>
          <div class="cfg-trow">
            <span class="cfg-trow-l">流式空闲超时</span>
            <div class="cfg-input-wrapper">
              <input v-model.number="timeoutForm.stream_idle_timeout" type="number" class="v2-input cfg-tnum" @change="saveTimeouts">
              <span class="cfg-input-unit">秒</span>
            </div>
          </div>
          <div class="cfg-trow">
            <span class="cfg-trow-l">非流式超时</span>
            <div class="cfg-input-wrapper">
              <input v-model.number="timeoutForm.non_stream_timeout" type="number" class="v2-input cfg-tnum" @change="saveTimeouts">
              <span class="cfg-input-unit">秒</span>
            </div>
          </div>
        </div>

        <!-- 基础配置 -->
        <div class="v2-card v2-card-pad">
          <div class="cfg-head">基础配置</div>
          <label class="cfg-toggle">
            <span class="cfg-toggle-c"><span class="cfg-toggle-t">开机自启</span><span class="cfg-toggle-d">登录系统后自动启动 CCG Gateway</span></span>
            <el-switch v-model="gatewayForm.launch_on_startup" :loading="gatewaySaving" @change="saveGateway" />
          </label>
          <label class="cfg-toggle">
            <span class="cfg-toggle-c"><span class="cfg-toggle-t">静默启动</span><span class="cfg-toggle-d">启动时不显示主窗口，仅在托盘运行</span></span>
            <el-switch v-model="gatewayForm.silent_startup" :loading="gatewaySaving" @change="saveGateway" />
          </label>
          <label class="cfg-toggle">
            <span class="cfg-toggle-c"><span class="cfg-toggle-t">关闭时最小化到托盘</span><span class="cfg-toggle-d">点关闭按钮时隐藏窗口，应用继续后台运行</span></span>
            <el-switch v-model="gatewayForm.minimize_to_tray_on_close" :loading="gatewaySaving" @change="saveGateway" />
          </label>
        </div>
      </div>
    </div>

    <V2Drawer v-model="webdavSettingsVisible" title="WebDAV 设置" @confirm="handleSaveWebdav">
      <div class="v2-field"><label class="v2-label">服务器地址</label><input v-model="webdavForm.url" class="v2-input" placeholder="https://dav.jianguoyun.com/dav/"></div>
      <div class="v2-grid-2">
        <div class="v2-field"><label class="v2-label">用户名</label><input v-model="webdavForm.username" class="v2-input"></div>
        <div class="v2-field">
          <label class="v2-label">密码</label>
          <div class="v2-input-wrapper">
            <input v-model="webdavForm.password" :type="showWebdavPassword ? 'text' : 'password'" class="v2-input">
            <el-tooltip :content="showWebdavPassword ? '隐藏密码' : '显示密码'" placement="top" effect="light" :show-after="250">
              <button type="button" class="v2-input-icon-btn" @click="showWebdavPassword = !showWebdavPassword">
                <svg v-if="showWebdavPassword" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
                <svg v-else width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
              </button>
            </el-tooltip>
          </div>
        </div>
      </div>
      <template #footer>
        <button class="v2-btn v2-btn-sm v2-btn-ghost" :disabled="testingWebdav" style="margin-right:auto" @click="handleTestWebdav">{{ testingWebdav ? '测试中...' : '测试链接' }}</button>
        <button class="v2-btn v2-btn-sm v2-btn-ghost" @click="webdavSettingsVisible = false">取消</button>
        <button class="v2-btn v2-btn-sm v2-btn-primary" :disabled="savingWebdav" @click="handleSaveWebdav">{{ savingWebdav ? '保存中...' : '保存' }}</button>
      </template>
    </V2Drawer>

    <V2Drawer v-model="webdavListVisible" title="管理 WebDAV 备份" :show-footer="false">
      <div v-loading="loadingWebdavList">
        <div class="webdav-table-wrapper">
          <table class="v2-table webdav-table">
            <thead>
              <tr>
                <th>备份文件</th>
                <th>大小</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="backup in webdavBackups" :key="backup.filename">
                <td class="mono">
                  <div class="webdav-file-cell">
                    <svg class="db-icon" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                      <ellipse cx="12" cy="5" rx="9" ry="3"></ellipse>
                      <path d="M3 5V19A9 3 0 0 0 21 19V5"></path>
                      <path d="M3 12A9 3 0 0 0 21 12"></path>
                    </svg>
                    <span class="filename-text">{{ backup.filename }}</span>
                  </div>
                </td>
                <td class="mono">{{ formatSize(backup.size) }}</td>
                <td>
                  <div class="webdav-row-actions">
                    <a class="webdav-link" @click="handleImportWebdav(backup.filename)">恢复</a>
                    <a class="webdav-link danger" @click="handleDeleteWebdav(backup.filename)">删除</a>
                  </div>
                </td>
              </tr>
              <tr v-if="webdavBackups.length === 0">
                <td colspan="3" class="v2-hint" style="text-align:center;padding:40px">
                  <div class="empty-state-content">
                    <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" style="color: var(--v2-text-3); margin-bottom: 8px;">
                      <ellipse cx="12" cy="5" rx="9" ry="3"></ellipse>
                      <path d="M3 5V19A9 3 0 0 0 21 19V5"></path>
                      <path d="M3 12A9 3 0 0 0 21 12"></path>
                    </svg>
                    <div>暂无备份文件</div>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </V2Drawer>

    <!-- 全局预设编辑抽屉 -->
    <V2Drawer v-model="presetDrawerVisible" :title="`编辑全局预设 - ${activeCliLabel}`" @confirm="handleSavePreset">
      <div class="preset-drawer-body">
        <div class="v2-field preset-editor-field">
          <div class="v2-file-editor preset-file-editor">
            <div class="v2-file-editor-header">
              <div class="v2-file-editor-title">
                <svg class="file-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                  <polyline points="14 2 14 8 20 8"/>
                </svg>
                <span class="v2-file-editor-name">{{ isJsonFormat ? 'settings.json' : 'config.toml' }}</span>
              </div>
              <button v-if="isJsonFormat" class="v2-file-editor-action" type="button" @click="formatPresetJson">
                <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>
                <span>格式化</span>
              </button>
            </div>
            <div class="v2-file-editor-body preset-editor-body">
              <V2CodeEditor
                v-model="presetTempConfig"
                class="cfg-preset-editor"
                :placeholder="placeholder"
                @blur="validatePresetConfig"
              />
            </div>
          </div>
          <div v-if="presetValidationError" class="json-err" style="color: var(--v2-danger); font-size: var(--v2-fs-xs); margin-top: 6px;">{{ presetValidationError }}</div>
        </div>

        <div class="cfg-writemode-section">
          <div class="cfg-writemode-row">
            <div class="v2-seg" style="max-width: 200px;">
              <div class="v2-seg-slider" :style="{ transform: `translateX(${presetTempWriteMode === 'merge' ? 0 : 1}00%)`, width: 'calc((100% - 8px) / 2)' }"></div>
              <button class="v2-seg-btn" :class="{ active: presetTempWriteMode === 'merge' }" type="button" @click="presetTempWriteMode = 'merge'">增量合并</button>
              <button class="v2-seg-btn" :class="{ active: presetTempWriteMode === 'overwrite' }" type="button" @click="presetTempWriteMode = 'overwrite'">全量写入</button>
            </div>
            <el-tooltip
              effect="light"
              placement="top"
              :offset="10"
              :show-after="150"
              :enterable="true"
              popper-class="v2-profile-pop v2-scope"
            >
              <template #content>
                <div class="write-mode-help-content">
                  <div class="tooltip-title">配置写入模式</div>
                  <div class="tooltip-item">
                    <strong>增量合并</strong>
                    <span>只写入需要变更的字段，保留配置文件中已有的其他配置（如 MCP / plugin 开关等配置）。</span>
                  </div>
                  <div class="tooltip-item">
                    <strong>全量写入</strong>
                    <span>每次写入时完全覆盖配置文件。中转路由会备份原始文件，关闭时自动恢复。保持配置干净。</span>
                  </div>
                </div>
              </template>
              <span class="help-icon-wrapper" style="display: inline-flex; align-items: center; justify-content: center; cursor: pointer; color: var(--v2-text-3);">
                <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="help-icon">
                  <circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><line x1="12" y1="17" x2="12.01" y2="17"/>
                </svg>
              </span>
            </el-tooltip>
          </div>
        </div>
      </div>
    </V2Drawer>
  </div>
</template>

<script setup lang="ts">
import V2Drawer from '@/components/V2Drawer.vue'
import V2CodeEditor from '@/components/V2CodeEditor.vue'
import { confirm } from '@/utils/confirm'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'
import { useSettingsStore } from '@/stores/settings'
import { useUiStore } from '@/stores/ui'
import { CLI_TABS } from '@/types/models'
import { validateJson, formatJson as formatJsonUtil } from '@/utils/json'
import * as backupApi from '@/api/backup'
import type { WebdavSettings, WebdavBackup } from '@/api/backup'
import type { CliType } from '@/types/models'

const settingsStore = useSettingsStore()
const uiStore = useUiStore()

const activeCliTab = computed<CliType>({
  get: () => uiStore.configActiveCliTab,
  set: (v) => uiStore.setConfigActiveCliTab(v)
})
const activeBackupTab = computed({
  get: () => uiStore.configActiveBackupTab,
  set: (v) => uiStore.setConfigActiveBackupTab(v as 'local' | 'webdav')
})

// ===== CLI 运行配置 =====
const cliForm = ref({ config_dir: '', default_json_config: '', config_write_mode: 'merge' as 'overwrite' | 'merge' })
const defaultConfigDir = ref('')
const validationError = ref('')
const isRestoring = ref(false)

const presetDrawerVisible = ref(false)
const presetTempConfig = ref('')
const presetTempWriteMode = ref<'overwrite' | 'merge'>('merge')
const presetValidationError = ref('')

const activeCliLabel = computed(() => {
  switch (activeCliTab.value) {
    case 'claude_code': return 'Claude Code'
    case 'codex': return 'Codex'
    case 'gemini': return 'Gemini'
    default: return activeCliTab.value
  }
})

function getPresetPreviewText(config: string): string {
  if (!config) return ''
  const trimmed = config.trim()
  if (!trimmed) return ''
  
  let formatted = trimmed
  if (isJsonFormat.value) {
    try {
      formatted = JSON.stringify(JSON.parse(trimmed), null, 2)
    } catch {
      // fallback
    }
  }
  
  const lines = formatted.split('\n')
  if (lines.length > 5) {
    return lines.slice(0, 4).join('\n') + '\n...'
  }
  return lines.join('\n')
}

function openPresetDrawer() {
  presetTempConfig.value = cliForm.value.default_json_config
  presetTempWriteMode.value = cliForm.value.config_write_mode || 'merge'
  presetValidationError.value = ''
  presetDrawerVisible.value = true
}

function validatePresetConfig(): boolean {
  presetValidationError.value = ''
  const config = presetTempConfig.value.trim()
  if (!config) return true
  if (isJsonFormat.value) {
    presetValidationError.value = validateJson(config)
    return !presetValidationError.value
  }
  if (activeCliTab.value === 'codex') {
    if (config.includes('{') || (config.includes('[') && config.includes(']') && config.includes(','))) {
      presetValidationError.value = 'TOML 格式错误: 请使用 TOML 格式而非 JSON 格式'
      return false
    }
  }
  return true
}

function formatPresetJson() {
  const result = formatJsonUtil(presetTempConfig.value)
  if (result === presetTempConfig.value) {
    presetValidationError.value = validateJson(presetTempConfig.value)
  } else {
    presetTempConfig.value = result
    presetValidationError.value = ''
  }
}

async function handleSavePreset() {
  if (!validatePresetConfig()) {
    notify('配置格式错误，请修正后再保存', 'error')
    return
  }
  try {
    cliForm.value.default_json_config = presetTempConfig.value
    cliForm.value.config_write_mode = presetTempWriteMode.value
    await settingsStore.updateCli(activeCliTab.value, { ...cliForm.value })
    notify('预设配置已保存')
    presetDrawerVisible.value = false
  } catch (e: any) {
    notify(getErrorMessage(e, '保存失败'), 'error')
  }
}

const isJsonFormat = computed(() => activeCliTab.value === 'claude_code' || activeCliTab.value === 'gemini')
const placeholder = computed(() => {
  switch (activeCliTab.value) {
    case 'codex': return 'model_reasoning_effort = "high"\nmodel_reasoning_summary = "detailed"'
    case 'claude_code': return '{\n  "env": {},\n  "permissions": {}\n}'
    case 'gemini': return '{\n  "theme": "dark"\n}'
    default: return '{}'
  }
})

function loadCliForm() {
  const s = settingsStore.settings?.cli_settings?.[activeCliTab.value]
  if (s) {
    cliForm.value = { config_dir: s.config_dir, default_json_config: s.default_json_config, config_write_mode: s.config_write_mode || 'merge' }
    defaultConfigDir.value = s.default_config_dir
    validationError.value = ''
  }
}
watch([() => settingsStore.settings, activeCliTab], loadCliForm, { immediate: true })

async function saveCliConfigDir() {
  const s = settingsStore.settings?.cli_settings?.[activeCliTab.value]
  if (s && s.config_dir === cliForm.value.config_dir) {
    return
  }
  try {
    await settingsStore.updateCli(activeCliTab.value, { ...cliForm.value })
    notify('Agent 目录已更新')
  } catch (e: any) {
    notify(getErrorMessage(e, '更新失败'), 'error')
  }
}

async function handleRestoreDefault() {
  if (isRestoring.value) return
  isRestoring.value = true
  if (cliForm.value.config_dir !== defaultConfigDir.value) {
    cliForm.value.config_dir = defaultConfigDir.value
    await saveCliConfigDir()
  }
  setTimeout(() => {
    isRestoring.value = false
  }, 600)
}

// ===== 超时 / 基础配置 =====
const timeoutForm = ref({ stream_first_byte_timeout: 30, stream_idle_timeout: 60, non_stream_timeout: 120 })
const gatewayForm = ref({ launch_on_startup: false, silent_startup: false, minimize_to_tray_on_close: true })
const gatewaySaving = ref(false)

watch(() => settingsStore.settings, (settings) => {
  if (settings) {
    timeoutForm.value = { ...settings.timeouts }
    gatewayForm.value = {
      launch_on_startup: settings.gateway.launch_on_startup,
      silent_startup: settings.gateway.silent_startup,
      minimize_to_tray_on_close: settings.gateway.minimize_to_tray_on_close
    }
  }
}, { immediate: true })

async function saveTimeouts() {
  try {
    await settingsStore.updateTimeouts(timeoutForm.value)
    notify('请求超时已保存')
  } catch (e: any) {
    notify(getErrorMessage(e, '保存失败'), 'error')
  }
}
async function saveGateway() {
  gatewaySaving.value = true
  try {
    await settingsStore.updateGateway(gatewayForm.value)
    notify('基础配置已保存')
  } catch (e: any) {
    notify(getErrorMessage(e, '保存失败'), 'error')
  } finally {
    gatewaySaving.value = false
  }
}

// ===== 备份 =====
const webdavForm = ref<WebdavSettings>({ url: '', username: '', password: '' })
const showWebdavPassword = ref(false)

const exportingLocal = ref(false)
const importingLocal = ref(false)
const testingWebdav = ref(false)
const savingWebdav = ref(false)
const exportingWebdav = ref(false)
const loadingWebdavList = ref(false)
const webdavListVisible = ref(false)
const webdavSettingsVisible = ref(false)
const webdavBackups = ref<WebdavBackup[]>([])

watch(webdavSettingsVisible, (open) => {
  if (open) {
    showWebdavPassword.value = false
  }
})

async function loadWebdavSettings() {
  try {
    const { data } = await backupApi.getWebdavSettings()
    webdavForm.value = data
  } catch { /* ignore */ }
}
async function handleExportLocal() {
  exportingLocal.value = true
  try {
    const success = await backupApi.exportToLocalWithDialog()
    if (success) notify('导出成功')
  } catch (error: any) {
    notify(getErrorMessage(error, '导出失败'), 'error')
  } finally {
    exportingLocal.value = false
  }
}
async function handleImportLocal(file: File) {
  try {
    await confirm('导入将覆盖当前所有数据，确定继续？', '警告')
    importingLocal.value = true
    await backupApi.importFromLocal(file)
    notify('导入成功，应用将自动退出，请重新打开应用')
  } catch { /* cancel */ } finally {
    importingLocal.value = false
  }
  return false
}
async function handleTestWebdav() {
  testingWebdav.value = true
  try {
    const { data } = await backupApi.testWebdavConnection(webdavForm.value)
    notify(data.success ? '连接成功' : '连接失败', data.success ? 'success' : 'error')
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
    await confirm('导入将覆盖当前所有数据，确定继续？', '警告')
    await backupApi.importFromWebdav(filename)
    notify('导入成功，应用将自动退出，请重新打开应用')
    webdavListVisible.value = false
  } catch (error: any) {
    if (error !== 'cancel') notify(getErrorMessage(error, '导入失败'), 'error')
  }
}
async function handleDeleteWebdav(filename: string) {
  try {
    await confirm(`确定要删除远程备份 ${filename} 吗？`, '警告')
    await backupApi.deleteWebdavBackup(filename)
    notify('已删除')
    await handleShowWebdavList()
  } catch (error: any) {
    if (error !== 'cancel') notify(getErrorMessage(error, '删除失败'), 'error')
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
.cfg-grid { display: grid; grid-template-columns: 1.3fr 1fr; gap: 16px; align-items: stretch; }
@media (max-width: 940px) { .cfg-grid { grid-template-columns: 1fr; } }
.cfg-col { display: flex; flex-direction: column; gap: 16px; min-width: 0; }
.cfg-cli { display: flex; flex-direction: column; }



.cfg-head { font-size: var(--v2-fs-base); font-weight: var(--v2-fw-semibold); color: var(--v2-text); margin-bottom: 16px; }
.cfg-tabs { margin-bottom: 18px; }

.cfg-dir { position: relative; display: flex; align-items: center; }
.cfg-dir .v2-input { flex: 1; padding-right: 36px; }
.cfg-dir .v2-row-act { position: absolute; right: 4px; border: none; flex-shrink: 0; }
.cfg-dir .v2-row-act svg { fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }

.cfg-preset-card {
  background: var(--v2-bg-base);
  border: 1px solid transparent;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s ease;
  overflow: hidden;
  position: relative;
  display: flex;
  flex-direction: column;
}
.cfg-preset-card:hover {
  border-color: transparent;
}
.cfg-preset-body {
  position: relative;
  padding: 12px 14px;
  min-height: 44px;
  background: var(--v2-bg-base);
  transition: background-color 0.2s;
}
.cfg-preset-card:hover .cfg-preset-body {
  background: var(--v2-bg-base);
}
.cfg-preset-code {
  margin: 0;
  font-size: 12px;
  line-height: 1.5;
  white-space: pre;
  overflow: hidden;
}

.cfg-preset-overlay {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: var(--v2-bg-base);
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  transition: opacity 0.2s ease;
  pointer-events: none;
}
.cfg-preset-card:hover .cfg-preset-overlay {
  opacity: 0.95;
}
.overlay-text {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background: var(--v2-surface);
  border: 1px solid var(--v2-surface-3);
  padding: 6px 12px;
  border-radius: 6px;
  font-size: 12px;
  font-weight: var(--v2-fw-medium);
  color: var(--v2-text);
}
.cfg-preset-empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 44px;
}
.empty-hint {
  font-size: 12px;
  color: var(--v2-text-3);
  font-style: italic;
}
.cfg-preset-editor {
  min-height: 320px;
}

.cfg-writemode-section {
  margin-top: 20px;
  padding-top: 16px;
}
.cfg-writemode-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
}
.editor-info-tip {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 12px 14px;
  border-radius: var(--v2-r);
  background: var(--v2-selected-bg);
  border: 1px solid var(--v2-surface-3);
  color: var(--v2-text-2);
  font-size: var(--v2-fs-xs);
  line-height: 1.5;
}
.editor-info-tip svg {
  color: var(--v2-accent);
  margin-top: 2px;
  flex-shrink: 0;
}
.tip-content {
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.tip-title {
  font-weight: var(--v2-fw-semibold);
  color: var(--v2-text);
}
.tip-desc {
  color: var(--v2-text-3);
  font-size: 11px;
}

.cfg-backup { display: flex; align-items: center; justify-content: space-between; gap: 16px; }
.cfg-backup-acts { display: flex; gap: 4px; align-items: center; }
.cfg-backup-acts .v2-row-act { border: 1px solid var(--v2-surface-2); }
.cfg-backup-acts .v2-row-act svg { fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }

.cfg-toggle { display: flex; align-items: center; justify-content: space-between; gap: 20px; padding: 12px 0; border-bottom: 1px solid var(--v2-surface-2); }
.cfg-toggle:last-child { border-bottom: none; }
.cfg-toggle-c { min-width: 0; }
.cfg-toggle-t { display: block; font-size: var(--v2-fs-sm); font-weight: var(--v2-fw-medium); color: var(--v2-text); }
.cfg-toggle-d { display: block; margin-top: 3px; font-size: var(--v2-fs-xs); line-height: 1.4; color: var(--v2-text-3); }

.cfg-trow { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 11px 0; border-bottom: 1px solid var(--v2-surface-2); }
.cfg-trow:last-child { border-bottom: none; }
.cfg-trow-l { font-size: var(--v2-fs-sm); color: var(--v2-text); }

.cfg-input-wrapper {
  position: relative;
  display: inline-flex;
  align-items: center;
}
.cfg-input-unit {
  position: absolute;
  right: 12px;
  font-size: var(--v2-fs-xs);
  color: var(--v2-text-3);
  pointer-events: none;
}
.cfg-tnum {
  width: 100px;
  text-align: left;
  padding-right: 32px;
}

.cfg-link { color: var(--v2-accent); cursor: pointer; margin-left: 12px; font-size: var(--v2-fs-sm); }
.cfg-link.danger { color: var(--v2-danger); }

.spin-once {
  animation: spin-once 0.6s cubic-bezier(0.4, 0, 0.2, 1);
}
@keyframes spin-once {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.webdav-table-wrapper {
  overflow: auto;
  scrollbar-gutter: stable;
}
.webdav-table-wrapper thead th {
  position: sticky;
  top: 0;
  z-index: 1;
  text-align: center;
}
.webdav-table-wrapper tbody td {
  text-align: center;
}
.webdav-file-cell {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  max-width: 220px;
}
.db-icon {
  color: var(--v2-text-3);
  flex-shrink: 0;
}
.filename-text {
  display: block;
  color: var(--v2-text);
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.webdav-row-actions {
  display: inline-flex;
  align-items: center;
  gap: 10px;
}
.webdav-link {
  color: var(--v2-accent);
  cursor: pointer;
  font-size: var(--v2-fs-sm);
}
.webdav-link.danger {
  color: var(--v2-danger);
}
.empty-state-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--v2-text-3);
}

.write-mode-help-content {
  width: 280px;
}
.write-mode-help-content .tooltip-title {
  font-size: var(--v2-fs-sm);
  font-weight: var(--v2-fw-semibold);
  color: var(--v2-text);
  margin-bottom: 10px;
}
.write-mode-help-content .tooltip-item {
  margin-bottom: 12px;
}
.write-mode-help-content .tooltip-item:last-child {
  margin-bottom: 0;
}
.write-mode-help-content .tooltip-item strong {
  display: block;
  font-size: var(--v2-fs-xs);
  font-weight: var(--v2-fw-semibold);
  color: var(--v2-text-2);
  margin-bottom: 4px;
}
.write-mode-help-content .tooltip-item span {
  display: block;
  font-size: var(--v2-fs-xs);
  line-height: 1.5;
  color: var(--v2-text-3);
}
.v2-input-wrapper {
  position: relative;
  display: flex;
  align-items: center;
}
.v2-input-wrapper .v2-input {
  padding-right: 36px;
}
.v2-input-icon-btn {
  position: absolute;
  right: 10px;
  background: transparent;
  border: none;
  color: var(--v2-text-3);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  transition: color 0.15s;
}
.v2-input-icon-btn:hover {
  color: var(--v2-text);
}

.preset-drawer-body {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
.preset-editor-field {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  margin-bottom: 16px;
}
.preset-file-editor {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.preset-editor-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.cfg-preset-editor {
  flex: 1;
  height: 100%;
}
.cfg-writemode-section {
  flex-shrink: 0;
  margin-top: auto;
}
</style>
