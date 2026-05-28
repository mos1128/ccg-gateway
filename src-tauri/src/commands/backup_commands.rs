use super::*;

/// 退出应用程序（导入后需要手动重启）
fn exit_application(app: tauri::AppHandle) {
    tokio::spawn(async move {
        // 延迟 3 秒，等待响应返回前端并给用户时间看提示
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        app.exit(0);
    });
}

// Backup commands
#[tauri::command]
pub async fn get_webdav_settings(db: State<'_, SqlitePool>) -> Result<WebdavSettings> {
    // Try to get existing settings
    let settings = sqlx::query_as::<_, WebdavSettings>(
        "SELECT url, username, password FROM webdav_settings WHERE id = 1",
    )
    .fetch_optional(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    match settings {
        Some(s) => Ok(s),
        None => {
            // Create default settings
            let now = now_timestamp();
            sqlx::query(
                "INSERT INTO webdav_settings (id, url, username, password, updated_at) VALUES (1, '', '', '', ?)"
            )
            .bind(now)
            .execute(db.inner())
            .await
            .map_err(map_db_error)?;

            Ok(WebdavSettings {
                url: String::new(),
                username: String::new(),
                password: String::new(),
            })
        }
    }
}

#[tauri::command]
pub async fn update_webdav_settings(
    db: State<'_, SqlitePool>,
    input: WebdavSettingsUpdate,
) -> Result<WebdavSettings> {
    let now = now_timestamp();
    let current = get_webdav_settings(db.clone()).await?;

    sqlx::query(
        "UPDATE webdav_settings SET url = ?, username = ?, password = ?, updated_at = ? WHERE id = 1"
    )
    .bind(input.url.unwrap_or(current.url))
    .bind(input.username.unwrap_or(current.username))
    .bind(input.password.unwrap_or(current.password))
    .bind(now)
    .execute(db.inner())
    .await
    .map_err(map_db_error)?;

    get_webdav_settings(db).await
}

#[tauri::command]
pub async fn test_webdav_connection(
    url: String,
    username: String,
    password: String,
) -> Result<bool> {
    use reqwest::Client;

    let client = Client::new();
    let response = client
        .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
        .basic_auth(&username, Some(&password))
        .header("Depth", "0")
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    Ok(response.status().is_success() || response.status().as_u16() == 207)
}

#[tauri::command]
pub async fn export_to_local_path(path: String) -> Result<()> {
    let db_path = get_data_dir().join("ccg_gateway.db");
    let content = tokio::fs::read(&db_path)
        .await
        .map_err(|e| format!("Failed to read database: {}", e))?;
    tokio::fs::write(&path, &content)
        .await
        .map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn import_from_local(app: tauri::AppHandle, data: Vec<u8>) -> Result<()> {
    let db_path = get_data_dir().join("ccg_gateway.db");

    // Write the database file
    tokio::fs::write(&db_path, &data)
        .await
        .map_err(|e| format!("Failed to write database: {}", e))?;

    // 退出应用，用户需手动重启
    exit_application(app);

    Ok(())
}

#[tauri::command]
pub async fn export_to_webdav(db: State<'_, SqlitePool>) -> Result<String> {
    use reqwest::Client;

    let settings = get_webdav_settings(db.clone()).await?;
    if settings.url.is_empty() {
        return Err("WebDAV URL not configured".to_string());
    }

    // Read database file
    let db_path = get_data_dir().join("ccg_gateway.db");
    let content = tokio::fs::read(&db_path)
        .await
        .map_err(|e| format!("Failed to read database: {}", e))?;

    // Generate filename
    let filename = format!("ccg_gateway_{}.db", local_compact_datetime());

    // Ensure remote directory exists
    let client = Client::new();
    let remote_dir = format!("{}/ccg-gateway-backup", settings.url.trim_end_matches('/'));

    // Try to create directory (ignore error if exists)
    let _ = client
        .request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), &remote_dir)
        .basic_auth(&settings.username, Some(&settings.password))
        .send()
        .await;

    // Upload file
    let remote_file = format!("{}/{}", remote_dir, filename);
    let response = client
        .put(&remote_file)
        .basic_auth(&settings.username, Some(&settings.password))
        .body(content)
        .send()
        .await
        .map_err(|e| format!("Upload failed: {}", e))?;

    if !response.status().is_success() && response.status().as_u16() != 201 {
        return Err(format!("Upload failed with status: {}", response.status()));
    }

    Ok(filename)
}

#[tauri::command]
pub async fn list_webdav_backups(db: State<'_, SqlitePool>) -> Result<Vec<WebdavBackup>> {
    use reqwest::Client;

    let settings = get_webdav_settings(db).await?;
    if settings.url.is_empty() {
        return Err("WebDAV URL not configured".to_string());
    }

    let client = Client::new();
    let remote_dir = format!("{}/ccg-gateway-backup", settings.url.trim_end_matches('/'));

    let response = client
        .request(
            reqwest::Method::from_bytes(b"PROPFIND").unwrap(),
            &remote_dir,
        )
        .basic_auth(&settings.username, Some(&settings.password))
        .header("Depth", "1")
        .header("Content-Type", "application/xml")
        .body(
            r#"<?xml version="1.0" encoding="utf-8"?>
            <propfind xmlns="DAV:">
                <prop>
                    <getcontentlength/>
                    <getlastmodified/>
                </prop>
            </propfind>"#,
        )
        .send()
        .await
        .map_err(|e| format!("Failed to list backups: {}", e))?;

    if !response.status().is_success() && response.status().as_u16() != 207 {
        return Ok(Vec::new());
    }

    let body = response.text().await.map_err(|e| e.to_string())?;

    // Parse XML response using quick-xml
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(&body);
    reader.config_mut().trim_text(true);

    let mut backups = Vec::new();
    let mut current_href = String::new();
    let mut current_size: i64 = 0;
    let mut current_modified = String::new();
    let mut in_response = false;
    let mut current_tag = String::new();

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name.ends_with(":response") || name == "response" {
                    in_response = true;
                    current_href.clear();
                    current_size = 0;
                    current_modified.clear();
                }
                current_tag = name;
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default().trim().to_string();
                if in_response && !text.is_empty() {
                    if current_tag.ends_with(":href") || current_tag == "href" {
                        current_href = text;
                    } else if current_tag.ends_with(":getcontentlength")
                        || current_tag == "getcontentlength"
                    {
                        current_size = text.parse::<i64>().unwrap_or(0);
                    } else if current_tag.ends_with(":getlastmodified")
                        || current_tag == "getlastmodified"
                    {
                        current_modified = text;
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name.ends_with(":response") || name == "response" {
                    in_response = false;

                    // Check if this is a .db file we care about
                    if current_href.contains("ccg_gateway_") && current_href.ends_with(".db") {
                        // Extract filename from href
                        if let Some(start) = current_href.rfind('/') {
                            let filename = current_href[start + 1..].to_string();
                            if filename.starts_with("ccg_gateway_") {
                                backups.push(WebdavBackup {
                                    filename,
                                    size: current_size,
                                    modified: current_modified.clone(),
                                });
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(format!(
                    "XML parse error at position {}: {}",
                    reader.buffer_position(),
                    e
                ))
            }
            _ => {}
        }
        buf.clear();
    }

    // Sort by filename descending (newest first based on timestamp in name)
    backups.sort_by(|a, b| b.filename.cmp(&a.filename));

    Ok(backups)
}

#[tauri::command]
pub async fn import_from_webdav(app: tauri::AppHandle, db: State<'_, SqlitePool>, filename: String) -> Result<()> {
    use reqwest::Client;

    let settings = get_webdav_settings(db).await?;
    if settings.url.is_empty() {
        return Err("WebDAV URL not configured".to_string());
    }

    let client = Client::new();
    let remote_file = format!(
        "{}/ccg-gateway-backup/{}",
        settings.url.trim_end_matches('/'),
        filename
    );

    let response = client
        .get(&remote_file)
        .basic_auth(&settings.username, Some(&settings.password))
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Download failed with status: {}",
            response.status()
        ));
    }

    let content = response.bytes().await.map_err(|e| e.to_string())?;

    // Write to database file
    let db_path = get_data_dir().join("ccg_gateway.db");

    tokio::fs::write(&db_path, &content)
        .await
        .map_err(|e| format!("Failed to write database: {}", e))?;

    // 退出应用，用户需手动重启
    exit_application(app);

    Ok(())
}

#[tauri::command]
pub async fn delete_webdav_backup(db: State<'_, SqlitePool>, filename: String) -> Result<()> {
    use reqwest::Client;

    let settings = get_webdav_settings(db).await?;
    if settings.url.is_empty() {
        return Err("WebDAV URL not configured".to_string());
    }

    let client = Client::new();
    let remote_file = format!(
        "{}/ccg-gateway-backup/{}",
        settings.url.trim_end_matches('/'),
        filename
    );

    let response = client
        .delete(&remote_file)
        .basic_auth(&settings.username, Some(&settings.password))
        .send()
        .await
        .map_err(|e| format!("Delete failed: {}", e))?;

    if !response.status().is_success() && response.status().as_u16() != 204 {
        return Err(format!("Delete failed with status: {}", response.status()));
    }

    Ok(())
}
