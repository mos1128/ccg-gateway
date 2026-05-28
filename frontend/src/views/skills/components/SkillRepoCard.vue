<template>
  <div class="repo-card" @click="$emit('open', repo)">
    <div class="repo-icon-box">
      <svg width="24" height="24"><use href="#icon-store"/></svg>
    </div>
    <div class="repo-info-main">
      <div class="repo-name-title">{{ repo.name }}</div>
      <div class="repo-source-subtitle mono">{{ repo.source }}</div>
    </div>
    <div class="repo-actions-overlay" @click.stop>
      <button class="action-icon" title="重装仓库" :disabled="loading" @click="$emit('reinstall', repo)">
        <svg width="18" height="18"><use href="#icon-refresh"/></svg>
      </button>
      <button class="action-icon delete" title="删除" @click="$emit('remove', repo)">
        <svg width="18" height="18"><use href="#icon-trash"/></svg>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { SkillRepo } from '@/types/models'

defineProps<{
  repo: SkillRepo
  loading: boolean
}>()

defineEmits<{
  open: [repo: SkillRepo]
  reinstall: [repo: SkillRepo]
  remove: [repo: SkillRepo]
}>()
</script>

<style scoped>
.repo-card {
  background: var(--color-bg);
  border-radius: 16px;
  border: 1px solid var(--color-bg-subtle);
  padding: 20px;
  cursor: pointer;
  position: relative;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  gap: 16px;
}
.repo-card:hover { border-color: var(--color-primary); background: var(--color-bg-page); }
.repo-icon-box {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  background: var(--color-bg-subtle);
  color: var(--color-text-muted);
  display: flex;
  align-items: center;
  justify-content: center;
}
.repo-info-main { flex: 1; min-width: 0; }
.repo-name-title {
  font-weight: var(--fw-700);
  font-size: var(--fs-14);
  color: var(--color-text);
  margin-bottom: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}
.repo-source-subtitle {
  font-size: var(--fs-12);
  color: var(--color-text-weak);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.repo-actions-overlay { display: flex; gap: 4px; flex-shrink: 0; }
</style>
