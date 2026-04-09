<template>
  <router-view />
</template>

<script setup lang="ts">
</script>

<style>
/* ========== 字体规范 ========== */
:root {
  /* 字体权重 */
  --fw-400: 400;
  --fw-500: 500;
  --fw-600: 600;
  --fw-700: 700;

  /* 字体大小 */
  --fs-12: 12px;
  --fs-14: 14px;
  --fs-16: 16px;
  --fs-20: 20px;
  --fs-24: 24px;
  --fs-32: 32px;
}

*, *::before, *::after {
  box-sizing: border-box;
}

html, body, #app {
  margin: 0;
  padding: 0;
  height: 100%;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  font-size: var(--fs-14);
  font-weight: var(--fw-400);
}

/* 等宽字体 */
.mono, .code-block, code, pre {
  font-family: ui-monospace, "SF Mono", Consolas, "Liberation Mono", Menlo, monospace;
}

/* ========== 字体大小类 ========== */
.text-12 { font-size: var(--fs-12); }
.text-14 { font-size: var(--fs-14); }
.text-16 { font-size: var(--fs-16); }
.text-20 { font-size: var(--fs-20); }
.text-24 { font-size: var(--fs-24); }
.text-32 { font-size: var(--fs-32); }

/* ========== 字体权重类 ========== */
.fw-normal { font-weight: var(--fw-400); }
.fw-medium { font-weight: var(--fw-600); }
.fw-bold { font-weight: var(--fw-700); }

/* ========== 颜色类 ========== */
.text-muted { color: #94a3b8; }
.text-secondary { color: #64748b; }
.text-primary { color: #0f172a; }
.text-info { color: #0ea5e9; }
.text-upper { text-transform: uppercase; letter-spacing: 0.5px; }
.text-center { text-align: center; }

/* ========== Global Ethereal Scrollbar ========== */
::-webkit-scrollbar {
  width: 14px;
  height: 14px;
}

::-webkit-scrollbar-thumb {
  background-color: #cbd5e1;
  border-radius: 10px;
  border: 4px solid transparent;
  background-clip: padding-box;
}

::-webkit-scrollbar-thumb:hover {
  background-color: #94a3b8;
}

::-webkit-scrollbar-track {
  background-color: transparent;
}

/* Element Plus Specific Overrides */
.el-textarea__inner::-webkit-scrollbar {
  width: 14px;
}
</style>
