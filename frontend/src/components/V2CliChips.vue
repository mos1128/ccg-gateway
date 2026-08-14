<template>
  <div class="v2-chip-row">
    <button
      v-for="c in tabs"
      :key="c.id"
      type="button"
      class="v2-chip"
      :class="{ on: flags[c.id] }"
      @click="emit('toggle', c.id, !flags[c.id])"
    >
      <span class="v2-chip-icon-wrapper">
        <CliBrandIcon :type="c.id" width="13" height="13" />
      </span>
      {{ c.label }}
    </button>
  </div>
</template>

<script setup lang="ts">
import type { AgentFeatureName, CliType, CliFlags } from '@/types/models'
import CliBrandIcon from '@/components/CliBrandIcon.vue'
import { useAgentStore } from '@/stores/agents'

const props = defineProps<{ flags: CliFlags; feature?: AgentFeatureName }>()
const emit = defineEmits<{ toggle: [cliType: CliType, enabled: boolean] }>()
const agentStore = useAgentStore()
const tabs = computed(() => (props.feature
  ? agentStore.agentsFor(props.feature)
  : agentStore.visibleAgents
).map((agent) => ({ id: agent.id, label: agent.name })))
</script>

<style scoped>
.v2-chip-icon-wrapper {
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.v2-chip.on {
  color: var(--v2-text);
}
</style>
