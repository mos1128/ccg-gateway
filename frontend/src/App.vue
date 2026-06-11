<template>
  <router-view />
</template>

<script setup lang="ts">
import { useThemeStore } from '@/stores/theme'
useThemeStore()
</script>

<style>
/* 仅保留基础 reset 与必要全局；设计系统(令牌/基元)见 layouts/V2Layout.vue(--v2-*) */
:root {
  --el-mask-color: rgba(255, 255, 255, 0.6);
  --font-ui: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', 'PingFang SC', 'Microsoft YaHei', 'Microsoft YaHei UI', 'Noto Sans CJK SC', Arial, sans-serif;
  --font-mono: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Microsoft YaHei', 'Microsoft YaHei UI', monospace;
}
html.dark {
  --el-mask-color: rgba(0, 0, 0, 0.4);
}

*, *::before, *::after {
  box-sizing: border-box;
}

html, body, #app {
  margin: 0;
  padding: 0;
  height: 100%;
  font-family: var(--font-ui);
  font-size: 14px;
  font-weight: 400;
}

input, textarea, button, select {
  font: inherit;
}

.mono, code, pre {
  font-family: var(--font-mono);
}
</style>
