<template>
  <div class="scheduled-page">
    <svg style="display:none">
      <defs>
        <symbol id="icon-plus" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M5 12h14"/><path d="M12 5v14"/>
        </symbol>
        <symbol id="icon-play" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polygon points="5 3 19 12 5 21 5 3"/>
        </symbol>
        <symbol id="icon-edit" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
        </symbol>
        <symbol id="icon-list" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M8 6h13"/><path d="M8 12h13"/><path d="M8 18h13"/><path d="M3 6h.01"/><path d="M3 12h.01"/><path d="M3 18h.01"/>
        </symbol>
        <symbol id="icon-calendar-clock" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 7.5V6a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h4"/>
          <path d="M16 2v4"/><path d="M8 2v4"/><path d="M3 10h18"/>
          <circle cx="16" cy="16" r="5"/><path d="M16 13v3l2 1"/>
        </symbol>
        <symbol id="icon-trash" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/>
        </symbol>
      </defs>
    </svg>

    <div class="page-header">
      <p class="page-subtitle">统一管理保活和后续扩展的后台任务</p>
      <button class="action-icon primary" @click="handleAdd" title="添加定时任务">
        <svg width="20" height="20"><use href="#icon-plus"/></svg>
      </button>
    </div>

    <div class="list-container" v-loading="loading">
      <div v-if="tasks.length === 0" class="empty-state">
        <svg width="64" height="64" color="var(--color-border)"><use href="#icon-calendar-clock"/></svg>
        <p>暂无定时任务</p>
      </div>
      <div v-else class="task-table-wrap">
        <table class="flat-table task-table">
          <thead>
            <tr>
              <th>启用</th>
              <th>任务名称</th>
              <th>类型</th>
              <th>执行对象</th>
              <th>计划时间</th>
              <th>上次结果</th>
              <th>下次执行</th>
              <th>失败次数</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="task in tasks" :key="task.id">
              <td>
                <el-switch :model-value="task.enabled" @change="(value: string | number | boolean) => handleToggle(task, value as boolean)" />
              </td>
              <td class="task-name">{{ task.name }}</td>
              <td>{{ taskTypeLabel(task.task_type) }}</td>
              <td>{{ formatTarget(task) }}</td>
              <td>{{ task.schedule_expr }}</td>
              <td>
                <span class="pill" :class="statusClass(task.last_status)">{{ statusLabel(task.last_status) }}</span>
              </td>
              <td>{{ formatTime(task.next_run_at) }}</td>
              <td class="mono">{{ task.retry_count }}/{{ task.retry_limit }}</td>
              <td>
                <div class="row-actions">
                  <button class="action-icon success" title="立即执行" :disabled="runningIds.includes(task.id)" @click="handleRunNow(task)">
                    <svg width="16" height="16"><use href="#icon-play"/></svg>
                  </button>
                  <button class="action-icon" title="执行记录" @click="openRuns(task)">
                    <svg width="16" height="16"><use href="#icon-list"/></svg>
                  </button>
                  <button class="action-icon" title="编辑" @click="handleEdit(task)">
                    <svg width="16" height="16"><use href="#icon-edit"/></svg>
                  </button>
                  <button class="action-icon delete" title="删除" @click="handleDelete(task)">
                    <svg width="16" height="16"><use href="#icon-trash"/></svg>
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <AppModal v-model="showDialog" :title="editingTask ? '编辑定时任务' : '添加定时任务'" width="760px" @confirm="handleSave">
      <div class="form-grid">
        <div class="form-group">
          <label class="c-label">任务名称 <span class="required">*</span></label>
          <input v-model="form.name" class="b-input" placeholder="例如: Claude 默认服务商保活">
        </div>
        <div class="form-group">
          <label class="c-label">任务类型</label>
          <input class="b-input" value="服务商保活" disabled>
        </div>
      </div>

      <div class="form-grid">
        <div class="form-group">
          <label class="c-label">终端类型</label>
          <AppSelect
            :model-value="form.cli_type"
            :options="cliSelectOptions"
            width="100%"
            @update:model-value="value => form.cli_type = value as CliType"
          />
        </div>
        <div class="form-group">
          <label class="c-label">Profile</label>
          <AppSelect
            :model-value="form.profile"
            :options="profileSelectOptions"
            width="100%"
            :disabled="!showProfileSelect"
            @update:model-value="value => form.profile = value as ProviderProfile"
          />
        </div>
      </div>

      <div class="form-grid">
        <div class="form-group">
          <label class="c-label">模型名 <span class="required">*</span></label>
          <input v-model="form.model_name" class="b-input mono" placeholder="claude-sonnet-4-5">
        </div>
        <div class="form-group">
          <label class="c-label">每日执行时间</label>
          <div class="time-picker">
            <AppSelect
              :model-value="scheduleHour"
              :options="hourOptions"
              width="100%"
              @update:model-value="value => scheduleHour = String(value)"
            />
            <span class="time-separator">:</span>
            <AppSelect
              :model-value="scheduleMinute"
              :options="minuteOptions"
              width="100%"
              @update:model-value="value => scheduleMinute = String(value)"
            />
          </div>
        </div>
      </div>

      <div class="form-grid">
        <div class="form-group">
          <label class="c-label">失败重试次数</label>
          <input v-model.number="form.retry_limit" type="number" min="0" class="b-input">
        </div>
        <div class="form-group">
          <label class="c-label">重试间隔（分钟）</label>
          <input v-model.number="form.retry_interval_minutes" type="number" min="1" class="b-input">
        </div>
      </div>

      <div class="form-group">
        <label class="c-label">执行对象</label>
        <div class="b-segmented">
          <div class="b-seg-btn" :class="{ active: form.target_mode === 'all' }" @click="form.target_mode = 'all'">全部服务商</div>
          <div class="b-seg-btn" :class="{ active: form.target_mode === 'selected' }" @click="form.target_mode = 'selected'">指定服务商</div>
        </div>
      </div>

      <div v-if="form.target_mode === 'selected'" class="form-group provider-select-block" v-loading="providersLoading">
        <div class="provider-select-header">
          <label class="c-label">选择服务商</label>
          <span class="text-12 text-info fw-normal provider-toggle-all" @click="toggleAllProviders">
            {{ isAllProvidersSelected ? '取消全选' : '全选' }}
          </span>
        </div>
        <div class="provider-chip-list">
          <label
            v-for="provider in selectableProviders"
            :key="provider.id"
            class="provider-chip"
            :class="{ selected: form.provider_ids.includes(provider.id) }"
            @click="toggleProvider(provider.id)"
          >
            <div class="provider-check">
              <span v-if="form.provider_ids.includes(provider.id)" class="provider-checkmark">✓</span>
            </div>
            <span class="provider-name">{{ provider.name }}</span>
          </label>
        </div>
        <div v-if="selectableProviders.length === 0" class="text-muted text-14 provider-empty">
          当前终端和 Profile 下暂无已启用服务商
        </div>
      </div>

      <div class="form-footer-row">
        <span class="text-secondary">创建后由后台调度器按时间执行</span>
        <el-switch v-model="form.enabled" active-text="启用" inactive-text="停用" />
      </div>
    </AppModal>

    <AppModal v-model="runsVisible" title="执行记录" width="980px" :show-footer="false">
      <div class="runs-layout">
        <div class="runs-table" v-loading="runsLoading">
          <table class="flat-table compact-table">
            <thead>
              <tr>
                <th>时间</th>
                <th>触发</th>
                <th>结果</th>
                <th>成功</th>
                <th>失败</th>
                <th>跳过</th>
                <th>耗时</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="run in runs" :key="run.id" :class="{ selected: selectedRun?.id === run.id }" @click="selectRun(run)">
                <td>{{ formatTime(run.started_at) }}</td>
                <td>{{ run.trigger_type === 'manual' ? '手动' : '定时' }}</td>
                <td><span class="pill" :class="statusClass(run.status)">{{ statusLabel(run.status) }}</span></td>
                <td class="mono">{{ run.success_count }}</td>
                <td class="mono">{{ run.failure_count }}</td>
                <td class="mono">{{ run.skipped_count }}</td>
                <td class="mono">{{ run.elapsed_ms }}ms</td>
              </tr>
            </tbody>
          </table>
        </div>

        <div class="items-table" v-loading="itemsLoading">
          <table class="flat-table compact-table">
            <thead>
              <tr>
                <th>服务商</th>
                <th>模型</th>
                <th>状态</th>
                <th>状态码</th>
                <th>耗时</th>
                <th>错误</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in runItems" :key="item.id">
                <td>{{ item.provider_name }}</td>
                <td class="mono">{{ item.model_name }}</td>
                <td><span class="pill" :class="statusClass(item.status)">{{ statusLabel(item.status) }}</span></td>
                <td class="mono">{{ item.status_code || '-' }}</td>
                <td class="mono">{{ item.elapsed_ms }}ms</td>
                <td class="error-cell" :title="item.error_message || ''">{{ item.error_message || '-' }}</td>
              </tr>
            </tbody>
          </table>
          <div v-if="selectedRun && runItems.length === 0" class="text-muted empty-inline">暂无明细</div>
        </div>
      </div>
    </AppModal>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import AppModal from '@/components/AppModal.vue'
import AppSelect, { type AppSelectOption } from '@/components/AppSelect.vue'
import { scheduledTasksApi } from '@/api/scheduledTasks'
import { providersApi } from '@/api/providers'
import { confirm } from '@/utils/confirm'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'
import {
  CLI_LABELS,
  CLI_TABS,
  PROFILE_CAPABLE_CLI_TYPES,
  type CliType,
  type Provider,
  type ProviderKeepalivePayload,
  type ProviderProfile,
  type ScheduledTask,
  type ScheduledTaskRun,
  type ScheduledTaskRunItem,
  type ScheduledTaskStatus,
  type ScheduledTaskType
} from '@/types/models'

const profileTabs: { id: ProviderProfile; label: string }[] = [
  { id: 'default', label: '默认' },
  { id: 'profile1', label: 'Profile 1' },
  { id: 'profile2', label: 'Profile 2' },
  { id: 'profile3', label: 'Profile 3' }
]

const defaultModels: Record<CliType, string> = {
  claude_code: 'claude-sonnet-4-5',
  codex: 'gpt-5.5',
  gemini: 'gemini-3.1-pro-preview'
}

const hourOptions: AppSelectOption[] = Array.from({ length: 24 }, (_, hour) => {
  const value = String(hour).padStart(2, '0')
  return { label: value, value }
})
const minuteOptions: AppSelectOption[] = Array.from({ length: 12 }, (_, index) => {
  const value = String(index * 5).padStart(2, '0')
  return { label: value, value }
})

interface FormState {
  name: string
  enabled: boolean
  cli_type: CliType
  profile: ProviderProfile
  target_mode: 'all' | 'selected'
  provider_ids: number[]
  model_name: string
  schedule_expr: string
  retry_limit: number
  retry_interval_minutes: number
}

const cliSelectOptions: AppSelectOption[] = CLI_TABS.map(cli => ({ label: cli.label, value: cli.id }))
const profileSelectOptions: AppSelectOption[] = profileTabs.map(profile => ({ label: profile.label, value: profile.id }))
const tasks = ref<ScheduledTask[]>([])
const loading = ref(false)
const showDialog = ref(false)
const editingTask = ref<ScheduledTask | null>(null)
const providersLoading = ref(false)
const providerOptions = ref<Provider[]>([])
const runningIds = ref<number[]>([])
const scheduleHour = ref('09')
const scheduleMinute = ref('00')

const runsVisible = ref(false)
const runsLoading = ref(false)
const itemsLoading = ref(false)
const activeTask = ref<ScheduledTask | null>(null)
const runs = ref<ScheduledTaskRun[]>([])
const selectedRun = ref<ScheduledTaskRun | null>(null)
const runItems = ref<ScheduledTaskRunItem[]>([])

const form = ref<FormState>(defaultForm())

const showProfileSelect = computed(() => PROFILE_CAPABLE_CLI_TYPES.includes(form.value.cli_type))
const selectableProviders = computed(() => providerOptions.value.filter(provider => provider.enabled))
const isAllProvidersSelected = computed(() =>
  selectableProviders.value.length > 0 && form.value.provider_ids.length === selectableProviders.value.length
)

function defaultForm(): FormState {
  return {
    name: '',
    enabled: true,
    cli_type: 'claude_code',
    profile: 'default',
    target_mode: 'all',
    provider_ids: [],
    model_name: defaultModels.claude_code,
    schedule_expr: '09:00',
    retry_limit: 3,
    retry_interval_minutes: 10
  }
}

async function fetchTasks() {
  loading.value = true
  try {
    const { data } = await scheduledTasksApi.list()
    tasks.value = data
  } finally {
    loading.value = false
  }
}

async function fetchProviders() {
  providersLoading.value = true
  try {
    const profile = showProfileSelect.value ? form.value.profile : 'default'
    const { data } = await providersApi.list(form.value.cli_type, profile)
    providerOptions.value = data
    form.value.provider_ids = form.value.provider_ids.filter(id => data.some(p => p.id === id && p.enabled))
  } finally {
    providersLoading.value = false
  }
}

function toggleProvider(id: number) {
  const index = form.value.provider_ids.indexOf(id)
  if (index >= 0) {
    form.value.provider_ids.splice(index, 1)
  } else {
    form.value.provider_ids.push(id)
  }
}

function toggleAllProviders() {
  if (isAllProvidersSelected.value) {
    form.value.provider_ids = []
  } else {
    form.value.provider_ids = selectableProviders.value.map(provider => provider.id)
  }
}

function handleAdd() {
  editingTask.value = null
  form.value = defaultForm()
  setScheduleParts(form.value.schedule_expr)
  providerOptions.value = []
  showDialog.value = true
  void fetchProviders()
}

function handleEdit(task: ScheduledTask) {
  const payload = parsePayload(task)
  editingTask.value = task
  form.value = {
    name: task.name,
    enabled: task.enabled,
    cli_type: payload?.cli_type || 'claude_code',
    profile: payload?.profile || 'default',
    target_mode: payload?.target_mode || 'all',
    provider_ids: payload?.provider_ids || [],
    model_name: payload?.model_name || defaultModels.claude_code,
    schedule_expr: task.schedule_expr,
    retry_limit: task.retry_limit,
    retry_interval_minutes: task.retry_interval_minutes
  }
  setScheduleParts(task.schedule_expr)
  showDialog.value = true
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

  const payload: ProviderKeepalivePayload = {
    target_mode: form.value.target_mode,
    cli_type: form.value.cli_type,
    profile: showProfileSelect.value ? form.value.profile : 'default',
    model_name: form.value.model_name.trim()
  }
  if (form.value.target_mode === 'selected') {
    payload.provider_ids = [...form.value.provider_ids]
  }

  const input = {
    name: form.value.name.trim(),
    task_type: 'provider_keepalive' as ScheduledTaskType,
    enabled: form.value.enabled,
    schedule_type: 'daily' as const,
    schedule_expr: scheduleExpr(),
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

async function fetchRuns() {
  if (!activeTask.value) return
  runsLoading.value = true
  try {
    const { data } = await scheduledTasksApi.runs({ task_id: activeTask.value.id, page: 1, page_size: 30 })
    runs.value = data.items
    if (runs.value.length > 0) {
      await selectRun(runs.value[0])
    }
  } finally {
    runsLoading.value = false
  }
}

async function selectRun(run: ScheduledTaskRun) {
  selectedRun.value = run
  itemsLoading.value = true
  try {
    const { data } = await scheduledTasksApi.runItems(run.id)
    runItems.value = data
  } finally {
    itemsLoading.value = false
  }
}

function parsePayload(task: ScheduledTask): ProviderKeepalivePayload | null {
  try {
    return JSON.parse(task.payload_json) as ProviderKeepalivePayload
  } catch {
    return null
  }
}

function setScheduleParts(value: string) {
  const match = /^(\d{2}):(\d{2})$/.exec(value)
  scheduleHour.value = match?.[1] || '09'
  scheduleMinute.value = match?.[2] || '00'
}

function scheduleExpr(): string {
  return `${scheduleHour.value}:${scheduleMinute.value}`
}

function formatTarget(task: ScheduledTask): string {
  const payload = parsePayload(task)
  if (!payload) return '-'
  if (payload.target_mode === 'all') {
    const cli = payload.cli_type ? CLI_LABELS[payload.cli_type] : '-'
    const profile = profileTabs.find(p => p.id === payload.profile)?.label || '默认'
    return `${cli} / ${profile} / 全部`
  }
  return `指定 ${payload.provider_ids?.length || 0} 个服务商`
}

function taskTypeLabel(type: ScheduledTaskType): string {
  return type === 'provider_keepalive' ? '服务商保活' : type
}

function statusLabel(status: ScheduledTaskStatus | 'skipped'): string {
  const labels: Record<string, string> = {
    pending: '待执行',
    running: '执行中',
    success: '成功',
    partial_failed: '部分失败',
    failed: '失败',
    retrying: '重试中',
    skipped: '跳过'
  }
  return labels[status] || status
}

function statusClass(status: ScheduledTaskStatus | 'skipped'): string {
  if (status === 'success') return 'pill-green'
  if (status === 'failed') return 'pill-red'
  if (status === 'partial_failed' || status === 'retrying') return 'pill-blue'
  return 'pill-grey'
}

function formatTime(timestamp: number | null): string {
  if (!timestamp) return '-'
  return new Date(timestamp * 1000).toLocaleString()
}

watch(() => form.value.cli_type, (cliType, oldCliType) => {
  if (!PROFILE_CAPABLE_CLI_TYPES.includes(cliType)) {
    form.value.profile = 'default'
  }
  if (!oldCliType || form.value.model_name === defaultModels[oldCliType]) {
    form.value.model_name = defaultModels[cliType]
  }
  if (showDialog.value) void fetchProviders()
})

watch(() => form.value.profile, () => {
  if (showDialog.value) void fetchProviders()
})

onMounted(fetchTasks)
</script>

<style scoped>
.scheduled-page {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.task-table-wrap {
  flex: 1;
  overflow: auto;
  background: var(--color-bg);
  border-radius: 16px;
}

.task-table {
  min-width: 1100px;
}

.task-name {
  font-weight: var(--fw-600);
  color: var(--color-text);
}

.row-actions {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
}

.row-actions .action-icon {
  font-size: var(--fs-12);
}

.form-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

.provider-select-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.provider-select-header .c-label {
  margin-bottom: 0;
}

.provider-toggle-all {
  cursor: pointer;
}

.provider-chip-list {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  min-height: 34px;
}

.provider-chip {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  padding: 6px 12px;
  border-radius: 8px;
  transition: all 0.2s;
  user-select: none;
  color: var(--color-text-weak);
  border: 1px solid var(--color-border);
  background: var(--color-bg);
  max-width: 100%;
}

.provider-chip.selected {
  color: var(--color-text);
  border-color: var(--color-primary);
  background: var(--color-primary-5);
}

.provider-check {
  width: 16px;
  height: 16px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  flex-shrink: 0;
  border: 2px solid var(--color-border);
  background: transparent;
}

.provider-chip.selected .provider-check {
  border-color: var(--color-primary);
  background: var(--color-primary);
}

.provider-checkmark {
  color: var(--color-bg);
  font-size: var(--fs-12);
  font-weight: var(--fw-700);
  line-height: 1;
}

.provider-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.provider-empty {
  padding: 8px 0;
}

.time-picker {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
  align-items: center;
  gap: 10px;
}

.time-separator {
  color: var(--color-text-muted);
  font-size: var(--fs-20);
  line-height: 1;
  font-weight: var(--fw-600);
  padding-bottom: 2px;
}

.form-footer-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.runs-layout {
  display: grid;
  grid-template-columns: 1fr;
  gap: 18px;
}

.runs-table,
.items-table {
  overflow: auto;
  border: 1px solid var(--color-border-light);
  border-radius: 12px;
  max-height: 280px;
}

.compact-table {
  min-width: 860px;
}

.compact-table tr {
  cursor: pointer;
}

.compact-table tr.selected td {
  background: var(--color-primary-5);
}

.error-cell {
  max-width: 240px;
}

.empty-inline {
  padding: 16px;
  text-align: center;
}
</style>
