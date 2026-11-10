//! `Explorer2-Setup.exe`: one download, double-click, installed.
//!
//! Carries `explorer2.exe` + `explorer2-recover.exe` (see `build.rs`),
//! unpacks them into a temp folder and runs the app's own installer there
//! (`explorer2.exe --install --quiet`, D-023: per user, no admin, journaled,
//! uninstall via "Apps & Features"). The setup adds no install logic of its
//! own, so the installed state is exactly the one `--install` creates.
//!
//! `--quiet` (also `/quiet`, `/S`): no dialogs, no start afterwards; result on
//! stdout and in the exit code (0 = installed, 1 = failed, 2 = setup without
//! content, 1223 = cancelled).
#![cfg_attr(windows, windows_subsystem = "windows")]

use explorer_integrate::install::{self, InstallPaths, APP_EXE};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const EXIT_CANCELLED: i32 = 1223; // ERROR_CANCELLED

#[cfg(payload)]
fn payload() -> Option<[(&'static str, &'static [u8]); 2]> {
    Some([
        (APP_EXE, include_bytes!(env!("EXPLORER2_PAYLOAD_APP"))),
        (
            install::RECOVER_EXE,
            include_bytes!(env!("EXPLORER2_PAYLOAD_RECOVER")),
        ),
    ])
}

#[cfg(not(payload))]
fn payload() -> Option<[(&'static str, &'static [u8]); 2]> {
    None
}

fn main() {
    let quiet = std::env::args()
        .skip(1)
        .any(|a| matches!(a.as_str(), "--quiet" | "/quiet" | "/S" | "/s"));
    // no settings file yet: the Windows language (or EXPLORER2_LANG)
    explorer_i18n::init("");
    let code = run(quiet);
    std::process::exit(code);
}

fn run(quiet: bool) -> i32 {
    let say = |text: &str| {
        // GUI subsystem: stdout may be missing → never panic on it
        let _ = writeln!(std::io::stdout(), "{text}");
    };
    let fail = |text: &str| {
        say(text);
        if !quiet {
            ui::error(text);
        }
        1
    };
    let Some(files) = payload() else {
        say(explorer_i18n::tr!(
            "This setup was built without the program files."
        ));
        if !quiet {
            ui::error(explorer_i18n::tr!(
                "This setup was built without the program files."
            ));
        }
        return 2;
    };
    let Some(paths) = InstallPaths::default_paths() else {
        return fail(explorer_i18n::tr!(
            "Installation folder cannot be determined (LOCALAPPDATA/APPDATA missing)."
        ));
    };
    if !quiet && !ui::confirm(&question(&paths)) {
        say(explorer_i18n::tr!("Cancelled."));
        return EXIT_CANCELLED;
    }

    let tmp = std::env::temp_dir().join(format!("Explorer2-Setup-{}", std::process::id()));
    let result = unpack(&tmp, &files).and_then(|()| install_from(&tmp));
    remove_dir(&tmp);
    match result {
        Ok(text) => say(&text),
        Err(e) => return fail(&explorer_i18n::tr!("Installation failed: {0}", e)),
    }
    if !quiet
        && ui::ask(&explorer_i18n::tr!(
            "Explorer 2.0 {0} is installed.\n\nYou find it in the Start menu under “Explorer 2.0”. To uninstall, use “Settings → Apps” or the Start menu entry “Explorer 2.0 – Windows-Standard wiederherstellen”.\n\nStart it now?",
            VERSION
        ))
    {
        // `--new-instance`: an older copy that is still running must not get
        // the window (it would hand over to itself, D-023 update case)
        if let Err(e) = Command::new(paths.app()).arg("--new-instance").spawn() {
            ui::error(&explorer_i18n::tr!("Installed, but starting failed: {0}", e));
        }
    }
    0
}

fn question(paths: &InstallPaths) -> String {
    let installed = explorer_integrate::system_registry()
        .is_some_and(|reg| install::is_installed(reg.as_ref(), paths));
    let what = if installed {
        explorer_i18n::tr!(
            "Explorer 2.0 is already installed. Update to version {0}?",
            VERSION
        )
    } else {
        explorer_i18n::tr!("Install Explorer 2.0 {0} for this user?", VERSION)
    };
    explorer_i18n::tr!(
        "{0}\n\nTarget: {1}\n\nWithout administrator rights; a Start menu entry and “Apps & Features” are created. You switch on Windows integrations (open folders, Win+E) yourself afterwards in the settings – undoable at any time with one button.",
        what,
        paths.program_dir.display()
    )
}

fn unpack(dir: &Path, files: &[(&str, &[u8])]) -> Result<(), String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| explorer_i18n::tr!("Temp folder {0}: {1}", dir.display(), e))?;
    for (name, bytes) in files {
        let path = dir.join(name);
        std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755));
        }
    }
    Ok(())
}

/// The app's own installer; `--quiet` = it starts nothing itself, so no
/// child inherits our output pipe and `output()` returns right away.
fn install_from(dir: &Path) -> Result<String, String> {
    let app: PathBuf = dir.join(APP_EXE);
    let out = Command::new(&app)
        .args(["--install", "--quiet"])
        .output()
        .map_err(|e| explorer_i18n::tr!("{0} cannot be started: {1}", app.display(), e))?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if out.status.success() {
        Ok(text)
    } else if text.is_empty() {
        Err(explorer_i18n::tr!("Installer ended with {0}", out.status))
    } else {
        Err(text)
    }
}

/// Virus scanners may still hold the fresh files for a moment.
fn remove_dir(dir: &Path) {
    for _ in 0..10 {
        if std::fs::remove_dir_all(dir).is_ok() || !dir.exists() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

#[cfg(windows)]
mod ui {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, IDOK, IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONQUESTION, MB_OK,
        MB_OKCANCEL, MB_SETFOREGROUND, MB_YESNO, MESSAGEBOX_STYLE,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn show(text: &str, style: MESSAGEBOX_STYLE) -> i32 {
        let (t, c) = (wide(text), wide("Explorer 2.0 Setup"));
        // SAFETY: both strings are NUL-terminated and outlive the call.
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                t.as_ptr(),
                c.as_ptr(),
                style | MB_SETFOREGROUND,
            )
        }
    }

    pub fn confirm(text: &str) -> bool {
        show(text, MB_OKCANCEL | MB_ICONQUESTION) == IDOK
    }

    pub fn ask(text: &str) -> bool {
        show(text, MB_YESNO | MB_ICONINFORMATION) == IDYES
    }

    pub fn error(text: &str) {
        show(text, MB_OK | MB_ICONERROR);
    }
}

/// Elsewhere (only for tests of the flow): text on stderr, no questions.
#[cfg(not(windows))]
mod ui {
    use std::io::Write;

    pub fn confirm(text: &str) -> bool {
        let _ = writeln!(std::io::stderr(), "{text}");
        true
    }

    pub fn ask(text: &str) -> bool {
        let _ = writeln!(std::io::stderr(), "{text}");
        false
    }

    pub fn error(text: &str) {
        let _ = writeln!(std::io::stderr(), "{text}");
    }
}
