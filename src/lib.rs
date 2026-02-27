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
    let dest = resolve_link_destination(from, to)?;
    remove_existing(&dest)?;
    std::os::unix::fs::symlink(from, &dest)
}

// Helper to backup an existing target and create a symlink
//
// @param from: &Path - the source path for the symlink
// @param to: &Path - the destination path to backup and replace
// @return Result<()> - if backup and replacement were successful
pub fn backup_and_replace(from: &Path, to: &Path) -> std::io::Result<()> {
    let dest = resolve_link_destination(from, to)?;
    let backup_dir = match std::fs::metadata(from) {
        Ok(meta) if meta.is_dir() => from.to_path_buf(),
        _ => from
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| from.to_path_buf()),
    };

    std::fs::create_dir_all(&backup_dir)?;
    let base_name = dest
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "backup".to_string());
    let backup_path = unique_backup_path(&backup_dir, &base_name);

    std::fs::rename(&dest, &backup_path)?;
    std::os::unix::fs::symlink(from, &dest)
}

// Helper to resolve the actual destination path for a symlink
//
// Uses the source path to decide file vs dir semantics, then adjusts the
// destination accordingly.
//
// Rules:
// - If <from> is a dir and <to> exists as a file -> error
// - If <from> is a dir and <to> is dir or missing -> link at <to>
// - If <from> is a file and <to> exists as dir -> link at <to>/<from basename>
// - If <from> is a file and <to> is file or missing -> link at <to>
pub fn resolve_link_destination(from: &Path, to: &Path) -> std::io::Result<PathBuf> {
    let from_meta = std::fs::metadata(from)?;
    let to_meta = std::fs::symlink_metadata(to).ok();

    if from_meta.is_dir() {
        if let Some(meta) = to_meta {
            if meta.is_file() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("destination is file for directory source: {}", to.display()),
                ));
            }
        }

        return Ok(to.to_path_buf());
    }

    if let Some(meta) = to_meta {
        if meta.is_dir() {
            let name = from.file_name().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("source has no basename: {}", from.display()),
                )
            })?;
            return Ok(to.join(name));
        }
    }

    Ok(to.to_path_buf())
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
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    if meta.file_type().is_symlink() || meta.is_file() {
        std::fs::remove_file(path)
    } else {
        std::fs::remove_dir_all(path)
    }
}
