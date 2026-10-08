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
    let exe_path = std::env::current_exe()
        .map_err(|e| crate::tr!("err.auto_launch.exe_path_failed", e))?;

    #[cfg(target_os = "macos")]
    let app_path = get_macos_app_bundle_path(&exe_path).unwrap_or(exe_path);

    #[cfg(not(target_os = "macos"))]
    let app_path = exe_path;

    AutoLaunchBuilder::new()
        .set_app_name("CCG Gateway")
        .set_app_path(&app_path.to_string_lossy())
        .build()
        .map_err(|e| crate::tr!("err.auto_launch.create_failed", e))
}

pub fn enable_auto_launch() -> Result<(), String> {
    get_auto_launch()?
        .enable()
        .map_err(|e| crate::tr!("err.auto_launch.enable_failed", e))
}

pub fn disable_auto_launch() -> Result<(), String> {
    get_auto_launch()?
        .disable()
        .map_err(|e| crate::tr!("err.auto_launch.disable_failed", e))
}
