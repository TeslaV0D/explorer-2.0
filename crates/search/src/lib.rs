//! The search index kept in step with the disk (PLAN-SEARCH.md S1.3):
//!
//! - [`walk`]: parallel first build of a drive, and of any folder that
//!   appears later (moved in, created while offline).
//! - [`refresh`]: after loading a saved index, look at every folder's
//!   modification time and read again only the ones that changed.
//! - [`live`]: apply one changed path (from the file system watcher) by
//!   looking at the disk, never trusting the event's kind.
//! - [`watch`]: recursive watcher → set of changed paths + overflow flag.
//! - [`service`]: one maintainer thread per drive (load or build, refresh,
//!   live batches, reorder, save); searches read in parallel.
//!
//! Links (symlinks, junctions) are indexed but never followed: no loops,
//! no double counting (`C:\Documents and Settings`).

pub mod live;
pub mod refresh;
pub mod service;
pub mod walk;
pub mod watch;

pub use service::{Config, Hits, Origin, Phase, Search, Status, Volume};

use explorer_core::Entry;
use explorer_index::Meta;
use std::path::Path;

/// File names compare without case on Windows.
pub const CASE_INSENSITIVE: bool = cfg!(windows);

pub(crate) fn meta_of(e: &Entry) -> Meta {
    Meta {
        is_dir: e.is_dir(),
        size: if e.is_dir() { 0 } else { e.size },
        mtime: e.modified.unwrap_or(0),
    }
}

/// Modification time of `path` itself (links not followed), seconds.
pub(crate) fn disk_meta(path: &Path) -> std::io::Result<(Meta, bool)> {
    let md = std::fs::symlink_metadata(path)?;
    // Rust reports symlinks and junctions (name-surrogate reparse points) as
    // symlinks; cloud placeholders (OneDrive) stay regular files/folders
    let link = md.file_type().is_symlink();
    // a link to a folder is shown as a folder (like the list does), but not entered
    let is_dir = if link {
        std::fs::metadata(path).map(|m| m.is_dir()).unwrap_or(false)
    } else {
        md.is_dir()
    };
    let mtime = md
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs() as i64);
    Ok((
        Meta {
            is_dir,
            size: if is_dir { 0 } else { md.len() },
            mtime,
        },
        link,
    ))
}

/// Indexing must not slow down the user's work (Windows: below normal;
/// the disk I/O follows the thread priority).
pub(crate) fn background_priority() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::{
            GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL,
        };
        // SAFETY: pseudo handle of the calling thread.
        unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL) };
    }
}

/// Name key for comparing a folder's children with the disk.
pub(crate) fn key(name: &str) -> String {
    if CASE_INSENSITIVE {
        name.to_lowercase()
    } else {
        name.to_string()
    }
}

#[cfg(test)]
pub(crate) mod testutil {
    use explorer_index::Index;
    use std::path::Path;

    /// Every entry below the root as (path relative to the root, kind, size).
    pub fn snapshot(ix: &Index, root: &Path) -> Vec<(String, bool, u64)> {
        let base = root.to_string_lossy().to_string();
        let mut v: Vec<(String, bool, u64)> = (0..ix.slots() as u32)
            .filter(|&i| ix.exists(i) && ix.parent(i) != explorer_index::NONE)
            .map(|i| {
                let p = ix.path(i);
                let rel = p
                    .strip_prefix(&base)
                    .unwrap_or(&p)
                    .trim_start_matches(['/', '\\'])
                    .replace('\\', "/");
                let m = ix.meta(i);
                (rel, m.is_dir, m.size)
            })
            .collect();
        v.sort();
        v
    }

    pub fn tree(root: &Path) {
        for (dir, files) in [
            ("a", &["1.txt", "2.jpg"][..]),
            ("a/b", &["tief.pdf"][..]),
            ("a/b/c", &[][..]),
            ("Leer", &[][..]),
            ("Ünïcode ordner", &["straße.md", "x y z.txt"][..]),
            ("node_modules/pkg", &["index.js"][..]),
        ] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
            for f in files {
                std::fs::write(root.join(dir).join(f), f.as_bytes()).unwrap();
            }
        }
        std::fs::write(root.join("oben.txt"), b"12345").unwrap();
    }
}
