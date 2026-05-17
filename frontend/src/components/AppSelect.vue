<template>
  <div
    class="app-select"
    :class="{ open, disabled }"
    :style="{ width }"
    @click.stop="toggle"
  >
    <div class="app-select-trigger">{{ selectedLabel }}</div>
    <svg class="chevron" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <path d="m6 9 6 6 6-6"/>
    </svg>
    <div class="app-select-options">
      <div
        v-for="option in options"
        :key="String(option.value)"
        class="app-select-option"
        :class="{ selected: option.value === modelValue, disabled: option.disabled }"
        @click.stop="selectOption(option)"
      >
        {{ option.label }}
      </div>
    </div>
  </div>
</template>

<script lang="ts">
export interface AppSelectOption {
  label: string
  value: string | number
  disabled?: boolean
}

let selectSeed = 0
</script>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'

const props = withDefaults(defineProps<{
  modelValue: string | number
  options: AppSelectOption[]
  width?: string
  placeholder?: string
  disabled?: boolean
}>(), {
  width: '160px',
  placeholder: '请选择',
  disabled: false
})

const emit = defineEmits<{
  'update:modelValue': [value: string | number]
  'change': [value: string | number]
}>()

const open = ref(false)
const selectId = ++selectSeed

const selectedLabel = computed(() => {
  return props.options.find(option => option.value === props.modelValue)?.label || props.placeholder
})

function close() {
  open.value = false
}

function toggle() {
  if (props.disabled) return
  const nextOpen = !open.value
  window.dispatchEvent(new CustomEvent('app-select-open', { detail: selectId }))
  open.value = nextOpen
}

function selectOption(option: AppSelectOption) {
  if (props.disabled || option.disabled) return
  emit('update:modelValue', option.value)
  emit('change', option.value)
  close()
}

function handleOtherSelect(event: Event) {
  const detail = (event as CustomEvent<number>).detail
  if (detail !== selectId) close()
}

onMounted(() => {
  document.addEventListener('click', close)
  window.addEventListener('app-select-open', handleOtherSelect)
})

onUnmounted(() => {
  document.removeEventListener('click', close)
  window.removeEventListener('app-select-open', handleOtherSelect)
})
</script>

<style scoped>
.app-select {
  position: relative;
}

.app-select.disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.app-select-trigger {
  padding: 9px 36px 9px 16px;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  font-size: var(--fs-14);
  font-weight: var(--fw-400);
  color: var(--color-text);
  background: color-mix(in srgb, var(--color-bg) 80%, transparent);
  box-shadow: 0 1px 3px var(--color-shadow);
  cursor: pointer;
  transition: all 0.2s;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  user-select: none;
}

.app-select.disabled .app-select-trigger {
  cursor: not-allowed;
}

.app-select:hover:not(.disabled) .app-select-trigger {
  border-color: var(--color-border-hover);
  background: var(--color-bg);
}

.app-select.open .app-select-trigger {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-primary) 10%, transparent);
  background: var(--color-bg);
}

.chevron {
  position: absolute;
  right: 12px;
  top: 50%;
  transform: translateY(-50%);
  color: var(--color-text-muted);
  pointer-events: none;
  transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
}

.app-select.open .chevron {
  transform: translateY(-50%) rotate(180deg);
  color: var(--color-primary);
}

.app-select-options {
  position: absolute;
  top: calc(100% + 6px);
  left: 0;
  right: auto;
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: 12px;
  box-shadow: 0 10px 40px -10px var(--color-shadow-lg);
  padding: 4px;
  z-index: 50;
  opacity: 0;
  transform: translateY(-5px);
  pointer-events: none;
  transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  min-width: 100%;
  max-height: 250px;
  overflow-y: auto;
}

.app-select.open .app-select-options {
  opacity: 1;
  transform: translateY(0);
  pointer-events: auto;
}

.app-select-option {
  padding: 10px 12px;
  border-radius: 8px;
  font-size: var(--fs-14);
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: all 0.1s;
  display: flex;
  align-items: center;
  margin-bottom: 2px;
  white-space: nowrap;
}

.app-select-option:hover:not(.disabled) {
  background: var(--color-bg-subtle);
  color: var(--color-text);
}

.app-select-option.selected {
  font-weight: var(--fw-600);
  color: var(--color-primary);
  background: var(--color-primary-light);
}

.app-select-option.disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
