#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod config;
mod gfx;
mod nav;

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

fn main() -> Result<(), slint::PlatformError> {
    nav::mark_process_start();
    let arg = std::env::args().nth(1);
    if arg.as_deref() == Some("--version") {
        attach_parent_console();
        // Never panic on a closed/invalid stdout (GUI subsystem, panic=abort).
        let _ = writeln!(std::io::stdout(), "explorer2 {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let start = arg
        .map(|a| PathBuf::from(nav::clean_input(&a)))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let start = explorer_fs::nearest_existing_ancestor(
        &std::path::absolute(&start).unwrap_or(start.clone()),
    )
    .unwrap_or(start);

    // Must run before the first window exists (renderer is chosen per process).
    let renderer = gfx::select();
    let settings = config::load();

    let win = MainWindow::new()?;
    nav::init(&win, settings.show_hidden);
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

    win.on_navigate({
        let w = win.as_weak();
        move |s| {
            if let Some(w) = w.upgrade() {
                nav::navigate(&w, PathBuf::from(nav::clean_input(&s)), nav::Reason::User);
            }
        }
    });
    win.on_go_up({
        let w = win.as_weak();
        move || {
            if let Some(w) = w.upgrade() {
                nav::go_up(&w);
            }
        }
    });
    win.on_open_row({
        let w = win.as_weak();
        move |i| {
            if let Some(w) = w.upgrade() {
                nav::open_row(&w, i as usize);
            }
        }
    });
    win.on_toggle_hidden({
        let w = win.as_weak();
        move |on| {
            if let Some(w) = w.upgrade() {
                nav::set_show_hidden(&w, on);
            }
        }
    });

    nav::navigate(&win, start, nav::Reason::User);
    // Fires once the event loop runs (window + renderer initialized) -> separates
    // startup cost from data latency in traces.
    slint::Timer::single_shot(std::time::Duration::ZERO, || {
        nav::trace_line(&format!(
            "event loop running at={:.1}ms",
            nav::since_start_ms()
        ));
    });
    gfx::refresh_probe_in_background(&renderer);
    win.run()
}
