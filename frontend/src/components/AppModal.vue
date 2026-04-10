<template>
  <Teleport to="body">
    <div class="modal-overlay" :class="{ active: modelValue }">
      <div class="modal-content" :style="contentStyle">
        <div class="modal-header">
          <div class="modal-title">{{ title }}</div>
          <div class="modal-close" @click="handleClose">×</div>
        </div>
        <div class="modal-body">
          <slot />
        </div>
        <div v-if="showFooter" class="modal-footer">
          <slot name="footer">
            <slot name="footer-extra"></slot>
            <button class="b-button-outline" :disabled="cancelDisabled" @click="handleCancel">{{ cancelText }}</button>
            <button class="b-button" :disabled="confirmDisabled" @click="handleConfirm">{{ confirmText }}</button>
          </slot>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{
  modelValue: boolean
  title: string
  width?: string
  showFooter?: boolean
  cancelText?: string
  confirmText?: string
  cancelDisabled?: boolean
  confirmDisabled?: boolean
}>(), {
  width: '640px',
  showFooter: true,
  cancelText: '取消',
  confirmText: '保存',
  cancelDisabled: false,
  confirmDisabled: false
})

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  'cancel': []
  'confirm': []
}>()

const contentStyle = computed(() => ({ width: props.width }))

function handleClose() {
  emit('update:modelValue', false)
}

function handleCancel() {
  emit('cancel')
  handleClose()
}

function handleConfirm() {
  emit('confirm')
}
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: var(--color-scrim-dark);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.2s;
}
.modal-overlay.active {
  opacity: 1;
  pointer-events: auto;
}
.modal-content {
  background: var(--color-bg);
  border-radius: 20px;
  max-width: 95vw;
  box-shadow: 0 25px 50px -12px var(--color-shadow-lg);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  max-height: 90vh;
}
.modal-header {
  padding: 24px 32px;
  border-bottom: 1px solid var(--color-bg-subtle);
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-shrink: 0;
}
.modal-title {
  font-size: var(--fs-20);
  font-weight: var(--fw-600);
  color: var(--color-text);
}
.modal-close {
  font-size: var(--fs-24);
  color: var(--color-text-weak);
  cursor: pointer;
  line-height: 1;
  transition: color 0.2s;
}
.modal-close:hover {
  color: var(--color-text-muted);
}
.modal-body {
  padding: 32px;
  overflow-y: auto;
}
.modal-footer {
  padding: 20px 32px;
  background: var(--color-bg-page);
  border-top: 1px solid var(--color-bg-subtle);
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  flex-shrink: 0;
}

.b-button {
  background: var(--color-primary);
  color: var(--color-bg);
  border: none;
  padding: 10px 20px;
  border-radius: 10px;
  font-size: var(--fs-14);
  font-weight: var(--fw-600);
  cursor: pointer;
  display: flex;
  align-items: center;
  transition: all 0.2s;
}
.b-button:hover {
  background: var(--color-primary-hover);
}

.b-button-outline {
  background: var(--color-bg);
  color: var(--color-text-secondary);
  border: 1px solid var(--color-border);
  padding: 10px 20px;
  border-radius: 10px;
  font-size: var(--fs-14);
  font-weight: var(--fw-600);
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  align-items: center;
}
.b-button-outline:hover {
  background: var(--color-bg-page);
  color: var(--color-text);
  border-color: var(--color-border-hover);
}
</style>