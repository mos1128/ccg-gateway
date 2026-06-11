<template>
  <!-- Menu mode: slot trigger, no selected state -->
  <div
    v-if="mode === 'menu'"
    class="app-select app-select-menu"
    :class="{ open, 'size-small': size === 'small' }"
    @click.stop="toggle"
  >
    <slot name="trigger" />
    <div class="app-select-options" :class="[menuAlign]">
      <div
        v-for="option in options"
        :key="String(option.value)"
        class="app-select-option"
        :class="{ disabled: option.disabled }"
        @click.stop="selectOption(option)"
      >
        <span class="option-label">{{ option.label }}</span>
      </div>
    </div>
  </div>

  <!-- Default select mode -->
  <div
    v-else
    class="app-select"
    :class="{ open, disabled, 'size-small': size === 'small' }"
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
        <svg v-if="option.value === modelValue" class="check-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="20 6 9 17 4 12"/>
        </svg>
        <span class="option-label">{{ option.label }}</span>
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
  modelValue?: string | number
  options: AppSelectOption[]
  mode?: 'select' | 'menu'
  menuAlign?: 'left' | 'right'
  width?: string
  placeholder?: string
  disabled?: boolean
  size?: 'default' | 'small'
}>(), {
  modelValue: '',
  mode: 'select',
  menuAlign: 'right',
  width: '160px',
  placeholder: '请选择',
  disabled: false,
  size: 'default'
})

const emit = defineEmits<{
  'update:modelValue': [value: string | number]
  'change': [value: string | number]
  'select': [value: string | number]
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
  if (props.mode === 'menu') {
    emit('select', option.value)
  } else {
    emit('update:modelValue', option.value)
    emit('change', option.value)
  }
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

.app-select-menu {
  display: inline-flex;
  cursor: pointer;
}

.app-select.disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.app-select-trigger {
  padding: 8px 36px 8px 16px;
  border: 1px solid var(--v2-surface-3);
  border-radius: var(--v2-r-sm);
  font-size: var(--v2-fs-base);
  font-weight: 400;
  color: var(--v2-text);
  background: var(--v2-surface);
  cursor: pointer;
  transition: all 0.15s ease;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  user-select: none;
}

.app-select.disabled .app-select-trigger {
  cursor: not-allowed;
}


.app-select.open .app-select-trigger {
  background: var(--v2-surface);
  border-color: var(--v2-accent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--v2-accent) 12%, transparent);
}

.chevron {
  position: absolute;
  right: 12px;
  top: 50%;
  transform: translateY(-50%);
  color: var(--v2-text-3);
  pointer-events: none;
  transition: transform 0.2s ease;
}

.app-select.open .chevron {
  transform: translateY(-50%) rotate(180deg);
}

.app-select-options {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: auto;
  background: var(--v2-surface);
  border: 1px solid var(--v2-surface-3);
  border-radius: var(--v2-r-sm);
  box-shadow: var(--v2-shadow-pop);
  padding: 4px;
  z-index: 50;
  opacity: 0;
  transform: translateY(-4px);
  pointer-events: none;
  transition: all 0.15s ease;
  min-width: 100%;
  max-height: 250px;
  overflow-y: auto;
}

.app-select-menu .app-select-options {
  min-width: 140px;
}

.app-select-menu .app-select-options.right {
  left: auto;
  right: 0;
}

.app-select.open .app-select-options {
  opacity: 1;
  transform: translateY(0);
  pointer-events: auto;
}

.app-select-option {
  position: relative;
  padding: 8px 14px 8px 32px;
  border-radius: var(--v2-r-sm);
  font-size: var(--v2-fs-base);
  color: var(--v2-text-2);
  cursor: pointer;
  transition: all 0.1s;
  white-space: nowrap;
}

.app-select-menu .app-select-option {
  padding: 8px 14px;
}

.app-select-option:hover:not(.disabled) {
  background: var(--v2-surface-2);
  color: var(--v2-text);
}

.app-select-option.selected {
  font-weight: 600;
  color: var(--v2-accent);
  background: transparent;
}

.app-select-option.disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.option-label {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
}

.check-icon {
  position: absolute;
  left: 10px;
  top: 50%;
  transform: translateY(-50%);
  color: var(--v2-accent);
}

/* Small size styles */
.app-select.size-small .app-select-trigger {
  padding: 6px 32px 6px 12px;
  font-size: 13px;
  border-radius: var(--v2-r-sm);
}

.app-select.size-small .chevron {
  right: 10px;
  width: 14px;
  height: 14px;
}

.app-select.size-small .app-select-option {
  padding: 6px 12px 6px 28px;
  font-size: 13px;
}

.app-select.size-small .check-icon {
  left: 10px;
  width: 11px;
  height: 11px;
}

.app-select.size-small.app-select-menu .app-select-option {
  padding: 6px 12px;
}
</style>
