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

      <!-- 核心图表分析区 -->
      <div class="b-card responsive-bottom-card" style="margin-bottom: 24px; padding: 24px; min-width: 400px;">
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; flex-wrap: wrap; gap: 16px;">
          <div class="b-card-title" style="margin-bottom: 0;">趋势与分布分析</div>
          <div style="display: flex; gap: 16px; align-items: center; flex-wrap: wrap;">
            <div class="b-segmented">
              <div class="b-seg-btn" :class="{ active: metricMode === 'requests' }" @click="metricMode = 'requests'">请求次数</div>
              <div class="b-seg-btn" :class="{ active: metricMode === 'tokens' }" @click="metricMode = 'tokens'">Token 消耗</div>
            </div>
            <div class="b-segmented">
              <div class="b-seg-btn" :class="{ active: dimMode === 'provider' }" @click="dimMode = 'provider'">按服务商</div>
              <div class="b-seg-btn" :class="{ active: dimMode === 'model' }" @click="dimMode = 'model'">按模型</div>
            </div>
          </div>
        </div>
        <div style="height: 350px; width: 100%;">
          <v-chart class="chart" :option="chartOption" autoresize />
        </div>
      </div>

      <!-- 多维数据明细表 -->
      <div class="b-card responsive-bottom-card" style="padding: 24px; min-width: 400px; display: flex; flex-direction: column;">
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; flex-wrap: wrap; gap: 16px;">
          <div class="b-card-title" style="margin-bottom: 0;">详细数据记录</div>
          <div style="display: flex; gap: 12px; flex-wrap: wrap;">
            <select v-model="filterDate" class="filter-select">
              <option value="7days">最近 7 天</option>
              <option value="today">今天</option>
            </select>
            <select v-model="filterProvider" class="filter-select">
              <option value="all">所有服务商</option>
              <option v-for="p in uniqueProviders" :key="p" :value="p">{{ p }}</option>
            </select>
            <select v-model="filterModel" class="filter-select">
              <option value="all">所有模型</option>
              <option v-for="m in uniqueModels" :key="m" :value="m">{{ m }}</option>
            </select>
          </div>
        </div>
        <div class="stats-table-wrapper" style="height: 400px;">
          <table class="flat-table">
            <thead>
              <tr>
                <th>日期</th>
                <th>服务商</th>
                <th>模型</th>
                <th>请求总数</th>
                <th>成功率</th>
                <th>总 Token 消耗</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in filteredTableData" :key="`${row.date}-${row.provider_name}-${row.model_id}`">
                <td class="table-cell">{{ row.date }}</td>
                <td class="table-cell"><span class="badge" :class="getProviderBadgeClass(row.provider_name)">{{ row.provider_name }}</span></td>
                <td class="table-cell">{{ row.model_id }}</td>
                <td class="table-cell mono">{{ row.total_requests }}</td>
                <td class="table-cell" :class="getSuccessRateColor(row.total_success, row.total_requests)">{{ ((row.total_success / (row.total_requests || 1)) * 100).toFixed(1) }}%</td>
                <td class="table-cell mono">{{ formatTokens(row.total_tokens) }}</td>
              </tr>
              <tr v-if="filteredTableData.length === 0">
                <td colspan="6" style="text-align: center; color: var(--color-text-weak); padding: 24px;">暂无数据</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref, reactive, computed } from 'vue'
import { confirm } from '@/utils/confirm'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'

import { use } from 'echarts/core'
import { LineChart, BarChart } from 'echarts/charts'
import { TooltipComponent, GridComponent, DatasetComponent, TransformComponent, LegendComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import VChart from 'vue-echarts'
import * as echarts from 'echarts/core'

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
const filterDate = ref<'7days' | 'today'>('7days')
const filterProvider = ref<string>('all')
const filterModel = ref<string>('all')

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
  const p3 = statsApi.getAdvanced({ start_date: formatLocalDate(sevenDaysAgo), end_date: formatLocalDate(today) })
  
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

const chartOption = computed(() => {
  const dates: string[] = []
  for (let i = 6; i >= 0; i--) {
    const d = new Date()
    d.setDate(d.getDate() - i)
    dates.push(formatLocalDate(d))
  }

  const metricKey = metricMode.value === 'requests' ? 'total_requests' : 'total_tokens'

  // 堆叠柱状图 (按服务商或模型)
  const groupKey = dimMode.value === 'provider' ? 'provider_name' : 'model_id'
  const groups = new Set<string>()
  advancedStats.value.forEach(s => groups.add(s[groupKey]))
  const groupArray = Array.from(groups).sort()

  const seriesData: any[] = []
  groupArray.forEach((gName, idx) => {
    const data = dates.map(d => {
      let sum = 0
      advancedStats.value.forEach(s => {
        if (s.date === d && s[groupKey] === gName) sum += s[metricKey]
      })
      return sum
    })
    
    const color = PALETTE[idx % PALETTE.length]
    seriesData.push({
      name: gName,
      type: 'bar',
      stack: 'total',
      barWidth: '40%',
      itemStyle: { color },
      data
    })
  })

  // 对顶部的柱子应用圆角
  if (seriesData.length > 0) {
    for (let i = 0; i < dates.length; i++) {
      let topSeriesIdx = -1
      for (let j = seriesData.length - 1; j >= 0; j--) {
        if (seriesData[j].data[i] > 0) {
          topSeriesIdx = j
          break
        }
      }
      if (topSeriesIdx !== -1) {
        if (!seriesData[topSeriesIdx].itemStyle) seriesData[topSeriesIdx].itemStyle = {}
        if (!seriesData[topSeriesIdx].itemStyle.borderRadius) {
           seriesData[topSeriesIdx].itemStyle.borderRadius = [0, 0, 0, 0] // default
        }
        // We can't easily do per-item border radius in simple series definition without using function,
        // so we skip dynamic per-bar radius for simplicity and compatibility.
      }
    }
  }

  return {
    tooltip: { 
      trigger: 'axis', 
      axisPointer: { type: 'shadow' }, 
      valueFormatter: (value: any) => metricMode.value === 'tokens' ? formatTokens(value) : value,
      backgroundColor: 'rgba(255, 255, 255, 0.9)', 
      borderColor: '#e2e8f0', 
      textStyle: { color: '#0f172a' } 
    },
    legend: { top: 0, right: 0, icon: 'circle', textStyle: { color: '#64748b' } },
    grid: { top: 40, right: 40, bottom: 20, left: 50, containLabel: true },
    xAxis: { type: 'category', data: dates, axisLine: { lineStyle: { color: '#e2e8f0' } }, axisLabel: { color: '#64748b' } },
    yAxis: { 
      type: 'value', 
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
  
  if (filterDate.value === 'today') {
    const todayStr = formatLocalDate(new Date())
    result = result.filter(r => r.date === todayStr)
  }
  
  if (filterProvider.value !== 'all') {
    result = result.filter(r => r.provider_name === filterProvider.value)
  }
  
  if (filterModel.value !== 'all') {
    result = result.filter(r => r.model_id === filterModel.value)
  }
  
  return result
})

function getProviderBadgeClass(providerName: string) {
  const n = providerName.toLowerCase()
  if (n.includes('ali') || n.includes('阿里')) return 'prov-ali'
  if (n.includes('deepseek')) return 'prov-deepseek'
  if (n.includes('open') || n.includes('gpt')) return 'prov-openai'
  if (n.includes('claude') || n.includes('anthropic')) return 'prov-claude'
  return 'prov-default'
}

function getSuccessRateColor(success: number, total: number) {
  if (!total) return 'text-muted'
  const rate = success / total
  if (rate >= 0.95) return 'text-green'
  if (rate >= 0.8) return 'text-warning'
  return 'text-danger'
}

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
.text-warning { color: #eab308; }
.text-danger { color: #ef4444; }
.text-muted { color: var(--color-text-weak); }
.table-cell { font-size: var(--fs-14); color: var(--color-text); }

.chart { width: 100%; height: 100%; }

/* Stats Table Wrapper */
.stats-table-wrapper { overflow-y: auto; }

.filter-select { padding: 8px 12px; border: 1px solid var(--color-border); border-radius: 6px; outline: none; font-size: var(--fs-14); color: var(--color-text); min-width: 120px; background: var(--color-bg); }

/* Flat Table */
.flat-table { width: 100%; border-collapse: separate; border-spacing: 0; text-align: left; }
.flat-table th, .flat-table td { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; box-sizing: border-box; text-align: left; }
.flat-table th { padding: 12px 16px; font-size: var(--fs-12); font-weight: var(--fw-600); color: var(--color-text-muted); text-transform: uppercase; background: var(--color-bg-page); border-bottom: 1px solid var(--color-border); position: sticky; top: 0; z-index: 10; }
.flat-table td { padding: 12px 16px; font-size: var(--fs-14); color: var(--color-text); border-bottom: 1px solid var(--color-bg-subtle); }
.flat-table tr:last-child td { border-bottom: none; }
.flat-table tr:hover td { background: var(--color-bg-page); }

.badge { padding: 2px 8px; border-radius: 12px; font-size: 12px; font-weight: 500; }
.prov-ali { background: #e0f2fe; color: #0284c7; }
.prov-deepseek { background: #f3e8ff; color: #7e22ce; }
.prov-openai { background: #dcfce7; color: #059669; }
.prov-claude { background: #fce7f3; color: #db2777; }
.prov-default { background: #f1f5f9; color: #475569; }
</style>