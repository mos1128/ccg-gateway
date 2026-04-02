use crate::config::get_data_dir;
use crate::db::models::{MarketplaceInfo, MarketplacePlugin, PluginFavoriteItem, PluginItem};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;
use tokio::sync::Mutex;

type Result<T> = std::result::Result<T, String>;

/// 缓存操作互斥锁，防止并发读写竞态
static CACHE_LOCK: Mutex<()> = Mutex::const_new(());

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

/// 收藏插件安装返回结果
#[derive(Debug, Serialize)]
pub struct FavoriteInstallResult {
    pub cli_output: String,
    pub plugins: Vec<PluginItem>,
    pub marketplaces: Vec<MarketplaceInfo>,
}

// ==================== CLI 执行 ====================

/// 执行 claude 命令，返回完整输出
fn run_claude(args: &[&str]) -> Result<String> {
    let args_str = args
        .iter()
        .map(|s| {
            if s.contains(' ') || s.contains('"') {
                format!("\"{}\"", s.replace('"', "\\\""))
            } else {
                s.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

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

/// 写入缓存文件
fn write_cache(cache: &PluginsCache) -> Result<()> {
    let cache_path = get_cache_path();

    // 确保目录存在
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建缓存目录失败: {}", e))?;
    }

    let content =
        serde_json::to_string_pretty(cache).map_err(|e| format!("序列化缓存失败: {}", e))?;
    std::fs::write(&cache_path, content).map_err(|e| format!("写入缓存失败: {}", e))?;

    Ok(())
}

/// 读取缓存，不存在则自动生成（带互斥锁，防止并发竞态）
pub async fn read_cache(
    config_dir: &std::path::Path,
    favorite_ids: &HashSet<String>,
) -> Result<PluginsCache> {
    let _guard = CACHE_LOCK.lock().await;

    // 尝试读取现有缓存
    let cache_path = get_cache_path();
    if cache_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&cache_path) {
            if let Ok(cache) = serde_json::from_str(&content) {
                return Ok(cache);
            }
        }
    }

    // 缓存不存在或损坏，生成并写入
    let cache = generate_cache(config_dir, favorite_ids)?;
    write_cache(&cache)?;
    Ok(cache)
}

/// 强制刷新缓存（带互斥锁）
pub async fn refresh_cache(
    config_dir: &std::path::Path,
    favorite_ids: &HashSet<String>,
) -> Result<PluginsCache> {
    let _guard = CACHE_LOCK.lock().await;

    let cache = generate_cache(config_dir, favorite_ids)?;
    write_cache(&cache)?;
    Ok(cache)
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
fn get_installed_plugins_sync() -> Result<std::collections::HashMap<String, (Option<String>, bool)>>
{
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
        // 优先级：url > repo > path
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
                // 优先级：url > repo > path
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
    // 优先从 known_marketplaces.json 获取安装路径
    let config_path =
        if let Some(install_path) = get_marketplace_install_path(config_dir, marketplace_name) {
            std::path::Path::new(&install_path)
                .join(".claude-plugin")
                .join("marketplace.json")
        } else {
            // 回退到默认路径
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

// ==================== 缓存生成与更新 ====================

/// 生成完整插件缓存
fn generate_cache(
    config_dir: &std::path::Path,
    favorite_ids: &HashSet<String>,
) -> Result<PluginsCache> {
    let installed_map = get_installed_plugins_sync()?;
    let marketplaces = get_marketplaces_from_known_json(config_dir)?;

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
    cache
        .plugins
        .sort_by(|a, b| match (a.is_installed, b.is_installed) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
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

/// 获取收藏列表（独立接口）
pub async fn get_favorites(
    config_dir: &std::path::Path,
    favorites: Vec<(String, String, String, Option<String>)>,
) -> Result<Vec<PluginFavoriteItem>> {
    // 从缓存读取已安装状态
    let cache = read_cache(config_dir, &HashSet::new()).await?;
    let installed_set: HashSet<String> = cache
        .plugins
        .iter()
        .filter(|p| p.is_installed)
        .map(|p| format!("{}@{}", p.name, p.marketplace_name))
        .collect();

    Ok(favorites
        .into_iter()
        .map(
            |(plugin_id, plugin_name, marketplace_name, marketplace_source)| {
                let is_installed = installed_set.contains(&plugin_id);
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

/// 获取插件列表（读缓存或生成）
pub async fn get_plugins(
    config_dir: &std::path::Path,
    favorite_ids: HashSet<String>,
) -> Result<Vec<PluginItem>> {
    let mut cache = read_cache(config_dir, &favorite_ids).await?;
    update_favorite_status(&mut cache, &favorite_ids);
    Ok(cache.plugins)
}

/// 获取市场列表
pub fn get_marketplaces(config_dir: &std::path::Path) -> Result<Vec<MarketplaceInfo>> {
    // 直接从 known_marketplaces.json 查询（确保 source_type 始终是最新的）
    get_marketplaces_from_known_json(config_dir)
}

/// 刷新插件缓存
pub async fn refresh_plugins(
    config_dir: &std::path::Path,
    favorite_ids: HashSet<String>,
) -> Result<Vec<PluginItem>> {
    let cache = refresh_cache(config_dir, &favorite_ids).await?;
    Ok(cache.plugins)
}

/// 插件操作
pub async fn plugin_action(
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

    // 读取缓存并增量更新
    let mut cache = read_cache(config_dir, &favorite_ids).await?;
    update_installed_status(&mut cache)?;
    update_favorite_status(&mut cache, &favorite_ids);

    // 保存缓存
    {
        let _guard = CACHE_LOCK.lock().await;
        write_cache(&cache)?;
    }

    Ok(PluginActionResult {
        cli_output,
        plugins: cache.plugins,
    })
}

/// 更新缓存中的收藏状态并保存
pub async fn update_cache_favorite_status(
    config_dir: &std::path::Path,
    favorite_ids: &HashSet<String>,
) -> Result<Vec<PluginItem>> {
    let mut cache = read_cache(config_dir, favorite_ids).await?;
    update_favorite_status(&mut cache, favorite_ids);

    {
        let _guard = CACHE_LOCK.lock().await;
        write_cache(&cache)?;
    }

    Ok(cache.plugins)
}

/// 市场操作
pub async fn marketplace_action(
    action: &str,
    param: &str,
    config_dir: &std::path::Path,
    favorite_ids: HashSet<String>,
) -> Result<MarketplaceActionResult> {
    // 执行 CLI 命令
    let cli_output = match action {
        "add" => run_claude(&["plugin", "marketplace", "add", param]),
        "remove" => run_claude(&["plugin", "marketplace", "remove", param]),
        "update" => run_claude(&["plugin", "marketplace", "update", param]),
        _ => return Err(format!("未知操作: {}", action)),
    }?;

    // 全量重建缓存
    let cache = refresh_cache(config_dir, &favorite_ids).await?;

    Ok(MarketplaceActionResult {
        cli_output,
        plugins: cache.plugins,
        marketplaces: cache.marketplaces,
    })
}

/// 安装收藏的插件（包含市场检查和安装）
pub async fn install_favorite_plugin(
    plugin_id: &str,
    marketplace_name: &str,
    marketplace_source: Option<&str>,
    config_dir: &std::path::Path,
    favorite_ids: HashSet<String>,
) -> Result<FavoriteInstallResult> {
    let mut cli_outputs = Vec::new();
    let mut marketplaces = get_marketplaces_from_known_json(config_dir)?;

    // 检查市场是否存在
    let marketplace_exists = marketplaces.iter().any(|m| m.name == marketplace_name);

    if !marketplace_exists {
        let source =
            marketplace_source.ok_or("市场不存在且无法获取来源信息，请手动安装市场后再试")?;

        // 判断是否为本地路径（包含 :\ 或 \ 或以字母盘开头）
        let is_local_path = source.contains(":\\")
            || source.contains(':') && source.contains('\\')
            || !source.contains('/');

        if is_local_path {
            return Err("该插件来自本地市场，无法自动恢复".to_string());
        }

        // 安装市场
        let market_output = run_claude(&["plugin", "marketplace", "add", source])?;
        cli_outputs.push(format!("[安装市场] {}", market_output));

        // 刷新市场列表
        marketplaces = get_marketplaces_from_known_json(config_dir)?;
    }

    // 安装插件
    let plugin_output = run_claude(&["plugin", "install", plugin_id])?;
    cli_outputs.push(format!("[安装插件] {}", plugin_output));

    // 全量重建缓存
    let cache = refresh_cache(config_dir, &favorite_ids).await?;

    Ok(FavoriteInstallResult {
        cli_output: cli_outputs.join("\n"),
        plugins: cache.plugins,
        marketplaces,
    })
}
