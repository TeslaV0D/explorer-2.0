//! M8: install / update this copy for the current user (no admin):
//! `%LOCALAPPDATA%\Programs\Explorer2` + Start menu + "Apps & Features",
//! journaled through `explorer-integrate` (the reset and the uninstaller
//! remove it again). Used by `explorer2.exe --install` and the settings.

use explorer_integrate::install::{self, InstallPaths, InstallReport};
use explorer_integrate::{JournalPaths, MemRegistry, Registry};
use std::path::PathBuf;

fn registry() -> Box<dyn Registry> {
    explorer_integrate::system_registry().unwrap_or_else(|| Box::new(MemRegistry::new()))
}

/// Folder of the running exe (without a `\\?\` prefix).
pub fn exe_dir() -> Option<PathBuf> {
    let p = std::env::current_exe().ok()?;
    let s = p.to_string_lossy().into_owned();
    let p = PathBuf::from(s.strip_prefix(r"\\?\").unwrap_or(&s));
    p.parent().map(PathBuf::from)
}

/// Install (or update) from the folder of the running exe.
pub fn install_here() -> Result<(InstallPaths, InstallReport), String> {
    let ip = InstallPaths::default_paths().ok_or("Installationsordner nicht bestimmbar")?;
    let jp = JournalPaths::default_paths().ok_or("Konfigurationsordner nicht gefunden")?;
    let src = exe_dir().ok_or("Programmordner nicht bestimmbar")?;
    let mut reg = registry();
    let r = install::install(
        reg.as_mut(),
        &jp,
        &ip,
        &src,
        env!("CARGO_PKG_VERSION"),
        &|s| explorer_shell::shortcut_bytes(&s.target, &s.args, &s.description),
    )?;
    Ok((ip, r))
}

/// Running from the installed copy (not a download/portable copy)?
pub fn running_installed() -> bool {
    let (Some(ip), Some(dir)) = (InstallPaths::default_paths(), exe_dir()) else {
        return false;
    };
    let canon = |p: &std::path::Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let (a, b) = (canon(&ip.program_dir), canon(&dir));
    if cfg!(windows) {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    } else {
        a == b
    }
}

pub fn is_installed() -> bool {
    InstallPaths::default_paths().is_some_and(|ip| install::is_installed(registry().as_ref(), &ip))
}

/// `explorer2.exe --install [--quiet]`: console output, exit code; starts
/// the installed copy afterwards unless `--quiet`.
pub fn cli(quiet: bool) -> i32 {
    use std::io::Write;
    let out = |s: &str| {
        let _ = writeln!(std::io::stdout(), "{s}");
    };
    match install_here() {
        Ok((ip, r)) => {
            out(&format!(
                "Explorer 2.0 installiert: {} ({} Verknüpfungen im Startmenü{})",
                r.program_dir.display(),
                r.shortcuts,
                if r.repointed.is_empty() {
                    String::new()
                } else {
                    format!(", umgestellt: {}", r.repointed.join(", "))
                }
            ));
            if !quiet {
                let _ = std::process::Command::new(ip.app())
                    .arg("--new-instance")
                    .spawn();
            }
            0
        }
        Err(e) => {
            out(&format!("Installation fehlgeschlagen: {e}"));
            1
        }
    }
}

/// Settings → "Installation": state text and the button label ("" = none).
pub fn describe() -> Option<(String, String)> {
    let ip = InstallPaths::default_paths()?;
    let here = exe_dir()
        .map(|d| d.display().to_string())
        .unwrap_or_default();
    let version = env!("CARGO_PKG_VERSION");
    Some(match (is_installed(), running_installed()) {
        (true, true) => (
            format!(
                "Installiert (Version {version}) in {}. Deinstallieren über „Apps & Features“ oder das Startmenü.",
                ip.program_dir.display()
            ),
            String::new(),
        ),
        (true, false) => (
            format!(
                "Installiert in {}; diese Kopie läuft aus {here}.",
                ip.program_dir.display()
            ),
            "Installation mit dieser Kopie aktualisieren".into(),
        ),
        (false, _) => (
            format!(
                "Nicht installiert – läuft aus {here}. Installieren legt Explorer 2.0 fest nach {} (ohne Administratorrechte) und ins Startmenü.",
                ip.program_dir.display()
            ),
            "Installieren".into(),
        ),
    })
}

/// After "Installieren" in the confirmation: install, then continue in the
/// installed copy (this window closes).
pub fn install_and_restart(win: &crate::MainWindow) {
    match install_here() {
        Ok((ip, r)) => {
            crate::nav::trace_line(&format!(
                "install ok dir={} shortcuts={} repointed={:?}",
                r.program_dir.display(),
                r.shortcuts,
                r.repointed
            ));
            if running_installed() {
                crate::integration::refresh(win);
                return;
            }
            crate::nav::save_session_now();
            match std::process::Command::new(ip.app())
                .arg("--new-instance")
                .spawn()
            {
                Ok(_) => {
                    crate::nav::trace_line("install: continuing in the installed copy");
                    let _ = slint::quit_event_loop();
                }
                Err(e) => win.set_error(format!("Installiert, Start fehlgeschlagen: {e}").into()),
            }
        }
        Err(e) => {
            crate::nav::trace_line(&format!("install failed: {e}"));
            win.set_error(format!("Installation fehlgeschlagen: {e}").into());
        }
    }
}
