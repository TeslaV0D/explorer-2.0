//! Bring a saved index up to date (start of the app): a folder's
//! modification time changes when an entry in it is added, removed or
//! renamed (NTFS, FAT, ext4, APFS), so only folders whose time moved are read
//! again; the others cost one `stat`.
//!
//! Known limit (as without a journal in Everything): a file changed in place
//! while the app was closed keeps its old size/date until the next change
//! of its folder; the live watcher keeps everything exact while running.

use crate::walk::{add_subtree, Report, Walker};
use crate::{disk_meta, key, meta_of};
use explorer_index::{Index, Meta};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RefreshReport {
    pub dirs_checked: usize,
    pub dirs_read: usize,
    pub added: usize,
    pub removed: usize,
    pub updated: usize,
    pub errors: usize,
    pub ms: u64,
    pub cancelled: bool,
}

impl RefreshReport {
    pub fn changed(&self) -> bool {
        self.added + self.removed + self.updated > 0
    }
}

/// `since`: when the saved index was last known to be exact (Unix seconds).
/// Folders modified at or after `since - 2` are read again even if their
/// time looks unchanged (a change in the same second as the scan).
pub fn refresh(ix: &mut Index, root_id: u32, w: &Walker, since: i64) -> RefreshReport {
    let mut r = Refresh::new(root_id, since);
    r.step(ix, w, None);
    r.report()
}

/// [`refresh`] in slices: the caller holds the write lock only for one
/// [`Refresh::step`], so searches keep running while a big drive is checked.
/// Ids stay valid between steps (the index never reuses one).
pub struct Refresh {
    stack: Vec<u32>,
    since: i64,
    rep: RefreshReport,
    t0: Instant,
}

impl Refresh {
    pub fn new(root_id: u32, since: i64) -> Refresh {
        Refresh {
            stack: vec![root_id],
            since,
            rep: RefreshReport::default(),
            t0: Instant::now(),
        }
    }

    /// Works until done (true) or `deadline` (false: call again).
    pub fn step(&mut self, ix: &mut Index, w: &Walker, deadline: Option<Instant>) -> bool {
        let rep = &mut self.rep;
        while let Some(dir) = self.stack.pop() {
            if w.cancel
                .is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed))
            {
                rep.cancelled = true;
                self.stack.clear();
                break;
            }
            if !ix.exists(dir) {
                continue;
            }
            rep.dirs_checked += 1;
            let path = ix.path(dir);
            let is_root = ix.parent(dir) == explorer_index::NONE;
            let (meta, link) = match disk_meta(Path::new(&path)) {
                Ok(m) => m,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound && !is_root => {
                    rep.removed += 1;
                    ix.remove(dir);
                    continue;
                }
                Err(_) => {
                    rep.errors += 1;
                    continue;
                }
            };
            if !meta.is_dir || link && !is_root {
                // turned into a file or a link: the parent's pass fixed it already
                continue;
            }
            let stored = ix.meta(dir).mtime;
            if meta.mtime != stored || meta.mtime >= self.since - 2 {
                read_again(ix, dir, Path::new(&path), w, rep);
                ix.set_meta(dir, meta);
            }
            // deeper folders may have changed even if this one did not (links
            // among them are skipped by their own `disk_meta` above)
            self.stack
                .extend(ix.children(dir).filter(|&c| ix.is_dir(c)));
            if deadline.is_some_and(|d| Instant::now() >= d) {
                break;
            }
        }
        rep.ms = self.t0.elapsed().as_millis() as u64;
        self.stack.is_empty()
    }

    pub fn report(&self) -> RefreshReport {
        self.rep
    }
}

/// Compare one folder's children with the disk (a burst of changes in
/// one folder: one read instead of a lookup per event).
pub(crate) fn sync_dir(ix: &mut Index, dir: u32, path: &Path, w: &Walker) -> RefreshReport {
    let mut rep = RefreshReport::default();
    if let Ok((meta, _)) = disk_meta(path) {
        read_again(ix, dir, path, w, &mut rep);
        if meta.is_dir {
            ix.set_meta(dir, meta);
        }
    }
    rep
}

/// Compare the children of `dir` with the disk and fix the difference.
fn read_again(ix: &mut Index, dir: u32, path: &Path, w: &Walker, rep: &mut RefreshReport) {
    rep.dirs_read += 1;
    let disk = match explorer_fs::read_dir(path) {
        Ok(d) => d,
        Err(_) => {
            rep.errors += 1;
            return;
        }
    };
    let mut known: HashMap<String, u32> = ix.children(dir).map(|c| (key(ix.name(c)), c)).collect();
    for e in disk {
        let child = path.join(&e.name);
        if w.exclusions.excludes(&child.to_string_lossy(), &e.name) {
            continue;
        }
        let meta = meta_of(&e);
        match known.remove(&key(&e.name)) {
            Some(id) if ix.is_dir(id) == meta.is_dir => {
                if ix.name(id) != e.name {
                    // case-only rename (Windows)
                    ix.rename(id, dir, &e.name);
                    rep.updated += 1;
                }
                let old = ix.meta(id);
                if old.size != meta.size || !meta.is_dir && old.mtime != meta.mtime {
                    ix.set_meta(id, meta);
                    rep.updated += 1;
                }
            }
            Some(id) => {
                // file ↔ folder under the same name
                ix.remove(id);
                rep.removed += 1;
                add_new(ix, dir, &child, &e, meta, w, rep);
            }
            None => add_new(ix, dir, &child, &e, meta, w, rep),
        }
    }
    for (_, gone) in known {
        ix.remove(gone);
        rep.removed += 1;
    }
}

fn add_new(
    ix: &mut Index,
    dir: u32,
    child: &Path,
    e: &explorer_core::Entry,
    meta: Meta,
    w: &Walker,
    rep: &mut RefreshReport,
) {
    let id = ix.add(dir, &e.name, meta);
    rep.added += 1;
    if e.is_dir() && !e.is_link {
        let r: Report = add_subtree(ix, id, child, w);
        rep.added += r.entries;
        rep.errors += r.errors;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{snapshot, tree};
    use crate::walk::build;
    use explorer_index::Exclusions;

    fn mutate(root: &Path) {
        std::fs::write(root.join("a/neu.txt"), b"neu").unwrap();
        std::fs::remove_file(root.join("a/1.txt")).unwrap();
        std::fs::rename(root.join("a/b"), root.join("Leer/b-verschoben")).unwrap();
        std::fs::create_dir_all(root.join("frisch/tief/drin")).unwrap();
        std::fs::write(root.join("frisch/tief/drin/z.bin"), b"zz").unwrap();
        std::fs::remove_dir_all(root.join("Ünïcode ordner")).unwrap();
        std::fs::write(root.join("oben.txt"), b"1234567890").unwrap(); // in place
                                                                       // a file becomes a folder of the same name
        std::fs::remove_file(root.join("a/2.jpg")).unwrap();
        std::fs::create_dir(root.join("a/2.jpg")).unwrap();
    }

    #[test]
    fn after_changes_equals_a_fresh_build() {
        let d = tempfile::tempdir().unwrap();
        tree(d.path());
        let ex = Exclusions::parse("node_modules", crate::CASE_INSENSITIVE);
        let w = Walker::new(&ex);
        let (mut ix, _) = build(d.path(), &w);
        let built = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        mutate(d.path());
        let root = ix.roots()[0];
        // changes in the same second as the build: only `since` catches them
        let rep = refresh(&mut ix, root, &w, built);
        let (fresh, _) = build(d.path(), &w);
        assert_eq!(
            snapshot(&ix, d.path()),
            snapshot(&fresh, d.path()),
            "{rep:?}"
        );
        assert!(
            rep.added >= 4 && rep.removed >= 3 && rep.changed(),
            "{rep:?}"
        );
        // and it can still be searched + saved
        ix.rebuild_order();
        let back = explorer_index::Index::from_bytes(&ix.to_bytes("t", 1))
            .unwrap()
            .0;
        assert_eq!(snapshot(&back, d.path()), snapshot(&fresh, d.path()));
    }

    #[test]
    fn nothing_changed_nothing_read() {
        let d = tempfile::tempdir().unwrap();
        tree(d.path());
        let ex = Exclusions::default();
        let w = Walker::new(&ex);
        let (mut ix, _) = build(d.path(), &w);
        let root = ix.roots()[0];
        // NTFS may update a folder's last-write time a moment after files were
        // created in it (CI #155): a first pass takes such late times in
        let _ = refresh(&mut ix, root, &w, i64::MAX / 2);
        // exact "since" later than every folder time: only times are compared
        let rep = refresh(&mut ix, root, &w, i64::MAX / 2);
        assert!(!rep.changed(), "{rep:?}");
        assert_eq!(
            rep.dirs_read, 0,
            "unchanged times, old `since`: no folder read again"
        );
        assert!(rep.dirs_checked >= 7);
    }

    #[test]
    fn vanished_root_children_and_errors_do_not_panic() {
        let d = tempfile::tempdir().unwrap();
        tree(d.path());
        let ex = Exclusions::default();
        let w = Walker::new(&ex);
        let (mut ix, _) = build(d.path(), &w);
        let built = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        for e in std::fs::read_dir(d.path()).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                std::fs::remove_dir_all(p).unwrap();
            } else {
                std::fs::remove_file(p).unwrap();
            }
        }
        let root = ix.roots()[0];
        // deleted in the same second as the build: `since` makes the root count
        let rep = refresh(&mut ix, root, &w, built);
        assert_eq!(ix.len(), 1, "only the root is left: {rep:?}");
    }
}
