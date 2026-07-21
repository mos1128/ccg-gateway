use atomic_write_file::AtomicWriteFile;
use std::io::Write;
use std::path::Path;

pub(super) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("配置路径 {} 没有父目录", path.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("创建目录 {} 失败: {}", parent.display(), error))?;
    let mut file = AtomicWriteFile::options()
        .open(path)
        .map_err(|error| format!("创建临时配置文件失败: {}", error))?;
    file.write_all(bytes)
        .map_err(|error| format!("写入临时配置文件失败: {}", error))?;
    file.commit()
        .map_err(|error| format!("替换配置文件 {} 失败: {}", path.display(), error))
}
