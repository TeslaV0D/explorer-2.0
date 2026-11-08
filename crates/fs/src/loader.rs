//! Background directory loading with cancellation and batched delivery.

use crate::{enumerate, FsError};
use explorer_core::Entry;
use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// First batch is small so the UI can paint immediately; later ones are large.
pub const FIRST_BATCH: usize = 256;
pub const NEXT_BATCH: usize = 8192;

#[derive(Debug)]
pub enum LoadEvent {
    /// More entries (unsorted). `first` marks the first batch of a load.
    Batch {
        entries: Vec<Entry>,
        first: bool,
    },
    Done {
        total: usize,
        elapsed: Duration,
    },
    Failed(FsError),
}

/// Dropping the handle cancels the load (without blocking the caller).
pub struct LoadHandle {
    cancel: Arc<AtomicBool>,
}

impl LoadHandle {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Drop for LoadHandle {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Loads `dir` on a background thread, streaming events to `sink` (called on
/// that thread). After cancellation no further events are sent.
pub fn load_dir(dir: PathBuf, mut sink: impl FnMut(LoadEvent) + Send + 'static) -> LoadHandle {
    let cancel = Arc::new(AtomicBool::new(false));
    let c = cancel.clone();
    std::thread::Builder::new()
        .name("dir-loader".into())
        .spawn(move || {
            let start = Instant::now();
            let mut buf = Vec::with_capacity(FIRST_BATCH);
            let mut first = true;
            let mut total = 0usize;
            let res = enumerate(&dir, &mut |e| {
                if c.load(Ordering::Relaxed) {
                    return ControlFlow::Break(());
                }
                buf.push(e);
                total += 1;
                let limit = if first { FIRST_BATCH } else { NEXT_BATCH };
                if buf.len() >= limit {
                    sink(LoadEvent::Batch {
                        entries: std::mem::take(&mut buf),
                        first,
                    });
                    first = false;
                }
                ControlFlow::Continue(())
            });
            if c.load(Ordering::Relaxed) {
                return;
            }
            match res {
                Ok(()) => {
                    if !buf.is_empty() {
                        sink(LoadEvent::Batch {
                            entries: buf,
                            first,
                        });
                    }
                    sink(LoadEvent::Done {
                        total,
                        elapsed: start.elapsed(),
                    });
                }
                Err(e) => sink(LoadEvent::Failed(e)),
            }
        })
        .expect("spawning loader thread failed");
    LoadHandle { cancel }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn collect(dir: PathBuf) -> Vec<LoadEvent> {
        let (tx, rx) = mpsc::channel();
        let _h = load_dir(dir, move |e| {
            let _ = tx.send(e);
        });
        let mut out = Vec::new();
        while let Ok(e) = rx.recv_timeout(Duration::from_secs(10)) {
            let end = !matches!(e, LoadEvent::Batch { .. });
            out.push(e);
            if end {
                break;
            }
        }
        out
    }

    #[test]
    fn batches_then_done() {
        let d = tempfile::tempdir().unwrap();
        let n = FIRST_BATCH + 100;
        for i in 0..n {
            std::fs::write(d.path().join(format!("{i}")), b"").unwrap();
        }
        let ev = collect(d.path().to_path_buf());
        let mut got = 0;
        for (i, e) in ev.iter().enumerate() {
            match e {
                LoadEvent::Batch { entries, first } => {
                    assert_eq!(*first, i == 0);
                    if i == 0 {
                        assert_eq!(entries.len(), FIRST_BATCH);
                    }
                    got += entries.len();
                }
                LoadEvent::Done { total, .. } => assert_eq!(*total, n),
                LoadEvent::Failed(e) => panic!("{e}"),
            }
        }
        assert_eq!(got, n);
        assert!(matches!(ev.last(), Some(LoadEvent::Done { .. })));
    }

    #[test]
    fn empty_dir_sends_only_done() {
        let d = tempfile::tempdir().unwrap();
        let ev = collect(d.path().to_path_buf());
        assert!(matches!(ev.as_slice(), [LoadEvent::Done { total: 0, .. }]));
    }

    #[test]
    fn missing_dir_fails() {
        let d = tempfile::tempdir().unwrap();
        let ev = collect(d.path().join("gone"));
        assert!(
            matches!(ev.as_slice(), [LoadEvent::Failed(e)] if e.kind == crate::FsErrorKind::NotFound)
        );
    }

    #[test]
    fn cancelled_load_sends_nothing_more() {
        let d = tempfile::tempdir().unwrap();
        for i in 0..2000 {
            std::fs::write(d.path().join(format!("{i}")), b"").unwrap();
        }
        let (tx, rx) = mpsc::channel();
        let h = load_dir(d.path().to_path_buf(), move |e| {
            let _ = tx.send(matches!(e, LoadEvent::Done { .. }));
        });
        h.cancel();
        drop(h);
        // Either it finished before cancel was seen, or nothing (more) arrives.
        let events: Vec<bool> = rx.iter().collect(); // ends when thread drops tx
        assert!(events.len() <= 2000 / FIRST_BATCH + 2);
    }
}
