# CCG Gateway

中文 | [English](README-en.md)

<div align="center">
<strong>智能 AI 模型网关 | 统一代理 · 直连写入 · 智能故障转移 · 精准计费</strong>

[![Rust](https://img.shields.io/badge/Rust-1.80+-orange.svg)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.0+-blue.svg)](https://tauri.app/)
[![Vue](https://img.shields.io/badge/Vue-3.5+-brightgreen.svg)](https://vuejs.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.9+-blue.svg)](https://www.typescriptlang.org/)
[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

</div>

## 📖 简介

CCG Gateway 是一款面向 Claude Code、Codex、Gemini CLI、OpenCode、Kimi Code、ZCode、Grok Build、Pi、Oh My Pi、DeepSeek Harness 等 Agent 的桌面端管理工具，集智能网关、账号管理、配置管理于一体。

本项目根据作者实际需求立项，解决使用过程中遇到的各种痛点，开发过程中参考了部分开源项目，详见 [致谢](#-致谢)。

---

## ✨ 特性

- 🧩 **多 Agent 管理** - 内置 10+ Agent 模板，也可用自定义模板接入新的 Agent
- 🔌 **多协议路由** - 支持 Anthropic Messages / OpenAI Chat / OpenAI Responses / Gemini generateContent 四类端点
- 🔄 **协议转换** - Anthropic Messages / OpenAI Chat / OpenAI Responses 三者互转，Agent 可直接使用不同协议的服务商
- 🔀 **故障转移** - 上游失败自动重试、切换服务商、熔断冷却并定期回检，用户零感知
- 🛡️ **流式校验** - 流式首包校验通过才落地到客户端，上游报错时静默换服务商，不中断 Agent 任务 
- 🔁 **模型映射** - Agent 与服务商模型名不一致时按通配符自动改写，无需手改配置文件
- 🚫 **模型黑名单** - 服务商不支持的模型自动跳过，路由到支持该模型的服务商
- 💰 **价格同步** - 自动同步模型官方价格（含长上下文分层价），服务商只需配置一个倍率
- 📋 **模型列表同步** - 快速拉取服务商可用模型，模型映射下拉直接选
- 🧪 **可用性检测** - 批量检测多个服务商的模型可用性与耗时
- ⏰ **定时任务** - 在闲置时段发起小额调用，提前额度重置时间；账号保活
- 📊 **全维度统计** - 服务商 / 模型双维度的 Token 用量、费用、请求数与缓存命中率
- 🔍 **请求日志** - 每次调用的状态、首字节 / 总耗时、Token 明细、缓存命中率、费用推导、原始请求与响应
- 💬 **会话追溯** - 按项目分组浏览会话历史，可查看思考过程、工具调用及返回结果
- 🗂️ **多 Profile** - 使用同一个 Agent 并行开发多个项目时，不同项目使用不同服务商
- 🧰 **快捷配置** - MCP、提示词、Skill 配置一次，即可应用到多个 Agent
- ☁️ **跨设备同步** - 本地导出导入与 WebDAV 云备份，跨设备快速恢复完整配置

---

## 📸 界面预览

<div align="center">
  <img src="img/1.png" width="48%" />
  <img src="img/2.png" width="48%" />
  <img src="img/3.png" width="48%" />
  <img src="img/4.png" width="48%" />
  <img src="img/5.png" width="48%" />
  <img src="img/6.png" width="48%" />
  <img src="img/7.png" width="48%" />
  <img src="img/8.png" width="48%" />
  <img src="img/9.png" width="48%" />
  <img src="img/10.png" width="48%" />
</div>

---

## 💡 功能说明

> 此处仅对部分功能点做说明/释义，方便快速上手！！！

### 仪表盘

- 双维度图表：图例可勾选，KPI 统计卡片随勾选联动。
- 支持按日期区间与快捷区间筛选
- 统计数据定时自动刷新，可随时暂停。

### Agent 模板

- 用户模板可以新增 Agent，也可以使用相同 `id` 覆盖内置模板。
- 用户模板默认放在 `~/.ccg-gateway/agent-definitions/{id}.json`，修改后需要重启软件。字段定义、可用值和完整示例见 [Agent 模板开发指南](agent-template-guide.md)。

### 中转服务商

- 端点类型：按 Anthropic / OpenAI Chat / OpenAI Responses / Gemini 声明服务商协议，网关按 Agent 的实际请求路径匹配。
- 协议转换：端点类型与 Agent 的请求协议不同时自动转换请求与响应（含流式），例如 Claude Code 直接使用 OpenAI Chat 的服务商。Anthropic Messages / OpenAI Chat / OpenAI Responses 三者可互转，Gemini 不参与转换，只能原样转发。服务商列表中带转换标记的即为需要转换的服务商。
- 模型映射：Agent 模型名称与服务商模型名称不一致时自动映射，无需手动修改配置文件。
  - 通配符：`*` 任意长度字符，`?` 单个字符
  - 示例：`*opus* -> gml-5` 表示将名称含有 opus 的模型映射到服务商的 gml-5 模型
  - 映射目标支持下拉选择服务商的可用模型，也可手动输入
- 故障转移规则：请求失败后先在当前服务商重试（可配置连续重试次数，默认 3 次），失败后自动切换下一个服务商；所有服务商轮完一圈后从头再轮，直到成功或全部熔断。连续失败达到阈值（默认 5 次）后服务商熔断冷却（默认 10 分钟），期间由其他服务商接管。密钥错误、模型不存在等请求本身的问题直接切换服务商，不浪费重试。

### 多 Profile

- 同一个 Agent 下可创建多个 Profile，各自维护独立的服务商列表，双击标签即可重命名。
- 每个 Profile 都会生成对应的启动命令，可一键复制（例如 `claude --settings ~/.claude/settings-ccg-work.json`）。
- 通过对应启动命令启动的 Agent 互不干扰。

### 官方账号

- 支持多个账号的凭证配置，支持一键读取已登录信息。
- 支持拖拽快速切换当前使用的账号凭证。
- 官方账号不通过网关转发，规避账号风控。

### 全局设置

- 全局预设：会写入各 Agent 的配置文件中（例如 `~/.claude/settings.json`），无需配置 BASE_URL 和 AUTH_TOKEN，网关会自动写入。
- 增量 / 全量写入：增量写入会保留 Agent 自己写入的配置，全量写入不会保留 Agent 自己写入的配置。

### 日志管理

- 请求日志：分为请求元数据与请求详情
  - 元数据：请求时间、Agent、服务商、状态、首字节 / 总耗时、Token 明细、缓存命中率、费用、模型映射、错误信息等。
  - 请求详情：Agent 请求头 / 请求体、网关转发请求头 / 请求体、服务商响应头 / 响应体。
  - 费用明细：每笔费用可查看逐项计算过程（token 数 × 单价 × 倍率）与价格来源；错误信息按「网关捕获 / 上游错误」分类展示。
- 系统日志：记录 User-Agent 未匹配、Agent / 协议配置冲突、无可用服务商等诊断事件，同类事件会自动去重。
- 日志级别：全量记录、失败时记录详情、停用日志；全量记录无论请求是否成功都会记录请求详情，停用日志则不会记录任何日志信息。
- 请求详情数据通过文件存储，可清理大体积日志同时保留元数据。
- 清理粒度：全部日志 / 全部详情 / 统计数据 / 30 天前日志 / 30 天前详情，清理统计数据后可重新统计用量与请求次数。

### MCP / 提示词 / Skill / 插件管理

- MCP：一次配置，多 Agent 启用 / 停用，特殊 Agent 会自动转换格式。
- 提示词：一次配置，多 Agent 启用 / 停用。
- Skill：支持添加远程 Git 仓库或本地目录作为技能仓库，浏览并安装其中的技能，提供收藏、重装、卸载与失效标记。
- 插件：输入 Git 仓库地址即可直接安装 DeepSeek Harness 插件，提供更新与卸载。

### 外观与体验

- 主题切换：支持全局浅色 / 暗色主题一键切换。
- 自动记忆窗口尺寸，下次启动自动恢复。
- 内置更新检查：从 GitHub Releases 检查新版本。

---

## 🚀 快速开始

### 方式一：Releases 下载（多平台）

1. 前往 [Releases](https://github.com/mos1128/ccg-gateway/releases) 页面下载最新版本。
2. 按操作系统选择对应文件：Windows 为 `.exe`，macOS 为 `.dmg`（Universal），Linux 为 `.AppImage`。

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

```bash
# 启动开发环境（前端 + 后端），需要安装 tauri-cli
./dev.bat
```

**方式 3-2：手动安装依赖并启动**

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

| 环境变量               | 默认值              | 说明                      |
| ------------------ | ---------------- | ----------------------- |
| `CCG_GATEWAY_HOST` | `127.0.0.1`      | 后端 API 服务器监听地址          |
| `CCG_GATEWAY_PORT` | `7788`           | 后端 API 服务器端口            |
| `CCG_DATA_DIR`     | `~/.ccg-gateway` | 数据库、日志和用户 Agent 模板的存储目录 |
| `CCG_LOG_FILE`     | `false`          | 设为 `true` 或 `1` 开启文件日志  |
| `CCG_LOG_LEVEL`    | 见下方说明            | 日志级别配置                  |

**CCG_LOG_LEVEL 说明**

支持分模块配置日志级别，格式：`全局级别,模块1=级别,模块2=级别`

- 全局：控制所有模块的默认日志级别
- `ccg_gateway`：桌面应用主程序
- `ccg_gateway_lib`：核心网关库

默认值：

- 开发构建：`info,ccg_gateway=debug,ccg_gateway_lib=debug`（全局 info，核心模块 debug）
- 发布构建：`info`

例如 `CCG_LOG_LEVEL=warn,ccg_gateway_lib=trace` 表示全局 warn，但 ccg_gateway_lib 输出 trace 级别日志。

`CCG_LOG_FILE=true` 会额外写入文件日志，不会关闭控制台或 systemd journal 输出。需要降低 journal 输出时，请设置 `CCG_LOG_LEVEL=warn` 或其他更高阈值。

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
- [linux.do](https://linux.do/) - 友善的非Linux社区

---

<div align="center">
<strong>如果这个项目对你有帮助，请给一个 ⭐️ Star 支持一下！</strong>
</div>
