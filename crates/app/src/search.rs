//! The search index service inside the app (PLAN-SEARCH.md S1.4a): started
//! in the background a moment after the window is up (never in the start
//! path), stopped – and saved – when the app ends.
//!
//! Overrides for tests: `EXPLORER2_INDEX=off` (never start),
//! `EXPLORER2_INDEX_ROOTS` (paths as in `PATH`, instead of the local
//! drives), `EXPLORER2_INDEX_DELAY_MS` (default 3000).

use crate::MainWindow;
use explorer_search::{Origin, Phase, Search, Status, Volume};
use slint::ComponentHandle;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

static SERVICE: Mutex<Option<Search>> = Mutex::new(None);
/// Exclusive lock on `index/lock` while this process keeps the index: a
/// second window (`--new-instance`) must not write the same files.
static LOCK: Mutex<Option<std::fs::File>> = Mutex::new(None);
/// Held while a stopped service saves: a new start waits for it (never two
/// services on the same files).
static STOP_GATE: Mutex<()> = Mutex::new(());
/// Bumped by every start/stop: a start that finishes after a newer
/// request is dropped (switched off while the drives were looked up).
static EPOCH: AtomicU64 = AtomicU64::new(0);
/// A UI update is queued already (changes come in bursts while building).
static UI_PENDING: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// Last traced phase per root (trace only on changes).
    static LAST: RefCell<HashMap<String, Phase>> = RefCell::new(HashMap::new());
}

fn lock() -> std::sync::MutexGuard<'static, Option<Search>> {
    SERVICE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Disabled for this process by the environment (tests).
fn env_off() -> bool {
    std::env::var("EXPLORER2_INDEX").is_ok_and(|v| v.eq_ignore_ascii_case("off"))
}

/// What gets indexed: the local fixed drives (Windows), the home folder
/// elsewhere. Blocking (drive lookup): call off the UI thread.
fn roots() -> Vec<PathBuf> {
    if let Some(v) = std::env::var_os("EXPLORER2_INDEX_ROOTS") {
        return std::env::split_paths(&v)
            .filter(|p| !p.as_os_str().is_empty())
            .collect();
    }
    if cfg!(windows) {
        explorer_fs::places::drives()
            .into_iter()
            .filter(|d| d.kind == explorer_fs::places::DriveKind::Fixed)
            .map(|d| d.root)
            .collect()
    } else {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .into_iter()
            .collect()
    }
}

/// Called once after the window is set up.
pub fn init(win: &MainWindow, enabled: bool) {
    win.set_opt_index_enabled(enabled);
    if env_off() {
        win.set_search_status(
            "Suchindex für diesen Start abgeschaltet (EXPLORER2_INDEX=off).".into(),
        );
        return;
    }
    if !enabled {
        win.set_search_status(off_text().into());
        return;
    }
    win.set_search_status("Suchindex startet gleich …".into());
    let delay = std::env::var("EXPLORER2_INDEX_DELAY_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);
    let weak = win.as_weak();
    slint::Timer::single_shot(Duration::from_millis(delay), move || {
        if let Some(w) = weak.upgrade() {
            start(&w);
        }
    });
}

fn off_text() -> &'static str {
    "Suchindex aus: die Suche durchsucht dann nur den aktuellen Ordner."
}

fn start(win: &MainWindow) {
    let epoch = EPOCH.fetch_add(1, Ordering::SeqCst) + 1;
    // a new service: its phases are traced from the start again
    LAST.with_borrow_mut(|m| m.clear());
    let settings = crate::config::load();
    let Some(dir) = crate::config::config_dir().map(|d| d.join("index")) else {
        win.set_search_status("Suchindex: kein Ordner für die Indexdateien.".into());
        return;
    };
    let exclusions =
        explorer_index::Exclusions::with_defaults(&settings.index_exclude, cfg!(windows));
    let weak = win.as_weak();
    let spawned = std::thread::Builder::new()
        .name("index-start".into())
        .spawn(move || {
            let _gate = STOP_GATE.lock().unwrap_or_else(|e| e.into_inner());
            if !take_lock(&dir) {
                crate::nav::trace_line("search: index kept by another window");
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak.upgrade() {
                        w.set_search_status(
                            "Der Suchindex wird von einem anderen Explorer-2.0-Fenster gepflegt."
                                .into(),
                        );
                    }
                });
                return;
            }
            let roots = roots();
            crate::nav::trace_line(&format!("search: start roots={roots:?}"));
            let cfg = explorer_search::Config::new(dir, exclusions);
            let w = weak.clone();
            let service = Search::start(&roots, cfg, move |_| notify_ui(w.clone()));
            let mut g = lock();
            if EPOCH.load(Ordering::SeqCst) != epoch {
                drop(g);
                service.shutdown(); // switched off meanwhile
                release_lock();
                return;
            }
            if let Some(old) = g.replace(service) {
                drop(g);
                old.shutdown();
            }
            notify_ui(weak);
        });
    if spawned.is_err() {
        win.set_search_status("Suchindex konnte nicht starten.".into());
    }
}

/// Takes the index lock (true: this process keeps the index).
fn take_lock(dir: &std::path::Path) -> bool {
    let mut g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if g.is_some() {
        return true;
    }
    let _ = std::fs::create_dir_all(dir);
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join("lock"));
    match file {
        Ok(f) if f.try_lock().is_ok() => {
            *g = Some(f);
            true
        }
        _ => false,
    }
}

fn release_lock() {
    LOCK.lock().unwrap_or_else(|e| e.into_inner()).take();
}

/// Switches the index on or off (setting already saved by the caller).
pub fn set_enabled(win: &MainWindow, on: bool) {
    win.set_opt_index_enabled(on);
    if env_off() {
        return;
    }
    if on {
        if lock().is_none() {
            win.set_search_status("Suchindex startet …".into());
            start(win);
        }
    } else {
        EPOCH.fetch_add(1, Ordering::SeqCst);
        let old = lock().take();
        win.set_search_status(off_text().into());
        if let Some(s) = old {
            // stopping saves the index: not on the UI thread
            let _ = std::thread::Builder::new()
                .name("index-stop".into())
                .spawn(move || {
                    let _gate = STOP_GATE.lock().unwrap_or_else(|e| e.into_inner());
                    s.shutdown();
                    release_lock();
                });
        } else {
            release_lock();
        }
    }
}

/// Compare every indexed drive with the disk again.
pub fn rescan(win: &MainWindow) {
    let n = with_volumes(|v| {
        for vol in v {
            vol.rescan();
        }
        v.len()
    });
    crate::nav::trace_line(&format!("search: rescan volumes={n}"));
    if n == 0 {
        win.set_search_status(
            if win.get_opt_index_enabled() {
                "Suchindex läuft noch nicht."
            } else {
                off_text()
            }
            .into(),
        );
    }
}

/// The indexed drives (empty while off or starting).
pub fn with_volumes<R>(f: impl FnOnce(&[Arc<Volume>]) -> R) -> R {
    let g = lock();
    match g.as_ref() {
        Some(s) => f(s.volumes()),
        None => f(&[]),
    }
}

/// End of the app: stop every maintainer and wait for the saves.
pub fn shutdown() {
    EPOCH.fetch_add(1, Ordering::SeqCst);
    let old = lock().take();
    if let Some(s) = old {
        let t0 = std::time::Instant::now();
        s.shutdown();
        crate::nav::trace_line(&format!(
            "search: stopped + saved in {:.0}ms",
            t0.elapsed().as_secs_f64() * 1000.0
        ));
    }
    release_lock();
}

/// From the maintainer threads: one UI update per 250 ms at most.
fn notify_ui(weak: slint::Weak<MainWindow>) {
    if UI_PENDING.swap(true, Ordering::SeqCst) {
        return;
    }
    let _ = slint::invoke_from_event_loop(move || {
        slint::Timer::single_shot(Duration::from_millis(250), move || {
            UI_PENDING.store(false, Ordering::SeqCst);
            if let Some(w) = weak.upgrade() {
                update_ui(&w);
            }
        });
    });
}

fn update_ui(win: &MainWindow) {
    let lines: Vec<(String, Status)> = with_volumes(|v| {
        v.iter()
            .map(|vol| (vol.root().to_string_lossy().into_owned(), vol.status()))
            .collect()
    });
    for (root, st) in &lines {
        let changed = LAST.with_borrow_mut(|m| m.insert(root.clone(), st.phase) != Some(st.phase));
        if changed {
            crate::nav::trace_line(&format!(
                "search: {root} phase={:?} entries={} origin={} live={}",
                st.phase,
                st.entries,
                match &st.origin {
                    Some(Origin::Loaded { ms }) => format!("loaded({ms}ms)"),
                    Some(Origin::Built { ms, .. }) => format!("built({ms}ms)"),
                    None => "-".into(),
                },
                st.live
            ));
        }
    }
    if !lines.is_empty() {
        let text: Vec<String> = lines.iter().map(|(r, s)| status_line(r, s)).collect();
        win.set_search_status(text.join("\n").into());
    }
    // shown results follow the index (new, renamed, deleted files)
    crate::nav::search_index_changed(win);
}

/// At most this many results are listed (sorted first, so the best ones):
/// more is never useful in a list and costs time + memory.
pub const MAX_HITS: usize = 20_000;

/// Results of one search.
pub struct Found {
    /// (entry, full path), sorted as asked, at most [`MAX_HITS`].
    pub items: Vec<(explorer_core::Entry, PathBuf)>,
    /// All matches (may exceed `items`).
    pub total: usize,
    /// The folder was walked (no index for it yet, or the index is off).
    pub walked: bool,
}

fn sort_by(key: explorer_core::SortKey) -> explorer_index::SortBy {
    use explorer_core::SortKey as K;
    use explorer_index::SortBy as B;
    match key {
        K::Size => B::Size,
        K::Modified => B::Modified,
        // "Typ" is the folder column in results: sorted afterwards
        K::Name | K::Type => B::Name,
    }
}

fn entry_of(ix: &explorer_index::Index, id: u32) -> (explorer_core::Entry, PathBuf) {
    let m = ix.meta(id);
    let e = explorer_core::Entry {
        name: ix.name(id).to_string(),
        kind: if m.is_dir {
            explorer_core::EntryKind::Dir
        } else {
            explorer_core::EntryKind::File
        },
        size: m.size,
        modified: (m.mtime != 0).then_some(m.mtime),
        hidden: false,
        system: false,
        readonly: false,
        is_link: false,
    };
    (e, PathBuf::from(ix.path(id)))
}

/// Searches `text` below `scope` (None: everything indexed). Uses the index
/// where it covers the scope; a folder without a ready index is walked (the
/// same query language, just slower). Blocking: call off the UI thread.
pub fn find(
    scope: Option<&std::path::Path>,
    text: &str,
    sort: explorer_core::SortSpec,
    cancel: &AtomicBool,
) -> Result<Found, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    // dates in the list are shown in UTC as well (same "today")
    let q = explorer_index::Query::parse(
        text,
        explorer_index::Now {
            unix: now,
            utc_offset: 0,
        },
    )
    .map_err(|e| format!("Suche: {e}"))?;
    let by = sort_by(sort.key);
    let vols: Vec<Arc<Volume>> = with_volumes(|v| v.to_vec());
    let vols: Vec<Arc<Volume>> = vols
        .into_iter()
        .filter(|v| v.covers(scope) && v.searchable())
        .collect();
    // `vols` = ready indexes that contain the scope or lie inside it ("/",
    // "everything"). None (index off, still building, a folder on a drive
    // that is not indexed): the folder is walked instead
    let in_index = scope.is_none() || !vols.is_empty();
    let mut items = Vec::new();
    let mut total = 0;
    let mut walked = false;
    if in_index {
        for v in &vols {
            v.search_with(&q, scope, by, sort.descending, |ix, ids| {
                total += ids.len();
                items.extend(ids.iter().take(MAX_HITS).map(|&id| entry_of(ix, id)));
            });
        }
    } else if let Some(s) = scope {
        // no index here (off, still building, not an indexed drive): walk it
        let settings = crate::config::load();
        let ex = explorer_index::Exclusions::with_defaults(&settings.index_exclude, cfg!(windows));
        let mut w = explorer_search::walk::Walker::new(&ex);
        w.cancel = Some(cancel);
        let (ix, rep) = explorer_search::walk::build(s, &w);
        if rep.cancelled {
            return Err("abgebrochen".into());
        }
        walked = true;
        let mut ids = ix.search(&q, None, by, sort.descending);
        ids.retain(|&id| ix.parent(id) != explorer_index::NONE);
        total = ids.len();
        items.extend(ids.iter().take(MAX_HITS).map(|&id| entry_of(&ix, id)));
    }
    // several drives, or the folder column: one order for all
    if vols.len() > 1 || sort.key == explorer_core::SortKey::Type {
        sort_found(&mut items, sort);
    }
    items.truncate(MAX_HITS);
    Ok(Found {
        items,
        total,
        walked,
    })
}

/// Sorts results like the list sorts a folder; "Typ" = the folder column.
pub(crate) fn sort_found(
    items: &mut [(explorer_core::Entry, PathBuf)],
    sort: explorer_core::SortSpec,
) {
    use explorer_core::SortKey as K;
    let folder = |p: &PathBuf| {
        p.parent()
            .map(|d| d.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    };
    items.sort_by(|a, b| {
        let o = match sort.key {
            K::Name => explorer_index::natural(&a.0.name, &b.0.name),
            K::Size => a.0.size.cmp(&b.0.size),
            K::Modified => a.0.modified.cmp(&b.0.modified),
            K::Type => folder(&a.1)
                .cmp(&folder(&b.1))
                .then_with(|| explorer_index::natural(&a.0.name, &b.0.name)),
        };
        if sort.descending {
            o.reverse()
        } else {
            o
        }
    });
}

/// One line of the index status (settings panel).
pub(crate) fn status_line(root: &str, st: &Status) -> String {
    let n = explorer_core::format_count(st.entries);
    let mut s = match st.phase {
        Phase::Waiting | Phase::Loading => format!("{root}: wird geladen …"),
        Phase::Building => format!("{root}: wird aufgebaut … {n} Einträge"),
        Phase::Refreshing => format!("{root}: {n} Einträge, wird mit der Platte abgeglichen …"),
        Phase::Ready => {
            let how = match &st.origin {
                Some(Origin::Built { ms, .. }) => format!(" (aufgebaut in {})", secs(*ms)),
                _ => String::new(),
            };
            format!("{root}: bereit, {n} Einträge{how}")
        }
        Phase::Stopped => format!("{root}: angehalten"),
    };
    if !st.live && st.phase != Phase::Stopped {
        s.push_str(" – ohne Live-Aktualisierung");
        if let Some(e) = &st.watch_error {
            s.push_str(&format!(" ({e})"));
        }
    }
    if let Some(e) = &st.save_error {
        s.push_str(&format!(" – Speichern fehlgeschlagen: {e}"));
    }
    s
}

fn secs(ms: u64) -> String {
    if ms < 10_000 {
        format!("{:.1} s", ms as f64 / 1000.0).replace('.', ",")
    } else if ms < 120_000 {
        format!("{} s", ms / 1000)
    } else {
        format!("{} min", ms / 60_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st(phase: Phase, entries: usize) -> Status {
        Status {
            phase,
            entries,
            live: true,
            ..Status::default()
        }
    }

    #[test]
    fn results_sort_like_a_folder_and_by_folder() {
        use explorer_core::{Entry, EntryKind, SortKey, SortSpec};
        let it = |name: &str, dir: &str, size: u64, m: i64| {
            (
                Entry {
                    name: name.into(),
                    kind: EntryKind::File,
                    size,
                    modified: Some(m),
                    hidden: false,
                    system: false,
                    readonly: false,
                    is_link: false,
                },
                PathBuf::from(dir).join(name),
            )
        };
        let mut v = vec![
            it("b10.txt", "/z", 5, 3),
            it("b2.txt", "/a", 9, 1),
            it("a.txt", "/m", 1, 2),
        ];
        let spec = |key, descending| SortSpec {
            key,
            descending,
            dirs_first: true,
        };
        let names = |v: &[(explorer_core::Entry, PathBuf)]| -> Vec<String> {
            v.iter().map(|x| x.0.name.clone()).collect()
        };
        sort_found(&mut v, spec(SortKey::Name, false));
        assert_eq!(names(&v), ["a.txt", "b2.txt", "b10.txt"], "natural order");
        sort_found(&mut v, spec(SortKey::Size, true));
        assert_eq!(names(&v), ["b2.txt", "b10.txt", "a.txt"]);
        sort_found(&mut v, spec(SortKey::Type, false));
        assert_eq!(
            names(&v),
            ["b2.txt", "a.txt", "b10.txt"],
            "by folder /a /m /z"
        );
        sort_found(&mut v, spec(SortKey::Modified, false));
        assert_eq!(names(&v), ["b2.txt", "a.txt", "b10.txt"]);
    }

    #[test]
    fn status_lines() {
        assert_eq!(
            status_line("C:\\", &st(Phase::Building, 120_000)),
            "C:\\: wird aufgebaut … 120.000 Einträge"
        );
        let mut s = st(Phase::Ready, 1_012_345);
        s.origin = Some(Origin::Built {
            why: "kein gespeicherter Index".into(),
            ms: 3_400,
        });
        assert_eq!(
            status_line("C:\\", &s),
            "C:\\: bereit, 1.012.345 Einträge (aufgebaut in 3,4 s)"
        );
        s.origin = Some(Origin::Loaded { ms: 58 });
        s.live = false;
        s.watch_error = Some("zu viele Überwachungen".into());
        assert_eq!(
            status_line("/home/a", &s),
            "/home/a: bereit, 1.012.345 Einträge – ohne Live-Aktualisierung (zu viele Überwachungen)"
        );
        let mut s = st(Phase::Refreshing, 5);
        s.save_error = Some("voll".into());
        assert_eq!(
            status_line("D:\\", &s),
            "D:\\: 5 Einträge, wird mit der Platte abgeglichen … – Speichern fehlgeschlagen: voll"
        );
        assert_eq!(secs(95_000), "95 s");
        assert_eq!(secs(600_000), "10 min");
    }
}
