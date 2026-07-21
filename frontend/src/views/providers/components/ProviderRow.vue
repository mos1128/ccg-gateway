<template>
  <div class="pt-row" :class="{ bl: provider.is_blacklisted, off: !provider.enabled }">
    <div class="pt-drag" aria-label="拖拽排序">
      <svg width="10" height="16" viewBox="0 0 10 16" fill="currentColor"><circle cx="3" cy="3" r="1.5"/><circle cx="3" cy="8" r="1.5"/><circle cx="3" cy="13" r="1.5"/><circle cx="7" cy="3" r="1.5"/><circle cx="7" cy="8" r="1.5"/><circle cx="7" cy="13" r="1.5"/></svg>
    </div>
    <div class="pt-cols m-route">
      <div class="pt-switch">
        <el-switch :model-value="provider.enabled" :loading="toggleLoading" @change="onToggleChange" />
      </div>
      <div class="pt-name" :class="{ off: !provider.enabled }">
        <OverflowText :text="provider.name" />
      </div>
      <div class="pt-cell pt-endpoint pt-col-endpoint mono"><OverflowText :text="provider.base_url" /></div>
      <div>
        <el-tooltip :content="statusTitle" placement="top" effect="light" :disabled="!provider.is_blacklisted" :show-after="250">
          <span class="v2-pill dot pt-status" :class="[health.cls, { 'pt-status-clickable': provider.is_blacklisted }]" @click="onStatusClick">{{ health.text }}</span>
        </el-tooltip>
      </div>
      <div class="pt-fail mono" :class="{ danger: failDanger }">{{ provider.consecutive_failures }}/{{ provider.failure_threshold }}</div>
      <div class="pt-cell mono pt-col-map" :class="{ muted: !mappingText }"><OverflowText :text="mappingText || '—'" /></div>
      <div class="pt-acts">
        <el-tooltip content="复制" placement="top" effect="light" :show-after="250">
          <button class="pt-act" @click="emit('copy', provider)"><svg width="16" height="16"><use href="#v2i-copy"/></svg></button>
        </el-tooltip>
        <el-tooltip content="编辑" placement="top" effect="light" :show-after="250">
          <button class="pt-act" @click="emit('edit', provider)"><svg width="16" height="16"><use href="#v2i-edit"/></svg></button>
        </el-tooltip>
        <el-tooltip content="重置并解除熔断" placement="top" effect="light" :show-after="250">
          <button class="pt-act" @click="emit('reset', provider)"><svg width="16" height="16"><use href="#v2i-refresh"/></svg></button>
        </el-tooltip>
        <el-tooltip content="删除" placement="top" effect="light" :show-after="250">
          <button class="pt-act danger" @click="emit('delete', provider)"><svg width="16" height="16"><use href="#v2i-trash"/></svg></button>
        </el-tooltip>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import OverflowText from './OverflowText.vue'
import type { Provider } from '@/types/models'

const props = defineProps<{
  provider: Provider
  unblacklistText: string
  toggleLoading?: boolean
}>()

const emit = defineEmits<{
  copy: [provider: Provider]
  edit: [provider: Provider]
  reset: [provider: Provider]
  delete: [provider: Provider]
  toggle: [payload: { provider: Provider; enabled: boolean }]
}>()

const failDanger = computed(() => props.provider.consecutive_failures >= props.provider.failure_threshold)
const mappingText = computed(() => {
  if (props.provider.model_maps?.length) return props.provider.model_maps.map((m) => m.target_model).join('、')
  if (props.provider.model_blacklist?.length) return `${props.provider.model_blacklist.length} 个黑名单`
  return ''
})

const health = computed(() => {
  if (props.provider.is_blacklisted) return { cls: 'v2-pill-danger', text: '熔断' }
  if (!props.provider.enabled) return { cls: 'v2-pill-neutral', text: '停用' }
  return { cls: 'v2-pill-success', text: '正常' }
})

const statusTitle = computed(() => {
  if (props.provider.is_blacklisted) return `${props.unblacklistText || '熔断中'} · 点击解除熔断`
  return ''
})

function onStatusClick() {
  if (props.provider.is_blacklisted) emit('reset', props.provider)
}
function onToggleChange(value: string | number | boolean) {
  if (props.toggleLoading) return
  emit('toggle', { provider: props.provider, enabled: value === true })
}
</script>
