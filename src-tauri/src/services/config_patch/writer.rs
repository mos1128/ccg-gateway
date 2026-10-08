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
        .ok_or_else(|| crate::tr!("err.patch.parent_missing", path.display()))?;
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
        .map_err(|error| crate::tr!("err.patch.temp_create_failed", error))?;
    file.write_all(bytes)
        .map_err(|error| crate::tr!("err.patch.temp_write_failed", error))?;
    file.commit()
        .map_err(|error| crate::tr!("err.patch.replace_failed", path.display(), error))
}

fn create_parent(parent: &Path, private: bool) -> Result<(), String> {
    #[cfg(unix)]
    if private {
        use std::os::unix::fs::DirBuilderExt as _;

        return std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)
            .map_err(|error| crate::tr!("err.patch.dir_create_failed", parent.display(), error));
    }
    #[cfg(not(unix))]
    let _ = private;
    std::fs::create_dir_all(parent)
        .map_err(|error| crate::tr!("err.patch.dir_create_failed", parent.display(), error))
}
