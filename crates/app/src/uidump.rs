//! Test mode `EXPLORER2_UI_DUMP=<file>`: the position of every named UI
//! element (`accessible-label`) is written to `<file>` whenever it changes,
//! so tests click "Einstellungen" instead of a pixel (redesign R0).
//!
//! Format (UTF-8, one element per line, logical pixels within the window,
//! drawing order = later lines lie on top):
//! ```text
//! window <x> <y> <scale>
//! <label>\t<x>\t<y>\t<width>\t<height>
//! ```
//! Hidden (clipped away / zero-sized) elements are left out.
//!
//! Walks Slint's item tree directly: the public testing API needs debug
//! info compiled into the exe (element names), this needs nothing.

use crate::MainWindow;
use i_slint_core::accessibility::AccessibleStringProperty;
use i_slint_core::item_tree::ItemRc;
use i_slint_core::window::{PopupWindowLocation, WindowInner};
use slint::ComponentHandle;
use std::ops::ControlFlow;
use std::time::Duration;

/// Starts the dump if the environment asks for it.
pub fn init(win: &MainWindow) {
    let Some(path) = std::env::var_os("EXPLORER2_UI_DUMP") else {
        return;
    };
    let path = std::path::PathBuf::from(path);
    let weak = win.as_weak();
    let mut last = String::new();
    let mut ticks = 0u32;
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(150),
        move || {
            let Some(w) = weak.upgrade() else { return };
            let text = dump(&w);
            ticks += 1;
            // also every ~1 s: a second test window may have overwritten the file
            if text != last || ticks.is_multiple_of(7) {
                let tmp = path.with_extension("tmp");
                if std::fs::write(&tmp, &text).is_ok() {
                    let _ = std::fs::rename(&tmp, &path);
                }
                last = text;
            }
        },
    );
    // lives as long as the app (test mode only)
    std::mem::forget(timer);
}

/// All named, visible elements in drawing order; open popups (menus) last,
/// at their place in the window.
pub fn dump(win: &MainWindow) -> String {
    let pos = win.window().position();
    let scale = win.window().scale_factor();
    let mut out = format!("window {} {} {scale}\n", pos.x, pos.y);
    let inner = WindowInner::from_pub(win.window());
    walk(&mut out, ItemRc::new_root(inner.component()), (0.0, 0.0));
    for popup in inner.active_popups().iter() {
        let offset = match &popup.location {
            PopupWindowLocation::ChildWindow(p) => (p.x, p.y),
            // a menu in its own native window: its place relative to ours
            PopupWindowLocation::TopLevel(adapter) => {
                let p = adapter.window().position();
                ((p.x - pos.x) as f32 / scale, (p.y - pos.y) as f32 / scale)
            }
        };
        walk(&mut out, ItemRc::new_root(popup.component.clone()), offset);
    }
    out
}

fn walk(out: &mut String, root: ItemRc, (dx, dy): (f32, f32)) {
    root.visit_descendants(|item| {
        if !item.is_visible() {
            return ControlFlow::<()>::Continue(());
        }
        if let Some(label) = item.accessible_string_property(AccessibleStringProperty::Label) {
            let g = item.geometry();
            if !label.is_empty() && g.size.width > 0.0 && g.size.height > 0.0 {
                let p = item.map_to_window(g.origin);
                out.push_str(&format!(
                    "{}\t{:.0}\t{:.0}\t{:.0}\t{:.0}\n",
                    label.replace(['\t', '\n'], " "),
                    p.x + dx,
                    p.y + dy,
                    g.size.width,
                    g.size.height
                ));
            }
        }
        ControlFlow::Continue(())
    });
}
