---
name: 颜色变量系统
description: 全局 CSS 颜色变量规范，统一管理前端色值
type: design
date: 2026-04-11
---

# 颜色变量系统设计

## 背景

前端代码库中存在大量硬编码颜色值（约 46 个唯一值），分散在各 Vue 组件中。统一提取为 CSS 变量后，可提升可维护性和一致性。

## 设计原则

1. **语义命名**：按用途命名，如 `--color-primary`、`--color-text-muted`
2. **全覆盖**：所有硬编码色值均有对应变量
3. **独立变量**：rgba 变体独立定义，不使用 color-mix
4. **统一前缀**：使用 `--color-*` 前缀，与现有 `--fs-*`、`--fw-*` 风格一致

## 变量定义

### 1. 主色系（Primary - Sky Blue）

| 变量名 | 值 | 用途 |
|---|---|---|
| `--color-primary` | `#0ea5e9` | 主色 |
| `--color-primary-hover` | `#0284c7` | 主色悬停 |
| `--color-primary-dark` | `#0369a1` | 主色深（ghost 按钮） |
| `--color-primary-light` | `#f0f9ff` | 主色浅背景 |
| `--color-primary-lighter` | `#e0f2fe` | 主色更浅 |
| `--color-primary-border` | `#bae6fd` | 主色边框 |
| `--color-primary-muted` | `#7dd3fc` | 禁用主色 |
| `--color-primary-4` | `rgba(14,165,233,0.04)` | 选中复选框背景 |
| `--color-primary-5` | `rgba(14,165,233,0.05)` | 禁用按钮背景 |
| `--color-primary-10` | `rgba(14,165,233,0.1)` | 按钮/焦点阴影 |
| `--color-primary-20` | `rgba(14,165,233,0.2)` | 按钮悬停 |
| `--color-primary-30` | `rgba(14,165,233,0.3)` | 图表渐变 |

### 2. 文字色系

| 变量名 | 值 | 用途 |
|---|---|---|
| `--color-text` | `#0f172a` | 主文字 |
| `--color-text-secondary` | `#475569` | 次要文字/标签 |
| `--color-text-muted` | `#64748b` | 弱化文字/图标 |
| `--color-text-weak` | `#94a3b8` | 最弱文字/提示 |
| `--color-text-dark` | `#334155` | 深色文字 |

### 3. 背景色系

| 变量名 | 值 | 用途 |
|---|---|---|
| `--color-bg` | `#ffffff` | 主背景/卡片 |
| `--color-bg-page` | `#f8fafc` | 页面背景 |
| `--color-bg-subtle` | `#f1f5f9` | 次级背景/悬停 |
| `--color-bg-muted` | `#f8fafc` | 表头/分区背景 |
| `--color-bg-tint` | `#f4f7fe` | 视图容器背景 |
| `--color-bg-80` | `rgba(255,255,255,0.8)` | 半透明白 |
| `--color-bg-90` | `rgba(255,255,255,0.9)` | 图表 tooltip |
| `--color-bg-95` | `rgba(255,255,255,0.95)` | 毛玻璃效果 |

### 4. 边框色系

| 变量名 | 值 | 用途 |
|---|---|---|
| `--color-border` | `#e2e8f0` | 主边框 |
| `--color-border-hover` | `#cbd5e1` | 边框悬停 |
| `--color-border-light` | `rgba(226,232,240,0.6)` | 浅边框/tab下划线 |
| `--color-border-medium` | `rgba(226,232,240,0.8)` | 卡片边框 |
| `--color-scrollbar` | `#cbd5e1` | 滚动条 |
| `--color-scrollbar-hover` | `#94a3b8` | 滚动条悬停 |

### 5. 状态色系

| 变量名 | 值 | 用途 |
|---|---|---|
| `--color-success` | `#10b981` | 成功/运行 |
| `--color-success-hover` | `#059669` | 成功悬停 |
| `--color-success-light` | `#ecfdf5` | 成功背景 |
| `--color-success-10` | `rgba(16,185,129,0.1)` | 成功浅背景 |
| `--color-success-30` | `rgba(16,185,129,0.3)` | 图表渐变 |
| `--color-success-40` | `rgba(16,185,129,0.4)` | 状态点阴影 |
| `--color-danger` | `#ef4444` | 危险/删除 |
| `--color-danger-hover` | `#dc2626` | 危险悬停 |
| `--color-danger-light` | `#fee2e2` | 危险背景 |
| `--color-danger-muted` | `#fca5a5` | 危险边框 |
| `--color-error` | `#f43f5e` | 错误/必填 |
| `--color-error-light` | `#fff1f2` | 错误背景 |
| `--color-error-2` | `rgba(244,63,94,0.02)` | 黑名单行背景 |
| `--color-error-10` | `rgba(244,63,94,0.1)` | 黑名单标签背景 |
| `--color-warning` | `#f59e0b` | 警告/收藏 |
| `--color-warning-10` | `rgba(245,158,11,0.1)` | 收藏背景 |

### 6. 阴影色系

| 变量名 | 值 | 用途 |
|---|---|---|
| `--color-shadow` | `rgba(0,0,0,0.03)` | 卡片阴影 |
| `--color-shadow-hover` | `rgba(0,0,0,0.05)` | 悬停阴影 |
| `--color-shadow-md` | `rgba(0,0,0,0.08)` | 分段控件阴影 |
| `--color-shadow-lg` | `rgba(0,0,0,0.1)` | 下拉/tooltip 阴影 |
| `--color-shadow-xl` | `rgba(0,0,0,0.2)` | 弹窗阴影 |
| `--color-scrim` | `rgba(15,23,42,0.1)` | 遮罩层 |
| `--color-scrim-heavy` | `rgba(15,23,42,0.25)` | 模态遮罩 |
| `--color-scrim-dark` | `rgba(15,23,42,0.4)` | 深遮罩 |
| `--color-overlay` | `rgba(148,163,184,0.05)` | 底栏背景 |
| `--color-overlay-8` | `rgba(148,163,184,0.08)` | 分段控件背景 |

### 7. 特殊色系

| 变量名 | 值 | 用途 |
|---|---|---|
| `--color-violet` | `#8b5cf6` | Skills 图标 |
| `--color-violet-light` | `#f5f3ff` | Skills 图标背景 |

## 实施范围

### 修改文件

1. `frontend/src/App.vue` - 新增 `:root` 颜色变量定义
2. `frontend/src/views/**/*.vue` - 替换硬编码色值为变量引用
3. `frontend/src/components/**/*.vue` - 替换硬编码色值为变量引用
4. `frontend/src/layouts/MainLayout.vue` - 替换硬编码色值为变量引用

### 替换规则

| 原值 | 新变量 |
|---|---|
| `#0ea5e9` | `var(--color-primary)` |
| `#0284c7` | `var(--color-primary-hover)` |
| `#0f172a` | `var(--color-text)` |
| `#475569` | `var(--color-text-secondary)` |
| `#64748b` | `var(--color-text-muted)` |
| `#94a3b8` | `var(--color-text-weak)` |
| `#f8fafc` | `var(--color-bg-page)` |
| `#f1f5f9` | `var(--color-bg-subtle)` |
| `#e2e8f0` | `var(--color-border)` |
| `#cbd5e1` | `var(--color-border-hover)` |
| ... | ... |

## 验证标准

1. 所有 `#xxxxxx` 格式的色值已替换为变量（除特殊场景）
2. 所有 `rgba(...)` 格式的色值已替换为变量
3. 应用视觉表现与修改前一致
4. 无新增 CSS 颜色硬编码
