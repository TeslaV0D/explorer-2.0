#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod config;
mod gfx;
mod keys;
mod nav;
mod open;

use std::io::Write;
use std::path::PathBuf;

slint::include_modules!();

/// GUI-subsystem exes have no console; attach to the parent's so CLI output
/// (`--version`) shows up in cmd/PowerShell. Redirected handles stay untouched.
#[cfg(windows)]
fn attach_parent_console() {
    // SAFETY: plain Win32 call without pointers; failure (no parent console) is fine.
    unsafe {
        let _ = windows_sys::Win32::System::Console::AttachConsole(
            windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS,
        );
    }
}

#[cfg(not(windows))]
fn attach_parent_console() {}

/// Wires a UI callback to a nav function with an upgraded window handle.
macro_rules! on {
    ($win:expr, $setter:ident, |$w:ident $(, $arg:ident)*| $body:expr) => {{
        let weak = $win.as_weak();
        $win.$setter(move |$($arg),*| {
            if let Some($w) = weak.upgrade() {
                $body;
            }
        });
    }};
}

fn main() -> Result<(), slint::PlatformError> {
    nav::mark_process_start();
    let arg = std::env::args().nth(1);
    if arg.as_deref() == Some("--version") {
        attach_parent_console();
        // Never panic on a closed/invalid stdout (GUI subsystem, panic=abort).
        let _ = writeln!(std::io::stdout(), "explorer2 {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // A path on the command line wins over the saved session (like `explorer.exe C:\x`).
    let start = arg.map(|a| {
        let p = PathBuf::from(nav::clean_input(&a));
        let p = std::path::absolute(&p).unwrap_or(p);
        explorer_fs::nearest_existing_ancestor(&p).unwrap_or(p)
    });

    // Must run before the first window exists (renderer is chosen per process).
    let renderer = gfx::select();
    let settings = config::load();
    let session = if start.is_none() {
        config::load_session()
    } else {
        None
    };

    let win = MainWindow::new()?;
    nav::trace_line(&format!("window created at={:.1}ms", nav::since_start_ms()));
    win.set_hw_accel(settings.hardware_acceleration);
    win.set_renderer_status(gfx::status_text(&renderer).into());
    win.on_toggle_hw_accel({
        let w = win.as_weak();
        move |on| {
            let res = config::update(|s| s.hardware_acceleration = on);
            nav::trace_line(&format!(
                "settings: hardware_acceleration={on} saved={}",
                res.is_ok()
            ));
            if let (Err(e), Some(w)) = (res, w.upgrade()) {
                w.set_error(e.into());
            }
        }
    });

    on!(win, on_pane_activate, |w, pi| nav::activate_pane(
        &w,
        pi as usize
    ));
    on!(win, on_navigate, |w, pi, s| nav::navigate(
        &w,
        pi as usize,
        s.as_str()
    ));
    on!(win, on_cancel_edit, |_w, pi| nav::cancel_edit(pi as usize));
    on!(win, on_open_row, |w, pi, row| nav::open_entry(
        &w,
        pi as usize,
        row as usize
    ));
    on!(win, on_go_up, |w, pi| nav::go_up(&w, pi as usize));
    on!(win, on_back, |w, pi| nav::back(&w, pi as usize));
    on!(win, on_forward, |w, pi| nav::forward(&w, pi as usize));
    on!(win, on_sort_by, |w, k| nav::sort_by(&w, k.as_str()));
    on!(win, on_row_pressed, |w, pi, row, ctrl, shift| {
        nav::row_pressed(&w, pi as usize, row as usize, ctrl, shift)
    });
    on!(win, on_clear_selection, |w, pi| nav::clear_selection(
        &w,
        pi as usize
    ));
    on!(win, on_tab_select, |w, pi, ti| nav::tab_select(
        &w,
        pi as usize,
        ti as usize
    ));
    on!(win, on_tab_close, |w, pi, ti| nav::tab_close(
        &w,
        pi as usize,
        ti as usize
    ));
    on!(win, on_tab_new, |w, pi| nav::tab_new(&w, pi as usize));
    on!(win, on_address_resized, |_w, pi, px| nav::address_resized(
        pi as usize,
        px
    ));
    on!(win, on_toggle_split, |w| nav::toggle_split(&w));
    on!(win, on_open_place, |w, p| nav::open_place(&w, p.as_str()));
    on!(win, on_toggle_hidden, |w, on| nav::set_show_hidden(&w, on));
    win.on_list_key({
        let weak = win.as_weak();
        move |pi, text, ctrl, shift, alt, page| {
            weak.upgrade().is_some_and(|w| {
                nav::list_key(&w, pi as usize, text.as_str(), ctrl, shift, alt, page)
            })
        }
    });

    nav::init(
        &win,
        settings.show_hidden,
        explorer_core::SortSpec {
            key: settings.sort_key,
            descending: settings.sort_desc,
            dirs_first: true,
        },
        start,
        session,
    );

    // Drives come and go (USB sticks, network shares): refresh the sidebar periodically.
    let drive_timer = slint::Timer::default();
    drive_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_secs(15),
        {
            let w = win.as_weak();
            move || {
                if let Some(w) = w.upgrade() {
                    nav::refresh_drives(&w);
                }
            }
        },
    );

    win.window().on_close_requested(|| {
        nav::save_session_now();
        slint::CloseRequestResponse::HideWindow
    });

    // Fires once the event loop runs (window + renderer initialized) -> separates
    // startup cost from data latency in traces.
    slint::Timer::single_shot(std::time::Duration::ZERO, || {
        nav::trace_line(&format!(
            "event loop running at={:.1}ms",
            nav::since_start_ms()
        ));
    });
    gfx::probe_in_background(&renderer);
    let res = win.run();
    nav::save_session_now();
    res
}
