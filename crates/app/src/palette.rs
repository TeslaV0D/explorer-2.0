//! Command palette (Ctrl+Shift+P): type to filter every command (fuzzy),
//! Enter runs it; a leading ">" jumps to places (sidebar, drives, recent
//! folders) instead. Commands come from `core::commands` and run through
//! the same paths as their shortcuts (`nav::run_action`, `fileops::command`).

use crate::keys::KeyAction;
use crate::{nav, MainWindow, PaletteItem};
use explorer_core::commands::{self, fuzzy_score, push_recent, rank, Need};
use explorer_core::view::ViewMode;
use explorer_i18n::tr;
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::cell::RefCell;
use std::rc::Rc;

const RECENT_KEEP: usize = 8;

enum Target {
    Cmd(&'static str),
    Go(String),
}

#[derive(Default)]
struct State {
    open: bool,
    pane: usize,
    targets: Vec<(Target, bool)>,
    model: Rc<VecModel<PaletteItem>>,
    recent: Vec<String>,
}

thread_local! {
    static ST: RefCell<State> = RefCell::new(State::default());
}

pub fn init(win: &MainWindow, recent: Vec<String>) {
    ST.with_borrow_mut(|s| {
        s.recent = recent;
        win.set_pal_items(ModelRc::from(s.model.clone()));
    });
}

/// Ctrl+Shift+P.
pub fn open(win: &MainWindow) {
    if crate::fileops::blocks_keys(win) {
        return; // a dialog or inline rename owns the keyboard
    }
    let pi = nav::active_pane();
    ST.with_borrow_mut(|s| {
        s.open = true;
        s.pane = pi;
    });
    query(win, "");
    // the palette element is created on open (fresh, empty, focused input)
    win.set_pal_open(true);
    nav::trace_line("palette open");
}

pub fn close(win: &MainWindow) {
    let pi = ST.with_borrow_mut(|s| {
        s.open = false;
        s.pane
    });
    win.set_pal_open(false);
    nav::focus_pane(pi);
    nav::trace_line("palette close");
}

fn enabled(win: &MainWindow, pi: usize, need: Need) -> bool {
    match need {
        Need::Always => true,
        Need::Selection => win.get_has_selection(),
        Need::Folder => nav::pane_dir(pi).is_some(),
        Need::Paste => win.get_can_paste(),
        Need::Undo => win.get_can_undo(),
        Need::Redo => win.get_can_redo(),
        Need::Trash => nav::pane_place(pi) == Some(explorer_core::places::VirtualPlace::Trash),
    }
}

/// The text changed: rebuild the list (commands, or places after ">").
pub fn query(win: &MainWindow, text: &str) {
    let pi = ST.with_borrow(|s| s.pane);
    let mut targets = Vec::new();
    let mut items = Vec::new();
    if let Some(q) = text.trim_start().strip_prefix('>') {
        let mut hits: Vec<(i32, String, String)> = nav::palette_places()
            .into_iter()
            .filter_map(|(label, path)| {
                let s = fuzzy_score(q, &label)
                    .map(|s| s + 20)
                    .or_else(|| fuzzy_score(q, &path))?;
                Some((s, label, path))
            })
            .collect();
        hits.sort_by_key(|h| std::cmp::Reverse(h.0));
        for (_, label, path) in hits.into_iter().take(50) {
            items.push(PaletteItem {
                label: label.into(),
                detail: path.as_str().into(),
                enabled: true,
            });
            targets.push((Target::Go(path), true));
        }
    } else {
        let recent = ST.with_borrow(|s| s.recent.clone());
        for c in rank(text, &recent, commands::ALL) {
            let ok = enabled(win, pi, c.need);
            items.push(PaletteItem {
                label: c.title().into(),
                detail: c.shortcut().into(),
                enabled: ok,
            });
            targets.push((Target::Cmd(c.id), ok));
        }
    }
    let n = items.len();
    ST.with_borrow_mut(|s| {
        s.model.set_vec(items);
        s.targets = targets;
    });
    // first runnable entry pre-selected
    let first = ST.with_borrow(|s| s.targets.iter().position(|(_, ok)| *ok).unwrap_or(0));
    win.set_pal_sel(first as i32);
    nav::trace_line(&format!(
        "palette query={text:?} items={n} first={:?}",
        ST.with_borrow(|s| s.model.iter().next().map(|i| i.label.to_string()))
    ));
}

/// Enter or click on entry `index`.
pub fn run(win: &MainWindow, index: i32) {
    let (target, pi) = ST.with_borrow(|s| {
        let t = s.targets.get(index.max(0) as usize).map(|(t, ok)| match t {
            Target::Cmd(id) => (Some(*id), None, *ok),
            Target::Go(p) => (None, Some(p.clone()), true),
        });
        (t, s.pane)
    });
    let Some((cmd, go, ok)) = target else { return };
    if !ok {
        nav::trace_line("palette: command not available here");
        win.set_error(tr!("Not available here").into());
        return;
    }
    close(win);
    if let Some(p) = go {
        nav::trace_line(&format!("palette go {p}"));
        nav::navigate(win, pi, &p);
        return;
    }
    let Some(id) = cmd else { return };
    let recent = ST.with_borrow_mut(|s| {
        push_recent(&mut s.recent, id, RECENT_KEEP);
        s.recent.clone()
    });
    if let Err(e) = crate::config::update(|s| s.recent_commands = recent) {
        win.set_error(e.into());
    }
    nav::trace_line(&format!("palette run {id}"));
    // run once the palette is gone and the list has its focus back: a command
    // that opens a dialog (confirm, conflict) must get the focus after that,
    // not race it (Enter in the dialog went nowhere, CI #60 W41)
    let weak = win.as_weak();
    slint::Timer::single_shot(std::time::Duration::ZERO, move || {
        if let Some(w) = weak.upgrade() {
            execute(&w, pi, id);
        }
    });
}

pub(crate) fn execute(win: &MainWindow, pi: usize, id: &str) {
    let act = |a: KeyAction| nav::run_action(win, pi, a);
    match id {
        "open" => act(KeyAction::Open),
        "copy" | "cut" | "paste" | "rename" | "delete" | "delete-permanent" | "new-folder"
        | "undo" | "redo" | "properties" | "restore" | "empty-trash" => {
            crate::fileops::command(win, pi, id)
        }
        "context-menu" => act(KeyAction::ContextMenu),
        "select-all" => act(KeyAction::SelectAll),
        "up" => act(KeyAction::Up),
        "back" => act(KeyAction::Back),
        "forward" => act(KeyAction::Forward),
        "refresh" => act(KeyAction::Refresh),
        "edit-path" => act(KeyAction::EditPath),
        "new-tab" => act(KeyAction::NewTab),
        "close-tab" => act(KeyAction::CloseTab),
        "next-tab" => act(KeyAction::NextTab),
        "prev-tab" => act(KeyAction::PrevTab),
        "toggle-split" => act(KeyAction::ToggleSplit),
        "switch-pane" => act(KeyAction::SwitchPane),
        "inspector" => crate::inspector::toggle(win),
        "view-details" => nav::set_view(win, ViewMode::Details),
        "view-medium" => nav::set_view(win, ViewMode::Medium),
        "view-large" => nav::set_view(win, ViewMode::Large),
        "view-xlarge" => nav::set_view(win, ViewMode::ExtraLarge),
        "view-all" => nav::apply_view_all(win),
        "sort-name" => nav::sort_pick(win, Some("name"), None),
        "sort-modified" => nav::sort_pick(win, Some("modified"), None),
        "sort-type" => nav::sort_pick(win, Some("type"), None),
        "sort-size" => nav::sort_pick(win, Some("size"), None),
        "sort-asc" => nav::sort_pick(win, None, Some(false)),
        "sort-desc" => nav::sort_pick(win, None, Some(true)),
        "toggle-hidden" => {
            let on = !win.get_show_hidden();
            win.set_show_hidden(on);
            nav::set_show_hidden(win, on);
        }
        "toggle-extensions" => {
            let on = !win.get_opt_show_ext();
            win.set_opt_show_ext(on);
            nav::set_option(win, "show_extensions", on);
        }
        "go-trash" => nav::navigate(win, pi, "::papierkorb"),
        "go-thispc" => nav::navigate(win, pi, "::dieser-pc"),
        "go-libraries" => nav::navigate(win, pi, "::bibliotheken"),
        "settings" => win.set_settings_open(true),
        "theme-system" => crate::theme::set_choice(win, "system"),
        "theme-light" => crate::theme::set_choice(win, "light"),
        "theme-dark" => crate::theme::set_choice(win, "dark"),
        "language-system" | "language-en" | "language-de" | "language-es" | "language-fr"
        | "language-ru" | "language-uk" | "language-pl" | "language-it" | "language-zh"
        | "language-ja" => crate::lang::choose_code(win, &id["language-".len()..]),
        "integration-reset" => crate::integration::ask_reset(win),
        "install" => crate::integration::ask_install(win),
        "toggle-folder-open" => crate::integration::toggle_feature(win, "folder-open"),
        "toggle-context-menu" => crate::integration::toggle_feature(win, "context-menu"),
        "toggle-win-e" => crate::integration::toggle_feature(win, "win-e"),
        "toggle-single-instance" => {
            let on = !win.get_opt_single_instance();
            nav::set_option(win, "single_instance", on);
            win.set_settings_open(true);
        }
        "search" => act(KeyAction::FocusSearch),
        "quick-search" => crate::quick::open(win),
        "search-rescan" => {
            crate::search::rescan(win);
            win.set_settings_open(true);
        }
        "toggle-index" => {
            let on = !win.get_opt_index_enabled();
            nav::set_option(win, "index_enabled", on);
            win.set_settings_open(true);
        }
        other => nav::trace_line(&format!("palette: unknown command {other}")),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_registered_command_is_handled() {
        // the match in `execute` must know every id of the registry
        let src = include_str!("palette.rs");
        let body = &src[src.find("fn execute").unwrap()..src.find("#[cfg(test)]").unwrap()];
        for c in explorer_core::commands::ALL {
            assert!(
                body.contains(&format!("\"{}\"", c.id)),
                "palette does not run {}",
                c.id
            );
        }
    }
}
