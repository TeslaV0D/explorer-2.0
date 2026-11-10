//! Settings → "Windows-Integration": the switchable integrations (M7), their
//! status and the one-button reset (R-001). Same code as the standalone
//! `explorer2-recover` tool (`explorer-integrate`).

use crate::{nav, MainWindow};
use explorer_i18n::tr;
use explorer_integrate::features::{self, Feature, State};
use explorer_integrate::{JournalPaths, MemRegistry, Registry};
use std::path::{Path, PathBuf};

fn registry() -> Box<dyn Registry> {
    // off Windows there is no registry: only files can be journaled there
    explorer_integrate::system_registry().unwrap_or_else(|| Box::new(MemRegistry::new()))
}

/// The exe the integrations point to (this one, without a `\\?\` prefix).
fn exe() -> Option<PathBuf> {
    let p = std::env::current_exe().ok()?;
    let s = p.to_string_lossy();
    Some(match s.strip_prefix(r"\\?\") {
        Some(rest) => PathBuf::from(rest),
        None => p,
    })
}

/// Path of the exe inside a registered command line (`"C:\x\explorer2.exe" "%1"`).
fn exe_of_command(cmd: &str) -> Option<PathBuf> {
    let rest = cmd.trim().strip_prefix('"')?;
    Some(PathBuf::from(rest.split('"').next()?))
}

/// Keep a copy of the recovery tool where it survives deleting the app's
/// folder (app data, not an integration → not journaled).
fn copy_recover_tool(exe: &Path) -> Option<PathBuf> {
    let src = exe.with_file_name(if cfg!(windows) {
        "explorer2-recover.exe"
    } else {
        "explorer2-recover"
    });
    let dir = std::env::var_os("LOCALAPPDATA")
        .map(|d| PathBuf::from(d).join("Explorer2"))
        .or_else(|| {
            JournalPaths::default_paths()?
                .primary
                .parent()
                .map(Path::to_path_buf)
        })?;
    let dst = dir.join(src.file_name()?);
    if !src.exists() {
        return dst.exists().then_some(dst);
    }
    let same = std::fs::read(&src).ok() == std::fs::read(&dst).ok();
    if !same {
        let res = std::fs::create_dir_all(&dir).and_then(|_| std::fs::copy(&src, &dst));
        nav::trace_line(&format!(
            "recover tool copied to {} ok={}",
            dst.display(),
            res.is_ok()
        ));
    }
    Some(dst)
}

/// Read the current state into the settings panel (on open, after changes).
pub fn refresh(win: &MainWindow) {
    win.set_integration_available(explorer_integrate::system_registry().is_some());
    match crate::installer::describe() {
        Some((text, button)) => {
            win.set_install_available(true);
            win.set_install_status(text.into());
            win.set_install_button(button.into());
        }
        None => win.set_install_available(false),
    }
    let Some(paths) = JournalPaths::default_paths() else {
        win.set_integration_status(tr!("Configuration folder not found.").into());
        win.set_integration_active(false);
        return;
    };
    let mut reg = registry();
    let st = explorer_integrate::status(reg.as_mut(), &paths);
    let mut states = Vec::new();
    if let Some(exe) = exe() {
        for f in features::ALL {
            let s = features::state(reg.as_ref(), f, &exe);
            let on = !matches!(s, State::Off);
            match f {
                Feature::FolderOpen => win.set_feat_folder_open(on),
                Feature::ContextMenu => win.set_feat_context_menu(on),
                Feature::WinE => win.set_feat_win_e(on),
            }
            states.push(format!("{}={s:?}", f.id()));
        }
    }
    nav::trace_line(&format!(
        "integration status entries={} active_points={} journal={:?} {}",
        st.entries,
        st.active_points,
        st.journal,
        states.join(" ")
    ));
    win.set_integration_status(st.summary().into());
    win.set_integration_active(st.any());
}

/// Checkbox / palette: switch one integration on or off.
pub fn set_feature(win: &MainWindow, id: &str, on: bool) {
    if explorer_integrate::system_registry().is_none() {
        nav::trace_line("integration feature: no registry on this platform");
        win.set_integration_status(tr!("Windows integration is only available on Windows.").into());
        win.set_settings_open(true);
        return;
    }
    let (Some(f), Some(paths), Some(exe)) =
        (Feature::parse(id), JournalPaths::default_paths(), exe())
    else {
        return;
    };
    let mut reg = registry();
    let msg = if on {
        let recover = copy_recover_tool(&exe);
        match features::enable(reg.as_mut(), &paths, f, &exe) {
            Ok(n) => {
                nav::trace_line(&format!(
                    "integration feature {} on n={n} exe={} recover={:?}",
                    f.id(),
                    exe.display(),
                    recover
                ));
                tr!(
                    "{0} – switched on. Please don't move Explorer 2.0 (otherwise reset first).{1}",
                    f.label(),
                    recover.map_or(String::new(), |r| tr!(" Emergency tool: {0}", r.display()))
                )
            }
            Err(e) => {
                nav::trace_line(&format!("integration feature {} on failed: {e}", f.id()));
                win.set_error(e.clone().into());
                e
            }
        }
    } else {
        let r = features::disable(reg.as_mut(), &paths, f);
        nav::trace_line(&format!(
            "integration feature {} off restored={} keys_removed={} kept={} errors={}",
            f.id(),
            r.restored,
            r.keys_removed,
            r.kept.len(),
            r.errors.len()
        ));
        if !r.ok() {
            win.set_error(r.errors.join("; ").into());
        }
        tr!(
            "{0} – switched off, Windows default active again.",
            f.label()
        )
    };
    refresh(win);
    win.set_integration_status(msg.into());
}

pub fn toggle_feature(win: &MainWindow, id: &str) {
    let on = match Feature::parse(id) {
        Some(Feature::FolderOpen) => win.get_feat_folder_open(),
        Some(Feature::ContextMenu) => win.get_feat_context_menu(),
        Some(Feature::WinE) => win.get_feat_win_e(),
        None => return,
    };
    // the panel shows the result
    refresh(win);
    set_feature(win, id, !on);
    win.set_settings_open(true);
}

/// After start: an integration that points to an exe that no longer exists
/// (the app was moved) is re-pointed to this exe – otherwise double clicking
/// a folder would fail with "Windows cannot find …".
pub fn startup(win: &MainWindow) {
    if explorer_integrate::system_registry().is_none() {
        return;
    }
    let weak = slint::ComponentHandle::as_weak(win);
    slint::Timer::single_shot(std::time::Duration::from_secs(2), move || {
        let (Some(_w), Some(paths), Some(exe)) =
            (weak.upgrade(), JournalPaths::default_paths(), exe())
        else {
            return;
        };
        let mut reg = registry();
        for f in features::ALL {
            if let State::OnElsewhere(cmd) = features::state(reg.as_ref(), f, &exe) {
                let old = exe_of_command(&cmd);
                if old.as_ref().is_some_and(|o| !o.exists()) {
                    let res = features::enable(reg.as_mut(), &paths, f, &exe);
                    nav::trace_line(&format!(
                        "integration {} re-pointed from missing {:?} to {} ok={}",
                        f.id(),
                        old,
                        exe.display(),
                        res.is_ok()
                    ));
                }
            }
        }
    });
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
        win.set_error(tr!("Reset incomplete: {0}", r.errors.join("; ")).into());
    }
    refresh(win);
    // show the outcome where the button is
    win.set_integration_status(r.summary().into());
    win.set_settings_open(true);
}

/// Button / palette "Installieren": ask first.
pub fn ask_install(win: &MainWindow) {
    refresh(win);
    if win.get_install_button().is_empty() {
        nav::trace_line("install: already running the installed copy");
        win.set_settings_open(true);
        return;
    }
    crate::fileops::confirm_install(
        win,
        &tr!(
            "{0}\n\nAfterwards Explorer 2.0 continues from the installation folder (this window closes).",
            win.get_install_status()
        ),
        &win.get_install_button(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exe_from_registered_command() {
        assert_eq!(
            exe_of_command(r#""C:\Program Files\Explorer2\explorer2.exe" "%1""#),
            Some(PathBuf::from(r"C:\Program Files\Explorer2\explorer2.exe"))
        );
        assert_eq!(exe_of_command("explorer2.exe %1"), None);
    }
}
