<template>
  <div class="logs-page">
    <svg style="display:none">
      <defs>
        <symbol id="icon-chevron" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="m6 9 6 6 6-6"/>
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
        <symbol id="icon-play" viewBox="0 0 24 24" fill="currentColor" stroke="none">
          <polygon points="6,3 20,12 6,21"/>
        </symbol>
        <symbol id="icon-pause" viewBox="0 0 24 24" fill="currentColor" stroke="none">
          <rect x="5" y="3" width="5" height="18" rx="1"/><rect x="14" y="3" width="5" height="18" rx="1"/>
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
          <div class="custom-select" :class="{ open: cliSelectOpen }" @click.stop="toggleSelect('cli')">
            <div class="custom-select-trigger">{{ getCliLabel(requestFilters.cli_type) }}</div>
            <svg class="chevron" width="16" height="16"><use href="#icon-chevron"/></svg>
            <div class="custom-select-options">
              <div class="custom-option" :class="{ selected: !requestFilters.cli_type }" @click.stop="requestFilters.cli_type = ''; cliSelectOpen = false; fetchRequestLogs()">全部终端<span v-if="!requestFilters.cli_type" class="check">✓</span></div>
              <div class="custom-option" :class="{ selected: requestFilters.cli_type === 'claude_code' }" @click.stop="requestFilters.cli_type = 'claude_code'; cliSelectOpen = false; fetchRequestLogs()">ClaudeCode<span v-if="requestFilters.cli_type === 'claude_code'" class="check">✓</span></div>
              <div class="custom-option" :class="{ selected: requestFilters.cli_type === 'codex' }" @click.stop="requestFilters.cli_type = 'codex'; cliSelectOpen = false; fetchRequestLogs()">Codex<span v-if="requestFilters.cli_type === 'codex'" class="check">✓</span></div>
              <div class="custom-option" :class="{ selected: requestFilters.cli_type === 'gemini' }" @click.stop="requestFilters.cli_type = 'gemini'; cliSelectOpen = false; fetchRequestLogs()">Gemini<span v-if="requestFilters.cli_type === 'gemini'" class="check">✓</span></div>
            </div>
          </div>
        </div>

        <div class="filter-group">
          <span class="filter-label">服务商</span>
          <div class="custom-select" style="width: 170px;" :class="{ open: providerSelectOpen }" @click.stop="toggleSelect('provider')">
            <div class="custom-select-trigger">{{ requestFilters.provider_name || '全部服务商' }}</div>
            <svg class="chevron" width="16" height="16"><use href="#icon-chevron"/></svg>
            <div class="custom-select-options" style="width: 220px;">
              <div class="custom-option" :class="{ selected: !requestFilters.provider_name }" @click.stop="requestFilters.provider_name = ''; providerSelectOpen = false; fetchRequestLogs()">全部服务商<span v-if="!requestFilters.provider_name" class="check">✓</span></div>
              <div v-for="p in providerOptions" :key="p" class="custom-option" :class="{ selected: requestFilters.provider_name === p }" @click.stop="requestFilters.provider_name = p; providerSelectOpen = false; fetchRequestLogs()">
                {{ p }}<span v-if="requestFilters.provider_name === p" class="check">✓</span>
              </div>
            </div>
          </div>
        </div>

        <div style="flex: 1;"></div>
        <div class="action-icon" :class="{ recording: logEnabled }" @click="logEnabled = !logEnabled; updateLogSettings()" :title="logEnabled ? '暂停记录' : '开启记录'">
          <svg width="16" height="16" v-if="logEnabled"><use href="#icon-pause"/></svg>
          <svg width="16" height="16" v-else><use href="#icon-play"/></svg>
        </div>
        <div style="width: 1px; height: 20px; background: #e2e8f0; margin: 0 4px;"></div>
        <div class="action-icon" @click="fetchRequestLogs" title="查询">
          <svg width="18" height="18"><use href="#icon-search"/></svg>
        </div>
        <div class="action-icon" @click="resetRequestFilters" title="重置">
          <svg width="18" height="18"><use href="#icon-refresh"/></svg>
        </div>
        <div class="action-icon delete" @click="clearRequestLogs" title="清空">
          <svg width="18" height="18"><use href="#icon-trash"/></svg>
        </div>
      </div>

      <!-- Super Clean Flat Table -->
      <div class="table-container" v-loading="requestLoading">
        <table class="flat-table">
          <thead>
            <tr>
              <th style="width: 60px;">ID</th>
              <th style="width: 160px;">时间</th>
              <th style="width: 100px;">终端</th>
              <th style="width: 130px;">服务商</th>
              <th style="width: 70px;">状态</th>
              <th style="width: 80px;">耗时</th>
              <th style="width: 120px;">Tokens</th>
              <th style="width: 60px; text-align: right;">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="row in requestLogs" :key="row.id">
              <td class="mono">{{ row.id }}</td>
              <td class="mono">{{ formatTime(row.created_at) }}</td>
              <td class="mono">{{ row.cli_type }}</td>
              <td class="mono">{{ row.provider_name }}</td>
              <td>
                <span v-if="row.status_code" :class="['pill', getStatusCodePill(row.status_code)]">{{ row.status_code }}</span>
                <span v-else>-</span>
              </td>
              <td class="mono" :class="{'text-danger': row.status_code && row.status_code >= 500}">
                {{ row.elapsed_ms }}ms
              </td>
              <td class="mono">
                <span v-if="row.input_tokens || row.output_tokens">{{ formatTokens(row.input_tokens) }} / {{ formatTokens(row.output_tokens) }}</span>
                <span v-else>-</span>
              </td>
              <td style="text-align: right;"><a class="table-link" @click="showRequestDetail(row.id)">详情</a></td>
            </tr>
            <tr v-if="requestLogs.length === 0">
              <td colspan="8" style="text-align: center; color: #94a3b8; padding: 40px; font-size: 13px;">暂无日志记录</td>
            </tr>
          </tbody>
        </table>
        
        <div class="pagination-footer">
          <span style="font-size: 13px; color: #64748b;">总计 {{ requestTotal }}</span>
          <!-- Still fallback to el-pagination for functionally complex pagers while removing background blocks -->
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
    </div>

    <!-- SYSTEM LOGS TAB -->
    <div v-if="activeTab === 'system'" class="tab-content">
      <!-- Filters & Actions -->
      <div class="filters-row">
        <div class="filter-group">
          <span class="filter-label">事件类型</span>
          <div class="custom-select" style="width: 170px;" :class="{ open: eventTypeSelectOpen }" @click.stop="toggleSelect('event')">
            <div class="custom-select-trigger">{{ formatEventType(systemFilters.event_type) || '全部事件' }}</div>
            <svg class="chevron" width="16" height="16"><use href="#icon-chevron"/></svg>
            <div class="custom-select-options" style="width: 200px;">
              <div class="custom-option" :class="{ selected: !systemFilters.event_type }" @click.stop="systemFilters.event_type = ''; eventTypeSelectOpen = false; fetchSystemLogs()">全部事件<span v-if="!systemFilters.event_type" class="check">✓</span></div>
              <div v-for="(label, key) in eventTypeMap" :key="key" class="custom-option" :class="{ selected: systemFilters.event_type === key }" @click.stop="systemFilters.event_type = key; eventTypeSelectOpen = false; fetchSystemLogs()">
                {{ label }}<span v-if="systemFilters.event_type === key" class="check">✓</span>
              </div>
            </div>
          </div>
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
      <div class="table-container" v-loading="systemLoading">
        <table class="flat-table">
          <thead>
            <tr>
              <th style="width: 70px;">ID</th>
              <th style="width: 170px;">时间</th>
              <th style="width: 160px;">事件类型</th>
              <th>消息脉络</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="row in systemLogs" :key="row.id">
              <td class="mono">{{ row.id }}</td>
              <td class="mono">{{ formatTime(row.created_at) }}</td>
              <td>{{ formatEventType(row.event_type) }}</td>
              <td>{{ row.message }}</td>
            </tr>
            <tr v-if="systemLogs.length === 0">
              <td colspan="4" style="text-align: center; color: #94a3b8; padding: 40px; font-size: 13px;">暂无日志记录</td>
            </tr>
          </tbody>
        </table>
        
        <div class="pagination-footer">
          <span style="font-size: 13px; color: #64748b;">总计 {{ systemTotal }}</span>
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
    </div>


    <!-- Request Detail Dialog -->
    <AppModal v-model="requestDetailVisible" title="请求详情" width="900px" :show-footer="false">
        <div v-if="requestDetail" class="detail-content">
            <!-- Summary -->
        <el-descriptions :column="3" border size="small">
          <el-descriptions-item label="ID">{{ requestDetail.id }}</el-descriptions-item>
          <el-descriptions-item label="时间">{{ formatTime(requestDetail.created_at) }}</el-descriptions-item>
          <el-descriptions-item label="耗时">{{ requestDetail.elapsed_ms }}ms</el-descriptions-item>
          <el-descriptions-item label="CLI类型">{{ requestDetail.cli_type }}</el-descriptions-item>
          <el-descriptions-item label="服务商">{{ requestDetail.provider_name }}</el-descriptions-item>
          <el-descriptions-item label="源模型">{{ requestDetail.source_model || '-' }}</el-descriptions-item>
          <el-descriptions-item label="映射模型">{{ requestDetail.target_model || '-' }}</el-descriptions-item>
          <el-descriptions-item label="Input Tokens">{{ formatTokens(requestDetail.input_tokens) }}</el-descriptions-item>
          <el-descriptions-item label="Output Tokens">{{ formatTokens(requestDetail.output_tokens) }}</el-descriptions-item>
        </el-descriptions>

        <!-- Error Message -->
        <el-alert v-if="requestDetail.error_message" :title="requestDetail.error_message" type="error" :closable="false" style="margin-top: 16px" />

        <!-- Request/Response Explorer -->
        <div class="cards-container">
          <el-card class="detail-card" shadow="hover">
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

          <el-card class="detail-card" shadow="hover">
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

          <el-card class="detail-card" style="grid-column: span 2;" shadow="hover">
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
import { ElMessageBox } from 'element-plus'
import { notify } from '@/utils/notification'
import AppModal from '@/components/AppModal.vue'
import { logsApi } from '@/api/logs'
import { providersApi } from '@/api/providers'
import { useUiStore } from '@/stores/ui'
import { formatJson as formatJsonUtil, formatTokens } from '@/utils/json'
import type { RequestLogListItem, RequestLogDetail, SystemLogItem } from '@/types/models'

const uiStore = useUiStore()
const activeTab = computed({
  get: () => uiStore.logsActiveTab,
  set: (val) => uiStore.setLogsActiveTab(val as 'request' | 'system')
})
const logEnabled = ref(false)
const providerOptions = ref<string[]>([])

// Dropdown State
const cliSelectOpen = ref(false)
const providerSelectOpen = ref(false)
const eventTypeSelectOpen = ref(false)

function closeAllSelects() {
  cliSelectOpen.value = false
  providerSelectOpen.value = false
  eventTypeSelectOpen.value = false
}

function toggleSelect(type: string) {
  const isCli = type === 'cli' && !cliSelectOpen.value
  const isProv = type === 'provider' && !providerSelectOpen.value
  const isEvent = type === 'event' && !eventTypeSelectOpen.value
  
  closeAllSelects()
  
  if (isCli) cliSelectOpen.value = true
  if (isProv) providerSelectOpen.value = true
  if (isEvent) eventTypeSelectOpen.value = true
}

onMounted(() => {
  document.addEventListener('click', closeAllSelects)
  fetchLogSettings()
  fetchProviders()
  fetchRequestLogs()
})

onUnmounted(() => {
  document.removeEventListener('click', closeAllSelects)
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
    logEnabled.value = res.data.debug_log
  } catch {}
}

async function updateLogSettings() {
  try {
    // using the toggled value
    await logsApi.updateSettings({ debug_log: logEnabled.value })
    notify('日志设置已更新')
  } catch {}
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

async function clearRequestLogs() {
  try {
    await ElMessageBox.confirm('确定要清空所有请求日志吗？', '清理确认')
    await logsApi.clearRequestLogs()
    notify('请求日志已清空')
    fetchRequestLogs()
  } catch {}
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
    await ElMessageBox.confirm('确定要清空所有系统日志吗？', '清理确认')
    await logsApi.clearSystemLogs()
    notify('系统日志已清空')
    fetchSystemLogs()
  } catch {}
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

const eventTypeMap: Record<string, string> = {
  no_provider_available: '无可用服务商',
  provider_blacklisted: '服务商黑名单',
  provider_recovered: '服务商恢复',
  provider_created: '服务商创建',
  provider_updated: '服务商更新',
  provider_deleted: '服务商删除',
  provider_reset: '状态重置',
}

function formatEventType(eventType: string): string {
  if (!eventType) return ''
  return eventTypeMap[eventType] || eventType
}

function getCliLabel(type: string): string {
  if (!type) return '全部终端'
  if (type === 'claude_code') return 'ClaudeCode'
  if (type === 'codex') return 'Codex'
  if (type === 'gemini') return 'Gemini'
  return type
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
function getStatusCodeType(code: number | null): string {
  if (!code) return 'info'
  if (code >= 200 && code < 300) return 'success'
  if (code >= 400 && code < 500) return 'warning'
  if (code >= 500) return 'danger'
  return 'info'
}

function getFullClientUrl(): string {
  if (!requestDetail.value) return ''
  const path = requestDetail.value.client_path
  return `http://localhost:7788/${path.startsWith('/') ? path.slice(1) : path}`
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
  color: #0f172a;
}

/* Tab Underlines */
.top-tabs { display: flex; gap: 32px; border-bottom: 1px solid rgba(226, 232, 240, 0.6); margin-bottom: 24px; padding-top: 8px; }
.tab-item { padding-bottom: 12px; color: #94a3b8; font-weight: 500; font-size: 15px; cursor: pointer; position: relative; transition: color 0.2s; }
.tab-item:hover { color: #475569; }
.tab-item.active { color: #0f172a; font-weight: 600; border-bottom: 2px solid #0f172a; }

/* Filter Container */
.filters-row { display: flex; gap: 8px; margin-bottom: 20px; align-items: center; }
.filter-group { display: flex; align-items: center; gap: 10px; margin-right: 8px; }
.filter-label { font-size: 12px; font-weight: 600; color: #94a3b8; text-transform: uppercase; }

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
.action-icon.delete:hover {
  background: #fee2e2;
  color: #ef4444;
}
.action-icon.recording {
  color: #10b981;
}
.action-icon.recording:hover {
  background: rgba(16, 185, 129, 0.1);
  color: #059669;
}

/* Pills */
.pill { padding: 4px 10px; border-radius: 999px; font-size: 11px; font-weight: 600; display: inline-flex; align-items: center; gap: 4px; letter-spacing: 0.3px; }
.pill-green { background: #ecfdf5; color: #10b981; }
.pill-red { background: #fff1f2; color: #f43f5e; }
.pill-grey { background: #f1f5f9; color: #64748b; font-weight: normal; }

/* Flat Glass Table - 1 Line Strict */
.table-container { background: #ffffff; border-radius: 12px; padding: 0; border: 1px solid #e2e8f0; box-shadow: 0 4px 15px rgba(0,0,0,0.02); overflow: hidden; }
.flat-table { width: 100%; border-collapse: collapse; text-align: left; }
.flat-table th, .flat-table td { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

.flat-table th { padding: 12px 20px; font-size: 12px; font-weight: 600; color: #64748b; text-transform: uppercase; border-bottom: 1px solid #e2e8f0; background: #f8fafc; }
.flat-table thead tr th:first-child { border-top-left-radius: 12px; }
.flat-table thead tr th:last-child { border-top-right-radius: 12px; }
.flat-table td { padding: 12px 20px; font-size: 13px; color: #0f172a; border-bottom: 1px solid #f1f5f9; }
.flat-table tr:last-child td { border-bottom: none; }
.flat-table tr:hover td { background: #f8fafc; }

.mono { font-family: "JetBrains Mono", monospace; color: #64748b; font-size: 12px; }
.text-danger { color: #f43f5e; font-weight: 600; }
.table-link { color: #0ea5e9; cursor: pointer; text-decoration: none; font-weight: 500; }
.table-link:hover { text-decoration: underline; }

.pagination-footer { padding: 12px 20px; display: flex; justify-content: space-between; align-items: center; border-top: 1px dashed rgba(226, 232, 240, 0.8); }
.pagination-footer :deep(.el-pagination) { justify-content: flex-end; }
.pagination-footer :deep(.el-pager li) { background: transparent !important; }
.pagination-footer :deep(.el-pager li.is-active) { color: #0ea5e9; background: #f0f9ff !important; font-weight: 700; border-radius: 6px; }
.pagination-footer :deep(.btn-prev), .pagination-footer :deep(.btn-next) { background: transparent !important; }

.pagination-footer :deep(.el-select__wrapper) { padding: 4px 12px; border: 1px solid #e2e8f0; border-radius: 8px; background: rgba(255,255,255,0.8); box-shadow: 0 1px 3px rgba(0,0,0,0.02); min-height: auto; transition: all 0.2s; }
.pagination-footer :deep(.el-select__wrapper:hover) { border-color: #cbd5e1; }
.pagination-footer :deep(.el-select__wrapper.is-focused) { border-color: #0ea5e9; box-shadow: 0 0 0 3px rgba(14, 165, 233, 0.1); }

/* Custom HTML Select (Headless UI Clone) */
.custom-select { position: relative; width: 150px; }
.custom-select-trigger { padding: 9px 36px 9px 16px; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 13px; font-weight: 500; color: #0f172a; background: rgba(255,255,255,0.8); box-shadow: 0 1px 3px rgba(0,0,0,0.02); cursor: pointer; transition: all 0.2s; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; user-select: none; }
.custom-select:hover .custom-select-trigger { border-color: #cbd5e1; background: #ffffff; }
.custom-select.open .custom-select-trigger { border-color: #0ea5e9; box-shadow: 0 0 0 3px rgba(14, 165, 233, 0.1); background: #ffffff; }
.custom-select .chevron { position: absolute; right: 12px; top: 50%; transform: translateY(-50%); color: #64748b; pointer-events: none; transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1); }
.custom-select.open .chevron { transform: translateY(-50%) rotate(180deg); color: #0ea5e9; }

.custom-select-options { position: absolute; top: calc(100% + 6px); left: 0; right: auto; background: #ffffff; border: 1px solid #e2e8f0; border-radius: 12px; box-shadow: 0 10px 40px -10px rgba(0,0,0,0.1); padding: 4px; z-index: 50; opacity: 0; transform: translateY(-5px); pointer-events: none; transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1); min-width: 100%; max-height: 250px; overflow-y: auto; }
.custom-select.open .custom-select-options { opacity: 1; transform: translateY(0); pointer-events: auto; }
.custom-option { padding: 10px 12px; border-radius: 8px; font-size: 13px; color: #475569; cursor: pointer; transition: all 0.1s; display: flex; align-items: center; justify-content: space-between; margin-bottom: 2px; }
.custom-option:hover { background: #f1f5f9; color: #0f172a; }
.custom-option.selected { font-weight: 600; color: #0ea5e9; background: #f0f9ff; }
.check { color:#0ea5e9; font-weight: bold; font-size:14px; margin-left:8px; }

/* Keep el-dialog styles clean to match ethereal frost inside detail view */
.detail-content { max-height: 60vh; overflow-y: auto; }
.cards-container { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin-top: 16px; }
.detail-card { margin: 0; }
.detail-card-header { display: flex; justify-content: space-between; font-weight: 600; }
.url-line { font-family: 'JetBrains Mono', monospace; font-size: 12px; color: #0ea5e9; word-break: break-all; margin-bottom: 12px; padding: 8px 12px; background: #f0f9ff; border-radius: 6px; }
.code-block { background: #f8fafc; padding: 12px; border-radius: 6px; font-family: 'JetBrains Mono', monospace; font-size: 12px; white-space: pre-wrap; word-break: break-all; max-height: 200px; overflow-y: auto; margin: 0; cursor: pointer; border: 1px solid transparent; transition: border-color 0.2s; }
.code-block:hover { border-color: #cbd5e1; }
</style>
