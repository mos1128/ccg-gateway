//! 词条域：命令层（mcp / skill / session / backup / prompt / system / update / agent / scheduled_task / stats 命令）

use crate::i18n::Lang;

pub fn text(lang: Lang, key: &str) -> Option<&'static str> {
    let text = match key {
        // mcp_commands.rs
        "err.mcp.agent_unknown" => match lang {
            Lang::Zh => "未知 Agent: {}",
            Lang::En => "Unknown Agent: {}",
        },
        "err.mcp.agent_unsupported" => match lang {
            Lang::Zh => "Agent {} 的 MCP 功能不可用",
            Lang::En => "MCP is not available for Agent {}",
        },
        "err.mcp.missing_file" => match lang {
            Lang::Zh => "Agent {} 的 MCP 缺少 file",
            Lang::En => "MCP is missing file for Agent {}",
        },
        "err.mcp.missing_format" => match lang {
            Lang::Zh => "Agent {} 的 MCP 缺少 format",
            Lang::En => "MCP is missing format for Agent {}",
        },
        "err.mcp.dsh_yaml_only" => match lang {
            Lang::Zh => "Agent {} 的 DSH MCP 只支持 yaml",
            Lang::En => "DSH MCP for Agent {} only supports yaml",
        },
        "err.mcp.format_json_toml_only" => match lang {
            Lang::Zh => "Agent {} 的 MCP format 只支持 json 或 toml",
            Lang::En => "MCP format for Agent {} only supports json or toml",
        },
        "err.mcp.missing_servers_path" => match lang {
            Lang::Zh => "Agent {} 的 MCP 缺少 servers_path",
            Lang::En => "MCP is missing servers_path for Agent {}",
        },
        "err.mcp.json_invalid" => match lang {
            Lang::Zh => "MCP JSON 格式错误: {}",
            Lang::En => "Invalid MCP JSON: {}",
        },
        "err.mcp.opencode_object_required" => match lang {
            Lang::Zh => "OpenCode MCP 配置必须是 JSON object",
            Lang::En => "OpenCode MCP config must be a JSON object",
        },
        "err.mcp.type_string_required" => match lang {
            Lang::Zh => "MCP type 必须是字符串",
            Lang::En => "MCP type must be a string",
        },
        "err.mcp.service_type_required" => match lang {
            Lang::Zh => "MCP 配置必须通过 command 或 url 明确服务类型",
            Lang::En => "MCP config must declare the service type via command or url",
        },
        "err.mcp.command_required" => match lang {
            Lang::Zh => "stdio MCP 必须提供非空 command",
            Lang::En => "stdio MCP requires a non-empty command",
        },
        "err.mcp.args_string_array_required" => match lang {
            Lang::Zh => "MCP args 必须是字符串数组",
            Lang::En => "MCP args must be an array of strings",
        },
        "err.mcp.env_object_required" => match lang {
            Lang::Zh => "MCP env 必须是 JSON object",
            Lang::En => "MCP env must be a JSON object",
        },
        "err.mcp.url_required" => match lang {
            Lang::Zh => "远程 MCP 必须提供非空 url",
            Lang::En => "Remote MCP requires a non-empty url",
        },
        "err.mcp.headers_object_required" => match lang {
            Lang::Zh => "MCP headers 必须是 JSON object",
            Lang::En => "MCP headers must be a JSON object",
        },
        "err.mcp.type_unsupported" => match lang {
            Lang::Zh => "OpenCode MCP 不支持 type: {}",
            Lang::En => "OpenCode MCP does not support type: {}",
        },
        "err.mcp.path_not_object" => match lang {
            Lang::Zh => "MCP 路径 `{}` 不是对象",
            Lang::En => "MCP path `{}` is not an object",
        },
        "err.mcp.path_not_toml_table" => match lang {
            Lang::Zh => "MCP 路径 `{}` 不是 TOML table",
            Lang::En => "MCP path `{}` is not a TOML table",
        },
        "err.mcp.json_format_invalid" => match lang {
            Lang::Zh => "JSON 格式错误: {}",
            Lang::En => "Invalid JSON: {}",
        },
        "err.mcp.not_found" => match lang {
            Lang::Zh => "MCP 不存在",
            Lang::En => "MCP not found",
        },
        "err.mcp.format_adapter_mismatch" => match lang {
            Lang::Zh => "MCP format 与 adapter 不匹配",
            Lang::En => "MCP format does not match the adapter",
        },
        "err.mcp.parse_failed_not_written" => match lang {
            Lang::Zh => "{} 解析失败，未写入: {}",
            Lang::En => "Failed to parse {}; file not written: {}",
        },
        // skill_commands.rs
        "err.skill.file_task_failed" => match lang {
            Lang::Zh => "Skill 文件任务失败: {}",
            Lang::En => "Skill file task failed: {}",
        },
        "err.skill.agent_unsupported" => match lang {
            Lang::Zh => "Agent {} 的 Skill 功能未启用",
            Lang::En => "Skills are not available for Agent {}",
        },
        "err.skill.directory_not_found" => match lang {
            Lang::Zh => "Skill 目录不存在: {}",
            Lang::En => "Skill directory not found: {}",
        },
        "err.skill.not_found" => match lang {
            Lang::Zh => "Skill 不存在",
            Lang::En => "Skill not found",
        },
        "err.skill.git_spawn_failed" => match lang {
            Lang::Zh => "git clone 执行失败: {}",
            Lang::En => "Failed to run git clone: {}",
        },
        "err.skill.git_clone_failed" => match lang {
            Lang::Zh => "git clone 失败: {}",
            Lang::En => "git clone failed: {}",
        },
        "err.skill.repo_not_found" => match lang {
            Lang::Zh => "未找到仓库 '{}'",
            Lang::En => "Repository '{}' not found",
        },
        "err.skill.local_dir_missing" => match lang {
            Lang::Zh => "本地目录 {} 不存在",
            Lang::En => "Local directory {} does not exist",
        },
        "err.skill.install_dir_taken" => match lang {
            Lang::Zh => "Skill 安装目录 '{}' 已被其他来源占用，请先卸载现有 Skill",
            Lang::En => "Skill install directory '{}' is already used by another source; uninstall the existing Skill first",
        },
        "err.skill.already_installed" => match lang {
            Lang::Zh => "Skill '{}' 已安装",
            Lang::En => "Skill '{}' is already installed",
        },
        "err.skill.source_dir_missing" => match lang {
            Lang::Zh => "技能目录不存在: {}",
            Lang::En => "Skill directory does not exist: {}",
        },
        "err.skill.not_in_manifest" => match lang {
            Lang::Zh => "未在 manifest 中找到 Skill '{}'",
            Lang::En => "Skill '{}' not found in manifest",
        },
        "err.skill.reinstall_repo_info_missing" => match lang {
            Lang::Zh => "Skill 缺少仓库信息，无法重装",
            Lang::En => "Skill is missing repository info and cannot be reinstalled",
        },
        "err.skill.backup_failed" => match lang {
            Lang::Zh => "Skill 备份失败: {}",
            Lang::En => "Failed to back up Skill: {}",
        },
        "err.skill.replace_restore_failed" => match lang {
            Lang::Zh => "Skill 替换失败: {}; 旧版本恢复失败: {}",
            Lang::En => "Failed to replace Skill: {}; failed to restore the previous version: {}",
        },
        "err.skill.replace_failed" => match lang {
            Lang::Zh => "Skill 替换失败: {}",
            Lang::En => "Failed to replace Skill: {}",
        },
        "err.skill.source_directory_missing" => match lang {
            Lang::Zh => "Skill 缺少 source_directory 信息",
            Lang::En => "Skill is missing source_directory info",
        },
        "err.skill.repo_info_missing" => match lang {
            Lang::Zh => "Skill 缺少仓库信息",
            Lang::En => "Skill is missing repository info",
        },
        "err.skill.favorite_not_found" => match lang {
            Lang::Zh => "Skill 收藏不存在",
            Lang::En => "Skill favorite not found",
        },
        "err.skill.not_in_repo" => match lang {
            Lang::Zh => "未在仓库中找到技能: {}",
            Lang::En => "Skill not found in repository: {}",
        },
        "err.skill.not_installed_reinstall" => match lang {
            Lang::Zh => "Skill '{}' 未安装，无法重装",
            Lang::En => "Skill '{}' is not installed and cannot be reinstalled",
        },
        // session_commands.rs
        "err.session.file_task_failed" => match lang {
            Lang::Zh => "Session 文件任务失败: {}",
            Lang::En => "Session file task failed: {}",
        },
        "err.session.file_not_found" => match lang {
            Lang::Zh => "会话文件不存在: {}",
            Lang::En => "Session file not found: {}",
        },
        "err.session.file_not_found_plain" => match lang {
            Lang::Zh => "会话文件不存在",
            Lang::En => "Session file not found",
        },
        "err.session.file_open_failed" => match lang {
            Lang::Zh => "打开会话文件失败: {}",
            Lang::En => "Failed to open session file: {}",
        },
        "err.session.file_read_failed" => match lang {
            Lang::Zh => "读取会话文件失败: {}",
            Lang::En => "Failed to read session file: {}",
        },
        "err.session.json_parse_failed" => match lang {
            Lang::Zh => "解析会话 JSON 失败: {}",
            Lang::En => "Failed to parse session JSON: {}",
        },
        "err.session.home_dir_missing" => match lang {
            Lang::Zh => "无法确定 OpenCode 会话的 home 目录",
            Lang::En => "Failed to determine the home directory for OpenCode sessions",
        },
        "err.session.opencode_db_not_found" => match lang {
            Lang::Zh => "未找到 OpenCode 会话数据库: {}",
            Lang::En => "OpenCode session database not found: {}",
        },
        "err.session.opencode_db_open_failed" => match lang {
            Lang::Zh => "打开 OpenCode 会话数据库失败: {}",
            Lang::En => "Failed to open OpenCode session database: {}",
        },
        "err.session.opencode_count_projects_failed" => match lang {
            Lang::Zh => "统计 OpenCode 项目数失败: {}",
            Lang::En => "Failed to count OpenCode projects: {}",
        },
        "err.session.opencode_query_projects_failed" => match lang {
            Lang::Zh => "查询 OpenCode 项目失败: {}",
            Lang::En => "Failed to query OpenCode projects: {}",
        },
        "err.session.opencode_query_preview_failed" => match lang {
            Lang::Zh => "查询 OpenCode 会话预览失败: {}",
            Lang::En => "Failed to query OpenCode session preview: {}",
        },
        "err.session.opencode_count_sessions_failed" => match lang {
            Lang::Zh => "统计 OpenCode 会话数失败: {}",
            Lang::En => "Failed to count OpenCode sessions: {}",
        },
        "err.session.opencode_query_sessions_failed" => match lang {
            Lang::Zh => "查询 OpenCode 会话失败: {}",
            Lang::En => "Failed to query OpenCode sessions: {}",
        },
        "err.session.opencode_query_messages_failed" => match lang {
            Lang::Zh => "查询 OpenCode 消息失败: {}",
            Lang::En => "Failed to query OpenCode messages: {}",
        },
        "err.session.opencode_query_parts_failed" => match lang {
            Lang::Zh => "查询 OpenCode 消息分片失败: {}",
            Lang::En => "Failed to query OpenCode message parts: {}",
        },
        "err.session.agent_unknown" => match lang {
            Lang::Zh => "未知 Agent: {}",
            Lang::En => "Unknown Agent: {}",
        },
        "err.session.agent_unsupported" => match lang {
            Lang::Zh => "Agent {} 的 Session 功能未启用",
            Lang::En => "Sessions are not available for Agent {}",
        },
        "err.session.missing_adapter" => match lang {
            Lang::Zh => "Agent {} 的 Session 缺少 adapter",
            Lang::En => "Sessions are missing adapter for Agent {}",
        },
        "err.session.adapter_unsupported" => match lang {
            Lang::Zh => "Session adapter `{}` 不存在",
            Lang::En => "Session adapter `{}` does not exist",
        },
        "err.session.directory_not_found" => match lang {
            Lang::Zh => "会话目录不存在: {}",
            Lang::En => "Session directory not found: {}",
        },
        "err.session.zcode_db_not_found" => match lang {
            Lang::Zh => "未找到 zcode 会话数据库: {}",
            Lang::En => "zcode session database not found: {}",
        },
        "err.session.zcode_db_open_failed" => match lang {
            Lang::Zh => "打开 zcode 会话数据库失败: {}",
            Lang::En => "Failed to open zcode session database: {}",
        },
        "err.session.zcode_query_tasks_failed" => match lang {
            Lang::Zh => "查询 zcode 任务失败: {}",
            Lang::En => "Failed to query zcode tasks: {}",
        },
        "err.session.zcode_query_sessions_failed" => match lang {
            Lang::Zh => "查询 zcode 会话失败: {}",
            Lang::En => "Failed to query zcode sessions: {}",
        },
        "err.session.rollout_read_failed" => match lang {
            Lang::Zh => "读取会话 '{}' 的 rollout 失败: {}",
            Lang::En => "Failed to read rollout for session '{}': {}",
        },
        "err.session.wire_read_failed" => match lang {
            Lang::Zh => "读取会话 '{}' 的 wire.jsonl 失败: {}",
            Lang::En => "Failed to read wire.jsonl for session '{}': {}",
        },
        "err.session.chat_history_read_failed" => match lang {
            Lang::Zh => "读取会话 '{}' 的 chat_history.jsonl 失败: {}",
            Lang::En => "Failed to read chat_history.jsonl for session '{}': {}",
        },
        "err.session.read_failed" => match lang {
            Lang::Zh => "读取会话 '{}' 失败: {}",
            Lang::En => "Failed to read session '{}': {}",
        },
        "err.session.delete_failed" => match lang {
            Lang::Zh => "删除会话失败: {}",
            Lang::En => "Failed to delete session: {}",
        },
        "err.session.delete_named_failed" => match lang {
            Lang::Zh => "删除会话 '{}' 失败: {}",
            Lang::En => "Failed to delete session '{}': {}",
        },
        "err.session.delete_project_failed" => match lang {
            Lang::Zh => "删除项目失败: {}",
            Lang::En => "Failed to delete project: {}",
        },
        "err.session.opencode_readonly_session" => match lang {
            Lang::Zh => "OpenCode 会话管理为只读，不支持删除会话",
            Lang::En => "OpenCode session management is read-only; deleting sessions is not supported",
        },
        "err.session.zcode_readonly_session" => match lang {
            Lang::Zh => "ZCode 会话管理为只读，不支持删除会话",
            Lang::En => "ZCode session management is read-only; deleting sessions is not supported",
        },
        "err.session.opencode_readonly_project" => match lang {
            Lang::Zh => "OpenCode 会话管理为只读，不支持删除项目",
            Lang::En => "OpenCode session management is read-only; deleting projects is not supported",
        },
        "err.session.zcode_readonly_project" => match lang {
            Lang::Zh => "ZCode 会话管理为只读，不支持删除项目",
            Lang::En => "ZCode session management is read-only; deleting projects is not supported",
        },
        // backup_commands.rs
        "err.backup.connection_failed" => match lang {
            Lang::Zh => "连接失败: {}",
            Lang::En => "Connection failed: {}",
        },
        "err.backup.read_db_failed" => match lang {
            Lang::Zh => "读取数据库失败: {}",
            Lang::En => "Failed to read database: {}",
        },
        "err.backup.write_file_failed" => match lang {
            Lang::Zh => "写入文件失败: {}",
            Lang::En => "Failed to write file: {}",
        },
        "err.backup.write_db_failed" => match lang {
            Lang::Zh => "写入数据库失败: {}",
            Lang::En => "Failed to write database: {}",
        },
        "err.backup.webdav_url_missing" => match lang {
            Lang::Zh => "未配置 WebDAV 地址",
            Lang::En => "WebDAV URL not configured",
        },
        "err.backup.upload_failed" => match lang {
            Lang::Zh => "上传失败: {}",
            Lang::En => "Upload failed: {}",
        },
        "err.backup.upload_status_failed" => match lang {
            Lang::Zh => "上传失败，状态码: {}",
            Lang::En => "Upload failed with status: {}",
        },
        "err.backup.list_failed" => match lang {
            Lang::Zh => "获取备份列表失败: {}",
            Lang::En => "Failed to list backups: {}",
        },
        "err.backup.xml_parse_failed" => match lang {
            Lang::Zh => "XML 解析错误，位置 {}: {}",
            Lang::En => "XML parse error at position {}: {}",
        },
        "err.backup.download_failed" => match lang {
            Lang::Zh => "下载失败: {}",
            Lang::En => "Download failed: {}",
        },
        "err.backup.download_status_failed" => match lang {
            Lang::Zh => "下载失败，状态码: {}",
            Lang::En => "Download failed with status: {}",
        },
        "err.backup.delete_failed" => match lang {
            Lang::Zh => "删除失败: {}",
            Lang::En => "Delete failed: {}",
        },
        "err.backup.delete_status_failed" => match lang {
            Lang::Zh => "删除失败，状态码: {}",
            Lang::En => "Delete failed with status: {}",
        },
        // prompt_commands.rs
        "err.prompt.agent_unknown" => match lang {
            Lang::Zh => "未知 Agent: {}",
            Lang::En => "Unknown Agent: {}",
        },
        "err.prompt.agent_unsupported" => match lang {
            Lang::Zh => "Agent {} 的 Prompt 功能不可用",
            Lang::En => "Prompts are not available for Agent {}",
        },
        "err.prompt.missing_file" => match lang {
            Lang::Zh => "Agent {} 的 Prompt 缺少 file",
            Lang::En => "Prompts are missing file for Agent {}",
        },
        "err.prompt.not_found" => match lang {
            Lang::Zh => "Prompt 不存在",
            Lang::En => "Prompt not found",
        },
        // update_commands.rs
        "err.update.network_failed" => match lang {
            Lang::Zh => "网络请求失败: {}",
            Lang::En => "Network request failed: {}",
        },
        "err.update.github_api_error" => match lang {
            Lang::Zh => "GitHub API 错误: {}",
            Lang::En => "GitHub API error: {}",
        },
        "err.update.response_parse_failed" => match lang {
            Lang::Zh => "解析响应失败: {}",
            Lang::En => "Failed to parse response: {}",
        },
        // agent_commands.rs
        "err.agent_cmd.json_format_invalid" => match lang {
            Lang::Zh => "JSON 格式错误: {}",
            Lang::En => "Invalid JSON: {}",
        },
        "err.agent_cmd.toml_format_invalid" => match lang {
            Lang::Zh => "TOML 格式错误: {}",
            Lang::En => "Invalid TOML: {}",
        },
        "err.agent_cmd.yaml_format_invalid" => match lang {
            Lang::Zh => "YAML 格式错误: {}",
            Lang::En => "Invalid YAML: {}",
        },
        _ => return None,
    };
    Some(text)
}
