<template>
  <div class="logs-page">
    <div class="logs-bar">
      <V2Tabs class="logs-tabs">
        <div class="v2-tab" :class="{ active: activeTab === 'request' }" @click="activeTab = 'request'">{{ t('logs.tabs.request') }}</div>
        <div class="v2-tab" :class="{ active: activeTab === 'system' }" @click="activeTab = 'system'">{{ t('logs.tabs.system') }}</div>
      </V2Tabs>
    </div>

    <!-- 请求日志 -->
    <template v-if="activeTab === 'request'">
      <div v-loading="requestLoading" class="v2-card logs-tablecard">
        <div class="logs-filters">
          <AppSelect :model-value="requestFilters.cli_type" :options="cliFilterOptions" width="120px" @change="handleRequestCliChange" />
          <AppSelect :model-value="requestFilters.provider_name" :options="providerFilterOptions" width="160px" @change="handleRequestProviderChange" />
          <div style="flex:1"></div>
          <span class="logs-flabel">{{ t('logs.filters.logLevel') }}</span>
          <AppSelect :model-value="logRecordMode" :options="logModeOptions" width="140px" @change="v => setLogMode(v as LogRecordMode)" />
          <el-tooltip :content="t('logs.filters.query')" placement="top" effect="light" :show-after="250">
            <button class="v2-row-act" @click="fetchRequestLogs"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg></button>
          </el-tooltip>
          <el-tooltip :content="t('logs.filters.reset')" placement="top" effect="light" :show-after="250">
            <button class="v2-row-act" @click="resetRequestFilters"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/><path d="M16 16h5v5"/></svg></button>
          </el-tooltip>
          <el-tooltip :content="t('logs.filters.clean')" placement="top" effect="light" :show-after="250">
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
          <table class="v2-table logs-request-table">
            <thead>
              <tr>
                <th>ID</th><th>Agent</th><th>{{ t('logs.table.provider') }}</th><th>{{ t('logs.table.status') }}</th>
                <th>
                  <el-tooltip :content="t('logs.table.durationTip')" placement="top" effect="light" :show-after="250">
                    <span>{{ t('logs.table.duration') }}</span>
                  </el-tooltip>
                </th>
                <th>
                  <el-tooltip :content="t('logs.table.speedTip')" placement="top" effect="light" :show-after="250">
                    <span>{{ t('logs.table.speed') }}</span>
                  </el-tooltip>
                </th>
                <th>
                  <el-tooltip :content="t('logs.table.tokensTip')" placement="top" effect="light" :show-after="250">
                    <span>{{ t('logs.table.tokens') }}</span>
                  </el-tooltip>
                </th>
                <th>
                  <el-tooltip :content="t('logs.table.cacheHitRateTip')" placement="top" effect="light" :show-after="250">
                    <span>{{ t('logs.table.cacheHitRate') }}</span>
                  </el-tooltip>
                </th>
                <th>{{ t('logs.table.cost') }}</th><th>{{ t('logs.table.time') }}</th><th>{{ t('logs.table.modelMap') }}</th><th class="logs-sticky-col">{{ t('logs.table.actions') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in requestLogs" :key="row.id">
                <td class="mono">{{ row.id }}</td>
                <td>
                  <el-tooltip :content="formatCliLabel(row.cli_type)" placement="top" effect="light" :show-after="250">
                    <div class="logs-cli-cell">
                      <span class="logs-cli-icon">
                        <CliBrandIcon :type="row.cli_type" width="14" height="14" />
                      </span>
                    </div>
                  </el-tooltip>
                </td>
                <td>{{ row.provider_name || '-' }}</td>
                <td>
                  <el-tooltip v-if="!row.finished_at" :content="t('logs.table.running')" placement="top" effect="light" :show-after="250">
                    <span class="v2-pill dot v2-pill-info logs-running">Run</span>
                  </el-tooltip>
                  <span v-else-if="row.status_code" class="v2-pill dot" :class="statusPill(row.status_code)">{{ row.status_code }}</span>
                  <span v-else>-</span>
                </td>
                <td class="mono" :class="elapsedTimeClass(row)">
                  <div class="logs-cell-stack logs-metric-stack">
                    <span class="logs-cell-line"><span class="logs-cell-label">{{ t('logs.table.firstByte') }}</span><span>{{ formatDuration(row.first_byte_ms) }}</span></span>
                    <span class="logs-cell-line"><span class="logs-cell-label">{{ t('logs.table.total') }}</span><span>{{ formatDuration(requestElapsedMs(row)) }}</span></span>
                  </div>
                </td>
                <td class="mono">
                  <span class="logs-speed-cell">{{ formatTokenSpeed(row) }}</span>
                </td>
                <td class="mono">
                  <div class="logs-cell-stack logs-metric-stack">
                    <span class="logs-cell-line"><span class="logs-cell-label">{{ t('logs.table.input') }}</span><span>{{ formatTokens(row.input_tokens) }}</span></span>
                    <span class="logs-cell-line"><span class="logs-cell-label">{{ t('logs.table.output') }}</span><span>{{ formatTokens(row.output_tokens) }}</span></span>
                  </div>
                </td>
                <td class="mono">{{ formatCacheHitRate(row) }}</td>
                <td class="mono logs-cost-cell">
                  <span>${{ formatCost(row.total_cost) }}</span>
                  <el-tooltip placement="top" effect="light" :show-after="150" :enterable="true" popper-class="v2-profile-pop v2-scope">
                    <template #content>
                      <div class="profile-help logs-cost-tooltip">
                        <div class="tooltip-title">{{ t('logs.cost.title') }}</div>
                        <div class="logs-cost-line">
                          <span class="logs-cost-label">{{ t('logs.cost.model') }}</span>
                          <span class="logs-cost-expr logs-cost-model"><OverflowText :text="costModel(row)" /></span>
                        </div>
                        <div v-for="line in costLines(row)" :key="line.label" class="logs-cost-line">
                          <span class="logs-cost-label">{{ line.label }}</span>
                          <span class="logs-cost-expr">{{ line.tokens }} × ${{ line.price }}/M</span>
                          <span class="logs-cost-amount">${{ line.amount }}</span>
                        </div>
                        <div class="logs-cost-line logs-cost-total">
                          <span class="logs-cost-label">{{ t('logs.cost.total') }}</span>
                          <span class="logs-cost-expr">{{ row.cost?.matched ? `× ${formatPrice(row.cost.multiplier)}` : t('logs.cost.notMatched') }}</span>
                          <span class="logs-cost-amount">${{ formatCost(row.total_cost) }}</span>
                        </div>
                        <div v-if="row.cost?.source" class="logs-cost-note">{{ t('logs.cost.source', { source: formatCostSource(row.cost.source) }) }}</div>
                        <div v-if="costNote(row)" class="logs-cost-note">{{ costNote(row) }}</div>
                      </div>
                    </template>
                    <span class="logs-cost-info" tabindex="0" :aria-label="t('logs.cost.viewDetail')"><el-icon><InfoFilled /></el-icon></span>
                  </el-tooltip>
                </td>
                <td class="mono">
                  <el-tooltip :content="formatFullTime(row.created_at)" placement="top" effect="light" :show-after="250">
                    <div class="logs-cell-stack logs-time-cell">
                      <span class="logs-cell-sub">{{ formatLogDate(row.created_at) }}</span>
                      <span>{{ formatClockTime(row.created_at) }}</span>
                    </div>
                  </el-tooltip>
                </td>
                <td class="mono logs-map">
                  <div v-if="requestModel(row)" class="logs-cell-stack logs-model-stack">
                    <span class="logs-cell-line logs-model-line">
                      <span class="logs-cell-label">{{ t('logs.table.request') }}</span>
                      <span class="logs-model-badge"><OverflowText :text="requestModel(row)" /></span>
                    </span>
                    <span class="logs-cell-line logs-model-line">
                      <span class="logs-cell-label">{{ t('logs.table.upstream') }}</span>
                      <span v-if="row.target_model" class="logs-model-badge"><OverflowText :text="row.target_model" /></span>
                      <span v-else class="logs-model-badge logs-model-direct">{{ t('logs.table.direct') }}</span>
                      <el-tooltip v-if="row.upstream_protocol" :content="t('logs.table.protocolConvert', { from: formatProtocolLabel(row.protocol), to: formatProtocolLabel(row.upstream_protocol) })" placement="top" effect="light" :show-after="250">
                        <span class="logs-model-protocol">
                          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 18h1.4c1.3 0 2.5-.6 3.3-1.7l6.1-8.6c.8-1.1 2-1.7 3.3-1.7H22"/><path d="m18 2 4 4-4 4"/><path d="M2 6h1.9c1.5 0 2.9.9 3.6 2.2"/><path d="M22 18h-5.9c-1.3 0-2.6-.7-3.3-1.8l-.5-.8"/><path d="m18 14 4 4-4 4"/></svg>
                        </span>
                      </el-tooltip>
                    </span>
                  </div>
                  <span v-else class="logs-model-empty">-</span>
                </td>
                <td class="logs-sticky-col"><a v-if="row.finished_at" class="logs-link" @click="showRequestDetail(row.id)">{{ t('logs.table.detail') }}</a><span v-else class="v2-hint">-</span></td>
              </tr>
              <tr v-if="requestLogs.length === 0"><td colspan="12" class="logs-empty">{{ t('logs.table.empty') }}</td></tr>
            </tbody>
          </table>
        </div>
        <div class="logs-pager">
          <span class="v2-hint">{{ t('logs.table.totalCount', { total: requestTotal }) }}</span>
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
          <el-tooltip :content="t('logs.filters.query')" placement="top" effect="light" :show-after="250">
            <button class="v2-row-act" @click="fetchSystemLogs"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg></button>
          </el-tooltip>
          <el-tooltip :content="t('logs.filters.reset')" placement="top" effect="light" :show-after="250">
            <button class="v2-row-act" @click="resetSystemFilters"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/><path d="M16 16h5v5"/></svg></button>
          </el-tooltip>
          <el-tooltip :content="t('logs.filters.clear')" placement="top" effect="light" :show-after="250">
            <button class="v2-row-act danger" @click="clearSystemLogs"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg></button>
          </el-tooltip>
        </div>

        <div class="logs-scroll">
          <table class="v2-table">
            <thead><tr><th>ID</th><th>{{ t('logs.table.time') }}</th><th>{{ t('logs.table.eventType') }}</th><th>{{ t('logs.table.message') }}</th></tr></thead>
            <tbody>
              <tr v-for="row in systemLogs" :key="row.id">
                <td class="mono">{{ row.id }}</td>
                <td class="mono">{{ formatTime(row.created_at) }}</td>
                <td>{{ formatEventType(row.event_type) }}</td>
                <td>{{ formatSystemLogMessage(row) }}</td>
              </tr>
              <tr v-if="systemLogs.length === 0"><td colspan="4" class="logs-empty">{{ t('logs.table.empty') }}</td></tr>
            </tbody>
          </table>
        </div>
        <div class="logs-pager">
          <span class="v2-hint">{{ t('logs.table.totalCount', { total: systemTotal }) }}</span>
          <el-pagination size="small" v-model:current-page="systemPage" v-model:page-size="systemPageSize" :page-sizes="[20, 50, 100]" :total="systemTotal" layout="sizes, prev, pager, next" @size-change="fetchSystemLogs" @current-change="fetchSystemLogs" />
        </div>
      </div>
    </template>

    <V2Drawer v-model="requestDetailVisible" :title="t('logs.detail.title')" :show-footer="false" width="80%">
      <div v-if="requestDetail" class="logs-detail">
        <div class="logs-detail-meta">
          <span class="v2-pill v2-pill-neutral">{{ formatCliLabel(requestDetail.cli_type) }}</span>
          <span class="v2-pill v2-pill-neutral mono">{{ requestDetail.profile || 'default' }}</span>
          <span class="v2-pill v2-pill-info mono">{{ formatProtocolLabel(requestDetail.protocol) }}<template v-if="requestDetail.upstream_protocol"> → {{ formatProtocolLabel(requestDetail.upstream_protocol) }}</template></span>
          <span class="v2-pill v2-pill-neutral">{{ requestDetail.provider_name || t('logs.detail.noProvider') }}</span>
        </div>
        <div v-if="requestDetail.error_message" class="logs-detail-err" :class="errorMessageClass(requestDetail)">
          <span class="logs-err-prefix">{{ errorMessagePrefix(requestDetail) }}</span>{{ errorMessageText(requestDetail) }}
        </div>
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
import V2Tabs from '@/components/V2Tabs.vue'
import AppSelect, { type AppSelectOption } from '@/components/AppSelect.vue'
import OverflowText from '@/components/OverflowText.vue'
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
import { InfoFilled } from '@element-plus/icons-vue'
import { useI18n } from 'vue-i18n'
import { PROTOCOL_LABELS } from '@/types/models'
import type { Protocol, RequestLogListItem, RequestLogDetail, SystemLogItem } from '@/types/models'

type LogRecordMode = 'full' | 'failure_only' | 'disabled'
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
interface CostLine {
  label: string
  tokens: string
  price: string
  amount: string
}

const DETAIL_FORMAT_CHARS = 128 * 1024
const DETAIL_PREVIEW_CHARS = 32 * 1024

const uiStore = useUiStore()
const agentStore = useAgentStore()
const { t } = useI18n()
const activeTab = computed({
  get: () => uiStore.logsActiveTab,
  set: (v) => uiStore.setLogsActiveTab(v as 'request' | 'system')
})
const logRecordMode = ref<LogRecordMode>('failure_only')
const cleanMenuItems = computed<AppSelectOption[]>(() => [
  { label: t('logs.clean.allLogs'), value: 'all_logs' },
  { label: t('logs.clean.allDetails'), value: 'all_details' },
  { label: t('logs.clean.statsData'), value: 'stats_data' },
  { label: t('logs.clean.oldLogs'), value: 'old_logs' },
  { label: t('logs.clean.oldDetails'), value: 'old_details' }
])
const gatewayUrl = ref('')
const providerOptions = ref<string[]>([])
let requestLogListener: (() => void) | null = null
let requestLogUpdateListener: (() => void) | null = null
let requestElapsedTimer: ReturnType<typeof setInterval> | null = null

const cliFilterOptions = computed<AppSelectOption[]>(() => [
  { label: t('logs.filters.allAgents'), value: '' },
  ...agentStore.agents.map((agent) => ({ label: agent.name, value: agent.id })),
])
const providerFilterOptions = computed<AppSelectOption[]>(() => [{ label: t('logs.filters.allProviders'), value: '' }, ...providerOptions.value.map(p => ({ label: p, value: p }))])
const logModeOptions = computed<AppSelectOption[]>(() => [
  { value: 'full', label: t('logs.mode.full') },
  { value: 'failure_only', label: t('logs.mode.failure_only') },
  { value: 'disabled', label: t('logs.mode.disabled') }
])

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
      title: t('logs.detail.sectionClient'),
      subtitle: getFullClientUrl(),
      badge: requestDetail.value.client_method,
      badgeClass: 'v2-pill-neutral',
      summary: formatDetailGroupSummary(detailBlockGroups.value.client),
      blocks: detailBlockGroups.value.client
    },
    {
      group: 'forward',
      title: t('logs.detail.sectionForward'),
      subtitle: requestDetail.value.forward_url || '-',
      badge: requestDetail.value.client_method,
      badgeClass: 'v2-pill-neutral',
      summary: formatDetailGroupSummary(detailBlockGroups.value.forward),
      blocks: detailBlockGroups.value.forward
    },
    {
      group: 'provider',
      title: t('logs.detail.sectionProvider'),
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
const CLEAN_CONFIRM_KEYS: Record<CleanAction, string> = {
  all_logs: 'logs.clean.confirmAllLogs',
  all_details: 'logs.clean.confirmAllDetails',
  stats_data: 'logs.clean.confirmStats',
  old_logs: 'logs.clean.confirmOldLogs',
  old_details: 'logs.clean.confirmOldDetails'
}
async function handleClean(action: string | number) {
  try {
    await confirm(t(CLEAN_CONFIRM_KEYS[action as CleanAction]), t('logs.clean.title'))
  } catch {
    return
  }
  requestLoading.value = true
  try {
    switch (action as CleanAction) {
      case 'all_logs': await logsApi.clearRequestLogs(); notify(t('logs.clean.logsCleared')); break
      case 'all_details': await logsApi.clearRequestDetailFiles(); notify(t('logs.clean.detailsCleared')); break
      case 'stats_data': await statsApi.clearStatsData(); notify(t('logs.clean.statsCleared')); break
      case 'old_logs': await logsApi.clearOldRequestLogs(30); notify(t('logs.clean.oldLogsCleared')); break
      case 'old_details': await logsApi.clearOldRequestDetailFiles(30); notify(t('logs.clean.oldDetailsCleared')); break
    }
    await fetchRequestLogs()
  } catch (e: any) {
    notify(getErrorMessage(e, t('logs.clean.cleanFailed')), 'error')
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
    await confirm(t('logs.clean.confirmSystem'), t('logs.clean.title'))
  } catch {
    return
  }
  systemLoading.value = true
  try {
    await logsApi.clearSystemLogs()
    notify(t('logs.clean.systemCleared'))
    await fetchSystemLogs()
  } catch (e: any) {
    notify(getErrorMessage(e, t('logs.clean.clearFailed')), 'error')
    systemLoading.value = false
  }
}
function formatTime(timestamp: number): string {
  const d = new Date(timestamp * 1000)
  const p = (n: number) => String(n).padStart(2, '0')
  return `${p(d.getMonth() + 1)}/${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`
}

function formatClockTime(timestamp: number): string {
  const d = new Date(timestamp * 1000)
  const p = (n: number) => String(n).padStart(2, '0')
  return `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`
}

function formatLogDate(timestamp: number): string {
  const d = new Date(timestamp * 1000)
  const p = (n: number) => String(n).padStart(2, '0')
  return `${String(d.getFullYear()).slice(-2)}-${p(d.getMonth() + 1)}-${p(d.getDate())}`
}

function formatFullTime(timestamp: number): string {
  const d = new Date(timestamp * 1000)
  const p = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`
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

function requestElapsedMs(row: RequestLogListItem): number {
  return row.finished_at ? row.elapsed_ms : Math.max(0, (currentTimestamp.value - row.created_at) * 1000)
}

function tokenSpeed(row: RequestLogListItem): number | null {
  if (!row.finished_at || row.output_tokens <= 0 || row.first_byte_ms <= 0) return null
  const generationMs = row.elapsed_ms - row.first_byte_ms
  return generationMs > 0 ? (row.output_tokens * 1000) / generationMs : null
}

function formatTokenSpeed(row: RequestLogListItem): string {
  const speed = tokenSpeed(row)
  if (speed === null) return '—'
  const value = speed >= 1000
    ? `${Number((speed / 1000).toFixed(1))}K`
    : Number(speed.toFixed(speed >= 100 ? 0 : 1)).toString()
  return `${value}t/s`
}

function requestModel(row: RequestLogListItem): string {
  return row.source_model || row.model_id || row.target_model || ''
}

function formatCacheHitRate(row: RequestLogListItem): string {
  const totalInput = row.input_tokens + row.cache_read_input_tokens + row.cache_creation_input_tokens
  return totalInput > 0 ? `${((row.cache_read_input_tokens / totalInput) * 100).toFixed(1)}%` : '-'
}

/** 单价去掉多余的 0，$3.00 显示成 $3，$0.014 保留原样。 */
function formatPrice(price: number): string {
  return Number(price.toFixed(4)).toString()
}

/** 费用浮窗按 token 类型逐行展示计算过程，四项始终列出。 */
function costLines(row: RequestLogListItem): CostLine[] {
  const multiplier = row.cost.multiplier > 0 ? row.cost.multiplier : 1
  const items: Array<[string, number, number]> = [
    [t('logs.cost.input'), row.input_tokens, row.cost.input_price_per_m],
    [t('logs.cost.output'), row.output_tokens, row.cost.output_price_per_m],
    [t('logs.cost.cacheRead'), row.cache_read_input_tokens, row.cost.cache_read_price_per_m],
    [t('logs.cost.cacheCreate'), row.cache_creation_input_tokens, row.cost.cache_creation_price_per_m]
  ]
  return items.map(([label, tokens, billedPrice]) => {
    const catalogPrice = billedPrice / multiplier
    return {
      label,
      tokens: formatTokens(tokens),
      price: formatPrice(catalogPrice),
      amount: formatCost((tokens * catalogPrice) / 1000000)
    }
  })
}

function costModel(row: RequestLogListItem): string {
  return row.model_id || row.source_model || '—'
}

/** models.dev 厂商渠道 id 的展示名，未收录的 id 原样显示。 */
function formatCostSource(source: string | null | undefined): string {
  if (!source) return ''
  const key = `logs.costSource.${source}`
  const label = t(key)
  if (!label || label === key) return source
  return label === source ? source : `${label}（${source}）`
}

function costNote(row: RequestLogListItem): string {
  const threshold = row.cost.tier_threshold_tokens
  return threshold ? t('logs.cost.longContext', { threshold: formatTokens(threshold) }) : ''
}

/** 网关生成的错误体（透传给客户端时）带这些 error.type。 */
const GATEWAY_ERROR_TYPES = ['upstream_error', 'upstream_http_error', 'upstream_stream_error', 'upstream_timeout', 'upstream_unavailable', 'upstream_body_error']

/** 旧数据（无 error_source）的兜底判定：网关自己生成的错误消息都是固定的中文短语。 */
const LEGACY_GATEWAY_ERROR_PREFIXES = [
  '上游请求失败',
  '首字节超时',
  '请求超时',
  '响应体读取超时',
  '读取响应体失败',
  '读取上游响应体失败',
  '读取上游错误响应失败',
  '读取上游错误响应超时',
  '上游流错误',
  '上游流协议错误',
  '上游流中断',
  '上游流在有效事件前结束',
  '上游流在完成事件前结束',
  '上游返回了空流',
  '上游返回了无效的 JSON 响应',
  '上游响应错误',
  '客户端在完成前断开了连接',
]

function isLegacyGatewayError(detail: RequestLogDetail): boolean {
  const msg = detail.error_message || ''
  if (LEGACY_GATEWAY_ERROR_PREFIXES.some((prefix) => msg.startsWith(prefix))) {
    return true
  }
  try {
    const parsed = JSON.parse(detail.provider_body || '')
    if (parsed?.error?.type && GATEWAY_ERROR_TYPES.includes(parsed.error.type)) {
      return true
    }
  } catch {
    // 非 JSON，继续
  }
  return false
}

function errorSource(detail: RequestLogDetail): 'gateway' | 'upstream' {
  if (detail.error_source === 'gateway' || detail.error_source === 'upstream') {
    return detail.error_source
  }
  return isLegacyGatewayError(detail) ? 'gateway' : 'upstream'
}

function errorMessagePrefix(detail: RequestLogDetail): string {
  return errorSource(detail) === 'gateway' ? t('logs.errorSource.gateway') : t('logs.errorSource.upstream')
}

function errorMessageClass(detail: RequestLogDetail): string {
  return errorSource(detail) === 'gateway' ? 'logs-detail-err-gateway' : 'logs-detail-err-upstream'
}

/** 网关生成的错误按 error_code + params 用当前语言渲染，其余（上游原文/旧数据）原样显示。 */
function errorMessageText(detail: RequestLogDetail): string {
  const raw = detail.error_message || ''
  if (detail.error_source === 'gateway' && detail.error_code) {
    const key = `logs.logErrors.${detail.error_code}`
    const label = t(key, parseMessageParams(detail.error_params))
    if (label && label !== key) return label
  }
  return raw
}

function parseMessageParams(raw: string | null | undefined): Record<string, unknown> {
  if (!raw) return {}
  try {
    const parsed: unknown = JSON.parse(raw)
    if (typeof parsed === 'object' && parsed !== null && !Array.isArray(parsed)) {
      return normalizeMessageParams(parsed as Record<string, unknown>)
    }
  } catch {
    // 参数不是合法 JSON 时按无参数处理
  }
  return {}
}

function normalizeMessageParams(params: Record<string, unknown>): Record<string, unknown> {
  const normalized: Record<string, unknown> = {}
  for (const [key, value] of Object.entries(params)) {
    normalized[key] = Array.isArray(value) ? value.join(', ') : value
  }
  return normalized
}

/** 系统日志：结构化数据显示为当前语言；旧数据原样显示保存时的文本。 */
function formatSystemLogMessage(row: SystemLogItem): string {
  if (!row.message_key) return row.message
  const params = parseMessageParams(row.message_params)
  if (typeof params.reason_code === 'string') {
    const reasonKey = `logs.logErrors.${params.reason_code}`
    const reasonParams = normalizeMessageParams((params.reason_params as Record<string, unknown>) || {})
    const reason = t(reasonKey, reasonParams)
    params.reason = reason !== reasonKey ? reason : String(params.reason ?? '')
  }
  const label = t(row.message_key, params)
  return label && label !== row.message_key ? label : row.message
}

function buildDetailBlocks(detail: RequestLogDetail): DetailBlock[] {
  return [
    createDetailBlock('client_headers', 'client', t('logs.detail.blockClientHeaders'), detail.client_headers),
    createDetailBlock('client_body', 'client', t('logs.detail.blockClientBody'), detail.client_body),
    createDetailBlock('forward_headers', 'forward', t('logs.detail.blockForwardHeaders'), detail.forward_headers),
    createDetailBlock('forward_body', 'forward', t('logs.detail.blockForwardBody'), detail.forward_body),
    createDetailBlock('provider_headers', 'provider', t('logs.detail.blockProviderHeaders'), detail.provider_headers),
    createDetailBlock('provider_body', 'provider', t('logs.detail.blockProviderBody'), detail.provider_body)
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
  if (!text || text === '—') return t('logs.detail.noContent')
  return formatDetailLength(text.length)
}
function formatDetailPreview(text: string): string {
  if (text.length <= DETAIL_PREVIEW_CHARS) return text
  return `${text.slice(0, DETAIL_PREVIEW_CHARS)}\n...`
}
function formatDetailLength(chars: number): string {
  if (chars < 1000) return t('logs.detail.chars', { count: chars })
  if (chars < 1000000) return t('logs.detail.chars', { count: `${(chars / 1000).toFixed(1)}K` })
  return t('logs.detail.chars', { count: `${(chars / 1000000).toFixed(1)}M` })
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
  return model ? t('logs.detail.providerSubtitleModel', { model }) : t('logs.detail.providerSubtitleFallback')
}
const EVENT_TYPE_KEYS = [
  'no_provider_available', 'provider_blacklisted', 'provider_recovered',
  'provider_created', 'provider_updated', 'provider_deleted',
  'provider_reset', 'scheduled_task_failed',
  'config_conflict', 'unknown_agent',
  'protocol_conflict', 'protocol_not_matched', 'passthrough_failed',
  'config_patch_failed', 'official_credential_write_failed',
  'gateway_endpoint_migration_failed',
]
const eventTypeOptions = computed<AppSelectOption[]>(() => [
  { label: t('logs.filters.allEvents'), value: '' },
  ...EVENT_TYPE_KEYS.map((value) => ({ value, label: t(`logs.eventType.${value}`) }))
])
function formatEventType(eventType: string): string {
  if (!eventType) return ''
  const key = `logs.eventType.${eventType}`
  const label = t(key)
  return label !== key ? label : eventType
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
    notify(t('logs.detail.copied'))
  } catch {
    notify(t('logs.detail.copyFailed'), 'error')
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
.logs-request-table { min-width: 1060px; }
.logs-scroll .logs-request-table th,
.logs-scroll .logs-request-table td { padding-left: 10px; padding-right: 10px; }
.logs-scroll .logs-request-table td { height: 56px; padding-top: 8px; padding-bottom: 8px; }
.logs-map { text-align: center; }
.logs-danger { color: var(--v2-danger); }
.logs-running { min-width: 42px; justify-content: center; }
.logs-link { color: var(--v2-accent); cursor: pointer; font-size: var(--v2-fs-sm); }
.logs-empty { text-align: center; color: var(--v2-text-3); padding: 40px; }
.logs-pager { display: flex; align-items: center; justify-content: space-between; padding: 10px 16px; border-top: 1px solid var(--v2-surface-2); flex-shrink: 0; }

.logs-detail { display: flex; flex-direction: column; gap: 12px; min-width: 0; }
.logs-detail-meta { display: flex; align-items: center; flex-wrap: wrap; gap: 6px; }
.logs-detail-err { padding: 10px 12px; border-radius: var(--v2-r-sm); font-size: var(--v2-fs-sm); line-height: 1.5; }
.logs-detail-err-gateway { background: var(--v2-danger-bg); color: var(--v2-danger); }
.logs-detail-err-upstream { background: var(--v2-warning-bg); color: var(--v2-warning); }
.logs-err-prefix { font-weight: var(--v2-fw-medium); margin-right: 4px; }
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
.logs-cell-stack { display: inline-flex; min-height: 34px; flex-direction: column; align-items: center; justify-content: center; gap: 3px; line-height: 1.1; vertical-align: middle; }
.logs-cell-line { display: inline-flex; max-width: 100%; align-items: center; gap: 5px; }
.logs-cell-label { flex: none; color: var(--v2-text-3); font-family: var(--font-ui); font-size: 11px; line-height: 1; }
.logs-cell-sub { color: var(--v2-text-3); font-size: 11px; }
.logs-metric-stack { align-items: flex-start; }
.logs-metric-stack .logs-cell-label { width: 11px; text-align: center; }
.logs-speed-cell { display: inline-block; white-space: nowrap; }
.logs-time-cell { gap: 3px; }
.logs-model-stack { width: 150px; align-items: stretch; }
.logs-model-line { width: 100%; min-width: 0; }
.logs-model-line .logs-cell-label { width: 24px; text-align: right; }
.logs-model-badge { display: block; min-width: 0; flex: 1; overflow: hidden; text-overflow: ellipsis; font-size: 11px; padding: 2px 6px; background: var(--v2-surface-2); border: 1px solid var(--v2-surface-2); border-radius: 4px; color: var(--v2-text-2); white-space: nowrap; }
.logs-model-direct { color: var(--v2-text-3); font-family: var(--font-ui); text-align: center; }
.logs-model-protocol { display: inline-flex; flex: none; align-items: center; color: var(--v2-warning); cursor: help; }
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
  background: var(--v2-row-hover-bg);
}

.logs-time-danger { color: var(--v2-danger); }
.logs-time-danger-slow { color: var(--v2-danger); font-weight: var(--v2-fw-regular); }
.logs-time-warning { color: var(--v2-warning); font-weight: var(--v2-fw-regular); }

.logs-cost-cell { white-space: nowrap; }
.logs-cost-info { display: inline-flex; align-items: center; justify-content: center; width: 14px; height: 14px; margin-left: 5px; color: var(--v2-text-3); cursor: help; vertical-align: -2px; }
.logs-cost-info:hover, .logs-cost-info:focus { color: var(--v2-text-2); outline: none; }
.logs-cost-info .el-icon { font-size: 14px; }
.logs-cost-tooltip { width: 300px; }
.logs-cost-line { display: flex; align-items: baseline; gap: 6px; font-family: var(--font-mono); font-size: var(--v2-fs-xs); }
.logs-cost-line + .logs-cost-line { margin-top: 3px; }
.logs-cost-label { width: 56px; flex: none; color: var(--v2-text-2); font-family: var(--font-ui); }
.logs-cost-expr { flex: 1; color: var(--v2-text-3); min-width: 0; }
.logs-cost-model { color: var(--v2-text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; text-align: right; }
.logs-cost-amount { flex: none; color: var(--v2-text); }
.logs-cost-total { margin-top: 5px; padding-top: 5px; border-top: 1px solid var(--v2-surface-3); }
.logs-cost-total .logs-cost-amount { font-weight: var(--v2-fw-medium); }
.logs-cost-note { margin-top: 6px; color: var(--v2-text-3); font-size: var(--v2-fs-xs); line-height: 1.5; }

</style>
