<template>
  <div class="logs-page">
    <svg style="display:none">
      <defs>
        <symbol id="icon-file-text" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/>
          <polyline points="14 2 14 8 20 8"/>
          <line x1="16" x2="8" y1="13" y2="13"/>
          <line x1="16" x2="8" y1="17" y2="17"/>
          <line x1="10" x2="8" y1="9" y2="9"/>
        </symbol>
        <symbol id="icon-search" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>
        </symbol>
        <symbol id="icon-refresh" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/><path d="M16 16h5v5"/>
        </symbol>
        <symbol id="icon-trash" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/><line x1="10" x2="10" y1="11" y2="17"/><line x1="14" x2="14" y1="11" y2="17"/>
        </symbol>
      </defs>
    </svg>



    <!-- Top Level Tabs -->
    <div class="top-tabs">
      <div :class="['tab-item', { active: activeTab === 'request' }]" @click="activeTab = 'request'">请求日志</div>
      <div :class="['tab-item', { active: activeTab === 'system' }]" @click="activeTab = 'system'">系统日志</div>
    </div>

    <!-- REQUEST LOGS TAB -->
    <div v-if="activeTab === 'request'" class="tab-content">
      <!-- Filters & Actions -->
      <div class="filters-row">
        <div class="filter-group">
          <span class="filter-label">终端</span>
          <AppSelect
            :model-value="requestFilters.cli_type"
            :options="cliFilterOptions"
            @change="handleRequestCliChange"
          />
        </div>

        <div class="filter-group">
          <span class="filter-label">服务商</span>
          <AppSelect
            :model-value="requestFilters.provider_name"
            :options="providerFilterOptions"
            @change="handleRequestProviderChange"
          />
        </div>

        <div style="flex: 1;"></div>
        <!-- 日志模式下拉选择器 -->
        <span class="filter-label">日志级别</span>
        <AppSelect
          :model-value="logRecordMode"
          :options="logModeOptions"
          width="140px"
          @change="value => setLogMode(value as LogRecordMode)"
        />
        <div style="width: 1px; height: 20px; background: var(--color-border); margin: 0 4px;"></div>
        <div class="action-icon" @click="fetchRequestLogs" title="查询">
          <svg width="18" height="18"><use href="#icon-search"/></svg>
        </div>
        <div class="action-icon" @click="resetRequestFilters" title="重置">
          <svg width="18" height="18"><use href="#icon-refresh"/></svg>
        </div>
        <AppSelect mode="menu" :options="cleanMenuItems" @select="handleClean">
          <template #trigger>
            <div class="action-icon delete" title="清理">
              <svg width="18" height="18"><use href="#icon-trash"/></svg>
            </div>
          </template>
        </AppSelect>
      </div>

      <!-- Super Clean Flat Table -->
      <div v-loading="requestLoading" class="list-container">
        <div v-if="requestLogs.length === 0" class="empty-state">
          <svg width="64" height="64" color="var(--color-border)"><use href="#icon-file-text"/></svg>
          <p>暂无日志记录</p>
        </div>
        <template v-else>
          <div class="table-container">
            <div class="table-wrapper">
              <table class="flat-table">
              <thead>
                <tr>
                  <th style="min-width: 60px;">ID</th>
                  <th style="min-width: 100px;">时间</th>
                  <th style="min-width: 100px;">CLI</th>
                  <th style="min-width: 100px;">服务商</th>
                  <th style="min-width: 60px;">状态</th>
                  <th style="min-width: 100px;">耗时</th>
                  <th style="min-width: 150px;">
                    <div class="token-head">
                      <span>Tokens</span>
                      <el-tooltip
                        effect="light"
                        placement="top"
                        :fallback-placements="['bottom', 'top', 'right', 'left']"
                        :offset="10"
                        :show-after="150"
                        :enterable="true"
                        popper-class="token-help-popper"
                      >
                        <template #content>
                          <div class="token-help-content">
                            <div class="tooltip-title">Token 显示顺序</div>
                            <div class="tooltip-item"><strong>输入</strong><span>普通输入 tokens</span></div>
                            <div class="tooltip-item"><strong>缓存读取</strong><span>命中的缓存输入 tokens</span></div>
                            <div class="tooltip-item"><strong>缓存创建</strong><span>本次创建的缓存输入 tokens</span></div>
                            <div class="tooltip-item"><strong>输出</strong><span>输出 tokens，包含 reasoning/thoughts</span></div>
                          </div>
                        </template>
                        <span class="help-icon-wrapper">
                          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="help-icon">
                            <circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><line x1="12" y1="17" x2="12.01" y2="17"/>
                          </svg>
                        </span>
                      </el-tooltip>
                    </div>
                  </th>
                  <th style="min-width: 100px;">模型映射</th>
                  <th class="col-sticky" style="width: 60px;">操作</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in requestLogs" :key="row.id">
                  <td class="mono">{{ row.id }}</td>
                  <td>{{ formatTime(row.created_at) }}</td>
                  <td>{{ row.cli_type }}</td>
                  <td>{{ row.provider_name }}</td>
                  <td>
                    <span v-if="row.status_code" :class="['pill', getStatusCodePill(row.status_code)]">{{ row.status_code }}</span>
                    <span v-else>-</span>
                  </td>
                  <td class="mono" :class="{'text-danger': row.status_code && row.status_code >= 500}">
                    {{ (row.elapsed_ms / 1000).toFixed(2) }}s
                  </td>
                  <td class="mono">
                    {{ formatTokenUsage(row) }}
                  </td>
                  <td class="mono text-left">{{ row.source_model || '-' }} → {{ row.target_model || '-' }}</td>
                  <td class="col-sticky">
                    <a class="table-link" @click="showRequestDetail(row.id)">详情</a>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <div class="pagination-footer">
            <span class="text-14 text-secondary">总计 {{ requestTotal }}</span>
            <el-pagination
              v-model:current-page="requestPage"
              v-model:page-size="requestPageSize"
              :page-sizes="[20, 50, 100]"
              :total="requestTotal"
              layout="sizes, prev, pager, next"
              @size-change="fetchRequestLogs"
              @current-change="fetchRequestLogs"
            />
          </div>
          </div>
        </template>
      </div>
    </div>

    <!-- SYSTEM LOGS TAB -->
    <div v-if="activeTab === 'system'" class="tab-content">
      <!-- Filters & Actions -->
      <div class="filters-row">
        <div class="filter-group">
          <span class="filter-label">事件类型</span>
          <AppSelect
            :model-value="systemFilters.event_type"
            :options="eventTypeOptions"
            @change="handleSystemEventTypeChange"
          />
        </div>

        <div style="flex: 1;"></div>
        <div class="action-icon" @click="fetchSystemLogs" title="查询">
          <svg width="18" height="18"><use href="#icon-search"/></svg>
        </div>
        <div class="action-icon" @click="resetSystemFilters" title="重置">
          <svg width="18" height="18"><use href="#icon-refresh"/></svg>
        </div>
        <div class="action-icon delete" @click="clearSystemLogs" title="清空">
          <svg width="18" height="18"><use href="#icon-trash"/></svg>
        </div>
      </div>

      <!-- Super Clean Flat Table -->
      <div v-loading="systemLoading" class="list-container">
        <div v-if="systemLogs.length === 0" class="empty-state">
          <svg width="64" height="64" color="var(--color-border)"><use href="#icon-file-text"/></svg>
          <p>暂无日志记录</p>
        </div>
        <template v-else>
          <div class="table-container">
            <div class="table-wrapper">
              <table class="flat-table">
              <thead>
                <tr>
                  <th style="min-width: 60px;">ID</th>
                  <th style="min-width: 100px;">时间</th>
                  <th style="min-width: 200px;">事件类型</th>
                  <th>事件消息</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in systemLogs" :key="row.id">
                  <td>{{ row.id }}</td>
                  <td>{{ formatTime(row.created_at) }}</td>
                  <td>{{ formatEventType(row.event_type) }}</td>
                  <td>{{ row.message }}</td>
                </tr>
              </tbody>
            </table>
          </div>

          <div class="pagination-footer">
            <span class="text-14 text-secondary">总计 {{ systemTotal }}</span>
            <el-pagination
              v-model:current-page="systemPage"
              v-model:page-size="systemPageSize"
              :page-sizes="[20, 50, 100]"
              :total="systemTotal"
              layout="sizes, prev, pager, next"
              @size-change="fetchSystemLogs"
              @current-change="fetchSystemLogs"
            />
          </div>
          </div>
        </template>
      </div>
    </div>


    <!-- Request Detail Dialog -->
    <AppModal v-model="requestDetailVisible" title="请求详情" width="900px" :show-footer="false">
        <div v-if="requestDetail" class="detail-content">
        <!-- Error Message -->
        <el-alert v-if="requestDetail.error_message" :title="requestDetail.error_message" type="error" :closable="false" style="margin-bottom: 16px" />

        <!-- Request/Response Explorer -->
        <div class="cards-container">
          <el-card class="detail-card" shadow="never">
            <template #header>
              <div class="detail-card-header">
                <span class="card-title">CLI 终端握手</span>
                <el-tag size="small" type="info">{{ requestDetail.client_method }}</el-tag>
              </div>
            </template>
            <div class="url-line">{{ getFullClientUrl() }}</div>
            <el-collapse>
              <el-collapse-item title="Request Headers">
                <pre class="code-block" @click="handleCopy(requestDetail.client_headers)">{{ formatJson(requestDetail.client_headers) }}</pre>
              </el-collapse-item>
              <el-collapse-item title="Request Body Payload">
                <pre class="code-block" @click="handleCopy(requestDetail.client_body)">{{ formatJson(requestDetail.client_body) }}</pre>
              </el-collapse-item>
            </el-collapse>
          </el-card>

          <el-card class="detail-card" shadow="never">
            <template #header>
              <div class="detail-card-header">
                <span class="card-title">网关路由分发</span>
                <el-tag size="small" type="info">{{ requestDetail.client_method }}</el-tag>
              </div>
            </template>
            <div class="url-line">{{ requestDetail.forward_url }}</div>
            <el-collapse>
              <el-collapse-item title="Forward Headers">
                <pre class="code-block" @click="handleCopy(requestDetail.forward_headers)">{{ formatJson(requestDetail.forward_headers) }}</pre>
              </el-collapse-item>
              <el-collapse-item title="Forward Body Payload">
                <pre class="code-block" @click="handleCopy(requestDetail.forward_body)">{{ formatJson(requestDetail.forward_body) }}</pre>
              </el-collapse-item>
            </el-collapse>
          </el-card>

          <el-card class="detail-card" style="grid-column: span 2;" shadow="never">
            <template #header>
              <div class="detail-card-header">
                <span class="card-title">服务商节点响应回传</span>
                <el-tag size="small" :type="getStatusCodeType(requestDetail.status_code)">
                  {{ requestDetail.status_code || '-' }}
                </el-tag>
              </div>
            </template>
            <el-collapse>
              <el-collapse-item title="Response Headers">
                <pre class="code-block" @click="handleCopy(requestDetail.provider_headers)">{{ formatJson(requestDetail.provider_headers) }}</pre>
              </el-collapse-item>
              <el-collapse-item title="Response Body Payload">
                <pre class="code-block" @click="handleCopy(requestDetail.provider_body)">{{ formatJson(requestDetail.provider_body) }}</pre>
              </el-collapse-item>
            </el-collapse>
          </el-card>
        </div>
      </div>
    </AppModal>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed } from 'vue'
import { confirm } from '@/utils/confirm'
import { notify } from '@/utils/notification'
import AppModal from '@/components/AppModal.vue'
import AppSelect, { type AppSelectOption } from '@/components/AppSelect.vue'
import { logsApi } from '@/api/logs'
import { providersApi } from '@/api/providers'
import { settingsApi } from '@/api/settings'
import { useUiStore } from '@/stores/ui'
import { getErrorMessage } from '@/utils/error'
import { formatJson as formatJsonUtil, formatTokens } from '@/utils/json'
import type { RequestLogListItem, RequestLogDetail, SystemLogItem } from '@/types/models'

type LogRecordMode = 'full' | 'failure_only' | 'disabled'

const logModeMap: Record<LogRecordMode, string> = {
  full: '全量记录',
  failure_only: '失败时记录详情',
  disabled: '停用日志'
}

const uiStore = useUiStore()
const activeTab = computed({
  get: () => uiStore.logsActiveTab,
  set: (val) => uiStore.setLogsActiveTab(val as 'request' | 'system')
})
const logRecordMode = ref<LogRecordMode>('failure_only')
const cleanMenuItems: AppSelectOption[] = [
  { label: '清理全部日志', value: 'all_logs' },
  { label: '清理全部详情', value: 'all_details' },
  { label: '清理30天前日志', value: 'old_logs' },
  { label: '清理30天前详情', value: 'old_details' }
]
const gatewayUrl = ref('')
const providerOptions = ref<string[]>([])
let requestLogListener: (() => void) | null = null

const cliFilterOptions: AppSelectOption[] = [
  { label: '全部终端', value: '' },
  { label: 'ClaudeCode', value: 'claude_code' },
  { label: 'Codex', value: 'codex' },
  { label: 'Gemini', value: 'gemini' }
]

const providerFilterOptions = computed<AppSelectOption[]>(() => [
  { label: '全部服务商', value: '' },
  ...providerOptions.value.map(provider => ({ label: provider, value: provider }))
])

const logModeOptions = computed<AppSelectOption[]>(() =>
  Object.entries(logModeMap).map(([value, label]) => ({ value, label }))
)

onMounted(async () => {
  fetchLogSettings()
  fetchGatewayStatus()
  fetchProviders()
  if (activeTab.value === 'request') {
    fetchRequestLogs()
  } else {
    fetchSystemLogs()
  }

  // 监听新请求日志事件
  requestLogListener = await logsApi.listenRequestLogs((log) => {
    // 只在请求日志页面且第一页无筛选时实时更新
    if (activeTab.value === 'request' && requestPage.value === 1 && !requestFilters.value.cli_type && !requestFilters.value.provider_name) {
      requestLogs.value.unshift(log)
      requestTotal.value += 1
      // 限制列表长度与 pageSize 一致
      if (requestLogs.value.length > requestPageSize.value) {
        requestLogs.value.pop()
      }
    }
  })
})

onUnmounted(() => {
  if (requestLogListener) {
    requestLogListener()
    requestLogListener = null
  }
})

// Request logs
const requestLogs = ref<RequestLogListItem[]>([])
const requestLoading = ref(false)
const requestPage = ref(1)
const requestPageSize = ref(20)
const requestTotal = ref(0)
const requestFilters = ref({
  cli_type: '',
  provider_name: ''
})
const requestDetailVisible = ref(false)
const requestDetail = ref<RequestLogDetail | null>(null)

// System logs
const systemLogs = ref<SystemLogItem[]>([])
const systemLoading = ref(false)
const systemPage = ref(1)
const systemPageSize = ref(20)
const systemTotal = ref(0)
const systemFilters = ref({
  event_type: ''
})

async function fetchProviders() {
  try {
    const res = await providersApi.list()
    const names = new Set<string>()
    res.data.forEach((p: any) => names.add(p.name))
    providerOptions.value = Array.from(names)
  } catch {}
}

async function fetchLogSettings() {
  try {
    const res = await logsApi.getSettings()
    const { debug_log, log_detail_mode } = res.data
    if (!debug_log) {
      logRecordMode.value = 'disabled'
    } else {
      logRecordMode.value = log_detail_mode
    }
  } catch {}
}

async function fetchGatewayStatus() {
  try {
    const { data } = await settingsApi.getStatus()
    gatewayUrl.value = data.gateway_url
  } catch {
    gatewayUrl.value = ''
  }
}

async function setLogMode(mode: LogRecordMode) {
  logRecordMode.value = mode
  try {
    if (mode === 'disabled') {
      await logsApi.updateSettings({ debug_log: false })
    } else {
      await logsApi.updateSettings({ debug_log: true, log_detail_mode: mode })
    }
  } catch {}
}

function handleRequestCliChange(value: string | number) {
  requestFilters.value.cli_type = String(value)
  requestPage.value = 1
  fetchRequestLogs()
}

function handleRequestProviderChange(value: string | number) {
  requestFilters.value.provider_name = String(value)
  requestPage.value = 1
  fetchRequestLogs()
}

function handleSystemEventTypeChange(value: string | number) {
  systemFilters.value.event_type = String(value)
  systemPage.value = 1
  fetchSystemLogs()
}

async function fetchRequestLogs() {
  requestLoading.value = true
  try {
    const params: any = {
      page: requestPage.value,
      page_size: requestPageSize.value
    }
    if (requestFilters.value.cli_type) params.cli_type = requestFilters.value.cli_type
    if (requestFilters.value.provider_name) params.provider_name = requestFilters.value.provider_name

    const res = await logsApi.listRequestLogs(params)
    requestLogs.value = res.data.items
    requestTotal.value = res.data.total
  } finally {
    requestLoading.value = false
  }
}

function resetRequestFilters() {
  requestFilters.value = { cli_type: '', provider_name: '' }
  requestPage.value = 1
  fetchRequestLogs()
}

type CleanAction = 'all_logs' | 'all_details' | 'old_logs' | 'old_details'

async function handleClean(action: string | number) {
  const confirmMap: Record<CleanAction, string> = {
    all_logs: '确定要清空所有请求日志吗？',
    all_details: '确定要清空所有请求详情文件吗？',
    old_logs: '确定要清理30天前的请求日志吗？',
    old_details: '确定要清理30天前的请求详情文件吗？'
  }

  try {
    await confirm(confirmMap[action as CleanAction], '清理确认')
  } catch {
    return
  }

  requestLoading.value = true
  try {
    switch (action as CleanAction) {
      case 'all_logs':
        await logsApi.clearRequestLogs()
        notify('请求日志已清空')
        break
      case 'all_details':
        await logsApi.clearRequestDetailFiles()
        notify('请求详情文件已清空')
        break
      case 'old_logs':
        await logsApi.clearOldRequestLogs(30)
        notify('30天前的请求日志已清理')
        break
      case 'old_details':
        await logsApi.clearOldRequestDetailFiles(30)
        notify('30天前的请求详情文件已清理')
        break
    }
    await fetchRequestLogs()
  } catch (e: any) {
    notify(getErrorMessage(e, '清理失败'), 'error')
    requestLoading.value = false
  }
}

async function showRequestDetail(id: number) {
  try {
    const res = await logsApi.getRequestLog(id)
    requestDetail.value = res.data
    requestDetailVisible.value = true
  } catch {}
}

async function fetchSystemLogs() {
  systemLoading.value = true
  try {
    const params: any = {
      page: systemPage.value,
      page_size: systemPageSize.value
    }
    if (systemFilters.value.event_type) params.event_type = systemFilters.value.event_type

    const res = await logsApi.listSystemLogs(params)
    systemLogs.value = res.data.items
    systemTotal.value = res.data.total
  } finally {
    systemLoading.value = false
  }
}

function resetSystemFilters() {
  systemFilters.value = { event_type: '' }
  systemPage.value = 1
  fetchSystemLogs()
}

async function clearSystemLogs() {
  try {
    await confirm('确定要清空所有系统日志吗？', '清理确认')
  } catch {
    return
  }

  systemLoading.value = true
  try {
    await logsApi.clearSystemLogs()
    notify('系统日志已清空')
    await fetchSystemLogs()
  } catch (e: any) {
    notify(e?.message || '清空失败', 'error')
    systemLoading.value = false
  }
}

function formatTime(timestamp: number): string {
  // Use a cleaner time format matching the prototype `MM/DD HH:mm:ss`
  const date = new Date(timestamp * 1000)
  const m = String(date.getMonth() + 1).padStart(2, '0')
  const d = String(date.getDate()).padStart(2, '0')
  const h = String(date.getHours()).padStart(2, '0')
  const min = String(date.getMinutes()).padStart(2, '0')
  const s = String(date.getSeconds()).padStart(2, '0')
  return `${m}/${d} ${h}:${min}:${s}`
}

function formatJson(str: string | null): string {
  if (!str) return ''
  return formatJsonUtil(str)
}

function formatTokenUsage(row: RequestLogListItem | RequestLogDetail): string {
  return [
    row.input_tokens,
    row.cache_read_input_tokens,
    row.cache_creation_input_tokens,
    row.output_tokens
  ].map(formatTokens).join(' / ')
}

const eventTypeMap: Record<string, string> = {
  no_provider_available: '无可用服务商',
  provider_blacklisted: '服务商黑名单',
  provider_recovered: '服务商恢复',
  provider_created: '服务商创建',
  provider_updated: '服务商更新',
  provider_deleted: '服务商删除',
  provider_reset: '状态重置',
  scheduled_task_failed: '定时任务失败',
}

const eventTypeOptions = computed<AppSelectOption[]>(() => [
  { label: '全部事件', value: '' },
  ...Object.entries(eventTypeMap).map(([value, label]) => ({ value, label }))
])

function formatEventType(eventType: string): string {
  if (!eventType) return ''
  return eventTypeMap[eventType] || eventType
}

// Flat table styling purely depends on specific css pills
function getStatusCodePill(code: number | null): string {
  if (!code) return 'pill-grey'
  if (code >= 200 && code < 300) return 'pill-green'
  if (code >= 400 && code < 500) return 'pill-grey'
  if (code >= 500) return 'pill-red'
  return 'pill-grey'
}

// Keeping original Element styling function backward compat for the Dialog View
function getStatusCodeType(code: number | null): 'success' | 'warning' | 'info' | 'danger' {
  if (!code) return 'info'
  if (code >= 200 && code < 300) return 'success'
  if (code >= 400 && code < 500) return 'warning'
  if (code >= 500) return 'danger'
  return 'info'
}

function getFullClientUrl(): string {
  if (!requestDetail.value) return ''
  const path = requestDetail.value.client_path
  const baseUrl = gatewayUrl.value.replace(/\/$/, '')
  return `${baseUrl}/${path.startsWith('/') ? path.slice(1) : path}`
}

async function handleCopy(content: string | null) {
  if (!content) return
  try {
    await navigator.clipboard.writeText(formatJson(content))
    notify('已复制到剪贴板')
  } catch {
    notify('复制失败', 'error')
  }
}

watch(activeTab, (tab) => {
  if (tab === 'request') fetchRequestLogs()
  else fetchSystemLogs()
})
</script>

<style scoped>
/* Scoped overrides for flat ethereal UI */
.logs-page {
  color: var(--color-text);
  height: 100%;
  display: flex;
  flex-direction: column;
}

/* Tab Underlines */
.top-tabs { display: flex; gap: 32px; border-bottom: 1px solid color-mix(in srgb, var(--color-border) 60%, transparent); margin: 0 40px 24px 40px; padding-top: 8px; flex-shrink: 0; }

.tab-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

/* Filter Container */
.filters-row { display: flex; gap: 8px; margin: 0 40px 20px 40px; align-items: center; flex-shrink: 0; }
.filter-group { display: flex; align-items: center; gap: 10px; margin-right: 8px; }
.filter-label { font-size: var(--fs-12); font-weight: var(--fw-600); color: var(--color-text-weak); text-transform: uppercase; }

/* Action Icon Buttons */

/* Flat Glass Table - 1 Line Strict */
.table-container {
  background: var(--color-bg);
  border-radius: 16px;
  padding: 0;
  border: 1px solid var(--color-border);
  box-shadow: 0 4px 15px var(--color-shadow);
  overflow: hidden;
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.table-wrapper {
  flex: 1;
  overflow: auto;
}
.flat-table tr:hover td.col-sticky { background: var(--color-bg-page); }

.token-head {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
}

:global(.token-help-popper.el-popper) {
  border-radius: 12px;
  padding: 16px;
  box-shadow: 0 8px 24px var(--color-shadow-lg);
}

.token-help-content {
  width: 220px;
}

.col-sticky {
  position: sticky;
  right: 0;
  background: var(--color-bg);
  width: 100px;
  box-shadow: -4px 0 12px rgba(0, 0, 0, 0.03);
}
.flat-table th.col-sticky {
  background: var(--color-bg-page);
  z-index: 20;
}

.flat-table td.text-left { text-align: left; }
.text-danger { color: var(--color-error); font-weight: var(--fw-600); }

.pagination-footer { padding: 12px 20px; display: flex; justify-content: space-between; align-items: center; border-top: 1px dashed color-mix(in srgb, var(--color-border) 80%, transparent); flex-shrink: 0; }
.pagination-footer :deep(.el-pagination) { justify-content: flex-end; }
.pagination-footer :deep(.el-pager li) { background: transparent !important; }
.pagination-footer :deep(.el-pager li.is-active) { color: var(--color-primary); background: var(--color-primary-light) !important; font-weight: var(--fw-700); border-radius: 6px; }
.pagination-footer :deep(.btn-prev), .pagination-footer :deep(.btn-next) { background: transparent !important; }

.pagination-footer :deep(.el-select__wrapper) { padding: 4px 12px; border: 1px solid var(--color-border); border-radius: 8px; background: color-mix(in srgb, var(--color-bg) 80%, transparent); box-shadow: 0 1px 3px var(--color-shadow); min-height: auto; transition: all 0.2s; }
.pagination-footer :deep(.el-select__wrapper:hover) { border-color: var(--color-border-hover); }
.pagination-footer :deep(.el-select__wrapper.is-focused) { border-color: var(--color-primary); box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-primary) 10%, transparent); }

/* Keep el-dialog styles clean to match ethereal frost inside detail view */
.detail-content { max-height: 60vh; overflow-y: auto; }
.cards-container { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin-top: 16px; }
.detail-card { margin: 0; background: var(--color-bg); }
.detail-card-header { display: flex; justify-content: space-between; font-weight: var(--fw-600); }
.url-line { font-size: var(--fs-12); color: var(--color-primary); word-break: break-all; margin-bottom: 12px; padding: 8px 12px; background: var(--color-primary-light); border-radius: 6px; }
.code-block { background: var(--color-bg-page); padding: 12px; border-radius: 6px; font-size: var(--fs-12); white-space: pre-wrap; word-break: break-all; max-height: 200px; overflow-y: auto; margin: 0; cursor: pointer; border: 1px solid transparent; transition: border-color 0.2s; }
.code-block:hover { border-color: var(--color-border-hover); }

/* el-collapse 标题背景色覆盖 */
.detail-content :deep(.el-collapse-item__header) {
  background: var(--color-bg);
}
.detail-content :deep(.el-collapse-item__wrap) {
  background: var(--color-bg);
}
</style>
