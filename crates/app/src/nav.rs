//! Navigation controller. All state lives on the UI thread; directory loading,
//! filtering, sorting and row formatting run on background threads and are
//! applied via `invoke_from_event_loop`, guarded by a generation counter so a
//! stale result can never overwrite a newer navigation.

use crate::keys::{map_key, KeyAction};
use crate::{Crumb, MainWindow, Row};
use explorer_core::breadcrumb::{crumbs, fit};
use explorer_core::selection::Selection;
use explorer_core::typeahead::TypeAhead;
use explorer_core::{
    format_count, format_datetime, format_size, sort_entries, type_label, Entry, SortKey, SortSpec,
    ViewFilter,
};
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
    crumb_model: Rc<VecModel<Crumb>>,
    selection: Selection,
    /// Name to select once the target folder is loaded (e.g. the folder we came from on "up").
    pending_select: Option<String>,
    /// Status text without the selection suffix.
    status_base: String,
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
        crumb_model: Rc::new(VecModel::default()),
        selection: Selection::new(0),
        pending_select: None,
        status_base: String::new(),
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

/// Status bar text: base + selection info ("3 Elemente ausgewählt").
pub fn status_with_selection(base: &str, selected: usize) -> String {
    match selected {
        0 => base.to_string(),
        1 => format!("{base}  |  1 Element ausgewählt"),
        n => format!("{base}  |  {} Elemente ausgewählt", format_count(n)),
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

pub fn init(win: &MainWindow, show_hidden: bool, sort: SortSpec) {
    STATE.with_borrow_mut(|s| {
        s.filter.show_hidden = show_hidden;
        s.sort = sort;
    });
    let (model, crumbs) = STATE.with_borrow(|s| (s.model.clone(), s.crumb_model.clone()));
    win.set_rows(ModelRc::from(model));
    win.set_crumbs(ModelRc::from(crumbs));
    win.set_show_hidden(show_hidden);
    show_sort(win, sort);
}

fn show_sort(win: &MainWindow, sort: SortSpec) {
    win.set_sort_key(sort.key.as_str().into());
    win.set_sort_desc(sort.descending);
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

fn apply(win: &MainWindow, gen: u64, path: PathBuf, reason: Reason, upd: Update, t0: Instant) {
    if STATE.with_borrow(|s| s.gen) != gen {
        return; // stale
    }
    match upd {
        Update::Rows {
            entries,
            mut rows,
            hidden,
            done,
            data_ready,
        } => {
            let changed = STATE.with_borrow(|s| s.path.as_deref() != Some(path.as_path()));
            let n = entries.len();
            let focus = STATE.with_borrow_mut(|s| {
                let mut sel = if changed {
                    Selection::new(n)
                } else {
                    remap_selection(&s.entries, &s.selection, &entries)
                };
                // Select the folder we came from (Explorer behaviour on "up"), once fully loaded.
                if done.is_some() {
                    if let Some(name) = s.pending_select.take() {
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
                s.entries = entries;
                s.model.set_vec(rows);
                let focus = sel.focus();
                s.selection = sel;
                focus
            });
            if changed {
                win.set_current_path(path.to_string_lossy().as_ref().into());
                win.set_editing_path(false);
                show_crumbs(&path);
                win.invoke_scroll_top();
                start_watch(win, &path);
                STATE.with_borrow_mut(|s| s.path = Some(path));
            }
            if let Some(f) = focus {
                win.invoke_ensure_visible(f as i32);
            }
            match done {
                None => {
                    set_status(win, format!("Lädt … {}", format_count(n)));
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
                    set_status(win, st);
                    trace(&format!(
                        "done gen={gen} total={total} shown={n} after {took:?} data_ready={:.1}ms at={:.1}ms",
                        data_ready.as_secs_f64() * 1000.0,
                        since_start_ms()
                    ));
                    trace(&format!("address={}", win.get_current_path()));
                    if trace_enabled() {
                        let order: Vec<String> = STATE.with_borrow(|s| {
                            s.entries.iter().take(8).map(|e| e.name.clone()).collect()
                        });
                        trace(&format!("order {order:?}"));
                    }
                    trace_selection();
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
            // Keep the previous listing; restore the address bar text.
            if let Some(cur) = current {
                win.set_current_path(cur.to_string_lossy().as_ref().into());
                let n = STATE.with_borrow(|s| s.model.row_count());
                set_status(win, count_label(n));
                trace(&format!("address={}", win.get_current_path()));
            } else {
                set_status(win, String::new());
            }
        }
    }
}

fn show_crumbs(path: &Path) {
    let list: Vec<Crumb> = fit(crumbs(&path.to_string_lossy()), 110)
        .into_iter()
        .map(|c| Crumb {
            label: c.label.into(),
            path: c.path.into(),
        })
        .collect();
    STATE.with_borrow(|s| s.crumb_model.set_vec(list));
}

fn set_status(win: &MainWindow, base: String) {
    let n = STATE.with_borrow_mut(|s| {
        s.status_base = base;
        s.selection.count()
    });
    refresh_status(win, n);
}

fn refresh_status(win: &MainWindow, selected: usize) {
    let text = STATE.with_borrow(|s| status_with_selection(&s.status_base, selected));
    win.set_status(text.into());
}

fn trace_selection() {
    if trace_enabled() {
        STATE.with_borrow(|s| {
            let names: Vec<&str> = s
                .selection
                .selected_indices()
                .iter()
                .take(10)
                .map(|&i| s.entries[i].name.as_str())
                .collect();
            let focus = s
                .selection
                .focus()
                .map(|f| s.entries[f].name.as_str())
                .unwrap_or("-");
            trace(&format!(
                "selection count={} focus={focus} names={names:?}",
                s.selection.count()
            ));
        });
    }
}

/// Applies a selection change and repaints only the rows whose state changed.
fn change_selection(win: &MainWindow, f: impl FnOnce(&mut Selection, &[Entry])) {
    let (focus, count) = STATE.with_borrow_mut(|s| {
        let before = s.selection.clone();
        let entries = std::mem::take(&mut s.entries);
        f(&mut s.selection, &entries);
        s.entries = entries;
        let after = &s.selection;
        let mut touched: Vec<usize> = before.selected_indices();
        touched.extend(after.selected_indices());
        touched.extend(before.focus());
        touched.extend(after.focus());
        touched.sort_unstable();
        touched.dedup();
        for i in touched {
            if let Some(mut row) = s.model.row_data(i) {
                let (sel, foc) = (after.is_selected(i), after.focus() == Some(i));
                if row.selected != sel || row.focused != foc {
                    row.selected = sel;
                    row.focused = foc;
                    s.model.set_row_data(i, row);
                }
            }
        }
        (after.focus(), after.count())
    });
    if let Some(i) = focus {
        win.invoke_ensure_visible(i as i32);
    }
    refresh_status(win, count);
    trace_selection();
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

fn reload(win: &MainWindow) {
    if let Some(p) = STATE.with_borrow(|s| s.path.clone()) {
        navigate(win, p, Reason::Refresh);
    }
}

pub fn go_up(win: &MainWindow) {
    let (parent, came_from) = STATE.with_borrow(|s| {
        let p = s.path.as_ref();
        (
            p.and_then(|p| p.parent()).map(Path::to_path_buf),
            p.and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned()),
        )
    });
    if let Some(p) = parent {
        STATE.with_borrow_mut(|s| s.pending_select = came_from);
        navigate(win, p, Reason::User);
    }
}

/// Opens entry `index`: folders navigate, files open with their default app.
pub fn open_entry(win: &MainWindow, index: usize) {
    let target = STATE.with_borrow(|s| {
        let e = s.entries.get(index)?;
        Some((s.path.as_ref()?.join(&e.name), e.is_dir()))
    });
    match target {
        Some((p, true)) => navigate(win, p, Reason::User),
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
    STATE.with_borrow_mut(|s| s.filter.show_hidden = on);
    reload(win);
}

/// Column header click: same column toggles direction, another column sorts ascending.
pub fn sort_by(win: &MainWindow, key: &str) {
    let Some(key) = SortKey::parse(key) else {
        return;
    };
    let sort = STATE.with_borrow_mut(|s| {
        if s.sort.key == key {
            s.sort.descending = !s.sort.descending;
        } else {
            s.sort.key = key;
            s.sort.descending = false;
        }
        s.sort
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
    reload(win);
}

pub fn row_pressed(win: &MainWindow, index: usize, ctrl: bool, shift: bool) {
    change_selection(win, |sel, _| match (ctrl, shift) {
        (_, true) => sel.shift_click(index),
        (true, false) => sel.ctrl_click(index),
        _ => sel.click(index),
    });
}

/// Click on empty space below the rows clears the selection (Explorer behaviour).
pub fn clear_selection(win: &MainWindow) {
    change_selection(win, |sel, _| sel.clear());
}

/// Handles a key in the file list. Returns true if consumed.
pub fn list_key(
    win: &MainWindow,
    text: &str,
    ctrl: bool,
    shift: bool,
    alt: bool,
    page_rows: i32,
) -> bool {
    let Some(action) = map_key(text, ctrl, shift, alt, page_rows as isize) else {
        return false;
    };
    match action {
        KeyAction::Move {
            delta,
            extend,
            focus_only,
        } => change_selection(win, |sel, _| sel.move_by(delta, extend, focus_only)),
        KeyAction::Edge {
            end,
            extend,
            focus_only,
        } => change_selection(win, |sel, _| {
            let target = if end { sel.len() as isize - 1 } else { 0 };
            sel.move_to(target, extend, focus_only)
        }),
        KeyAction::SelectAll => change_selection(win, |sel, _| sel.select_all()),
        KeyAction::ToggleFocused => change_selection(win, |sel, _| {
            if let Some(f) = sel.focus() {
                sel.ctrl_click(f);
            }
        }),
        KeyAction::Open => {
            let targets = STATE.with_borrow(|s| {
                let sel = s.selection.selected_indices();
                if sel.is_empty() {
                    s.selection.focus().into_iter().collect()
                } else {
                    sel
                }
            });
            match targets.as_slice() {
                [] => {}
                [one] => open_entry(win, *one),
                many => {
                    // Several items: open files (max 20, like a safety net); folders are skipped.
                    for &i in many.iter().take(20) {
                        if STATE.with_borrow(|s| s.entries.get(i).is_some_and(|e| !e.is_dir())) {
                            open_entry(win, i);
                        }
                    }
                }
            }
        }
        KeyAction::Up => go_up(win),
        KeyAction::Refresh => reload(win),
        KeyAction::EditPath => win.invoke_start_edit(),
        KeyAction::TypeChar(c) => change_selection(win, |sel, entries| {
            let found = STATE_TYPEAHEAD.with_borrow_mut(|t| {
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

thread_local! {
    // Separate cell: used inside `change_selection`, which already borrows STATE.
    static STATE_TYPEAHEAD: RefCell<TypeAhead> = RefCell::new(TypeAhead::default());
}

/// Esc in the address bar: back to breadcrumbs, restore the real path.
pub fn cancel_edit(win: &MainWindow) {
    if let Some(p) = STATE.with_borrow(|s| s.path.clone()) {
        win.set_current_path(p.to_string_lossy().as_ref().into());
    }
    win.set_editing_path(false);
    trace(&format!("address(cancel)={}", win.get_current_path()));
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
        assert_eq!(rows[0].kind, "Dateiordner");
        assert_eq!(rows[1].size, "2 KB");
        assert_eq!(rows[1].kind, "TXT-Datei");
        assert_eq!(rows[1].modified, "1970-01-01 00:00");
        assert!(!rows[1].selected && !rows[1].focused);

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
        // reloaded + resorted: "c" first, "b" deleted, "new" added
        let new = vec![
            e("c", false, false),
            e("new", false, false),
            e("a", false, false),
        ];
        let r = remap_selection(&old, &sel, &new);
        assert_eq!(r.selected_indices(), [0]);
        assert_eq!(r.focus(), Some(0));
    }
}
