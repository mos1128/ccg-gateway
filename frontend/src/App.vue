<template>
  <router-view />
</template>

<script setup lang="ts">
import { useUiStore } from '@/stores/ui'

// 初始化主题
const uiStore = useUiStore()
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

  /* ========== 颜色规范 ========== */
  /* 主色系 */
  --color-primary: #0ea5e9;
  --color-primary-hover: #0284c7;
  --color-primary-dark: #0369a1;
  --color-primary-light: #f0f9ff;
  --color-primary-lighter: #e0f2fe;
  --color-primary-border: #bae6fd;
  --color-primary-muted: #7dd3fc;
  --color-primary-5: rgba(14, 165, 233, 0.05);
  --color-primary-10: rgba(14, 165, 233, 0.1);
  --color-primary-20: rgba(14, 165, 233, 0.2);

  /* 文字色系 */
  --color-text: #0f172a;
  --color-text-secondary: #475569;
  --color-text-muted: #64748b;
  --color-text-weak: #94a3b8;
  --color-text-dark: #334155;

  /* 背景色系 */
  --color-bg: #ffffff;
  --color-bg-page: #f8fafc;
  --color-bg-subtle: #f1f5f9;
  --color-bg-80: rgba(255, 255, 255, 0.8);
  --color-bg-95: rgba(255, 255, 255, 0.95);

  /* 边框色系 */
  --color-border: #e2e8f0;
  --color-border-hover: #cbd5e1;
  --color-border-light: rgba(226, 232, 240, 0.6);
  --color-border-medium: rgba(226, 232, 240, 0.8);
  --color-scrollbar: #cbd5e1;
  --color-scrollbar-hover: #94a3b8;

  /* 状态色系 */
  --color-success: #10b981;
  --color-success-hover: #059669;
  --color-success-light: #ecfdf5;
  --color-success-10: rgba(16, 185, 129, 0.1);
  --color-success-30: rgba(16, 185, 129, 0.3);
  --color-success-40: rgba(16, 185, 129, 0.4);
  --color-danger: #ef4444;
  --color-danger-hover: #dc2626;
  --color-danger-light: #fee2e2;
  --color-danger-muted: #fca5a5;
  --color-error: #f43f5e;
  --color-error-light: #fff1f2;
  --color-error-2: rgba(244, 63, 94, 0.02);
  --color-error-10: rgba(244, 63, 94, 0.1);
  --color-warning: #f59e0b;
  --color-warning-10: rgba(245, 158, 11, 0.1);

  /* 阴影色系 */
  --color-shadow: rgba(0, 0, 0, 0.03);
  --color-shadow-lg: rgba(0, 0, 0, 0.1);
  --color-scrim: rgba(15, 23, 42, 0.25);
  --color-scrim-dark: rgba(15, 23, 42, 0.4);
  --color-overlay: rgba(148, 163, 184, 0.05);
  --color-overlay-8: rgba(148, 163, 184, 0.08);

  /* 特殊色系 */
  --color-violet: #8b5cf6;
  --color-violet-light: #f5f3ff;
}

/* ========== 暗色模式 ========== */
[data-theme="dark"] {
  /* 文字色系 - 反转 */
  --color-text: #f1f5f9;
  --color-text-secondary: #94a3b8;
  --color-text-muted: #64748b;
  --color-text-weak: #475569;
  --color-text-dark: #cbd5e1;

  /* 背景色系 - 反转 */
  --color-bg: #0f172a;
  --color-bg-page: #020617;
  --color-bg-subtle: #1e293b;
  --color-bg-80: rgba(15, 23, 42, 0.8);
  --color-bg-95: rgba(15, 23, 42, 0.95);

  /* 边框色系 */
  --color-border: #334155;
  --color-border-hover: #475569;
  --color-border-light: rgba(51, 65, 85, 0.6);
  --color-border-medium: rgba(51, 65, 85, 0.8);
  --color-scrollbar: #475569;
  --color-scrollbar-hover: #64748b;

  /* 主色系微调 */
  --color-primary-light: rgba(14, 165, 233, 0.15);
  --color-primary-lighter: rgba(14, 165, 233, 0.1);
  --color-primary-border: rgba(14, 165, 233, 0.3);

  /* 状态色系微调 */
  --color-success-light: rgba(16, 185, 129, 0.15);
  --color-danger-light: rgba(239, 68, 68, 0.15);
  --color-error-light: rgba(244, 63, 94, 0.15);
  --color-violet-light: rgba(139, 92, 246, 0.15);

  /* 阴影色系 - 更深 */
  --color-shadow: rgba(0, 0, 0, 0.2);
  --color-shadow-lg: rgba(0, 0, 0, 0.4);
  --color-scrim: rgba(0, 0, 0, 0.5);
  --color-scrim-dark: rgba(0, 0, 0, 0.6);
  --color-overlay: rgba(255, 255, 255, 0.03);
  --color-overlay-8: rgba(255, 255, 255, 0.05);
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
.text-muted { color: var(--color-text-weak); }
.text-secondary { color: var(--color-text-muted); }
.text-primary { color: var(--color-text); }
.text-info { color: var(--color-primary); }
.text-upper { text-transform: uppercase; letter-spacing: 0.5px; }
.text-center { text-align: center; }

/* ========== Global Ethereal Scrollbar ========== */
::-webkit-scrollbar {
  width: 14px;
  height: 14px;
}

::-webkit-scrollbar-thumb {
  background-color: var(--color-scrollbar);
  border-radius: 10px;
  border: 4px solid transparent;
  background-clip: padding-box;
}

::-webkit-scrollbar-thumb:hover {
  background-color: var(--color-scrollbar-hover);
}

::-webkit-scrollbar-track {
  background-color: transparent;
}

/* Element Plus 暗色主题适配 - 映射 data-theme="dark" 到 .dark */
[data-theme="dark"] {
  color-scheme: dark;
}

[data-theme="dark"] .el-button {
  --el-button-bg-color: var(--el-fill-color-blank);
  --el-button-text-color: var(--el-text-color-regular);
}

[data-theme="dark"] .el-input__wrapper,
[data-theme="dark"] .el-textarea__inner {
  background-color: var(--el-fill-color-blank);
}

[data-theme="dark"] .el-table {
  --el-table-bg-color: transparent;
  --el-table-tr-bg-color: transparent;
  --el-table-header-bg-color: var(--el-fill-color-light);
  --el-table-row-hover-bg-color: var(--el-fill-color);
}

[data-theme="dark"] .el-select-dropdown,
[data-theme="dark"] .el-dropdown-menu {
  background-color: var(--el-bg-color-overlay);
  border-color: var(--el-border-color-light);
}

[data-theme="dark"] .el-popper {
  --el-popper-bg-color: var(--el-bg-color-overlay);
}
</style>
