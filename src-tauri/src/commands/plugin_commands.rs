use super::*;

// ==================== 插件管理命令 ====================

/// 解析启用插件功能的 Agent，并校验其 adapter 为 dsh
fn plugins_agent_id() -> Result<String> {
    let agent_ids = crate::services::agent::agent_ids_for_feature("plugins");
    let agent_id = match agent_ids.as_slice() {
        [] => return Err("没有 Agent 启用插件功能".to_string()),
        [single] => single.to_string(),
        many => return Err(format!("插件功能同时启用了多个 Agent: {}", many.join(", "))),
    };

    let definition = crate::services::agent::get_definition(&agent_id)
        .ok_or_else(|| format!("未知 Agent: {}", agent_id))?;
    if definition.features.plugins.adapter.as_deref() != Some("dsh") {
        return Err("插件功能当前仅支持 dsh adapter".to_string());
    }

    Ok(agent_id)
}

#[tauri::command]
pub async fn get_installed_plugins(db: State<'_, SqlitePool>) -> Result<Vec<PluginItem>> {
    let agent_id = plugins_agent_id()?;
    let config_dir = get_cli_config_dir_path(db.inner(), &agent_id).await;
    crate::services::plugin::get_installed_plugins(&config_dir).await
}

#[tauri::command]
pub async fn plugin_action(
    action: String,
    profile: String,
    param: String,
) -> Result<crate::services::plugin::PluginActionResult> {
    crate::services::plugin::plugin_action(&action, &profile, &param).await
}
