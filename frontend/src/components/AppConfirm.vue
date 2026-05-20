<template>
  <Teleport to="body">
    <div class="confirm-overlay" :class="{ active: visible }">
      <div class="confirm-content" :class="{ 'confirm-content--markdown': renderMarkdown }">
        <div class="confirm-header">
          <div class="confirm-title">{{ title }}</div>
          <div class="confirm-close" @click="handleCancel">×</div>
        </div>
        <div v-if="renderMarkdown" class="confirm-body markdown-body" v-html="renderedMessage"></div>
        <div v-else class="confirm-body">{{ message }}</div>
        <div class="confirm-footer">
          <button class="b-button-outline" @click="handleCancel">{{ cancelText }}</button>
          <button class="b-button" @click="handleConfirm">{{ confirmText }}</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import MarkdownIt from 'markdown-it'
import { computed, ref } from 'vue'

interface ConfirmOptions {
  confirmText?: string
  cancelText?: string
  markdown?: boolean
}

const markdown = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true
})

const visible = ref(false)
const title = ref('')
const message = ref('')
const confirmText = ref('确定')
const cancelText = ref('取消')
const renderMarkdown = ref(false)
const renderedMessage = computed(() => markdown.render(message.value))

let resolvePromise: ((value: boolean) => void) | null = null

function open(msg: string, t?: string, options?: ConfirmOptions) {
  message.value = msg
  title.value = t || '提示'
  confirmText.value = options?.confirmText || '确定'
  cancelText.value = options?.cancelText || '取消'
  renderMarkdown.value = options?.markdown || false
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
.confirm-content--markdown {
  width: 560px;
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
.markdown-body {
  max-height: min(56vh, 520px);
  overflow-y: auto;
  line-height: 1.7;
  word-break: break-word;
}
.markdown-body :deep(p) {
  margin: 0 0 12px;
}
.markdown-body :deep(p:last-child) {
  margin-bottom: 0;
}
.markdown-body :deep(h1),
.markdown-body :deep(h2),
.markdown-body :deep(h3),
.markdown-body :deep(h4) {
  margin: 18px 0 10px;
  color: var(--color-text);
  line-height: 1.35;
}
.markdown-body :deep(h1) {
  font-size: var(--fs-20);
}
.markdown-body :deep(h2) {
  font-size: var(--fs-18);
}
.markdown-body :deep(h3),
.markdown-body :deep(h4) {
  font-size: var(--fs-16);
}
.markdown-body :deep(h1:first-child),
.markdown-body :deep(h2:first-child),
.markdown-body :deep(h3:first-child),
.markdown-body :deep(h4:first-child) {
  margin-top: 0;
}
.markdown-body :deep(ul),
.markdown-body :deep(ol) {
  margin: 0 0 12px;
  padding-left: 22px;
}
.markdown-body :deep(li + li) {
  margin-top: 6px;
}
.markdown-body :deep(a) {
  color: var(--color-primary);
  text-decoration: none;
}
.markdown-body :deep(a:hover) {
  text-decoration: underline;
}
.markdown-body :deep(code) {
  padding: 2px 5px;
  border-radius: 4px;
  background: var(--color-bg-subtle);
  color: var(--color-text);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.9em;
}
.markdown-body :deep(pre) {
  margin: 0 0 12px;
  padding: 12px;
  border-radius: 8px;
  overflow-x: auto;
  background: var(--color-bg-subtle);
}
.markdown-body :deep(pre code) {
  padding: 0;
  background: transparent;
}
.markdown-body :deep(blockquote) {
  margin: 0 0 12px;
  padding-left: 12px;
  border-left: 3px solid var(--color-border);
  color: var(--color-text-muted);
}
.confirm-footer {
  padding: 20px 32px;
  background: var(--color-bg-page);
  border-top: 1px solid var(--color-bg-subtle);
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}
</style>
