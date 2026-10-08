//! 词条域：Agent 服务（services/agent.rs、agent_config.rs、provider_profile.rs）

use crate::i18n::Lang;

pub fn text(lang: Lang, key: &str) -> Option<&'static str> {
    let text = match key {
        "err.agent.profiles_need_provider_config" => match lang {
            Lang::Zh => "enabled profiles 需要启用 provider_config",
            Lang::En => "enabled profiles requires provider_config",
        },
        "err.agent.provider_config_operations_required" => match lang {
            Lang::Zh => "enabled provider_config 必须提供 operations",
            Lang::En => "enabled provider_config must provide operations",
        },
        "err.agent.operation_id_invalid" => match lang {
            Lang::Zh => "{} operation id `{}` 为空或重复",
            Lang::En => "{} operation id `{}` is empty or duplicated",
        },
        "err.agent.operation_file_empty" => match lang {
            Lang::Zh => "{} operation `{}` 的 file 不能为空",
            Lang::En => "{} operation `{}` file cannot be empty",
        },
        "err.agent.operation_path_empty" => match lang {
            Lang::Zh => "{} operation `{}` 的 path 不能为空",
            Lang::En => "{} operation `{}` path cannot be empty",
        },
        "err.agent.file_format_conflict" => match lang {
            Lang::Zh => "{} 文件 `{}` 不能同时使用不同 format",
            Lang::En => "{} file `{}` cannot use different formats at the same time",
        },
        "err.agent.file_private_conflict" => match lang {
            Lang::Zh => "{} 文件 `{}` 不能同时使用不同 private 值",
            Lang::En => "{} file `{}` cannot use different private values at the same time",
        },
        "err.agent.env_path_single_key" => match lang {
            Lang::Zh => "{} operation `{}` 的 ENV path 必须只有一个 key",
            Lang::En => "{} operation `{}` ENV path must contain exactly one key",
        },
        "err.agent.operation_value_required" => match lang {
            Lang::Zh => "{} operation `{}` 缺少 value",
            Lang::En => "{} operation `{}` is missing value",
        },
        "err.agent.operation_value_unexpected" => match lang {
            Lang::Zh => "{} operation `{}` 不接受 value",
            Lang::En => "{} operation `{}` does not accept value",
        },
        "err.agent.template_value_only" => match lang {
            Lang::Zh => "{} operation `{}` 的 target/agent 模板变量只能用于 value",
            Lang::En => "{} operation `{}` target/agent template variables can only be used in value",
        },
        "err.agent.profile_template_forbidden" => match lang {
            Lang::Zh => "{} operation `{}` 不能使用 Profile 模板变量",
            Lang::En => "{} operation `{}` cannot use Profile template variables",
        },
        "err.agent.profile_path_template_invalid" => match lang {
            Lang::Zh => "{} operation `{}` 的 Profile 路径模板变量位置无效",
            Lang::En => "{} operation `{}` has an invalid Profile path template variable position",
        },
        "err.agent.global_preset_file_required" => match lang {
            Lang::Zh => "enabled global_preset 必须提供 file",
            Lang::En => "enabled global_preset must provide file",
        },
        "err.agent.global_preset_format_invalid" => match lang {
            Lang::Zh => "enabled global_preset 只支持 json、toml 或 yaml",
            Lang::En => "enabled global_preset only supports json, toml or yaml",
        },
        "err.agent.official_login_operations_required" => match lang {
            Lang::Zh => "enabled official_login 必须提供 operations",
            Lang::En => "enabled official_login must provide operations",
        },
        "err.agent.official_login_content_from_missing" => match lang {
            Lang::Zh => "official_login operation `{}` 缺少 content_from",
            Lang::En => "official_login operation `{}` is missing content_from",
        },
        "err.agent.official_login_source_path_forbidden" => match lang {
            Lang::Zh => "official_login operation `{}` 的非 JSON 来源不能配置 path",
            Lang::En => "official_login operation `{}` non-JSON source cannot configure path",
        },
        "err.agent.official_login_set_field_invalid" => match lang {
            Lang::Zh => "official_login operation `{}` 必须使用 JSON format，并正确配置 path 和 value",
            Lang::En => "official_login operation `{}` must use JSON format and configure path and value correctly",
        },
        "err.agent.official_login_format_conflict" => match lang {
            Lang::Zh => "official_login 逻辑文件 `{}` 不能使用不同 format",
            Lang::En => "official_login logical file `{}` cannot use different formats",
        },
        "err.agent.icon_view_box_required" => match lang {
            Lang::Zh => "Agent icon 必须提供 view_box 和非空 paths",
            Lang::En => "Agent icon must provide view_box and non-empty paths",
        },
        "err.agent.icon_gradient_min_stops" => match lang {
            Lang::Zh => "Agent icon linear_gradient 至少需要两个 stop",
            Lang::En => "Agent icon linear_gradient requires at least two stops",
        },
        "err.agent.icon_gradient_stop_invalid" => match lang {
            Lang::Zh => "Agent icon gradient stop 必须使用 0 到 1 的 offset 和六位十六进制颜色",
            Lang::En => "Agent icon gradient stop must use an offset between 0 and 1 and a six-digit hex color",
        },
        "err.agent.icon_path_gradient_required" => match lang {
            Lang::Zh => "Agent icon path 使用 linear_gradient 时必须提供渐变 stop",
            Lang::En => "Agent icon path using linear_gradient must provide gradient stops",
        },
        "err.agent.icon_path_fill_invalid" => match lang {
            Lang::Zh => "Agent icon path fill 必须是六位十六进制颜色或 linear_gradient",
            Lang::En => "Agent icon path fill must be a six-digit hex color or linear_gradient",
        },
        "err.agent.icon_path_opacity_invalid" => match lang {
            Lang::Zh => "Agent icon path opacity 必须在 0 到 1 之间",
            Lang::En => "Agent icon path opacity must be between 0 and 1",
        },
        "err.agent.icon_path_rule_invalid" => match lang {
            Lang::Zh => "Agent icon path rule 只支持 nonzero 或 evenodd",
            Lang::En => "Agent icon path rule only supports nonzero or evenodd",
        },
        "err.agent.skills_directory_required" => match lang {
            Lang::Zh => "enabled skills 必须提供 directory",
            Lang::En => "enabled skills must provide directory",
        },
        "err.agent.mcp_file_required" => match lang {
            Lang::Zh => "enabled mcp 必须提供 file",
            Lang::En => "enabled mcp must provide file",
        },
        "err.agent.mcp_dsh_yaml_only" => match lang {
            Lang::Zh => "mcp adapter dsh 只支持 yaml format",
            Lang::En => "mcp adapter dsh only supports yaml format",
        },
        "err.agent.mcp_dsh_no_servers_path" => match lang {
            Lang::Zh => "mcp adapter dsh 不使用 servers_path",
            Lang::En => "mcp adapter dsh does not use servers_path",
        },
        "err.agent.mcp_format_invalid" => match lang {
            Lang::Zh => "enabled mcp 只支持 json 或 toml",
            Lang::En => "enabled mcp only supports json or toml",
        },
        "err.agent.mcp_opencode_json_only" => match lang {
            Lang::Zh => "mcp adapter opencode 只支持 json format",
            Lang::En => "mcp adapter opencode only supports json format",
        },
        "err.agent.mcp_servers_path_required" => match lang {
            Lang::Zh => "enabled mcp 必须提供非空 servers_path",
            Lang::En => "enabled mcp must provide a non-empty servers_path",
        },
        "err.agent.adapter_required" => match lang {
            Lang::Zh => "enabled {} 必须提供 adapter",
            Lang::En => "enabled {} must provide adapter",
        },
        "err.agent.file_required" => match lang {
            Lang::Zh => "enabled {} 必须提供 file",
            Lang::En => "enabled {} must provide file",
        },
        "err.agent.profiles_file_required" => match lang {
            Lang::Zh => "enabled profiles 必须提供 profile_file",
            Lang::En => "enabled profiles must provide profile_file",
        },
        "err.agent.profile_file_placeholder" => match lang {
            Lang::Zh => "profiles.profile_file 必须包含 {profile}",
            Lang::En => "profiles.profile_file must contain {profile}",
        },
        "err.agent.profiles_operations_required" => match lang {
            Lang::Zh => "enabled profiles 必须提供 operations",
            Lang::En => "enabled profiles must provide operations",
        },
        "err.agent.launch_empty_arg" => match lang {
            Lang::Zh => "profiles.launch.{} 不能包含空参数",
            Lang::En => "profiles.launch.{} cannot contain empty arguments",
        },
        "err.agent.launch_template_forbidden" => match lang {
            Lang::Zh => "profiles.launch.{} 不能使用 target/agent 模板变量",
            Lang::En => "profiles.launch.{} cannot use target/agent template variables",
        },
        "err.agent.launch_relative_path_forbidden" => match lang {
            Lang::Zh => "profiles.launch.{} 不能使用 {profile.relative_path}",
            Lang::En => "profiles.launch.{} cannot use {profile.relative_path}",
        },
        "err.agent.launch_default_path_forbidden" => match lang {
            Lang::Zh => "profiles.launch.default 不能使用 {profile.absolute_path}",
            Lang::En => "profiles.launch.default cannot use {profile.absolute_path}",
        },
        "err.agent.schema_version_unsupported" => match lang {
            Lang::Zh => "不支持 schema_version {}",
            Lang::En => "Unsupported schema_version {}",
        },
        "err.agent.invalid_id" => match lang {
            Lang::Zh => "Agent id 格式无效",
            Lang::En => "Invalid agent id format",
        },
        "err.agent.name_empty" => match lang {
            Lang::Zh => "Agent name 不能为空",
            Lang::En => "Agent name cannot be empty",
        },
        "err.agent.remark_empty" => match lang {
            Lang::Zh => "Agent remark 不能为空",
            Lang::En => "Agent remark cannot be empty",
        },
        "err.agent.config_dir_empty" => match lang {
            Lang::Zh => "Agent config_dir 不能为空",
            Lang::En => "Agent config_dir cannot be empty",
        },
        "err.agent.sort_order_negative" => match lang {
            Lang::Zh => "Agent sort_order 不能小于 0",
            Lang::En => "Agent sort_order cannot be less than 0",
        },
        "err.agent.user_agent_pattern_required" => match lang {
            Lang::Zh => "user_agent 必须包含非空匹配项",
            Lang::En => "user_agent must contain non-empty patterns",
        },
        "err.agent.protocols_empty" => match lang {
            Lang::Zh => "protocols 不能为空",
            Lang::En => "protocols cannot be empty",
        },
        "err.agent.protocols_duplicated" => match lang {
            Lang::Zh => "protocols 不能重复",
            Lang::En => "protocols cannot be duplicated",
        },
        "err.agent.filename_id_mismatch" => match lang {
            Lang::Zh => "文件名必须与 Agent id `{}` 一致",
            Lang::En => "File name must match Agent id `{}`",
        },
        "err.agent.read_user_dir_failed" => match lang {
            Lang::Zh => "读取用户模板目录失败: {}",
            Lang::En => "Failed to read user template directory: {}",
        },
        "err.agent.read_user_dir_entry_failed" => match lang {
            Lang::Zh => "读取用户模板目录项失败: {}",
            Lang::En => "Failed to read user template directory entry: {}",
        },
        "err.agent.template_filename_not_utf8" => match lang {
            Lang::Zh => "模板文件名不是有效 UTF-8",
            Lang::En => "Template file name is not valid UTF-8",
        },
        "err.agent.read_user_template_failed" => match lang {
            Lang::Zh => "读取用户模板失败: {}",
            Lang::En => "Failed to read user template: {}",
        },
        "err.agent.duplicate_id" => match lang {
            Lang::Zh => "Agent id `{}` 重复",
            Lang::En => "Duplicate Agent id `{}`",
        },
        "err.agent.unknown_id" => match lang {
            Lang::Zh => "未知 Agent: {}",
            Lang::En => "Unknown Agent: {}",
        },
        "err.agent_config.profile_unsupported" => match lang {
            Lang::Zh => "Agent {} 不支持 Profile",
            Lang::En => "Agent {} does not support Profiles",
        },
        "err.agent_config.profile_relative_path_missing" => match lang {
            Lang::Zh => "当前 Profile 模板未提供 relative_path",
            Lang::En => "The current Profile template does not provide relative_path",
        },
        "err.agent_config.profile_absolute_path_missing" => match lang {
            Lang::Zh => "当前 Profile 未提供 absolute_path",
            Lang::En => "The current Profile does not provide absolute_path",
        },
        "err.agent_config.unknown_template_variable" => match lang {
            Lang::Zh => "未知 Profile 模板变量: {}",
            Lang::En => "Unknown Profile template variable: {}",
        },
        "err.agent_config.provider_file_format_conflict" => match lang {
            Lang::Zh => "Provider 配置文件 `{}` 不能同时使用不同 format",
            Lang::En => "Provider config file `{}` cannot use different formats at the same time",
        },
        "err.agent_config.provider_file_private_conflict" => match lang {
            Lang::Zh => "Provider 配置文件 `{}` 不能同时使用不同 private 值",
            Lang::En => "Provider config file `{}` cannot use different private values at the same time",
        },
        "err.agent_config.file_format_conflict" => match lang {
            Lang::Zh => "配置文件 `{}` 不能同时使用不同 format",
            Lang::En => "Config file `{}` cannot use different formats at the same time",
        },
        "err.agent_config.profile_file_not_declared" => match lang {
            Lang::Zh => "Agent {} 未声明 Profile 配置文件",
            Lang::En => "Agent {} does not declare a Profile config file",
        },
        "err.agent_config.global_preset_file_missing" => match lang {
            Lang::Zh => "global_preset 缺少 file",
            Lang::En => "global_preset is missing file",
        },
        "err.agent_config.global_preset_format_missing" => match lang {
            Lang::Zh => "global_preset 缺少 format",
            Lang::En => "global_preset is missing format",
        },
        "err.agent_config.read_file_failed" => match lang {
            Lang::Zh => "读取 {} 失败: {}",
            Lang::En => "Failed to read {}: {}",
        },
        "err.agent_config.write_mode_invalid" => match lang {
            Lang::Zh => "config_write_mode 只能是 'overwrite' 或 'merge'",
            Lang::En => "config_write_mode must be either 'overwrite' or 'merge'",
        },
        "err.agent_config.global_preset_disabled" => match lang {
            Lang::Zh => "Agent {} 未启用全局预设",
            Lang::En => "Global preset is not enabled for Agent {}",
        },
        "err.agent_config.provider_config_disabled" => match lang {
            Lang::Zh => "Provider 配置功能未启用",
            Lang::En => "The provider config feature is not enabled",
        },
        "err.agent_config.parse_file_failed" => match lang {
            Lang::Zh => "解析 {} 失败: {}",
            Lang::En => "Failed to parse {}: {}",
        },
        "err.agent_config.provider_direct_unsupported" => match lang {
            Lang::Zh => "Agent {} 不支持服务商直连模式",
            Lang::En => "Agent {} does not support provider direct mode",
        },
        "err.provider_profile.direct_provider_missing" => match lang {
            Lang::Zh => "直连服务商 {} 不存在",
            Lang::En => "Direct provider {} does not exist",
        },
        "err.provider_profile.cleanup_new_config_failed" => match lang {
            Lang::Zh => "清理新 Profile 配置失败: {}",
            Lang::En => "Failed to clean up the new Profile config: {}",
        },
        "err.provider_profile.restore_name_failed" => match lang {
            Lang::Zh => "恢复 Profile 名称失败: {}",
            Lang::En => "Failed to restore the Profile name: {}",
        },
        "err.provider_profile.restore_old_config_failed" => match lang {
            Lang::Zh => "恢复旧 Profile 配置失败: {}",
            Lang::En => "Failed to restore the old Profile config: {}",
        },
        "err.provider_profile.invalid_name" => match lang {
            Lang::Zh => "Profile 名称仅支持英文、数字、空格、下划线和短横线",
            Lang::En => "Profile names only support letters, digits, spaces, underscores and hyphens",
        },
        "err.provider_profile.agent_profile_unsupported" => match lang {
            Lang::Zh => "该 Agent 不支持 Profile",
            Lang::En => "This Agent does not support Profiles",
        },
        "err.provider_profile.create_failed" => match lang {
            Lang::Zh => "Profile 创建失败",
            Lang::En => "Failed to create Profile",
        },
        "err.provider_profile.default_rename_forbidden" => match lang {
            Lang::Zh => "默认 Profile 不能重命名",
            Lang::En => "The default Profile cannot be renamed",
        },
        "err.provider_profile.not_found" => match lang {
            Lang::Zh => "Profile 不存在",
            Lang::En => "Profile not found",
        },
        "err.provider_profile.rename_config_mismatch" => match lang {
            Lang::Zh => "Profile `{}` 的现有配置无法匹配 Gateway 或服务商，不能安全重命名",
            Lang::En => "The existing config of Profile `{}` does not match Gateway or any provider, so it cannot be renamed safely",
        },
        "err.provider_profile.cleanup_old_config_failed" => match lang {
            Lang::Zh => "清理旧 Profile 模板配置失败: {}; 恢复旧配置也失败: {}",
            Lang::En => "Failed to clean up the old Profile template config: {}; restoring the old config also failed: {}",
        },
        "err.provider_profile.rename_failed_with_restore" => match lang {
            Lang::Zh => "Profile 重命名失败: {}; 恢复旧模板配置也失败: {}",
            Lang::En => "Failed to rename Profile: {}; restoring the old template config also failed: {}",
        },
        "err.provider_profile.rewrite_failed_with_rollback" => match lang {
            Lang::Zh => "按新 Profile 模板重写配置失败: {}; 回滚也未完全成功: {}",
            Lang::En => "Failed to rewrite the config with the new Profile template: {}; the rollback was also incomplete: {}",
        },
        "err.provider_profile.rewrite_failed_restored" => match lang {
            Lang::Zh => "按新 Profile 模板重写配置失败，已恢复原 Profile: {}",
            Lang::En => "Failed to rewrite the config with the new Profile template, the original Profile has been restored: {}",
        },
        "err.provider_profile.default_delete_forbidden" => match lang {
            Lang::Zh => "默认 Profile 不能删除",
            Lang::En => "The default Profile cannot be deleted",
        },
        "err.provider_profile.delete_config_mismatch" => match lang {
            Lang::Zh => "Profile `{}` 的现有配置无法匹配 Gateway 或服务商，不能安全删除",
            Lang::En => "The existing config of Profile `{}` does not match Gateway or any provider, so it cannot be deleted safely",
        },
        "err.provider_profile.delete_data_failed" => match lang {
            Lang::Zh => "删除 Profile 数据失败: {}; 恢复配置也失败: {}",
            Lang::En => "Failed to delete Profile data: {}; restoring the config also failed: {}",
        },
        "err.provider_profile.name_exists" => match lang {
            Lang::Zh => "名称已存在",
            Lang::En => "The name already exists",
        },
        "err.provider_profile.check_failed" => match lang {
            Lang::Zh => "检查 {} 失败: {}",
            Lang::En => "Failed to check {}: {}",
        },
        "err.provider_profile.rename_file_failed" => match lang {
            Lang::Zh => "Profile 配置文件从 {} 重命名到 {} 失败: {}",
            Lang::En => "Failed to rename the Profile config file from {} to {}: {}",
        },
        "err.provider_profile.rename_commit_failed" => match lang {
            Lang::Zh => "Profile 数据库重命名提交失败: {}; 配置文件回滚也失败: {}",
            Lang::En => "Failed to commit the Profile database rename: {}; rolling back the config file also failed: {}",
        },
        "err.provider_profile.target_file_exists" => match lang {
            Lang::Zh => "目标 Profile 配置文件已存在: {}",
            Lang::En => "The target Profile config file already exists: {}",
        },
        "err.provider_profile.task_running" => match lang {
            Lang::Zh => "该 Profile 关联的定时任务正在执行，请稍后再删除",
            Lang::En => "A scheduled task associated with this Profile is running, please delete it later",
        },
        "err.provider_profile.duplicate_data" => match lang {
            Lang::Zh => "数据已存在，请勿重复添加",
            Lang::En => "The record already exists, do not add it again",
        },
        _ => return None,
    };
    Some(text)
}
