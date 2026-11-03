//! Single-step operations that run synchronously: rename, new folder.

use crate::names::{unique_name, validate_name, NEW_FOLDER};
use std::path::{Path, PathBuf};

/// Renames `path` to `new_name` in the same folder. Case-only changes are allowed.
pub fn rename(path: &Path, new_name: &str) -> Result<PathBuf, String> {
    validate_name(new_name)?;
    let old_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if old_name == new_name {
        return Ok(path.to_path_buf());
    }
    let target = path.with_file_name(new_name);
    let case_only = old_name.to_lowercase() == new_name.to_lowercase();
    if !case_only && std::fs::symlink_metadata(&target).is_ok() {
        return Err(format!("Es gibt bereits ein Element namens „{new_name}“."));
    }
    std::fs::rename(path, &target).map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => "Zugriff verweigert".to_string(),
        std::io::ErrorKind::NotFound => "Nicht gefunden".to_string(),
        _ => e.to_string(),
    })?;
    Ok(target)
}

/// Creates "Neuer Ordner" (or "Neuer Ordner (2)", …) in `dir`.
pub fn new_folder(dir: &Path) -> Result<PathBuf, String> {
    let name = unique_name(NEW_FOLDER, true, |n| {
        std::fs::symlink_metadata(dir.join(n)).is_ok()
    });
    let p = dir.join(name);
    std::fs::create_dir(&p).map_err(|e| e.to_string())?;
    Ok(p)
}
