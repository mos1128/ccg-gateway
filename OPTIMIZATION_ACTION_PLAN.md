# CCG Gateway 必要优化计划

> 基于 `OPTIMIZATION.md` 与当前代码核对后整理。本文只保留对个人本地使用仍然有明确收益的优化项，不包含日志脱敏类建议。

## 判断原则

- 优先处理会影响稳定性、响应速度、数据迁移安全或明显类型/语法正确性的问题。
- 维护性优化只保留能降低后续改动成本的项目，避免为了形式化分层进行大规模重构。
- 不把个人本地使用场景下可接受的安全暴露当作高优先级问题。

## P0：明确缺陷与低成本修复

### 1. 修复 `AllSettings` 类型缺失 `status`

- 位置：`frontend\src\types\models.ts`、`frontend\src\api\settings.ts`
- 问题：`settingsApi.getAll()` 返回了 `status`，但 `AllSettings` 类型没有声明，只能靠 `as AllSettings` 掩盖。
- 收益：恢复类型约束，避免后续访问系统状态时失去类型检查。
- 建议：给 `AllSettings` 增加 `status: SystemStatus`。

### 2. 删除 `providers/index.vue` 末尾多余字符

- 位置：`frontend\src\views\providers\index.vue`
- 问题：文件末尾 `</style>` 后存在多余 `>`。
- 收益：消除明确的模板/样式解析隐患。
- 建议：直接删除残留字符。

## P1：运行时稳定性与性能

### 3. 优化服务商选择的 N+1 查询

- 位置：`src-tauri\src\services\routing.rs`
- 问题：`select_provider` 和 `get_available_providers` 在 provider 循环中分别查询 model maps 与 blacklist。
- 收益：服务商数量增加时减少数据库往返，降低每次代理请求延迟。
- 建议：
  - 先查出候选 providers。
  - 批量查询 `provider_model_map` 和 `provider_model_blacklist`。
  - 按 `provider_id` 分组后组装 `ProviderWithMaps`。
  - 复用 `commands.rs` 中 `get_providers` 的批量查询思路。

### 4. 插件 CLI 执行改为异步进程与参数数组

- 位置：`src-tauri\src\services\plugin.rs`
- 问题：当前使用 `std::process::Command`，Windows 下还通过 `cmd /c` 拼接 `claude` 命令。
- 收益：避免阻塞 async 运行时，提升插件操作期间的 UI 响应稳定性；参数数组也比字符串拼接更稳。
- 建议：
  - 使用 `tokio::process::Command`。
  - Windows 与非 Windows 都直接执行 `claude` 并通过 `.args(args)` 传参。
  - 将依赖 `run_claude` 的调用链同步改为 async。

### 5. 分批处理 async 命令中的同步文件 IO

- 位置：`src-tauri\src\commands.rs`
- 问题：大量 Tauri async command 中使用 `std::fs::read_to_string`、`std::fs::write`、`std::fs::read_dir` 等同步 IO。
- 收益：减少 UI 卡顿和后台任务阻塞。
- 建议：
  - 高频路径优先改 `tokio::fs`。
  - 复杂目录遍历或大文件处理放入 `tokio::task::spawn_blocking`。
  - 不建议一次性全文件改造，按功能域分批处理：配置读写、MCP/Prompt 状态检查、会话扫描、备份导入导出。

### 6. 数据库迁移新增列优先使用 `ALTER TABLE ADD COLUMN`

- 位置：`src-tauri\src\db\schema_diff.rs`、`src-tauri\src\db\schema_migrator.rs`
- 问题：当前只要表结构 SQL 不一致就重建整表。
- 收益：新增字段场景迁移更快、更稳，降低大表迁移风险。
- 建议：
  - 差异检测增加 `AddColumn` 类型。
  - 仅在删除列、改类型、改约束等复杂变更时保留 `RebuildTable`。
  - 重建前生成唯一临时表名，避免固定 `_old` 冲突。

### 7. 给日志高频查询字段补索引

- 位置：`src-tauri\src\db\schema_definition.rs`
- 问题：`request_logs` 会按时间、CLI 类型、服务商、模型、状态等维度查询，但 schema 定义中没有索引支持。
- 收益：日志量增长后，列表与筛选查询更稳定。
- 建议：
  - 增加索引定义能力。
  - 优先考虑 `created_at`、`cli_type + created_at`、`provider_name + created_at`、`status_code + created_at`。

### 8. 请求详情文件增加清理策略

- 位置：`src-tauri\src\services\stats.rs`
- 问题：请求详情按日期目录落盘，目前只有全量清除能力。
- 收益：长期使用时避免磁盘空间和小文件数量无限增长。
- 建议：
  - 增加按保留天数清理。
  - 初始默认值可以保守，例如 30 或 90 天。

## P2：维护性优化

### 9. 拆分 `commands.rs`

- 位置：`src-tauri\src\commands.rs`
- 问题：单文件约 7700 行，包含 provider、credential、config、MCP、prompt、skill、session、backup 等多个领域。
- 收益：降低定位成本和冲突概率。
- 建议：
  - 先按领域迁移到子模块，不改变行为。
  - 优先拆出变动频繁的 `provider_commands`、`credential_commands`、`mcp_commands`、`prompt_commands`、`session_commands`、`backup_commands`。

### 10. 抽取 CLI 类型相关公共逻辑

- 位置：`src-tauri\src\commands.rs`
- 问题：`match cli_type` 分支大量重复。
- 收益：新增 CLI 类型或修改配置路径时改动面更小。
- 建议：
  - 先抽配置路径、默认文件名、启用状态判断等小 helper。
  - 暂不急于引入完整 trait/策略模式，避免重构过大。

### 11. 拆分大型前端页面

- 位置：`frontend\src\views\providers\index.vue`、`frontend\src\views\skills\index.vue`、`frontend\src\views\plugins\index.vue`
- 问题：页面承担列表、表单、弹窗、动作处理等多种职责。
- 收益：降低后续修改 UI 或业务逻辑时的回归风险。
- 建议：
  - `providers` 优先拆列表项、编辑弹窗、模型检测弹窗。
  - `skills` 与 `plugins` 优先拆表单弹窗和列表项。

### 12. 抽取重复的 `transformCliFlags`

- 位置：`frontend\src\api\mcp.ts`、`frontend\src\api\prompts.ts`、`frontend\src\api\skills.ts`
- 问题：三处实现相同。
- 收益：减少字段转换逻辑不一致的风险。
- 建议：抽到 `frontend\src\utils\cliFlags.ts`。

### 13. 补充基础路由兜底与基础键盘交互

- 位置：`frontend\src\router\index.ts`、`frontend\src\components\AppModal.vue`、`frontend\src\components\AppSelect.vue`
- 问题：缺少 404 兜底，弹窗不支持 ESC，下拉不支持键盘选择。
- 收益：改善基础交互完整性。
- 建议：
  - 添加 catch-all 路由。
  - `AppModal` 增加 ESC 关闭。
  - `AppSelect` 增加上下键与 Enter 确认。

## 不建议的执行方式

- 不建议先做大规模“纯重构”，应把 P0 和 P1 处理完后再逐步拆分。
- 不建议一次性把 `commands.rs` 所有同步 IO 全改完，风险和审查成本都高。
- 不建议为了 DRY 抽象过早引入复杂框架，先抽稳定的小 helper。
