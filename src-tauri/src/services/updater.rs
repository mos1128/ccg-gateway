use crate::config::get_data_dir;
use crate::time::now_timestamp;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

pub const GITHUB_OWNER: &str = "mos1128";
pub const GITHUB_REPO: &str = "ccg-gateway";
pub const DEFAULT_ACCEL_PREFIX: &str = "https://gh-proxy.com/";
pub const UPDATE_PROGRESS_EVENT: &str = "update-download-progress";
pub const DOWNLOAD_CANCELLED: &str = "update_download_cancelled";

const LATEST_MANIFEST_PATH: &str = "releases/latest/download/latest.json";
const PLATFORM_KEYS_MACOS: &[&str] = &["macos-universal", "macos-aarch64", "macos-x86_64"];
const PLATFORM_KEYS_WINDOWS: &[&str] = &["windows-x86_64"];
const PLATFORM_KEYS_LINUX: &[&str] = &["linux-x86_64", "linux-aarch64"];

#[derive(Default)]
pub struct UpdateDownloadState {
    pub busy: AtomicBool,
    pub cancel: AtomicBool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSettings {
    pub auto_check: bool,
    pub accel_enabled: bool,
    pub accel_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateAsset {
    pub name: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestManifest {
    pub version: String,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub pub_date: Option<String>,
    #[serde(default)]
    pub assets: HashMap<String, UpdateAsset>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateRelease {
    pub version: String,
    pub tag: String,
    pub notes: String,
    pub published_at: Option<String>,
    pub html_url: String,
    pub asset_key: Option<String>,
    pub asset: Option<UpdateAsset>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest: Option<UpdateRelease>,
    pub has_update: bool,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateSettingsResponse {
    pub auto_check: bool,
    pub accel_enabled: bool,
    pub accel_prefix: String,
    pub current_version: String,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateProgressPayload {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadResult {
    pub path: String,
    pub version: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallOutcome {
    pub kind: String,
    pub path: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateApplyResult {
    pub version: String,
    pub ok: bool,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiRelease {
    tag_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    assets: Vec<ApiAsset>,
}

#[derive(Debug, Deserialize)]
struct ApiAsset {
    name: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    digest: Option<String>,
}

pub async fn load_update_settings(db: &SqlitePool) -> Result<UpdateSettings, String> {
    let row = sqlx::query_as::<_, (i64, i64, String)>(
        "SELECT update_auto_check, update_accel_enabled, update_accel_prefix FROM gateway_settings WHERE id = 1",
    )
    .fetch_one(db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(UpdateSettings {
        auto_check: row.0 != 0,
        accel_enabled: row.1 != 0,
        accel_prefix: row.2,
    })
}

pub async fn save_update_settings(
    db: &SqlitePool,
    auto_check: Option<bool>,
    accel_enabled: Option<bool>,
    accel_prefix: Option<String>,
) -> Result<(), String> {
    let normalized_prefix = accel_prefix
        .as_deref()
        .map(normalize_accel_prefix)
        .transpose()?;

    let mut updates = Vec::new();
    if auto_check.is_some() {
        updates.push("update_auto_check = ?");
    }
    if accel_enabled.is_some() {
        updates.push("update_accel_enabled = ?");
    }
    if normalized_prefix.is_some() {
        updates.push("update_accel_prefix = ?");
    }
    if updates.is_empty() {
        return Ok(());
    }
    updates.push("updated_at = ?");

    let sql = format!(
        "UPDATE gateway_settings SET {} WHERE id = 1",
        updates.join(", ")
    );
    let mut query = sqlx::query(&sql);
    if let Some(auto_check) = auto_check {
        query = query.bind(if auto_check { 1i64 } else { 0 });
    }
    if let Some(accel_enabled) = accel_enabled {
        query = query.bind(if accel_enabled { 1i64 } else { 0 });
    }
    if let Some(accel_prefix) = normalized_prefix {
        query = query.bind(accel_prefix);
    }
    query
        .bind(now_timestamp())
        .execute(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn normalize_accel_prefix(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    if trimmed.chars().any(|c| c.is_whitespace()) {
        return Err(crate::tr!("err.update.accel_prefix_invalid"));
    }
    let parsed =
        reqwest::Url::parse(trimmed).map_err(|_| crate::tr!("err.update.accel_prefix_invalid"))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(crate::tr!("err.update.accel_prefix_invalid"));
    }
    Ok(if trimmed.ends_with('/') {
        trimmed.to_string()
    } else {
        format!("{}/", trimmed)
    })
}

fn effective_accel_prefix(settings: &UpdateSettings) -> Option<String> {
    if !settings.accel_enabled {
        return None;
    }
    normalize_accel_prefix(&settings.accel_prefix)
        .ok()
        .filter(|prefix| !prefix.is_empty())
}

pub fn platform_asset_keys() -> &'static [&'static str] {
    if cfg!(target_os = "windows") {
        PLATFORM_KEYS_WINDOWS
    } else if cfg!(target_os = "macos") {
        PLATFORM_KEYS_MACOS
    } else {
        PLATFORM_KEYS_LINUX
    }
}

pub fn platform_label() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

pub fn compare_versions(left: &str, right: &str) -> i32 {
    let left_parts = version_parts(left);
    let right_parts = version_parts(right);
    let count = left_parts.len().max(right_parts.len());
    for index in 0..count {
        let left = left_parts.get(index).copied().unwrap_or(0);
        let right = right_parts.get(index).copied().unwrap_or(0);
        if left > right {
            return 1;
        }
        if left < right {
            return -1;
        }
    }
    0
}

fn version_parts(input: &str) -> Vec<u64> {
    let trimmed = input.trim().trim_start_matches(['v', 'V']);
    let core = trimmed.split(['-', '+']).next().unwrap_or(trimmed).trim();
    core.split('.')
        .map(|part| {
            let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse::<u64>().unwrap_or(0)
        })
        .collect()
}

fn format_version(input: &str) -> String {
    input
        .trim()
        .trim_start_matches(['v', 'V'])
        .trim()
        .to_string()
}

fn sanitize_tag(tag: &str, version: &str) -> String {
    let trimmed = tag.trim();
    if !trimmed.is_empty()
        && trimmed.len() <= 128
        && !trimmed.contains("..")
        && trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'))
    {
        trimmed.to_string()
    } else {
        format!("v{}", format_version(version))
    }
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("ccg-gateway-updater")
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| crate::tr!("err.update.network_failed", e))
}

fn download_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("ccg-gateway-updater")
        .connect_timeout(Duration::from_secs(15))
        .pool_idle_timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| crate::tr!("err.update.network_failed", e))
}

fn github_url(path: &str) -> String {
    format!(
        "https://github.com/{}/{}/{}",
        GITHUB_OWNER, GITHUB_REPO, path
    )
}

fn prefixed_candidates(settings: &UpdateSettings, direct_url: &str) -> Vec<(String, &'static str)> {
    let mut candidates = Vec::new();
    if let Some(prefix) = effective_accel_prefix(settings) {
        candidates.push((format!("{}{}", prefix, direct_url), "mirror"));
    }
    candidates.push((direct_url.to_string(), "github"));
    candidates
}

async fn fetch_manifest(
    client: &reqwest::Client,
    settings: &UpdateSettings,
) -> Result<(LatestManifest, String), String> {
    let url = format!("{}?t={}", github_url(LATEST_MANIFEST_PATH), now_timestamp());
    let mut last_error = None;
    for (candidate, source) in prefixed_candidates(settings, &url) {
        match client
            .get(&candidate)
            .header("Accept", "application/json")
            .header("Cache-Control", "no-cache")
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                match response.json::<LatestManifest>().await {
                    Ok(mut manifest) => {
                        if manifest.version.trim().is_empty() {
                            manifest.version = format_version(&manifest.tag);
                        }
                        if manifest.version.is_empty() {
                            last_error = Some(crate::tr!("err.update.manifest_parse_failed"));
                            continue;
                        }
                        if manifest.tag.trim().is_empty() {
                            manifest.tag = format!("v{}", manifest.version);
                        }
                        return Ok((manifest, source.to_string()));
                    }
                    Err(error) => {
                        last_error = Some(crate::tr!("err.update.manifest_parse_failed"));
                        tracing::warn!("Failed to parse update manifest {}: {}", candidate, error);
                    }
                }
            }
            Ok(response) => {
                last_error = Some(crate::tr!("err.update.github_api_error", response.status()));
            }
            Err(error) => {
                last_error = Some(crate::tr!("err.update.network_failed", error));
            }
        }
    }
    Err(last_error.unwrap_or_else(|| crate::tr!("err.update.network_failed", "manifest")))
}

async fn fetch_api_release(
    client: &reqwest::Client,
    settings: &UpdateSettings,
) -> Result<(UpdateRelease, String), String> {
    let api_url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        GITHUB_OWNER, GITHUB_REPO
    );
    let mut last_error = None;
    for (candidate, source) in prefixed_candidates(settings, &api_url) {
        match client
            .get(&candidate)
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                let release = match response.json::<ApiRelease>().await {
                    Ok(release) => release,
                    Err(error) => {
                        last_error =
                            Some(crate::tr!("err.update.response_parse_failed", "release"));
                        tracing::warn!("Failed to parse GitHub release {}: {}", candidate, error);
                        continue;
                    }
                };
                let version = format_version(&release.tag_name);
                let tag = sanitize_tag(&release.tag_name, &version);
                let assets = release
                    .assets
                    .iter()
                    .map(|asset| UpdateAsset {
                        name: asset.name.clone(),
                        size: asset.size,
                        sha256: asset
                            .digest
                            .as_deref()
                            .and_then(|digest| digest.strip_prefix("sha256:"))
                            .map(|digest| digest.to_string()),
                    })
                    .collect::<Vec<_>>();
                let asset = select_api_asset(&assets);
                let html_url = if release.html_url.is_empty() {
                    github_url(&format!("releases/tag/{}", tag))
                } else {
                    release.html_url
                };
                return Ok((
                    UpdateRelease {
                        version,
                        tag,
                        notes: release
                            .body
                            .or(release.name)
                            .unwrap_or_default()
                            .trim()
                            .to_string(),
                        published_at: release.published_at,
                        html_url,
                        asset_key: asset.as_ref().map(|asset| asset.name.clone()),
                        asset,
                    },
                    format!("api-{}", source),
                ));
            }
            Ok(response) => {
                last_error = Some(crate::tr!("err.update.github_api_error", response.status()));
            }
            Err(error) => {
                last_error = Some(crate::tr!("err.update.network_failed", error));
            }
        }
    }
    Err(last_error.unwrap_or_else(|| crate::tr!("err.update.network_failed", "api")))
}

async fn fetch_redirect_release(
    client: &reqwest::Client,
    settings: &UpdateSettings,
) -> Result<(UpdateRelease, String), String> {
    let url = github_url("releases/latest");
    let mut last_error = None;
    for (candidate, source) in prefixed_candidates(settings, &url) {
        match client.get(&candidate).send().await {
            Ok(response) if response.status().is_success() => {
                let mut html = response.text().await.unwrap_or_default();
                let limit = 512 * 1024;
                if html.len() > limit {
                    let mut boundary = limit;
                    while boundary > 0 && !html.is_char_boundary(boundary) {
                        boundary -= 1;
                    }
                    html.truncate(boundary);
                }
                if let Some(tag) = extract_tag_from_html(&html) {
                    let version = format_version(&tag);
                    let tag = sanitize_tag(&tag, &version);
                    let asset_name = default_asset_name();
                    let asset = UpdateAsset {
                        name: asset_name.to_string(),
                        size: 0,
                        sha256: None,
                    };
                    return Ok((
                        UpdateRelease {
                            version,
                            tag: tag.clone(),
                            notes: String::new(),
                            published_at: None,
                            html_url: github_url(&format!("releases/tag/{}", tag)),
                            asset_key: Some(platform_asset_keys()[0].to_string()),
                            asset: Some(asset),
                        },
                        format!("redirect-{}", source),
                    ));
                }
                last_error = Some(crate::tr!("err.update.response_parse_failed", "tag"));
            }
            Ok(response) => {
                last_error = Some(crate::tr!("err.update.github_api_error", response.status()));
            }
            Err(error) => {
                last_error = Some(crate::tr!("err.update.network_failed", error));
            }
        }
    }
    Err(last_error.unwrap_or_else(|| crate::tr!("err.update.network_failed", "redirect")))
}

fn extract_tag_from_html(html: &str) -> Option<String> {
    let marker = "/releases/tag/";
    let index = html.find(marker)?;
    let rest = &html[index + marker.len()..];
    let tag: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'))
        .collect();
    if tag.is_empty() {
        None
    } else {
        Some(tag)
    }
}

fn default_asset_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "ccg-gateway.exe"
    } else if cfg!(target_os = "macos") {
        "ccg-gateway-macos-universal.dmg"
    } else {
        "ccg-gateway-linux-x86_64.AppImage"
    }
}

fn select_manifest_asset(manifest: &LatestManifest) -> (Option<String>, Option<UpdateAsset>) {
    for key in platform_asset_keys() {
        if let Some(asset) = manifest.assets.get(*key) {
            return (Some((*key).to_string()), Some(asset.clone()));
        }
    }
    for (key, asset) in &manifest.assets {
        if asset_matches_platform(&asset.name) {
            return (Some(key.clone()), Some(asset.clone()));
        }
    }
    (None, None)
}

fn asset_matches_platform(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if cfg!(target_os = "windows") {
        lower.ends_with(".exe")
    } else if cfg!(target_os = "macos") {
        lower.ends_with(".dmg")
    } else {
        lower.ends_with(".appimage")
    }
}

fn select_api_asset(assets: &[UpdateAsset]) -> Option<UpdateAsset> {
    if let Some(asset) = assets
        .iter()
        .find(|asset| asset.name.eq_ignore_ascii_case(default_asset_name()))
    {
        return Some(asset.clone());
    }
    assets
        .iter()
        .find(|asset| asset_matches_platform(&asset.name))
        .cloned()
}

fn release_from_manifest(manifest: LatestManifest) -> UpdateRelease {
    let (asset_key, asset) = select_manifest_asset(&manifest);
    let version = format_version(&manifest.version);
    let tag = sanitize_tag(&manifest.tag, &version);
    let html_url = if tag.trim().is_empty() {
        github_url("releases/latest")
    } else {
        github_url(&format!("releases/tag/{}", tag))
    };
    UpdateRelease {
        version,
        tag,
        notes: manifest.notes.trim().to_string(),
        published_at: manifest.pub_date,
        html_url,
        asset_key,
        asset,
    }
}

async fn fetch_release(
    client: &reqwest::Client,
    settings: &UpdateSettings,
) -> Result<(UpdateRelease, String), String> {
    match fetch_manifest(client, settings).await {
        Ok((manifest, source)) => return Ok((release_from_manifest(manifest), source)),
        Err(error) => {
            tracing::warn!(
                "Update manifest unavailable, falling back to GitHub API: {}",
                error
            );
        }
    }
    match fetch_api_release(client, settings).await {
        Ok(result) => return Ok(result),
        Err(error) => {
            tracing::warn!(
                "GitHub API unavailable, falling back to release redirect: {}",
                error
            );
        }
    }
    fetch_redirect_release(client, settings).await
}

pub async fn check_for_updates(
    app: &AppHandle,
    db: &SqlitePool,
) -> Result<UpdateCheckResult, String> {
    let settings = load_update_settings(db).await?;
    let client = http_client()?;
    let (release, source) = fetch_release(&client, &settings).await?;
    let current_version = app.package_info().version.to_string();
    let has_update = compare_versions(&release.version, &current_version) > 0;
    Ok(UpdateCheckResult {
        current_version,
        latest: Some(release),
        has_update,
        source,
    })
}

pub async fn get_update_settings(
    app: &AppHandle,
    db: &SqlitePool,
) -> Result<UpdateSettingsResponse, String> {
    let settings = load_update_settings(db).await?;
    Ok(UpdateSettingsResponse {
        auto_check: settings.auto_check,
        accel_enabled: settings.accel_enabled,
        accel_prefix: settings.accel_prefix,
        current_version: app.package_info().version.to_string(),
        platform: platform_label().to_string(),
    })
}

fn updates_dir() -> PathBuf {
    get_data_dir().join("updates")
}

fn sanitize_file_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty()
        || trimmed == "."
        || trimmed == ".."
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
    {
        return Err(crate::tr!("err.update.asset_missing"));
    }
    Ok(trimmed.to_string())
}

fn update_file_path(version: &str, asset_name: &str) -> Result<PathBuf, String> {
    let file_name = sanitize_file_name(asset_name)?;
    let version = format_version(version);
    if version.is_empty()
        || !version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'))
    {
        return Err(crate::tr!("err.update.asset_missing"));
    }
    Ok(updates_dir().join(format!("{}-{}", version, file_name)))
}

fn find_downloaded_file(version: &str) -> Option<PathBuf> {
    let prefix = format!("{}-", format_version(version));
    let entries = std::fs::read_dir(updates_dir()).ok()?;
    let mut candidates = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .map(|name| name.to_string_lossy().starts_with(&prefix))
                    .unwrap_or(false)
                && path
                    .file_name()
                    .map(|name| asset_matches_platform(&name.to_string_lossy()))
                    .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|path| {
        std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs())
            .unwrap_or(0)
    });
    candidates.pop()
}

fn build_download_candidates(
    settings: &UpdateSettings,
    tag: &str,
    asset_name: &str,
) -> Vec<String> {
    let tag = if tag.trim().is_empty() {
        format!("v{}", format_version(tag))
    } else {
        tag.trim().to_string()
    };
    let path = format!(
        "releases/download/{}/{}",
        tag,
        urlencoding::encode(asset_name)
    );
    let direct_url = github_url(&path);
    prefixed_candidates(settings, &direct_url)
        .into_iter()
        .map(|(url, _)| url)
        .collect()
}

pub async fn download_update(
    app: &AppHandle,
    db: &SqlitePool,
    state: &UpdateDownloadState,
    version: &str,
) -> Result<DownloadResult, String> {
    if state.busy.swap(true, Ordering::SeqCst) {
        return Err(crate::tr!("err.update.download_busy"));
    }
    state.cancel.store(false, Ordering::SeqCst);

    let result = download_update_inner(app, db, state, version).await;
    state.busy.store(false, Ordering::SeqCst);
    result
}

async fn download_update_inner(
    app: &AppHandle,
    db: &SqlitePool,
    state: &UpdateDownloadState,
    version: &str,
) -> Result<DownloadResult, String> {
    let settings = load_update_settings(db).await?;
    let client = http_client()?;
    let (release, _) = fetch_release(&client, &settings).await?;
    if compare_versions(&release.version, version) != 0 {
        return Err(crate::tr!("err.update.version_changed"));
    }
    let asset = release
        .asset
        .clone()
        .ok_or_else(|| crate::tr!("err.update.asset_missing"))?;
    let final_path = update_file_path(&release.version, &asset.name)?;
    let mut part_name = final_path.as_os_str().to_os_string();
    part_name.push(".part");
    let part_path = PathBuf::from(part_name);

    if let Some(parent) = final_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| crate::tr!("err.update.download_failed", e))?;
    }

    let urls = build_download_candidates(&settings, &release.tag, &asset.name);
    if urls.is_empty() {
        return Err(crate::tr!("err.update.no_download_source"));
    }

    let download_client = download_client()?;
    let mut last_error = crate::tr!("err.update.no_download_source");
    for (index, url) in urls.iter().enumerate() {
        if state.cancel.load(Ordering::SeqCst) {
            let _ = tokio::fs::remove_file(&part_path).await;
            return Err(DOWNLOAD_CANCELLED.to_string());
        }
        if index > 0 {
            let _ = tokio::fs::remove_file(&part_path).await;
        }
        match download_one(
            &download_client,
            state,
            url,
            &part_path,
            &release.version,
            app,
        )
        .await
        {
            Ok(()) => {
                if let Err(error) = verify_download(&part_path, &asset).await {
                    let _ = tokio::fs::remove_file(&part_path).await;
                    last_error = error;
                    continue;
                }
                let _ = tokio::fs::remove_file(&final_path).await;
                tokio::fs::rename(&part_path, &final_path)
                    .await
                    .map_err(|e| crate::tr!("err.update.download_failed", e))?;
                let size = tokio::fs::metadata(&final_path)
                    .await
                    .map(|meta| meta.len())
                    .unwrap_or(asset.size);
                return Ok(DownloadResult {
                    path: final_path.to_string_lossy().to_string(),
                    version: release.version,
                    size,
                });
            }
            Err(error) if error == DOWNLOAD_CANCELLED => {
                let _ = tokio::fs::remove_file(&part_path).await;
                return Err(DOWNLOAD_CANCELLED.to_string());
            }
            Err(error) => {
                last_error = error;
            }
        }
    }

    Err(last_error)
}

async fn download_one(
    client: &reqwest::Client,
    state: &UpdateDownloadState,
    url: &str,
    part_path: &Path,
    version: &str,
    app: &AppHandle,
) -> Result<(), String> {
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    let existing = tokio::fs::metadata(part_path)
        .await
        .map(|meta| meta.len())
        .unwrap_or(0);
    let mut request = client.get(url);
    if existing > 0 {
        request = request.header("Range", format!("bytes={}-", existing));
    }

    let send = request.send();
    tokio::pin!(send);
    let response_deadline = Instant::now() + Duration::from_secs(25);
    let response = loop {
        if state.cancel.load(Ordering::SeqCst) {
            return Err(DOWNLOAD_CANCELLED.to_string());
        }
        if Instant::now() >= response_deadline {
            return Err(crate::tr!(
                "err.update.download_failed",
                "connection timeout"
            ));
        }
        tokio::select! {
            result = &mut send => {
                break result.map_err(|e| crate::tr!("err.update.download_failed", e))?;
            }
            _ = tokio::time::sleep(Duration::from_millis(250)) => {}
        }
    };

    let status = response.status();
    if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        let _ = tokio::fs::remove_file(part_path).await;
        return Err(crate::tr!("err.update.download_failed", "range rejected"));
    }
    if !status.is_success() {
        return Err(crate::tr!("err.update.download_failed", status));
    }

    let appending = status == reqwest::StatusCode::PARTIAL_CONTENT && existing > 0;
    let start = if appending { existing } else { 0 };
    if !appending {
        let _ = tokio::fs::remove_file(part_path).await;
    }
    let total = response.content_length().map(|length| start + length);

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(appending)
        .truncate(!appending)
        .open(part_path)
        .await
        .map_err(|e| crate::tr!("err.update.download_failed", e))?;

    let mut downloaded = start;
    let mut last_percent = downloaded
        .saturating_mul(100)
        .checked_div(total.unwrap_or(0))
        .unwrap_or(0);
    let mut last_emit = Instant::now();
    let mut last_data = Instant::now();
    emit_progress(app, downloaded, total, version);

    let mut stream = response.bytes_stream();
    loop {
        if state.cancel.load(Ordering::SeqCst) {
            let _ = file.flush().await;
            return Err(DOWNLOAD_CANCELLED.to_string());
        }
        if last_data.elapsed() >= Duration::from_secs(30) {
            let _ = file.flush().await;
            return Err(crate::tr!("err.update.download_failed", "stalled"));
        }

        let next = tokio::select! {
            result = stream.next() => result,
            _ = tokio::time::sleep(Duration::from_millis(250)) => continue,
        };
        match next {
            None => break,
            Some(Err(error)) => {
                let _ = file.flush().await;
                return Err(crate::tr!("err.update.download_failed", error));
            }
            Some(Ok(chunk)) => {
                last_data = Instant::now();
                file.write_all(&chunk)
                    .await
                    .map_err(|e| crate::tr!("err.update.download_failed", e))?;
                downloaded += chunk.len() as u64;
                let percent = total
                    .filter(|total| *total > 0)
                    .map(|total| downloaded.saturating_mul(100) / total)
                    .unwrap_or(0);
                if percent >= last_percent + 1 || last_emit.elapsed() >= Duration::from_millis(200)
                {
                    last_percent = percent;
                    last_emit = Instant::now();
                    emit_progress(app, downloaded, total, version);
                }
            }
        }
    }

    file.flush()
        .await
        .map_err(|e| crate::tr!("err.update.download_failed", e))?;
    file.sync_all()
        .await
        .map_err(|e| crate::tr!("err.update.download_failed", e))?;

    if let Some(total) = total {
        if downloaded != total {
            return Err(crate::tr!(
                "err.update.download_incomplete",
                downloaded,
                total
            ));
        }
    }
    Ok(())
}

fn emit_progress(app: &AppHandle, downloaded: u64, total: Option<u64>, version: &str) {
    let _ = app.emit(
        UPDATE_PROGRESS_EVENT,
        UpdateProgressPayload {
            downloaded,
            total,
            version: version.to_string(),
        },
    );
}

async fn verify_download(path: &Path, asset: &UpdateAsset) -> Result<(), String> {
    let metadata = tokio::fs::metadata(path)
        .await
        .map_err(|e| crate::tr!("err.update.download_failed", e))?;
    if asset.size > 0 && metadata.len() != asset.size {
        return Err(crate::tr!(
            "err.update.download_incomplete",
            metadata.len(),
            asset.size
        ));
    }

    if let Some(expected) = asset.sha256.as_deref() {
        let expected = expected.trim().to_ascii_lowercase();
        let path = path.to_path_buf();
        let actual = tokio::task::spawn_blocking(move || sha256_file(&path))
            .await
            .map_err(|e| crate::tr!("err.update.checksum_failed", e))?
            .map_err(|e| crate::tr!("err.update.checksum_failed", e))?;
        if !actual.eq_ignore_ascii_case(&expected) {
            return Err(crate::tr!("err.update.checksum_failed", "sha256 mismatch"));
        }
    }

    if !verify_magic(path, metadata.len()).await {
        return Err(crate::tr!("err.update.invalid_file"));
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, std::io::Error> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

async fn verify_magic(path: &Path, size: u64) -> bool {
    use tokio::io::{AsyncReadExt, AsyncSeekExt};

    if cfg!(target_os = "macos") {
        if size < 512 {
            return false;
        }
        let mut file = match tokio::fs::File::open(path).await {
            Ok(file) => file,
            Err(_) => return false,
        };
        if file.seek(std::io::SeekFrom::End(-512)).await.is_err() {
            return false;
        }
        let mut buffer = [0u8; 4];
        return file.read_exact(&mut buffer).await.is_ok() && &buffer == b"koly";
    }

    let mut file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(_) => return false,
    };
    let mut buffer = [0u8; 4];
    if file.read_exact(&mut buffer).await.is_err() {
        return false;
    }
    if cfg!(target_os = "windows") {
        &buffer[..2] == b"MZ"
    } else {
        buffer == [0x7f, b'E', b'L', b'F']
    }
}

pub async fn install_update(
    app: &AppHandle,
    db: &SqlitePool,
    version: &str,
) -> Result<InstallOutcome, String> {
    let settings = load_update_settings(db).await?;
    let client = http_client()?;
    let (release, _) = fetch_release(&client, &settings).await?;
    if compare_versions(&release.version, version) != 0 {
        return Err(crate::tr!("err.update.version_changed"));
    }
    let asset = release
        .asset
        .ok_or_else(|| crate::tr!("err.update.asset_missing"))?;
    let expected_path = update_file_path(&release.version, &asset.name)?;
    let path = if expected_path.exists() {
        expected_path
    } else {
        find_downloaded_file(&release.version)
            .ok_or_else(|| crate::tr!("err.update.file_not_found"))?
    };

    if cfg!(target_os = "windows") {
        install_windows(app, &path, &release.version).await
    } else if cfg!(target_os = "macos") {
        install_macos(&path)
    } else {
        install_linux(app, &path, &release.version).await
    }
}

fn manual_outcome(path: &Path, message: &str) -> InstallOutcome {
    InstallOutcome {
        kind: "manual".to_string(),
        path: path.to_string_lossy().to_string(),
        message: Some(message.to_string()),
    }
}

async fn install_windows(
    app: &AppHandle,
    source: &Path,
    version: &str,
) -> Result<InstallOutcome, String> {
    if cfg!(debug_assertions) {
        return Ok(manual_outcome(source, &crate::tr!("err.update.dev_build")));
    }

    let target =
        std::env::current_exe().map_err(|e| crate::tr!("err.update.install_unsupported", e))?;
    ensure_parent_writable(&target)?;

    let mut command = Command::new(&target);
    command
        .arg("--ccg-apply-update")
        .arg(std::process::id().to_string())
        .arg(source)
        .arg(&target)
        .arg(get_data_dir())
        .arg(version)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command
        .spawn()
        .map_err(|e| crate::tr!("err.update.spawn_failed", e))?;

    schedule_app_exit(app);
    Ok(InstallOutcome {
        kind: "restart_scheduled".to_string(),
        path: source.to_string_lossy().to_string(),
        message: None,
    })
}

fn install_macos(source: &Path) -> Result<InstallOutcome, String> {
    Command::new("open")
        .arg(source)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| crate::tr!("err.update.spawn_failed", e))?;
    Ok(InstallOutcome {
        kind: "opened_installer".to_string(),
        path: source.to_string_lossy().to_string(),
        message: None,
    })
}

async fn install_linux(
    app: &AppHandle,
    source: &Path,
    version: &str,
) -> Result<InstallOutcome, String> {
    let Some(target) = std::env::var_os("APPIMAGE").map(PathBuf::from) else {
        return Ok(manual_outcome(
            source,
            &crate::tr!("err.update.not_appimage"),
        ));
    };
    ensure_parent_writable(&target)?;

    let data_dir = get_data_dir();
    let script_path =
        std::env::temp_dir().join(format!("ccg-gateway-update-{}.sh", std::process::id()));
    let script = format!(
        r#"#!/bin/sh
PID="$1"
SRC="$2"
TARGET="$3"
RESULT="$4"
VERSION="$5"
while kill -0 "$PID" 2>/dev/null; do
  sleep 0.3
done
OLD="$TARGET.old"
rm -f "$OLD"
if mv "$TARGET" "$OLD" 2>/dev/null && cp "$SRC" "$TARGET" && chmod +x "$TARGET"; then
  printf '{{"version":"%s","ok":true}}' "$VERSION" > "$RESULT"
  rm -f "$OLD" "$SRC"
  "$TARGET" >/dev/null 2>&1 &
else
  printf '{{"version":"%s","ok":false,"message":"replace failed"}}' "$VERSION" > "$RESULT"
  if [ -f "$OLD" ]; then mv "$OLD" "$TARGET"; fi
  "$TARGET" >/dev/null 2>&1 &
fi
rm -f "$0"
"#
    );
    tokio::fs::write(&script_path, script)
        .await
        .map_err(|e| crate::tr!("err.update.spawn_failed", e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755));
    }

    let mut command = Command::new("/bin/sh");
    command
        .arg(&script_path)
        .arg(std::process::id().to_string())
        .arg(source)
        .arg(&target)
        .arg(data_dir.join("update-result.json"))
        .arg(version)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command
        .spawn()
        .map_err(|e| crate::tr!("err.update.spawn_failed", e))?;

    schedule_app_exit(app);
    Ok(InstallOutcome {
        kind: "restart_scheduled".to_string(),
        path: source.to_string_lossy().to_string(),
        message: None,
    })
}

fn ensure_parent_writable(target: &Path) -> Result<(), String> {
    let parent = target
        .parent()
        .ok_or_else(|| crate::tr!("err.update.install_not_writable", target.display()))?;
    let probe = parent.join(format!(".ccg-update-write-{}", std::process::id()));
    std::fs::write(&probe, b"1")
        .map_err(|_| crate::tr!("err.update.install_not_writable", parent.display()))?;
    let _ = std::fs::remove_file(&probe);
    Ok(())
}

fn schedule_app_exit(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(600)).await;
        app.exit(0);
    });
}

pub fn take_install_result() -> Result<Option<UpdateApplyResult>, String> {
    let path = get_data_dir().join("update-result.json");
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&path);
    serde_json::from_str::<UpdateApplyResult>(&content)
        .map(Some)
        .map_err(|e| e.to_string())
}

pub fn cleanup_update_cache() {
    let dir = updates_dir();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        let cutoff = std::time::SystemTime::now()
            .checked_sub(Duration::from_secs(7 * 24 * 60 * 60))
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        for entry in entries.flatten() {
            let path = entry.path();
            let stale = entry
                .metadata()
                .and_then(|meta| meta.modified())
                .map(|modified| modified < cutoff)
                .unwrap_or(false);
            if stale {
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(name) = exe.file_name() {
            if let Some(parent) = exe.parent() {
                for suffix in ["old", "new"] {
                    let mut file_name = name.to_os_string();
                    file_name.push(".");
                    file_name.push(suffix);
                    let path = parent.join(file_name);
                    if !path.exists() {
                        continue;
                    }
                    std::thread::spawn(move || {
                        for _ in 0..20 {
                            if std::fs::remove_file(&path).is_ok() {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(500));
                        }
                    });
                }
            }
        }
    }
}

pub fn cancel_download(state: &UpdateDownloadState) {
    state.cancel.store(true, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions_with_prefix_and_suffix() {
        assert_eq!(compare_versions("v2.1.4", "2.1.3"), 1);
        assert_eq!(compare_versions("2.1.3", "v2.1.4"), -1);
        assert_eq!(compare_versions("2.1.3", "2.1.3"), 0);
        assert_eq!(compare_versions("2.1", "2.1.0"), 0);
        assert_eq!(compare_versions("2.1.10", "2.1.9"), 1);
        assert_eq!(compare_versions("2.2.0-beta.1", "2.2.0"), 0);
    }

    #[test]
    fn normalizes_accel_prefix() {
        assert_eq!(
            normalize_accel_prefix("https://gh-proxy.com").unwrap(),
            "https://gh-proxy.com/"
        );
        assert_eq!(
            normalize_accel_prefix(" https://mirror.example/gh/ ").unwrap(),
            "https://mirror.example/gh/"
        );
        assert_eq!(normalize_accel_prefix("").unwrap(), "");
        assert!(normalize_accel_prefix("ftp://example.com/").is_err());
        assert!(normalize_accel_prefix("not a url").is_err());
    }

    #[test]
    fn builds_mirror_before_direct() {
        let settings = UpdateSettings {
            auto_check: true,
            accel_enabled: true,
            accel_prefix: "https://mirror.example/".to_string(),
        };
        let urls = build_download_candidates(&settings, "v2.1.4", "ccg-gateway.exe");
        assert_eq!(urls.len(), 2);
        assert!(urls[0].starts_with("https://mirror.example/https://github.com/"));
        assert!(urls[1].starts_with("https://github.com/"));
    }

    #[test]
    fn rejects_unsafe_asset_names() {
        assert!(sanitize_file_name("../evil.exe").is_err());
        assert!(sanitize_file_name("ok-file.exe").is_ok());
    }

    #[test]
    fn extracts_release_tag_from_html() {
        assert_eq!(
            extract_tag_from_html("<a href=\"/mos/ccg/releases/tag/v2.1.4\">"),
            Some("v2.1.4".to_string())
        );
        assert_eq!(extract_tag_from_html("no tag here"), None);
    }

    #[test]
    fn sanitizes_release_tags() {
        assert_eq!(sanitize_tag("v2.1.4", "2.1.4"), "v2.1.4");
        assert_eq!(sanitize_tag("../../evil", "2.1.4"), "v2.1.4");
        assert_eq!(sanitize_tag("bad/tag", "2.1.4"), "v2.1.4");
    }
}
