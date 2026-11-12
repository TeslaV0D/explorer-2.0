//! Test mode `EXPLORER2_UI_DUMP=<file>`: the position of every named UI
//! element (`accessible-label`) is written to `<file>` whenever it changes,
//! so tests click "Einstellungen" instead of a pixel (redesign R0).
//!
//! Format (UTF-8, one element per line, logical pixels within the window,
//! drawing order = later lines lie on top):
//! ```text
//! window <x> <y> <scale>
//! <label>\t<x>\t<y>\t<width>\t<height>[\t<overflow>]
//! ```
//! Hidden (clipped away / zero-sized) elements are left out. `<overflow>`
//! (L5) marks a text that does not fit its box: `w<needed width>` (one line
//! cut off), `e<needed width>` (elided “…” on purpose: names, paths) or
//! `h<needed height>` (wrapped lines cut off) or `c<pixels>` (the text box
//! sticks out of a clipping parent – panel, scroll area, popup – on the right)
//! or `p<pixels>` (it sticks out of the element right around it).
//! The check measures every text: only with `EXPLORER2_UI_OVERFLOW=1`.
//!
//! Walks Slint's item tree directly: the public testing API needs debug
//! info compiled into the exe (element names), this needs nothing.

use crate::MainWindow;
use i_slint_core::accessibility::AccessibleStringProperty;
use i_slint_core::item_rendering::RenderText;
use i_slint_core::item_tree::{ItemRc, ParentItemTraversalMode};
use i_slint_core::items::{Clip, ComplexText, Flickable, SimpleText, TextOverflow, TextWrap};
use i_slint_core::layout::Orientation;
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
    let check = std::env::var_os("EXPLORER2_UI_OVERFLOW").is_some();
    root.visit_descendants(|item| {
        if !item.is_visible() {
            return ControlFlow::<()>::Continue(());
        }
        let g = item.geometry();
        if g.size.width <= 0.0 || g.size.height <= 0.0 {
            return ControlFlow::Continue(());
        }
        let label = item
            .accessible_string_property(AccessibleStringProperty::Label)
            .filter(|l| !l.is_empty());
        let p = item.map_to_window(g.origin);
        // measuring every text costs: only when asked for (gallery, layout check)
        let cut = || {
            if !check {
                return None;
            }
            overflow(item, g.size.width, g.size.height).or_else(|| cut_off(item, p.x, g.size.width))
        };
        // texts inside std widgets carry no label: listed only when they overflow
        let (name, cut) = match label {
            Some(l) => (l, cut()),
            None => match (text_of(item), cut()) {
                (Some(t), Some(c)) => (t, Some(c)),
                _ => return ControlFlow::Continue(()),
            },
        };
        out.push_str(&format!(
            "{}\t{:.0}\t{:.0}\t{:.0}\t{:.0}",
            name.replace(['\t', '\n'], " "),
            p.x + dx,
            p.y + dy,
            g.size.width,
            g.size.height
        ));
        if let Some(c) = cut {
            out.push('\t');
            out.push_str(&c);
        }
        out.push('\n');
        ControlFlow::Continue(())
    });
}

/// The content of a text element (None: not a text).
fn text_of(item: &ItemRc) -> Option<slint::SharedString> {
    if let Some(t) = item.downcast::<ComplexText>() {
        return Some(
            ComplexText::FIELD_OFFSETS
                .text()
                .apply_pin(t.as_pin_ref())
                .get(),
        );
    }
    item.downcast::<SimpleText>().map(|t| {
        SimpleText::FIELD_OFFSETS
            .text()
            .apply_pin(t.as_pin_ref())
            .get()
    })
}

/// Does a text element need more room than it got? (L5 layout check)
fn overflow(item: &ItemRc, width: f32, height: f32) -> Option<String> {
    let (wrap, elide) = if let Some(t) = item.downcast::<ComplexText>() {
        let t = t.as_pin_ref();
        (t.wrap(), t.overflow() == TextOverflow::Elide)
    } else {
        let t = item.downcast::<SimpleText>()?;
        let t = t.as_pin_ref();
        (t.wrap(), t.overflow() == TextOverflow::Elide)
    };
    let adapter = item.window_adapter()?;
    let info =
        |o: Orientation, cross: f32| item.borrow().as_ref().layout_info(o, cross, &adapter, item);
    // one pixel of tolerance: layouts round to whole pixels
    const TOL: f32 = 1.0;
    match wrap {
        TextWrap::NoWrap => {
            let need = info(Orientation::Horizontal, -1.0).preferred;
            // elided on purpose (“…”: file names, paths) or cut off
            let mark = if elide { 'e' } else { 'w' };
            (need > width + TOL).then(|| format!("{mark}{need:.0}"))
        }
        _ => {
            let longest = info(Orientation::Horizontal, -1.0).min;
            if !elide && longest > width + TOL {
                return Some(format!("w{longest:.0}"));
            }
            let need = info(Orientation::Vertical, width).preferred;
            (need > height + TOL).then(|| format!("h{need:.0}"))
        }
    }
}

/// How far a text sticks out of the nearest clipping parent on the right
/// (layouts never squeeze a one-line text: too long, it pushes its row out).
fn cut_off(item: &ItemRc, x: f32, width: f32) -> Option<String> {
    text_of(item)?;
    let mut parent = item.parent_item(ParentItemTraversalMode::StopAtPopups);
    // the element right around the text (button, check box, menu row …)
    if let Some(a) = parent.as_ref() {
        let g = a.geometry();
        if g.size.width > 0.0 {
            let over = x + width - (a.map_to_window(g.origin).x + g.size.width);
            if over > 1.0 {
                return Some(format!("p{over:.0}"));
            }
        }
    }
    while let Some(a) = parent {
        let clips = a
            .downcast::<Clip>()
            .is_some_and(|c| Clip::FIELD_OFFSETS.clip().apply_pin(c.as_pin_ref()).get())
            || a.downcast::<Flickable>().is_some();
        if clips {
            let g = a.geometry();
            let right = a.map_to_window(g.origin).x + g.size.width;
            let over = x + width - right;
            return (over > 1.0).then(|| format!("c{over:.0}"));
        }
        parent = a.parent_item(ParentItemTraversalMode::StopAtPopups);
    }
    None
}
