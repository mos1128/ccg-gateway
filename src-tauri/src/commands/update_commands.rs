use super::*;

const GITHUB_OWNER: &str = "mos1128";
const GITHUB_REPO: &str = "ccg-gateway";

#[derive(serde::Serialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub body: Option<String>,
    pub html_url: String,
    pub published_at: Option<String>,
}

#[tauri::command]
pub async fn check_for_updates() -> Result<Option<GitHubRelease>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        GITHUB_OWNER, GITHUB_REPO
    );

    let response = client
        .get(&url)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "ccg-gateway")
        .send()
        .await
        .map_err(|e| format!("网络请求失败: {}", e))?;

    if response.status() == 404 {
        return Ok(None);
    }

    if !response.status().is_success() {
        return Err(format!("GitHub API 错误: {}", response.status()));
    }

    let release: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    Ok(Some(GitHubRelease {
        tag_name: release["tag_name"].as_str().unwrap_or("").to_string(),
        name: release["name"].as_str().map(|s| s.to_string()),
        body: release["body"].as_str().map(|s| s.to_string()),
        html_url: release["html_url"].as_str().unwrap_or("").to_string(),
        published_at: release["published_at"].as_str().map(|s| s.to_string()),
    }))
}
