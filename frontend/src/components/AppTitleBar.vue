<template>
  <div v-if="isLinux" class="app-titlebar">
    <div class="titlebar-drag-region" @mousedown="startDrag" @dblclick="toggleMaximize">
      <span class="titlebar-title">CCG Gateway</span>
    </div>
    <div class="titlebar-controls">
      <button class="titlebar-button" type="button" title="最小化" @click="minimizeWindow">
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M5 12h14" />
        </svg>
      </button>
      <button class="titlebar-button" type="button" title="最大化" @click="toggleMaximize">
        <svg width="13" height="13" viewBox="0 0 24 24" aria-hidden="true">
          <rect x="5" y="5" width="14" height="14" rx="2" />
        </svg>
      </button>
      <button class="titlebar-button close" type="button" title="关闭" @click="closeWindow">
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M6 6l12 12M18 6L6 18" />
        </svg>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'

const isLinux = /Linux/i.test(navigator.userAgent)
const currentWindow = getCurrentWindow()

function logWindowControlError(action: string, error: unknown) {
  console.error(`Failed to ${action} window:`, error)
}

function minimizeWindow() {
  currentWindow.minimize().catch((error) => logWindowControlError('minimize', error))
}

function toggleMaximize() {
  currentWindow.toggleMaximize().catch((error) => logWindowControlError('toggle maximize', error))
}

function closeWindow() {
  currentWindow.close().catch((error) => logWindowControlError('close', error))
}

function startDrag(event: MouseEvent) {
  if (event.button !== 0 || event.detail > 1) return
  currentWindow.startDragging().catch((error) => logWindowControlError('start dragging', error))
}
</script>

<style scoped>
.app-titlebar {
  height: 34px;
  display: flex;
  align-items: center;
  background: var(--v2-surface-2);
  border-bottom: 1px solid var(--v2-surface-2);
  flex-shrink: 0;
  user-select: none;
}

.titlebar-drag-region {
  flex: 1;
  min-width: 0;
  height: 100%;
  display: flex;
  align-items: center;
  padding-left: 18px;
  cursor: default;
}

.titlebar-title {
  font-size: var(--v2-fs-xs);
  font-weight: var(--v2-fw-semibold);
  color: var(--v2-text-3);
}

.titlebar-controls {
  height: 100%;
  display: flex;
  align-items: stretch;
}

.titlebar-button {
  width: 46px;
  height: 100%;
  border: none;
  border-radius: 0;
  background: transparent;
  color: var(--v2-text-3);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  cursor: pointer;
  transition: background-color 0.16s ease, color 0.16s ease;
}

.titlebar-button svg {
  fill: none;
  stroke: currentColor;
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.titlebar-button:hover {
  background: var(--v2-surface-2);
  color: var(--v2-text);
}

.titlebar-button.close:hover {
  background: var(--v2-danger);
  color: var(--v2-on-danger);
}
</style>
