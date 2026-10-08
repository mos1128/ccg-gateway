//! 词条域：运行时服务（scheduler / dsh_mcp / plugin）

use crate::i18n::Lang;

pub fn text(lang: Lang, key: &str) -> Option<&'static str> {
    let text = match key {
        // 定时任务
        "err.scheduler.recovered_interrupted" => match lang {
            Lang::Zh => "应用异常退出，任务中断",
            Lang::En => "The app exited unexpectedly, so the task was interrupted",
        },
        "err.scheduler.task_name_required" => match lang {
            Lang::Zh => "任务名称不能为空",
            Lang::En => "Task name is required",
        },
        "err.scheduler.task_running_delete_later" => match lang {
            Lang::Zh => "任务正在执行，稍后再删除",
            Lang::En => "The task is running, please delete it later",
        },
        "err.scheduler.task_running" => match lang {
            Lang::Zh => "任务正在执行",
            Lang::En => "The task is running",
        },
        "err.scheduler.task_disabled" => match lang {
            Lang::Zh => "任务已停用",
            Lang::En => "The task is disabled",
        },
        "err.scheduler.task_type_unsupported" => match lang {
            Lang::Zh => "不支持的任务类型: {}",
            Lang::En => "Unsupported task type: {}",
        },
        "err.scheduler.payload_parse_failed" => match lang {
            Lang::Zh => "服务商调用参数解析失败: {}",
            Lang::En => "Failed to parse the provider keepalive payload: {}",
        },
        "err.scheduler.no_executable_provider" => match lang {
            Lang::Zh => "没有可执行的服务商",
            Lang::En => "No provider is available to run",
        },
        "err.scheduler.all_mode_missing_cli_type" => match lang {
            Lang::Zh => "全选模式缺少 cli_type",
            Lang::En => "All mode requires cli_type",
        },
        "err.scheduler.profile_missing" => match lang {
            Lang::Zh => "Profile 不存在",
            Lang::En => "Profile does not exist",
        },
        "err.scheduler.no_available_provider" => match lang {
            Lang::Zh => "没有可用服务商",
            Lang::En => "No provider is available",
        },
        "err.scheduler.selected_mode_missing_provider_ids" => match lang {
            Lang::Zh => "指定服务商模式缺少 provider_ids",
            Lang::En => "Selected mode requires provider_ids",
        },
        "err.scheduler.invalid_target_mode" => match lang {
            Lang::Zh => "执行对象只能是 all 或 selected",
            Lang::En => "Execution target must be all or selected",
        },
        "err.scheduler.provider_disabled" => match lang {
            Lang::Zh => "服务商已停用",
            Lang::En => "Provider is disabled",
        },
        "err.scheduler.provider_missing" => match lang {
            Lang::Zh => "服务商不存在",
            Lang::En => "Provider does not exist",
        },
        "err.scheduler.task_run_failed" => match lang {
            Lang::Zh => "定时任务 {} 执行失败: {}",
            Lang::En => "Scheduled task {} failed: {}",
        },
        "err.scheduler.unknown_error" => match lang {
            Lang::Zh => "未知错误",
            Lang::En => "Unknown error",
        },
        "err.scheduler.task_not_found" => match lang {
            Lang::Zh => "任务不存在",
            Lang::En => "Task does not exist",
        },
        "err.scheduler.task_type_not_supported" => match lang {
            Lang::Zh => "只支持服务商调用任务",
            Lang::En => "Only provider keepalive tasks are supported",
        },
        "err.scheduler.schedule_type_unsupported" => match lang {
            Lang::Zh => "只支持间隔执行或定期执行",
            Lang::En => "Only interval or daily schedules are supported",
        },
        "err.scheduler.payload_not_json" => match lang {
            Lang::Zh => "任务参数不是合法 JSON: {}",
            Lang::En => "Task payload is not valid JSON: {}",
        },
        "err.scheduler.model_name_required" => match lang {
            Lang::Zh => "模型名不能为空",
            Lang::En => "Model name is required",
        },
        "err.scheduler.profile_invalid" => match lang {
            Lang::Zh => "profile 参数无效",
            Lang::En => "Invalid profile parameter",
        },
        "err.scheduler.selected_mode_requires_provider" => match lang {
            Lang::Zh => "指定服务商模式必须选择服务商",
            Lang::En => "Selected mode requires at least one provider",
        },
        "err.scheduler.retry_limit_negative" => match lang {
            Lang::Zh => "重试次数不能小于 0",
            Lang::En => "Retry limit must be at least 0",
        },
        "err.scheduler.retry_interval_invalid" => match lang {
            Lang::Zh => "重试间隔必须大于 0",
            Lang::En => "Retry interval must be greater than 0",
        },
        "err.scheduler.interval_not_integer" => match lang {
            Lang::Zh => "执行间隔必须是整数分钟",
            Lang::En => "Interval must be an integer number of minutes",
        },
        "err.scheduler.interval_not_positive" => match lang {
            Lang::Zh => "执行间隔必须大于 0 分钟",
            Lang::En => "Interval must be greater than 0 minutes",
        },
        "err.scheduler.interval_too_large" => match lang {
            Lang::Zh => "执行间隔过大",
            Lang::En => "Interval is too large",
        },
        "err.scheduler.next_daily_run_failed" => match lang {
            Lang::Zh => "无法计算下次定期执行时间",
            Lang::En => "Failed to compute the next daily run time",
        },
        "err.scheduler.daily_time_required" => match lang {
            Lang::Zh => "定期执行时间不能为空",
            Lang::En => "Daily run time is required",
        },
        "err.scheduler.daily_time_invalid" => match lang {
            Lang::Zh => "定期执行时间无效",
            Lang::En => "Invalid daily run time",
        },
        "err.scheduler.daily_format_invalid" => match lang {
            Lang::Zh => "定期执行格式必须是 HH:mm 或 JSON",
            Lang::En => "Daily schedule must use the HH:mm or JSON format",
        },
        "err.scheduler.daily_days_out_of_range" => match lang {
            Lang::Zh => "执行周期必须是 1 到 365 天",
            Lang::En => "Run period must be between 1 and 365 days",
        },
        "err.scheduler.daily_hour_out_of_range" => match lang {
            Lang::Zh => "执行小时必须是 0 到 23",
            Lang::En => "Run hour must be between 0 and 23",
        },
        "err.scheduler.daily_minute_out_of_range" => match lang {
            Lang::Zh => "执行分钟必须是 0 到 59",
            Lang::En => "Run minute must be between 0 and 59",
        },
        // DeepSeek Harness MCP
        "err.dsh_mcp.config_must_be_object" => match lang {
            Lang::Zh => "DeepSeek Harness MCP 配置必须是 JSON object",
            Lang::En => "DeepSeek Harness MCP config must be a JSON object",
        },
        "err.dsh_mcp.stdio_command_required" => match lang {
            Lang::Zh => "stdio MCP 必须提供非空 command",
            Lang::En => "stdio MCP requires a non-empty command",
        },
        "err.dsh_mcp.args_must_be_string_array" => match lang {
            Lang::Zh => "MCP args 必须是字符串数组",
            Lang::En => "MCP args must be an array of strings",
        },
        "err.dsh_mcp.cwd_must_be_string" => match lang {
            Lang::Zh => "MCP cwd 必须是字符串",
            Lang::En => "MCP cwd must be a string",
        },
        "err.dsh_mcp.streamable_http_url_required" => match lang {
            Lang::Zh => "streamable-http MCP 必须提供非空 url",
            Lang::En => "streamable-http MCP requires a non-empty url",
        },
        "err.dsh_mcp.patch_read_failed" => match lang {
            Lang::Zh => "读取 {} 失败: {}",
            Lang::En => "Failed to read {}: {}",
        },
        "err.dsh_mcp.json_invalid" => match lang {
            Lang::Zh => "DeepSeek Harness MCP JSON 格式错误: {}",
            Lang::En => "DeepSeek Harness MCP JSON format error: {}",
        },
        "err.dsh_mcp.name_invalid" => match lang {
            Lang::Zh => "DeepSeek Harness MCP 名称必须匹配 [A-Za-z0-9_-]{1,32}",
            Lang::En => "DeepSeek Harness MCP name must match [A-Za-z0-9_-]{1,32}",
        },
        "err.dsh_mcp.transport_must_be_string" => match lang {
            Lang::Zh => "MCP transport/type 必须是字符串",
            Lang::En => "MCP transport/type must be a string",
        },
        "err.dsh_mcp.legacy_sse_unsupported" => match lang {
            Lang::Zh => "DeepSeek Harness 不支持旧式 SSE MCP，请使用 streamable-http",
            Lang::En => {
                "DeepSeek Harness does not support legacy SSE MCP, please use streamable-http"
            }
        },
        "err.dsh_mcp.transport_unsupported" => match lang {
            Lang::Zh => "DeepSeek Harness MCP 不支持 transport/type: {}",
            Lang::En => "DeepSeek Harness MCP does not support transport/type: {}",
        },
        "err.dsh_mcp.transport_conflict" => match lang {
            Lang::Zh => "MCP transport 与 type 冲突",
            Lang::En => "MCP transport conflicts with type",
        },
        "err.dsh_mcp.transport_ambiguous" => match lang {
            Lang::Zh => "MCP 配置必须通过 command 或 url 明确传输类型",
            Lang::En => "MCP config must declare the transport via command or url",
        },
        "err.dsh_mcp.string_map_required" => match lang {
            Lang::Zh => "MCP {} 必须是字符串值 JSON object",
            Lang::En => "MCP {} must be a JSON object with string values",
        },
        "err.dsh_mcp.tool_call_timeout_must_be_number" => match lang {
            Lang::Zh => "MCP toolCallTimeoutMs 必须是数字",
            Lang::En => "MCP toolCallTimeoutMs must be a number",
        },
        "err.dsh_mcp.fail_on_startup_error_must_be_bool" => match lang {
            Lang::Zh => "MCP failOnStartupError 必须是布尔值",
            Lang::En => "MCP failOnStartupError must be a boolean",
        },
        "err.dsh_mcp.reconnect_must_be_object" => match lang {
            Lang::Zh => "MCP reconnect 必须是 JSON object",
            Lang::En => "MCP reconnect must be a JSON object",
        },
        "err.dsh_mcp.marker_must_be_own_line" => match lang {
            Lang::Zh => "DeepSeek Harness MCP 管理标记必须独占一行",
            Lang::En => "DeepSeek Harness MCP markers must each be on their own line",
        },
        "err.dsh_mcp.marker_invalid" => match lang {
            Lang::Zh => "DeepSeek Harness MCP 管理标记缺失、重复或顺序错误",
            Lang::En => "DeepSeek Harness MCP markers are missing, duplicated, or out of order",
        },
        "err.dsh_mcp.block_unknown_plugin" => match lang {
            Lang::Zh => "DeepSeek Harness MCP 管理块包含未知插件",
            Lang::En => "DeepSeek Harness MCP managed block contains an unknown plugin",
        },
        "err.dsh_mcp.entry_missing_server_name" => match lang {
            Lang::Zh => "DeepSeek Harness MCP 管理项缺少 serverName",
            Lang::En => "DeepSeek Harness MCP entry is missing serverName",
        },
        "err.dsh_mcp.block_parse_failed" => match lang {
            Lang::Zh => "DeepSeek Harness MCP 管理块解析失败: {}",
            Lang::En => "Failed to parse the DeepSeek Harness MCP managed block: {}",
        },
        "err.dsh_mcp.block_must_be_yaml_array" => match lang {
            Lang::Zh => "DeepSeek Harness MCP 管理块必须是 YAML 数组",
            Lang::En => "DeepSeek Harness MCP managed block must be a YAML array",
        },
        "err.dsh_mcp.block_insert_only" => match lang {
            Lang::Zh => "DeepSeek Harness MCP 管理块只能包含 insert",
            Lang::En => "DeepSeek Harness MCP managed block may only contain insert",
        },
        "err.dsh_mcp.block_duplicate_entry" => match lang {
            Lang::Zh => "DeepSeek Harness MCP 管理块包含重复项: {}",
            Lang::En => "DeepSeek Harness MCP managed block contains a duplicate entry: {}",
        },
        "err.dsh_mcp.patch_parse_failed" => match lang {
            Lang::Zh => "DeepSeek Harness cordis.patch.yml 解析失败: {}",
            Lang::En => "Failed to parse DeepSeek Harness cordis.patch.yml: {}",
        },
        "err.dsh_mcp.patch_must_be_yaml_array" => match lang {
            Lang::Zh => "DeepSeek Harness cordis.patch.yml 顶层必须是 YAML 数组",
            Lang::En => "DeepSeek Harness cordis.patch.yml must be a YAML array at the top level",
        },
        "err.dsh_mcp.empty_patch_single_brackets" => match lang {
            Lang::Zh => "DeepSeek Harness 空 MCP patch 必须使用独占一行的 []",
            Lang::En => "An empty DeepSeek Harness MCP patch must use a standalone line with []",
        },
        "err.dsh_mcp.empty_patch_multiple_brackets" => match lang {
            Lang::Zh => "DeepSeek Harness 空 MCP patch 包含多个 []",
            Lang::En => "The empty DeepSeek Harness MCP patch contains multiple []",
        },
        "err.dsh_mcp.serialize_failed" => match lang {
            Lang::Zh => "DeepSeek Harness MCP 配置序列化失败: {}",
            Lang::En => "Failed to serialize the DeepSeek Harness MCP config: {}",
        },
        // 插件管理
        "err.plugin.dsh_spawn_failed" => match lang {
            Lang::Zh => "执行 dsh 命令遇到错误：{}（请确认已安装 dsh 且在 PATH 上）",
            Lang::En => {
                "Failed to run the dsh command: {} (please make sure dsh is installed and on PATH)"
            }
        },
        "err.plugin.exit_code" => match lang {
            Lang::Zh => "退出码 {}",
            Lang::En => "Exit code {}",
        },
        "err.plugin.dsh_failed" => match lang {
            Lang::Zh => "dsh 命令执行失败：\n{}",
            Lang::En => "dsh command failed:\n{}",
        },
        "err.plugin.invalid_profile" => match lang {
            Lang::Zh => "非法 profile 名: {}",
            Lang::En => "Invalid profile name: {}",
        },
        "err.plugin.read_profiles_dir_failed" => match lang {
            Lang::Zh => "读取 profiles 目录失败: {}",
            Lang::En => "Failed to read the profiles directory: {}",
        },
        "err.plugin.read_profiles_entry_failed" => match lang {
            Lang::Zh => "读取 profiles 目录条目失败: {}",
            Lang::En => "Failed to read a profiles directory entry: {}",
        },
        "err.plugin.read_profile_manifest_failed" => match lang {
            Lang::Zh => "读取 profile package.json 失败: {}",
            Lang::En => "Failed to read the profile package.json: {}",
        },
        "err.plugin.parse_profile_manifest_failed" => match lang {
            Lang::Zh => "解析 profile package.json 失败: {}",
            Lang::En => "Failed to parse the profile package.json: {}",
        },
        "err.plugin.param_required" => match lang {
            Lang::Zh => "参数不能为空",
            Lang::En => "Parameter is required",
        },
        "err.plugin.param_invalid_chars" => match lang {
            Lang::Zh => "参数包含非法字符",
            Lang::En => "Parameter contains invalid characters",
        },
        "err.plugin.unknown_action" => match lang {
            Lang::Zh => "未知操作: {}",
            Lang::En => "Unknown action: {}",
        },
        _ => return None,
    };
    Some(text)
}
