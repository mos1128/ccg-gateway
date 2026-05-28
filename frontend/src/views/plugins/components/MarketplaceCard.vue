<template>
  <div class="repo-card" @click="$emit('open', marketplace)">
    <div class="repo-icon-box">
      <svg width="24" height="24"><use href="#icon-store"/></svg>
    </div>
    <div class="repo-info-main">
      <div class="repo-name-title">{{ marketplace.name }}</div>
      <div class="repo-source-subtitle mono">{{ marketplace.marketplace_source || '内建市场' }}</div>
    </div>
    <div class="repo-actions-overlay" @click.stop>
      <button class="action-icon" title="同步市场" @click="$emit('update', marketplace)">
        <svg width="18" height="18"><use href="#icon-refresh"/></svg>
      </button>
      <button class="action-icon delete" title="删除市场" @click="$emit('remove', marketplace)">
        <svg width="18" height="18"><use href="#icon-trash"/></svg>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { MarketplaceInfo } from '@/types/models'

defineProps<{
  marketplace: MarketplaceInfo
}>()

defineEmits<{
  open: [marketplace: MarketplaceInfo]
  update: [marketplace: MarketplaceInfo]
  remove: [marketplace: MarketplaceInfo]
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
