# 开发规范

本文档适用于多人协作维护 CCG Gateway 项目，目标是约束边界、统一风格、避免无序堆逻辑。

---

## 一、全局规范

- 数据目录统一通过 `config.rs` 中的方法获取，禁止在业务代码里硬编码用户目录或额外拼接一套数据根目录。
- 数据库结构变更统一维护在 `db/schema_definition.rs`，每次主库或日志库结构变更都必须同步递增对应 `version`。
- 数据库迁移流程统一走 `db/mod.rs`，禁止绕过 `SchemaInspector / SchemaDiff / SchemaMigrator` 私自写分散迁移逻辑。
- 所有实体、数据库行结构、主要 DTO 统一维护在 `db/models.rs`；数据库字段变更时必须同步更新对应模型与前端类型。
- 后端接口参数、返回结构、字段命名发生变更时，必须同步更新：
  - `frontend/src/api/*`
  - `frontend/src/types/models.ts`
  - 相关页面/Store 的使用代码
- 前端与 Rust 后端通信统一走 Tauri IPC：
  - 前端统一通过 `frontend/src/api/tauri-bridge.ts` 的 `invoke`
  - 禁止组件内直接散写 `@tauri-apps/api/core` 调用
- 前端禁止直接请求业务后端或本地管理接口；需要桌面能力、文件能力、数据库能力时，统一新增 Tauri command。第三方公开 HTTP 接口可按现有模式使用，但要有明确必要性。
- 状态管理统一使用 Pinia；跨页面/跨组件共享状态禁止下沉到组件局部 `ref/reactive`。
- 包管理统一使用 `pnpm`，禁止混用 `npm` / `yarn` 生成新的锁文件。

---

## 二、模块职责

| 文件/目录 | 职责 |
| --- | --- |
| `src-tauri/src/lib.rs` | Tauri 应用装配、状态注册、command 注册、托盘与服务启动 |
| `src-tauri/src/commands.rs` | Tauri 命令入口与桌面侧编排；允许保留现有聚合逻辑，但新增复杂业务优先下沉 `services/` |
| `src-tauri/src/services/` | 业务逻辑层，外部请求、文件操作、CLI/插件/Skill 处理放在这里 |
| `src-tauri/src/services/provider.rs` | Provider 管理、配置同步 |
| `src-tauri/src/services/proxy.rs` | 请求转发、模型映射 |
| `src-tauri/src/services/routing.rs` | 路由规则、负载均衡 |
| `src-tauri/src/services/skill.rs` | Skill 仓库、发现、安装、收藏管理 |
| `src-tauri/src/services/plugin.rs` | Plugin 市场、安装、收藏管理 |
| `src-tauri/src/services/stats.rs` | 统计记录、系统日志写入 |
| `src-tauri/src/api/handlers.rs` | HTTP 代理处理，不直接承载桌面侧 command |
| `src-tauri/src/db/models.rs` | 数据模型、响应结构、DTO |
| `src-tauri/src/db/schema_definition.rs` | Schema 定义与版本号 |
| `frontend/src/api/` | 前端 API 封装，禁止页面直接拼命令名和参数 |
| `frontend/src/stores/` | Pinia 状态管理 |
| `frontend/src/views/` | 页面级 UI 与交互组织 |
| `frontend/src/types/models.ts` | 前端共享类型定义 |

---

## 三、Rust 侧约束

- `commands.rs` 新增接口时，优先保持薄入口：
  - 参数接收
  - 少量组装/编排
  - 调用 `services/` 或 `db` 能力
  - 返回统一错误

- 现有历史逻辑若仍在 `commands.rs`，修改时优先局部收敛，禁止继续扩大单函数职责。

- 返回值统一使用 `Result<T, String>` 或项目内等价别名；错误信息要对最终用户可读，禁止直接抛原始技术栈长报错。

- 如果是调用第三方工具的报错，可以原样返回给用户（例如：调用三方CLI命令）

- 数据库写操作涉及唯一约束、状态切换、批量更新时，优先复用已有错误映射与日志记录模式。

- 关键流程使用 `tracing` 记录；需要进入系统日志列表的事件，统一调用 `stats::record_system_log`。

- 涉及配置文件、凭证、插件、Skill、CLI 配置目录的逻辑，优先复用 `config.rs` 与现有 service 方法，禁止重复实现路径解析与目录探测。

---

## 四、前端约束

- 技术栈以 Vue 3 + TypeScript + Pinia + Vue Router + Element Plus 为准，新增代码默认使用 `<script setup lang="ts">`。
- TypeScript 处于 `strict` 模式，禁止主动引入 `any` 逃避类型检查；确有必要时，范围尽量收窄并说明原因。
- 所有后端返回结构、表单模型、列表项类型，优先复用 `frontend/src/types/models.ts`，不要在页面内重复定义同构接口。
- 页面不得直接写 `invoke('xxx')`，统一经过 `frontend/src/api/*` 封装后再给 Store 或 View 使用。
- 可复用的状态放 Store，可复用的请求放 `api/`，可复用的展示块放 `components/`，不要把请求、状态、视图细节糊在单个页面文件里。
- 项目已启用 `unplugin-auto-import`，新增 Vue / Pinia / Router 常用 API 时先遵循现有自动导入方式，不要重复引入无意义 import。

---

## 五、样式规范

- 全局字体、字号、字重变量定义于 `frontend/src/App.vue`，新增全局文字规范先补变量再使用，禁止到处硬编码。
- 全局 `.mono`、`code`、`pre` 的等宽字体由 `App.vue` 统一定义：
  - 禁止在组件 scoped 样式中重新定义 `.mono`
  - 禁止把颜色、尺寸等语义塞进 `.mono`
- 适合使用等宽字体的场景：
  - 数字 / 字母
  - 代码 / JSON
  - 路径 / URL

---

## 六、变更要求

- 新增功能前先复用现有模块，只有职责明显不合适时才新增文件。

- 任何跨层改动都要做全链路检查：Rust command、前端 API、类型、Store、View、日志/迁移是否同步。

- 修 bug 时优先补“边界约束”而不是只补表面分支，避免同类问题重复出现。
