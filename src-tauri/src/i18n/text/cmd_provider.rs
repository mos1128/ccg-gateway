//! 词条域：命令层（provider / model_sync / pricing / credential 命令）

use crate::i18n::Lang;

pub fn text(lang: Lang, key: &str) -> Option<&'static str> {
    let text = match key {
        "err.provider_cmd.not_found" => match lang {
            Lang::Zh => "服务商不存在",
            Lang::En => "Provider not found",
        },
        "err.provider_cmd.agent_unknown" => match lang {
            Lang::Zh => "未知 Agent: {}",
            Lang::En => "Unknown Agent: {}",
        },
        "err.provider_cmd.protocol_invalid" => match lang {
            Lang::Zh => "无效 Protocol: {}",
            Lang::En => "Invalid Protocol: {}",
        },
        "err.provider_cmd.protocol_ambiguous" => match lang {
            Lang::Zh => "该 Agent 支持多个 Protocol，请明确选择",
            Lang::En => "This Agent supports multiple Protocols; choose one explicitly",
        },
        "err.provider_cmd.protocol_not_supported" => match lang {
            Lang::Zh => "Agent {} 无法使用 Protocol {}",
            Lang::En => "Agent {} cannot use Protocol {}",
        },
        "err.provider_cmd.disabled_direct" => match lang {
            Lang::Zh => "已停用的服务商不能用于中转直连",
            Lang::En => "A disabled provider cannot be used for provider-direct mode",
        },
        "err.provider_cmd.agent_direct_unsupported" => match lang {
            Lang::Zh => "Agent {} 不支持服务商直连模式",
            Lang::En => "Agent {} does not support provider-direct mode",
        },
        "err.provider_cmd.agent_required" => match lang {
            Lang::Zh => "必须指定 Agent",
            Lang::En => "Agent is required",
        },
        "err.provider_cmd.profile_not_found" => match lang {
            Lang::Zh => "Profile 不存在",
            Lang::En => "Profile not found",
        },
        "err.credential.not_found" => match lang {
            Lang::Zh => "凭证不存在",
            Lang::En => "Credential not found",
        },
        "err.credential.no_enabled_provider" => match lang {
            Lang::Zh => "default Profile 下没有已启用的服务商，请先启用服务商",
            Lang::En => "No enabled provider under the default Profile; enable a provider first",
        },
        "err.credential.no_official_credential" => match lang {
            Lang::Zh => "没有可用官方凭证，请先添加凭证",
            Lang::En => "No official credential available; add a credential first",
        },
        "err.credential.agent_unknown" => match lang {
            Lang::Zh => "未知 Agent: {}",
            Lang::En => "Unknown Agent: {}",
        },
        "err.credential.cli_mode_unsupported" => match lang {
            Lang::Zh => "不支持的 CLI 模式",
            Lang::En => "Unsupported CLI mode",
        },
        "err.credential.cli_mode_invalid" => match lang {
            Lang::Zh => {
                "cli_mode 只能是 proxy_route / provider_direct / official_direct / disabled"
            }
            Lang::En => {
                "cli_mode must be one of proxy_route / provider_direct / official_direct / disabled"
            }
        },
        "err.credential.agent_gateway_unsupported" => match lang {
            Lang::Zh => "Agent {} 不支持自动写入网关配置",
            Lang::En => "Agent {} does not support automatic gateway config writes",
        },
        "err.credential.agent_direct_unsupported" => match lang {
            Lang::Zh => "Agent {} 不支持服务商直连模式",
            Lang::En => "Agent {} does not support provider-direct mode",
        },
        "err.credential.agent_official_unsupported" => match lang {
            Lang::Zh => "Agent {} 不支持托管官方凭证",
            Lang::En => "Agent {} does not support managed official credentials",
        },
        _ => return None,
    };
    Some(text)
}
