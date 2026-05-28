use crate::db::models::{MarketplaceInfo, MarketplacePlugin, PluginFavoriteItem, PluginItem};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

type Result<T> = std::result::Result<T, String>;

// ==================== 返回结构 ====================

/// 插件操作返回结果（仅 CLI 输出，前端自行刷新列表）
#[derive(Debug, Serialize)]
pub struct PluginActionResult {
    pub cli_output: String,
}

/// 市场操作返回结果
#[derive(Debug, Serialize)]
pub struct MarketplaceActionResult {
    pub cli_output: String,
}

/// 收藏插件安装返回结果
#[derive(Debug, Serialize)]
pub struct FavoriteInstallResult {
    pub cli_output: String,
}

// ==================== CLI 执行 ====================

/// 执行 claude 命令，返回完整输出
async fn run_claude(args: &[&str]) -> Result<String> {
    #[cfg(windows)]
    let output = {
        // Windows 上直接用 cmd /c 执行，可自动解析 .cmd/.bat/.exe 等 PATHEXT 扩展名
        let mut cmd_args = vec!["/c", "claude"];
        cmd_args.extend_from_slice(args);
        Command::new("cmd").args(&cmd_args).output().await
    };

    #[cfg(not(windows))]
    let output = Command::new("claude").args(args).output().await;

    let output = output.map_err(|e| format!("执行命令遇到错误：{}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if stdout.is_empty() {
        Ok(stderr)
    } else if stderr.is_empty() {
        Ok(stdout)
    } else {
        Ok(format!("{}\n{}", stdout, stderr))
    }
}

// ==================== 数据查询 ====================

/// 从 plugin id 解析插件名和市场名
fn parse_plugin_id(id: &str) -> (String, Option<String>) {
    match id.split_once('@') {
        Some((name, marketplace)) => (name.to_string(), Some(marketplace.to_string())),
        None => (id.to_string(), None),
    }
}

/// 获取已安装插件列表（同步版本，内部使用）
async fn get_installed_plugins_map(
) -> Result<std::collections::HashMap<String, (Option<String>, bool)>> {
    let stdout = run_claude(&["plugin", "list", "--json"]).await?;

    if stdout.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    #[derive(Deserialize)]
    struct PluginInfo {
        id: String,
        version: Option<String>,
        #[serde(default)]
        enabled: bool,
    }

    let plugins: Vec<PluginInfo> =
        serde_json::from_str(&stdout).map_err(|e| format!("解析插件列表失败: {}", e))?;

    Ok(plugins
        .into_iter()
        .map(|p| {
            let (name, marketplace) = parse_plugin_id(&p.id);
            let key = format!("{}@{}", name, marketplace.unwrap_or_default());
            (key, (p.version, p.enabled))
        })
        .collect())
}

/// 从 known_marketplaces.json 读取市场列表
fn get_marketplaces_from_known_json(config_dir: &std::path::Path) -> Result<Vec<MarketplaceInfo>> {
    let known_path = config_dir.join("plugins").join("known_marketplaces.json");

    if !known_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&known_path)
        .map_err(|e| format!("读取 known_marketplaces.json 失败: {}", e))?;

    #[derive(Deserialize)]
    struct MarketplaceSource {
        #[serde(default)]
        url: Option<String>,
        #[serde(default)]
        repo: Option<String>,
        #[serde(default)]
        path: Option<String>,
    }

    #[derive(Deserialize)]
    struct MarketplaceEntry {
        source: MarketplaceSource,
    }

    let known: std::collections::HashMap<String, MarketplaceEntry> = serde_json::from_str(&content)
        .map_err(|e| format!("解析 known_marketplaces.json 失败: {}", e))?;

    let mut marketplaces = Vec::new();

    for (name, entry) in known {
        let marketplace_source = entry.source.url.or(entry.source.repo).or(entry.source.path);
        marketplaces.push(MarketplaceInfo {
            name,
            marketplace_source,
        });
    }

    Ok(marketplaces)
}

/// 获取市场的安装路径
fn get_marketplace_install_path(
    config_dir: &std::path::Path,
    marketplace_name: &str,
) -> Option<String> {
    let known_path = config_dir.join("plugins").join("known_marketplaces.json");

    if !known_path.exists() {
        return None;
    }

    if let Ok(content) = std::fs::read_to_string(&known_path) {
        #[derive(Deserialize)]
        struct MarketplaceEntry {
            #[serde(default)]
            #[serde(rename = "installLocation")]
            install_location: Option<String>,
        }

        if let Ok(known) =
            serde_json::from_str::<std::collections::HashMap<String, MarketplaceEntry>>(&content)
        {
            if let Some(entry) = known.get(marketplace_name) {
                return entry.install_location.clone();
            }
        }
    }

    None
}

/// 获取市场的来源信息（url、repo 或 path）
pub fn get_marketplace_source_info(
    config_dir: &std::path::Path,
    marketplace_name: &str,
) -> Option<String> {
    let known_path = config_dir.join("plugins").join("known_marketplaces.json");

    if !known_path.exists() {
        return None;
    }

    if let Ok(content) = std::fs::read_to_string(&known_path) {
        #[derive(Deserialize)]
        struct MarketplaceSource {
            #[serde(default)]
            url: Option<String>,
            #[serde(default)]
            repo: Option<String>,
            #[serde(default)]
            path: Option<String>,
        }

        #[derive(Deserialize)]
        struct MarketplaceEntry {
            source: MarketplaceSource,
        }

        if let Ok(known) =
            serde_json::from_str::<std::collections::HashMap<String, MarketplaceEntry>>(&content)
        {
            if let Some(entry) = known.get(marketplace_name) {
                return entry
                    .source
                    .url
                    .clone()
                    .or(entry.source.repo.clone())
                    .or(entry.source.path.clone());
            }
        }
    }

    None
}

/// 获取市场的来源类型（github/git/directory）
pub fn get_marketplace_source_type(
    config_dir: &std::path::Path,
    marketplace_name: &str,
) -> Option<String> {
    let known_path = config_dir.join("plugins").join("known_marketplaces.json");

    if !known_path.exists() {
        return None;
    }

    if let Ok(content) = std::fs::read_to_string(&known_path) {
        #[derive(Deserialize)]
        struct MarketplaceSource {
            source: Option<String>,
        }

        #[derive(Deserialize)]
        struct MarketplaceEntry {
            source: MarketplaceSource,
        }

        if let Ok(known) =
            serde_json::from_str::<std::collections::HashMap<String, MarketplaceEntry>>(&content)
        {
            if let Some(entry) = known.get(marketplace_name) {
                return entry.source.source.clone();
            }
        }
    }

    None
}

/// 读取市场中的插件列表
fn read_marketplace_plugins(
    marketplace_name: &str,
    config_dir: &std::path::Path,
) -> Result<Vec<MarketplacePlugin>> {
    let config_path =
        if let Some(install_path) = get_marketplace_install_path(config_dir, marketplace_name) {
            std::path::Path::new(&install_path)
                .join(".claude-plugin")
                .join("marketplace.json")
        } else {
            config_dir
                .join("plugins")
                .join("marketplaces")
                .join(marketplace_name)
                .join(".claude-plugin")
                .join("marketplace.json")
        };

    if !config_path.exists() {
        return Err(format!("市场配置文件不存在: {}", config_path.display()));
    }

    let content =
        std::fs::read_to_string(&config_path).map_err(|e| format!("读取市场配置失败: {}", e))?;

    #[derive(Deserialize)]
    struct MarketplaceConfig {
        plugins: Vec<PluginEntry>,
    }

    #[derive(Deserialize)]
    struct PluginEntry {
        name: String,
        description: Option<String>,
    }

    let config: MarketplaceConfig =
        serde_json::from_str(&content).map_err(|e| format!("解析市场配置失败: {}", e))?;

    Ok(config
        .plugins
        .into_iter()
        .map(|p| MarketplacePlugin {
            name: p.name,
            version: None,
            description: p.description,
            marketplace_name: marketplace_name.to_string(),
        })
        .collect())
}

// ==================== 公开接口 ====================

/// 获取已安装插件列表
/// 以 CLI `claude plugin list --json` 为权威数据源，
/// 用 marketplace 元数据补充 description 等字段
pub async fn get_installed_plugins(config_dir: &std::path::Path) -> Result<Vec<PluginItem>> {
    let installed_map = get_installed_plugins_map().await?;
    let marketplaces = get_marketplaces_from_known_json(config_dir)?;

    // 构建 marketplace 插件的元数据索引: "name@market" -> MarketplacePlugin
    let mut meta_index: std::collections::HashMap<String, MarketplacePlugin> =
        std::collections::HashMap::new();
    for market in &marketplaces {
        if let Ok(plugins) = read_marketplace_plugins(&market.name, config_dir) {
            for plugin in plugins {
                let key = format!("{}@{}", plugin.name, plugin.marketplace_name);
                meta_index.insert(key, plugin);
            }
        }
    }

    // 以 installed_map 为主数据源构建结果
    let mut result: Vec<PluginItem> = installed_map
        .iter()
        .map(|(key, (version, enabled))| {
            let meta = meta_index.get(key);
            let (name, marketplace_name) = match key.split_once('@') {
                Some((n, m)) => (n.to_string(), m.to_string()),
                None => (key.clone(), String::new()),
            };
            PluginItem {
                name,
                version: version.clone(),
                description: meta.and_then(|m| m.description.clone()),
                marketplace_name,
                is_installed: None,
                is_enabled: Some(*enabled),
                is_favorited: None,
            }
        })
        .collect();

    result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(result)
}

/// 获取指定市场中的插件列表（按需加载，用户点击市场时调用）
/// 只标记 is_installed，不需要 is_enabled/is_favorited（市场视图不展示这些）
pub async fn get_marketplace_plugins(
    market_name: &str,
    config_dir: &std::path::Path,
) -> Result<Vec<PluginItem>> {
    let installed_map = get_installed_plugins_map().await?;
    let plugins = read_marketplace_plugins(market_name, config_dir)?;

    let mut result: Vec<PluginItem> = plugins
        .into_iter()
        .map(|plugin| {
            let key = format!("{}@{}", plugin.name, plugin.marketplace_name);
            let (is_installed, version) = if let Some((ver, _)) = installed_map.get(&key) {
                (true, ver.clone())
            } else {
                (false, plugin.version.clone())
            };
            PluginItem {
                name: plugin.name,
                version,
                description: plugin.description,
                marketplace_name: plugin.marketplace_name,
                is_installed: Some(is_installed),
                is_enabled: None,
                is_favorited: None,
            }
        })
        .collect();

    result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(result)
}

/// 获取市场列表
pub fn get_marketplaces(config_dir: &std::path::Path) -> Result<Vec<MarketplaceInfo>> {
    get_marketplaces_from_known_json(config_dir)
}

/// 获取收藏列表（独立接口）
pub async fn get_favorites(
    favorites: Vec<(String, String, String, Option<String>)>,
) -> Result<Vec<PluginFavoriteItem>> {
    let installed_map = get_installed_plugins_map().await?;

    Ok(favorites
        .into_iter()
        .map(
            |(plugin_id, plugin_name, marketplace_name, marketplace_source)| {
                let is_installed = installed_map.contains_key(&plugin_id);
                PluginFavoriteItem {
                    plugin_id,
                    plugin_name,
                    marketplace_name,
                    is_installed,
                    marketplace_source,
                }
            },
        )
        .collect())
}

/// 插件操作
pub async fn plugin_action(action: &str, plugin_id: &str) -> Result<PluginActionResult> {
    let cli_output = match action {
        "install" => run_claude(&["plugin", "install", plugin_id]).await,
        "uninstall" => run_claude(&["plugin", "uninstall", plugin_id]).await,
        "enable" => run_claude(&["plugin", "enable", plugin_id]).await,
        "disable" => run_claude(&["plugin", "disable", plugin_id]).await,
        "update" => run_claude(&["plugin", "update", plugin_id]).await,
        _ => return Err(format!("未知操作: {}", action)),
    }?;
    Ok(PluginActionResult { cli_output })
}

/// 市场操作
pub async fn marketplace_action(action: &str, param: &str) -> Result<MarketplaceActionResult> {
    let cli_output = match action {
        "add" => run_claude(&["plugin", "marketplace", "add", param]).await,
        "remove" => run_claude(&["plugin", "marketplace", "remove", param]).await,
        "update" => run_claude(&["plugin", "marketplace", "update", param]).await,
        _ => return Err(format!("未知操作: {}", action)),
    }?;
    Ok(MarketplaceActionResult { cli_output })
}

/// 安装收藏的插件（包含市场检查和安装）
pub async fn install_favorite_plugin(
    plugin_id: &str,
    marketplace_name: &str,
    marketplace_source: Option<&str>,
    config_dir: &std::path::Path,
) -> Result<FavoriteInstallResult> {
    let mut cli_outputs = Vec::new();
    let marketplaces = get_marketplaces_from_known_json(config_dir)?;

    // 检查市场是否存在
    let marketplace_exists = marketplaces.iter().any(|m| m.name == marketplace_name);

    if !marketplace_exists {
        let source =
            marketplace_source.ok_or("市场不存在且无法获取来源信息，请手动安装市场后再试")?;

        // 判断是否为本地路径
        let is_local_path = source.contains(":\\")
            || source.contains(':') && source.contains('\\')
            || !source.contains('/');

        if is_local_path {
            return Err("该插件来自本地市场，无法自动恢复".to_string());
        }

        let market_output = run_claude(&["plugin", "marketplace", "add", source]).await?;
        cli_outputs.push(format!("[安装市场] {}", market_output));
    }

    let plugin_output = run_claude(&["plugin", "install", plugin_id]).await?;
    cli_outputs.push(format!("[安装插件] {}", plugin_output));

    Ok(FavoriteInstallResult {
        cli_output: cli_outputs.join("\n"),
    })
}
