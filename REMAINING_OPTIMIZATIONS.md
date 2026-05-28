# 剩余优化项

> 基于 OPTIMIZATION_ACTION_PLAN.md 中 P1-5 和 P2-10 两项的未覆盖部分。
> 已完成的改动中建立了以下模式，新改动需保持一致。

## 已建立的模式（必须复用）

### 异步 IO 模式

1. **配置文件读写**（已改为 `tokio::fs`）：`commands.rs` 中的 `remove_file_if_exists`、`write_claude_gateway_settings`、`check_*_uses_gateway`、`remove_*_config_content` 等函数已全部使用 `tokio::fs`。
2. **密集文件遍历**（使用 `spawn_blocking`）：`session_commands.rs` 和 `skill_commands.rs` 各定义了 `run_session_blocking` / `run_skill_blocking` 包装器，将同步目录遍历放入 blocking 线程池。
3. **存在性检查**：统一使用 `tokio::fs::try_exists(&path).await.unwrap_or(false)` 替代 `path.exists()`。

### CLI 类型公共逻辑模式

`commands/cli_helpers.rs` 已抽取：
- `claude_settings_filename(profile) -> &'static str`
- `mcp_config_path(config_dir, cli_type) -> Option<PathBuf>`
- `prompt_file_path(config_dir, cli_type) -> Option<PathBuf>`

新增的 helper 应放在同一文件中，签名风格保持一致（接收 `&Path` + `&str`，返回具体路径）。

---

## P1-5：同步 IO 改异步（未覆盖部分）

### 1. `session_commands.rs` — 大量 `std::fs` 直接调用

该文件已有 `run_session_blocking` 模式，但很多函数内部仍直接使用 `std::fs`。需要将这些调用统一包进 `run_session_blocking` 或改为 `tokio::fs`。

**涉及的函数/场景**：

| 函数 | 当前用法 | 建议改法 |
|------|---------|---------|
| `parse_claude_session_info` | `std::fs::File::open` + BufReader 逐行解析 | 整个函数是 CPU + IO 混合，用 `run_session_blocking` 包裹整体调用 |
| `extract_codex_cwd` | `std::fs::File::open` + BufReader | 同上，包裹调用点 |
| `get_codex_projects` | `std::fs::read_dir`、`walkdir` 遍历 | 已是同步函数，确保调用方用 `run_session_blocking` 包裹 |
| `get_gemini_projects` | `std::fs::read_dir` 嵌套遍历 | 同上 |
| `get_codex_sessions_async` | 内部 `walkdir` + `std::fs::File::open` 提取首条消息 | 将整个文件遍历+解析逻辑放入 `run_session_blocking` |
| `get_gemini_sessions_async` | `std::fs::read_dir` + `std::fs::read_to_string` | 同上 |
| `get_codex_messages_async` | `walkdir` 查找 + `std::fs::File::open` 逐行解析 | 同上 |
| `get_session_projects` | `std::fs::read_dir` 遍历项目目录 | 同上 |
| `get_project_sessions` | `std::fs::read_dir` 遍历会话文件 | 同上 |
| `get_session_messages` (claude) | `std::fs::read_to_string` 读整个文件 | 用 `tokio::fs::read_to_string` 或 `run_session_blocking` |
| `delete_session` | `std::fs::remove_file` | 改为 `tokio::fs::remove_file` |
| `delete_project` | `std::fs::remove_dir_all` / `std::fs::remove_file` | 改为 `tokio::fs::remove_dir_all` / `tokio::fs::remove_file` |

**策略**：对于只做单次文件操作的（delete_session、delete_project），直接改 `tokio::fs`。对于涉及 walkdir 遍历 + 大量小文件解析的（projects/sessions 列表），将整块逻辑包进 `run_session_blocking`。

### 2. `skill_commands.rs` — `reinstall_skill_impl` 中的直接调用

`skill_commands.rs` 大部分已用 `run_skill_blocking` 包裹，但 `reinstall_skill_impl` 函数中有一段 `std::fs::rename` + `std::fs::remove_dir_all` 的原子替换逻辑（约 850-880 行）仍是裸调用：

```rust
std::fs::rename(&skill_path, &backup_path)
std::fs::rename(&temp_path, &skill_path)
std::fs::remove_dir_all(&temp_path)
std::fs::remove_dir_all(&backup_path)
```

**建议**：将这段原子替换逻辑整体包入一个 `run_skill_blocking` 闭包。注意 rename 操作需要原子性，不能拆成多个 async 调用中间穿插 await。

### 3. `skill_commands.rs` — `scan_cached_repo_skills`

```rust
let content = std::fs::read_to_string(file_path).map_err(|e| e.to_string())?;
```

该函数在 `run_skill_blocking` 内被调用（通过 `scan_cached_repo_skills_async`），所以实际上已经在 blocking 线程中。**无需改动**，仅需确认调用链正确。

---

## P2-10：CLI 类型公共逻辑抽取（未覆盖部分）

### 1. 会话相关路径 — `session_commands.rs`

以下 `match cli_type` 分支重复出现多次：

```rust
// 项目目录
match cli_type.as_str() {
    "codex" => base_dir.join("sessions"),
    "gemini" => base_dir.join("tmp"),
    _ => base_dir.join("projects"),
}

// 会话文件路径
match cli_type.as_str() {
    "gemini" => base_dir.join("tmp").join(project).join("chats").join(format!("{}.json", id)),
    _ => base_dir.join("projects").join(project).join(format!("{}.jsonl", id)),
}

// 项目删除路径
match cli_type.as_str() {
    "gemini" => base_dir.join("tmp").join(project),
    _ => base_dir.join("projects").join(project),
}
```

**建议**：在 `cli_helpers.rs` 中新增：

```rust
pub fn projects_dir(base_dir: &Path, cli_type: &str) -> PathBuf {
    match cli_type {
        "codex" => base_dir.join("sessions"),
        "gemini" => base_dir.join("tmp"),
        _ => base_dir.join("projects"),
    }
}

pub fn session_file_path(base_dir: &Path, cli_type: &str, project: &str, session_id: &str) -> PathBuf {
    match cli_type {
        "gemini" => base_dir.join("tmp").join(project).join("chats").join(format!("{}.json", session_id)),
        _ => base_dir.join("projects").join(project).join(format!("{}.jsonl", session_id)),
    }
}

pub fn project_dir(base_dir: &Path, cli_type: &str, project: &str) -> PathBuf {
    match cli_type {
        "gemini" => base_dir.join("tmp").join(project),
        _ => base_dir.join("projects").join(project),
    }
}
```

### 2. 配置格式验证 — `settings_commands.rs`

```rust
match cli_type.as_str() {
    "claude_code" | "gemini" => {
        serde_json::from_str::<serde_json::Value>(config_trimmed)
            .map_err(|e| format!("JSON 格式错误: {}", e))?;
    }
    "codex" => {
        config_trimmed.parse::<toml_edit::DocumentMut>()
            .map_err(|e| format!("TOML 格式错误: {}", e))?;
    }
    _ => {}
}
```

**建议**：在 `cli_helpers.rs` 中新增：

```rust
pub fn validate_config_format(cli_type: &str, content: &str) -> Result<(), String> {
    match cli_type {
        "claude_code" | "gemini" => {
            serde_json::from_str::<serde_json::Value>(content)
                .map_err(|e| format!("JSON 格式错误: {}", e))?;
        }
        "codex" => {
            content.parse::<toml_edit::DocumentMut>()
                .map_err(|e| format!("TOML 格式错误: {}", e))?;
        }
        _ => {}
    }
    Ok(())
}
```

### 3. `credential_commands.rs` — `parse_display_info` 和 `read_cli_credential_impl_async`

这两个函数内部有大段 `match cli_type` 分支处理不同 CLI 类型的文件结构。但这些逻辑比较复杂（涉及多个文件路径、JSON 解析策略），短期内抽取收益不大。

**建议**：暂不抽取。等后续新增第四种 CLI 类型时再统一重构。

### 4. `commands.rs` — `check_cli_enabled` / `sync_cli_config`

```rust
async fn check_cli_enabled(db: &SqlitePool, cli_type: &str, gateway_url: &str) -> bool {
    match cli_type {
        "claude_code" => check_claude_uses_gateway(db, cli_type, gateway_url).await,
        "codex" => check_codex_uses_gateway(db, cli_type, gateway_url).await,
        "gemini" => check_gemini_uses_gateway(db, cli_type, gateway_url).await,
        _ => false,
    }
}
```

这是 dispatch 模式，每个分支调用不同的实现函数。**已经是最简形式**，无需进一步抽取。

---

## 优先级建议

| 优先级 | 改动 | 理由 |
|--------|------|------|
| 高 | session_commands.rs 的遍历函数包入 spawn_blocking | 会话列表是高频操作，目录遍历阻塞 async runtime 影响 UI 响应 |
| 高 | delete_session / delete_project 改 tokio::fs | 简单直接，一行改完 |
| 中 | skill_commands.rs reinstall 原子替换包入 spawn_blocking | 涉及多次 rename，虽然不高频但有阻塞风险 |
| 中 | session 路径 helper 抽取到 cli_helpers.rs | 4 处重复，改动面小且降低出错概率 |
| 低 | validate_config_format 抽取 | 只有 1 处使用，收益有限 |
| 暂缓 | credential_commands 的 match cli_type | 逻辑复杂，抽取反而降低可读性 |
