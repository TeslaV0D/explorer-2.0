//! Global quick search (Ctrl+Shift+F, PLAN-SEARCH.md S1.4c): an overlay like
//! the command palette over every indexed drive. Enter opens the hit
//! (folders in the active tab, files with their program), Ctrl+Enter /
//! Shift+Enter shows it selected in its folder.

use crate::{nav, MainWindow, PaletteItem};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Hits listed (the best ones; the tab search shows them all).
pub const SHOWN: usize = 50;

#[derive(Default)]
struct State {
    open: bool,
    pane: usize,
    model: Rc<VecModel<PaletteItem>>,
    /// Behind the rows: (full path, is folder).
    hits: Vec<(PathBuf, bool)>,
    /// Newer queries win; results of older ones are dropped.
    gen: u64,
    cancel: Option<Arc<AtomicBool>>,
}

thread_local! {
    static ST: RefCell<State> = RefCell::new(State::default());
    static TIMER: slint::Timer = slint::Timer::default();
}

pub fn init(win: &MainWindow) {
    ST.with_borrow(|s| win.set_qs_items(ModelRc::from(s.model.clone())));
}

pub fn open(win: &MainWindow) {
    if crate::fileops::blocks_keys(win) {
        return; // a dialog or inline rename owns the keyboard
    }
    let pi = nav::active_pane();
    ST.with_borrow_mut(|s| {
        s.open = true;
        s.pane = pi;
        s.hits.clear();
        s.model.set_vec(Vec::new());
    });
    win.set_qs_sel(0);
    win.set_qs_footer(index_summary(win).into());
    win.set_qs_empty("Tippen, um alle Laufwerke zu durchsuchen".into());
    win.set_qs_open(true);
    nav::trace_line("quick search open");
}

pub fn close(win: &MainWindow) {
    TIMER.with(|t| t.stop());
    let pi = ST.with_borrow_mut(|s| {
        s.open = false;
        if let Some(c) = s.cancel.take() {
            c.store(true, Ordering::Relaxed);
        }
        s.pane
    });
    win.set_qs_open(false);
    nav::focus_pane(pi);
    nav::trace_line("quick search close");
}

/// What the index covers (footer while nothing is typed).
fn index_summary(win: &MainWindow) -> String {
    let (n, entries, ready) = crate::search::with_volumes(|v| {
        (
            v.len(),
            v.iter().map(|vol| vol.status().entries).sum::<usize>(),
            v.iter().all(|vol| vol.searchable()),
        )
    });
    if n == 0 {
        return if win.get_opt_index_enabled() {
            "Der Suchindex wird vorbereitet …".into()
        } else {
            "Der Suchindex ist aus (Einstellungen → Suche).".into()
        };
    }
    let drives = if n == 1 {
        "1 Laufwerk".to_string()
    } else {
        format!("{n} Laufwerke")
    };
    let mut s = format!(
        "{drives}, {} Einträge",
        explorer_core::format_count(entries)
    );
    if !ready {
        s.push_str(" (wird noch aufgebaut)");
    }
    s
}

/// Typing: searches once typing pauses a moment.
pub fn query(win: &MainWindow, text: &str) {
    let weak = win.as_weak();
    let text = text.to_string();
    TIMER.with(|t| {
        t.start(
            slint::TimerMode::SingleShot,
            Duration::from_millis(120),
            move || {
                if let Some(w) = weak.upgrade() {
                    run_query(&w, &text);
                }
            },
        )
    });
}

fn run_query(win: &MainWindow, text: &str) {
    let cancel = Arc::new(AtomicBool::new(false));
    let gen = ST.with_borrow_mut(|s| {
        s.gen += 1;
        if let Some(old) = s.cancel.replace(cancel.clone()) {
            old.store(true, Ordering::Relaxed);
        }
        s.gen
    });
    let text = text.trim().to_string();
    if text.is_empty() {
        ST.with_borrow_mut(|s| {
            s.hits.clear();
            s.model.set_vec(Vec::new());
        });
        win.set_qs_footer(index_summary(win).into());
        win.set_qs_empty("Tippen, um alle Laufwerke zu durchsuchen".into());
        return;
    }
    let weak = win.as_weak();
    let sort = explorer_core::SortSpec {
        key: explorer_core::SortKey::Name,
        descending: false,
        dirs_first: false,
    };
    let _ = std::thread::Builder::new()
        .name("quick-search".into())
        .spawn(move || {
            let t0 = std::time::Instant::now();
            let res = crate::search::find(None, &text, sort, &cancel);
            let ms = t0.elapsed().as_secs_f64() * 1000.0;
            let _ = slint::invoke_from_event_loop(move || {
                let Some(w) = weak.upgrade() else { return };
                let current = ST.with_borrow(|s| s.open && s.gen == gen);
                if current {
                    show(&w, &text, res, ms);
                }
            });
        });
}

fn show(win: &MainWindow, text: &str, res: Result<crate::search::Found, String>, ms: f64) {
    let found = match res {
        Ok(f) => f,
        Err(e) => {
            ST.with_borrow_mut(|s| {
                s.hits.clear();
                s.model.set_vec(Vec::new());
            });
            win.set_qs_empty(e.into());
            win.set_qs_footer("".into());
            return;
        }
    };
    let mut items = found.items;
    rank(&mut items, text);
    items.truncate(SHOWN);
    nav::trace_line(&format!(
        "quick search query={text:?} total={} shown={} first={:?} in {ms:.1}ms",
        found.total,
        items.len(),
        items.first().map(|i| i.0.name.as_str())
    ));
    let rows: Vec<PaletteItem> = items
        .iter()
        .map(|(e, p)| PaletteItem {
            label: e.name.as_str().into(),
            detail: p
                .parent()
                .map(|d| d.to_string_lossy().into_owned())
                .unwrap_or_default()
                .into(),
            enabled: true,
        })
        .collect();
    ST.with_borrow_mut(|s| {
        s.hits = items.iter().map(|(e, p)| (p.clone(), e.is_dir())).collect();
        s.model.set_vec(rows);
    });
    win.set_qs_sel(0);
    win.set_qs_empty(
        if crate::search::with_volumes(|v| v.is_empty()) {
            "Ohne Suchindex gibt es hier nichts zu finden (Einstellungen → Suche)."
        } else {
            "Nichts gefunden"
        }
        .into(),
    );
    let total = explorer_core::format_count(found.total);
    win.set_qs_footer(
        match found.total {
            0 => String::new(),
            n if n > items.len() => format!(
                "{} von {total} Treffern · Enter öffnet · Strg+Enter zeigt im Ordner",
                items.len()
            ),
            _ => format!("{total} Treffer · Enter öffnet · Strg+Enter zeigt im Ordner"),
        }
        .into(),
    );
}

/// The best first: the name is the text, then names starting with it, then
/// shorter names (a list of 20 000 sorted by name puts "a…" first otherwise).
/// Stable: equal ranks keep the natural name order.
pub(crate) fn rank(items: &mut [(explorer_core::Entry, PathBuf)], text: &str) {
    let q = text.trim().to_lowercase();
    // ranking makes sense for plain words only (not "ext:pdf", "*.txt", "a | b")
    if q.is_empty() || q.contains([':', '*', '?', '|', '"', '!', ' ']) {
        return;
    }
    items.sort_by_cached_key(|(e, _)| {
        let n = e.name.to_lowercase();
        let stem = n
            .rsplit_once('.')
            .map_or(n.as_str(), |(s, _)| s)
            .to_string();
        let class = if n == q || stem == q {
            0
        } else if n.starts_with(&q) {
            1
        } else {
            2
        };
        (class, n.chars().count())
    });
}

/// Enter (`alt` = Ctrl/Shift+Enter: show in its folder).
pub fn run(win: &MainWindow, i: i32, alt: bool) {
    let hit = ST.with_borrow(|s| s.hits.get(i.max(0) as usize).cloned());
    let pi = ST.with_borrow(|s| s.pane);
    close(win);
    let Some((path, is_dir)) = hit else { return };
    nav::trace_line(&format!("quick search run alt={alt} {}", path.display()));
    if alt {
        nav::show_in_folder(win, pi, &path);
    } else if is_dir {
        nav::navigate(win, pi, &path.to_string_lossy());
    } else if let Err(e) = crate::open::open_path(Path::new(&path)) {
        win.set_error(format!("{e}: {}", path.display()).into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(name: &str) -> (explorer_core::Entry, PathBuf) {
        (
            explorer_core::Entry {
                name: name.into(),
                kind: explorer_core::EntryKind::File,
                size: 0,
                modified: None,
                hidden: false,
                system: false,
                readonly: false,
                is_link: false,
            },
            PathBuf::from("/x").join(name),
        )
    }

    #[test]
    fn best_hits_first() {
        let mut v: Vec<_> = [
            "abericht.txt",
            "bericht-2025.pdf",
            "Bericht.docx",
            "bericht",
            "xbericht",
        ]
        .into_iter()
        .map(hit)
        .collect();
        rank(&mut v, "Bericht");
        let names: Vec<&str> = v.iter().map(|h| h.0.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "bericht",
                "Bericht.docx",
                "bericht-2025.pdf",
                "xbericht",
                "abericht.txt"
            ]
        );
        // query language: the order of the index stays
        let mut v: Vec<_> = ["b.pdf", "a.pdf"].into_iter().map(hit).collect();
        rank(&mut v, "ext:pdf");
        assert_eq!(v[0].0.name, "b.pdf");
    }
}
