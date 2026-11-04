use crate::db::models::{
    ConfigFormat, ConfigOperation, ConfigOperationKind, OfficialCredentialFile,
    OfficialCredentialPayload, OfficialLoginFeature, OfficialLoginOperation,
    OfficialLoginOperationKind, OfficialManagedConfig,
};
use crate::services::cli_config::{get_cli_config_dir_path, resolve_cli_config_file_from_dir};
use crate::services::{agent, config_patch};
use serde_json::{Map, Value};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug)]
enum PreparedFile {
    Fields(ConfigFormat, Value),
    Text(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dsh_account_snapshot_excludes_shared_records_and_switch_merges_grant() {
        let root =
            std::env::temp_dir().join(format!("ccg-dsh-account-check-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&root).await.unwrap();
        let db = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("CREATE TABLE cli_settings (cli_type TEXT, config_dir TEXT)")
            .execute(&db)
            .await
            .unwrap();
        sqlx::query("INSERT INTO cli_settings VALUES ('deepseek_harness', ?)")
            .bind(root.to_string_lossy().as_ref())
            .execute(&db)
            .await
            .unwrap();
        let path = root.join(".credentials.yaml");
        let original = "version: 1\nrefs:\n  CCG_GATEWAY_API_KEY: gateway-token\nrecords:\n  deepseek-account-platform/default:\n    kind: grant\n    payload:\n      version: 1\n      token: account-one\n      issuer: https://platform.deepseek.com\n  deepseek-account-platform/device:\n    kind: grant\n    payload: device-identity\n  client-connection/browser-session:\n    kind: grant\n    payload: browser-session\n";
        tokio::fs::write(&path, original).await.unwrap();
        let snapshot = read_current_payload(&db, "deepseek_harness").await.unwrap();
        assert!(!snapshot.contains("gateway-token"));
        assert!(!snapshot.contains("device-identity"));
        assert!(!snapshot.contains("browser-session"));
        let mut payload: Value = serde_json::from_str(&snapshot).unwrap();
        payload["files"]["deepseek_account"]["content"]["account"]["payload"]["token"] =
            serde_json::json!("account-two");
        let snapshot = serde_json::to_string(&payload).unwrap();
        apply_payload(&db, "deepseek_harness", &snapshot)
            .await
            .unwrap();
        assert!(payload_matches(&db, "deepseek_harness", &snapshot)
            .await
            .unwrap());
        let switched: Value =
            serde_yaml::from_str(&tokio::fs::read_to_string(&path).await.unwrap()).unwrap();
        assert_eq!(
            switched["records"]["deepseek-account-platform/default"]["payload"]["token"],
            "account-two"
        );
        remove_payload(&db, "deepseek_harness", &snapshot)
            .await
            .unwrap();
        let removed: Value =
            serde_yaml::from_str(&tokio::fs::read_to_string(&path).await.unwrap()).unwrap();
        let mut expected: Value = serde_yaml::from_str(original).unwrap();
        expected["records"]
            .as_object_mut()
            .unwrap()
            .remove("deepseek-account-platform/default");
        assert_eq!(removed, expected);
        db.close().await;
        tokio::fs::remove_dir_all(&root).await.unwrap();
    }
}

fn operation_format(operation: &OfficialLoginOperation) -> ConfigFormat {
    operation.format.unwrap_or(ConfigFormat::Json)
}

pub fn parse_payload(json: &str) -> Result<OfficialCredentialPayload, String> {
    let payload: OfficialCredentialPayload = serde_json::from_str(json)
        .map_err(|error| crate::tr!("err.official_credential.payload_json_invalid", error))?;
    if payload.schema_version != 1 {
        return Err(crate::tr!(
            "err.official_credential.schema_version_unsupported",
            payload.schema_version
        ));
    }
    if payload.files.is_empty() {
        return Err(crate::tr!("err.official_credential.files_empty"));
    }
    Ok(payload)
}

pub fn validate_payload(agent_id: &str, json: &str) -> Result<(), String> {
    agent::validate_agent_id(agent_id)?;
    let payload = parse_payload(json)?;
    let definition = agent::get_definition(agent_id).expect("validated Agent");
    let feature = &definition.features.official_login;
    if !feature.enabled {
        return Err(crate::tr!("err.official_credential.feature_disabled_for_agent", agent_id));
    }
    for operation in &feature.operations {
        let source = match operation.op {
            OfficialLoginOperationKind::ReplaceFile => operation.content_from.as_ref(),
            OfficialLoginOperationKind::SetField => operation.value_from.as_ref(),
        };
        if let Some(source) = source {
            let file = payload.files.get(&source.file_id).ok_or_else(|| {
                crate::tr!(
                    "err.official_credential.missing_logical_file",
                    source.file_id
                )
            })?;
            let expected_format = operation.source_format();
            if file.format != expected_format {
                return Err(crate::tr!(
                    "err.official_credential.logical_file_format_expected",
                    source.file_id,
                    format!("{:?}", expected_format)
                ));
            }
            if file.format == ConfigFormat::Json && value_at(&file.content, &source.path).is_none()
            {
                return Err(crate::tr!(
                    "err.official_credential.missing_source_path",
                    source.file_id,
                    source.path.join(".")
                ));
            }
            if file.format != ConfigFormat::Json
                && (!source.path.is_empty() || !file.content.is_string())
            {
                return Err(crate::tr!(
                    "err.official_credential.non_json_requires_raw_text",
                    source.file_id
                ));
            }
        }
    }
    Ok(())
}

fn value_at<'a>(value: &'a Value, path: &[String]) -> Option<&'a Value> {
    path.iter().try_fold(value, |current, key| current.get(key))
}

fn set_value_at(
    value: &mut Value,
    path: &[String],
    next: Value,
    context: &str,
) -> Result<(), String> {
    if path.is_empty() {
        *value = next;
        return Ok(());
    }
    let mut current = value;
    for key in &path[..path.len() - 1] {
        if current.get(key).is_none() {
            current
                .as_object_mut()
                .ok_or_else(|| crate::tr!("err.official_credential.path_conflict", context))?
                .insert(key.clone(), Value::Object(Map::new()));
        }
        current = current
            .get_mut(key)
            .ok_or_else(|| crate::tr!("err.official_credential.path_missing", context))?;
        if !current.is_object() {
            return Err(crate::tr!("err.official_credential.path_conflict", context));
        }
    }
    current
        .as_object_mut()
        .ok_or_else(|| crate::tr!("err.official_credential.target_not_object", context))?
        .insert(path[path.len() - 1].clone(), next);
    Ok(())
}

fn source_value(
    payload: &OfficialCredentialPayload,
    operation: &OfficialLoginOperation,
) -> Result<Value, String> {
    let source = match operation.op {
        OfficialLoginOperationKind::ReplaceFile => operation.content_from.as_ref(),
        OfficialLoginOperationKind::SetField => operation.value_from.as_ref(),
    }
    .ok_or_else(|| crate::tr!("err.official_credential.missing_source", operation.id))?;
    let file = payload.files.get(&source.file_id).ok_or_else(|| {
        crate::tr!(
            "err.official_credential.logical_file_not_found",
            source.file_id
        )
    })?;
    if file.format != operation.source_format() {
        return Err(crate::tr!(
            "err.official_credential.logical_file_format_mismatch",
            source.file_id
        ));
    }
    if file.format != ConfigFormat::Json && !source.path.is_empty() {
        return Err(crate::tr!(
            "err.official_credential.non_json_no_source_path",
            source.file_id
        ));
    }
    value_at(&file.content, &source.path)
        .cloned()
        .ok_or_else(|| {
            crate::tr!(
                "err.official_credential.source_path_not_found",
                source.file_id
            )
        })
}

fn prepared_replacement(
    payload: &OfficialCredentialPayload,
    operation: &OfficialLoginOperation,
) -> Result<PreparedFile, String> {
    let value = source_value(payload, operation)?;
    match operation_format(operation) {
        ConfigFormat::Json => Ok(PreparedFile::Fields(ConfigFormat::Json, value)),
        ConfigFormat::Jsonc
        | ConfigFormat::Toml
        | ConfigFormat::Yaml
        | ConfigFormat::CordisYaml
        | ConfigFormat::Env => value
            .as_str()
            .map(|content| PreparedFile::Text(content.to_string()))
            .ok_or_else(|| {
                crate::tr!("err.official_credential.content_must_be_text", operation.id)
            }),
    }
}

fn operation_value(
    payload: &OfficialCredentialPayload,
    operation: &OfficialLoginOperation,
) -> Result<Value, String> {
    match (&operation.value, &operation.value_from) {
        (Some(value), None) => Ok(value.clone()),
        (None, Some(_)) => source_value(payload, operation),
        _ => Err(crate::tr!(
            "err.official_credential.value_exclusive",
            operation.id
        )),
    }
}

fn remove_matching_value(
    current: &mut Value,
    path: &[String],
    expected: &Value,
    context: &str,
) -> Result<bool, String> {
    let Some((key, remaining)) = path.split_first() else {
        return Err(crate::tr!("err.official_credential.path_required", context));
    };
    let Some(object) = current.as_object_mut() else {
        return Ok(false);
    };
    if remaining.is_empty() {
        if object.get(key) == Some(expected) {
            object.remove(key);
            return Ok(true);
        }
        return Ok(false);
    }
    let Some(child) = object.get_mut(key) else {
        return Ok(false);
    };
    let removed = remove_matching_value(child, remaining, expected, context)?;
    if removed && child.as_object().is_some_and(Map::is_empty) {
        object.remove(key);
    }
    Ok(removed)
}

fn parse_target_fields(path: &Path, content: &str, format: ConfigFormat) -> Result<Value, String> {
    match format {
        ConfigFormat::Json => serde_json::from_str(content).map_err(|error| {
            crate::tr!(
                "err.official_credential.target_file_invalid_json",
                path.display(),
                error
            )
        }),
        ConfigFormat::Yaml if content.trim().is_empty() => Ok(Value::Object(Map::new())),
        ConfigFormat::Yaml => serde_yaml::from_str(content).map_err(|_| {
            crate::tr!(
                "err.official_credential.target_file_invalid_yaml",
                path.display()
            )
        }),
        _ => Err(crate::tr!(
            "err.official_credential.set_field_format_unsupported",
            path.display()
        )),
    }
}

async fn read_target_fields(path: &Path, format: ConfigFormat) -> Result<Value, String> {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => parse_target_fields(path, &content, format),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Value::Object(Map::new())),
        Err(error) => Err(crate::tr!(
            "err.official_credential.read_target_file_failed",
            path.display(),
            error
        )),
    }
}

async fn write_prepared(path: &Path, content: PreparedFile, private: bool) -> Result<(), String> {
    let text = match content {
        PreparedFile::Fields(ConfigFormat::Json, value) if !private => {
            return config_patch::write_atomic_json(path, &value).await;
        }
        PreparedFile::Fields(ConfigFormat::Yaml, value) => {
            serde_yaml::to_string(&value).map_err(|error| error.to_string())?
        }
        PreparedFile::Fields(_, value) => {
            serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?
        }
        PreparedFile::Text(text) => text,
    };
    config_patch::write_atomic_text_with_privacy(path, &text, private).await
}

async fn resolved_feature(db: &SqlitePool, agent_id: &str) -> Result<OfficialLoginFeature, String> {
    let resolved = agent::get_agent(db, agent_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| crate::tr!("err.official_credential.unknown_agent", agent_id))?;
    let feature = resolved.features.official_login;
    if feature.enabled {
        Ok(feature)
    } else {
        Err(crate::tr!("err.official_credential.feature_disabled"))
    }
}

async fn resolved_operations(
    db: &SqlitePool,
    agent_id: &str,
) -> Result<Vec<OfficialLoginOperation>, String> {
    Ok(resolved_feature(db, agent_id).await?.operations)
}

fn builtin_feature(agent_id: &str) -> Result<OfficialLoginFeature, String> {
    let definition = agent::get_definition(agent_id)
        .ok_or_else(|| crate::tr!("err.official_credential.unknown_agent", agent_id))?;
    let feature = &definition.features.official_login;
    if feature.operations.is_empty() {
        return Err(crate::tr!(
            "err.official_credential.no_operations",
            agent_id
        ));
    }
    Ok(feature.clone())
}

fn builtin_operations(agent_id: &str) -> Result<Vec<OfficialLoginOperation>, String> {
    Ok(builtin_feature(agent_id)?.operations)
}

fn managed_config_operation(
    managed: &OfficialManagedConfig,
    id: &str,
    path: &[&str],
    value: Value,
) -> ConfigOperation {
    let mut full_path = vec!["providers".to_string(), managed.provider.clone()];
    full_path.extend(path.iter().map(|part| part.to_string()));
    ConfigOperation {
        id: id.to_string(),
        op: ConfigOperationKind::Set,
        file: managed.file.clone(),
        format: managed.format,
        private: false,
        preserve_on_remove: false,
        path: full_path,
        value: Some(value),
    }
}

/// Restore the managed provider and OAuth reference after Kimi logout removed
/// them. Kimi can then refresh its own managed model catalog on the next start.
async fn ensure_managed_official_config(
    config_dir: &Path,
    managed: &OfficialManagedConfig,
) -> Result<(), String> {
    let path = resolve_cli_config_file_from_dir(config_dir, &managed.file);
    let content = match tokio::fs::read_to_string(&path).await {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.to_string()),
    };
    let existing_base_url = if content.trim().is_empty() {
        None
    } else {
        let value: ::toml::Value = ::toml::from_str(&content).map_err(|error| error.to_string())?;
        value
            .get("providers")
            .and_then(|providers| providers.get(&managed.provider))
            .and_then(|provider| provider.get("base_url"))
            .and_then(::toml::Value::as_str)
            .filter(|base_url| !base_url.trim().is_empty())
            .map(str::to_string)
    };

    let mut operations = vec![
        managed_config_operation(
            managed,
            "managed-provider-type",
            &["type"],
            Value::String(managed.provider_type.clone()),
        ),
        managed_config_operation(
            managed,
            "managed-provider-api-key",
            &["api_key"],
            Value::String(String::new()),
        ),
        managed_config_operation(
            managed,
            "managed-provider-oauth-storage",
            &["oauth", "storage"],
            Value::String(managed.oauth_storage.clone()),
        ),
        managed_config_operation(
            managed,
            "managed-provider-oauth-key",
            &["oauth", "key"],
            Value::String(managed.oauth_key.clone()),
        ),
    ];
    if existing_base_url.is_none() {
        let base_url = managed
            .base_url_env
            .as_deref()
            .and_then(|name| std::env::var(name).ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .or_else(|| managed.base_url.clone());
        if let Some(base_url) = base_url {
            operations.push(managed_config_operation(
                managed,
                "managed-provider-base-url",
                &["base_url"],
                Value::String(base_url),
            ));
        }
    }
    if let Some(oauth_host) = managed
        .oauth_host_env
        .as_deref()
        .and_then(|name| std::env::var(name).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        operations.push(managed_config_operation(
            managed,
            "managed-provider-oauth-host",
            &["oauth", "oauth_host"],
            Value::String(oauth_host),
        ));
    }

    let context = config_patch::PatchContext {
        target_endpoint: String::new(),
        target_token: String::new(),
        agent_id: managed.provider.clone(),
    };
    config_patch::patch_file(&path, managed.format, &operations, &context)
        .await
        .map(|_| ())
}

/// Remove the managed provider, its generated model aliases and the managed
/// services when leaving official mode.
async fn remove_managed_official_config(
    config_dir: &Path,
    managed: &OfficialManagedConfig,
) -> Result<(), String> {
    let path = resolve_cli_config_file_from_dir(config_dir, &managed.file);
    let content = match tokio::fs::read_to_string(&path).await {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.to_string()),
    };
    let value: ::toml::Value = ::toml::from_str(&content).map_err(|error| error.to_string())?;
    if value
        .get("providers")
        .and_then(|providers| providers.get(&managed.provider))
        .is_none()
    {
        return Ok(());
    }

    let managed_models: Vec<String> = value
        .get("models")
        .and_then(::toml::Value::as_table)
        .map(|models| {
            models
                .iter()
                .filter(|(_, model)| {
                    model.get("provider").and_then(::toml::Value::as_str)
                        == Some(managed.provider.as_str())
                })
                .map(|(alias, _)| alias.clone())
                .collect()
        })
        .unwrap_or_default();
    let default_model = value
        .get("default_model")
        .and_then(::toml::Value::as_str)
        .map(str::to_string);

    let mut operations = vec![ConfigOperation {
        id: "managed-provider-remove".to_string(),
        op: ConfigOperationKind::Remove,
        file: managed.file.clone(),
        format: managed.format,
        private: false,
        preserve_on_remove: false,
        path: vec!["providers".to_string(), managed.provider.clone()],
        value: None,
    }];
    for alias in &managed_models {
        operations.push(ConfigOperation {
            id: format!("managed-model-remove-{alias}"),
            op: ConfigOperationKind::Remove,
            file: managed.file.clone(),
            format: managed.format,
            private: false,
            preserve_on_remove: false,
            path: vec!["models".to_string(), alias.clone()],
            value: None,
        });
    }
    if default_model
        .as_deref()
        .is_some_and(|alias| managed_models.iter().any(|model| model == alias))
    {
        operations.push(ConfigOperation {
            id: "managed-default-model-remove".to_string(),
            op: ConfigOperationKind::Remove,
            file: managed.file.clone(),
            format: managed.format,
            private: false,
            preserve_on_remove: false,
            path: vec!["default_model".to_string()],
            value: None,
        });
    }
    for service in &managed.services {
        let uses_managed_oauth = value
            .get("services")
            .and_then(|services| services.get(service))
            .and_then(|service| service.get("oauth"))
            .and_then(|oauth| oauth.get("key"))
            .and_then(::toml::Value::as_str)
            == Some(managed.oauth_key.as_str());
        if uses_managed_oauth {
            operations.push(ConfigOperation {
                id: format!("managed-service-remove-{service}"),
                op: ConfigOperationKind::Remove,
                file: managed.file.clone(),
                format: managed.format,
                private: false,
                preserve_on_remove: false,
                path: vec!["services".to_string(), service.clone()],
                value: None,
            });
        }
    }

    let context = config_patch::PatchContext {
        target_endpoint: String::new(),
        target_token: String::new(),
        agent_id: managed.provider.clone(),
    };
    config_patch::patch_file(&path, managed.format, &operations, &context)
        .await
        .map(|_| ())
}

pub async fn apply_payload(
    db: &SqlitePool,
    agent_id: &str,
    json: &str,
) -> Result<Vec<PathBuf>, String> {
    let payload = parse_payload(json)?;
    let feature = resolved_feature(db, agent_id).await?;
    let operations = &feature.operations;
    let config_dir = get_cli_config_dir_path(db, agent_id).await;
    let mut prepared: HashMap<PathBuf, PreparedFile> = HashMap::new();
    let mut private_paths = HashSet::new();

    for operation in operations {
        let path = resolve_cli_config_file_from_dir(&config_dir, &operation.file);
        if operation.private {
            private_paths.insert(path.clone());
        }
        match operation.op {
            OfficialLoginOperationKind::ReplaceFile => {
                prepared.insert(path, prepared_replacement(&payload, operation)?);
            }
            OfficialLoginOperationKind::SetField => {
                if !matches!(
                    operation.format,
                    Some(ConfigFormat::Json | ConfigFormat::Yaml)
                ) {
                    return Err(crate::tr!(
                        "err.official_credential.set_field_format_unsupported",
                        operation.id
                    ));
                }
                let format = operation_format(operation);
                let next = operation_value(&payload, operation)?;
                if !prepared.contains_key(&path) {
                    prepared.insert(
                        path.clone(),
                        PreparedFile::Fields(format, read_target_fields(&path, format).await?),
                    );
                }
                let PreparedFile::Fields(_, target) =
                    prepared.get_mut(&path).expect("prepared target")
                else {
                    return Err(crate::tr!(
                        "err.official_credential.replace_conflict",
                        operation.id
                    ));
                };
                set_value_at(target, &operation.path, next, &operation.id)?;
            }
        }
    }

    let mut written = Vec::with_capacity(prepared.len());
    for (path, content) in prepared {
        write_prepared(&path, content, private_paths.contains(&path)).await?;
        written.push(path);
    }
    if let Some(managed) = feature.managed_config.as_ref() {
        ensure_managed_official_config(&config_dir, managed).await?;
    }
    Ok(written)
}

pub async fn remove_payload(
    db: &SqlitePool,
    agent_id: &str,
    json: &str,
) -> Result<Vec<PathBuf>, String> {
    let payload = parse_payload(json)?;
    let feature = builtin_feature(agent_id)?;
    let operations = &feature.operations;
    let config_dir = get_cli_config_dir_path(db, agent_id).await;
    let mut prepared: HashMap<PathBuf, (ConfigFormat, Value)> = HashMap::new();
    let mut remove_files = HashSet::new();
    let mut private_paths = HashSet::new();

    for operation in operations {
        if operation.preserve_on_remove {
            continue;
        }
        let path = resolve_cli_config_file_from_dir(&config_dir, &operation.file);
        if operation.private {
            private_paths.insert(path.clone());
        }
        match operation.op {
            OfficialLoginOperationKind::ReplaceFile => {
                let actual = match tokio::fs::read_to_string(&path).await {
                    Ok(content) => content,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(error) => return Err(error.to_string()),
                };
                let matches = match prepared_replacement(&payload, operation)? {
                    PreparedFile::Fields(_, expected) => serde_json::from_str::<Value>(&actual)
                        .map(|value| value == expected)
                        .unwrap_or(false),
                    PreparedFile::Text(expected) => actual == expected,
                };
                if matches {
                    remove_files.insert(path);
                }
            }
            OfficialLoginOperationKind::SetField => {
                if !matches!(
                    operation.format,
                    Some(ConfigFormat::Json | ConfigFormat::Yaml)
                ) {
                    return Err(crate::tr!(
                        "err.official_credential.set_field_format_unsupported",
                        operation.id
                    ));
                }
                let format = operation_format(operation);
                let expected = operation_value(&payload, operation)?;
                if let Some((_, target)) = prepared.get_mut(&path) {
                    remove_matching_value(target, &operation.path, &expected, &operation.id)?;
                } else {
                    let mut target = read_target_fields(&path, format).await?;
                    if remove_matching_value(
                        &mut target,
                        &operation.path,
                        &expected,
                        &operation.id,
                    )? {
                        prepared.insert(path, (format, target));
                    }
                }
            }
        }
    }

    let remove_managed_config = feature.managed_config.is_some() && !remove_files.is_empty();
    let mut touched = Vec::new();
    for (path, (format, value)) in prepared {
        if !remove_files.contains(&path) {
            write_prepared(
                &path,
                PreparedFile::Fields(format, value),
                private_paths.contains(&path),
            )
            .await?;
            touched.push(path);
        }
    }
    for path in remove_files {
        match tokio::fs::remove_file(&path).await {
            Ok(()) => touched.push(path),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(crate::tr!(
                    "err.official_credential.remove_file_failed",
                    path.display(),
                    error
                ));
            }
        }
    }
    if remove_managed_config {
        if let Some(managed) = feature.managed_config.as_ref() {
            remove_managed_official_config(&config_dir, managed).await?;
        }
    }
    Ok(touched)
}

pub async fn payload_matches(db: &SqlitePool, agent_id: &str, json: &str) -> Result<bool, String> {
    let payload = parse_payload(json)?;
    // Detection must keep working after a feature is disabled so
    // the dashboard can still remove credentials previously written by CCG.
    let operations = builtin_operations(agent_id)?;
    let config_dir = get_cli_config_dir_path(db, agent_id).await;
    if operations.is_empty() {
        return Ok(false);
    }
    for operation in &operations {
        let path = resolve_cli_config_file_from_dir(&config_dir, &operation.file);
        match operation.op {
            OfficialLoginOperationKind::ReplaceFile => {
                let actual = match tokio::fs::read_to_string(&path).await {
                    Ok(content) => content,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
                    Err(error) => return Err(error.to_string()),
                };
                let matches = match prepared_replacement(&payload, operation)? {
                    PreparedFile::Fields(_, expected) => serde_json::from_str::<Value>(&actual)
                        .map(|value| value == expected)
                        .unwrap_or(false),
                    PreparedFile::Text(expected) => actual == expected,
                };
                if !matches {
                    return Ok(false);
                }
            }
            OfficialLoginOperationKind::SetField => {
                let actual = match tokio::fs::read_to_string(&path).await {
                    Ok(content) => {
                        parse_target_fields(&path, &content, operation_format(operation))?
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
                    Err(error) => return Err(error.to_string()),
                };
                let expected = operation_value(&payload, operation)?;
                if value_at(&actual, &operation.path) != Some(&expected) {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}

pub async fn read_current_payload(db: &SqlitePool, agent_id: &str) -> Result<String, String> {
    let operations = resolved_operations(db, agent_id).await?;
    let config_dir = get_cli_config_dir_path(db, agent_id).await;
    let mut files: HashMap<String, OfficialCredentialFile> = HashMap::new();
    for operation in &operations {
        let path = resolve_cli_config_file_from_dir(&config_dir, &operation.file);
        match operation.op {
            OfficialLoginOperationKind::ReplaceFile => {
                let source = operation
                    .content_from
                    .as_ref()
                    .ok_or_else(|| {
                        crate::tr!("err.official_credential.missing_content_from", operation.id)
                    })?;
                let format = operation_format(operation);
                let raw = match tokio::fs::read_to_string(&path).await {
                    Ok(content) => content,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
                    Err(error) => {
                        return Err(crate::tr!(
                            "err.official_credential.read_target_file_failed",
                            path.display(),
                            error
                        ))
                    }
                };
                let content = if format == ConfigFormat::Json {
                    if raw.trim().is_empty() {
                        Value::Object(Map::new())
                    } else {
                        serde_json::from_str(&raw).map_err(|error| {
                            crate::tr!(
                                "err.official_credential.target_file_invalid_json",
                                path.display(),
                                error
                            )
                        })?
                    }
                } else {
                    Value::String(raw)
                };
                files.insert(
                    source.file_id.clone(),
                    OfficialCredentialFile { format, content },
                );
            }
            OfficialLoginOperationKind::SetField => {
                let Some(source) = &operation.value_from else {
                    continue;
                };
                let target = read_target_fields(&path, operation_format(operation)).await?;
                let Some(current) = value_at(&target, &operation.path).cloned() else {
                    continue;
                };
                let file =
                    files
                        .entry(source.file_id.clone())
                        .or_insert_with(|| OfficialCredentialFile {
                            format: ConfigFormat::Json,
                            content: Value::Object(Map::new()),
                        });
                set_value_at(&mut file.content, &source.path, current, &operation.id)?;
            }
        }
    }
    serde_json::to_string(&OfficialCredentialPayload {
        schema_version: 1,
        files,
    })
    .map_err(|error| error.to_string())
}

fn find_display_value(value: &Value) -> Option<String> {
    const DISPLAY_KEYS: &[&str] = &["active_email", "email", "active", "account"];
    match value {
        Value::Object(object) => {
            for key in DISPLAY_KEYS {
                if let Some(value) = object.get(*key).and_then(Value::as_str) {
                    return Some(value.to_string());
                }
            }
            object.values().find_map(find_display_value)
        }
        Value::Array(values) => values.iter().find_map(find_display_value),
        _ => None,
    }
}

pub fn display_info(json: &str) -> String {
    match parse_payload(json) {
        Ok(payload) => payload
            .files
            .values()
            .find_map(|file| find_display_value(&file.content))
            .unwrap_or_else(|| {
                crate::tr!("err.official_credential.saved_files", payload.files.len())
            }),
        Err(_) => crate::tr!("err.official_credential.invalid_format"),
    }
}
