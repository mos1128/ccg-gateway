use super::*;

// Prompt commands
#[tauri::command]
pub async fn get_prompts(db: State<'_, SqlitePool>) -> Result<Vec<PromptResponse>> {
    let prompts = sqlx::query_as::<_, PromptPreset>("SELECT * FROM prompt_presets ORDER BY id")
        .fetch_all(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for prompt in prompts {
        let mut cli_flags = Vec::new();
        for cli_type in CliType::ALL.iter().map(CliType::as_str) {
            let enabled = prompt_enabled_in_file_async(db.inner(), cli_type, &prompt.content).await;
            cli_flags.push(PromptCliFlag {
                cli_type: cli_type.to_string(),
                enabled,
            });
        }

        results.push(PromptResponse {
            id: prompt.id,
            name: prompt.name,
            content: prompt.content,
            cli_flags,
        });
    }
    Ok(results)
}

#[tauri::command]
pub async fn get_prompt(db: State<'_, SqlitePool>, id: i64) -> Result<PromptResponse> {
    let prompt = sqlx::query_as::<_, PromptPreset>("SELECT * FROM prompt_presets WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Prompt not found".to_string())?;

    let mut cli_flags = Vec::new();
    for cli_type in CliType::ALL.iter().map(CliType::as_str) {
        let enabled = prompt_enabled_in_file_async(db.inner(), cli_type, &prompt.content).await;
        cli_flags.push(PromptCliFlag {
            cli_type: cli_type.to_string(),
            enabled,
        });
    }

    Ok(PromptResponse {
        id: prompt.id,
        name: prompt.name,
        content: prompt.content,
        cli_flags,
    })
}

#[tauri::command]
pub async fn create_prompt(
    db: State<'_, SqlitePool>,
    input: PromptCreate,
) -> Result<PromptResponse> {
    let now = now_timestamp();

    let result =
        sqlx::query("INSERT INTO prompt_presets (name, content, updated_at) VALUES (?, ?, ?)")
            .bind(&input.name)
            .bind(&input.content)
            .bind(now)
            .execute(db.inner())
            .await
            .map_err(map_db_error)?;

    let id = result.last_insert_rowid();

    // Sync to CLI files if cli_flags provided
    let cli_flags = input.cli_flags.unwrap_or_default();
    if !cli_flags.is_empty() {
        sync_single_prompt_to_cli(db.inner(), &input.content, &cli_flags).await?;
    }

    get_prompt(db, id).await
}

#[tauri::command]
pub async fn update_prompt(
    db: State<'_, SqlitePool>,
    id: i64,
    input: PromptUpdate,
) -> Result<PromptResponse> {
    let now = now_timestamp();

    let content = if input.name.is_some() || input.content.is_some() {
        let current =
            sqlx::query_as::<_, PromptPreset>("SELECT * FROM prompt_presets WHERE id = ?")
                .bind(id)
                .fetch_optional(db.inner())
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Prompt not found".to_string())?;

        let new_name = input.name.unwrap_or(current.name.clone());
        let new_content = input.content.unwrap_or(current.content.clone());

        sqlx::query("UPDATE prompt_presets SET name = ?, content = ?, updated_at = ? WHERE id = ?")
            .bind(&new_name)
            .bind(&new_content)
            .bind(now)
            .bind(id)
            .execute(db.inner())
            .await
            .map_err(map_db_error)?;

        new_content
    } else {
        // Get current values if not updating
        let current =
            sqlx::query_as::<_, PromptPreset>("SELECT * FROM prompt_presets WHERE id = ?")
                .bind(id)
                .fetch_optional(db.inner())
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Prompt not found".to_string())?;
        current.content
    };

    // Sync to CLI files if cli_flags provided
    if let Some(cli_flags) = input.cli_flags {
        sync_single_prompt_to_cli(db.inner(), &content, &cli_flags).await?;
    }

    get_prompt(db, id).await
}

#[tauri::command]
pub async fn toggle_prompt_cli(
    db: State<'_, SqlitePool>,
    id: i64,
    cli_type: String,
    enabled: bool,
) -> Result<PromptResponse> {
    let prompt = sqlx::query_as::<_, PromptPreset>("SELECT * FROM prompt_presets WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Prompt not found".to_string())?;

    sync_prompt_to_cli_async(db.inner(), &prompt.content, &cli_type, enabled).await?;

    get_prompt(db, id).await
}

#[tauri::command]
pub async fn delete_prompt(db: State<'_, SqlitePool>, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM prompt_presets WHERE id = ?")
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

    // Sync prompt configs to CLI files
    sync_prompt_configs_to_cli(db).await?;

    Ok(())
}

async fn sync_prompt_to_cli_async(
    db: &SqlitePool,
    prompt_content: &str,
    cli_type: &str,
    is_enabled: bool,
) -> Result<()> {
    let path = get_prompt_file_path(db, cli_type)
        .await
        .ok_or_else(|| format!("Invalid CLI type: {}", cli_type))?;

    if let Some(parent) = path.parent() {
        if !tokio::fs::try_exists(parent).await.unwrap_or(false) {
            return Ok(());
        }

        if is_enabled {
            tokio::fs::write(&path, prompt_content).await.map_err(|e| {
                tracing::error!("Failed to write prompt file: {}", e);
                e.to_string()
            })?;
        } else if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            let file_content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
            if normalize_text(prompt_content) == normalize_text(&file_content) {
                tokio::fs::write(&path, "").await.map_err(|e| {
                    tracing::error!("Failed to clear prompt file: {}", e);
                    e.to_string()
                })?;
            }
        }
    }

    Ok(())
}

// Sync a single prompt to CLI files based on enabled flags
async fn sync_single_prompt_to_cli(
    db: &SqlitePool,
    prompt_content: &str,
    cli_flags: &[PromptCliFlag],
) -> Result<()> {
    for cli_type in CliType::ALL.iter().map(CliType::as_str) {
        let is_enabled = cli_flags
            .iter()
            .any(|f| f.cli_type == cli_type && f.enabled);

        sync_prompt_to_cli_async(db, prompt_content, cli_type, is_enabled).await?;
    }

    Ok(())
}

async fn sync_prompt_configs_to_cli(_db: State<'_, SqlitePool>) -> Result<()> {
    // This function is no longer used, keeping for compatibility
    Ok(())
}

async fn get_prompt_file_path(db: &SqlitePool, cli_type: &str) -> Option<std::path::PathBuf> {
    let base_path = get_cli_config_dir_path(db, cli_type).await;
    cli_helpers::prompt_file_path(&base_path, cli_type)
}
