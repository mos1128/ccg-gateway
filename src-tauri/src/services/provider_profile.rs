use crate::db::models::{
    Provider, ProviderProfileCreate, ProviderProfileRename, ProviderProfileResponse,
};
use crate::services::agent;
use crate::services::routing::{normalize_profile, normalize_profile_name, DEFAULT_PROFILE};
use crate::time::now_timestamp;
use sqlx::{SqlitePool, Transaction};
use std::path::PathBuf;

type Result<T> = std::result::Result<T, String>;

enum ActiveProfileMode {
    ProxyRoute,
    ProviderDirect(Provider),
    None,
}

async fn active_profile_mode(
    db: &SqlitePool,
    cli_type: &str,
    profile: &str,
    gateway_url: &str,
) -> Result<ActiveProfileMode> {
    if crate::services::agent_config::is_provider_config_applied(db, cli_type, gateway_url, profile)
        .await
    {
        return Ok(ActiveProfileMode::ProxyRoute);
    }
    let Some(provider_id) =
        crate::services::agent_config::provider_direct_active_provider_id(db, cli_type, profile)
            .await?
    else {
        return Ok(ActiveProfileMode::None);
    };
    let provider = sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE id = ?")
        .bind(provider_id)
        .fetch_optional(db)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("直连服务商 {} 不存在", provider_id))?;
    Ok(ActiveProfileMode::ProviderDirect(provider))
}

async fn remove_profile_mode(
    db: &SqlitePool,
    cli_type: &str,
    profile: &str,
    gateway_url: &str,
    mode: &ActiveProfileMode,
) -> Result<()> {
    match mode {
        ActiveProfileMode::ProxyRoute => {
            crate::services::agent_config::sync_proxy_route_config(
                db,
                cli_type,
                false,
                gateway_url,
                profile,
                "",
                None,
                "merge",
            )
            .await?;
        }
        ActiveProfileMode::ProviderDirect(provider) => {
            let mut provider = provider.clone();
            provider.profile = profile.to_string();
            crate::services::agent_config::remove_provider_direct_config_for_provider(
                db, &provider,
            )
            .await?;
        }
        ActiveProfileMode::None => {}
    }
    Ok(())
}

async fn apply_profile_mode(
    db: &SqlitePool,
    cli_type: &str,
    profile: &str,
    gateway_url: &str,
    mode: &ActiveProfileMode,
) -> Result<()> {
    match mode {
        ActiveProfileMode::ProxyRoute => {
            crate::services::agent_config::sync_proxy_route_config(
                db,
                cli_type,
                true,
                gateway_url,
                profile,
                "",
                None,
                "merge",
            )
            .await?;
        }
        ActiveProfileMode::ProviderDirect(provider) => {
            let mut provider = provider.clone();
            provider.profile = profile.to_string();
            crate::services::agent_config::write_provider_direct_config_for_profile_rename(
                db, &provider,
            )
            .await?;
        }
        ActiveProfileMode::None => {}
    }
    Ok(())
}

async fn rollback_renamed_profile(
    db: &SqlitePool,
    cli_type: &str,
    old_profile: &str,
    new_profile: &str,
    gateway_url: &str,
    mode: &ActiveProfileMode,
    config_rename: Option<&ProfileConfigRename>,
) -> Result<()> {
    let mut recovery_errors = Vec::new();
    if let Err(error) = remove_profile_mode(db, cli_type, new_profile, gateway_url, mode).await {
        recovery_errors.push(format!("清理新 Profile 配置失败: {}", error));
    }

    let reverse_rename = config_rename.map(|rename| ProfileConfigRename {
        old_path: rename.new_path.clone(),
        new_path: rename.old_path.clone(),
    });
    if let Err(error) = rename_profile_records_and_file(
        db,
        cli_type,
        new_profile,
        old_profile,
        reverse_rename.as_ref(),
    )
    .await
    {
        recovery_errors.push(format!("恢复 Profile 名称失败: {}", error));
        return Err(recovery_errors.join("; "));
    }

    if let Err(error) = apply_profile_mode(db, cli_type, old_profile, gateway_url, mode).await {
        recovery_errors.push(format!("恢复旧 Profile 配置失败: {}", error));
    }

    if recovery_errors.is_empty() {
        Ok(())
    } else {
        Err(recovery_errors.join("; "))
    }
}

pub fn invalid_profile_message() -> String {
    "Profile 名称仅支持英文、数字、空格、下划线和短横线".to_string()
}

pub fn validate_provider_profile(profile: Option<&str>) -> Result<String> {
    normalize_profile(profile).ok_or_else(invalid_profile_message)
}

pub fn validate_cli_type(cli_type: &str) -> Result<String> {
    agent::validate_agent_id(cli_type)
}

pub async fn validate_profile_cli_type(db: &SqlitePool, cli_type: &str) -> Result<String> {
    let cli_type = validate_cli_type(cli_type)?;
    let supports_profiles = agent::get_agent(db, &cli_type)
        .await
        .map_err(|error| error.to_string())?
        .is_some_and(|resolved| resolved.features.profiles.enabled);
    supports_profiles
        .then_some(cli_type)
        .ok_or_else(|| "该 Agent 不支持 Profile".to_string())
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
    match validate_profile_cli_type(db, cli_type).await {
        Ok(_) => provider_profile_exists(db, cli_type, profile).await,
        Err(_) => Ok(profile == DEFAULT_PROFILE),
    }
}

pub async fn list_provider_profile_names(db: &SqlitePool, cli_type: &str) -> Result<Vec<String>> {
    let cli_type = validate_cli_type(cli_type)?;
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
    let cli_type = validate_profile_cli_type(db, cli_type).await?;
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
    let cli_type = validate_profile_cli_type(db, &input.cli_type).await?;
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
    gateway_url: &str,
    profile: &str,
    input: ProviderProfileRename,
) -> Result<ProviderProfileResponse> {
    let cli_type = validate_profile_cli_type(db, &input.cli_type).await?;
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
    let config_rename = profile_config_rename(db, &cli_type, &old_profile, &new_profile).await?;
    let active_mode = active_profile_mode(db, &cli_type, &old_profile, gateway_url).await?;
    if matches!(&active_mode, ActiveProfileMode::None)
        && profile_config_files_exist(db, &cli_type, &old_profile).await?
    {
        return Err(format!(
            "Profile `{}` 的现有配置无法匹配 Gateway 或服务商，不能安全重命名",
            old_profile
        ));
    }
    if let Err(error) =
        remove_profile_mode(db, &cli_type, &old_profile, gateway_url, &active_mode).await
    {
        let restore_error =
            apply_profile_mode(db, &cli_type, &old_profile, gateway_url, &active_mode)
                .await
                .err();
        return Err(match restore_error {
            Some(restore_error) => format!(
                "清理旧 Profile 模板配置失败: {}; 恢复旧配置也失败: {}",
                error, restore_error
            ),
            None => error,
        });
    }

    if let Err(error) = rename_profile_records_and_file(
        db,
        &cli_type,
        &old_profile,
        &new_profile,
        config_rename.as_ref(),
    )
    .await
    {
        let restore_error =
            apply_profile_mode(db, &cli_type, &old_profile, gateway_url, &active_mode)
                .await
                .err();
        return Err(match restore_error {
            Some(restore_error) => format!(
                "Profile 重命名失败: {}; 恢复旧模板配置也失败: {}",
                error, restore_error
            ),
            None => error,
        });
    }

    if let Err(error) =
        apply_profile_mode(db, &cli_type, &new_profile, gateway_url, &active_mode).await
    {
        let recovery_error = rollback_renamed_profile(
            db,
            &cli_type,
            &old_profile,
            &new_profile,
            gateway_url,
            &active_mode,
            config_rename.as_ref(),
        )
        .await
        .err();
        return Err(match recovery_error {
            Some(recovery_error) => format!(
                "按新 Profile 模板重写配置失败: {}; 回滚也未完全成功: {}",
                error, recovery_error
            ),
            None => format!("按新 Profile 模板重写配置失败，已恢复原 Profile: {}", error),
        });
    }

    Ok(provider_profile_response(cli_type, new_profile, sort_order))
}

pub async fn delete_profile(
    db: &SqlitePool,
    gateway_url: &str,
    cli_type: &str,
    profile: &str,
) -> Result<()> {
    let cli_type = validate_profile_cli_type(db, cli_type).await?;
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

    let active_mode = active_profile_mode(db, &cli_type, &profile, gateway_url).await?;
    if matches!(&active_mode, ActiveProfileMode::None)
        && profile_config_files_exist(db, &cli_type, &profile).await?
    {
        return Err(format!(
            "Profile `{}` 的现有配置无法匹配 Gateway 或服务商，不能安全删除",
            profile
        ));
    }
    remove_profile_mode(db, &cli_type, &profile, gateway_url, &active_mode).await?;

    let delete_result: Result<()> = async {
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
        Ok(())
    }
    .await;
    if let Err(error) = delete_result {
        let restore_error = apply_profile_mode(db, &cli_type, &profile, gateway_url, &active_mode)
            .await
            .err();
        return Err(match restore_error {
            Some(restore_error) => format!(
                "删除 Profile 数据失败: {}; 恢复配置也失败: {}",
                error, restore_error
            ),
            None => error,
        });
    }

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

async fn profile_config_files_exist(
    db: &SqlitePool,
    cli_type: &str,
    profile: &str,
) -> Result<bool> {
    for path in
        crate::services::agent_config::profile_operation_paths(db, cli_type, profile).await?
    {
        if tokio::fs::try_exists(&path)
            .await
            .map_err(|error| format!("检查 {} 失败: {}", path.display(), error))?
        {
            return Ok(true);
        }
    }
    if let Some((_, path)) =
        crate::services::agent_config::profile_file(db, cli_type, profile).await?
    {
        return tokio::fs::try_exists(&path)
            .await
            .map_err(|error| format!("检查 {} 失败: {}", path.display(), error));
    }
    Ok(false)
}

async fn remove_profile_config_files(db: &SqlitePool, cli_type: &str, profile: &str) -> Result<()> {
    if profile == DEFAULT_PROFILE {
        return Ok(());
    }

    if let Some((_, path)) =
        crate::services::agent_config::profile_file(db, cli_type, profile).await?
    {
        let shares_default_file =
            crate::services::agent_config::profile_file(db, cli_type, DEFAULT_PROFILE)
                .await?
                .is_some_and(|(_, default_path)| default_path == path);
        if shares_default_file {
            return Ok(());
        }
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            tokio::fs::remove_file(&path)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

struct ProfileConfigRename {
    old_path: PathBuf,
    new_path: PathBuf,
}

async fn rename_profile_records_and_file(
    db: &SqlitePool,
    cli_type: &str,
    old_profile: &str,
    new_profile: &str,
    config_rename: Option<&ProfileConfigRename>,
) -> Result<()> {
    let mut tx = db.begin().await.map_err(|error| error.to_string())?;
    sqlx::query(
        "UPDATE provider_profiles SET name = ?, updated_at = ? WHERE cli_type = ? AND name = ?",
    )
    .bind(new_profile)
    .bind(now_timestamp())
    .bind(cli_type)
    .bind(old_profile)
    .execute(&mut *tx)
    .await
    .map_err(map_db_error)?;
    sqlx::query(
        "UPDATE providers SET profile = ?, updated_at = ? WHERE cli_type = ? AND profile = ?",
    )
    .bind(new_profile)
    .bind(now_timestamp())
    .bind(cli_type)
    .bind(old_profile)
    .execute(&mut *tx)
    .await
    .map_err(map_db_error)?;
    rewrite_tasks_profile_tx(&mut tx, cli_type, old_profile, new_profile).await?;

    if let Some(rename) = config_rename {
        tokio::fs::rename(&rename.old_path, &rename.new_path)
            .await
            .map_err(|error| {
                format!(
                    "Profile 配置文件从 {} 重命名到 {} 失败: {}",
                    rename.old_path.display(),
                    rename.new_path.display(),
                    error
                )
            })?;
    }
    if let Err(error) = tx.commit().await {
        if let Some(rename) = config_rename {
            if let Err(rollback_error) = tokio::fs::rename(&rename.new_path, &rename.old_path).await
            {
                return Err(format!(
                    "Profile 数据库重命名提交失败: {}; 配置文件回滚也失败: {}",
                    error, rollback_error
                ));
            }
        }
        return Err(error.to_string());
    }
    Ok(())
}

async fn profile_config_rename(
    db: &SqlitePool,
    cli_type: &str,
    old_profile: &str,
    new_profile: &str,
) -> Result<Option<ProfileConfigRename>> {
    if old_profile == DEFAULT_PROFILE || new_profile == DEFAULT_PROFILE {
        return Ok(None);
    }

    let Some((_, old_path)) =
        crate::services::agent_config::profile_file(db, cli_type, old_profile).await?
    else {
        return Ok(None);
    };
    let Some((_, new_path)) =
        crate::services::agent_config::profile_file(db, cli_type, new_profile).await?
    else {
        return Ok(None);
    };
    if old_path == new_path {
        return Ok(None);
    }
    if tokio::fs::try_exists(&new_path).await.unwrap_or(false) {
        return Err(format!(
            "目标 Profile 配置文件已存在: {}",
            new_path.display()
        ));
    }
    if !tokio::fs::try_exists(&old_path).await.unwrap_or(false) {
        return Ok(None);
    }

    Ok(Some(ProfileConfigRename { old_path, new_path }))
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
