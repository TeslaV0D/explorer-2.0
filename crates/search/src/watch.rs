//! Recursive watch of a whole drive: changed paths are collected in a set
//! (bursts of the same path count once) and applied in batches; a lost
//! event (buffer overflow, watcher error) marks the volume for a refresh.

use explorer_fs::Debounce;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// More pending paths than this: drop them and refresh instead (memory bound).
pub const MAX_PENDING: usize = 200_000;

/// Changes waiting to be applied, shared by the watcher and the maintainer.
pub struct Pending {
    inner: Mutex<Inner>,
    wake: Condvar,
}

struct Inner {
    paths: HashSet<PathBuf>,
    overflow: bool,
    stop: bool,
    debounce: Debounce,
}

/// Far enough ahead to end any burst (resets the debounce).
fn later() -> Instant {
    Instant::now() + Duration::from_secs(3600)
}

/// What [`Pending::take`] hands out.
#[derive(Debug, Default)]
pub struct Batch {
    pub paths: Vec<PathBuf>,
    /// Events were lost: compare the whole drive with the disk (then
    /// apply `paths` too).
    pub overflow: bool,
}

impl Pending {
    /// `quiet`: wait this long after the last event; `max_wait`: but no
    /// longer than this after the first (a busy drive still updates).
    pub fn new(quiet: Duration, max_wait: Duration) -> Pending {
        Pending {
            inner: Mutex::new(Inner {
                paths: HashSet::new(),
                overflow: false,
                stop: false,
                debounce: Debounce::new(quiet, max_wait),
            }),
            wake: Condvar::new(),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn push(&self, paths: impl IntoIterator<Item = PathBuf>) {
        let mut g = self.lock();
        g.paths.extend(paths);
        if g.paths.len() > MAX_PENDING {
            g.paths = HashSet::new();
            g.overflow = true;
        }
        g.debounce.event(Instant::now());
        self.wake.notify_all();
    }

    /// Events were lost (or a full check is wanted). Paths keep being
    /// collected: a file changed in place does not show in its folder's time.
    pub fn overflow(&self) {
        self.lock().overflow = true;
        self.wake.notify_all();
    }

    pub fn stop(&self) {
        self.lock().stop = true;
        self.wake.notify_all();
    }

    pub fn stopped(&self) -> bool {
        self.lock().stop
    }

    /// Waits until a batch is due, `stop` was called, or `timeout` passed.
    pub fn wait(&self, timeout: Duration) {
        let g = self.lock();
        if g.stop || g.overflow {
            return;
        }
        let wait = match g.debounce.deadline() {
            Some(d) => d.saturating_duration_since(Instant::now()).min(timeout),
            None => timeout,
        };
        if wait.is_zero() {
            return;
        }
        let _ = self.wake.wait_timeout(g, wait);
    }

    /// The due batch, if any (`force`: whatever is there, e.g. at shutdown).
    pub fn take(&self, force: bool) -> Option<Batch> {
        let mut g = self.lock();
        // a lost event makes the batch due at once
        let due = g.debounce.poll(Instant::now()) || g.overflow;
        if !due && !(force && !g.paths.is_empty()) {
            return None;
        }
        g.debounce.poll(later());
        let paths = std::mem::take(&mut g.paths).into_iter().collect();
        let overflow = std::mem::replace(&mut g.overflow, false);
        Some(Batch { paths, overflow })
    }
}

/// Keeps the watch alive; dropping it stops watching.
pub struct TreeWatcher {
    _watcher: RecommendedWatcher,
}

/// Watches everything below `root` (ReadDirectoryChangesW with subtree on
/// Windows, inotify per folder on Linux) into `pending`.
pub fn watch_tree(root: &Path, pending: Arc<Pending>) -> notify::Result<TreeWatcher> {
    let mut watcher =
        notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
            Ok(ev) if ev.need_rescan() => pending.overflow(),
            Ok(ev) => {
                if !ev.paths.is_empty() {
                    pending.push(ev.paths);
                }
            }
            Err(_) => pending.overflow(),
        })?;
    watcher.watch(root, RecursiveMode::Recursive)?;
    Ok(TreeWatcher { _watcher: watcher })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_debounces_dedups_and_overflows() {
        let p = Pending::new(Duration::from_millis(30), Duration::from_millis(200));
        assert!(p.take(false).is_none());
        p.push([PathBuf::from("/a"), PathBuf::from("/b")]);
        p.push([PathBuf::from("/a")]);
        assert!(p.take(false).is_none(), "still within the quiet time");
        std::thread::sleep(Duration::from_millis(40));
        let b = p.take(false).unwrap();
        assert_eq!(b.paths.len(), 2);
        assert!(!b.overflow);
        assert!(p.take(false).is_none());
        // forced: right away
        p.push([PathBuf::from("/c")]);
        assert_eq!(p.take(true).unwrap().paths, vec![PathBuf::from("/c")]);
        assert!(p.take(false).is_none(), "forced take resets the burst");
        // overflow: due at once, paths kept
        p.push([PathBuf::from("/d")]);
        p.overflow();
        let b = p.take(false).unwrap();
        assert!(b.overflow);
        assert_eq!(b.paths, vec![PathBuf::from("/d")]);
        // too many paths: overflow
        p.push((0..MAX_PENDING + 1).map(|i| PathBuf::from(format!("/{i}"))));
        let b = p.take(true).unwrap();
        assert!(b.overflow && b.paths.is_empty());
    }

    #[test]
    fn wait_returns_on_stop() {
        let p = Arc::new(Pending::new(
            Duration::from_millis(30),
            Duration::from_millis(200),
        ));
        let q = p.clone();
        let t = std::thread::spawn(move || {
            let t0 = Instant::now();
            q.wait(Duration::from_secs(10));
            t0.elapsed()
        });
        std::thread::sleep(Duration::from_millis(50));
        p.stop();
        assert!(t.join().unwrap() < Duration::from_secs(5));
        assert!(p.stopped());
    }
}
