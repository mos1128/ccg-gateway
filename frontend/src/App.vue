<template>
  <el-config-provider :locale="elLocale">
    <router-view />
  </el-config-provider>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import en from 'element-plus/es/locale/lang/en'
import { useThemeStore } from '@/stores/theme'
import { useLocaleStore } from '@/stores/locale'

useThemeStore()

const localeStore = useLocaleStore()
// Element Plus 内置文案（分页、日期面板等）跟随界面语言
const elLocale = computed(() => (localeStore.locale === 'zh-CN' ? zhCn : en))

localeStore.init()
</script>

<style>
/* 仅保留基础 reset 与必要全局；设计系统(令牌/基元)见 layouts/V2Layout.vue(--v2-*) */
:root {
  --el-mask-color: rgba(255, 255, 255, 0.4);
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
  font-weight: var(--v2-fw-regular);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-rendering: optimizeLegibility;
}

input, textarea, button, select {
  font: inherit;
}

.mono, code, pre {
  font-family: var(--font-mono);
}
</style>
