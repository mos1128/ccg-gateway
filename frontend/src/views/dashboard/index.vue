<template>
  <div>
    <div class="dash-body">
      <!-- 顶部：KPI 全宽 -->
      <div class="v2-kpi-grid">
        <div v-for="k in kpis" :key="k.label" class="v2-kpi" :style="{ '--kpi-accent': k.borderColor || 'var(--v2-surface-3)' }">
          <div class="kpi-header">
            <span class="v2-kpi-label">{{ k.label }}</span>
            <span class="kpi-icon-badge" :style="{ color: k.borderColor || 'var(--v2-text)' }">
              <svg v-if="k.icon === 'token'" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"></path></svg>
              <svg v-else-if="k.icon === 'cache'" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon></svg>
              <svg v-else-if="k.icon === 'request'" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12"></polyline></svg>
              <svg v-else-if="k.icon === 'success'" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg>
              <svg v-else-if="k.icon === 'cost'" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="1" x2="12" y2="23"></line><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"></path></svg>
            </span>
          </div>
          <div class="v2-kpi-value mono" :style="k.color ? { color: k.color } : undefined">{{ k.value }}</div>
        </div>
      </div>

      <!-- 下方：left CLI client, right stats overview -->
      <div class="dash-row">
        <aside class="dash-rail">
          <div class="v2-card v2-card-pad rail-card">
            <div class="rail-head">
              <span class="v2-card-title">路由模式</span>
              <el-tooltip effect="light" placement="top" :show-after="150" :enterable="true" popper-class="v2-profile-pop v2-scope">
                <template #content>
                  <div class="profile-help">
                    <div class="tooltip-title">模式说明</div>
                    <div class="tooltip-item"><strong>路由：</strong><span>写入网关地址，Agent 请求会经过 CCG Gateway，并按服务商规则路由。</span></div>
                    <div class="tooltip-item"><strong>直连：</strong><span>写入默认服务商配置，Agent 直接请求该服务商，不经过网关路由。</span></div>
                    <div class="tooltip-item"><strong>官方：</strong><span>写入官方凭证，Agent 直接连接官方服务。</span></div>
                    <div class="tooltip-item"><strong>停用：</strong><span>清除已写入的路由配置，Agent 不受 CCG Gateway 管理。</span></div>
                  </div>
                </template>
                <span class="v2-help">
                  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
                </span>
              </el-tooltip>
            </div>
            <div class="cli-list">
              <div v-for="cli in cliList" :key="cli.type" class="cli-row" :class="{ loading: cliLoading[cli.type] }">
                <div class="cli-id">
                  <span class="cli-brand-icon" :class="cli.type">
                    <CliBrandIcon :type="cli.type" width="14" height="14" />
                  </span>
                  <span class="cli-name">{{ cli.label }}</span>
                </div>
                <div class="v2-seg cli-modes">
                  <div class="v2-seg-slider" :style="{ transform: `translateX(${modeOptions.findIndex(m => getCliMode(cli.type) === m.id) * 100}%)`, width: 'calc((100% - 8px) / 4)' }"></div>
                  <button
                    v-for="m in modeOptions"
                    :key="m.id"
                    class="v2-seg-btn"
                    :class="{ active: getCliMode(cli.type) === m.id, disabled: isModeDisabled(cli.type, m.id) }"
                    @click="setMode(cli.type, m.id)"
                  >{{ m.label }}</button>
                </div>
              </div>
            </div>
          </div>
        </aside>

        <div class="v2-card v2-card-pad chart-card">
          <div class="chart-head">
            <div class="chart-title-wrap">
              <span class="v2-card-title">统计总览</span>
              <span class="refresh-indicator" :class="{ refreshing: statsLoading, paused: isPaused }" @click="togglePause">
                <div class="indicator-ring-wrap">
                  <svg class="indicator-ring" width="14" height="14" viewBox="0 0 16 16">
                    <circle class="ring-track" cx="8" cy="8" r="6.5" fill="none" stroke="currentColor" stroke-width="1.8" />
                    <circle
                      v-if="!isPaused && !statsLoading"
                      :key="refreshKey"
                      class="ring-progress"
                      cx="8"
                      cy="8"
                      r="6.5"
                      fill="none"
                      stroke="var(--v2-success)"
                      stroke-width="1.8"
                      stroke-linecap="round"
                      stroke-dasharray="41"
                      stroke-dashoffset="41"
                    />
                    <circle
                      v-else-if="statsLoading"
                      class="ring-loading"
                      cx="8"
                      cy="8"
                      r="6.5"
                      fill="none"
                      stroke="var(--v2-success)"
                      stroke-width="1.8"
                      stroke-linecap="round"
                      stroke-dasharray="12 28"
                    />
                    <circle
                      v-else
                      class="ring-paused"
                      cx="8"
                      cy="8"
                      r="2"
                      fill="var(--v2-text-3)"
                    />
                  </svg>
                </div>
                <span class="indicator-text">{{ statsLoading ? '正在刷新' : (isPaused ? '已停止刷新' : '自动更新') }}</span>
              </span>
            </div>
            <div class="chart-toggles">
              <div class="v2-seg">
                <div class="v2-seg-slider" :style="{ transform: `translateX(${metricTabs.findIndex(t => metricMode === t.id) * 100}%)`, width: 'calc((100% - 8px) / 3)' }"></div>
                <button v-for="t in metricTabs" :key="t.id" class="v2-seg-btn" :class="{ active: metricMode === t.id }" @click="metricMode = t.id">{{ t.label }}</button>
              </div>
              <div class="v2-seg">
                <div class="v2-seg-slider" :style="{ transform: `translateX(${dimTabs.findIndex(t => dimMode === t.id) * 100}%)`, width: 'calc((100% - 8px) / 2)' }"></div>
                <button v-for="t in dimTabs" :key="t.id" class="v2-seg-btn" :class="{ active: dimMode === t.id }" @click="dimMode = t.id">{{ t.label }}</button>
              </div>
            </div>
          </div>
          <div class="chart-wrap" @mouseenter="onChartEnter" @mouseleave="onChartLeave">
            <v-chart class="chart" :option="chartOption" :init-options="chartInitOptions" autoresize @legendselectchanged="onLegendChange" />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { use } from 'echarts/core'
import { LineChart, BarChart } from 'echarts/charts'
import { TooltipComponent, GridComponent, DatasetComponent, TransformComponent, LegendComponent } from 'echarts/components'
import { CanvasRenderer, SVGRenderer } from 'echarts/renderers'
import VChart from 'vue-echarts'
import { statsApi } from '@/api/stats'
import { useSettingsStore } from '@/stores/settings'
import { useThemeStore } from '@/stores/theme'
import { useAutoRefresh } from '@/composables/useAutoRefresh'
import { formatCost, formatTokens } from '@/utils/json'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'
import { CLI_TABS, CLI_LABELS, CLI_TYPES } from '@/types/models'
import type { CliType, CliMode, ProviderStats, AdvancedStatsRow } from '@/types/models'
import CliBrandIcon from '@/components/CliBrandIcon.vue'

use([LineChart, BarChart, TooltipComponent, GridComponent, DatasetComponent, TransformComponent, LegendComponent, CanvasRenderer, SVGRenderer])

const settingsStore = useSettingsStore()
const themeStore = useThemeStore()
const REFRESH_MS = 5000
const CHART_FONT = "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', 'PingFang SC', 'Microsoft YaHei', 'Microsoft YaHei UI', Arial, sans-serif"
const chartInitOptions = { renderer: 'svg' } as const

// ===== CLI 模式控制 =====
const cliList = CLI_TABS.map(({ id, label }) => ({ type: id, label }))
const cliLoading = reactive(Object.fromEntries(CLI_TYPES.map((t) => [t, false] as [CliType, boolean])) as Record<CliType, boolean>)
const modeOptions: { id: CliMode; label: string }[] = [
  { id: 'proxy_route', label: '路由' },
  { id: 'provider_direct', label: '直连' },
  { id: 'official_direct', label: '官方' },
  { id: 'disabled', label: '停用' }
]
const modeLabels: Record<CliMode, string> = { proxy_route: '中转路由', provider_direct: '中转直连', official_direct: '官方直连', disabled: '停用' }

function getCliMode(cli: CliType): CliMode {
  return settingsStore.settings?.cli_settings?.[cli]?.cli_mode ?? 'disabled'
}
// function isRouteMode(cli: CliType) {
//   return getCliMode(cli) === 'proxy_route'
// }
function isModeDisabled(cli: CliType, mode: CliMode) {
  return cli === 'claude_code' && mode === 'official_direct'
}
async function setMode(cli: CliType, mode: CliMode) {
  if (getCliMode(cli) === mode || cliLoading[cli]) return
  if (isModeDisabled(cli, mode)) {
    notify('Claude Code 暂不支持官方直连', 'warning')
    return
  }
  cliLoading[cli] = true
  try {
    await settingsStore.setDashboardCliMode(cli, mode)
    notify(`${CLI_LABELS[cli]} 已切换至 ${modeLabels[mode]}`)
  } catch (e) {
    notify(`切换失败: ${getErrorMessage(e)}`, 'error')
  } finally {
    cliLoading[cli] = false
  }
}

// ===== KPI =====
const providerStats = ref<ProviderStats[]>([])
const statsLoading = ref(false)
const refreshKey = ref(0)
const isPaused = ref(false)

function togglePause() {
  isPaused.value = !isPaused.value
  if (!isPaused.value) {
    fetchStats()
  }
}

const kpis = computed(() => {
  const s = providerStats.value
  const reqs = s.reduce((a, x) => a + x.total_requests, 0)
  const succ = s.reduce((a, x) => a + x.total_success, 0)
  const cache = s.reduce((a, x) => a + (x.total_cache_read_tokens || 0) + (x.total_cache_creation_tokens || 0), 0)
  const billable = s.reduce((a, x) => a + x.total_tokens, 0) - cache
  const cost = s.reduce((a, x) => a + (x.total_cost || 0), 0)
  const rate = reqs > 0 ? (succ / reqs) * 100 : 0

  let successColor = 'var(--v2-success)'
  if (rate < 50) {
    successColor = 'var(--v2-danger)'
  } else if (rate <= 80) {
    successColor = 'var(--v2-warning)'
  }

  return [
    {
      label: '计费 Token',
      value: formatTokens(billable),
      color: '',
      borderColor: 'var(--v2-accent)',
      icon: 'token'
    },
    {
      label: '缓存 Token',
      value: formatTokens(cache),
      color: '',
      borderColor: 'var(--v2-chart-purple)',
      icon: 'cache'
    },
    {
      label: '请求总数',
      value: reqs.toLocaleString(),
      color: '',
      borderColor: 'var(--v2-chart-cyan)',
      icon: 'request'
    },
    {
      label: '全局成功率',
      value: reqs > 0 ? rate.toFixed(1) + '%' : '0%',
      color: successColor,
      borderColor: successColor,
      icon: 'success'
    },
    {
      label: '费用',
      value: formatCost(cost),
      color: 'var(--v2-warning)',
      borderColor: 'var(--v2-warning)',
      icon: 'cost'
    }
  ]
})

// ===== 图表 =====
const advancedStats = ref<AdvancedStatsRow[]>([])
const pendingAdvanced = ref<AdvancedStatsRow[] | null>(null)
const chartHovering = ref(false)
const legendSelectedMap = ref<Record<string, Record<string, boolean>>>({})
const metricMode = ref<'tokens' | 'cost' | 'requests'>('tokens')
const dimMode = ref<'provider' | 'model'>('provider')
const metricTabs = [{ id: 'tokens', label: 'Token' }, { id: 'cost', label: '费用' }, { id: 'requests', label: '请求数' }] as const
const dimTabs = [{ id: 'provider', label: '服务商' }, { id: 'model', label: '模型' }] as const

const PALETTE = ['#3b82f6', '#10b981', '#f59e0b', '#ef4444', 'var(--v2-chart-purple)', 'var(--v2-chart-cyan)', '#f97316', '#ec4899', '#14b8a6', '#6366f1']

const isDark = computed(() => themeStore.theme === 'dark')
function cssVar(name: string) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim()
}
function chartColor(value: string) {
  if (!value.startsWith('var(')) return value
  return cssVar(value.slice(4, -1)) || value
}

const ct = computed(() => {
  const axis = isDark.value ? cssVar('--v2-surface-2') : cssVar('--v2-surface-3')
  return {
    axis,
    label: cssVar('--v2-text-3'),
    split: axis,
    tipBg: cssVar('--v2-surface'),
    tipBorder: cssVar('--v2-surface-3'),
    tipText: cssVar('--v2-text-3'),
    tipTitle: cssVar('--v2-text')
  }
})

function fmt(d: Date) {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}
function fmtToken(v: number) {
  if (v >= 1e6) return (v / 1e6).toFixed(1) + 'M'
  if (v >= 1e3) return (v / 1e3).toFixed(1) + 'K'
  return String(v)
}
function fmtMetric(v: number) {
  return metricMode.value === 'cost' ? formatCost(v) : fmtToken(v)
}

async function fetchStats() {
  statsLoading.value = true
  try {
    const today = new Date()
    const start = new Date(today)
    start.setDate(today.getDate() - 9)
    const { data: prov } = await statsApi.getProviders({})
    providerStats.value = prov
    const { data: adv } = await statsApi.getAdvanced({ start_date: fmt(start), end_date: fmt(today) })
    if (chartHovering.value) pendingAdvanced.value = adv
    else advancedStats.value = adv
  } finally {
    statsLoading.value = false
    refreshKey.value++
  }
}
function onChartEnter() {
  chartHovering.value = true
}
function onChartLeave() {
  chartHovering.value = false
  if (pendingAdvanced.value) {
    advancedStats.value = pendingAdvanced.value
    pendingAdvanced.value = null
  }
}
function onLegendChange(e: { selected?: Record<string, boolean> }) {
  if (e.selected) legendSelectedMap.value = { ...legendSelectedMap.value, [dimMode.value]: { ...e.selected } }
}

const chartOption = computed(() => {
  const dates: string[] = []
  for (let i = 9; i >= 0; i--) {
    const d = new Date()
    d.setDate(d.getDate() - i)
    dates.push(fmt(d))
  }
  const isCost = metricMode.value === 'cost'
  const isTokens = metricMode.value === 'tokens'
  const isBar = isTokens || isCost
  const groupKey = dimMode.value === 'provider' ? 'provider_name' : 'model_id'
  const palette = PALETTE.map(chartColor)

  const totals = new Map<string, number>()
  advancedStats.value.forEach((s) => {
    const v = isCost ? s.total_cost : isTokens ? s.total_tokens : s.total_success
    totals.set(s[groupKey], (totals.get(s[groupKey]) || 0) + v)
  })
  const groups = Array.from(totals.entries())
    .filter(([, sum]) => sum > 0)
    .sort((a, b) => b[1] - a[1])
    .map(([n]) => n)

  const dayTotals = dates.map((d) => {
    let sum = 0
    advancedStats.value.forEach((s) => {
      if (s.date === d) {
        sum += isCost ? s.total_cost : s.total_tokens
      }
    })
    return sum
  })

  const series: any[] = groups.map((g, idx) => {
    const color = palette[idx % palette.length]
    if (isBar) {
      const data = dates.map((d, di) => {
        let sum = 0, input = 0, output = 0, cacheRead = 0, cacheCreation = 0, cost = 0
        advancedStats.value.forEach((s) => {
          if (s.date === d && s[groupKey] === g) {
            sum += isCost ? s.total_cost : s.total_tokens
            input += s.total_input_tokens || 0
            output += s.total_output_tokens || 0
            cacheRead += s.total_cache_read_tokens || 0
            cacheCreation += s.total_cache_creation_tokens || 0
            cost += s.total_cost || 0
          }
        })
        const minVal = dayTotals[di] * 0.035
        const val = (sum > 0 && sum < minVal) ? minVal : sum
        return { value: val, actualValue: sum, input, output, cacheRead, cacheCreation, cost, name: g, color }
      })
      const colorG = {
        type: 'linear',
        x: 0,
        y: 0,
        x2: 0,
        y2: 1,
        colorStops: [
          { offset: 0, color },
          { offset: 1, color: color + 'd8' }
        ]
      }
      return {
        name: g,
        type: 'bar',
        stack: 'total',
        barWidth: '72%',
        itemStyle: { color: colorG },
        data
      }
    }
    const data = dates.map((d) => {
      let sum = 0
      advancedStats.value.forEach((s) => { if (s.date === d && s[groupKey] === g) sum += s.total_success })
      return sum
    })
    return {
      name: g, type: 'line', smooth: true, showSymbol: false, itemStyle: { color },
      areaStyle: { color: { type: 'linear', x: 0, y: 0, x2: 0, y2: 1, colorStops: [{ offset: 0, color }, { offset: 1, color: 'transparent' }] }, opacity: 0.18 },
      data
    }
  })

  if (isBar) {
    series.reverse()

    const selectedMap = legendSelectedMap.value[dimMode.value] || {}

    dates.forEach((_, di) => {
      let topSi = -1
      for (let si = series.length - 1; si >= 0; si--) {
        if (series[si].data[di].actualValue > 0 && selectedMap[series[si].name] !== false) {
          topSi = si
          break
        }
      }

      if (topSi !== -1) {
        const item = series[topSi].data[di]
        series[topSi].data[di] = {
          ...item,
          itemStyle: {
            ...item.itemStyle,
            borderRadius: [4, 4, 0, 0]
          }
        }
      }
    })
  }

  const c = ct.value
  return {
    tooltip: {
      trigger: 'axis',
      appendTo: 'body',
      transitionDuration: 0,
      extraCssText: `position: fixed; box-shadow: var(--v2-shadow-pop); border-radius: var(--v2-r); font-family: ${CHART_FONT};`,
      axisPointer: { type: isBar ? 'shadow' : 'line' },
      backgroundColor: c.tipBg,
      borderColor: c.tipBorder,
      textStyle: { color: c.tipText, fontFamily: CHART_FONT, fontSize: 13, fontWeight: 500 },
      position: (point: any, _params: any, _dom: any, _rect: any, size: any) => {
        const x = point[0]
        const y = point[1]
        const viewWidth = size.viewSize[0]
        const viewHeight = size.viewSize[1]
        const boxWidth = size.contentSize[0]
        const boxHeight = size.contentSize[1]

        let left = x + 20
        if (left + boxWidth > viewWidth) {
          left = x - boxWidth - 20
        }
        if (left < 0) left = 0

        let top = y - boxHeight / 2
        if (top < 10) top = 10
        if (top + boxHeight > viewHeight - 10) {
          top = Math.max(10, viewHeight - boxHeight - 10)
        }

        return [left, top]
      },
      formatter: (params: any[]) => {
        if (!params.length) return ''
        const isCost = metricMode.value === 'cost'
        const isTokens = metricMode.value === 'tokens'
        const isBar = isTokens || isCost

        // 计算当前所有激活系列的总计值
        let grandTotal = 0
        params.forEach((p) => {
          const val = Number(isBar ? (p.data?.actualValue !== undefined ? p.data.actualValue : p.value) : p.value)
          if (!isNaN(val) && val > 0) {
            const selectedMap = legendSelectedMap.value[dimMode.value] || {}
            if (selectedMap[p.seriesName] !== false) {
              grandTotal += val
            }
          }
        })

        const vis = params.filter((p) => Number(isBar ? p.data?.actualValue : p.value) > 0)
        vis.sort((a, b) => {
          const valA = Number(isBar ? a.data?.actualValue : a.value)
          const valB = Number(isBar ? b.data?.actualValue : b.value)
          return valB - valA
        })
        if (!vis.length) return ''

        let html = `<div style="margin-bottom:8px;display:flex;justify-content:space-between;align-items:center;gap:16px;border-bottom:1px solid ${c.split};padding-bottom:6px;color:${c.tipTitle};font-weight:600;">
          <span>${params[0].axisValue}</span>
          <span style="font-size:12px;">总计: ${fmtMetric(grandTotal)}</span>
        </div>`

        if (isBar) {
          vis.forEach((p, i) => {
            const d = p.data
            html += `<div style="margin-bottom:6px;${i > 0 ? `border-top:1px solid ${c.split};padding-top:6px;` : ''}">
              <div style="display:flex;align-items:center;gap:6px;font-size:14px;color:${c.tipTitle};font-weight:600;">
                ${d.name}</div>
              <div style="font-size:12px;margin-top:1px;">
                输入 ${fmtToken(d.input)} / 输出 ${fmtToken(d.output)} &nbsp;•&nbsp; 缓存读 ${fmtToken(d.cacheRead)} / 缓存创 ${fmtToken(d.cacheCreation)}
              </div></div>`
          })
        } else {
          vis.forEach((p, i) => {
            html += `<div style="margin-bottom:4px;display:flex;align-items:center;gap:6px;font-size:14px;color:${c.tipTitle};font-weight:600;${i > 0 ? `border-top:1px solid ${c.split};padding-top:6px;` : ''}">
              <span>${p.seriesName}:</span><span>${p.value}</span></div>`
          })
        }
        return html
      }
    },
    textStyle: { fontFamily: CHART_FONT, fontSize: 13, fontWeight: 500 },
    legend: { data: groups, bottom: 0, left: 'center', type: 'scroll', icon: 'circle', selected: legendSelectedMap.value[dimMode.value], textStyle: { color: c.label, fontFamily: CHART_FONT, fontSize: 14, fontWeight: 500 } },
    grid: { top: 16, right: '1.5%', bottom: 36, left: '1.5%', containLabel: true },
    xAxis: { type: 'category', data: dates, boundaryGap: isBar, axisLine: { lineStyle: { color: c.axis } }, axisTick: { show: false }, axisLabel: { color: c.label, fontFamily: CHART_FONT, fontSize: 13, fontWeight: 500, formatter: (v: string) => v.substring(5) } },
    yAxis: { type: 'value', splitNumber: 4, splitLine: { lineStyle: { type: 'dashed', color: c.split } }, axisLabel: { color: c.label, fontFamily: CHART_FONT, fontSize: 13, fontWeight: 500, formatter: (v: number) => isCost ? formatCost(v) : fmtToken(v) } },
    series
  }
})

useAutoRefresh(fetchStats, {
  intervalMs: REFRESH_MS,
  immediate: true,
  paused: isPaused,
  onError: (e) => notify(getErrorMessage(e, '数据刷新失败'), 'error')
})

onMounted(() => {
  if (!settingsStore.settings) settingsStore.fetchSettings()
})
</script>

<style scoped>
.dash-body {
  display: flex;
  flex-direction: column;
  gap: 16px;
}


.dash-row {
  display: grid;
  grid-template-columns: 280px 1fr;
  gap: 16px;
  align-items: stretch;
}
@media (max-width: 920px) {
  .dash-row { grid-template-columns: 1fr; }
}
.dash-rail { display: flex; }
.dash-rail .rail-card { flex: 1; }

.rail-card {
  display: flex;
  flex-direction: column;
}
.rail-head {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 28px;
  margin-bottom: 10px;
}
.cli-list { display: flex; flex-direction: column; }
.cli-row {
  padding: 13px 0;
  border-bottom: 1px solid var(--v2-surface-2);
}
.cli-row:last-child { border-bottom: none; }
.cli-row.loading { opacity: 0.55; pointer-events: none; }
.cli-id {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
}
.cli-brand-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: 4px;
  flex-shrink: 0;
}
.cli-brand-icon.claude_code {
  background: color-mix(in srgb, var(--v2-brand-claude) 12%, transparent);
  color: var(--v2-brand-claude);
}
.cli-brand-icon.codex {
  background: color-mix(in srgb, var(--v2-brand-openai) 12%, transparent);
  color: var(--v2-brand-openai);
}
.cli-brand-icon.gemini {
  background: color-mix(in srgb, var(--v2-brand-gemini) 12%, transparent);
  color: var(--v2-brand-gemini);
}
.cli-icon {
  width: 13px;
  height: 13px;
}
.cli-name { font-size: var(--v2-fs-sm); font-weight: 500; color: var(--v2-text); }
.cli-modes {
  display: grid !important;
  grid-auto-flow: column;
  grid-auto-columns: 1fr;
  width: 100%;
  max-width: 100%;
}
.cli-modes .v2-seg-btn {
  flex: 1;
  padding: 5px 7px;
  font-size: var(--v2-fs-sm);
  line-height: 20px;
  min-width: 0;
  text-align: center;
}
.cli-modes .v2-seg-btn.disabled {
  opacity: 0.42;
  color: var(--v2-text-3) !important;
  cursor: not-allowed;
  background: transparent !important;
}

.chart-card {
  min-width: 0;
}
.chart-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 10px;
}
.chart-toggles { display: flex; gap: 10px; flex-wrap: wrap; }
.chart-wrap { height: 300px; width: 100%; }
.chart { height: 100%; width: 100%; }
@media (max-width: 640px) {
  .chart-wrap { height: 260px; }
}

/* KPI 与刷新指示器新增样式 */
.v2-kpi {
  background: var(--v2-surface);
  border: 1px solid var(--v2-surface-3);
  border-radius: var(--v2-r);
  padding: 16px 18px;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  box-shadow: var(--v2-shadow-card);
  position: relative;
  overflow: hidden;
  min-height: 94px;
}

.v2-kpi::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 3px;
  background: var(--kpi-accent, var(--v2-surface-3));
}

.kpi-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
  width: 100%;
}

.v2-kpi-label {
  font-size: var(--v2-fs-xs);
  color: var(--v2-text-3);
  font-weight: 500;
  margin: 0 !important;
  text-transform: uppercase;
  letter-spacing: 0;
}

.kpi-icon-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: color-mix(in srgb, var(--kpi-accent) 10%, var(--v2-surface-2));
}

.v2-kpi-value {
  font-size: var(--v2-fs-xl);
  font-weight: 700;
  letter-spacing: 0;
  line-height: 1.1;
  color: var(--v2-text);
}

.chart-title-wrap {
  display: flex;
  align-items: center;
  gap: 8px;
}
.kpi-icon-wrap {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 50%;
  transition: all 0.2s ease;
}
.refresh-indicator {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: var(--v2-fs-xs);
  color: var(--v2-text-3);
  padding: 4px 10px 4px 8px;
  border-radius: 999px;
  background: var(--v2-surface-2);
  transition: all 0.3s ease;
  position: relative;
  overflow: hidden;
  cursor: pointer;
  user-select: none;
}
.refresh-indicator:hover {
  background: var(--v2-surface-3);
  color: var(--v2-text);
}
.indicator-ring-wrap {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  color: var(--v2-text-3);
}
.indicator-ring {
  overflow: visible;
}
.ring-track {
  opacity: 0.15;
}
.ring-progress {
  transform: rotate(-90deg);
  transform-origin: center;
  animation: ring-countdown 4.6s linear forwards;
}
@keyframes ring-countdown {
  from { stroke-dashoffset: 41; }
  to { stroke-dashoffset: 0; }
}
.ring-loading {
  transform-origin: center;
  animation: ring-spin 1s linear infinite;
}
@keyframes ring-spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}
.ring-paused {
  transition: all 0.3s ease;
}
.indicator-text {
  font-size: 11px;
  font-weight: 500;
  line-height: 1;
}
</style>
