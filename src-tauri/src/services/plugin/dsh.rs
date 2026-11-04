use super::{item, run_cli, validate_param, PluginActionResult, Result};
use crate::db::models::PluginItem;
use serde::Deserialize;
use std::path::Path;

/// profile 模板内置的 bundle，不属于用户安装的插件（防御性过滤）
const IN_BOX_BUNDLES: [&str; 6] = [
    "@deepseek-ai/dsh-base",
    "@deepseek-ai/dsh-web-app",
    "@deepseek-ai/dsh-headless",
    "@deepseek-ai/dsh-acp-app",
    "@deepseek-ai/dsh-sdk-app",
    "@deepseek-ai/dsh-sdk-minimal",
];

/// 校验 profile 名：与 dsh 的 resolveProfileDir 约束对齐（非空、非保留名、不含路径分隔符），
/// 同时拒绝会被 cmd /c 重解析的元字符
fn validate_profile(profile: &str) -> Result<()> {
    let invalid = profile.is_empty()
        || profile == "."
        || profile == ".."
        || profile == "node_modules"
        || profile.contains('/')
        || profile.contains('\\')
        || profile.chars().any(|c| "&|^<>%\"".contains(c));
    if invalid {
        return Err(crate::tr!("err.plugin.invalid_profile", profile));
    }
    Ok(())
}

/// 获取已安装插件列表
/// 扫描 `<config_dir>/profiles/` 下所有已初始化的 profile，
/// 读取各 profile package.json 的 dependencies（用户安装的包），
/// 并从 node_modules 中各包的 package.json 补充 version/description
pub async fn list(config_dir: &Path) -> Result<Vec<PluginItem>> {
    let profiles_dir = config_dir.join("profiles");

    if !profiles_dir.exists() {
        // 尚未创建任何 profile（未执行过 dsh），视为空列表
        return Ok(vec![]);
    }

    #[derive(Deserialize)]
    struct ProfileManifest {
        #[serde(default)]
        dependencies: Option<std::collections::HashMap<String, serde_json::Value>>,
    }

    #[derive(Deserialize)]
    struct PackageManifest {
        #[serde(default)]
        version: Option<String>,
        #[serde(default)]
        description: Option<String>,
    }

    let mut result: Vec<PluginItem> = Vec::new();

    let entries = std::fs::read_dir(&profiles_dir)
        .map_err(|e| crate::tr!("err.plugin.read_profiles_dir_failed", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| crate::tr!("err.plugin.read_profiles_entry_failed", e))?;
        let profile_name = entry.file_name().to_string_lossy().to_string();

        // node_modules 是 profiles 下的共享模块 fallback，不是 profile
        if profile_name == "node_modules" || !entry.path().is_dir() {
            continue;
        }

        let profile_dir = entry.path();
        let manifest_path = profile_dir.join("package.json");
        if !manifest_path.exists() {
            // 未初始化的目录，不是有效 profile
            continue;
        }

        let content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| crate::tr!("err.plugin.read_profile_manifest_failed", e))?;
        let manifest: ProfileManifest = serde_json::from_str(&content)
            .map_err(|e| crate::tr!("err.plugin.parse_profile_manifest_failed", e))?;

        for (name, spec) in manifest
            .dependencies
            .unwrap_or_default()
            .into_iter()
            .filter(|(name, _)| !IN_BOX_BUNDLES.contains(&name.as_str()))
        {
            let local_path = profile_dir
                .join("node_modules")
                .join(&name)
                .join("package.json");
            let pkg_path = if local_path.exists() {
                local_path
            } else {
                profiles_dir
                    .join("node_modules")
                    .join(&name)
                    .join("package.json")
            };
            let (version, description) = if pkg_path.exists() {
                std::fs::read_to_string(&pkg_path)
                    .ok()
                    .and_then(|content| serde_json::from_str::<PackageManifest>(&content).ok())
                    .map(|pkg| (pkg.version, pkg.description))
                    .unwrap_or((None, None))
            } else {
                (None, None)
            };
            result.push(item(
                Some(profile_name.clone()),
                name,
                version.or_else(|| spec.as_str().map(str::to_string)),
                description,
            ));
        }
    }

    // 先按 profile 再按名称排序，同 profile 的插件聚在一起
    result.sort_by(|a, b| {
        let a_key = (a.profile.to_lowercase(), a.name.to_lowercase());
        let b_key = (b.profile.to_lowercase(), b.name.to_lowercase());
        a_key.cmp(&b_key)
    });
    Ok(result)
}

/// 插件操作：install 的 param 为仓库地址，uninstall/update 的 param 为插件包名；
/// profile 不存在时由 dsh 自动初始化
pub async fn action(
    config_dir: &Path,
    action: &str,
    profile: &str,
    param: &str,
) -> Result<PluginActionResult> {
    validate_profile(profile)?;
    validate_param(param)?;
    let param = param.trim();

    let subcommand = match action {
        "install" => "add",
        "uninstall" => "remove",
        "update" => "update",
        _ => return Err(crate::tr!("err.plugin.unknown_action", action)),
    };

    let cli_output = run_cli(
        "dsh",
        &["plugin", "--profile", profile, subcommand, param],
        Some(("DSH_HOME", config_dir)),
    )
    .await?;
    Ok(PluginActionResult { cli_output })
}
