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
          <div class="kpi-title">活跃服务商</div>
          <div class="kpi-value mono">{{ kpiData.providers }}</div>
        </div>
      </div>

      <!-- 底部图表与日志 -->
      <div style="display: flex; gap: 24px; flex-wrap: wrap;">
        <!-- 图表区 -->
        <div class="b-card responsive-bottom-card" style="flex: 1; margin-bottom: 0; min-width: 400px;">
          <div class="b-card-title">请求统计趋势</div>
          <div style="height: 240px; width: 100%;">
            <v-chart class="chart" :option="chartOption" autoresize />
          </div>
        </div>
        
        <!-- 服务商统计 -->
        <div class="b-card responsive-bottom-card" style="flex: 1; margin-bottom: 0; display: flex; flex-direction: column; min-width: 400px; padding: 24px;">
          <div class="b-card-title" style="margin-bottom: 16px;">服务商统计</div>
          <div style="flex: 1; min-height: 240px;">
            <el-table :data="providerStats" style="width: 100%" :max-height="240">
              <el-table-column prop="provider_name" label="服务商" min-width="100" show-overflow-tooltip>
                <template #default="scope">
                  <span class="table-cell">{{ scope.row.provider_name }}</span>
                </template>
              </el-table-column>
              <el-table-column prop="total_requests" label="请求" min-width="100">
                <template #default="scope">
                  <span class="table-cell">{{ scope.row.total_requests }}</span>
                </template>
              </el-table-column>
              <el-table-column label="成功率" min-width="100">
                <template #default="scope">
                  <span class="table-cell">{{ scope.row.success_rate.toFixed(1) }}%</span>
                </template>
              </el-table-column>
              <el-table-column label="Token" min-width="100">
                <template #default="scope">
                  <span class="table-cell">{{ formatTokens(scope.row.total_tokens) }}</span>
                </template>
              </el-table-column>
            </el-table>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref, reactive, computed } from 'vue'
import { ElMessageBox } from 'element-plus'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'

import { use } from 'echarts/core'
import { LineChart, BarChart } from 'echarts/charts'
import { TooltipComponent, GridComponent, DatasetComponent, TransformComponent, LegendComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import VChart from 'vue-echarts'
import * as echarts from 'echarts/core'

use([LineChart, BarChart, TooltipComponent, GridComponent, DatasetComponent, TransformComponent, LegendComponent, CanvasRenderer])

import { useDashboardStore } from '@/stores/dashboard'
import { useProviderStore } from '@/stores/providers'
import { useSettingsStore } from '@/stores/settings'
import { statsApi } from '@/api/stats'
import { formatTokens } from '@/utils/json'
import type { ProviderStats, DailyStats } from '@/types/models'

const dashboardStore = useDashboardStore()
const providerStore = useProviderStore()
const settingsStore = useSettingsStore()

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

const kpiData = computed(() => {
  const stats = providerStats.value
  const totalRequests = stats.reduce((sum, s) => sum + s.total_requests, 0)
  const totalSuccess = stats.reduce((sum, s) => sum + s.total_success, 0)
  const totalTokens = stats.reduce((sum, s) => sum + s.total_tokens, 0)
  const activeProviders = stats.filter(s => s.total_requests > 0).length
  const successRate = totalRequests > 0 ? (totalSuccess / totalRequests) * 100 : 0

  return {
    requests: totalRequests.toLocaleString(),
    successRate: totalRequests > 0 ? successRate.toFixed(1) + '%' : '0%',
    tokens: formatTokens(totalTokens),
    providers: activeProviders
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
      await ElMessageBox.confirm('当前是官方模式，是否切换至中转模式并启用代理？', '提示', {
        confirmButtonText: '切换并启用', cancelButtonText: '取消'
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

async function fetchStats() {
  const providerRes = await statsApi.getProviders({})
  providerStats.value = providerRes.data
}

function formatLocalDate(d: Date): string {
  const year = d.getFullYear()
  const month = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

async function fetchChartData() {
  const today = new Date()
  const sevenDaysAgo = new Date(today)
  sevenDaysAgo.setDate(today.getDate() - 6)
  const params = { start_date: formatLocalDate(sevenDaysAgo), end_date: formatLocalDate(today) }
  const dailyRes = await statsApi.getDaily(params)
  dailyStats.value = dailyRes.data
}

const chartOption = computed(() => {
  const dates: string[] = []
  for (let i = 6; i >= 0; i--) {
    const d = new Date()
    d.setDate(d.getDate() - i)
    dates.push(formatLocalDate(d))
  }
  
  const dateMap = new Map<string, { reqs: number; success: number }>()
  dates.forEach(d => dateMap.set(d, { reqs: 0, success: 0 }))
  
  dailyStats.value.forEach(s => {
    const ex = dateMap.get(s.usage_date)
    if (ex) {
      ex.reqs += s.success_count + s.failure_count
      ex.success += s.success_count
    }
  })

  const reqData = dates.map(d => dateMap.get(d)!.reqs)
  const successData = dates.map(d => dateMap.get(d)!.success)

  return {
    tooltip: { trigger: 'axis', backgroundColor: 'rgba(255, 255, 255, 0.9)', borderColor: '#e2e8f0', textStyle: { color: '#0f172a' } },
    legend: { show: false },
    grid: { top: 20, right: 40, bottom: 20, left: 40, containLabel: true },
    xAxis: {
      type: 'category',
      boundaryGap: false,
      data: dates,
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: { color: '#94a3b8', margin: 12 }
    },
    yAxis: {
      type: 'value',
      name: '',
      splitLine: { lineStyle: { type: 'dashed', color: '#f1f5f9' } },
      axisLabel: { color: '#94a3b8' }
    },
    series: [
      {
        name: '总请求数',
        type: 'line',
        smooth: true,
        symbol: 'none',
        lineStyle: { width: 3, color: '#0ea5e9' },
        areaStyle: {
          color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
            { offset: 0, color: 'rgba(14, 165, 233, 0.3)' },
            { offset: 1, color: 'rgba(14, 165, 233, 0.0)' }
          ])
        },
        data: reqData
      },
      {
        name: '成功请求数',
        type: 'line',
        smooth: true,
        symbol: 'none',
        lineStyle: { width: 3, color: '#10b981' },
        areaStyle: {
          color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
            { offset: 0, color: 'rgba(16, 185, 129, 0.3)' },
            { offset: 1, color: 'rgba(16, 185, 129, 0.0)' }
          ])
        },
        data: successData
      }
    ]
  }
})

onMounted(async () => {
  await Promise.all([
    dashboardStore.fetchStatus(),
    providerStore.fetchProviders(),
    settingsStore.fetchSettings(),
    fetchStats(),
    fetchChartData()
  ])
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

.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 32px; flex-shrink: 0; }
.page-title { font-size: var(--fs-20); font-weight: var(--fw-700); margin: 0; letter-spacing: -0.5px; }

.b-card { background: var(--color-bg); border-radius: 16px; box-shadow: 0 4px 12px rgba(0,0,0,0.03); padding: 24px; margin-bottom: 24px; transition: border-color 0.2s; border: 1px solid transparent; }
.b-card:hover { border-color: var(--color-border); }
.b-card-title { font-size: var(--fs-16); font-weight: var(--fw-600); margin-bottom: 20px; color: var(--color-text); }

.status-dot { width: 10px; height: 10px; border-radius: 50%; background: var(--color-text-weak); }
.status-dot.running { background: var(--color-success); box-shadow: 0 0 8px rgba(16, 185, 129, 0.4); }

.cli-title { font-size: var(--fs-16); font-weight: var(--fw-700); color: var(--color-text); }
.cli-disabled { font-size: var(--fs-14); font-weight: var(--fw-400); color: var(--color-text-weak); margin-left: 4px; }

.b-segmented { display: inline-flex; background: var(--color-border); padding: 4px; border-radius: 10px; }
.b-seg-btn { text-align: center; padding: 6px 16px; font-size: var(--fs-14); color: var(--color-text-muted); border-radius: 8px; font-weight: var(--fw-500); transition: all 0.2s ease; cursor: pointer; }
.b-seg-btn.active { background: var(--color-bg); color: var(--color-text); box-shadow: 0 1px 3px rgba(0,0,0,0.1); pointer-events: none; }

.kpi-card { flex: 1; padding: 24px 20px !important; margin-bottom: 0 !important; text-align: center; display: flex; flex-direction: column; justify-content: center; }
.kpi-title { font-size: var(--fs-14); font-weight: var(--fw-500); color: var(--color-text-muted); margin-bottom: 12px; }
.kpi-value { font-size: var(--fs-32); font-weight: var(--fw-700); letter-spacing: -1px; }

.text-blue { color: var(--color-primary); }
.text-green { color: var(--color-success); }
.table-cell { font-size: var(--fs-14); color: var(--color-text); }

.chart { width: 100%; height: 100%; }
</style>
