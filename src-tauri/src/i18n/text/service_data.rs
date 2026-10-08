//! 词条域：数据服务（model_sync / official_credential / pricing / config_patch）

use crate::i18n::Lang;

pub fn text(lang: Lang, key: &str) -> Option<&'static str> {
    let text = match key {
        // model_sync
        "err.model_sync.provider_not_found" => match lang {
            Lang::Zh => "服务商不存在",
            Lang::En => "Provider not found",
        },
        "err.model_sync.model_name_required" => match lang {
            Lang::Zh => "模型名称不能为空",
            Lang::En => "Model name must not be empty",
        },
        "err.model_sync.model_not_found" => match lang {
            Lang::Zh => "模型不存在",
            Lang::En => "Model not found",
        },
        "err.model_sync.protocol_invalid" => match lang {
            Lang::Zh => "协议无效: {}",
            Lang::En => "Invalid protocol: {}",
        },
        "err.model_sync.upstream_empty" => match lang {
            Lang::Zh => "上游返回空模型列表",
            Lang::En => "Upstream returned an empty model list",
        },
        "err.model_sync.stale_snapshot" => match lang {
            Lang::Zh => "服务商配置已变更，已丢弃过期模型列表",
            Lang::En => "Provider configuration changed, so the stale model list was discarded",
        },
        "err.model_sync.request_failed" => match lang {
            Lang::Zh => "请求模型列表失败: {}",
            Lang::En => "Failed to request model list: {}",
        },
        "err.model_sync.read_failed" => match lang {
            Lang::Zh => "读取模型列表失败: {}",
            Lang::En => "Failed to read model list: {}",
        },
        "err.model_sync.parse_failed" => match lang {
            Lang::Zh => "解析模型列表失败: {}",
            Lang::En => "Failed to parse model list: {}",
        },
        "err.model_sync.openai_cursor_missing" => match lang {
            Lang::Zh => "OpenAI 分页响应缺少有效游标",
            Lang::En => "OpenAI pagination response is missing a valid cursor",
        },
        "err.model_sync.openai_page_limit" => match lang {
            Lang::Zh => "OpenAI 模型列表分页超过上限",
            Lang::En => "OpenAI model list pagination exceeded the page limit",
        },
        "err.model_sync.anthropic_request_failed" => match lang {
            Lang::Zh => "请求 Anthropic 模型列表失败: {}",
            Lang::En => "Failed to request Anthropic model list: {}",
        },
        "err.model_sync.anthropic_read_failed" => match lang {
            Lang::Zh => "读取 Anthropic 模型列表失败: {}",
            Lang::En => "Failed to read Anthropic model list: {}",
        },
        "err.model_sync.anthropic_parse_failed" => match lang {
            Lang::Zh => "解析 Anthropic 模型列表失败: {}",
            Lang::En => "Failed to parse Anthropic model list: {}",
        },
        "err.model_sync.anthropic_last_id_missing" => match lang {
            Lang::Zh => "Anthropic 分页响应缺少有效 last_id",
            Lang::En => "Anthropic pagination response is missing a valid last_id",
        },
        "err.model_sync.anthropic_page_limit" => match lang {
            Lang::Zh => "Anthropic 模型列表分页超过上限",
            Lang::En => "Anthropic model list pagination exceeded the page limit",
        },
        "err.model_sync.gemini_request_failed" => match lang {
            Lang::Zh => "请求 Gemini 模型列表失败: {}",
            Lang::En => "Failed to request Gemini model list: {}",
        },
        "err.model_sync.gemini_read_failed" => match lang {
            Lang::Zh => "读取 Gemini 模型列表失败: {}",
            Lang::En => "Failed to read Gemini model list: {}",
        },
        "err.model_sync.gemini_parse_failed" => match lang {
            Lang::Zh => "解析 Gemini 模型列表失败: {}",
            Lang::En => "Failed to parse Gemini model list: {}",
        },
        "err.model_sync.gemini_page_token_repeated" => match lang {
            Lang::Zh => "Gemini 分页响应包含重复 page token",
            Lang::En => "Gemini pagination response contains a repeated page token",
        },
        "err.model_sync.gemini_page_limit" => match lang {
            Lang::Zh => "Gemini 模型列表分页超过上限",
            Lang::En => "Gemini model list pagination exceeded the page limit",
        },
        "err.model_sync.empty_list" => match lang {
            Lang::Zh => "模型列表为空",
            Lang::En => "Model list is empty",
        },
        "err.model_sync.write_state_failed" => match lang {
            Lang::Zh => "{}; 写入同步状态失败: {}",
            Lang::En => "{}; Failed to write sync state: {}",
        },
        // official_credential
        "err.official_credential.payload_json_invalid" => match lang {
            Lang::Zh => "凭证 JSON 格式无效: {}",
            Lang::En => "Invalid credential JSON: {}",
        },
        "err.official_credential.schema_version_unsupported" => match lang {
            Lang::Zh => "不支持凭证 schema_version {}",
            Lang::En => "Unsupported credential schema_version {}",
        },
        "err.official_credential.files_empty" => match lang {
            Lang::Zh => "凭证 files 不能为空",
            Lang::En => "Credential files must not be empty",
        },
        "err.official_credential.feature_disabled_for_agent" => match lang {
            Lang::Zh => "Agent {} 的官方凭证功能未启用",
            Lang::En => "The official credential feature is not enabled for Agent {}",
        },
        "err.official_credential.missing_logical_file" => match lang {
            Lang::Zh => "凭证缺少逻辑文件 `{}`",
            Lang::En => "Credential is missing logical file `{}`",
        },
        "err.official_credential.logical_file_format_expected" => match lang {
            Lang::Zh => "逻辑文件 `{}` 应为 {} 格式",
            Lang::En => "Logical file `{}` must be in {} format",
        },
        "err.official_credential.missing_source_path" => match lang {
            Lang::Zh => "逻辑文件 `{}` 缺少来源路径 `{}`",
            Lang::En => "Logical file `{}` is missing source path `{}`",
        },
        "err.official_credential.non_json_requires_raw_text" => match lang {
            Lang::Zh => "非 JSON 逻辑文件 `{}` 必须使用原始文本且不能配置来源路径",
            Lang::En => {
                "Non-JSON logical file `{}` must use raw text and cannot declare a source path"
            }
        },
        "err.official_credential.path_conflict" => match lang {
            Lang::Zh => "{} 路径结构冲突",
            Lang::En => "{} path structure conflict",
        },
        "err.official_credential.path_missing" => match lang {
            Lang::Zh => "{} 路径不存在",
            Lang::En => "{} path does not exist",
        },
        "err.official_credential.target_not_object" => match lang {
            Lang::Zh => "{} 目标不是对象",
            Lang::En => "{} target is not an object",
        },
        "err.official_credential.missing_source" => match lang {
            Lang::Zh => "operation `{}` 缺少凭证来源",
            Lang::En => "Operation `{}` is missing a credential source",
        },
        "err.official_credential.logical_file_not_found" => match lang {
            Lang::Zh => "逻辑文件 `{}` 不存在",
            Lang::En => "Logical file `{}` does not exist",
        },
        "err.official_credential.logical_file_format_mismatch" => match lang {
            Lang::Zh => "逻辑文件 `{}` 格式与模板不一致",
            Lang::En => "Logical file `{}` format does not match the template",
        },
        "err.official_credential.non_json_no_source_path" => match lang {
            Lang::Zh => "非 JSON 逻辑文件 `{}` 不支持来源路径",
            Lang::En => "Non-JSON logical file `{}` does not support a source path",
        },
        "err.official_credential.source_path_not_found" => match lang {
            Lang::Zh => "逻辑文件 `{}` 中的来源路径不存在",
            Lang::En => "Source path does not exist in logical file `{}`",
        },
        "err.official_credential.content_must_be_text" => match lang {
            Lang::Zh => "operation `{}` 的凭证内容必须是文本",
            Lang::En => "Credential content of operation `{}` must be text",
        },
        "err.official_credential.value_exclusive" => match lang {
            Lang::Zh => "operation `{}` 的 value 和 value_from 必须二选一",
            Lang::En => "Operation `{}` must use either value or value_from",
        },
        "err.official_credential.path_required" => match lang {
            Lang::Zh => "{} path 不能为空",
            Lang::En => "{} path must not be empty",
        },
        "err.official_credential.target_file_invalid_json" => match lang {
            Lang::Zh => "目标文件 {} 不是合法 JSON: {}",
            Lang::En => "Target file {} is not valid JSON: {}",
        },
        "err.official_credential.read_target_file_failed" => match lang {
            Lang::Zh => "读取目标文件 {} 失败: {}",
            Lang::En => "Failed to read target file {}: {}",
        },
        "err.official_credential.unknown_agent" => match lang {
            Lang::Zh => "未知 Agent: {}",
            Lang::En => "Unknown Agent: {}",
        },
        "err.official_credential.feature_disabled" => match lang {
            Lang::Zh => "该 Agent 的官方凭证功能未启用",
            Lang::En => "The official credential feature is not enabled for this Agent",
        },
        "err.official_credential.no_operations" => match lang {
            Lang::Zh => "Agent {} 没有官方凭证写入规则",
            Lang::En => "Agent {} has no official credential write rules",
        },
        "err.official_credential.set_field_json_only" => match lang {
            Lang::Zh => "operation `{}` 的 set_field 只支持 JSON",
            Lang::En => "set_field of operation `{}` supports JSON only",
        },
        "err.official_credential.replace_conflict" => match lang {
            Lang::Zh => "operation `{}` 与同一目标文件的原文替换规则冲突",
            Lang::En => "Operation `{}` conflicts with a raw replacement rule for the same target file",
        },
        "err.official_credential.remove_file_failed" => match lang {
            Lang::Zh => "删除 {} 失败: {}",
            Lang::En => "Failed to delete {}: {}",
        },
        "err.official_credential.missing_content_from" => match lang {
            Lang::Zh => "operation `{}` 缺少 content_from",
            Lang::En => "Operation `{}` is missing content_from",
        },
        "err.official_credential.saved_files" => match lang {
            Lang::Zh => "已保存 {} 个文件",
            Lang::En => "{} files saved",
        },
        "err.official_credential.invalid_format" => match lang {
            Lang::Zh => "无效凭证格式",
            Lang::En => "Invalid credential format",
        },
        // pricing
        "err.pricing.request_source_failed" => match lang {
            Lang::Zh => "请求价格源失败: {}",
            Lang::En => "Failed to request price source: {}",
        },
        "err.pricing.source_http_status" => match lang {
            Lang::Zh => "价格源返回 HTTP {}",
            Lang::En => "Price source returned HTTP {}",
        },
        "err.pricing.source_http_status_detail" => match lang {
            Lang::Zh => "价格源返回 HTTP {}: {}",
            Lang::En => "Price source returned HTTP {}: {}",
        },
        "err.pricing.read_source_failed" => match lang {
            Lang::Zh => "读取价格源失败: {}",
            Lang::En => "Failed to read price source: {}",
        },
        "err.pricing.parse_source_failed" => match lang {
            Lang::Zh => "解析价格源失败: {}",
            Lang::En => "Failed to parse price source: {}",
        },
        "err.pricing.no_billable_models" => match lang {
            Lang::Zh => "价格源未返回可计费文本模型",
            Lang::En => "Price source returned no billable text models",
        },
        "err.pricing.write_state_failed" => match lang {
            Lang::Zh => "{}; 写入同步状态失败: {}",
            Lang::En => "{}; Failed to write sync state: {}",
        },
        // config_patch
        "err.config_patch.missing_target_endpoint" => match lang {
            Lang::Zh => "operation `{}` 缺少 target.endpoint",
            Lang::En => "Operation `{}` is missing target.endpoint",
        },
        "err.config_patch.missing_target_token" => match lang {
            Lang::Zh => "operation `{}` 缺少 target.token",
            Lang::En => "Operation `{}` is missing target.token",
        },
        "err.config_patch.unknown_template_variable" => match lang {
            Lang::Zh => "operation `{}` 使用了未知模板变量",
            Lang::En => "Operation `{}` uses an unknown template variable",
        },
        "err.config_patch.missing_value" => match lang {
            Lang::Zh => "operation `{}` 缺少 value",
            Lang::En => "Operation `{}` is missing value",
        },
        "err.config_patch.operation_id_required" => match lang {
            Lang::Zh => "operation id 不能为空",
            Lang::En => "Operation id must not be empty",
        },
        "err.config_patch.path_required" => match lang {
            Lang::Zh => "operation `{}` 的 path 不能为空",
            Lang::En => "Path of operation `{}` must not be empty",
        },
        "err.config_patch.operation_path_required" => match lang {
            Lang::Zh => "operation `{}` path 不能为空",
            Lang::En => "Operation `{}` path must not be empty",
        },
        "err.config_patch.value_not_accepted" => match lang {
            Lang::Zh => "operation `{}` 不接受 value",
            Lang::En => "Operation `{}` does not accept value",
        },
        "err.config_patch.path_conflict" => match lang {
            Lang::Zh => "operation `{}` 路径结构冲突",
            Lang::En => "Operation `{}` path structure conflict",
        },
        "err.config_patch.path_not_found" => match lang {
            Lang::Zh => "operation `{}` 路径不存在",
            Lang::En => "Operation `{}` path does not exist",
        },
        "err.config_patch.target_not_object" => match lang {
            Lang::Zh => "operation `{}` 目标不是对象",
            Lang::En => "Operation `{}` target is not an object",
        },
        "err.config_patch.read_file_failed" => match lang {
            Lang::Zh => "读取 {} 失败: {}",
            Lang::En => "Failed to read {}: {}",
        },
        "err.config_patch.write_task_failed" => match lang {
            Lang::Zh => "配置写入任务失败: {}",
            Lang::En => "Config write task failed: {}",
        },
        "err.config_patch.json_parse_failed" => match lang {
            Lang::Zh => "JSON 解析失败: {}",
            Lang::En => "Failed to parse JSON: {}",
        },
        "err.config_patch.json_root_must_be_object" => match lang {
            Lang::Zh => "JSON 顶层必须是对象",
            Lang::En => "JSON root must be an object",
        },
        "err.config_patch.yaml_parse_failed" => match lang {
            Lang::Zh => "{} YAML 解析失败: {}",
            Lang::En => "Failed to parse {} YAML: {}",
        },
        "err.config_patch.yaml_root_must_be_object" => match lang {
            Lang::Zh => "{} YAML 顶层必须是对象",
            Lang::En => "{} YAML root must be an object",
        },
        "err.config_patch.label_target_config" => match lang {
            Lang::Zh => "目标配置",
            Lang::En => "Target config",
        },
        _ => return None,
    };
    Some(text)
}
