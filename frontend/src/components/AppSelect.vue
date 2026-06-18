<template>
  <el-select
    :model-value="modelValue"
    @update:model-value="onSelectChange"
    :disabled="disabled"
    :size="size"
    :placeholder="placeholder"
    :empty-values="[undefined, null]"
    :style="{ width }"
    class="app-select-el"
  >
    <el-option
      v-for="option in options"
      :key="String(option.value)"
      :label="option.label"
      :value="option.value"
      :disabled="option.disabled"
    />
  </el-select>
</template>

<script lang="ts">
export interface AppSelectOption {
  label: string
  value: string | number
  disabled?: boolean
}
</script>

<script setup lang="ts">
withDefaults(defineProps<{
  modelValue?: string | number
  options: AppSelectOption[]
  width?: string
  placeholder?: string
  disabled?: boolean
  size?: 'default' | 'small'
}>(), {
  modelValue: '',
  width: '160px',
  placeholder: '请选择',
  disabled: false,
  size: 'default'
})

const emit = defineEmits<{
  'update:modelValue': [value: string | number]
  'change': [value: string | number]
}>()

function onSelectChange(value: string | number) {
  emit('update:modelValue', value)
  emit('change', value)
}
</script>

<style scoped>
/* Make el-select trigger box look flat, borderless and match the custom style */
.app-select-el :deep(.el-select__wrapper) {
  box-shadow: none !important;
  border: 1px solid transparent !important;
  background-color: var(--v2-bg-base) !important;
  border-radius: var(--v2-r) !important;
}

.app-select-el.el-select--small :deep(.el-select__wrapper) {
  border-radius: var(--v2-r-sm) !important;
}

/* Ensure no border when hovered or focused */
.app-select-el :deep(.el-select__wrapper:hover),
.app-select-el :deep(.el-select__wrapper.is-focused) {
  box-shadow: none !important;
  border-color: transparent !important;
}

/* Align text and style color inside the trigger to match V2 theme */
.app-select-el :deep(.el-select__placeholder),
.app-select-el :deep(.el-select__selected-item) {
  color: var(--v2-text) !important;
  font-weight: var(--v2-fw-medium) !important;
  font-size: 13px !important;
}

.app-select-el.el-select--small :deep(.el-select__placeholder),
.app-select-el.el-select--small :deep(.el-select__selected-item) {
  font-size: 12px !important;
}

/* Chevron arrow styling inside select */
.app-select-el :deep(.el-select__suffix) {
  color: var(--v2-text-3) !important;
}
</style>
