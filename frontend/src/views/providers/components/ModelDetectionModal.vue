<template>
  <V2Drawer
    v-model="visible"
    title="检测模型可用性"
    width="60%"
    :show-footer="true"
    confirm-text="开始检测"
    :confirm-disabled="loading"
    @confirm="$emit('confirm')"
  >
    <div style="display: flex; gap: 12px; align-items: flex-end; margin-bottom: 24px;">
      <div style="flex: 1;">
        <label class="c-label">检测模型</label>
        <input type="text" :value="model" class="v2-input" placeholder="输入模型名称" @input="handleModelInput">
      </div>
      <div style="flex: 1;">
        <label class="c-label">测试文本</label>
        <input type="text" :value="testText" class="v2-input" placeholder="输入测试文本" @input="handleTestTextInput">
      </div>
    </div>

    <div style="margin-bottom: 24px;">
      <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;">
        <label class="c-label" style="margin-bottom: 0;">选择服务商</label>
      </div>
      <div v-if="providers.length > 0" class="det-provider-groups">
        <section v-for="group in providerGroups" :key="String(group.enabled)" class="det-provider-group" :aria-label="`${group.label}服务商`">
          <div class="det-provider-group-head">
            <div class="det-provider-group-title">
              <span class="v2-pill dot" :class="group.enabled ? 'v2-pill-success' : 'v2-pill-neutral'">{{ group.label }}</span>
              <span class="text-muted">已选 {{ group.selectedCount }} / {{ group.providers.length }}</span>
            </div>
            <button
              type="button"
              class="v2-btn v2-btn-sm v2-btn-ghost"
              :disabled="group.providers.length === 0"
              :aria-label="`${group.allSelected ? '取消全选' : '全选'}${group.label}服务商`"
              @click="$emit('toggle-group', group.enabled)"
            >
              {{ group.allSelected ? '取消全选' : '全选' }}
            </button>
          </div>
          <div v-if="group.providers.length > 0" class="v2-chip-row">
            <button
              v-for="provider in group.providers"
              :key="provider.id"
              type="button"
              class="v2-chip"
              :class="{ on: selectedIds.includes(provider.id) }"
              :aria-pressed="selectedIds.includes(provider.id)"
              @click="$emit('toggle-provider', provider.id)"
            >
              <span class="v2-chip-dot"></span>{{ provider.name }}
            </button>
          </div>
          <div v-else class="text-muted text-14">暂无{{ group.label }}的服务商</div>
        </section>
      </div>
      <div v-else class="text-muted text-14" style="padding: 8px 0;">
        当前 Agent 类型暂无服务商
      </div>
    </div>

    <div v-if="results.length > 0 || loading" class="det-results">
      <div class="det-subtitle">检测结果</div>
      <div class="st-table-wrap">
        <table class="v2-table">
          <thead>
            <tr>
              <th>服务商</th>
              <th>测试模型</th>
              <th>状态</th>
              <th>状态码</th>
              <th>耗时</th>
              <th>响应</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="result in results" :key="result.provider_id">
              <td>{{ result.provider_name }}</td>
              <td class="mono">{{ result.actual_model }}</td>
              <td>
                <span class="v2-pill dot" :class="statusClass(getResultStatus(result))">
                  {{ statusLabel(getResultStatus(result)) }}
                </span>
              </td>
              <td class="mono">{{ result.status_code || '-' }}</td>
              <td class="mono">
                <span v-if="result.status_code === null && result.elapsed_ms === 0">-</span>
                <span v-else>{{ result.elapsed_ms }}ms</span>
              </td>
              <td class="st-err">
                <span v-if="result.status_code === null && result.elapsed_ms === 0" style="font-style: italic; color: var(--v2-text-3);">Testing...</span>
                <el-tooltip v-else :content="result.response_text || ''" placement="top" effect="light" :disabled="!result.response_text" :show-after="250">
                  <span style="cursor: pointer;" @click="$emit('copy-response', result.response_text || '')">
                    {{ result.response_text || '-' }}
                  </span>
                </el-tooltip>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </V2Drawer>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import V2Drawer from '@/components/V2Drawer.vue'
import type { Provider, TestProviderResult } from '@/types/models'

const props = defineProps<{
  modelValue: boolean
  model: string
  testText: string
  providers: Provider[]
  selectedIds: number[]
  loading: boolean
  results: TestProviderResult[]
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  'update:model': [value: string]
  'update:testText': [value: string]
  confirm: []
  'toggle-group': [enabled: boolean]
  'toggle-provider': [id: number]
  'copy-response': [text: string]
}>()

const visible = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value)
})

const providerGroups = computed(() => [true, false].map((enabled) => {
  const providers = props.providers.filter((provider) => provider.enabled === enabled)
  const selectedCount = providers.filter((provider) => props.selectedIds.includes(provider.id)).length
  return {
    enabled,
    label: enabled ? '已启用' : '未启用',
    providers,
    selectedCount,
    allSelected: providers.length > 0 && selectedCount === providers.length
  }
}))

function handleModelInput(event: Event) {
  emit('update:model', (event.target as HTMLInputElement).value)
}

function handleTestTextInput(event: Event) {
  emit('update:testText', (event.target as HTMLInputElement).value)
}

function getResultStatus(result: TestProviderResult): 'pending' | 'success' | 'failed' {
  if (result.status_code === null && result.elapsed_ms === 0) return 'pending'
  if (result.status_code !== null && result.status_code >= 200 && result.status_code < 300) return 'success'
  return 'failed'
}

function statusLabel(status: 'pending' | 'success' | 'failed'): string {
  const labels = { pending: '检测中', success: '成功', failed: '失败' }
  return labels[status]
}

function statusClass(status: 'pending' | 'success' | 'failed'): string {
  if (status === 'success') return 'v2-pill-success'
  if (status === 'failed') return 'v2-pill-danger'
  return 'v2-pill-neutral'
}
</script>

<style scoped>
.c-label { display: block; font-size: var(--v2-fs-sm); font-weight: var(--v2-fw-medium); color: var(--v2-text-2); margin-bottom: 7px; }
.text-14 { font-size: var(--v2-fs-base); }
.text-muted { color: var(--v2-text-3); }

.det-provider-groups { display: grid; gap: 12px; }
.det-provider-group { padding: 14px; border: 1px solid var(--v2-surface-3); border-radius: var(--v2-r); }
.det-provider-group-head { display: flex; flex-wrap: wrap; justify-content: space-between; align-items: center; gap: 8px; margin-bottom: 12px; }
.det-provider-group-title { display: flex; align-items: center; gap: 10px; font-size: var(--v2-fs-xs); }

.det-results { margin-top: 18px; }
.det-subtitle { font-size: var(--v2-fs-xs); font-weight: var(--v2-fw-semibold); color: var(--v2-text-3); margin-bottom: 8px; }
.st-table-wrap { overflow-x: auto; border: 1px solid var(--v2-surface-3); border-radius: var(--v2-r); }
.det-results .v2-table th,
.det-results .v2-table td {
  padding: 8px 10px;
  text-align: center;
}
.st-err { max-width: 220px; overflow: hidden; text-overflow: ellipsis; text-align: left; }
</style>
