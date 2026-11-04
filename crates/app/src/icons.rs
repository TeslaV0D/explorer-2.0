//! Shell icons and type names for the file list.
//!
//! - Loader threads give every row an `icon` id (`RowLabels::icon_id`) via the
//!   shared [`IconRegistry`]; they never touch images.
//! - The UI holds one image per id (`icons` model, index = id, 0 = none). A
//!   worker thread loads icons from the shell and fills the model in batches;
//!   all rows using an id update by themselves.
//! - Per-extension icons are requested as soon as their id appears (few);
//!   per-file icons (exe, lnk, …) only for visible rows.
//! - Without shell support (Linux) nothing is loaded and rows keep the
//!   built-in fallback look.

use crate::MainWindow;
use explorer_core::{extension, type_label, Entry};
use explorer_shell::{icon_key, IconKey, IconRegistry, IconSize, SHELL_SUPPORTED};
use slint::{ComponentHandle, Model, VecModel};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;
use std::sync::{mpsc, OnceLock};
use std::time::Instant;

pub fn registry() -> &'static IconRegistry {
    static R: OnceLock<IconRegistry> = OnceLock::new();
    R.get_or_init(IconRegistry::new)
}

/// Per-`prepare` cache: avoids a registry lock / shell lookup per row
/// (a 100 000-file folder has only a handful of distinct extensions).
#[derive(Default)]
pub struct RowLabels {
    ext_ids: HashMap<String, u32>,
    folder_id: Option<u32>,
    types: HashMap<String, String>,
    dir_type: Option<String>,
}

impl RowLabels {
    pub fn icon_id(&mut self, dir: &Path, e: &Entry) -> i32 {
        let custom_folder = e.is_dir() && (e.readonly || e.system);
        if e.is_dir() && !custom_folder {
            let id = *self
                .folder_id
                .get_or_insert_with(|| registry().id_for(IconKey::Folder));
            return id as i32;
        }
        let ext = extension(&e.name);
        if !e.is_dir() {
            if let Some(&id) = self.ext_ids.get(ext) {
                return id as i32;
            }
        }
        let key = icon_key(dir, &e.name, e.is_dir(), custom_folder);
        let shared = matches!(key, IconKey::Ext(_));
        let id = registry().id_for(key);
        if shared {
            self.ext_ids.insert(ext.to_string(), id);
        }
        id as i32
    }

    /// Shell type name ("Textdokument"), or the built-in label without shell.
    pub fn type_name(&mut self, e: &Entry) -> String {
        if !SHELL_SUPPORTED {
            return type_label(e);
        }
        if e.is_dir() {
            return self
                .dir_type
                .get_or_insert_with(|| {
                    explorer_shell::type_name("", true).unwrap_or_else(|| type_label(e))
                })
                .clone();
        }
        let ext = extension(&e.name);
        if let Some(t) = self.types.get(ext) {
            return t.clone();
        }
        let t = explorer_shell::type_name(ext, false).unwrap_or_else(|| type_label(e));
        self.types.insert(ext.to_string(), t.clone());
        t
    }
}

// ---------------------------------------------------------------- UI side

struct Ui {
    model: Rc<VecModel<slint::Image>>,
    requested: HashSet<u32>,
    worker: Option<mpsc::Sender<(u32, IconKey)>>,
    loaded: usize,
    started: Option<Instant>,
}

thread_local! {
    static UI: RefCell<Ui> = RefCell::new(Ui {
        model: Rc::new(VecModel::from(vec![slint::Image::default()])),
        requested: HashSet::new(),
        worker: None,
        loaded: 0,
        started: None,
    });
}

type Batch = Vec<(u32, slint::SharedPixelBuffer<slint::Rgba8Pixel>)>;

pub fn init(win: &MainWindow) {
    UI.with_borrow(|u| win.set_icons(u.model.clone().into()));
    if !SHELL_SUPPORTED {
        return;
    }
    let size = IconSize::for_scale(win.window().scale_factor());
    let (tx, rx) = mpsc::channel::<(u32, IconKey)>();
    let weak = win.as_weak();
    let spawned = std::thread::Builder::new()
        .name("icons".into())
        .spawn(move || {
            explorer_shell::init_thread();
            while let Ok(first) = rx.recv() {
                // gather what is queued (up to 64) → one UI update per batch
                let mut jobs = vec![first];
                while jobs.len() < 64 {
                    match rx.try_recv() {
                        Ok(j) => jobs.push(j),
                        Err(_) => break,
                    }
                }
                let batch: Batch = jobs
                    .into_iter()
                    .filter_map(|(id, key)| {
                        let icon = explorer_shell::load_icon(&key, size)?;
                        Some((
                            id,
                            slint::SharedPixelBuffer::clone_from_slice(
                                &icon.rgba,
                                icon.width,
                                icon.height,
                            ),
                        ))
                    })
                    .collect();
                let weak = weak.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if weak.upgrade().is_some() {
                        apply(batch);
                    }
                });
            }
        });
    match spawned {
        Ok(_) => UI.with_borrow_mut(|u| u.worker = Some(tx)),
        Err(e) => crate::nav::trace_line(&format!("icons: worker failed: {e}")),
    }
}

fn apply(batch: Batch) {
    let (n, total, ms) = UI.with_borrow_mut(|u| {
        for (id, buf) in batch.iter() {
            let id = *id as usize;
            if id < u.model.row_count() {
                u.model
                    .set_row_data(id, slint::Image::from_rgba8(buf.clone()));
            }
        }
        u.loaded += batch.len();
        let ms = u
            .started
            .map(|t| t.elapsed().as_secs_f64() * 1000.0)
            .unwrap_or(0.0);
        (batch.len(), u.loaded, ms)
    });
    crate::nav::trace_line(&format!("icons loaded n={n} total={total} after={ms:.1}ms"));
}

fn request(u: &mut Ui, id: u32, key: IconKey) {
    if let Some(tx) = &u.worker {
        if u.requested.insert(id) {
            u.started.get_or_insert_with(Instant::now);
            let _ = tx.send((id, key));
        }
    }
}

/// New ids appeared (rows were prepared): grow the model, request shared icons.
pub fn sync() {
    let reg = registry();
    UI.with_borrow_mut(|u| {
        let have = u.model.row_count();
        let want = reg.len() + 1;
        for id in have..want {
            u.model.push(slint::Image::default());
            if let Some(key) = reg.key(id as u32) {
                if !key.is_individual() {
                    request(u, id as u32, key);
                }
            }
        }
    });
}

/// Rows `ids` are (about to be) visible: load their per-file icons.
pub fn need(ids: impl IntoIterator<Item = i32>) {
    let reg = registry();
    UI.with_borrow_mut(|u| {
        for id in ids {
            if id <= 0 || u.requested.contains(&(id as u32)) {
                continue;
            }
            if let Some(key) = reg.key(id as u32) {
                request(u, id as u32, key);
            }
        }
    });
}

/// Id for an icon shown right away (sidebar): registered and requested now
/// (UI thread). Without shell the id stays empty → fallback square.
pub fn eager(key: IconKey) -> i32 {
    let id = registry().id_for(key) as i32;
    sync();
    need([id]);
    id
}

/// Rows visible around a scroll position (with a margin, so scrolling
/// a few rows never shows placeholders).
pub fn visible_range(first: i32, count: i32, len: usize) -> std::ops::Range<usize> {
    const MARGIN: i32 = 20;
    let start = (first - MARGIN).max(0) as usize;
    let end = ((first + count + MARGIN).max(0) as usize).min(len);
    start.min(end)..end
}

#[cfg(test)]
mod tests {
    use super::*;
    use explorer_core::EntryKind;

    fn entry(name: &str, dir: bool) -> Entry {
        Entry {
            name: name.into(),
            kind: if dir { EntryKind::Dir } else { EntryKind::File },
            size: 0,
            modified: None,
            hidden: false,
            system: false,
            readonly: false,
            is_link: false,
        }
    }

    #[test]
    fn ids_are_shared_per_extension_and_individual_for_programs() {
        let mut l = RowLabels::default();
        let d = Path::new("/x");
        let a = l.icon_id(d, &entry("a.txt", false));
        let b = l.icon_id(d, &entry("b.TXT", false));
        assert_eq!(a, b, "same extension (case-insensitive) → same icon");
        let f1 = l.icon_id(d, &entry("Ordner", true));
        let f2 = l.icon_id(d, &entry("Andere", true));
        assert_eq!(f1, f2, "plain folders share one icon");
        let e1 = l.icon_id(d, &entry("a.exe", false));
        let e2 = l.icon_id(d, &entry("b.exe", false));
        assert_ne!(e1, e2, "programs carry their own icon");
        let mut custom = entry("Custom", true);
        custom.readonly = true;
        assert_ne!(
            l.icon_id(d, &custom),
            f1,
            "folder with desktop.ini attributes"
        );
        // a fresh cache maps to the same registry ids
        assert_eq!(RowLabels::default().icon_id(d, &entry("z.txt", false)), a);
    }

    #[test]
    fn type_names_fall_back_without_shell() {
        let mut l = RowLabels::default();
        if !SHELL_SUPPORTED {
            assert_eq!(l.type_name(&entry("a.txt", false)), "TXT-Datei");
            assert_eq!(l.type_name(&entry("d", true)), "Dateiordner");
        }
    }

    #[test]
    fn visible_range_has_margin_and_bounds() {
        assert_eq!(visible_range(0, 30, 1000), 0..50);
        assert_eq!(visible_range(100, 30, 1000), 80..150);
        assert_eq!(visible_range(990, 30, 1000), 970..1000);
        assert_eq!(visible_range(0, 30, 5), 0..5);
        assert_eq!(visible_range(50, 30, 0), 0..0);
    }
}
