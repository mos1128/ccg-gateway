<template>
  <div class="plugin-card">
    <div class="card-top">
      <div class="plugin-icon">
        <svg width="24" height="24"><use href="#icon-puzzle"/></svg>
      </div>
      <div class="plugin-info">
        <div class="plugin-title-row">
          <h3 class="plugin-name">{{ plugin.name }}</h3>
          <span v-if="plugin.version" class="plugin-ver mono">v{{ plugin.version }}</span>
        </div>
        <div class="plugin-market">@{{ plugin.marketplace_name }}</div>
      </div>
      <div class="card-actions">
        <button
          class="action-icon star"
          :class="{ 'star-active': isFavorite }"
          title="收藏/取消"
          @click="$emit('favorite', plugin)"
        >
          <svg width="18" height="18" :style="isFavorite ? 'fill: var(--color-warning);' : ''">
            <use href="#icon-star"/>
          </svg>
        </button>
        <button class="action-icon" title="更新" :disabled="operationDisabled" @click="$emit('update', plugin)">
          <svg width="18" height="18"><use href="#icon-refresh"/></svg>
        </button>
        <button class="action-icon delete" title="卸载" :disabled="operationDisabled" @click="$emit('uninstall', plugin)">
          <svg width="18" height="18"><use href="#icon-trash"/></svg>
        </button>
      </div>
    </div>

    <div class="cli-toggles">
      <div class="toggle-item">
        <span class="toggle-label">Claude Code</span>
        <el-switch
          size="small"
          :model-value="plugin.is_enabled ?? false"
          @change="emitToggleEnable"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { PluginItem } from '@/types/models'

const props = defineProps<{
  plugin: PluginItem
  isFavorite: boolean
  operationDisabled: boolean
}>()

const emit = defineEmits<{
  favorite: [plugin: PluginItem]
  update: [plugin: PluginItem]
  uninstall: [plugin: PluginItem]
  'toggle-enable': [plugin: PluginItem, enabled: boolean]
}>()

function emitToggleEnable(value: string | number | boolean) {
  emit('toggle-enable', props.plugin, Boolean(value))
}
</script>

<style scoped>
.plugin-card {
  background: var(--color-bg);
  border-radius: 16px;
  border: 1px solid var(--color-border);
  padding: 24px;
  box-shadow: 0 4px 12px var(--color-shadow);
  display: flex;
  flex-direction: column;
  gap: 20px;
}
.card-top { display: flex; gap: 16px; align-items: flex-start; }
.plugin-icon {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  background: var(--color-primary-lighter);
  color: var(--color-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.plugin-info { flex: 1; min-width: 0; }
.plugin-title-row { display: flex; align-items: center; gap: 8px; min-width: 0; }
.plugin-name {
  font-size: var(--fs-16);
  font-weight: var(--fw-700);
  color: var(--color-text);
  margin: 0 0 4px 0;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  text-overflow: ellipsis;
}
.plugin-ver { font-size: var(--fs-12); color: var(--color-text-weak); }
.plugin-market {
  font-size: var(--fs-12);
  color: var(--color-text-muted);
  font-weight: var(--fw-400);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.card-actions { display: flex; gap: 4px; flex-shrink: 0; }
.cli-toggles {
  display: flex;
  flex-direction: column;
  gap: 12px;
  background: var(--color-bg-page);
  padding: 16px;
  border-radius: 12px;
}
.toggle-item { display: flex; justify-content: space-between; align-items: center; }
.toggle-label { font-size: var(--fs-14); font-weight: var(--fw-400); color: var(--color-text-secondary); }
</style>
