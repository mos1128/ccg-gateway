use crate::services::updater::UpdateApplyResult;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const APPLY_UPDATE_FLAG: &str = "--ccg-apply-update";

pub fn run_from_args() -> Option<i32> {
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let index = args
        .iter()
        .position(|arg| arg.to_string_lossy() == APPLY_UPDATE_FLAG)?;
    let rest = &args[index + 1..];
    if rest.len() < 5 {
        return Some(2);
    }
    let parent_pid = match rest[0].to_string_lossy().parse::<u32>() {
        Ok(pid) => pid,
        Err(_) => return Some(2),
    };
    let source = PathBuf::from(&rest[1]);
    let target = PathBuf::from(&rest[2]);
    let data_dir = PathBuf::from(&rest[3]);
    let version = rest[4].to_string_lossy().to_string();
    Some(apply_update(
        parent_pid, &source, &target, &data_dir, &version,
    ))
}

fn apply_update(
    _parent_pid: u32,
    source: &Path,
    target: &Path,
    data_dir: &Path,
    version: &str,
) -> i32 {
    if !source.exists() {
        write_result(data_dir, version, false, Some("downloaded file is missing"));
        start_target(target);
        return 1;
    }

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut last_error: Option<String>;
    loop {
        match try_swap(source, target) {
            Ok(()) => {
                write_result(data_dir, version, true, None);
                start_target(target);
                return 0;
            }
            Err(error) => {
                last_error = Some(error.to_string());
                if Instant::now() >= deadline {
                    break;
                }
                std::thread::sleep(Duration::from_millis(300));
            }
        }
    }

    write_result(
        data_dir,
        version,
        false,
        Some(last_error.as_deref().unwrap_or("update failed")),
    );
    start_target(target);
    1
}

fn try_swap(source: &Path, target: &Path) -> std::io::Result<()> {
    let new_path = sibling(target, "new");
    let old_path = sibling(target, "old");

    if old_path.exists() {
        if target.exists() {
            let _ = fs::remove_file(&old_path);
        } else {
            fs::rename(&old_path, target)?;
        }
    }

    fs::copy(source, &new_path)?;
    fs::rename(target, &old_path)?;
    if let Err(error) = fs::rename(&new_path, target) {
        let _ = fs::rename(&old_path, target);
        return Err(error);
    }
    let _ = fs::remove_file(&new_path);
    Ok(())
}

fn sibling(target: &Path, suffix: &str) -> PathBuf {
    let mut name = target
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_else(|| std::ffi::OsString::from("ccg-gateway.exe"));
    name.push(".");
    name.push(suffix);
    target.with_file_name(name)
}

fn write_result(data_dir: &Path, version: &str, ok: bool, message: Option<&str>) {
    let result = UpdateApplyResult {
        version: version.to_string(),
        ok,
        message: message.map(|message| message.to_string()),
    };
    let Ok(content) = serde_json::to_vec(&result) else {
        return;
    };
    let _ = fs::create_dir_all(data_dir);
    let _ = fs::write(data_dir.join("update-result.json"), content);
}

fn start_target(target: &Path) {
    let mut command = Command::new(target);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let _ = command.spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swaps_target_with_source() {
        let dir = std::env::temp_dir().join(format!(
            "ccg-gateway-update-helper-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let target = dir.join("app.exe");
        let source = dir.join("download.exe");
        fs::write(&target, b"old").unwrap();
        fs::write(&source, b"new").unwrap();

        try_swap(&source, &target).unwrap();

        assert_eq!(fs::read(&target).unwrap(), b"new");
        assert_eq!(fs::read(dir.join("app.exe.old")).unwrap(), b"old");
        assert!(!dir.join("app.exe.new").exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
