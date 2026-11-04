mod dsh;

use crate::db::models::PluginItem;
use serde::Serialize;
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;

const CLI_TIMEOUT: Duration = Duration::from_secs(15);

pub(crate) type Result<T> = std::result::Result<T, String>;

/// 插件操作返回结果（仅 CLI 输出，前端自行刷新列表）
#[derive(Debug, Serialize)]
pub struct PluginActionResult {
    pub cli_output: String,
}

pub async fn get_installed_plugins(config_dir: &Path) -> Result<Vec<PluginItem>> {
    dsh::list(config_dir).await
}

pub async fn plugin_action(
    config_dir: &Path,
    action: &str,
    profile: &str,
    param: &str,
) -> Result<PluginActionResult> {
    dsh::action(config_dir, action, profile, param).await
}

pub(super) async fn run_cli(
    binary: &str,
    args: &[&str],
    env: Option<(&str, &Path)>,
) -> Result<String> {
    #[cfg(windows)]
    let mut command = {
        // Windows 上通过 cmd /c 执行，可自动解析 .cmd/.bat/.exe 等 PATHEXT 扩展名
        let mut command = Command::new("cmd");
        command.args(["/c", binary]);
        command.args(args);
        if let Some((key, value)) = env {
            command.env(key, value);
        }
        command
    };

    #[cfg(not(windows))]
    let mut command = {
        let mut command = Command::new(binary);
        command.args(args);
        if let Some((key, value)) = env {
            command.env(key, value);
        }
        command
    };

    command.kill_on_drop(true);
    let output = tokio::time::timeout(CLI_TIMEOUT, command.output())
        .await
        .map_err(|_| crate::tr!("err.plugin.cli_timeout", binary))?
        .map_err(|error| crate::tr!("err.plugin.cli_spawn_failed", binary, error))?;

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
            crate::tr!("err.plugin.exit_code", output.status.code().unwrap_or(-1))
        } else {
            combined
        };
        return Err(crate::tr!("err.plugin.cli_failed", binary, detail));
    }

    Ok(combined)
}

pub(super) fn validate_param(param: &str) -> Result<()> {
    let param = param.trim();
    if param.is_empty() {
        return Err(crate::tr!("err.plugin.param_required"));
    }
    if param.chars().any(|c| "&|^<>%\"\r\n".contains(c)) || param.contains("  ") {
        return Err(crate::tr!("err.plugin.param_invalid_chars"));
    }
    Ok(())
}

pub(super) fn item(
    profile: Option<String>,
    name: String,
    version: Option<String>,
    description: Option<String>,
) -> PluginItem {
    PluginItem {
        profile: profile.unwrap_or_default(),
        name,
        version,
        description,
    }
}
