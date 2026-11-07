//! Colour scheme of the window: light / dark / follow Windows (live), with
//! the Windows accent colour. Colours come from `core::theme::tokens` and go
//! into the Slint global `Theme`; the std widgets follow via
//! `Palette.color-scheme` (main.slint), the title bar via DWM.

use crate::{nav, DesignColors, MainWindow, Theme, ThemeColors};
use explorer_core::theme::{hex, tokens, Design, Rgba, SystemTheme, ThemeChoice, Tokens};
use slint::{Color, ComponentHandle};
use std::cell::RefCell;
use std::time::Duration;

#[derive(Default)]
struct State {
    choice: ThemeChoice,
    use_accent: bool,
    system: SystemTheme,
    /// Last applied (dark, colours) → nothing to do when unchanged.
    applied: Option<Tokens>,
}

thread_local! {
    static ST: RefCell<State> = RefCell::new(State::default());
}

/// Before the window is shown (no light flash): read the system state,
/// apply, start the watcher.
pub fn init(win: &MainWindow, choice: ThemeChoice, use_accent: bool) {
    let t0 = std::time::Instant::now();
    let system = explorer_shell::system_theme();
    ST.with_borrow_mut(|s| {
        s.choice = choice;
        s.use_accent = use_accent;
        s.system = system;
    });
    win.set_opt_system_accent(use_accent);
    apply(win, "init");
    nav::trace_line(&format!(
        "theme init ms={:.2}",
        t0.elapsed().as_secs_f64() * 1000.0
    ));
    let weak = win.as_weak();
    explorer_shell::watch_system_theme(move || {
        let now = explorer_shell::system_theme();
        let weak = weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            let Some(w) = weak.upgrade() else { return };
            let changed = ST.with_borrow_mut(|s| {
                let c = s.system != now;
                s.system = now;
                c
            });
            if changed {
                apply(&w, "system");
                // winit re-applies the system title bar on WM_SETTINGCHANGE,
                // possibly after us → set ours again once that settled
                let weak = w.as_weak();
                slint::Timer::single_shot(Duration::from_millis(400), move || {
                    if let Some(w) = weak.upgrade() {
                        title_bar(&w, 0);
                    }
                });
            }
        });
    });
    title_bar(win, 0);
}

/// "Farbschema" buttons / palette commands.
pub fn set_choice(win: &MainWindow, name: &str) {
    let Some(choice) = ThemeChoice::parse(name) else {
        return;
    };
    ST.with_borrow_mut(|s| s.choice = choice);
    if let Err(e) = crate::config::update(|s| s.theme = choice) {
        win.set_error(e.into());
    }
    apply(win, "choice");
    title_bar(win, 0);
}

/// "Windows-Akzentfarbe verwenden".
pub fn set_use_accent(win: &MainWindow, on: bool) {
    ST.with_borrow_mut(|s| s.use_accent = on);
    win.set_opt_system_accent(on);
    apply(win, "accent");
}

pub fn is_dark() -> bool {
    ST.with_borrow(|s| s.choice.is_dark(s.system.dark))
}

fn apply(win: &MainWindow, why: &str) {
    let (choice, t) = ST.with_borrow(|s| {
        let dark = s.choice.is_dark(s.system.dark);
        let accent = s.system.accent.filter(|_| s.use_accent);
        (s.choice, tokens(dark, accent.as_ref()))
    });
    win.set_theme_choice(choice.as_str().into());
    if ST.with_borrow(|s| s.applied == Some(t)) {
        return;
    }
    ST.with_borrow_mut(|s| s.applied = Some(t));
    let g = win.global::<Theme>();
    g.set_dark(t.dark);
    g.set_c(colors(&t));
    g.set_d(design_colors(&t.ds));
    let sys = ST.with_borrow(|s| s.system);
    nav::trace_line(&format!(
        "theme why={why} choice={} dark={} accent={} selection={} system_dark={} system_accent={}",
        choice.as_str(),
        t.dark,
        hex([t.accent[0], t.accent[1], t.accent[2]]),
        hex([t.selection[0], t.selection[1], t.selection[2]]),
        sys.dark,
        sys.accent.map_or("-".to_string(), |a| hex(a.base)),
    ));
}

/// The native window exists only some time after the event loop started →
/// retry until there is an HWND (like the drop target).
fn title_bar(win: &MainWindow, attempt: u32) {
    let hwnd = crate::fileops::window_handle(win);
    if hwnd == 0 {
        if cfg!(windows) && attempt < 100 {
            let weak = win.as_weak();
            slint::Timer::single_shot(Duration::from_millis(50), move || {
                if let Some(w) = weak.upgrade() {
                    title_bar(&w, attempt + 1);
                }
            });
        }
        return;
    }
    let dark = is_dark();
    match explorer_shell::set_dark_title_bar(hwnd, dark) {
        Ok(()) => nav::trace_line(&format!("theme title bar dark={dark}")),
        Err(e) => nav::trace_line(&format!("theme title bar failed: {e}")),
    }
}

fn col(c: Rgba) -> Color {
    Color::from_argb_u8(c[3], c[0], c[1], c[2])
}

fn colors(t: &Tokens) -> ThemeColors {
    ThemeColors {
        content: col(t.content),
        card: col(t.card),
        chrome: col(t.chrome),
        panel: col(t.panel),
        header: col(t.header),
        tabs_active: col(t.tabs_active),
        tabs_idle: col(t.tabs_idle),
        line: col(t.line),
        line_strong: col(t.line_strong),
        outline: col(t.outline),
        hover: col(t.hover),
        hover_strong: col(t.hover_strong),
        row_hover: col(t.row_hover),
        accent_hover: col(t.accent_hover),
        current: col(t.current),
        selection: col(t.selection),
        selection_idle: col(t.selection_idle),
        drop: col(t.drop),
        accent: col(t.accent),
        band: col(t.band),
        text: col(t.text),
        text2: col(t.text2),
        text3: col(t.text3),
        text4: col(t.text4),
        muted: col(t.muted),
        faint: col(t.faint),
        disabled: col(t.disabled),
        danger: col(t.danger),
        warning: col(t.warning),
        track: col(t.track),
        folder: col(t.folder),
        file: col(t.file),
        drive: col(t.drive),
        scrim: col(t.scrim),
        scrim_soft: col(t.scrim_soft),
        shadow: col(t.shadow),
        shadow_soft: col(t.shadow_soft),
    }
}

fn design_colors(d: &Design) -> DesignColors {
    DesignColors {
        app: col(d.app),
        panel: col(d.panel),
        surface: col(d.surface),
        raised: col(d.raised),
        overlay: col(d.overlay),
        hover: col(d.hover),
        pressed: col(d.pressed),
        selected: col(d.selected),
        selected_idle: col(d.selected_idle),
        border: col(d.border),
        border_strong: col(d.border_strong),
        text: col(d.text),
        text2: col(d.text2),
        muted: col(d.muted),
        faint: col(d.faint),
        icon: col(d.icon),
        icon_muted: col(d.icon_muted),
        accent: col(d.accent),
        on_accent: col(d.on_accent),
        danger: col(d.danger),
        warning: col(d.warning),
        success: col(d.success),
        folder: col(d.folder),
        file: col(d.file),
        scrim: col(d.scrim),
        shadow: col(d.shadow),
    }
}
