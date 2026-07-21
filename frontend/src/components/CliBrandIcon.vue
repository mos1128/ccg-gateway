<template>
  <svg
    v-if="templateIcon"
    class="cli-icon"
    :width="width"
    :height="height"
    :viewBox="templateIcon.view_box"
    :style="{ color: templateIcon.color || 'var(--v2-text-2)' }"
    fill="currentColor"
  >
    <path
      v-for="(path, index) in templateIcon.paths"
      :key="index"
      :d="path.d"
      :opacity="path.opacity"
      :fill-rule="path.fill_rule"
      :clip-rule="path.clip_rule"
    />
  </svg>
  <el-icon
    v-else
    class="cli-icon generic"
    :style="{ width: iconSize(width), height: iconSize(height), fontSize: iconSize(width) }"
  >
    <Monitor />
  </el-icon>
</template>

<script setup lang="ts">
import { Monitor } from '@element-plus/icons-vue'
import { useAgentStore } from '@/stores/agents'

const props = defineProps<{
  type: string
  width?: string | number
  height?: string | number
}>()

const agentStore = useAgentStore()
const templateIcon = computed(() => agentStore.get(props.type)?.icon)

function iconSize(value?: string | number) {
  return typeof value === 'number' ? `${value}px` : value || '1em'
}
</script>

<style scoped>
.cli-icon {
  display: inline-block;
  vertical-align: middle;
}
.cli-icon.generic { color: var(--v2-text-2); }
</style>
