use crate::config::get_data_dir;
use crate::db::models::SkillRepo;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, String>;

const SKILL_STORAGE_JSON: &str = "config.json";
const DEFAULT_SKILL_CACHE_DIR: &str = ".cache";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledSkillManifestEntry {
    pub directory: String,
    pub name: String,
    pub description: Option<String>,
    pub repo: Option<SkillRepo>,
    pub readme_url: Option<String>,
    pub installed_at: i64,
    pub source_directory: Option<String>, // 仓库中的原始相对路径，用于匹配收藏
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SkillStorage {
    #[serde(default)]
    repos: Vec<StoredSkillRepo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSkillRepo {
    name: String,
    source: String,
    #[serde(default)]
    skills: Vec<StoredInstalledSkill>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredInstalledSkill {
    directory: String,
    installed_at: i64,
    source_directory: Option<String>,
}

impl StoredSkillRepo {
    fn new(repo: SkillRepo) -> Self {
        Self {
            name: repo.name,
            source: repo.source,
            skills: Vec::new(),
        }
    }

    fn to_repo(&self) -> SkillRepo {
        SkillRepo {
            name: self.name.clone(),
            source: self.source.clone(),
        }
    }
}

impl From<&InstalledSkillManifestEntry> for StoredInstalledSkill {
    fn from(entry: &InstalledSkillManifestEntry) -> Self {
        Self {
            directory: entry.directory.clone(),
            installed_at: entry.installed_at,
            source_directory: entry.source_directory.clone(),
        }
    }
}

impl StoredInstalledSkill {
    fn into_manifest(self, repo: SkillRepo) -> InstalledSkillManifestEntry {
        InstalledSkillManifestEntry {
            directory: self.directory.clone(),
            name: self.directory,
            description: None,
            repo: Some(repo),
            readme_url: None,
            installed_at: self.installed_at,
            source_directory: self.source_directory,
        }
    }
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

pub fn get_skill_cache_dir() -> PathBuf {
    let dir = get_ssot_dir().join(DEFAULT_SKILL_CACHE_DIR);
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn get_skill_storage_path() -> PathBuf {
    get_ssot_dir().join(SKILL_STORAGE_JSON)
}

fn default_skill_repos() -> Vec<SkillRepo> {
    vec![SkillRepo {
        name: "skills".to_string(),
        source: "anthropics/skills".to_string(),
    }]
}

fn default_skill_storage() -> SkillStorage {
    SkillStorage {
        repos: default_skill_repos()
            .into_iter()
            .map(StoredSkillRepo::new)
            .collect(),
    }
}

fn sort_storage(storage: &mut SkillStorage) {
    for repo in &mut storage.repos {
        repo.skills
            .sort_by(|a, b| a.installed_at.cmp(&b.installed_at));
    }
    storage
        .repos
        .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
}

fn save_skill_storage(mut storage: SkillStorage) -> Result<()> {
    sort_storage(&mut storage);
    write_json(&get_skill_storage_path(), &storage)
}

fn load_skill_storage() -> Result<SkillStorage> {
    ensure_default_skill_repos()?;
    let mut storage: SkillStorage = read_json_or_default(&get_skill_storage_path())?;
    sort_storage(&mut storage);
    Ok(storage)
}

pub fn ensure_default_skill_repos() -> Result<()> {
    let path = get_skill_storage_path();
    if path.exists() {
        return Ok(());
    }

    write_json(&path, &default_skill_storage())
}

pub fn load_skill_repos() -> Result<Vec<SkillRepo>> {
    let storage = load_skill_storage()?;
    Ok(storage
        .repos
        .into_iter()
        .map(|repo| repo.to_repo())
        .collect())
}

pub fn get_skill_repo(name: &str) -> Result<Option<SkillRepo>> {
    let storage = load_skill_storage()?;
    Ok(storage
        .repos
        .into_iter()
        .find(|repo| repo.name == name)
        .map(|repo| repo.to_repo()))
}

pub fn upsert_skill_repo(repo: SkillRepo) -> Result<()> {
    let mut storage = load_skill_storage()?;

    if let Some(existing) = storage.repos.iter_mut().find(|item| item.name == repo.name) {
        existing.source = repo.source;
    } else {
        storage.repos.push(StoredSkillRepo::new(repo));
    }

    save_skill_storage(storage)
}

pub fn remove_skill_repo(name: &str) -> Result<Option<SkillRepo>> {
    let mut storage = load_skill_storage()?;
    let Some(index) = storage.repos.iter().position(|repo| repo.name == name) else {
        return Ok(None);
    };

    let removed = storage.repos.remove(index).to_repo();

    save_skill_storage(storage)?;
    Ok(Some(removed))
}

pub fn load_installed_skill_manifest() -> Result<Vec<InstalledSkillManifestEntry>> {
    let storage = load_skill_storage()?;
    let mut entries = Vec::new();

    for repo in storage.repos {
        let repo_info = repo.to_repo();
        for skill in repo.skills {
            entries.push(skill.into_manifest(repo_info.clone()));
        }
    }

    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(entries)
}

pub fn upsert_installed_skill_manifest_entry(entry: InstalledSkillManifestEntry) -> Result<()> {
    let repo = entry
        .repo
        .clone()
        .ok_or_else(|| "Skill missing repo info".to_string())?;
    let mut storage = load_skill_storage()?;

    for repo_entry in &mut storage.repos {
        repo_entry
            .skills
            .retain(|item| item.directory != entry.directory);
    }

    if let Some(repo_entry) = storage.repos.iter_mut().find(|item| item.name == repo.name) {
        repo_entry.source = repo.source;
        repo_entry.skills.push(StoredInstalledSkill::from(&entry));
    } else {
        let mut repo_entry = StoredSkillRepo::new(repo);
        repo_entry.skills.push(StoredInstalledSkill::from(&entry));
        storage.repos.push(repo_entry);
    }

    save_skill_storage(storage)
}

pub fn remove_installed_skill_manifest_entry(directory: &str) -> Result<()> {
    let mut storage = load_skill_storage()?;

    for repo in &mut storage.repos {
        repo.skills.retain(|item| item.directory != directory);
    }

    save_skill_storage(storage)
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
            if name != DEFAULT_SKILL_CACHE_DIR {
                directories.push(name.to_string());
            }
        }
    }

    directories.sort();
    Ok(directories)
}

/// 生成缓存目录名，从 URL 中提取仓库名
pub fn get_cached_repo_dir(source: &str) -> PathBuf {
    let repo_name = source
        .trim()
        .strip_suffix(".git")
        .unwrap_or(source)
        .split('/')
        .last()
        .unwrap_or("repo");
    get_skill_cache_dir().join(repo_name)
}

/// 删除缓存的仓库目录
pub fn delete_cached_repo_dir(source: &str) {
    let cache_dir = get_cached_repo_dir(source);
    if cache_dir.exists() {
        let _ = std::fs::remove_dir_all(&cache_dir);
        tracing::info!("Deleted cached repo dir: {}", cache_dir.display());
    }
}

pub fn is_local_repo_source(source: &str) -> bool {
    let path = Path::new(source);
    path.is_absolute() || source.contains(":\\") || source.starts_with('/')
}

pub fn ensure_repo_exists(repo: &SkillRepo) -> Result<()> {
    let mut storage = load_skill_storage()?;

    if let Some(existing) = storage.repos.iter_mut().find(|item| item.name == repo.name) {
        existing.source = repo.source.clone();
    } else {
        storage.repos.push(StoredSkillRepo::new(repo.clone()));
    }

    save_skill_storage(storage)
}
