<template>
  <div class="fav-card">
    <div class="fav-main">
      <div class="fav-info">
        <div class="fav-name">{{ favorite.name }}</div>
        <div class="fav-market" :title="favorite.repo.source">
          来自仓库: {{ favorite.repo.name || favorite.repo.source }}
        </div>
      </div>
      <div class="fav-actions">
        <button class="action-icon star-active" title="取消收藏" @click="$emit('remove', favorite)">
          <svg width="18" height="18" style="fill: var(--color-warning);"><use href="#icon-star"/></svg>
        </button>
        <button
          v-if="favorite.is_installed"
          class="action-icon installed"
          title="重装"
          :disabled="installing"
          @click="$emit('install', favorite, true)"
        >
          <svg width="18" height="18"><use href="#icon-refresh"/></svg>
        </button>
        <button
          v-else
          class="action-icon primary"
          title="安装技能"
          :disabled="installing"
          @click="$emit('install', favorite, false)"
        >
          <svg width="18" height="18"><use href="#icon-plus"/></svg>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { SkillFavoriteItem } from '@/types/models'

defineProps<{
  favorite: SkillFavoriteItem
  installing: boolean
}>()

defineEmits<{
  remove: [favorite: SkillFavoriteItem]
  install: [favorite: SkillFavoriteItem, reinstall: boolean]
}>()
</script>

<style scoped>
.fav-card {
  background: var(--color-bg);
  border-radius: 16px;
  border: 1px solid var(--color-bg-subtle);
  padding: 20px;
}
.fav-main { display: flex; justify-content: space-between; align-items: center; gap: 16px; }
.fav-info { min-width: 0; flex: 1; }
.fav-name {
  font-weight: var(--fw-700);
  font-size: var(--fs-16);
  color: var(--color-text);
  margin-bottom: 4px;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  text-overflow: ellipsis;
}
.fav-market {
  font-size: var(--fs-12);
  color: var(--color-text-weak);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.fav-actions { flex-shrink: 0; display: flex; gap: 4px; }
</style>
