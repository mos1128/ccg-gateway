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
              </div>
            </div>
            <div class="cli-switch-slot">
              <el-switch
                v-if="getCliMode(cli.type) !== 'official_direct'"
                :model-value="isRouteMode(cli.type)"
                @change="(val: string | number | boolean) => handleCliToggle(cli.type, val as boolean)"
                :loading="cliLoading[cli.type]"
                title="开：中转路由；关：中转直连"
              />
            </div>
          </div>
          
          <div class="b-segmented b-segmented-fill">
            <div class="b-seg-btn" :class="{ active: getCliMode(cli.type) !== 'official_direct' }" @click="handleModeSwitch(cli.type, 'proxy_route')">中转路由</div>
            <div class="b-seg-btn" :class="{ active: getCliMode(cli.type) === 'official_direct' }" @click="handleModeSwitch(cli.type, 'official_direct')">官方直连</div>
          </div>
        </div>
      </div>

      <!-- 中部关键指标 KPI -->
      <div style="display: flex; gap: 24px; margin-bottom: 20px;">
        <div class="b-card kpi-card">
          <div class="kpi-title">计费Token</div>
          <div class="kpi-value mono text-blue">{{ kpiData.billableTokens }}</div>
        </div>
        <div class="b-card kpi-card">
          <div class="kpi-title">缓存Token</div>
          <div class="kpi-value mono">{{ kpiData.cacheTokens }}</div>
        </div>
        <div class="b-card kpi-card">
          <div class="kpi-title">请求总数</div>
          <div class="kpi-value mono">{{ kpiData.requests }}</div>
        </div>
        <div class="b-card kpi-card">
          <div class="kpi-title">全局成功率</div>
          <div class="kpi-value mono text-green">{{ kpiData.successRate }}</div>
        </div>
        <div class="b-card kpi-card">
          <div class="kpi-title">费用</div>
          <div class="kpi-value mono">{{ kpiData.cost }}</div>
        </div>
      </div>

      <!-- 底部图表与明细 -->
      <div style="display: flex; gap: 24px; flex-direction: column;">
        <div class="b-card responsive-bottom-card" style="width: 100%; margin-bottom: 0; padding: 24px;">
          <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; flex-wrap: wrap; gap: 16px;">
            <div class="b-card-title" style="margin-bottom: 0;">统计总览</div>
            <div style="display: flex; gap: 16px; align-items: center; flex-wrap: wrap;">
              <div class="b-segmented">
                <div class="b-seg-btn" :class="{ active: metricMode === 'tokens' }" @click="metricMode = 'tokens'">Token</div>
                <div class="b-seg-btn" :class="{ active: metricMode === 'cost' }" @click="metricMode = 'cost'">费用</div>
                <div class="b-seg-btn" :class="{ active: metricMode === 'requests' }" @click="metricMode = 'requests'">请求数</div>
              </div>
              <div class="b-segmented">
                <div class="b-seg-btn" :class="{ active: dimMode === 'provider' }" @click="dimMode = 'provider'">服务商</div>
                <div class="b-seg-btn" :class="{ active: dimMode === 'model' }" @click="dimMode = 'model'">模型</div>
              </div>
            </div>
          </div>
          <div style="height: 260px; width: 100%;" @mouseenter="handleChartMouseEnter" @mouseleave="handleChartMouseLeave">
            <v-chart class="chart" :option="chartOption" autoresize @legendselectchanged="handleLegendSelectChanged" />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref, reactive, computed } from 'vue'
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
import { formatCost, formatTokens } from '@/utils/json'
import { useAutoRefresh } from '@/composables/useAutoRefresh'
import { CLI_TABS } from '@/types/models'
import type { CliType, CliMode, ProviderStats, AdvancedStatsRow } from '@/types/models'

const providerStore = useProviderStore()
const settingsStore = useSettingsStore()
const DASHBOARD_REFRESH_INTERVAL_MS = 5_000

const cliList = CLI_TABS.map(({ id, label }) => ({ type: id, label }))

const cliLoading = reactive<Record<CliType, boolean>>({
  claude_code: false,
  codex: false,
  gemini: false
})

const providerStats = ref<ProviderStats[]>([])
const advancedStats = ref<AdvancedStatsRow[]>([])
const pendingAdvancedStats = ref<AdvancedStatsRow[] | null>(null)
const chartHovering = ref(false)
const legendSelectedMap = ref<Record<string, Record<string, boolean>>>({})

// UI State
const metricMode = ref<'requests' | 'tokens' | 'cost'>('tokens')
const dimMode = ref<'provider' | 'model'>('provider')
const legendStateKey = computed(() => dimMode.value)

const kpiData = computed(() => {
  const stats = providerStats.value
  const totalRequests = stats.reduce((sum, s) => sum + s.total_requests, 0)
  const totalSuccess = stats.reduce((sum, s) => sum + s.total_success, 0)
  const totalCacheReadTokens = stats.reduce((sum, s) => sum + (s.total_cache_read_tokens || 0), 0)
  const totalCacheCreationTokens = stats.reduce((sum, s) => sum + (s.total_cache_creation_tokens || 0), 0)
  const totalCacheTokens = totalCacheReadTokens + totalCacheCreationTokens
  const totalBillableTokens = stats.reduce((sum, s) => sum + s.total_tokens, 0) - totalCacheTokens
  const totalCost = stats.reduce((sum, s) => sum + (s.total_cost || 0), 0)
  const successRate = totalRequests > 0 ? (totalSuccess / totalRequests) * 100 : 0

  return {
    billableTokens: formatTokens(totalBillableTokens),
    cacheTokens: formatTokens(totalCacheTokens),
    requests: totalRequests.toLocaleString(),
    successRate: totalRequests > 0 ? successRate.toFixed(1) + '%' : '0%',
    cost: formatCost(totalCost)
  }
})

function getCliEnabled(cliType: CliType): boolean {
  return isRouteMode(cliType)
}

function isRouteMode(cliType: CliType): boolean {
  return getCliMode(cliType) === 'proxy_route'
}

function getCliMode(cliType: CliType): CliMode {
  return settingsStore.settings?.cli_settings?.[cliType]?.cli_mode ?? 'proxy_route'
}

const cliModeLabels: Record<CliMode, string> = {
  proxy_route: '中转路由',
  provider_direct: '中转直连',
  official_direct: '官方直连'
}

async function handleModeSwitch(cliType: CliType, targetMode: Extract<CliMode, 'proxy_route' | 'official_direct'>) {
  if (getCliMode(cliType) === targetMode) return
  if (cliType === 'claude_code' && targetMode === 'official_direct') {
    notify('Claude Code 暂不支持官方直连', 'warning')
    return
  }
  cliLoading[cliType] = true
  try {
    await settingsStore.setDashboardCliMode(cliType, targetMode)
    notify(`${cliType} 已切换至 ${cliModeLabels[targetMode]}`)
  } catch (e: any) {
    notify(`切换失败: ${getErrorMessage(e)}`, 'error')
  } finally {
    cliLoading[cliType] = false
  }
}

async function handleCliToggle(cliType: CliType, enabled: boolean) {
  const targetMode: CliMode = enabled ? 'proxy_route' : 'provider_direct'
  if (getCliMode(cliType) === targetMode) return
  cliLoading[cliType] = true
  try {
    await settingsStore.setDashboardCliMode(cliType, targetMode)
    notify(`${cliType} 已切换至 ${cliModeLabels[targetMode]}`)
  } catch (e: any) {
    notify(`操作失败: ${getErrorMessage(e)}`, 'error')
  } finally {
    cliLoading[cliType] = false
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
  const startDate = formatLocalDate(sevenDaysAgo)
  const endDate = formatLocalDate(today)

  // KPI 数据先加载
  const resProv = await statsApi.getProviders({})
  providerStats.value = resProv.data

  // 图表和明细数据后加载
  const resAdv = await statsApi.getAdvanced({ start_date: startDate, end_date: endDate })
  applyAdvancedStats(resAdv.data)
}

function applyAdvancedStats(rows: AdvancedStatsRow[]) {
  if (chartHovering.value) {
    pendingAdvancedStats.value = rows
    return
  }
  advancedStats.value = rows
}

function handleChartMouseEnter() {
  chartHovering.value = true
}

function handleChartMouseLeave() {
  chartHovering.value = false
  if (!pendingAdvancedStats.value) return
  advancedStats.value = pendingAdvancedStats.value
  pendingAdvancedStats.value = null
}

function handleLegendSelectChanged(event: { selected?: Record<string, boolean> }) {
  if (!event.selected) return
  legendSelectedMap.value = {
    ...legendSelectedMap.value,
    [legendStateKey.value]: { ...event.selected }
  }
}

useAutoRefresh(async () => {
  await fetchStats()
}, {
  intervalMs: DASHBOARD_REFRESH_INTERVAL_MS,
  immediate: true,
  onError: (e) => notify(getErrorMessage(e, '数据刷新失败'), 'error')
})

// === Chart Logic ===
const PALETTE = ['#5470c6', '#91cc75', '#fac858', '#ee6666', '#73c0de', '#3ba272', '#fc8452', '#9a60b4', '#ea7ccc', '#00b4d8', '#f472b6', '#fbbf24']
const BAR_RADIUS = 4

function formatTokenValue(value: number): string {
  if (value >= 1000000) return (value / 1000000).toFixed(1) + 'M'
  if (value >= 1000) return (value / 1000).toFixed(1) + 'K'
  return value.toString()
}

function formatMetricValue(value: number): string {
  if (metricMode.value === 'cost') return formatCost(value)
  return formatTokenValue(value)
}

const chartOption = computed(() => {
  const dates: string[] = []
  for (let i = 6; i >= 0; i--) {
    const d = new Date()
    d.setDate(d.getDate() - i)
    dates.push(formatLocalDate(d))
  }

  const isTokens = metricMode.value === 'tokens'
  const isCost = metricMode.value === 'cost'
  const isBar = isTokens || isCost
  const groupKey = dimMode.value === 'provider' ? 'provider_name' : 'model_id'

  // Get unique groups and total for sorting
  const groupTotals = new Map<string, number>()
  advancedStats.value.forEach(s => {
    const val = isCost ? s.total_cost : isTokens ? s.total_tokens : s.total_success
    groupTotals.set(s[groupKey], (groupTotals.get(s[groupKey]) || 0) + val)
  })
  const groupArray = Array.from(groupTotals.entries())
    .sort((a, b) => b[1] - a[1])
    .map(([name]) => name)

  const seriesData: any[] = groupArray.map((gName, idx) => {
    const color = PALETTE[idx % PALETTE.length]

    if (isBar) {
      const data = dates.map(d => {
        let sum = 0, input = 0, output = 0, cacheRead = 0, cacheCreation = 0, cost = 0
        advancedStats.value.forEach(s => {
          if (s.date === d && s[groupKey] === gName) {
            sum += isCost ? s.total_cost : s.total_tokens
            input += s.total_input_tokens || 0
            output += s.total_output_tokens || 0
            cacheRead += s.total_cache_read_tokens || 0
            cacheCreation += s.total_cache_creation_tokens || 0
            cost += s.total_cost || 0
          }
        })
        return { value: sum, input, output, cacheRead, cacheCreation, cost, name: gName }
      })
      return { name: gName, type: 'bar', stack: 'total', barWidth: '60%', itemStyle: { color }, data }
    } else {
      const data = dates.map(d => {
        let sum = 0
        advancedStats.value.forEach(s => {
          if (s.date === d && s[groupKey] === gName) {
            sum += s.total_success
          }
        })
        return sum
      })
      return {
        name: gName, type: 'line', smooth: true, showSymbol: false, itemStyle: { color },
        areaStyle: {
          color: { type: 'linear', x: 0, y: 0, x2: 0, y2: 1, colorStops: [{ offset: 0, color }, { offset: 1, color: 'transparent' }] },
          opacity: 0.2
        },
        data
      }
    }
  })

  // Apply border radius for bar chart
  if (isBar) {
    const topSeriesByDate = dates.map((_, dateIdx) => {
      for (let seriesIdx = seriesData.length - 1; seriesIdx >= 0; seriesIdx--) {
        if (seriesData[seriesIdx].data[dateIdx].value > 0) return seriesIdx
      }
      return -1
    })
    seriesData.forEach((series, seriesIdx) => {
      series.data = series.data.map((item: any, dateIdx: number) => {
        if (item.value <= 0 || topSeriesByDate[dateIdx] !== seriesIdx) return item
        return { ...item, itemStyle: { borderRadius: [BAR_RADIUS, BAR_RADIUS, 0, 0] } }
      })
    })
  }

  return {
    tooltip: {
      trigger: 'axis',
      appendTo: 'body',
      transitionDuration: 0,
      extraCssText: 'position: fixed;',
      axisPointer: { type: isBar ? 'shadow' : 'line' },
      backgroundColor: 'rgba(255, 255, 255, 0.95)',
      borderColor: '#e2e8f0',
      textStyle: { color: '#0f172a' },
      formatter: (params: any[]) => {
        if (!params.length) return ''
        const visibleParams = params.filter(p => Number(isBar ? p.data?.value : p.value) > 0)
        if (!visibleParams.length) return ''
        const date = params[0].name
        let html = `<div style="font-weight: 600; margin-bottom: 8px;">${date}</div>`

        if (isBar) {
          visibleParams.forEach(p => {
            const d = p.data
            const costLine = isCost ? `<div>- 费用: ${formatCost(d.cost)}</div>` : ''
            html += `<div style="margin-bottom: 6px;">
              <div style="display: flex; align-items: center; gap: 6px; font-weight: 600;">
                <span style="display:inline-block;width:10px;height:10px;border-radius:50%;background-color:${p.color};"></span>
                ${d.name} (总计: ${formatMetricValue(d.value)})
              </div>
              <div style="padding-left: 16px; color: #64748b; font-size: 13px;">
                <div>- 输入: ${formatTokenValue(d.input)}</div>
                <div>- 输出: ${formatTokenValue(d.output)}</div>
                <div>- 缓存读取: ${formatTokenValue(d.cacheRead)}</div>
                <div>- 缓存创建: ${formatTokenValue(d.cacheCreation)}</div>
                ${costLine}
              </div>
            </div>`
          })
        } else {
          visibleParams.forEach(p => {
            html += `<div style="margin-bottom: 4px; display: flex; align-items: center; gap: 6px;">
              <span style="display:inline-block;width:10px;height:10px;border-radius:50%;background-color:${p.color};"></span>
              <span style="font-weight: 500;">${p.seriesName}:</span>
              <span>${p.value}</span>
            </div>`
          })
        }
        return html
      }
    },
    legend: {
      bottom: 0,
      left: 'center',
      type: 'scroll',
      icon: 'circle',
      selected: legendSelectedMap.value[legendStateKey.value],
      textStyle: { color: '#64748b' }
    },
    grid: { top: 20, right: '4%', bottom: 40, left: '3%', containLabel: true },
    xAxis: { 
      type: 'category', 
      data: dates, 
      boundaryGap: isBar, 
      axisLine: { lineStyle: { color: '#e2e8f0' } }, 
      axisLabel: { 
        color: '#64748b',
        formatter: (value: string) => value.substring(5)
      } 
    },
    yAxis: {
      type: 'value',
      splitNumber: 4,
      splitLine: { lineStyle: { type: 'dashed', color: '#f1f5f9' } },
      axisLabel: {
        color: '#64748b',
        formatter: (value: number) => {
          if (isCost) return formatCost(value)
          if (value >= 1000000) return (value / 1000000).toFixed(1) + 'M'
          if (value >= 1000) return (value / 1000).toFixed(1) + 'K'
          return value
        }
      }
    },
    series: seriesData
  }
})

onMounted(() => {
  void providerStore.fetchProviders()
  void settingsStore.fetchSettings()
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
  padding: 8px 40px;
}

.b-card { background: var(--color-bg); border-radius: 16px; box-shadow: 0 4px 12px var(--color-shadow); padding: 24px; margin-bottom: 24px; border: 1px solid transparent; }
.b-card-title { font-size: var(--fs-16); font-weight: var(--fw-600); margin-bottom: 20px; color: var(--color-text); }

.status-dot { width: 10px; height: 10px; border-radius: 50%; background: var(--color-text-weak); }
.status-dot.running { background: var(--color-success); box-shadow: 0 0 8px var(--color-success-40); }

.cli-title { font-size: var(--fs-16); font-weight: var(--fw-700); color: var(--color-text); }
.cli-disabled { font-size: var(--fs-14); font-weight: var(--fw-400); color: var(--color-text-weak); margin-left: 4px; }
.cli-switch-slot { width: 50px; min-height: 32px; display: flex; align-items: center; justify-content: flex-end; flex-shrink: 0; }


.kpi-card { flex: 1; padding: 24px 20px !important; margin-bottom: 0 !important; text-align: center; display: flex; flex-direction: column; justify-content: center; }
.kpi-title { font-size: var(--fs-14); font-weight: var(--fw-500); color: var(--color-text-muted); margin-bottom: 12px; }
.kpi-value { font-size: var(--fs-32); font-weight: var(--fw-700); letter-spacing: -1px; }

.text-blue { color: var(--color-primary); }
.text-green { color: var(--color-success); }
.table-cell { font-size: var(--fs-14); color: var(--color-text); }

.chart { width: 100%; height: 100%; }

/* Stats Table Wrapper */
.stats-table-wrapper { overflow-y: auto; }

</style>
