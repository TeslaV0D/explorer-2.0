//! One changed path → the index. The watcher's event kind is never trusted
//! (renames come as two events, overflows lose some, editors replace files):
//! the disk says what is there now.

use crate::walk::{add_subtree, Walker};
use crate::{disk_meta, key};
use explorer_index::Index;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applied {
    /// New entry (for a folder: with everything inside, `count` entries).
    Added {
        count: usize,
    },
    Updated,
    Removed,
    /// Outside the indexed root, excluded, or unreadable.
    Ignored,
    Unchanged,
}

/// Apply the current state of `path` (below the root `root_id`).
pub fn apply(ix: &mut Index, root_id: u32, path: &Path, w: &Walker) -> Applied {
    let root_path = ix.path(root_id);
    let p = path.to_string_lossy();
    if !inside(&p, &root_path) {
        return Applied::Ignored;
    }
    let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else {
        return Applied::Ignored;
    };
    if w.exclusions.excludes(&p, &name) {
        return Applied::Ignored;
    }
    match disk_meta(path) {
        Ok((meta, link)) => {
            let Some(parent) = path.parent() else {
                return Applied::Ignored;
            };
            let pid = match ix.find(&parent.to_string_lossy()) {
                Some(id) => id,
                // the parent is new too (created + filled faster than the events)
                None => match apply(ix, root_id, parent, w) {
                    Applied::Added { .. } => return Applied::Added { count: 1 },
                    _ => match ix.find(&parent.to_string_lossy()) {
                        Some(id) => id,
                        None => return Applied::Ignored,
                    },
                },
            };
            if !ix.is_dir(pid) {
                return Applied::Ignored;
            }
            match ix.children(pid).find(|&c| key(ix.name(c)) == key(&name)) {
                Some(id) if ix.is_dir(id) == meta.is_dir => {
                    let mut changed = false;
                    if ix.name(id) != name {
                        ix.rename(id, pid, &name);
                        changed = true;
                    }
                    let old = ix.meta(id);
                    if old.size != meta.size || old.mtime != meta.mtime {
                        ix.set_meta(id, meta);
                        changed = true;
                    }
                    if changed {
                        Applied::Updated
                    } else {
                        Applied::Unchanged
                    }
                }
                existing => {
                    if let Some(old) = existing {
                        ix.remove(old); // file ↔ folder under the same name
                    }
                    let id = ix.add(pid, &name, meta);
                    let mut count = 1;
                    if meta.is_dir && !link {
                        // moved in with content, or filled before we looked
                        count += add_subtree(ix, id, path, w).entries;
                    }
                    Applied::Added { count }
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => match ix.find(&p) {
            Some(id) if id != root_id => {
                ix.remove(id);
                Applied::Removed
            }
            _ => Applied::Unchanged,
        },
        Err(_) => Applied::Ignored,
    }
}

/// One unit of a batch of changed paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Work {
    /// Look at this path.
    Path(PathBuf),
    /// Many changes in this folder: compare its children once.
    Folder(PathBuf),
}

/// From this many changes in one folder on, it is read once instead of
/// looking up every name (a lookup walks the folder's children).
pub const BULK: usize = 32;

/// Orders a batch of changed paths into work: paths grouped by folder,
/// big groups as one [`Work::Folder`].
pub fn plan(paths: impl IntoIterator<Item = PathBuf>) -> Vec<Work> {
    let mut by_dir: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for p in paths {
        let dir = p.parent().map(Path::to_path_buf).unwrap_or_default();
        by_dir.entry(dir).or_default().push(p);
    }
    let mut dirs: Vec<_> = by_dir.into_iter().collect();
    // parents first: a new folder is then added with its content in one go
    dirs.sort_by_key(|(d, _)| d.components().count());
    let mut out = Vec::new();
    for (dir, mut paths) in dirs {
        if paths.len() >= BULK {
            out.push(Work::Folder(dir));
        } else {
            paths.sort();
            paths.dedup();
            out.extend(paths.into_iter().map(Work::Path));
        }
    }
    out
}

/// What one unit of work did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ran {
    Unchanged,
    Changed,
    /// Something new appeared (in the folder [`recheck_target`] names).
    Added,
}

/// Does one unit.
pub fn run(ix: &mut Index, root_id: u32, work: &Work, w: &Walker) -> Ran {
    let from = |a: Applied| match a {
        Applied::Unchanged | Applied::Ignored => Ran::Unchanged,
        Applied::Added { .. } => Ran::Added,
        Applied::Updated | Applied::Removed => Ran::Changed,
    };
    match work {
        Work::Path(p) => from(apply(ix, root_id, p, w)),
        Work::Folder(dir) => {
            let p = dir.to_string_lossy();
            match ix.find(&p) {
                Some(id) if ix.is_dir(id) => {
                    let r = crate::refresh::sync_dir(ix, id, dir, w);
                    if r.added > 0 {
                        Ran::Added
                    } else if r.changed() {
                        Ran::Changed
                    } else {
                        Ran::Unchanged
                    }
                }
                // the folder itself is new (or gone): it decides for its content
                _ => from(apply(ix, root_id, dir, w)),
            }
        }
    }
}

/// inotify (Linux) reports a new folder before it watches it: files made in
/// that moment never come as events. A folder that got something new is
/// therefore compared with the disk once more a moment later. Windows and
/// macOS watch the whole tree at once and do not need this.
pub const RECHECK_NEW: bool = cfg!(target_os = "linux");

/// The folder to compare again after `work` added something.
pub fn recheck_target(work: &Work) -> Option<PathBuf> {
    match work {
        Work::Path(p) => p.parent().map(Path::to_path_buf),
        Work::Folder(d) => Some(d.clone()),
    }
}

/// `path` lies below `root` (same rules as the index: case only on Unix).
pub(crate) fn inside(path: &str, root: &str) -> bool {
    // A Windows root is stored with `\`, a watcher or an address typed with
    // `/` must still count as inside it (`C:/Users` and `C:\Users`). Unix
    // roots keep both characters.
    let windows = |s: &str| s.contains('\\') || (s.len() >= 2 && s.as_bytes()[1] == b':');
    let windows = windows(path) || windows(root);
    let norm = |s: &str| {
        let s = if crate::CASE_INSENSITIVE {
            s.to_lowercase()
        } else {
            s.to_string()
        };
        if windows {
            s.replace('\\', "/")
        } else {
            s
        }
    };
    let (p, r) = (norm(path), norm(root));
    p.len() > r.len() && p.starts_with(&r) && (r.ends_with('/') || p[r.len()..].starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{snapshot, tree};
    use crate::walk::build;
    use explorer_index::Exclusions;

    #[test]
    fn applying_the_changed_paths_equals_a_fresh_build() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        tree(r);
        let ex = Exclusions::parse("node_modules;*.tmp", crate::CASE_INSENSITIVE);
        let w = Walker::new(&ex);
        let (mut ix, _) = build(r, &w);
        let root = ix.roots()[0];
        // what a watcher reports (any order, duplicates, both rename ends)
        std::fs::write(r.join("a/neu.txt"), b"neu").unwrap();
        std::fs::rename(r.join("a/b"), r.join("Leer/b2")).unwrap();
        std::fs::create_dir_all(r.join("frisch/x/y")).unwrap();
        std::fs::write(r.join("frisch/x/y/z.md"), b"z").unwrap();
        std::fs::remove_dir_all(r.join("Ünïcode ordner")).unwrap();
        std::fs::write(r.join("oben.txt"), b"lang und neu").unwrap();
        std::fs::write(r.join("weg.tmp"), b"excluded").unwrap();
        std::fs::remove_file(r.join("a/2.jpg")).unwrap();
        std::fs::create_dir(r.join("a/2.jpg")).unwrap();
        let events = [
            "a/neu.txt",
            "a/b",
            "Leer/b2",
            "frisch/x/y/z.md",
            "frisch/x/y",
            "frisch",
            "Ünïcode ordner/straße.md",
            "Ünïcode ordner",
            "oben.txt",
            "oben.txt",
            "weg.tmp",
            "a/2.jpg",
            "gibt-es-nicht",
        ];
        for e in events {
            apply(&mut ix, root, &r.join(e), &w);
        }
        let (fresh, _) = build(r, &w);
        assert_eq!(snapshot(&ix, r), snapshot(&fresh, r));
        // outside the root: ignored
        assert_eq!(
            apply(&mut ix, root, Path::new("/somewhere/else"), &w),
            Applied::Ignored
        );
        assert_eq!(
            apply(&mut ix, root, &r.join("oben.txt"), &w),
            Applied::Unchanged
        );
        // results searchable without a rebuild (unsorted tail)
        let q = explorer_index::Query::parse(
            "z.md",
            explorer_index::Now {
                unix: 0,
                utc_offset: 0,
                offset_at: None,
            },
        )
        .unwrap();
        assert_eq!(
            ix.search(&q, None, explorer_index::SortBy::Name, false)
                .len(),
            1
        );
    }

    #[test]
    fn a_new_folder_compared_again_finds_files_without_events() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        tree(r);
        let ex = Exclusions::default();
        let w = Walker::new(&ex);
        let (mut ix, _) = build(r, &w);
        let root = ix.roots()[0];
        let applied_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        std::fs::create_dir_all(r.join("a/neu/tiefer")).unwrap();
        let work = Work::Path(r.join("a/neu"));
        assert_eq!(run(&mut ix, root, &work, &w), Ran::Added);
        // made after the folder was read, before inotify watched it: no event
        std::fs::write(r.join("a/neu/spaet.txt"), b"s").unwrap();
        std::fs::write(r.join("a/neu/tiefer/noch.txt"), b"n").unwrap();
        let target = recheck_target(&work).unwrap();
        assert_eq!(target, r.join("a"));
        let start = ix.find(&target.to_string_lossy()).unwrap();
        let mut again = crate::refresh::Refresh::new(start, applied_at);
        assert!(again.step(&mut ix, &w, None));
        assert_eq!(snapshot(&ix, r), snapshot(&build(r, &w).0, r));
        // a burst in one folder also names that folder
        assert_eq!(
            recheck_target(&Work::Folder(r.join("x"))),
            Some(r.join("x"))
        );
    }

    #[test]
    fn root_detection() {
        if crate::CASE_INSENSITIVE {
            assert!(inside(r"C:\a\b", r"c:\"));
            assert!(inside(r"c:\A", r"C:\"));
        } else {
            assert!(inside("/home/a/b", "/home/a"));
            assert!(!inside("/home/ab", "/home/a"));
            assert!(!inside("/home/a", "/home/a"));
            assert!(!inside("/home/A/b", "/home/a"));
        }
        // separators, on either platform (the root is a Windows path)
        assert!(inside(r"C:/Users/foo", r"C:\Users"));
        assert!(inside(r"C:\Users/foo", "C:/Users"));
        assert!(!inside(r"C:/Users", r"C:\Users"));
        assert!(!inside(r"C:/Users2/foo", r"C:\Users"));
    }
}
