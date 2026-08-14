<template>
  <svg
    v-if="templateIcon"
    class="cli-icon"
    :width="width"
    :height="height"
    :viewBox="templateIcon.view_box"
    fill="currentColor"
  >
    <defs v-if="templateIcon.linear_gradient">
      <linearGradient :id="gradientId" x1="0" y1="0" x2="1" y2="1">
        <stop
          v-for="stop in templateIcon.linear_gradient"
          :key="stop.offset"
          :offset="stop.offset"
          :stop-color="stop.color"
        />
      </linearGradient>
    </defs>
    <path
      v-for="(path, index) in templateIcon.paths"
      :key="index"
      :d="path.d"
      :fill="pathFill(path.fill)"
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
import { getCurrentInstance } from 'vue'
import { Monitor } from '@element-plus/icons-vue'
import { useAgentStore } from '@/stores/agents'

const props = defineProps<{
  type: string
  width?: string | number
  height?: string | number
}>()

const agentStore = useAgentStore()
const templateIcon = computed(() => agentStore.get(props.type)?.icon)
const gradientId = `agent-icon-gradient-${getCurrentInstance()?.uid}`

function pathFill(fill?: string | null): string | undefined {
  if (!fill) return undefined
  return fill === 'linear_gradient' ? `url(#${gradientId})` : fill
}

function iconSize(value?: string | number) {
  return typeof value === 'number' ? `${value}px` : value || '1em'
}
</script>

<style scoped>
.cli-icon {
  display: inline-block;
  vertical-align: middle;
  color: var(--v2-text-2);
}
.cli-icon.generic { color: var(--v2-text-2); }
</style>
