<template>
  <div class="dashboard-page">
    <div class="scroll-area">
      <!-- 顶部状态卡片区 -->
      <div style="display: flex; gap: 24px; margin-bottom: 20px;">
        <div v-for="cli in cliList" :key="cli.type" class="b-card" style="flex: 1; margin-bottom: 0;">
          <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
            <div style="display: flex; align-items: center; gap: 12px;">
              <div class="status-dot" :class="{ running: getCliEnabled(cli.type) }"></div>
              <div class="cli-title">
                {{ cli.label }}
                <span v-if="!getCliEnabled(cli.type)" class="cli-disabled">(已禁用)</span>
              </div>
            </div>
            <el-switch :model-value="getCliEnabled(cli.type)" @change="(val: string | number | boolean) => handleCliToggle(cli.type, val as boolean)" :loading="cliLoading[cli.type]" />
          </div>
          
          <div class="b-segmented" style="width: 100%;">
            <div class="b-seg-btn" :class="{ active: getCliMode(cli.type) === 'proxy' }" @click="handleModeSwitch(cli.type, 'proxy')" style="flex: 1;">中转模式</div>
            <div class="b-seg-btn" :class="{ active: getCliMode(cli.type) === 'direct' }" @click="handleModeSwitch(cli.type, 'direct')" style="flex: 1;">官方模式</div>
          </div>
        </div>
      </div>

      <!-- 中部关键指标 KPI -->
      <div style="display: flex; gap: 24px; margin-bottom: 20px;">
        <div class="b-card kpi-card">
          <div class="kpi-title">请求总数</div>
          <div class="kpi-value mono text-blue">{{ kpiData.requests }}</div>
        </div>
        <div class="b-card kpi-card">
          <div class="kpi-title">全局成功率</div>
          <div class="kpi-value mono text-green">{{ kpiData.successRate }}</div>
        </div>
        <div class="b-card kpi-card">
          <div class="kpi-title">Token消耗</div>
          <div class="kpi-value mono">{{ kpiData.tokens }}</div>
        </div>
        <div class="b-card kpi-card">
          <div class="kpi-title">缓存Token</div>
          <div class="kpi-value mono">{{ kpiData.cachedTokens }}</div>
        </div>
      </div>

      <!-- 底部图表与明细 -->
      <div style="display: flex; gap: 24px; flex-wrap: wrap;">
        <!-- 核心图表分析区 -->
        <div class="b-card responsive-bottom-card" style="flex: 1; margin-bottom: 0; padding: 24px; min-width: 400px;">
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; flex-wrap: wrap; gap: 16px;">
          <div class="b-card-title" style="margin-bottom: 0;">请求统计</div>
          <div style="display: flex; gap: 16px; align-items: center; flex-wrap: wrap;">
            <div class="b-segmented">
              <div class="b-seg-btn" :class="{ active: metricMode === 'requests' }" @click="metricMode = 'requests'">请求</div>
              <div class="b-seg-btn" :class="{ active: metricMode === 'tokens' }" @click="metricMode = 'tokens'">Token</div>
            </div>
            <div class="b-segmented">
              <div class="b-seg-btn" :class="{ active: dimMode === 'provider' }" @click="dimMode = 'provider'">服务商</div>
              <div class="b-seg-btn" :class="{ active: dimMode === 'model' }" @click="dimMode = 'model'">模型</div>
            </div>
          </div>
        </div>
        <div style="height: 260px; width: 100%;">
          <v-chart class="chart" :option="chartOption" autoresize />
        </div>
      </div>

      <!-- 多维数据明细表 -->
      <div class="b-card responsive-bottom-card" style="flex: 1; margin-bottom: 0; padding: 24px; min-width: 400px; display: flex; flex-direction: column;">
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; flex-wrap: wrap; gap: 16px;">
          <div class="b-card-title" style="margin-bottom: 0;">请求明细</div>
          <div style="display: flex; gap: 12px; flex-wrap: wrap;">
            <svg style="display: none;">
              <symbol id="icon-chevron" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="m6 9 6 6 6-6"/>
              </symbol>
            </svg>
            <div class="custom-select" :class="{ open: providerSelectOpen }" @click.stop="toggleSelect('provider')">
              <div class="custom-select-trigger">{{ filterProvider === 'all' ? '所有服务商' : filterProvider }}</div>
              <svg class="chevron" width="16" height="16"><use href="#icon-chevron"/></svg>
              <div class="custom-select-options">
                <div class="custom-option" :class="{ selected: filterProvider === 'all' }" @click.stop="filterProvider = 'all'; providerSelectOpen = false">所有服务商</div>
                <div v-for="p in uniqueProviders" :key="p" class="custom-option" :class="{ selected: filterProvider === p }" @click.stop="filterProvider = p; providerSelectOpen = false">
                  {{ p }}
                </div>
              </div>
            </div>
            <div class="custom-select" :class="{ open: modelSelectOpen }" @click.stop="toggleSelect('model')">
              <div class="custom-select-trigger">{{ filterModel === 'all' ? '所有模型' : filterModel }}</div>
              <svg class="chevron" width="16" height="16"><use href="#icon-chevron"/></svg>
              <div class="custom-select-options">
                <div class="custom-option" :class="{ selected: filterModel === 'all' }" @click.stop="filterModel = 'all'; modelSelectOpen = false">所有模型</div>
                <div v-for="m in uniqueModels" :key="m" class="custom-option" :class="{ selected: filterModel === m }" @click.stop="filterModel = m; modelSelectOpen = false">
                  {{ m }}
                </div>
              </div>
            </div>
          </div>
        </div>
        <div class="stats-table-wrapper" style="height: 260px;">
          <table class="flat-table">
            <thead>
                <tr>
                  <th style="min-width: 100px;">日期</th>
                  <th style="min-width: 100px;">服务商</th>
                  <th style="min-width: 100px;">模型</th>
                  <th style="min-width: 60px;">请求</th>
                  <th style="min-width: 60px;">Token</th>
                </tr>
              </thead>
            <tbody>
              <tr v-for="row in filteredTableData" :key="`${row.date}-${row.provider_name}-${row.model_id}`">
                <td class="table-cell">{{ row.date }}</td>
                <td class="table-cell">{{ row.provider_name }}</td>
                <td class="table-cell">{{ row.model_id }}</td>
                <td class="table-cell mono">{{ row.total_requests }}</td>
                <td class="table-cell mono">{{ formatTokens(row.total_tokens) }}</td>
              </tr>
              <tr v-if="filteredTableData.length === 0">
                <td colspan="5" style="text-align: center; color: var(--color-text-weak); padding: 24px;">暂无数据</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref, reactive, computed } from 'vue'
import { confirm } from '@/utils/confirm'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'

import { use } from 'echarts/core'
import { LineChart, BarChart } from 'echarts/charts'
import { TooltipComponent, GridComponent, DatasetComponent, TransformComponent, LegendComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import VChart from 'vue-echarts'

use([LineChart, BarChart, TooltipComponent, GridComponent, DatasetComponent, TransformComponent, LegendComponent, CanvasRenderer])

import { useProviderStore } from '@/stores/providers'
import { useSettingsStore } from '@/stores/settings'
import { statsApi } from '@/api/stats'
import { formatTokens } from '@/utils/json'
import { useAutoRefresh } from '@/composables/useAutoRefresh'
import type { ProviderStats, DailyStats, AdvancedStatsRow } from '@/types/models'

const providerStore = useProviderStore()
const settingsStore = useSettingsStore()
const DASHBOARD_REFRESH_INTERVAL_MS = 5_000

const cliList = [
  { type: 'claude_code', label: 'Claude Code' },
  { type: 'codex', label: 'Codex' },
  { type: 'gemini', label: 'Gemini' }
]

const cliLoading = reactive<Record<string, boolean>>({
  claude_code: false,
  codex: false,
  gemini: false
})

const providerStats = ref<ProviderStats[]>([])
const dailyStats = ref<DailyStats[]>([])
const advancedStats = ref<AdvancedStatsRow[]>([])

// UI State
const metricMode = ref<'requests' | 'tokens'>('requests')
const dimMode = ref<'provider' | 'model'>('provider')
const filterProvider = ref<string>('all')
const filterModel = ref<string>('all')

const providerSelectOpen = ref(false)
const modelSelectOpen = ref(false)

function closeAllSelects() {
  providerSelectOpen.value = false
  modelSelectOpen.value = false
}

function toggleSelect(type: string) {
  const isProv = type === 'provider' && !providerSelectOpen.value
  const isModel = type === 'model' && !modelSelectOpen.value

  closeAllSelects()

  if (isProv) providerSelectOpen.value = true
  if (isModel) modelSelectOpen.value = true
}

const kpiData = computed(() => {
  const stats = providerStats.value
  const totalRequests = stats.reduce((sum, s) => sum + s.total_requests, 0)
  const totalSuccess = stats.reduce((sum, s) => sum + s.total_success, 0)
  const totalTokens = stats.reduce((sum, s) => sum + s.total_tokens, 0)
  const totalCachedTokens = stats.reduce((sum, s) => sum + (s.total_cache_read_tokens || 0) + (s.total_cache_creation_tokens || 0), 0)
  const successRate = totalRequests > 0 ? (totalSuccess / totalRequests) * 100 : 0

  return {
    requests: totalRequests.toLocaleString(),
    successRate: totalRequests > 0 ? successRate.toFixed(1) + '%' : '0%',
    tokens: formatTokens(totalTokens),
    cachedTokens: formatTokens(totalCachedTokens)
  }
})

function getCliEnabled(cliType: string): boolean {
  const settings = settingsStore.settings?.cli_settings?.[cliType]
  if (!settings) return false
  if (settings.cli_mode === 'direct') return false
  return settings.enabled ?? false
}

function getCliMode(cliType: string): 'proxy' | 'direct' {
  return settingsStore.settings?.cli_settings?.[cliType]?.cli_mode ?? 'proxy'
}

async function handleModeSwitch(cliType: string, targetMode: 'proxy' | 'direct') {
  if (getCliMode(cliType) === targetMode) return
  if (cliType === 'claude_code' && targetMode === 'direct') {
    notify('Claude Code 暂不支持官方模式', 'warning')
    return
  }
  cliLoading[cliType] = true
  try {
    await settingsStore.setCliMode(cliType, targetMode)
    notify(`${cliType} 已切换至 ${targetMode === 'proxy' ? '中转模式' : '官方模式'}`)
  } catch (e: any) {
    notify(`切换失败: ${getErrorMessage(e)}`, 'error')
  } finally {
    cliLoading[cliType] = false
  }
}

async function handleCliToggle(cliType: string, enabled: boolean) {
  if (enabled && getCliMode(cliType) === 'direct') {
    try {
      await confirm('当前是官方模式，是否切换至中转模式并启用代理？', '提示', {
        confirmText: '切换并启用', cancelText: '取消'
      })
      cliLoading[cliType] = true
      try {
        await settingsStore.setCliMode(cliType, 'proxy')
        await settingsStore.updateCli(cliType, { enabled: true })
        notify(`${cliType} 已切换至中转模式并启用`)
      } catch (e: any) { notify(`操作失败: ${getErrorMessage(e)}`, 'error') }
      finally { cliLoading[cliType] = false }
    } catch { notify('操作已取消', 'info') }
  } else {
    cliLoading[cliType] = true
    try {
      await settingsStore.updateCli(cliType, { enabled })
      notify(`${cliType} 已${enabled ? '启用' : '禁用'}`)
    } catch (e: any) { notify(`操作失败: ${getErrorMessage(e)}`, 'error') }
    finally { cliLoading[cliType] = false }
  }
}

function formatLocalDate(d: Date): string {
  const year = d.getFullYear()
  const month = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

async function fetchStats() {
  const today = new Date()
  const sevenDaysAgo = new Date(today)
  sevenDaysAgo.setDate(today.getDate() - 6)
  
  const p1 = statsApi.getProviders({})
  const p2 = statsApi.getDaily({ start_date: formatLocalDate(sevenDaysAgo), end_date: formatLocalDate(today) })
  const p3 = statsApi.getAdvanced({})
  
  const [resProv, resDaily, resAdv] = await Promise.all([p1, p2, p3])
  providerStats.value = resProv.data
  dailyStats.value = resDaily.data
  advancedStats.value = resAdv.data
}

useAutoRefresh(async () => {
  await fetchStats()
}, {
  intervalMs: DASHBOARD_REFRESH_INTERVAL_MS,
  immediate: true,
  onError: (e) => notify(getErrorMessage(e, '数据刷新失败'), 'error')
})

// === Chart Logic ===
const PALETTE = ['#0ea5e9', '#8b5cf6', '#10b981', '#f59e0b', '#ef4444', '#ec4899', '#14b8a6', '#6366f1']
const BAR_RADIUS = 4

const chartOption = computed(() => {
  const dates: string[] = []
  for (let i = 6; i >= 0; i--) {
    const d = new Date()
    d.setDate(d.getDate() - i)
    dates.push(formatLocalDate(d))
  }

  const metricKey = metricMode.value === 'requests' ? 'total_requests' : 'total_tokens'

  // 堆叠柱状图 (按服务商或模型，固定前5)
  const groupKey = dimMode.value === 'provider' ? 'provider_name' : 'model_id'
  const groupTotals = new Map<string, number>()
  advancedStats.value.forEach(s => {
    groupTotals.set(s[groupKey], (groupTotals.get(s[groupKey]) || 0) + s[metricKey])
  })
  const groupArray = Array.from(groupTotals.entries())
    .sort((a, b) => b[1] - a[1])
    .slice(0, 5)
    .map(([name]) => name)

  const rawSeriesData = groupArray.map((gName, idx) => {
    const data = dates.map(d => {
      let sum = 0
      advancedStats.value.forEach(s => {
        if (s.date === d && s[groupKey] === gName) sum += s[metricKey]
      })
      return sum
    })
    
    const color = PALETTE[idx % PALETTE.length]
    return {
      name: gName,
      color,
      data
    }
  })

  const topSeriesByDate = dates.map((_, dateIdx) => {
    for (let seriesIdx = rawSeriesData.length - 1; seriesIdx >= 0; seriesIdx--) {
      if (rawSeriesData[seriesIdx].data[dateIdx] > 0) return seriesIdx
    }
    return -1
  })

  const seriesData: any[] = rawSeriesData.map((series, seriesIdx) => ({
    name: series.name,
    type: 'bar',
    stack: 'total',
    barWidth: '60%',
    barGap: '10%',
    itemStyle: { color: series.color },
    data: series.data.map((value, dateIdx) => {
      if (value <= 0 || topSeriesByDate[dateIdx] !== seriesIdx) return value
      return { value, itemStyle: { borderRadius: [BAR_RADIUS, BAR_RADIUS, 0, 0] } }
    })
  }))

  return {
    tooltip: { 
      trigger: 'axis', 
      axisPointer: { type: 'shadow' }, 
      valueFormatter: (value: any) => metricMode.value === 'tokens' ? formatTokens(value) : value,
      backgroundColor: 'rgba(255, 255, 255, 0.9)', 
      borderColor: '#e2e8f0', 
      textStyle: { color: '#0f172a' } 
    },
    legend: { bottom: 0, left: 'center', icon: 'circle', textStyle: { color: '#64748b' } },
    grid: { top: 20, right: '3%', bottom: 40, left: '3%', containLabel: true },
    xAxis: { type: 'category', data: dates, axisLine: { lineStyle: { color: '#e2e8f0' } }, axisLabel: { color: '#64748b' } },
    yAxis: {
      type: 'value',
      splitNumber: 4,
      splitLine: { lineStyle: { type: 'dashed', color: '#f1f5f9' } }, 
      axisLabel: { 
        color: '#64748b',
        formatter: (value: number) => {
          if (value >= 1000000) return (value / 1000000).toFixed(1) + 'M'
          if (value >= 1000) return (value / 1000).toFixed(1) + 'K'
          return value
        }
      } 
    },
    series: seriesData
  }
})

// === Table Logic ===
const uniqueProviders = computed(() => {
  const p = new Set<string>()
  advancedStats.value.forEach(s => p.add(s.provider_name))
  return Array.from(p).sort()
})

const uniqueModels = computed(() => {
  const m = new Set<string>()
  advancedStats.value.forEach(s => m.add(s.model_id))
  return Array.from(m).sort()
})

const filteredTableData = computed(() => {
  let result = advancedStats.value

  if (filterProvider.value !== 'all') {
    result = result.filter(r => r.provider_name === filterProvider.value)
  }

  if (filterModel.value !== 'all') {
    result = result.filter(r => r.model_id === filterModel.value)
  }

  return result
})

onMounted(() => {
  document.addEventListener('click', closeAllSelects)
  void providerStore.fetchProviders()
  void settingsStore.fetchSettings()
})

onUnmounted(() => {
  document.removeEventListener('click', closeAllSelects)
})
</script>

<style scoped>
.dashboard-page {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.scroll-area {
  flex: 1;
  overflow-y: auto;
  padding: 0 40px 16px 40px;
}

.b-card { background: var(--color-bg); border-radius: 16px; box-shadow: 0 4px 12px var(--color-shadow); padding: 24px; margin-bottom: 24px; border: 1px solid transparent; }
.b-card-title { font-size: var(--fs-16); font-weight: var(--fw-600); margin-bottom: 20px; color: var(--color-text); }

.status-dot { width: 10px; height: 10px; border-radius: 50%; background: var(--color-text-weak); }
.status-dot.running { background: var(--color-success); box-shadow: 0 0 8px var(--color-success-40); }

.cli-title { font-size: var(--fs-16); font-weight: var(--fw-700); color: var(--color-text); }
.cli-disabled { font-size: var(--fs-14); font-weight: var(--fw-400); color: var(--color-text-weak); margin-left: 4px; }

.b-segmented { display: inline-flex; background: var(--color-border); padding: 4px; border-radius: 10px; }
.b-seg-btn { text-align: center; padding: 6px 16px; font-size: var(--fs-14); color: var(--color-text-muted); border-radius: 8px; font-weight: var(--fw-500); transition: all 0.2s ease; cursor: pointer; }
.b-seg-btn.active { background: var(--color-bg); color: var(--color-primary); box-shadow: 0 1px 3px var(--color-shadow-lg); pointer-events: none; }

.kpi-card { flex: 1; padding: 24px 20px !important; margin-bottom: 0 !important; text-align: center; display: flex; flex-direction: column; justify-content: center; }
.kpi-title { font-size: var(--fs-14); font-weight: var(--fw-500); color: var(--color-text-muted); margin-bottom: 12px; }
.kpi-value { font-size: var(--fs-32); font-weight: var(--fw-700); letter-spacing: -1px; }

.text-blue { color: var(--color-primary); }
.text-green { color: var(--color-success); }
.table-cell { font-size: var(--fs-14); color: var(--color-text); }

.chart { width: 100%; height: 100%; }

/* Stats Table Wrapper */
.stats-table-wrapper { overflow-y: auto; }

.custom-select { position: relative; width: 160px; }
.custom-select-trigger { padding: 9px 36px 9px 16px; border: 1px solid var(--color-border); border-radius: 8px; font-size: var(--fs-14); font-weight: var(--fw-400); color: var(--color-text); background: color-mix(in srgb, var(--color-bg) 80%, transparent); box-shadow: 0 1px 3px var(--color-shadow); cursor: pointer; transition: all 0.2s; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; user-select: none; }
.custom-select:hover .custom-select-trigger { border-color: var(--color-border-hover); background: var(--color-bg); }
.custom-select.open .custom-select-trigger { border-color: var(--color-primary); box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-primary) 10%, transparent); background: var(--color-bg); }
.custom-select .chevron { position: absolute; right: 12px; top: 50%; transform: translateY(-50%); color: var(--color-text-muted); pointer-events: none; transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1); }
.custom-select.open .chevron { transform: translateY(-50%) rotate(180deg); color: var(--color-primary); }
.custom-select-options { position: absolute; top: calc(100% + 6px); left: 0; right: auto; background: var(--color-bg); border: 1px solid var(--color-border); border-radius: 12px; box-shadow: 0 10px 40px -10px var(--color-shadow-lg); padding: 4px; z-index: 50; opacity: 0; transform: translateY(-5px); pointer-events: none; transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1); min-width: 100%; max-height: 250px; overflow-y: auto; }
.custom-select.open .custom-select-options { opacity: 1; transform: translateY(0); pointer-events: auto; }
.custom-option { padding: 10px 12px; border-radius: 8px; font-size: var(--fs-14); color: var(--color-text-secondary); cursor: pointer; transition: all 0.1s; display: flex; align-items: center; margin-bottom: 2px; }
.custom-option:hover { background: var(--color-bg-subtle); color: var(--color-text); }
.custom-option.selected { font-weight: var(--fw-600); color: var(--color-primary); background: var(--color-primary-light); }

/* Flat Table */
.flat-table { width: max-content; min-width: 100%; border-collapse: separate; border-spacing: 0; text-align: center; }
.flat-table th, .flat-table td { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; box-sizing: border-box; text-align: center; }
.flat-table th { padding: 12px 20px; font-size: var(--fs-12); font-weight: var(--fw-600); color: var(--color-text-muted); text-transform: uppercase; background: var(--color-bg-page); border-bottom: 1px solid var(--color-border); position: sticky; top: 0; z-index: 10; }
.flat-table td { padding: 12px 20px; font-size: var(--fs-14); color: var(--color-text); border-bottom: 1px solid var(--color-bg-subtle); }
.flat-table tr:last-child td { border-bottom: none; }
.flat-table tr:hover td { background: var(--color-bg-page); }

</style>
