use super::*;

async fn run_session_blocking<T, F>(task: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    tokio::task::spawn_blocking(task)
        .await
        .map_err(|e| format!("Session file task failed: {}", e))?
}

/// Parse Claude Code session file to extract info (first_message, git_branch, summary)
/// Returns (first_message, git_branch, summary)
fn parse_claude_session_info(file_path: &std::path::Path) -> (String, String, String) {
    use std::io::{BufRead, BufReader};

    let mut first_message = String::new();
    let mut git_branch = String::new();
    let mut summary = String::new();

    // Check file size to avoid reading very large files entirely
    let file_size = file_path.metadata().map(|m| m.len()).unwrap_or(0);
    let should_limit_read = file_size > 10 * 1024 * 1024; // 10MB

    let file = match std::fs::File::open(file_path) {
        Ok(f) => f,
        Err(_) => return (first_message, git_branch, summary),
    };

    let reader = BufReader::new(file);
    let mut lines_read = 0;
    let max_lines = if should_limit_read { 50 } else { 200 };

    for line in reader.lines() {
        if lines_read >= max_lines {
            break;
        }
        lines_read += 1;

        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let data: serde_json::Value = match serde_json::from_str(line) {
            Ok(d) => d,
            Err(_) => continue,
        };

        // Extract summary
        if data.get("type").and_then(|t| t.as_str()) == Some("summary") {
            if let Some(s) = data.get("summary").and_then(|s| s.as_str()) {
                summary = s.to_string();
            }
        }

        // Extract git branch
        if git_branch.is_empty() {
            if let Some(branch) = data.get("gitBranch").and_then(|b| b.as_str()) {
                git_branch = branch.to_string();
            }
        }

        // Extract first message from user type
        if first_message.is_empty() && data.get("type").and_then(|t| t.as_str()) == Some("user") {
            if let Some(message) = data.get("message") {
                if let Some(content) = message.get("content") {
                    let text = if let Some(content_str) = content.as_str() {
                        // content is a string
                        if content_str != "Warmup" {
                            content_str.chars().take(200).collect::<String>()
                        } else {
                            String::new()
                        }
                    } else if let Some(content_arr) = content.as_array() {
                        // content is an array of items
                        let mut text_parts = Vec::new();
                        for item in content_arr {
                            if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                                if let Some(t) = item.get("text").and_then(|t| t.as_str()) {
                                    text_parts.push(t);
                                }
                            }
                        }
                        let joined = text_parts.join("\n");
                        if !joined.is_empty() && joined != "Warmup" {
                            joined.chars().take(200).collect::<String>()
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };

                    if !text.is_empty() {
                        first_message = text;
                    }
                }
            }
        }
    }

    (first_message, git_branch, summary)
}

/// Decode Claude Code project name to (display_name, full_path)
/// Format: D--my-develop-project-other -> ("other", "D:\\my-develop\\project\\other")
fn decode_claude_project_name(encoded_name: &str) -> (String, String) {
    #[cfg(target_os = "windows")]
    {
        // Windows format: D--path-parts (drive letter + double dash + path with single dashes)
        if encoded_name.len() >= 3
            && encoded_name.chars().nth(1) == Some('-')
            && encoded_name.chars().nth(2) == Some('-')
        {
            let drive = encoded_name
                .chars()
                .next()
                .unwrap()
                .to_uppercase()
                .to_string();
            let path_part = &encoded_name[3..]; // Skip "D--"
            let path_parts: Vec<&str> = path_part.split('-').collect();
            let full_path = format!("{}:\\{}", drive, path_parts.join("\\"));
            let display_name = path_parts.last().unwrap_or(&encoded_name).to_string();
            return (display_name, full_path);
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Unix format: starts with - then path parts separated by -
        if encoded_name.starts_with("-") {
            let parts: Vec<&str> = encoded_name[1..].split('-').collect();
            let full_path = format!("/{}", parts.join("/"));
            let display_name = parts.last().unwrap_or(&encoded_name).to_string();
            return (display_name, full_path);
        }
    }
    (encoded_name.to_string(), encoded_name.to_string())
}

// Extract cwd from Codex session file
fn extract_codex_cwd(file_path: &std::path::Path) -> Option<String> {
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(file_path).ok()?;
    let reader = BufReader::new(file);

    for line in reader.lines().flatten().take(50) {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&line) {
            if data.get("type").and_then(|t| t.as_str()) == Some("session_meta") {
                if let Some(cwd) = data
                    .get("payload")
                    .and_then(|p| p.get("cwd"))
                    .and_then(|c| c.as_str())
                {
                    return Some(cwd.to_string());
                }
            }
        }
    }
    None
}

// Handle Codex projects (group sessions by cwd)
fn get_codex_projects(
    sessions_dir: std::path::PathBuf,
    page: i64,
    page_size: i64,
) -> Result<PaginatedProjects> {
    use std::collections::HashMap;
    use walkdir::WalkDir;

    if !sessions_dir.exists() {
        return Ok(PaginatedProjects {
            items: vec![],
            total: 0,
            page,
            page_size,
        });
    }

    // Group sessions by cwd (search recursively in date subdirectories)
    let mut project_map: HashMap<String, Vec<(std::path::PathBuf, std::fs::Metadata)>> =
        HashMap::new();

    // Use WalkDir to recursively search all subdirectories
    for entry in WalkDir::new(&sessions_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if filename.starts_with("rollout-") && filename.ends_with(".jsonl") {
                if let Some(cwd) = extract_codex_cwd(path) {
                    if let Ok(meta) = path.metadata() {
                        project_map
                            .entry(cwd)
                            .or_insert_with(Vec::new)
                            .push((path.to_path_buf(), meta));
                    }
                }
            }
        }
    }

    // Build project list
    let mut projects_data: Vec<(String, String, usize, i64, f64)> = Vec::new();
    for (cwd, files) in project_map {
        let total_size: i64 = files.iter().map(|(_, m)| m.len() as i64).sum();
        let last_modified = files
            .iter()
            .filter_map(|(_, m)| m.modified().ok())
            .map(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs_f64())
                    .unwrap_or(0.0)
            })
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);

        let display_name = std::path::Path::new(&cwd)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        projects_data.push((
            cwd.clone(),
            display_name,
            files.len(),
            total_size,
            last_modified,
        ));
    }

    // Sort by last_modified descending
    projects_data.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap_or(std::cmp::Ordering::Equal));

    let total = projects_data.len() as i64;
    let start = ((page - 1) * page_size) as usize;
    let items: Vec<_> = projects_data
        .into_iter()
        .skip(start)
        .take(page_size as usize)
        .map(
            |(cwd, display_name, session_count, total_size, last_modified)| ProjectInfo {
                name: cwd.clone(),
                display_name,
                full_path: cwd,
                session_count: session_count as i64,
                total_size,
                last_modified,
            },
        )
        .collect();

    Ok(PaginatedProjects {
        items,
        total,
        page,
        page_size,
    })
}

/// Calculate SHA256 hash of a path (same as Gemini CLI)
fn get_path_hash(path: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Build hash -> path mapping for Gemini projects using rainbow table method
fn build_gemini_path_mapping(
    target_hashes: &std::collections::HashSet<String>,
) -> std::collections::HashMap<String, String> {
    use std::collections::HashMap;

    let mut results: HashMap<String, String> = HashMap::new();
    let home = dirs::home_dir().unwrap_or_default();

    // Define search paths with max depth
    let mut search_paths: Vec<(std::path::PathBuf, usize)> = vec![
        (home.clone(), 0),
        (home.join("Desktop"), 4),
        (home.join("Documents"), 4),
        (home.join("Downloads"), 3),
        (home.join("Projects"), 4),
        (home.join("Code"), 4),
        (home.join("workspace"), 4),
        (home.join("dev"), 4),
        (home.join("src"), 4),
        (home.join("work"), 4),
        (home.join("repos"), 4),
        (home.join("github"), 4),
    ];

    // Windows specific paths
    #[cfg(target_os = "windows")]
    {
        for drive in ["C:", "D:", "E:", "F:"] {
            let drive_path = std::path::PathBuf::from(format!("{}\\", drive));
            if drive_path.exists() {
                search_paths.extend(vec![
                    (drive_path.join("Projects"), 4),
                    (drive_path.join("Code"), 4),
                    (drive_path.join("workspace"), 4),
                    (drive_path.join("dev"), 4),
                ]);
            }
        }
    }

    fn scan_dir(
        dir_path: &std::path::Path,
        max_depth: usize,
        current_depth: usize,
        target_hashes: &std::collections::HashSet<String>,
        results: &mut std::collections::HashMap<String, String>,
    ) {
        if current_depth > max_depth || results.len() >= target_hashes.len() {
            return;
        }

        // Calculate hash for current directory
        let path_str = dir_path.to_string_lossy().to_string();
        let path_hash = get_path_hash(&path_str);
        if target_hashes.contains(&path_hash) && !results.contains_key(&path_hash) {
            results.insert(path_hash, path_str);
        }

        if results.len() >= target_hashes.len() {
            return;
        }

        // Scan subdirectories
        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let item_path = entry.path();
                if !item_path.is_dir() {
                    continue;
                }

                let name = item_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                // Skip hidden and common irrelevant directories
                if name.starts_with('.')
                    || name == "node_modules"
                    || name == "venv"
                    || name == "__pycache__"
                    || name == "Library"
                    || name == "Applications"
                    || name == "target"
                    || name == "dist"
                    || name == "build"
                {
                    continue;
                }

                scan_dir(
                    &item_path,
                    max_depth,
                    current_depth + 1,
                    target_hashes,
                    results,
                );
                if results.len() >= target_hashes.len() {
                    return;
                }
            }
        }
    }

    for (search_path, depth) in search_paths {
        if search_path.exists() {
            scan_dir(&search_path, depth, 0, target_hashes, &mut results);
        }
        if results.len() >= target_hashes.len() {
            break;
        }
    }

    results
}

// Handle Gemini projects (from hash directories with chats subfolder)
fn get_gemini_projects(
    tmp_dir: std::path::PathBuf,
    page: i64,
    page_size: i64,
) -> Result<PaginatedProjects> {
    use std::collections::HashSet;

    if !tmp_dir.exists() {
        return Ok(PaginatedProjects {
            items: vec![],
            total: 0,
            page,
            page_size,
        });
    }

    let mut project_dirs: Vec<(std::path::PathBuf, f64)> = Vec::new();
    let mut all_hashes: HashSet<String> = HashSet::new();

    if let Ok(entries) = std::fs::read_dir(&tmp_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let chats_dir = path.join("chats");
            if chats_dir.exists() {
                if let Ok(meta) = path.metadata() {
                    if let Ok(mtime) = meta.modified() {
                        let secs = mtime
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs_f64())
                            .unwrap_or(0.0);

                        // Check if it's a valid 64-char hex hash
                        if name.len() == 64 && name.chars().all(|c| c.is_ascii_hexdigit()) {
                            all_hashes.insert(name.clone());
                        }
                        project_dirs.push((path, secs));
                    }
                }
            }
        }
    }

    // Sort by last_modified descending
    project_dirs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let total = project_dirs.len() as i64;
    let start = ((page - 1) * page_size) as usize;
    let page_dirs: Vec<_> = project_dirs
        .into_iter()
        .skip(start)
        .take(page_size as usize)
        .collect();

    // Build path mapping using rainbow table method
    let path_mapping = build_gemini_path_mapping(&all_hashes);

    let mut projects = Vec::new();
    for (path, _) in page_dirs {
        let hash_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let chats_dir = path.join("chats");
        let mut session_count = 0i64;
        let mut total_size = 0i64;
        let mut last_modified = 0f64;

        if let Ok(entries) = std::fs::read_dir(&chats_dir) {
            for entry in entries.flatten() {
                let session_path = entry.path();
                if session_path.is_file() {
                    let filename = session_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");

                    if filename.starts_with("session-") && filename.ends_with(".json") {
                        session_count += 1;
                        if let Ok(meta) = session_path.metadata() {
                            total_size += meta.len() as i64;
                            if let Ok(mtime) = meta.modified() {
                                let secs = mtime
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_secs_f64())
                                    .unwrap_or(0.0);
                                if secs > last_modified {
                                    last_modified = secs;
                                }
                            }
                        }
                    }
                }
            }
        }

        if session_count > 0 {
            let is_hash = hash_name.len() == 64 && hash_name.chars().all(|c| c.is_ascii_hexdigit());

            // Try to get project path from rainbow table
            let (display_name, full_path) = if is_hash {
                if let Some(real_path) = path_mapping.get(hash_name) {
                    let name = std::path::Path::new(real_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&format!("Project {}", &hash_name[..8]))
                        .to_string();
                    (name, real_path.clone())
                } else {
                    (
                        format!("Project {}", &hash_name[..8]),
                        hash_name.to_string(),
                    )
                }
            } else {
                (hash_name.to_string(), hash_name.to_string())
            };

            projects.push(ProjectInfo {
                name: hash_name.to_string(),
                display_name,
                full_path,
                session_count,
                total_size,
                last_modified,
            });
        }
    }

    Ok(PaginatedProjects {
        items: projects,
        total,
        page,
        page_size,
    })
}

// Handle Codex sessions (find by cwd)
fn get_codex_sessions(
    sessions_dir: std::path::PathBuf,
    project_name: &str,
    page: i64,
    page_size: i64,
) -> Result<PaginatedSessions> {
    use std::io::{BufRead, BufReader};
    use walkdir::WalkDir;

    if !sessions_dir.exists() {
        return Ok(PaginatedSessions {
            items: vec![],
            total: 0,
            page,
            page_size,
        });
    }

    let mut session_files: Vec<(std::path::PathBuf, std::fs::Metadata)> = Vec::new();

    // Use WalkDir to recursively search all subdirectories
    for entry in WalkDir::new(&sessions_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if filename.starts_with("rollout-") && filename.ends_with(".jsonl") {
                if let Some(cwd) = extract_codex_cwd(path) {
                    if cwd == project_name {
                        if let Ok(meta) = path.metadata() {
                            session_files.push((path.to_path_buf(), meta));
                        }
                    }
                }
            }
        }
    }

    // Sort by mtime descending
    session_files.sort_by(|a, b| {
        let a_mtime =
            a.1.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
        let b_mtime =
            b.1.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
        b_mtime
            .partial_cmp(&a_mtime)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let total = session_files.len() as i64;
    let start = ((page - 1) * page_size) as usize;
    let page_files: Vec<_> = session_files
        .into_iter()
        .skip(start)
        .take(page_size as usize)
        .collect();

    let mut sessions = Vec::new();
    for (path, meta) in page_files {
        let session_id = path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let size = meta.len() as i64;
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);

        // Try to extract first message
        let mut first_message = String::new();
        if let Ok(file) = std::fs::File::open(&path) {
            let reader = BufReader::new(file);
            for line in reader.lines().flatten().take(200) {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&line) {
                    if data.get("type").and_then(|t| t.as_str()) == Some("event_msg") {
                        if let Some(payload) = data.get("payload") {
                            if payload.get("type").and_then(|t| t.as_str()) == Some("user_message")
                            {
                                if let Some(msg) = payload.get("message").and_then(|m| m.as_str()) {
                                    first_message = msg.chars().take(200).collect();
                                    break;
                                } else if let Some(arr) =
                                    payload.get("message").and_then(|m| m.as_array())
                                {
                                    let mut text_parts = Vec::new();
                                    for item in arr {
                                        if let Some(text) =
                                            item.get("text").and_then(|t| t.as_str())
                                        {
                                            text_parts.push(text);
                                        }
                                    }
                                    let joined = text_parts.join("\n");
                                    if !joined.is_empty() {
                                        first_message = joined.chars().take(200).collect();
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        sessions.push(SessionInfo {
            session_id,
            size,
            mtime,
            first_message,
            git_branch: String::new(),
            summary: String::new(),
        });
    }

    Ok(PaginatedSessions {
        items: sessions,
        total,
        page,
        page_size,
    })
}

// Handle Codex sessions (find by cwd) - 异步版本，支持自定义配置目录
async fn get_codex_sessions_async(
    db: &SqlitePool,
    agent_id: &str,
    project_name: &str,
    page: i64,
    page_size: i64,
) -> Result<PaginatedSessions> {
    let config_dir = get_cli_config_dir_path(db, agent_id).await;
    let sessions_dir = config_dir.join("sessions");
    let project_name = project_name.to_string();

    run_session_blocking(move || get_codex_sessions(sessions_dir, &project_name, page, page_size))
        .await
}

// Handle Gemini sessions
fn get_gemini_sessions(
    chats_dir: std::path::PathBuf,
    page: i64,
    page_size: i64,
) -> Result<PaginatedSessions> {
    if !chats_dir.exists() {
        return Ok(PaginatedSessions {
            items: vec![],
            total: 0,
            page,
            page_size,
        });
    }

    let mut session_files: Vec<(std::path::PathBuf, std::fs::Metadata)> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&chats_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if filename.starts_with("session-") && filename.ends_with(".json") {
                    if let Ok(meta) = path.metadata() {
                        session_files.push((path, meta));
                    }
                }
            }
        }
    }

    // Sort by mtime descending
    session_files.sort_by(|a, b| {
        let a_mtime =
            a.1.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
        let b_mtime =
            b.1.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
        b_mtime
            .partial_cmp(&a_mtime)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let total = session_files.len() as i64;
    let start = ((page - 1) * page_size) as usize;
    let page_files: Vec<_> = session_files
        .into_iter()
        .skip(start)
        .take(page_size as usize)
        .collect();

    let mut sessions = Vec::new();
    for (path, meta) in page_files {
        let session_id = path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let size = meta.len() as i64;
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);

        // Try to extract first message
        let mut first_message = String::new();
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(messages) = json.get("messages").and_then(|m| m.as_array()) {
                    for msg in messages {
                        if msg.get("type").and_then(|t| t.as_str()) == Some("user") {
                            if let Some(content_val) = msg.get("content") {
                                if let Some(text) = content_val.as_str() {
                                    first_message = text.chars().take(200).collect();
                                    break;
                                } else if let Some(arr) = content_val.as_array() {
                                    for item in arr {
                                        if let Some(text) =
                                            item.get("text").and_then(|t| t.as_str())
                                        {
                                            first_message = text.chars().take(200).collect();
                                            break;
                                        }
                                    }
                                    if !first_message.is_empty() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        sessions.push(SessionInfo {
            session_id,
            size,
            mtime,
            first_message,
            git_branch: String::new(),
            summary: String::new(),
        });
    }

    Ok(PaginatedSessions {
        items: sessions,
        total,
        page,
        page_size,
    })
}

// Handle Gemini sessions - 异步版本，支持自定义配置目录
async fn get_gemini_sessions_async(
    db: &SqlitePool,
    agent_id: &str,
    project_name: &str,
    page: i64,
    page_size: i64,
) -> Result<PaginatedSessions> {
    let config_dir = get_cli_config_dir_path(db, agent_id).await;
    let chats_dir = config_dir.join("tmp").join(project_name).join("chats");
    run_session_blocking(move || get_gemini_sessions(chats_dir, page, page_size)).await
}

// Parse Codex messages from JSONL file - 异步版本，支持自定义配置目录
async fn get_codex_messages_async(
    db: &SqlitePool,
    agent_id: &str,
    session_id: &str,
) -> Result<Vec<SessionMessage>> {
    use std::io::{BufRead, BufReader};
    use walkdir::WalkDir;

    let config_dir = get_cli_config_dir_path(db, agent_id).await;
    let sessions_dir = config_dir.join("sessions");
    let session_id = session_id.to_string();

    run_session_blocking(move || {
        // Find the session file by searching recursively
        let mut session_file_path: Option<std::path::PathBuf> = None;
        for entry in WalkDir::new(&sessions_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                // Match session_id which is the stem (filename without extension)
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if stem == session_id {
                        session_file_path = Some(path.to_path_buf());
                        break;
                    }
                }
            }
        }

        let session_file =
            session_file_path.ok_or_else(|| format!("Session file not found: {}", session_id))?;

        let file = std::fs::File::open(&session_file)
            .map_err(|e| format!("Failed to open session file: {}", e))?;
        let reader = BufReader::new(file);

        let mut messages = Vec::new();

        for line in reader.lines().flatten() {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&line) {
                let msg_type = data.get("type").and_then(|t| t.as_str());

                // Only process response_item for structured messages
                if msg_type == Some("response_item") {
                    if let Some(payload) = data.get("payload") {
                        let item_type = payload.get("type").and_then(|t| t.as_str());
                        let role = payload.get("role").and_then(|r| r.as_str());
                        let timestamp = data.get("timestamp").and_then(|t| t.as_i64());

                        // User messages
                        if role == Some("user") && item_type == Some("message") {
                            if let Some(content_list) =
                                payload.get("content").and_then(|c| c.as_array())
                            {
                                let text_parts: Vec<String> = content_list
                                    .iter()
                                    .filter_map(|item| {
                                        if item.get("type").and_then(|t| t.as_str())
                                            == Some("input_text")
                                        {
                                            item.get("text")
                                                .and_then(|t| t.as_str())
                                                .map(|s| s.to_string())
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                                if !text_parts.is_empty() {
                                    messages.push(SessionMessage {
                                        role: "user".to_string(),
                                        content: text_parts.join("\n\n"),
                                        timestamp,
                                    });
                                }
                            }
                        }
                        // Assistant messages
                        else if role == Some("assistant") && item_type == Some("message") {
                            if let Some(content_list) =
                                payload.get("content").and_then(|c| c.as_array())
                            {
                                let text_parts: Vec<String> = content_list
                                    .iter()
                                    .filter_map(|item| {
                                        let item_type = item.get("type").and_then(|t| t.as_str());
                                        if item_type == Some("output_text")
                                            || item_type == Some("text")
                                        {
                                            item.get("text")
                                                .and_then(|t| t.as_str())
                                                .map(|s| s.to_string())
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                                if !text_parts.is_empty() {
                                    messages.push(SessionMessage {
                                        role: "assistant".to_string(),
                                        content: text_parts.join("\n\n"),
                                        timestamp,
                                    });
                                }
                            }
                        }
                        // Reasoning summary
                        else if item_type == Some("reasoning") {
                            let summary = payload.get("summary").and_then(|s| s.as_array());
                            if let Some(summary_arr) = summary {
                                let text_parts: Vec<String> = summary_arr
                                    .iter()
                                    .filter_map(|item| {
                                        if item.get("type").and_then(|t| t.as_str())
                                            == Some("summary_text")
                                        {
                                            item.get("text")
                                                .and_then(|t| t.as_str())
                                                .map(|s| s.to_string())
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                                if !text_parts.is_empty() {
                                    messages.push(SessionMessage {
                                        role: "assistant".to_string(),
                                        content: format!("**[推理]**\n{}", text_parts.join("\n")),
                                        timestamp,
                                    });
                                }
                            }
                        }
                        // Function call (tool use)
                        else if item_type == Some("function_call") {
                            let name = payload
                                .get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("unknown");
                            let arguments = payload
                                .get("arguments")
                                .and_then(|a| a.as_str())
                                .unwrap_or("{}");
                            let args_str =
                                match serde_json::from_str::<serde_json::Value>(arguments) {
                                    Ok(args_obj) => serde_json::to_string_pretty(&args_obj)
                                        .unwrap_or_else(|_| arguments.to_string()),
                                    Err(_) => arguments.to_string(),
                                };
                            messages.push(SessionMessage {
                                role: "assistant".to_string(),
                                content: format!(
                                    "**[调用工具: {}]**\n```json\n{}\n```",
                                    name, args_str
                                ),
                                timestamp,
                            });
                        }
                        // Function call output (tool result)
                        else if item_type == Some("function_call_output") {
                            let output =
                                payload.get("output").and_then(|o| o.as_str()).unwrap_or("");
                            if !output.is_empty() {
                                messages.push(SessionMessage {
                                    role: "user".to_string(),
                                    content: format!("**[工具结果]**\n```\n{}\n```", output),
                                    timestamp,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(messages)
    })
    .await
}

// Parse Claude Code messages from JSONL content
fn parse_claude_jsonl(content: &str) -> Result<Vec<SessionMessage>> {
    use std::io::{BufRead, BufReader};

    let mut messages = Vec::new();
    let reader = BufReader::new(content.as_bytes());

    for line in reader.lines().flatten() {
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&line) {
            let msg_type = data.get("type").and_then(|t| t.as_str());

            if msg_type == Some("user") || msg_type == Some("assistant") {
                let role = msg_type.unwrap();
                let timestamp = data.get("timestamp").and_then(|t| t.as_i64());

                if let Some(message) = data.get("message") {
                    let content_val = message.get("content");

                    let content = if let Some(arr) = content_val.and_then(|c| c.as_array()) {
                        let mut text_parts = Vec::new();
                        for item in arr {
                            if let Some(item_type) = item.get("type").and_then(|t| t.as_str()) {
                                match item_type {
                                    "text" => {
                                        if let Some(text) =
                                            item.get("text").and_then(|t| t.as_str())
                                        {
                                            text_parts.push(text.to_string());
                                        }
                                    }
                                    "tool_use" if role == "assistant" => {
                                        // Tool call from assistant
                                        let tool_name = item
                                            .get("name")
                                            .and_then(|n| n.as_str())
                                            .unwrap_or("unknown");
                                        let tool_input = item.get("input");
                                        let input_str = if let Some(input) = tool_input {
                                            serde_json::to_string_pretty(input)
                                                .unwrap_or_else(|_| "{}".to_string())
                                        } else {
                                            "{}".to_string()
                                        };
                                        text_parts.push(format!(
                                            "**[调用工具: {}]**\n```json\n{}\n```",
                                            tool_name, input_str
                                        ));
                                    }
                                    "tool_result" if role == "user" => {
                                        // Tool result from user
                                        let result_content = item.get("content");
                                        let result_str = if let Some(content) = result_content {
                                            if let Some(s) = content.as_str() {
                                                s.to_string()
                                            } else {
                                                serde_json::to_string_pretty(content)
                                                    .unwrap_or_else(|_| "".to_string())
                                            }
                                        } else {
                                            String::new()
                                        };
                                        if !result_str.is_empty() {
                                            text_parts.push(format!(
                                                "**[工具结果]**\n```\n{}\n```",
                                                result_str
                                            ));
                                        }
                                    }
                                    "thinking" if role == "assistant" => {
                                        // Thinking from assistant
                                        if let Some(thinking) =
                                            item.get("thinking").and_then(|t| t.as_str())
                                        {
                                            if !thinking.is_empty() {
                                                text_parts
                                                    .push(format!("**[思考]**\n{}", thinking));
                                            }
                                        }
                                    }
                                    "image" => {
                                        text_parts.push("[图片]".to_string());
                                    }
                                    _ => {}
                                }
                            }
                        }
                        text_parts.join("\n\n")
                    } else if let Some(text) = content_val.and_then(|c| c.as_str()) {
                        text.to_string()
                    } else {
                        continue;
                    };

                    if !content.is_empty() && content != "Warmup" {
                        messages.push(SessionMessage {
                            role: role.to_string(),
                            content,
                            timestamp,
                        });
                    }
                }
            }
        }
    }

    Ok(messages)
}

fn get_session_messages_from_file(
    cli_type: &str,
    session_file: std::path::PathBuf,
) -> Result<Vec<SessionMessage>> {
    let content = std::fs::read_to_string(&session_file)
        .map_err(|e| format!("Failed to read session file: {}", e))?;

    parse_session_messages_content(cli_type, &content)
}

fn parse_session_messages_content(cli_type: &str, content: &str) -> Result<Vec<SessionMessage>> {
    // For Claude Code JSONL format
    if cli_type == "claude_code" {
        return parse_claude_jsonl(content);
    }

    // For Gemini JSON format
    let json: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| format!("Failed to parse session JSON: {}", e))?;

    let mut messages = Vec::new();

    // Try to parse messages in different formats
    if let Some(msgs) = json.get("messages").and_then(|m| m.as_array()) {
        // Standard format with messages array
        for msg in msgs {
            let msg_type = msg.get("type").and_then(|t| t.as_str()).unwrap_or("");

            let timestamp = msg
                .get("timestamp")
                .and_then(|t| t.as_str())
                .map(|s| {
                    chrono::DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|dt| dt.timestamp())
                })
                .flatten();

            if msg_type == "user" {
                // User message
                let mut text_parts = Vec::new();
                if let Some(content_val) = msg.get("content") {
                    if let Some(text) = content_val.as_str() {
                        text_parts.push(text.to_string());
                    } else if let Some(arr) = content_val.as_array() {
                        for item in arr {
                            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                                text_parts.push(text.to_string());
                            }
                        }
                    }
                }

                let content = text_parts.join("\n\n");

                if !content.is_empty() {
                    messages.push(SessionMessage {
                        role: "user".to_string(),
                        content,
                        timestamp,
                    });
                }
            } else if msg_type == "gemini" || msg_type == "assistant" || msg_type == "ai" {
                // Gemini/Assistant message - may contain content, thoughts, and toolCalls
                let mut text_parts = Vec::new();

                // Get main content
                if let Some(content_val) = msg.get("content") {
                    if let Some(text) = content_val.as_str() {
                        if !text.is_empty() {
                            text_parts.push(text.to_string());
                        }
                    }
                }

                // Handle thoughts
                if let Some(thoughts) = msg.get("thoughts").and_then(|t| t.as_array()) {
                    for thought in thoughts {
                        if let Some(desc) = thought.get("description").and_then(|d| d.as_str()) {
                            if !desc.is_empty() {
                                text_parts.push(format!("**[思考]**\n{}", desc));
                            }
                        }
                    }
                }

                // Handle tool calls
                if let Some(tool_calls) = msg.get("toolCalls").and_then(|t| t.as_array()) {
                    for tool_call in tool_calls {
                        let tool_name = tool_call
                            .get("displayName")
                            .or_else(|| tool_call.get("name"))
                            .and_then(|n| n.as_str())
                            .unwrap_or("unknown");
                        let result_display = tool_call
                            .get("resultDisplay")
                            .and_then(|r| r.as_str())
                            .unwrap_or("");
                        if !result_display.is_empty() {
                            text_parts
                                .push(format!("**[工具: {}]**\n{}", tool_name, result_display));
                        }
                    }
                }

                let final_content = text_parts.join("\n\n");
                if !final_content.is_empty() {
                    messages.push(SessionMessage {
                        role: "assistant".to_string(),
                        content: final_content,
                        timestamp,
                    });
                }
            }
        }
    } else if let Some(conversation) = json.as_object() {
        // Try to parse as flat object with role-based keys
        for (key, value) in conversation {
            if key == "id" || key == "title" || key == "created_at" || key == "updated_at" {
                continue;
            }
            let role = if key.starts_with("user") || key.starts_with("human") {
                "user"
            } else if key.starts_with("assistant") || key.starts_with("ai") {
                "assistant"
            } else {
                continue;
            };

            if let Some(text) = value.as_str() {
                messages.push(SessionMessage {
                    role: role.to_string(),
                    content: text.to_string(),
                    timestamp: None,
                });
            }
        }
    }

    Ok(messages)
}

async fn session_adapter(db: &SqlitePool, agent_id: &str) -> Result<String> {
    let resolved = crate::services::agent::get_agent(db, agent_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("未知 Agent: {}", agent_id))?;
    let feature = resolved.features.sessions;
    if !feature.enabled {
        return Err(format!("Agent {} 的 Session 功能未启用", agent_id));
    }
    let adapter = feature
        .adapter
        .ok_or_else(|| format!("Agent {} 的 Session 缺少 adapter", agent_id))?;
    if !matches!(adapter.as_str(), "claude_code" | "codex" | "gemini") {
        return Err(format!("Session adapter `{}` 不存在", adapter));
    }
    Ok(adapter)
}

// Session commands
#[tauri::command]
pub async fn get_session_projects(
    db: State<'_, SqlitePool>,
    cli_type: String,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<PaginatedProjects> {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).clamp(1, 100);
    let adapter = session_adapter(db.inner(), &cli_type).await?;

    let base_dir = get_cli_base_dir_async(db.inner(), &cli_type).await;
    let projects_dir = cli_helpers::projects_dir(&base_dir, &adapter);

    // For Codex, we need special handling since sessions are not in project folders
    if adapter == "codex" {
        return run_session_blocking(move || get_codex_projects(projects_dir, page, page_size))
            .await;
    }

    // For Gemini, check if sessions are in hash directories with chats subfolder
    if adapter == "gemini" {
        return run_session_blocking(move || get_gemini_projects(projects_dir, page, page_size))
            .await;
    }

    run_session_blocking(move || {
        let mut projects = Vec::new();

        if projects_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&projects_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string();

                        if name.is_empty() || name.starts_with('.') {
                            continue;
                        }

                        // Count sessions and calculate size
                        let mut session_count = 0i64;
                        let mut total_size = 0i64;
                        let mut last_modified = 0f64;

                        if let Ok(sessions) = std::fs::read_dir(&path) {
                            for session in sessions.flatten() {
                                let session_path = session.path();
                                if session_path.is_file() {
                                    // Only count .jsonl files, exclude index and agent files
                                    let ext = session_path
                                        .extension()
                                        .and_then(|e| e.to_str())
                                        .unwrap_or("");
                                    if ext != "jsonl" {
                                        continue;
                                    }
                                    let stem = session_path
                                        .file_stem()
                                        .and_then(|s| s.to_str())
                                        .unwrap_or("");
                                    if stem == "sessions-index" || stem.starts_with("agent-") {
                                        continue;
                                    }

                                    session_count += 1;
                                    if let Ok(meta) = session_path.metadata() {
                                        total_size += meta.len() as i64;
                                        if let Ok(mtime) = meta.modified() {
                                            let secs = mtime
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .map(|d| d.as_secs_f64())
                                                .unwrap_or(0.0);
                                            if secs > last_modified {
                                                last_modified = secs;
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        let (display_name, full_path) = if adapter == "claude_code" {
                            // Decode path from project name (format: -D-my-develop-project-other)
                            decode_claude_project_name(&name)
                        } else {
                            (name.clone(), path.to_string_lossy().to_string())
                        };

                        projects.push(ProjectInfo {
                            name: name.clone(),
                            display_name,
                            full_path,
                            session_count,
                            total_size,
                            last_modified,
                        });
                    }
                }
            }
        }

        // Sort by last_modified descending
        projects.sort_by(|a, b| {
            b.last_modified
                .partial_cmp(&a.last_modified)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let total = projects.len() as i64;
        let start = ((page - 1) * page_size) as usize;
        let items: Vec<_> = projects
            .into_iter()
            .skip(start)
            .take(page_size as usize)
            .collect();

        Ok(PaginatedProjects {
            items,
            total,
            page,
            page_size,
        })
    })
    .await
}

#[tauri::command]
pub async fn get_project_sessions(
    db: State<'_, SqlitePool>,
    cli_type: String,
    project_name: String,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<PaginatedSessions> {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).clamp(1, 100);
    let adapter = session_adapter(db.inner(), &cli_type).await?;

    // Special handling for Codex
    if adapter == "codex" {
        return get_codex_sessions_async(db.inner(), &cli_type, &project_name, page, page_size)
            .await;
    }

    // Special handling for Gemini
    if adapter == "gemini" {
        return get_gemini_sessions_async(db.inner(), &cli_type, &project_name, page, page_size)
            .await;
    }

    // Claude Code default handling
    let base_dir = get_cli_base_dir_async(db.inner(), &cli_type).await;
    let project_dir = base_dir.join("projects").join(&project_name);

    run_session_blocking(move || {
        let mut sessions = Vec::new();

        if project_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&project_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        // Only process .jsonl files
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        if ext != "jsonl" {
                            continue;
                        }

                        let session_id = path
                            .file_stem()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string();

                        // Skip empty, index files, and agent files
                        if session_id.is_empty()
                            || session_id == "sessions-index"
                            || session_id.starts_with("agent-")
                        {
                            continue;
                        }

                        let mut size = 0i64;
                        let mut mtime = 0f64;

                        if let Ok(meta) = path.metadata() {
                            size = meta.len() as i64;
                            if let Ok(mt) = meta.modified() {
                                mtime = mt
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_secs_f64())
                                    .unwrap_or(0.0);
                            }
                        }

                        // Try to read first message from JSONL (Claude Code uses JSONL format)
                        let (first_message, git_branch, _) = parse_claude_session_info(&path);

                        sessions.push(SessionInfo {
                            session_id,
                            size,
                            mtime,
                            first_message,
                            git_branch,
                            summary: String::new(),
                        });
                    }
                }
            }
        }

        // Sort by mtime descending
        sessions.sort_by(|a, b| {
            b.mtime
                .partial_cmp(&a.mtime)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let total = sessions.len() as i64;
        let start = ((page - 1) * page_size) as usize;
        let items: Vec<_> = sessions
            .into_iter()
            .skip(start)
            .take(page_size as usize)
            .collect();

        Ok(PaginatedSessions {
            items,
            total,
            page,
            page_size,
        })
    })
    .await
}

#[tauri::command]
pub async fn get_session_messages(
    db: State<'_, SqlitePool>,
    cli_type: String,
    project_name: String,
    session_id: String,
) -> Result<Vec<SessionMessage>> {
    let adapter = session_adapter(db.inner(), &cli_type).await?;
    // Special handling for Codex JSONL format
    if adapter == "codex" {
        return get_codex_messages_async(db.inner(), &cli_type, &session_id).await;
    }

    let base_dir = get_cli_base_dir_async(db.inner(), &cli_type).await;
    let session_file =
        cli_helpers::session_file_path(&base_dir, &adapter, &project_name, &session_id);

    run_session_blocking(move || get_session_messages_from_file(&adapter, session_file)).await
}

#[tauri::command]
pub async fn delete_session(
    db: State<'_, SqlitePool>,
    cli_type: String,
    project_name: String,
    session_id: String,
) -> Result<()> {
    let adapter = session_adapter(db.inner(), &cli_type).await?;
    let base_dir = get_cli_base_dir_async(db.inner(), &cli_type).await;

    // Special handling for Codex - need to search recursively
    if adapter == "codex" {
        let sessions_dir = base_dir.join("sessions");
        return run_session_blocking(move || {
            use walkdir::WalkDir;

            for entry in WalkDir::new(&sessions_dir)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.is_file() {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if stem == session_id {
                            // Verify the cwd matches project_name
                            if let Some(cwd) = extract_codex_cwd(path) {
                                if cwd == project_name {
                                    std::fs::remove_file(path)
                                        .map_err(|e| format!("Failed to delete session: {}", e))?;
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
            Err("Session file not found".to_string())
        })
        .await;
    }

    let session_file =
        cli_helpers::session_file_path(&base_dir, &adapter, &project_name, &session_id);

    if tokio::fs::metadata(&session_file).await.is_err() {
        return Err(format!(
            "Session file not found: {}",
            session_file.display()
        ));
    }

    tokio::fs::remove_file(&session_file).await.map_err(|e| {
        format!(
            "Failed to delete session '{}': {}",
            session_file.display(),
            e
        )
    })?;

    Ok(())
}

#[tauri::command]
pub async fn delete_project(
    db: State<'_, SqlitePool>,
    cli_type: String,
    project_name: String,
) -> Result<()> {
    let adapter = session_adapter(db.inner(), &cli_type).await?;
    let base_dir = get_cli_base_dir_async(db.inner(), &cli_type).await;

    if adapter == "codex" {
        // For Codex, delete all session files matching the project cwd
        let sessions_dir = base_dir.join("sessions");
        return run_session_blocking(move || {
            use walkdir::WalkDir;

            if sessions_dir.exists() {
                // Use WalkDir to recursively search all subdirectories
                for entry in WalkDir::new(&sessions_dir)
                    .follow_links(false)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let path = entry.path();
                    if path.is_file() {
                        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        if filename.starts_with("rollout-") && filename.ends_with(".jsonl") {
                            if let Some(cwd) = extract_codex_cwd(path) {
                                if cwd == project_name {
                                    let _ = std::fs::remove_file(path);
                                }
                            }
                        }
                    }
                }
            }
            Ok(())
        })
        .await;
    }

    // For Claude Code and Gemini, delete the project directory
    let project_dir = cli_helpers::project_dir(&base_dir, &adapter, &project_name);

    tokio::fs::remove_dir_all(&project_dir)
        .await
        .map_err(|e| format!("Failed to delete project: {}", e))?;

    Ok(())
}
