//! Inspector (details pane, Alt+P): preview + facts about the selection of
//! the active pane, or about the folder itself when nothing is selected.
//!
//! Cheap facts are shown at once; preview (shell image / text), picture
//! size, creation time and folder sizes come from a background thread.
//! Every update has a generation; stale results are dropped and a running
//! folder count is cancelled.

use crate::{InfoRow, MainWindow};
use explorer_core::inspect::{attributes, count_text, image_size, text_preview, totals};
use explorer_core::{format_local, format_size};
use explorer_i18n::{tr, tr_n};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::cell::RefCell;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Selection changes this close together are shown once (arrow key repeat).
const DEBOUNCE: Duration = Duration::from_millis(120);
const HEAD_BYTES: usize = 64 * 1024;
const PREVIEW_PX: u32 = 256;

#[derive(Default)]
struct State {
    open: bool,
    gen: u64,
    /// Key of what is shown (skip identical updates).
    shown_key: String,
    scheduled: bool,
    cancel: Option<Arc<AtomicBool>>,
    rows: Rc<VecModel<InfoRow>>,
}

thread_local! {
    static ST: RefCell<State> = RefCell::new(State::default());
}

/// A fact: `id` is fixed (logic, grouping), the label is translated (D-029).
fn row(id: &str, value: impl Into<String>) -> InfoRow {
    let gid = group_of(id);
    InfoRow {
        id: id.into(),
        key: label_of(id).into(),
        value: value.into().into(),
        gid: gid.into(),
        group: group_label(gid).into(),
        first: false,
    }
}

/// Label of a fact id in the current language.
fn label_of(id: &str) -> &'static str {
    match id {
        "size" => tr!("Size"),
        "modified" => tr!("Modified"),
        "created" => tr!("Created"),
        "attributes" => tr!("Attributes"),
        "dimensions" => tr!("Dimensions"),
        "contents" => tr!("Contents"),
        "files-size" => tr!("Size of files"),
        "total" => tr!("Total"),
        "total-size" => tr!("Total size"),
        "unreadable" => tr!("Not readable"),
        "path" => tr!("Path"),
        _ => "",
    }
}

/// Group of a fact in the inspector (redesign R7): general / date / location.
fn group_of(id: &str) -> &'static str {
    match id {
        "modified" | "created" => "date",
        "path" => "location",
        _ => "general",
    }
}

fn group_label(gid: &str) -> &'static str {
    match gid {
        "date" => tr!("DATE"),
        "location" => tr!("LOCATION"),
        _ => tr!("GENERAL"),
    }
}

fn group_rank(gid: &str) -> u8 {
    match gid {
        "general" => 0,
        "date" => 1,
        _ => 2,
    }
}

/// Facts grouped (stable within a group); the first of each group carries the heading.
fn arrange(rows: &mut [InfoRow]) {
    rows.sort_by_key(|r| group_rank(r.gid.as_str()));
    let mut last = String::new();
    for r in rows.iter_mut() {
        r.first = r.gid.as_str() != last;
        last = r.gid.to_string();
    }
}

pub fn init(win: &MainWindow, open: bool) {
    ST.with_borrow_mut(|s| {
        s.open = open;
        win.set_insp_rows(ModelRc::from(s.rows.clone()));
    });
    win.set_insp_open(open);
    changed(win);
}

/// Alt+P / toolbar button.
pub fn toggle(win: &MainWindow) {
    let open = ST.with_borrow_mut(|s| {
        s.open = !s.open;
        s.shown_key.clear();
        s.open
    });
    win.set_insp_open(open);
    crate::nav::trace_line(&format!("inspector open={open}"));
    if let Err(e) = crate::config::update(|s| s.inspector = open) {
        win.set_error(e.into());
    }
    if open {
        changed(win);
    } else {
        cancel_running();
    }
}

/// Window hidden: drop the preview image and the fact rows.
pub fn release(win: &MainWindow) {
    cancel_running();
    ST.with_borrow_mut(|s| {
        s.shown_key.clear();
        s.gen += 1;
        s.rows.set_vec(Vec::new());
    });
    win.set_insp_image(slint::Image::default());
    win.set_insp_has_image(false);
    win.set_insp_text("".into());
}

fn cancel_running() {
    ST.with_borrow_mut(|s| {
        if let Some(c) = s.cancel.take() {
            c.store(true, Ordering::Relaxed);
        }
    });
}

/// Something in the window changed (selection, folder, tab): refresh soon.
pub fn changed(win: &MainWindow) {
    let schedule = ST.with_borrow_mut(|s| {
        if !s.open || s.scheduled {
            return false;
        }
        s.scheduled = true;
        true
    });
    if schedule {
        let weak = win.as_weak();
        slint::Timer::single_shot(DEBOUNCE, move || {
            ST.with_borrow_mut(|s| s.scheduled = false);
            if let Some(w) = weak.upgrade() {
                update(&w);
            }
        });
    }
}

/// Language switched (L4): build the shown facts again.
pub fn relabel(win: &MainWindow) {
    ST.with_borrow_mut(|s| s.shown_key = Default::default());
    changed(win);
}

/// Results of the background part.
#[derive(Default)]
struct Extra {
    rows: Vec<(String, String)>,
    image: Option<slint::SharedPixelBuffer<slint::Rgba8Pixel>>,
    text: Option<String>,
    /// Final (not a progress update).
    done: bool,
}

fn update(win: &MainWindow) {
    let Some(t) = crate::nav::inspect_target() else {
        return;
    };
    let gen = ST.with_borrow_mut(|s| {
        if s.shown_key == t.key {
            return None;
        }
        s.shown_key = t.key.clone();
        if let Some(c) = s.cancel.take() {
            c.store(true, Ordering::Relaxed);
        }
        s.gen += 1;
        Some(s.gen)
    });
    let Some(gen) = gen else { return };
    let cancel = Arc::new(AtomicBool::new(false));
    ST.with_borrow_mut(|s| s.cancel = Some(cancel.clone()));

    win.set_insp_image(slint::Image::default());
    win.set_insp_has_image(false);
    win.set_insp_text("".into());
    let mut rows = Vec::new();
    // background job: (paths to look at, what to compute)
    let mut job: Option<Job> = None;
    match t.items.as_slice() {
        [] => {
            win.set_insp_title(t.title.as_str().into());
            win.set_insp_subtitle(tr!("Folder").into());
            // the open folder: its direct content (already loaded) – walking
            // the tree just because a folder (or C:\) is open would be costly
            let (files, folders, bytes) = t.direct;
            rows.push(row("contents", count_text(files, folders)));
            rows.push(row("files-size", format_size(bytes)));
            if let Some(d) = &t.dir {
                rows.push(row("path", d.display().to_string()));
            }
        }
        [(path, e, shown, kind)] => {
            win.set_insp_title(shown.as_str().into());
            win.set_insp_subtitle(kind.as_str().into());
            if !e.is_dir() {
                let size = if e.size >= 1024 {
                    tr!("{0} ({1} bytes)", format_size(e.size), group(e.size))
                } else {
                    format_size(e.size)
                };
                rows.push(row("size", size));
            }
            if let Some(m) = e.modified {
                rows.push(row("modified", format_local(m)));
            }
            rows.push(row("attributes", attributes(e)));
            if let Some(p) = path {
                if e.is_dir() && p.parent().is_some_and(|q| !q.as_os_str().is_empty()) {
                    rows.push(row("contents", tr!("counting …")));
                }
                rows.push(row("path", p.display().to_string()));
                // a whole drive is never counted automatically
                let drive_root = p.parent().is_none_or(|q| q.as_os_str().is_empty());
                job = Some(if e.is_dir() && drive_root {
                    Job::None
                } else if e.is_dir() {
                    Job::Folders(vec![p.clone()], 0, 0, 0)
                } else {
                    Job::File(
                        p.clone(),
                        (PREVIEW_PX as f32 * win.window().scale_factor()) as u32,
                    )
                });
            }
        }
        many => {
            let (files, folders, bytes) = totals(many.iter().map(|(_, e, _, _)| e));
            win.set_insp_title(
                tr_n!(
                    many.len(),
                    "{0} item selected",
                    "{0} items selected",
                    explorer_core::format_count(many.len())
                )
                .into(),
            );
            win.set_insp_subtitle(count_text(files, folders).into());
            rows.push(row("files-size", format_size(bytes)));
            let dirs: Vec<PathBuf> = many
                .iter()
                .filter(|(p, e, _, _)| e.is_dir() && p.is_some())
                .filter_map(|(p, _, _, _)| p.clone())
                .collect();
            if !dirs.is_empty() {
                rows.push(row("total", tr!("counting …")));
                job = Some(Job::Folders(dirs, files as u64, folders as u64, bytes));
            }
        }
    }
    arrange(&mut rows);
    let summary: Vec<String> = rows
        .iter()
        .map(|r| format!("{}={}", r.key, r.value))
        .collect();
    ST.with_borrow(|s| s.rows.set_vec(rows));
    crate::nav::trace_line(&format!("inspector rows {summary:?}"));
    crate::nav::trace_line(&format!(
        "inspector gen={gen} items={} title={}",
        t.items.len(),
        win.get_insp_title()
    ));
    if let Some(job) = job.filter(|j| !matches!(j, Job::None)) {
        spawn(win, gen, cancel, job);
    }
}

enum Job {
    None,
    /// File + preview size in physical px.
    File(PathBuf, u32),
    /// Folders to count + files/folders/bytes already known (the selected
    /// items themselves in a multi-selection).
    Folders(Vec<PathBuf>, u64, u64, u64),
}

fn spawn(win: &MainWindow, gen: u64, cancel: Arc<AtomicBool>, job: Job) {
    let weak = win.as_weak();
    let send = move |extra: Extra| {
        let weak = weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w) = weak.upgrade() {
                apply(&w, gen, extra);
            }
        });
    };
    let _ = std::thread::Builder::new()
        .name("inspector".into())
        .spawn(move || match job {
            Job::None => {}
            Job::File(p, px) => send(file_facts(&p, px)),
            Job::Folders(dirs, files0, folders0, bytes0) => {
                let mut total = explorer_fs::DirStats::default();
                let mut complete = true;
                for d in &dirs {
                    let base = total;
                    let s = explorer_fs::dir_stats(d, &cancel, Duration::from_millis(250), |p| {
                        let mut sum = base;
                        add(&mut sum, p);
                        send(Extra {
                            rows: folder_rows(&sum, (files0, folders0, bytes0), false),
                            ..Default::default()
                        });
                    });
                    add(&mut total, &s);
                    total.truncated |= s.truncated;
                    complete &= s.complete;
                    if cancel.load(Ordering::Relaxed) {
                        return; // a newer selection took over
                    }
                }
                send(Extra {
                    rows: folder_rows(&total, (files0, folders0, bytes0), complete),
                    done: true,
                    ..Default::default()
                });
            }
        });
}

fn add(sum: &mut explorer_fs::DirStats, s: &explorer_fs::DirStats) {
    sum.files += s.files;
    sum.folders += s.folders;
    sum.bytes += s.bytes;
    sum.unreadable += s.unreadable;
}

/// ("contents", "1,234 files, 56 folders") + "total-size" for one folder;
/// "total" for a multi-selection (incl. the selected files and folders).
fn folder_rows(
    s: &explorer_fs::DirStats,
    (files0, folders0, bytes0): (u64, u64, u64),
    complete: bool,
) -> Vec<(String, String)> {
    let multi = folders0 > 0 || files0 > 0;
    let files = (s.files + files0) as usize;
    let folders = (s.folders + folders0) as usize;
    let more = if complete {
        String::new()
    } else if s.truncated {
        tr!(
            " (more – counting stopped at {0} entries)",
            explorer_core::format_count(explorer_fs::MAX_ENTRIES as usize)
        )
    } else {
        " …".to_string()
    };
    let id = if multi { "total" } else { "contents" };
    let mut v = vec![
        (
            id.to_string(),
            format!("{}{more}", count_text(files, folders)),
        ),
        (
            "total-size".to_string(),
            format!("{}{more}", format_size(s.bytes + bytes0)),
        ),
    ];
    if s.unreadable > 0 {
        v.push((
            "unreadable".into(),
            tr_n!(
                s.unreadable,
                "{0} folder (not counted)",
                "{0} folders (not counted)",
                explorer_core::format_count(s.unreadable as usize)
            ),
        ));
    }
    v
}

/// Created time, picture size, text preview and the shell's image (preview
/// or large icon) of one file.
fn file_facts(p: &Path, px: u32) -> Extra {
    let mut rows = Vec::new();
    if let Ok(md) = std::fs::metadata(p) {
        if let Some(secs) = md
            .created()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        {
            rows.push(("created".to_string(), format_local(secs.as_secs() as i64)));
        }
    }
    let mut head = Vec::with_capacity(HEAD_BYTES);
    if let Ok(f) = std::fs::File::open(p) {
        let _ = f.take(HEAD_BYTES as u64).read_to_end(&mut head);
    }
    let dims = image_size(&head);
    if let Some((w, h)) = dims {
        rows.push(("dimensions".to_string(), tr!("{0} × {1} pixels", w, h)));
    }
    let text = if dims.is_none() {
        text_preview(&head, 40, 200).filter(|t| !t.is_empty())
    } else {
        None
    };
    let image = explorer_shell::thumbnail(p, px).map(|t| {
        let mut buf =
            slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(t.image.width, t.image.height);
        buf.make_mut_bytes().copy_from_slice(&t.image.rgba);
        buf
    });
    Extra {
        rows,
        image,
        text,
        done: true,
    }
}

fn apply(win: &MainWindow, gen: u64, extra: Extra) {
    let current = ST.with_borrow(|s| s.gen == gen && s.open);
    if !current {
        return;
    }
    ST.with_borrow(|s| {
        let mut v: Vec<InfoRow> = (0..slint::Model::row_count(&*s.rows))
            .filter_map(|i| slint::Model::row_data(&*s.rows, i))
            .collect();
        for (k, val) in &extra.rows {
            match v.iter_mut().find(|r| r.id == k.as_str()) {
                Some(r) => r.value = val.as_str().into(),
                None => {
                    // insert new facts before the path (path stays last-ish)
                    let at = v.iter().position(|r| r.id == "path").unwrap_or(v.len());
                    v.insert(at, row(k, val.as_str()));
                }
            }
        }
        arrange(&mut v);
        s.rows.set_vec(v);
    });
    if let Some(buf) = extra.image {
        win.set_insp_image(slint::Image::from_rgba8(buf));
        win.set_insp_has_image(true);
    }
    if let Some(t) = extra.text {
        win.set_insp_text(t.into());
    }
    if extra.done {
        let summary: Vec<String> = ST.with_borrow(|s| {
            (0..slint::Model::row_count(&*s.rows))
                .filter_map(|i| slint::Model::row_data(&*s.rows, i))
                .map(|r| format!("{}={}", r.key, r.value))
                .collect()
        });
        crate::nav::trace_line(&format!(
            "inspector done gen={gen} image={} text={} rows={summary:?}",
            win.get_insp_has_image(),
            !win.get_insp_text().is_empty()
        ));
    }
}

/// 1234567 → "1,234,567" (grouping of the current language).
fn group(n: u64) -> String {
    explorer_i18n::format_integer(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facts_are_grouped_with_one_heading_each() {
        let mut v = vec![
            row("size", "1 KB"),
            row("modified", "today"),
            row("attributes", "-"),
            row("created", "yesterday"),
            row("dimensions", "2 × 2 pixels"),
            row("path", "/x"),
        ];
        arrange(&mut v);
        let ids: Vec<&str> = v.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "size",
                "attributes",
                "dimensions",
                "modified",
                "created",
                "path"
            ]
        );
        // tests see the English source texts (German comes from lang/de)
        assert_eq!(
            (v[0].key.as_str(), v[0].group.as_str()),
            ("Size", "GENERAL")
        );
        assert_eq!(v[5].gid.as_str(), "location");
        let firsts: Vec<bool> = v.iter().map(|r| r.first).collect();
        assert_eq!(firsts, [true, false, false, true, false, true]);
    }

    #[test]
    fn grouping_and_folder_rows() {
        assert_eq!(group(0), "0");
        assert_eq!(group(1234), "1,234");
        assert_eq!(group(1234567), "1,234,567");
        let s = explorer_fs::DirStats {
            files: 3,
            folders: 1,
            bytes: 2048,
            unreadable: 1,
            complete: true,
            truncated: false,
        };
        let r = folder_rows(&s, (0, 0, 0), true);
        assert_eq!(
            r[0],
            ("contents".to_string(), "3 files, 1 folder".to_string())
        );
        assert_eq!(r[1].0, "total-size");
        assert!(r[2].1.starts_with("1 folder"));
        let partial = folder_rows(&s, (2, 1, 10), false);
        assert_eq!(
            partial[0],
            ("total".to_string(), "5 files, 2 folders …".to_string())
        );
    }
}
