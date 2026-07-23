# Agent 模板编写指南

Agent 模板用于描述客户端的配置目录、请求协议、配置文件和功能支持情况。内置模板位于源码的 `src-tauri/agent-definitions/{id}.json`，会编译进程序；用户模板放在 `<数据目录>/agent-definitions/{id}.json`，默认是 `~/.ccg-gateway/agent-definitions/{id}.json`。设置 `CCG_DATA_DIR` 后，数据目录以该环境变量为准。

软件启动时会合并两类模板：有效用户模板的 `id` 与内置模板相同时覆盖内置模板，使用新 `id` 时新增 Agent。用户目录不存在时只使用内置模板；用户模板读取或校验失败时会显示定义错误，并继续使用同 ID 的有效内置模板。模板变更需要重启软件后生效。

两类模板使用相同格式，只接受 `agent-definition.schema.json` 中声明的字段，文件名必须与 `id` 一致，且不能执行命令、脚本、网络请求或 OAuth 流程。本文统一使用“功能”“服务商”“配置档案”；JSON key 和固定取值保留代码原名。

## 1. 最小结构

除标注为可选的顶层字段外，其余顶层字段和 `features` 下的功能都必须出现。暂不支持的功能只写 `{ "enabled": false }`。

```json
{
  "schema_version": 1,
  "id": "example",
  "sort_order": 100,
  "name": "Example CLI",
  "remark": "启用前请先完成必要的客户端配置。",
  "icon": {
    "view_box": "0 0 24 24",
    "color": "#5865f2",
    "paths": [{ "d": "M4 4h16v16H4z" }]
  },
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

## 2. 顶层 key

| Key | 用途 | 可选值与注意点 |
| --- | --- | --- |
| `schema_version` | 模板版本 | 当前只能是 `1`。 |
| `id` | Agent 的稳定标识 | 只能包含小写字母、数字、`_`、`-`，并且必须与文件名一致。 |
| `sort_order` | Agent 展示和匹配顺序 | 大于等于 `0`；相同值再按 `id` 排序。 |
| `name` | 前端显示名称 | 非空字符串。 |
| `remark` | 前端提示信息 | 可选；非空字符串。用于说明启用前提或使用注意事项，不参与配置写入和请求处理。 |
| `icon` | 前端 Agent 图标 | 可选；使用 SVG `view_box`、可选六位十六进制 `color` 和一个或多个 `path` 描述，避免注入原始 SVG。 |
| `config_dir` | 默认配置目录 | 用户在数据库中设置的自定义目录优先。 |
| `user_agent` | 识别客户端请求 | 非空字符串数组；按不区分大小写的子串匹配。 |
| `protocols` | Agent 支持的请求协议 | `anthropic_messages`、`openai_chat`、`openai_responses`、`gemini_generate_content`；至少一个且不能重复。 |
| `features` | 功能配置集合 | 必须包含下文列出的全部功能。 |

`remark` 会显示在 Agent 列表和能力信息中。没有需要提示的内容时应省略该字段，不要填写空字符串或通用说明。

### 路径规则

`file` 和 `directory` 使用同一套规则：

- 绝对路径直接使用。
- `~`、`~/`、`~\` 相对用户主目录展开。
- 其他路径相对 `config_dir` 解析，允许使用 `..`。

## 3. 功能 key

所有功能都用 `enabled` 表示是否支持。设为 `true` 时，再提供该功能要求的其他 key。

### `provider_config` - 服务商配置

用于写入 CCG 路由或服务商直连配置。

| Key | 用途 | 可选值与注意点 |
| --- | --- | --- |
| `enabled` | 是否支持服务商配置写入 | `true` / `false`；同时控制 CCG 路由和服务商直连。 |
| `operations` | 配置文件写入规则 | `enabled: true` 时不能为空；路由和直连共用同一组规则。 |

`operations` 的写法见第 4 节。

这里的 operations 只描述默认配置，`file`、`path` 和固定值都应写明，不能使用 Profile 模板变量。是否开启 `profiles` 不影响默认配置写入。

运行时地址和密钥直接写成 `value` 占位符：

- `"value": "{target.endpoint}"`：路由模式写 Gateway 地址，直连模式写服务商地址。
- `"value": "{target.token}"`：路由模式写 Gateway Token，直连模式写服务商 API Key。

`profiles.operations` 使用相同占位符，但每个非默认配置档案会独立展开和执行。

### `global_preset` - 全局预设

用于给默认配置档案写入一份基础配置。

| Key | 用途 | 可选值与注意点 |
| --- | --- | --- |
| `enabled` | 是否支持全局预设 | `true` / `false`。 |
| `file` | 预设写入的文件 | 开启时必填。 |
| `format` | 文件格式 | `json` 或 `toml`。 |

字段保护：`global_preset` 与 `provider_config.operations` 写入同一目标时，预设中的同路径值会被忽略，避免用户错误配置导致核心值被覆盖。

### `profiles` - 配置档案

用于在默认配置之外扩展非默认配置档案。非默认配置有自己的文件和 operations，不复用 `provider_config.operations`。

| Key | 用途 | 可选值与注意点 |
| --- | --- | --- |
| `enabled` | 是否支持多个配置档案 | `true` / `false`。 |
| `profile_file` | 非默认配置档案的文件名模式 | 开启时必填，且必须包含 `{profile}` 变量。 |
| `operations` | 非默认配置档案的写入规则 | 开启时不能为空；写法与 `provider_config.operations` 相同，可以使用下表中的变量。 |
| `launch.default` | 默认配置的启动命令参数 | 字符串数组，可省略；只生成供用户复制的命令，不会执行。 |
| `launch.non_default` | 非默认配置档案的启动命令参数 | 字符串数组，可省略；只生成供用户复制的命令，不会执行。 |

可用变量：

| 变量 | 含义 |
| --- | --- |
| `{profile}` | 当前配置档案名称；可用于 `profile_file`、Profile operation 的 `file`、`path`、`value`，以及启动参数。 |
| `{profile.relative_path}` | `profile_file` 展开后的相对路径；只用于 Profile operation 的 `file`、`path`。 |
| `{profile.absolute_path}` | 相对有效 `config_dir` 解析后的绝对路径；只用于 `launch.non_default`。 |

每个非默认配置档案执行 operations 时都会使用自己的变量值。未知变量或错误位置会报错。

删除和重命名配置档案时，会先按旧上下文逆向处理 `profiles.operations` 的全部目标文件。`set` 只删除仍等于模板写入值的字段；`remove` 无法恢复原值。已有目标文件但无法匹配 Gateway 或服务商配置时，操作会直接报错。

### `official_login` - 官方直连

用于把已保存的官方凭证写入客户端，以支持官方直连。

| Key | 用途 | 可选值与注意点 |
| --- | --- | --- |
| `enabled` | 是否支持官方直连 | `true` / `false`。 |
| `operations` | 官方凭证写入规则 | 开启时不能为空，写法见第 5 节。 |

该功能只管理凭证文件和认证字段，不执行登录命令、OAuth 或 Token 刷新。登录流程由客户端自行实现，CCG 只做凭证托管：用户先在客户端完成登录，把生成的凭证文件导入 CCG，之后由 CCG 负责把该凭证写回客户端配置。对于会自动刷新 Token 的客户端，CCG 写入的凭证可能被客户端后续刷新覆盖，属于预期行为。

### `model_mapping` - 模型映射

| Key | 用途 | 可选值与注意点 |
| --- | --- | --- |
| `enabled` | 是否支持模型映射 | `true` / `false`。 |

### `token_usage` - Token 用量统计

| Key | 用途 | 可选值与注意点 |
| --- | --- | --- |
| `enabled` | 是否支持 Token 用量统计 | `true` / `false`。 |

### `skills` - 技能

| Key | 用途 | 可选值与注意点 |
| --- | --- | --- |
| `enabled` | 是否支持 Skills | `true` / `false`。 |
| `directory` | Skill 根目录 | 开启时必填；相对 `config_dir` 解析。 |

### `mcp` - MCP 服务

| Key | 用途 | 可选值与注意点 |
| --- | --- | --- |
| `enabled` | 是否支持 MCP | `true` / `false`。 |
| `file` | MCP 配置文件 | 开启时必填。 |
| `format` | 文件格式 | `json` 或 `toml`。 |
| `adapter` | MCP 结构适配器 | 可选；目前支持 `opencode`。省略时原样写入 MCP 配置。适配发生在文件序列化之前。 |
| `servers_path` | MCP 服务集合的字段路径 | 非空字符串数组。 |

### `sessions` - 会话

| Key | 用途 | 可选值与注意点 |
| --- | --- | --- |
| `enabled` | 是否支持会话读取 | `true` / `false`。 |
| `adapter` | 会话解析实现 | 开启时必填；必须是已经编译进 CCG 的实现。 |

### `plugins` - 插件

| Key | 用途 | 可选值与注意点 |
| --- | --- | --- |
| `enabled` | 是否支持插件管理 | `true` / `false`。 |
| `adapter` | 插件管理实现 | 开启时必填；必须是已经编译进 CCG 的实现。 |

### `prompts` - 提示词

| Key | 用途 | 可选值与注意点 |
| --- | --- | --- |
| `enabled` | 是否支持提示词文件管理 | `true` / `false`。 |
| `file` | 提示词文件 | 开启时必填；只支持单一文件。 |

只有 `sessions` 和 `plugins` 可以声明 `adapter`。新增协议、会话解析或插件生命周期仍需修改 Rust 代码。

## 4. 服务商配置 operations

一个 operation 只处理一个字段：

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

| Key | 用途 | 可选值与注意点 |
| --- | --- | --- |
| `id` | operation 标识 | 在当前 `operations` 中必须非空且唯一。 |
| `op` | 操作类型 | `set`：设置字段；`remove`：删除字段。 |
| `file` | 目标文件 | `provider_config.operations` 必须写明确路径；`profiles.operations` 可以使用 Profile 模板变量。 |
| `format` | 目标格式 | `json`、`jsonc`、`toml`、`env`。同一文件不能混用格式。 |
| `path` | 字段路径 | 非空字符串数组；`env` 只能有一个元素。 |
| `value` | 写入值 | `set` 时必填；可使用字符串、布尔值、数字、数组或对象，字符串中可使用下表占位符。 |

`remove` 不能设置 `value`，且停用时无法恢复被删除的旧值。密钥应使用 `{target.token}`，不要直接写入模板。

| 占位符 | 写入内容 | 可用位置 |
| --- | --- | --- |
| `{target.endpoint}` | 路由模式为 Gateway 地址，直连模式为服务商地址。 | operation 的 `value`。 |
| `{target.token}` | 路由模式为 Gateway Token，直连模式为服务商 API Key。 | operation 的 `value`。 |
| `{agent.id}` | 当前 Agent ID。 | operation 的 `value`。 |
| `{profile}` | 当前非默认配置档案名称。 | Profile operation 的 `file`、`path`、`value`。 |
| `{profile.relative_path}` | `profile_file` 展开后的相对路径。 | Profile operation 的 `file`、`path`。 |

`value` 的普通内容是固定值，`{...}` 是执行时替换的动态部分；两者属于同一个字段。数组、对象及其字符串成员也会递归替换。`provider_config.operations` 不能使用 `{profile...}`。

一个 `operations` 可以修改多个文件。执行时会先解析全部目标，再逐文件原子替换；停用、删除或重命名时也会逐文件逆向处理。不提供跨文件事务，失败后可重复执行同一配置。

## 5. 官方直连 operations

官方凭证操作支持两种 `op`：

- `replace_file`：用凭证中的完整内容替换目标文件；支持 `json`、`jsonc`、`toml`、`env`，省略 `format` 时兼容为 `json`。
- `set_field`：只设置目标 JSON 的一个字段，适合还包含用户设置的文件。

| Key | 用途 | 可选值与注意点 |
| --- | --- | --- |
| `id` | operation 标识 | 非空字符串。 |
| `op` | 操作类型 | `replace_file` 或 `set_field`。 |
| `file` | 目标凭证或设置文件 | 相对 `config_dir` 解析。 |
| `format` | 目标格式 | `replace_file` 支持 `json`、`jsonc`、`toml`、`env`；`set_field` 固定为 `json`。 |
| `path` | 目标字段路径 | `set_field` 必填。 |
| `content_from` | 整文件内容来源 | `replace_file` 必填，结构见下表。 |
| `value` | 固定字段值 | `set_field` 时与 `value_from` 二选一。 |
| `value_from` | 凭证中的字段值来源 | `set_field` 时与 `value` 二选一，结构见下表。 |

`content_from` 和这里的 `value_from` 都使用凭证来源对象。这里的 `value_from` 表示“从已导入凭证取值”，不是服务商 operation 的模板变量：

| Key | 用途 | 注意点 |
| --- | --- | --- |
| `file_id` | 凭证数据中的逻辑文件 ID | 非空字符串。 |
| `path` | 从该逻辑文件中取值的字段路径 | 可省略；省略时取整个 JSON。 |

示例：

```json
{
  "id": "replace-auth",
  "op": "replace_file",
  "file": "auth.json",
  "content_from": { "file_id": "agent_auth" }
}
```

## 6. 内置模板实战参考

| Agent | 官方直连 | 配置档案 | 配置与 MCP |
| --- | --- | --- | --- |
| Claude Code | 不支持 | 支持 | 服务商配置在 `~/.claude/settings*.json`，MCP 在 `~/.claude.json`。 |
| Codex | 支持 | 支持 | 默认服务商配置与 MCP 共用 `~/.codex/config.toml`。 |
| Gemini CLI | 支持 | 不支持 | 一组操作会同时修改 `.env` 和 `settings.json`。 |

### Claude Code：配置档案与 MCP 分文件

- `official_login.enabled` 为 `false`。
- 默认 operations 明确写入 `settings.json`；非默认 operations 写入 `settings-ccg-{profile}.json`。
- 两组 operations 都在文件的 `env` 中写入 `{target.endpoint}` 和 `{target.token}`；非默认档案的路由 Token 会按当前配置档案解析。
- 非默认配置档案通过 `claude --settings {profile.absolute_path}` 启动；`{profile.absolute_path}` 是该 JSON 文件的绝对路径。
- MCP 的 `file` 是 `../.claude.json`，从 `~/.claude` 解析后指向 `~/.claude.json`；`servers_path` 是 `mcpServers`。

完整配置见 [`claude_code.json`](../src-tauri/agent-definitions/claude_code.json)。

### Codex：配置档案、官方直连与 MCP 共用配置

- 默认 operations 明确写入 `config.toml` 和 `model_providers.ccg-gateway`；非默认 operations 写入 `{profile}.config.toml` 和 `model_providers.ccg-gateway-{profile}`。
- 非默认配置档案通过 `codex --profile {profile}` 启动。
- 官方直连把逻辑文件 `codex_auth` 完整写入 `auth.json`。
- MCP 与默认配置档案共用 `config.toml`，`servers_path` 是 `mcp_servers`；非默认配置档案仍使用各自的 `{profile}.config.toml`。

完整配置见 [`codex.json`](../src-tauri/agent-definitions/codex.json)。

### Gemini CLI：无配置档案，多文件操作

- `profiles.enabled` 为 `false`，始终使用默认配置。
- CCG 路由和服务商直连都在 `.env` 写入 `GEMINI_API_KEY`、`GOOGLE_GEMINI_BASE_URL`，同时在 `settings.json` 写入认证类型 `gemini-api-key`。
- 官方直连替换 `oauth_creds.json`、`google_accounts.json`，同时把 `settings.json` 的认证类型改为 `oauth-personal`。

这个模板说明一组 `operations` 可以包含不同文件和不同格式；每个文件内部的格式保持一致即可。

完整配置见 [`gemini.json`](../src-tauri/agent-definitions/gemini.json)。

## 7. 使用或提交前检查

1. 文件名与 `id` 一致，UA 和协议来自客户端真实请求。
2. 所有相对路径都从 `config_dir` 展开检查过。
3. 开启的功能提供了必填 key，关闭的功能没有遗留无效配置。
4. `provider_config.operations` 包含完整写入规则，地址和密钥使用 `{target.endpoint}`、`{target.token}`。
5. `profiles.profile_file` 和 `profiles.operations` 使用正确的 Profile 变量，不会覆盖默认配置或其他客户端文件。
6. 多文件操作已验证重复执行、切换模式和停用后的结果。

更完整的边界、生命周期和验证要求见 `docs/agent-integration-final-design.md`。
