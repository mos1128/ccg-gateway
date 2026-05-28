<template>
  <div class="discover-item">
    <div class="discover-info">
      <div class="discover-name-row">
        <span class="discover-name">{{ skill.name }}</span>
        <span class="mono text-12 text-muted">{{ skill.directory }}</span>
      </div>
      <el-tooltip v-if="skill.description" effect="light" placement="top" :enterable="true" :show-after="200">
        <template #content>
          <div class="text-14" style="max-width: 350px; line-height: 1.6; word-break: break-word; user-select: text; color: var(--color-text-dark);">
            {{ skill.description }}
          </div>
        </template>
        <div class="discover-desc" @click="$emit('copy-description', skill.description)">
          {{ skill.description }}
        </div>
      </el-tooltip>
      <div v-else class="discover-desc">暂无描述</div>
    </div>
    <div class="discover-actions">
      <button
        v-if="skill.is_installed"
        class="action-icon installed"
        title="重装"
        :disabled="installing"
        @click="$emit('install', skill, true)"
      >
        <svg width="18" height="18"><use href="#icon-refresh"/></svg>
      </button>
      <button
        v-else
        class="action-icon primary"
        title="安装技能"
        :disabled="installing"
        @click="$emit('install', skill, false)"
      >
        <svg width="18" height="18"><use href="#icon-plus"/></svg>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { DiscoverableSkill } from '@/types/models'

defineProps<{
  skill: DiscoverableSkill
  installing: boolean
}>()

defineEmits<{
  install: [skill: DiscoverableSkill, reinstall: boolean]
  'copy-description': [text: string]
}>()
</script>

<style scoped>
.discover-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px 24px;
  border-bottom: 1px solid var(--color-bg-subtle);
  transition: background 0.2s;
}
.discover-item:last-child { border-bottom: none; }
.discover-item:hover { background: var(--color-bg-page); }
.discover-info { flex: 1; min-width: 0; padding-right: 40px; }
.discover-name-row { margin-bottom: 6px; display: flex; align-items: center; gap: 8px; }
.discover-name { font-weight: var(--fw-700); font-size: var(--fs-14); color: var(--color-text); }
.discover-desc {
  font-size: var(--fs-14);
  color: var(--color-text-muted);
  line-height: 1.5;
  cursor: pointer;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  text-overflow: ellipsis;
}
.discover-actions { flex-shrink: 0; display: flex; gap: 4px; }
</style>
