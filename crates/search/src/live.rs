//! One changed path → the index. The watcher's event kind is never trusted
//! (renames come as two events, overflows lose some, editors replace files):
//! the disk says what is there now.

use crate::walk::{add_subtree, Walker};
use crate::{disk_meta, key};
use explorer_index::Index;
use std::path::Path;

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

/// `path` lies below `root` (same rules as the index: case only on Unix).
fn inside(path: &str, root: &str) -> bool {
    let (p, r) = if crate::CASE_INSENSITIVE {
        (path.to_lowercase(), root.to_lowercase())
    } else {
        (path.to_string(), root.to_string())
    };
    p.len() > r.len()
        && p.starts_with(&r)
        && (r.ends_with(['/', '\\']) || p[r.len()..].starts_with(['/', '\\']))
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
    }
}
