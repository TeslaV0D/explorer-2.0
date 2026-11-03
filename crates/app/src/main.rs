#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use explorer_core::{format_datetime, format_size, sort_entries, Entry, SortSpec};
use slint::{ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

slint::include_modules!();

fn fmt_size(e: &Entry) -> String {
    if e.is_dir() {
        String::new()
    } else {
        format_size(e.size)
    }
}

fn load(path: &Path) -> Vec<Entry> {
    let mut v = explorer_fs::read_dir(path).unwrap_or_default();
    sort_entries(&mut v, SortSpec::default());
    v
}

fn show(win: &MainWindow, path: &Path, entries: &[Entry]) {
    let rows: Vec<Row> = entries
        .iter()
        .map(|e| Row {
            name: e.name.clone().into(),
            size: fmt_size(e).into(),
            modified: e.modified.map(format_datetime).unwrap_or_default().into(),
            is_dir: e.is_dir(),
        })
        .collect();
    win.set_current_path(SharedString::from(path.to_string_lossy().as_ref()));
    win.set_rows(ModelRc::new(VecModel::from(rows)));
}

fn main() -> Result<(), slint::PlatformError> {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("explorer2 {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let start = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let win = MainWindow::new()?;
    let state = Rc::new(RefCell::new((start.clone(), load(&start))));
    show(&win, &state.borrow().0, &state.borrow().1);

    let nav = {
        let (w, st) = (win.as_weak(), state.clone());
        move |p: PathBuf| {
            if !p.is_dir() {
                return;
            }
            let entries = load(&p);
            *st.borrow_mut() = (p.clone(), entries);
            if let Some(w) = w.upgrade() {
                show(&w, &st.borrow().0, &st.borrow().1);
            }
        }
    };
    win.on_navigate({
        let nav = nav.clone();
        move |s| nav(PathBuf::from(s.as_str()))
    });
    win.on_go_up({
        let (nav, st) = (nav.clone(), state.clone());
        move || {
            let parent = st.borrow().0.parent().map(|p| p.to_path_buf());
            if let Some(p) = parent {
                nav(p);
            }
        }
    });
    win.on_open_row({
        let (nav, st) = (nav.clone(), state.clone());
        move |i| {
            let target = {
                let s = st.borrow();
                s.1.get(i as usize)
                    .filter(|e| e.is_dir())
                    .map(|e| s.0.join(&e.name))
            };
            if let Some(p) = target {
                nav(p);
            }
        }
    });
    win.run()
}
