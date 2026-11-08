//! `explorer2-recover` – puts Windows back to its own defaults (R-001) and
//! uninstalls Explorer 2.0 (M8).
//!
//! Console only, no UI/GPU: works when Explorer 2.0 itself does not start,
//! also in Safe Mode. Same reset as the app's settings button.
//!
//! ```text
//! explorer2-recover                  show what is active, ask, reset
//! explorer2-recover --yes            reset without asking
//! explorer2-recover --status         only show (exit 1 = something is active)
//! explorer2-recover --uninstall      reset + remove the installation (asks)
//! explorer2-recover --uninstall --quiet
//! ```
//! Exit codes: 0 ok / nothing active, 1 active (`--status`) or errors,
//! 2 usage / cancelled without a console.

use explorer_integrate::install::{self, InstallPaths};
use explorer_integrate::{reset, status, JournalPaths, MemRegistry, Registry, ResetReport};
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

const USAGE: &str = "Explorer 2.0 – Windows-Standard wiederherstellen

  explorer2-recover                  zeigt den Stand, fragt nach und setzt zurück
  explorer2-recover --yes            setzt ohne Nachfrage zurück
  explorer2-recover --status         zeigt nur den Stand (Exit-Code 1 = aktiv)
  explorer2-recover --uninstall      setzt zurück und entfernt Explorer 2.0
  explorer2-recover --uninstall --quiet   dasselbe ohne Nachfrage

Entfernt alle Windows-Integrationen von Explorer 2.0 (Ordner-Verben,
Win+E, Kontextmenü, Umgebungsvariablen, Verknüpfungen …) und stellt die
vorherigen Werte exakt wieder her. Werte, die inzwischen eine andere
Software geändert hat, bleiben unangetastet. Ihre Einstellungen
(%APPDATA%\\Explorer2) bleiben beim Deinstallieren erhalten.";

enum Mode {
    Reset {
        yes: bool,
    },
    Status,
    Uninstall {
        quiet: bool,
    },
    /// Internal: run from a temp copy, deletes the program folder.
    UninstallFinish(PathBuf),
}

fn out(s: &str) {
    // never panic on a closed console (GUI double click, redirected output)
    let _ = writeln!(io::stdout(), "{s}");
}

/// `true` = go on. Without a console there is nobody to ask → `None`.
fn ask(question: &str) -> Option<bool> {
    if !io::stdin().is_terminal() {
        return None;
    }
    let _ = write!(io::stdout(), "{question} [j/N] ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    let _ = io::stdin().lock().read_line(&mut line);
    Some(matches!(
        line.trim().to_lowercase().as_str(),
        "j" | "ja" | "y" | "yes"
    ))
}

fn report(r: &ResetReport) {
    out(&r.summary_de());
    for k in &r.kept {
        out(&format!("  belassen: {k}"));
    }
    for e in &r.errors {
        out(&format!("  Fehler: {e}"));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] => Mode::Reset { yes: false },
        ["--yes"] | ["-y"] => Mode::Reset { yes: true },
        ["--status"] => Mode::Status,
        ["--uninstall"] => Mode::Uninstall { quiet: false },
        ["--uninstall", "--quiet"] | ["--quiet", "--uninstall"] => Mode::Uninstall { quiet: true },
        ["--uninstall-finish", dir] => Mode::UninstallFinish(PathBuf::from(dir)),
        ["--help"] | ["-h"] | ["/?"] => {
            out(USAGE);
            return;
        }
        _ => {
            out(USAGE);
            std::process::exit(2);
        }
    };
    if let Mode::UninstallFinish(dir) = &mode {
        std::process::exit(finish_uninstall(dir));
    }
    let Some(paths) = JournalPaths::default_paths() else {
        out("Fehler: Konfigurationsordner nicht gefunden (APPDATA fehlt).");
        std::process::exit(1);
    };
    let mut reg: Box<dyn Registry> = match explorer_integrate::system_registry() {
        Some(r) => r,
        // other platforms: no registry, files only
        None => Box::new(MemRegistry::new()),
    };
    out("Explorer 2.0 – Windows-Standard wiederherstellen");
    out(&format!("Journal: {}", paths.primary.display()));
    let st = status(reg.as_mut(), &paths);
    out(&format!("Stand: {}", st.summary_de()));
    match mode {
        Mode::Status => std::process::exit(if st.any() { 1 } else { 0 }),
        Mode::Reset { yes } => {
            if !yes {
                match ask("Jetzt alles auf Windows-Standard zurücksetzen?") {
                    Some(true) => {}
                    Some(false) => {
                        out("Abgebrochen – nichts geändert.");
                        return;
                    }
                    None => {
                        out("Keine Konsole für die Nachfrage – bitte mit --yes starten.");
                        std::process::exit(2);
                    }
                }
            }
            let r = reset(reg.as_mut(), &paths);
            report(&r);
            std::process::exit(if r.ok() { 0 } else { 1 });
        }
        Mode::Uninstall { quiet } => {
            let Some(ip) = InstallPaths::default_paths() else {
                out("Fehler: Installationsordner nicht bestimmbar (LOCALAPPDATA fehlt).");
                std::process::exit(1);
            };
            out(&format!("Installation: {}", ip.program_dir.display()));
            if !quiet {
                match ask(
                    "Explorer 2.0 deinstallieren und alle Windows-Integrationen zurücksetzen?",
                ) {
                    Some(true) => {}
                    Some(false) => {
                        out("Abgebrochen – nichts geändert.");
                        return;
                    }
                    None => {
                        out("Keine Konsole für die Nachfrage – bitte mit --quiet starten.");
                        std::process::exit(2);
                    }
                }
            }
            let r = install::uninstall(reg.as_mut(), &paths, &ip);
            report(&r);
            let code = remove_installation(&ip.program_dir);
            std::process::exit(if r.ok() && code == 0 { 0 } else { 1 });
        }
        Mode::UninstallFinish(_) => unreachable!(),
    }
}

/// A running exe cannot delete its own folder: when we live inside it, a copy
/// in %TEMP% finishes the job after we exit.
fn remove_installation(dir: &Path) -> i32 {
    let me = std::env::current_exe().ok();
    let inside = me
        .as_deref()
        .and_then(|m| m.parent())
        .is_some_and(|p| same_dir(p, dir));
    if !inside {
        return finish_uninstall(dir);
    }
    let Some(me) = me else { return 1 };
    let copy = std::env::temp_dir().join(format!(
        "explorer2-uninstall-{}{}",
        std::process::id(),
        if cfg!(windows) { ".exe" } else { "" }
    ));
    if let Err(e) = std::fs::copy(&me, &copy) {
        out(&format!(
            "Fehler: {e} – Programmordner bitte von Hand löschen: {}",
            dir.display()
        ));
        return 1;
    }
    match std::process::Command::new(&copy)
        .arg("--uninstall-finish")
        .arg(dir)
        .spawn()
    {
        Ok(_) => {
            out("Programmordner wird entfernt …");
            0
        }
        Err(e) => {
            out(&format!(
                "Fehler: {e} – Programmordner bitte von Hand löschen: {}",
                dir.display()
            ));
            1
        }
    }
}

fn same_dir(a: &Path, b: &Path) -> bool {
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let (a, b) = (canon(a), canon(b));
    if cfg!(windows) {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    } else {
        a == b
    }
}

/// Delete the program folder (waits for the app/uninstaller to exit), then
/// let the temp copy remove itself.
fn finish_uninstall(dir: &Path) -> i32 {
    let code = match install::remove_program_dir(dir, 40) {
        Ok(()) => {
            out("Explorer 2.0 wurde entfernt. Ihre Einstellungen bleiben in %APPDATA%\\Explorer2.");
            0
        }
        Err(e) => {
            out(&format!(
                "Programmordner nicht entfernt ({e}) – ist Explorer 2.0 noch offen? Ordner: {}",
                dir.display()
            ));
            1
        }
    };
    #[cfg(windows)]
    if let Ok(me) = std::env::current_exe() {
        if me
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("explorer2-uninstall-"))
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let _ = std::process::Command::new("cmd")
                .args([
                    "/C",
                    "ping",
                    "-n",
                    "3",
                    "127.0.0.1",
                    ">nul",
                    "&",
                    "del",
                    "/f",
                    "/q",
                ])
                .arg(&me)
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
        }
    }
    code
}
