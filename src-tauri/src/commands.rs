use crate::config::{expand_home_path, get_data_dir, get_default_cli_config_dir, shrink_home_path};
use crate::db::models::{
    CliSettingsResponse, CliSettingsUpdate, DailyStats, DiscoverableSkill, GatewaySettings,
    InstalledSkillResponse, MarketplaceInfo, McpCliFlag, McpConfig, McpCreate, McpResponse,
    McpUpdate, OfficialCredential, OfficialCredentialCreate, OfficialCredentialResponse,
    OfficialCredentialUpdate, PaginatedLogs, PaginatedProjects, PaginatedSessions,
    PluginFavoriteItem, PluginItem, ProjectInfo, PromptCliFlag, PromptCreate, PromptPreset,
    PromptResponse, PromptUpdate, Provider, ProviderCreate, ProviderResponse,
    ProviderStatsResponse, ProviderStatsRow, ProviderUpdate, RequestLogDetail, RequestLogItem,
    SessionInfo, SessionMessage, SkillCliFlag, SkillFavorite, SkillFavoriteItem, SkillRepo,
    SkillRepoCreate, SystemLogItem, SystemLogListResponse, SystemStatus, TestProviderModelsInput,
    TimeoutSettings,
    TimeoutSettingsUpdate, WebdavBackup, WebdavSettings, WebdavSettingsUpdate,
};
use crate::services::skill::{self, is_local_repo_source, InstalledSkillManifestEntry};
use crate::LogDb;
use serde::Serialize;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use tauri::{Emitter, Manager, State};

type Result<T> = std::result::Result<T, String>;

fn serialize_toml_document<T: Serialize>(
    value: &T,
    context: &str,
) -> Result<toml_edit::DocumentMut> {
    toml_edit::ser::to_document(value)
        .map_err(|e| format!("Failed to serialize {}: {}", context, e))
}

fn serialize_toml_table<T: Serialize>(value: &T, context: &str) -> Result<toml_edit::Table> {
    Ok(serialize_toml_document(value, context)?.as_table().clone())
}

fn codex_gateway_document() -> Result<toml_edit::DocumentMut> {
    r#"model_provider = "ccg-gateway"

[model_providers.ccg-gateway]
name = "ccg-gateway"
base_url = "http://127.0.0.1:7788"
wire_api = "responses"
requires_openai_auth = false
"#
    .parse::<toml_edit::DocumentMut>()
    .map_err(|e| format!("Failed to build Codex gateway config: {}", e))
}

fn remove_file_if_exists(path: &std::path::Path, label: &str) -> Result<()> {
    if path.exists() {
        tracing::info!("删除直连模式文件: {:?}", path);
        std::fs::remove_file(path).map_err(|e| {
            tracing::error!("删除 {} 失败: {}", label, e);
            e.to_string()
        })?;
    }
    Ok(())
}

fn remove_codex_direct_mode_files(config_dir: &std::path::Path, use_merge: bool) -> Result<()> {
    let auth_path = config_dir.join("auth.json");
    let config_path = config_dir.join("config.toml");

    remove_file_if_exists(&auth_path, "auth.json")?;

    if use_merge {
        tracing::info!("Codex 增量模式切换到中转时保留 config.toml，供后续合并");
    } else {
        remove_file_if_exists(&config_path, "config.toml")?;
    }

    Ok(())
}

fn remove_gemini_direct_mode_files(config_dir: &std::path::Path, use_merge: bool) -> Result<()> {
    let oauth_path = config_dir.join("oauth_creds.json");
    let accounts_path = config_dir.join("google_accounts.json");
    let settings_path = config_dir.join("settings.json");

    remove_file_if_exists(&oauth_path, "oauth_creds.json")?;
    remove_file_if_exists(&accounts_path, "google_accounts.json")?;

    if use_merge {
        tracing::info!("Gemini 增量模式切换到中转时保留 settings.json，供后续合并");
    } else {
        remove_file_if_exists(&settings_path, "settings.json")?;
    }

    Ok(())
}

fn parse_codex_mcp_toml_table(mcp_config_json: &str) -> Result<toml_edit::Table> {
    let value = serde_json::from_str::<serde_json::Value>(mcp_config_json)
        .map_err(|e| format!("Codex MCP JSON 格式错误: {}", e))?;

    if !value.is_object() {
        return Err("Codex MCP 配置必须是 JSON object".to_string());
    }

    validate_toml_compatible_json(&value)?;
    serialize_toml_table(&value, "Codex MCP config")
}

fn validate_toml_compatible_json(value: &serde_json::Value) -> Result<()> {
    match value {
        serde_json::Value::Null => Err("Codex MCP 配置不能包含 null，TOML 不支持 null".to_string()),
        serde_json::Value::Array(items) => {
            for item in items {
                validate_toml_compatible_json(item)?;
            }
            Ok(())
        }
        serde_json::Value::Object(map) => {
            for value in map.values() {
                validate_toml_compatible_json(value)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_mcp_json_preserves_stdio_fields() {
        let server_table = parse_codex_mcp_toml_table(
            r#"{
                "type": "stdio",
                "command": "auggie",
                "args": ["--mcp"],
                "env": {
                    "AUGMENT_API_TOKEN": "token",
                    "AUGMENT_API_URL": "https://ace-test.heroman.wtf/"
                }
            }"#,
        )
        .expect("stdio MCP should parse");

        let mut doc = toml_edit::DocumentMut::new();
        doc["mcp_servers"] = toml_edit::table();
        doc["mcp_servers"]["auggie"] = toml_edit::Item::Table(server_table);

        let parsed = doc
            .to_string()
            .parse::<toml::Value>()
            .expect("serialized TOML should parse");

        let server = parsed
            .get("mcp_servers")
            .and_then(|value| value.get("auggie"))
            .expect("mcp server should exist");

        assert_eq!(
            server.get("type").and_then(|value| value.as_str()),
            Some("stdio")
        );
        assert_eq!(
            server.get("command").and_then(|value| value.as_str()),
            Some("auggie")
        );
        assert_eq!(
            server
                .get("args")
                .and_then(|value| value.get(0))
                .and_then(|value| value.as_str()),
            Some("--mcp")
        );
        assert_eq!(
            server
                .get("env")
                .and_then(|value| value.get("AUGMENT_API_TOKEN"))
                .and_then(|value| value.as_str()),
            Some("token")
        );
    }

    #[test]
    fn codex_mcp_json_preserves_sse_headers() {
        let server_table = parse_codex_mcp_toml_table(
            r#"{
                "type": "sse",
                "url": "https://mcp.api-inference.modelscope.net/f3b382c4523044/sse",
                "headers": {
                    "Authorization": "Bearer example-token"
                }
            }"#,
        )
        .expect("sse MCP should parse");

        let mut doc = toml_edit::DocumentMut::new();
        doc["mcp_servers"] = toml_edit::table();
        doc["mcp_servers"]["fetch"] = toml_edit::Item::Table(server_table);

        let parsed = doc
            .to_string()
            .parse::<toml::Value>()
            .expect("serialized TOML should parse");

        let server = parsed
            .get("mcp_servers")
            .and_then(|value| value.get("fetch"))
            .expect("mcp server should exist");

        assert_eq!(
            server.get("type").and_then(|value| value.as_str()),
            Some("sse")
        );
        assert_eq!(
            server.get("url").and_then(|value| value.as_str()),
            Some("https://mcp.api-inference.modelscope.net/f3b382c4523044/sse")
        );
        assert_eq!(
            server
                .get("headers")
                .and_then(|value| value.get("Authorization"))
                .and_then(|value| value.as_str()),
            Some("Bearer example-token")
        );
    }

    #[test]
    fn codex_mcp_json_requires_object_root() {
        let err = parse_codex_mcp_toml_table(r#""not-an-object""#)
            .expect_err("non-object MCP config should fail");

        assert!(err.contains("JSON object"));
    }

    #[test]
    fn codex_mcp_json_rejects_null() {
        let err = parse_codex_mcp_toml_table(
            r#"{
                "type": "sse",
                "url": null
            }"#,
        )
        .expect_err("null value should fail");

        assert!(err.contains("null"));
    }

    #[test]
    fn claude_gateway_fields_are_protected_from_custom_config() {
        let custom_config = serde_json::json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://example.com",
                "ANTHROPIC_AUTH_TOKEN": "user-token",
                "FOO": "bar"
            },
            "other": 1
        });

        let sanitized = sanitize_json_config(custom_config, &claude_gateway_json_template());

        assert_eq!(sanitized["env"]["FOO"], "bar");
        assert_eq!(sanitized["other"], 1);
        assert!(sanitized["env"].get("ANTHROPIC_BASE_URL").is_none());
        assert!(sanitized["env"].get("ANTHROPIC_AUTH_TOKEN").is_none());
    }

    #[test]
    fn gemini_gateway_fields_are_protected_from_custom_config() {
        let custom_config = serde_json::json!({
            "security": {
                "auth": {
                    "selectedType": "oauth-personal"
                }
            },
            "theme": "dark"
        });

        let sanitized = sanitize_json_config(custom_config, &gemini_gateway_json_template());

        assert_eq!(sanitized["theme"], "dark");
        assert!(
            sanitized["security"]["auth"].get("selectedType").is_none(),
            "selectedType should be stripped from custom config"
        );
    }

    #[test]
    fn removing_claude_gateway_content_preserves_other_fields() {
        let temp_dir =
            std::env::temp_dir().join(format!("ccg-gateway-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).expect("temp dir should be created");

        let config_path = temp_dir.join("settings.json");
        std::fs::write(
            &config_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "env": {
                    "ANTHROPIC_BASE_URL": "http://127.0.0.1:7788",
                    "ANTHROPIC_AUTH_TOKEN": "ccg-gateway",
                    "KEEP": "yes"
                },
                "theme": "solarized",
                "custom_flag": true
            }))
            .expect("json should serialize"),
        )
        .expect("settings.json should be written");

        let gateway_config = claude_gateway_json_template();
        remove_json_config_content(
            &config_path,
            &gateway_config,
            r#"{"theme":"solarized","env":{"ANTHROPIC_BASE_URL":"https://should-be-ignored"}}"#,
            &gateway_config,
        )
        .expect("gateway content removal should succeed");

        let result: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&config_path).expect("settings.json should be readable"),
        )
        .expect("settings.json should remain valid JSON");

        assert_eq!(result["env"]["KEEP"], "yes");
        assert_eq!(result["custom_flag"], true);
        assert!(result.get("theme").is_none());
        assert!(result["env"].get("ANTHROPIC_BASE_URL").is_none());
        assert!(result["env"].get("ANTHROPIC_AUTH_TOKEN").is_none());

        std::fs::remove_dir_all(&temp_dir).expect("temp dir should be removed");
    }

    #[test]
    fn removing_codex_gateway_content_preserves_other_tables() {
        let temp_dir =
            std::env::temp_dir().join(format!("ccg-gateway-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).expect("temp dir should be created");

        let config_path = temp_dir.join("config.toml");
        std::fs::write(
            &config_path,
            r#"model_provider = "ccg-gateway"
model = "gpt-5.4"

[model_providers.ccg-gateway]
name = "ccg-gateway"
base_url = "http://127.0.0.1:7788"
wire_api = "responses"
requires_openai_auth = false

[mcp_servers.universal-db]
command = "cmd"
"#,
        )
        .expect("config.toml should be written");

        remove_codex_gateway_config_content(&config_path, "model = \"gpt-5.4\"\n")
            .expect("gateway config removal should succeed");

        let result = std::fs::read_to_string(&config_path).expect("config.toml should be readable");
        assert!(!result.contains("model_provider = "));
        assert!(!result.contains("[model_providers.ccg-gateway]"));
        assert!(!result.contains("model = \"gpt-5.4\""));
        assert!(result.contains("[mcp_servers.universal-db]"));

        std::fs::remove_dir_all(&temp_dir).expect("temp dir should be removed");
    }

    #[test]
    fn removing_codex_gateway_auth_preserves_other_keys() {
        let temp_dir =
            std::env::temp_dir().join(format!("ccg-gateway-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).expect("temp dir should be created");

        let auth_path = temp_dir.join("auth.json");
        std::fs::write(
            &auth_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "OPENAI_API_KEY": "ccg-gateway",
                "EXTRA": "keep"
            }))
            .expect("json should serialize"),
        )
        .expect("auth.json should be written");

        remove_codex_gateway_auth_content(&auth_path).expect("auth cleanup should succeed");

        let result: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&auth_path).expect("auth.json should be readable"),
        )
        .expect("auth.json should remain valid JSON");

        assert_eq!(result["EXTRA"], "keep");
        assert!(result.get("OPENAI_API_KEY").is_none());

        std::fs::remove_dir_all(&temp_dir).expect("temp dir should be removed");
    }

    #[test]
    fn removing_gemini_gateway_env_preserves_other_lines() {
        let temp_dir =
            std::env::temp_dir().join(format!("ccg-gateway-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).expect("temp dir should be created");

        let env_path = temp_dir.join(".env");
        std::fs::write(
            &env_path,
            "GEMINI_API_KEY=ccg-gateway\nGOOGLE_GEMINI_BASE_URL=http://127.0.0.1:7788\nEXTRA=keep\n",
        )
        .expect(".env should be written");

        remove_gemini_gateway_env_content(&env_path).expect(".env cleanup should succeed");

        let result = std::fs::read_to_string(&env_path).expect(".env should be readable");
        assert_eq!(result, "EXTRA=keep\n");

        std::fs::remove_dir_all(&temp_dir).expect("temp dir should be removed");
    }

    #[test]
    fn codex_direct_to_proxy_merge_preserves_config_toml() {
        let temp_dir =
            std::env::temp_dir().join(format!("ccg-gateway-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).expect("temp dir should be created");

        let auth_path = temp_dir.join("auth.json");
        let config_path = temp_dir.join("config.toml");
        std::fs::write(&auth_path, r#"{"OPENAI_API_KEY":"official"}"#)
            .expect("auth.json should be written");
        std::fs::write(&config_path, "model = \"legacy\"\n")
            .expect("config.toml should be written");

        remove_codex_direct_mode_files(&temp_dir, true).expect("merge cleanup should succeed");

        assert!(!auth_path.exists(), "auth.json should be removed");
        assert!(
            config_path.exists(),
            "config.toml should be preserved in merge mode"
        );
        assert_eq!(
            std::fs::read_to_string(&config_path).expect("config.toml should be readable"),
            "model = \"legacy\"\n"
        );

        std::fs::remove_dir_all(&temp_dir).expect("temp dir should be removed");
    }

    #[test]
    fn codex_direct_to_proxy_overwrite_removes_config_toml() {
        let temp_dir =
            std::env::temp_dir().join(format!("ccg-gateway-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).expect("temp dir should be created");

        let auth_path = temp_dir.join("auth.json");
        let config_path = temp_dir.join("config.toml");
        std::fs::write(&auth_path, r#"{"OPENAI_API_KEY":"official"}"#)
            .expect("auth.json should be written");
        std::fs::write(&config_path, "model = \"legacy\"\n")
            .expect("config.toml should be written");

        remove_codex_direct_mode_files(&temp_dir, false).expect("overwrite cleanup should succeed");

        assert!(!auth_path.exists(), "auth.json should be removed");
        assert!(
            !config_path.exists(),
            "config.toml should be removed in overwrite mode"
        );

        std::fs::remove_dir_all(&temp_dir).expect("temp dir should be removed");
    }
}

fn map_db_error(e: sqlx::Error) -> String {
    let err_str = e.to_string();
    if err_str.contains("code: 2067") || err_str.contains("UNIQUE constraint failed") {
        if err_str.contains("providers.cli_type") && err_str.contains("providers.name") {
            return "同类型的服务商名称已存在".to_string();
        }
        if err_str.contains("provider_model_map.provider_id")
            && err_str.contains("provider_model_map.source_model")
        {
            return "该服务商已存在相同的模型映射".to_string();
        }
        if err_str.contains("provider_model_blacklist.provider_id")
            && err_str.contains("provider_model_blacklist.model_pattern")
        {
            return "该服务商已存在相同的黑名单模式".to_string();
        }
        if err_str.contains("mcp_configs.name") {
            return "MCP 配置名称已存在".to_string();
        }
        if err_str.contains("prompt_presets.name") {
            return "提示词预设名称已存在".to_string();
        }
        if err_str.contains("skill_configs.directory") {
            return "该目录已安装过 Skill".to_string();
        }
        if err_str.contains("official_credentials.cli_type")
            && err_str.contains("official_credentials.name")
        {
            return "同类型的凭证名称已存在".to_string();
        }
        if err_str.contains("plugin_favorites.plugin_id") {
            return "该插件已收藏".to_string();
        }
        if err_str.contains("skill_favorites.skill_key") {
            return "该技能已收藏".to_string();
        }
        return "数据已存在，请勿重复添加".to_string();
    }
    err_str
}

#[tauri::command]
pub async fn get_providers(
    db: State<'_, SqlitePool>,
    cli_type: Option<String>,
) -> Result<Vec<ProviderResponse>> {
    let providers = if let Some(ct) = cli_type {
        sqlx::query_as::<_, Provider>(
            "SELECT * FROM providers WHERE cli_type = ? ORDER BY sort_order, id",
        )
        .bind(&ct)
        .fetch_all(db.inner())
        .await
    } else {
        sqlx::query_as::<_, Provider>("SELECT * FROM providers ORDER BY sort_order, id")
            .fetch_all(db.inner())
            .await
    };

    let providers = providers.map_err(|e| e.to_string())?;

    // 批量查询所有 model_maps（避免 N+1 问题）
    let all_maps: Vec<(i64, i64, String, String, i64)> = sqlx::query_as(
        "SELECT id, provider_id, source_model, target_model, enabled FROM provider_model_map ORDER BY provider_id, id",
    )
    .fetch_all(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    // 批量查询所有 model_blacklist（避免 N+1 问题）
    let all_blacklist: Vec<(i64, i64, String)> = sqlx::query_as(
        "SELECT id, provider_id, model_pattern FROM provider_model_blacklist ORDER BY provider_id, id",
    )
    .fetch_all(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    // 按 provider_id 分组
    let maps_by_provider: HashMap<i64, Vec<_>> = all_maps.into_iter().fold(
        HashMap::new(),
        |mut acc, (id, provider_id, source_model, target_model, enabled)| {
            acc.entry(provider_id).or_insert_with(Vec::new).push((
                id,
                source_model,
                target_model,
                enabled,
            ));
            acc
        },
    );

    let blacklist_by_provider: HashMap<i64, Vec<_>> = all_blacklist.into_iter().fold(
        HashMap::new(),
        |mut acc, (id, provider_id, model_pattern)| {
            acc.entry(provider_id)
                .or_insert_with(Vec::new)
                .push((id, model_pattern));
            acc
        },
    );

    // 组装结果
    let results: Vec<ProviderResponse> = providers
        .into_iter()
        .map(|provider| {
            let mut response = ProviderResponse::from(provider.clone());

            // 从分组数据中获取 model_maps
            response.model_maps = maps_by_provider
                .get(&provider.id)
                .map(|maps| {
                    maps.iter()
                        .map(|(id, source_model, target_model, enabled)| {
                            crate::db::models::ModelMapResponse {
                                id: *id,
                                source_model: source_model.clone(),
                                target_model: target_model.clone(),
                                enabled: *enabled != 0,
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            // 从分组数据中获取 model_blacklist
            response.model_blacklist = blacklist_by_provider
                .get(&provider.id)
                .map(|blacklist| {
                    blacklist
                        .iter()
                        .map(
                            |(id, model_pattern)| crate::db::models::ModelBlacklistResponse {
                                id: *id,
                                model_pattern: model_pattern.clone(),
                            },
                        )
                        .collect()
                })
                .unwrap_or_default();

            response
        })
        .collect();

    Ok(results)
}

#[tauri::command]
pub async fn get_provider(db: State<'_, SqlitePool>, id: i64) -> Result<ProviderResponse> {
    let provider = sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Provider not found".to_string())?;

    let mut response = ProviderResponse::from(provider);

    // Load model maps
    let maps: Vec<(i64, String, String, i64)> = sqlx::query_as(
        "SELECT id, source_model, target_model, enabled FROM provider_model_map WHERE provider_id = ? ORDER BY id",
    )
    .bind(id)
    .fetch_all(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    response.model_maps = maps
        .into_iter()
        .map(
            |(id, source_model, target_model, enabled)| crate::db::models::ModelMapResponse {
                id,
                source_model,
                target_model,
                enabled: enabled != 0,
            },
        )
        .collect();

    // Load model blacklist
    let blacklist: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, model_pattern FROM provider_model_blacklist WHERE provider_id = ? ORDER BY id",
    )
    .bind(id)
    .fetch_all(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    response.model_blacklist = blacklist
        .into_iter()
        .map(|(id, model_pattern)| crate::db::models::ModelBlacklistResponse { id, model_pattern })
        .collect();

    Ok(response)
}

#[tauri::command]
pub async fn create_provider(
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    input: ProviderCreate,
) -> Result<ProviderResponse> {
    let now = chrono::Utc::now().timestamp();
    let cli_type = input.cli_type.unwrap_or_else(|| "claude_code".to_string());
    let provider_name = input.name.clone();

    // Normalize custom_useragent: treat empty string as None
    let custom_ua = input
        .custom_useragent
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let result = sqlx::query(
        r#"
        INSERT INTO providers (cli_type, name, base_url, api_key, enabled, failure_threshold, blacklist_minutes, consecutive_failures, sort_order, custom_useragent, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, 0, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM providers), ?, ?, ?)
        "#,
    )
    .bind(&cli_type)
    .bind(&input.name)
    .bind(&input.base_url)
    .bind(&input.api_key)
    .bind(input.enabled.unwrap_or(true) as i64)
    .bind(input.failure_threshold.unwrap_or(3))
    .bind(input.blacklist_minutes.unwrap_or(10))
    .bind(&custom_ua)
    .bind(now)
    .bind(now)
    .execute(db.inner())
    .await
    .map_err(map_db_error)?;

    let id = result.last_insert_rowid();

    // Insert model maps if provided
    if let Some(model_maps) = input.model_maps {
        for map in model_maps {
            sqlx::query(
                "INSERT INTO provider_model_map (provider_id, source_model, target_model, enabled) VALUES (?, ?, ?, ?)",
            )
            .bind(id)
            .bind(&map.source_model)
            .bind(&map.target_model)
            .bind(map.enabled as i64)
            .execute(db.inner())
            .await
            .map_err(map_db_error)?;
        }
    }

    // Insert model blacklist if provided
    if let Some(model_blacklist) = input.model_blacklist {
        for item in model_blacklist {
            sqlx::query(
                "INSERT INTO provider_model_blacklist (provider_id, model_pattern) VALUES (?, ?)",
            )
            .bind(id)
            .bind(&item.model_pattern)
            .execute(db.inner())
            .await
            .map_err(map_db_error)?;
        }
    }

    // Log system event
    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "provider_created",
        &format!("服务商 {} 已创建", provider_name),
    )
    .await;

    get_provider(db, id).await
}

#[tauri::command]
pub async fn update_provider(
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    id: i64,
    input: ProviderUpdate,
) -> Result<ProviderResponse> {
    let now = chrono::Utc::now().timestamp();

    // Get provider name for logging
    let provider_name: Option<(String,)> =
        sqlx::query_as("SELECT name FROM providers WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?;

    let provider_name = provider_name
        .map(|(n,)| n)
        .unwrap_or_else(|| format!("Provider#{}", id));

    // Check if model maps will be updated (before moving)
    let has_model_maps_update = input.model_maps.is_some();
    let has_model_blacklist_update = input.model_blacklist.is_some();

    // Build dynamic update query
    let mut updates = vec!["updated_at = ?".to_string()];
    let mut has_updates = false;

    if input.name.is_some() {
        updates.push("name = ?".to_string());
        has_updates = true;
    }
    if input.base_url.is_some() {
        updates.push("base_url = ?".to_string());
        has_updates = true;
    }
    if input.api_key.is_some() {
        updates.push("api_key = ?".to_string());
        has_updates = true;
    }
    if input.enabled.is_some() {
        updates.push("enabled = ?".to_string());
        has_updates = true;
    }
    if input.failure_threshold.is_some() {
        updates.push("failure_threshold = ?".to_string());
        has_updates = true;
    }
    if input.blacklist_minutes.is_some() {
        updates.push("blacklist_minutes = ?".to_string());
        has_updates = true;
    }
    if input.custom_useragent.is_some() {
        updates.push("custom_useragent = ?".to_string());
        has_updates = true;
    }

    if has_updates {
        let query = format!("UPDATE providers SET {} WHERE id = ?", updates.join(", "));
        let mut q = sqlx::query(&query).bind(now);

        if let Some(ref name) = input.name {
            q = q.bind(name);
        }
        if let Some(ref base_url) = input.base_url {
            q = q.bind(base_url);
        }
        if let Some(ref api_key) = input.api_key {
            q = q.bind(api_key);
        }
        if let Some(enabled) = input.enabled {
            q = q.bind(enabled as i64);
        }
        if let Some(failure_threshold) = input.failure_threshold {
            q = q.bind(failure_threshold);
        }
        if let Some(blacklist_minutes) = input.blacklist_minutes {
            q = q.bind(blacklist_minutes);
        }
        if let Some(ref custom_useragent) = input.custom_useragent {
            // Normalize: treat empty string as NULL
            let ua = custom_useragent.trim();
            if ua.is_empty() {
                q = q.bind(None::<String>);
            } else {
                q = q.bind(ua);
            }
        }

        q.bind(id).execute(db.inner()).await.map_err(map_db_error)?;
    }

    // Update model maps if provided
    if let Some(model_maps) = input.model_maps {
        // Delete existing maps
        sqlx::query("DELETE FROM provider_model_map WHERE provider_id = ?")
            .bind(id)
            .execute(db.inner())
            .await
            .map_err(map_db_error)?;

        // Insert new maps
        for map in model_maps {
            sqlx::query(
                "INSERT INTO provider_model_map (provider_id, source_model, target_model, enabled) VALUES (?, ?, ?, ?)",
            )
            .bind(id)
            .bind(&map.source_model)
            .bind(&map.target_model)
            .bind(map.enabled as i64)
            .execute(db.inner())
            .await
            .map_err(map_db_error)?;
        }
    }

    // Update model blacklist if provided
    if let Some(model_blacklist) = input.model_blacklist {
        // Delete existing blacklist
        sqlx::query("DELETE FROM provider_model_blacklist WHERE provider_id = ?")
            .bind(id)
            .execute(db.inner())
            .await
            .map_err(map_db_error)?;

        // Insert new blacklist
        for item in model_blacklist {
            sqlx::query(
                "INSERT INTO provider_model_blacklist (provider_id, model_pattern) VALUES (?, ?)",
            )
            .bind(id)
            .bind(&item.model_pattern)
            .execute(db.inner())
            .await
            .map_err(map_db_error)?;
        }
    }

    // Log system event (only if there were actual updates)
    if has_updates || has_model_maps_update || has_model_blacklist_update {
        let _ = crate::services::stats::record_system_log(
            &log_db.0,
            "provider_updated",
            &format!("服务商 {} 已更新", provider_name),
        )
        .await;
    }

    get_provider(db, id).await
}

#[tauri::command]
pub async fn delete_provider(
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    id: i64,
) -> Result<()> {
    // Get provider name before deletion
    let provider_name: Option<(String,)> =
        sqlx::query_as("SELECT name FROM providers WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?;

    let provider_name = provider_name
        .map(|(n,)| n)
        .unwrap_or_else(|| format!("Provider#{}", id));

    // Delete associated model maps first (cascade delete)
    sqlx::query("DELETE FROM provider_model_map WHERE provider_id = ?")
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

    // Delete associated model blacklist
    sqlx::query("DELETE FROM provider_model_blacklist WHERE provider_id = ?")
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

    // Then delete the provider
    sqlx::query("DELETE FROM providers WHERE id = ?")
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

    // Log system event
    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "provider_deleted",
        &format!("服务商 {} 已删除", provider_name),
    )
    .await;

    Ok(())
}

#[tauri::command]
pub async fn reorder_providers(db: State<'_, SqlitePool>, ids: Vec<i64>) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }

    // 使用 CASE WHEN 批量更新（避免 N 次单独更新）
    let case_clauses: Vec<String> = ids
        .iter()
        .enumerate()
        .map(|(idx, id)| format!("WHEN {} THEN {}", id, idx))
        .collect();

    let id_list: Vec<String> = ids.iter().map(|id| id.to_string()).collect();

    let sql = format!(
        "UPDATE providers SET sort_order = CASE id {} END WHERE id IN ({})",
        case_clauses.join(" "),
        id_list.join(", ")
    );

    sqlx::query(&sql)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

    Ok(())
}

#[tauri::command]
pub async fn reset_provider_failures(
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    id: i64,
) -> Result<()> {
    // Get provider name for logging
    let provider_name: Option<(String,)> =
        sqlx::query_as("SELECT name FROM providers WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?;

    let provider_name = provider_name
        .map(|(n,)| n)
        .unwrap_or_else(|| format!("Provider#{}", id));

    sqlx::query(
        "UPDATE providers SET consecutive_failures = 0, blacklisted_until = NULL WHERE id = ?",
    )
    .bind(id)
    .execute(db.inner())
    .await
    .map_err(map_db_error)?;

    // Log system event
    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "provider_reset",
        &format!("服务商 {} 状态已手动重置", provider_name),
    )
    .await;

    Ok(())
}

#[tauri::command]
pub async fn test_provider_models(
    app: tauri::AppHandle,
    db: State<'_, SqlitePool>,
    input: TestProviderModelsInput,
) -> Result<()> {
    use crate::services::provider as provider_service;

    let db_pool = db.inner().clone();
    let model_name = input.model_name.clone();

    for provider_id in input.provider_ids {
        let pool = db_pool.clone();
        let model = model_name.clone();
        let app_handle = app.clone();

        tokio::spawn(async move {
            let result = provider_service::test_provider_model(&pool, provider_id, &model).await;
            if let Err(e) = app_handle.emit("provider-test-result", result) {
                tracing::error!(error = %e, "Failed to emit test result");
            }
        });
    }

    Ok(())
}

// Settings commands
#[tauri::command]
pub async fn get_gateway_settings(db: State<'_, SqlitePool>) -> Result<GatewaySettings> {
    sqlx::query_as::<_, GatewaySettings>("SELECT debug_log FROM gateway_settings WHERE id = 1")
        .fetch_one(db.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_gateway_settings(db: State<'_, SqlitePool>, debug_log: bool) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let debug_log_val = if debug_log { 1i64 } else { 0 };

    sqlx::query("UPDATE gateway_settings SET debug_log = ?, updated_at = ? WHERE id = 1")
        .bind(debug_log_val)
        .bind(now)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

    Ok(())
}

#[tauri::command]
pub async fn get_timeout_settings(db: State<'_, SqlitePool>) -> Result<TimeoutSettings> {
    sqlx::query_as::<_, TimeoutSettings>(
        "SELECT stream_first_byte_timeout, stream_idle_timeout, non_stream_timeout FROM timeout_settings WHERE id = 1",
    )
    .fetch_one(db.inner())
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_timeout_settings(
    db: State<'_, SqlitePool>,
    input: TimeoutSettingsUpdate,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let current = get_timeout_settings(db.clone()).await?;

    sqlx::query(
        "UPDATE timeout_settings SET stream_first_byte_timeout = ?, stream_idle_timeout = ?, non_stream_timeout = ?, updated_at = ? WHERE id = 1",
    )
    .bind(input.stream_first_byte_timeout.unwrap_or(current.stream_first_byte_timeout))
    .bind(input.stream_idle_timeout.unwrap_or(current.stream_idle_timeout))
    .bind(input.non_stream_timeout.unwrap_or(current.non_stream_timeout))
    .bind(now)
    .execute(db.inner())
    .await
    .map_err(map_db_error)?;
    Ok(())
}

#[tauri::command]
pub async fn get_cli_settings(
    db: State<'_, SqlitePool>,
    cli_type: String,
) -> Result<CliSettingsResponse> {
    let row = sqlx::query_as::<_, CliSettingsRowWithoutConfigDir>(
        "SELECT cli_type, default_json_config, cli_mode, config_write_mode, updated_at FROM cli_settings WHERE cli_type = ?",
    )
    .bind(&cli_type)
    .fetch_optional(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    // 获取配置目录
    let config_dir = get_cli_config_dir_path(db.inner(), &cli_type)
        .await
        .to_string_lossy()
        .to_string();
    let default_config_dir = get_default_cli_config_dir(&cli_type)
        .to_string_lossy()
        .to_string();

    if let Some(row) = row {
        let enabled = check_cli_enabled(db.inner(), &cli_type).await;

        Ok(CliSettingsResponse {
            cli_type: row.cli_type,
            enabled,
            default_json_config: row.default_json_config.unwrap_or_default(),
            cli_mode: row.cli_mode,
            config_dir,
            default_config_dir,
            config_write_mode: row.config_write_mode,
        })
    } else {
        Ok(CliSettingsResponse {
            cli_type: cli_type.clone(),
            enabled: false,
            default_json_config: String::new(),
            cli_mode: "proxy".to_string(),
            config_dir,
            default_config_dir,
            config_write_mode: "merge".to_string(),
        })
    }
}

/// CliSettingsRow without config_dir (for backward compatibility with old databases)
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct CliSettingsRowWithoutConfigDir {
    pub cli_type: String,
    pub default_json_config: Option<String>,
    pub cli_mode: String,
    pub config_write_mode: String,
    pub updated_at: i64,
}

#[tauri::command]
pub async fn update_cli_settings(
    db: State<'_, SqlitePool>,
    cli_type: String,
    input: CliSettingsUpdate,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    // Validate and update database
    if let Some(ref config) = input.default_json_config {
        let config_trimmed = config.trim();

        // Validate format if config is not empty
        if !config_trimmed.is_empty() {
            match cli_type.as_str() {
                "claude_code" | "gemini" => {
                    // Validate JSON format
                    serde_json::from_str::<serde_json::Value>(config_trimmed)
                        .map_err(|e| format!("JSON 格式错误: {}", e))?;
                }
                "codex" => {
                    // Validate TOML format
                    config_trimmed
                        .parse::<toml_edit::DocumentMut>()
                        .map_err(|e| format!("TOML 格式错误: {}", e))?;
                }
                _ => {}
            }
        }

        sqlx::query(
            "UPDATE cli_settings SET default_json_config = ?, updated_at = ? WHERE cli_type = ?",
        )
        .bind(config_trimmed)
        .bind(now)
        .bind(&cli_type)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

        // 配置更新后，自动同步到 CLI 配置文件
        let mode: String =
            sqlx::query_as::<_, (String,)>("SELECT cli_mode FROM cli_settings WHERE cli_type = ?")
                .bind(&cli_type)
                .fetch_optional(db.inner())
                .await
                .map_err(|e| e.to_string())?
                .map(|r| r.0)
                .unwrap_or_else(|| "proxy".to_string());

        if mode == "proxy" {
            // 中转模式：如果 CLI 已启用，重新同步配置
            let enabled = check_cli_enabled(db.inner(), &cli_type).await;
            if enabled {
                tracing::info!("{} CLI 已启用，配置变更后自动同步配置文件", cli_type);
                sync_cli_config(db.inner(), &cli_type, true, config_trimmed).await?;
            }
        } else {
            // 直连模式：重新同步凭证配置
            tracing::info!("{} 直连模式，配置变更后自动同步凭证配置", cli_type);
            auto_sync_credential_in_direct_mode(db.inner(), &cli_type).await?;
        }
    }

    // Update config_write_mode if provided
    if let Some(ref write_mode) = input.config_write_mode {
        if write_mode != "overwrite" && write_mode != "merge" {
            return Err("config_write_mode 只能是 'overwrite' 或 'merge'".to_string());
        }
        sqlx::query(
            "UPDATE cli_settings SET config_write_mode = ?, updated_at = ? WHERE cli_type = ?",
        )
        .bind(write_mode)
        .bind(now)
        .bind(&cli_type)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;
    }

    // Update config_dir if provided
    if let Some(ref config_dir) = input.config_dir {
        // 收缩路径为 ~ 开头的相对路径，便于跨设备同步
        let shrunk_path = shrink_home_path(config_dir);
        sqlx::query("UPDATE cli_settings SET config_dir = ?, updated_at = ? WHERE cli_type = ?")
            .bind(&shrunk_path)
            .bind(now)
            .bind(&cli_type)
            .execute(db.inner())
            .await
            .map_err(map_db_error)?;
    }

    // Update CLI config file if enabled flag is provided
    if let Some(enabled) = input.enabled {
        // 检查当前模式
        let current_mode: Option<(String,)> =
            sqlx::query_as("SELECT cli_mode FROM cli_settings WHERE cli_type = ?")
                .bind(&cli_type)
                .fetch_optional(db.inner())
                .await
                .map_err(|e| e.to_string())?;

        let mode = current_mode
            .map(|r| r.0)
            .unwrap_or_else(|| "proxy".to_string());

        // 只有在中转模式下才处理 enabled 参数
        if mode == "proxy" {
            // 检查 CLI 当前是否已经处于目标状态
            let current_enabled = check_cli_enabled(db.inner(), &cli_type).await;

            if current_enabled == enabled {
                tracing::info!(
                    "{} CLI 已经处于目标状态（enabled={}），跳过操作",
                    cli_type,
                    enabled
                );
            } else {
                // Get default_json_config from database (without config_dir to avoid column errors)
                let row = sqlx::query_as::<_, CliSettingsRowWithoutConfigDir>(
                    "SELECT cli_type, default_json_config, cli_mode, config_write_mode, updated_at FROM cli_settings WHERE cli_type = ?",
                )
                .bind(&cli_type)
                .fetch_optional(db.inner())
                .await
                .map_err(|e| e.to_string())?;

                let default_config = row
                    .as_ref()
                    .and_then(|r| r.default_json_config.clone())
                    .unwrap_or_default();
                tracing::info!("{} 执行 CLI 状态切换：enabled={}", cli_type, enabled);
                sync_cli_config(db.inner(), &cli_type, enabled, &default_config).await?;
            }
        } else {
            tracing::info!("{} 处于直连模式，忽略 enabled 参数", cli_type);
        }
    }

    Ok(())
}

// Normalize text for comparison: trim, normalize whitespace, remove extra blank lines
fn normalize_text(text: &str) -> String {
    text.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<&str>>()
        .join("\n")
}

// Check if MCP config exists in the CLI config file - 异步版本，支持自定义配置目录
async fn mcp_enabled_in_file_async(db: &SqlitePool, cli_type: &str, mcp_name: &str) -> bool {
    let config_dir = get_cli_config_dir_path(db, cli_type).await;

    let config_path = match cli_type {
        "claude_code" => config_dir.parent().map(|p| p.join(".claude.json")),
        "gemini" => Some(config_dir.join("settings.json")),
        "codex" => Some(config_dir.join("config.toml")),
        _ => None,
    };

    let path = match config_path {
        Some(p) => p,
        None => return false,
    };

    if !path.exists() {
        return false;
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    match cli_type {
        "claude_code" | "gemini" => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(config) => config
                .get("mcpServers")
                .and_then(|v| v.as_object())
                .map(|servers| servers.contains_key(mcp_name))
                .unwrap_or(false),
            Err(_) => false,
        },
        "codex" => match content.parse::<toml_edit::DocumentMut>() {
            Ok(doc) => doc
                .get("mcp_servers")
                .and_then(|v| v.as_table())
                .map(|servers| servers.contains_key(mcp_name))
                .unwrap_or(false),
            Err(_) => false,
        },
        _ => false,
    }
}

// Check if prompt content matches the file content - 异步版本，支持自定义配置目录
async fn prompt_enabled_in_file_async(
    db: &SqlitePool,
    cli_type: &str,
    prompt_content: &str,
) -> bool {
    let config_dir = get_cli_config_dir_path(db, cli_type).await;

    let prompt_path = match cli_type {
        "claude_code" => config_dir.join("CLAUDE.md"),
        "codex" => config_dir.join("AGENTS.md"),
        "gemini" => config_dir.join("GEMINI.md"),
        _ => return false,
    };

    if !prompt_path.exists() {
        return false;
    }

    let file_content = match std::fs::read_to_string(&prompt_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    // Normalize and compare
    normalize_text(prompt_content) == normalize_text(&file_content)
}

// ============================================================================
// CLI 配置目录获取（统一入口）
// ============================================================================

/// 获取 CLI 配置目录
/// 优先级：数据库配置 > 默认路径
pub async fn get_cli_config_dir_path(db: &SqlitePool, cli_type: &str) -> std::path::PathBuf {
    // 1. 查询数据库
    let result: Option<(Option<String>,)> =
        sqlx::query_as("SELECT config_dir FROM cli_settings WHERE cli_type = ?")
            .bind(cli_type)
            .fetch_optional(db)
            .await
            .ok()
            .flatten();

    // 2. 有配置则展开路径，否则使用默认
    match result.and_then(|r| r.0) {
        Some(path) => std::path::PathBuf::from(expand_home_path(&path)),
        None => get_default_cli_config_dir(cli_type),
    }
}

// ============================================================================
// 内部辅助函数
// ============================================================================

async fn check_cli_enabled(db: &SqlitePool, cli_type: &str) -> bool {
    match cli_type {
        "claude_code" => check_claude_uses_gateway(db, cli_type).await,
        "codex" => check_codex_uses_gateway(db, cli_type).await,
        "gemini" => check_gemini_uses_gateway(db, cli_type).await,
        _ => false,
    }
}

async fn check_claude_uses_gateway(db: &SqlitePool, cli_type: &str) -> bool {
    let config_dir = get_cli_config_dir_path(db, cli_type).await;
    let config_path = config_dir.join("settings.json");

    if !config_path.exists() {
        return false;
    }

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let content_trimmed = content.trim();
    if content_trimmed.is_empty() || content_trimmed == "{}" {
        return false;
    }

    match serde_json::from_str::<serde_json::Value>(content_trimmed) {
        Ok(data) => {
            if let Some(env) = data.get("env") {
                if let Some(base_url) = env.get("ANTHROPIC_BASE_URL").and_then(|v| v.as_str()) {
                    return base_url.contains("127.0.0.1:7788")
                        || base_url.contains("localhost:7788");
                }
            }
            false
        }
        Err(_) => false,
    }
}

async fn check_codex_uses_gateway(db: &SqlitePool, cli_type: &str) -> bool {
    let config_dir = get_cli_config_dir_path(db, cli_type).await;
    let config_path = config_dir.join("config.toml");

    if !config_path.exists() {
        return false;
    }

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    if content.trim().is_empty() {
        return false;
    }

    match content.parse::<toml_edit::DocumentMut>() {
        Ok(doc) => {
            // Check if model_provider is "ccg-gateway"
            if let Some(provider) = doc.get("model_provider").and_then(|v| v.as_str()) {
                if provider == "ccg-gateway" {
                    return true;
                }
            }
            false
        }
        Err(_) => false,
    }
}

async fn check_gemini_uses_gateway(db: &SqlitePool, cli_type: &str) -> bool {
    let config_dir = get_cli_config_dir_path(db, cli_type).await;
    let env_path = config_dir.join(".env");

    if !env_path.exists() {
        return false;
    }

    let content = match std::fs::read_to_string(&env_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    // Check if .env contains GOOGLE_GEMINI_BASE_URL pointing to gateway
    for line in content.lines() {
        if line.starts_with("GOOGLE_GEMINI_BASE_URL=") {
            let url = line.split('=').nth(1).unwrap_or("");
            return url.contains("127.0.0.1:7788") || url.contains("localhost:7788");
        }
    }
    false
}

// Get the config file path for MCP/prompts sync (different for Codex)
async fn get_mcp_config_path(db: &SqlitePool, cli_type: &str) -> Option<std::path::PathBuf> {
    let base_path = get_cli_config_dir_path(db, cli_type).await;

    match cli_type {
        "claude_code" => {
            // Claude Code MCP goes to ~/.claude.json (parent of config_dir)
            base_path.parent().map(|p| p.join(".claude.json"))
        }
        "codex" => Some(base_path.join("config.toml")),
        "gemini" => Some(base_path.join("settings.json")),
        _ => None,
    }
}

async fn get_config_write_mode(db: &SqlitePool, cli_type: &str) -> String {
    sqlx::query_as::<_, (String,)>("SELECT config_write_mode FROM cli_settings WHERE cli_type = ?")
        .bind(cli_type)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .map(|r| r.0)
        .unwrap_or_else(|| "merge".to_string())
}

async fn sync_cli_config(
    db: &SqlitePool,
    cli_type: &str,
    enabled: bool,
    default_config: &str,
) -> Result<()> {
    let write_mode = get_config_write_mode(db, cli_type).await;
    match cli_type {
        "claude_code" => sync_claude_code_config(db, enabled, default_config, &write_mode).await,
        "codex" => sync_codex_config(db, enabled, default_config, &write_mode).await,
        "gemini" => sync_gemini_config(db, enabled, default_config, &write_mode).await,
        _ => Err("Invalid CLI type".to_string()),
    }
}

fn claude_gateway_json_template() -> serde_json::Value {
    serde_json::json!({
        "env": {
            "ANTHROPIC_BASE_URL": "",
            "ANTHROPIC_AUTH_TOKEN": ""
        }
    })
}

fn gemini_gateway_json_template() -> serde_json::Value {
    serde_json::json!({
        "security": {
            "auth": {
                "selectedType": ""
            }
        }
    })
}

fn sanitize_json_config(
    config: serde_json::Value,
    protected_template: &serde_json::Value,
) -> serde_json::Value {
    let mut sanitized = config;
    deep_remove(&mut sanitized, protected_template);
    sanitized
}

fn remove_json_config_content(
    config_path: &std::path::Path,
    gateway_template: &serde_json::Value,
    default_config: &str,
    protected_template: &serde_json::Value,
) -> Result<()> {
    if !config_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(config_path).map_err(|e| {
        tracing::error!("Failed to read {}: {}", config_path.display(), e);
        e.to_string()
    })?;

    let mut config = match serde_json::from_str::<serde_json::Value>(&content) {
        Ok(config) => config,
        Err(e) => {
            tracing::warn!(
                "Failed to parse JSON config {}, leaving file untouched: {}",
                config_path.display(),
                e
            );
            return Ok(());
        }
    };

    deep_remove(&mut config, gateway_template);

    if !default_config.is_empty() {
        if let Ok(preset) = serde_json::from_str::<serde_json::Value>(default_config) {
            let sanitized_preset = sanitize_json_config(preset, protected_template);
            deep_remove(&mut config, &sanitized_preset);
        }
    }

    let config_str = serde_json::to_string_pretty(&config).map_err(|e| {
        tracing::error!(
            "Failed to serialize config {}: {}",
            config_path.display(),
            e
        );
        e.to_string()
    })?;
    std::fs::write(config_path, config_str).map_err(|e| {
        tracing::error!("Failed to write {}: {}", config_path.display(), e);
        e.to_string()
    })?;
    Ok(())
}

fn remove_codex_gateway_auth_content(auth_path: &std::path::Path) -> Result<()> {
    if !auth_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(auth_path).map_err(|e| {
        tracing::error!("Failed to read auth.json: {}", e);
        e.to_string()
    })?;

    let mut auth = match serde_json::from_str::<serde_json::Value>(&content) {
        Ok(auth) => auth,
        Err(e) => {
            tracing::warn!("Failed to parse auth.json, leaving file untouched: {}", e);
            return Ok(());
        }
    };

    if let Some(object) = auth.as_object_mut() {
        let should_remove = object
            .get("OPENAI_API_KEY")
            .and_then(|value| value.as_str())
            .map(|value| value == "ccg-gateway")
            .unwrap_or(false);
        if should_remove {
            object.remove("OPENAI_API_KEY");
        }
    }

    let auth_str = serde_json::to_string_pretty(&auth).map_err(|e| {
        tracing::error!("Failed to serialize auth.json: {}", e);
        e.to_string()
    })?;
    std::fs::write(auth_path, auth_str).map_err(|e| {
        tracing::error!("Failed to write auth.json: {}", e);
        e.to_string()
    })?;
    Ok(())
}

fn remove_gemini_gateway_env_content(env_path: &std::path::Path) -> Result<()> {
    if !env_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(env_path).map_err(|e| {
        tracing::error!("Failed to read .env file: {}", e);
        e.to_string()
    })?;

    let filtered_lines = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed == "GEMINI_API_KEY=ccg-gateway" {
                return false;
            }
            if let Some(url) = trimmed.strip_prefix("GOOGLE_GEMINI_BASE_URL=") {
                if url.contains("127.0.0.1:7788") || url.contains("localhost:7788") {
                    return false;
                }
            }
            true
        })
        .collect::<Vec<_>>();

    let new_content = if filtered_lines.is_empty() {
        String::new()
    } else {
        filtered_lines.join("\n") + "\n"
    };

    std::fs::write(env_path, new_content).map_err(|e| {
        tracing::error!("Failed to write .env file: {}", e);
        e.to_string()
    })?;
    Ok(())
}

fn remove_codex_gateway_config_content(
    config_path: &std::path::Path,
    default_config: &str,
) -> Result<()> {
    if !config_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(config_path).map_err(|e| {
        tracing::error!("Failed to read config.toml: {}", e);
        e.to_string()
    })?;

    let mut doc = match content.parse::<toml_edit::DocumentMut>() {
        Ok(doc) => doc,
        Err(e) => {
            tracing::warn!("Failed to parse config.toml, leaving file untouched: {}", e);
            return Ok(());
        }
    };

    doc.remove("model_provider");
    doc.remove("model_providers");

    if !default_config.is_empty() {
        if let Ok(mut preset_doc) = default_config.parse::<toml_edit::DocumentMut>() {
            preset_doc.remove("model_provider");
            preset_doc.remove("model_providers");
            for (key, _) in preset_doc.iter() {
                doc.remove(key);
            }
        }
    }

    std::fs::write(config_path, doc.to_string()).map_err(|e| {
        tracing::error!("Failed to write config.toml: {}", e);
        e.to_string()
    })?;
    Ok(())
}

fn deep_merge(base: &mut serde_json::Value, override_val: &serde_json::Value) {
    if let (Some(base_obj), Some(override_obj)) = (base.as_object_mut(), override_val.as_object()) {
        for (key, value) in override_obj {
            if let Some(base_value) = base_obj.get_mut(key) {
                if base_value.is_object() && value.is_object() {
                    deep_merge(base_value, value);
                } else {
                    *base_value = value.clone();
                }
            } else {
                base_obj.insert(key.clone(), value.clone());
            }
        }
    }
}

/// 从 base 中移除 template 中出现的所有叶子节点 key，自底向上清理空的中间对象
fn deep_remove(base: &mut serde_json::Value, template: &serde_json::Value) {
    if let (Some(base_obj), Some(tmpl_obj)) = (base.as_object_mut(), template.as_object()) {
        for (key, tmpl_val) in tmpl_obj {
            if tmpl_val.is_object() {
                // 递归移除子节点
                if let Some(base_val) = base_obj.get_mut(key) {
                    deep_remove(base_val, tmpl_val);
                    // 如果子对象变空了，移除这个键
                    if base_val.as_object().map(|o| o.is_empty()).unwrap_or(false) {
                        base_obj.remove(key);
                    }
                }
            } else {
                // 叶子节点：直接移除
                base_obj.remove(key);
            }
        }
    }
}

// Sync Claude Code configuration (settings.json)
async fn sync_claude_code_config(
    db: &SqlitePool,
    enabled: bool,
    default_config: &str,
    write_mode: &str,
) -> Result<()> {
    let config_dir = get_cli_config_dir_path(db, "claude_code").await;
    let config_path = config_dir.join("settings.json");

    let use_merge = write_mode == "merge";

    if enabled {
        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                tracing::error!("Failed to create directory: {}", e);
                e.to_string()
            })?;
        }

        // Build gateway config
        let gateway_config = serde_json::json!({
            "env": {
                "ANTHROPIC_BASE_URL": "http://127.0.0.1:7788",
                "ANTHROPIC_AUTH_TOKEN": "ccg-gateway"
            }
        });
        let protected_gateway_fields = claude_gateway_json_template();

        let mut config = if use_merge {
            // merge 模式：先读取现有文件作为基础
            if config_path.exists() {
                std::fs::read_to_string(&config_path)
                    .ok()
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                    .unwrap_or_else(|| serde_json::json!({}))
            } else {
                serde_json::json!({})
            }
        } else {
            serde_json::json!({})
        };

        // 合并 gateway 配置
        deep_merge(&mut config, &gateway_config);

        // Merge user's custom config if provided
        if !default_config.is_empty() {
            match serde_json::from_str::<serde_json::Value>(default_config) {
                Ok(custom_config) => {
                    let sanitized_config =
                        sanitize_json_config(custom_config, &protected_gateway_fields);
                    deep_merge(&mut config, &sanitized_config);
                }
                Err(e) => {
                    tracing::warn!("Failed to parse custom config (invalid JSON): {}", e);
                }
            }
        }

        // Write config file
        let config_str = serde_json::to_string_pretty(&config).map_err(|e| {
            tracing::error!("Failed to serialize config: {}", e);
            e.to_string()
        })?;
        std::fs::write(&config_path, config_str).map_err(|e| {
            tracing::error!("Failed to write config file: {}", e);
            e.to_string()
        })?;
    } else {
        let gateway_config = claude_gateway_json_template();
        remove_json_config_content(
            &config_path,
            &gateway_config,
            default_config,
            &gateway_config,
        )?;
        tracing::info!("已从 Claude Code settings.json 中移除 gateway 及预设配置");
    }

    Ok(())
}

// Sync Codex configuration (auth.json + config.toml)
async fn sync_codex_config(
    db: &SqlitePool,
    enabled: bool,
    default_config: &str,
    write_mode: &str,
) -> Result<()> {
    let codex_dir = get_cli_config_dir_path(db, "codex").await;
    let auth_path = codex_dir.join("auth.json");
    let config_path = codex_dir.join("config.toml");

    let use_merge = write_mode == "merge";

    if enabled {
        // Create config directory if it doesn't exist
        std::fs::create_dir_all(&codex_dir).map_err(|e| {
            tracing::error!("Failed to create Codex directory: {}", e);
            e.to_string()
        })?;

        // Write auth.json with gateway API key
        let auth = serde_json::json!({
            "OPENAI_API_KEY": "ccg-gateway"
        });
        let auth_str = serde_json::to_string_pretty(&auth).map_err(|e| {
            tracing::error!("Failed to serialize auth.json: {}", e);
            e.to_string()
        })?;
        std::fs::write(&auth_path, auth_str).map_err(|e| {
            tracing::error!("Failed to write auth.json: {}", e);
            e.to_string()
        })?;

        // merge 模式下保留现有文件中未被 gateway / 预设覆盖的顶层 key。
        let existing_content = if use_merge && config_path.exists() {
            std::fs::read_to_string(&config_path).ok()
        } else {
            None
        };

        let mut final_doc = if let Some(ref content) = existing_content {
            content.parse::<toml_edit::DocumentMut>().unwrap_or_else(|e| {
                tracing::warn!("Failed to parse existing Codex config.toml: {}", e);
                toml_edit::DocumentMut::new()
            })
        } else {
            toml_edit::DocumentMut::new()
        };

        // 移除原有的 gateway 相关的配置
        final_doc.remove("model_provider");
        final_doc.remove("model_providers");

        if !default_config.is_empty() {
            match default_config.parse::<toml_edit::DocumentMut>() {
                Ok(mut custom_doc) => {
                    custom_doc.remove("model_provider");
                    custom_doc.remove("model_providers");

                    for (k, v) in custom_doc.iter() {
                        final_doc.insert(&k, v.clone());
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse custom config (invalid TOML): {}", e);
                }
            }
        }

        let gateway_doc = codex_gateway_document()?;
        for (k, v) in gateway_doc.iter() {
            final_doc.insert(&k, v.clone());
        }

        let final_content = final_doc.to_string();

        std::fs::write(&config_path, final_content).map_err(|e| {
            tracing::error!("Failed to write config.toml: {}", e);
            e.to_string()
        })?;
    } else {
        remove_codex_gateway_auth_content(&auth_path)?;
        remove_codex_gateway_config_content(&config_path, default_config)?;
        tracing::info!("已从 Codex 配置中移除 gateway 及预设配置");
    }

    Ok(())
}

// Sync Gemini configuration (settings.json + .env)
async fn sync_gemini_config(
    db: &SqlitePool,
    enabled: bool,
    default_config: &str,
    write_mode: &str,
) -> Result<()> {
    let gemini_dir = get_cli_config_dir_path(db, "gemini").await;
    let config_path = gemini_dir.join("settings.json");
    let env_path = gemini_dir.join(".env");

    let use_merge = write_mode == "merge";

    if enabled {
        // Create config directory if it doesn't exist
        std::fs::create_dir_all(&gemini_dir).map_err(|e| {
            tracing::error!("Failed to create Gemini directory: {}", e);
            e.to_string()
        })?;

        // Write .env file with gateway address
        let env_content =
            "GEMINI_API_KEY=ccg-gateway\nGOOGLE_GEMINI_BASE_URL=http://127.0.0.1:7788\n"
                .to_string();
        std::fs::write(&env_path, env_content).map_err(|e| {
            tracing::error!("Failed to write .env file: {}", e);
            e.to_string()
        })?;

        // Build gateway config
        let gateway_config = serde_json::json!({
            "security": {
                "auth": {
                    "selectedType": "gemini-api-key"
                }
            }
        });
        let protected_gateway_fields = gemini_gateway_json_template();

        let mut config = if use_merge {
            // merge 模式：先读取现有文件作为基础
            if config_path.exists() {
                std::fs::read_to_string(&config_path)
                    .ok()
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                    .unwrap_or_else(|| serde_json::json!({}))
            } else {
                serde_json::json!({})
            }
        } else {
            serde_json::json!({})
        };

        // 合并 gateway 配置
        deep_merge(&mut config, &gateway_config);

        // Merge user's custom config if provided
        if !default_config.is_empty() {
            match serde_json::from_str::<serde_json::Value>(default_config) {
                Ok(custom_config) => {
                    let sanitized_config =
                        sanitize_json_config(custom_config, &protected_gateway_fields);
                    deep_merge(&mut config, &sanitized_config);
                }
                Err(e) => {
                    tracing::warn!("Failed to parse custom config (invalid JSON): {}", e);
                }
            }
        }

        // Write config file
        let config_str = serde_json::to_string_pretty(&config).map_err(|e| {
            tracing::error!("Failed to serialize config.json: {}", e);
            e.to_string()
        })?;
        std::fs::write(&config_path, config_str).map_err(|e| {
            tracing::error!("Failed to write config.json: {}", e);
            e.to_string()
        })?;
    } else {
        let gateway_config = gemini_gateway_json_template();
        remove_gemini_gateway_env_content(&env_path)?;
        remove_json_config_content(
            &config_path,
            &gateway_config,
            default_config,
            &gateway_config,
        )?;
        tracing::info!("已从 Gemini 配置中移除 gateway 及预设配置");
    }

    Ok(())
}

// Log commands
#[tauri::command]
pub async fn get_request_logs(
    log_db: State<'_, crate::LogDb>,
    page: Option<i64>,
    page_size: Option<i64>,
    cli_type: Option<String>,
    provider_name: Option<String>,
) -> Result<PaginatedLogs> {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;
    let pool = &log_db.0;

    let mut sql = "SELECT id, created_at, cli_type, provider_name, model_id, status_code, elapsed_ms, input_tokens, output_tokens, client_method, client_path, source_model, target_model FROM request_logs WHERE 1=1".to_string();
    let mut count_sql = "SELECT COUNT(*) FROM request_logs WHERE 1=1".to_string();

    if cli_type.is_some() {
        sql.push_str(" AND cli_type = ?");
        count_sql.push_str(" AND cli_type = ?");
    }
    if provider_name.is_some() {
        sql.push_str(" AND provider_name = ?");
        count_sql.push_str(" AND provider_name = ?");
    }

    sql.push_str(" ORDER BY id DESC LIMIT ? OFFSET ?");

    let mut q = sqlx::query_as::<_, RequestLogItem>(&sql);

    if let Some(ct) = &cli_type {
        q = q.bind(ct);
    }
    if let Some(pn) = &provider_name {
        q = q.bind(pn);
    }

    q = q.bind(page_size).bind(offset);

    let items = q.fetch_all(pool).await.map_err(|e| e.to_string())?;

    let mut count_q = sqlx::query_as::<_, (i64,)>(&count_sql);
    if let Some(ct) = &cli_type {
        count_q = count_q.bind(ct);
    }
    if let Some(pn) = &provider_name {
        count_q = count_q.bind(pn);
    }

    let (total,) = count_q.fetch_one(pool).await.map_err(|e| e.to_string())?;

    Ok(PaginatedLogs {
        items,
        total,
        page,
        page_size,
    })
}

#[tauri::command]
pub async fn clear_request_logs(log_db: State<'_, crate::LogDb>) -> Result<()> {
    sqlx::query("DELETE FROM request_logs")
        .execute(&log_db.0)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("VACUUM")
        .execute(&log_db.0)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_request_log_detail(
    log_db: State<'_, crate::LogDb>,
    id: i64,
) -> Result<RequestLogDetail> {
    sqlx::query_as::<_, RequestLogDetail>(
        "SELECT id, created_at, cli_type, provider_name, model_id, status_code, elapsed_ms, input_tokens, output_tokens, client_method, client_path, client_headers, client_body, forward_url, forward_headers, forward_body, provider_headers, provider_body, error_message, source_model, target_model FROM request_logs WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&log_db.0)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Log not found".to_string())
}

// System logs commands
#[tauri::command]
pub async fn get_system_logs(
    log_db: State<'_, crate::LogDb>,
    page: Option<i64>,
    page_size: Option<i64>,
    event_type: Option<String>,
) -> Result<SystemLogListResponse> {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    // Build query
    let mut sql =
        "SELECT id, created_at, event_type, message FROM system_logs WHERE 1=1".to_string();
    let mut count_sql = "SELECT COUNT(*) FROM system_logs WHERE 1=1".to_string();

    if event_type.is_some() {
        sql.push_str(" AND event_type = ?");
        count_sql.push_str(" AND event_type = ?");
    }

    sql.push_str(" ORDER BY id DESC LIMIT ? OFFSET ?");
    let mut q = sqlx::query_as::<_, SystemLogItem>(&sql);

    if let Some(et) = &event_type {
        q = q.bind(et);
    }

    q = q.bind(page_size).bind(offset);

    let items = q.fetch_all(&log_db.0).await.map_err(|e| e.to_string())?;

    // Get total count
    let mut count_q = sqlx::query_as::<_, (i64,)>(&count_sql);
    if let Some(et) = &event_type {
        count_q = count_q.bind(et);
    }
    let (total,) = count_q
        .fetch_one(&log_db.0)
        .await
        .map_err(|e| e.to_string())?;

    Ok(SystemLogListResponse {
        items,
        total,
        page,
        page_size,
    })
}

#[tauri::command]
pub async fn clear_system_logs(log_db: State<'_, crate::LogDb>) -> Result<()> {
    sqlx::query("DELETE FROM system_logs")
        .execute(&log_db.0)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("VACUUM")
        .execute(&log_db.0)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// System status
#[tauri::command]
pub async fn get_system_status() -> Result<SystemStatus> {
    Ok(SystemStatus {
        status: "running".to_string(),
        port: 7788,
        uptime: 0,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

// Toggle devtools
#[tauri::command]
pub async fn toggle_devtools(app: tauri::AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_devtools_open() {
            window.close_devtools();
        } else {
            window.open_devtools();
        }
    }
    Ok(())
}

// MCP commands
#[tauri::command]
pub async fn get_mcps(db: State<'_, SqlitePool>) -> Result<Vec<McpResponse>> {
    let mcps = sqlx::query_as::<_, McpConfig>("SELECT * FROM mcp_configs ORDER BY id")
        .fetch_all(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    let cli_types = vec!["claude_code", "codex", "gemini"];

    let mut results = Vec::new();
    for mcp in mcps {
        // Read real status from config files
        let mut cli_flags = Vec::new();
        for cli_type in &cli_types {
            let enabled = mcp_enabled_in_file_async(db.inner(), cli_type, &mcp.name).await;
            cli_flags.push(McpCliFlag {
                cli_type: cli_type.to_string(),
                enabled,
            });
        }

        results.push(McpResponse {
            id: mcp.id,
            name: mcp.name,
            config_json: mcp.config_json,
            cli_flags,
        });
    }
    Ok(results)
}

#[tauri::command]
pub async fn get_mcp(db: State<'_, SqlitePool>, id: i64) -> Result<McpResponse> {
    let mcp = sqlx::query_as::<_, McpConfig>("SELECT * FROM mcp_configs WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "MCP not found".to_string())?;

    // Read real status from config files
    let cli_types = vec!["claude_code", "codex", "gemini"];
    let mut cli_flags = Vec::new();
    for cli_type in &cli_types {
        let enabled = mcp_enabled_in_file_async(db.inner(), cli_type, &mcp.name).await;
        cli_flags.push(McpCliFlag {
            cli_type: cli_type.to_string(),
            enabled,
        });
    }

    Ok(McpResponse {
        id: mcp.id,
        name: mcp.name,
        config_json: mcp.config_json,
        cli_flags,
    })
}

#[tauri::command]
pub async fn create_mcp(db: State<'_, SqlitePool>, input: McpCreate) -> Result<McpResponse> {
    let now = chrono::Utc::now().timestamp();

    // Validate JSON format if config_json is not empty
    let config_trimmed = input.config_json.trim();
    if !config_trimmed.is_empty() {
        serde_json::from_str::<serde_json::Value>(config_trimmed)
            .map_err(|e| format!("JSON 格式错误: {}", e))?;
    }

    let result =
        sqlx::query("INSERT INTO mcp_configs (name, config_json, updated_at) VALUES (?, ?, ?)")
            .bind(&input.name)
            .bind(config_trimmed)
            .bind(now)
            .execute(db.inner())
            .await
            .map_err(map_db_error)?;

    let id = result.last_insert_rowid();

    // Sync to CLI files if cli_flags provided
    let cli_flags = input.cli_flags.unwrap_or_default();
    if !cli_flags.is_empty() {
        sync_single_mcp_to_cli(db.inner(), id, &input.name, config_trimmed, &cli_flags).await?;
    }

    get_mcp(db, id).await
}

#[tauri::command]
pub async fn update_mcp(
    db: State<'_, SqlitePool>,
    id: i64,
    input: McpUpdate,
) -> Result<McpResponse> {
    let now = chrono::Utc::now().timestamp();

    // Validate JSON format if config_json is provided and not empty
    if let Some(ref config) = input.config_json {
        let config_trimmed = config.trim();
        if !config_trimmed.is_empty() {
            serde_json::from_str::<serde_json::Value>(config_trimmed)
                .map_err(|e| format!("JSON 格式错误: {}", e))?;
        }
    }

    let (name, config_json) = if input.name.is_some() || input.config_json.is_some() {
        let current = sqlx::query_as::<_, McpConfig>("SELECT * FROM mcp_configs WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "MCP not found".to_string())?;

        let new_name = input.name.unwrap_or(current.name.clone());
        let new_config = input
            .config_json
            .map(|c| c.trim().to_string())
            .unwrap_or(current.config_json.clone());

        sqlx::query(
            "UPDATE mcp_configs SET name = ?, config_json = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&new_name)
        .bind(&new_config)
        .bind(now)
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

        (new_name, new_config)
    } else {
        // Get current values if not updating
        let current = sqlx::query_as::<_, McpConfig>("SELECT * FROM mcp_configs WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "MCP not found".to_string())?;
        (current.name, current.config_json)
    };

    // Sync to CLI files if cli_flags provided
    if let Some(cli_flags) = input.cli_flags {
        sync_single_mcp_to_cli(db.inner(), id, &name, &config_json, &cli_flags).await?;
    }

    get_mcp(db, id).await
}

#[tauri::command]
pub async fn toggle_mcp_cli(
    db: State<'_, SqlitePool>,
    id: i64,
    cli_type: String,
    enabled: bool,
) -> Result<McpResponse> {
    let mcp = sqlx::query_as::<_, McpConfig>("SELECT * FROM mcp_configs WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "MCP not found".to_string())?;

    sync_mcp_to_cli_async(db.inner(), &mcp.name, &mcp.config_json, &cli_type, enabled).await?;

    get_mcp(db, id).await
}

#[tauri::command]
pub async fn delete_mcp(db: State<'_, SqlitePool>, id: i64) -> Result<()> {
    // Get MCP name before deletion
    let mcp = sqlx::query_as::<_, McpConfig>("SELECT * FROM mcp_configs WHERE id = ?")
        .bind(id)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "MCP not found".to_string())?;

    let mcp_name = mcp.name.clone();

    // Delete from database
    sqlx::query("DELETE FROM mcp_configs WHERE id = ?")
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

    // Remove from all CLI configs
    delete_mcp_from_cli(db.inner(), &mcp_name).await?;

    Ok(())
}

async fn sync_mcp_to_cli_async(
    db: &SqlitePool,
    mcp_name: &str,
    mcp_config_json: &str,
    cli_type: &str,
    is_enabled: bool,
) -> Result<()> {
    let path = get_mcp_config_path(db, cli_type)
        .await
        .ok_or_else(|| format!("Invalid CLI type: {}", cli_type))?;

    if !is_enabled && !path.exists() {
        return Ok(());
    }

    if cli_type == "codex" {
        sync_single_codex_mcp(path, mcp_name, mcp_config_json, is_enabled)?;
        return Ok(());
    }

    let mut config = if path.exists() {
        let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str::<serde_json::Value>(&content)
            .unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if is_enabled {
        if let Ok(mcp_json) = serde_json::from_str::<serde_json::Value>(mcp_config_json) {
            if let Some(obj) = config.as_object_mut() {
                if !obj.contains_key("mcpServers") {
                    obj.insert("mcpServers".to_string(), serde_json::json!({}));
                }
                if let Some(servers) = obj.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
                    servers.insert(mcp_name.to_string(), mcp_json);
                }
            }
        }
    } else if let Some(obj) = config.as_object_mut() {
        if let Some(servers) = obj.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
            servers.remove(mcp_name);
        }
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let config_str = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(&path, config_str).map_err(|e| e.to_string())?;

    Ok(())
}

// Sync a single MCP to CLI files based on enabled flags
async fn sync_single_mcp_to_cli(
    db: &SqlitePool,
    _mcp_id: i64,
    mcp_name: &str,
    mcp_config_json: &str,
    cli_flags: &[McpCliFlag],
) -> Result<()> {
    let cli_types = vec!["claude_code", "codex", "gemini"];

    for cli_type in cli_types {
        // Check if this MCP is enabled for this CLI
        let is_enabled = cli_flags
            .iter()
            .any(|f| f.cli_type == cli_type && f.enabled);

        sync_mcp_to_cli_async(db, mcp_name, mcp_config_json, cli_type, is_enabled).await?;
    }

    Ok(())
}

// Helper function to sync a single MCP to Codex config.toml
fn sync_single_codex_mcp(
    config_path: std::path::PathBuf,
    mcp_name: &str,
    mcp_config_json: &str,
    is_enabled: bool,
) -> Result<()> {
    // Read existing TOML or create new one
    let mut doc = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).map_err(|e| {
            tracing::error!("Failed to read config.toml: {}", e);
            e.to_string()
        })?;
        content
            .parse::<toml_edit::DocumentMut>()
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to parse config.toml, creating new: {}", e);
                toml_edit::DocumentMut::new()
            })
    } else {
        toml_edit::DocumentMut::new()
    };

    // Ensure mcp_servers table exists
    if !doc.contains_table("mcp_servers") {
        doc["mcp_servers"] = toml_edit::table();
    }

    if is_enabled {
        let server_table = parse_codex_mcp_toml_table(mcp_config_json)?;
        doc["mcp_servers"][mcp_name] = toml_edit::Item::Table(server_table);
    } else {
        // Remove this MCP by name
        if let Some(table) = doc.get_mut("mcp_servers").and_then(|v| v.as_table_mut()) {
            table.remove(mcp_name);
        }
    }

    // Write config file
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            tracing::error!("Failed to create directory: {}", e);
            e.to_string()
        })?;
    }
    std::fs::write(&config_path, doc.to_string()).map_err(|e| {
        tracing::error!("Failed to write config.toml: {}", e);
        e.to_string()
    })?;

    Ok(())
}

// Delete a single MCP from all CLI configs
async fn delete_mcp_from_cli(db: &SqlitePool, mcp_name: &str) -> Result<()> {
    let cli_types = vec!["claude_code", "codex", "gemini"];

    for cli_type in cli_types {
        let config_path = get_mcp_config_path(db, cli_type).await;
        if let Some(path) = config_path {
            if !path.exists() {
                continue;
            }

            if cli_type == "codex" {
                // Handle Codex TOML format
                let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                let mut doc = content
                    .parse::<toml_edit::DocumentMut>()
                    .unwrap_or_else(|_| toml_edit::DocumentMut::new());

                if let Some(table) = doc["mcp_servers"].as_table_mut() {
                    table.remove(mcp_name);
                }

                std::fs::write(&path, doc.to_string()).map_err(|e| e.to_string())?;
            } else {
                // Handle Claude/Gemini JSON format
                let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                let mut config: serde_json::Value =
                    serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}));

                if let Some(mcp_servers) =
                    config.get_mut("mcpServers").and_then(|v| v.as_object_mut())
                {
                    mcp_servers.remove(mcp_name);
                }

                let config_str =
                    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
                std::fs::write(&path, config_str).map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(())
}

// Prompt commands
#[tauri::command]
pub async fn get_prompts(db: State<'_, SqlitePool>) -> Result<Vec<PromptResponse>> {
    let prompts = sqlx::query_as::<_, PromptPreset>("SELECT * FROM prompt_presets ORDER BY id")
        .fetch_all(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    let cli_types = vec!["claude_code", "codex", "gemini"];

    let mut results = Vec::new();
    for prompt in prompts {
        // Read real status from prompt files
        let mut cli_flags = Vec::new();
        for cli_type in &cli_types {
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

    // Read real status from prompt files
    let cli_types = vec!["claude_code", "codex", "gemini"];
    let mut cli_flags = Vec::new();
    for cli_type in &cli_types {
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
    let now = chrono::Utc::now().timestamp();

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
    let now = chrono::Utc::now().timestamp();

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
        if !parent.exists() {
            return Ok(());
        }

        if is_enabled {
            std::fs::write(&path, prompt_content).map_err(|e| {
                tracing::error!("Failed to write prompt file: {}", e);
                e.to_string()
            })?;
        } else if path.exists() {
            let file_content = std::fs::read_to_string(&path).unwrap_or_default();
            if normalize_text(prompt_content) == normalize_text(&file_content) {
                std::fs::write(&path, "").map_err(|e| {
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
    let cli_types = vec!["claude_code", "codex", "gemini"];

    for cli_type in cli_types {
        // Check if this prompt is enabled for this CLI
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

    match cli_type {
        "claude_code" => Some(base_path.join("CLAUDE.md")),
        "codex" => Some(base_path.join("AGENTS.md")),
        "gemini" => Some(base_path.join("GEMINI.md")),
        _ => None,
    }
}

// Stats commands
#[tauri::command]
pub async fn get_daily_stats(
    log_db: State<'_, crate::LogDb>,
    start_date: Option<String>,
    end_date: Option<String>,
    cli_type: Option<String>,
) -> Result<Vec<DailyStats>> {
    let pool = &log_db.0;

    let mut query = "SELECT * FROM usage_daily WHERE 1=1".to_string();
    if start_date.is_some() {
        query.push_str(" AND usage_date >= ?");
    }
    if end_date.is_some() {
        query.push_str(" AND usage_date <= ?");
    }
    if cli_type.is_some() {
        query.push_str(" AND cli_type = ?");
    }
    query.push_str(" ORDER BY usage_date DESC");

    let mut q = sqlx::query_as::<_, DailyStats>(&query);
    if let Some(ref sd) = start_date {
        q = q.bind(sd);
    }
    if let Some(ref ed) = end_date {
        q = q.bind(ed);
    }
    if let Some(ref ct) = cli_type {
        q = q.bind(ct);
    }

    q.fetch_all(pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_provider_stats(
    log_db: State<'_, crate::LogDb>,
    start_date: Option<String>,
    end_date: Option<String>,
    cli_type: Option<String>,
    provider_name: Option<String>,
) -> Result<Vec<ProviderStatsResponse>> {
    let pool = &log_db.0;

    let mut query = r#"
        SELECT
            provider_name,
            COUNT(*) as total_requests,
            SUM(CASE WHEN status_code >= 200 AND status_code < 300 THEN 1 ELSE 0 END) as total_success,
            SUM(input_tokens + output_tokens) as total_tokens,
            SUM(elapsed_ms) as total_elapsed_ms
        FROM request_logs
        WHERE 1=1
    "#.to_string();

    if start_date.is_some() {
        query.push_str(" AND datetime(created_at, 'unixepoch', 'localtime') >= ?");
    }
    if end_date.is_some() {
        query.push_str(" AND datetime(created_at, 'unixepoch', 'localtime') <= ?");
    }
    if cli_type.is_some() {
        query.push_str(" AND cli_type = ?");
    }
    if provider_name.is_some() {
        query.push_str(" AND provider_name = ?");
    }
    query.push_str(" GROUP BY provider_name ORDER BY total_requests DESC");

    let mut q = sqlx::query_as::<_, ProviderStatsRow>(&query);
    if let Some(ref sd) = start_date {
        q = q.bind(sd);
    }
    if let Some(ref ed) = end_date {
        q = q.bind(ed);
    }
    if let Some(ref ct) = cli_type {
        q = q.bind(ct);
    }
    if let Some(ref pn) = provider_name {
        q = q.bind(pn);
    }

    let rows = q.fetch_all(pool).await.map_err(|e| e.to_string())?;

    let results = rows
        .into_iter()
        .map(|row| ProviderStatsResponse {
            provider_name: row.provider_name,
            total_requests: row.total_requests,
            total_success: row.total_success,
            total_tokens: row.total_tokens,
            total_elapsed_ms: row.total_elapsed_ms,
            success_rate: if row.total_requests > 0 {
                (row.total_success as f64 / row.total_requests as f64) * 100.0
            } else {
                0.0
            },
        })
        .collect();

    Ok(results)
}

// Session helpers

/// 获取CLI基础目录（异步版本，支持自定义配置目录）
async fn get_cli_base_dir_async(db: &SqlitePool, cli_type: &str) -> std::path::PathBuf {
    get_cli_config_dir_path(db, cli_type).await
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
                    (drive_path.join("my-develop"), 5),
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

// Handle Codex sessions (find by cwd) - 异步版本，支持自定义配置目录
async fn get_codex_sessions_async(
    db: &SqlitePool,
    project_name: &str,
    page: i64,
    page_size: i64,
) -> Result<PaginatedSessions> {
    use std::io::{BufRead, BufReader};
    use walkdir::WalkDir;

    let config_dir = get_cli_config_dir_path(db, "codex").await;
    let sessions_dir = config_dir.join("sessions");

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

// Handle Gemini sessions - 异步版本，支持自定义配置目录
async fn get_gemini_sessions_async(
    db: &SqlitePool,
    project_name: &str,
    page: i64,
    page_size: i64,
) -> Result<PaginatedSessions> {
    let config_dir = get_cli_config_dir_path(db, "gemini").await;
    let chats_dir = config_dir.join("tmp").join(project_name).join("chats");

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

// Parse Codex messages from JSONL file - 异步版本，支持自定义配置目录
async fn get_codex_messages_async(
    db: &SqlitePool,
    session_id: &str,
) -> Result<Vec<SessionMessage>> {
    use std::io::{BufRead, BufReader};
    use walkdir::WalkDir;

    let config_dir = get_cli_config_dir_path(db, "codex").await;
    let sessions_dir = config_dir.join("sessions");

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
                                    if item_type == Some("output_text") || item_type == Some("text")
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
                        let args_str = match serde_json::from_str::<serde_json::Value>(arguments) {
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
                        let output = payload.get("output").and_then(|o| o.as_str()).unwrap_or("");
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

    let base_dir = get_cli_base_dir_async(db.inner(), &cli_type).await;
    let projects_dir = match cli_type.as_str() {
        "codex" => base_dir.join("sessions"),
        "gemini" => base_dir.join("tmp"),
        _ => base_dir.join("projects"),
    };

    // For Codex, we need special handling since sessions are not in project folders
    if cli_type == "codex" {
        return get_codex_projects(projects_dir, page, page_size);
    }

    // For Gemini, check if sessions are in hash directories with chats subfolder
    if cli_type == "gemini" {
        return get_gemini_projects(projects_dir, page, page_size);
    }

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

                    let (display_name, full_path) = if cli_type == "claude_code" {
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

    // Special handling for Codex
    if cli_type == "codex" {
        return get_codex_sessions_async(db.inner(), &project_name, page, page_size).await;
    }

    // Special handling for Gemini
    if cli_type == "gemini" {
        return get_gemini_sessions_async(db.inner(), &project_name, page, page_size).await;
    }

    // Claude Code default handling
    let base_dir = get_cli_base_dir_async(db.inner(), &cli_type).await;
    let project_dir = base_dir.join("projects").join(&project_name);

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
}

#[tauri::command]
pub async fn get_session_messages(
    db: State<'_, SqlitePool>,
    cli_type: String,
    project_name: String,
    session_id: String,
) -> Result<Vec<SessionMessage>> {
    // Special handling for Codex JSONL format
    if cli_type == "codex" {
        return get_codex_messages_async(db.inner(), &session_id).await;
    }

    let base_dir = get_cli_base_dir_async(db.inner(), &cli_type).await;
    let session_file = match cli_type.as_str() {
        "gemini" => base_dir
            .join("tmp")
            .join(&project_name)
            .join("chats")
            .join(format!("{}.json", session_id)),
        _ => base_dir
            .join("projects")
            .join(&project_name)
            .join(format!("{}.jsonl", session_id)),
    };

    let content = std::fs::read_to_string(&session_file)
        .map_err(|e| format!("Failed to read session file: {}", e))?;

    // For Claude Code JSONL format
    if cli_type == "claude_code" {
        return parse_claude_jsonl(&content);
    }

    // For Gemini JSON format
    let json: serde_json::Value = serde_json::from_str(&content)
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

#[tauri::command]
pub async fn delete_session(
    db: State<'_, SqlitePool>,
    cli_type: String,
    project_name: String,
    session_id: String,
) -> Result<()> {
    let base_dir = get_cli_base_dir_async(db.inner(), &cli_type).await;

    // Special handling for Codex - need to search recursively
    if cli_type == "codex" {
        use walkdir::WalkDir;
        let sessions_dir = base_dir.join("sessions");
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
        return Err("Session file not found".to_string());
    }

    let session_file = match cli_type.as_str() {
        "gemini" => base_dir
            .join("tmp")
            .join(&project_name)
            .join("chats")
            .join(format!("{}.json", session_id)),
        _ => base_dir
            .join("projects")
            .join(&project_name)
            .join(format!("{}.jsonl", session_id)),
    };

    if !session_file.exists() {
        return Err(format!(
            "Session file not found: {}",
            session_file.display()
        ));
    }

    std::fs::remove_file(&session_file).map_err(|e| {
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
    let base_dir = get_cli_base_dir_async(db.inner(), &cli_type).await;

    if cli_type == "codex" {
        // For Codex, delete all session files matching the project cwd
        use walkdir::WalkDir;
        let sessions_dir = base_dir.join("sessions");
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
        return Ok(());
    }

    // For Claude Code and Gemini, delete the project directory
    let project_dir = match cli_type.as_str() {
        "gemini" => base_dir.join("tmp").join(&project_name),
        _ => base_dir.join("projects").join(&project_name),
    };

    std::fs::remove_dir_all(&project_dir)
        .map_err(|e| format!("Failed to delete project: {}", e))?;

    Ok(())
}

/// 退出应用程序（导入后需要手动重启）
async fn exit_application() -> Result<()> {
    tokio::spawn(async {
        // 延迟 3 秒，等待响应返回前端并给用户时间看提示
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        std::process::exit(0);
    });

    Ok(())
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
            let now = chrono::Utc::now().timestamp();
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
    let now = chrono::Utc::now().timestamp();
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
pub async fn export_to_local() -> Result<Vec<u8>> {
    // Get the database path from config
    let db_path = get_data_dir().join("ccg_gateway.db");

    // Read the database file
    let content = std::fs::read(&db_path).map_err(|e| format!("Failed to read database: {}", e))?;

    Ok(content)
}

#[tauri::command]
pub async fn import_from_local(data: Vec<u8>) -> Result<()> {
    let db_path = get_data_dir().join("ccg_gateway.db");

    // Write the database file
    std::fs::write(&db_path, &data).map_err(|e| format!("Failed to write database: {}", e))?;

    // 退出应用，用户需手动重启
    exit_application().await?;

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
    let content = std::fs::read(&db_path).map_err(|e| format!("Failed to read database: {}", e))?;

    // Generate filename
    let filename = format!(
        "ccg_gateway_{}.db",
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    );

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
pub async fn import_from_webdav(db: State<'_, SqlitePool>, filename: String) -> Result<()> {
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

    std::fs::write(&db_path, &content).map_err(|e| format!("Failed to write database: {}", e))?;

    // 退出应用，用户需手动重启
    exit_application().await?;

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

// ==================== Skill 相关命令 ====================

fn get_ssot_dir() -> std::path::PathBuf {
    skill::get_ssot_dir()
}

// 获取 CLI 的 skills 目录（异步版本，支持自定义配置目录）
async fn get_skill_cli_dir_async(db: &SqlitePool, cli_type: &str) -> Option<std::path::PathBuf> {
    let config_dir = get_cli_config_dir_path(db, cli_type).await;
    Some(config_dir.join("skills"))
}

// 检查 skill 是否在 CLI 目录中启用（异步版本）
async fn skill_enabled_in_cli_async(db: &SqlitePool, cli_type: &str, directory: &str) -> bool {
    let cli_dir = match get_skill_cli_dir_async(db, cli_type).await {
        Some(d) => d,
        None => return false,
    };

    let skill_path = cli_dir.join(directory);
    skill_path.exists()
}

async fn build_skill_cli_flags(db: &SqlitePool, directory: &str) -> Vec<SkillCliFlag> {
    vec![
        SkillCliFlag {
            cli_type: "claude_code".to_string(),
            enabled: skill_enabled_in_cli_async(db, "claude_code", directory).await,
        },
        SkillCliFlag {
            cli_type: "codex".to_string(),
            enabled: skill_enabled_in_cli_async(db, "codex", directory).await,
        },
        SkillCliFlag {
            cli_type: "gemini".to_string(),
            enabled: skill_enabled_in_cli_async(db, "gemini", directory).await,
        },
    ]
}

// 解析 SKILL.md frontmatter
fn parse_skill_metadata(content: &str) -> (Option<String>, Option<String>) {
    let content = content.trim_start_matches('\u{feff}');
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return (None, None);
    }
    let front_matter = parts[1].trim();
    let mut name = None;
    let mut description = None;
    for line in front_matter.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("name:") {
            name = Some(val.trim().to_string());
        } else if let Some(val) = line.strip_prefix("description:") {
            description = Some(val.trim().to_string());
        }
    }
    (name, description)
}

// 递归复制目录
fn copy_dir_recursive(src: &std::path::Path, dest: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            std::fs::copy(&path, &dest_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn normalize_skill_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn normalize_optional_text(text: Option<String>) -> Option<String> {
    text.and_then(|value| normalize_skill_text(&value))
}

fn skill_install_directory_name_from_parts(directory: &str, repo_name: &str) -> String {
    if directory == "." {
        repo_name.to_string()
    } else {
        std::path::Path::new(directory)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| directory.to_string())
    }
}

fn skill_install_directory_name(skill_item: &DiscoverableSkill) -> String {
    skill_install_directory_name_from_parts(&skill_item.directory, &skill_item.repo.name)
}

fn read_installed_skill_metadata(directory: &str) -> (Option<String>, Option<String>) {
    let skill_md_path = get_ssot_dir().join(directory).join("SKILL.md");
    if !skill_md_path.exists() {
        return (None, None);
    }

    let content = match std::fs::read_to_string(skill_md_path) {
        Ok(content) => content,
        Err(_) => return (None, None),
    };

    parse_skill_metadata(&content)
}

fn file_modified_at(path: &std::path::Path) -> i64 {
    path.metadata()
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_else(|| chrono::Utc::now().timestamp())
}

// 同步 skill 到 CLI 目录（异步版本）
async fn sync_skill_to_cli_async(db: &SqlitePool, directory: &str, cli_type: &str) -> Result<()> {
    let ssot_dir = get_ssot_dir();
    let source = ssot_dir.join(directory);
    if !source.exists() {
        return Err(format!("Skill directory not found: {}", source.display()));
    }
    let cli_dir = match get_skill_cli_dir_async(db, cli_type).await {
        Some(d) => d,
        None => return Err(format!("Unsupported CLI type: {}", cli_type)),
    };

    let dest = cli_dir.join(directory);
    copy_dir_recursive(&source, &dest)?;
    tracing::info!("Synced skill {} to {}", directory, cli_type);
    Ok(())
}

// 从 CLI 目录移除 skill（异步版本）
async fn remove_skill_from_cli_async(
    db: &SqlitePool,
    directory: &str,
    cli_type: &str,
) -> Result<()> {
    let cli_dir = match get_skill_cli_dir_async(db, cli_type).await {
        Some(d) => d,
        None => return Ok(()),
    };
    let skill_folder = cli_dir.join(directory);
    if skill_folder.exists() {
        std::fs::remove_dir_all(&skill_folder).map_err(|e| e.to_string())?;
        tracing::info!("Removed skill {} from {}", directory, cli_type);
    }
    Ok(())
}

// 从所有 CLI 目录移除 skill（异步版本）
async fn remove_skill_from_all_cli_async(db: &SqlitePool, directory: &str) -> Result<()> {
    for cli_type in ["claude_code", "codex", "gemini"] {
        remove_skill_from_cli_async(db, directory, cli_type).await?;
    }
    Ok(())
}

async fn uninstall_skill_directory_async(
    db: &SqlitePool,
    directory: &str,
    error_if_missing: bool,
) -> Result<()> {
    let ssot_dir = get_ssot_dir();
    let skill_path = ssot_dir.join(directory);
    let manifest_exists = skill::load_installed_skill_manifest()?
        .iter()
        .any(|entry| entry.directory == directory);

    if !manifest_exists && !skill_path.exists() {
        if error_if_missing {
            return Err("Skill not found".to_string());
        }
        return Ok(());
    }

    remove_skill_from_all_cli_async(db, directory).await?;

    if skill_path.exists() {
        std::fs::remove_dir_all(&skill_path).map_err(|e| e.to_string())?;
    }

    if manifest_exists {
        skill::remove_installed_skill_manifest_entry(directory)?;
    }

    tracing::info!("Uninstalled skill: {}", directory);
    Ok(())
}

async fn load_installed_skill_responses(db: &SqlitePool) -> Result<Vec<InstalledSkillResponse>> {
    let favorite_keys = get_skill_favorite_keys(db).await?;

    let manifest_entries = skill::load_installed_skill_manifest()?;
    let mut manifest_map = manifest_entries
        .into_iter()
        .map(|entry| (entry.directory.clone(), entry))
        .collect::<HashMap<String, InstalledSkillManifestEntry>>();

    let ssot_dir = get_ssot_dir();
    let mut results = Vec::new();

    for directory in skill::list_installed_skill_directories()? {
        let mut entry =
            manifest_map
                .remove(&directory)
                .unwrap_or_else(|| InstalledSkillManifestEntry {
                    directory: directory.clone(),
                    name: directory.clone(),
                    description: None,
                    repo: None,
                    readme_url: None,
                    installed_at: file_modified_at(&ssot_dir.join(&directory)),
                    source_directory: None,
                });

        let (disk_name, disk_description) = read_installed_skill_metadata(&directory);
        if let Some(name) = normalize_optional_text(disk_name) {
            entry.name = name;
        }
        if let Some(description) = normalize_optional_text(disk_description) {
            entry.description = Some(description);
        } else {
            entry.description = normalize_optional_text(entry.description);
        }

        let (is_favorited, can_favorite, favorite_key, market_display) =
            build_skill_favorite_info(&entry, &favorite_keys);
        let cli_flags = build_skill_cli_flags(db, &directory).await;
        results.push(InstalledSkillResponse {
            id: directory.clone(),
            name: entry.name,
            description: entry.description,
            directory,
            repo: entry.repo,
            readme_url: entry.readme_url,
            installed_at: entry.installed_at,
            cli_flags,
            exists_on_disk: true,
            is_favorited,
            can_favorite,
            favorite_key,
            market_display,
        });
    }

    for (directory, mut entry) in manifest_map {
        entry.description = normalize_optional_text(entry.description);
        let (is_favorited, can_favorite, favorite_key, market_display) =
            build_skill_favorite_info(&entry, &favorite_keys);
        let cli_flags = build_skill_cli_flags(db, &directory).await;
        results.push(InstalledSkillResponse {
            id: directory.clone(),
            name: entry.name,
            description: entry.description,
            directory,
            repo: entry.repo,
            readme_url: entry.readme_url,
            installed_at: entry.installed_at,
            cli_flags,
            exists_on_disk: false,
            is_favorited,
            can_favorite,
            favorite_key,
            market_display,
        });
    }

    results.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(results)
}

fn build_skill_favorite_info(
    entry: &InstalledSkillManifestEntry,
    favorite_keys: &std::collections::HashSet<String>,
) -> (bool, bool, Option<String>, String) {
    if let (Some(repo), Some(source_dir)) = (&entry.repo, &entry.source_directory) {
        let key = format!("{}:{}", repo.name, source_dir);
        let is_favorited = favorite_keys.contains(&key);
        let market_display = if is_local_repo_source(&repo.source) {
            String::new()
        } else {
            format!("@{}", repo.source)
        };
        (is_favorited, true, Some(key), market_display)
    } else if let Some(repo) = &entry.repo {
        // 本地仓库且未保存 source_directory 的旧数据
        if is_local_repo_source(&repo.source) {
            let source_dir = if entry.directory == repo.name {
                "."
            } else {
                &entry.directory
            };
            let key = format!("{}:{}", repo.name, source_dir);
            let is_favorited = favorite_keys.contains(&key);
            (is_favorited, true, Some(key), String::new())
        } else {
            (false, false, None, String::new())
        }
    } else {
        (false, false, None, String::new())
    }
}

// ==================== 仓库管理命令 ====================

#[tauri::command]
pub async fn get_skill_repos() -> Result<Vec<SkillRepo>> {
    skill::load_skill_repos()
}

/// 从 source 提取仓库名称
fn extract_repo_name(source: &str) -> String {
    let source = source.trim().strip_suffix(".git").unwrap_or(source.trim());

    // 本地路径：取最后一段
    if source.contains(':') && source.contains('\\') || source.starts_with('/') {
        return std::path::Path::new(source)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or(source.to_string());
    }

    // URL：取最后一段路径
    source
        .split('/')
        .filter(|s| !s.is_empty())
        .last()
        .unwrap_or(source)
        .to_string()
}

/// 执行 git clone（浅克隆，自动使用主分支）
fn git_clone_repo(source: &str) -> Result<std::path::PathBuf> {
    let cache_dir = skill::get_cached_repo_dir(source);

    // 如果已存在，直接返回
    if cache_dir.exists() {
        return Ok(cache_dir);
    }

    // 补全 URL：owner/repo 格式转为 https://github.com/owner/repo
    let git_url = if source.contains("://") || source.contains("git@") {
        source.to_string()
    } else if source.split('/').filter(|s| !s.is_empty()).count() == 2 {
        format!("https://github.com/{}", source)
    } else {
        source.to_string()
    };

    let output = std::process::Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            &git_url,
            cache_dir.to_str().unwrap_or(""),
        ])
        .output()
        .map_err(|e| format!("git clone 执行失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git clone 失败: {}", stderr));
    }

    tracing::info!("git clone 成功: {} -> {}", git_url, cache_dir.display());
    Ok(cache_dir)
}

#[tauri::command]
pub async fn add_skill_repo(input: SkillRepoCreate) -> Result<SkillRepo> {
    let url = input.url.trim();

    // 1. 本地目录：直接存储
    let path = std::path::Path::new(url);
    if skill::is_local_repo_source(url) && path.exists() && path.is_dir() {
        let repo = SkillRepo {
            name: extract_repo_name(url),
            source: url.to_string(),
        };
        skill::upsert_skill_repo(repo.clone())?;
        return Ok(repo);
    }

    // 2. 远程仓库：尝试 git clone 验证
    git_clone_repo(url)?;

    let repo = SkillRepo {
        name: extract_repo_name(url),
        source: url.to_string(),
    };
    skill::upsert_skill_repo(repo.clone())?;
    Ok(repo)
}

#[tauri::command]
pub async fn remove_skill_repo(db: State<'_, SqlitePool>, name: String) -> Result<()> {
    let installed_directories = skill::load_installed_skill_manifest()?
        .into_iter()
        .filter(|entry| entry.repo.as_ref().map(|repo| repo.name.as_str()) == Some(name.as_str()))
        .map(|entry| entry.directory)
        .collect::<Vec<_>>();

    for directory in installed_directories {
        uninstall_skill_directory_async(db.inner(), &directory, false).await?;
    }

    if let Some(repo) = skill::remove_skill_repo(&name)? {
        if !skill::is_local_repo_source(&repo.source) {
            skill::delete_cached_repo_dir(&repo.source);
        }
    }
    Ok(())
}

async fn sync_skill_favorites_repo(
    db: &SqlitePool,
    old_name: &str,
    repo: &SkillRepo,
) -> Result<()> {
    let favorites = sqlx::query_as::<_, SkillFavorite>(
        "SELECT * FROM skill_favorites WHERE repo_name = ? ORDER BY created_at DESC",
    )
    .bind(old_name)
    .fetch_all(db)
    .await
    .map_err(map_db_error)?;

    for favorite in favorites {
        let new_key = format!("{}:{}", repo.name, favorite.directory);

        sqlx::query("DELETE FROM skill_favorites WHERE skill_key = ? AND id != ?")
            .bind(&new_key)
            .bind(favorite.id)
            .execute(db)
            .await
            .map_err(map_db_error)?;

        sqlx::query(
            "UPDATE skill_favorites SET skill_key = ?, repo_name = ?, repo_source = ? WHERE id = ?",
        )
        .bind(&new_key)
        .bind(&repo.name)
        .bind(&repo.source)
        .bind(favorite.id)
        .execute(db)
        .await
        .map_err(map_db_error)?;
    }

    Ok(())
}

#[tauri::command]
pub async fn update_skill_repo(
    db: State<'_, SqlitePool>,
    old_name: String,
    new_url: String,
) -> Result<SkillRepo> {
    let old_repo = skill::get_skill_repo(&old_name)?;

    // 清理旧缓存
    if let Some(old_repo) = &old_repo {
        if !skill::is_local_repo_source(&old_repo.source) {
            skill::delete_cached_repo_dir(&old_repo.source);
        }
    }

    // 1. 本地目录：直接存储
    let path = std::path::Path::new(&new_url);
    if skill::is_local_repo_source(&new_url) && path.exists() && path.is_dir() {
        let repo = SkillRepo {
            name: extract_repo_name(&new_url),
            source: new_url.to_string(),
        };
        skill::replace_skill_repo(&old_name, repo.clone())?;
        sync_skill_favorites_repo(db.inner(), &old_name, &repo).await?;
        return Ok(repo);
    }

    // 2. 远程仓库：尝试 git clone 验证
    git_clone_repo(&new_url)?;

    let repo = SkillRepo {
        name: extract_repo_name(&new_url),
        source: new_url.to_string(),
    };
    skill::replace_skill_repo(&old_name, repo.clone())?;
    sync_skill_favorites_repo(db.inner(), &old_name, &repo).await?;
    Ok(repo)
}

// ==================== Skill 发现命令 ====================

#[tauri::command]
pub async fn discover_repo_skills(
    db: State<'_, SqlitePool>,
    name: String,
) -> Result<Vec<DiscoverableSkill>> {
    let repo = skill::get_skill_repo(&name)?.ok_or_else(|| format!("未找到仓库 '{}'", name))?;

    let favorite_keys = get_skill_favorite_keys(db.inner()).await?;

    // 1. 本地目录
    if skill::is_local_repo_source(&repo.source) {
        return scan_local_repo_skills(&repo, &favorite_keys).await;
    }

    // 2. 远程仓库：git clone 或使用缓存
    let cache_dir = git_clone_repo(&repo.source)?;
    scan_cached_repo_skills(&cache_dir, &repo, &favorite_keys)
}

#[tauri::command]
pub async fn refresh_repo_skills(
    db: State<'_, SqlitePool>,
    name: String,
) -> Result<Vec<DiscoverableSkill>> {
    let repo = skill::get_skill_repo(&name)?.ok_or_else(|| format!("未找到仓库 '{}'", name))?;

    let favorite_keys = get_skill_favorite_keys(db.inner()).await?;

    // 1. 本地目录
    if skill::is_local_repo_source(&repo.source) {
        return scan_local_repo_skills(&repo, &favorite_keys).await;
    }

    // 2. 远程仓库：删除缓存后重新 clone
    skill::delete_cached_repo_dir(&repo.source);
    let cache_dir = git_clone_repo(&repo.source)?;
    scan_cached_repo_skills(&cache_dir, &repo, &favorite_keys)
}

async fn get_skill_favorite_keys(db: &SqlitePool) -> Result<std::collections::HashSet<String>> {
    let keys: Vec<String> = sqlx::query("SELECT skill_key FROM skill_favorites")
        .map(|row: sqlx::sqlite::SqliteRow| row.get::<String, _>(0))
        .fetch_all(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(keys.into_iter().collect())
}

/// 扫描缓存的仓库目录
fn scan_cached_repo_skills(
    cache_dir: &std::path::Path,
    repo: &SkillRepo,
    favorite_keys: &std::collections::HashSet<String>,
) -> Result<Vec<DiscoverableSkill>> {
    let mut skills = Vec::new();

    for entry in walkdir::WalkDir::new(cache_dir)
        .max_depth(5)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_name() == "SKILL.md" {
            let file_path = entry.path();
            let parent_dir = file_path.parent().unwrap_or(cache_dir);

            let relative_path = parent_dir
                .strip_prefix(cache_dir)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            let content = std::fs::read_to_string(file_path).map_err(|e| e.to_string())?;
            let (skill_name, description) = parse_skill_metadata(&content);

            let directory_name = if relative_path.is_empty() {
                cache_dir
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or("repo".to_string())
            } else {
                parent_dir
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| relative_path.clone())
            };

            let directory_str = if relative_path.is_empty() {
                ".".to_string()
            } else {
                relative_path
            };
            let install_dir = skill_install_directory_name_from_parts(&directory_str, &repo.name);
            let key = format!("{}:{}", repo.name, &directory_str);
            let is_installed = get_ssot_dir().join(&install_dir).exists();
            skills.push(DiscoverableSkill {
                key: key.clone(),
                name: skill_name.unwrap_or_else(|| directory_name.clone()),
                description: description.unwrap_or_default(),
                directory: directory_str,
                install_directory: install_dir,
                readme_url: None,
                repo: repo.clone(),
                is_favorited: favorite_keys.contains(&key),
                is_installed,
            });
        }
    }

    // 排序：收藏优先，已安装次之，最后按名称
    skills.sort_by(|a, b| {
        if a.is_favorited != b.is_favorited {
            return a.is_favorited.cmp(&b.is_favorited).reverse();
        }
        if a.is_installed != b.is_installed {
            return a.is_installed.cmp(&b.is_installed).reverse();
        }
        a.name.to_lowercase().cmp(&b.name.to_lowercase())
    });
    Ok(skills)
}

/// 扫描本地仓库目录
async fn scan_local_repo_skills(
    repo: &SkillRepo,
    favorite_keys: &std::collections::HashSet<String>,
) -> Result<Vec<DiscoverableSkill>> {
    let root_path = std::path::Path::new(&repo.source);

    if !root_path.exists() || !root_path.is_dir() {
        return Err(format!("本地目录 {} 不存在", repo.source));
    }

    scan_cached_repo_skills(root_path, repo, favorite_keys)
}

// ==================== Skill 安装/卸载命令 ====================

async fn install_skill_inner(
    db: &SqlitePool,
    skill_item: DiscoverableSkill,
    reinstall: bool,
) -> Result<InstalledSkillResponse> {
    let ssot_dir = get_ssot_dir();
    let directory_name = skill_install_directory_name(&skill_item);
    let skill_path = ssot_dir.join(&directory_name);
    let existing = skill::load_installed_skill_manifest()?
        .into_iter()
        .find(|entry| entry.directory == directory_name);

    skill::ensure_repo_exists(&skill_item.repo)?;

    if (existing.is_some() || skill_path.exists()) && !reinstall {
        return Err(format!("Skill '{}' is already installed", directory_name));
    }

    // 如果是重装，先删除旧的 SSOT 目录
    if reinstall && skill_path.exists() {
        let _ = std::fs::remove_dir_all(&skill_path);
    }

    // 根据类型进行安装
    if skill::is_local_repo_source(&skill_item.repo.source) {
        let source_path = std::path::Path::new(&skill_item.repo.source);
        let skill_source_path = if skill_item.directory == "." {
            source_path.to_path_buf()
        } else {
            source_path.join(&skill_item.directory)
        };

        let dest_path = ssot_dir.join(&directory_name);
        if !skill_source_path.exists() {
            return Err(format!("技能目录不存在: {}", skill_source_path.display()));
        }

        copy_dir_recursive(&skill_source_path, &dest_path)?;
    } else {
        // 从 git clone 缓存目录复制
        let cache_dir = git_clone_repo(&skill_item.repo.source)?;
        let skill_source_path = if skill_item.directory == "." {
            cache_dir.to_path_buf()
        } else {
            cache_dir.join(&skill_item.directory)
        };

        let dest_path = ssot_dir.join(&directory_name);
        if !skill_source_path.exists() {
            return Err(format!("技能目录不存在: {}", skill_source_path.display()));
        }

        copy_dir_recursive(&skill_source_path, &dest_path)?;
    }

    let now = chrono::Utc::now().timestamp();
    skill::upsert_installed_skill_manifest_entry(InstalledSkillManifestEntry {
        directory: directory_name.clone(),
        name: directory_name.clone(),
        description: None,
        repo: Some(skill_item.repo.clone()),
        readme_url: None,
        installed_at: now,
        source_directory: Some(skill_item.directory.clone()),
    })?;

    let mut cli_flags = build_skill_cli_flags(db, &directory_name).await;
    for flag in cli_flags.iter().filter(|flag| flag.enabled) {
        sync_skill_to_cli_async(db, &directory_name, &flag.cli_type).await?;
    }
    cli_flags = build_skill_cli_flags(db, &directory_name).await;

    Ok(InstalledSkillResponse {
        id: directory_name.clone(),
        name: skill_item.name,
        description: normalize_skill_text(&skill_item.description),
        directory: directory_name,
        repo: Some(skill_item.repo.clone()),
        readme_url: skill_item.readme_url,
        installed_at: now,
        cli_flags,
        exists_on_disk: true,
        is_favorited: skill_item.is_favorited,
        can_favorite: true,
        favorite_key: Some(skill_item.key.clone()),
        market_display: if is_local_repo_source(&skill_item.repo.source) {
            String::new()
        } else {
            format!("@{}", skill_item.repo.source)
        },
    })
}

#[tauri::command]
pub async fn install_skill(
    db: State<'_, SqlitePool>,
    skill: DiscoverableSkill,
    reinstall: Option<bool>,
) -> Result<InstalledSkillResponse> {
    install_skill_inner(db.inner(), skill, reinstall.unwrap_or(false)).await
}

#[tauri::command]
pub async fn reinstall_installed_skill(
    db: State<'_, SqlitePool>,
    directory: String,
) -> Result<InstalledSkillResponse> {
    let manifest_entries = skill::load_installed_skill_manifest()?;
    let entry = manifest_entries
        .iter()
        .find(|e| e.directory == directory)
        .ok_or_else(|| format!("Skill '{}' not found in manifest", directory))?;

    let repo = entry
        .repo
        .as_ref()
        .ok_or_else(|| "Skill missing repo info, cannot reinstall".to_string())?;

    let source_dir = entry
        .source_directory
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or_else(|| {
            if skill::is_local_repo_source(&repo.source) && directory == repo.name {
                "."
            } else {
                &directory
            }
        });

    let key = format!("{}:{}", repo.name, source_dir);

    let (disk_name, disk_description) = read_installed_skill_metadata(&directory);
    let name = disk_name.unwrap_or_else(|| entry.name.clone());
    let description = disk_description.unwrap_or_default();

    let discoverable = DiscoverableSkill {
        key,
        name,
        description,
        directory: source_dir.to_string(),
        install_directory: directory.clone(),
        readme_url: None,
        repo: repo.clone(),
        is_favorited: false,
        is_installed: true,
    };

    install_skill_inner(db.inner(), discoverable, true).await
}

#[tauri::command]
pub async fn uninstall_skill(db: State<'_, SqlitePool>, id: String) -> Result<()> {
    uninstall_skill_directory_async(db.inner(), &id, true).await
}

// ==================== 已安装 Skill 管理命令 ====================

#[tauri::command]
pub async fn get_installed_skills(
    db: State<'_, SqlitePool>,
) -> Result<Vec<InstalledSkillResponse>> {
    load_installed_skill_responses(db.inner()).await
}

#[tauri::command]
pub async fn toggle_skill_cli(
    db: State<'_, SqlitePool>,
    id: String,
    cli_type: String,
    enabled: bool,
) -> Result<()> {
    let directory = id;
    if enabled {
        sync_skill_to_cli_async(db.inner(), &directory, &cli_type).await?;
    } else {
        remove_skill_from_cli_async(db.inner(), &directory, &cli_type).await?;
    }

    Ok(())
}

// ==================== Skill 收藏命令 ====================

#[tauri::command]
pub async fn get_skill_favorites(db: State<'_, SqlitePool>) -> Result<Vec<SkillFavoriteItem>> {
    let favorites = sqlx::query_as::<_, SkillFavorite>(
        "SELECT * FROM skill_favorites ORDER BY created_at DESC",
    )
    .fetch_all(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    let ssot_dir = get_ssot_dir();
    Ok(favorites
        .into_iter()
        .map(|favorite| {
            let repo = SkillRepo {
                name: favorite.repo_name,
                source: favorite.repo_source,
            };
            let installed_directory =
                skill_install_directory_name_from_parts(&favorite.directory, &repo.name);
            SkillFavoriteItem {
                key: favorite.skill_key,
                name: favorite.name,
                description: favorite.description,
                directory: favorite.directory,
                readme_url: favorite.readme_url,
                repo,
                is_installed: ssot_dir.join(installed_directory).exists(),
            }
        })
        .collect())
}

#[tauri::command]
pub async fn add_skill_favorite(
    db: State<'_, SqlitePool>,
    skill_item: DiscoverableSkill,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT OR REPLACE INTO skill_favorites (skill_key, name, description, directory, readme_url, repo_name, repo_source, repo_branch, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&skill_item.key)
    .bind(&skill_item.name)
    .bind(normalize_skill_text(&skill_item.description))
    .bind(&skill_item.directory)
    .bind(&skill_item.readme_url)
    .bind(&skill_item.repo.name)
    .bind(&skill_item.repo.source)
    .bind(None::<String>)  // repo_branch 不再使用
    .bind(now)
    .execute(db.inner())
    .await
    .map_err(map_db_error)?;
    Ok(())
}

#[tauri::command]
pub async fn toggle_installed_skill_favorite(
    db: State<'_, SqlitePool>,
    directory: String,
) -> Result<bool> {
    let manifest_entries = skill::load_installed_skill_manifest()?;
    let entry = manifest_entries
        .iter()
        .find(|e| e.directory == directory)
        .ok_or_else(|| format!("Skill '{}' not found in manifest", directory))?;

    let (repo, source_dir) = match (&entry.repo, &entry.source_directory) {
        (Some(r), Some(s)) => (r, s),
        (Some(r), None) => {
            // 旧数据，尝试推断 source_directory
            if is_local_repo_source(&r.source) && entry.directory == r.name {
                static DOT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
                (r, DOT.get_or_init(|| ".".to_string()))
            } else {
                return Err("Skill missing source_directory info".to_string());
            }
        }
        _ => return Err("Skill missing repo info".to_string()),
    };

    let key = format!("{}:{}", repo.name, source_dir);

    // 检查是否已收藏
    let existing = sqlx::query("SELECT 1 FROM skill_favorites WHERE skill_key = ?")
        .bind(&key)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    if existing.is_some() {
        // 已收藏，删除
        sqlx::query("DELETE FROM skill_favorites WHERE skill_key = ?")
            .bind(&key)
            .execute(db.inner())
            .await
            .map_err(map_db_error)?;
        Ok(false)
    } else {
        // 未收藏，添加
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO skill_favorites (skill_key, name, description, directory, readme_url, repo_name, repo_source, repo_branch, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&key)
        .bind(&entry.name)
        .bind(&entry.description)
        .bind(source_dir)
        .bind(&entry.readme_url)
        .bind(&repo.name)
        .bind(&repo.source)
        .bind(None::<String>)
        .bind(now)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;
        Ok(true)
    }
}

#[tauri::command]
pub async fn remove_skill_favorite(db: State<'_, SqlitePool>, key: String) -> Result<()> {
    sqlx::query("DELETE FROM skill_favorites WHERE skill_key = ?")
        .bind(&key)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;
    Ok(())
}

#[tauri::command]
pub async fn install_favorite_skill(
    db: State<'_, SqlitePool>,
    key: String,
) -> Result<InstalledSkillResponse> {
    let favorite =
        sqlx::query_as::<_, SkillFavorite>("SELECT * FROM skill_favorites WHERE skill_key = ?")
            .bind(&key)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Skill favorite not found".to_string())?;

    let repo = SkillRepo {
        name: favorite.repo_name.clone(),
        source: favorite.repo_source.clone(),
    };

    // 确保仓库缓存存在
    let cache_dir = if skill::is_local_repo_source(&repo.source) {
        std::path::Path::new(&repo.source).to_path_buf()
    } else {
        git_clone_repo(&repo.source)?
    };

    // 检查 directory 是否有效，无效则从仓库扫描修复
    let skill_path = if favorite.directory == "." {
        cache_dir.to_path_buf()
    } else {
        cache_dir.join(&favorite.directory)
    };

    let (directory, skill_key) = if skill_path.exists() {
        (favorite.directory.clone(), favorite.skill_key.clone())
    } else {
        // 扫描仓库找到正确的 skill
        let skills = scan_cached_repo_skills(&cache_dir, &repo, &std::collections::HashSet::new())?;
        let skill = skills
            .iter()
            .find(|s| s.name == favorite.name)
            .ok_or_else(|| format!("未在仓库中找到技能: {}", favorite.name))?;

        // 更新数据库中的 directory 和 key
        sqlx::query("UPDATE skill_favorites SET directory = ?, skill_key = ? WHERE skill_key = ?")
            .bind(&skill.directory)
            .bind(&skill.key)
            .bind(&favorite.skill_key)
            .execute(db.inner())
            .await
            .map_err(map_db_error)?;

        (skill.directory.clone(), skill.key.clone())
    };

    install_skill_inner(
        db.inner(),
        DiscoverableSkill {
            key: skill_key,
            name: favorite.name,
            description: favorite.description.unwrap_or_default(),
            directory: directory.clone(),
            install_directory: skill_install_directory_name_from_parts(
                &directory,
                &favorite.repo_name,
            ),
            readme_url: favorite.readme_url,
            repo,
            is_favorited: true,
            is_installed: false,
        },
        false,
    )
    .await
}

// ==================== 检查更新命令 ====================

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

// ==================== Official Credential 相关命令 ====================

/// 解析凭证 JSON 生成显示信息
fn parse_display_info(cli_type: &str, credential_json: &str) -> String {
    // 尝试解析为文件列表格式
    if let Ok(files) = serde_json::from_str::<Vec<serde_json::Value>>(credential_json) {
        match cli_type {
            "claude_code" => {
                // 查找 settings.json 文件
                if let Some(file) = files.iter().find(|f| {
                    f.get("path")
                        .and_then(|p| p.as_str())
                        .map(|p| p.contains("settings.json"))
                        .unwrap_or(false)
                }) {
                    if let Some(content) = file.get("content").and_then(|c| c.as_str()) {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(content) {
                            return data
                                .get("ANTHROPIC_API_KEY")
                                .and_then(|v| v.as_str())
                                .map(|key| {
                                    if key.len() > 12 {
                                        format!("sk-ant-...{}", &key[key.len() - 8..])
                                    } else {
                                        "已配置".to_string()
                                    }
                                })
                                .unwrap_or_else(|| "未知".to_string());
                        }
                    }
                }
                "未配置".to_string()
            }
            "codex" => {
                // 查找 auth.json 文件
                if let Some(file) = files.iter().find(|f| {
                    f.get("path")
                        .and_then(|p| p.as_str())
                        .map(|p| p.contains("auth.json"))
                        .unwrap_or(false)
                }) {
                    if let Some(content) = file.get("content").and_then(|c| c.as_str()) {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(content) {
                            return data
                                .get("tokens")
                                .and_then(|t| t.get("access_token"))
                                .and_then(|v| v.as_str())
                                .map(|_| "已配置".to_string())
                                .unwrap_or_else(|| "未知".to_string());
                        }
                    }
                }
                "未配置".to_string()
            }
            "gemini" => {
                // 查找 google_accounts.json 文件
                if let Some(file) = files.iter().find(|f| {
                    f.get("path")
                        .and_then(|p| p.as_str())
                        .map(|p| p.contains("google_accounts.json"))
                        .unwrap_or(false)
                }) {
                    if let Some(content) = file.get("content").and_then(|c| c.as_str()) {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(content) {
                            return data
                                .get("active")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "已配置".to_string());
                        }
                    }
                }
                "未配置".to_string()
            }
            _ => "未知".to_string(),
        }
    } else {
        // 兼容旧格式：直接解析为 JSON 对象
        match serde_json::from_str::<serde_json::Value>(credential_json) {
            Ok(creds) => match cli_type {
                "claude_code" => creds
                    .get("ANTHROPIC_API_KEY")
                    .and_then(|v| v.as_str())
                    .map(|key| {
                        if key.len() > 12 {
                            format!("sk-ant-...{}", &key[key.len() - 8..])
                        } else {
                            "已配置".to_string()
                        }
                    })
                    .unwrap_or_else(|| "未知".to_string()),
                "codex" => creds
                    .get("tokens")
                    .and_then(|t| t.get("active_email"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "已配置".to_string()),
                "gemini" => creds
                    .get("google_accounts")
                    .and_then(|g| g.get("active"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "已配置".to_string()),
                _ => "未知".to_string(),
            },
            Err(_) => "无效 JSON".to_string(),
        }
    }
}

/// 读取 CLI 当前凭证（异步版本，支持自定义配置目录）
async fn read_cli_credential_impl_async(db: &SqlitePool, cli_type: &str) -> Result<String> {
    let config_dir = get_cli_config_dir_path(db, cli_type).await;

    match cli_type {
        "claude_code" => {
            let config_path = config_dir.join("settings.json");

            // 如果文件不存在，返回空内容（而不是报错）
            if !config_path.exists() {
                let files = vec![serde_json::json!({
                    "path": format!("{}/settings.json", config_dir.display()),
                    "content": ""
                })];
                return Ok(serde_json::to_string(&files).unwrap());
            }

            let content =
                std::fs::read_to_string(&config_path).map_err(|e| format!("读取失败: {}", e))?;

            let files = vec![serde_json::json!({
                "path": format!("{}/settings.json", config_dir.display()),
                "content": content
            })];
            Ok(serde_json::to_string(&files).unwrap())
        }
        "codex" => {
            let auth_path = config_dir.join("auth.json");

            // 如果文件不存在，返回空的文件列表（而不是报错）
            if !auth_path.exists() {
                let files = vec![serde_json::json!({
                    "path": format!("{}/auth.json", config_dir.display()),
                    "content": ""
                })];
                return Ok(serde_json::to_string(&files).unwrap());
            }

            let content =
                std::fs::read_to_string(&auth_path).map_err(|e| format!("读取失败: {}", e))?;

            let files = vec![serde_json::json!({
                "path": format!("{}/auth.json", config_dir.display()),
                "content": content
            })];
            Ok(serde_json::to_string(&files).unwrap())
        }
        "gemini" => {
            let oauth_path = config_dir.join("oauth_creds.json");
            let accounts_path = config_dir.join("google_accounts.json");
            let settings_path = config_dir.join("settings.json");

            let mut files = vec![];

            // 即使文件不存在，也添加空内容的占位符
            if oauth_path.exists() {
                let content = std::fs::read_to_string(&oauth_path)
                    .map_err(|e| format!("读取 oauth_creds.json 失败: {}", e))?;
                files.push(serde_json::json!({
                    "path": format!("{}/oauth_creds.json", config_dir.display()),
                    "content": content
                }));
            } else {
                files.push(serde_json::json!({
                    "path": format!("{}/oauth_creds.json", config_dir.display()),
                    "content": ""
                }));
            }

            if accounts_path.exists() {
                let content = std::fs::read_to_string(&accounts_path)
                    .map_err(|e| format!("读取 google_accounts.json 失败: {}", e))?;
                files.push(serde_json::json!({
                    "path": format!("{}/google_accounts.json", config_dir.display()),
                    "content": content
                }));
            } else {
                files.push(serde_json::json!({
                    "path": format!("{}/google_accounts.json", config_dir.display()),
                    "content": ""
                }));
            }

            if settings_path.exists() {
                let content = std::fs::read_to_string(&settings_path)
                    .map_err(|e| format!("读取 settings.json 失败: {}", e))?;
                files.push(serde_json::json!({
                    "path": format!("{}/settings.json", config_dir.display()),
                    "content": content
                }));
            } else {
                files.push(serde_json::json!({
                    "path": format!("{}/settings.json", config_dir.display()),
                    "content": ""
                }));
            }

            Ok(serde_json::to_string(&files).unwrap())
        }
        _ => Err("Unsupported CLI type".to_string()),
    }
}

/// 同步凭证到 CLI 配置文件（异步版本，支持自定义配置目录）
async fn sync_credential_to_cli_async(
    db: &SqlitePool,
    cli_type: &str,
    credential_json: &str,
    default_config: &str,
) -> Result<()> {
    // 解析文件列表
    let files: Vec<serde_json::Value> = serde_json::from_str(credential_json)
        .map_err(|e| format!("解析凭证文件列表失败: {}", e))?;

    let config_dir = get_cli_config_dir_path(db, cli_type).await;
    let write_mode = get_config_write_mode(db, cli_type).await;
    let use_merge = write_mode == "merge";

    match cli_type {
        "claude_code" => {
            // TODO: Claude Code 的具体实现待完善
            tracing::warn!("Claude Code 的直连模式配置写入功能尚未实现");
        }
        "codex" => {
            let auth_path = config_dir.join("auth.json");
            let config_path = config_dir.join("config.toml");

            // 直连模式不备份
            std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;

            // 查找 auth.json 文件
            let auth_file = files.iter().find(|f| {
                f.get("path")
                    .and_then(|p| p.as_str())
                    .map(|p| p.contains("auth.json"))
                    .unwrap_or(false)
            });

            if let Some(file) = auth_file {
                let content = file.get("content").and_then(|c| c.as_str()).unwrap_or("");

                // 只有当内容不为空时才写入
                if !content.is_empty() {
                    tracing::info!(
                        "写入 Codex auth.json，内容长度: {}，路径: {:?}",
                        content.len(),
                        auth_path
                    );
                    std::fs::write(&auth_path, content).map_err(|e| {
                        tracing::error!("写入 auth.json 失败: {}", e);
                        e.to_string()
                    })?;
                    tracing::info!("Codex auth.json 写入成功");
                } else {
                    tracing::warn!("Codex auth.json 内容为空，跳过写入");
                }
            } else {
                tracing::warn!("未找到 Codex auth.json 文件配置");
            }

            // 处理 config.toml
            if !default_config.is_empty() {
                let existing_content = if use_merge && config_path.exists() {
                    std::fs::read_to_string(&config_path).ok()
                } else {
                    None
                };

                let mut final_doc = if let Some(ref content) = existing_content {
                    content.parse::<toml_edit::DocumentMut>().unwrap_or_else(|e| {
                        tracing::warn!("Failed to parse existing Codex config.toml: {}", e);
                        toml_edit::DocumentMut::new()
                    })
                } else {
                    toml_edit::DocumentMut::new()
                };

                match default_config.parse::<toml_edit::DocumentMut>() {
                    Ok(custom_doc) => {
                        for (k, v) in custom_doc.iter() {
                            final_doc.insert(&k, v.clone());
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to parse Codex default_config (invalid TOML): {}",
                            e
                        );
                    }
                }

                let final_content = final_doc.to_string();

                tracing::info!(
                    "写入 Codex config.toml（{}模式），路径: {:?}",
                    write_mode,
                    config_path
                );
                std::fs::write(&config_path, final_content).map_err(|e| {
                    tracing::error!("写入 config.toml 失败: {}", e);
                    e.to_string()
                })?;
                tracing::info!("Codex config.toml 写入成功");
            } else {
                tracing::info!("Codex 全局配置为空，跳过写入 config.toml");
            }
        }
        "gemini" => {
            let oauth_path = config_dir.join("oauth_creds.json");
            let accounts_path = config_dir.join("google_accounts.json");
            let settings_path = config_dir.join("settings.json");
            let env_path = config_dir.join(".env");

            // 直连模式不备份
            std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;

            // 用于存储 settings.json 的内容
            let mut settings_content: Option<String> = None;

            // 写入各个文件
            for file in files.iter() {
                let path_str = file.get("path").and_then(|p| p.as_str()).unwrap_or("");
                let content = file.get("content").and_then(|c| c.as_str()).unwrap_or("");

                if path_str.contains("oauth_creds.json") && !content.is_empty() {
                    tracing::info!(
                        "写入 Gemini oauth_creds.json，内容长度: {}，路径: {:?}",
                        content.len(),
                        oauth_path
                    );
                    std::fs::write(&oauth_path, content).map_err(|e| {
                        tracing::error!("写入 oauth_creds.json 失败: {}", e);
                        e.to_string()
                    })?;
                    tracing::info!("Gemini oauth_creds.json 写入成功");
                } else if path_str.contains("google_accounts.json") && !content.is_empty() {
                    tracing::info!(
                        "写入 Gemini google_accounts.json，内容长度: {}，路径: {:?}",
                        content.len(),
                        accounts_path
                    );
                    std::fs::write(&accounts_path, content).map_err(|e| {
                        tracing::error!("写入 google_accounts.json 失败: {}", e);
                        e.to_string()
                    })?;
                    tracing::info!("Gemini google_accounts.json 写入成功");
                } else if path_str.contains("settings.json") && !content.is_empty() {
                    settings_content = Some(content.to_string());
                }
            }

            // 删除 .env 文件（如果存在）
            if env_path.exists() {
                let _ = std::fs::remove_file(&env_path);
            }

            // 处理 settings.json
            // 1. 根据写入模式决定是否读取现有文件作为基础
            let mut config = if use_merge && settings_path.exists() {
                std::fs::read_to_string(&settings_path)
                    .ok()
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                    .unwrap_or_else(|| serde_json::json!({}))
            } else {
                serde_json::json!({})
            };

            // 2. 合并凭证中的 settings.json（凭证配置覆盖现有配置）
            if let Some(ref user_settings) = settings_content {
                tracing::info!(
                    "Gemini 凭证中包含 settings.json，内容长度: {}",
                    user_settings.len()
                );
                if let Ok(cred_val) = serde_json::from_str::<serde_json::Value>(user_settings) {
                    deep_merge(&mut config, &cred_val);
                    tracing::info!("Gemini 凭证配置合并成功");
                }
            } else {
                tracing::info!("Gemini 凭证中不包含 settings.json");
            }

            // 3. 合并全局配置（全局配置优先级最高）
            if !default_config.is_empty() {
                tracing::info!("Gemini 全局配置不为空，长度: {}", default_config.len());
                if let Ok(default_val) = serde_json::from_str::<serde_json::Value>(default_config) {
                    deep_merge(&mut config, &default_val);
                    tracing::info!("Gemini 全局配置合并成功");
                }
            } else {
                tracing::info!("Gemini 全局配置为空");
            }

            // 检查最终配置
            let is_empty = config.as_object().map(|o| o.is_empty()).unwrap_or(true);
            tracing::info!("Gemini settings.json 最终配置是否为空: {}", is_empty);

            // 只有当配置不为空对象时才写入
            if !is_empty {
                tracing::info!(
                    "写入 Gemini settings.json（{}模式），路径: {:?}",
                    write_mode,
                    settings_path
                );
                std::fs::write(
                    &settings_path,
                    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?,
                )
                .map_err(|e| {
                    tracing::error!("写入 settings.json 失败: {}", e);
                    e.to_string()
                })?;
                tracing::info!("Gemini settings.json 写入成功");
            } else {
                tracing::warn!("Gemini settings.json 配置为空对象，跳过写入");
            }
        }
        _ => return Err("不支持的 CLI 类型".to_string()),
    }

    Ok(())
}

/// 在直连模式下，自动同步第一个凭证到 CLI 配置文件
async fn auto_sync_credential_in_direct_mode(db: &SqlitePool, cli_type: &str) -> Result<()> {
    tracing::info!(
        "auto_sync_credential_in_direct_mode 被调用，cli_type: {}",
        cli_type
    );

    // 检查当前是否为直连模式
    let current_mode: Option<(String,)> =
        sqlx::query_as("SELECT cli_mode FROM cli_settings WHERE cli_type = ?")
            .bind(cli_type)
            .fetch_optional(db)
            .await
            .map_err(|e| e.to_string())?;

    let mode = current_mode
        .map(|r| r.0)
        .unwrap_or_else(|| "proxy".to_string());
    tracing::info!("{} 当前模式: {}", cli_type, mode);

    if mode != "direct" {
        tracing::debug!("{} 当前不是直连模式，跳过自动同步", cli_type);
        return Ok(());
    }

    // 获取第一个凭证（sort_order = 0）
    let cred: Option<OfficialCredential> = sqlx::query_as(
        "SELECT * FROM official_credentials WHERE cli_type = ? AND sort_order = 0 LIMIT 1",
    )
    .bind(cli_type)
    .fetch_optional(db)
    .await
    .map_err(|e| e.to_string())?;

    if let Some(cred) = cred {
        tracing::info!("{} 找到凭证 ID: {}, 名称: {}", cli_type, cred.id, cred.name);

        // 获取全局配置
        let default_config = sqlx::query_as::<_, (Option<String>,)>(
            "SELECT default_json_config FROM cli_settings WHERE cli_type = ?",
        )
        .bind(cli_type)
        .fetch_optional(db)
        .await
        .map_err(|e| e.to_string())?
        .and_then(|r| r.0)
        .unwrap_or_default();

        tracing::info!("{} 全局配置长度: {}", cli_type, default_config.len());
        tracing::info!("{} 开始同步凭证到文件", cli_type);

        match sync_credential_to_cli_async(db, cli_type, &cred.credential_json, &default_config)
            .await
        {
            Ok(_) => {
                tracing::info!("{} 凭证同步成功", cli_type);
                Ok(())
            }
            Err(e) => {
                tracing::error!("{} 凭证同步失败: {}", cli_type, e);
                Err(e)
            }
        }
    } else {
        tracing::warn!("{} 没有可用的凭证，跳过同步", cli_type);
        Ok(())
    }
}

/// 删除直连模式写入的所有文件（异步版本，支持自定义配置目录）
async fn remove_direct_mode_files_async(db: &SqlitePool, cli_type: &str) -> Result<()> {
    let config_dir = get_cli_config_dir_path(db, cli_type).await;
    let use_merge = get_config_write_mode(db, cli_type).await == "merge";

    match cli_type {
        "claude_code" => {
            // TODO: Claude Code 的具体实现待完善
            tracing::warn!("Claude Code 的直连模式文件删除功能尚未实现");
            Ok(())
        }
        "codex" => remove_codex_direct_mode_files(&config_dir, use_merge),
        "gemini" => remove_gemini_direct_mode_files(&config_dir, use_merge),
        _ => Err("不支持的 CLI 类型".to_string()),
    }
}

#[tauri::command]
pub async fn get_credentials(
    db: State<'_, SqlitePool>,
    cli_type: String,
) -> Result<Vec<OfficialCredentialResponse>> {
    let creds: Vec<OfficialCredential> = sqlx::query_as(
        "SELECT * FROM official_credentials WHERE cli_type = ? ORDER BY sort_order, id",
    )
    .bind(&cli_type)
    .fetch_all(db.inner())
    .await
    .map_err(|e| e.to_string())?;

    let results = creds
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            let display_info = parse_display_info(&c.cli_type, &c.credential_json);
            OfficialCredentialResponse {
                is_active: i == 0,
                id: c.id,
                cli_type: c.cli_type,
                name: c.name,
                credential_json: c.credential_json,
                sort_order: c.sort_order,
                display_info,
            }
        })
        .collect();

    Ok(results)
}

#[tauri::command]
pub async fn create_credential(
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    input: OfficialCredentialCreate,
) -> Result<OfficialCredentialResponse> {
    let now = chrono::Utc::now().timestamp();

    // Check if this is the first credential for this cli_type
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM official_credentials WHERE cli_type = ?")
            .bind(&input.cli_type)
            .fetch_one(db.inner())
            .await
            .map_err(|e| e.to_string())?;

    let sort_order = if count.0 == 0 { 0i64 } else { count.0 };

    let result = sqlx::query(
        "INSERT INTO official_credentials (cli_type, name, credential_json, sort_order, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&input.cli_type)
    .bind(&input.name)
    .bind(&input.credential_json)
    .bind(sort_order)
    .bind(now)
    .bind(now)
    .execute(db.inner())
    .await
    .map_err(map_db_error)?;

    let id = result.last_insert_rowid();

    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "credential_created",
        &format!("凭证 {} 已创建", input.name),
    )
    .await;

    // 如果是直连模式，自动同步到文件
    if let Err(e) = auto_sync_credential_in_direct_mode(db.inner(), &input.cli_type).await {
        tracing::error!("自动同步凭证失败: {}", e);
    }

    get_credential(db, id).await
}

#[tauri::command]
pub async fn get_credential(
    db: State<'_, SqlitePool>,
    id: i64,
) -> Result<OfficialCredentialResponse> {
    let cred =
        sqlx::query_as::<_, OfficialCredential>("SELECT * FROM official_credentials WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "凭证不存在".to_string())?;

    Ok(OfficialCredentialResponse {
        is_active: cred.sort_order == 0,
        id: cred.id,
        display_info: parse_display_info(&cred.cli_type, &cred.credential_json),
        cli_type: cred.cli_type,
        name: cred.name,
        credential_json: cred.credential_json,
        sort_order: cred.sort_order,
    })
}

#[tauri::command]
pub async fn update_credential(
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    id: i64,
    input: OfficialCredentialUpdate,
) -> Result<OfficialCredentialResponse> {
    let now = chrono::Utc::now().timestamp();

    let cred_name: Option<(String,)> =
        sqlx::query_as("SELECT name FROM official_credentials WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?;

    let cred_name = cred_name.ok_or_else(|| "凭证不存在".to_string())?.0;

    let mut updates = vec!["updated_at = ?".to_string()];
    if input.name.is_some() {
        updates.push("name = ?".to_string());
    }
    if input.credential_json.is_some() {
        updates.push("credential_json = ?".to_string());
    }

    let query = format!(
        "UPDATE official_credentials SET {} WHERE id = ?",
        updates.join(", ")
    );
    let mut q = sqlx::query(&query).bind(now);
    if let Some(ref name) = input.name {
        q = q.bind(name);
    }
    if let Some(ref json) = input.credential_json {
        q = q.bind(json);
    }
    q.bind(id).execute(db.inner()).await.map_err(map_db_error)?;

    let _ = crate::services::stats::record_system_log(
        &log_db.0,
        "credential_updated",
        &format!("凭证 {} 已更新", cred_name),
    )
    .await;

    // 获取更新后的凭证信息
    let updated_cred: Option<OfficialCredential> =
        sqlx::query_as("SELECT * FROM official_credentials WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?;

    // 如果是直连模式，自动同步到文件
    if let Some(cred) = updated_cred {
        if let Err(e) = auto_sync_credential_in_direct_mode(db.inner(), &cred.cli_type).await {
            tracing::error!("自动同步凭证失败: {}", e);
        }
    }

    get_credential(db, id).await
}

#[tauri::command]
pub async fn delete_credential(
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    id: i64,
) -> Result<()> {
    let cred_info: Option<(String,)> =
        sqlx::query_as("SELECT name FROM official_credentials WHERE id = ?")
            .bind(id)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?;

    if let Some((name,)) = cred_info {
        let _ = crate::services::stats::record_system_log(
            &log_db.0,
            "credential_deleted",
            &format!("凭证 {} 已删除", name),
        )
        .await;
    }

    sqlx::query("DELETE FROM official_credentials WHERE id = ?")
        .bind(id)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

    Ok(())
}

#[tauri::command]
pub async fn reorder_credentials(db: State<'_, SqlitePool>, ids: Vec<i64>) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }

    // 获取第一个凭证的 cli_type（用于后续同步）
    let cli_type: Option<(String,)> =
        sqlx::query_as("SELECT cli_type FROM official_credentials WHERE id = ?")
            .bind(ids[0])
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?;

    // 使用 CASE WHEN 批量更新（避免 N 次单独更新）
    let case_clauses: Vec<String> = ids
        .iter()
        .enumerate()
        .map(|(idx, id)| format!("WHEN {} THEN {}", id, idx))
        .collect();

    let id_list: Vec<String> = ids.iter().map(|id| id.to_string()).collect();

    let sql = format!(
        "UPDATE official_credentials SET sort_order = CASE id {} END WHERE id IN ({})",
        case_clauses.join(" "),
        id_list.join(", ")
    );

    sqlx::query(&sql)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

    // 如果是直连模式，自动同步到文件
    if let Some((cli_type_str,)) = cli_type {
        if let Err(e) = auto_sync_credential_in_direct_mode(db.inner(), &cli_type_str).await {
            tracing::error!("自动同步凭证失败: {}", e);
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn read_cli_credential(db: State<'_, SqlitePool>, cli_type: String) -> Result<String> {
    read_cli_credential_impl_async(db.inner(), &cli_type).await
}

#[tauri::command]
pub async fn get_cli_mode(db: State<'_, SqlitePool>, cli_type: String) -> Result<String> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT cli_mode FROM cli_settings WHERE cli_type = ?")
            .bind(&cli_type)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?;

    Ok(row.map(|r| r.0).unwrap_or_else(|| "proxy".to_string()))
}

#[tauri::command]
pub async fn set_cli_mode(
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    cli_type: String,
    mode: String,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    let current_mode: Option<(String,)> =
        sqlx::query_as("SELECT cli_mode FROM cli_settings WHERE cli_type = ?")
            .bind(&cli_type)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?;

    let current_mode = current_mode
        .map(|r| r.0)
        .unwrap_or_else(|| "proxy".to_string());

    if current_mode == mode {
        return Ok(());
    }

    sqlx::query("UPDATE cli_settings SET cli_mode = ?, updated_at = ? WHERE cli_type = ?")
        .bind(&mode)
        .bind(now)
        .bind(&cli_type)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

    if mode == "direct" {
        // 步骤1: 如果从中转模式切换过来，先关闭 CLI
        if current_mode == "proxy" {
            // 检查是否真的有中转配置（通过检查配置文件）
            let has_gateway_config = check_cli_enabled(db.inner(), &cli_type).await;

            if has_gateway_config {
                let default_config = sqlx::query_as::<_, (Option<String>,)>(
                    "SELECT default_json_config FROM cli_settings WHERE cli_type = ?",
                )
                .bind(&cli_type)
                .fetch_optional(db.inner())
                .await
                .map_err(|e| e.to_string())?
                .and_then(|r| r.0)
                .unwrap_or_default();

                tracing::info!("{} 从中转模式切换到直连模式，先关闭 CLI", cli_type);
                // 关闭中转模式（会自动处理备份恢复）
                sync_cli_config(db.inner(), &cli_type, false, &default_config).await?;
            } else {
                tracing::info!("{} 当前没有中转配置，跳过关闭 CLI 步骤", cli_type);
            }
        }

        // 步骤2: 应用直连模式配置
        if let Ok(Some(cred)) = sqlx::query_as::<_, OfficialCredential>(
            "SELECT * FROM official_credentials WHERE cli_type = ? AND sort_order = 0 LIMIT 1",
        )
        .bind(&cli_type)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())
        {
            let default_config = sqlx::query_as::<_, (Option<String>,)>(
                "SELECT default_json_config FROM cli_settings WHERE cli_type = ?",
            )
            .bind(&cli_type)
            .fetch_optional(db.inner())
            .await
            .map_err(|e| e.to_string())?
            .and_then(|r| r.0)
            .unwrap_or_default();

            tracing::info!("开始同步 {} 凭证到文件", cli_type);
            match sync_credential_to_cli_async(
                db.inner(),
                &cli_type,
                &cred.credential_json,
                &default_config,
            )
            .await
            {
                Ok(_) => {
                    tracing::info!("{} 凭证同步成功", cli_type);
                }
                Err(e) => {
                    tracing::error!("{} 凭证同步失败: {}", cli_type, e);
                    return Err(format!("同步凭证失败: {}", e));
                }
            }
        } else {
            tracing::warn!("{} 没有可用的凭证", cli_type);
        }

        let _ = crate::services::stats::record_system_log(
            &log_db.0,
            "cli_mode_changed",
            &format!("{} 已切换到直连模式", cli_type),
        )
        .await;
    } else {
        // 切换到中转模式

        // 步骤1: 如果从直连模式切换过来，先删除直连模式的文件
        if current_mode == "direct" {
            tracing::info!("{} 从直连模式切换到中转模式，先删除直连模式文件", cli_type);
            remove_direct_mode_files_async(db.inner(), &cli_type).await?;
        }

        // 步骤2: 开启中转模式
        let default_config = sqlx::query_as::<_, (Option<String>,)>(
            "SELECT default_json_config FROM cli_settings WHERE cli_type = ?",
        )
        .bind(&cli_type)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .and_then(|r| r.0)
        .unwrap_or_default();

        sync_cli_config(db.inner(), &cli_type, true, &default_config).await?;

        let _ = crate::services::stats::record_system_log(
            &log_db.0,
            "cli_mode_changed",
            &format!("{} 已切换到中转模式", cli_type),
        )
        .await;
    }

    Ok(())
}

// ==================== 插件管理命令 ====================

use crate::services::plugin::{MarketplaceActionResult, PluginActionResult};

/// 获取收藏列表
async fn get_favorites_raw(
    db: &SqlitePool,
) -> Result<Vec<(String, String, String, Option<String>)>> {
    let favorites: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT plugin_id, plugin_name, marketplace_name, marketplace_source FROM plugin_favorites",
    )
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(favorites)
}

#[tauri::command]
pub async fn get_installed_plugins(db: State<'_, SqlitePool>) -> Result<Vec<PluginItem>> {
    let config_dir = get_cli_config_dir_path(db.inner(), "claude_code").await;
    crate::services::plugin::get_installed_plugins(&config_dir)
}

#[tauri::command]
pub async fn get_marketplace_plugins(
    db: State<'_, SqlitePool>,
    market_name: String,
) -> Result<Vec<PluginItem>> {
    let config_dir = get_cli_config_dir_path(db.inner(), "claude_code").await;
    crate::services::plugin::get_marketplace_plugins(&market_name, &config_dir)
}

#[tauri::command]
pub async fn get_plugin_favorites(db: State<'_, SqlitePool>) -> Result<Vec<PluginFavoriteItem>> {
    let favorites = get_favorites_raw(db.inner()).await?;
    crate::services::plugin::get_favorites(favorites)
}

#[tauri::command]
pub async fn get_marketplaces(db: State<'_, SqlitePool>) -> Result<Vec<MarketplaceInfo>> {
    let config_dir = get_cli_config_dir_path(db.inner(), "claude_code").await;
    crate::services::plugin::get_marketplaces(&config_dir)
}

#[tauri::command]
pub async fn plugin_action(
    action: String,
    plugin_id: String,
) -> Result<PluginActionResult> {
    crate::services::plugin::plugin_action(&action, &plugin_id)
}

#[tauri::command]
pub async fn add_plugin_favorite(
    db: State<'_, SqlitePool>,
    plugin_id: String,
    plugin_name: String,
    marketplace_name: String,
) -> Result<String> {
    let config_dir = get_cli_config_dir_path(db.inner(), "claude_code").await;
    let marketplace_source =
        crate::services::plugin::get_marketplace_source_info(&config_dir, &marketplace_name);
    let source_type =
        crate::services::plugin::get_marketplace_source_type(&config_dir, &marketplace_name);

    let now = chrono::Utc::now().timestamp();

    sqlx::query(
        "INSERT OR REPLACE INTO plugin_favorites (plugin_id, plugin_name, marketplace_name, created_at, marketplace_source) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&plugin_id)
    .bind(&plugin_name)
    .bind(&marketplace_name)
    .bind(now)
    .bind(&marketplace_source)
    .execute(db.inner())
    .await
    .map_err(map_db_error)?;

    if source_type.as_deref() == Some("directory") {
        Ok("该插件来自本地市场，可能不支持跨设备恢复".to_string())
    } else {
        Ok(String::new())
    }
}

#[tauri::command]
pub async fn remove_plugin_favorite(db: State<'_, SqlitePool>, plugin_id: String) -> Result<()> {
    sqlx::query("DELETE FROM plugin_favorites WHERE plugin_id = ?")
        .bind(&plugin_id)
        .execute(db.inner())
        .await
        .map_err(map_db_error)?;

    Ok(())
}

#[tauri::command]
pub async fn marketplace_action(
    action: String,
    param: String,
) -> Result<MarketplaceActionResult> {
    crate::services::plugin::marketplace_action(&action, &param)
}

#[tauri::command]
pub async fn install_favorite_plugin(
    db: State<'_, SqlitePool>,
    plugin_id: String,
    marketplace_name: String,
    marketplace_source: Option<String>,
) -> Result<crate::services::plugin::FavoriteInstallResult> {
    let config_dir = get_cli_config_dir_path(db.inner(), "claude_code").await;
    crate::services::plugin::install_favorite_plugin(
        &plugin_id,
        &marketplace_name,
        marketplace_source.as_deref(),
        &config_dir,
    )
}
