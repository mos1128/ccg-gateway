<template>
  <div class="v2-empty">
    <div class="v2-empty-icon"><slot name="icon" /></div>
    <div class="v2-empty-title">{{ title }}</div>
    <div v-if="description" class="v2-empty-desc">{{ description }}</div>
    <div v-if="$slots.action" class="v2-empty-action"><slot name="action" /></div>
  </div>
</template>

<script setup lang="ts">
defineProps<{ title: string; description?: string }>()
</script>

<style scoped>
.v2-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 9px;
  padding: 60px 24px;
  text-align: center;
}
.v2-empty-icon { color: var(--v2-text-3); opacity: 0.55; margin-bottom: 3px; }
.v2-empty-icon :deep(svg) { display: block; fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
.v2-empty-title { font-size: var(--v2-fs-md); font-weight: 600; color: var(--v2-text-2); }
.v2-empty-desc { font-size: var(--v2-fs-sm); color: var(--v2-text-3); max-width: 340px; line-height: 1.5; }
.v2-empty-action { margin-top: 6px; }
</style>
