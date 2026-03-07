<template>
  <div class="dashboard">
    <!-- 网关状态卡片 -->
    <el-row :gutter="16">
      <el-col :span="8" v-for="cli in cliList" :key="cli.type">
        <el-card class="status-card" shadow="always">
          <div class="status-card-content">
            <div class="status-left">
              <span class="status-indicator" :class="getCliEnabled(cli.type) ? 'running' : 'stopped'"></span>
              <div class="status-info">
                <span class="status-name">{{ cli.label }}</span>
                <div class="status-details">
                  <span class="status-text">{{ getCliEnabled(cli.type) ? '运行中' : '已停止' }}</span>
                  <el-tag :type="getCliMode(cli.type) === 'proxy' ? 'success' : 'info'" size="small" class="mode-tag">
                    {{ getCliMode(cli.type) === 'proxy' ? '中转模式' : '官方模式' }}
                  </el-tag>
                </div>
              </div>
            </div>
            <el-switch
              :model-value="getCliEnabled(cli.type)"
              @change="(val: boolean) => handleCliToggle(cli.type, val)"
              :loading="cliLoading[cli.type]"
            />
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- KPI 概览区 -->
    <el-row :gutter="16" class="kpi-row">
      <el-col :span="6" v-for="kpi in kpiList" :key="kpi.key">
        <el-card class="kpi-card" shadow="always">
          <div class="kpi-value">{{ kpi.value }}</div>
          <div class="kpi-label">{{ kpi.label }}</div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 服务商统计 & 请求趋势 -->
    <el-row :gutter="16" class="main-row">
      <el-col :span="12">
        <el-card class="main-card" shadow="always">
          <template #header>
            <div class="card-header">
              <span>服务商统计</span>
              <el-date-picker
                v-model="dateRange"
                type="daterange"
                range-separator="-"
                start-placeholder="开始"
                end-placeholder="结束"
                value-format="YYYY-MM-DD"
                size="small"
                style="width: 200px"
                @change="fetchStats"
              />
            </div>
          </template>
          <el-table :data="providerStats" stripe size="small" class="stats-table">
            <el-table-column prop="cli_type" label="CLI" width="100" />
            <el-table-column prop="provider_name" label="服务商" />
            <el-table-column prop="total_requests" label="请求" width="80" />
            <el-table-column label="成功率" width="80">
              <template #default="{ row }">
                <el-tag :type="row.success_rate >= 90 ? 'success' : row.success_rate >= 70 ? 'warning' : 'danger'" size="small">
                  {{ row.success_rate.toFixed(1) }}%
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="Token" width="100">
              <template #default="{ row }">{{ formatTokens(row.total_tokens) }}</template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card class="main-card" shadow="always">
          <template #header>请求趋势</template>
          <div ref="chartRef" class="chart-container"></div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref, reactive, computed, nextTick } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import * as echarts from 'echarts/core'
import { BarChart } from 'echarts/charts'
import { GridComponent, TooltipComponent, LegendComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import { useDashboardStore } from '@/stores/dashboard'
import { useProviderStore } from '@/stores/providers'
import { useSettingsStore } from '@/stores/settings'
import { statsApi } from '@/api/stats'
import type { ProviderStats, DailyStats } from '@/types/models'

echarts.use([BarChart, GridComponent, TooltipComponent, LegendComponent, CanvasRenderer])

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

const dateRange = ref<[string, string] | null>(null)
const providerStats = ref<ProviderStats[]>([])
const dailyStats = ref<DailyStats[]>([])
const chartRef = ref<HTMLElement>()
let chart: echarts.ECharts | null = null

const kpiList = computed(() => {
  const stats = providerStats.value
  const totalRequests = stats.reduce((sum, s) => sum + s.total_requests, 0)
  const totalSuccess = stats.reduce((sum, s) => sum + s.total_success, 0)
  const totalTokens = stats.reduce((sum, s) => sum + s.total_tokens, 0)
  const activeProviders = stats.filter(s => s.total_requests > 0).length
  const successRate = totalRequests > 0 ? (totalSuccess / totalRequests) * 100 : 0

  return [
    { key: 'requests', label: '总请求数', value: totalRequests.toLocaleString(), change: 0, changeType: '' },
    { key: 'success', label: '整体成功率', value: successRate.toFixed(1) + '%', change: 0, changeType: '' },
    { key: 'tokens', label: 'Token 消耗', value: formatTokens(totalTokens), change: 0, changeType: '' },
    { key: 'providers', label: '活跃服务商', value: activeProviders, change: 0, changeType: '' }
  ]
})

function getCliEnabled(cliType: string): boolean {
  const settings = settingsStore.settings?.cli_settings?.[cliType]
  if (!settings) return false
  
  // enabled 是从配置文件实时读取的
  // 官方模式下，如果配置文件中还有网关地址，也会显示为 enabled=true
  // 但逻辑上官方模式不应该使用网关，所以这里强制显示为 false
  if (settings.cli_mode === 'direct') return false
  
  return settings.enabled ?? false
}

function getCliMode(cliType: string): 'proxy' | 'direct' {
  return settingsStore.settings?.cli_settings?.[cliType]?.cli_mode ?? 'proxy'
}

function formatTokens(tokens: number): string {
  if (!tokens) return '0'
  if (tokens < 1000) return tokens.toString()
  return (tokens / 1000).toFixed(1) + 'K'
}

async function handleCliToggle(cliType: string, enabled: boolean) {
  // 如果要启用 CLI，但当前是官方模式，提示用户切换
  if (enabled && getCliMode(cliType) === 'direct') {
    try {
      await ElMessageBox.confirm(
        '当前是官方模式，网关代理不会生效。是否切换至中转模式？',
        '提示',
        {
          confirmButtonText: '切换至中转模式',
          cancelButtonText: '保持官方模式',
          type: 'warning'
        }
      )
      // 用户选择切换至中转模式
      cliLoading[cliType] = true
      try {
        await settingsStore.setCliMode(cliType, 'proxy')
        await settingsStore.updateCli(cliType, { enabled: true })
        ElMessage.success(`${cliType} 已切换至中转模式并启用`)
      } catch (error: any) {
        console.error('CLI toggle error:', error)
        const errorMsg = error?.message || error?.toString() || '操作失败'
        ElMessage.error(`操作失败: ${errorMsg}`)
      } finally {
        cliLoading[cliType] = false
      }
    } catch {
      // 用户取消或选择保持官方模式，不做任何操作
      ElMessage.info(`${cliType} 保持官方模式`)
    }
  } else {
    // 正常的启用/禁用操作
    cliLoading[cliType] = true
    try {
      await settingsStore.updateCli(cliType, { enabled })
      ElMessage.success(`${cliType} 已${enabled ? '启用' : '禁用'}`)
    } catch (error: any) {
      console.error('CLI toggle error:', error)
      const errorMsg = error?.message || error?.toString() || '操作失败'
      ElMessage.error(`操作失败: ${errorMsg}`)
    } finally {
      cliLoading[cliType] = false
    }
  }
}

async function fetchStats() {
  const params: any = {}
  if (dateRange.value) {
    params.start_date = dateRange.value[0]
    params.end_date = dateRange.value[1]
  }
  const providerRes = await statsApi.getProviders(params)
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
  const fiveDaysAgo = new Date(today)
  fiveDaysAgo.setDate(today.getDate() - 4)
  const params = {
    start_date: formatLocalDate(fiveDaysAgo),
    end_date: formatLocalDate(today)
  }
  const dailyRes = await statsApi.getDaily(params)
  dailyStats.value = dailyRes.data
  updateChart()
}

function updateChart() {
  if (!chart) return

  // 生成最近5天的日期
  const dates: string[] = []
  for (let i = 4; i >= 0; i--) {
    const d = new Date()
    d.setDate(d.getDate() - i)
    dates.push(formatLocalDate(d))
  }

  // 汇总数据
  const dateMap = new Map<string, { success: number; failure: number }>()
  for (const date of dates) {
    dateMap.set(date, { success: 0, failure: 0 })
  }
  for (const stat of dailyStats.value) {
    const existing = dateMap.get(stat.usage_date)
    if (existing) {
      existing.success += stat.success_count
      existing.failure += stat.failure_count
    }
  }

  const successData = dates.map(d => dateMap.get(d)!.success)
  const failureData = dates.map(d => dateMap.get(d)!.failure)

  chart.setOption({
    tooltip: { trigger: 'axis' },
    legend: { data: ['成功', '失败'], top: 0 },
    grid: { top: 30, bottom: 20, left: 40, right: 10 },
    xAxis: { type: 'category', data: dates },
    yAxis: { type: 'value' },
    series: [
      { name: '成功', type: 'bar', stack: 'total', data: successData, itemStyle: { color: '#67C23A' } },
      { name: '失败', type: 'bar', stack: 'total', data: failureData, itemStyle: { color: '#F56C6C' } }
    ]
  })
}

onMounted(async () => {
  await Promise.all([
    dashboardStore.fetchStatus(),
    providerStore.fetchProviders(),
    settingsStore.fetchSettings(),
    fetchStats(),
    fetchChartData()
  ])
  await nextTick()
  if (chartRef.value && !chart) {
    chart = echarts.init(chartRef.value)
    updateChart()
  }
})
</script>

<style scoped>
.dashboard {
  padding: 0;
}

.status-card {
  margin-bottom: 16px;
}

.status-card-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.status-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.status-indicator {
  width: 12px;
  height: 12px;
  border-radius: 50%;
}

.status-indicator.running {
  background: #67C23A;
  box-shadow: 0 0 8px rgba(103, 194, 58, 0.6);
}

.status-indicator.stopped {
  background: #909399;
}

.status-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.status-name {
  font-size: 16px;
  font-weight: 500;
  color: #303133;
}

.status-details {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-text {
  font-size: 12px;
  color: #909399;
}

.mode-tag {
  font-size: 11px;
}

.kpi-row {
  margin-bottom: 16px;
}

.kpi-card :deep(.el-card__body) {
  text-align: center;
  padding: 20px;
}

.kpi-value {
  font-size: 28px;
  font-weight: 600;
  color: #409EFF;
}

.kpi-label {
  font-size: 14px;
  color: #606266;
}

.main-row {
  margin-bottom: 16px;
}

.main-card {
  height: 320px;
}

.main-card :deep(.el-card__body) {
  height: calc(100% - 56px);
  padding: 16px;
}

.stats-table {
  height: 100%;
}

.chart-container {
  height: calc(320px - 56px - 32px);
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
}

.card-header :deep(.el-date-editor) {
  flex-shrink: 0;
  flex-grow: 0;
  width: fit-content !important;
}

.card-header :deep(.el-range-input) {
  width: 80px;
}

.main-card :deep(.el-card__header) {
  display: block;
}
</style>
