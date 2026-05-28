<template>
  <div
    class="provider-row"
    :class="{ blacklisted: provider.is_blacklisted }"
    :style="{ borderBottom: isLast ? 'none' : '1px solid var(--color-bg-subtle)' }"
  >
    <div class="provider-main">
      <div class="drag-handle" aria-label="拖拽排序">
        <div class="drag-dot"></div>
        <div class="drag-dot"></div>
        <div class="drag-dot"></div>
      </div>

      <div class="provider-info">
        <div class="provider-title-row">
          <div class="text-16 fw-medium text-primary provider-name" :class="{ disabled: !provider.enabled }">
            {{ provider.name }}
          </div>
          <div v-if="provider.is_blacklisted" class="tag tag-error">
            {{ unblacklistText }}
          </div>
          <div v-else-if="!provider.enabled" class="tag tag-muted">已禁用</div>
          <div v-if="provider.model_maps.length > 0" class="tag tag-success">
            {{ modelMapsText }}
          </div>
          <div v-if="provider.model_blacklist?.length" class="tag tag-warning">
            {{ provider.model_blacklist.length }}个黑名单配置
          </div>
        </div>
      </div>
    </div>

    <div class="provider-side">
      <div class="failure-box">
        <div class="text-12 text-muted failure-label">失败阈值</div>
        <div
          class="mono text-16 failure-value"
          :class="{ danger: provider.consecutive_failures >= provider.failure_threshold }"
        >
          {{ provider.consecutive_failures }}/{{ provider.failure_threshold }}
        </div>
      </div>

      <div class="provider-actions">
        <el-switch :model-value="provider.enabled" :loading="toggleLoading" @change="emitToggle" />

        <div class="icon-row">
          <div class="action-icon" @click="$emit('copy', provider)" title="复制">
            <svg width="18" height="18"><use href="#icon-copy"/></svg>
          </div>
          <div class="action-icon" @click="$emit('edit', provider)" title="编辑">
            <svg width="18" height="18"><use href="#icon-edit"/></svg>
          </div>
          <div class="action-icon" @click="$emit('reset', provider)" title="重置并解除拉黑">
            <svg width="18" height="18"><use href="#icon-refresh"/></svg>
          </div>
          <div class="action-icon delete" @click="$emit('delete', provider)" title="删除">
            <svg width="18" height="18"><use href="#icon-trash"/></svg>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { Provider } from '@/types/models'

const props = defineProps<{
  provider: Provider
  isLast: boolean
  modelMapsText: string
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

function emitToggle(value: string | number | boolean) {
  emit('toggle', { provider: props.provider, enabled: Boolean(value) })
}
</script>

<style scoped>
.provider-row {
  padding: 24px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: var(--color-bg);
}
.provider-row.blacklisted { background: var(--color-error-2); }
.provider-main { display: flex; align-items: center; gap: 16px; flex: 1; min-width: 0; }
.provider-info { flex: 1; min-width: 0; }
.provider-title-row { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
.provider-name { white-space: nowrap; }
.provider-name.disabled { color: var(--color-text-weak); }
.provider-side { display: flex; align-items: center; gap: 40px; flex-shrink: 0; margin-left: 24px; }
.failure-box { display: flex; flex-direction: column; align-items: center; min-width: 64px; }
.failure-label { margin-bottom: 2px; white-space: nowrap; }
.failure-value { font-weight: var(--fw-500); color: var(--color-text); }
.failure-value.danger { color: var(--color-danger); }
.provider-actions { display: flex; align-items: center; gap: 24px; }
.icon-row { display: flex; align-items: center; gap: 8px; }
.tag {
  padding: 4px 10px;
  border-radius: 999px;
  font-size: var(--fs-12);
  font-weight: var(--fw-400);
  white-space: nowrap;
}
.tag-error { background: var(--color-error-10); color: var(--color-error); }
.tag-muted { background: var(--color-bg-subtle); color: var(--color-text-muted); }
.tag-success { background: var(--color-success-10); color: var(--color-success); }
.tag-warning { background: var(--color-warning-10); color: var(--color-warning); }
.drag-handle { display: flex; flex-direction: column; gap: 3px; cursor: grab; padding: 8px; margin-left: -8px; opacity: 0.3; transition: opacity 0.2s; flex-shrink: 0; }
.drag-handle:hover { opacity: 0.8; }
.drag-dot { width: 4px; height: 4px; border-radius: 50%; background: var(--color-text-muted); }
</style>
