//! Parallel walk: worker threads read folders, one collector inserts into
//! the index (the index has a single writer) and hands out the subfolders.

use crate::meta_of;
use explorer_index::{Exclusions, Index};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Condvar, Mutex};
use std::time::Instant;

/// How to walk.
#[derive(Clone, Copy)]
pub struct Walker<'a> {
    pub exclusions: &'a Exclusions,
    /// Reader threads (folders are read in parallel; the disk decides).
    pub threads: usize,
    pub cancel: Option<&'a AtomicBool>,
    /// Called about every 20 000 entries.
    pub progress: Option<&'a (dyn Fn(Progress) + Sync)>,
}

impl<'a> Walker<'a> {
    pub fn new(exclusions: &'a Exclusions) -> Walker<'a> {
        Walker {
            exclusions,
            threads: std::thread::available_parallelism()
                .map_or(4, |n| n.get() * 2)
                .clamp(2, 16),
            cancel: None,
            progress: None,
        }
    }

    fn cancelled(&self) -> bool {
        self.cancel.is_some_and(|c| c.load(Ordering::Relaxed))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Progress {
    pub entries: usize,
    pub dirs: usize,
    pub errors: usize,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Report {
    pub entries: usize,
    pub dirs: usize,
    /// Folders that could not be read (no access, vanished meanwhile).
    pub errors: usize,
    pub ms: u64,
    pub cancelled: bool,
}

impl std::ops::AddAssign for Report {
    fn add_assign(&mut self, o: Report) {
        self.entries += o.entries;
        self.dirs += o.dirs;
        self.errors += o.errors;
        self.ms += o.ms;
        self.cancelled |= o.cancelled;
    }
}

/// A fresh index of `root` (in name order, ready to search).
pub fn build(root: &Path, w: &Walker) -> (Index, Report) {
    let mut ix = Index::new(crate::CASE_INSENSITIVE);
    let meta = crate::disk_meta(root)
        .map(|(m, _)| m)
        .unwrap_or(explorer_index::Meta {
            is_dir: true,
            size: 0,
            mtime: 0,
        });
    let root_id = ix.add_root(&root.to_string_lossy(), meta);
    let report = add_subtree(&mut ix, root_id, root, w);
    ix.rebuild_order();
    (ix, report)
}

struct Queue {
    jobs: VecDeque<(PathBuf, u32)>,
    done: bool,
}

/// Everything below the folder `dir_id` (at `dir`), which must have no
/// children in the index yet.
pub fn add_subtree(ix: &mut Index, dir_id: u32, dir: &Path, w: &Walker) -> Report {
    let t0 = Instant::now();
    let mut rep = Report::default();
    let shared = (
        Mutex::new(Queue {
            jobs: VecDeque::from([(dir.to_path_buf(), dir_id)]),
            done: false,
        }),
        Condvar::new(),
    );
    // jobs queued + being read + read but not yet inserted
    let mut in_flight = 1usize;
    let (tx, rx) = mpsc::channel::<(
        u32,
        PathBuf,
        Result<Vec<explorer_core::Entry>, explorer_fs::FsError>,
    )>();
    std::thread::scope(|s| {
        for _ in 0..w.threads.max(1) {
            let tx = tx.clone();
            let shared = &shared;
            s.spawn(move || {
                crate::background_priority();
                loop {
                    let job = {
                        let (lock, cv) = shared;
                        let mut q = lock.lock().unwrap_or_else(|e| e.into_inner());
                        loop {
                            if q.done {
                                return;
                            }
                            if let Some(j) = q.jobs.pop_front() {
                                break j;
                            }
                            q = cv.wait(q).unwrap_or_else(|e| e.into_inner());
                        }
                    };
                    let res = explorer_fs::read_dir(&job.0);
                    if tx.send((job.1, job.0, res)).is_err() {
                        return;
                    }
                }
            });
        }
        drop(tx);
        let stop = |shared: &(Mutex<Queue>, Condvar)| {
            let (lock, cv) = shared;
            lock.lock().unwrap_or_else(|e| e.into_inner()).done = true;
            cv.notify_all();
        };
        let mut last_report = 0usize;
        while in_flight > 0 {
            let Ok((id, path, res)) = rx.recv() else {
                break;
            };
            in_flight -= 1;
            if w.cancelled() {
                rep.cancelled = true;
                break;
            }
            rep.dirs += 1;
            let entries = match res {
                Ok(e) => e,
                Err(_) => {
                    rep.errors += 1;
                    continue;
                }
            };
            let mut new_jobs = Vec::new();
            for e in entries {
                let child = path.join(&e.name);
                if w.exclusions.excludes(&child.to_string_lossy(), &e.name) {
                    continue;
                }
                let cid = ix.add(id, &e.name, meta_of(&e));
                rep.entries += 1;
                if e.is_dir() && !e.is_link {
                    new_jobs.push((child, cid));
                }
            }
            if !new_jobs.is_empty() {
                in_flight += new_jobs.len();
                let (lock, cv) = &shared;
                lock.lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .jobs
                    .extend(new_jobs);
                cv.notify_all();
            }
            if let Some(p) = w.progress {
                if rep.entries >= last_report + 20_000 {
                    last_report = rep.entries;
                    p(Progress {
                        entries: rep.entries,
                        dirs: rep.dirs,
                        errors: rep.errors,
                        elapsed_ms: t0.elapsed().as_millis() as u64,
                    });
                }
            }
        }
        stop(&shared);
    });
    rep.ms = t0.elapsed().as_millis() as u64;
    rep
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{snapshot, tree};

    #[test]
    fn builds_everything_but_the_excluded() {
        let d = tempfile::tempdir().unwrap();
        tree(d.path());
        let ex = Exclusions::parse("node_modules", crate::CASE_INSENSITIVE);
        let (ix, rep) = build(d.path(), &Walker::new(&ex));
        let snap = snapshot(&ix, d.path());
        let names: Vec<&str> = snap.iter().map(|s| s.0.as_str()).collect();
        assert!(names.contains(&"a/b/tief.pdf"));
        assert!(names.contains(&"Ünïcode ordner/straße.md"));
        assert!(names.contains(&"Leer"));
        assert!(
            !names.iter().any(|n| n.contains("node_modules")),
            "{names:?}"
        );
        assert_eq!(rep.entries, snap.len());
        assert_eq!(rep.errors, 0);
        assert!(snap.contains(&("oben.txt".into(), false, 5)));
        // searchable right away (name order built)
        let q = explorer_index::Query::parse(
            "tief",
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
        assert_eq!(ix.unsorted(), 0);
    }

    #[test]
    fn same_result_with_one_or_many_threads() {
        let d = tempfile::tempdir().unwrap();
        for i in 0..40 {
            let p = d.path().join(format!("d{i}/e{}", i % 7));
            std::fs::create_dir_all(&p).unwrap();
            for f in 0..15 {
                std::fs::write(p.join(format!("f{f}.txt")), b"x").unwrap();
            }
        }
        let ex = Exclusions::default();
        let mut one = Walker::new(&ex);
        one.threads = 1;
        let (a, _) = build(d.path(), &one);
        let (b, _) = build(d.path(), &Walker::new(&ex));
        assert_eq!(snapshot(&a, d.path()), snapshot(&b, d.path()));
        assert_eq!(a.len(), 1 + 40 + 40 + 40 * 15);
    }

    #[test]
    fn cancel_stops_early() {
        let d = tempfile::tempdir().unwrap();
        tree(d.path());
        let ex = Exclusions::default();
        let stop = AtomicBool::new(true);
        let mut w = Walker::new(&ex);
        w.cancel = Some(&stop);
        let (_, rep) = build(d.path(), &w);
        assert!(rep.cancelled);
    }

    #[cfg(unix)]
    #[test]
    fn links_are_listed_not_followed() {
        let d = tempfile::tempdir().unwrap();
        tree(d.path());
        // a loop: a/b/c/zurück -> a
        std::os::unix::fs::symlink(d.path().join("a"), d.path().join("a/b/c/zurück")).unwrap();
        let (ix, rep) = build(d.path(), &Walker::new(&Exclusions::default()));
        let snap = snapshot(&ix, d.path());
        assert!(snap.iter().any(|s| s.0 == "a/b/c/zurück"));
        assert!(!snap.iter().any(|s| s.0.starts_with("a/b/c/zurück/")));
        assert_eq!(rep.errors, 0);
    }
}
