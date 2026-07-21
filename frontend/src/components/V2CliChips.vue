<template>
  <div class="v2-chip-row">
    <button
      v-for="c in tabs"
      :key="c.id"
      type="button"
      class="v2-chip"
      :class="[c.id, { on: flags[c.id] }]"
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
  : agentStore.agents
).map((agent) => ({ id: agent.id, label: agent.name })))
</script>

<style scoped>
.v2-chip-icon-wrapper {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  filter: grayscale(1) opacity(0.5);
  transition: filter 0.15s, opacity 0.15s;
}
.v2-chip.on .v2-chip-icon-wrapper {
  filter: none;
}
.v2-chip.claude_code {
  --v2-chip-agent: var(--v2-brand-claude);
}
.v2-chip.codex {
  --v2-chip-agent: var(--v2-brand-openai);
}
.v2-chip.gemini {
  --v2-chip-agent: var(--v2-brand-gemini);
}
.v2-chip.on {
  background: color-mix(in srgb, var(--v2-chip-agent) 10%, var(--v2-surface));
  border-color: transparent;
  color: var(--v2-text);
  box-shadow: none;
}
</style>
