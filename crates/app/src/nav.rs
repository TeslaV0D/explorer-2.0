//! Navigation controller. All state lives on the UI thread; directory loading,
//! filtering, sorting and row formatting run on background threads and are
//! applied via `invoke_from_event_loop`, guarded by a generation counter so a
//! stale result can never overwrite a newer navigation.

use crate::{MainWindow, Row};
use explorer_core::{
    format_count, format_datetime, format_size, sort_entries, Entry, SortSpec, ViewFilter,
};
use explorer_fs::{load_dir, watch_dir, DirWatcher, FsError, FsErrorKind, LoadEvent, LoadHandle};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::cell::RefCell;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// User navigation: shows progress, first batch paints immediately, scrolls to top.
    User,
    /// Watcher/settings refresh of the current folder: silent, keeps scroll position.
    Refresh,
}

struct State {
    /// Folder currently displayed.
    path: Option<PathBuf>,
    /// Displayed (filtered + sorted) entries, index-aligned with the row model.
    entries: Vec<Entry>,
    gen: u64,
    loader: Option<LoadHandle>,
    watcher: Option<DirWatcher>,
    filter: ViewFilter,
    sort: SortSpec,
    model: Rc<VecModel<Row>>,
    /// Earliest time the next watcher refresh may run (adaptive throttle).
    refresh_allowed_at: Instant,
    refresh_scheduled: bool,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
        path: None,
        entries: Vec::new(),
        gen: 0,
        loader: None,
        watcher: None,
        filter: ViewFilter::default(),
        sort: SortSpec::default(),
        model: Rc::new(VecModel::default()),
        refresh_allowed_at: Instant::now(),
        refresh_scheduled: false,
    });
}

/// Minimum gap between two watcher refreshes: at least 500 ms, and 4x the cost
/// of the last load (a 300 ms folder refreshes at most every 1.2 s), capped at 10 s.
/// Prevents a constantly written file (logs, downloads) from pinning a CPU core.
pub fn refresh_cooldown(last_load: Duration) -> Duration {
    (last_load * 4).clamp(Duration::from_millis(500), Duration::from_secs(10))
}

/// "1 Element" / "3 Elemente" with German thousands separator.
pub fn count_label(n: usize) -> String {
    if n == 1 {
        "1 Element".to_string()
    } else {
        format!("{} Elemente", format_count(n))
    }
}

static PROCESS_START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

/// Call first thing in `main` so traces can report time since process start.
pub fn mark_process_start() {
    PROCESS_START.get_or_init(Instant::now);
}

/// Milliseconds since process start (for `at=` trace fields).
pub fn since_start_ms() -> f64 {
    PROCESS_START
        .get_or_init(Instant::now)
        .elapsed()
        .as_secs_f64()
        * 1000.0
}

fn trace_enabled() -> bool {
    std::env::var_os("EXPLORER2_TRACE").is_some()
}

pub fn trace_line(msg: &str) {
    trace(msg);
}

fn trace(msg: &str) {
    if trace_enabled() {
        let _ = writeln!(std::io::stderr(), "[trace] {msg}");
    }
}

/// Strips whitespace and surrounding quotes from pasted paths ("C:\x y").
pub fn clean_input(s: &str) -> String {
    let t = s.trim();
    let t = t
        .strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .unwrap_or(t);
    t.trim().to_string()
}

pub fn init(win: &MainWindow) {
    let model = STATE.with_borrow(|s| s.model.clone());
    win.set_rows(ModelRc::from(model));
    win.set_show_hidden(STATE.with_borrow(|s| s.filter.show_hidden));
}

enum Update {
    Rows {
        entries: Vec<Entry>,
        rows: Vec<Row>,
        hidden: usize,
        done: Option<(usize, Duration)>,
        /// navigate -> data prepared, measured on the loader thread (independent
        /// of when the UI thread gets to apply it, e.g. during renderer init).
        data_ready: Duration,
    },
    Failed(FsError),
}

fn to_row(e: &Entry) -> Row {
    Row {
        name: e.name.as_str().into(),
        size: if e.is_dir() {
            Default::default()
        } else {
            format_size(e.size).into()
        },
        modified: e.modified.map(format_datetime).unwrap_or_default().into(),
        is_dir: e.is_dir(),
        dim: e.hidden,
    }
}

/// Filter + sort + format (runs on the loader thread).
fn prepare(all: Vec<Entry>, filter: ViewFilter, sort: SortSpec) -> (Vec<Entry>, Vec<Row>, usize) {
    let total = all.len();
    let mut vis: Vec<Entry> = all.into_iter().filter(|e| filter.is_visible(e)).collect();
    let hidden = total - vis.len();
    sort_entries(&mut vis, sort);
    let rows = vis.iter().map(to_row).collect();
    (vis, rows, hidden)
}

pub fn navigate(win: &MainWindow, target: PathBuf, reason: Reason) {
    let target = std::path::absolute(&target).unwrap_or(target);
    let (gen, filter, sort) = STATE.with_borrow_mut(|s| {
        s.gen += 1;
        (s.gen, s.filter, s.sort)
    });
    if reason == Reason::User {
        win.set_status("Lädt …".into());
        win.set_error("".into());
    }
    trace(&format!(
        "navigate gen={gen} {reason:?} {} at={:.1}ms",
        target.display(),
        since_start_ms()
    ));
    let weak = win.as_weak();
    let path = target.clone();
    let t0 = Instant::now();
    let mut acc: Vec<Entry> = Vec::new();
    let handle = load_dir(target, move |ev| {
        let upd = match ev {
            LoadEvent::Batch { entries, first } => {
                acc.extend(entries);
                if !(first && reason == Reason::User) {
                    return; // only the first batch is painted early; refreshes never flash partial lists
                }
                let (entries, rows, hidden) = prepare(acc.clone(), filter, sort);
                Update::Rows {
                    entries,
                    rows,
                    hidden,
                    done: None,
                    data_ready: t0.elapsed(),
                }
            }
            LoadEvent::Done { total, .. } => {
                let (entries, rows, hidden) = prepare(std::mem::take(&mut acc), filter, sort);
                Update::Rows {
                    entries,
                    rows,
                    hidden,
                    done: Some((total, t0.elapsed())),
                    data_ready: t0.elapsed(),
                }
            }
            LoadEvent::Failed(e) => Update::Failed(e),
        };
        let (weak, path) = (weak.clone(), path.clone());
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w) = weak.upgrade() {
                apply(&w, gen, path, reason, upd, t0);
            }
        });
    });
    STATE.with_borrow_mut(|s| s.loader = Some(handle)); // drops (= cancels) the previous load
}

fn apply(win: &MainWindow, gen: u64, path: PathBuf, reason: Reason, upd: Update, t0: Instant) {
    if STATE.with_borrow(|s| s.gen) != gen {
        return; // stale
    }
    match upd {
        Update::Rows {
            entries,
            rows,
            hidden,
            done,
            data_ready,
        } => {
            let changed = STATE.with_borrow(|s| s.path.as_deref() != Some(path.as_path()));
            let n = entries.len();
            STATE.with_borrow_mut(|s| {
                s.entries = entries;
                s.model.set_vec(rows);
            });
            if changed {
                win.set_current_path(path.to_string_lossy().as_ref().into());
                win.invoke_scroll_top();
                start_watch(win, &path);
                STATE.with_borrow_mut(|s| s.path = Some(path));
            }
            match done {
                None => {
                    win.set_status(format!("Lädt … {}", format_count(n)).into());
                    trace(&format!(
                        "first paint gen={gen} rows={n} after {:?} data_ready={:.1}ms at={:.1}ms",
                        t0.elapsed(),
                        data_ready.as_secs_f64() * 1000.0,
                        since_start_ms()
                    ));
                }
                Some((total, took)) => {
                    STATE.with_borrow_mut(|s| {
                        s.refresh_allowed_at = Instant::now() + refresh_cooldown(took)
                    });
                    let mut st = count_label(n);
                    if hidden > 0 {
                        st.push_str(&format!(" ({} ausgeblendet)", format_count(hidden)));
                    }
                    if reason == Reason::User {
                        st.push_str(&format!(" · {} ms", took.as_millis()));
                    }
                    win.set_status(st.into());
                    trace(&format!(
                        "done gen={gen} total={total} shown={n} after {took:?} data_ready={:.1}ms at={:.1}ms",
                        data_ready.as_secs_f64() * 1000.0,
                        since_start_ms()
                    ));
                    trace(&format!("address={}", win.get_current_path()));
                }
            }
        }
        Update::Failed(e) => {
            trace(&format!("failed gen={gen} {:?} {}", e.kind, path.display()));
            let current = STATE.with_borrow(|s| s.path.clone());
            let vanished =
                e.kind == FsErrorKind::NotFound && current.as_deref() == Some(path.as_path());
            if vanished {
                // Current folder was deleted/renamed → go to the nearest existing parent.
                if let Some(up) = explorer_fs::nearest_existing_ancestor(&path) {
                    navigate(win, up, Reason::User);
                    win.set_error(
                        format!("Ordner existiert nicht mehr: {}", path.display()).into(),
                    );
                    return;
                }
            }
            win.set_error(format!("{}: {}", e.message, path.display()).into());
            win.set_status("".into());
            // Keep the previous listing; restore the address bar text.
            if let Some(cur) = current {
                win.set_current_path(cur.to_string_lossy().as_ref().into());
                restore_status(win);
                trace(&format!("address={}", win.get_current_path()));
            }
        }
    }
}

fn restore_status(win: &MainWindow) {
    let n = STATE.with_borrow(|s| s.model.row_count());
    win.set_status(count_label(n).into());
}

fn start_watch(win: &MainWindow, path: &Path) {
    let weak = win.as_weak();
    let watched = path.to_path_buf();
    let w = watch_dir(path, move || {
        let (weak, watched) = (weak.clone(), watched.clone());
        let _ = slint::invoke_from_event_loop(move || request_refresh(weak, watched));
    });
    if let Err(e) = &w {
        trace(&format!("watch failed (auto-refresh off): {e}"));
    }
    STATE.with_borrow_mut(|s| s.watcher = w.ok()); // drops the old watcher
}

/// Watcher fired: refresh now, or once after the cooldown (coalescing all events in between).
fn request_refresh(weak: slint::Weak<MainWindow>, watched: PathBuf) {
    let still_here = STATE.with_borrow(|s| s.path.as_deref() == Some(watched.as_path()));
    if !still_here {
        return;
    }
    let (allowed_at, scheduled) =
        STATE.with_borrow(|s| (s.refresh_allowed_at, s.refresh_scheduled));
    if scheduled {
        return;
    }
    let now = Instant::now();
    if now >= allowed_at {
        if let Some(w) = weak.upgrade() {
            trace("watch: refresh");
            navigate(&w, watched, Reason::Refresh);
        }
    } else {
        STATE.with_borrow_mut(|s| s.refresh_scheduled = true);
        trace(&format!(
            "watch: refresh throttled for {:?}",
            allowed_at - now
        ));
        slint::Timer::single_shot(allowed_at - now, move || {
            STATE.with_borrow_mut(|s| s.refresh_scheduled = false);
            request_refresh(weak, watched);
        });
    }
}

pub fn go_up(win: &MainWindow) {
    let parent = STATE.with_borrow(|s| {
        s.path
            .as_ref()
            .and_then(|p| p.parent())
            .map(Path::to_path_buf)
    });
    if let Some(p) = parent {
        navigate(win, p, Reason::User);
    }
}

pub fn open_row(win: &MainWindow, index: usize) {
    let target = STATE.with_borrow(|s| {
        let e = s.entries.get(index).filter(|e| e.is_dir())?;
        Some(s.path.as_ref()?.join(&e.name))
    });
    if let Some(p) = target {
        navigate(win, p, Reason::User);
    }
}

pub fn set_show_hidden(win: &MainWindow, on: bool) {
    let cur = STATE.with_borrow_mut(|s| {
        s.filter.show_hidden = on;
        s.path.clone()
    });
    if let Some(p) = cur {
        navigate(win, p, Reason::Refresh);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use explorer_core::EntryKind;

    fn e(name: &str, dir: bool, hidden: bool) -> Entry {
        Entry {
            name: name.into(),
            kind: if dir { EntryKind::Dir } else { EntryKind::File },
            size: 2048,
            modified: Some(0),
            hidden,
            system: false,
            readonly: false,
            is_link: false,
        }
    }

    #[test]
    fn cooldown_is_adaptive_and_bounded() {
        let ms = Duration::from_millis;
        assert_eq!(refresh_cooldown(ms(0)), ms(500));
        assert_eq!(refresh_cooldown(ms(100)), ms(500));
        assert_eq!(refresh_cooldown(ms(300)), ms(1200));
        assert_eq!(
            refresh_cooldown(Duration::from_secs(60)),
            Duration::from_secs(10)
        );
    }

    #[test]
    fn count_labels() {
        assert_eq!(count_label(0), "0 Elemente");
        assert_eq!(count_label(1), "1 Element");
        assert_eq!(count_label(100_000), "100.000 Elemente");
    }

    #[test]
    fn clean_input_strips_quotes_and_space() {
        assert_eq!(
            clean_input("  \"C:\\Program Files\"  "),
            "C:\\Program Files"
        );
        assert_eq!(clean_input("/tmp/x"), "/tmp/x");
        assert_eq!(clean_input("\""), "\"");
    }

    #[test]
    fn prepare_filters_sorts_and_formats() {
        let all = vec![
            e("b.txt", false, false),
            e(".h", false, true),
            e("dir", true, false),
        ];
        let (vis, rows, hidden) = prepare(all.clone(), ViewFilter::default(), SortSpec::default());
        assert_eq!(hidden, 1);
        assert_eq!(
            vis.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
            ["dir", "b.txt"]
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].size, "");
        assert_eq!(rows[1].size, "2 KB");
        assert_eq!(rows[1].modified, "1970-01-01 00:00");

        let show = ViewFilter {
            show_hidden: true,
            show_protected_os: false,
        };
        let (vis, rows, hidden) = prepare(all, show, SortSpec::default());
        assert_eq!((vis.len(), hidden), (3, 0));
        assert!(rows.iter().any(|r| r.dim));
    }
}
