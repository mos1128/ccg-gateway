//! 通用 / 基础设施词条：托盘菜单、启动配置校验、网关设置命令、数据库错误映射。

use crate::i18n::Lang;

pub fn text(lang: Lang, key: &str) -> Option<&'static str> {
    let text = match key {
        // 托盘菜单
        "tray.show" => match lang {
            Lang::Zh => "显示窗口",
            Lang::En => "Show Window",
        },
        "tray.quit" => match lang {
            Lang::Zh => "退出",
            Lang::En => "Quit",
        },

        // commands.rs 辅助函数
        "err.cmd.mcp_json_invalid" => match lang {
            Lang::Zh => "MCP JSON 格式错误: {}",
            Lang::En => "Invalid MCP JSON: {}",
        },
        "err.cmd.mcp_toml_object_required" => match lang {
            Lang::Zh => "TOML MCP 配置必须是 JSON object",
            Lang::En => "TOML MCP config must be a JSON object",
        },
        "err.cmd.toml_null_unsupported" => match lang {
            Lang::Zh => "Codex MCP 配置不能包含 null，TOML 不支持 null",
            Lang::En => "Codex MCP config cannot contain null; TOML does not support null",
        },

        // map_db_error
        "err.db.duplicate_provider_name" => match lang {
            Lang::Zh => "同类型同 Profile 的服务商名称已存在",
            Lang::En => "A provider with the same name already exists for this type and profile",
        },
        "err.db.duplicate_model_map" => match lang {
            Lang::Zh => "该服务商已存在相同的模型映射",
            Lang::En => "This provider already has the same model mapping",
        },
        "err.db.duplicate_blacklist" => match lang {
            Lang::Zh => "该服务商已存在相同的黑名单模式",
            Lang::En => "This provider already has the same blacklist pattern",
        },
        "err.db.duplicate_mcp_name" => match lang {
            Lang::Zh => "MCP 配置名称已存在",
            Lang::En => "An MCP config with this name already exists",
        },
        "err.db.duplicate_prompt_name" => match lang {
            Lang::Zh => "提示词预设名称已存在",
            Lang::En => "A prompt preset with this name already exists",
        },
        "err.db.duplicate_skill_dir" => match lang {
            Lang::Zh => "该目录已安装过 Skill",
            Lang::En => "A skill from this directory is already installed",
        },
        "err.db.duplicate_credential_name" => match lang {
            Lang::Zh => "同类型的凭证名称已存在",
            Lang::En => "A credential with the same name already exists for this type",
        },
        "err.db.duplicate_skill_favorite" => match lang {
            Lang::Zh => "该技能已收藏",
            Lang::En => "This skill is already in favorites",
        },
        "err.db.duplicate" => match lang {
            Lang::Zh => "数据已存在，请勿重复添加",
            Lang::En => "The record already exists",
        },

        // config.rs 校验
        "err.cfg.host_required" => match lang {
            Lang::Zh => "监听地址不能为空",
            Lang::En => "Listen address is required",
        },
        "err.cfg.host_invalid" => match lang {
            Lang::Zh => "监听地址必须是有效的 IP 地址或主机名",
            Lang::En => "Listen address must be a valid IP address or hostname",
        },
        "err.cfg.config_dir_create_failed" => match lang {
            Lang::Zh => "无法创建配置目录 {}：{}",
            Lang::En => "Failed to create config directory {}: {}",
        },
        "err.cfg.bootstrap_serialize_failed" => match lang {
            Lang::Zh => "无法序列化启动配置：{}",
            Lang::En => "Failed to serialize bootstrap config: {}",
        },
        "err.cfg.bootstrap_create_failed" => match lang {
            Lang::Zh => "无法创建启动配置 {}：{}",
            Lang::En => "Failed to create bootstrap config {}: {}",
        },
        "err.cfg.bootstrap_write_failed" => match lang {
            Lang::Zh => "无法写入启动配置 {}：{}",
            Lang::En => "Failed to write bootstrap config {}: {}",
        },
        "err.cfg.bootstrap_replace_failed" => match lang {
            Lang::Zh => "无法替换启动配置 {}：{}",
            Lang::En => "Failed to replace bootstrap config {}: {}",
        },
        "err.cfg.log_level_required" => match lang {
            Lang::Zh => "日志级别不能为空",
            Lang::En => "Log level is required",
        },
        "err.cfg.log_level_invalid" => match lang {
            Lang::Zh => "日志级别格式无效：{}",
            Lang::En => "Invalid log level format: {}",
        },
        "err.cfg.data_dir_required" => match lang {
            Lang::Zh => "数据目录不能为空",
            Lang::En => "Data directory is required",
        },
        "err.cfg.data_dir_absolute_required" => match lang {
            Lang::Zh => "数据目录必须使用绝对路径",
            Lang::En => "Data directory must be an absolute path",
        },
        "err.cfg.data_dir_create_failed" => match lang {
            Lang::Zh => "无法创建数据目录 {}：{}",
            Lang::En => "Failed to create data directory {}: {}",
        },
        "err.cfg.data_dir_not_writable" => match lang {
            Lang::Zh => "数据目录不可写 {}：{}",
            Lang::En => "Data directory is not writable {}: {}",
        },

        // settings_commands.rs
        "err.settings.port_invalid" => match lang {
            Lang::Zh => "监听端口必须是 1 到 65535 之间的整数",
            Lang::En => "Listen port must be an integer between 1 and 65535",
        },
        "err.settings.bind_failed" => match lang {
            Lang::Zh => "无法监听 {}：{}",
            Lang::En => "Failed to listen on {}: {}",
        },
        "err.settings.no_provider_in_default_profile" => match lang {
            Lang::Zh => "default Profile 下没有已启用的服务商，请先启用服务商",
            Lang::En => "No enabled provider in the default profile; enable a provider first",
        },
        "err.settings.preset_disabled" => match lang {
            Lang::Zh => "Agent {} 的全局预设功能不可用",
            Lang::En => "The global preset feature is not available for Agent {}",
        },
        "err.settings.config_dir_required" => match lang {
            Lang::Zh => "Agent 配置目录不能为空",
            Lang::En => "Agent config directory is required",
        },
        "err.settings.write_mode_invalid" => match lang {
            Lang::Zh => "config_write_mode 只能是 'overwrite' 或 'merge'",
            Lang::En => "config_write_mode must be 'overwrite' or 'merge'",
        },
        "err.settings.unsupported_language" => match lang {
            Lang::Zh => "不支持的界面语言：{}",
            Lang::En => "Unsupported language: {}",
        },

        // auto_launch.rs
        "err.auto_launch.exe_path_failed" => match lang {
            Lang::Zh => "无法获取应用路径: {}",
            Lang::En => "Failed to get the application path: {}",
        },
        "err.auto_launch.create_failed" => match lang {
            Lang::Zh => "创建 AutoLaunch 失败: {}",
            Lang::En => "Failed to create AutoLaunch: {}",
        },
        "err.auto_launch.enable_failed" => match lang {
            Lang::Zh => "启用开机自启失败: {}",
            Lang::En => "Failed to enable launch on startup: {}",
        },
        "err.auto_launch.disable_failed" => match lang {
            Lang::Zh => "禁用开机自启失败: {}",
            Lang::En => "Failed to disable launch on startup: {}",
        },

        // commands/cli_helpers.rs
        "err.cli_helpers.unknown_agent" => match lang {
            Lang::Zh => "未知 Agent: {}",
            Lang::En => "Unknown Agent: {}",
        },
        "err.cli_helpers.json_invalid" => match lang {
            Lang::Zh => "JSON 格式错误: {}",
            Lang::En => "Invalid JSON: {}",
        },
        "err.cli_helpers.toml_invalid" => match lang {
            Lang::Zh => "TOML 格式错误: {}",
            Lang::En => "Invalid TOML: {}",
        },
        "err.cli_helpers.yaml_invalid" => match lang {
            Lang::Zh => "YAML 格式错误: {}",
            Lang::En => "Invalid YAML: {}",
        },
        "err.cli_helpers.preset_format_unsupported" => match lang {
            Lang::Zh => "Agent {} 未声明可用的全局预设格式",
            Lang::En => "Agent {} does not declare a supported global preset format",
        },

        // commands/plugin_commands.rs
        "err.plugin.no_agent_enabled" => match lang {
            Lang::Zh => "没有 Agent 启用插件功能",
            Lang::En => "No Agent has the plugin feature enabled",
        },
        "err.plugin.multiple_agents" => match lang {
            Lang::Zh => "插件功能同时启用了多个 Agent: {}",
            Lang::En => "The plugin feature is enabled for multiple Agents: {}",
        },
        "err.plugin.unknown_agent" => match lang {
            Lang::Zh => "未知 Agent: {}",
            Lang::En => "Unknown Agent: {}",
        },
        "err.plugin.dsh_only" => match lang {
            Lang::Zh => "插件功能当前仅支持 dsh adapter",
            Lang::En => "The plugin feature currently only supports the dsh adapter",
        },

        // services/config_patch/*
        "err.patch.json_parse_failed" => match lang {
            Lang::Zh => "{} JSON 解析失败: {}",
            Lang::En => "Failed to parse {} JSON: {}",
        },
        "err.patch.json_object_required" => match lang {
            Lang::Zh => "{} JSON 顶层必须是对象",
            Lang::En => "{} JSON top level must be an object",
        },
        "err.patch.toml_parse_failed" => match lang {
            Lang::Zh => "{} TOML 解析失败: {}",
            Lang::En => "Failed to parse {} TOML: {}",
        },
        "err.patch.toml_parse_failed_generic" => match lang {
            Lang::Zh => "TOML 解析失败: {}",
            Lang::En => "Failed to parse TOML: {}",
        },
        "err.patch.preset_format_unsupported" => match lang {
            Lang::Zh => "global_preset 只支持 JSON、TOML 或 YAML",
            Lang::En => "global_preset only supports JSON, TOML or YAML",
        },
        "err.patch.toml_null_unsupported" => match lang {
            Lang::Zh => "TOML 不支持 null",
            Lang::En => "TOML does not support null",
        },
        "err.patch.path_conflict" => match lang {
            Lang::Zh => "operation `{}` 路径结构冲突",
            Lang::En => "Path structure conflict in operation `{}`",
        },
        "err.patch.path_empty" => match lang {
            Lang::Zh => "operation `{}` path 不能为空",
            Lang::En => "Path of operation `{}` must not be empty",
        },
        "err.patch.parent_missing" => match lang {
            Lang::Zh => "配置路径 {} 没有父目录",
            Lang::En => "Config path {} has no parent directory",
        },
        "err.patch.temp_create_failed" => match lang {
            Lang::Zh => "创建临时配置文件失败: {}",
            Lang::En => "Failed to create temporary config file: {}",
        },
        "err.patch.temp_write_failed" => match lang {
            Lang::Zh => "写入临时配置文件失败: {}",
            Lang::En => "Failed to write temporary config file: {}",
        },
        "err.patch.replace_failed" => match lang {
            Lang::Zh => "替换配置文件 {} 失败: {}",
            Lang::En => "Failed to replace config file {}: {}",
        },
        "err.patch.dir_create_failed" => match lang {
            Lang::Zh => "创建目录 {} 失败: {}",
            Lang::En => "Failed to create directory {}: {}",
        },
        "err.patch.json_number_out_of_range" => match lang {
            Lang::Zh => "JSON 数字超出支持范围",
            Lang::En => "JSON number is out of the supported range",
        },
        "err.patch.toml_number_out_of_range" => match lang {
            Lang::Zh => "TOML 数字超出支持范围",
            Lang::En => "TOML number is out of the supported range",
        },
        "err.patch.env_path_single_key" => match lang {
            Lang::Zh => "operation `{}` 的 ENV path 必须只有一个 key",
            Lang::En => "ENV path of operation `{}` must contain exactly one key",
        },
        "patch.label.target" => match lang {
            Lang::Zh => "目标配置",
            Lang::En => "target config",
        },
        "patch.label.preset" => match lang {
            Lang::Zh => "全局预设",
            Lang::En => "global preset",
        },

        // 会话消息内容标签（读取时渲染）
        "session.thinking" => match lang {
            Lang::Zh => "**[思考]**\n{}",
            Lang::En => "**[Thinking]**\n{}",
        },
        "session.reasoning" => match lang {
            Lang::Zh => "**[推理]**\n{}",
            Lang::En => "**[Reasoning]**\n{}",
        },
        "session.tool_result" => match lang {
            Lang::Zh => "**[工具结果]**\n```\n{}\n```",
            Lang::En => "**[Tool Result]**\n```\n{}\n```",
        },
        "session.tool_result_named" => match lang {
            Lang::Zh => "**[工具结果: {}]**\n```\n{}\n```",
            Lang::En => "**[Tool Result: {}]**\n```\n{}\n```",
        },
        "session.tool" => match lang {
            Lang::Zh => "**[工具: {}]**\n{}",
            Lang::En => "**[Tool: {}]**\n{}",
        },
        "session.tool_call" => match lang {
            Lang::Zh => "**[调用工具: {}]**\n```json\n{}\n```",
            Lang::En => "**[Tool Call: {}]**\n```json\n{}\n```",
        },
        "session.image" => match lang {
            Lang::Zh => "[图片]",
            Lang::En => "[Image]",
        },
        "session.audio" => match lang {
            Lang::Zh => "[音频]",
            Lang::En => "[Audio]",
        },
        "session.video" => match lang {
            Lang::Zh => "[视频]",
            Lang::En => "[Video]",
        },

        // 协议转换层注入的说明文本与标签
        "translate.tool_media_moved" => match lang {
            Lang::Zh => "[ccg: 上一条工具结果里的图片附在本条消息中]",
            Lang::En => "[ccg: images from the previous tool result are attached to this message]",
        },
        "translate.json_object_only" => match lang {
            Lang::Zh => "只输出一个 JSON 对象，不要带解释文字或代码块围栏。",
            Lang::En => "Output only a single JSON object, without any explanation text or code fences.",
        },
        "translate.json_schema_only" => match lang {
            Lang::Zh => "只输出符合以下 JSON Schema 的 JSON，不要带解释文字或代码块围栏：\n{}",
            Lang::En => "Output only JSON conforming to the following JSON Schema, without any explanation text or code fences:\n{}",
        },
        "translate.tool_call_flattened" => match lang {
            Lang::Zh => "[ccg: 调用工具 {}]\n{}",
            Lang::En => "[ccg: tool call {}]\n{}",
        },
        "translate.tool_labeled" => match lang {
            Lang::Zh => "[ccg: {}]\n{}",
            Lang::En => "[ccg: {}]\n{}",
        },
        "translate.tool_error_label" => match lang {
            Lang::Zh => "工具报错",
            Lang::En => "tool error",
        },
        "translate.tool_result_label" => match lang {
            Lang::Zh => "工具结果",
            Lang::En => "tool result",
        },
        "translate.stream_buffer_overflow" => match lang {
            Lang::Zh => "上游流超过缓冲上限且没有可解析的帧边界",
            Lang::En => "Upstream stream exceeded the buffer limit without a parseable frame boundary",
        },
        "translate.empty_stream_body" => match lang {
            Lang::Zh => "上游在流式请求下返回了没有内容的响应体",
            Lang::En => "Upstream returned a response body without content for a streaming request",
        },
        "translate.upstream_response_failed" => match lang {
            Lang::Zh => "上游响应失败",
            Lang::En => "Upstream response failed",
        },

        // 模型可用性检测
        "test.success" => match lang {
            Lang::Zh => "请求成功",
            Lang::En => "Success",
        },
        "test.timeout" => match lang {
            Lang::Zh => "请求超时",
            Lang::En => "Request timeout",
        },
        "test.stream_error" => match lang {
            Lang::Zh => "流错误: {}",
            Lang::En => "Stream error: {}",
        },
        "test.empty_stream" => match lang {
            Lang::Zh => "空流",
            Lang::En => "Empty stream",
        },
        "test.stream_timeout" => match lang {
            Lang::Zh => "流超时",
            Lang::En => "Stream timeout",
        },
        "test.default_probe_text" => match lang {
            Lang::Zh => "今天天气不错",
            Lang::En => "What is the weather like today?",
        },

        // 其他展示值
        "scheduler.task_placeholder" => match lang {
            Lang::Zh => "任务",
            Lang::En => "Task",
        },
        "profile.label.default" => match lang {
            Lang::Zh => "默认",
            Lang::En => "Default",
        },
        "err.skill.repo_name_used" => match lang {
            Lang::Zh => "仓库名称 '{}' 已被来源 '{}' 使用",
            Lang::En => "Repository name '{}' is already used by source '{}'",
        },

        _ => return None,
    };
    Some(text)
}
