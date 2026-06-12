use crate::db::models::{ProviderProfileCreate, ProviderProfileRename, ProviderProfileResponse};
use crate::services::cli_config::{
    claude_settings_filename, codex_profile_config_path, get_cli_config_dir_path,
};
use crate::services::routing::{
    is_valid_profile_name, normalize_profile, normalize_profile_name, DEFAULT_PROFILE,
};
use crate::time::now_timestamp;
use sqlx::{SqlitePool, Transaction};

type Result<T> = std::result::Result<T, String>;

pub fn invalid_profile_message() -> String {
    "Profile 名称仅支持英文、数字、空格、下划线和短横线".to_string()
}

pub fn validate_provider_profile(profile: Option<&str>) -> Result<String> {
    normalize_profile(profile).ok_or_else(invalid_profile_message)
}

pub fn validate_cli_type(cli_type: &str) -> Result<String> {
    match cli_type.trim() {
        "claude_code" | "codex" | "gemini" => Ok(cli_type.trim().to_string()),
        _ => Err("不支持的 CLI 类型".to_string()),
    }
}

pub fn validate_profile_cli_type(cli_type: &str) -> Result<String> {
    match cli_type.trim() {
        "claude_code" | "codex" => Ok(cli_type.trim().to_string()),
        _ => Err("该 CLI 类型不支持 Profile".to_string()),
    }
}

pub async fn provider_profile_exists(
    db: &SqlitePool,
    cli_type: &str,
    profile: &str,
) -> Result<bool> {
    let cli_type = validate_cli_type(cli_type)?;
    let profile = validate_provider_profile(Some(profile))?;
    if profile == DEFAULT_PROFILE {
        return Ok(true);
    }

    ensure_legacy_provider_profiles(db).await?;
    sqlx::query_as::<_, (String,)>(
        "SELECT name FROM provider_profiles WHERE cli_type = ? AND lower(name) = ?",
    )
    .bind(&cli_type)
    .bind(&profile)
    .fetch_optional(db)
    .await
    .map(|row| row.is_some())
    .map_err(|e| e.to_string())
}

pub async fn provider_profile_exists_if_supported(
    db: &SqlitePool,
    cli_type: &str,
    profile: &str,
) -> Result<bool> {
    match validate_profile_cli_type(cli_type) {
        Ok(_) => provider_profile_exists(db, cli_type, profile).await,
        Err(_) => Ok(profile == DEFAULT_PROFILE),
    }
}

pub async fn list_provider_profile_names(db: &SqlitePool, cli_type: &str) -> Result<Vec<String>> {
    let cli_type = validate_cli_type(cli_type)?;
    ensure_legacy_provider_profiles(db).await?;
    let rows: Vec<(String,)> =
        sqlx::query_as(
            "SELECT name FROM provider_profiles WHERE cli_type = ? ORDER BY sort_order, created_at, name",
        )
        .bind(&cli_type)
        .fetch_all(db)
        .await
        .map_err(|e| e.to_string())?;

    let mut profiles = Vec::with_capacity(rows.len() + 1);
    profiles.push(DEFAULT_PROFILE.to_string());
    profiles.extend(rows.into_iter().map(|(name,)| name));
    Ok(profiles)
}

pub async fn list_profiles(
    db: &SqlitePool,
    cli_type: &str,
) -> Result<Vec<ProviderProfileResponse>> {
    let cli_type = validate_profile_cli_type(cli_type)?;
    ensure_legacy_provider_profiles(db).await?;
    let rows: Vec<(String, i64)> =
        sqlx::query_as("SELECT name, sort_order FROM provider_profiles WHERE cli_type = ? ORDER BY sort_order, created_at, name")
            .bind(&cli_type)
            .fetch_all(db)
            .await
            .map_err(|e| e.to_string())?;

    let mut profiles = vec![provider_profile_response(
        cli_type.clone(),
        DEFAULT_PROFILE.to_string(),
        0,
    )];
    profiles.extend(
        rows.into_iter().map(|(name, sort_order)| {
            provider_profile_response(cli_type.clone(), name, sort_order)
        }),
    );
    Ok(profiles)
}

pub async fn create_profile(
    db: &SqlitePool,
    input: ProviderProfileCreate,
) -> Result<ProviderProfileResponse> {
    let cli_type = validate_profile_cli_type(&input.cli_type)?;
    let name = unique_profile_name(db, &cli_type, &input.name, None).await?;

    let now = now_timestamp();
    let result = sqlx::query(
        r#"
        INSERT INTO provider_profiles (cli_type, name, sort_order, created_at, updated_at)
        VALUES (?, ?, (SELECT COALESCE(MAX(sort_order), 0) + 10 FROM provider_profiles WHERE cli_type = ?), ?, ?)
        "#,
    )
    .bind(&cli_type)
    .bind(&name)
    .bind(&cli_type)
    .bind(now)
    .bind(now)
    .execute(db)
    .await
    .map_err(map_db_error)?;

    if result.rows_affected() == 0 {
        return Err("Profile 创建失败".to_string());
    }

    let (sort_order,): (i64,) =
        sqlx::query_as("SELECT sort_order FROM provider_profiles WHERE cli_type = ? AND name = ?")
            .bind(&cli_type)
            .bind(&name)
            .fetch_one(db)
            .await
            .map_err(|e| e.to_string())?;

    Ok(provider_profile_response(cli_type, name, sort_order))
}

pub async fn rename_profile(
    db: &SqlitePool,
    profile: &str,
    input: ProviderProfileRename,
) -> Result<ProviderProfileResponse> {
    let cli_type = validate_profile_cli_type(&input.cli_type)?;
    let old_profile = validate_provider_profile(Some(profile))?;
    let new_profile = unique_profile_name(db, &cli_type, &input.name, Some(&old_profile)).await?;
    if old_profile == DEFAULT_PROFILE {
        return Err("默认 Profile 不能重命名".to_string());
    }
    if old_profile == new_profile {
        let (sort_order,): (i64,) = sqlx::query_as(
            "SELECT sort_order FROM provider_profiles WHERE cli_type = ? AND name = ?",
        )
        .bind(&cli_type)
        .bind(&old_profile)
        .fetch_optional(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Profile 不存在".to_string())?;
        return Ok(provider_profile_response(cli_type, new_profile, sort_order));
    }

    let (sort_order,): (i64,) =
        sqlx::query_as("SELECT sort_order FROM provider_profiles WHERE cli_type = ? AND name = ?")
            .bind(&cli_type)
            .bind(&old_profile)
            .fetch_optional(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Profile 不存在".to_string())?;

    let mut tx = db.begin().await.map_err(|e| e.to_string())?;
    sqlx::query(
        "UPDATE provider_profiles SET name = ?, updated_at = ? WHERE cli_type = ? AND name = ?",
    )
    .bind(&new_profile)
    .bind(now_timestamp())
    .bind(&cli_type)
    .bind(&old_profile)
    .execute(&mut *tx)
    .await
    .map_err(map_db_error)?;
    sqlx::query(
        "UPDATE providers SET profile = ?, updated_at = ? WHERE cli_type = ? AND profile = ?",
    )
    .bind(&new_profile)
    .bind(now_timestamp())
    .bind(&cli_type)
    .bind(&old_profile)
    .execute(&mut *tx)
    .await
    .map_err(map_db_error)?;
    rewrite_tasks_profile_tx(&mut tx, &cli_type, &old_profile, &new_profile).await?;
    tx.commit().await.map_err(|e| e.to_string())?;

    rename_profile_config_files(db, &cli_type, &old_profile, &new_profile).await?;

    Ok(provider_profile_response(cli_type, new_profile, sort_order))
}

pub async fn delete_profile(db: &SqlitePool, cli_type: &str, profile: &str) -> Result<()> {
    let cli_type = validate_profile_cli_type(cli_type)?;
    let profile = validate_provider_profile(Some(profile))?;
    if profile == DEFAULT_PROFILE {
        return Err("默认 Profile 不能删除".to_string());
    }

    let exists: Option<(String,)> =
        sqlx::query_as("SELECT name FROM provider_profiles WHERE cli_type = ? AND name = ?")
            .bind(&cli_type)
            .bind(&profile)
            .fetch_optional(db)
            .await
            .map_err(|e| e.to_string())?;
    if exists.is_none() {
        return Err("Profile 不存在".to_string());
    }

    let mut tx = db.begin().await.map_err(|e| e.to_string())?;
    let provider_ids: Vec<(i64,)> =
        sqlx::query_as("SELECT id FROM providers WHERE cli_type = ? AND profile = ?")
            .bind(&cli_type)
            .bind(&profile)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    for (id,) in provider_ids {
        sqlx::query("DELETE FROM provider_model_map WHERE provider_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        sqlx::query("DELETE FROM provider_model_blacklist WHERE provider_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
    }

    sqlx::query("DELETE FROM providers WHERE cli_type = ? AND profile = ?")
        .bind(&cli_type)
        .bind(&profile)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
    sqlx::query("DELETE FROM provider_profiles WHERE cli_type = ? AND name = ?")
        .bind(&cli_type)
        .bind(&profile)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
    tx.commit().await.map_err(|e| e.to_string())?;

    remove_profile_config_files(db, &cli_type, &profile).await?;
    Ok(())
}

async fn unique_profile_name(
    db: &SqlitePool,
    cli_type: &str,
    requested_name: &str,
    current_name: Option<&str>,
) -> Result<String> {
    let base = normalize_profile_name(requested_name).ok_or_else(invalid_profile_message)?;
    if current_name == Some(base.as_str()) {
        return Ok(base);
    }

    let exists = base == DEFAULT_PROFILE
        || sqlx::query_as::<_, (String,)>(
            "SELECT name FROM provider_profiles WHERE cli_type = ? AND lower(name) = ?",
        )
        .bind(cli_type)
        .bind(&base)
        .fetch_optional(db)
        .await
        .map_err(|e| e.to_string())?
        .is_some();

    if exists {
        return Err("名称已存在".to_string());
    }

    Ok(base)
}

fn provider_profile_response(
    cli_type: String,
    name: String,
    sort_order: i64,
) -> ProviderProfileResponse {
    let is_default = name == DEFAULT_PROFILE;
    ProviderProfileResponse {
        cli_type,
        label: if is_default {
            "默认".to_string()
        } else {
            name.clone()
        },
        name,
        is_default,
        sort_order,
    }
}

async fn ensure_legacy_provider_profiles(db: &SqlitePool) -> Result<()> {
    let now = now_timestamp();
    let legacy_profiles: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT DISTINCT cli_type, profile FROM providers
        WHERE profile <> ?
          AND NOT EXISTS (
              SELECT 1 FROM provider_profiles
              WHERE provider_profiles.cli_type = providers.cli_type
                AND provider_profiles.name = providers.profile
          )
        ORDER BY cli_type, profile
        "#,
    )
    .bind(DEFAULT_PROFILE)
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())?;

    for (index, (cli_type, profile)) in legacy_profiles.into_iter().enumerate() {
        if !is_valid_profile_name(&profile) {
            continue;
        }
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO provider_profiles (cli_type, name, sort_order, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&cli_type)
        .bind(&profile)
        .bind((index as i64 + 1) * 10)
        .bind(now)
        .bind(now)
        .execute(db)
        .await
        .map_err(map_db_error)?;
    }

    Ok(())
}

async fn remove_profile_config_files(db: &SqlitePool, cli_type: &str, profile: &str) -> Result<()> {
    if profile == DEFAULT_PROFILE {
        return Ok(());
    }

    match cli_type {
        "claude_code" => {
            let claude_dir = get_cli_config_dir_path(db, "claude_code").await;
            let claude_path = claude_dir.join(claude_settings_filename(profile));
            if tokio::fs::try_exists(&claude_path).await.unwrap_or(false) {
                tokio::fs::remove_file(&claude_path)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
        "codex" => {
            let codex_dir = get_cli_config_dir_path(db, "codex").await;
            let codex_path = codex_profile_config_path(&codex_dir, profile);
            if tokio::fs::try_exists(&codex_path).await.unwrap_or(false) {
                tokio::fs::remove_file(&codex_path)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
        _ => {}
    }

    Ok(())
}

async fn rename_profile_config_files(
    db: &SqlitePool,
    cli_type: &str,
    old_profile: &str,
    new_profile: &str,
) -> Result<()> {
    if old_profile == DEFAULT_PROFILE || new_profile == DEFAULT_PROFILE {
        return Ok(());
    }

    match cli_type {
        "claude_code" => {
            let claude_dir = get_cli_config_dir_path(db, "claude_code").await;
            let old_claude = claude_dir.join(claude_settings_filename(old_profile));
            let new_claude = claude_dir.join(claude_settings_filename(new_profile));
            if tokio::fs::try_exists(&old_claude).await.unwrap_or(false)
                && !tokio::fs::try_exists(&new_claude).await.unwrap_or(false)
            {
                tokio::fs::rename(&old_claude, &new_claude)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
        "codex" => {
            let codex_dir = get_cli_config_dir_path(db, "codex").await;
            let old_codex = codex_profile_config_path(&codex_dir, old_profile);
            let new_codex = codex_profile_config_path(&codex_dir, new_profile);
            if tokio::fs::try_exists(&old_codex).await.unwrap_or(false)
                && !tokio::fs::try_exists(&new_codex).await.unwrap_or(false)
            {
                tokio::fs::rename(&old_codex, &new_codex)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
        _ => {}
    }

    Ok(())
}

async fn rewrite_tasks_profile_tx(
    tx: &mut Transaction<'_, sqlx::Sqlite>,
    cli_type: &str,
    old_profile: &str,
    new_profile: &str,
) -> Result<()> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, payload_json FROM scheduled_tasks WHERE task_type = 'provider_keepalive'",
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| e.to_string())?;

    for (id, payload_json) in rows {
        let Ok(mut payload) = serde_json::from_str::<serde_json::Value>(&payload_json) else {
            continue;
        };
        if payload
            .get("profile")
            .and_then(|value| value.as_str())
            .map(|profile| profile == old_profile)
            .unwrap_or(false)
            && payload
                .get("cli_type")
                .and_then(|value| value.as_str())
                .map(|task_cli_type| task_cli_type == cli_type)
                .unwrap_or(false)
        {
            payload["profile"] = serde_json::Value::String(new_profile.to_string());
            sqlx::query("UPDATE scheduled_tasks SET payload_json = ?, updated_at = ? WHERE id = ?")
                .bind(payload.to_string())
                .bind(now_timestamp())
                .bind(id)
                .execute(&mut **tx)
                .await
                .map_err(map_db_error)?;
        }
    }

    Ok(())
}

fn map_db_error(e: sqlx::Error) -> String {
    let err_str = e.to_string();
    if err_str.contains("code: 2067") || err_str.contains("UNIQUE constraint failed") {
        if err_str.contains("provider_profiles") {
            return "名称已存在".to_string();
        }
        return "数据已存在，请勿重复添加".to_string();
    }
    err_str
}
