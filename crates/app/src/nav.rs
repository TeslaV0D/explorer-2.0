//! Window controller: panes (1 or 2, split view) → tabs → one folder each.
//!
//! All state lives on the UI thread. Directory loading, filtering, sorting and
//! row formatting run on background threads; results are routed back by a
//! stable **tab id** (tabs can be closed or moved meanwhile) and guarded by a
//! generation counter so a stale result never overwrites a newer navigation.

use crate::keys::{map_key, KeyAction};
use crate::{Crumb, MainWindow, PaneData, PlaceItem, Row, TabInfo};
use explorer_core::breadcrumb::{crumbs, fit};
use explorer_core::history::History;
use explorer_core::selection::Selection;
use explorer_core::session::{PaneSession, Session};
use explorer_core::typeahead::TypeAhead;
use explorer_core::{
    format_count, format_datetime, format_size, format_usage, sort_entries, type_label, Entry,
    SortKey, SortSpec, ViewFilter,
};
use explorer_fs::places::{drive_display_name, used_fraction};
use explorer_fs::{load_dir, watch_dir, DirWatcher, FsError, FsErrorKind, LoadEvent, LoadHandle};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// User navigation: shows progress, first batch paints immediately, scrolls to top.
    User,
    /// Watcher/settings/sort refresh of the current folder: silent, keeps scroll + selection.
    Refresh,
}

/// How a user navigation affects the tab's history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hist {
    Push,
    Keep,
}

type TabId = u64;

struct Tab {
    id: TabId,
    /// Folder displayed (or to be displayed once the tab gets loaded).
    path: Option<PathBuf>,
    /// Displayed (filtered + sorted) entries, index-aligned with the row model.
    entries: Vec<Entry>,
    model: Rc<VecModel<Row>>,
    crumbs: Rc<VecModel<Crumb>>,
    selection: Selection,
    gen: u64,
    loader: Option<LoadHandle>,
    watcher: Option<DirWatcher>,
    history: History,
    /// Name to select once the target folder is loaded (folder we came from on "up").
    pending_select: Option<String>,
    status_base: String,
    /// Needs a reload when it becomes visible (never loaded, or sort/filter changed).
    stale: bool,
    refresh_allowed_at: Instant,
    refresh_scheduled: bool,
}

impl Tab {
    fn new(id: TabId, path: Option<PathBuf>) -> Self {
        Tab {
            id,
            path,
            entries: Vec::new(),
            model: Rc::new(VecModel::default()),
            crumbs: Rc::new(VecModel::default()),
            selection: Selection::new(0),
            gen: 0,
            loader: None,
            watcher: None,
            history: History::default(),
            pending_select: None,
            status_base: String::new(),
            stale: true,
            refresh_allowed_at: Instant::now(),
            refresh_scheduled: false,
        }
    }

    fn title(&self) -> String {
        match &self.path {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.to_string_lossy().into_owned()),
            None => "…".to_string(),
        }
    }
}

struct Pane {
    tabs: Vec<Tab>,
    active: usize,
    /// One-shot UI requests (see PaneData in main.slint).
    scroll_to: i32,
    scroll_seq: i32,
    top_seq: i32,
    edit_seq: i32,
    focus_seq: i32,
    /// Breadcrumb budget in characters, from the address bar width.
    crumb_chars: usize,
}

impl Pane {
    fn new(tab: Tab) -> Self {
        Pane {
            tabs: vec![tab],
            active: 0,
            scroll_to: 0,
            scroll_seq: 0,
            top_seq: 0,
            edit_seq: 0,
            focus_seq: 0,
            crumb_chars: 60,
        }
    }

    fn tab(&self) -> &Tab {
        &self.tabs[self.active]
    }
}

struct App {
    panes: Vec<Pane>,
    active_pane: usize,
    filter: ViewFilter,
    sort: SortSpec,
    next_tab_id: TabId,
    next_gen: u64,
    pane_model: Rc<VecModel<PaneData>>,
    place_model: Rc<VecModel<PlaceItem>>,
    drive_model: Rc<VecModel<PlaceItem>>,
    session_save_scheduled: bool,
    drives_loading: bool,
}

thread_local! {
    static APP: RefCell<App> = RefCell::new(App {
        panes: Vec::new(),
        active_pane: 0,
        filter: ViewFilter::default(),
        sort: SortSpec::default(),
        next_tab_id: 1,
        next_gen: 1,
        pane_model: Rc::new(VecModel::default()),
        place_model: Rc::new(VecModel::default()),
        drive_model: Rc::new(VecModel::default()),
        session_save_scheduled: false,
        drives_loading: false,
    });
    // Separate cell: used inside selection changes, which already borrow APP.
    static TYPEAHEAD: RefCell<TypeAhead> = RefCell::new(TypeAhead::default());
}

// ---------------------------------------------------------------- helpers

/// Minimum gap between two watcher refreshes: at least 500 ms, and 4x the cost
/// of the last load (a 300 ms folder refreshes at most every 1.2 s), capped at 10 s.
/// Prevents a constantly written file (logs, downloads) from pinning a CPU core.
pub fn refresh_cooldown(last_load: Duration) -> Duration {
    (last_load * 4).clamp(MIN_REFRESH_GAP, Duration::from_secs(10))
}

/// Minimum time between the starts of two watcher refreshes of the same tab.
pub const MIN_REFRESH_GAP: Duration = Duration::from_millis(500);

/// "1 Element" / "3 Elemente" with German thousands separator.
pub fn count_label(n: usize) -> String {
    if n == 1 {
        "1 Element".to_string()
    } else {
        format!("{} Elemente", format_count(n))
    }
}

/// Status bar text: base + selection info ("3 Elemente ausgewählt").
pub fn status_with_selection(base: &str, selected: usize) -> String {
    match selected {
        0 => base.to_string(),
        1 => format!("{base}  |  1 Element ausgewählt"),
        n => format!("{base}  |  {} Elemente ausgewählt", format_count(n)),
    }
}

/// Breadcrumb character budget for an address bar of `px` pixels.
pub fn crumb_budget(px: f32) -> usize {
    (((px - 24.0) / 7.5).max(12.0)) as usize
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

fn to_row(e: &Entry) -> Row {
    Row {
        name: e.name.as_str().into(),
        size: if e.is_dir() {
            Default::default()
        } else {
            format_size(e.size).into()
        },
        modified: e.modified.map(format_datetime).unwrap_or_default().into(),
        kind: type_label(e).into(),
        is_dir: e.is_dir(),
        dim: e.hidden,
        selected: false,
        focused: false,
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

/// Carries the selection over to a reloaded list of the same folder, matched by name.
fn remap_selection(old_entries: &[Entry], old: &Selection, new_entries: &[Entry]) -> Selection {
    let index: HashMap<&str, usize> = new_entries
        .iter()
        .enumerate()
        .map(|(i, e)| (e.name.as_str(), i))
        .collect();
    old.remap(new_entries.len(), |i| {
        old_entries
            .get(i)
            .and_then(|e| index.get(e.name.as_str()).copied())
    })
}

/// Finds a tab by id → (pane index, tab index).
fn locate(app: &App, id: TabId) -> Option<(usize, usize)> {
    app.panes
        .iter()
        .enumerate()
        .find_map(|(pi, p)| p.tabs.iter().position(|t| t.id == id).map(|ti| (pi, ti)))
}

fn with_tab<R>(id: TabId, f: impl FnOnce(&mut Tab, usize, bool) -> R) -> Option<R> {
    APP.with_borrow_mut(|app| {
        let (pi, ti) = locate(app, id)?;
        let visible = app.panes[pi].active == ti;
        Some(f(&mut app.panes[pi].tabs[ti], pi, visible))
    })
}

fn active_tab_id(pane: usize) -> Option<TabId> {
    APP.with_borrow(|app| app.panes.get(pane).map(|p| p.tab().id))
}

// ---------------------------------------------------------------- UI sync

/// Pushes a pane's state to its `PaneData` row.
fn sync_pane(pi: usize) {
    APP.with_borrow(|app| {
        let Some(p) = app.panes.get(pi) else { return };
        let t = p.tab();
        let data = PaneData {
            tabs: ModelRc::new(VecModel::from(
                p.tabs
                    .iter()
                    .map(|t| TabInfo {
                        title: t.title().into(),
                        path: t
                            .path
                            .as_ref()
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_default()
                            .into(),
                    })
                    .collect::<Vec<_>>(),
            )),
            active_tab: p.active as i32,
            rows: ModelRc::from(t.model.clone()),
            crumbs: ModelRc::from(t.crumbs.clone()),
            path: t
                .path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default()
                .into(),
            can_back: t.history.can_back(),
            can_forward: t.history.can_forward(),
            scroll_to: p.scroll_to,
            scroll_seq: p.scroll_seq,
            top_seq: p.top_seq,
            edit_seq: p.edit_seq,
            focus_seq: p.focus_seq,
        };
        if pi < app.pane_model.row_count() {
            app.pane_model.set_row_data(pi, data);
        } else {
            app.pane_model.push(data);
        }
    });
}

fn sync_all(win: &MainWindow) {
    let n = APP.with_borrow(|app| app.panes.len());
    APP.with_borrow(|app| {
        while app.pane_model.row_count() > n {
            app.pane_model.remove(app.pane_model.row_count() - 1);
        }
    });
    for pi in 0..n {
        sync_pane(pi);
    }
    sync_window(win);
}

/// Window-level state that depends on the active pane/tab.
fn sync_window(win: &MainWindow) {
    let (active, split, path, status) = APP.with_borrow(|app| {
        let t = app.panes[app.active_pane].tab();
        (
            app.active_pane,
            app.panes.len() > 1,
            t.path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            status_with_selection(&t.status_base, t.selection.count()),
        )
    });
    win.set_active_pane(active as i32);
    win.set_split(split);
    win.set_active_path(path.into());
    win.set_status(status.into());
    schedule_session_save();
}

fn request(pi: usize, f: impl FnOnce(&mut Pane)) {
    APP.with_borrow_mut(|app| {
        if let Some(p) = app.panes.get_mut(pi) {
            f(p);
        }
    });
    sync_pane(pi);
}

fn request_focus(pi: usize) {
    request(pi, |p| p.focus_seq += 1);
}

// ---------------------------------------------------------------- init / session

/// Sets up panes from the session (or a start path) and loads the visible tabs.
pub fn init(
    win: &MainWindow,
    show_hidden: bool,
    sort: SortSpec,
    start: Option<PathBuf>,
    session: Option<Session>,
) {
    let (pane_model, place_model, drive_model) = APP.with_borrow_mut(|app| {
        app.filter.show_hidden = show_hidden;
        app.sort = sort;
        (
            app.pane_model.clone(),
            app.place_model.clone(),
            app.drive_model.clone(),
        )
    });
    win.set_panes(ModelRc::from(pane_model));
    win.set_places(ModelRc::from(place_model));
    win.set_drives(ModelRc::from(drive_model));
    win.set_show_hidden(show_hidden);
    show_sort(win, sort);

    let fallback = || {
        std::env::current_dir()
            .ok()
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
    };
    let usable = |s: &str| explorer_fs::nearest_existing_ancestor(Path::new(s));
    let restored_active = session
        .as_ref()
        .filter(|_| start.is_none())
        .map(|s| s.active_pane)
        .unwrap_or(0);
    let layout: Vec<(Vec<PathBuf>, usize)> = match (start, session) {
        (Some(p), _) => vec![(vec![p], 0)],
        (None, Some(s)) => s
            .panes
            .iter()
            .map(|p: &PaneSession| {
                let tabs: Vec<PathBuf> = p.tabs.iter().filter_map(|t| usable(t)).collect();
                let active = p.active_tab.min(tabs.len().saturating_sub(1));
                (tabs, active)
            })
            .filter(|(t, _)| !t.is_empty())
            .collect(),
        (None, None) => Vec::new(),
    };
    let layout = if layout.is_empty() {
        vec![(fallback().into_iter().collect::<Vec<_>>(), 0)]
    } else {
        layout
    };
    APP.with_borrow_mut(|app| {
        for (tabs, active) in layout {
            let mut pane_tabs = Vec::new();
            for p in tabs {
                let id = app.next_tab_id;
                app.next_tab_id += 1;
                pane_tabs.push(Tab::new(id, Some(p)));
            }
            if pane_tabs.is_empty() {
                continue;
            }
            let mut pane = Pane::new(pane_tabs.remove(0));
            pane.tabs.extend(pane_tabs);
            pane.active = active.min(pane.tabs.len() - 1);
            app.panes.push(pane);
        }
        if app.panes.is_empty() {
            let id = app.next_tab_id;
            app.next_tab_id += 1;
            app.panes
                .push(Pane::new(Tab::new(id, Some(PathBuf::from("/")))));
        }
        app.panes.truncate(2);
        app.active_pane = restored_active.min(app.panes.len() - 1);
    });
    if trace_enabled() {
        let s = current_session();
        let tabs: Vec<usize> = s.panes.iter().map(|p| p.tabs.len()).collect();
        trace(&format!(
            "layout panes={} tabs={tabs:?} split={} active_pane={}",
            s.panes.len(),
            s.split,
            s.active_pane
        ));
    }
    let n = APP.with_borrow(|app| app.panes.len());
    for pi in 0..n {
        ensure_loaded(win, pi); // only the visible tab of each pane; others load when selected
    }
    sync_all(win);
    let active = APP.with_borrow(|app| app.active_pane);
    request_focus(active);
    refresh_places(win);
}

pub fn current_session() -> Session {
    APP.with_borrow(|app| Session {
        panes: app
            .panes
            .iter()
            .map(|p| PaneSession {
                tabs: p
                    .tabs
                    .iter()
                    .filter_map(|t| t.path.as_ref())
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect(),
                active_tab: p.active,
            })
            .collect(),
        active_pane: app.active_pane,
        split: app.panes.len() > 1,
    })
}

/// Debounced session save (tabs/paths change often while browsing).
fn schedule_session_save() {
    let already =
        APP.with_borrow_mut(|app| std::mem::replace(&mut app.session_save_scheduled, true));
    if already {
        return;
    }
    slint::Timer::single_shot(Duration::from_millis(400), || {
        APP.with_borrow_mut(|app| app.session_save_scheduled = false);
        save_session_now();
    });
}

pub fn save_session_now() {
    let s = current_session();
    match crate::config::save_session(&s) {
        Ok(()) => trace(&format!(
            "session saved panes={} split={}",
            s.panes.len(),
            s.split
        )),
        Err(e) => trace(&format!("session not saved: {e}")),
    }
}

// ---------------------------------------------------------------- places / drives

pub fn refresh_places(win: &MainWindow) {
    let places: Vec<PlaceItem> = explorer_fs::places::known_folders()
        .into_iter()
        .map(|p| PlaceItem {
            label: p.kind.label_de().into(),
            path: p.path.to_string_lossy().as_ref().into(),
            is_drive: false,
            detail: Default::default(),
            used: 0.0,
            has_bar: false,
        })
        .collect();
    APP.with_borrow(|app| app.place_model.set_vec(places));
    refresh_drives(win);
}

/// Drive info can block (network/optical drives) → background thread.
pub fn refresh_drives(win: &MainWindow) {
    if APP.with_borrow_mut(|app| std::mem::replace(&mut app.drives_loading, true)) {
        return;
    }
    let weak = win.as_weak();
    let _ = std::thread::Builder::new()
        .name("drives".into())
        .spawn(move || {
            let t0 = Instant::now();
            let drives = explorer_fs::places::drives();
            let took = t0.elapsed();
            let items: Vec<(String, String, String, f32, bool)> = drives
                .iter()
                .map(|d| {
                    (
                        drive_display_name(d),
                        d.root.to_string_lossy().into_owned(),
                        d.space.map(|(f, t)| format_usage(f, t)).unwrap_or_default(),
                        used_fraction(d.space),
                        d.space.is_some(),
                    )
                })
                .collect();
            let _ = slint::invoke_from_event_loop(move || {
                APP.with_borrow_mut(|app| {
                    app.drives_loading = false;
                    app.drive_model.set_vec(
                        items
                            .into_iter()
                            .map(|(label, path, detail, used, has_bar)| PlaceItem {
                                label: label.into(),
                                path: path.into(),
                                is_drive: true,
                                detail: detail.into(),
                                used,
                                has_bar,
                            })
                            .collect::<Vec<_>>(),
                    );
                });
                trace(&format!(
                    "drives loaded n={} in {took:?}",
                    APP.with_borrow(|a| a.drive_model.row_count())
                ));
                let _ = weak;
            });
        });
}

pub fn open_place(win: &MainWindow, path: &str) {
    let pi = APP.with_borrow(|app| app.active_pane);
    if let Some(id) = active_tab_id(pi) {
        navigate_tab(win, id, PathBuf::from(path), Reason::User, Hist::Push);
    }
    request_focus(pi);
}

// ---------------------------------------------------------------- loading

/// Loads the visible tab of a pane if it has never been loaded or is stale.
fn ensure_loaded(win: &MainWindow, pi: usize) {
    let target = APP.with_borrow(|app| {
        let t = app.panes.get(pi)?.tab();
        t.stale
            .then(|| (t.id, t.path.clone(), t.entries.is_empty() && t.gen == 0))
    });
    if let Some((id, Some(path), first)) = target {
        let reason = if first { Reason::User } else { Reason::Refresh };
        navigate_tab(win, id, path, reason, Hist::Keep);
    }
}

fn navigate_tab(win: &MainWindow, id: TabId, target: PathBuf, reason: Reason, hist: Hist) {
    let target = std::path::absolute(&target).unwrap_or(target);
    let (filter, sort) = APP.with_borrow(|app| (app.filter, app.sort));
    let gen = APP.with_borrow_mut(|app| {
        app.next_gen += 1;
        app.next_gen
    });
    let Some(pi) = with_tab(id, |t, pi, _| {
        t.gen = gen;
        t.stale = false;
        if hist == Hist::Push {
            if let Some(cur) = t.path.clone() {
                if cur != target {
                    t.history.visit(cur);
                }
            }
        }
        pi
    }) else {
        return;
    };
    if reason == Reason::User {
        set_tab_status(win, id, "Lädt …".into());
        win.set_error("".into());
    }
    trace(&format!(
        "navigate tab={id} gen={gen} {reason:?} {} at={:.1}ms",
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
                apply(&w, id, gen, path, reason, upd, t0);
            }
        });
    });
    with_tab(id, |t, _, _| t.loader = Some(handle)); // drops (= cancels) the previous load
    sync_pane(pi);
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

fn apply(
    win: &MainWindow,
    id: TabId,
    gen: u64,
    path: PathBuf,
    reason: Reason,
    upd: Update,
    t0: Instant,
) {
    let current = with_tab(id, |t, _, _| (t.gen == gen).then(|| t.path.clone()));
    let Some(Some(current)) = current else {
        return; // tab closed or stale result
    };
    match upd {
        Update::Rows {
            entries,
            mut rows,
            hidden,
            done,
            data_ready,
        } => {
            let path_changed = current.as_deref() != Some(path.as_path());
            let n = entries.len();
            let (pi, focus, needs_setup) = with_tab(id, |t, pi, _| {
                // New tabs already carry their path → also set up crumbs/watcher on the first load.
                let needs_setup = path_changed || t.watcher.is_none();
                let mut sel = if path_changed {
                    Selection::new(n)
                } else {
                    remap_selection(&t.entries, &t.selection, &entries)
                };
                // Select the folder we came from (Explorer behaviour on "up"), once fully loaded.
                if done.is_some() {
                    if let Some(name) = t.pending_select.take() {
                        if let Some(i) = entries.iter().position(|e| e.name == name) {
                            sel.click(i);
                        }
                    }
                }
                for i in sel.selected_indices() {
                    rows[i].selected = true;
                }
                if let Some(f) = sel.focus() {
                    rows[f].focused = true;
                }
                t.entries = entries;
                t.model.set_vec(rows);
                let focus = sel.focus();
                t.selection = sel;
                if path_changed {
                    t.path = Some(path.clone());
                }
                (pi, focus, needs_setup)
            })
            .unwrap_or((0, None, false));
            if needs_setup {
                let chars =
                    APP.with_borrow(|app| app.panes.get(pi).map(|p| p.crumb_chars).unwrap_or(60));
                show_crumbs(id, &path, chars);
                if path_changed || reason == Reason::User {
                    request(pi, |p| p.top_seq += 1);
                }
                start_watch(win, id, &path);
            }
            if let Some(f) = focus {
                request(pi, |p| {
                    p.scroll_to = f as i32;
                    p.scroll_seq += 1;
                });
            }
            match done {
                None => {
                    set_tab_status(win, id, format!("Lädt … {}", format_count(n)));
                    trace(&format!(
                        "first paint tab={id} gen={gen} rows={n} after {:?} data_ready={:.1}ms at={:.1}ms",
                        t0.elapsed(),
                        data_ready.as_secs_f64() * 1000.0,
                        since_start_ms()
                    ));
                }
                Some((total, took)) => {
                    with_tab(id, |t, _, _| {
                        // Expensive loads push the next refresh further out (never earlier).
                        t.refresh_allowed_at = t
                            .refresh_allowed_at
                            .max(Instant::now() + refresh_cooldown(took))
                    });
                    let mut st = count_label(n);
                    if hidden > 0 {
                        st.push_str(&format!(" ({} ausgeblendet)", format_count(hidden)));
                    }
                    if reason == Reason::User {
                        st.push_str(&format!(" · {} ms", took.as_millis()));
                    }
                    set_tab_status(win, id, st);
                    trace(&format!(
                        "done tab={id} gen={gen} total={total} shown={n} after {took:?} data_ready={:.1}ms at={:.1}ms",
                        data_ready.as_secs_f64() * 1000.0,
                        since_start_ms()
                    ));
                    trace(&format!("address={}", path.display()));
                    if trace_enabled() {
                        let order: Vec<String> = with_tab(id, |t, _, _| {
                            t.entries.iter().take(8).map(|e| e.name.clone()).collect()
                        })
                        .unwrap_or_default();
                        trace(&format!("order {order:?}"));
                    }
                    trace_selection(id);
                }
            }
            sync_pane(pi);
            sync_window(win);
        }
        Update::Failed(e) => {
            trace(&format!(
                "failed tab={id} gen={gen} {:?} {}",
                e.kind,
                path.display()
            ));
            let vanished =
                e.kind == FsErrorKind::NotFound && current.as_deref() == Some(path.as_path());
            if vanished {
                // Current folder was deleted/renamed → go to the nearest existing parent.
                if let Some(up) = explorer_fs::nearest_existing_ancestor(&path) {
                    navigate_tab(win, id, up, Reason::User, Hist::Keep);
                    win.set_error(
                        format!("Ordner existiert nicht mehr: {}", path.display()).into(),
                    );
                    return;
                }
            }
            win.set_error(format!("{}: {}", e.message, path.display()).into());
            // Keep the previous listing; the address bar shows the real path again.
            let n = with_tab(id, |t, _, _| t.model.row_count()).unwrap_or(0);
            if current.is_some() {
                set_tab_status(win, id, count_label(n));
                trace(&format!(
                    "address={}",
                    current
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default()
                ));
            } else {
                set_tab_status(win, id, String::new());
            }
            if let Some((pi, _)) = APP.with_borrow(|app| locate(app, id)) {
                sync_pane(pi);
            }
            sync_window(win);
        }
    }
}

fn show_crumbs(id: TabId, path: &Path, chars: usize) {
    let list: Vec<Crumb> = fit(crumbs(&path.to_string_lossy()), chars)
        .into_iter()
        .map(|c| Crumb {
            label: c.label.into(),
            path: c.path.into(),
        })
        .collect();
    with_tab(id, |t, _, _| t.crumbs.set_vec(list));
}

fn set_tab_status(win: &MainWindow, id: TabId, base: String) {
    with_tab(id, |t, _, _| t.status_base = base);
    let active =
        APP.with_borrow(|app| app.panes.get(app.active_pane).map(|p| p.tab().id) == Some(id));
    if active {
        sync_window(win);
    }
}

fn trace_selection(id: TabId) {
    if trace_enabled() {
        with_tab(id, |t, pi, _| {
            let names: Vec<&str> = t
                .selection
                .selected_indices()
                .iter()
                .take(10)
                .map(|&i| t.entries[i].name.as_str())
                .collect();
            let focus = t
                .selection
                .focus()
                .map(|f| t.entries[f].name.as_str())
                .unwrap_or("-");
            trace(&format!(
                "selection pane={pi} tab={} count={} focus={focus} names={names:?}",
                t.id,
                t.selection.count()
            ));
        });
    }
}

/// Applies a selection change to the active tab of `pi` and repaints only changed rows.
fn change_selection(win: &MainWindow, pi: usize, f: impl FnOnce(&mut Selection, &[Entry])) {
    let Some(id) = active_tab_id(pi) else { return };
    let focus = with_tab(id, |t, _, _| {
        let before = t.selection.clone();
        let entries = std::mem::take(&mut t.entries);
        f(&mut t.selection, &entries);
        t.entries = entries;
        let after = &t.selection;
        let mut touched: Vec<usize> = before.selected_indices();
        touched.extend(after.selected_indices());
        touched.extend(before.focus());
        touched.extend(after.focus());
        touched.sort_unstable();
        touched.dedup();
        for i in touched {
            if let Some(mut row) = t.model.row_data(i) {
                let (sel, foc) = (after.is_selected(i), after.focus() == Some(i));
                if row.selected != sel || row.focused != foc {
                    row.selected = sel;
                    row.focused = foc;
                    t.model.set_row_data(i, row);
                }
            }
        }
        after.focus()
    })
    .flatten();
    if let Some(i) = focus {
        request(pi, |p| {
            p.scroll_to = i as i32;
            p.scroll_seq += 1;
        });
    }
    sync_window(win);
    trace_selection(id);
}

fn start_watch(win: &MainWindow, id: TabId, path: &Path) {
    let weak = win.as_weak();
    let watched = path.to_path_buf();
    let w = watch_dir(path, move || {
        let (weak, watched) = (weak.clone(), watched.clone());
        let _ = slint::invoke_from_event_loop(move || request_refresh(weak, id, watched));
    });
    if let Err(e) = &w {
        trace(&format!("watch failed (auto-refresh off): {e}"));
    }
    with_tab(id, |t, _, _| t.watcher = w.ok()); // drops the old watcher
}

/// Watcher fired: refresh now, or once after the cooldown (coalescing all events in between).
fn request_refresh(weak: slint::Weak<MainWindow>, id: TabId, watched: PathBuf) {
    let state = with_tab(id, |t, _, _| {
        (
            t.path.as_deref() == Some(watched.as_path()),
            t.refresh_allowed_at,
            t.refresh_scheduled,
        )
    });
    let Some((true, allowed_at, false)) = state else {
        return;
    };
    let now = Instant::now();
    if now >= allowed_at {
        if let Some(w) = weak.upgrade() {
            // Spacing is guaranteed between refresh *starts*: events that arrive while
            // this refresh is still loading must not start another one right away.
            with_tab(id, |t, _, _| t.refresh_allowed_at = now + MIN_REFRESH_GAP);
            trace("watch: refresh");
            navigate_tab(&w, id, watched, Reason::Refresh, Hist::Keep);
        }
    } else {
        with_tab(id, |t, _, _| t.refresh_scheduled = true);
        trace(&format!(
            "watch: refresh throttled for {:?}",
            allowed_at - now
        ));
        slint::Timer::single_shot(allowed_at - now, move || {
            with_tab(id, |t, _, _| t.refresh_scheduled = false);
            request_refresh(weak, id, watched);
        });
    }
}

fn reload_tab(win: &MainWindow, id: TabId) {
    if let Some(Some(p)) = with_tab(id, |t, _, _| t.path.clone()) {
        navigate_tab(win, id, p, Reason::Refresh, Hist::Keep);
    }
}

/// Sort/filter changed: reload visible tabs now, the others when they become visible.
fn invalidate_all(win: &MainWindow) {
    let visible: Vec<TabId> = APP.with_borrow_mut(|app| {
        for p in &mut app.panes {
            for t in &mut p.tabs {
                t.stale = true;
            }
        }
        app.panes.iter().map(|p| p.tab().id).collect()
    });
    for id in visible {
        reload_tab(win, id);
    }
}

// ---------------------------------------------------------------- public actions

pub fn activate_pane(win: &MainWindow, pi: usize) {
    let changed = APP.with_borrow_mut(|app| {
        if pi < app.panes.len() && app.active_pane != pi {
            app.active_pane = pi;
            true
        } else {
            false
        }
    });
    if changed {
        trace(&format!("pane active={pi}"));
        sync_window(win);
    }
}

pub fn navigate(win: &MainWindow, pi: usize, target: &str) {
    activate_pane(win, pi);
    if let Some(id) = active_tab_id(pi) {
        navigate_tab(
            win,
            id,
            PathBuf::from(clean_input(target)),
            Reason::User,
            Hist::Push,
        );
    }
}

pub fn cancel_edit(pi: usize) {
    if let Some(Some(p)) = active_tab_id(pi).and_then(|id| with_tab(id, |t, _, _| t.path.clone())) {
        trace(&format!("address(cancel)={}", p.display()));
    }
}

pub fn go_up(win: &MainWindow, pi: usize) {
    let Some(id) = active_tab_id(pi) else { return };
    let target = with_tab(id, |t, _, _| {
        let p = t.path.as_ref()?;
        let parent = p.parent()?.to_path_buf();
        t.pending_select = p.file_name().map(|n| n.to_string_lossy().into_owned());
        Some(parent)
    })
    .flatten();
    if let Some(p) = target {
        navigate_tab(win, id, p, Reason::User, Hist::Push);
    }
}

pub fn back(win: &MainWindow, pi: usize) {
    history_step(win, pi, true);
}

pub fn forward(win: &MainWindow, pi: usize) {
    history_step(win, pi, false);
}

fn history_step(win: &MainWindow, pi: usize, back: bool) {
    activate_pane(win, pi);
    let Some(id) = active_tab_id(pi) else { return };
    let target = with_tab(id, |t, _, _| {
        let cur = t.path.clone()?;
        let target = if back {
            t.history.back(cur.clone())
        } else {
            t.history.forward(cur.clone())
        }?;
        // Going back to the parent selects the folder we came from, like Explorer.
        if cur.parent() == Some(target.as_path()) {
            t.pending_select = cur.file_name().map(|n| n.to_string_lossy().into_owned());
        }
        Some(target)
    })
    .flatten();
    if let Some(p) = target {
        trace(&format!(
            "history {} -> {}",
            if back { "back" } else { "forward" },
            p.display()
        ));
        navigate_tab(win, id, p, Reason::User, Hist::Keep);
    }
}

/// Opens entry `index` of pane `pi`: folders navigate, files open with their default app.
pub fn open_entry(win: &MainWindow, pi: usize, index: usize) {
    let Some(id) = active_tab_id(pi) else { return };
    let target = with_tab(id, |t, _, _| {
        let e = t.entries.get(index)?;
        Some((t.path.as_ref()?.join(&e.name), e.is_dir()))
    })
    .flatten();
    match target {
        Some((p, true)) => navigate_tab(win, id, p, Reason::User, Hist::Push),
        Some((p, false)) => {
            trace(&format!("open file {}", p.display()));
            if let Err(e) = crate::open::open_path(&p) {
                win.set_error(format!("{e}: {}", p.display()).into());
            }
        }
        None => {}
    }
}

pub fn set_show_hidden(win: &MainWindow, on: bool) {
    if let Err(e) = crate::config::update(|s| s.show_hidden = on) {
        win.set_error(e.into());
    }
    APP.with_borrow_mut(|app| app.filter.show_hidden = on);
    invalidate_all(win);
}

fn show_sort(win: &MainWindow, sort: SortSpec) {
    win.set_sort_key(sort.key.as_str().into());
    win.set_sort_desc(sort.descending);
}

/// Column header click: same column toggles direction, another column sorts ascending.
pub fn sort_by(win: &MainWindow, key: &str) {
    let Some(key) = SortKey::parse(key) else {
        return;
    };
    let sort = APP.with_borrow_mut(|app| {
        if app.sort.key == key {
            app.sort.descending = !app.sort.descending;
        } else {
            app.sort.key = key;
            app.sort.descending = false;
        }
        app.sort
    });
    trace(&format!(
        "sort key={} desc={}",
        sort.key.as_str(),
        sort.descending
    ));
    show_sort(win, sort);
    if let Err(e) = crate::config::update(|s| {
        s.sort_key = sort.key;
        s.sort_desc = sort.descending;
    }) {
        win.set_error(e.into());
    }
    invalidate_all(win);
}

pub fn row_pressed(win: &MainWindow, pi: usize, index: usize, ctrl: bool, shift: bool) {
    activate_pane(win, pi);
    change_selection(win, pi, |sel, _| match (ctrl, shift) {
        (_, true) => sel.shift_click(index),
        (true, false) => sel.ctrl_click(index),
        _ => sel.click(index),
    });
}

/// Click on empty space below the rows clears the selection (Explorer behaviour).
pub fn clear_selection(win: &MainWindow, pi: usize) {
    change_selection(win, pi, |sel, _| sel.clear());
}

pub fn address_resized(pi: usize, px: f32) {
    let chars = crumb_budget(px);
    let tab = APP.with_borrow_mut(|app| {
        let p = app.panes.get_mut(pi)?;
        if p.crumb_chars == chars {
            return None;
        }
        p.crumb_chars = chars;
        let t = p.tab();
        Some((t.id, t.path.clone()?))
    });
    if let Some((id, path)) = tab {
        show_crumbs(id, &path, chars);
    }
}

// ---------------------------------------------------------------- tabs & panes

pub fn tab_select(win: &MainWindow, pi: usize, ti: usize) {
    activate_pane(win, pi);
    let changed = APP.with_borrow_mut(|app| {
        let p = app.panes.get_mut(pi)?;
        (ti < p.tabs.len() && p.active != ti).then(|| p.active = ti)
    });
    if changed.is_some() {
        trace(&format!("tab select pane={pi} tab_index={ti}"));
        ensure_loaded(win, pi);
        sync_pane(pi);
        sync_window(win);
        request_focus(pi);
    }
}

/// New tab next to the current one, showing the same folder (Ctrl+T).
pub fn tab_new(win: &MainWindow, pi: usize) {
    activate_pane(win, pi);
    let id = APP.with_borrow_mut(|app| {
        let id = app.next_tab_id;
        app.next_tab_id += 1;
        let p = app.panes.get_mut(pi)?;
        let path = p.tab().path.clone();
        let at = p.active + 1;
        p.tabs.insert(at, Tab::new(id, path));
        p.active = at;
        Some(id)
    });
    if let Some(id) = id {
        trace(&format!("tab new pane={pi} tab={id}"));
        ensure_loaded(win, pi);
        sync_pane(pi);
        sync_window(win);
        request_focus(pi);
    }
}

/// Closes a tab (Ctrl+W, ×, middle click). The last tab of the second pane closes
/// the split; the very last tab of the window is kept.
pub fn tab_close(win: &MainWindow, pi: usize, ti: usize) {
    let (n_tabs, n_panes) = APP.with_borrow(|app| {
        (
            app.panes.get(pi).map(|p| p.tabs.len()).unwrap_or(0),
            app.panes.len(),
        )
    });
    if n_tabs == 0 {
        return;
    }
    if n_tabs == 1 {
        if n_panes > 1 {
            close_pane(win, pi);
        }
        return;
    }
    APP.with_borrow_mut(|app| {
        let p = &mut app.panes[pi];
        let t = p.tabs.remove(ti.min(p.tabs.len() - 1)); // drop cancels loader + watcher
        trace(&format!("tab close pane={pi} tab={}", t.id));
        if p.active > ti || p.active >= p.tabs.len() {
            p.active = p.active.saturating_sub(1).min(p.tabs.len() - 1);
        }
    });
    ensure_loaded(win, pi);
    sync_pane(pi);
    sync_window(win);
    request_focus(pi);
}

fn close_pane(win: &MainWindow, pi: usize) {
    APP.with_borrow_mut(|app| {
        if app.panes.len() > 1 && pi < app.panes.len() {
            app.panes.remove(pi);
            app.active_pane = 0;
        }
    });
    trace("split off");
    sync_all(win);
    request_focus(0);
}

/// "Zwei Ansichten": opens a second pane on the current folder, or closes it.
pub fn toggle_split(win: &MainWindow) {
    let split = APP.with_borrow(|app| app.panes.len() > 1);
    if split {
        let pi = APP.with_borrow(|app| app.panes.len() - 1);
        close_pane(win, pi);
        return;
    }
    APP.with_borrow_mut(|app| {
        let id = app.next_tab_id;
        app.next_tab_id += 1;
        let path = app.panes[app.active_pane].tab().path.clone();
        app.panes.push(Pane::new(Tab::new(id, path)));
        app.active_pane = 1;
    });
    trace("split on");
    ensure_loaded(win, 1);
    sync_all(win);
    request_focus(1);
}

fn switch_pane(win: &MainWindow) {
    let n = APP.with_borrow(|app| app.panes.len());
    if n < 2 {
        return;
    }
    let next = (APP.with_borrow(|app| app.active_pane) + 1) % n;
    activate_pane(win, next);
    request_focus(next);
}

fn cycle_tab(win: &MainWindow, pi: usize, delta: isize) {
    let target = APP.with_borrow(|app| {
        let p = app.panes.get(pi)?;
        let n = p.tabs.len() as isize;
        (n > 1).then(|| ((p.active as isize + delta).rem_euclid(n)) as usize)
    });
    if let Some(ti) = target {
        tab_select(win, pi, ti);
    }
}

/// Handles a key in a pane's file list. Returns true if consumed.
pub fn list_key(
    win: &MainWindow,
    pi: usize,
    text: &str,
    ctrl: bool,
    shift: bool,
    alt: bool,
    page_rows: i32,
) -> bool {
    let Some(action) = map_key(text, ctrl, shift, alt, page_rows as isize) else {
        return false;
    };
    if std::env::var_os("EXPLORER2_TRACE_KEYS").is_some() {
        trace(&format!("key pane={pi} action={action:?} page={page_rows}"));
    }
    activate_pane(win, pi);
    match action {
        KeyAction::Move {
            delta,
            extend,
            focus_only,
        } => change_selection(win, pi, |sel, _| sel.move_by(delta, extend, focus_only)),
        KeyAction::Edge {
            end,
            extend,
            focus_only,
        } => change_selection(win, pi, |sel, _| {
            let target = if end { sel.len() as isize - 1 } else { 0 };
            sel.move_to(target, extend, focus_only)
        }),
        KeyAction::SelectAll => change_selection(win, pi, |sel, _| sel.select_all()),
        KeyAction::ToggleFocused => change_selection(win, pi, |sel, _| {
            if let Some(f) = sel.focus() {
                sel.ctrl_click(f);
            }
        }),
        KeyAction::Open => {
            let targets = active_tab_id(pi)
                .and_then(|id| {
                    with_tab(id, |t, _, _| {
                        let sel = t.selection.selected_indices();
                        if sel.is_empty() {
                            t.selection.focus().into_iter().collect()
                        } else {
                            sel
                        }
                    })
                })
                .unwrap_or_default();
            match targets.as_slice() {
                [] => {}
                [one] => open_entry(win, pi, *one),
                many => {
                    // Several items: open files (max 20, as a safety net); folders are skipped.
                    let files: Vec<usize> = active_tab_id(pi)
                        .and_then(|id| {
                            with_tab(id, |t, _, _| {
                                many.iter()
                                    .copied()
                                    .filter(|&i| t.entries.get(i).is_some_and(|e| !e.is_dir()))
                                    .take(20)
                                    .collect()
                            })
                        })
                        .unwrap_or_default();
                    for i in files {
                        open_entry(win, pi, i);
                    }
                }
            }
        }
        KeyAction::Up => go_up(win, pi),
        KeyAction::Refresh => {
            if let Some(id) = active_tab_id(pi) {
                reload_tab(win, id);
            }
        }
        KeyAction::EditPath => request(pi, |p| p.edit_seq += 1),
        KeyAction::Back => back(win, pi),
        KeyAction::Forward => forward(win, pi),
        KeyAction::NewTab => tab_new(win, pi),
        KeyAction::CloseTab => {
            let ti = APP.with_borrow(|app| app.panes[pi].active);
            tab_close(win, pi, ti);
        }
        KeyAction::NextTab => cycle_tab(win, pi, 1),
        KeyAction::PrevTab => cycle_tab(win, pi, -1),
        KeyAction::SwitchPane => switch_pane(win),
        KeyAction::ToggleSplit => toggle_split(win),
        KeyAction::TypeChar(c) => change_selection(win, pi, |sel, entries| {
            let found = TYPEAHEAD.with_borrow_mut(|t| {
                t.feed(
                    c,
                    Instant::now(),
                    sel.focus(),
                    |i| entries[i].name.as_str(),
                    entries.len(),
                )
            });
            if let Some(i) = found {
                sel.click(i);
            }
        }),
    }
    true
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
    fn count_and_status_labels() {
        assert_eq!(count_label(0), "0 Elemente");
        assert_eq!(count_label(1), "1 Element");
        assert_eq!(count_label(100_000), "100.000 Elemente");
        assert_eq!(status_with_selection("5 Elemente", 0), "5 Elemente");
        assert_eq!(
            status_with_selection("5 Elemente", 1),
            "5 Elemente  |  1 Element ausgewählt"
        );
        assert_eq!(
            status_with_selection("5 Elemente", 1200),
            "5 Elemente  |  1.200 Elemente ausgewählt"
        );
    }

    #[test]
    fn crumb_budget_from_width() {
        assert_eq!(crumb_budget(0.0), 12);
        assert_eq!(crumb_budget(774.0), 100);
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
        assert_eq!(rows[0].kind, "Dateiordner");
        assert_eq!(rows[1].size, "2 KB");
        assert_eq!(rows[1].kind, "TXT-Datei");
        assert_eq!(rows[1].modified, "1970-01-01 00:00");
        let show = ViewFilter {
            show_hidden: true,
            show_protected_os: false,
        };
        let (vis, rows, hidden) = prepare(all, show, SortSpec::default());
        assert_eq!((vis.len(), hidden), (3, 0));
        assert!(rows.iter().any(|r| r.dim));
    }

    #[test]
    fn selection_survives_reload_by_name() {
        let old = vec![
            e("a", false, false),
            e("b", false, false),
            e("c", false, false),
        ];
        let mut sel = Selection::new(3);
        sel.click(1);
        sel.ctrl_click(2);
        let new = vec![
            e("c", false, false),
            e("new", false, false),
            e("a", false, false),
        ];
        let r = remap_selection(&old, &sel, &new);
        assert_eq!(r.selected_indices(), [0]);
        assert_eq!(r.focus(), Some(0));
    }

    #[test]
    fn tab_titles() {
        assert_eq!(
            Tab::new(1, Some(PathBuf::from("/home/boss/Bilder"))).title(),
            "Bilder"
        );
        assert_eq!(Tab::new(1, Some(PathBuf::from("/"))).title(), "/");
        assert_eq!(Tab::new(1, None).title(), "…");
    }
}
