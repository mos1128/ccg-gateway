use crate::db::models::PluginItem;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

type Result<T> = std::result::Result<T, String>;

// ==================== 返回结构 ====================

/// 插件操作返回结果（仅 CLI 输出，前端自行刷新列表）
#[derive(Debug, Serialize)]
pub struct PluginActionResult {
    pub cli_output: String,
}

// ==================== 常量 ====================

/// profile 模板内置的 bundle，不属于用户安装的插件（防御性过滤）
const IN_BOX_BUNDLES: [&str; 3] = [
    "@deepseek-ai/dsh-base",
    "@deepseek-ai/dsh-web-app",
    "@deepseek-ai/dsh-headless",
];

// ==================== CLI 执行 ====================

/// 执行 dsh 命令，返回完整输出；非零退出码视为失败
async fn run_dsh(args: &[&str]) -> Result<String> {
    #[cfg(windows)]
    let output = {
        // Windows 上直接用 cmd /c 执行，可自动解析 .cmd/.bat/.exe 等 PATHEXT 扩展名
        let mut cmd_args = vec!["/c", "dsh"];
        cmd_args.extend_from_slice(args);
        Command::new("cmd").args(&cmd_args).output().await
    };

    #[cfg(not(windows))]
    let output = Command::new("dsh").args(args).output().await;

    let output = output
        .map_err(|e| format!("执行 dsh 命令遇到错误：{}（请确认已安装 dsh 且在 PATH 上）", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let combined = if stdout.is_empty() {
        stderr
    } else if stderr.is_empty() {
        stdout
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    if !output.status.success() {
        let detail = if combined.is_empty() {
            format!("退出码 {}", output.status.code().unwrap_or(-1))
        } else {
            combined
        };
        return Err(format!("dsh 命令执行失败：\n{}", detail));
    }

    Ok(combined)
}

// ==================== 校验 ====================

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
        return Err(format!("非法 profile 名: {}", profile));
    }
    Ok(())
}

// ==================== 数据查询 ====================

/// 获取已安装插件列表
/// 扫描 `<config_dir>/profiles/` 下所有已初始化的 profile，
/// 读取各 profile package.json 的 dependencies（用户安装的包），
/// 并从 node_modules 中各包的 package.json 补充 version/description
pub async fn get_installed_plugins(config_dir: &std::path::Path) -> Result<Vec<PluginItem>> {
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
        .map_err(|e| format!("读取 profiles 目录失败: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("读取 profiles 目录条目失败: {}", e))?;
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
            .map_err(|e| format!("读取 profile package.json 失败: {}", e))?;
        let manifest: ProfileManifest = serde_json::from_str(&content)
            .map_err(|e| format!("解析 profile package.json 失败: {}", e))?;

        for name in manifest
            .dependencies
            .unwrap_or_default()
            .into_keys()
            .filter(|name| !IN_BOX_BUNDLES.contains(&name.as_str()))
        {
            let pkg_path = profile_dir
                .join("node_modules")
                .join(&name)
                .join("package.json");
            let (version, description) = if pkg_path.exists() {
                std::fs::read_to_string(&pkg_path)
                    .ok()
                    .and_then(|content| serde_json::from_str::<PackageManifest>(&content).ok())
                    .map(|pkg| (pkg.version, pkg.description))
                    .unwrap_or((None, None))
            } else {
                (None, None)
            };
            result.push(PluginItem {
                profile: profile_name.clone(),
                name,
                version,
                description,
            });
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

// ==================== 公开接口 ====================

/// 插件操作：install 的 param 为仓库地址，uninstall/update 的 param 为插件包名；
/// profile 不存在时由 dsh 自动初始化
pub async fn plugin_action(action: &str, profile: &str, param: &str) -> Result<PluginActionResult> {
    validate_profile(profile)?;

    let param = param.trim();
    if param.is_empty() {
        return Err("参数不能为空".to_string());
    }
    // Windows 上经 cmd /c 执行，元字符会被 shell 重解析，直接拒绝
    if param.chars().any(|c| "&|^<>%\"".contains(c)) || param.contains("  ") {
        return Err("参数包含非法字符".to_string());
    }

    let subcommand = match action {
        "install" => "add",
        "uninstall" => "remove",
        "update" => "update",
        _ => return Err(format!("未知操作: {}", action)),
    };

    let cli_output = run_dsh(&["plugin", "--profile", profile, subcommand, param]).await?;
    Ok(PluginActionResult { cli_output })
}
