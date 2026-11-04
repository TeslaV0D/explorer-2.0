//! Inspector (details pane, Alt+P): preview + facts about the selection of
//! the active pane, or about the folder itself when nothing is selected.
//!
//! Cheap facts are shown at once; preview (shell image / text), picture
//! size, creation time and folder sizes come from a background thread.
//! Every update has a generation; stale results are dropped and a running
//! folder count is cancelled.

use crate::{InfoRow, MainWindow};
use explorer_core::inspect::{attributes_de, count_de, image_size, text_preview, totals};
use explorer_core::{format_datetime, format_size};
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

fn row(key: &str, value: impl Into<String>) -> InfoRow {
    InfoRow {
        key: key.into(),
        value: value.into().into(),
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
            win.set_insp_subtitle("Ordner".into());
            // the open folder: its direct content (already loaded) – walking
            // the tree just because a folder (or C:\) is open would be costly
            let (files, folders, bytes) = t.direct;
            rows.push(row("Inhalt", count_de(files, folders)));
            rows.push(row("Größe der Dateien", format_size(bytes)));
            if let Some(d) = &t.dir {
                rows.push(row("Pfad", d.display().to_string()));
            }
        }
        [(path, e, shown, kind)] => {
            win.set_insp_title(shown.as_str().into());
            win.set_insp_subtitle(kind.as_str().into());
            if !e.is_dir() {
                let size = if e.size >= 1024 {
                    format!("{} ({} Bytes)", format_size(e.size), group(e.size))
                } else {
                    format_size(e.size)
                };
                rows.push(row("Größe", size));
            }
            if let Some(m) = e.modified {
                rows.push(row("Geändert", format_datetime(m)));
            }
            rows.push(row("Attribute", attributes_de(e)));
            if let Some(p) = path {
                if e.is_dir() && p.parent().is_some_and(|q| !q.as_os_str().is_empty()) {
                    rows.push(row("Inhalt", "wird gezählt …"));
                }
                rows.push(row("Pfad", p.display().to_string()));
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
                format!(
                    "{} Elemente ausgewählt",
                    explorer_core::format_count(many.len())
                )
                .into(),
            );
            win.set_insp_subtitle(count_de(files, folders).into());
            rows.push(row("Größe der Dateien", format_size(bytes)));
            let dirs: Vec<PathBuf> = many
                .iter()
                .filter(|(p, e, _, _)| e.is_dir() && p.is_some())
                .filter_map(|(p, _, _, _)| p.clone())
                .collect();
            if !dirs.is_empty() {
                rows.push(row("Gesamt", "wird gezählt …"));
                job = Some(Job::Folders(dirs, files as u64, folders as u64, bytes));
            }
        }
    }
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

/// "Inhalt: 1.234 Dateien, 56 Ordner" + "Gesamtgröße" for one folder;
/// "Gesamt" for a multi-selection (incl. the selected files and folders).
fn folder_rows(
    s: &explorer_fs::DirStats,
    (files0, folders0, bytes0): (u64, u64, u64),
    complete: bool,
) -> Vec<(String, String)> {
    let multi = folders0 > 0 || files0 > 0;
    let files = (s.files + files0) as usize;
    let folders = (s.folders + folders0) as usize;
    let more = if complete {
        ""
    } else if s.truncated {
        " (mehr – Zählung bei 500.000 Einträgen beendet)"
    } else {
        " …"
    };
    let label = if multi { "Gesamt" } else { "Inhalt" };
    let mut v = vec![
        (
            label.to_string(),
            format!("{}{more}", count_de(files, folders)),
        ),
        (
            "Gesamtgröße".to_string(),
            format!("{}{more}", format_size(s.bytes + bytes0)),
        ),
    ];
    if s.unreadable > 0 {
        v.push((
            "Nicht lesbar".into(),
            format!("{} Ordner (nicht mitgezählt)", s.unreadable),
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
            rows.push((
                "Erstellt".to_string(),
                format_datetime(secs.as_secs() as i64),
            ));
        }
    }
    let mut head = Vec::with_capacity(HEAD_BYTES);
    if let Ok(f) = std::fs::File::open(p) {
        let _ = f.take(HEAD_BYTES as u64).read_to_end(&mut head);
    }
    let dims = image_size(&head);
    if let Some((w, h)) = dims {
        rows.push(("Abmessungen".to_string(), format!("{w} × {h} Pixel")));
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
            match v.iter_mut().find(|r| r.key == k.as_str()) {
                Some(r) => r.value = val.as_str().into(),
                None => {
                    // insert new facts before "Pfad" (path stays last-ish)
                    let at = v.iter().position(|r| r.key == "Pfad").unwrap_or(v.len());
                    v.insert(at, row(k, val.as_str()));
                }
            }
        }
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

/// 1234567 → "1.234.567" (German grouping).
fn group(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grouping_and_folder_rows() {
        assert_eq!(group(0), "0");
        assert_eq!(group(1234), "1.234");
        assert_eq!(group(1234567), "1.234.567");
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
            ("Inhalt".to_string(), "3 Dateien, 1 Ordner".to_string())
        );
        assert_eq!(r[1].0, "Gesamtgröße");
        assert!(r[2].1.starts_with("1 Ordner"));
        let partial = folder_rows(&s, (2, 1, 10), false);
        assert_eq!(
            partial[0],
            ("Gesamt".to_string(), "5 Dateien, 2 Ordner …".to_string())
        );
    }
}
