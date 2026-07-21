use super::*;

async fn resolve_prompt_file(db: &SqlitePool, agent_id: &str) -> Result<std::path::PathBuf> {
    let agent = crate::services::agent::get_agent(db, agent_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("未知 Agent: {}", agent_id))?;
    let feature = agent.features.prompts;
    if !feature.enabled {
        return Err(format!("Agent {} 的 Prompt 功能不可用", agent_id));
    }
    let file = feature
        .file
        .as_deref()
        .ok_or_else(|| format!("Agent {} 的 Prompt 缺少 file", agent_id))?;
    Ok(crate::services::cli_config::resolve_cli_config_file(db, agent_id, file).await)
}

fn normalize_prompt_text(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

async fn prompt_enabled_in_file_async(
    db: &SqlitePool,
    agent_id: &str,
    prompt_content: &str,
) -> bool {
    let Ok(path) = resolve_prompt_file(db, agent_id).await else {
        return false;
    };
    let Ok(file_content) = tokio::fs::read_to_string(path).await else {
        return false;
    };
    normalize_prompt_text(prompt_content) == normalize_prompt_text(&file_content)
}

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
        for cli_type in crate::services::agent::agent_ids_for_feature("prompts") {
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
    for cli_type in crate::services::agent::agent_ids_for_feature("prompts") {
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
    let prompt = sqlx::query_as::<_, PromptPreset>("SELECT * FROM prompt_presets WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Prompt not found".to_string())?;

    sqlx::query("DELETE FROM prompt_presets WHERE id = ?")
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

    for agent_id in crate::services::agent::agent_ids_for_feature("prompts") {
        sync_prompt_to_cli_async(db.inner(), &prompt.content, agent_id, false).await?;
    }

    Ok(())
}

async fn sync_prompt_to_cli_async(
    db: &SqlitePool,
    prompt_content: &str,
    cli_type: &str,
    is_enabled: bool,
) -> Result<()> {
    let path = resolve_prompt_file(db, cli_type).await?;

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
            if normalize_prompt_text(prompt_content) == normalize_prompt_text(&file_content) {
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
    for cli_type in crate::services::agent::agent_ids_for_feature("prompts") {
        let is_enabled = cli_flags
            .iter()
            .any(|f| f.cli_type == cli_type && f.enabled);

        sync_prompt_to_cli_async(db, prompt_content, cli_type, is_enabled).await?;
    }

    Ok(())
}
