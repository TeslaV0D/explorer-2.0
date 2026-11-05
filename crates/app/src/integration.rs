//! Settings → "Windows-Integration": status and the one-button reset (R-001).
//! Same code as the standalone `explorer2-recover` tool (`explorer-integrate`).

use crate::{nav, MainWindow};
use explorer_integrate::{JournalPaths, MemRegistry, Registry};

fn registry() -> Box<dyn Registry> {
    // off Windows there is no registry: only files can be journaled there
    explorer_integrate::system_registry().unwrap_or_else(|| Box::new(MemRegistry::new()))
}

/// Read the current state into the settings panel (on open, after a reset).
pub fn refresh(win: &MainWindow) {
    let Some(paths) = JournalPaths::default_paths() else {
        win.set_integration_status("Konfigurationsordner nicht gefunden.".into());
        win.set_integration_active(false);
        return;
    };
    let st = explorer_integrate::status(registry().as_mut(), &paths);
    nav::trace_line(&format!(
        "integration status entries={} active_points={} journal={:?}",
        st.entries, st.active_points, st.journal
    ));
    win.set_integration_status(st.summary_de().into());
    win.set_integration_active(st.any());
}

/// Button / palette: ask first (the dialog shows what is active).
pub fn ask_reset(win: &MainWindow) {
    refresh(win);
    let status = win.get_integration_status();
    if !win.get_integration_active() {
        nav::trace_line("integration reset: nothing active");
        win.set_settings_open(true);
        return;
    }
    crate::fileops::confirm_integration_reset(
        win,
        &format!(
            "Alle Windows-Integrationen von Explorer 2.0 entfernen und die vorherigen \
             Windows-Einstellungen exakt wiederherstellen?\n\n{status}\n\nWerte, die inzwischen \
             eine andere Software geändert hat, bleiben unangetastet."
        ),
    );
}

/// After "Zurücksetzen" in the dialog.
pub fn reset(win: &MainWindow) {
    let Some(paths) = JournalPaths::default_paths() else {
        return;
    };
    let r = explorer_integrate::reset(registry().as_mut(), &paths);
    nav::trace_line(&format!(
        "integration reset restored={} keys_removed={} swept={} kept={} errors={} journal={:?}",
        r.restored,
        r.keys_removed,
        r.swept,
        r.kept.len(),
        r.errors.len(),
        r.journal
    ));
    if !r.ok() {
        win.set_error(format!("Zurücksetzen unvollständig: {}", r.errors.join("; ")).into());
    }
    refresh(win);
    // show the outcome where the button is
    win.set_integration_status(r.summary_de().into());
    win.set_settings_open(true);
}
