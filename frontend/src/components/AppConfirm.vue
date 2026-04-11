<template>
  <Teleport to="body">
    <div class="confirm-overlay" :class="{ active: visible }">
      <div class="confirm-content">
        <div class="confirm-header">
          <div class="confirm-title">{{ title }}</div>
          <div class="confirm-close" @click="handleCancel">×</div>
        </div>
        <div class="confirm-body">{{ message }}</div>
        <div class="confirm-footer">
          <button class="b-button-outline" @click="handleCancel">{{ cancelText }}</button>
          <button class="b-button" @click="handleConfirm">{{ confirmText }}</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const visible = ref(false)
const title = ref('')
const message = ref('')
const confirmText = ref('确定')
const cancelText = ref('取消')

let resolvePromise: ((value: boolean) => void) | null = null

function open(msg: string, t?: string, options?: { confirmText?: string; cancelText?: string }) {
  message.value = msg
  title.value = t || '提示'
  confirmText.value = options?.confirmText || '确定'
  cancelText.value = options?.cancelText || '取消'
  visible.value = true
  return new Promise<boolean>((resolve) => {
    resolvePromise = resolve
  })
}

function handleConfirm() {
  visible.value = false
  if (resolvePromise) {
    resolvePromise(true)
    resolvePromise = null
  }
}

function handleCancel() {
  visible.value = false
  if (resolvePromise) {
    resolvePromise(false)
    resolvePromise = null
  }
}

defineExpose({ open })
</script>

<style scoped>
.confirm-overlay {
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
.confirm-overlay.active {
  opacity: 1;
  pointer-events: auto;
}
.confirm-content {
  background: var(--color-bg);
  border-radius: 20px;
  width: 400px;
  max-width: 95vw;
  box-shadow: 0 25px 50px -12px var(--color-shadow-lg);
  overflow: hidden;
}
.confirm-header {
  padding: 24px 32px;
  border-bottom: 1px solid var(--color-bg-subtle);
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.confirm-title {
  font-size: var(--fs-20);
  font-weight: var(--fw-600);
  color: var(--color-text);
}
.confirm-close {
  font-size: var(--fs-24);
  color: var(--color-text-weak);
  cursor: pointer;
  line-height: 1;
  transition: color 0.2s;
}
.confirm-close:hover {
  color: var(--color-text-muted);
}
.confirm-body {
  padding: 32px;
  font-size: var(--fs-14);
  color: var(--color-text-secondary);
}
.confirm-footer {
  padding: 20px 32px;
  background: var(--color-bg-page);
  border-top: 1px solid var(--color-bg-subtle);
  display: flex;
  justify-content: flex-end;
  gap: 12px;
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
}
.b-button-outline:hover {
  background: var(--color-bg-page);
  color: var(--color-text);
  border-color: var(--color-border-hover);
}
</style>