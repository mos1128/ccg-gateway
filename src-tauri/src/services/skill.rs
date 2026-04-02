use crate::config::get_data_dir;
use crate::db::models::SkillRepo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, String>;

const DEFAULT_SKILL_REPOS_JSON: &str = "repos.json";
const DEFAULT_SKILL_CACHE_DIR: &str = "cache";
const INSTALLED_SKILL_MANIFEST_JSON: &str = "manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledSkillManifestEntry {
    pub directory: String,
    pub name: String,
    pub description: Option<String>,
    pub repo: Option<SkillRepo>,
    pub readme_url: Option<String>,
    pub installed_at: i64,
}

fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let content = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(path, content).map_err(|e| e.to_string())
}

fn read_json_or_default<T>(path: &Path) -> Result<T>
where
    T: for<'de> Deserialize<'de> + Default,
{
    if !path.exists() {
        return Ok(T::default());
    }

    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    if content.trim().is_empty() {
        return Ok(T::default());
    }

    serde_json::from_str(&content).map_err(|e| e.to_string())
}

pub fn get_ssot_dir() -> PathBuf {
    let dir = get_data_dir().join("skills");
    std::fs::create_dir_all(&dir).ok();
    dir
}

pub fn get_skill_repo_dir() -> PathBuf {
    let dir = get_data_dir().join("skill_repo");
    std::fs::create_dir_all(&dir).ok();
    dir
}

pub fn get_skill_cache_dir() -> PathBuf {
    let dir = get_skill_repo_dir().join(DEFAULT_SKILL_CACHE_DIR);
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn get_skill_repos_path() -> PathBuf {
    get_skill_repo_dir().join(DEFAULT_SKILL_REPOS_JSON)
}

fn get_installed_skill_manifest_path() -> PathBuf {
    get_ssot_dir().join(INSTALLED_SKILL_MANIFEST_JSON)
}

fn default_skill_repos() -> Vec<SkillRepo> {
    vec![
        SkillRepo {
            name: "skills".to_string(),
            source: "anthropics/skills".to_string(),
            branch: Some("main".to_string()),
        },
        SkillRepo {
            name: "awesome-claude-skills".to_string(),
            source: "ComposioHQ/awesome-claude-skills".to_string(),
            branch: Some("master".to_string()),
        },
    ]
}

pub fn ensure_default_skill_repos() -> Result<()> {
    let path = get_skill_repos_path();
    if path.exists() {
        return Ok(());
    }

    write_json(&path, &default_skill_repos())
}

pub fn load_skill_repos() -> Result<Vec<SkillRepo>> {
    ensure_default_skill_repos()?;
    let mut repos: Vec<SkillRepo> = read_json_or_default(&get_skill_repos_path())?;
    repos.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(repos)
}

pub fn get_skill_repo(name: &str) -> Result<Option<SkillRepo>> {
    let repos = load_skill_repos()?;
    Ok(repos.into_iter().find(|repo| repo.name == name))
}

pub fn upsert_skill_repo(repo: SkillRepo) -> Result<()> {
    let mut repos = load_skill_repos()?;
    repos.retain(|item| item.name != repo.name);
    repos.push(repo);
    repos.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    write_json(&get_skill_repos_path(), &repos)
}

pub fn replace_skill_repo(old_name: &str, repo: SkillRepo) -> Result<()> {
    let mut repos = load_skill_repos()?;
    repos.retain(|item| item.name != old_name && item.name != repo.name);
    repos.push(repo);
    repos.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    write_json(&get_skill_repos_path(), &repos)
}

pub fn remove_skill_repo(name: &str) -> Result<Option<SkillRepo>> {
    let mut repos = load_skill_repos()?;
    let removed = repos.iter().find(|repo| repo.name == name).cloned();
    repos.retain(|repo| repo.name != name);
    write_json(&get_skill_repos_path(), &repos)?;
    Ok(removed)
}

pub fn load_installed_skill_manifest() -> Result<Vec<InstalledSkillManifestEntry>> {
    read_json_or_default(&get_installed_skill_manifest_path())
}

pub fn save_installed_skill_manifest(entries: &[InstalledSkillManifestEntry]) -> Result<()> {
    write_json(&get_installed_skill_manifest_path(), entries)
}

pub fn upsert_installed_skill_manifest_entry(entry: InstalledSkillManifestEntry) -> Result<()> {
    let mut entries = load_installed_skill_manifest()?;
    entries.retain(|item| item.directory != entry.directory);
    entries.push(entry);
    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    save_installed_skill_manifest(&entries)
}

pub fn remove_installed_skill_manifest_entry(directory: &str) -> Result<()> {
    let mut entries = load_installed_skill_manifest()?;
    entries.retain(|item| item.directory != directory);
    save_installed_skill_manifest(&entries)
}

pub fn list_installed_skill_directories() -> Result<Vec<String>> {
    let ssot_dir = get_ssot_dir();
    let mut directories = Vec::new();

    for entry in std::fs::read_dir(&ssot_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.path().is_dir() {
            continue;
        }

        if let Some(name) = entry.file_name().to_str() {
            directories.push(name.to_string());
        }
    }

    directories.sort();
    Ok(directories)
}

pub fn get_cached_repo_zip(owner: &str, name: &str, branch: &str) -> PathBuf {
    get_skill_cache_dir().join(format!("{}_{}__{}.zip", owner, name, branch))
}

fn get_legacy_cached_repo_zip(owner: &str, name: &str, branch: &str) -> PathBuf {
    get_skill_repo_dir().join(format!("{}_{}__{}.zip", owner, name, branch))
}

pub fn read_cached_zip(owner: &str, name: &str, branch: &str) -> Option<Vec<u8>> {
    let cache_path = get_cached_repo_zip(owner, name, branch);
    if cache_path.exists() {
        return std::fs::read(cache_path).ok();
    }

    let legacy_path = get_legacy_cached_repo_zip(owner, name, branch);
    if legacy_path.exists() {
        return std::fs::read(legacy_path).ok();
    }

    None
}

pub fn save_zip_to_cache(owner: &str, name: &str, branch: &str, bytes: &[u8]) -> Result<()> {
    let path = get_cached_repo_zip(owner, name, branch);
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    tracing::info!("Saved repo ZIP to cache: {}", path.display());
    Ok(())
}

pub fn delete_cached_repo_zip(owner: &str, name: &str) {
    for cache_dir in [get_skill_cache_dir(), get_skill_repo_dir()] {
        if let Ok(entries) = std::fs::read_dir(&cache_dir) {
            let prefix = format!("{}_{}_", owner, name);
            for entry in entries.flatten() {
                if let Some(filename) = entry.file_name().to_str() {
                    if filename.starts_with(&prefix) && filename.ends_with(".zip") {
                        let _ = std::fs::remove_file(entry.path());
                        tracing::info!("Deleted cached ZIP: {}", filename);
                    }
                }
            }
        }
    }
}

pub fn migrate_legacy_skill_repos(repos: &[SkillRepo]) -> Result<()> {
    let path = get_skill_repos_path();
    if path.exists() || repos.is_empty() {
        return Ok(());
    }

    let mut migrated = repos.to_vec();
    migrated.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    write_json(&path, &migrated)
}

pub fn migrate_legacy_installed_skill_manifest(entries: &[InstalledSkillManifestEntry]) -> Result<()> {
    let path = get_installed_skill_manifest_path();
    if path.exists() || entries.is_empty() {
        return Ok(());
    }

    let mut migrated = entries.to_vec();
    migrated.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    write_json(&path, &migrated)
}

pub fn is_local_repo_source(source: &str) -> bool {
    let path = Path::new(source);
    path.is_absolute() || source.contains(":\\") || source.starts_with('/')
}

pub fn repo_map(repos: &[SkillRepo]) -> HashMap<String, SkillRepo> {
    repos
        .iter()
        .cloned()
        .map(|repo| (repo.name.clone(), repo))
        .collect()
}
