# Agent 集成最终设计

本文是 Agent 模板、运行时配置和客户端专属代码之间的边界约定。目标是让新增 Agent 尽可能只通过模板描述，同时避免把脚本语言或不安全的动态行为引入配置文件。

## 1. 核心原则

1. Agent 模板可以编译期内置，也可以在启动时从数据目录下的 `agent-definitions` 目录加载；有效用户模板按 `id` 覆盖内置模板，不支持运行时热加载或编辑。
2. UA 只用于识别客户端；请求方法和路径决定 `Protocol`。
3. Provider 的地址、密钥、协议以及 Profile 名称和排序属于 SQLite 运行数据；Profile 如何映射到客户端文件由模板声明。
4. 普通文件配置优先使用结构化 operations；JSON 不执行脚本、命令、表达式或网络下载。
5. `adapter` 只表示已经编译进 CCG 的客户端专属行为。Provider、Profile、Official login 和 Skills 都不使用 adapter；当前只有 Session/Plugin 可以声明 adapter。
6. 未知 Agent、未知协议、未知模板变量都必须报错或进入诊断，不能回退到另一个 Agent。

## 2. 模板结构

内置模板位于 `src-tauri/agent-definitions/{id}.json`；用户模板位于 `<数据目录>/agent-definitions/{id}.json`，默认是 `~/.ccg-gateway/agent-definitions/{id}.json`，设置 `CCG_DATA_DIR` 后使用该目录。两者使用相同结构和运行时校验，文件名必须与模板 `id` 一致。用户目录不存在时只加载内置模板；用户模板读取或校验失败时记录定义错误，并保留同 ID 的有效内置模板。

每个模板必须包含：

```json
{
  "schema_version": 1,
  "id": "example",
  "sort_order": 100,
  "name": "Example CLI",
  "config_dir": "~/.example",
  "user_agent": ["example-cli"],
  "protocols": ["openai_responses"],
  "features": {
    "provider_config": {
      "enabled": true,
      "operations": []
    },
    "global_preset": { "enabled": false },
    "profiles": { "enabled": false },
    "official_login": { "enabled": false },
    "model_mapping": { "enabled": false },
    "token_usage": { "enabled": false },
    "skills": { "enabled": false },
    "mcp": { "enabled": false },
    "sessions": { "enabled": false },
    "plugins": { "enabled": false },
    "prompts": { "enabled": false }
  }
}
```

构建期 Schema 和运行时校验会检查 ID、排序、协议、操作格式、必需的目录/文件和功能参数。有效定义按 `sort_order`、再按 `id` 排序；UA 多重命中会记录诊断并选择排序靠前的 Agent。相对 `file` 和 `directory` 都相对有效 `config_dir` 解析，数据库中的非空自定义目录优先于模板默认目录。

## 3. 能力归属

| 能力 | 模板可描述的部分 | 仍需通用代码或 adapter 的部分 |
| --- | --- | --- |
| Provider 路由/直连 | 文件、格式、字段、运行时值和安全清理 | 通用 operation 执行器；无 Agent 专属 Provider adapter |
| 全局预设 | 目标文件和格式 | 用户预设内容保存在数据库；通用 merge/overwrite 执行器负责写入 |
| Profile | 非默认配置文件、operations 和启动参数 | 通用生命周期代码保存名称/排序并协调数据库与文件；无 Profile adapter |
| Official login | JSON 凭证文件替换/字段写入 | 通用凭证执行器；OAuth、登录命令和 Token 刷新本身不由模板执行，无 Official adapter |
| Skills | 相对 `config_dir` 的目标根目录 | 通用目录复制、检测和删除；无 adapter |
| MCP | 文件、格式和 servers path | 通用 JSON/TOML 执行器；无 adapter |
| Prompt | 单一目标文件 | 通用文本执行器；无 adapter |
| Session/Plugin | 功能开关和 adapter 名称 | 需要客户端专属的会话解析、Plugin 生命周期 adapter |
| 协议代理 | 顶层 `protocols` 选择已有协议 | 新协议的路径识别、鉴权、转换、Token 解析必须改 Rust |

### 3.1 当前内置 Agent

| Agent | Protocol | Provider 配置 | Profile | Official login | Skills |
| --- | --- | --- | --- | --- | --- |
| Claude Code | `anthropic_messages` | JSON operations | `profile_file` + operations + `launch` | 关闭 | 目录 |
| Codex | `openai_responses` | TOML operations | `profile_file` + operations + `launch` | `codex_auth` 文件操作 | 目录 |
| Gemini CLI | `gemini_generate_content` | `.env` + JSON operations | 关闭 | OAuth/账号文件操作 | 目录 |

## 4. Provider 配置执行器

实现位于 `src-tauri/src/services/agent_config.rs` 和 `src-tauri/src/services/config_patch/`。

### 4.1 模式契约

```json
"provider_config": {
  "enabled": true,
  "operations": [/* target.* */]
}
```

`provider_config.operations` 只负责默认配置，必须写明固定的 `file`、`path` 和客户端配置项标识，不能使用 Profile 模板变量。CCG 路由和服务商直连写入相同的客户端字段，共用这组 operations；`value` 中的 `{target.endpoint}`、`{target.token}` 由当前模式解析为 Gateway 或服务商的地址与密钥，普通 `value` 则作为固定值写入。一组 operations 可以同时修改多个文件，执行器按解析后的物理路径分组。

支持的操作：

```text
set          根据 format 设置结构化字段或 ENV 变量
remove       根据 format 删除结构化字段或 ENV 变量
```

### 4.2 Profile 扩展

`profiles` 只负责非默认配置档案。非默认配置使用自己的 operations，不复用 `provider_config.operations`；默认配置只在 `launch.default` 中保留启动命令。

```json
"profiles": {
  "enabled": true,
  "profile_file": "{profile}.config.toml",
  "operations": [
    {
      "id": "select-profile-provider",
      "op": "set",
      "file": "{profile.relative_path}",
      "format": "toml",
      "path": ["model_provider"],
      "value": "ccg-gateway-{profile}"
    }
  ],
  "launch": {
    "default": ["codex"],
    "non_default": ["codex", "--profile", "{profile}"]
  }
}
```

`profiles.profile_file` 必须包含 `{profile}`，用于文件状态、启动路径、改名和删除。`profiles.operations` 可以使用 `{profile}` 和 `{profile.relative_path}`；`{profile.absolute_path}` 是将相对路径相对有效 `config_dir` 解析后的绝对路径，只应在 `launch.non_default` 中使用。变量只做受限字符串替换；`launch` 仅生成供用户复制的命令，不执行模板中的命令。

Profile 删除和重命名必须先解析旧 Profile 的全部 operation 目标。`set` operation 仅在当前值仍等于旧上下文写入值时逆向删除；`remove` operation 不可恢复。重命名在旧上下文清理后迁移 `profile_file` 和数据库记录，再按新上下文正向执行全部 operations；删除在数据库记录删除前完成逆向清理。其他 operation 目标只清理受管字段，不直接删除文件。已有目标文件但无法匹配 Gateway 或任何服务商上下文时，删除和重命名必须报错。

Gateway Token 仍由通用路由规则生成：默认配置为 `ccg-gateway`，非默认配置档案为 `ccg-gateway-{profile}`。

### 4.3 写入生命周期

对路由和直连写入都遵循同一流程：

1. 解析模板变量并解析配置目录。
2. `merge` 从现有文档开始；如果预设变更，先条件移除旧预设。
3. `overwrite` 从空文档开始。
4. 合并 `global_preset`（仅默认配置档案），再用当前模式的 `{target.*}` 执行 operations。
5. 先完成全部目标的结构化解析和内容计算，再逐文件原子替换；任一格式错误都会阻止本轮开始写入。

重复启用是幂等的。模式切换先用旧模式的 `{target.*}` 条件清理，再用新模式的 `{target.*}` 写入同一组字段。

多文件没有跨文件系统事务。预检可以避免“后一个文件损坏、前一个文件已写”的情况，但磁盘、权限或进程中断仍可能使部分原子替换成功；重新执行同一目标模式即可幂等修复。

### 4.4 检测与禁用

检测不是读取固定字段的 Agent 分支，而是把整组模板 operations 应用于当前文件并进行 JSON/TOML/JSONC/ENV 语义比较。所有目标文件和所有操作都匹配时，模式才算生效。Dashboard 默认 Profile 的直连检测会遍历该 Agent 的 Provider；服务商页面也按 Profile 检测。

禁用使用 `safely_remove_operations`：

- 只有当前值仍等于 CCG 解析出的写入值时才反向删除。
- 用户后来修改过的值不删除。
- JSON 对象和 TOML 表在变空后向上清理。
- `.env` 只移除模板声明的键，不删除文件或其他键。

全局预设使用同样的条件移除规则。这样停用路由或直连不会破坏用户后来编辑的配置，也不需要按 Agent ID 编写清理函数。

## 5. 模式识别顺序

模式没有单独持久化枚举，每次从客户端文件和 CCG 数据检测：

```text
operations 匹配 Gateway 上下文          -> proxy_route
operations 匹配 default Provider 上下文 -> provider_direct
匹配已保存 Official credential    -> official_direct
否则                              -> disabled
```

Gateway 路由优先于服务商直连，服务商直连优先于官方凭证。Profile 的请求路由仍由 Gateway Token 选择；Gemini 因 `profiles.enabled: false` 固定使用 `default`。

## 6. Official login 边界

Official login 使用独立的 `official_login.operations`：

- `replace_file` 用逻辑 `file_id` 完整替换凭证文件。
- `set_field` 只写指定 JSON 字段，适合还包含用户设置的文件。
- 不执行 OAuth、CLI 登录命令或脚本。

Official operation 的 `value_from` 是已导入凭证中的字段来源，与 Provider operation `value` 中的 `{...}` 模板变量不是同一概念。

Official credential 的写入、检测和清理由同一个通用凭证执行器根据模板正向或反向执行，不按 Agent ID 分派清理函数，也不应与 Provider operations 混合。Gemini 的 `.env` 和 `settings.json` 可以同时存在：Provider 停用只清理 `.env` 中的 Provider 键和 settings 中仍匹配的 Provider 字段；Official operations 独立管理 OAuth 文件和认证类型。

## 7. 数据归属

模板保存（内置或用户提供）：

- Agent ID、名称、排序、UA、Protocol。
- 功能最终状态。
- 文件格式、路径和结构化操作。
- Profile 的非默认配置文件、operations 和启动参数模式。
- 需要时的 Session/Plugin adapter 名称。
- Official credential 的逻辑文件来源和目标。

SQLite 保存：

- Provider 地址、API Key、Protocol、模型、价格、排序和熔断状态。
- Profile 名称和排序。
- `cli_settings` 的配置目录、全局预设文本、写入模式和最近使用记录。
- Official credential 实例、日志、统计和诊断。

不把 Agent 排序、功能或用户模板内容保存到 SQLite。

## 8. 前端与命令层约束

- Agent 列表、Dashboard、Provider 标签和功能入口从 `get_agents` 返回的模板读取。
- Dashboard 的路由和直连按钮都检查 `provider_config.enabled` 及 `operations` 非空；不维护 Agent ID 白名单。
- Provider 写入、检测、清理统一调用 `agent_config`；`commands.rs` 不再包含 Claude/Codex/Gemini Provider 分支。
- Profile 页面统一调用模板驱动的文件状态和启动命令接口；改名时按旧上下文安全清理，重命名文件后用新上下文重新应用 operations。
- Skills 页面只读取 `skills.directory`，并相对有效 `config_dir` 解析，不得读取或显示 adapter。

## 9. 数据库与升级

Provider 表的 `protocol` 是必填运行数据，创建和更新时必须属于 Agent 顶层 `protocols`。数据库 Schema 升级只负责补齐运行数据；不会扫描或迁移旧 Codex `[profiles.*]`、旧 Provider key 或旧凭证文件格式。`cli_settings` 不保存模式枚举，模式由文件检测恢复。

启动时加载并合并 Agent 模板，只为当前有效定义补齐缺失的 `cli_settings` 行，不主动重写客户端文件。新增、修改或删除用户模板需要重启软件后生效；移除模板不会删除其已有 SQLite 运行数据。配置写入发生在用户切换模式、写入 Provider、确保 Profile、保存 Official credential 或同步 MCP/Skill 时。

## 10. 验证要求

至少覆盖：

1. 三份内置模板的构建期 Schema 和运行时校验。
2. 用户模板新增、同 ID 覆盖、无效覆盖回退和定义错误记录。
3. JSON、TOML、JSONC、ENV 的设置、检测、条件清理和幂等重复写入。
4. Gemini `.env` 与 `settings.json` 的多文件写入，停用后保留非 CCG 内容。
5. Codex/Claude 非默认 Profile operations、文件和重命名/删除。
6. merge/overwrite 的预设更新、用户修改值保留和配置损坏保护。
7. Provider 直连、Gateway 路由、官方直连和停用之间的模式切换。

常用命令：

```powershell
cd src-tauri
cargo fmt --all -- --check
cargo test --lib
cargo check --all-targets

cd ..\frontend
pnpm exec vue-tsc --noEmit
```

## 11. 明确不做

- 运行时编辑或热加载 Agent JSON。
- 模板执行 Shell、JavaScript、Python、动态库或网络下载。
- 自定义 UA 正则、显式 Agent 路由前缀或跨协议转换。
- 通用 Session DSL、OAuth DSL 或 Plugin 安装 DSL。
- 旧版 Codex Profile 配置迁移。
- 为 Skills 预留 adapter。
