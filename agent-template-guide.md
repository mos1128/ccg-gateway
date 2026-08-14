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
| `skills` | 允许管理该 Agent 的 Skills | `directory`。 |
| `mcp` | 允许管理该 Agent 的 MCP 配置 | `file`、`format`、`servers_path`。 |
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
| `file` | 非空路径字符串 | 目标配置文件。默认配置必须写明确路径；Profile operation 可以使用 Profile 占位符。 |
| `format` | `json`、`jsonc`、`toml`、`yaml`、`env` | 目标文件格式。同一组 operations 中，同一个文件不能声明不同格式。 |
| `path` | 至少包含一个非空字符串的数组 | 目标字段路径，例如 `["env", "API_KEY"]`。`env` 格式必须且只能有一个元素。 |
| `value` | 字符串、布尔值、数字、数组或对象 | `op: "set"` 时必填，`op: "remove"` 时禁止出现。字符串中可以使用占位符。 |

`path` 按层级定位字段。例如 `["provider", "openai", "apiKey"]` 表示 JSON/TOML 中的 `provider.openai.apiKey`；对于 `env`，`["API_KEY"]` 表示环境变量 `API_KEY`。

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
| `file` | 非空路径字符串 | 默认配置档案使用的预设文件。 |
| `format` | `json`、`toml` 或 `yaml` | 预设文件格式。 |

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

#### 官方凭证 operation

| Key | 类型与可用值 | 含义与注意事项 |
| --- | --- | --- |
| `id` | 非空字符串 | operation 标识。 |
| `op` | `replace_file` 或 `set_field` | 替换整个文件，或只设置 JSON 中的一个字段。 |
| `file` | 非空路径字符串 | 写回凭证或设置的目标文件。 |
| `format` | `json`、`jsonc`、`toml`、`yaml`、`env` | `replace_file` 可省略，省略时按 `json`；`set_field` 必须明确写 `json`。 |
| `path` | 字符串数组 | `set_field` 的目标字段路径，不能为空；`replace_file` 不使用。 |
| `content_from` | 凭证来源对象 | `replace_file` 必填，表示整文件内容来自哪个逻辑凭证文件。 |
| `value` | 任意固定 JSON 值 | `set_field` 时与 `value_from` 二选一。 |
| `value_from` | 凭证来源对象 | `set_field` 时与 `value` 二选一，表示从凭证中取值。 |

凭证来源对象包含：

| Key | 类型与可用值 | 含义与注意事项 |
| --- | --- | --- |
| `file_id` | 非空字符串 | CCG 凭证数据中的逻辑文件 ID。相同 ID 表示同一份导入内容。 |
| `path` | 可选字符串数组 | 从逻辑文件中取值的路径。省略时取整个文件；非 JSON 来源不能使用该 key。 |

同一个 `file_id` 被多个 operation 使用时，`format` 必须一致。对于会自动刷新 Token 的 Agent，CCG 写入的凭证可能随后被 Agent 覆盖，这是正常行为。

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
| `directory` | 非空路径字符串 | `enabled: true` 时必填，表示 Skill 根目录。相对路径从 `config_dir` 解析。 |

### `mcp`

| Key | 类型与可用值 | 含义与注意事项 |
| --- | --- | --- |
| `enabled` | `true` / `false` | 是否支持 MCP 配置管理。 |
| `file` | 非空路径字符串 | `enabled: true` 时必填，表示 MCP 所在配置文件。 |
| `format` | `json` 或 `toml` | MCP 配置文件格式。 |
| `servers_path` | 至少包含一个非空字符串的数组 | MCP 服务集合在配置文件中的字段路径，例如 `["mcpServers"]` 或 `["mcp", "servers"]`。 |
| `adapter` | 可选；当前只能是 `opencode` | 省略时按 CCG 的标准 MCP 结构直接写入；`opencode` 会先转换为 OpenCode 的结构，并且只支持 `json` 格式。 |

### `sessions`

| Key | 类型与可用值 | 含义与注意事项 |
| --- | --- | --- |
| `enabled` | `true` / `false` | 是否支持读取和管理会话。 |
| `adapter` | 非空字符串 | `enabled: true` 时必填。当前可用值为 `claude_code`、`codex`、`gemini`、`opencode`、`kimi_code`、`zcode`。 |

Session adapter 是已经编译进 CCG 的解析实现。用户模板不能只靠填写新的字符串来增加一种会话格式。

### `plugins`

| Key | 类型与可用值 | 含义与注意事项 |
| --- | --- | --- |
| `enabled` | `true` / `false` | 是否支持插件管理。 |
| `adapter` | 非空字符串 | `enabled: true` 时必填。当前实际实现为 `claude_code`。 |

Plugin adapter 是已经编译进 CCG 的插件生命周期实现，不能由模板新增。

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

### Gemini CLI：一组配置写入多个格式的文件

```json
"provider_config": {
  "enabled": true,
  "operations": [
    {
      "id": "set-token",
      "op": "set",
      "file": ".env",
      "format": "env",
      "path": ["GEMINI_API_KEY"],
      "value": "{target.token}"
    },
    {
      "id": "set-endpoint",
      "op": "set",
      "file": ".env",
      "format": "env",
      "path": ["GOOGLE_GEMINI_BASE_URL"],
      "value": "{target.endpoint}"
    },
    {
      "id": "set-auth-type",
      "op": "set",
      "file": "settings.json",
      "format": "json",
      "path": ["security", "auth", "selectedType"],
      "value": "gemini-api-key"
    }
  ]
}
```

一组 operations 可以同时修改 `.env` 和 JSON。限制是同一个目标文件不能混用格式，而不是整个数组只能使用一种格式。`env` 的 `path` 必须只有一个元素。

Gemini CLI 的官方登录同时使用整文件替换和单字段设置：

```json
"official_login": {
  "enabled": true,
  "operations": [
    {
      "id": "replace-gemini-oauth",
      "op": "replace_file",
      "file": "oauth_creds.json",
      "content_from": { "file_id": "gemini_oauth" }
    },
    {
      "id": "replace-gemini-accounts",
      "op": "replace_file",
      "file": "google_accounts.json",
      "content_from": { "file_id": "gemini_accounts" }
    },
    {
      "id": "set-auth-type",
      "op": "set_field",
      "file": "settings.json",
      "format": "json",
      "path": ["security", "auth", "selectedType"],
      "value": "oauth-personal"
    }
  ]
}
```

凭证文件适合 `replace_file`；同时包含用户设置的 `settings.json` 只修改认证类型，因此使用 `set_field`。

### OpenCode：一个 Agent 支持多协议及 MCP 结构适配

```json
"protocols": ["openai_responses", "anthropic_messages"]
```

```json
{
  "id": "set-anthropic-endpoint",
  "op": "set",
  "file": "opencode.json",
  "format": "json",
  "path": ["provider", "anthropic", "options", "baseURL"],
  "value": "{target.endpoint}/v1"
}
```

```json
"mcp": {
  "enabled": true,
  "file": "opencode.json",
  "format": "json",
  "adapter": "opencode",
  "servers_path": ["mcp"]
}
```

`protocols` 可以包含多个值。`{target.endpoint}` 可以作为字符串的一部分使用，因此 Anthropic 地址会额外带上 `/v1`。OpenCode 的 MCP 结构与标准结构不同，所以声明 `adapter: "opencode"`；该 adapter 只能配合 `json`。

### ZCode：深层路径、布尔值、数字和嵌套 `servers_path`

```json
{
  "id": "set-openai-key-required",
  "op": "set",
  "file": "v2/config.json",
  "format": "json",
  "path": ["provider", "ccg-openai", "options", "apiKeyRequired"],
  "value": true
}
```

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

## 5. 使用前检查

1. 文件名与 `id` 完全一致，`id` 没有大写字母。
2. `user_agent` 和 `protocols` 来自 Agent 的真实请求，不是根据名称猜测。
3. `features` 下 11 个功能 key 全部存在，开启的功能包含其必填 key。
4. 每个相对路径都从最终生效的 `config_dir` 展开检查过。
5. 同一文件只使用一种 `format`，`env` operation 的 `path` 只有一个元素。
6. 地址和密钥使用 `{target.endpoint}`、`{target.token}`，模板中不包含真实密钥。
7. Profile 占位符只出现在允许的位置，不会覆盖默认配置或其他文件。
8. 修改模板后重启软件，并在 Agent 页面确认没有“定义加载错误”。
