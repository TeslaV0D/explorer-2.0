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

fn usage() -> &'static str {
    explorer_i18n::tr!(
        "Explorer 2.0 – restore the Windows default

  explorer2-recover                  shows the state, asks and resets
  explorer2-recover --yes            resets without asking
  explorer2-recover --status         only shows the state (exit code 1 = active)
  explorer2-recover --uninstall      resets and removes Explorer 2.0
  explorer2-recover --uninstall --quiet   the same without asking

Removes all Windows integrations of Explorer 2.0 (folder verbs, Win+E,
context menu, environment variables, shortcuts …) and restores the previous
values exactly. Values that other software has changed meanwhile stay
untouched. Your settings (%APPDATA%\\Explorer2) are kept when uninstalling."
    )
}

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
    let _ = write!(io::stdout(), "{question} {} ", explorer_i18n::tr!("[y/N]"));
    let _ = io::stdout().flush();
    let mut line = String::new();
    let _ = io::stdin().lock().read_line(&mut line);
    // English and German answers always work, plus the current language's word
    let a = line.trim().to_lowercase();
    let yes = explorer_i18n::tr!("yes").to_lowercase();
    Some(
        matches!(a.as_str(), "j" | "ja" | "y" | "yes")
            || (!a.is_empty() && (a == yes || yes.starts_with(a.as_str()))),
    )
}

fn report(r: &ResetReport) {
    out(&r.summary());
    for k in &r.kept {
        out(&explorer_i18n::tr!("  left as it is: {0}", k));
    }
    for e in &r.errors {
        out(&explorer_i18n::tr!("  error: {0}", e));
    }
}

fn main() {
    // no settings file here: the Windows language (or EXPLORER2_LANG)
    explorer_i18n::init("");
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
            out(usage());
            return;
        }
        _ => {
            out(usage());
            std::process::exit(2);
        }
    };
    if let Mode::UninstallFinish(dir) = &mode {
        std::process::exit(finish_uninstall(dir));
    }
    let Some(paths) = JournalPaths::default_paths() else {
        out(explorer_i18n::tr!(
            "Error: configuration folder not found (APPDATA missing)."
        ));
        std::process::exit(1);
    };
    let mut reg: Box<dyn Registry> = match explorer_integrate::system_registry() {
        Some(r) => r,
        // other platforms: no registry, files only
        None => Box::new(MemRegistry::new()),
    };
    out(explorer_i18n::tr!(
        "Explorer 2.0 – restore the Windows default"
    ));
    out(&explorer_i18n::tr!("Journal: {0}", paths.primary.display()));
    let st = status(reg.as_mut(), &paths);
    out(&explorer_i18n::tr!("State: {0}", st.summary()));
    match mode {
        Mode::Status => std::process::exit(if st.any() { 1 } else { 0 }),
        Mode::Reset { yes } => {
            if !yes {
                match ask(explorer_i18n::tr!(
                    "Reset everything to the Windows default now?"
                )) {
                    Some(true) => {}
                    Some(false) => {
                        out(explorer_i18n::tr!("Cancelled – nothing changed."));
                        return;
                    }
                    None => {
                        out(explorer_i18n::tr!(
                            "No console to ask – please start with --yes."
                        ));
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
                out(explorer_i18n::tr!(
                    "Error: installation folder cannot be determined (LOCALAPPDATA missing)."
                ));
                std::process::exit(1);
            };
            out(&explorer_i18n::tr!(
                "Installation: {0}",
                ip.program_dir.display()
            ));
            if !quiet {
                match ask(explorer_i18n::tr!(
                    "Uninstall Explorer 2.0 and reset all Windows integrations?"
                )) {
                    Some(true) => {}
                    Some(false) => {
                        out(explorer_i18n::tr!("Cancelled – nothing changed."));
                        return;
                    }
                    None => {
                        out(explorer_i18n::tr!(
                            "No console to ask – please start with --quiet."
                        ));
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
        out(&explorer_i18n::tr!(
            "Error: {0} – please delete the program folder by hand: {1}",
            e,
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
            out(explorer_i18n::tr!("Removing the program folder …"));
            0
        }
        Err(e) => {
            out(&explorer_i18n::tr!(
                "Error: {0} – please delete the program folder by hand: {1}",
                e,
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
            out(explorer_i18n::tr!(
                "Explorer 2.0 was removed. Your settings stay in %APPDATA%\\Explorer2."
            ));
            0
        }
        Err(e) => {
            out(&explorer_i18n::tr!(
                "Program folder not removed ({0}) – is Explorer 2.0 still open? Folder: {1}",
                e,
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
