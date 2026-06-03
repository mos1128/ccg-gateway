<template>
  <div class="skill-card">
    <div class="card-top">
      <div class="skill-icon">
        <svg width="24" height="24"><use href="#icon-zap"/></svg>
      </div>
      <div class="skill-info">
        <div class="skill-title-row">
          <h3 class="skill-name">{{ skill.name }}</h3>
          <div v-if="!skill.exists_on_disk" class="tag tag-red">缺失文件</div>
        </div>
        <div v-if="skill.market_display" class="skill-market" :title="skill.market_display">
          {{ skill.repo?.name ? `@${skill.repo.name}` : skill.market_display }}
        </div>
        <div v-else class="skill-source mono">本地安装</div>
      </div>
      <div class="card-actions">
        <button
          class="action-icon star"
          :class="{ 'star-active': skill.is_favorited }"
          :title="skill.is_favorited ? '取消收藏' : '收藏技能'"
          :disabled="!skill.can_favorite"
          @click="$emit('favorite', skill)"
        >
          <svg width="18" height="18" :style="skill.is_favorited ? 'fill: var(--color-warning);' : ''">
            <use href="#icon-star"/>
          </svg>
        </button>
        <button class="action-icon" title="重装/更新" :disabled="reinstalling" @click="$emit('reinstall', skill)">
          <svg width="18" height="18"><use href="#icon-refresh"/></svg>
        </button>
        <button class="action-icon delete" title="卸载" @click="$emit('uninstall', skill)">
          <svg width="18" height="18"><use href="#icon-trash"/></svg>
        </button>
      </div>
    </div>

    <div class="cli-toggles">
      <div class="toggle-item">
        <span class="toggle-label">Claude Code</span>
        <el-switch
          size="small"
          :model-value="skill.cli_flags?.claude_code"
          @change="emitCliToggle('claude_code', $event)"
        />
      </div>
      <div class="toggle-item">
        <span class="toggle-label">Codex</span>
        <el-switch
          size="small"
          :model-value="skill.cli_flags?.codex"
          @change="emitCliToggle('codex', $event)"
        />
      </div>
      <div class="toggle-item">
        <span class="toggle-label">Gemini</span>
        <el-switch
          size="small"
          :model-value="skill.cli_flags?.gemini"
          @change="emitCliToggle('gemini', $event)"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { CliType, InstalledSkill } from '@/types/models'

const props = defineProps<{
  skill: InstalledSkill
  reinstalling: boolean
}>()

const emit = defineEmits<{
  favorite: [skill: InstalledSkill]
  reinstall: [skill: InstalledSkill]
  uninstall: [skill: InstalledSkill]
  'cli-toggle': [skill: InstalledSkill, cliType: CliType, enabled: boolean]
}>()

function emitCliToggle(cliType: CliType, value: string | number | boolean) {
  emit('cli-toggle', props.skill, cliType, Boolean(value))
}
</script>

<style scoped>
.skill-card {
  background: var(--color-bg);
  border-radius: 16px;
  border: 1px solid color-mix(in srgb, var(--color-border) 80%, transparent);
  padding: 24px;
  box-shadow: 0 4px 12px var(--color-shadow);
  display: flex;
  flex-direction: column;
  gap: 20px;
}
.card-top { display: flex; gap: 16px; align-items: flex-start; }
.skill-icon {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  background: var(--color-violet-light);
  color: var(--color-violet);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.skill-info { flex: 1; min-width: 0; }
.skill-title-row { display: flex; align-items: center; gap: 8px; min-width: 0; }
.skill-name {
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
.skill-market {
  font-size: var(--fs-12);
  color: var(--color-text-muted);
  font-weight: var(--fw-400);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.skill-source { font-size: var(--fs-12); color: var(--color-text-weak); }
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
.toggle-label { font-size: var(--fs-14); font-weight: var(--fw-500); color: var(--color-text-secondary); }
.tag { padding: 2px 8px; border-radius: 4px; font-size: var(--fs-12); font-weight: var(--fw-700); text-transform: uppercase; }
.tag-red { background: var(--color-error-light); color: var(--color-error); flex-shrink: 0; }
</style>
