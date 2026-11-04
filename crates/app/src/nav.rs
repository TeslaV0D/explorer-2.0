//! Window controller: panes (1 or 2, split view) → tabs → one folder each.
//!
//! All state lives on the UI thread. Directory loading, filtering, sorting and
//! row formatting run on background threads; results are routed back by a
//! stable **tab id** (tabs can be closed or moved meanwhile) and guarded by a
//! generation counter so a stale result never overwrites a newer navigation.

use crate::keys::{map_key_grid, KeyAction};
use crate::{Crumb, MainWindow, PaneData, PlaceItem, Row, TabInfo};
use explorer_core::breadcrumb::{crumbs, fit};
use explorer_core::history::History;
use explorer_core::places::VirtualPlace;
use explorer_core::selection::Selection;
use explorer_core::session::{PaneSession, Session};
use explorer_core::typeahead::TypeAhead;
use explorer_core::view::ViewMode;
use explorer_core::{
    format_count, format_datetime, format_size, format_usage, sort_entries_with_type_names, Entry,
    SortKey, SortSpec, ViewFilter,
};
use explorer_fs::places::{drive_display_name, used_fraction};
use explorer_fs::{load_dir, watch_dir, DirWatcher, FsError, FsErrorKind, LoadEvent, LoadHandle};
use explorer_shell::IconKey;
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
    /// Names to select once the folder is loaded (folder we came from on "up",
    /// pasted items, a new folder).
    pending_select: Vec<String>,
    /// Start inline rename on this name after the next load (new folder).
    pending_rename: Option<String>,
    status_base: String,
    /// Needs a reload when it becomes visible (never loaded, or sort/filter changed).
    stale: bool,
    refresh_allowed_at: Instant,
    refresh_scheduled: bool,
    /// Data behind the rows of a virtual place (trash items, drive roots).
    virtual_data: VirtualData,
    /// Network folder we already showed the login dialog for (no prompt loops).
    login_tried: Option<PathBuf>,
    /// Icon view: one entry per line of tiles. A stable model that only grows
    /// or shrinks at the end – an `int` repeater would rebuild every tile on
    /// each change (Slint compares models by pointer), losing hover and
    /// double-clicks.
    grid: Rc<VecModel<i32>>,
    /// View of this tab: the folder's remembered view, else the default.
    view: ViewMode,
}

/// What stands behind the rows of a virtual place, keyed by `Entry::name`.
#[derive(Default)]
enum VirtualData {
    #[default]
    None,
    Trash(Vec<explorer_ops::trashview::TrashEntry>),
    /// Row name → what it opens ("Dieser PC": drive roots, "Bibliotheken": libraries).
    Targets(HashMap<String, PathBuf>),
    /// One library: its display name and row name → member folder.
    Library(String, HashMap<String, PathBuf>),
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
            pending_select: Vec::new(),
            pending_rename: None,
            status_base: String::new(),
            stale: true,
            refresh_allowed_at: Instant::now(),
            refresh_scheduled: false,
            virtual_data: VirtualData::None,
            login_tried: None,
            grid: Rc::new(VecModel::default()),
            view: ViewMode::Details,
        }
    }

    fn title(&self) -> String {
        if let Some(v) = self.path.as_deref().and_then(VirtualPlace::from_path) {
            return v.label().to_string();
        }
        if let VirtualData::Library(display, _) = &self.virtual_data {
            return display.clone();
        }
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
    rename_index: i32,
    rename_seq: i32,
    rename_text: String,
    rename_sel: i32,
    menu_seq: i32,
    menu_row: i32,
    /// Last visible item range (first, count) reported by the UI.
    vp: (i32, i32),
    /// Items per line as laid out by the UI (1 in details).
    cols: usize,
    /// One entry per column of the icon grid (stable model, like `Tab::grid`).
    cols_model: Rc<VecModel<i32>>,
    /// Selection kept while a Ctrl rubber band is drawn (None = no band).
    band_base: Option<Vec<usize>>,
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
            vp: (0, 60),
            cols: 1,
            cols_model: Rc::new(VecModel::from(vec![0])),
            band_base: None,
            rename_index: -1,
            rename_seq: 0,
            rename_text: String::new(),
            rename_sel: 0,
            menu_seq: 0,
            menu_row: -1,
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
    /// View for folders without a remembered one (settings.ini `view`).
    default_view: ViewMode,
    /// Remembered view per folder (views.ini).
    views: explorer_core::views::FolderViews,
    /// Last Ctrl+wheel size step (wheels/touchpads send bursts of events).
    last_zoom: Option<Instant>,
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
        default_view: ViewMode::Details,
        views: explorer_core::views::FolderViews::new(cfg!(windows)),
        last_zoom: None,
    });
    // Separate cell: used inside selection changes, which already borrow APP.
    static TYPEAHEAD: RefCell<TypeAhead> = RefCell::new(TypeAhead::default());
    /// Paths marked by "cut" (shown faded until pasted or replaced).
    static CUT: RefCell<std::collections::HashSet<PathBuf>> = RefCell::new(Default::default());
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

fn to_row(e: &Entry, dir: &Path, labels: &mut crate::icons::RowLabels) -> Row {
    Row {
        name: labels.display_name(e).into(),
        size: if e.is_dir() {
            Default::default()
        } else {
            format_size(e.size).into()
        },
        modified: e.modified.map(format_datetime).unwrap_or_default().into(),
        kind: labels.type_name(e).into(),
        icon: labels.icon_id(dir, e),
        thumb: 0,
        is_dir: e.is_dir(),
        dim: e.hidden,
        selected: false,
        focused: false,
        cut: false,
    }
}

/// Filter + sort + format (runs on the loader thread).
fn prepare(
    all: Vec<Entry>,
    filter: ViewFilter,
    sort: SortSpec,
    dir: &Path,
) -> (Vec<Entry>, Vec<Row>, usize) {
    prepare_with(all, filter, sort, dir, None)
}

/// `kinds`: text for the "Typ" column per entry name (virtual places show
/// e.g. the original location there); sorting by that column uses it too.
fn prepare_with(
    all: Vec<Entry>,
    filter: ViewFilter,
    sort: SortSpec,
    dir: &Path,
    kinds: Option<&HashMap<String, String>>,
) -> (Vec<Entry>, Vec<Row>, usize) {
    let total = all.len();
    let mut vis: Vec<Entry> = all.into_iter().filter(|e| filter.is_visible(e)).collect();
    let hidden = total - vis.len();
    let mut labels = crate::icons::RowLabels::default();
    labels.show_extensions = filter.show_extensions;
    let kind_of = |labels: &mut crate::icons::RowLabels, e: &Entry| match kinds {
        Some(k) => k.get(&e.name).cloned().unwrap_or_default(),
        None => labels.type_name(e),
    };
    // "Typ" sorts by the names shown in the column (shell names on Windows)
    sort_entries_with_type_names(&mut vis, sort, |e| kind_of(&mut labels, e));
    let rows = vis
        .iter()
        .map(|e| {
            let mut r = to_row(e, dir, &mut labels);
            if kinds.is_some() {
                r.kind = kind_of(&mut labels, e).into();
            }
            r
        })
        .collect();
    (vis, rows, hidden)
}

/// Lists a virtual place (blocking, loader thread).
fn load_virtual(v: VirtualPlace, filter: ViewFilter, sort: SortSpec) -> Update {
    let t0 = Instant::now();
    let (all, kinds, vdata) = match v {
        VirtualPlace::Trash => match explorer_ops::trashview::list() {
            Ok(items) => {
                let kinds = items
                    .iter()
                    .map(|i| {
                        (
                            i.entry.name.clone(),
                            i.original_parent.display().to_string(),
                        )
                    })
                    .collect::<HashMap<_, _>>();
                let all: Vec<Entry> = items.iter().map(|i| i.entry.clone()).collect();
                (all, kinds, VirtualData::Trash(items))
            }
            Err(message) => {
                return Update::Failed(FsError {
                    kind: FsErrorKind::Other,
                    message,
                })
            }
        },
        VirtualPlace::Libraries => {
            if !explorer_shell::SHELL_SUPPORTED {
                return Update::Failed(FsError {
                    kind: FsErrorKind::Other,
                    message: "Bibliotheken gibt es nur unter Windows".into(),
                });
            }
            let mut kinds = HashMap::new();
            let mut targets = HashMap::new();
            let mut icons = HashMap::new();
            let mut all = Vec::new();
            for l in explorer_shell::libraries() {
                let name = unique_name(&targets, &l.display);
                kinds.insert(name.clone(), folder_count(l.folders.len()));
                // a library with one folder opens that folder directly (Explorer)
                let target = match l.folders.as_slice() {
                    [one] => one.clone(),
                    _ => explorer_core::places::library_path(&l.stem),
                };
                targets.insert(name.clone(), target);
                icons.insert(name.clone(), IconKey::File(l.file.clone()));
                all.push(dir_entry(name));
            }
            return finish_virtual(
                all,
                kinds,
                icons,
                VirtualData::Targets(targets),
                filter,
                sort,
                t0,
            );
        }
        VirtualPlace::ThisPc => {
            let drives = explorer_fs::places::drives();
            let mut kinds = HashMap::new();
            let mut targets = HashMap::new();
            let all = drives
                .iter()
                .map(|d| {
                    let name = drive_display_name(d);
                    kinds.insert(
                        name.clone(),
                        d.space.map(|(f, t)| format_usage(f, t)).unwrap_or_default(),
                    );
                    targets.insert(name.clone(), d.root.clone());
                    Entry {
                        name,
                        kind: explorer_core::EntryKind::Dir,
                        size: d.space.map(|(_, t)| t).unwrap_or(0),
                        modified: None,
                        hidden: false,
                        system: false,
                        readonly: false,
                        is_link: false,
                    }
                })
                .collect();
            (all, kinds, VirtualData::Targets(targets))
        }
    };
    // drives get their real icon (system drive, USB stick, DVD …)
    let icons = match &vdata {
        VirtualData::Targets(t) => t
            .iter()
            .map(|(k, v)| (k.clone(), IconKey::File(v.clone())))
            .collect(),
        _ => HashMap::new(),
    };
    finish_virtual(all, kinds, icons, vdata, filter, sort, t0)
}

/// Sorted rows of a virtual listing; `icons` overrides the icon per row name
/// (items do not live under the placeholder path, so the default would be wrong).
fn finish_virtual(
    all: Vec<Entry>,
    kinds: HashMap<String, String>,
    icons: HashMap<String, IconKey>,
    vdata: VirtualData,
    filter: ViewFilter,
    sort: SortSpec,
    t0: Instant,
) -> Update {
    let total = all.len();
    let (entries, mut rows, hidden) = prepare_with(all, filter, sort, Path::new(""), Some(&kinds));
    for (row, e) in rows.iter_mut().zip(&entries) {
        if let Some(k) = icons.get(&e.name) {
            row.icon = crate::icons::registry().id_for(k.clone()) as i32;
        }
    }
    Update::Rows {
        entries,
        rows,
        hidden,
        done: Some((total, t0.elapsed())),
        data_ready: t0.elapsed(),
        vdata,
    }
}

fn dir_entry(name: String) -> Entry {
    Entry {
        name,
        kind: explorer_core::EntryKind::Dir,
        size: 0,
        modified: None,
        hidden: false,
        system: false,
        readonly: false,
        is_link: false,
    }
}

fn folder_count(n: usize) -> String {
    if n == 1 {
        "1 Ordner".into()
    } else {
        format!("{n} Ordner")
    }
}

/// `name`, or `name (2)`, `name (3)` … if a row with that name exists.
fn unique_name<V>(taken: &HashMap<String, V>, name: &str) -> String {
    let mut n = name.to_string();
    let mut i = 2;
    while taken.contains_key(&n) {
        n = format!("{name} ({i})");
        i += 1;
    }
    n
}

/// One library (blocking, loader thread): one folder → open it; several →
/// list them; unknown → error.
fn load_library(stem: &str, filter: ViewFilter, sort: SortSpec) -> Update {
    let t0 = Instant::now();
    let Some(lib) = explorer_shell::library(stem) else {
        return Update::Failed(FsError {
            kind: FsErrorKind::NotFound,
            message: format!("Bibliothek nicht gefunden: {stem}"),
        });
    };
    if let [one] = lib.folders.as_slice() {
        return Update::Redirect(one.clone());
    }
    let mut kinds = HashMap::new();
    let mut targets = HashMap::new();
    let mut icons = HashMap::new();
    let mut all = Vec::new();
    for f in &lib.folders {
        let base = f
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| f.to_string_lossy().into_owned());
        let name = unique_name(&targets, &base);
        kinds.insert(
            name.clone(),
            f.parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
        );
        targets.insert(name.clone(), f.clone());
        icons.insert(name.clone(), IconKey::File(f.clone()));
        all.push(dir_entry(name));
    }
    finish_virtual(
        all,
        kinds,
        icons,
        VirtualData::Library(lib.display, targets),
        filter,
        sort,
        t0,
    )
}

/// Lists the shares of a server (`\\nas`) like a folder (blocking, loader
/// thread). Asks for credentials once if the server wants them.
fn load_shares(
    server: &str,
    dir: &Path,
    filter: ViewFilter,
    sort: SortSpec,
    hwnd: isize,
) -> Update {
    use explorer_fs::network::{self, NetError};
    let t0 = Instant::now();
    let mut res = network::shares(server);
    if let Err(NetError::LoginRequired(_)) = &res {
        trace(&format!("login prompt \\\\{server}"));
        res = match network::connect(server, None, hwnd) {
            Ok(()) => network::shares(server),
            Err(m) => Err(NetError::Other(m)),
        };
    }
    let shares = match res {
        Ok(s) => s,
        Err(e) => {
            return Update::Failed(FsError {
                kind: FsErrorKind::Other,
                message: e.message().to_string(),
            })
        }
    };
    let kinds: HashMap<String, String> = shares
        .iter()
        .map(|s| {
            let k = if s.remark.is_empty() {
                "Freigegebener Ordner".to_string()
            } else {
                s.remark.clone()
            };
            (s.name.clone(), k)
        })
        .collect();
    let all: Vec<Entry> = shares
        .iter()
        .map(|s| Entry {
            name: s.name.clone(),
            kind: explorer_core::EntryKind::Dir,
            size: 0,
            modified: None,
            hidden: false,
            system: false,
            readonly: false,
            is_link: false,
        })
        .collect();
    let total = all.len();
    let (entries, rows, hidden) = prepare_with(all, filter, sort, dir, Some(&kinds));
    Update::Rows {
        entries,
        rows,
        hidden,
        done: Some((total, t0.elapsed())),
        data_ready: t0.elapsed(),
        vdata: VirtualData::None,
    }
}

/// `\\nas` (a server, not a folder): shares are listed, nothing can be created there.
fn is_server(p: &Path) -> bool {
    explorer_fs::network::server_root(&p.to_string_lossy()).is_some()
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
        let lines = if t.view.is_grid() {
            t.model.row_count().div_ceil(p.cols.max(1))
        } else {
            0
        };
        while t.grid.row_count() < lines {
            t.grid.push(t.grid.row_count() as i32);
        }
        while t.grid.row_count() > lines {
            t.grid.remove(t.grid.row_count() - 1);
        }
        let cols = p.cols.max(1);
        while p.cols_model.row_count() < cols {
            p.cols_model.push(p.cols_model.row_count() as i32);
        }
        while p.cols_model.row_count() > cols {
            p.cols_model.remove(p.cols_model.row_count() - 1);
        }
        let place = t.path.as_deref().and_then(VirtualPlace::from_path);
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
            view_mode: t.view.as_str().into(),
            tile_w: t.view.tile().0,
            tile_h: t.view.tile().1,
            img_size: t.view.image_px() as f32,
            grid_lines: ModelRc::from(t.grid.clone()),
            grid_cols: ModelRc::from(p.cols_model.clone()),
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
            rename_index: p.rename_index,
            rename_seq: p.rename_seq,
            rename_text: p.rename_text.as_str().into(),
            rename_sel: p.rename_sel,
            menu_seq: p.menu_seq,
            menu_row: p.menu_row,
            date_label: place
                .map(|v| v.columns().0)
                .unwrap_or("Änderungsdatum")
                .into(),
            type_label: place.map(|v| v.columns().1).unwrap_or("Typ").into(),
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
    let (active, split, path, status, has_sel) = APP.with_borrow(|app| {
        let t = app.panes[app.active_pane].tab();
        (
            app.active_pane,
            app.panes.len() > 1,
            t.path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            status_with_selection(&t.status_base, t.selection.count()),
            t.selection.count() > 0 || t.selection.focus().is_some(),
        )
    });
    show_view(win, tab_view(active));
    win.set_active_pane(active as i32);
    win.set_split(split);
    let place = match VirtualPlace::from_path(Path::new(&path)) {
        Some(VirtualPlace::Trash) => "trash",
        Some(VirtualPlace::ThisPc) => "thispc",
        Some(VirtualPlace::Libraries) => "libraries",
        None if explorer_core::places::is_placeholder(Path::new(&path)) => "libraries",
        None => "",
    };
    win.set_active_path(path.into());
    win.set_status(status.into());
    win.set_has_selection(has_sel);
    win.set_active_place(place.into());
    crate::inspector::changed(win);
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
    filter: ViewFilter,
    sort: SortSpec,
    start: Option<PathBuf>,
    session: Option<Session>,
    view: ViewMode,
) {
    show_view(win, view);
    let show_hidden = filter.show_hidden;
    win.set_opt_show_ext(filter.show_extensions);
    win.set_opt_show_protected(filter.show_protected_os);
    let (pane_model, place_model, drive_model) = APP.with_borrow_mut(|app| {
        app.filter = filter;
        app.sort = sort;
        app.default_view = view;
        app.views = crate::config::load_views();
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
    let usable = |s: &str| match VirtualPlace::from_path(Path::new(s)) {
        Some(v) => Some(v.path()),
        None if explorer_core::places::is_placeholder(Path::new(s)) => Some(PathBuf::from(s)),
        None => explorer_fs::nearest_existing_ancestor(Path::new(s)),
    };
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
            // the folder's own icon (Downloads, Bilder … carry one via desktop.ini)
            icon: crate::icons::eager(IconKey::File(p.path.clone())),
        })
        .chain(std::iter::once(PlaceItem {
            label: VirtualPlace::Trash.label().into(),
            path: VirtualPlace::Trash.path().to_string_lossy().as_ref().into(),
            is_drive: false,
            detail: Default::default(),
            used: 0.0,
            has_bar: false,
            icon: crate::icons::eager(IconKey::Shell(explorer_shell::RECYCLE_BIN.into())),
        }))
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
                                icon: crate::icons::eager(IconKey::File(PathBuf::from(&path))),
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
    let place = VirtualPlace::from_input(&target.to_string_lossy());
    let server = explorer_fs::network::server_root(&target.to_string_lossy());
    let library = explorer_core::places::library_of(&target).map(str::to_string);
    let target = match (place, &server) {
        (Some(v), _) => v.path(),
        (None, Some(s)) => PathBuf::from(format!(r"\\{s}")),
        (None, None) if library.is_some() => target,
        (None, None) => {
            let target = std::path::absolute(&target).unwrap_or(target);
            // show long names like Explorer (and keep path comparisons consistent with the shell)
            explorer_fs::path::long_path(&target)
        }
    };
    let (filter, sort) = APP.with_borrow(|app| (app.filter, app.sort));
    let gen = APP.with_borrow_mut(|app| {
        app.next_gen += 1;
        app.next_gen
    });
    // the folder's remembered view (else the default)
    let view = APP.with_borrow(|app| {
        app.views
            .get(&target.to_string_lossy())
            .unwrap_or(app.default_view)
    });
    let Some(pi) = with_tab(id, |t, pi, _| {
        t.gen = gen;
        if t.view != view {
            trace(&format!("view follows folder mode={}", view.as_str()));
            t.view = view;
        }
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
    if let Some(v) = place {
        // virtual place: one listing on a helper thread, no partial batches
        with_tab(id, |t, _, _| t.loader = None); // cancels a running folder load
        let _ = std::thread::Builder::new()
            .name("virtual-loader".into())
            .spawn(move || {
                let upd = load_virtual(v, filter, sort);
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak.upgrade() {
                        apply(&w, id, gen, path, reason, upd, t0);
                    }
                });
            });
        sync_pane(pi);
        return;
    }
    if let Some(stem) = library {
        with_tab(id, |t, _, _| t.loader = None);
        let _ = std::thread::Builder::new()
            .name("library-loader".into())
            .spawn(move || {
                let upd = load_library(&stem, filter, sort);
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak.upgrade() {
                        apply(&w, id, gen, path, reason, upd, t0);
                    }
                });
            });
        sync_pane(pi);
        return;
    }
    if let Some(server) = server {
        // a server: its shares, on a helper thread (may show the login dialog)
        with_tab(id, |t, _, _| t.loader = None);
        let hwnd = crate::fileops::window_handle(win);
        let _ = std::thread::Builder::new()
            .name("share-loader".into())
            .spawn(move || {
                let upd = load_shares(&server, &path, filter, sort, hwnd);
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak.upgrade() {
                        apply(&w, id, gen, path, reason, upd, t0);
                    }
                });
            });
        sync_pane(pi);
        return;
    }
    let mut acc: Vec<Entry> = Vec::new();
    let path_for_rows = target.clone();
    let handle = load_dir(target, move |ev| {
        let upd = match ev {
            LoadEvent::Batch { entries, first } => {
                acc.extend(entries);
                if !(first && reason == Reason::User) {
                    return; // only the first batch is painted early; refreshes never flash partial lists
                }
                let (entries, rows, hidden) = prepare(acc.clone(), filter, sort, &path_for_rows);
                Update::Rows {
                    entries,
                    rows,
                    hidden,
                    done: None,
                    data_ready: t0.elapsed(),
                    vdata: VirtualData::None,
                }
            }
            LoadEvent::Done { total, .. } => {
                let (entries, rows, hidden) =
                    prepare(std::mem::take(&mut acc), filter, sort, &path_for_rows);
                Update::Rows {
                    entries,
                    rows,
                    hidden,
                    done: Some((total, t0.elapsed())),
                    data_ready: t0.elapsed(),
                    vdata: VirtualData::None,
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
        vdata: VirtualData,
    },
    /// Go there instead (a library with a single folder).
    Redirect(PathBuf),
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
            vdata,
        } => {
            let path_changed = current.as_deref() != Some(path.as_path());
            let n = entries.len();
            let (pi, focus, needs_setup, rename_at) = with_tab(id, |t, pi, _| {
                // New tabs already carry their path → also set up crumbs/watcher on the first load.
                let needs_setup = path_changed || t.watcher.is_none();
                let mut sel = if path_changed {
                    Selection::new(n)
                } else {
                    remap_selection(&t.entries, &t.selection, &entries)
                };
                // Select the folder we came from ("up") or pasted/new items, once fully loaded.
                let mut rename_at = None;
                if done.is_some() && !t.pending_select.is_empty() {
                    let found: Vec<usize> = t
                        .pending_select
                        .iter()
                        .filter_map(|name| entries.iter().position(|e| &e.name == name))
                        .collect();
                    if !found.is_empty() {
                        t.pending_select.clear();
                        sel.clear();
                        for (k, &i) in found.iter().enumerate() {
                            if k == 0 {
                                sel.click(i);
                            } else {
                                sel.ctrl_click(i);
                            }
                        }
                    }
                }
                if done.is_some() {
                    if let Some(name) = t.pending_rename.take() {
                        rename_at = entries.iter().position(|e| e.name == name);
                    }
                }
                CUT.with_borrow(|cut| {
                    if !cut.is_empty() {
                        for (row, e) in rows.iter_mut().zip(&entries) {
                            row.cut = cut.contains(&path.join(&e.name));
                        }
                    }
                });
                for i in sel.selected_indices() {
                    rows[i].selected = true;
                }
                if let Some(f) = sel.focus() {
                    rows[f].focused = true;
                }
                t.entries = entries;
                t.virtual_data = vdata;
                t.login_tried = None;
                // icons of the first screen (per-file ones only load when visible)
                crate::icons::need(rows.iter().take(80).map(|r| r.icon));
                t.model.set_vec(rows);
                let focus = sel.focus();
                t.selection = sel;
                if path_changed {
                    t.path = Some(path.clone());
                }
                (pi, focus, needs_setup, rename_at)
            })
            .unwrap_or((0, None, false, None));
            crate::icons::sync();
            if active_tab_id(pi) == Some(id) {
                refresh_thumbs(win, pi);
            }
            if needs_setup {
                let chars =
                    APP.with_borrow(|app| app.panes.get(pi).map(|p| p.crumb_chars).unwrap_or(60));
                show_crumbs(id, &path, chars);
                if path_changed || reason == Reason::User {
                    request(pi, |p| p.top_seq += 1);
                }
                if !explorer_core::places::is_placeholder(&path) && !is_server(&path) {
                    start_watch(win, id, &path);
                } else {
                    with_tab(id, |t, _, _| t.watcher = None); // refreshed by our own actions
                }
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
                        // names as listed (hidden extensions, …), for tests
                        let shown: Vec<String> = with_tab(id, |t, _, _| {
                            (0..t.model.row_count().min(8))
                                .filter_map(|i| t.model.row_data(i).map(|r| r.name.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                        trace(&format!("shown {shown:?}"));
                        // distinct type names (shell names on Windows), for tests
                        let kinds: Vec<String> = with_tab(id, |t, _, _| {
                            let mut k: Vec<String> = Vec::new();
                            // all rows (folders sort first, so the first screen can be
                            // folders only), stop after a few distinct names
                            for i in 0..t.model.row_count() {
                                if let Some(r) = t.model.row_data(i) {
                                    let s = r.kind.to_string();
                                    if !k.contains(&s) {
                                        k.push(s);
                                        if k.len() >= 10 {
                                            break;
                                        }
                                    }
                                }
                            }
                            k
                        })
                        .unwrap_or_default();
                        trace(&format!("kinds {kinds:?}"));
                    }
                    trace_selection(id);
                }
            }
            sync_pane(pi);
            sync_window(win);
            if let Some(i) = rename_at {
                crate::fileops::begin_rename(win, pi, i);
            }
        }
        Update::Redirect(to) => {
            trace(&format!("redirect {} -> {}", path.display(), to.display()));
            navigate_tab(win, id, to, reason, Hist::Keep);
        }
        Update::Failed(e) => {
            trace(&format!(
                "failed tab={id} gen={gen} {:?} {}",
                e.kind,
                path.display()
            ));
            if matches!(
                e.kind,
                FsErrorKind::PermissionDenied | FsErrorKind::LoginRequired
            ) && ask_login(win, id, &path, reason)
            {
                return; // retried after the login dialog
            }
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
    if explorer_core::places::library_of(path).is_some() {
        let title = with_tab(id, |t, _, _| t.title()).unwrap_or_default();
        let two = vec![
            Crumb {
                label: VirtualPlace::Libraries.label().into(),
                path: VirtualPlace::Libraries
                    .path()
                    .to_string_lossy()
                    .as_ref()
                    .into(),
            },
            Crumb {
                label: title.into(),
                path: path.to_string_lossy().as_ref().into(),
            },
        ];
        with_tab(id, |t, _, _| t.crumbs.set_vec(two));
        return;
    }
    if let Some(v) = VirtualPlace::from_path(path) {
        let one = vec![Crumb {
            label: v.label().into(),
            path: path.to_string_lossy().as_ref().into(),
        }];
        with_tab(id, |t, _, _| t.crumbs.set_vec(one));
        return;
    }
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

/// A network folder refused access: show the Windows login dialog once (on a
/// helper thread, it blocks) and load again. `false` if not a network folder
/// or already tried.
fn ask_login(win: &MainWindow, id: TabId, path: &Path, reason: Reason) -> bool {
    let Some((server, share)) = explorer_fs::network::share_of(&path.to_string_lossy()) else {
        return false;
    };
    if !explorer_fs::network::LOGIN_SUPPORTED {
        return false;
    }
    let first = with_tab(id, |t, _, _| {
        let first = t.login_tried.as_deref() != Some(path);
        t.login_tried = Some(path.to_path_buf());
        first
    })
    .unwrap_or(false);
    if !first {
        return false;
    }
    trace(&format!("login prompt {}", path.display()));
    let hwnd = crate::fileops::window_handle(win);
    let weak = win.as_weak();
    let path = path.to_path_buf();
    let _ = std::thread::Builder::new()
        .name("login".into())
        .spawn(move || {
            let res = explorer_fs::network::connect(&server, Some(&share), hwnd);
            let _ = slint::invoke_from_event_loop(move || {
                let Some(w) = weak.upgrade() else { return };
                match res {
                    Ok(()) => navigate_tab(&w, id, path, reason, Hist::Keep),
                    Err(m) => {
                        w.set_error(format!("{m}: {}", path.display()).into());
                        set_tab_status(&w, id, String::new());
                    }
                }
            });
        });
    true
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

/// Address bar text → path like Explorer: quotes stripped, `%VAR%`, `~` (Unix),
/// `shell:Downloads\x`. Text that already names an existing path is taken as typed.
pub fn resolve_input(s: &str) -> Result<PathBuf, String> {
    use explorer_core::address::{expand_env, expand_home, shell_folder};
    let t = clean_input(s);
    if Path::new(&t).exists() {
        return Ok(PathBuf::from(t));
    }
    let t = expand_env(&t, |n| std::env::var(n).ok());
    let t = if cfg!(windows) {
        t
    } else {
        expand_home(&t, std::env::var("HOME").ok().as_deref())
    };
    if let Some(stem) = t
        .get(..6)
        .filter(|p| p.eq_ignore_ascii_case("shell:"))
        .and_then(|_| explorer_shell::stem_for_shell_name(&t[6..]))
    {
        return Ok(explorer_core::places::library_path(stem));
    }
    match shell_folder(&t) {
        Ok(None) => Ok(PathBuf::from(t)),
        Ok(Some((f, sub))) => explorer_fs::places::known_folder_path(f)
            .map(|p| if sub.is_empty() { p } else { p.join(sub) })
            .ok_or_else(|| format!("Diesen Ort gibt es hier nicht: {t}")),
        Err(name) => Err(format!("Unbekannter Shell-Ort: shell:{name}")),
    }
}

pub fn navigate(win: &MainWindow, pi: usize, target: &str) {
    activate_pane(win, pi);
    let Some(id) = active_tab_id(pi) else { return };
    match resolve_input(target) {
        Ok(p) => navigate_tab(win, id, p, Reason::User, Hist::Push),
        Err(m) => {
            trace(&format!("address error: {m}"));
            win.set_error(m.into());
            cancel_edit(pi);
        }
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
        if explorer_core::places::library_of(p).is_some() {
            return Some(VirtualPlace::Libraries.path());
        }
        if VirtualPlace::from_path(p).is_some() {
            return None; // top level
        }
        // a share's parent is its server, a drive root's parent is "Dieser PC" (Explorer)
        let s = p.to_string_lossy();
        if is_server(p) {
            return Some(VirtualPlace::ThisPc.path());
        }
        let (parent, name) = if let Some(server) = explorer_fs::network::share_root_parent(&s) {
            // `\\nas\film` has no file name in std's eyes (it is the prefix)
            let share = explorer_fs::network::share_of(&s).map(|(_, sh)| sh);
            (PathBuf::from(server), share)
        } else {
            match p.parent() {
                Some(parent) if !parent.as_os_str().is_empty() => (
                    parent.to_path_buf(),
                    p.file_name().map(|n| n.to_string_lossy().into_owned()),
                ),
                _ => return Some(VirtualPlace::ThisPc.path()),
            }
        };
        t.pending_select = name.into_iter().collect();
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
            t.pending_select = cur
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .into_iter()
                .collect();
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

/// Rubber band on empty list space (`phase` 0 start, 1 move, 2 end): the
/// items it touches are selected, with Ctrl in addition to the selection at
/// the start.
#[allow(clippy::too_many_arguments)]
pub fn band(
    win: &MainWindow,
    pi: usize,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    ctrl: bool,
    phase: i32,
) {
    activate_pane(win, pi);
    let Some(id) = active_tab_id(pi) else { return };
    if phase == 0 {
        let base = if ctrl {
            with_tab(id, |t, _, _| t.selection.selected_indices()).unwrap_or_default()
        } else {
            Vec::new()
        };
        APP.with_borrow_mut(|app| {
            if let Some(p) = app.panes.get_mut(pi) {
                p.band_base = Some(base);
            }
        });
    }
    let base = APP.with_borrow(|app| app.panes.get(pi).and_then(|p| p.band_base.clone()));
    let Some(base) = base else { return };
    let g = crate::dnd::geometry(pi).unwrap_or_default();
    let n = with_tab(id, |t, _, _| t.entries.len()).unwrap_or(0);
    let hits = explorer_core::view::band_hits(x0, y0, x1, y1, g.line_h, g.cols, g.item_w, n);
    change_selection(win, pi, |sel, _| sel.set_band(&base, &hits));
    if phase == 2 {
        APP.with_borrow_mut(|app| {
            if let Some(p) = app.panes.get_mut(pi) {
                p.band_base = None;
            }
        });
        trace(&format!("band done pane={pi} hits={}", hits.len()));
    }
}

/// Single-click mode: a click opens. Clicks right after such an open are the
/// rest of a double click and land on whatever the next folder shows under
/// the cursor → ignored (Explorer does the same).
pub fn click_open(win: &MainWindow, pi: usize, index: usize) {
    thread_local! {
        static LAST: std::cell::Cell<Option<Instant>> = const { std::cell::Cell::new(None) };
    }
    const GUARD: Duration = Duration::from_millis(600);
    let now = Instant::now();
    if LAST.get().is_some_and(|t| now.duration_since(t) < GUARD) {
        trace("single-click open ignored (double click)");
        return;
    }
    LAST.set(Some(now));
    trace(&format!("single-click open pane={pi} row={index}"));
    open_entry(win, pi, index);
}

/// Opens entry `index` of pane `pi`: folders navigate, files open with their default app.
pub fn open_entry(win: &MainWindow, pi: usize, index: usize) {
    let Some(id) = active_tab_id(pi) else { return };
    let target = with_tab(id, |t, _, _| {
        let e = t.entries.get(index)?;
        match &t.virtual_data {
            VirtualData::Targets(targets) | VirtualData::Library(_, targets) => {
                Some((targets.get(&e.name)?.clone(), true))
            }
            VirtualData::Trash(_) => None, // items in the trash do not open (restore first)
            VirtualData::None => Some((t.path.as_ref()?.join(&e.name), e.is_dir())),
        }
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

/// Settings panel ("Ordneroptionen"): `name` = show_extensions |
/// show_protected | single_click.
pub fn set_option(win: &MainWindow, name: &str, on: bool) {
    trace(&format!("option {name}={on}"));
    let res = crate::config::update(|s| match name {
        "show_extensions" => s.show_extensions = on,
        "show_protected" => s.show_protected = on,
        "single_click" => s.single_click = on,
        "system_accent" => s.system_accent = on,
        _ => {}
    });
    if let Err(e) = res {
        win.set_error(e.into());
    }
    match name {
        "show_extensions" => {
            APP.with_borrow_mut(|app| app.filter.show_extensions = on);
            invalidate_all(win);
        }
        "show_protected" => {
            APP.with_borrow_mut(|app| app.filter.show_protected_os = on);
            invalidate_all(win);
        }
        "single_click" => win.set_opt_single_click(on),
        "system_accent" => crate::theme::set_use_accent(win, on),
        _ => {}
    }
}

fn show_view(win: &MainWindow, m: ViewMode) {
    win.set_view_mode(m.as_str().into());
}

/// View of the active tab of pane `pi`.
fn tab_view(pi: usize) -> ViewMode {
    active_tab_id(pi)
        .and_then(|id| with_tab(id, |t, _, _| t.view))
        .unwrap_or_default()
}

/// Details or icons (Ctrl+Shift+1/2/3/6, Ctrl+wheel, status bar buttons)
/// for the active tab; remembered for its folder (views.ini).
pub fn set_view(win: &MainWindow, m: ViewMode) {
    let pi = APP.with_borrow(|app| app.active_pane);
    let Some(id) = active_tab_id(pi) else { return };
    let Some((path, focus)) = with_tab(id, |t, _, _| {
        if t.view == m {
            return None;
        }
        t.view = m;
        Some((t.path.clone(), t.selection.focus()))
    })
    .flatten() else {
        return;
    };
    show_view(win, m);
    trace(&format!("view mode={} img={}", m.as_str(), m.image_px()));
    if let Some(p) = path {
        let views = APP.with_borrow_mut(|app| {
            app.views.set(&p.to_string_lossy(), m);
            app.views.clone()
        });
        if let Err(e) = crate::config::save_views(&views) {
            win.set_error(format!("Ansicht nicht gespeichert: {e}").into());
        }
    }
    sync_pane(pi);
    refresh_thumbs(win, pi);
    // keep the focused item in view after the layout changed
    if let Some(f) = focus {
        request(pi, |p| {
            p.scroll_to = f as i32;
            p.scroll_seq += 1;
        });
    }
}

/// Settings: the active tab's view becomes the default and every folder
/// forgets its own view.
pub fn apply_view_all(win: &MainWindow) {
    let m = tab_view(APP.with_borrow(|app| app.active_pane));
    let panes = APP.with_borrow_mut(|app| {
        app.default_view = m;
        app.views.clear();
        for p in &mut app.panes {
            for t in &mut p.tabs {
                t.view = m;
            }
        }
        app.panes.len()
    });
    trace(&format!("view applied to all folders mode={}", m.as_str()));
    let views = APP.with_borrow(|app| app.views.clone());
    if let Err(e) = crate::config::save_views(&views)
        .and_then(|_| crate::config::update(|s| s.view = m).map(|_| ()))
    {
        win.set_error(e.into());
    }
    for pi in 0..panes {
        sync_pane(pi);
        refresh_thumbs(win, pi);
    }
}

/// Ctrl+wheel: one size up/down, at most one step per `ZOOM_GAP` (a notch
/// can arrive as several events, touchpads send many small ones).
pub fn zoom(win: &MainWindow, up: bool) {
    const ZOOM_GAP: Duration = Duration::from_millis(250);
    let now = Instant::now();
    let (allowed, pi) = APP.with_borrow_mut(|app| {
        let allowed = app
            .last_zoom
            .is_none_or(|t| now.duration_since(t) >= ZOOM_GAP);
        if allowed {
            app.last_zoom = Some(now); // measured from the start of a step
        }
        (allowed, app.active_pane)
    });
    let cur = tab_view(pi);
    if allowed {
        set_view(win, cur.step(up));
    }
}

/// Status bar buttons: "details", or "large" = icons (keeps the icon size if already icons).
pub fn set_view_named(win: &MainWindow, name: &str) {
    let cur = tab_view(APP.with_borrow(|app| app.active_pane));
    let m = match name {
        "large" if cur.is_grid() => cur,
        n => ViewMode::parse(n).unwrap_or(cur),
    };
    set_view(win, m);
}

/// Icon view: thumbnail ids for the visible tiles of pane `pi` (requested
/// lazily; rows that already show the right id are left alone).
fn refresh_thumbs(win: &MainWindow, pi: usize) {
    let view = tab_view(pi);
    if !view.is_grid() {
        return;
    }
    let Some(id) = active_tab_id(pi) else { return };
    let (first, count) = APP.with_borrow(|app| app.panes.get(pi).map(|p| p.vp).unwrap_or((0, 60)));
    let px = (view.image_px() as f32 * win.window().scale_factor()).round() as u32;
    let items: Vec<(usize, crate::thumbs::ThumbKey)> = with_tab(id, |t, _, _| {
        let dir = t
            .path
            .as_ref()
            .filter(|p| !explorer_core::places::is_placeholder(p));
        crate::icons::visible_range(first, count, t.entries.len())
            .filter_map(|i| {
                let e = t.entries.get(i)?;
                let path = match (&t.virtual_data, dir) {
                    (VirtualData::Targets(m) | VirtualData::Library(_, m), _) => {
                        m.get(&e.name)?.clone()
                    }
                    (VirtualData::Trash(_), _) => return None,
                    (VirtualData::None, Some(d)) => d.join(&e.name),
                    (VirtualData::None, None) => return None,
                };
                Some((
                    i,
                    crate::thumbs::ThumbKey {
                        path,
                        mtime: e.modified,
                        px,
                    },
                ))
            })
            .collect()
    })
    .unwrap_or_default();
    let (idx, keys): (Vec<usize>, Vec<_>) = items.into_iter().unzip();
    let ids = crate::thumbs::want(keys);
    with_tab(id, |t, _, _| {
        for (i, tid) in idx.into_iter().zip(ids) {
            if let Some(mut row) = t.model.row_data(i) {
                if row.thumb != tid {
                    row.thumb = tid;
                    t.model.set_row_data(i, row);
                }
            }
        }
    });
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
    apply_sort(win, sort);
}

/// Sort menu: pick a column (keeps the direction) or a direction.
pub fn sort_pick(win: &MainWindow, key: Option<&str>, descending: Option<bool>) {
    let sort = APP.with_borrow_mut(|app| {
        if let Some(k) = key.and_then(SortKey::parse) {
            app.sort.key = k;
        }
        if let Some(d) = descending {
            app.sort.descending = d;
        }
        app.sort
    });
    apply_sort(win, sort);
}

fn apply_sort(win: &MainWindow, sort: SortSpec) {
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
    // (rows per page, items per row)
    (page_rows, cols): (i32, i32),
) -> bool {
    let Some(action) = map_key_grid(text, ctrl, shift, alt, page_rows as isize, cols as isize)
    else {
        return false;
    };
    if crate::fileops::blocks_keys(win) {
        return false; // dialog open or inline rename active
    }
    if std::env::var_os("EXPLORER2_TRACE_KEYS").is_some() {
        trace(&format!("key pane={pi} action={action:?} page={page_rows}"));
    }
    activate_pane(win, pi);
    run_action(win, pi, action);
    true
}

/// Executes a list action in pane `pi` (keys and the command palette).
pub fn run_action(win: &MainWindow, pi: usize, action: KeyAction) {
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
        KeyAction::View(m) => set_view(win, m),
        KeyAction::ToggleInspector => crate::inspector::toggle(win),
        KeyAction::Palette => crate::palette::open(win),
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
        KeyAction::Copy => crate::fileops::command(win, pi, "copy"),
        KeyAction::Cut => crate::fileops::command(win, pi, "cut"),
        KeyAction::Paste => crate::fileops::command(win, pi, "paste"),
        KeyAction::Delete { permanent: false } => crate::fileops::command(win, pi, "delete"),
        KeyAction::Delete { permanent: true } => {
            crate::fileops::command(win, pi, "delete-permanent")
        }
        KeyAction::Rename => crate::fileops::command(win, pi, "rename"),
        KeyAction::NewFolder => crate::fileops::command(win, pi, "new-folder"),
        KeyAction::Undo => crate::fileops::command(win, pi, "undo"),
        KeyAction::Redo => crate::fileops::command(win, pi, "redo"),
        KeyAction::Escape => crate::fileops::command(win, pi, "escape"),
        KeyAction::ContextMenu => {
            // the UI computes the position of the focused row and calls back
            let row = focused_index(pi).map(|i| i as i32).unwrap_or(-1);
            request(pi, |p| {
                p.menu_row = row;
                p.menu_seq += 1;
            });
        }
        KeyAction::Properties => crate::fileops::command(win, pi, "properties"),
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
}

/// The list of pane `pi` scrolled/resized: load per-file icons of the visible rows.
/// The UI laid out `cols` items per line (geometry report).
pub fn set_cols(pi: usize, cols: usize) {
    let changed = APP.with_borrow_mut(|app| match app.panes.get_mut(pi) {
        Some(p) if p.cols != cols => {
            p.cols = cols;
            true
        }
        _ => false,
    });
    if changed {
        sync_pane(pi);
    }
}

pub fn viewport(win: &MainWindow, pi: usize, first: i32, count: i32) {
    APP.with_borrow_mut(|app| {
        if let Some(p) = app.panes.get_mut(pi) {
            p.vp = (first, count);
        }
    });
    refresh_thumbs(win, pi);
    let Some(id) = active_tab_id(pi) else { return };
    let ids: Vec<i32> = with_tab(id, |t, _, _| {
        crate::icons::visible_range(first, count, t.model.row_count())
            .filter_map(|i| t.model.row_data(i).map(|r| r.icon))
            .collect()
    })
    .unwrap_or_default();
    crate::icons::need(ids);
}

// ---------------------------------------------------------------- file-operation support

/// Active pane index.
pub(crate) fn active_pane() -> usize {
    APP.with_borrow(|app| app.active_pane)
}

/// Folder shown in pane `pi`.
/// Folder shown in pane `pi`; `None` in virtual places, so file operations
/// (paste, new folder, drops, …) can never act on a placeholder path.
pub(crate) fn pane_dir(pi: usize) -> Option<PathBuf> {
    let id = active_tab_id(pi)?;
    with_tab(id, |t, _, _| t.path.clone())
        .flatten()
        .filter(|p| !explorer_core::places::is_placeholder(p) && !is_server(p))
}

/// Virtual place shown in pane `pi`, if any.
pub(crate) fn pane_place(pi: usize) -> Option<VirtualPlace> {
    let id = active_tab_id(pi)?;
    with_tab(id, |t, _, _| {
        let p = t.path.as_deref()?;
        VirtualPlace::from_path(p)
            .or_else(|| explorer_core::places::library_of(p).map(|_| VirtualPlace::Libraries))
    })
    .flatten()
}

/// Selected (or focused) trash items of pane `pi` (empty outside the trash).
pub(crate) fn selected_trash(pi: usize) -> Vec<explorer_ops::trashview::TrashEntry> {
    let Some(id) = active_tab_id(pi) else {
        return Vec::new();
    };
    with_tab(id, |t, _, _| {
        let VirtualData::Trash(items) = &t.virtual_data else {
            return Vec::new();
        };
        let mut idx = t.selection.selected_indices();
        if idx.is_empty() {
            idx.extend(t.selection.focus());
        }
        idx.iter()
            .filter_map(|&i| t.entries.get(i))
            .filter_map(|e| items.iter().find(|it| it.entry.name == e.name).cloned())
            .collect()
    })
    .unwrap_or_default()
}

/// All items of the trash shown in pane `pi`.
pub(crate) fn all_trash(pi: usize) -> Vec<explorer_ops::trashview::TrashEntry> {
    let Some(id) = active_tab_id(pi) else {
        return Vec::new();
    };
    with_tab(id, |t, _, _| match &t.virtual_data {
        VirtualData::Trash(items) => items.clone(),
        _ => Vec::new(),
    })
    .unwrap_or_default()
}

/// Rows shown in pane `pi` (0 if the pane does not exist).
pub(crate) fn row_count(pi: usize) -> usize {
    active_tab_id(pi)
        .and_then(|id| with_tab(id, |t, _, _| t.model.row_count()))
        .unwrap_or(0)
}

/// Is row `index` of pane `pi` selected?
pub(crate) fn is_selected(pi: usize, index: usize) -> bool {
    active_tab_id(pi)
        .and_then(|id| with_tab(id, |t, _, _| t.selection.is_selected(index)))
        .unwrap_or(false)
}

/// Selected items of pane `pi` (the focused item if nothing is selected).
pub(crate) fn selected_paths(pi: usize) -> Vec<PathBuf> {
    let Some(id) = active_tab_id(pi) else {
        return Vec::new();
    };
    with_tab(id, |t, _, _| {
        let Some(dir) = t
            .path
            .as_ref()
            .filter(|p| !explorer_core::places::is_placeholder(p))
        else {
            return Vec::new();
        };
        let mut idx = t.selection.selected_indices();
        if idx.is_empty() {
            idx.extend(t.selection.focus());
        }
        idx.iter()
            .filter_map(|&i| t.entries.get(i))
            .map(|e| dir.join(&e.name))
            .collect()
    })
    .unwrap_or_default()
}

/// What the inspector shows: the selected items of the active pane, or the
/// folder itself when nothing is selected.
pub(crate) struct InspectTarget {
    /// Changes whenever the shown content may change (pane, tab, load, selection).
    pub key: String,
    /// Real folder of the tab (`None` in virtual places).
    pub dir: Option<PathBuf>,
    /// Tab title (folder name / place label).
    pub title: String,
    /// (path if real, entry, name as listed, type as listed)
    pub items: Vec<(Option<PathBuf>, Entry, String, String)>,
    /// Direct content of the folder (files, folders, bytes of the files).
    pub direct: (usize, usize, u64),
}

pub(crate) fn inspect_target() -> Option<InspectTarget> {
    let pi = APP.with_borrow(|app| app.active_pane);
    let id = active_tab_id(pi)?;
    with_tab(id, |t, _, _| {
        let dir = t
            .path
            .clone()
            .filter(|p| !explorer_core::places::is_placeholder(p) && !is_server(p));
        let sel = t.selection.selected_indices();
        let items = sel
            .iter()
            .filter_map(|&i| {
                let e = t.entries.get(i)?;
                let row = t.model.row_data(i)?;
                let path = match &t.virtual_data {
                    VirtualData::Targets(m) | VirtualData::Library(_, m) => m.get(&e.name).cloned(),
                    VirtualData::Trash(_) => None,
                    VirtualData::None => dir.as_ref().map(|d| d.join(&e.name)),
                };
                Some((path, e.clone(), row.name.to_string(), row.kind.to_string()))
            })
            .collect();
        InspectTarget {
            key: format!("{pi}/{id}/{}/{:?}/{}", t.gen, sel, t.entries.len()),
            dir,
            title: t.title(),
            items,
            direct: explorer_core::inspect::totals(&t.entries),
        }
    })
}

/// Places for the palette's ">" mode: (label, path) of the sidebar places,
/// drives, virtual places and recently visited folders (deduplicated).
pub(crate) fn palette_places() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut push = |label: String, path: String| {
        if !path.is_empty() && !out.iter().any(|(_, p)| *p == path) {
            out.push((label, path));
        }
    };
    APP.with_borrow(|app| {
        for m in [&app.place_model, &app.drive_model] {
            for i in 0..m.row_count() {
                if let Some(p) = m.row_data(i) {
                    push(p.label.to_string(), p.path.to_string());
                }
            }
        }
        for v in VirtualPlace::ALL {
            push(
                v.label().to_string(),
                v.path().to_string_lossy().into_owned(),
            );
        }
        for p in &app.panes {
            for t in &p.tabs {
                for h in t.path.iter().chain(t.history.recent()) {
                    let label = VirtualPlace::from_path(h)
                        .map(|v| v.label().to_string())
                        .or_else(|| h.file_name().map(|n| n.to_string_lossy().into_owned()))
                        .unwrap_or_else(|| h.to_string_lossy().into_owned());
                    push(label, h.to_string_lossy().into_owned());
                }
            }
        }
    });
    out
}

/// Name shown for entry `index` of pane `pi` (may lack a hidden extension).
pub(crate) fn shown_name_at(pi: usize, index: usize) -> Option<String> {
    let id = active_tab_id(pi)?;
    with_tab(id, |t, _, _| {
        t.model.row_data(index).map(|r| r.name.to_string())
    })
    .flatten()
}

/// Entry `index` of pane `pi`: (full path, name, is_dir).
pub(crate) fn entry_at(pi: usize, index: usize) -> Option<(PathBuf, String, bool)> {
    let id = active_tab_id(pi)?;
    with_tab(id, |t, _, _| {
        let e = t.entries.get(index)?;
        let dir = t
            .path
            .as_ref()
            .filter(|p| !explorer_core::places::is_placeholder(p))?;
        Some((dir.join(&e.name), e.name.clone(), e.is_dir()))
    })
    .flatten()
}

/// Index of the focused (or first selected) entry of pane `pi`.
pub(crate) fn focused_index(pi: usize) -> Option<usize> {
    let id = active_tab_id(pi)?;
    with_tab(id, |t, _, _| {
        t.selection
            .focus()
            .filter(|&f| t.selection.is_selected(f))
            .or_else(|| t.selection.selected_indices().first().copied())
            .or(t.selection.focus())
    })
    .flatten()
}

/// Shows (or hides with `index < 0`) the inline rename editor of pane `pi`.
pub(crate) fn show_rename(pi: usize, index: i32, text: &str, sel_end: usize) {
    if index >= 0 {
        request(pi, |p| {
            p.scroll_to = index;
            p.scroll_seq += 1;
        });
    }
    request(pi, |p| {
        p.rename_index = index;
        p.rename_text = text.to_string();
        p.rename_sel = sel_end as i32;
        if index >= 0 {
            p.rename_seq += 1;
        }
    });
}

pub(crate) fn focus_pane(pi: usize) {
    request_focus(pi);
}

/// Replaces the "cut" marks and repaints the affected rows of all loaded tabs.
pub(crate) fn set_cut(paths: &[PathBuf]) {
    CUT.with_borrow_mut(|c| {
        c.clear();
        c.extend(paths.iter().cloned());
    });
    APP.with_borrow(|app| {
        CUT.with_borrow(|cut| {
            for p in &app.panes {
                for t in &p.tabs {
                    let Some(dir) = t.path.as_ref() else { continue };
                    for (i, e) in t.entries.iter().enumerate() {
                        let want = cut.contains(&dir.join(&e.name));
                        if let Some(mut row) = t.model.row_data(i) {
                            if row.cut != want {
                                row.cut = want;
                                t.model.set_row_data(i, row);
                            }
                        }
                    }
                }
            }
        })
    });
}

/// Reloads every tab that shows one of `dirs`; `select` = names to select per folder.
pub(crate) fn refresh_dirs(
    win: &MainWindow,
    dirs: &[PathBuf],
    select: &[(PathBuf, Vec<String>)],
    rename: Option<(&Path, &str)>,
) {
    let ids: Vec<TabId> = APP.with_borrow_mut(|app| {
        let mut ids = Vec::new();
        for p in &mut app.panes {
            for t in &mut p.tabs {
                let Some(path) = t.path.clone() else { continue };
                if !dirs.contains(&path) {
                    continue;
                }
                if let Some((_, names)) = select.iter().find(|(d, _)| *d == path) {
                    t.pending_select = names.clone();
                }
                if let Some((d, name)) = rename {
                    if d == path {
                        t.pending_rename = Some(name.to_string());
                    }
                }
                t.stale = true;
                ids.push(t.id);
            }
        }
        // only visible tabs reload now; hidden ones are stale and reload when shown
        ids.retain(|id| app.panes.iter().any(|p| p.tab().id == *id));
        ids
    });
    for id in ids {
        reload_tab(win, id);
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
    fn address_input_resolves_env_shell_and_home() {
        std::env::set_var("EXPLORER2_TEST_DIR", "/opt/x");
        assert_eq!(
            resolve_input("%EXPLORER2_TEST_DIR%/a").unwrap(),
            PathBuf::from("/opt/x/a")
        );
        assert!(resolve_input("shell:gibtsnicht")
            .unwrap_err()
            .contains("shell:gibtsnicht"));
        if !cfg!(windows) {
            let home = std::env::var("HOME").unwrap();
            assert_eq!(
                resolve_input("~/q").unwrap(),
                PathBuf::from(format!("{home}/q"))
            );
            assert_eq!(
                resolve_input("shell:Profile").unwrap(),
                PathBuf::from(&home)
            );
            assert!(resolve_input("shell:Fonts").is_ok());
            assert!(resolve_input("shell:CommonStartup")
                .unwrap_err()
                .contains("nicht"));
        }
        assert_eq!(
            resolve_input("Papierkorb").unwrap(),
            PathBuf::from("Papierkorb"),
            "virtual alias passes through"
        );
    }

    #[test]
    fn library_helpers() {
        let mut m: HashMap<String, PathBuf> = HashMap::new();
        assert_eq!(unique_name(&m, "Fotos"), "Fotos");
        m.insert("Fotos".into(), PathBuf::new());
        m.insert("Fotos (2)".into(), PathBuf::new());
        assert_eq!(unique_name(&m, "Fotos"), "Fotos (3)");
        assert_eq!(folder_count(1), "1 Ordner");
        assert_eq!(folder_count(3), "3 Ordner");
        assert_eq!(
            resolve_input("shell:DocumentsLibrary").unwrap(),
            explorer_core::places::library_path("Documents")
        );
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
        let (vis, rows, hidden) = prepare(
            all.clone(),
            ViewFilter::default(),
            SortSpec::default(),
            Path::new("/d"),
        );
        assert_eq!(hidden, 1);
        assert_eq!(
            vis.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
            ["dir", "b.txt"]
        );
        if explorer_shell::SHELL_SUPPORTED {
            // shell names depend on the Windows display language ("File folder" / "Dateiordner")
            assert!(!rows[0].kind.is_empty() && rows[0].kind != rows[1].kind);
        } else {
            assert_eq!(rows[0].kind, "Dateiordner");
            assert_eq!(rows[1].kind, "TXT-Datei");
        }
        assert_eq!(rows[1].size, "2 KB");
        assert_eq!(rows[1].modified, "1970-01-01 00:00");
        let show = ViewFilter {
            show_hidden: true,
            ..Default::default()
        };
        let (vis, rows, hidden) = prepare(all, show, SortSpec::default(), Path::new("/d"));
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
