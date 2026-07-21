use ::auto_launch::{AutoLaunch, AutoLaunchBuilder};

#[cfg(target_os = "macos")]
fn get_macos_app_bundle_path(exe_path: &std::path::Path) -> Option<std::path::PathBuf> {
    let path_str = exe_path.to_string_lossy();
    if let Some(app_pos) = path_str.find(".app/Contents/MacOS/") {
        let app_bundle_end = app_pos + 4;
        Some(std::path::PathBuf::from(&path_str[..app_bundle_end]))
    } else {
        None
    }
}

fn get_auto_launch() -> Result<AutoLaunch, String> {
    let exe_path = std::env::current_exe().map_err(|e| format!("无法获取应用路径: {e}"))?;

    #[cfg(target_os = "macos")]
    let app_path = get_macos_app_bundle_path(&exe_path).unwrap_or(exe_path);

    #[cfg(not(target_os = "macos"))]
    let app_path = exe_path;

    AutoLaunchBuilder::new()
        .set_app_name("CCG Gateway")
        .set_app_path(&app_path.to_string_lossy())
        .build()
        .map_err(|e| format!("创建 AutoLaunch 失败: {e}"))
}

pub fn enable_auto_launch() -> Result<(), String> {
    get_auto_launch()?
        .enable()
        .map_err(|e| format!("启用开机自启失败: {e}"))
}

pub fn disable_auto_launch() -> Result<(), String> {
    get_auto_launch()?
        .disable()
        .map_err(|e| format!("禁用开机自启失败: {e}"))
}
