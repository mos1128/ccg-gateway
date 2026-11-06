<template>
  <div
    ref="root"
    class="v2-chip-row"
    :class="{ 'is-scrollable': scrollable, 'is-overflowing': isOverflowing }"
  >
    <button
      v-if="scrollable && isOverflowing"
      class="v2-chips-arrow"
      type="button"
      :aria-label="t('common.scrollLeft')"
      :disabled="!canScrollLeft"
      @click="scrollChips(-1)"
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="m15 18-6-6 6-6" />
      </svg>
    </button>

    <div ref="scroller" class="v2-chip-track" @scroll="updateState">
      <button
        v-for="c in tabs"
        :key="c.id"
        type="button"
        class="v2-chip"
        :class="{ on: isOn(c.id) }"
        @click="handleClick(c.id)"
      >
        <span class="v2-chip-icon-wrapper">
          <CliBrandIcon :type="c.id" width="13" height="13" />
        </span>
        {{ c.label }}
      </button>
    </div>

    <button
      v-if="scrollable && isOverflowing"
      class="v2-chips-arrow"
      type="button"
      :aria-label="t('common.scrollRight')"
      :disabled="!canScrollRight"
      @click="scrollChips(1)"
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="m9 18 6-6-6-6" />
      </svg>
    </button>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { AgentFeatureName, CliType, CliFlags } from '@/types/models'
import CliBrandIcon from '@/components/CliBrandIcon.vue'
import { useAgentStore } from '@/stores/agents'

const props = withDefaults(defineProps<{
  flags?: CliFlags
  feature?: AgentFeatureName
  mode?: 'toggle' | 'select'
  modelValue?: CliType
  scrollable?: boolean
}>(), {
  mode: 'toggle',
  scrollable: false,
})

const emit = defineEmits<{
  toggle: [cliType: CliType, enabled: boolean]
  'update:modelValue': [cliType: CliType]
}>()

const { t } = useI18n()
const agentStore = useAgentStore()
const tabs = computed(() => (props.feature
  ? agentStore.agentsFor(props.feature)
  : agentStore.visibleAgents
).map((agent) => ({ id: agent.id, label: agent.name })))

const root = ref<HTMLElement | null>(null)
const scroller = ref<HTMLElement | null>(null)
const isOverflowing = ref(false)
const canScrollLeft = ref(false)
const canScrollRight = ref(false)

let resizeObserver: ResizeObserver | null = null

function isOn(cliType: CliType) {
  return props.mode === 'select' ? props.modelValue === cliType : Boolean(props.flags?.[cliType])
}

function handleClick(cliType: CliType) {
  if (props.mode === 'select') emit('update:modelValue', cliType)
  else emit('toggle', cliType, !props.flags?.[cliType])
}

function updateState() {
  const element = scroller.value
  const container = root.value
  if (!element || !container) return

  if (!props.scrollable) {
    isOverflowing.value = false
    return
  }

  const overflowing = element.scrollWidth > element.clientWidth + 1

  if (isOverflowing.value !== overflowing) {
    isOverflowing.value = overflowing
    if (!overflowing) element.scrollLeft = 0
    nextTick(updateState)
  }

  const maxScrollLeft = Math.max(0, element.scrollWidth - element.clientWidth)
  canScrollLeft.value = overflowing && element.scrollLeft > 1
  canScrollRight.value = overflowing && element.scrollLeft < maxScrollLeft - 1
}

function scrollChips(direction: -1 | 1) {
  const element = scroller.value
  if (!element) return
  element.scrollBy({ left: direction * Math.max(160, element.clientWidth * 0.6), behavior: 'smooth' })
}

function scrollActiveChipIntoView() {
  if (props.mode !== 'select') return
  const element = scroller.value
  const activeChip = element?.querySelector<HTMLElement>('.v2-chip.on')
  if (!element || !activeChip) return

  const containerRect = element.getBoundingClientRect()
  const chipRect = activeChip.getBoundingClientRect()

  if (chipRect.left < containerRect.left) {
    element.scrollBy({ left: chipRect.left - containerRect.left, behavior: 'smooth' })
  } else if (chipRect.right > containerRect.right) {
    element.scrollBy({ left: chipRect.right - containerRect.right, behavior: 'smooth' })
  }
}

watch([tabs, () => props.modelValue], () => {
  nextTick(() => {
    updateState()
    scrollActiveChipIntoView()
  })
})

onMounted(() => {
  const element = scroller.value
  if (element) {
    resizeObserver = new ResizeObserver(() => {
      updateState()
      nextTick(scrollActiveChipIntoView)
    })
    if (root.value) resizeObserver.observe(root.value)
  }

  nextTick(() => {
    updateState()
    scrollActiveChipIntoView()
  })
})

onBeforeUnmount(() => {
  resizeObserver?.disconnect()
})
</script>

<style scoped>
.v2-chip-icon-wrapper {
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.v2-chip-track {
  display: flex;
  flex-wrap: wrap;
  flex: 1 1 auto;
  gap: 8px;
  min-width: 0;
}
.v2-chip-row.is-scrollable {
  width: 100%;
  flex-wrap: nowrap;
  align-items: center;
  gap: 4px;
}
.v2-chip-row.is-scrollable .v2-chip-track {
  flex-wrap: nowrap;
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
}
.v2-chip-row.is-scrollable .v2-chip-track::-webkit-scrollbar {
  display: none;
}
.v2-chip-row.is-scrollable .v2-chip {
  flex: 0 0 auto;
}
.v2-chips-arrow {
  display: inline-flex;
  width: 22px;
  height: 30px;
  flex-shrink: 0;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--v2-text-2);
  cursor: pointer;
  transition: color 0.15s, background-color 0.15s, opacity 0.15s;
}
.v2-chips-arrow:hover:not(:disabled) {
  background: var(--v2-surface-2);
  color: var(--v2-text);
}
.v2-chips-arrow:disabled {
  color: var(--v2-text-3);
  cursor: default;
  opacity: 0.35;
}
</style>
