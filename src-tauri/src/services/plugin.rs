use crate::config::get_data_dir;
use crate::db::models::{MarketplaceInfo, MarketplacePlugin, PluginItem};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;

type Result<T> = std::result::Result<T, String>;

// ==================== 缓存结构 ====================

/// 插件缓存结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginsCache {
    pub plugins: Vec<PluginItem>,
    pub marketplaces: Vec<MarketplaceInfo>,
    pub generated_at: i64,
}

/// 操作返回结果
#[derive(Debug, Serialize)]
pub struct PluginActionResult {
    pub cli_output: String,
    pub plugins: Vec<PluginItem>,
}

/// 市场操作返回结果
#[derive(Debug, Serialize)]
pub struct MarketplaceActionResult {
    pub cli_output: String,
    pub plugins: Vec<PluginItem>,
    pub marketplaces: Vec<MarketplaceInfo>,
}

// ==================== CLI 执行 ====================

/// 执行 claude 命令，返回完整输出
fn run_claude(args: &[&str]) -> Result<String> {
    let args_str = args.iter().map(|s| {
        if s.contains(' ') || s.contains('"') {
            format!("\"{}\"", s.replace('"', "\\\""))
        } else {
            s.to_string()
        }
    }).collect::<Vec<_>>().join(" ");

    let output = Command::new("cmd")
        .args(["/c", &format!("claude {}", args_str)])
        .output()
        .map_err(|e| format!("执行命令遇到错误：{}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    // 合并 stdout 和 stderr 作为完整输出
    if stdout.is_empty() {
        Ok(stderr)
    } else if stderr.is_empty() {
        Ok(stdout)
    } else {
        Ok(format!("{}\n{}", stdout, stderr))
    }
}

// ==================== 缓存文件操作 ====================

/// 获取缓存文件路径（软件数据目录）
fn get_cache_path() -> PathBuf {
    get_data_dir().join("plugins_cache.json")
}

/// 读取缓存
fn read_cache() -> Option<PluginsCache> {
    let cache_path = get_cache_path();
    if !cache_path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&cache_path).ok()?;
    serde_json::from_str(&content).ok()
}

/// 写入缓存
fn write_cache(cache: &PluginsCache) -> Result<()> {
    let cache_path = get_cache_path();

    // 确保目录存在
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建缓存目录失败: {}", e))?;
    }

    let content = serde_json::to_string_pretty(cache).map_err(|e| format!("序列化缓存失败: {}", e))?;
    std::fs::write(&cache_path, content).map_err(|e| format!("写入缓存失败: {}", e))?;

    Ok(())
}

// ==================== 数据查询 ====================

/// 从 plugin id 解析插件名和市场名
fn parse_plugin_id(id: &str) -> (String, Option<String>) {
    match id.split_once('@') {
        Some((name, marketplace)) => (name.to_string(), Some(marketplace.to_string())),
        None => (id.to_string(), None),
    }
}

/// 获取市场目录
fn get_marketplaces_dir(config_dir: &std::path::Path) -> PathBuf {
    config_dir.join("plugins").join("marketplaces")
}

/// 获取已安装插件列表（同步版本，内部使用）
fn get_installed_plugins_sync() -> Result<std::collections::HashMap<String, (Option<String>, bool)>> {
    let stdout = run_claude(&["plugin", "list", "--json"])?;

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

    let plugins: Vec<PluginInfo> = serde_json::from_str(&stdout)
        .map_err(|e| format!("解析插件列表失败: {}", e))?;

    Ok(plugins.into_iter().map(|p| {
        let (name, marketplace) = parse_plugin_id(&p.id);
        let key = format!("{}@{}", name, marketplace.unwrap_or_default());
        (key, (p.version, p.enabled))
    }).collect())
}

/// 获取已安装市场列表
fn get_marketplaces_sync(config_dir: &std::path::Path) -> Result<Vec<MarketplaceInfo>> {
    let marketplaces_dir = get_marketplaces_dir(config_dir);

    if !marketplaces_dir.exists() {
        return Ok(vec![]);
    }

    let mut marketplaces = Vec::new();

    let entries = std::fs::read_dir(&marketplaces_dir)
        .map_err(|e| format!("读取市场目录失败: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.is_dir() {
            let config_path = path.join(".claude-plugin").join("marketplace.json");
            if config_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&config_path) {
                    if let Ok(mut info) = serde_json::from_str::<MarketplaceInfo>(&content) {
                        if info.name.is_empty() {
                            info.name = entry.file_name().to_string_lossy().to_string();
                        }
                        marketplaces.push(info);
                    }
                }
            }
        }
    }

    Ok(marketplaces)
}

/// 读取市场中的插件列表
fn read_marketplace_plugins(marketplace_name: &str, config_dir: &std::path::Path) -> Result<Vec<MarketplacePlugin>> {
    let marketplaces_dir = get_marketplaces_dir(config_dir);
    let config_path = marketplaces_dir.join(marketplace_name).join(".claude-plugin").join("marketplace.json");

    if !config_path.exists() {
        return Err(format!("市场配置文件不存在: {}", config_path.display()));
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("读取市场配置失败: {}", e))?;

    #[derive(Deserialize)]
    struct MarketplaceConfig {
        plugins: Vec<PluginEntry>,
    }

    #[derive(Deserialize)]
    struct PluginEntry {
        name: String,
        description: Option<String>,
    }

    let config: MarketplaceConfig = serde_json::from_str(&content)
        .map_err(|e| format!("解析市场配置失败: {}", e))?;

    Ok(config.plugins.into_iter().map(|p| MarketplacePlugin {
        name: p.name,
        version: None,
        description: p.description,
        marketplace_name: marketplace_name.to_string(),
    }).collect())
}

/// 检查市场是否存在
pub fn check_marketplace_exists(name: &str, config_dir: &std::path::Path) -> Result<bool> {
    let marketplaces = get_marketplaces_sync(config_dir)?;
    Ok(marketplaces.iter().any(|m| m.name == name))
}

// ==================== 缓存生成与更新 ====================

/// 生成完整插件缓存
fn generate_cache(config_dir: &std::path::Path, favorite_ids: &HashSet<String>) -> Result<PluginsCache> {
    let installed_map = get_installed_plugins_sync()?;
    let marketplaces = get_marketplaces_sync(config_dir)?;

    let mut installed_plugins = Vec::new();
    let mut not_installed_plugins = Vec::new();

    // 以市场配置为基准遍历
    for market in &marketplaces {
        if let Ok(plugins) = read_marketplace_plugins(&market.name, config_dir) {
            for plugin in plugins {
                let key = format!("{}@{}", plugin.name, plugin.marketplace_name);

                if let Some((version, enabled)) = installed_map.get(&key) {
                    installed_plugins.push(PluginItem {
                        name: plugin.name,
                        version: version.clone(),
                        description: plugin.description,
                        marketplace_name: plugin.marketplace_name,
                        is_installed: true,
                        is_enabled: Some(*enabled),
                        is_favorited: favorite_ids.contains(&key),
                    });
                } else {
                    not_installed_plugins.push(PluginItem {
                        name: plugin.name,
                        version: plugin.version,
                        description: plugin.description,
                        marketplace_name: plugin.marketplace_name,
                        is_installed: false,
                        is_enabled: None,
                        is_favorited: favorite_ids.contains(&key),
                    });
                }
            }
        }
    }

    // 排序
    installed_plugins.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    not_installed_plugins.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    // 已安装的放前面
    installed_plugins.extend(not_installed_plugins);

    Ok(PluginsCache {
        plugins: installed_plugins,
        marketplaces,
        generated_at: chrono::Utc::now().timestamp(),
    })
}

/// 更新缓存中的安装状态（增量更新）
fn update_installed_status(cache: &mut PluginsCache) -> Result<()> {
    let installed_map = get_installed_plugins_sync()?;

    for plugin in &mut cache.plugins {
        let key = format!("{}@{}", plugin.name, plugin.marketplace_name);
        if let Some((version, enabled)) = installed_map.get(&key) {
            plugin.is_installed = true;
            plugin.version = version.clone();
            plugin.is_enabled = Some(*enabled);
        } else {
            plugin.is_installed = false;
            plugin.version = None;
            plugin.is_enabled = None;
        }
    }

    // 重新排序
    cache.plugins.sort_by(|a, b| {
        match (a.is_installed, b.is_installed) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    cache.generated_at = chrono::Utc::now().timestamp();
    Ok(())
}

/// 更新缓存中的收藏状态（增量更新）
fn update_favorite_status(cache: &mut PluginsCache, favorite_ids: &HashSet<String>) {
    for plugin in &mut cache.plugins {
        let key = format!("{}@{}", plugin.name, plugin.marketplace_name);
        plugin.is_favorited = favorite_ids.contains(&key);
    }
    cache.generated_at = chrono::Utc::now().timestamp();
}

// ==================== 公开接口 ====================

/// 获取插件列表（读缓存或生成）
pub fn get_plugins(config_dir: &std::path::Path, favorite_ids: HashSet<String>) -> Result<Vec<PluginItem>> {
    // 尝试读取缓存
    if let Some(mut cache) = read_cache() {
        // 更新收藏状态（可能被其他操作改变）
        update_favorite_status(&mut cache, &favorite_ids);
        return Ok(cache.plugins);
    }

    // 缓存不存在，生成新缓存
    let cache = generate_cache(config_dir, &favorite_ids)?;
    let plugins = cache.plugins.clone();
    write_cache(&cache)?;
    Ok(plugins)
}

/// 获取市场列表
pub fn get_marketplaces(config_dir: &std::path::Path) -> Result<Vec<MarketplaceInfo>> {
    // 尝试从缓存读取
    if let Some(cache) = read_cache() {
        return Ok(cache.marketplaces);
    }

    // 缓存不存在，直接查询
    get_marketplaces_sync(config_dir)
}

/// 刷新插件缓存
pub fn refresh_plugins(config_dir: &std::path::Path, favorite_ids: HashSet<String>) -> Result<Vec<PluginItem>> {
    let cache = generate_cache(config_dir, &favorite_ids)?;
    let plugins = cache.plugins.clone();
    write_cache(&cache)?;
    Ok(plugins)
}

/// 插件操作
pub fn plugin_action(
    action: &str,
    plugin_id: &str,
    config_dir: &std::path::Path,
    favorite_ids: HashSet<String>,
) -> Result<PluginActionResult> {
    // 执行 CLI 命令
    let cli_output = match action {
        "install" => run_claude(&["plugin", "install", plugin_id]),
        "uninstall" => run_claude(&["plugin", "uninstall", plugin_id]),
        "enable" => run_claude(&["plugin", "enable", plugin_id]),
        "disable" => run_claude(&["plugin", "disable", plugin_id]),
        "update" => run_claude(&["plugin", "update", plugin_id]),
        _ => return Err(format!("未知操作: {}", action)),
    }?;

    // 读取或创建缓存
    let mut cache = match read_cache() {
        Some(c) => c,
        None => generate_cache(config_dir, &favorite_ids)?,
    };

    // 更新安装状态
    update_installed_status(&mut cache)?;
    update_favorite_status(&mut cache, &favorite_ids);

    // 保存缓存
    write_cache(&cache)?;

    Ok(PluginActionResult {
        cli_output,
        plugins: cache.plugins,
    })
}

/// 收藏操作（更新缓存）
pub fn favorite_action(
    config_dir: &std::path::Path,
    favorite_ids: HashSet<String>,
) -> Result<PluginActionResult> {
    // 收藏操作不需要执行 CLI，由上层处理数据库操作

    // 读取或创建缓存
    let mut cache = match read_cache() {
        Some(c) => c,
        None => generate_cache(config_dir, &favorite_ids)?,
    };

    // 更新收藏状态
    update_favorite_status(&mut cache, &favorite_ids);

    // 保存缓存
    write_cache(&cache)?;

    Ok(PluginActionResult {
        cli_output: String::new(),
        plugins: cache.plugins,
    })
}

/// 市场操作
pub fn marketplace_action(
    action: &str,
    param: &str,
    config_dir: &std::path::Path,
    favorite_ids: HashSet<String>,
) -> Result<MarketplaceActionResult> {
    // 执行 CLI 命令
    let cli_output = match action {
        "add" => run_claude(&["marketplace", "add", param]),
        "remove" => run_claude(&["marketplace", "remove", param]),
        "update" => run_claude(&["marketplace", "update", param]),
        _ => return Err(format!("未知操作: {}", action)),
    }?;

    // 重新生成完整缓存（市场变化可能影响插件列表）
    let cache = generate_cache(config_dir, &favorite_ids)?;
    write_cache(&cache)?;

    Ok(MarketplaceActionResult {
        cli_output,
        plugins: cache.plugins,
        marketplaces: cache.marketplaces,
    })
}