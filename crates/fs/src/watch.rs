//! Directory change watching (ReadDirectoryChangesW / inotify via `notify`) with debouncing.

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Coalesces bursts of events: fires after `quiet` without new events, but at
/// the latest `max_wait` after the first event of a burst (so a constantly
/// changing folder still refreshes).
#[derive(Debug, Clone)]
pub struct Debounce {
    quiet: Duration,
    max_wait: Duration,
    first: Option<Instant>,
    last: Option<Instant>,
}

impl Debounce {
    pub fn new(quiet: Duration, max_wait: Duration) -> Self {
        Self {
            quiet,
            max_wait,
            first: None,
            last: None,
        }
    }

    pub fn event(&mut self, now: Instant) {
        self.first.get_or_insert(now);
        self.last = Some(now);
    }

    pub fn deadline(&self) -> Option<Instant> {
        Some((self.last? + self.quiet).min(self.first? + self.max_wait))
    }

    /// Returns true (and resets) if the burst is due.
    pub fn poll(&mut self, now: Instant) -> bool {
        match self.deadline() {
            Some(d) if now >= d => {
                self.first = None;
                self.last = None;
                true
            }
            _ => false,
        }
    }
}

/// Keeps the watch alive; dropping it stops watching and ends the worker thread.
pub struct DirWatcher {
    _watcher: RecommendedWatcher,
}

/// Watches `dir` (non-recursive) and calls `on_change` (on a worker thread)
/// once per debounced burst of changes.
pub fn watch_dir(dir: &Path, on_change: impl Fn() + Send + 'static) -> notify::Result<DirWatcher> {
    let (tx, rx) = mpsc::channel::<()>();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        // Errors (e.g. overflow) also mean "something changed" → refresh.
        let relevant = res.map(|e| !e.kind.is_access()).unwrap_or(true);
        if relevant {
            let _ = tx.send(());
        }
    })?;
    watcher.watch(dir, RecursiveMode::NonRecursive)?;
    std::thread::Builder::new()
        .name("dir-watch".into())
        .spawn(move || {
            let mut deb = Debounce::new(Duration::from_millis(100), Duration::from_millis(500));
            loop {
                let msg = match deb.deadline() {
                    Some(d) => rx.recv_timeout(d.saturating_duration_since(Instant::now())),
                    None => rx.recv().map_err(|_| mpsc::RecvTimeoutError::Disconnected),
                };
                match msg {
                    Ok(()) => deb.event(Instant::now()),
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
                if deb.poll(Instant::now()) {
                    on_change();
                }
            }
        })
        .expect("spawning watch thread failed");
    Ok(DirWatcher { _watcher: watcher })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn debounce_quiet_period() {
        let t0 = Instant::now();
        let ms = Duration::from_millis;
        let mut d = Debounce::new(ms(100), ms(500));
        assert!(!d.poll(t0));
        d.event(t0);
        d.event(t0 + ms(50));
        assert!(!d.poll(t0 + ms(120)));
        assert!(d.poll(t0 + ms(150)));
        assert!(d.deadline().is_none());
    }

    #[test]
    fn debounce_max_wait_under_constant_events() {
        let t0 = Instant::now();
        let ms = Duration::from_millis;
        let mut d = Debounce::new(ms(100), ms(500));
        let mut fired_at = None;
        for i in 0..20 {
            let now = t0 + ms(i * 50);
            d.event(now);
            if d.poll(now) {
                fired_at = Some(i * 50);
                break;
            }
        }
        assert_eq!(fired_at, Some(500));
    }

    #[test]
    fn watcher_reports_new_file_once_per_burst() {
        let d = tempfile::tempdir().unwrap();
        let hits = Arc::new(AtomicUsize::new(0));
        let h = hits.clone();
        let _w = watch_dir(d.path(), move || {
            h.fetch_add(1, Ordering::SeqCst);
        })
        .unwrap();
        std::thread::sleep(Duration::from_millis(200));
        for i in 0..20 {
            std::fs::write(d.path().join(format!("n{i}.txt")), b"x").unwrap();
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while hits.load(Ordering::SeqCst) == 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        std::thread::sleep(Duration::from_millis(700));
        let n = hits.load(Ordering::SeqCst);
        assert!(
            (1..=3).contains(&n),
            "expected 1..=3 debounced callbacks, got {n}"
        );
    }
}
