use atomic_write_file::AtomicWriteFile;
use std::io::Write;
use std::path::Path;

pub(super) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    write_atomic_with_privacy(path, bytes, false)
}

pub(super) fn write_atomic_with_privacy(
    path: &Path,
    bytes: &[u8],
    private: bool,
) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("配置路径 {} 没有父目录", path.display()))?;
    create_parent(parent, private)?;
    #[cfg(unix)]
    let mut options = AtomicWriteFile::options();
    #[cfg(not(unix))]
    let options = AtomicWriteFile::options();
    #[cfg(unix)]
    if private {
        use atomic_write_file::unix::OpenOptionsExt as _;
        use std::os::unix::fs::OpenOptionsExt as _;

        options.preserve_mode(false).mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("创建临时配置文件失败: {}", error))?;
    file.write_all(bytes)
        .map_err(|error| format!("写入临时配置文件失败: {}", error))?;
    file.commit()
        .map_err(|error| format!("替换配置文件 {} 失败: {}", path.display(), error))
}

fn create_parent(parent: &Path, private: bool) -> Result<(), String> {
    #[cfg(unix)]
    if private {
        use std::os::unix::fs::DirBuilderExt as _;

        return std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)
            .map_err(|error| format!("创建目录 {} 失败: {}", parent.display(), error));
    }
    #[cfg(not(unix))]
    let _ = private;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("创建目录 {} 失败: {}", parent.display(), error))
}
