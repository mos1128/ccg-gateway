<template>
  <div class="logs-page">
    <div class="logs-bar">
      <div class="v2-tabs logs-tabs">
        <div class="v2-tab" :class="{ active: activeTab === 'request' }" @click="activeTab = 'request'">请求日志</div>
        <div class="v2-tab" :class="{ active: activeTab === 'system' }" @click="activeTab = 'system'">系统日志</div>
      </div>
    </div>

    <!-- 请求日志 -->
    <template v-if="activeTab === 'request'">
      <div v-loading="requestLoading" class="v2-card logs-tablecard">
        <div class="logs-filters">
          <AppSelect :model-value="requestFilters.cli_type" :options="cliFilterOptions" width="120px" @change="handleRequestCliChange" />
          <AppSelect :model-value="requestFilters.provider_name" :options="providerFilterOptions" width="160px" @change="handleRequestProviderChange" />
          <div style="flex:1"></div>
          <span class="logs-flabel">日志级别</span>
          <AppSelect :model-value="logRecordMode" :options="logModeOptions" width="140px" @change="v => setLogMode(v as LogRecordMode)" />
          <el-tooltip content="查询" placement="top" effect="light" :show-after="250">
            <button class="v2-row-act" @click="fetchRequestLogs"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg></button>
          </el-tooltip>
          <el-tooltip content="重置" placement="top" effect="light" :show-after="250">
            <button class="v2-row-act" @click="resetRequestFilters"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/><path d="M16 16h5v5"/></svg></button>
          </el-tooltip>
          <el-tooltip content="清理" placement="top" effect="light" :show-after="250">
            <el-dropdown trigger="click" placement="bottom-end" @command="handleClean">
              <button class="v2-row-act danger"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg></button>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item v-for="item in cleanMenuItems" :key="String(item.value)" :command="item.value">{{ item.label }}</el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </el-tooltip>
        </div>

        <div class="logs-scroll">
          <table class="v2-table">
            <thead>
              <tr>
                <th>ID</th><th>时间</th><th>Agent</th><th>Profile</th><th>端点类型</th><th>服务商</th><th>状态</th><th>耗时 (首/总)</th>
                <th>
                  <el-tooltip content="输入 / 输出" placement="top" effect="light" :show-after="250">
                    <span>Token (I/O)</span>
                  </el-tooltip>
                </th>
                <th>
                  <el-tooltip content="缓存读取 / 缓存创建" placement="top" effect="light" :show-after="250">
                    <span>Cache (R/C)</span>
                  </el-tooltip>
                </th>
                <th>费用</th><th>模型映射</th><th class="logs-sticky-col">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in requestLogs" :key="row.id">
                <td class="mono">{{ row.id }}</td>
                <td class="mono">{{ formatTime(row.created_at) }}</td>
                <td>
                  <el-tooltip :content="formatCliLabel(row.cli_type)" placement="top" effect="light" :show-after="250">
                    <div class="logs-cli-cell">
                      <span class="logs-cli-icon">
                        <CliBrandIcon :type="row.cli_type" width="14" height="14" />
                      </span>
                    </div>
                  </el-tooltip>
                </td>
                <td class="mono">{{ row.profile || 'default' }}</td>
                <td class="mono">{{ formatProtocolLabel(row.protocol) }}</td>
                <td>
                  <span>{{ row.provider_name || '-' }}</span>
                  <span v-if="row.provider_id != null" class="logs-provider-id mono">#{{ row.provider_id }}</span>
                </td>
                <td>
                  <el-tooltip v-if="!row.finished_at" content="请求进行中" placement="top" effect="light" :show-after="250">
                    <span class="v2-pill dot v2-pill-info logs-running">Run</span>
                  </el-tooltip>
                  <span v-else-if="row.status_code" class="v2-pill dot" :class="statusPill(row.status_code)">{{ row.status_code }}</span>
                  <span v-else>-</span>
                </td>
                <td class="mono" :class="elapsedTimeClass(row)">{{ formatLatencyPair(row) }}</td>
                <td class="mono">
                  <span class="tok-group">
                    <span class="tok-val">{{ formatTokens(row.input_tokens) }}</span>
                    <span class="tok-sep">/</span>
                    <span class="tok-val">{{ formatTokens(row.output_tokens) }}</span>
                  </span>
                </td>
                <td class="mono">
                  <span class="tok-group">
                    <span class="tok-val" :class="{ zero: !row.cache_read_input_tokens }">{{ formatTokens(row.cache_read_input_tokens) }}</span>
                    <span class="tok-sep">/</span>
                    <span class="tok-val" :class="{ zero: !row.cache_creation_input_tokens }">{{ formatTokens(row.cache_creation_input_tokens) }}</span>
                  </span>
                </td>
                <td class="mono">${{ formatCost(row.total_cost) }}</td>
                <td class="mono logs-map">
                  <template v-if="row.source_model || row.target_model">
                    <span class="logs-model-badge">{{ row.source_model || '-' }}</span>
                    <span class="logs-model-arrow">→</span>
                    <span class="logs-model-badge">{{ row.target_model || '-' }}</span>
                  </template>
                  <span v-else class="logs-model-empty">-</span>
                </td>
                <td class="logs-sticky-col"><a v-if="row.finished_at" class="logs-link" @click="showRequestDetail(row.id)">详情</a><span v-else class="v2-hint">-</span></td>
              </tr>
              <tr v-if="requestLogs.length === 0"><td colspan="13" class="logs-empty">暂无日志记录</td></tr>
            </tbody>
          </table>
        </div>
        <div class="logs-pager">
          <span class="v2-hint">总计 {{ requestTotal }}</span>
          <el-pagination size="small" v-model:current-page="requestPage" v-model:page-size="requestPageSize" :page-sizes="[20, 50, 100]" :total="requestTotal" layout="sizes, prev, pager, next" @size-change="fetchRequestLogs" @current-change="fetchRequestLogs" />
        </div>
      </div>
    </template>

    <!-- 系统日志 -->
    <template v-else>
      <div v-loading="systemLoading" class="v2-card logs-tablecard">
        <div class="logs-filters">
          <AppSelect :model-value="systemFilters.event_type" :options="eventTypeOptions" width="140px" @change="handleSystemEventTypeChange" />
          <div style="flex:1"></div>
          <el-tooltip content="查询" placement="top" effect="light" :show-after="250">
            <button class="v2-row-act" @click="fetchSystemLogs"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg></button>
          </el-tooltip>
          <el-tooltip content="重置" placement="top" effect="light" :show-after="250">
            <button class="v2-row-act" @click="resetSystemFilters"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/><path d="M16 16h5v5"/></svg></button>
          </el-tooltip>
          <el-tooltip content="清空" placement="top" effect="light" :show-after="250">
            <button class="v2-row-act danger" @click="clearSystemLogs"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg></button>
          </el-tooltip>
        </div>

        <div class="logs-scroll">
          <table class="v2-table">
            <thead><tr><th>ID</th><th>时间</th><th>事件类型</th><th>事件消息</th></tr></thead>
            <tbody>
              <tr v-for="row in systemLogs" :key="row.id">
                <td class="mono">{{ row.id }}</td>
                <td class="mono">{{ formatTime(row.created_at) }}</td>
                <td>{{ formatEventType(row.event_type) }}</td>
                <td>{{ row.message }}</td>
              </tr>
              <tr v-if="systemLogs.length === 0"><td colspan="4" class="logs-empty">暂无日志记录</td></tr>
            </tbody>
          </table>
        </div>
        <div class="logs-pager">
          <span class="v2-hint">总计 {{ systemTotal }}</span>
          <el-pagination size="small" v-model:current-page="systemPage" v-model:page-size="systemPageSize" :page-sizes="[20, 50, 100]" :total="systemTotal" layout="sizes, prev, pager, next" @size-change="fetchSystemLogs" @current-change="fetchSystemLogs" />
        </div>
      </div>
    </template>

    <V2Drawer v-model="requestDetailVisible" title="请求详情" :show-footer="false" width="80%">
      <div v-if="requestDetail" class="logs-detail">
        <div class="logs-detail-meta">
          <span class="v2-pill v2-pill-neutral">{{ formatCliLabel(requestDetail.cli_type) }}</span>
          <span class="v2-pill v2-pill-neutral mono">{{ requestDetail.profile || 'default' }}</span>
          <span class="v2-pill v2-pill-info mono">{{ formatProtocolLabel(requestDetail.protocol) }}</span>
          <span class="v2-pill v2-pill-neutral">{{ requestDetail.provider_name || '未选择服务商' }}<template v-if="requestDetail.provider_id != null"> #{{ requestDetail.provider_id }}</template></span>
        </div>
        <div v-if="requestDetail.error_message" class="logs-detail-err">{{ requestDetail.error_message }}</div>
        <div class="logs-detail-list">
          <section v-for="section in detailSections" :key="section.group" class="logs-sec" :class="{ open: expandedDetailGroups[section.group] }">
            <div class="logs-sec-toggle" role="button" tabindex="0" :aria-expanded="expandedDetailGroups[section.group]" @click="toggleDetailGroup(section.group)" @keydown.enter.prevent="toggleDetailGroup(section.group)" @keydown.space.prevent="toggleDetailGroup(section.group)">
              <span class="logs-sec-main">
                <span class="logs-sec-title">{{ section.title }}</span>
                <span class="logs-sec-sub mono" @click.stop @mousedown.stop>{{ section.subtitle }}</span>
              </span>
              <span class="logs-sec-side">
                <span class="logs-sec-side-top">
                  <span class="v2-pill" :class="[section.badgeClass, { dot: section.group === 'provider' }]">{{ section.badge }}</span>
                  <svg class="logs-sec-caret" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>
                </span>
                <span class="logs-sec-summary">{{ section.summary }}</span>
              </span>
            </div>
            <div v-if="expandedDetailGroups[section.group]" class="logs-sec-body">
              <div v-for="block in section.blocks" :key="block.key" class="logs-block">
                <div class="logs-block-h">
                  <span>{{ block.label }}</span>
                  <span class="logs-block-meta">
                    <span class="mono">{{ block.meta }}</span>
                  </span>
                </div>
                <div class="logs-textbox mono" @click="copyDetailBlock(block)">
                  <pre class="logs-pre"><code>{{ block.previewText }}</code></pre>
                </div>
              </div>
            </div>
          </section>
        </div>
      </div>
    </V2Drawer>
  </div>
</template>

<script setup lang="ts">
import V2Drawer from '@/components/V2Drawer.vue'
import AppSelect, { type AppSelectOption } from '@/components/AppSelect.vue'
import CliBrandIcon from '@/components/CliBrandIcon.vue'
import { confirm } from '@/utils/confirm'
import { notify } from '@/utils/notification'
import { logsApi } from '@/api/logs'
import { statsApi } from '@/api/stats'
import { providersApi } from '@/api/providers'
import { settingsApi } from '@/api/settings'
import { useUiStore } from '@/stores/ui'
import { useAgentStore } from '@/stores/agents'
import { getErrorMessage } from '@/utils/error'
import { formatCost, formatJson as formatJsonUtil, formatTokens } from '@/utils/json'
import { PROTOCOL_LABELS } from '@/types/models'
import type { Protocol, RequestLogListItem, RequestLogDetail, SystemLogItem } from '@/types/models'

type LogRecordMode = 'full' | 'failure_only' | 'disabled'
const logModeMap: Record<LogRecordMode, string> = { full: '全量记录', failure_only: '失败时记录详情', disabled: '停用日志' }
type DetailBlockGroup = 'client' | 'forward' | 'provider'
type DetailBlockKey = 'client_headers' | 'client_body' | 'forward_headers' | 'forward_body' | 'provider_headers' | 'provider_body'
interface DetailBlock {
  key: DetailBlockKey
  group: DetailBlockGroup
  label: string
  fullText: string
  previewText: string
  meta: string
}
interface DetailSection {
  group: DetailBlockGroup
  title: string
  subtitle: string
  badge: string | number
  badgeClass: string
  summary: string
  blocks: DetailBlock[]
}

const DETAIL_FORMAT_CHARS = 128 * 1024
const DETAIL_PREVIEW_CHARS = 32 * 1024

const uiStore = useUiStore()
const agentStore = useAgentStore()
const activeTab = computed({
  get: () => uiStore.logsActiveTab,
  set: (v) => uiStore.setLogsActiveTab(v as 'request' | 'system')
})
const logRecordMode = ref<LogRecordMode>('failure_only')
const cleanMenuItems: AppSelectOption[] = [
  { label: '清理全部日志', value: 'all_logs' },
  { label: '清理全部详情', value: 'all_details' },
  { label: '清理统计数据', value: 'stats_data' },
  { label: '清理30天前日志', value: 'old_logs' },
  { label: '清理30天前详情', value: 'old_details' }
]
const gatewayUrl = ref('')
const providerOptions = ref<string[]>([])
let requestLogListener: (() => void) | null = null
let requestLogUpdateListener: (() => void) | null = null
let requestElapsedTimer: ReturnType<typeof setInterval> | null = null

const cliFilterOptions = computed<AppSelectOption[]>(() => [
  { label: '全部 Agent', value: '' },
  ...agentStore.agents.map((agent) => ({ label: agent.name, value: agent.id })),
])
const providerFilterOptions = computed<AppSelectOption[]>(() => [{ label: '全部服务商', value: '' }, ...providerOptions.value.map(p => ({ label: p, value: p }))])
const logModeOptions = computed<AppSelectOption[]>(() => Object.entries(logModeMap).map(([value, label]) => ({ value, label })))

const requestLogs = ref<RequestLogListItem[]>([])
const requestLoading = ref(false)
const requestPage = ref(1)
const requestPageSize = ref(20)
const requestTotal = ref(0)
const requestFilters = ref({ cli_type: '', provider_name: '' })
const requestDetailVisible = ref(false)
const requestDetail = ref<RequestLogDetail | null>(null)
const currentTimestamp = ref(Math.floor(Date.now() / 1000))
const detailBlocks = ref<DetailBlock[]>([])
const detailBlockGroups = computed(() => ({
  client: detailBlocks.value.filter((block) => block.group === 'client'),
  forward: detailBlocks.value.filter((block) => block.group === 'forward'),
  provider: detailBlocks.value.filter((block) => block.group === 'provider')
}))
const expandedDetailGroups = ref<Record<DetailBlockGroup, boolean>>(createCollapsedDetailGroups())
const detailSections = computed<DetailSection[]>(() => {
  if (!requestDetail.value) return []
  return [
    {
      group: 'client',
      title: 'Agent请求',
      subtitle: getFullClientUrl(),
      badge: requestDetail.value.client_method,
      badgeClass: 'v2-pill-neutral',
      summary: formatDetailGroupSummary(detailBlockGroups.value.client),
      blocks: detailBlockGroups.value.client
    },
    {
      group: 'forward',
      title: '网关路由转发',
      subtitle: requestDetail.value.forward_url || '-',
      badge: requestDetail.value.client_method,
      badgeClass: 'v2-pill-neutral',
      summary: formatDetailGroupSummary(detailBlockGroups.value.forward),
      blocks: detailBlockGroups.value.forward
    },
    {
      group: 'provider',
      title: '服务商响应',
      subtitle: requestDetail.value.error_message || formatProviderSubtitle(),
      badge: requestDetail.value.status_code || '-',
      badgeClass: statusPill(requestDetail.value.status_code),
      summary: formatDetailGroupSummary(detailBlockGroups.value.provider),
      blocks: detailBlockGroups.value.provider
    }
  ]
})

const systemLogs = ref<SystemLogItem[]>([])
const systemLoading = ref(false)
const systemPage = ref(1)
const systemPageSize = ref(20)
const systemTotal = ref(0)
const systemFilters = ref({ event_type: '' })

async function fetchProviders() {
  try {
    const res = await providersApi.list()
    providerOptions.value = Array.from(new Set(res.data.map((p) => p.name)))
  } catch { /* ignore */ }
}
async function fetchLogSettings() {
  try {
    const res = await logsApi.getSettings()
    logRecordMode.value = res.data.debug_log ? res.data.log_detail_mode : 'disabled'
  } catch { /* ignore */ }
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
    if (mode === 'disabled') await logsApi.updateSettings({ debug_log: false })
    else await logsApi.updateSettings({ debug_log: true, log_detail_mode: mode })
  } catch { /* ignore */ }
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
    const params: any = { page: requestPage.value, page_size: requestPageSize.value }
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
function shouldIgnoreStaleRequestLog(current: RequestLogListItem, next: RequestLogListItem): boolean {
  if (current.finished_at && !next.finished_at) return true
  if (!next.finished_at && current.first_byte_ms > next.first_byte_ms) return true
  return false
}
function replaceRequestLog(log: RequestLogListItem): boolean {
  const index = requestLogs.value.findIndex(item => item.id === log.id)
  if (index < 0) return false
  if (shouldIgnoreStaleRequestLog(requestLogs.value[index], log)) return true
  requestLogs.value.splice(index, 1, log)
  return true
}
type CleanAction = 'all_logs' | 'all_details' | 'stats_data' | 'old_logs' | 'old_details'
async function handleClean(action: string | number) {
  const confirmMap: Record<CleanAction, string> = {
    all_logs: '确定要清空所有请求日志吗？', all_details: '确定要清空所有请求详情文件吗？', stats_data: '确定要清空所有统计数据吗？',
    old_logs: '确定要清理30天前的请求日志吗？', old_details: '确定要清理30天前的请求详情文件吗？'
  }
  try {
    await confirm(confirmMap[action as CleanAction], '清理确认')
  } catch {
    return
  }
  requestLoading.value = true
  try {
    switch (action as CleanAction) {
      case 'all_logs': await logsApi.clearRequestLogs(); notify('请求日志已清空'); break
      case 'all_details': await logsApi.clearRequestDetailFiles(); notify('请求详情文件已清空'); break
      case 'stats_data': await statsApi.clearStatsData(); notify('统计数据已清空'); break
      case 'old_logs': await logsApi.clearOldRequestLogs(30); notify('30天前的请求日志已清理'); break
      case 'old_details': await logsApi.clearOldRequestDetailFiles(30); notify('30天前的请求详情文件已清理'); break
    }
    await fetchRequestLogs()
  } catch (e: any) {
    notify(getErrorMessage(e, '清理失败'), 'error')
    requestLoading.value = false
  }
}
async function showRequestDetail(id: number) {
  try {
    requestDetail.value = null
    detailBlocks.value = []
    expandedDetailGroups.value = createCollapsedDetailGroups()
    const res = await logsApi.getRequestLog(id)
    requestDetail.value = res.data
    detailBlocks.value = buildDetailBlocks(res.data)
    requestDetailVisible.value = true
  } catch { /* ignore */ }
}
async function fetchSystemLogs() {
  systemLoading.value = true
  try {
    const params: any = { page: systemPage.value, page_size: systemPageSize.value }
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
    notify(getErrorMessage(e, '清空失败'), 'error')
    systemLoading.value = false
  }
}
function formatTime(timestamp: number): string {
  const d = new Date(timestamp * 1000)
  const p = (n: number) => String(n).padStart(2, '0')
  return `${p(d.getMonth() + 1)}/${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`
}

function formatCliLabel(type: string): string {
  return agentStore.get(type)?.name || type
}

function formatProtocolLabel(protocol: Protocol | null): string {
  return protocol ? PROTOCOL_LABELS[protocol] || protocol : '-'
}

function elapsedTimeClass(row: RequestLogListItem) {
  if (!row.finished_at) return ''
  const isErr = row.status_code && row.status_code >= 500
  if (isErr) return 'logs-time-danger'
  if (row.elapsed_ms >= 50000) return 'logs-time-danger-slow'
  if (row.elapsed_ms >= 20000) return 'logs-time-warning'
  return ''
}

function formatDuration(ms: number): string {
  return ms > 0 ? `${(ms / 1000).toFixed(2)}s` : '-'
}

function formatLatencyPair(row: RequestLogListItem): string {
  const elapsed = row.finished_at ? row.elapsed_ms : Math.max(0, (currentTimestamp.value - row.created_at) * 1000)
  return `${formatDuration(row.first_byte_ms)}/${formatDuration(elapsed)}`
}

function buildDetailBlocks(detail: RequestLogDetail): DetailBlock[] {
  return [
    createDetailBlock('client_headers', 'client', '请求头', detail.client_headers),
    createDetailBlock('client_body', 'client', '请求体', detail.client_body),
    createDetailBlock('forward_headers', 'forward', '转发头', detail.forward_headers),
    createDetailBlock('forward_body', 'forward', '转发体', detail.forward_body),
    createDetailBlock('provider_headers', 'provider', '响应头', detail.provider_headers),
    createDetailBlock('provider_body', 'provider', '响应体', detail.provider_body)
  ]
}
function createDetailBlock(key: DetailBlockKey, group: DetailBlockGroup, label: string, raw: string | null | undefined): DetailBlock {
  const fullText = formatDetailText(raw)
  return {
    key,
    group,
    label,
    fullText,
    previewText: formatDetailPreview(fullText),
    meta: formatDetailMeta(fullText)
  }
}
function formatDetailText(raw: string | null | undefined): string {
  if (!raw) return '—'
  return raw.length <= DETAIL_FORMAT_CHARS ? formatJsonUtil(raw) : raw
}
function formatDetailMeta(text: string): string {
  if (!text || text === '—') return '无内容'
  return formatDetailLength(text.length)
}
function formatDetailPreview(text: string): string {
  if (text.length <= DETAIL_PREVIEW_CHARS) return text
  return `${text.slice(0, DETAIL_PREVIEW_CHARS)}\n...`
}
function formatDetailLength(chars: number): string {
  if (chars < 1000) return `${chars} 字符`
  if (chars < 1000000) return `${(chars / 1000).toFixed(1)}K 字符`
  return `${(chars / 1000000).toFixed(1)}M 字符`
}
function createCollapsedDetailGroups(): Record<DetailBlockGroup, boolean> {
  return { client: false, forward: false, provider: false }
}
function toggleDetailGroup(group: DetailBlockGroup) {
  const next = !expandedDetailGroups.value[group]
  expandedDetailGroups.value = createCollapsedDetailGroups()
  expandedDetailGroups.value[group] = next
}
function formatDetailGroupSummary(blocks: DetailBlock[]): string {
  return blocks.map((block) => `${block.label} ${block.meta}`).join(' / ')
}
function formatProviderSubtitle(): string {
  if (!requestDetail.value) return ''
  const model = requestDetail.value.target_model || requestDetail.value.source_model
  return model ? `模型 ${model}` : '响应头 / 响应体'
}
const eventTypeMap: Record<string, string> = {
  no_provider_available: '无可用服务商', provider_blacklisted: '服务商黑名单', provider_recovered: '服务商恢复',
  provider_created: '服务商创建', provider_updated: '服务商更新', provider_deleted: '服务商删除',
  provider_reset: '状态重置', scheduled_task_failed: '定时任务失败',
  config_conflict: 'Agent 配置冲突', unknown_agent: '未知 Agent',
  protocol_conflict: '端点类型冲突', protocol_not_matched: '端点类型未匹配',
  config_patch_failed: '配置写入失败', official_credential_write_failed: '官方凭证写入失败',
}
const eventTypeOptions = computed<AppSelectOption[]>(() => [{ label: '全部事件', value: '' }, ...Object.entries(eventTypeMap).map(([value, label]) => ({ value, label }))])
function formatEventType(eventType: string): string {
  return eventType ? (eventTypeMap[eventType] || eventType) : ''
}
function statusPill(code: number | null): string {
  if (!code) return 'v2-pill-neutral'
  if (code >= 200 && code < 300) return 'v2-pill-success'
  if (code >= 500) return 'v2-pill-danger'
  return 'v2-pill-neutral'
}
function getFullClientUrl(): string {
  if (!requestDetail.value) return ''
  const path = requestDetail.value.client_path
  const baseUrl = gatewayUrl.value.replace(/\/$/, '')
  return `${baseUrl}/${path.startsWith('/') ? path.slice(1) : path}`
}
async function copyDetailBlock(block: DetailBlock) {
  if (!block.fullText || block.fullText === '—') return
  try {
    await navigator.clipboard.writeText(block.fullText)
    notify('已复制到剪贴板')
  } catch {
    notify('复制失败', 'error')
  }
}

watch(activeTab, (tab) => {
  if (tab === 'request') fetchRequestLogs()
  else fetchSystemLogs()
})

watch(requestDetailVisible, (visible) => {
  if (visible) return
  requestDetail.value = null
  detailBlocks.value = []
  expandedDetailGroups.value = createCollapsedDetailGroups()
})

onMounted(async () => {
  if (!agentStore.agents.length) await agentStore.fetchAgents()
  fetchLogSettings()
  fetchGatewayStatus()
  fetchProviders()
  const listeners = await Promise.all([
    logsApi.listenRequestLogs((log) => {
      if (activeTab.value === 'request' && requestPage.value === 1 && !requestFilters.value.cli_type && !requestFilters.value.provider_name) {
        if (!replaceRequestLog(log)) {
          requestLogs.value.unshift(log)
          requestTotal.value += 1
          if (requestLogs.value.length > requestPageSize.value) requestLogs.value.pop()
        }
      }
    }),
    logsApi.listenRequestLogUpdates((log) => {
      replaceRequestLog(log)
    })
  ])
  requestLogListener = listeners[0]
  requestLogUpdateListener = listeners[1]
  if (activeTab.value === 'request') fetchRequestLogs()
  else fetchSystemLogs()
  requestElapsedTimer = setInterval(() => {
    currentTimestamp.value = Math.floor(Date.now() / 1000)
  }, 1000)
})
onUnmounted(() => {
  if (requestLogListener) {
    requestLogListener()
    requestLogListener = null
  }
  if (requestLogUpdateListener) {
    requestLogUpdateListener()
    requestLogUpdateListener = null
  }
  if (requestElapsedTimer) {
    clearInterval(requestElapsedTimer)
    requestElapsedTimer = null
  }
})
</script>

<style scoped>
.logs-page { flex: 1; min-height: 0; display: flex; flex-direction: column; margin-top: -16px; }
.logs-bar { flex-shrink: 0; }
.logs-tabs { margin-bottom: 16px; }
.logs-filters { display: flex; align-items: center; gap: 8px; flex-shrink: 0; padding: 12px 16px; border-bottom: 1px solid var(--v2-surface-2); }
.logs-flabel { font-size: var(--v2-fs-xs); color: var(--v2-text-3); }

.logs-tablecard { flex: 1; min-height: 0; display: flex; flex-direction: column; padding: 0; overflow: hidden; }
.logs-scroll { flex: 1; overflow: auto; }
.logs-scroll thead th { position: sticky; top: 0; z-index: 1; text-align: center; }
.logs-scroll tbody td { text-align: center; }
.logs-map { text-align: center; }
.logs-danger { color: var(--v2-danger); }
.logs-running { min-width: 42px; justify-content: center; }
.logs-link { color: var(--v2-accent); cursor: pointer; font-size: var(--v2-fs-sm); }
.logs-empty { text-align: center; color: var(--v2-text-3); padding: 40px; }
.logs-pager { display: flex; align-items: center; justify-content: space-between; padding: 10px 16px; border-top: 1px solid var(--v2-surface-2); flex-shrink: 0; }

.logs-detail { display: flex; flex-direction: column; gap: 12px; min-width: 0; }
.logs-detail-meta { display: flex; align-items: center; flex-wrap: wrap; gap: 6px; }
.logs-detail-err { padding: 10px 12px; border-radius: var(--v2-r-sm); background: var(--v2-danger-bg); color: var(--v2-danger); font-size: var(--v2-fs-sm); }
.logs-detail-list { display: flex; flex-direction: column; gap: 10px; }
.logs-sec { min-width: 0; border: 1px solid var(--v2-surface-3); border-radius: var(--v2-r-sm); background: var(--v2-surface); overflow: hidden; transition: border-color 0.15s, box-shadow 0.15s; }
.logs-sec.open { border-color: color-mix(in srgb, var(--v2-accent) 28%, var(--v2-surface-3)); box-shadow: 0 8px 22px rgba(25, 36, 64, 0.06); }
.logs-sec-toggle { width: 100%; min-height: 72px; border: none; background: var(--v2-surface); color: var(--v2-text); display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 12px 14px; text-align: left; cursor: pointer; }
.logs-sec-toggle:hover { background: var(--v2-surface-2); }
.logs-sec-main { min-width: 0; display: flex; flex-direction: column; gap: 6px; }
.logs-sec-title { font-size: var(--v2-fs-sm); font-weight: var(--v2-fw-semibold); color: var(--v2-text); }
.logs-sec-sub { max-width: 100%; color: var(--v2-text-3); font-size: var(--v2-fs-xs); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; cursor: text; user-select: text; }
.logs-sec-side { width: min(420px, 42%); flex-shrink: 0; display: flex; flex-direction: column; align-items: stretch; gap: 7px; color: var(--v2-text-3); font-size: var(--v2-fs-xs); }
.logs-sec-side-top { display: flex; align-items: center; justify-content: flex-end; gap: 10px; }
.logs-sec-summary { display: block; width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; text-align: right; }
.logs-sec-caret { flex-shrink: 0; color: var(--v2-text-3); transition: transform 0.15s; }
.logs-sec.open .logs-sec-caret { transform: rotate(180deg); }
.logs-sec-body { padding: 0 14px 14px; }
.logs-block { margin-bottom: 10px; }
.logs-block:last-child { margin-bottom: 0; }
.logs-block-h { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-bottom: 6px; color: var(--v2-text-2); font-size: var(--v2-fs-xs); }
.logs-block-meta { display: inline-flex; align-items: center; gap: 8px; color: var(--v2-text-3); white-space: nowrap; }
.logs-textbox { background: var(--v2-surface); color: var(--v2-text); padding: 8px 11px; border-radius: var(--v2-r-sm); font-size: var(--v2-fs-sm); line-height: 1.55; max-height: min(42vh, 420px); overflow: auto; cursor: pointer; border: 1px solid var(--v2-surface-3); box-shadow: none; content-visibility: auto; }
.logs-textbox:hover { border-color: var(--v2-surface-3); }
.logs-pre { margin: 0; white-space: pre-wrap; word-break: break-word; font: inherit; }

.logs-cli-cell { display: inline-flex; align-items: center; gap: 6px; }
.logs-cli-icon { display: inline-flex; align-items: center; justify-content: center; width: 14px; height: 14px; flex-shrink: 0; }
.logs-cli-text { font-size: var(--v2-fs-sm); color: var(--v2-text); }
.logs-provider-id { margin-left: 5px; color: var(--v2-text-3); font-size: var(--v2-fs-xs); }

.logs-model-badge { display: inline-block; font-size: var(--v2-fs-xs); padding: 2px 6px; background: var(--v2-surface-2); border: 1px solid var(--v2-surface-2); border-radius: 4px; color: var(--v2-text-2); white-space: nowrap; vertical-align: middle; }
.logs-model-arrow { margin: 0 4px; color: var(--v2-text-3); font-size: var(--v2-fs-xs); vertical-align: middle; }
.logs-model-empty { color: var(--v2-text-3); }

.logs-scroll th.logs-sticky-col {
  position: sticky;
  right: 0;
  z-index: 3;
  background: var(--v2-surface-2);
  border-left: 1px solid var(--v2-surface-3);
}
.logs-scroll td.logs-sticky-col {
  position: sticky;
  right: 0;
  z-index: 2;
  background: var(--v2-surface);
  border-left: 1px solid var(--v2-surface-3);
}
.logs-scroll tbody tr:hover td.logs-sticky-col {
  background: var(--v2-surface-2);
}

.logs-time-danger { color: var(--v2-danger); }
.logs-time-danger-slow { color: var(--v2-danger); font-weight: var(--v2-fw-regular); }
.logs-time-warning { color: var(--v2-warning); font-weight: var(--v2-fw-regular); }

.tok-group { display: inline-flex; align-items: center; gap: 3px; font-family: inherit; }
.tok-val { font-size: var(--v2-fs-sm); color: var(--v2-text); }
.tok-sep { color: var(--v2-text-3); margin: 0 1px; font-size: var(--v2-fs-sm); }

</style>
