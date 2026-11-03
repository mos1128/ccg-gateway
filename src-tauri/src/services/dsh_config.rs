use crate::db::models::{AgentInfo, ConfigFormat};
use crate::services::{cli_config, config_patch};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use yaml_edit::{Mapping, YamlFile};

const PROFILE_PATCH_FILE: &str = "cordis.patch.yml";
const DEFAULT_PROFILE_NAMES: [&str; 2] = ["desktop", "web"];

pub fn resolve_cordis_targets(config_dir: &Path, file: &str) -> Result<Vec<PathBuf>, String> {
    let normalized = file.replace('\\', "/");
    if !normalized.starts_with("profiles/") || !normalized.contains("/*/") {
        return Ok(vec![cli_config::resolve_cli_config_file_from_dir(
            config_dir, file,
        )]);
    }

    let mut targets = cli_config::resolve_cli_config_file_targets(config_dir, file)?;
    targets.retain(|path| {
        let Some(profile_dir) = path.parent() else {
            return false;
        };
        let name = profile_dir.file_name().and_then(|name| name.to_str());
        name.is_some_and(|name| DEFAULT_PROFILE_NAMES.contains(&name))
            || profile_dir.join("package.json").is_file()
    });
    let profiles = config_dir.join("profiles");
    let suffix = normalized
        .split_once("/*/")
        .map(|(_, suffix)| suffix)
        .unwrap_or(PROFILE_PATCH_FILE);
    for name in DEFAULT_PROFILE_NAMES {
        let path = profiles.join(name).join(suffix);
        if !targets.contains(&path) {
            targets.push(path);
        }
    }
    targets.sort();
    Ok(targets)
}

fn clean_routes(plugin: &str, config: &Mapping) -> bool {
    let mut changed = false;
    for (id, path, api) in [
        ("llm-deepseek", &[][..], None),
        (
            "llm-pi-ai",
            &["providers", "openai"][..],
            Some("openai-responses"),
        ),
        (
            "llm-pi-ai",
            &["providers", "anthropic"][..],
            Some("anthropic-messages"),
        ),
    ] {
        if id != plugin {
            continue;
        }
        let Some(route) = path
            .iter()
            .try_fold(config.clone(), |node, key| node.get_mapping(*key))
        else {
            continue;
        };
        if !route
            .get("apiKeyEnv")
            .is_some_and(|value| value.yaml_eq(&"CCG_GATEWAY_API_KEY"))
        {
            continue;
        }
        route.remove("baseURL");
        route.remove("apiKeyEnv");
        if api.is_some_and(|api| route.get("api").is_some_and(|value| value.yaml_eq(&api))) {
            route.remove("api");
        }
        if route.is_empty() {
            if let Some((key, parent)) = path.split_last() {
                if let Some(parent) = parent
                    .iter()
                    .try_fold(config.clone(), |node, key| node.get_mapping(*key))
                {
                    parent.remove(*key);
                }
            }
        }
        changed = true;
    }
    if changed
        && plugin == "llm-pi-ai"
        && config
            .get_mapping("providers")
            .is_some_and(|providers| providers.is_empty())
    {
        config.remove("providers");
    }
    changed
}

fn validated_content(document: &YamlFile) -> Result<String, String> {
    let content = document.to_string();
    serde_yaml::from_str::<serde_yaml::Value>(&content)
        .map_err(|error| crate::tr!("err.dsh_mcp.patch_parse_failed", error))?;
    Ok(content)
}

fn clean_settings(content: &str) -> Result<Option<String>, String> {
    let document = YamlFile::from_str(content)
        .map_err(|error| crate::tr!("err.dsh_mcp.patch_parse_failed", error))?;
    let Some(config) = document.document().and_then(|doc| doc.as_mapping()) else {
        return Ok(None);
    };
    let mut changed = false;
    for plugin in ["llm-deepseek", "llm-pi-ai"] {
        if let Some(config) = config.get_mapping(plugin) {
            changed |= clean_routes(plugin, &config);
        }
    }
    if changed {
        validated_content(&document).map(Some)
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_initialized_profiles_and_falls_back_to_desktop_and_web() {
        let root = std::env::temp_dir().join(format!("ccg-dsh-targets-{}", uuid::Uuid::new_v4()));
        let profiles = root.join("profiles");
        for name in ["custom", "desktop", "node_modules", ".web.staging-test"] {
            std::fs::create_dir_all(profiles.join(name)).unwrap();
        }
        std::fs::write(profiles.join("custom/package.json"), "{}").unwrap();
        std::fs::write(profiles.join("desktop/package.json"), "{}").unwrap();

        let targets = resolve_cordis_targets(&root, "profiles/*/cordis.patch.yml").unwrap();
        let names: Vec<_> = targets
            .iter()
            .filter_map(|path| path.parent()?.file_name()?.to_str())
            .collect();
        assert_eq!(names, ["custom", "desktop", "web"]);

        std::fs::remove_dir_all(&root).unwrap();
        let targets = resolve_cordis_targets(&root, "profiles/*/cordis.patch.yml").unwrap();
        let names: Vec<_> = targets
            .iter()
            .filter_map(|path| path.parent()?.file_name()?.to_str())
            .collect();
        assert_eq!(names, ["desktop", "web"]);
        assert_eq!(
            resolve_cordis_targets(&root, "cordis.patch.yml").unwrap(),
            vec![root.join("cordis.patch.yml")]
        );
    }

    #[tokio::test]
    async fn root_cleanup_removes_managed_block_and_keeps_other_patches() {
        let root = std::env::temp_dir().join(format!("ccg-dsh-root-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("cordis.patch.yml");
        let content = "# User settings\n- id: ui-settings\n  config:\n    enabled: true\n# >>> CCG Gateway Cordis Config >>>\n- id: llm-pi-ai\n  config:\n    providers:\n      openai:\n        baseURL: http://127.0.0.1:7788\n        apiKeyEnv: CCG_GATEWAY_API_KEY\n# <<< CCG Gateway Cordis Config <<<\n# >>> CCG Gateway DeepSeek Harness MCP >>>\n- insert:\n  - id: ccg-mcp-example\n    name: '@deepseek-ai/dsh-mcp-client'\n    config:\n      serverName: example\n      transport: stdio\n      command: test-command\n# <<< CCG Gateway DeepSeek Harness MCP <<<\n";
        std::fs::write(&path, content).unwrap();

        clean_root_patch(&path).await.unwrap();
        let cleaned = std::fs::read_to_string(&path).unwrap();
        assert!(!cleaned.contains("CCG Gateway Cordis Config"));
        assert!(cleaned.contains("ui-settings"));
        assert!(cleaned.contains("CCG Gateway DeepSeek Harness MCP"));
        assert!(crate::services::dsh_mcp::contains(&cleaned, "example"));
        clean_root_patch(&path).await.unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), cleaned);

        std::fs::write(
            &path,
            "# >>> CCG Gateway Cordis Config >>>\n- id: llm-pi-ai\n  config:\n    providers: {}\n# <<< CCG Gateway Cordis Config <<<\n",
        )
        .unwrap();
        clean_root_patch(&path).await.unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap().trim(), "[]");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn cleanup_skips_the_managed_cordis_block() {
        let content = "# >>> CCG Gateway Cordis Config >>>\n- id: llm-pi-ai\n  config:\n    providers:\n      openai:\n        baseURL: http://127.0.0.1:7788\n        apiKeyEnv: CCG_GATEWAY_API_KEY\n# <<< CCG Gateway Cordis Config <<<\n";
        assert!(clean_profile(content).unwrap().is_none());
    }

    #[test]
    fn cleanup_keeps_unrelated_empty_catalog_routes() {
        let original = "# User settings\nllm-pi-ai:\n  providers:\n    openai:\n      baseURL: http://localhost:7788\n      apiKeyEnv: CCG_GATEWAY_API_KEY\n      api: openai-responses\n    anthropic: {}\nui-settings:\n  enabled: !!js 'true'\n";
        let cleaned = clean_settings(original).unwrap().unwrap();
        assert!(cleaned.starts_with("# User settings\n"));
        assert!(cleaned.contains("enabled: !!js 'true'"));
        let config: serde_yaml::Value = serde_yaml::from_str(&cleaned).unwrap();
        assert!(config["llm-pi-ai"]["providers"].get("openai").is_none());
        assert!(config["llm-pi-ai"]["providers"]["anthropic"]
            .as_mapping()
            .unwrap()
            .is_empty());
        assert!(clean_settings(&cleaned).unwrap().is_none());
    }

    #[test]
    fn migrated_routes_are_cleared_without_changing_user_settings_or_expressions() {
        let original = "# User profile\n- id: llm-pi-ai\n  config:\n    providers:\n      openai:\n        baseURL: http://127.0.0.1:7788\n        apiKeyEnv: CCG_GATEWAY_API_KEY\n        api: openai-responses\n        models: !!js '[{id: \"custom\"}]'\n      anthropic:\n        baseURL: https://user.example\n        apiKeyEnv: USER_KEY\n- id: ui-settings\n  config:\n    enabled: !!js 'true'\n";
        let cleaned = clean_profile(original).unwrap().unwrap();
        assert!(cleaned.starts_with("# User profile\n"));
        assert!(cleaned.contains("models: !!js '[{id: \"custom\"}]'"));
        assert!(cleaned.contains("enabled: !!js 'true'"));
        let rows: Vec<serde_yaml::Value> = serde_yaml::from_str(&cleaned).unwrap();
        assert_eq!(rows.len(), 2);
        let providers = &rows[0]["config"]["providers"];
        assert!(providers["openai"].get("baseURL").is_none());
        assert!(providers["openai"].get("apiKeyEnv").is_none());
        assert!(providers["openai"].get("api").is_none());
        assert_eq!(providers["anthropic"]["baseURL"], "https://user.example");
        assert_eq!(providers["anthropic"]["apiKeyEnv"], "USER_KEY");
        assert!(clean_profile(&cleaned).unwrap().is_none());
    }

    #[test]
    fn migrated_empty_provider_is_removed() {
        let content = "- id: llm-pi-ai\n  config:\n    providers:\n      openai:\n        baseURL: http://localhost:7788\n        apiKeyEnv: CCG_GATEWAY_API_KEY\n        api: openai-responses\n";
        let cleaned = clean_profile(content).unwrap().unwrap();
        let rows: Vec<serde_yaml::Value> = serde_yaml::from_str(&cleaned).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0]["config"].as_mapping().unwrap().is_empty());
        assert!(clean_profile(&cleaned).unwrap().is_none());
    }

    #[test]
    fn cleanup_handles_flow_sequences_and_document_end_markers() {
        for original in [
            "# Flow profile\r\n[{id: llm-deepseek, config: {apiKeyEnv: 'CCG_GATEWAY_API_KEY', baseURL: 'http://localhost:7788', models: !!js '[{id: \"custom\"}]'}}]\r\n",
            "# Block profile\n---\n- id: llm-deepseek\n  config:\n    apiKeyEnv: CCG_GATEWAY_API_KEY\n    baseURL: http://localhost:7788\n    models: !!js '[{id: \"custom\"}]'\n...\n",
        ] {
            let cleaned = clean_profile(original).unwrap().unwrap();
            let rows: Vec<serde_yaml::Value> = serde_yaml::from_str(&cleaned).unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0]["id"], "llm-deepseek");
            assert!(rows[0]["config"].get("apiKeyEnv").is_none());
            assert!(rows[0]["config"].get("baseURL").is_none());
            assert!(cleaned.starts_with(original.lines().next().unwrap()));
            assert!(cleaned.contains("models: !!js '[{id: \"custom\"}]'"));
            if original.ends_with("...\n") {
                assert!(cleaned.ends_with("...\n"));
            }
            if original.contains("\r\n") {
                assert!(!cleaned.replace("\r\n", "").contains('\n'));
            }
            assert!(clean_profile(&cleaned).unwrap().is_none());
        }
    }

    #[test]
    fn cleanup_preserves_mcp_management_markers() {
        let mcp_block = "# >>> CCG Gateway DeepSeek Harness MCP >>>\n- insert:\n  - id: ccg-mcp-example\n    name: '@deepseek-ai/dsh-mcp-client'\n    config:\n      serverName: example\n      transport: stdio\n      command: test-command\n# <<< CCG Gateway DeepSeek Harness MCP <<<\n";
        let original = format!(
            "- id: llm-deepseek\n  config:\n    apiKeyEnv: CCG_GATEWAY_API_KEY\n    baseURL: http://localhost:7788\n{mcp_block}"
        );
        let cleaned = clean_profile(&original).unwrap().unwrap();
        assert!(cleaned.contains(mcp_block));
        assert!(crate::services::dsh_mcp::contains(&cleaned, "example"));
        assert!(clean_profile(&cleaned).unwrap().is_none());
    }
}

fn clean_profile(content: &str) -> Result<Option<String>, String> {
    if config_patch::has_cordis_block(content) {
        return Ok(None);
    }
    let document = YamlFile::from_str(content)
        .map_err(|error| crate::tr!("err.dsh_mcp.patch_parse_failed", error))?;
    let Some(rows) = document.document().and_then(|doc| doc.as_sequence()) else {
        return Ok(None);
    };
    let mut changed = false;
    for row in rows.values() {
        if row.get("insert").is_some() {
            continue;
        }
        let Some(id) = row.get("id") else {
            continue;
        };
        let Some(config) = row.as_mapping().and_then(|row| row.get_mapping("config")) else {
            continue;
        };
        for plugin in ["llm-deepseek", "llm-pi-ai"] {
            if id.yaml_eq(&plugin) {
                changed |= clean_routes(plugin, &config);
            }
        }
    }
    if changed {
        validated_content(&document).map(Some)
    } else {
        Ok(None)
    }
}

async fn clean_root_patch(path: &Path) -> Result<(), String> {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => {
            let next = config_patch::reset_content(ConfigFormat::CordisYaml, &content)?;
            if next != content {
                config_patch::write_atomic_text(path, &next).await?;
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    Ok(())
}

pub async fn clean_legacy_config(db: &SqlitePool, agent: &AgentInfo) -> Result<(), String> {
    if !agent
        .features
        .provider_config
        .operations
        .iter()
        .any(|operation| operation.format == ConfigFormat::CordisYaml)
    {
        return Ok(());
    }
    let root = cli_config::get_cli_config_dir_path(db, &agent.id).await;
    clean_root_patch(&root.join(PROFILE_PATCH_FILE)).await?;
    let legacy = root.join("settings.yaml");
    if tokio::fs::try_exists(&legacy)
        .await
        .map_err(|error| error.to_string())?
    {
        let content = tokio::fs::read_to_string(&legacy)
            .await
            .map_err(|error| error.to_string())?;
        if let Some(next) = clean_settings(&content)? {
            config_patch::write_atomic_text(&legacy, &next).await?;
        }
    }
    let mut profiles = match tokio::fs::read_dir(root.join("profiles")).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.to_string()),
    };
    while let Some(profile) = profiles
        .next_entry()
        .await
        .map_err(|error| error.to_string())?
    {
        let path = profile.path().join("cordis.patch.yml");
        if !tokio::fs::try_exists(&path)
            .await
            .map_err(|error| error.to_string())?
        {
            continue;
        }
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|error| error.to_string())?;
        if let Some(next) = clean_profile(&content)? {
            config_patch::write_atomic_text(&path, &next).await?;
        }
    }
    Ok(())
}
