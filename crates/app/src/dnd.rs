//! Drag & drop with other applications (Windows OLE).
//!
//! - Drops from other apps (and, on Windows, from our own lists) arrive via
//!   `explorer_shell::register_drop_target`; the target pane/row is found by
//!   hit-testing the list geometry that each pane reports from Slint.
//! - Drags out start `SHDoDragDrop` (modal) from a timer, i.e. after the Slint
//!   callback returned, so the drag's message loop never re-enters a running
//!   Slint handler.
//! - Linux keeps Slint's in-app `DragArea`/`DropArea` (no external DnD).

use crate::nav::{self, trace_line as trace};
use crate::MainWindow;
use explorer_shell::{DropEffect, DropEvent, DropPhase, SHELL_SUPPORTED};
use slint::ComponentHandle;
use std::cell::RefCell;
use std::path::PathBuf;
use std::time::Duration;

/// List area of one pane in window-logical pixels + its scroll offset.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Geom {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Slint `content-y` (≤ 0 when scrolled down).
    pub content_y: f32,
}

pub const ROW_HEIGHT: f32 = 24.0;

/// Pane and row under (x, y): row -1 = below the rows (folder background).
/// `rows(pane)` = number of rows shown in that pane (pure → tested).
pub fn hit(geoms: &[Geom], x: f32, y: f32, rows: impl Fn(usize) -> usize) -> Option<(usize, i32)> {
    let (pi, g) = geoms
        .iter()
        .enumerate()
        .find(|(_, g)| x >= g.x && x < g.x + g.w && y >= g.y && y < g.y + g.h)?;
    let i = ((y - g.y - g.content_y) / ROW_HEIGHT).floor();
    let row = if i >= 0.0 && (i as usize) < rows(pi) {
        i as i32
    } else {
        -1
    };
    Some((pi, row))
}

thread_local! {
    static GEOMS: RefCell<Vec<Geom>> = const { RefCell::new(Vec::new()) };
}

pub fn set_geometry(pi: usize, g: Geom) {
    GEOMS.with_borrow_mut(|v| {
        if v.len() <= pi {
            v.resize(pi + 1, Geom::default());
        }
        v[pi] = g;
    });
}

/// Window became visible: take over drops from other apps (Windows only).
pub fn init(win: &MainWindow) {
    if !SHELL_SUPPORTED {
        return;
    }
    win.set_os_dnd(true);
    let weak = win.as_weak();
    // the native window exists once the event loop runs
    slint::Timer::single_shot(Duration::ZERO, move || {
        let Some(w) = weak.upgrade() else { return };
        let hwnd = crate::fileops::window_handle(&w);
        let weak = w.as_weak();
        let res = explorer_shell::register_drop_target(hwnd, move |ev| {
            weak.upgrade()
                .map(|w| on_drop_event(&w, hwnd, ev))
                .unwrap_or(DropEffect::None)
        });
        trace(&format!("ole drop target registered={}", res.is_ok()));
        // CI hook: exercise the modal drag loop inside the running event loop
        // (without a pressed button the drag ends at once)
        if std::env::var_os("EXPLORER2_TEST_OLE_DRAG").is_some() {
            let weak = w.as_weak();
            slint::Timer::single_shot(Duration::from_secs(2), move || {
                if let Some(w) = weak.upgrade() {
                    os_drag(&w, 0, 0);
                }
            });
        }
        if let Err(e) = res {
            trace(&format!("ole drop target failed: {e}"));
        }
    });
}

fn on_drop_event(win: &MainWindow, hwnd: isize, ev: &DropEvent) -> DropEffect {
    if ev.phase == DropPhase::Leave {
        return DropEffect::None;
    }
    let (cx, cy) = explorer_shell::screen_to_client(hwnd, ev.screen.0, ev.screen.1);
    let scale = win.window().scale_factor();
    let (x, y) = (cx as f32 / scale, cy as f32 / scale);
    let target = GEOMS.with_borrow(|g| hit(g, x, y, nav::row_count));
    let Some((pi, row)) = target else {
        return DropEffect::None;
    };
    // dropping on a file row drops into the shown folder (like Explorer)
    let row = if row >= 0 && !nav::entry_at(pi, row as usize).is_some_and(|(_, _, d)| d) {
        -1
    } else {
        row
    };
    crate::fileops::ole_drop(win, pi, row, ev)
}

/// A row drag left the drag threshold (Windows): start the system drag.
pub fn os_drag(win: &MainWindow, pi: usize, row: usize) {
    let items: Vec<PathBuf> = {
        let sel = nav::selected_paths(pi);
        match nav::entry_at(pi, row) {
            Some((p, _, _)) if !sel.contains(&p) => vec![p],
            _ => sel,
        }
    };
    if items.is_empty() {
        return;
    }
    let weak = win.as_weak();
    slint::Timer::single_shot(Duration::ZERO, move || {
        let Some(w) = weak.upgrade() else { return };
        let hwnd = crate::fileops::window_handle(&w);
        trace(&format!("ole drag start n={}", items.len()));
        let res = explorer_shell::start_drag(hwnd, &items);
        trace(&format!("ole drag result {res:?}"));
        // the drag's modal loop swallowed the button release
        w.window()
            .dispatch_event(slint::platform::WindowEvent::PointerReleased {
                position: slint::LogicalPosition::new(0.0, 0.0),
                button: slint::platform::PointerEventButton::Left,
            });
        if let Ok(DropEffect::Move) = res {
            // another app moved our files: show it right away
            let dirs: Vec<PathBuf> = items
                .iter()
                .filter_map(|p| p.parent().map(Into::into))
                .collect();
            nav::refresh_dirs(&w, &dirs, &[], None);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_testing_panes_rows_and_scroll() {
        let left = Geom {
            x: 210.0,
            y: 140.0,
            w: 500.0,
            h: 400.0,
            content_y: 0.0,
        };
        let right = Geom {
            x: 711.0,
            y: 140.0,
            w: 489.0,
            h: 400.0,
            content_y: -48.0,
        };
        let geoms = [left, right];
        let rows = |pi: usize| if pi == 0 { 5 } else { 100 };
        assert_eq!(hit(&geoms, 300.0, 141.0, rows), Some((0, 0)));
        assert_eq!(
            hit(&geoms, 300.0, 140.0 + 24.0 * 4.0 + 1.0, rows),
            Some((0, 4))
        );
        assert_eq!(
            hit(&geoms, 300.0, 140.0 + 24.0 * 5.0 + 1.0, rows),
            Some((0, -1)),
            "below the rows"
        );
        // right pane scrolled by two rows: the first visible row is row 2
        assert_eq!(hit(&geoms, 800.0, 141.0, rows), Some((1, 2)));
        assert_eq!(hit(&geoms, 100.0, 200.0, rows), None, "sidebar");
        assert_eq!(hit(&geoms, 300.0, 100.0, rows), None, "header / toolbar");
        assert_eq!(hit(&[], 300.0, 200.0, rows), None);
    }
}
