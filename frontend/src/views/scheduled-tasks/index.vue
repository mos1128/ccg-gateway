<template>
  <div class="st-page">
    <div v-loading="loading" class="v2-card st-shell">
      <div class="st-top">
        <button class="v2-btn v2-btn-sm v2-btn-primary" @click="handleAdd">
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="M12 5v14"/></svg>
          添加
        </button>
      </div>

      <V2Empty class="st-empty-card" v-if="tasks.length === 0" title="还没有定时任务" description="在闲置时段调用服务商，触发计费窗口更新或保活">
        <template #icon><svg width="40" height="40" viewBox="0 0 24 24"><path d="M21 7.5V6a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h4"/><path d="M16 2v4M8 2v4M3 10h18"/><circle cx="16" cy="16" r="5"/><path d="M16 13v3l2 1"/></svg></template>
      </V2Empty>

      <div v-else class="st-tablewrap">
        <table class="v2-table">
          <thead>
            <tr>
              <th>启用</th><th>任务名称</th><th>类型</th><th>执行计划</th><th>上次结果</th><th>下次执行</th><th>失败</th><th>操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="task in tasks" :key="task.id">
              <td><el-switch :model-value="task.enabled" @change="(v: string | number | boolean) => handleToggle(task, v as boolean)" /></td>
              <td class="st-name">{{ task.name }}</td>
              <td>{{ taskTypeLabel(task.task_type) }}</td>
              <td>{{ scheduleLabel(task) }}</td>
              <td><span class="v2-pill dot" :class="statusClass(task.last_status)">{{ statusLabel(task.last_status) }}</span></td>
              <td class="mono">{{ formatTime(task.next_run_at) }}</td>
              <td class="mono">{{ task.retry_count }}/{{ task.retry_limit }}</td>
              <td>
                <div class="st-acts">
                  <el-tooltip content="立即执行" placement="top" effect="light" :show-after="250">
                    <button class="v2-row-act" :disabled="runningIds.includes(task.id)" @click="handleRunNow(task)">
                      <svg v-if="runningIds.includes(task.id)" class="spin" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M21 12a9 9 0 1 1-6.219-8.56"/>
                      </svg>
                      <svg v-else width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <polygon points="6 4 20 12 6 20 6 4"/>
                      </svg>
                    </button>
                  </el-tooltip>
                  <el-tooltip content="执行记录" placement="top" effect="light" :show-after="250">
                    <button class="v2-row-act" @click="openRuns(task)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01"/></svg></button>
                  </el-tooltip>
                  <el-tooltip content="编辑" placement="top" effect="light" :show-after="250">
                    <button class="v2-row-act" @click="handleEdit(task)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.1 2.1 0 0 1 3 3L12 15l-4 1 1-4z"/></svg></button>
                  </el-tooltip>
                  <el-tooltip content="删除" placement="top" effect="light" :show-after="250">
                    <button class="v2-row-act danger" @click="handleDelete(task)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg></button>
                  </el-tooltip>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <V2Drawer v-model="showDialog" :title="editingTask ? '编辑定时任务' : '添加定时任务'" @confirm="handleSave">
      <div class="v2-grid-2">
        <div class="v2-field"><label class="v2-label">任务名称 <span class="req">*</span></label><input v-model="form.name" class="v2-input" placeholder="例如：Claude 默认服务商调用"></div>
        <div class="v2-field"><label class="v2-label">任务类型</label><input class="v2-input" value="服务商调用" disabled></div>
      </div>
      <div class="v2-grid-2">
        <div class="v2-field"><label class="v2-label">Agent 类型</label><AppSelect :model-value="form.cli_type" :options="cliSelectOptions" width="100%" @change="v => form.cli_type = v as CliType" /></div>
        <div class="v2-field"><label class="v2-label">Profile</label><AppSelect :model-value="form.profile" :options="profileSelectOptions" width="100%" @change="v => form.profile = v as ProviderProfile" /></div>
      </div>
      <div class="v2-grid-2">
        <div class="v2-field"><label class="v2-label">模型名 <span class="req">*</span></label><input v-model="form.model_name" class="v2-input mono" :placeholder="DEFAULT_MODEL_NAMES[form.cli_type]"></div>
        <div class="v2-field"><label class="v2-label">执行对象</label><AppSelect :model-value="form.target_mode" :options="targetModeOptions" width="100%" @change="v => form.target_mode = v as 'all' | 'selected'" /></div>
      </div>
      <div class="v2-grid-2">
        <div class="v2-field"><label class="v2-label">执行方式</label><AppSelect :model-value="form.schedule_type" :options="scheduleTypeOptions" width="100%" @change="v => form.schedule_type = v as ScheduledTaskScheduleType" /></div>
        <div v-if="form.schedule_type === 'interval'" class="v2-field"><label class="v2-label">执行间隔（分钟）</label><input v-model.number="form.interval_minutes" type="number" min="1" class="v2-input" placeholder="例如：60"></div>
        <div v-else class="v2-field">
          <label class="v2-label">定期执行</label>
          <div class="st-periodic">
            <span class="st-unit">每</span>
            <input v-model.number="form.period_days" type="number" min="1" max="365" class="v2-input st-days">
            <span class="st-unit">天</span>
            <AppSelect :model-value="form.period_hour" :options="hourOptions" width="100%" @change="v => form.period_hour = String(v)" />
            <span class="st-unit">:</span>
            <AppSelect :model-value="form.period_minute" :options="minuteOptions" width="100%" @change="v => form.period_minute = String(v)" />
          </div>
        </div>
      </div>
      <div class="v2-grid-2">
        <div class="v2-field"><label class="v2-label">目标失败重试次数</label><input v-model.number="form.retry_limit" type="number" min="0" class="v2-input"></div>
        <div class="v2-field"><label class="v2-label">重试间隔（分钟）</label><input v-model.number="form.retry_interval_minutes" type="number" min="1" class="v2-input"></div>
      </div>

      <div v-if="form.target_mode === 'selected'" class="v2-field" v-loading="providersLoading">
        <div class="st-prov-head">
          <label class="v2-label" style="margin:0">选择服务商</label>
          <span class="st-prov-all" @click="toggleAllProviders">{{ isAllProvidersSelected ? '取消全选' : '全选' }}</span>
        </div>
        <div class="st-prov-list">
          <button
            v-for="provider in selectableProviders"
            :key="provider.id"
            class="v2-chip"
            :class="{ on: form.provider_ids.includes(provider.id) }"
            type="button"
            @click="toggleProvider(provider.id)"
          >
            <span class="v2-chip-dot"></span>
            {{ provider.name }}
          </button>
        </div>
        <div v-if="selectableProviders.length === 0" class="v2-hint">当前 Agent 和 Profile 下暂无已启用服务商</div>
      </div>
    </V2Drawer>

    <V2Drawer v-model="runsVisible" title="执行记录" :show-footer="false" width="60%">
      <div class="st-runs" v-loading="runsLoading">
        <div class="st-subtitle">运行历史</div>
        <div class="st-table-wrap">
          <table class="v2-table">
            <thead><tr><th>时间</th><th>触发</th><th>结果</th><th>成功</th><th>失败</th><th>跳过</th><th>耗时</th></tr></thead>
            <tbody>
              <tr v-for="run in runs" :key="run.id" class="st-run-row" :class="{ on: selectedRun?.id === run.id }" @click="selectRun(run)">
                <td class="mono">{{ formatTime(run.started_at) }}</td>
                <td>{{ run.trigger_type === 'manual' ? '手动' : '定时' }}</td>
                <td><span class="v2-pill dot" :class="statusClass(run.status)">{{ statusLabel(run.status) }}</span></td>
                <td class="mono">{{ run.success_count }}</td>
                <td class="mono">{{ run.failure_count }}</td>
                <td class="mono">{{ run.skipped_count }}</td>
                <td class="mono">{{ run.elapsed_ms }}ms</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-if="runs.length === 0" class="v2-hint st-inline">暂无运行记录</div>
      </div>
      <div class="st-runs" v-loading="itemsLoading">
        <div class="st-subtitle">明细</div>
        <div class="st-table-wrap">
          <table class="v2-table">
            <thead><tr><th>服务商</th><th>模型</th><th>状态</th><th>状态码</th><th>耗时</th><th>错误</th></tr></thead>
            <tbody>
              <tr v-for="item in runItems" :key="item.id">
                <td>{{ item.provider_name }}</td>
                <td class="mono">{{ item.model_name }}</td>
                <td><span class="v2-pill dot" :class="statusClass(item.status)">{{ statusLabel(item.status) }}</span></td>
                <td class="mono">{{ item.status_code || '-' }}</td>
                <td class="mono">{{ item.elapsed_ms }}ms</td>
                 <td class="st-err">
                  <el-tooltip :content="item.error_message || ''" placement="top" effect="light" :disabled="!item.error_message" :show-after="250">
                    <span>{{ item.error_message || '-' }}</span>
                  </el-tooltip>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-if="selectedRun && runItems.length === 0" class="v2-hint st-inline">暂无明细</div>
      </div>
    </V2Drawer>
  </div>
</template>

<script setup lang="ts">
import V2Drawer from '@/components/V2Drawer.vue'
import V2Empty from '@/components/V2Empty.vue'
import AppSelect, { type AppSelectOption } from '@/components/AppSelect.vue'
import { scheduledTasksApi } from '@/api/scheduledTasks'
import { providersApi } from '@/api/providers'
import { confirm } from '@/utils/confirm'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'
import { DEFAULT_MODEL_NAMES, getReusableModelName } from '@/utils/modelDefaults'
import {
  CLI_TABS,
  PROFILE_CAPABLE_CLI_TYPES,
  type CliType,
  type Provider,
  type ProviderKeepalivePayload,
  type ProviderProfile,
  type ProviderProfileItem,
  type ScheduledTask,
  type ScheduledTaskRun,
  type ScheduledTaskRunItem,
  type ScheduledTaskScheduleType,
  type ScheduledTaskStatus,
  type ScheduledTaskType
} from '@/types/models'

const profileTabs = ref<ProviderProfileItem[]>([
  { cli_type: 'claude_code', name: 'default', label: '默认', is_default: true, sort_order: 0 }
])

const hourOptions: AppSelectOption[] = Array.from({ length: 24 }, (_, hour) => {
  const value = String(hour).padStart(2, '0')
  return { label: value, value }
})
const minuteOptions: AppSelectOption[] = Array.from({ length: 60 }, (_, minute) => {
  const value = String(minute).padStart(2, '0')
  return { label: value, value }
})
const targetModeOptions: AppSelectOption[] = [
  { label: '全部服务商', value: 'all' },
  { label: '指定服务商', value: 'selected' }
]
const scheduleTypeOptions: AppSelectOption[] = [
  { label: '执行间隔', value: 'interval' },
  { label: '定期执行', value: 'daily' }
]

interface FormState {
  name: string
  cli_type: CliType
  profile: ProviderProfile
  target_mode: 'all' | 'selected'
  provider_ids: number[]
  model_name: string
  schedule_type: ScheduledTaskScheduleType
  interval_minutes: number
  period_days: number
  period_hour: string
  period_minute: string
  retry_limit: number
  retry_interval_minutes: number
}

const cliSelectOptions: AppSelectOption[] = CLI_TABS.map(cli => ({ label: cli.label, value: cli.id }))
function profileDisplayLabel(profile: ProviderProfileItem) {
  if (profile.is_default) return profile.label
  return profile.label.replace(/_/g, ' ')
}

const profileSelectOptions = computed<AppSelectOption[]>(() =>
  profileTabs.value.map(profile => ({ label: profileDisplayLabel(profile), value: profile.name }))
)
const tasks = ref<ScheduledTask[]>([])
const loading = ref(false)
const showDialog = ref(false)
const editingTask = ref<ScheduledTask | null>(null)
const providersLoading = ref(false)
const providerOptions = ref<Provider[]>([])
const runningIds = ref<number[]>([])

const runsVisible = ref(false)
const runsLoading = ref(false)
const itemsLoading = ref(false)
const activeTask = ref<ScheduledTask | null>(null)
const runs = ref<ScheduledTaskRun[]>([])
const selectedRun = ref<ScheduledTaskRun | null>(null)
const runItems = ref<ScheduledTaskRunItem[]>([])

const form = ref<FormState>(defaultForm())
let hasLoadedTasks = false
let scheduledTaskListener: (() => void) | null = null

const supportsProfiles = computed(() => PROFILE_CAPABLE_CLI_TYPES.includes(form.value.cli_type))
const selectableProviders = computed(() => providerOptions.value.filter(p => p.enabled))
const isAllProvidersSelected = computed(() => selectableProviders.value.length > 0 && form.value.provider_ids.length === selectableProviders.value.length)

function defaultForm(): FormState {
  return {
    name: '', cli_type: 'claude_code', profile: 'default', target_mode: 'all', provider_ids: [],
    model_name: getReusableModelName('claude_code'), schedule_type: 'interval', interval_minutes: 60,
    period_days: 1, period_hour: '09', period_minute: '00', retry_limit: 3, retry_interval_minutes: 10
  }
}

async function fetchTasks(options: { silent?: boolean } = {}) {
  if (!options.silent) loading.value = true
  try {
    const { data } = await scheduledTasksApi.list()
    tasks.value = data
  } finally {
    if (!options.silent) loading.value = false
  }
}
async function refreshScheduledTasks() {
  await fetchTasks({ silent: hasLoadedTasks })
  hasLoadedTasks = true
  if (runsVisible.value && activeTask.value) await fetchRuns({ silent: true, preserveSelection: true })
}
async function fetchProviders() {
  providersLoading.value = true
  try {
    const profile = supportsProfiles.value ? form.value.profile : 'default'
    const { data } = await providersApi.list(form.value.cli_type, profile)
    providerOptions.value = data
    form.value.provider_ids = form.value.provider_ids.filter(id => data.some(p => p.id === id && p.enabled))
  } finally {
    providersLoading.value = false
  }
}

async function fetchProfiles() {
  if (!supportsProfiles.value) {
    profileTabs.value = [{ cli_type: form.value.cli_type, name: 'default', label: '默认', is_default: true, sort_order: 0 }]
    form.value.profile = 'default'
    return
  }
  const { data } = await providersApi.listProfiles(form.value.cli_type)
  profileTabs.value = data
  if (!profileTabs.value.some(profile => profile.name === form.value.profile)) {
    form.value.profile = 'default'
  }
}

function toggleProvider(id: number) {
  const i = form.value.provider_ids.indexOf(id)
  if (i >= 0) form.value.provider_ids.splice(i, 1)
  else form.value.provider_ids.push(id)
}
function toggleAllProviders() {
  form.value.provider_ids = isAllProvidersSelected.value ? [] : selectableProviders.value.map(p => p.id)
}
function handleAdd() {
  editingTask.value = null
  form.value = defaultForm()
  providerOptions.value = []
  showDialog.value = true
  void fetchProfiles()
  void fetchProviders()
}
function handleEdit(task: ScheduledTask) {
  const payload = parsePayload(task)
  const daily = parseDailySchedule(task.schedule_expr)
  editingTask.value = task
  form.value = {
    name: task.name,
    cli_type: payload?.cli_type || 'claude_code',
    profile: payload?.profile || 'default',
    target_mode: payload?.target_mode || 'all',
    provider_ids: payload?.provider_ids || [],
    model_name: payload?.model_name || getReusableModelName(payload?.cli_type || 'claude_code'),
    schedule_type: normalizeScheduleType(task.schedule_type),
    interval_minutes: parseIntervalMinutes(task),
    period_days: daily?.days || 1,
    period_hour: daily?.hour || '09',
    period_minute: daily?.minute || '00',
    retry_limit: task.retry_limit,
    retry_interval_minutes: task.retry_interval_minutes
  }
  showDialog.value = true
  void fetchProfiles()
  void fetchProviders()
}
async function handleSave() {
  if (!form.value.name.trim() || !form.value.model_name.trim()) {
    notify('请填写任务名称和模型名', 'error')
    return
  }
  if (form.value.target_mode === 'selected' && form.value.provider_ids.length === 0) {
    notify('请至少选择一个服务商', 'error')
    return
  }
  const scheduleInput = buildScheduleInput()
  if (!scheduleInput) return
  const modelName = form.value.model_name.trim()
  const payload: ProviderKeepalivePayload = {
    target_mode: form.value.target_mode,
    cli_type: form.value.cli_type,
    profile: supportsProfiles.value ? form.value.profile : 'default',
    model_name: modelName
  }
  if (form.value.target_mode === 'selected') payload.provider_ids = [...form.value.provider_ids]
  const input = {
    name: form.value.name.trim(),
    task_type: 'provider_keepalive' as ScheduledTaskType,
    enabled: editingTask.value?.enabled ?? true,
    schedule_type: scheduleInput.type,
    schedule_expr: scheduleInput.expr,
    payload_json: JSON.stringify(payload),
    retry_limit: Number(form.value.retry_limit),
    retry_interval_minutes: Number(form.value.retry_interval_minutes)
  }
  try {
    if (editingTask.value) {
      await scheduledTasksApi.update(editingTask.value.id, input)
      notify('任务已更新')
    } else {
      await scheduledTasksApi.create(input)
      notify('任务已创建')
    }
    showDialog.value = false
    await fetchTasks()
  } catch (e: any) {
    notify(getErrorMessage(e, '保存失败'), 'error')
  }
}
async function handleToggle(task: ScheduledTask, enabled: boolean) {
  try {
    await scheduledTasksApi.update(task.id, { enabled })
    task.enabled = enabled
    notify(enabled ? '已启用' : '已停用')
    await fetchTasks()
  } catch (e: any) {
    notify(getErrorMessage(e, '更新失败'), 'error')
  }
}
async function handleRunNow(task: ScheduledTask) {
  runningIds.value.push(task.id)
  try {
    const { data } = await scheduledTasksApi.runNow(task.id)
    notify(`执行完成：${statusLabel(data.status)}`)
    await fetchTasks()
  } catch (e: any) {
    notify(getErrorMessage(e, '执行失败'), 'error')
  } finally {
    runningIds.value = runningIds.value.filter(id => id !== task.id)
  }
}
async function handleDelete(task: ScheduledTask) {
  try {
    await confirm(`确定删除定时任务「${task.name}」吗？`, '删除定时任务')
    await scheduledTasksApi.delete(task.id)
    notify('已删除')
    await fetchTasks()
  } catch (e: any) {
    if (e === 'cancel') return
    notify(getErrorMessage(e, '删除失败'), 'error')
  }
}
async function openRuns(task: ScheduledTask) {
  activeTask.value = task
  runsVisible.value = true
  selectedRun.value = null
  runItems.value = []
  await fetchRuns()
}
async function fetchRuns(options: { silent?: boolean; preserveSelection?: boolean } = {}) {
  if (!activeTask.value) return
  if (!options.silent) runsLoading.value = true
  try {
    const { data } = await scheduledTasksApi.runs({ task_id: activeTask.value.id, page: 1, page_size: 30 })
    runs.value = data.items
    if (runs.value.length === 0) {
      selectedRun.value = null
      runItems.value = []
      return
    }
    const current = options.preserveSelection && selectedRun.value ? runs.value.find(r => r.id === selectedRun.value?.id) : null
    await selectRun(current || runs.value[0], { silent: options.silent })
  } finally {
    if (!options.silent) runsLoading.value = false
  }
}
async function selectRun(run: ScheduledTaskRun, options: { silent?: boolean } = {}) {
  selectedRun.value = run
  if (!options.silent) itemsLoading.value = true
  try {
    const { data } = await scheduledTasksApi.runItems(run.id)
    runItems.value = data
  } finally {
    if (!options.silent) itemsLoading.value = false
  }
}
function parsePayload(task: ScheduledTask): ProviderKeepalivePayload | null {
  try {
    return JSON.parse(task.payload_json) as ProviderKeepalivePayload
  } catch {
    return null
  }
}
function normalizeScheduleType(type: ScheduledTaskScheduleType): ScheduledTaskScheduleType {
  return type === 'daily' ? 'daily' : 'interval'
}
function parseIntervalMinutes(task: ScheduledTask): number {
  const minutes = Number(task.schedule_expr)
  return Number.isInteger(minutes) && minutes > 0 ? minutes : 60
}
interface DailySchedule { days: number; hour: string; minute: string }
function parseDailySchedule(scheduleExpr: string): DailySchedule | null {
  const value = scheduleExpr.trim()
  try {
    const data = JSON.parse(value) as { days?: number; hour?: number; minute?: number }
    const days = Number(data.days), hour = Number(data.hour), minute = Number(data.minute)
    if (isValidDailyParts(days, hour, minute)) return { days, hour: String(hour).padStart(2, '0'), minute: String(minute).padStart(2, '0') }
  } catch {
    const match = /^(\d{1,2}):(\d{1,2})$/.exec(value)
    if (match) {
      const hour = Number(match[1]), minute = Number(match[2])
      if (isValidDailyParts(1, hour, minute)) return { days: 1, hour: String(hour).padStart(2, '0'), minute: String(minute).padStart(2, '0') }
    }
  }
  return null
}
function isValidDailyParts(days: unknown, hour: unknown, minute: unknown): boolean {
  return Number.isInteger(days) && Number(days) >= 1 && Number(days) <= 365
    && Number.isInteger(hour) && Number(hour) >= 0 && Number(hour) <= 23
    && Number.isInteger(minute) && Number(minute) >= 0 && Number(minute) <= 59
}
function buildScheduleInput(): { type: ScheduledTaskScheduleType; expr: string } | null {
  if (form.value.schedule_type === 'interval') {
    const m = Number(form.value.interval_minutes)
    if (!Number.isInteger(m) || m <= 0) {
      notify('执行间隔必须是大于 0 的整数分钟', 'error')
      return null
    }
    return { type: 'interval', expr: String(m) }
  }
  const days = Number(form.value.period_days), hour = Number(form.value.period_hour), minute = Number(form.value.period_minute)
  if (!isValidDailyParts(days, hour, minute)) {
    notify('定期执行必须是 1-365 天、0-23 时、0-59 分', 'error')
    return null
  }
  return { type: 'daily', expr: JSON.stringify({ days, hour, minute }) }
}
function scheduleLabel(task: ScheduledTask): string {
  if (task.schedule_type === 'daily') {
    const s = parseDailySchedule(task.schedule_expr)
    return s ? `每 ${s.days} 天 ${s.hour}:${s.minute}` : task.schedule_expr
  }
  const minutes = Number(task.schedule_expr)
  if (!Number.isInteger(minutes) || minutes <= 0) return task.schedule_expr
  if (minutes % 1440 === 0) return `每 ${minutes / 1440} 天`
  if (minutes % 60 === 0) return `每 ${minutes / 60} 小时`
  return `每 ${minutes} 分钟`
}
function taskTypeLabel(type: ScheduledTaskType): string {
  return type === 'provider_keepalive' ? '服务商调用' : type
}
function statusLabel(status: ScheduledTaskStatus | 'skipped'): string {
  const labels: Record<string, string> = { pending: '待执行', running: '执行中', success: '成功', partial_failed: '部分失败', failed: '失败', retrying: '重试中', skipped: '全部跳过' }
  return labels[status] || status
}
function statusClass(status: ScheduledTaskStatus | 'skipped'): string {
  if (status === 'success') return 'v2-pill-success'
  if (status === 'failed') return 'v2-pill-danger'
  if (status === 'partial_failed' || status === 'retrying') return 'v2-pill-warn'
  if (status === 'running') return 'v2-pill-info'
  return 'v2-pill-neutral'
}
function formatTime(timestamp: number | null): string {
  if (!timestamp) return '-'
  return new Date(timestamp * 1000).toLocaleString()
}
function handleScheduledTaskChange() {
  void refreshScheduledTasks().catch((e) => notify(getErrorMessage(e, '定时任务刷新失败'), 'error'))
}

onMounted(async () => {
  try {
    if (supportsProfiles.value) await fetchProfiles()
    await fetchTasks()
    hasLoadedTasks = true
  } catch (e: any) {
    notify(getErrorMessage(e, '定时任务加载失败'), 'error')
  }
  try {
    scheduledTaskListener = await scheduledTasksApi.listenChanges(handleScheduledTaskChange)
  } catch (e: any) {
    notify(getErrorMessage(e, '定时任务事件监听失败'), 'error')
  }
})
onUnmounted(() => {
  if (scheduledTaskListener) {
    scheduledTaskListener()
    scheduledTaskListener = null
  }
})

watch(() => form.value.cli_type, async (cliType, oldCliType) => {
  if (!PROFILE_CAPABLE_CLI_TYPES.includes(cliType)) {
    form.value.profile = 'default'
  }
  if (!oldCliType || form.value.model_name === getReusableModelName(oldCliType)) {
    form.value.model_name = getReusableModelName(cliType)
  }
  if (showDialog.value) {
    await fetchProfiles()
    void fetchProviders()
  }
})
watch(() => form.value.profile, () => {
  if (showDialog.value) void fetchProviders()
})
</script>

<style scoped>
.st-page { flex: 1; min-height: 0; display: flex; flex-direction: column; }
.st-shell { flex: 1; min-height: 0; display: flex; flex-direction: column; padding: 0; overflow: hidden; }
.st-top { flex-shrink: 0; display: flex; justify-content: flex-end; padding: 13px 18px; border-bottom: 1px solid var(--v2-surface-2); }
.st-empty-card { flex: 1; min-height: 0; }
.st-tablewrap { flex: 1; min-height: 0; overflow: auto; scrollbar-gutter: stable; }
.st-tablewrap thead th { position: sticky; top: 0; z-index: 1; text-align: center; }
.st-tablewrap tbody td { text-align: center; }
.st-name { font-weight: var(--v2-fw-medium); }
.st-acts { display: flex; align-items: center; justify-content: center; gap: 2px; }
.st-acts .v2-row-act svg { fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }

.st-empty { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 9px; padding: 60px 24px; text-align: center; color: var(--v2-text-3); }
.st-empty-t { font-size: var(--v2-fs-md); font-weight: var(--v2-fw-medium); color: var(--v2-text-2); }
.st-empty-s { font-size: var(--v2-fs-sm); color: var(--v2-text-3); max-width: 320px; line-height: 1.5; margin-bottom: 8px; }

.st-periodic { display: grid; grid-template-columns: auto minmax(56px, 1fr) auto 1fr auto 1fr; align-items: center; gap: 8px; }
.st-unit { color: var(--v2-text-3); font-size: var(--v2-fs-sm); white-space: nowrap; }
.st-days { min-width: 0; }

.st-prov-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
.st-prov-all { cursor: pointer; font-size: var(--v2-fs-xs); color: var(--v2-accent); }
.st-prov-list { display: flex; gap: 8px; flex-wrap: wrap; }

.st-runs { margin-bottom: 18px; }
.st-runs:last-child { margin-bottom: 0; }
.st-subtitle { font-size: var(--v2-fs-xs); font-weight: var(--v2-fw-semibold); color: var(--v2-text-3); margin-bottom: 8px; }
.st-table-wrap { overflow-x: auto; border: 1px solid var(--v2-surface-3); border-radius: var(--v2-r); }
.st-runs .v2-table th,
.st-runs .v2-table td {
  padding: 8px 10px;
  text-align: center;
}
.st-run-row { cursor: pointer; }
.st-run-row.on td { background: var(--v2-surface-2); }
.st-err { max-width: 220px; overflow: hidden; text-overflow: ellipsis; text-align: left; }
.st-inline { padding: 14px; text-align: center; }
</style>
