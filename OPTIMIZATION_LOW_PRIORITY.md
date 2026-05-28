# CCG Gateway 低优先级优化清单

> 本文记录“值得做，但不需要优先投入”的优化项。它们主要改善可维护性、边界稳定性、长期使用体验或局部性能，不属于当前必须先处理的问题。

## 执行原则

- 优先在相关功能改动时顺手处理，不建议单独发起大规模重构。
- 每次只处理一个小范围，避免把低优先级优化变成高风险改动。
- 如果某项开始影响实际使用体验，可提升到 `OPTIMIZATION_ACTION_PLAN.md`。

## 后端

### 1. 更优雅地关闭应用

- 位置：`src-tauri\src\lib.rs`、`src-tauri\src\commands.rs`
- 现状：部分退出路径使用 `std::process::exit(0)`。
- 价值：减少强制退出带来的资源清理不完整问题。
- 建议：在菜单退出和命令退出中优先走 Tauri 应用退出流程。

### 2. `expand_home_path` 失败时返回更明确的错误

- 位置：`src-tauri\src\config.rs`
- 现状：主目录获取失败时可能回退为空路径。
- 价值：配置异常时更容易定位问题。
- 建议：让调用方显式处理错误，或至少保留原始路径并记录 warning。

### 3. 抽取重复的定时任务运行记录表定义

- 位置：`src-tauri\src\db\schema_definition.rs`
- 现状：主库和日志库里 `scheduled_task_runs`、`scheduled_task_run_items` 定义重复。
- 价值：降低后续字段变更漏改的概率。
- 建议：抽成共享函数，主库和日志库复用。

### 4. 统一 stats 数据库迁移流程

- 位置：`src-tauri\src\db\mod.rs`
- 现状：`init_stats_db` 使用硬编码 `CREATE TABLE IF NOT EXISTS`，版本号单独写死。
- 价值：长期维护多个数据库 schema 时更一致。
- 建议：后续 stats 表继续扩展时，再接入统一 schema/migrator。

### 5. 表名和临时表名转义

- 位置：`src-tauri\src\db\schema_inspector.rs`、`src-tauri\src\db\schema_migrator.rs`
- 现状：`PRAGMA table_info({})`、`ALTER TABLE ...` 等 SQL 使用字符串拼接。
- 价值：增强 schema 工具的健壮性。
- 建议：增加 identifier quote helper，统一处理内部表名。

### 6. 明确区分主库和日志库初始化参数

- 位置：`src-tauri\src\db\mod.rs`
- 现状：通过路径后缀判断是否是日志库。
- 价值：减少未来改数据库文件名时的隐式耦合。
- 建议：将 `init_db` 拆为显式 `init_main_db` / `init_log_db`，或增加枚举参数。

### 7. 日志 body 体积控制

- 位置：`src-tauri\src\api\handlers.rs`
- 现状：`truncate_body` 只做 UTF-8 转换，没有真正截断。
- 价值：开启完整调试日志时，避免大请求/响应占用过多内存和磁盘。
- 建议：增加可配置上限，例如 1MB 或 5MB，并标记已截断。

### 8. 代理处理中的重复逻辑抽取

- 位置：`src-tauri\src\api\handlers.rs`
- 现状：流式和非流式错误处理、响应头复制逻辑有重复。
- 价值：降低后续改错误响应或 header 策略时漏改的概率。
- 建议：抽取错误构造 helper 和响应头复制 helper。

### 9. 代理路径的局部性能优化

- 位置：`src-tauri\src\api\handlers.rs`、`src-tauri\src\services\proxy.rs`
- 现状：存在每次编译正则、大 body clone、逐字节拼接等小开销。
- 价值：大请求或高频请求时减少额外消耗。
- 建议：
  - `extract_model_from_path` 的 Regex 使用 `OnceLock`。
  - `apply_body_model_mapping` 仅在需要修改时复制 body。
  - 流式 body 合并前预计算容量。

### 10. Provider 健康测试复用 HTTP client

- 位置：`src-tauri\src\services\provider.rs`
- 现状：测试请求时创建新的 `reqwest::Client`。
- 价值：减少重复 client 初始化开销。
- 建议：复用应用级 client 或服务级静态 client。

### 11. 定时任务历史迁移批量写入

- 位置：`src-tauri\src\services\scheduler.rs`
- 现状：历史 runs/items 逐条插入日志库。
- 价值：历史记录较多时提升迁移速度。
- 建议：使用事务包裹，必要时分批插入。

### 12. 定时任务按 ID 查询服务商

- 位置：`src-tauri\src\services\scheduler.rs`
- 现状：`resolve_provider_ids` 为了匹配少量 ID 读取全部 providers。
- 价值：服务商很多时减少无关查询和内存构造。
- 建议：改为 `WHERE id IN (...)` 查询，再按输入 ID 顺序组装结果。

### 13. skill 存储增加轻量缓存或写入保护

- 位置：`src-tauri\src\services\skill.rs`
- 现状：每次操作读写整个 JSON 文件。
- 价值：skill 数量变多时降低 IO，减少异常退出时写坏文件的概率。
- 建议：先做原子写入；缓存和文件锁等到实际需要再加。

## 前端

### 14. API 参数命名风格收敛

- 位置：`frontend\src\api`
- 现状：API 层混用 snake_case 与 camelCase，再在调用处转换。
- 价值：减少新增接口时的认知成本。
- 建议：前端内部统一 camelCase，进入 Tauri invoke 前集中转换。

### 15. stores 错误处理风格统一

- 位置：`frontend\src\stores`
- 现状：不同 store 对 API 异常处理不一致。
- 价值：减少 loading 状态残留和未捕获异常。
- 建议：补齐写操作 try/finally，错误提示由页面或统一通知层处理。

### 16. sessions 默认分页大小下调

- 位置：`frontend\src\stores\sessions.ts`
- 现状：`pageSize` 默认 1000。
- 价值：项目和会话很多时降低初次加载压力。
- 建议：改为 50 或 100，并保留分页加载能力。

### 17. UI 状态按需持久化

- 位置：`frontend\src\stores\ui.ts`
- 现状：部分 tab、筛选、布局状态刷新后重置。
- 价值：长期使用时减少重复切换。
- 建议：只持久化高频状态，避免把临时弹窗状态写入 localStorage。

### 18. 图标使用方式收敛

- 位置：`frontend\src\views`、`frontend\src\main.ts`
- 现状：页面内有重复 SVG，`main.ts` 全量注册 Element Plus 图标。
- 价值：降低页面模板噪音和包体积。
- 建议：优先把重复 SVG 抽成小组件；图标按需注册可放到后续构建体积优化阶段。

### 19. credentials store 增加按 CLI 缓存

- 位置：`frontend\src\stores\credentials.ts`
- 现状：切换 CLI 类型时覆盖当前列表。
- 价值：减少来回切换时重复请求。
- 建议：参考 providers store 的分片缓存方式。

### 20. 统一插件页通知逻辑

- 位置：`frontend\src\views\plugins\index.vue`、`frontend\src\utils\notification.ts`
- 现状：插件页有自定义 notify，时长和全局工具不一致。
- 价值：交互反馈更一致。
- 建议：改用全局 notification 工具或扩展全局工具参数。

### 21. Dashboard 图表颜色适配暗色模式

- 位置：`frontend\src\views\dashboard\index.vue`
- 现状：图表 palette 使用固定十六进制颜色。
- 价值：暗色模式下可读性更稳定。
- 建议：根据主题选择 palette，或使用 CSS 变量派生颜色。

### 22. AppConfirm 危险操作按钮样式区分

- 位置：`frontend\src\components\AppConfirm.vue`
- 现状：删除等危险操作确认按钮与普通确认按钮视觉一致。
- 价值：减少误操作。
- 建议：根据 confirm type 使用 danger 样式。

### 23. AppSelect 下拉方向自适应

- 位置：`frontend\src\components\AppSelect.vue`
- 现状：靠近窗口底部时仍向下展开。
- 价值：小窗口或底部工具栏场景下避免下拉被截断。
- 建议：打开时检测可用空间，必要时向上展开。

## 文档和规范

### 24. README 补充数据文件和日志清理说明

- 位置：`README.md`、`README-en.md`
- 价值：用户知道数据库、日志、请求详情文件在哪里，以及如何清理。
- 建议：等清理策略实现后一起补充。

### 25. 为关键迁移和代理逻辑补轻量测试

- 位置：`src-tauri\src\db`、`src-tauri\src\services`
- 价值：降低后续重构迁移器、路由选择、模型映射时的回归风险。
- 建议：优先覆盖 schema diff、provider 过滤、model mapping。
