//! Folder size for the inspector: files, folders and bytes below a folder,
//! counted on a background thread with cancel and progress.
//! Links/junctions are counted but never followed (no loops, no double
//! counting through mount points); unreadable folders are skipped and counted.

use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DirStats {
    pub files: u64,
    pub folders: u64,
    pub bytes: u64,
    /// Folders that could not be read (access denied, vanished).
    pub unreadable: u64,
    /// The whole tree was visited (not cancelled, not over the limit).
    pub complete: bool,
    /// Stopped at `MAX_ENTRIES` (the numbers are a lower bound).
    pub truncated: bool,
}

/// Counting stops here: an automatic count must never walk a whole disk.
pub const MAX_ENTRIES: u64 = 500_000;

/// Walks `root` (not counted itself). `progress` gets intermediate results
/// about every `every`; returns the final result (or the partial one when
/// `cancel` was set).
pub fn dir_stats(
    root: &Path,
    cancel: &AtomicBool,
    every: Duration,
    progress: impl FnMut(&DirStats),
) -> DirStats {
    dir_stats_limited(root, cancel, every, MAX_ENTRIES, progress)
}

/// [`dir_stats`] with an explicit entry limit (tests).
pub fn dir_stats_limited(
    root: &Path,
    cancel: &AtomicBool,
    every: Duration,
    limit: u64,
    mut progress: impl FnMut(&DirStats),
) -> DirStats {
    let mut s = DirStats::default();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    let mut last = Instant::now();
    while let Some(dir) = stack.pop() {
        let res = crate::enumerate(&dir, &mut |e| {
            if cancel.load(Ordering::Relaxed) || s.files + s.folders >= limit {
                return ControlFlow::Break(());
            }
            if e.is_dir() {
                s.folders += 1;
                if !e.is_link {
                    stack.push(dir.join(&e.name));
                }
            } else {
                s.files += 1;
                s.bytes += e.size;
            }
            ControlFlow::Continue(())
        });
        if res.is_err() {
            s.unreadable += 1;
        }
        if cancel.load(Ordering::Relaxed) {
            return s;
        }
        if s.files + s.folders >= limit {
            s.truncated = true;
            return s;
        }
        if last.elapsed() >= every {
            last = Instant::now();
            progress(&s);
        }
    }
    s.complete = true;
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_a_tree_and_can_be_cancelled() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("a/b")).unwrap();
        std::fs::write(d.path().join("x.bin"), [0u8; 100]).unwrap();
        std::fs::write(d.path().join("a/y.bin"), [0u8; 20]).unwrap();
        std::fs::write(d.path().join("a/b/z.bin"), [0u8; 3]).unwrap();
        let no = AtomicBool::new(false);
        let s = dir_stats(d.path(), &no, Duration::ZERO, |_| {});
        assert_eq!((s.files, s.folders, s.bytes, s.complete), (3, 2, 123, true));
        let yes = AtomicBool::new(true);
        assert!(!dir_stats(d.path(), &yes, Duration::ZERO, |_| {}).complete);
        let capped = dir_stats_limited(d.path(), &no, Duration::ZERO, 2, |_| {});
        assert!(capped.truncated && !capped.complete && capped.files + capped.folders <= 2);
        let gone = dir_stats(&d.path().join("gibtsnicht"), &no, Duration::ZERO, |_| {});
        assert_eq!((gone.unreadable, gone.files), (1, 0));
    }

    #[cfg(unix)]
    #[test]
    fn links_are_not_followed() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir(d.path().join("real")).unwrap();
        std::fs::write(d.path().join("real/f"), [0u8; 10]).unwrap();
        std::os::unix::fs::symlink(d.path(), d.path().join("real/loop")).unwrap();
        let s = dir_stats(d.path(), &AtomicBool::new(false), Duration::ZERO, |_| {});
        assert!(s.complete);
        assert_eq!(s.bytes, 10, "the loop back to the root is not walked");
    }
}
