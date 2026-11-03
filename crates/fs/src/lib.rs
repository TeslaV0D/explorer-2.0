//! Directory enumeration. Portable `std::fs` implementation first;
//! a Windows fast path (FindFirstFileExW / NtQueryDirectoryFile) comes in M1.

use explorer_core::{Entry, EntryKind};
use std::io;
use std::path::Path;
use std::time::UNIX_EPOCH;

/// Reads a directory. Per-entry errors are skipped, not fatal: one broken
/// entry must never hide the rest of the folder.
pub fn read_dir(path: &Path) -> io::Result<Vec<Entry>> {
    let mut out = Vec::new();
    for item in std::fs::read_dir(path)? {
        let Ok(item) = item else { continue };
        let Ok(meta) = item.metadata() else { continue };
        let ft = meta.file_type();
        let kind = if ft.is_symlink() {
            EntryKind::Symlink
        } else if ft.is_dir() {
            EntryKind::Dir
        } else {
            EntryKind::File
        };
        let name = item.file_name().to_string_lossy().into_owned();
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64);
        out.push(Entry {
            hidden: is_hidden(&name, &meta),
            name,
            kind,
            size: if kind == EntryKind::Dir {
                0
            } else {
                meta.len()
            },
            modified,
        });
    }
    Ok(out)
}

#[cfg(windows)]
fn is_hidden(_name: &str, meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const HIDDEN: u32 = 0x2;
    const SYSTEM: u32 = 0x4;
    meta.file_attributes() & (HIDDEN | SYSTEM) != 0
}

#[cfg(not(windows))]
fn is_hidden(name: &str, _meta: &std::fs::Metadata) -> bool {
    name.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_files_and_dirs() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a.txt"), b"hello").unwrap();
        std::fs::create_dir(d.path().join("sub")).unwrap();
        let mut v = read_dir(d.path()).unwrap();
        explorer_core::sort_entries(&mut v, Default::default());
        assert_eq!(v.len(), 2);
        assert!(v[0].is_dir() && v[0].name == "sub");
        assert_eq!(v[1].size, 5);
    }

    #[test]
    fn missing_dir_is_error_not_panic() {
        assert!(read_dir(Path::new("/definitely/not/here")).is_err());
    }
}
