<template>
  <AppModal
    v-model="visible"
    title="检测模型可用性"
    width="800px"
    :show-footer="true"
    confirm-text="开始检测"
    @confirm="$emit('confirm')"
  >
    <div style="display: flex; gap: 12px; align-items: flex-end; margin-bottom: 24px;">
      <div style="flex: 1;">
        <label class="c-label">检测模型</label>
        <input type="text" :value="model" class="b-input" placeholder="输入模型名称" @input="handleModelInput">
      </div>
    </div>

    <div style="margin-bottom: 24px;">
      <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;">
        <label class="c-label" style="margin-bottom: 0;">选择服务商</label>
        <span class="text-12 text-info fw-normal" style="cursor: pointer;" @click="$emit('toggle-all')">
          {{ isAllSelected ? '取消全选' : '全选' }}
        </span>
      </div>
      <div style="display: flex; gap: 10px; flex-wrap: wrap;">
        <label
          v-for="provider in providers"
          :key="provider.id"
          class="text-14"
          style="display: flex; align-items: center; gap: 6px; cursor: pointer; padding: 6px 12px; border-radius: 8px; transition: all 0.2s; user-select: none;"
          :style="providerStyle(provider.id)"
          @click="$emit('toggle-provider', provider.id)"
        >
          <div
            style="width: 16px; height: 16px; border-radius: 4px; display: flex; align-items: center; justify-content: center; transition: all 0.2s; flex-shrink: 0;"
            :style="checkboxStyle(provider.id)"
          >
            <span v-if="selectedIds.includes(provider.id)" class="text-12 fw-bold" style="color: var(--color-bg);">✓</span>
          </div>
          {{ provider.name }}
        </label>
      </div>
      <div v-if="providers.length === 0" class="text-muted text-14" style="padding: 8px 0;">
        当前 CLI 类型无已启用的服务商
      </div>
    </div>

    <div v-if="results.length > 0 || loading" style="border: 1px solid var(--color-border); border-radius: 12px; overflow: hidden; box-shadow: 0 4px 15px var(--color-shadow);">
      <table class="flat-table">
        <colgroup>
          <col style="width: 20%;"><col style="width: 25%;"><col style="width: 12%;"><col style="width: 13%;"><col style="width: 30%;">
        </colgroup>
        <thead>
          <tr>
            <th>服务商</th><th>测试模型</th><th>状态码</th><th>耗时</th><th>响应</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="result in results" :key="result.provider_id">
            <td class="fw-normal">{{ result.provider_name }}</td>
            <td class="mono">{{ result.actual_model }}</td>
            <td>
              <span v-if="result.status_code === null && result.elapsed_ms === 0" class="pill pill-grey">...</span>
              <span v-else-if="result.status_code !== null" :class="['pill', getPillClass(result.status_code)]">{{ result.status_code }}</span>
              <span v-else class="pill pill-red">ERR</span>
            </td>
            <td class="mono">
              <span v-if="result.status_code === null && result.elapsed_ms === 0">-</span>
              <span v-else>{{ result.elapsed_ms }}ms</span>
            </td>
            <td :style="{ color: responseColor(result) }">
              <span v-if="result.status_code === null && result.elapsed_ms === 0" style="font-style: italic;">Testing...</span>
              <el-tooltip v-else effect="light" placement="top" :enterable="true" :show-after="200">
                <template #content>
                  <div class="text-14" style="max-width: 350px; line-height: 1.6; word-break: break-word; user-select: text; color: var(--color-text-dark);">
                    {{ result.response_text }}
                  </div>
                </template>
                <span style="cursor: pointer;" @click="$emit('copy-response', result.response_text)">{{ result.response_text }}</span>
              </el-tooltip>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </AppModal>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import AppModal from '@/components/AppModal.vue'
import type { Provider, TestProviderResult } from '@/types/models'

const props = defineProps<{
  modelValue: boolean
  model: string
  providers: Provider[]
  selectedIds: number[]
  isAllSelected: boolean
  loading: boolean
  results: TestProviderResult[]
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  'update:model': [value: string]
  confirm: []
  'toggle-all': []
  'toggle-provider': [id: number]
  'copy-response': [text: string]
}>()

const visible = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value)
})

function handleModelInput(event: Event) {
  emit('update:model', (event.target as HTMLInputElement).value)
}

function providerStyle(id: number) {
  const selected = props.selectedIds.includes(id)
  return {
    color: selected ? 'var(--color-text)' : 'var(--color-text-weak)',
    border: selected ? '1px solid var(--color-primary)' : '1px solid var(--color-border)',
    background: selected ? 'var(--color-primary-5)' : 'var(--color-bg)'
  }
}

function checkboxStyle(id: number) {
  const selected = props.selectedIds.includes(id)
  return {
    border: selected ? '2px solid var(--color-primary)' : '2px solid var(--color-border)',
    background: selected ? 'var(--color-primary)' : 'transparent'
  }
}

function getPillClass(code: number | null): string {
  if (!code) return 'pill-grey'
  if (code >= 200 && code < 300) return 'pill-green'
  if (code >= 400 && code < 500) return 'pill-grey'
  if (code >= 500) return 'pill-red'
  return 'pill-grey'
}

function responseColor(result: TestProviderResult): string {
  if (result.status_code !== null && result.status_code >= 200 && result.status_code < 300) {
    return 'var(--color-text-muted)'
  }
  if (result.status_code === null && result.elapsed_ms === 0) {
    return 'var(--color-text-weak)'
  }
  return 'var(--color-error)'
}
</script>
