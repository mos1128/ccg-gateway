<template>
  <router-view />
</template>

<script setup lang="ts">
import { useThemeStore } from '@/stores/theme'
useThemeStore()
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

  /* Element Plus 覆盖 */
  --el-mask-color: rgba(255, 255, 255, 0.6);
}

/* ========== 暗色模式 ========== */
html.dark {
  /* 主色系 - 保持不变 */
  --color-primary: #0ea5e9;
  --color-primary-hover: #38bdf8;
  --color-primary-dark: #0369a1;
  --color-primary-light: #0c4a6e;
  --color-primary-lighter: #082f49;
  --color-primary-border: #0369a1;
  --color-primary-muted: #7dd3fc;
  --color-primary-5: rgba(14, 165, 233, 0.15);
  --color-primary-10: rgba(14, 165, 233, 0.25);
  --color-primary-20: rgba(14, 165, 233, 0.35);

  /* 文字色系 */
  --color-text: #f1f5f9;
  --color-text-secondary: #cbd5e1;
  --color-text-muted: #94a3b8;
  --color-text-weak: #64748b;
  --color-text-dark: #e2e8f0;

  /* 背景色系 */
  --color-bg: #1e293b;
  --color-bg-page: #0f172a;
  --color-bg-subtle: #334155;
  --color-bg-80: rgba(30, 41, 59, 0.8);
  --color-bg-95: rgba(30, 41, 59, 0.95);

  /* 边框色系 */
  --color-border: #334155;
  --color-border-hover: #475569;
  --color-border-light: rgba(51, 65, 85, 0.6);
  --color-border-medium: rgba(51, 65, 85, 0.8);
  --color-scrollbar: #475569;
  --color-scrollbar-hover: #64748b;

  /* 状态色系 */
  --color-success: #10b981;
  --color-success-hover: #34d399;
  --color-success-light: #064e3b;
  --color-success-10: rgba(16, 185, 129, 0.2);
  --color-success-30: rgba(16, 185, 129, 0.4);
  --color-success-40: rgba(16, 185, 129, 0.5);
  --color-danger: #f87171;
  --color-danger-hover: #fca5a5;
  --color-danger-light: #450a0a;
  --color-danger-muted: #fca5a5;
  --color-error: #fb7185;
  --color-error-light: #4c0519;
  --color-error-2: rgba(251, 113, 133, 0.1);
  --color-error-10: rgba(251, 113, 133, 0.2);
  --color-warning: #fbbf24;
  --color-warning-10: rgba(251, 191, 36, 0.2);

  /* 阴影色系 */
  --color-shadow: rgba(0, 0, 0, 0.2);
  --color-shadow-lg: rgba(0, 0, 0, 0.4);
  --color-scrim: rgba(0, 0, 0, 0.5);
  --color-scrim-dark: rgba(0, 0, 0, 0.7);
  --color-overlay: rgba(0, 0, 0, 0.1);
  --color-overlay-8: rgba(0, 0, 0, 0.15);

  /* 特殊色系 */
  --color-violet: #a78bfa;
  --color-violet-light: #2e1065;

  /* Element Plus 覆盖 */
  --el-mask-color: rgba(0, 0, 0, 0.4);
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

/* ========== 输入框 ========== */
.b-input {
  width: 100%;
  padding: 10px 14px;
  background: var(--color-bg);
  color: var(--color-text);
  border: 1px solid var(--color-border);
  border-radius: 10px;
  font-size: var(--fs-14);
  outline: none;
  transition: all 0.2s;
}
.b-input:focus {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 1px var(--color-primary-10);
}
.b-input::placeholder {
  color: var(--color-text-weak);
}

textarea.b-input {
  resize: vertical;
  line-height: 1.6;
}

/* ========== 分段按钮 ========== */
.b-segmented { display: inline-flex; background: var(--color-border); padding: 4px; border-radius: 10px; }
.b-seg-btn { text-align: center; padding: 6px 16px; font-size: var(--fs-14); color: var(--color-text-muted); border-radius: 8px; font-weight: var(--fw-500); transition: all 0.2s ease; cursor: pointer; }
.b-seg-btn.active { background: var(--color-bg); color: var(--color-primary); box-shadow: 0 1px 3px var(--color-shadow-lg); pointer-events: none; }

/* ========== 操作图标按钮 ========== */
.action-icon {
  width: 34px; height: 34px; display: flex; align-items: center; justify-content: center;
  border-radius: 8px; color: var(--color-text-muted); cursor: pointer;
  transition: all 0.2s; background: transparent; border: none; outline: none; flex-shrink: 0; padding: 0;
}
.action-icon:hover { background: var(--color-bg-subtle); color: var(--color-text); }
.action-icon.delete:hover { background: var(--color-danger-light); color: var(--color-danger); }
.action-icon.primary { color: var(--color-primary); background: color-mix(in srgb, var(--color-primary) 10%, transparent); }
.action-icon.primary:hover { background: color-mix(in srgb, var(--color-primary) 20%, transparent); color: var(--color-primary); }
.action-icon.installed { color: var(--color-warning); background: color-mix(in srgb, var(--color-warning) 10%, transparent); }
.action-icon.star-active { color: var(--color-warning); background: color-mix(in srgb, var(--color-warning) 10%, transparent); }
.action-icon.success { color: var(--color-success); background: color-mix(in srgb, var(--color-success) 10%, transparent); }
.action-icon.success:hover { background: color-mix(in srgb, var(--color-success) 20%, transparent); color: var(--color-success); }
.action-icon:disabled, .action-icon.disabled { opacity: 0.45; cursor: not-allowed; pointer-events: none; }
.action-icon:disabled:hover, .action-icon.disabled:hover { background: transparent; color: var(--color-text-muted); }

/* ========== 顶部选项卡 ========== */
.tab-item { padding-bottom: 12px; color: var(--color-text-weak); font-weight: var(--fw-400); font-size: var(--fs-14); cursor: pointer; position: relative; transition: color 0.2s; }
.tab-item:hover { color: var(--color-text-secondary); }
.tab-item.active { color: var(--color-primary); font-weight: var(--fw-600); border-bottom: 2px solid var(--color-primary); }

/* ========== 提示框 ========== */
.tooltip-title { font-size: var(--fs-14); font-weight: var(--fw-700); color: var(--color-text); margin-bottom: 10px; }
.tooltip-item { margin-bottom: 8px; font-size: var(--fs-12); line-height: 1.5; color: var(--color-text-muted); }
.tooltip-item:last-child { margin-bottom: 0; }
.tooltip-item strong { display: block; color: var(--color-text-dark); font-weight: var(--fw-600); margin-bottom: 2px; }

/* ========== 帮助图标 ========== */
.help-icon-wrapper { display: inline-flex; align-items: center; cursor: help; position: relative; }
.help-icon { color: var(--color-text-weak); transition: color 0.2s; }
.help-icon-wrapper:hover .help-icon { color: var(--color-text-muted); }

/* ========== 标签 ========== */
.pill { padding: 4px 10px; border-radius: 999px; font-size: var(--fs-12); font-weight: var(--fw-600); display: inline-flex; align-items: center; gap: 4px; letter-spacing: 0.3px; }
.pill-green { background: var(--color-success-light); color: var(--color-success); }
.pill-red { background: var(--color-error-light); color: var(--color-error); }
.pill-blue { background: var(--color-primary-light); color: var(--color-primary); }
.pill-grey { background: var(--color-bg-subtle); color: var(--color-text-muted); font-weight: var(--fw-400); }

/* ========== 表格 ========== */
.flat-table { width: max-content; min-width: 100%; border-collapse: separate; border-spacing: 0; text-align: center; }
.flat-table th, .flat-table td { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; box-sizing: border-box; }
.flat-table th { padding: 12px 20px; font-size: var(--fs-12); font-weight: var(--fw-600); color: var(--color-text-muted); text-transform: uppercase; background: var(--color-bg-page); border-bottom: 1px solid var(--color-border); position: sticky; top: 0; z-index: 10; }
.flat-table td { padding: 12px 20px; font-size: var(--fs-14); color: var(--color-text); border-bottom: 1px solid var(--color-bg-subtle); }
.flat-table tr:last-child td { border-bottom: none; }
.flat-table tr:hover td { background: var(--color-bg-page); }
.table-link { color: var(--color-primary); cursor: pointer; text-decoration: none; font-weight: var(--fw-400); font-size: var(--fs-14); }
.table-link:hover { text-decoration: underline; }

/* ========== 按钮 ========== */
.b-button { background: var(--color-primary); color: var(--color-bg); border: none; padding: 8px 16px; border-radius: 8px; font-size: var(--fs-14); font-weight: var(--fw-400); cursor: pointer; display: flex; align-items: center; transition: all 0.2s; white-space: nowrap; }
.b-button:hover:not(:disabled) { background: var(--color-primary-hover); }
.b-button:disabled { opacity: 0.5; cursor: not-allowed; }
.b-button-outline { background: var(--color-bg); color: var(--color-text-secondary); border: 1px solid var(--color-border); padding: 8px 16px; border-radius: 8px; font-size: var(--fs-14); font-weight: var(--fw-600); cursor: pointer; display: flex; align-items: center; transition: all 0.2s; }
.b-button-outline:hover { background: var(--color-bg-page); color: var(--color-text); border-color: var(--color-border-hover); }
.save-button { background: var(--color-primary-10); color: var(--color-primary); border: none; padding: 6px 14px; border-radius: 10px; font-size: var(--fs-14); font-weight: var(--fw-600); cursor: pointer; display: flex; align-items: center; transition: all 0.2s; flex-shrink: 0; }
.save-button:hover { background: var(--color-primary-20); }

/* ========== 表单 ========== */
.form-group { margin-bottom: 24px; }
.c-label { display: block; font-size: var(--fs-14); font-weight: var(--fw-400); color: var(--color-text-secondary); margin-bottom: 12px; }
.required { color: var(--color-error); }

/* ========== 页面布局 ========== */
.page-header { display: flex; justify-content: space-between; align-items: center; flex-shrink: 0; margin: 0 40px 32px 40px; }
.page-subtitle { font-size: var(--fs-14); color: var(--color-text-muted); margin: 0; }
.list-container { flex: 1; display: flex; flex-direction: column; min-height: 0; margin: 0 40px; }
.scroll-area { flex: 1; overflow-y: auto; padding: 8px 0; }
.empty-state { flex: 1; display: flex; flex-direction: column; justify-content: center; align-items: center; color: var(--color-text-weak); background: var(--color-bg); border-radius: 16px; border: 2px dashed var(--color-border); }
.empty-state p { margin-top: 16px; font-size: var(--fs-14); }

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

::-webkit-scrollbar-corner {
  background-color: transparent;
}

::-webkit-resizer {
  background-color: transparent;
}
</style>
