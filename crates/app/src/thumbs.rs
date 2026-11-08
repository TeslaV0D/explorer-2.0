//! Thumbnails for the icon view: requested only for visible tiles, loaded by
//! two worker threads (newest request first, stale ones skipped), kept in the
//! UI's `thumbs` image model within a memory budget (least recently shown
//! are dropped and simply load again when they come back into view).
//!
//! Keys include the file's modification time, so an edited picture gets a
//! new thumbnail instead of the cached one.

use crate::MainWindow;
use explorer_shell::{thumbnail, SHELL_SUPPORTED};
use slint::{Model, VecModel};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex};

/// Decoded thumbnails kept in memory (RGBA bytes).
const BUDGET_BYTES: usize = 96 * 1024 * 1024;
const WORKERS: usize = 2;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ThumbKey {
    pub path: PathBuf,
    pub mtime: Option<i64>,
    /// Physical pixels (image edge × display scale).
    pub px: u32,
}

/// Pending requests, shared with the workers: newest first; `wanted` is the
/// set of ids currently visible (anything else is skipped).
#[derive(Default)]
struct Queue {
    jobs: Vec<(u32, ThumbKey)>,
    wanted: HashSet<u32>,
}

type Shared = Arc<(Mutex<Queue>, Condvar)>;

struct Ui {
    model: Rc<VecModel<slint::Image>>,
    ids: HashMap<ThumbKey, u32>,
    sizes: HashMap<u32, usize>,
    /// Loaded ids, least recently shown first.
    lru: VecDeque<u32>,
    bytes: usize,
    requested: HashSet<u32>,
    queue: Option<Shared>,
    loaded_total: usize,
    /// Real content previews among them (the rest are large icons).
    previews: usize,
}

thread_local! {
    static UI: RefCell<Ui> = RefCell::new(Ui {
        model: Rc::new(VecModel::from(vec![slint::Image::default()])),
        ids: HashMap::new(),
        sizes: HashMap::new(),
        lru: VecDeque::new(),
        bytes: 0,
        requested: HashSet::new(),
        queue: None,
        loaded_total: 0,
        previews: 0,
    });
}

pub fn init(win: &MainWindow) {
    UI.with_borrow(|u| win.set_thumbs(u.model.clone().into()));
    if !SHELL_SUPPORTED {
        return; // tiles keep their icon / fallback
    }
    let shared: Shared = Arc::new((Mutex::new(Queue::default()), Condvar::new()));
    UI.with_borrow_mut(|u| u.queue = Some(shared.clone()));
    for n in 0..WORKERS {
        let shared = shared.clone();
        let _ = std::thread::Builder::new()
            .name(format!("thumbs-{n}"))
            .spawn(move || worker(shared));
    }
}

fn worker(shared: Shared) {
    explorer_shell::init_thread();
    loop {
        let (id, key) = next_job(&shared);
        let img = thumbnail(&key.path, key.px).map(|t| {
            let (w, h) = (t.image.width, t.image.height);
            let mut buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(w, h);
            buf.make_mut_bytes().copy_from_slice(&t.image.rgba);
            (buf, t.preview)
        });
        let _ = slint::invoke_from_event_loop(move || loaded(id, img));
    }
}

/// Newest still-visible request; waits while there is none.
fn next_job(shared: &Shared) -> (u32, ThumbKey) {
    let (lock, cv) = &**shared;
    let mut q = lock.lock().unwrap_or_else(|e| e.into_inner());
    loop {
        while let Some((id, key)) = q.jobs.pop() {
            if q.wanted.contains(&id) {
                return (id, key);
            }
            skipped(id);
        }
        q = cv.wait(q).unwrap_or_else(|e| e.into_inner());
    }
}

/// A request was dropped because its tile scrolled away: allow it again.
fn skipped(id: u32) {
    let _ = slint::invoke_from_event_loop(move || {
        UI.with_borrow_mut(|u| {
            u.requested.remove(&id);
        });
    });
}

fn loaded(id: u32, img: Option<(slint::SharedPixelBuffer<slint::Rgba8Pixel>, bool)>) {
    UI.with_borrow_mut(|u| {
        let Some((buf, preview)) = img else {
            return; // no image: keep `requested`, the tile shows its icon
        };
        let bytes = buf.as_bytes().len();
        if (id as usize) < u.model.row_count() {
            u.model
                .set_row_data(id as usize, slint::Image::from_rgba8(buf));
        }
        u.sizes.insert(id, bytes);
        u.bytes += bytes;
        u.lru.retain(|&x| x != id);
        u.lru.push_back(id);
        u.loaded_total += 1;
        u.previews += preview as usize;
        // over budget: forget the least recently shown (they reload when visible)
        while u.bytes > BUDGET_BYTES && u.lru.len() > 1 {
            let Some(old) = u.lru.pop_front() else { break };
            u.bytes -= u.sizes.remove(&old).unwrap_or(0);
            u.requested.remove(&old);
            if (old as usize) < u.model.row_count() {
                u.model.set_row_data(old as usize, slint::Image::default());
            }
        }
        if u.loaded_total <= 20 || u.loaded_total.is_multiple_of(50) {
            crate::nav::trace_line(&format!(
                "thumbs loaded n={} previews={} kept={} bytes={}",
                u.loaded_total,
                u.previews,
                u.lru.len(),
                u.bytes
            ));
        }
    });
}

/// Tiles `keys` are visible: returns their ids (0 without shell support),
/// requests the ones not loaded yet, and marks them recently shown.
pub fn want(keys: Vec<ThumbKey>) -> Vec<i32> {
    if !SHELL_SUPPORTED {
        return vec![0; keys.len()];
    }
    UI.with_borrow_mut(|u| {
        let mut out = Vec::with_capacity(keys.len());
        let mut new_jobs = Vec::new();
        let mut wanted = HashSet::new();
        for key in keys {
            let next = u.ids.len() as u32 + 1;
            let id = *u.ids.entry(key.clone()).or_insert(next);
            while u.model.row_count() <= id as usize {
                u.model.push(slint::Image::default());
            }
            wanted.insert(id);
            if u.sizes.contains_key(&id) {
                // loaded: most recently shown now
                u.lru.retain(|&x| x != id);
                u.lru.push_back(id);
            } else if u.requested.insert(id) {
                new_jobs.push((id, key));
            }
            out.push(id as i32);
        }
        if let Some(shared) = &u.queue {
            let (lock, cv) = &**shared;
            let mut q = lock.lock().unwrap_or_else(|e| e.into_inner());
            q.wanted = wanted;
            // first visible tile is processed first (popped from the end)
            q.jobs.extend(new_jobs.into_iter().rev());
            cv.notify_all();
        }
        out
    })
}
