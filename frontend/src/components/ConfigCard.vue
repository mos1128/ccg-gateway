<template>
  <div class="v2-card v2-ccard">
    <div class="v2-ccard-head">
      <div class="v2-ccard-icon">
        <svg v-if="icon === 'mcp'" width="18" height="18" viewBox="0 0 24 24"><path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"/><path d="m3.3 7 8.7 5 8.7-5"/><path d="M12 22V12"/></svg>
        <svg v-else width="18" height="18" viewBox="0 0 24 24"><path d="M3 21c3 0 7-1 7-8V5c0-1.1-.9-2-2-2H4c-1.1 0-2 .9-2 2v6c0 1.1.9 2 2 2h4c0 3.5-1 4.4-2 5.5l-1 1"/><path d="M15 21c3 0 7-1 7-8V5c0-1.1-.9-2-2-2h-4c-1.1 0-2 .9-2 2v6c0 1.1.9 2 2 2h4c0 3.5-1 4.4-2 5.5l-1 1"/></svg>
      </div>
      <div class="v2-ccard-tt">
        <div class="v2-ccard-name">{{ title }}</div>
        <div v-if="subtitle" class="v2-ccard-sub mono">{{ subtitle }}</div>
      </div>
      <div class="v2-ccard-acts">
        <el-tooltip content="编辑" placement="top" effect="light" :show-after="250">
          <button class="v2-row-act" type="button" @click="emit('edit')"><svg width="16" height="16" viewBox="0 0 24 24"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.1 2.1 0 0 1 3 3L12 15l-4 1 1-4z"/></svg></button>
        </el-tooltip>
        <el-tooltip content="删除" placement="top" effect="light" :show-after="250">
          <button class="v2-row-act danger" type="button" @click="emit('delete')"><svg width="16" height="16" viewBox="0 0 24 24"><path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg></button>
        </el-tooltip>
      </div>
    </div>
    <V2CliChips :flags="flags" @toggle="(c, e) => emit('toggle', c, e)" />
  </div>
</template>

<script setup lang="ts">
import V2CliChips from '@/components/V2CliChips.vue'
import type { CliType, CliFlags } from '@/types/models'

defineProps<{
  icon: 'mcp' | 'prompt'
  title: string
  subtitle?: string
  flags: CliFlags
}>()

const emit = defineEmits<{
  edit: []
  delete: []
  toggle: [cliType: CliType, enabled: boolean]
}>()
</script>

<style scoped>
.v2-row-act svg { fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
</style>
