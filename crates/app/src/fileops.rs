//! File operations in the UI: clipboard (incl. the Windows system clipboard),
//! background jobs with the operations panel, conflict and confirmation
//! dialogs, inline rename, new folder and undo/redo.
//!
//! All state lives on the UI thread (`thread_local!`); jobs report back via
//! `slint::invoke_from_event_loop`.

use crate::nav::{self, trace_line as trace};
use crate::{JobRow, MainWindow};
use explorer_core::places::VirtualPlace;
use explorer_core::{format_datetime, format_size};
use explorer_ops::clipboard::{self, ClipFiles};
use explorer_ops::exec::{
    Choice, Conflict, Done, Event, JobHandle, JobSpec, Kind, Progress, Report,
};
use explorer_ops::undo::{entry_from_report, undo, Redo, Revert, UndoEntry, UndoStack};
use explorer_ops::{names, simple};
use slint::{ComponentHandle, VecModel};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime};

/// Jobs shorter than this never show up in the panel (no flicker for small copies).
const PANEL_DELAY: Duration = Duration::from_millis(300);

struct Job {
    id: i32,
    handle: JobHandle,
    spec: JobSpec,
    label: &'static str,
    title: String,
    started: Instant,
    progress: Progress,
    paused: bool,
    is_redo: bool,
    shown: bool,
}

/// What a confirmation dialog will do on "OK".
enum Confirm {
    DeletePermanent(Vec<PathBuf>),
    PurgeTrash(Vec<explorer_ops::trashview::TrashEntry>),
}

struct Rename {
    pi: usize,
    path: PathBuf,
    name: String,
    /// Extension not shown in the list (and not in the editor): kept on commit.
    hidden_ext: Option<String>,
}

#[derive(Default)]
struct Ops {
    clip: Option<ClipFiles>,
    /// The last copy/cut went to the OS clipboard (paste reads it from there).
    system_clip: bool,
    undo: UndoStack,
    jobs: Vec<Job>,
    next_id: i32,
    conflicts: VecDeque<(i32, Conflict)>,
    confirm: Option<Confirm>,
    rename: Option<Rename>,
    undo_running: bool,
    job_model: Rc<VecModel<JobRow>>,
}

thread_local! {
    static OPS: RefCell<Ops> = RefCell::new(Ops::default());
}

pub fn init(win: &MainWindow) {
    OPS.with_borrow(|o| win.set_jobs(o.job_model.clone().into()));
    sync_flags(win);
}

/// True while a dialog or the inline rename editor owns the keyboard.
pub fn blocks_keys(win: &MainWindow) -> bool {
    win.get_conflict_open() || win.get_confirm_open() || OPS.with_borrow(|o| o.rename.is_some())
}

fn sync_flags(win: &MainWindow) {
    OPS.with_borrow(|o| {
        win.set_can_paste(cfg!(windows) || o.clip.is_some());
        win.set_can_undo(o.undo.can_undo() && !o.undo_running);
        win.set_can_redo(o.undo.can_redo() && !o.undo_running);
    });
}

fn count_text(paths: &[PathBuf]) -> String {
    match paths {
        [one] => format!(
            "„{}“",
            one.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| one.display().to_string())
        ),
        many => format!("{} Elemente", many.len()),
    }
}

fn dir_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.display().to_string())
}

pub fn job_title(spec: &JobSpec) -> String {
    let what = count_text(&spec.sources);
    let dest = spec.dest.as_deref().map(dir_name).unwrap_or_default();
    match spec.kind {
        Kind::Copy => format!("Kopiere {what} nach „{dest}“"),
        Kind::Move => format!("Verschiebe {what} nach „{dest}“"),
        Kind::Trash => format!("Verschiebe {what} in den Papierkorb"),
        Kind::DeletePermanent => format!("Lösche {what} endgültig"),
    }
}

fn eta_text(secs: f64) -> String {
    let s = secs.ceil() as u64;
    if s < 60 {
        format!("noch {s} s")
    } else if s < 3600 {
        format!("noch {} min", s.div_ceil(60))
    } else {
        format!("noch {} h {} min", s / 3600, (s % 3600) / 60)
    }
}

/// "12,3 MB von 100 MB · 25 MB/s · noch 4 s" (pure, tested).
pub fn job_detail(p: &Progress, elapsed: Duration, paused: bool) -> String {
    if paused {
        return "Angehalten".into();
    }
    if p.total_bytes == 0 {
        return format!("{} von {} Elementen", p.done_items, p.total_items);
    }
    let mut s = format!(
        "{} von {}",
        format_size(p.done_bytes),
        format_size(p.total_bytes)
    );
    let secs = elapsed.as_secs_f64();
    if secs >= 0.5 && p.done_bytes > 0 {
        let rate = p.done_bytes as f64 / secs;
        s.push_str(&format!(" · {}/s", format_size(rate as u64)));
        let left = p.total_bytes.saturating_sub(p.done_bytes) as f64 / rate;
        s.push_str(&format!(" · {}", eta_text(left)));
    }
    s
}

fn job_fraction(p: &Progress) -> f32 {
    if p.total_bytes > 0 {
        p.done_bytes as f32 / p.total_bytes as f32
    } else if p.total_items > 0 {
        p.done_items as f32 / p.total_items as f32
    } else {
        0.0
    }
}

fn sync_jobs() {
    OPS.with_borrow(|o| {
        let rows: Vec<JobRow> = o
            .jobs
            .iter()
            .filter(|j| j.shown)
            .map(|j| JobRow {
                id: j.id,
                title: j.title.as_str().into(),
                detail: job_detail(&j.progress, j.started.elapsed(), j.paused).into(),
                fraction: job_fraction(&j.progress),
                paused: j.paused,
            })
            .collect();
        o.job_model.set_vec(rows);
    });
}

// ---------------------------------------------------------------- commands

/// Toolbar buttons and shortcuts end up here.
pub fn command(win: &MainWindow, pi: usize, cmd: &str) {
    nav::activate_pane(win, pi);
    trace(&format!("command {cmd} pane={pi}"));
    if let Some(place) = nav::pane_place(pi) {
        return place_command(win, pi, place, cmd);
    }
    match cmd {
        "copy" | "cut" => set_clipboard(win, pi, cmd == "cut"),
        "paste" => paste(win, pi),
        "delete" => {
            let paths = nav::selected_paths(pi);
            if !paths.is_empty() {
                start_job(
                    win,
                    JobSpec {
                        kind: Kind::Trash,
                        sources: paths,
                        dest: None,
                    },
                    "Löschen",
                    false,
                );
            }
        }
        "delete-permanent" => {
            let paths = nav::selected_paths(pi);
            if paths.is_empty() {
                return;
            }
            let text = match paths.as_slice() {
                [_] => format!(
                    "{} endgültig löschen?\nDas kann nicht rückgängig gemacht werden.",
                    count_text(&paths)
                ),
                _ => format!(
                    "Diese {} Elemente endgültig löschen?\nDas kann nicht rückgängig gemacht werden.",
                    paths.len()
                ),
            };
            OPS.with_borrow_mut(|o| o.confirm = Some(Confirm::DeletePermanent(paths)));
            win.set_confirm_text(text.into());
            win.set_confirm_ok("Endgültig löschen".into());
            win.set_confirm_open(true);
        }
        "rename" => {
            if let Some(i) = nav::focused_index(pi) {
                begin_rename(win, pi, i);
            }
        }
        "new-folder" => new_folder(win, pi),
        "properties" => {
            let paths = nav::selected_paths(pi);
            let paths = if paths.is_empty() {
                nav::pane_dir(pi).into_iter().collect()
            } else {
                paths
            };
            if let Err(e) = explorer_shell::show_properties(window_handle(win), &paths) {
                win.set_error(e.into());
            }
        }
        "undo" => run_undo(win),
        "redo" => run_redo(win),
        "escape" => {
            // Esc drops the "cut" marks (Explorer behaviour)
            let was_cut = OPS.with_borrow_mut(|o| {
                let cut = o.clip.as_ref().is_some_and(|c| c.cut);
                if cut {
                    o.clip = None;
                }
                cut
            });
            if was_cut {
                nav::set_cut(&[]);
                sync_flags(win);
            }
        }
        _ => {}
    }
}

fn set_clipboard(win: &MainWindow, pi: usize, cut: bool) {
    let paths = nav::selected_paths(pi);
    if paths.is_empty() {
        return;
    }
    let clip = ClipFiles {
        paths: paths.clone(),
        cut,
    };
    let system = clipboard::set_files(&clip);
    trace(&format!(
        "clipboard {} n={} system={system}",
        if cut { "cut" } else { "copy" },
        paths.len()
    ));
    OPS.with_borrow_mut(|o| {
        o.clip = Some(clip);
        o.system_clip = system;
    });
    nav::set_cut(if cut { &paths } else { &[] });
    sync_flags(win);
}

/// What a paste would insert: the OS clipboard wins on Windows (files copied in
/// Explorer), the internal clipboard is the fallback when the OS one failed.
fn clipboard_files() -> Option<ClipFiles> {
    if cfg!(windows) {
        if let Some(c) = clipboard::get_files() {
            return Some(c);
        }
        if OPS.with_borrow(|o| o.system_clip) {
            return None; // replaced by something else (text, image) since our copy
        }
    }
    OPS.with_borrow(|o| o.clip.clone())
}

fn paste(win: &MainWindow, pi: usize) {
    let Some(dest) = nav::pane_dir(pi) else {
        return;
    };
    let Some(clip) = clipboard_files() else {
        trace("paste: clipboard empty");
        return;
    };
    if clip.paths.is_empty() {
        return;
    }
    if clip.cut {
        // a cut is pasted once; afterwards the clipboard is empty (Explorer behaviour)
        OPS.with_borrow_mut(|o| o.clip = None);
        if OPS.with_borrow(|o| o.system_clip) || cfg!(windows) {
            clipboard::clear();
        }
        sync_flags(win);
    }
    let (kind, label) = if clip.cut {
        (Kind::Move, "Verschieben")
    } else {
        (Kind::Copy, "Kopieren")
    };
    start_job(
        win,
        JobSpec {
            kind,
            sources: clip.paths,
            dest: Some(dest),
        },
        label,
        false,
    );
}

// ---------------------------------------------------------------- virtual places

/// Commands inside the recycle bin / "Dieser PC".
fn place_command(win: &MainWindow, pi: usize, place: VirtualPlace, cmd: &str) {
    match (place, cmd) {
        (VirtualPlace::Trash, "restore") => {
            let items = nav::selected_trash(pi);
            if !items.is_empty() {
                trash_action(win, TrashAction::Restore, items);
            }
        }
        (VirtualPlace::Trash, "delete" | "delete-permanent") => {
            let items = nav::selected_trash(pi);
            if !items.is_empty() {
                let text = match items.as_slice() {
                    [one] => format!(
                        "„{}“ endgültig löschen?\nDas kann nicht rückgängig gemacht werden.",
                        one.entry.name
                    ),
                    many => format!(
                        "Diese {} Elemente endgültig löschen?\nDas kann nicht rückgängig gemacht werden.",
                        many.len()
                    ),
                };
                ask_purge(win, text, items);
            }
        }
        (VirtualPlace::Trash, "empty-trash") => {
            let items = nav::all_trash(pi);
            if items.is_empty() {
                win.set_error("Der Papierkorb ist leer.".into());
                return;
            }
            let text = format!(
                "Alle {} Elemente im Papierkorb endgültig löschen?\nDas kann nicht rückgängig gemacht werden.",
                items.len()
            );
            ask_purge(win, text, items);
        }
        (_, "undo") => run_undo(win),
        (_, "redo") => run_redo(win),
        (_, "escape" | "rename" | "properties") => {}
        (_, other) => {
            trace(&format!(
                "command {other} not available in {}",
                place.label()
            ));
            win.set_error(format!("Hier nicht möglich ({}).", place.label()).into());
        }
    }
}

fn ask_purge(win: &MainWindow, text: String, items: Vec<explorer_ops::trashview::TrashEntry>) {
    OPS.with_borrow_mut(|o| o.confirm = Some(Confirm::PurgeTrash(items)));
    win.set_confirm_text(text.into());
    win.set_confirm_ok("Endgültig löschen".into());
    win.set_confirm_open(true);
}

#[derive(Debug, Clone, Copy)]
enum TrashAction {
    Restore,
    Purge,
}

/// Restore / purge on a helper thread (the shell can be slow), then refresh
/// the trash view and the folders the items return to.
fn trash_action(
    win: &MainWindow,
    action: TrashAction,
    items: Vec<explorer_ops::trashview::TrashEntry>,
) {
    trace(&format!("trash {action:?} n={}", items.len()));
    let mut dirs: Vec<PathBuf> = items.iter().map(|i| i.original_parent.clone()).collect();
    dirs.push(VirtualPlace::Trash.path());
    let select: Vec<(PathBuf, Vec<String>)> = match action {
        TrashAction::Restore => items
            .iter()
            .map(|i| (i.original_parent.clone(), vec![i.real_name.clone()]))
            .collect(),
        TrashAction::Purge => Vec::new(),
    };
    let weak = win.as_weak();
    std::thread::spawn(move || {
        let res = match action {
            TrashAction::Restore => explorer_ops::trashview::restore(&items),
            TrashAction::Purge => explorer_ops::trashview::purge(&items),
        };
        let _ = slint::invoke_from_event_loop(move || {
            let Some(w) = weak.upgrade() else { return };
            trace(&format!("trash {action:?} done ok={}", res.is_ok()));
            if let Err(e) = res {
                w.set_error(e.into());
            }
            dirs.sort();
            dirs.dedup();
            nav::refresh_dirs(&w, &dirs, &select, None);
        });
    });
}

/// Moves `paths` to the trash (drop on the recycle bin view).
fn trash_paths(win: &MainWindow, paths: Vec<PathBuf>) {
    let paths: Vec<PathBuf> = paths
        .into_iter()
        .filter(|p| !explorer_core::places::is_placeholder(p))
        .collect();
    if !paths.is_empty() {
        start_job(
            win,
            JobSpec {
                kind: Kind::Trash,
                sources: paths,
                dest: None,
            },
            "Löschen",
            false,
        );
    }
}

// ---------------------------------------------------------------- context menu

/// Native window handle (HWND) for shell dialogs/menus; 0 if unavailable.
pub fn window_handle(win: &MainWindow) -> isize {
    #[cfg(windows)]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let h = win.window().window_handle();
        match h.window_handle().map(|w| w.as_raw()) {
            Ok(RawWindowHandle::Win32(w)) => w.hwnd.get(),
            _ => 0,
        }
    }
    #[cfg(not(windows))]
    {
        let _ = win;
        0
    }
}

/// Right-click / Shift+F10 on row `row` (-1 = folder background) at window position (x, y).
pub fn context_menu(win: &MainWindow, pi: usize, row: i32, x: f32, y: f32) {
    nav::activate_pane(win, pi);
    if row >= 0 && !nav::is_selected(pi, row as usize) {
        // Explorer: right-click on an unselected item selects just that item
        nav::row_pressed(win, pi, row as usize, false, false);
    }
    let Some(dir) = nav::pane_dir(pi) else { return };
    let items = if row >= 0 {
        nav::selected_paths(pi)
    } else {
        Vec::new()
    };
    trace(&format!(
        "context menu pane={pi} row={row} items={}",
        items.len()
    ));
    if !explorer_shell::SHELL_SUPPORTED {
        return; // no shell menu on this platform (own fallback menu: later)
    }
    // The menu runs a modal message loop → start it after this Slint callback returned.
    let weak = win.as_weak();
    slint::Timer::single_shot(std::time::Duration::ZERO, move || {
        if let Some(w) = weak.upgrade() {
            show_menu_now(&w, pi, dir, items, x, y);
        }
    });
}

fn show_menu_now(win: &MainWindow, pi: usize, dir: PathBuf, items: Vec<PathBuf>, x: f32, y: f32) {
    let hwnd = window_handle(win);
    let at = explorer_shell::client_to_screen(hwnd, x, y, win.window().scale_factor());
    let outcome = explorer_shell::show_context_menu(hwnd, &dir, &items, at);
    trace(&format!("context menu result {outcome:?}"));
    use explorer_shell::{MenuOutcome, OwnVerb};
    match outcome {
        MenuOutcome::Own(v) => {
            let cmd = match v {
                OwnVerb::Rename => "rename",
                OwnVerb::Delete => "delete",
                OwnVerb::Cut => "cut",
                OwnVerb::Copy => "copy",
                OwnVerb::Paste => "paste",
                OwnVerb::NewFolder => "new-folder",
            };
            command(win, pi, cmd);
        }
        MenuOutcome::Shell(_) => {
            // shell commands (Send to → ZIP, "New" items, …) may change the folder;
            // the watcher catches it too, this makes it immediate
            nav::refresh_dirs(win, &[dir], &[], None);
        }
        MenuOutcome::Failed(e) => win.set_error(format!("Kontextmenü: {e}").into()),
        MenuOutcome::Dismissed => {}
    }
    nav::focus_pane(pi);
}

// ---------------------------------------------------------------- drag & drop

const DRAG_PREFIX: &str = "explorer2-drag:";

/// Payload of a row drag: which pane/row started it (resolved to paths on drop,
/// so it always reflects the selection at drop time).
pub fn drag_payload(pi: usize, row: usize) -> slint::DataTransfer {
    let mut d = slint::DataTransfer::from(slint::SharedString::from(format!(
        "{DRAG_PREFIX}{pi}:{row}"
    )));
    if let Some((p, _, _)) = nav::entry_at(pi, row) {
        d.set_file_paths([p]);
    }
    d
}

/// Items being dragged: the selection if the dragged row is part of it, else the row.
fn drag_sources(data: &slint::DataTransfer) -> Vec<PathBuf> {
    if let Ok(text) = data.plain_text() {
        if let Some(rest) = text.strip_prefix(DRAG_PREFIX) {
            let mut it = rest.split(':').filter_map(|x| x.parse::<usize>().ok());
            if let (Some(pi), Some(row)) = (it.next(), it.next()) {
                let Some((path, _, _)) = nav::entry_at(pi, row) else {
                    return Vec::new();
                };
                let sel = nav::selected_paths(pi);
                return if sel.contains(&path) { sel } else { vec![path] };
            }
        }
    }
    data.file_paths()
        .map(|it| it.map(Path::to_path_buf).collect())
        .unwrap_or_default()
}

fn drop_target(pi: usize, row: i32) -> Option<PathBuf> {
    if row < 0 {
        return nav::pane_dir(pi);
    }
    match nav::entry_at(pi, row as usize)? {
        (p, _, true) => Some(p),
        _ => None,
    }
}

/// Decides the drop action: 0 = none, 1 = copy, 2 = move (pure → tested).
pub fn drop_decision(sources: &[PathBuf], dest: &Path, copy: bool) -> i32 {
    drop_decision_for(sources, dest, copy, cfg!(windows))
}

/// Windows paths compare case-insensitively ("C:\\A" == "c:\\a").
fn norm(p: &Path, windows: bool) -> PathBuf {
    if windows {
        PathBuf::from(p.to_string_lossy().to_lowercase())
    } else {
        p.to_path_buf()
    }
}

fn drop_decision_for(sources: &[PathBuf], dest: &Path, copy: bool, windows: bool) -> i32 {
    if sources.is_empty() {
        return 0;
    }
    let dest = norm(dest, windows);
    let sources: Vec<PathBuf> = sources.iter().map(|s| norm(s, windows)).collect();
    // never into itself or one of its own subfolders
    if sources.iter().any(|s| dest.starts_with(s)) {
        return 0;
    }
    if copy {
        return 1;
    }
    // moving within the same folder does nothing
    if sources.iter().all(|s| s.parent() == Some(dest.as_path())) {
        return 0;
    }
    2
}

pub fn drop_check(pi: usize, row: i32, data: &slint::DataTransfer, copy: bool) -> i32 {
    if nav::pane_place(pi) == Some(VirtualPlace::Trash) {
        return if drag_sources(data).is_empty() { 0 } else { 2 };
    }
    match drop_target(pi, row) {
        Some(dest) => drop_decision(&drag_sources(data), &dest, copy),
        None => 0,
    }
}

pub fn drop_files(
    win: &MainWindow,
    pi: usize,
    row: i32,
    data: &slint::DataTransfer,
    copy: bool,
) -> i32 {
    if nav::pane_place(pi) == Some(VirtualPlace::Trash) {
        let sources = drag_sources(data);
        trace(&format!("drop on trash n={}", sources.len()));
        trash_paths(win, sources);
        return 2;
    }
    let Some(dest) = drop_target(pi, row) else {
        return 0;
    };
    let sources = drag_sources(data);
    let action = drop_decision(&sources, &dest, copy);
    trace(&format!(
        "drop pane={pi} row={row} n={} action={action} dest={}",
        sources.len(),
        dest.display()
    ));
    let (kind, label) = match action {
        1 => (Kind::Copy, "Kopieren"),
        2 => (Kind::Move, "Verschieben"),
        _ => return 0,
    };
    start_job(
        win,
        JobSpec {
            kind,
            sources,
            dest: Some(dest),
        },
        label,
        false,
    );
    action
}

/// OLE drop on pane `pi` / row `row` (-1 = shown folder): decide (and on
/// `Drop` perform) like Explorer – same drive moves, other drive copies,
/// Ctrl copies, Shift moves; never into itself.
pub fn ole_drop(
    win: &MainWindow,
    pi: usize,
    row: i32,
    ev: &explorer_shell::DropEvent,
) -> explorer_shell::DropEffect {
    use explorer_shell::{default_effect, DropEffect, DropPhase};
    if nav::pane_place(pi) == Some(VirtualPlace::Trash) {
        // dropping on the recycle bin deletes (Explorer)
        if ev.phase == DropPhase::Drop {
            trace(&format!("ole drop on trash n={}", ev.paths.len()));
            trash_paths(win, ev.paths.to_vec());
        }
        return if ev.allow_move {
            DropEffect::Move
        } else {
            DropEffect::None
        };
    }
    let Some(dest) = drop_target(pi, row) else {
        return DropEffect::None;
    };
    let effect = default_effect(ev.paths, &dest, ev.keys, ev.allow_copy, ev.allow_move);
    // same rules as in-app drags (not into itself, no-op moves)
    let copy = effect == DropEffect::Copy;
    if effect == DropEffect::None || drop_decision(ev.paths, &dest, copy) == 0 {
        return DropEffect::None;
    }
    if ev.phase == DropPhase::Drop {
        trace(&format!(
            "ole drop pane={pi} row={row} n={} effect={effect:?} dest={}",
            ev.paths.len(),
            dest.display()
        ));
        let (kind, label) = if copy {
            (Kind::Copy, "Kopieren")
        } else {
            (Kind::Move, "Verschieben")
        };
        start_job(
            win,
            JobSpec {
                kind,
                sources: ev.paths.to_vec(),
                dest: Some(dest),
            },
            label,
            false,
        );
    }
    effect
}

// ---------------------------------------------------------------- rename / new folder

pub fn begin_rename(win: &MainWindow, pi: usize, index: usize) {
    let Some((path, name, is_dir)) = nav::entry_at(pi, index) else {
        return;
    };
    let _ = win;
    // with "hide extensions of known types" the editor shows the name as listed
    let shown = nav::shown_name_at(pi, index).unwrap_or_else(|| name.clone());
    let hidden_ext = explorer_core::display::hidden_ext(&name, &shown).map(str::to_string);
    let text = if hidden_ext.is_some() {
        shown
    } else {
        name.clone()
    };
    let sel = names::rename_selection_len(&text, is_dir);
    trace(&format!(
        "rename start pane={pi} name={name} edit={text} hidden_ext={}",
        hidden_ext.as_deref().unwrap_or("-")
    ));
    nav::show_rename(pi, index as i32, &text, sel);
    OPS.with_borrow_mut(|o| {
        o.rename = Some(Rename {
            pi,
            path,
            name,
            hidden_ext,
        })
    });
}

pub fn rename_commit(win: &MainWindow, pi: usize, text: &str) {
    let Some(r) = OPS.with_borrow_mut(|o| o.rename.take_if(|r| r.pi == pi)) else {
        return;
    };
    nav::show_rename(pi, -1, "", 0);
    let typed = text;
    let full = explorer_core::display::rename_result(typed, r.hidden_ext.as_deref());
    let text = full.as_str();
    if text == r.name {
        trace("rename unchanged");
        nav::focus_pane(pi);
        return;
    }
    match simple::rename(&r.path, text) {
        Ok(new) => {
            trace(&format!("rename ok {} -> {}", r.name, new.display()));
            let report = Report {
                done: vec![Done::Moved {
                    from: r.path.clone(),
                    to: new.clone(),
                }],
                ..Default::default()
            };
            push_undo(
                win,
                entry_from_report(
                    "Umbenennen",
                    &report,
                    Some(Redo::Rename {
                        from: r.path.clone(),
                        to: new.clone(),
                    }),
                ),
                false,
            );
            if let Some(dir) = new.parent() {
                nav::refresh_dirs(
                    win,
                    &[dir.to_path_buf()],
                    &[(dir.to_path_buf(), vec![dir_name(&new)])],
                    None,
                );
            }
            nav::focus_pane(pi);
        }
        Err(msg) => {
            trace(&format!("rename failed: {msg}"));
            win.set_error(msg.into());
            // back into the editor with the typed text (like Explorer)
            // (without the hidden extension: it is added again on the next commit)
            if let Some(i) = nav::focused_index(pi) {
                let len = typed.len();
                nav::show_rename(pi, i as i32, typed, len);
                OPS.with_borrow_mut(|o| o.rename = Some(r));
            }
        }
    }
}

pub fn rename_cancel(pi: usize) {
    if OPS
        .with_borrow_mut(|o| o.rename.take_if(|r| r.pi == pi))
        .is_some()
    {
        trace("rename cancelled");
        nav::show_rename(pi, -1, "", 0);
        nav::focus_pane(pi);
    }
}

fn new_folder(win: &MainWindow, pi: usize) {
    let Some(dir) = nav::pane_dir(pi) else { return };
    match simple::new_folder(&dir) {
        Ok(p) => {
            trace(&format!("new folder {}", p.display()));
            let report = Report {
                done: vec![Done::Created(p.clone())],
                ..Default::default()
            };
            push_undo(
                win,
                entry_from_report("Neuer Ordner", &report, Some(Redo::NewFolder(p.clone()))),
                false,
            );
            let name = dir_name(&p);
            nav::refresh_dirs(
                win,
                std::slice::from_ref(&dir),
                &[(dir.clone(), vec![name.clone()])],
                Some((&dir, &name)),
            );
        }
        Err(msg) => win.set_error(msg.into()),
    }
}

// ---------------------------------------------------------------- jobs

fn start_job(win: &MainWindow, spec: JobSpec, label: &'static str, is_redo: bool) {
    let id = OPS.with_borrow_mut(|o| {
        o.next_id += 1;
        o.next_id
    });
    let title = job_title(&spec);
    trace(&format!(
        "job start id={id} {:?} n={}",
        spec.kind,
        spec.sources.len()
    ));
    let weak = win.as_weak();
    let handle = explorer_ops::spawn(spec.clone(), move |e| {
        let weak = weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w) = weak.upgrade() {
                on_event(&w, id, e);
            }
        });
    });
    OPS.with_borrow_mut(|o| {
        o.jobs.push(Job {
            id,
            handle,
            spec,
            label,
            title,
            started: Instant::now(),
            progress: Progress::default(),
            paused: false,
            is_redo,
            shown: false,
        })
    });
    slint::Timer::single_shot(PANEL_DELAY, move || {
        let shown = OPS.with_borrow_mut(|o| {
            o.jobs
                .iter_mut()
                .find(|j| j.id == id)
                .map(|j| j.shown = true)
                .is_some()
        });
        if shown {
            trace(&format!("job panel shown id={id}"));
            sync_jobs();
        }
    });
}

fn on_event(win: &MainWindow, id: i32, e: Event) {
    match e {
        Event::Started {
            total_items,
            total_bytes,
        } => OPS.with_borrow_mut(|o| {
            if let Some(j) = o.jobs.iter_mut().find(|j| j.id == id) {
                j.progress.total_items = total_items;
                j.progress.total_bytes = total_bytes;
            }
        }),
        Event::Progress(p) => {
            OPS.with_borrow_mut(|o| {
                if let Some(j) = o.jobs.iter_mut().find(|j| j.id == id) {
                    j.progress = p;
                }
            });
            sync_jobs();
        }
        Event::Conflict(c) => {
            trace(&format!("conflict job={id} target={}", c.target.display()));
            OPS.with_borrow_mut(|o| o.conflicts.push_back((id, c)));
            show_next_conflict(win);
        }
        Event::Finished(report) => {
            let job = OPS.with_borrow_mut(|o| {
                o.conflicts.retain(|(j, _)| *j != id);
                let pos = o.jobs.iter().position(|j| j.id == id)?;
                Some(o.jobs.remove(pos))
            });
            sync_jobs();
            if let Some(job) = job {
                finish_job(win, job, report);
            }
        }
    }
}

fn finish_job(win: &MainWindow, job: Job, r: Report) {
    trace(&format!(
        "job done id={} {:?} done={} errors={} skipped={} cancelled={}",
        job.id,
        job.spec.kind,
        r.done.len(),
        r.errors.len(),
        r.skipped,
        r.cancelled
    ));
    for (p, e) in &r.errors {
        trace(&format!("job error {}: {e}", p.display()));
    }
    push_undo(
        win,
        entry_from_report(job.label, &r, Some(Redo::Job(job.spec.clone()))),
        job.is_redo,
    );
    if let Some((p, e)) = r.errors.first() {
        let more = if r.errors.len() > 1 {
            format!(" (+{} weitere Fehler)", r.errors.len() - 1)
        } else {
            String::new()
        };
        win.set_error(format!("{e}: {}{more}", p.display()).into());
    } else if r.cancelled {
        win.set_error("Vorgang abgebrochen.".into());
    }
    if job.spec.kind == Kind::Move {
        nav::set_cut(&[]);
    }
    // refresh source and target folders; select what arrived in the target
    let mut dirs: Vec<PathBuf> = job
        .spec
        .sources
        .iter()
        .filter_map(|s| s.parent().map(Path::to_path_buf))
        .collect();
    let mut select = Vec::new();
    if let Some(dest) = &job.spec.dest {
        dirs.push(dest.clone());
        let arrived: Vec<String> = r
            .done
            .iter()
            .filter_map(|d| match d {
                Done::Created(p) | Done::Moved { to: p, .. } => Some(p),
                _ => None,
            })
            .filter(|p| p.parent() == Some(dest.as_path()))
            .map(|p| dir_name(p))
            .collect();
        if !arrived.is_empty() {
            select.push((dest.clone(), arrived));
        }
    }
    if matches!(job.spec.kind, Kind::Trash)
        || r.done.iter().any(|d| matches!(d, Done::Trashed { .. }))
    {
        dirs.push(VirtualPlace::Trash.path()); // an open recycle bin view shows it at once
    }
    dirs.sort();
    dirs.dedup();
    nav::refresh_dirs(win, &dirs, &select, None);
}

fn push_undo(win: &MainWindow, entry: Option<UndoEntry>, after_redo: bool) {
    if let Some(e) = entry {
        OPS.with_borrow_mut(|o| {
            if after_redo {
                o.undo.push_after_redo(e)
            } else {
                o.undo.push(e)
            }
        });
    }
    sync_flags(win);
}

fn show_next_conflict(win: &MainWindow) {
    if win.get_conflict_open() {
        return;
    }
    let Some(c) = OPS.with_borrow(|o| o.conflicts.front().map(|(_, c)| c.clone())) else {
        return;
    };
    let name = dir_name(&c.target);
    let info = |size: u64, mtime: Option<SystemTime>, is_dir: bool| {
        let date = mtime
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| format_datetime(d.as_secs() as i64))
            .unwrap_or_default();
        if is_dir {
            format!("Ordner · {date}")
        } else {
            format!("{} · {date}", format_size(size))
        }
    };
    let dirs = c.source_is_dir && c.target_is_dir;
    let title = if dirs {
        format!("Der Ordner „{name}“ ist im Ziel bereits vorhanden.")
    } else {
        format!("Im Ziel ist bereits ein Element namens „{name}“ vorhanden.")
    };
    win.set_conflict_title(title.into());
    win.set_conflict_source(
        format!(
            "Neu: {} (aus {})",
            info(c.source_size, c.source_mtime, c.source_is_dir),
            c.source
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default()
        )
        .into(),
    );
    win.set_conflict_target(
        format!(
            "Vorhanden: {}",
            info(c.target_size, c.target_mtime, c.target_is_dir)
        )
        .into(),
    );
    win.set_conflict_dirs(dirs);
    win.set_conflict_all(false);
    win.set_conflict_open(true);
    trace(&format!("conflict dialog name={name} dirs={dirs}"));
}

pub fn conflict_answer(win: &MainWindow, n: i32) {
    let choice = match n {
        0 => Choice::Replace,
        1 => Choice::Skip,
        2 => Choice::KeepBoth,
        _ => Choice::Cancel,
    };
    let all = win.get_conflict_all();
    win.set_conflict_open(false);
    OPS.with_borrow_mut(|o| {
        if let Some((id, _)) = o.conflicts.pop_front() {
            if let Some(j) = o.jobs.iter().find(|j| j.id == id) {
                j.handle.answer(choice, all);
            }
        }
    });
    trace(&format!("conflict answer {choice:?} all={all}"));
    show_next_conflict(win);
    if !win.get_conflict_open() {
        nav::focus_pane(nav::active_pane());
    }
}

pub fn confirm_answer(win: &MainWindow, ok: bool) {
    win.set_confirm_open(false);
    let what = OPS.with_borrow_mut(|o| o.confirm.take());
    trace(&format!("confirm {ok}"));
    match (ok, what) {
        (true, Some(Confirm::DeletePermanent(paths))) => start_job(
            win,
            JobSpec {
                kind: Kind::DeletePermanent,
                sources: paths,
                dest: None,
            },
            "Löschen",
            false,
        ),
        (true, Some(Confirm::PurgeTrash(items))) => trash_action(win, TrashAction::Purge, items),
        _ => {}
    }
    nav::focus_pane(nav::active_pane());
}

pub fn job_pause(id: i32, paused: bool) {
    OPS.with_borrow_mut(|o| {
        if let Some(j) = o.jobs.iter_mut().find(|j| j.id == id) {
            j.handle.set_paused(paused);
            j.paused = paused;
        }
    });
    trace(&format!("job pause id={id} {paused}"));
    sync_jobs();
}

pub fn job_cancel(win: &MainWindow, id: i32) {
    let showing = OPS.with_borrow_mut(|o| {
        if let Some(j) = o.jobs.iter().find(|j| j.id == id) {
            j.handle.cancel();
        }
        let front = o.conflicts.front().map(|(j, _)| *j);
        o.conflicts.retain(|(j, _)| *j != id);
        front == Some(id)
    });
    trace(&format!("job cancel id={id}"));
    if showing {
        win.set_conflict_open(false);
        show_next_conflict(win);
    }
}

/// Window closes: cancel running jobs and wait (bounded) so no temp files stay behind.
pub fn shutdown() {
    let handles: Vec<i32> = OPS.with_borrow(|o| {
        for j in &o.jobs {
            j.handle.cancel();
        }
        o.jobs.iter().map(|j| j.id).collect()
    });
    if handles.is_empty() {
        return;
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline
        && OPS.with_borrow(|o| o.jobs.iter().any(|j| !j.handle.is_finished()))
    {
        std::thread::sleep(Duration::from_millis(10));
    }
    trace(&format!("shutdown: cancelled {} job(s)", handles.len()));
}

// ---------------------------------------------------------------- undo / redo

fn revert_dirs(e: &UndoEntry) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    for r in &e.reverts {
        let paths: Vec<&Path> = match r {
            Revert::TrashCreated(p) => vec![p],
            Revert::MoveBack { from, to } => vec![from, to],
            Revert::RestoreTrashed { original, .. } => vec![original],
        };
        dirs.extend(
            paths
                .into_iter()
                .filter_map(|p| p.parent().map(Path::to_path_buf)),
        );
    }
    dirs.sort();
    dirs.dedup();
    dirs
}

fn run_undo(win: &MainWindow) {
    let entry = OPS.with_borrow_mut(|o| {
        if o.undo_running {
            None
        } else {
            o.undo.pop_undo()
        }
    });
    let Some(entry) = entry else { return };
    OPS.with_borrow_mut(|o| o.undo_running = true);
    sync_flags(win);
    trace(&format!("undo start {}", entry.label));
    let weak = win.as_weak();
    std::thread::spawn(move || {
        let errors = undo(&entry);
        let _ = slint::invoke_from_event_loop(move || {
            OPS.with_borrow_mut(|o| {
                o.undo_running = false;
                if entry.redo.is_some() {
                    o.undo.push_redo(entry.clone());
                }
            });
            trace(&format!(
                "undo done {} errors={}",
                entry.label,
                errors.len()
            ));
            if let Some(w) = weak.upgrade() {
                if let Some(e) = errors.first() {
                    w.set_error(format!("Rückgängig: {e}").into());
                }
                let mut dirs = revert_dirs(&entry);
                dirs.push(VirtualPlace::Trash.path()); // undo moves things in/out of the trash
                nav::refresh_dirs(&w, &dirs, &[], None);
                sync_flags(&w);
            }
        });
    });
}

fn run_redo(win: &MainWindow) {
    let entry = OPS.with_borrow_mut(|o| {
        if o.undo_running {
            None
        } else {
            o.undo.pop_redo()
        }
    });
    let Some(entry) = entry else { return };
    trace(&format!("redo {}", entry.label));
    let label: &'static str = match entry.label.as_str() {
        "Kopieren" => "Kopieren",
        "Verschieben" => "Verschieben",
        "Umbenennen" => "Umbenennen",
        "Neuer Ordner" => "Neuer Ordner",
        _ => "Löschen",
    };
    match entry.redo {
        Some(Redo::Job(spec)) => start_job(win, spec, label, true),
        Some(Redo::Rename { from, to }) => {
            let name = dir_name(&to);
            match simple::rename(&from, &name) {
                Ok(new) => {
                    let report = Report {
                        done: vec![Done::Moved {
                            from: from.clone(),
                            to: new.clone(),
                        }],
                        ..Default::default()
                    };
                    push_undo(
                        win,
                        entry_from_report(
                            label,
                            &report,
                            Some(Redo::Rename {
                                from,
                                to: new.clone(),
                            }),
                        ),
                        true,
                    );
                    if let Some(d) = new.parent() {
                        nav::refresh_dirs(
                            win,
                            &[d.to_path_buf()],
                            &[(d.to_path_buf(), vec![name])],
                            None,
                        );
                    }
                }
                Err(e) => win.set_error(format!("Wiederholen: {e}").into()),
            }
        }
        Some(Redo::NewFolder(p)) => match std::fs::create_dir(&p) {
            Ok(()) => {
                let report = Report {
                    done: vec![Done::Created(p.clone())],
                    ..Default::default()
                };
                push_undo(
                    win,
                    entry_from_report(label, &report, Some(Redo::NewFolder(p.clone()))),
                    true,
                );
                if let Some(d) = p.parent() {
                    nav::refresh_dirs(
                        win,
                        &[d.to_path_buf()],
                        &[(d.to_path_buf(), vec![dir_name(&p)])],
                        None,
                    );
                }
            }
            Err(e) => win.set_error(format!("Wiederholen: {e}").into()),
        },
        None => {}
    }
    sync_flags(win);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_rules() {
        let p = |s: &str| PathBuf::from(s);
        let src = vec![p("/a/x.txt"), p("/a/Ordner")];
        assert_eq!(drop_decision(&src, Path::new("/b"), false), 2);
        assert_eq!(drop_decision(&src, Path::new("/b"), true), 1);
        assert_eq!(
            drop_decision(&src, Path::new("/a"), false),
            0,
            "same folder move"
        );
        assert_eq!(
            drop_decision(&src, Path::new("/a"), true),
            1,
            "same folder copy = Kopie"
        );
        assert_eq!(
            drop_decision(&src, Path::new("/a/Ordner"), false),
            0,
            "into itself"
        );
        assert_eq!(
            drop_decision(&src, Path::new("/a/Ordner/sub"), true),
            0,
            "into own subfolder"
        );
        assert_eq!(drop_decision(&[], Path::new("/b"), false), 0);
        // Windows: case-insensitive (shell paths vs. typed paths)
        let w = vec![p("C:/Daten/x.txt"), p("C:/Daten/Ordner")]; // "/" works on both
        assert_eq!(
            drop_decision_for(&w, Path::new("c:/daten"), false, true),
            0,
            "same folder"
        );
        assert_eq!(
            drop_decision_for(&w, Path::new("C:/DATEN/ordner/sub"), true, true),
            0,
            "into itself"
        );
        assert_eq!(drop_decision_for(&w, Path::new("D:/Ziel"), false, true), 2);
    }

    #[test]
    fn titles() {
        let spec = |kind, n: usize| JobSpec {
            kind,
            sources: (0..n)
                .map(|i| PathBuf::from(format!("/a/f{i}.txt")))
                .collect(),
            dest: Some(PathBuf::from("/b/Ziel")),
        };
        assert_eq!(
            job_title(&spec(Kind::Copy, 1)),
            "Kopiere „f0.txt“ nach „Ziel“"
        );
        assert_eq!(
            job_title(&spec(Kind::Move, 3)),
            "Verschiebe 3 Elemente nach „Ziel“"
        );
        assert_eq!(
            job_title(&spec(Kind::Trash, 2)),
            "Verschiebe 2 Elemente in den Papierkorb"
        );
        assert_eq!(
            job_title(&spec(Kind::DeletePermanent, 1)),
            "Lösche „f0.txt“ endgültig"
        );
    }

    #[test]
    fn details() {
        let p = Progress {
            done_bytes: 50 << 20,
            total_bytes: 100 << 20,
            done_items: 1,
            total_items: 2,
            ..Default::default()
        };
        let d = job_detail(&p, Duration::from_secs(2), false);
        assert!(d.starts_with("50 MB von 100 MB"), "{d}");
        assert!(d.contains("25 MB/s"), "{d}");
        assert!(d.ends_with("noch 2 s"), "{d}");
        assert_eq!(job_detail(&p, Duration::from_secs(2), true), "Angehalten");
        let items = Progress {
            done_items: 3,
            total_items: 10,
            ..Default::default()
        };
        assert_eq!(
            job_detail(&items, Duration::ZERO, false),
            "3 von 10 Elementen"
        );
        assert_eq!(eta_text(125.0), "noch 3 min");
        assert_eq!(eta_text(3700.0), "noch 1 h 1 min");
    }
}
