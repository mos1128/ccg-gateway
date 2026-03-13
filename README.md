# CCG Gateway

<div align="center">
<strong>智能 AI 模型网关 | 统一代理 · 负载均衡 · 故障转移</strong>

[![Rust](https://img.shields.io/badge/Rust-1.80+-orange.svg)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.0+-blue.svg)](https://tauri.app/)
[![Vue](https://img.shields.io/badge/Vue-3.5+-brightgreen.svg)](https://vuejs.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.9+-blue.svg)](https://www.typescriptlang.org/)
[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
</div>

## 📖 项目简介

CCG Gateway 是一个为多种 AI CLI 工具（Claude Code、Codex、Gemini）设计的智能网关服务，提供统一的代理接口、智能负载均衡和自动故障转移能力。基于 Rust + Tauri 架构打造的桌面应用，性能更强、资源占用更低。

本项目根据作者自身的实际需求立项，开发过程中大量参考了已有开源项目，具体开源项目列表请见 [致谢](#-致谢)。

### 核心特性

- 🎯 **智能路由** - 基于优先级和可用性的自动服务商选择
- 🛡️ **故障转移** - 服务商状态异常，自动切换服务商
- 🗺️ **映射转发** - 服务商支持模型名称映射、UA映射
- 📊 **日志统计** - 完整的请求日志和统计分析
- ⚙️ **预设配置** - 一键注入全局配置、MCP、全局提示词等内容
- 🔄 **动态状态** - 网关状态、MCP、全局提示词根据真实配置实时显示
- 💾 **备份恢复** - 支持配置导出/导入，WebDAV 云备份自动同步
- 🔐 **凭证管理** - 支持多账号凭证存储，方便切换不同官方账号
- 🎛️ **双模式切换** - 支持中转模式/官方模式，各 CLI 可独立配置

---

## 🚀 快速开始

### 方式一：Releases 下载（推荐）

1. 前往 [Releases](https://github.com/mos1128/ccg-gateway/releases) 页面下载最新版本
2. 根据操作系统选择对应文件。

### 方式二：从源码运行

#### 环境要求

- Rust 1.80+
- Node.js 18+
- pnpm

#### 快速启动

**方式 2-1：一键启动脚本（推荐）**

脚本会自动启动前端开发服务器和 Tauri 后端，需要安装 `tauri-cli` ，支持热重载。

```bash
# 启动开发环境（前端 + 后端）
dev.bat
```

**方式 2-2：手动安装依赖并启动**

通过 `cargo` 直接运行，不支持热重载，需要手动重启后端。

```bash
# 安装前端依赖
cd frontend
pnpm install

# 启动前端开发服务器
pnpm dev

# 在另一个终端，启动 Tauri 后端
cd src-tauri
cargo run
```

---

## ⚙️ 配置指南

### 环境变量配置

CCG Gateway 通过环境变量进行配置，所有配置项均有默认值，不配置即可直接使用。

| 环境变量 | 默认值 | 说明 | 适用场景 |
|---------|------|------|--------|
| `CCG_GATEWAY_HOST` | `127.0.0.1` | 后端 API 服务器监听地址 | 打包后的应用 + 开发模式 |
| `CCG_GATEWAY_PORT` | `7788` | 后端 API 服务器端口 | 打包后的应用 + 开发模式 |
| `CCG_DATA_DIR` | `~/.ccg-gateway` | 数据存和日志文件存储目录 | 打包后的应用 + 开发模式 |
| `CCG_LOG_FILE` | `false` | 设为 `true` 或 `1` 开启文件日志 | 打包后的应用 + 开发模式 |
| `CCG_LOG_LEVEL` | 见下方说明 | 日志级别配置 | 打包后的应用 + 开发模式 |

**CCG_LOG_LEVEL 说明**

支持分模块配置日志级别，格式：`全局级别，模块 1=级别，模块 2=级别`

- 全局：控制所有模块的默认日志级别
- `ccg_gateway`：桌面应用主程序
- `ccg_gateway_lib`：核心网关库

默认值：`info,ccg_gateway=debug,ccg_gateway_lib=debug`（全局 info，两个核心模块 debug）

示例：`CCG_LOG_LEVEL=warn,ccg_gateway_lib=trace` 表示全局 warn，但 ccg_gateway_lib 输出 trace 级别日志。

#### 如何设置环境变量

**Windows (PowerShell)**
```powershell
# 临时设置（当前终端会话有效）
$env:CCG_GATEWAY_PORT="8080"
$env:CCG_DATA_DIR="D:\ccg-data"

# 永久设置（需要管理员权限）
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

## 💡 功能介绍

### 1. 网关转发

转发 CLI 的请求到服务商。

- 支持拖拽调整服务商优先级
- 当前服务商不可用时自动切换到下一个服务商
- 自动拉黑连续失败 N 次的服务商 M 分钟
- 支持自定义 UserAgent

> **使用场景**
>
> 小帅订阅了三个中转服务商：服务商 A 每 4 小时重置额度，服务商 B 每 9 小时重置额度，服务商 C 按量计费。
>
> 为了保证可用性和性价比，小帅配置服务商 A 拉黑时长：4 小时，配置服务商 B 拉黑时长：9 小时，将服务商 C **拖拽** 到最后作为兜底。
>
> 网关会优先转发请求到服务商 A，服务商 A 不可用再转发到服务商 B，服务商 B 不可用再转发到服务商 C，当服务商 A 恢复后继续使用服务商 A。

### 2. 模型映射

解决服务商模型名称与 CLI 模型不一致的问题。

支持的通配符：`*` - 匹配任意数量字符；`?` - 匹配单个字符

> **使用场景**
>
> 服务商 A 的模型命名是 `cc-opus-4.5`，而其他服务商的模型命名都遵循官方是 `claude-opus-4-5-20251101`，小帅就为服务商 A 配置了模型映射：
>
> ```
> *opus* -> cc-opus-4.5
> *haiku* -> cc-haiku-4.5
> ```
>
> 这样 CLI 无需任何额外配置，所有请求都能正确转发到服务商。

### 3. 配置管理

- **CLI 全局配置**：为各 CLI 预设全局配置，启用网关时自动注入
- **MCP 配置**：支持配置 MCP，一键应用到各个 CLI
- **提示词预设**：支持配置全局提示词，一键应用到各个 CLI
- **Skill 管理**：自定义 Skill 源；一键安装/重装/卸载 Skill；一键应用到各个 CLI
- **超时设置**：可配置流式和非流式请求的超时时间
- **模式切换**：支持中转模式/官方模式切换，各 CLI 独立配置

### 4. 请求日志与统计

- **请求日志**：详细记录每个请求的完整信息（请求内容、响应内容、耗时、token 用量等）
- **系统日志**：记录服务商切换、故障、拉黑等系统事件
- **数据统计**：可视化看板展示请求趋势、服务商成功率、Token 消耗

> **使用场景**
>
> 小帅求知欲强，想知道 CLI 工作时会发送哪些请求，请求的内容是什么。

### 5. 会话管理

可查看各个 CLI 的所有会话。

> **使用场景**
>
> 小帅求知欲强，想看每个会话 AI 思考了什么内容，调用了什么工具，工具的返回结果是什么。

### 6. 凭证管理

支持为各 CLI 添加多个官方凭证，方便在不同账号间快速切换。


---

## ✨ 一些巧思

### 复制服务商

可能服务商为多个 CLI 都提供了服务，重复填也太麻烦了，单独开发复制功能意义也不大。

于是产生了一个可以作为机制的 bug：小帅先添加任意一个 CLI 的服务商，然后点击编辑，再将弹窗关闭。切换到另外一个 CLI 的配置 Tab，点击添加服务商。发现原本的内容居然还在，直接保存就好啦。

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

---

<div align="center">
<strong>如果这个项目对你有帮助，请给一个 ⭐️ Star 支持一下！</strong>
</div>
