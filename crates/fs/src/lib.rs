//! File system layer: fast directory enumeration, background loading, watching.
//!
//! - Windows: `FindFirstFileExW` + `FindExInfoBasic` + `FIND_FIRST_EX_LARGE_FETCH`,
//!   attributes/size/mtime come straight from the find data (no per-file syscall),
//!   long paths via `\\?\` verbatim prefix.
//! - Other platforms: portable `std::fs` implementation (used for tests/dev).

mod loader;
pub mod network;
pub mod path;
pub mod places;
mod stats;
pub use stats::{dir_stats, DirStats};
#[cfg(not(windows))]
mod portable;
mod watch;
#[cfg(windows)]
mod win;

pub use loader::{load_dir, LoadEvent, LoadHandle};
pub use watch::{watch_dir, Debounce, DirWatcher};

use explorer_core::Entry;
use std::fmt;
use std::io;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsErrorKind {
    NotFound,
    PermissionDenied,
    NotADirectory,
    /// A network folder wants credentials (logon failure, not authenticated).
    LoginRequired,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsError {
    pub kind: FsErrorKind,
    pub message: String,
}

impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for FsError {}

/// Windows ERROR_DIRECTORY ("The directory name is invalid").
const WIN_ERROR_DIRECTORY: i32 = 267;

impl From<io::Error> for FsError {
    fn from(e: io::Error) -> Self {
        let kind = match e.kind() {
            io::ErrorKind::NotFound => FsErrorKind::NotFound,
            io::ErrorKind::PermissionDenied => FsErrorKind::PermissionDenied,
            io::ErrorKind::NotADirectory => FsErrorKind::NotADirectory,
            _ if cfg!(windows) && e.raw_os_error() == Some(WIN_ERROR_DIRECTORY) => {
                FsErrorKind::NotADirectory
            }
            _ if cfg!(windows)
                && e.raw_os_error()
                    .is_some_and(|c| c != 5 && network::is_login_error(c as u32)) =>
            {
                FsErrorKind::LoginRequired
            }
            _ => FsErrorKind::Other,
        };
        let message = match kind {
            FsErrorKind::NotFound => "Ordner nicht gefunden".to_string(),
            FsErrorKind::PermissionDenied => "Zugriff verweigert".to_string(),
            FsErrorKind::NotADirectory => "Kein Ordner".to_string(),
            FsErrorKind::LoginRequired => "Anmeldung erforderlich".to_string(),
            FsErrorKind::Other => e.to_string(),
        };
        FsError { kind, message }
    }
}

/// Streams the entries of `dir` to `f`. `.`/`..` are skipped; a broken single
/// entry is skipped instead of failing the whole folder. Return
/// `ControlFlow::Break` from `f` to stop early (cancellation).
pub fn enumerate(dir: &Path, f: &mut dyn FnMut(Entry) -> ControlFlow<()>) -> Result<(), FsError> {
    #[cfg(windows)]
    {
        win::enumerate(dir, f)
    }
    #[cfg(not(windows))]
    {
        portable::enumerate(dir, f)
    }
}

/// Reads a whole directory (unsorted).
pub fn read_dir(dir: &Path) -> Result<Vec<Entry>, FsError> {
    let mut out = Vec::new();
    enumerate(dir, &mut |e| {
        out.push(e);
        ControlFlow::Continue(())
    })?;
    Ok(out)
}

/// First existing directory walking up from `p` (used when the current folder vanishes).
pub fn nearest_existing_ancestor(p: &Path) -> Option<PathBuf> {
    p.ancestors()
        .find(|a| !a.as_os_str().is_empty() && a.is_dir())
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use explorer_core::{sort_entries, SortSpec};

    #[test]
    fn lists_files_and_dirs_with_metadata() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a.txt"), b"hello").unwrap();
        std::fs::create_dir(d.path().join("sub")).unwrap();
        let mut v = read_dir(d.path()).unwrap();
        sort_entries(&mut v, SortSpec::default());
        assert_eq!(v.len(), 2);
        assert!(v[0].is_dir() && v[0].name == "sub");
        assert_eq!(v[1].name, "a.txt");
        assert_eq!(v[1].size, 5);
        assert!(v[1].modified.unwrap() > 1_600_000_000);
        assert!(!v[1].hidden && !v[1].is_link);
    }

    #[test]
    fn empty_dir_is_ok() {
        let d = tempfile::tempdir().unwrap();
        assert!(read_dir(d.path()).unwrap().is_empty());
    }

    #[test]
    fn missing_dir_is_not_found() {
        let d = tempfile::tempdir().unwrap();
        let err = read_dir(&d.path().join("nope")).unwrap_err();
        assert_eq!(err.kind, FsErrorKind::NotFound);
    }

    #[test]
    fn file_is_not_a_directory() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("f.txt");
        std::fs::write(&f, b"x").unwrap();
        let err = read_dir(&f).unwrap_err();
        assert!(
            matches!(err.kind, FsErrorKind::NotADirectory | FsErrorKind::NotFound),
            "{err:?}"
        );
    }

    #[test]
    fn unicode_names() {
        let d = tempfile::tempdir().unwrap();
        for n in ["Ärger.txt", "日本語.md", "emoji_🚀.bin"] {
            std::fs::write(d.path().join(n), b"").unwrap();
        }
        let mut names: Vec<_> = read_dir(d.path())
            .unwrap()
            .into_iter()
            .map(|e| e.name)
            .collect();
        names.sort();
        assert_eq!(names, ["emoji_🚀.bin", "Ärger.txt", "日本語.md"]);
    }

    #[test]
    fn long_paths_over_260_chars() {
        let d = tempfile::tempdir().unwrap();
        let mut p = d.path().to_path_buf();
        while p.as_os_str().len() < 300 {
            p.push("a_rather_long_directory_name_0123456789");
        }
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("deep.txt"), b"1").unwrap();
        let v = read_dir(&p).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].name, "deep.txt");
    }

    #[test]
    fn early_break_stops_enumeration() {
        let d = tempfile::tempdir().unwrap();
        for i in 0..50 {
            std::fs::write(d.path().join(format!("{i}.txt")), b"").unwrap();
        }
        let mut n = 0;
        enumerate(d.path(), &mut |_| {
            n += 1;
            if n == 10 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })
        .unwrap();
        assert_eq!(n, 10);
    }

    #[test]
    fn ancestor_fallback() {
        let d = tempfile::tempdir().unwrap();
        let gone = d.path().join("x").join("y");
        assert_eq!(nearest_existing_ancestor(&gone).unwrap(), d.path());
    }

    #[cfg(unix)]
    #[test]
    fn dotfiles_hidden_and_symlink_to_dir_navigable() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join(".secret"), b"").unwrap();
        std::fs::create_dir(d.path().join("real")).unwrap();
        std::os::unix::fs::symlink(d.path().join("real"), d.path().join("link")).unwrap();
        std::os::unix::fs::symlink(d.path().join("missing"), d.path().join("dangling")).unwrap();
        let v = read_dir(d.path()).unwrap();
        let get = |n: &str| v.iter().find(|e| e.name == n).unwrap();
        assert!(get(".secret").hidden);
        assert!(get("link").is_link && get("link").is_dir());
        assert!(get("dangling").is_link && !get("dangling").is_dir());
    }

    #[cfg(windows)]
    #[test]
    fn windows_hidden_and_system_attributes() {
        use std::os::windows::ffi::OsStrExt;
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("hid.txt");
        std::fs::write(&f, b"").unwrap();
        let wide: Vec<u16> = f.as_os_str().encode_wide().chain(Some(0)).collect();
        // FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM
        let ok = unsafe {
            windows_sys::Win32::Storage::FileSystem::SetFileAttributesW(wide.as_ptr(), 0x2 | 0x4)
        };
        assert_ne!(ok, 0);
        let v = read_dir(d.path()).unwrap();
        assert!(v[0].hidden && v[0].system);
        assert!(!explorer_core::ViewFilter::default().is_visible(&v[0]));
    }

    #[cfg(windows)]
    #[test]
    fn windows_drive_root_lists() {
        let v = read_dir(Path::new("C:\\")).unwrap();
        assert!(v
            .iter()
            .any(|e| e.name.eq_ignore_ascii_case("Windows") && e.is_dir()));
    }
}
