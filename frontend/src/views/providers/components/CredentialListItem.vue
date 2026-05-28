<template>
  <div class="credential-row" :style="{ borderBottom: isLast ? 'none' : '1px solid var(--color-bg-subtle)' }">
    <div class="credential-main">
      <div class="drag-handle" aria-label="拖拽排序">
        <div class="drag-dot"></div>
        <div class="drag-dot"></div>
        <div class="drag-dot"></div>
      </div>

      <div>
        <div class="credential-title-row">
          <div class="text-16 fw-medium text-primary">{{ credential.name }}</div>
          <div v-if="credential.is_active" class="tag tag-success">激活中</div>
        </div>
      </div>
    </div>

    <div class="credential-actions">
      <div class="action-icon" @click="$emit('edit', credential)" title="编辑">
        <svg width="18" height="18"><use href="#icon-edit"/></svg>
      </div>
      <div class="action-icon delete" @click="$emit('delete', credential)" title="删除">
        <svg width="18" height="18"><use href="#icon-trash"/></svg>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { OfficialCredential } from '@/types/models'

defineProps<{
  credential: OfficialCredential
  isLast: boolean
}>()

defineEmits<{
  edit: [credential: OfficialCredential]
  delete: [credential: OfficialCredential]
}>()
</script>

<style scoped>
.credential-row {
  padding: 24px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: var(--color-bg);
}
.credential-main { display: flex; align-items: center; gap: 16px; }
.credential-title-row { display: flex; align-items: center; gap: 12px; }
.credential-actions { display: flex; align-items: center; gap: 12px; }
.tag {
  padding: 4px 10px;
  border-radius: 999px;
  font-size: var(--fs-12);
  font-weight: var(--fw-400);
}
.tag-success { background: var(--color-success-10); color: var(--color-success); }
.drag-handle { display: flex; flex-direction: column; gap: 3px; cursor: grab; padding: 8px; margin-left: -8px; opacity: 0.3; transition: opacity 0.2s; }
.drag-handle:hover { opacity: 0.8; }
.drag-dot { width: 4px; height: 4px; border-radius: 50%; background: var(--color-text-muted); }
</style>
