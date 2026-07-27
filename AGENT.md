# 开发规范

只保留会直接影响实现决策的核心规则。修改前先确认现有做法，精准变更，不顺手重构无关代码。

## 一、Agent 模板优先

- 新增或适配 Agent 时，先判断是否能通过 `src-tauri/agent-definitions/{id}.json` 完成。Agent 名称、识别规则、已有协议、配置路径、配置写入和功能开关应优先由模板声明，不要硬编码 Agent ID 或配置路径。
- 模板格式以 `src-tauri/agent-definitions/agent-definition.schema.json` 为准，字段含义与边界见 `agent-template-guide.md`。
- 只修改现有模板内容时，不应改动通用 Rust 或前端代码。
- 新增模板字段时，必须同步 schema、`db/models.rs`、`services/agent.rs` 的运行时校验、前端共享类型和模板指南。
- 只有模板无法表达的新行为才修改代码，例如新增请求协议、会话解析器、插件生命周期或 MCP 适配器。
- 模板修改后需要重启应用，并在 Agent 页面确认没有“定义加载错误”。

## 二、后端边界

- 数据目录统一通过 `config.rs` 获取；Agent 配置目录和文件路径统一复用 `services/cli_config.rs`，禁止重复实现路径展开。
- 新增 Tauri command 放入 `src-tauri/src/commands/` 对应领域文件。`commands.rs` 仍有存量编排逻辑，但不再继续扩张。
- command 只负责参数接收和流程编排；可复用的业务、配置写入和文件处理逻辑放入 `services/`。
- Agent 配置写入复用 `services/agent_config.rs`、`services/config_patch/` 和 `services/official_credential.rs`，不要为单个 Agent 复制一套写入逻辑。
- 数据库结构统一维护在 `db/schema_definition.rs`，结构变更必须递增对应版本，并通过 `db/mod.rs` 的现有迁移流程执行。
- 数据库模型和跨层共享 DTO 维护在 `db/models.rs`；模块私有类型留在所属模块。
- 面向用户的错误信息必须可读。关键流程使用 `tracing`；需要展示在系统日志中的事件使用现有日志记录方法。

## 三、前端边界

- 技术栈为 Vue 3、TypeScript、Pinia、Vue Router 和 Element Plus；新增组件使用 `<script setup lang="ts">`，保持 TypeScript strict，不用 `any` 逃避类型检查。
- 前端通过 `frontend/src/api/tauri-bridge.ts` 调用 Tauri IPC。页面和组件不得直接写 `invoke`，请求统一封装在 `frontend/src/api/`。
- 跨层类型优先复用 `frontend/src/types/models.ts`；共享状态放 Store，页面私有状态留在页面或页面级组件。
- 页面沿用 `V2Layout.vue` 的布局、设计变量和 `.v2-*` 基础样式；已有组件能满足需求时直接复用，不新增平行设计体系。

## 四、变更与验证

- 后端接口、数据库字段或共享 DTO 变更时，同步检查 Rust 模型、前端 API、前端类型、Store 和页面使用处。
- Agent 模板或 schema 变更时，同步检查内置模板、`agent-template-guide.md`、`README.md` 和 `README-en.md`。
- 只运行与改动相关的检查：前端使用 `pnpm`，禁止混用 `npm`、`yarn` 或 `npx`；Rust 使用 Cargo。
- 前端代码至少执行 TypeScript 检查，Rust 或 schema 代码至少执行 `cargo check`；纯文档修改执行格式和链接检查即可。
- 不修改与任务无关的格式、依赖、锁文件或生成文件。
