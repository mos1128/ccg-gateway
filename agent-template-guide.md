# Agent 模板字段指南

Agent 模板是一个声明式 JSON 文件，用来告诉 CCG Gateway：如何识别一个 Agent、它使用什么请求协议、配置文件在哪里，以及 CCG 可以为它管理哪些功能。

内置模板位于 `src-tauri/agent-definitions/{id}.json`。用户模板位于 `<数据目录>/agent-definitions/{id}.json`，默认数据目录是 `~/.ccg-gateway`；设置 `CCG_DATA_DIR` 后以该环境变量为准。

用户模板与内置模板的 `id` 相同时，用户模板覆盖内置模板；使用新 `id` 时新增 Agent。模板只在软件启动时加载，修改后需要重启。加载失败时可在 Agent 页面查看“定义加载错误”，同 ID 的有效内置模板仍会继续使用。

## 1. 最小合法模板

除 `remark` 和 `icon` 外，所有顶层 key 都必须出现。`features` 下的 11 个功能也必须全部出现；不支持的功能写成 `{ "enabled": false }`。

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
    "provider_config": { "enabled": false },
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

模板只接受 schema 中声明的 key。字段名拼错、添加未知 key、缺少必填 key 或使用错误类型都会导致模板加载失败。

## 2. 顶层 key

| Key | 类型与可用值 | 含义与注意事项 |
| --- | --- | --- |
| `schema_version` | 整数；当前只能是 `1` | Agent 模板格式版本，不是 Agent 自身版本。 |
| `id` | 非空字符串；只能包含小写字母、数字、`_`、`-` | Agent 的稳定标识。文件名必须是 `{id}.json`。与内置模板同名会覆盖内置模板。 |
| `sort_order` | 大于等于 `0` 的整数 | Agent 的展示顺序；值相同时按 `id` 排序。多个 `user_agent` 同时匹配时，也会优先选择排在前面的 Agent。 |
| `name` | 非空字符串 | 前端显示的 Agent 名称。 |
| `remark` | 可选；非空字符串 | 显示在 Agent 列表和能力信息中，适合说明启用前提或特殊限制。没有实际信息时应省略。 |
| `icon` | 可选对象 | Agent 图标，具体 key 见下文。 |
| `config_dir` | 非空路径字符串 | Agent 的默认配置目录。用户在 CCG 中设置的自定义目录优先。 |
| `config_dir_env` | 可选；环境变量名 | 非空环境变量覆盖 `config_dir`，CCG 自定义目录仍优先。例如 DSH 使用 `DSH_HOME`。 |
| `user_agent` | 至少包含一个非空字符串的数组 | 用于识别请求来自哪个 Agent。匹配时忽略大小写，并按子串匹配。应填写从真实请求中观察到的稳定特征。 |
| `protocols` | 至少包含一个协议值的数组；不能重复 | 声明该 Agent 可能发送的请求协议。可用值见下文。 |
| `features` | 对象 | 功能集合，必须包含第 3 节列出的全部功能 key。 |

### `protocols` 可用值

| 值 | 含义 |
| --- | --- |
| `anthropic_messages` | Anthropic Messages API。 |
| `openai_chat` | OpenAI Chat Completions API。 |
| `openai_responses` | OpenAI Responses API。 |
| `gemini_generate_content` | Gemini GenerateContent API。 |

一个 Agent 可以声明多个协议，但这里只表示“允许接收哪些协议”，不会自动增加协议转换能力。新增协议仍需修改 CCG 的 Rust 代码。

### `icon` 及 `paths` 的 key

| Key | 类型与可用值 | 含义与注意事项 |
| --- | --- | --- |
| `view_box` | 必填；非空字符串 | SVG 的 `viewBox`，例如 `"0 0 24 24"`。 |
| `linear_gradient` | 可选；至少两个 stop 对象 | 从左上到右下的线性渐变，供 `paths[].fill` 引用。 |
| `linear_gradient[].offset` | 必填；`0` 到 `1` 的数字 | 渐变色标位置。 |
| `linear_gradient[].color` | 必填；`#` 加 6 位十六进制颜色 | 渐变色标颜色。 |
| `paths` | 必填；至少一个 path 对象 | SVG 路径列表。不能直接放入完整 SVG。 |
| `paths[].d` | 必填；非空字符串 | SVG path 的 `d` 数据。 |
| `paths[].fill` | 可选；`#` 加 6 位十六进制颜色或 `linear_gradient` | 当前路径的填充色；不设置时使用界面默认图标色。`linear_gradient` 需要同时声明渐变 stop。 |
| `paths[].opacity` | 可选；`0` 到 `1` 的数字 | 当前路径的不透明度。 |
| `paths[].fill_rule` | 可选；`nonzero` 或 `evenodd` | SVG 填充规则。 |
| `paths[].clip_rule` | 可选；`nonzero` 或 `evenodd` | SVG 裁剪规则。 |

### 路径规则

`config_dir` 是其他配置路径的基准目录。它可以写绝对路径，也可以用 `~`、`~/`、`~\` 表示当前用户主目录；建议不要使用其他形式的相对路径。

各功能的 `file` 和 `directory` 按以下规则解析：

- 绝对路径直接使用。
- `~`、`~/`、`~\` 相对当前用户主目录展开。
- 其他路径相对最终生效的 `config_dir` 解析。
- 相对路径允许使用 `..`，因此可以访问 `config_dir` 的上级目录。

## 3. `features` 下的 key

所有功能都有 `enabled`，取值只能是 `true` 或 `false`。设为 `false` 时建议只保留 `enabled`；设为 `true` 时必须提供该功能要求的其他 key。

| 功能 key | 定义 | `enabled: true` 时还需要 |
| --- | --- | --- |
| `provider_config` | 写入 CCG 路由或服务商直连配置 | 非空 `operations`。 |
| `global_preset` | 把全局预设写入默认配置文件 | `file`、`format`。 |
| `profiles` | 为非默认配置档案写入独立配置 | `profile_file`、非空 `operations`；同时必须启用 `provider_config`。 |
| `official_login` | 把 CCG 托管的官方凭证写入 Agent | 非空 `operations`。 |
| `model_mapping` | 允许该 Agent 使用模型映射 | 无其他 key。 |
| `token_usage` | 允许统计该 Agent 的 Token 用量 | 无其他 key。 |
| `skills` | 允许管理该 Agent 的 Skills | `directory` 或 `directories`（二选一）。 |
| `mcp` | 允许管理该 Agent 的 MCP 配置 | `file`、`format`；除 `dsh` adapter 外还需要 `servers_path`。 |
| `sessions` | 允许读取和管理该 Agent 的会话 | `adapter`。 |
| `plugins` | 允许管理该 Agent 的插件 | `adapter`。 |
| `prompts` | 允许管理该 Agent 的提示词文件 | `file`。 |

### `provider_config`

`provider_config` 同时控制两种模式：

- CCG 路由：写入 Gateway 地址和 Gateway Token，请求经过 CCG。
- 服务商直连：写入服务商地址和 API Key，请求不经过 CCG。

两种模式使用同一组 `operations`，运行时通过占位符写入不同内容。

| Key | 类型与可用值 | 含义与注意事项 |
| --- | --- | --- |
| `enabled` | `true` / `false` | 是否支持自动写入服务商配置。 |
| `operations` | operation 对象数组 | `enabled: true` 时不能为空。这里只描述默认配置档案，不能使用 Profile 占位符。 |

#### 配置 operation

一个 operation 只处理一个字段。

```json
{
  "id": "set-endpoint",
  "op": "set",
  "file": "settings.json",
  "format": "json",
  "path": ["env", "API_BASE_URL"],
  "value": "{target.endpoint}"
}
```

| Key | 类型与可用值 | 含义与注意事项 |
| --- | --- | --- |
| `id` | 非空字符串 | operation 的标识。在同一个 `operations` 数组中必须唯一。 |
| `op` | `set` 或 `remove` | `set` 写入字段；`remove` 删除字段。 |
| `file` | 非空路径字符串 | 目标配置文件。默认配置必须写明确路径；Profile operation 可以使用 Profile 占位符。可以包含一个 `*` 目录段，写入时会展开为实际存在的同级目录。 |
| `format` | `json`、`jsonc`、`toml`、`yaml`、`cordis_yaml`、`env` | 目标文件格式。同一组 operations 中，同一个文件不能声明不同格式。`cordis_yaml` 用于 DSH 的 Cordis 补丁。 |
| `private` | 可选布尔值，默认 `false` | 在 Unix 上以 `0700` 创建缺失的父目录、以 `0600` 写文件；即使选择覆盖模式也合并原文件，保留其他凭证。同一文件的 operations 必须使用相同值。 |
| `preserve_on_remove` | 可选布尔值，默认 `false` | 仅供 `set` 使用。关闭时保留此字段，例如共享凭证库的 `version`。 |
| `path` | 至少包含一个非空字符串的数组 | 目标字段路径，例如 `["env", "API_KEY"]`。`env` 格式必须且只能有一个元素。 |
| `value` | 字符串、布尔值、数字、数组或对象 | `op: "set"` 时必填，`op: "remove"` 时禁止出现。字符串中可以使用占位符。 |

`path` 按层级定位字段。例如 `["provider", "openai", "apiKey"]` 表示 JSON/TOML 中的 `provider.openai.apiKey`；对于 `env`，`["API_KEY"]` 表示环境变量 `API_KEY`。

`cordis_yaml` 的 `path` 至少包含两项：首项是 Cordis 条目的 `id`，其余项定位该条目的 `config` 字段。例如 `["llm-pi-ai", "providers", "openai", "baseURL"]` 写为 `- id: llm-pi-ai` 下的 `config.providers.openai.baseURL`。只修改 `CCG Gateway Cordis Config` 标记块，覆盖模式也保留块外的 MCP、用户补丁、注释和 `!!js` 原文；关闭时仅移除仍匹配的受管字段，空管理块被删除。

#### 配置 operation 占位符

| 占位符 | 运行时含义 | 可用位置 |
| --- | --- | --- |
| `{target.endpoint}` | 路由模式为 Gateway 地址；直连模式为服务商地址 | `value`。 |
| `{target.token}` | 路由模式为 Gateway Token；直连模式为服务商 API Key | `value`。 |
| `{agent.id}` | 当前 Agent 的 `id` | `value`。 |
| `{profile}` | 当前非默认配置档案名称 | `profiles.profile_file`，Profile operation 的 `file`、`path`、`value`，以及启动参数。 |
| `{profile.relative_path}` | `profile_file` 展开后的相对路径 | Profile operation 的 `file`、`path`。 |
| `{profile.absolute_path}` | `profile_file` 相对最终 `config_dir` 解析后的绝对路径 | 仅限 `profiles.launch.non_default`。 |

占位符可以嵌在普通字符串中，例如 `"{target.endpoint}/v1"`。数组、对象及其字符串成员也会递归替换。

特殊情况：

- `{target.*}` 和 `{agent.*}` 只能用于 `value`，不能用于 `file` 或 `path`。
- `provider_config.operations` 不能使用任何 Profile 占位符。
- `{profile.relative_path}` 不能用于 `value` 或启动参数。
- `{profile.absolute_path}` 不能用于 operation，只能用于非默认配置档案的启动参数。
- `remove` 在停用时无法恢复原值，应只用于确认可以永久移除的字段。
- 多文件操作按文件分别原子写入，但不提供跨文件事务。部分文件失败后可以重复执行同一配置。

### `global_preset`

| Key | 类型与可用值 | 含义与注意事项 |
| --- | --- | --- |
| `enabled` | `true` / `false` | 是否允许写入全局预设。 |
| `file` | 非空路径字符串 | 默认配置档案使用的预设文件。与配置 operation 一样支持一个 `*` 目录段。 |
| `format` | `json`、`toml`、`yaml` 或 `cordis_yaml` | 预设文件格式。`cordis_yaml` 的输入仍为 YAML 映射，以条目 ID 为键、完整配置对象为值。 |

`global_preset` 只作用于默认配置档案。它与 `provider_config.operations` 写入同一文件和字段时，该预设字段会被忽略，避免覆盖地址、密钥等核心配置。

### `profiles`

| Key | 类型与可用值 | 含义与注意事项 |
| --- | --- | --- |
| `enabled` | `true` / `false` | 是否支持默认配置之外的配置档案。设为 `true` 时，`provider_config.enabled` 也必须是 `true`。 |
| `profile_file` | 包含 `{profile}` 的非空路径字符串 | 非默认配置档案的文件名模式，例如 `"settings-{profile}.json"`。 |
| `operations` | 配置 operation 数组 | 每个非默认配置档案独立执行的写入规则，不能为空，不复用 `provider_config.operations`。 |
| `launch` | 可选对象 | 生成供用户复制的启动命令参数，CCG 不会执行命令。 |
| `launch.default` | 字符串数组 | 默认配置档案的完整命令及参数。`launch` 出现时必填。 |
| `launch.non_default` | 字符串数组 | 非默认配置档案的完整命令及参数。`launch` 出现时必填。 |

`launch.default` 和 `launch.non_default` 不能包含空字符串，也不能使用 `{target.*}`、`{agent.*}` 或 `{profile.relative_path}`。`{profile.absolute_path}` 只能用于 `launch.non_default`。

删除或重命名配置档案时，CCG 会用旧 Profile 上下文逆向处理全部 `profiles.operations`：

- `set` 只删除仍等于模板写入值的字段，用户后来修改过的值会保留。
- `remove` 删除的旧值无法恢复。
- 已有文件无法匹配当前 Gateway 或服务商配置时，操作会报错。

### `official_login`

`official_login` 只负责凭证的读取、保存和写回，不执行登录命令、OAuth 流程或 Token 刷新。用户先在 Agent 中完成登录，再把凭证导入 CCG。

| Key | 类型与可用值 | 含义与注意事项 |
| --- | --- | --- |
| `enabled` | `true` / `false` | 是否支持托管官方凭证。 |
| `operations` | 官方凭证 operation 数组 | `enabled: true` 时不能为空。 |
| `managed_config` | 可选对象 | 官方登录会同时删除 provider/模型配置的 Agent 使用。写回凭证时补回 provider 与 OAuth 引用，清理时移除该 provider、其模型及声明的服务。模式切换仍保持互斥。 |

`managed_config` 字段：

| Key | 类型与可用值 | 含义 |
| --- | --- | --- |
| `file` | 非空路径字符串 | provider 所在的配置文件。 |
| `format` | `toml` | 当前只支持 TOML。 |
| `provider` | 非空字符串 | 官方 provider 名称，例如 `managed:kimi-code`。 |
| `provider_type` | 非空字符串 | provider 的 `type` 值，例如 Kimi 的 `kimi`。 |
| `oauth_key` | 非空字符串 | OAuth 引用中的 `key`。 |
| `oauth_storage` | 非空字符串 | OAuth 引用中的 `storage`，Kimi 使用 `file`。 |
| `base_url` | 可选字符串 | provider 缺失时写入的默认地址。 |
| `base_url_env` | 可选环境变量名 | 优先从该环境变量读取地址。 |
| `oauth_host_env` | 可选环境变量名 | 从该环境变量读取 `oauth_host`。 |
| `services` | 可选字符串数组 | 清理官方模式时需要一并移除的服务配置。 |

#### 官方凭证 operation

| Key | 类型与可用值 | 含义与注意事项 |
| --- | --- | --- |
| `id` | 非空字符串 | operation 标识。 |
| `op` | `replace_file` 或 `set_field` | 替换整个文件，或只设置 JSON / YAML 中的一个字段。 |
| `file` | 非空路径字符串 | 写回凭证或设置的目标文件。 |
| `format` | `json`、`jsonc`、`toml`、`yaml`、`env` | 目标文件格式。`replace_file` 可省略，省略时按 `json`；`set_field` 必须明确写 `json` 或 `yaml`。 |
| `private` | 可选布尔值，默认 `false` | 与配置 operation 相同：写入时使用私有文件权限。共享凭证库应设为 `true`；同一目标文件的声明必须一致。 |
| `preserve_on_remove` | 可选布尔值，默认 `false` | 清理官方凭证时保留该 operation 写入的内容，适用于共享凭证库的格式版本等公共元数据。 |
| `path` | 字符串数组 | `set_field` 的目标字段路径，不能为空；`replace_file` 不使用。 |
| `content_from` | 凭证来源对象 | `replace_file` 必填，表示整文件内容来自哪个逻辑凭证文件。 |
| `value` | 任意固定 JSON 值 | `set_field` 时与 `value_from` 二选一。 |
| `value_from` | 凭证来源对象 | `set_field` 时与 `value` 二选一，表示从凭证中取值。 |

凭证来源对象包含：

| Key | 类型与可用值 | 含义与注意事项 |
| --- | --- | --- |
| `file_id` | 非空字符串 | CCG 凭证数据中的逻辑文件 ID。相同 ID 表示同一份导入内容。 |
| `path` | 可选字符串数组 | 从逻辑文件中取值的路径。省略时取整个文件；非 JSON 来源不能使用该 key。 |

`set_field` 的逻辑凭证文件始终使用 `json` 格式保存提取的字段，目标文件可以是 JSON 或 YAML；`replace_file` 的来源格式与目标格式一致，非 JSON 格式保存原文。同一个 `file_id` 被多个 operation 使用时，来源格式必须一致；同一目标文件的格式也必须一致。对于会自动刷新 Token 的 Agent，CCG 写入的凭证可能随后被 Agent 覆盖，这是正常行为。

DeepSeek Harness 的桌面版与 CLI 共用 `$DSH_HOME/.credentials.yaml`（默认 `~/.dsh/.credentials.yaml`）。内置模板只导入 `records.deepseek-account-platform/default` 的授权记录，包括 `kind` 与 `payload` 中的 `version`、`token`、`issuer`。网关/API 密钥、浏览器会话及设备标识不会进入账号凭证快照。写回时合并账号记录并设置顶层 `version: 1`；清理时只删除匹配的账号记录，保留顶层版本和其他记录。默认目录会读取 CCG 进程的 `DSH_HOME`，显式设置的 CCG 配置目录优先。用户先在 Harness 完成登录，再通过服务商页面的“官方账号”入口读取并保存凭证；在 Harness 中选择 `deepseek-account` 路由使用该登录。

### `model_mapping` 和 `token_usage`

这两个功能只有 `enabled`：

```json
"model_mapping": { "enabled": true },
"token_usage": { "enabled": true }
```

`model_mapping` 表示该 Agent 的请求可以参与模型名称映射；`token_usage` 表示 CCG 可以统计其 Token 用量。它们不需要文件路径或 adapter。

### `skills`

| Key | 类型与可用值 | 含义与注意事项 |
| --- | --- | --- |
| `enabled` | `true` / `false` | 是否支持 Skills 管理。 |
| `directory` | 非空路径字符串 | Skill 根目录；与 `directories` 二选一。相对路径从 `config_dir` 解析。 |
| `directories` | 非空路径数组 | 多个 Skill 根目录；与 `directory` 二选一。启用时写入每个目录，停用时从每个目录删除，任一目录存在即视为启用。 |

Codex 使用官方用户级目录 `$HOME/.agents/skills`：

```json
"skills": {
  "enabled": true,
  "directory": "../.agents/skills"
}
```

Codex 仍会读取旧的 `$CODEX_HOME/skills`，但源码已将其标记为 deprecated；CCG 不再双写，避免同一 Skill 在两个路径各出现一次。`directories` 仍可供用户模板同时维护多个目录。

### `mcp`

| Key | 类型与可用值 | 含义与注意事项 |
| --- | --- | --- |
| `enabled` | `true` / `false` | 是否支持 MCP 配置管理。 |
| `file` | 非空路径字符串 | `enabled: true` 时必填，表示 MCP 所在配置文件。 |
| `format` | `json`、`toml` 或 `yaml` | MCP 配置文件格式；`yaml` 仅供 `dsh` adapter 使用。 |
| `servers_path` | 非空字符串数组 | MCP 服务集合在配置文件中的字段路径，例如 `["mcpServers"]` 或 `["mcp", "servers"]`；`dsh` adapter 不使用且不能配置该字段。 |
| `adapter` | 可选；`opencode` 或 `dsh` | 省略时按 CCG 的标准 MCP 结构直接写入。`opencode` 只支持 `json`；`dsh` 只支持 `yaml`，会把服务转换为 `@deepseek-ai/dsh-mcp-client` Cordis 条目。 |

### `sessions`

| Key | 类型与可用值 | 含义与注意事项 |
| --- | --- | --- |
| `enabled` | `true` / `false` | 是否支持读取和管理会话。 |
| `adapter` | 非空字符串 | `enabled: true` 时必填。当前可用值为 `claude_code`、`codex`、`gemini`、`opencode`、`kimi_code`、`zcode`、`grok`、`pi`、`omp`、`dsh`。 |

Session adapter 是已经编译进 CCG 的解析实现。用户模板不能只靠填写新的字符串来增加一种会话格式。

### `plugins`

| Key | 类型与可用值 | 含义与注意事项 |
| --- | --- | --- |
| `enabled` | `true` / `false` | 是否支持插件管理。 |
| `adapter` | 非空字符串 | `enabled: true` 时必填。当前实际实现为 `dsh`。 |

Plugin adapter 是已经编译进 CCG 的插件生命周期实现，不能由模板新增。`dsh` adapter 通过 `dsh plugin` 命令（pnpm 转发）管理 DeepSeek Harness profile 的插件，并将 CCG 解析的配置根目录传为 `DSH_HOME`。DSH bundle 会自动激活；普通依赖只安装，不会自动激活。`desktop` profile 必须先由桌面版初始化。

### `prompts`

| Key | 类型与可用值 | 含义与注意事项 |
| --- | --- | --- |
| `enabled` | `true` / `false` | 是否支持提示词文件管理。 |
| `file` | 非空路径字符串 | `enabled: true` 时必填。当前只支持一个提示词文件。 |

## 4. 现有模板中的边界情况

以下片段均直接取自内置模板，分别展示容易忽略的特殊情况。

### Claude Code：Profile 文件变量与上级目录

```json
"profiles": {
  "enabled": true,
  "profile_file": "settings-ccg-{profile}.json",
  "operations": [
    {
      "id": "set-profile-endpoint",
      "op": "set",
      "file": "{profile.relative_path}",
      "format": "json",
      "path": ["env", "ANTHROPIC_BASE_URL"],
      "value": "{target.endpoint}"
    },
    {
      "id": "set-profile-token",
      "op": "set",
      "file": "{profile.relative_path}",
      "format": "json",
      "path": ["env", "ANTHROPIC_AUTH_TOKEN"],
      "value": "{target.token}"
    }
  ],
  "launch": {
    "default": ["claude"],
    "non_default": ["claude", "--settings", "{profile.absolute_path}"]
  }
}
```

```json
"mcp": {
  "enabled": true,
  "file": "../.claude.json",
  "format": "json",
  "servers_path": ["mcpServers"]
}
```

这里同时展示了三种路径：

- `profile_file` 定义非默认配置档案的文件名模式。
- operation 用 `{profile.relative_path}` 指向该文件。
- 启动参数用 `{profile.absolute_path}` 得到最终绝对路径。

Claude Code 的 `config_dir` 是 `~/.claude`，因此 MCP 的 `../.claude.json` 最终指向 `~/.claude.json`。这说明 `file` 不必位于 `config_dir` 内部。

### Codex：TOML 嵌套字段与不同类型的固定值

```json
{
  "id": "set-provider-name",
  "op": "set",
  "file": "config.toml",
  "format": "toml",
  "path": ["model_providers", "ccg-gateway", "name"],
  "value": "ccg-gateway"
}
```

```json
{
  "id": "disable-openai-auth",
  "op": "set",
  "file": "config.toml",
  "format": "toml",
  "path": ["model_providers", "ccg-gateway", "requires_openai_auth"],
  "value": false
}
```

```json
{
  "id": "set-token",
  "op": "set",
  "file": "config.toml",
  "format": "toml",
  "path": ["model_providers", "ccg-gateway", "experimental_bearer_token"],
  "value": "{target.token}"
}
```

同一个 `operations` 可以同时写固定字符串、布尔值和运行时占位符。`path` 的每个元素对应一层 TOML 结构，不应把 `model_providers.ccg-gateway` 合并成一个字符串。

Codex 的官方凭证直接替换整个文件：

```json
"official_login": {
  "enabled": true,
  "operations": [
    {
      "id": "replace-codex-auth",
      "op": "replace_file",
      "file": "auth.json",
      "content_from": { "file_id": "codex_auth" }
    }
  ]
}
```

`format` 被省略，因此按 `json` 处理；`content_from` 没有 `path`，表示写入整个逻辑文件 `codex_auth`。

### OpenCode：一个 Agent 支持多协议及 MCP 结构适配

```json
"protocols": ["openai_responses", "anthropic_messages", "openai_chat"]
```

```json
{
  "id": "set-anthropic-endpoint",
  "op": "set",
  "file": "opencode.json",
  "format": "json",
  "path": ["providers", "anthropic", "settings", "baseURL"],
  "value": "{target.endpoint}/v1"
}
```

```json
"mcp": {
  "enabled": true,
  "file": "opencode.json",
  "format": "json",
  "adapter": "opencode",
  "servers_path": ["mcp", "servers"]
}
```

`protocols` 可以包含多个值。`{target.endpoint}` 可以作为字符串的一部分使用，因此 Anthropic 地址会额外带上 `/v1`。OpenCode V2 的 provider 位于 `providers.<id>.settings`，MCP 位于 `mcp.servers`；模板只支持 V2 结构。OpenCode 的 MCP 元素结构与标准结构不同，所以声明 `adapter: "opencode"`；该 adapter 只能配合 `json`。

OpenCode V2 的会话存放在 `opencode.db` 的 `session_v2` / `session_message` 表；CCG 只读取 V2 结构，旧版 `session` / `message` / `part` 表需要先由 OpenCode 完成 V1 → V2 迁移。

### DeepSeek Harness：用户级配置、凭据和 Cordis MCP patch

```json
"mcp": {
  "enabled": true,
  "file": "cordis.patch.yml",
  "format": "yaml",
  "adapter": "dsh"
}
```

`dsh` adapter 不使用 `servers_path`。它将 CCG 的 stdio 或 Streamable HTTP MCP 配置转换为独立的 `@deepseek-ai/dsh-mcp-client` 条目，并只更新根目录 `cordis.patch.yml` 中由 CCG 标记的管理块；该块只 insert 新条目，不会覆盖 profile 配置，块外的用户 patch 保持原文。DSH 的 `serverName` 要求 MCP 名称匹配 `[A-Za-z0-9_-]{1,32}`，且不支持旧式 SSE transport。DSH 凭据写入 `.credentials.yaml` 时应为对应 operation 配置 `"private": true`。

DeepSeek Harness 模板接管原生 `llm-deepseek`（DeepSeek 官方 API 路由，Anthropic Messages）以及 `llm-pi-ai` 内置 `openai` 和 `anthropic` 路由。原生路由只写入 `baseURL` 和 `apiKeyEnv`，其余字段沿用 DSH 的默认 catalog；如需自定义 `models`、`reasoningEffort` 等参数，请放入 CCG 全局预设的 `llm-deepseek` 条目。两个 `llm-pi-ai` 路由复用 DSH 已安装的 provider catalog，不声明 `api`，因此继续使用每个模型在目录中的协议和模型列表；启用时也会移除旧管理块中固定的 `api` 覆盖。不需要声明静态模型列表或额外安装 SDK。三条受管路由共用 `.credentials.yaml` 中的 `refs.CCG_GATEWAY_API_KEY`；写入时确保 `version: 1` 并移除旧版顶层同名密钥，关闭时保留版本和其他凭证。仅支持新版凭证结构，不迁移旧版平铺密钥；升级后停用再启用即可重建 CCG 密钥，其他旧密钥需要用户自行重新配置。保留已有的 `refs` 和 `records`，不要覆盖官方网页搜索使用的 `DEEPSEEK_API_KEY`。`llm-deepseek-account` 仍是官方账号登录路由，不经过网关。

服务商配置和全局预设使用 `cordis_yaml` 写入各 profile 自己的 `profiles/<name>/cordis.patch.yml`，不再写入根目录 home patch，也不再依赖 `settings.yaml` 的一次性导入。目标文件为 `profiles/*/cordis.patch.yml`：写入时展开所有已初始化（存在 `package.json`）的 profile，并额外包含 `desktop` 和 `web`（目录不存在时创建）。根目录 home patch 的优先级高于 profile patch，会覆盖 profile 配置，并让 DSH 设置编辑器因“被 home patch 覆盖”而拒绝保存；升级时会移除根目录中旧的 CCG 管理块。DSH 按完整 `config` 替换同 ID 条目的配置；接管期间需要保留的额外 provider、自定义模型和参数应放入 CCG 全局预设的对应 ID 下。移除管理块后，profile 自身的设置重新生效。

写入或切换模式时，DSH 适配会通过 YAML 节点编辑，清理旧 `settings.yaml` 和已初始化 profile 的 `cordis.patch.yml` 中使用 `CCG_GATEWAY_API_KEY` 的路由字段。直接移除对应字段，不追加覆盖条目；保留其他配置、注释、管理标记和 `!!js` 表达式，支持块式、流式 YAML 及文档结束符。写回前校验完整 YAML，清理幂等，避免关闭新管理块后重新启用旧网关地址。`settings.yaml.imported` 是 DSH 的历史导入文件，不参与运行配置，CCG 不修改它。

DSH 的会话位于 `<config_dir>/sessions/--<cwd>--/<session-id>/session.vN.jsonl[.zstd]`。`dsh` session adapter 只读解析物理 header、`user/message`、`assistant/message`、`tool/call` 和 `tool/result`，支持 raw JSONL 与 Zstandard 压缩；删除操作被拒绝，由 DSH 自己管理会话文件。

### ZCode：深层路径、数字和嵌套 `servers_path`

```json
{
  "id": "set-openai-model-context",
  "op": "set",
  "file": "v2/config.json",
  "format": "json",
  "path": ["provider", "ccg-openai", "models", "gpt-5.6-sol", "limit", "context"],
  "value": 1000000
}
```

```json
"mcp": {
  "enabled": true,
  "file": "cli/config.json",
  "format": "json",
  "servers_path": ["mcp", "servers"]
}
```

`value` 会保留 JSON 类型，布尔值和数字不要写成字符串。`path` 和 `servers_path` 都可以有任意合理深度；`servers_path: ["mcp", "servers"]` 表示 MCP 集合位于 `mcp.servers`。

写入前应先关闭 ZCode 桌面端，避免其内存状态回写覆盖 `v2/config.json`。ZCode 适配只覆盖桌面端，不写入 CLI/TUI 的 `v2/provider_config.json`。

## 5. 使用前检查

1. 文件名与 `id` 完全一致，`id` 没有大写字母。
2. `user_agent` 和 `protocols` 来自 Agent 的真实请求，不是根据名称猜测。
3. `features` 下 11 个功能 key 全部存在，开启的功能包含其必填 key。
4. 每个相对路径都从最终生效的 `config_dir` 展开检查过。
5. 同一文件只使用一种 `format`，`env` operation 的 `path` 只有一个元素。
6. 地址和密钥使用 `{target.endpoint}`、`{target.token}`，模板中不包含真实密钥。
7. Profile 占位符只出现在允许的位置，不会覆盖默认配置或其他文件。
8. 修改模板后重启软件，并在 Agent 页面确认没有“定义加载错误”。
