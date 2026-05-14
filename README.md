# CCG Gateway

中文 | [English](README-en.md)

<div align="center">
<strong>智能 AI 模型网关 | 统一代理 · 负载均衡 · 故障转移</strong>

[![Rust](https://img.shields.io/badge/Rust-1.80+-orange.svg)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.0+-blue.svg)](https://tauri.app/)
[![Vue](https://img.shields.io/badge/Vue-3.5+-brightgreen.svg)](https://vuejs.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.9+-blue.svg)](https://www.typescriptlang.org/)
[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
</div>

## 📖 项目简介

CCG Gateway 是一款为 Claude Code、Codex、Gemini CLI 打造的桌面端管理工具，集智能网关与配置管理于一体。  
本项目根据作者实际需求立项，解决使用过程中遇到的各种痛点，开发过程中参考了部分开源项目，详见 [致谢](#-致谢)。

---

## 🔥 核心问题

**服务商不稳定**

服务商出现额度重置窗口期、限流、宕机等情况？网关自动切换至可用服务商，定期回检，用户零感知。  
更多便捷功能：服务商可用性检测；模型名称映射；跳过缺失的模型，自动路由至可用服务商；自定义请求的 User-Agent。

**多账号切换繁琐**

多个官方账号 or 多个中转服务商？拖拽快速切换账号 / 调整优先级。

**多项目多服务商并行**

同一个 Agent，多项目并行开发，想不同的项目使用不同服务商？支持配置多套 Profile ，各 Profile 独立配置服务商。

**成本难以估算**

统计报表涵盖了服务商 / 模型双维度的 Token 用量（输入 / 输出 / 缓存），将其代入服务商计费规则，轻松估算成本。  
使用次数计费的 codingplan ？统计报表也涵盖了服务商 / 模型双维度的请求数量。

**请求信息不透明**

请求日志记录了每次调用的状态、耗时、Token计量（多维度）、Agent 请求、服务商响应等数据，信息一目了然。

**会话难以追溯**

按项目分组浏览会话历史，可查看 AI 的思考过程、工具调用及返回结果。

**多 Agent 重复配置**

MCP、预设提示词、Skill、plugin 等工具仅需一次配置，即可快速应用到多个 Agent 。

**跨设备同步配置**

支持本地导出和 WebDAV 云备份，跨设备快速恢复完整配置。

---

## 📸 界面预览

<div align="center">
  <img src="img/PixPin_2026-04-12_17-27-28.png" width="48%" />
  <img src="img/PixPin_2026-04-12_17-27-53.png" width="48%" />
  <img src="img/PixPin_2026-04-12_17-29-51.png" width="48%" />
  <img src="img/PixPin_2026-04-12_17-31-51.png" width="48%" />
  <img src="img/PixPin_2026-04-12_17-32-07.png" width="48%" />
  <img src="img/PixPin_2026-04-12_17-32-49.png" width="48%" />
  <img src="img/PixPin_2026-04-12_17-33-03.png" width="48%" />
  <img src="img/PixPin_2026-04-12_17-39-14.png" width="48%" />
</div>

---

## 💡 功能说明

> 此处仅列出一些独特功能做介绍，方便快速上手。

### 统计总览

统计总览记录了**服务商 / 模型**双维度的**请求次数**和 **Token 用量**，其中 Token 用量又细分为**输入 / 输出 / 缓存**三种数据。  
服务商的定价为：输入10元 / M，缓存5元 / M，输出30元 / M；代入 Token 用量即可快速估算使用成本，也可以为购买 CodingPlan 作出决策。

### 中转服务商

- 模型映射：Agent 模型名称与服务商模型名称不一致时自动映射，无需手动修改配置文件中的模型名称。
  - 通配符：`*` 任意长度字符，`?` 单个字符
  - 示例：`*opus* -> gml-5`  表示将名称含有 opus 的模型映射到服务商的 gml-5 模型
- 模型黑名单：配置服务商不支持的模型，请求时自动跳过该服务商，路由至支持的服务商。
- 故障拉黑：连续失败 N 次后自动拉黑 M 分钟，定期自动恢复。

### 官方账号

- 支持多个账号的凭证配置，支持从 Agent 一键读取。
- 支持拖拽快速切换当前使用的账号凭证。
- 官方账号不通过网关转发，走 Agent 自身请求，规避安全风险。

### 全局设置

- CLI 运行配置：支持配置 Agent 的数据目录，便于 WSL 用户正确写入文件。
- 全局预设：会写入各 Agent 的配置文件中（例如 `~/.claude/settings.json` ），无需配置 BASE_URL 和 AUTH_TOKEN，网关会自动写入。
- 增量 / 全量写入：增量写入会保留 Agent 自己写入的配置，全量写入不会保留 Agent 自己写入的配置。

### 日志管理

- 请求日志：分为请求元数据与请求详情
  - 元数据：请求时间、Agent、服务商、状态、耗时、TOKENS明细、模型映射、错误信息等。
  - 请求详情：Agent 请求头 / 请求体、网关转发请求头 / 请求体、服务商响应头/响应体。
- 日志级别：全量记录、失败时记录详情、停用日志；全量记录无论请求是否成功都会记录请求详情，停用日志则不会记录任何日志信息。
- 请求详情数据通过文件存储，可清理大体积日志同时保留元数据。

### MCP / 提示词 / Skill / 插件管理

- **MCP**：一次配置，多 CLI 启用/禁用，codex 会自动转换为 Toml 格式
- **预设提示词**：一次配置，多 CLI 启用/禁用
- **Skill**：可视化管理，支持从本地目录或远程 Git 仓库安装，提供技能收藏与快速重装功能
- **plugin**：可视化管理，支持从本地目录或远程 Git 仓库安装，提供插件收藏与快速重装功能

### 外观与体验

- **主题切换**：支持全局浅色/暗色主题一键切换
- **古法调色**：人工配色，提供舒适的视觉体验

---

## 🚀 快速开始

### 方式一：Releases 下载（多平台）

1. 前往 [Releases](https://github.com/mos1128/ccg-gateway/releases) 页面下载最新版本
2. 根据操作系统选择对应文件。

### 方式二：Scoop 安装（Windows）

```powershell
scoop install extras/ccg-gateway
```

### 方式三：从源码运行

#### 环境要求

- Rust 1.80+
- Node.js 18+
- pnpm 11+

#### 快速启动

**方式 3-1：一键启动脚本**

脚本会自动启动前端开发服务器和 Tauri 后端，需要安装 `tauri-cli` 。

```bash
# 启动开发环境（前端 + 后端）
./dev.bat
```

**方式 3-2：手动安装依赖并启动**

通过 `cargo` 直接运行，不支持热重载，需要手动重启后端。

```bash
# 启动前端开发服务器
cd frontend
pnpm install
pnpm dev

# 新开终端，启动 Tauri 后端
cd src-tauri
cargo run
```

---

## ⚙️ 配置指南

### 环境变量配置

CCG Gateway 通过环境变量进行配置，所有配置项均有默认值，开箱即用。

| 环境变量 | 默认值 | 说明 |
|---------|------|------|
| `CCG_GATEWAY_HOST` | `127.0.0.1` | 后端 API 服务器监听地址 |
| `CCG_GATEWAY_PORT` | `7788` | 后端 API 服务器端口 |
| `CCG_DATA_DIR` | `~/.ccg-gateway` | 配置文件和日志文件存储目录 |
| `CCG_LOG_FILE` | `false` | 设为 `true` 或 `1` 开启文件日志 |
| `CCG_LOG_LEVEL` | 见下方说明 | 日志级别配置 |

**CCG_LOG_LEVEL 说明**

支持分模块配置日志级别，格式：`全局级别，模块1=级别，模块2=级别`

- 全局：控制所有模块的默认日志级别
- `ccg_gateway`：桌面应用主程序
- `ccg_gateway_lib`：核心网关库

默认值：`info,ccg_gateway=debug,ccg_gateway_lib=debug`（全局 info，核心模块 debug）  
例如`CCG_LOG_LEVEL=warn,ccg_gateway_lib=trace` 表示全局 warn，但 ccg_gateway_lib 输出 trace 级别日志。

#### 如何设置环境变量

**Windows (PowerShell)**
```powershell
# 临时设置（当前终端会话有效）
$env:CCG_GATEWAY_PORT="8080"
$env:CCG_DATA_DIR="D:\ccg-data"

# 永久设置
[System.Environment]::SetEnvironmentVariable('CCG_GATEWAY_PORT', '8080', 'User')
```

**macOS / Linux (Bash/Zsh)**
```bash
# 临时设置（当前终端会话有效）
export CCG_GATEWAY_PORT=8080
export CCG_DATA_DIR="/opt/ccg-data"

# 永久设置（添加到 ~/.bashrc 或 ~/.zshrc）
echo 'export CCG_GATEWAY_PORT=8080' >> ~/.bashrc
echo 'export CCG_DATA_DIR="/opt/ccg-data"' >> ~/.bashrc
source ~/.bashrc
```

---

## 🤝 贡献指南

欢迎提交 Issue 和 Pull Request！

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

---

## 🙏 致谢

感谢各开源作者的贡献：

- [cc-switch](https://github.com/farion1231/cc-switch) - A cross-platform desktop All-in-One assistant tool for Claude Code, Codex & Gemini CLI.
- [coding-tool](https://github.com/CooperJiang/coding-tool) - claudecode|codex|gemini cli 增强工具.
- [code-switch-R](https://github.com/Rogers-F/code-switch-R) - Claude Code & Codex 多供应商代理与管理工具
- [LinuxDo](https://linux.do) - 热情友好的非Linux社区

---

<div align="center">
<strong>如果这个项目对你有帮助，请给一个 ⭐️ Star 支持一下！</strong>
</div>
