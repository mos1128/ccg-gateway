<template>
  <AppModal v-model="visible" :title="title" width="720px" @confirm="$emit('confirm')">
    <div style="display: flex; gap: 32px; margin-bottom: 32px;">
      <div style="flex: 1;">
        <label class="c-label">服务商名称 <span class="required">*</span></label>
        <input type="text" v-model="form.name" class="b-input" placeholder="例如: OpenAI 官方">
      </div>
      <div style="flex: 1;">
        <label class="c-label">Base URL <span class="required">*</span></label>
        <input type="text" v-model="form.base_url" class="b-input" :placeholder="baseUrlPlaceholder">
      </div>
    </div>

    <div style="margin-bottom: 40px;">
      <label class="c-label">{{ activeCliType === 'claude_code' ? 'API Token' : 'API Key' }} <span class="required">*</span></label>
      <input type="text" v-model="form.api_key" class="b-input" placeholder="sk-...">
    </div>

    <div style="display: flex; gap: 32px; margin-bottom: 40px; padding: 32px 24px; background: var(--color-bg-page); border-radius: 12px; border: 1px solid var(--color-bg-subtle);">
      <div style="flex: 1;">
        <label class="c-label">失败鉴权阈值 (次)</label>
        <input type="number" v-model.number="form.failure_threshold" class="b-input">
      </div>
      <div style="flex: 1;">
        <label class="c-label">拉黑时长 (分钟)</label>
        <input type="number" v-model.number="form.blacklist_minutes" class="b-input">
      </div>
      <div style="flex: 1;">
        <label class="c-label">自定义 UA (选填)</label>
        <input type="text" v-model="form.custom_useragent" class="b-input" placeholder="留空则使用原始">
      </div>
    </div>

    <div style="margin-bottom: 40px;">
      <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px;">
        <div>
          <div class="text-16 fw-normal text-primary">模型映射</div>
          <div class="text-12 text-secondary" style="margin-top: 6px;">映射 CLI 请求的源模型名称为服务商模型</div>
        </div>
        <button class="b-button-outline text-14" style="padding: 6px 12px;" @click="$emit('add-model-map')">+ 添加</button>
      </div>

      <div style="display: flex; flex-direction: column; gap: 20px;">
        <div v-for="(map, index) in form.model_maps" :key="'map-' + index" style="display: flex; gap: 16px; align-items: center;">
          <input type="text" v-model="map.source_model" class="b-input" placeholder="CLI 源模型" style="flex: 1;">
          <div class="text-secondary fw-normal">→</div>
          <input type="text" v-model="map.target_model" class="b-input" placeholder="服务商模型" style="flex: 1;">
          <div class="b-button-icon" @click="$emit('remove-model-map', index)">×</div>
        </div>
      </div>
    </div>

    <div>
      <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px;">
        <div>
          <div class="text-16 fw-normal text-primary">模型黑名单</div>
          <div class="text-12 text-secondary" style="margin-top: 6px;">配置服务商不支持的模型名称</div>
        </div>
        <button class="b-button-outline text-14" style="padding: 6px 12px;" @click="$emit('add-model-blacklist')">+ 添加</button>
      </div>

      <div style="display: flex; flex-direction: column; gap: 20px;">
        <div v-for="(item, index) in form.model_blacklist" :key="'blk-' + index" style="display: flex; gap: 16px; align-items: center;">
          <input type="text" v-model="item.model_pattern" class="b-input" placeholder="模型名称" style="flex: 1;">
          <div class="b-button-icon" @click="$emit('remove-model-blacklist', index)">×</div>
        </div>
      </div>
    </div>
  </AppModal>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import AppModal from '@/components/AppModal.vue'
import type { CliType } from '@/types/models'

interface ProviderEditForm {
  name: string
  base_url: string
  api_key: string
  failure_threshold: number
  blacklist_minutes: number
  custom_useragent: string
  model_maps: Array<{ source_model: string; target_model: string; enabled: boolean }>
  model_blacklist: Array<{ model_pattern: string }>
}

const props = defineProps<{
  modelValue: boolean
  title: string
  form: ProviderEditForm
  activeCliType: CliType
  baseUrlPlaceholder: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  confirm: []
  'add-model-map': []
  'remove-model-map': [index: number]
  'add-model-blacklist': []
  'remove-model-blacklist': [index: number]
}>()

const visible = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value)
})
</script>

<style scoped>
.b-button-icon {
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  color: var(--color-text-muted);
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: 0.2s;
}
.b-button-icon:hover {
  background: var(--color-danger-light);
  color: var(--color-danger);
  border-color: var(--color-danger-muted);
}
</style>
