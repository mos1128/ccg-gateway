<template>
  <Teleport to="body">
    <div class="confirm-overlay v2-scope" :class="{ active: visible }">
      <div class="confirm-content" :class="{ 'confirm-content--markdown': renderMarkdown }">
        <div class="confirm-header">
          <div class="confirm-title">{{ title }}</div>
          <button class="confirm-close" type="button" :aria-label="t('common.close')" @click="handleCancel">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 6l12 12M18 6L6 18"/></svg>
          </button>
        </div>
        <div v-if="renderMarkdown" class="confirm-body confirm-body--markdown">
          <V2Markdown :content="message" />
        </div>
        <div v-else class="confirm-body">{{ message }}</div>
        <div class="confirm-footer">
          <button class="v2-btn v2-btn-sm v2-btn-outline" @click="handleCancel">{{ cancelText }}</button>
          <button class="v2-btn v2-btn-sm v2-btn-primary" @click="handleConfirm">{{ confirmText }}</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, watch, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import V2Markdown from '@/components/V2Markdown.vue'

interface ConfirmOptions {
  confirmText?: string
  cancelText?: string
  markdown?: boolean
}

const visible = ref(false)
const title = ref('')
const message = ref('')
const confirmText = ref('')
const cancelText = ref('')
const renderMarkdown = ref(false)
const { t } = useI18n()

let resolvePromise: ((value: boolean) => void) | null = null

function open(msg: string, titleText?: string, options?: ConfirmOptions) {
  message.value = msg
  title.value = titleText || t('common.confirmTitle')
  confirmText.value = options?.confirmText || t('common.ok')
  cancelText.value = options?.cancelText || t('common.cancel')
  renderMarkdown.value = options?.markdown || false
  visible.value = true
  return new Promise<boolean>((resolve) => {
    resolvePromise = resolve
  })
}

function handleConfirm() {
  visible.value = false
  if (document.activeElement instanceof HTMLElement) {
    document.activeElement.blur()
  }
  if (resolvePromise) {
    resolvePromise(true)
    resolvePromise = null
  }
}

function handleCancel() {
  visible.value = false
  if (document.activeElement instanceof HTMLElement) {
    document.activeElement.blur()
  }
  if (resolvePromise) {
    resolvePromise(false)
    resolvePromise = null
  }
}

function onKey(e: KeyboardEvent) {
  if (e.key === 'Escape' && visible.value) {
    e.stopPropagation()
    handleCancel()
  }
}

watch(visible, (v) => {
  if (v) window.addEventListener('keydown', onKey)
  else window.removeEventListener('keydown', onKey)
})

onUnmounted(() => {
  window.removeEventListener('keydown', onKey)
})

defineExpose({ open })
</script>

<style scoped>
.confirm-overlay {
  position: fixed;
  inset: 0;
  background: var(--v2-mask-bg);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.22s ease;
}
.confirm-overlay.active {
  opacity: 1;
  pointer-events: auto;
}
.confirm-content {
  background: var(--v2-surface);
  border-radius: var(--v2-r-lg);
  width: 420px;
  max-width: 95vw;
  box-shadow: var(--v2-shadow-pop);
  overflow: hidden;
  border: 1px solid var(--v2-surface-3);
  display: flex;
  flex-direction: column;
  transform: scale(0.92);
  opacity: 0;
  transition: transform 0.28s cubic-bezier(0.34, 1.56, 0.64, 1), opacity 0.22s;
}
.confirm-overlay.active .confirm-content {
  transform: scale(1);
  opacity: 1;
}
.confirm-content--markdown {
  width: 560px;
}
.confirm-header {
  padding: 20px 24px 12px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.confirm-title {
  font-size: var(--v2-fs-md);
  font-weight: var(--v2-fw-medium);
  color: var(--v2-text);
}
.confirm-close {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--v2-text-3);
  border-radius: var(--v2-r-sm);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.confirm-close:hover {
  background: var(--v2-surface-2);
  color: var(--v2-text);
}
.confirm-body {
  padding: 0 24px 24px;
  font-size: var(--v2-fs-sm);
  color: var(--v2-text-2);
  line-height: 1.6;
}
.confirm-body--markdown {
  max-height: min(56vh, 520px);
  overflow-y: auto;
}
.confirm-footer {
  padding: 16px 24px 20px;
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>
