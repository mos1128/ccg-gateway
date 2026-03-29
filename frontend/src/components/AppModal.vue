<template>
  <Teleport to="body">
    <div class="modal-overlay" :class="{ active: modelValue }" @click.self="handleClose">
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
            <button class="b-button-outline" @click="handleCancel">{{ cancelText }}</button>
            <button class="b-button" @click="handleConfirm">{{ confirmText }}</button>
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
}>(), {
  width: '640px',
  showFooter: true,
  cancelText: '取消',
  confirmText: '确定'
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
  background: rgba(15, 23, 42, 0.4);
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
  background: #ffffff;
  border-radius: 20px;
  max-width: 95vw;
  box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.2);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  max-height: 90vh;
}
.modal-header {
  padding: 24px 32px;
  border-bottom: 1px solid #f1f5f9;
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-shrink: 0;
}
.modal-title {
  font-size: 20px;
  font-weight: 600;
  color: #0f172a;
}
.modal-close {
  font-size: 24px;
  color: #94a3b8;
  cursor: pointer;
  line-height: 1;
  transition: color 0.2s;
}
.modal-close:hover {
  color: #64748b;
}
.modal-body {
  padding: 32px;
  overflow-y: auto;
}
.modal-footer {
  padding: 20px 32px;
  background: #f8fafc;
  border-top: 1px solid #f1f5f9;
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  flex-shrink: 0;
}

.b-button {
  background: #0ea5e9;
  color: #ffffff;
  border: none;
  padding: 10px 20px;
  border-radius: 10px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  display: flex;
  align-items: center;
  box-shadow: 0 4px 12px rgba(14, 165, 233, 0.2);
  transition: all 0.2s;
}
.b-button:hover {
  background: #0284c7;
  transform: translateY(-1px);
}

.b-button-outline {
  background: #ffffff;
  color: #475569;
  border: 1px solid #e2e8f0;
  padding: 10px 20px;
  border-radius: 10px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  align-items: center;
}
.b-button-outline:hover {
  background: #f8fafc;
  color: #0f172a;
  border-color: #cbd5e1;
}
</style>