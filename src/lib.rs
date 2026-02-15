use std::path::{Path, PathBuf};
pub mod config_parser;

// Helper to make an absolute path out of a Path
//
// @param path: &Path - the path to canonicalize
// @return PathBuf - the canonicalized path or the initial Path converted to PathBuf
pub fn canonicalize_or_fallback(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

// Helper to resolve a symlink target into an absolute path
//
// `read_link` can return a relative target, which is interpreted relative to the
// symlink's parent directory. This helper normalizes that into a concrete path
// so it can be compared reliably with the expected target.
//
// @param link_path: &Path - the path to the symlink
// @param target: &Path - the raw target path read from the symlink
// @return PathBuf - the resolved target path
pub fn resolve_symlink_target(link_path: &Path, target: &Path) -> PathBuf {
    if target.is_relative() {
        link_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(target)
    } else {
        target.to_path_buf()
    }
}

// Helper to remove existing target and create a symlink
//
// @param from: &Path - the source path for the symlink
// @param to: &Path - the destination path for the symlink
// @return Result<()> - if replacement was successful
pub fn replace_link(from: &Path, to: &Path) -> std::io::Result<()> {
    remove_existing(to)?;
    std::os::unix::fs::symlink(from, to)
}

// Helper to backup an existing target and create a symlink
//
// @param from: &Path - the source path for the symlink
// @param to: &Path - the destination path to backup and replace
// @return Result<()> - if backup and replacement were successful
pub fn backup_and_replace(from: &Path, to: &Path) -> std::io::Result<()> {
    let backup_dir = match std::fs::metadata(from) {
        Ok(meta) if meta.is_dir() => from.to_path_buf(),
        _ => from
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| from.to_path_buf()),
    };

    std::fs::create_dir_all(&backup_dir)?;
    let base_name = to
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "backup".to_string());
    let backup_path = unique_backup_path(&backup_dir, &base_name);

    std::fs::rename(to, &backup_path)?;
    std::os::unix::fs::symlink(from, to)
}

// Helper to create a unique backup path with a numeric suffix
//
// @param dir: &Path - the directory where backup should be created
// @param name: &str - the base name of the file being backed up
// @return PathBuf - the unique backup path
pub fn unique_backup_path(dir: &Path, name: &str) -> PathBuf {
    let base = format!("{}.bak.dbdm", name);
    let mut path = dir.join(&base);
    let mut counter = 1;
    while path.exists() {
        let candidate = format!("{}.{}", base, counter);
        path = dir.join(candidate);
        counter += 1;
    }
    path
}

// Helper to remove existing path whether file, directory, or symlink
//
// @param path: &Path - the path to remove
// @return Result<()> - if removal was successful
pub fn remove_existing(path: &Path) -> std::io::Result<()> {
    let meta = std::fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || meta.is_file() {
        std::fs::remove_file(path)
    } else {
        std::fs::remove_dir_all(path)
    }
}
