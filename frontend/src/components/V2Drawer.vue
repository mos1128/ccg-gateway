<template>
  <Teleport to="body">
    <Transition name="v2dr">
      <div
        v-if="modelValue"
        class="v2dr-mask v2-scope"
        @pointerdown="handleMaskPointerDown"
        @pointerup="handleMaskPointerUp"
        @pointercancel="resetMaskPointerDown"
      >
        <div class="v2dr" :style="{ width: props.width }" role="dialog">
          <div class="v2dr-head">
            <div class="v2dr-title">{{ title }}</div>
            <button class="v2dr-close" type="button" aria-label="关闭" @click="close">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 6l12 12M18 6L6 18"/></svg>
            </button>
          </div>
          <div class="v2dr-body"><slot /></div>
          <div v-if="showFooter" class="v2dr-foot">
            <slot name="footer">
              <button class="v2-btn v2-btn-sm v2-btn-ghost" @click="close">{{ cancelText }}</button>
              <button class="v2-btn v2-btn-sm v2-btn-primary" :disabled="confirmDisabled" @click="emit('confirm')">{{ confirmText }}</button>
            </slot>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
const props = withDefaults(defineProps<{
  modelValue: boolean
  title: string
  width?: string
  showFooter?: boolean
  cancelText?: string
  confirmText?: string
  confirmDisabled?: boolean
  closeOnMask?: boolean
}>(), {
  width: '50%',
  showFooter: true,
  cancelText: '取消',
  confirmText: '保存',
  confirmDisabled: false,
  closeOnMask: true
})

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  confirm: []
}>()

function close() {
  emit('update:modelValue', false)
}
let maskPointerStarted = false
function resetMaskPointerDown() {
  maskPointerStarted = false
}
function isMaskEvent(e: PointerEvent) {
  return e.target === e.currentTarget
}
function handleMaskPointerDown(e: PointerEvent) {
  maskPointerStarted = isMaskEvent(e)
}
function handleMaskPointerUp(e: PointerEvent) {
  const shouldClose = maskPointerStarted && isMaskEvent(e)
  resetMaskPointerDown()
  if (shouldClose && props.closeOnMask) close()
}
function onKey(e: KeyboardEvent) {
  if (e.key === 'Escape' && props.modelValue) {
    e.stopPropagation()
    close()
  }
}
watch(() => props.modelValue, (v) => {
  if (v) window.addEventListener('keydown', onKey)
  else window.removeEventListener('keydown', onKey)
}, { immediate: true })
onUnmounted(() => window.removeEventListener('keydown', onKey))
</script>

<style>
.v2dr-mask {
  position: fixed;
  inset: 0;
  background: var(--v2-mask-bg);
  backdrop-filter: blur(2px);
  display: flex;
  justify-content: flex-end;
  z-index: 1000;
}
.v2dr {
  width: 50%;
  min-width: min(480px, 94vw);
  max-width: 94vw;
  height: 100%;
  background: var(--v2-surface);
  border-left: 1px solid var(--v2-surface-3);
  box-shadow: var(--v2-shadow-pop);
  display: flex;
  flex-direction: column;
  color: var(--v2-text);
}
.v2dr-head {
  flex-shrink: 0;
  height: 56px;
  padding: 0 18px 0 20px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--v2-surface);
  border-bottom: 1px solid var(--v2-surface-3);
}
.v2dr-title { font-size: var(--v2-fs-md); font-weight: var(--v2-fw-medium); }
.v2dr-close {
  width: 32px;
  height: 32px;
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
.v2dr-close:hover { background: var(--v2-surface-2); color: var(--v2-text); }
.v2dr-body { flex: 1; overflow-y: auto; padding: 20px; background: var(--v2-surface); }
.v2dr-foot {
  flex-shrink: 0;
  padding: 14px 20px;
  background: var(--v2-surface);
  border-top: 1px solid var(--v2-surface-3);
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

.v2dr-enter-active, .v2dr-leave-active { transition: opacity 0.22s ease; }
.v2dr-enter-active .v2dr, .v2dr-leave-active .v2dr { transition: transform 0.28s cubic-bezier(0.32, 0.72, 0, 1); }
.v2dr-enter-from, .v2dr-leave-to { opacity: 0; }
.v2dr-enter-from .v2dr, .v2dr-leave-to .v2dr { transform: translateX(100%); }
</style>
