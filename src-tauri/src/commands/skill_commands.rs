use super::*;

// ==================== Skill 相关命令 ====================

async fn run_skill_blocking<T, F>(task: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    tokio::task::spawn_blocking(task)
        .await
        .map_err(|e| format!("Skill file task failed: {}", e))?
}

async fn load_skill_repos_async() -> Result<Vec<SkillRepo>> {
    run_skill_blocking(skill::load_skill_repos).await
}

async fn upsert_skill_repo_async(repo: SkillRepo) -> Result<()> {
    run_skill_blocking(move || skill::upsert_skill_repo(repo)).await
}

async fn remove_skill_repo_async(name: &str) -> Result<Option<SkillRepo>> {
    let name = name.to_string();
    run_skill_blocking(move || skill::remove_skill_repo(&name)).await
}

async fn get_skill_repo_async(name: &str) -> Result<Option<SkillRepo>> {
    let name = name.to_string();
    run_skill_blocking(move || skill::get_skill_repo(&name)).await
}

async fn load_installed_skill_manifest_async() -> Result<Vec<InstalledSkillManifestEntry>> {
    run_skill_blocking(skill::load_installed_skill_manifest).await
}

async fn upsert_installed_skill_manifest_entry_async(
    entry: InstalledSkillManifestEntry,
) -> Result<()> {
    run_skill_blocking(move || skill::upsert_installed_skill_manifest_entry(entry)).await
}

async fn remove_installed_skill_manifest_entry_async(directory: &str) -> Result<()> {
    let directory = directory.to_string();
    run_skill_blocking(move || skill::remove_installed_skill_manifest_entry(&directory)).await
}

async fn list_installed_skill_directories_async() -> Result<Vec<String>> {
    run_skill_blocking(skill::list_installed_skill_directories).await
}

async fn ensure_repo_exists_async(repo: SkillRepo) -> Result<()> {
    run_skill_blocking(move || skill::ensure_repo_exists(&repo)).await
}

fn get_ssot_dir() -> std::path::PathBuf {
    skill::get_ssot_dir()
}

// 获取 CLI 的 skills 目录（异步版本，支持自定义配置目录）
async fn get_skill_cli_dir_async(db: &SqlitePool, cli_type: &str) -> Option<std::path::PathBuf> {
    let config_dir = get_cli_config_dir_path(db, cli_type).await;
    Some(config_dir.join("skills"))
}

// 检查 skill 是否在 CLI 目录中启用（异步版本）
async fn skill_enabled_in_cli_async(db: &SqlitePool, cli_type: &str, directory: &str) -> bool {
    let cli_dir = match get_skill_cli_dir_async(db, cli_type).await {
        Some(d) => d,
        None => return false,
    };

    let skill_path = cli_dir.join(directory);
    tokio::fs::metadata(skill_path).await.is_ok()
}

async fn build_skill_cli_flags(db: &SqlitePool, directory: &str) -> Vec<SkillCliFlag> {
    vec![
        SkillCliFlag {
            cli_type: "claude_code".to_string(),
            enabled: skill_enabled_in_cli_async(db, "claude_code", directory).await,
        },
        SkillCliFlag {
            cli_type: "codex".to_string(),
            enabled: skill_enabled_in_cli_async(db, "codex", directory).await,
        },
        SkillCliFlag {
            cli_type: "gemini".to_string(),
            enabled: skill_enabled_in_cli_async(db, "gemini", directory).await,
        },
    ]
}

// 解析 SKILL.md frontmatter
fn parse_skill_metadata(content: &str) -> (Option<String>, Option<String>) {
    let content = content.trim_start_matches('\u{feff}');
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return (None, None);
    }
    let front_matter = parts[1].trim();
    let mut name = None;
    let mut description = None;
    for line in front_matter.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("name:") {
            name = Some(val.trim().to_string());
        } else if let Some(val) = line.strip_prefix("description:") {
            description = Some(val.trim().to_string());
        }
    }
    (name, description)
}

// 递归复制目录
fn copy_dir_recursive(src: &std::path::Path, dest: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            std::fs::copy(&path, &dest_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn normalize_skill_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn normalize_optional_text(text: Option<String>) -> Option<String> {
    text.and_then(|value| normalize_skill_text(&value))
}

fn skill_install_directory_name_from_parts(directory: &str, repo_name: &str) -> String {
    if directory == "." {
        repo_name.to_string()
    } else {
        std::path::Path::new(directory)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| directory.to_string())
    }
}

fn skill_install_directory_name(skill_item: &DiscoverableSkill) -> String {
    skill_install_directory_name_from_parts(&skill_item.directory, &skill_item.repo.name)
}

async fn read_installed_skill_metadata_async(directory: &str) -> (Option<String>, Option<String>) {
    let skill_md_path = get_ssot_dir().join(directory).join("SKILL.md");
    let content = match tokio::fs::read_to_string(skill_md_path).await {
        Ok(content) => content,
        Err(_) => return (None, None),
    };

    parse_skill_metadata(&content)
}

async fn file_modified_at_async(path: &std::path::Path) -> i64 {
    tokio::fs::metadata(path)
        .await
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_else(now_timestamp)
}

// 同步 skill 到 CLI 目录（异步版本）
async fn sync_skill_to_cli_async(db: &SqlitePool, directory: &str, cli_type: &str) -> Result<()> {
    let ssot_dir = get_ssot_dir();
    let source = ssot_dir.join(directory);
    let cli_dir = match get_skill_cli_dir_async(db, cli_type).await {
        Some(d) => d,
        None => return Err(format!("Unsupported CLI type: {}", cli_type)),
    };

    let dest = cli_dir.join(directory);
    run_skill_blocking(move || {
        if !source.exists() {
            return Err(format!("Skill directory not found: {}", source.display()));
        }
        copy_dir_recursive(&source, &dest)
    })
    .await?;
    tracing::info!("Synced skill {} to {}", directory, cli_type);
    Ok(())
}

// 从 CLI 目录移除 skill（异步版本）
async fn remove_skill_from_cli_async(
    db: &SqlitePool,
    directory: &str,
    cli_type: &str,
) -> Result<()> {
    let cli_dir = match get_skill_cli_dir_async(db, cli_type).await {
        Some(d) => d,
        None => return Ok(()),
    };
    let skill_folder = cli_dir.join(directory);
    let removed = run_skill_blocking(move || {
        if skill_folder.exists() {
            std::fs::remove_dir_all(&skill_folder).map_err(|e| e.to_string())?;
            Ok(true)
        } else {
            Ok(false)
        }
    })
    .await?;

    if removed {
        tracing::info!("Removed skill {} from {}", directory, cli_type);
    }
    Ok(())
}

// 从所有 CLI 目录移除 skill（异步版本）
async fn remove_skill_from_all_cli_async(db: &SqlitePool, directory: &str) -> Result<()> {
    for cli_type in CliType::ALL.iter().map(CliType::as_str) {
        remove_skill_from_cli_async(db, directory, cli_type).await?;
    }
    Ok(())
}

// 批量设置 CLI 启用状态（内部方法）
async fn batch_set_skill_cli(db: &SqlitePool, directory: &str, cli_types: &[String]) -> Result<()> {
    // 先从所有 CLI 移除
    remove_skill_from_all_cli_async(db, directory).await?;

    // 再启用指定的 CLI
    for cli_type in cli_types {
        sync_skill_to_cli_async(db, directory, cli_type).await?;
    }
    Ok(())
}

// 检测技能在各 CLI 的启用状态（遍历文件系统）
async fn detect_skill_cli_status(db: &SqlitePool, directory: &str) -> Vec<String> {
    let mut enabled_clis = Vec::new();
    for cli_type in CliType::ALL.iter().map(CliType::as_str) {
        if skill_enabled_in_cli_async(db, cli_type, directory).await {
            enabled_clis.push(cli_type.to_string());
        }
    }
    enabled_clis
}

async fn uninstall_skill_directory_async(
    db: &SqlitePool,
    directory: &str,
    error_if_missing: bool,
) -> Result<()> {
    let ssot_dir = get_ssot_dir();
    let skill_path = ssot_dir.join(directory);
    let manifest_exists = load_installed_skill_manifest_async()
        .await?
        .iter()
        .any(|entry| entry.directory == directory);
    let skill_exists = tokio::fs::metadata(&skill_path).await.is_ok();

    if !manifest_exists && !skill_exists {
        if error_if_missing {
            return Err("Skill not found".to_string());
        }
        return Ok(());
    }

    remove_skill_from_all_cli_async(db, directory).await?;

    if skill_exists {
        run_skill_blocking(move || std::fs::remove_dir_all(&skill_path).map_err(|e| e.to_string()))
            .await?;
    }

    if manifest_exists {
        remove_installed_skill_manifest_entry_async(directory).await?;
    }

    tracing::info!("Uninstalled skill: {}", directory);
    Ok(())
}

async fn load_installed_skill_responses(db: &SqlitePool) -> Result<Vec<InstalledSkillResponse>> {
    let favorite_keys = get_skill_favorite_keys(db).await?;

    let manifest_entries = load_installed_skill_manifest_async().await?;
    let mut manifest_map = manifest_entries
        .into_iter()
        .map(|entry| (entry.directory.clone(), entry))
        .collect::<HashMap<String, InstalledSkillManifestEntry>>();

    let ssot_dir = get_ssot_dir();
    let mut results = Vec::new();

    for directory in list_installed_skill_directories_async().await? {
        let mut entry = match manifest_map.remove(&directory) {
            Some(entry) => entry,
            None => InstalledSkillManifestEntry {
                directory: directory.clone(),
                name: directory.clone(),
                description: None,
                repo: None,
                readme_url: None,
                installed_at: file_modified_at_async(&ssot_dir.join(&directory)).await,
                source_directory: None,
            },
        };

        let (disk_name, disk_description) = read_installed_skill_metadata_async(&directory).await;
        if let Some(name) = normalize_optional_text(disk_name) {
            entry.name = name;
        }
        if let Some(description) = normalize_optional_text(disk_description) {
            entry.description = Some(description);
        } else {
            entry.description = normalize_optional_text(entry.description);
        }

        let (is_favorited, can_favorite, favorite_key, market_display) =
            build_skill_favorite_info(&entry, &favorite_keys);
        let cli_flags = build_skill_cli_flags(db, &directory).await;
        results.push(InstalledSkillResponse {
            id: directory.clone(),
            name: entry.name,
            description: entry.description,
            directory,
            repo: entry.repo,
            readme_url: entry.readme_url,
            installed_at: entry.installed_at,
            cli_flags,
            exists_on_disk: true,
            is_favorited,
            can_favorite,
            favorite_key,
            market_display,
        });
    }

    for (directory, mut entry) in manifest_map {
        entry.description = normalize_optional_text(entry.description);
        let (is_favorited, can_favorite, favorite_key, market_display) =
            build_skill_favorite_info(&entry, &favorite_keys);
        let cli_flags = build_skill_cli_flags(db, &directory).await;
        results.push(InstalledSkillResponse {
            id: directory.clone(),
            name: entry.name,
            description: entry.description,
            directory,
            repo: entry.repo,
            readme_url: entry.readme_url,
            installed_at: entry.installed_at,
            cli_flags,
            exists_on_disk: false,
            is_favorited,
            can_favorite,
            favorite_key,
            market_display,
        });
    }

    results.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(results)
}

fn build_skill_favorite_info(
    entry: &InstalledSkillManifestEntry,
    favorite_keys: &std::collections::HashSet<String>,
) -> (bool, bool, Option<String>, String) {
    if let (Some(repo), Some(source_dir)) = (&entry.repo, &entry.source_directory) {
        let key = format!("{}:{}", repo.name, source_dir);
        let is_favorited = favorite_keys.contains(&key);
        let market_display = if is_local_repo_source(&repo.source) {
            String::new()
        } else {
            format!("@{}", repo.source)
        };
        (is_favorited, true, Some(key), market_display)
    } else if let Some(repo) = &entry.repo {
        // 本地仓库且未保存 source_directory 的旧数据
        if is_local_repo_source(&repo.source) {
            let source_dir = if entry.directory == repo.name {
                "."
            } else {
                &entry.directory
            };
            let key = format!("{}:{}", repo.name, source_dir);
            let is_favorited = favorite_keys.contains(&key);
            (is_favorited, true, Some(key), String::new())
        } else {
            (false, false, None, String::new())
        }
    } else {
        (false, false, None, String::new())
    }
}

// ==================== 仓库管理命令 ====================

#[tauri::command]
pub async fn get_skill_repos() -> Result<Vec<SkillRepo>> {
    load_skill_repos_async().await
}

/// 从 source 提取仓库名称
fn extract_repo_name(source: &str) -> String {
    let source = source.trim().strip_suffix(".git").unwrap_or(source.trim());

    // 本地路径：取最后一段
    if source.contains(':') && source.contains('\\') || source.starts_with('/') {
        return std::path::Path::new(source)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or(source.to_string());
    }

    // URL：取最后一段路径
    source
        .split('/')
        .filter(|s| !s.is_empty())
        .last()
        .unwrap_or(source)
        .to_string()
}

/// 执行 git clone（浅克隆，自动使用主分支）
async fn git_clone_repo(source: &str) -> Result<std::path::PathBuf> {
    let cache_dir = skill::get_cached_repo_dir(source);

    // 如果已存在，直接返回
    if tokio::fs::metadata(&cache_dir).await.is_ok() {
        return Ok(cache_dir);
    }

    // 补全 URL：owner/repo 格式转为 https://github.com/owner/repo
    let git_url = if source.contains("://") || source.contains("git@") {
        source.to_string()
    } else if source.split('/').filter(|s| !s.is_empty()).count() == 2 {
        format!("https://github.com/{}", source)
    } else {
        source.to_string()
    };

    let output = tokio::process::Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            &git_url,
            cache_dir.to_str().unwrap_or(""),
        ])
        .output()
        .await
        .map_err(|e| format!("git clone 执行失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git clone 失败: {}", stderr));
    }

    tracing::info!("git clone 成功: {} -> {}", git_url, cache_dir.display());
    Ok(cache_dir)
}

async fn delete_cached_repo_dir_async(source: &str) -> Result<()> {
    let source = source.to_string();
    run_skill_blocking(move || {
        skill::delete_cached_repo_dir(&source);
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn add_skill_repo(input: SkillRepoCreate) -> Result<SkillRepo> {
    let url = input.url.trim();

    // 1. 本地目录：直接存储
    let path = std::path::Path::new(url);
    if skill::is_local_repo_source(url)
        && tokio::fs::metadata(path)
            .await
            .map(|metadata| metadata.is_dir())
            .unwrap_or(false)
    {
        let repo = SkillRepo {
            name: extract_repo_name(url),
            source: url.to_string(),
        };
        upsert_skill_repo_async(repo.clone()).await?;
        return Ok(repo);
    }

    // 2. 远程仓库：尝试 git clone 验证
    git_clone_repo(url).await?;

    let repo = SkillRepo {
        name: extract_repo_name(url),
        source: url.to_string(),
    };
    upsert_skill_repo_async(repo.clone()).await?;
    Ok(repo)
}

#[tauri::command]
pub async fn remove_skill_repo(db: State<'_, SqlitePool>, name: String) -> Result<()> {
    let installed_directories = load_installed_skill_manifest_async()
        .await?
        .into_iter()
        .filter(|entry| entry.repo.as_ref().map(|repo| repo.name.as_str()) == Some(name.as_str()))
        .map(|entry| entry.directory)
        .collect::<Vec<_>>();

    for directory in installed_directories {
        uninstall_skill_directory_async(db.inner(), &directory, false).await?;
    }

    if let Some(repo) = remove_skill_repo_async(&name).await? {
        if !skill::is_local_repo_source(&repo.source) {
            delete_cached_repo_dir_async(&repo.source).await?;
        }
    }
    Ok(())
}

// ==================== Skill 发现命令 ====================

#[tauri::command]
pub async fn discover_repo_skills(
    db: State<'_, SqlitePool>,
    name: String,
) -> Result<Vec<DiscoverableSkill>> {
    let repo = get_skill_repo_async(&name)
        .await?
        .ok_or_else(|| format!("未找到仓库 '{}'", name))?;

    let favorite_keys = get_skill_favorite_keys(db.inner()).await?;

    // 1. 本地目录
    if skill::is_local_repo_source(&repo.source) {
        return scan_local_repo_skills(&repo, &favorite_keys).await;
    }

    // 2. 远程仓库：git clone 或使用缓存
    let cache_dir = git_clone_repo(&repo.source).await?;
    scan_cached_repo_skills_async(cache_dir, repo, favorite_keys).await
}

#[tauri::command]
pub async fn reinstall_skill_repo(
    db: State<'_, SqlitePool>,
    name: String,
) -> Result<Vec<DiscoverableSkill>> {
    let repo = get_skill_repo_async(&name)
        .await?
        .ok_or_else(|| format!("未找到仓库 '{}'", name))?;

    let favorite_keys = get_skill_favorite_keys(db.inner()).await?;

    // 1. 本地目录
    if skill::is_local_repo_source(&repo.source) {
        return scan_local_repo_skills(&repo, &favorite_keys).await;
    }

    // 2. 远程仓库：删除缓存后重新 clone
    delete_cached_repo_dir_async(&repo.source).await?;
    let cache_dir = git_clone_repo(&repo.source).await?;
    scan_cached_repo_skills_async(cache_dir, repo, favorite_keys).await
}

async fn get_skill_favorite_keys(db: &SqlitePool) -> Result<std::collections::HashSet<String>> {
    let keys: Vec<String> = sqlx::query("SELECT skill_key FROM skill_favorites")
        .map(|row: sqlx::sqlite::SqliteRow| row.get::<String, _>(0))
        .fetch_all(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(keys.into_iter().collect())
}

/// 扫描缓存的仓库目录
fn scan_cached_repo_skills(
    cache_dir: &std::path::Path,
    repo: &SkillRepo,
    favorite_keys: &std::collections::HashSet<String>,
) -> Result<Vec<DiscoverableSkill>> {
    let mut skills = Vec::new();

    for entry in walkdir::WalkDir::new(cache_dir)
        .max_depth(5)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_name() == "SKILL.md" {
            let file_path = entry.path();
            let parent_dir = file_path.parent().unwrap_or(cache_dir);

            let relative_path = parent_dir
                .strip_prefix(cache_dir)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            let content = std::fs::read_to_string(file_path).map_err(|e| e.to_string())?;
            let (skill_name, description) = parse_skill_metadata(&content);

            let directory_name = if relative_path.is_empty() {
                cache_dir
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or("repo".to_string())
            } else {
                parent_dir
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| relative_path.clone())
            };

            let directory_str = if relative_path.is_empty() {
                ".".to_string()
            } else {
                relative_path
            };
            let install_dir = skill_install_directory_name_from_parts(&directory_str, &repo.name);
            let key = format!("{}:{}", repo.name, &directory_str);
            let is_installed = get_ssot_dir().join(&install_dir).exists();
            skills.push(DiscoverableSkill {
                key: key.clone(),
                name: skill_name.unwrap_or_else(|| directory_name.clone()),
                description: description.unwrap_or_default(),
                directory: directory_str,
                install_directory: install_dir,
                readme_url: None,
                repo: repo.clone(),
                is_favorited: favorite_keys.contains(&key),
                is_installed,
            });
        }
    }

    // 排序：收藏优先，已安装次之，最后按名称
    skills.sort_by(|a, b| {
        if a.is_favorited != b.is_favorited {
            return a.is_favorited.cmp(&b.is_favorited).reverse();
        }
        if a.is_installed != b.is_installed {
            return a.is_installed.cmp(&b.is_installed).reverse();
        }
        a.name.to_lowercase().cmp(&b.name.to_lowercase())
    });
    Ok(skills)
}

async fn scan_cached_repo_skills_async(
    cache_dir: std::path::PathBuf,
    repo: SkillRepo,
    favorite_keys: std::collections::HashSet<String>,
) -> Result<Vec<DiscoverableSkill>> {
    run_skill_blocking(move || scan_cached_repo_skills(&cache_dir, &repo, &favorite_keys)).await
}

/// 扫描本地仓库目录
async fn scan_local_repo_skills(
    repo: &SkillRepo,
    favorite_keys: &std::collections::HashSet<String>,
) -> Result<Vec<DiscoverableSkill>> {
    let root_path = std::path::Path::new(&repo.source);

    if !tokio::fs::metadata(root_path)
        .await
        .map(|metadata| metadata.is_dir())
        .unwrap_or(false)
    {
        return Err(format!("本地目录 {} 不存在", repo.source));
    }

    scan_cached_repo_skills_async(root_path.to_path_buf(), repo.clone(), favorite_keys.clone())
        .await
}

// ==================== Skill 安装/卸载命令 ====================

async fn install_skill_inner(
    db: &SqlitePool,
    skill_item: DiscoverableSkill,
    reinstall: bool,
) -> Result<InstalledSkillResponse> {
    let ssot_dir = get_ssot_dir();
    let directory_name = skill_install_directory_name(&skill_item);
    let skill_path = ssot_dir.join(&directory_name);
    let existing = load_installed_skill_manifest_async()
        .await?
        .into_iter()
        .find(|entry| entry.directory == directory_name);

    ensure_repo_exists_async(skill_item.repo.clone()).await?;

    let skill_exists = tokio::fs::metadata(&skill_path).await.is_ok();
    if (existing.is_some() || skill_exists) && !reinstall {
        return Err(format!("Skill '{}' is already installed", directory_name));
    }

    // 如果是重装，先删除旧的 SSOT 目录
    if reinstall && skill_exists {
        let path = skill_path.clone();
        run_skill_blocking(move || {
            let _ = std::fs::remove_dir_all(&path);
            Ok(())
        })
        .await?;
    }

    // 根据类型进行安装
    let skill_source_path = if skill::is_local_repo_source(&skill_item.repo.source) {
        let source_path = std::path::Path::new(&skill_item.repo.source);
        if skill_item.directory == "." {
            source_path.to_path_buf()
        } else {
            source_path.join(&skill_item.directory)
        }
    } else {
        // 从 git clone 缓存目录复制
        let cache_dir = git_clone_repo(&skill_item.repo.source).await?;
        if skill_item.directory == "." {
            cache_dir.to_path_buf()
        } else {
            cache_dir.join(&skill_item.directory)
        }
    };

    let dest_path = ssot_dir.join(&directory_name);
    run_skill_blocking(move || {
        if !skill_source_path.exists() {
            return Err(format!("技能目录不存在: {}", skill_source_path.display()));
        }
        copy_dir_recursive(&skill_source_path, &dest_path)
    })
    .await?;

    let now = now_timestamp();
    upsert_installed_skill_manifest_entry_async(InstalledSkillManifestEntry {
        directory: directory_name.clone(),
        name: directory_name.clone(),
        description: None,
        repo: Some(skill_item.repo.clone()),
        readme_url: None,
        installed_at: now,
        source_directory: Some(skill_item.directory.clone()),
    })
    .await?;

    // 不触发 CLI 同步，状态实时检测
    let cli_flags = build_skill_cli_flags(db, &directory_name).await;

    Ok(InstalledSkillResponse {
        id: directory_name.clone(),
        name: skill_item.name,
        description: normalize_skill_text(&skill_item.description),
        directory: directory_name,
        repo: Some(skill_item.repo.clone()),
        readme_url: skill_item.readme_url,
        installed_at: now,
        cli_flags,
        exists_on_disk: true,
        is_favorited: skill_item.is_favorited,
        can_favorite: true,
        favorite_key: Some(skill_item.key.clone()),
        market_display: if is_local_repo_source(&skill_item.repo.source) {
            String::new()
        } else {
            format!("@{}", skill_item.repo.source)
        },
    })
}

#[tauri::command]
pub async fn install_skill(
    db: State<'_, SqlitePool>,
    skill: DiscoverableSkill,
    reinstall: Option<bool>,
) -> Result<InstalledSkillResponse> {
    install_skill_inner(db.inner(), skill, reinstall.unwrap_or(false)).await
}

#[tauri::command]
async fn reinstall_skill_impl(
    db: &SqlitePool,
    directory: String,
) -> Result<InstalledSkillResponse> {
    // 1. 检测当前 CLI 启用状态
    let enabled_clis = detect_skill_cli_status(db, &directory).await;

    // 2. 从 manifest 获取信息
    let manifest_entries = load_installed_skill_manifest_async().await?;
    let entry = manifest_entries
        .iter()
        .find(|e| e.directory == directory)
        .ok_or_else(|| format!("Skill '{}' not found in manifest", directory))?;

    let repo = entry
        .repo
        .as_ref()
        .ok_or_else(|| "Skill missing repo info, cannot reinstall".to_string())?;

    let source_dir = entry
        .source_directory
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or_else(|| {
            if skill::is_local_repo_source(&repo.source) && directory == repo.name {
                "."
            } else {
                &directory
            }
        });

    // 3. 从仓库源复制到临时目录（避免中间状态暴露）
    let ssot_dir = get_ssot_dir();
    let skill_path = ssot_dir.join(&directory);
    let temp_path = ssot_dir.join(format!(".temp_{}_{}", directory, uuid::Uuid::new_v4()));
    let backup_path = ssot_dir.join(format!(".backup_{}_{}", directory, uuid::Uuid::new_v4()));

    let skill_source_path = if skill::is_local_repo_source(&repo.source) {
        let base = std::path::Path::new(&repo.source);
        if source_dir == "." {
            base.to_path_buf()
        } else {
            base.join(source_dir)
        }
    } else {
        let cache_dir = git_clone_repo(&repo.source).await?;
        if source_dir == "." {
            cache_dir.to_path_buf()
        } else {
            cache_dir.join(source_dir)
        }
    };

    run_skill_blocking(move || {
        if !skill_source_path.exists() {
            return Err(format!("技能目录不存在: {}", skill_source_path.display()));
        }

        if let Err(e) = copy_dir_recursive(&skill_source_path, &temp_path) {
            let _ = std::fs::remove_dir_all(&temp_path);
            return Err(e);
        }

        let had_existing = skill_path.exists();
        if had_existing {
            if let Err(e) = std::fs::rename(&skill_path, &backup_path) {
                let _ = std::fs::remove_dir_all(&temp_path);
                return Err(format!("Skill 备份失败: {}", e));
            }
        }

        if let Err(e) = std::fs::rename(&temp_path, &skill_path) {
            let _ = std::fs::remove_dir_all(&temp_path);
            if had_existing {
                if let Err(restore_err) = std::fs::rename(&backup_path, &skill_path) {
                    return Err(format!(
                        "Skill 替换失败: {}; 旧版本恢复失败: {}",
                        e, restore_err
                    ));
                }
            }
            return Err(format!("Skill 替换失败: {}", e));
        }

        if had_existing {
            if let Err(e) = std::fs::remove_dir_all(&backup_path) {
                tracing::warn!(
                    error = %e,
                    path = %backup_path.display(),
                    "Failed to remove skill backup"
                );
            }
        }

        Ok(())
    })
    .await?;

    // 5. 恢复 CLI 启用状态
    batch_set_skill_cli(db, &directory, &enabled_clis).await?;

    // 6. 返回结果
    let cli_flags = build_skill_cli_flags(db, &directory).await;
    let (disk_name, disk_description) = read_installed_skill_metadata_async(&directory).await;
    let key = format!("{}:{}", repo.name, source_dir);

    Ok(InstalledSkillResponse {
        id: directory.clone(),
        name: disk_name.unwrap_or_else(|| entry.name.clone()),
        description: normalize_skill_text(&disk_description.unwrap_or_default()),
        directory,
        repo: Some(repo.clone()),
        readme_url: None,
        installed_at: entry.installed_at,
        cli_flags,
        exists_on_disk: true,
        is_favorited: false,
        can_favorite: true,
        favorite_key: Some(key),
        market_display: if is_local_repo_source(&repo.source) {
            String::new()
        } else {
            format!("@{}", repo.source)
        },
    })
}

#[tauri::command]
pub async fn reinstall_skill(
    db: State<'_, SqlitePool>,
    directory: String,
) -> Result<InstalledSkillResponse> {
    reinstall_skill_impl(db.inner(), directory).await
}

#[tauri::command]
pub async fn uninstall_skill(db: State<'_, SqlitePool>, id: String) -> Result<()> {
    uninstall_skill_directory_async(db.inner(), &id, true).await
}

// ==================== 已安装 Skill 管理命令 ====================

#[tauri::command]
pub async fn get_installed_skills(
    db: State<'_, SqlitePool>,
) -> Result<Vec<InstalledSkillResponse>> {
    load_installed_skill_responses(db.inner()).await
}

#[tauri::command]
pub async fn toggle_skill_cli(
    db: State<'_, SqlitePool>,
    id: String,
    cli_type: String,
    enabled: bool,
) -> Result<()> {
    let directory = id;
    if enabled {
        sync_skill_to_cli_async(db.inner(), &directory, &cli_type).await?;
    } else {
        remove_skill_from_cli_async(db.inner(), &directory, &cli_type).await?;
    }

    Ok(())
}

// ==================== Skill 收藏命令 ====================

#[tauri::command]
pub async fn get_skill_favorites(db: State<'_, SqlitePool>) -> Result<Vec<SkillFavoriteItem>> {
    let favorites = sqlx::query_as::<_, SkillFavorite>(
        "SELECT * FROM skill_favorites ORDER BY created_at DESC",
    )
    .fetch_all(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    let ssot_dir = get_ssot_dir();
    let mut items = Vec::with_capacity(favorites.len());
    for favorite in favorites {
        let repo = SkillRepo {
            name: favorite.repo_name,
            source: favorite.repo_source,
        };
        let installed_directory =
            skill_install_directory_name_from_parts(&favorite.directory, &repo.name);
        let is_installed = tokio::fs::metadata(ssot_dir.join(installed_directory))
            .await
            .is_ok();
        items.push(SkillFavoriteItem {
            key: favorite.skill_key,
            name: favorite.name,
            description: favorite.description,
            directory: favorite.directory,
            readme_url: favorite.readme_url,
            repo,
            is_installed,
        });
    }
    Ok(items)
}

#[tauri::command]
pub async fn add_skill_favorite(
    db: State<'_, SqlitePool>,
    skill_item: DiscoverableSkill,
) -> Result<()> {
    let now = now_timestamp();
    sqlx::query(
        "INSERT OR REPLACE INTO skill_favorites (skill_key, name, description, directory, readme_url, repo_name, repo_source, repo_branch, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&skill_item.key)
    .bind(&skill_item.name)
    .bind(normalize_skill_text(&skill_item.description))
    .bind(&skill_item.directory)
    .bind(&skill_item.readme_url)
    .bind(&skill_item.repo.name)
    .bind(&skill_item.repo.source)
    .bind(None::<String>)  // repo_branch 不再使用
    .bind(now)
    .execute(db.inner())
    .await
    .map_err(map_db_error)?;
    Ok(())
}

#[tauri::command]
pub async fn toggle_installed_skill_favorite(
    db: State<'_, SqlitePool>,
    directory: String,
) -> Result<bool> {
    let manifest_entries = load_installed_skill_manifest_async().await?;
    let entry = manifest_entries
        .iter()
        .find(|e| e.directory == directory)
        .ok_or_else(|| format!("Skill '{}' not found in manifest", directory))?;

    let (repo, source_dir) = match (&entry.repo, &entry.source_directory) {
        (Some(r), Some(s)) => (r, s),
        (Some(r), None) => {
            // 旧数据，尝试推断 source_directory
            if is_local_repo_source(&r.source) && entry.directory == r.name {
                static DOT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
                (r, DOT.get_or_init(|| ".".to_string()))
            } else {
                return Err("Skill missing source_directory info".to_string());
            }
        }
        _ => return Err("Skill missing repo info".to_string()),
    };

    let key = format!("{}:{}", repo.name, source_dir);

    // 检查是否已收藏
    let existing = sqlx::query("SELECT 1 FROM skill_favorites WHERE skill_key = ?")
        .bind(&key)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    if existing.is_some() {
        // 已收藏，删除
        sqlx::query("DELETE FROM skill_favorites WHERE skill_key = ?")
            .bind(&key)
            .execute(db.inner())
            .await
            .map_err(map_db_error)?;
        Ok(false)
    } else {
        // 未收藏，添加
        let now = now_timestamp();
        sqlx::query(
            "INSERT INTO skill_favorites (skill_key, name, description, directory, readme_url, repo_name, repo_source, repo_branch, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&key)
        .bind(&entry.name)
        .bind(&entry.description)
        .bind(source_dir)
        .bind(&entry.readme_url)
        .bind(&repo.name)
        .bind(&repo.source)
        .bind(None::<String>)
        .bind(now)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;
        Ok(true)
    }
}

#[tauri::command]
pub async fn remove_skill_favorite(db: State<'_, SqlitePool>, key: String) -> Result<()> {
    sqlx::query("DELETE FROM skill_favorites WHERE skill_key = ?")
        .bind(&key)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;
    Ok(())
}

#[tauri::command]
pub async fn install_favorite_skill(
    db: State<'_, SqlitePool>,
    key: String,
) -> Result<InstalledSkillResponse> {
    let favorite =
        sqlx::query_as::<_, SkillFavorite>("SELECT * FROM skill_favorites WHERE skill_key = ?")
            .bind(&key)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Skill favorite not found".to_string())?;

    let repo = SkillRepo {
        name: favorite.repo_name.clone(),
        source: favorite.repo_source.clone(),
    };

    // 检查仓库是否已安装，未安装则静默安装
    let existing_repo = get_skill_repo_async(&repo.name).await?;
    if existing_repo.is_none() {
        // 静默安装仓库
        if skill::is_local_repo_source(&repo.source) {
            let path = std::path::Path::new(&repo.source);
            if !tokio::fs::metadata(path)
                .await
                .map(|metadata| metadata.is_dir())
                .unwrap_or(false)
            {
                return Err(format!("本地目录 {} 不存在", repo.source));
            }
            upsert_skill_repo_async(repo.clone()).await?;
        } else {
            git_clone_repo(&repo.source).await?;
            upsert_skill_repo_async(repo.clone()).await?;
        }
    }

    // 确保仓库缓存存在
    let cache_dir = if skill::is_local_repo_source(&repo.source) {
        std::path::Path::new(&repo.source).to_path_buf()
    } else {
        git_clone_repo(&repo.source).await?
    };

    // 检查 directory 是否有效，无效则从仓库扫描修复
    let skill_path = if favorite.directory == "." {
        cache_dir.to_path_buf()
    } else {
        cache_dir.join(&favorite.directory)
    };

    let (directory, skill_key) = if tokio::fs::metadata(&skill_path).await.is_ok() {
        (favorite.directory.clone(), favorite.skill_key.clone())
    } else {
        // 扫描仓库找到正确的 skill
        let skills =
            scan_cached_repo_skills_async(cache_dir.clone(), repo.clone(), Default::default())
                .await?;
        let skill = skills
            .iter()
            .find(|s| s.name == favorite.name)
            .ok_or_else(|| format!("未在仓库中找到技能: {}", favorite.name))?;

        // 更新数据库中的 directory 和 key
        sqlx::query("UPDATE skill_favorites SET directory = ?, skill_key = ? WHERE skill_key = ?")
            .bind(&skill.directory)
            .bind(&skill.key)
            .bind(&favorite.skill_key)
            .execute(db.inner())
            .await
            .map_err(map_db_error)?;

        (skill.directory.clone(), skill.key.clone())
    };

    install_skill_inner(
        db.inner(),
        DiscoverableSkill {
            key: skill_key,
            name: favorite.name,
            description: favorite.description.unwrap_or_default(),
            directory: directory.clone(),
            install_directory: skill_install_directory_name_from_parts(
                &directory,
                &favorite.repo_name,
            ),
            readme_url: favorite.readme_url,
            repo,
            is_favorited: true,
            is_installed: false,
        },
        false,
    )
    .await
}

#[tauri::command]
pub async fn reinstall_favorite_skill(
    db: State<'_, SqlitePool>,
    key: String,
) -> Result<InstalledSkillResponse> {
    let favorite =
        sqlx::query_as::<_, SkillFavorite>("SELECT * FROM skill_favorites WHERE skill_key = ?")
            .bind(&key)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Skill favorite not found".to_string())?;

    // 计算安装目录
    let directory =
        skill_install_directory_name_from_parts(&favorite.directory, &favorite.repo_name);

    // 检查是否已安装
    let ssot_dir = get_ssot_dir();
    if tokio::fs::metadata(ssot_dir.join(&directory))
        .await
        .is_err()
    {
        return Err(format!(
            "Skill '{}' is not installed, cannot reinstall",
            directory
        ));
    }

    reinstall_skill_impl(db.inner(), directory).await
}
