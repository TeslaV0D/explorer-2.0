//! Opening files with their default application.

use explorer_i18n::tr;
use std::path::Path;

/// Opens `path` like a double-click in Explorer (default verb / default app).
#[cfg(windows)]
pub fn open_path(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let wide = |s: &std::ffi::OsStr| s.encode_wide().chain(Some(0)).collect::<Vec<u16>>();
    let file = wide(path.as_os_str());
    let dir = wide(path.parent().unwrap_or(path).as_os_str());
    // SAFETY: all strings are NUL-terminated and live across the call; null verb = default verb.
    let r = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            std::ptr::null(),
            file.as_ptr(),
            std::ptr::null(),
            dir.as_ptr(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecute returns a value > 32 on success.
    if r as isize > 32 {
        Ok(())
    } else {
        Err(tr!("Opening failed (code {0})", r as isize))
    }
}

/// Non-Windows (dev/tests): `xdg-open`, overridable via `EXPLORER2_OPEN_CMD`.
#[cfg(not(windows))]
pub fn open_path(path: &Path) -> Result<(), String> {
    let cmd = std::env::var("EXPLORER2_OPEN_CMD").unwrap_or_else(|_| "xdg-open".into());
    std::process::Command::new(&cmd)
        .arg(path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| tr!("Opening failed ({0}): {1}", cmd, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real ShellExecute path on Windows: a .cmd file run via its default verb writes a marker.
    #[cfg(windows)]
    #[test]
    fn shell_execute_runs_default_verb() {
        let d = std::env::temp_dir().join(format!("explorer2-open-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let marker = d.join("marker.txt");
        let script = d.join("write marker.cmd"); // space in name on purpose
        std::fs::write(&script, format!("@echo ok> \"{}\"\r\n", marker.display())).unwrap();
        open_path(&script).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while !marker.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(marker.exists(), "default verb did not run");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Non-Windows: the opener receives the exact path (spaces intact); a missing opener is an error.
    #[cfg(not(windows))]
    #[test]
    fn opener_gets_exact_path_and_missing_opener_errors() {
        use std::os::unix::fs::PermissionsExt;
        let d = std::env::temp_dir().join(format!("explorer2-open-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let marker = d.join("marker.txt");
        let opener = d.join("opener.sh");
        std::fs::write(
            &opener,
            format!("#!/bin/sh\nprintf '%s' \"$1\" > '{}'\n", marker.display()),
        )
        .unwrap();
        std::fs::set_permissions(&opener, std::fs::Permissions::from_mode(0o755)).unwrap();
        let file = d.join("my file.txt");
        std::fs::write(&file, b"x").unwrap();
        // Only this test touches EXPLORER2_OPEN_CMD.
        std::env::set_var("EXPLORER2_OPEN_CMD", &opener);
        open_path(&file).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::fs::read_to_string(&marker)
            .map(|s| s.is_empty())
            .unwrap_or(true)
            && std::time::Instant::now() < deadline
        {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(
            std::fs::read_to_string(&marker).unwrap(),
            file.to_string_lossy()
        );
        std::env::set_var("EXPLORER2_OPEN_CMD", d.join("does-not-exist"));
        assert!(open_path(&file).is_err());
        std::env::remove_var("EXPLORER2_OPEN_CMD");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[cfg(windows)]
    #[test]
    fn missing_file_is_error() {
        assert!(open_path(Path::new(r"C:\definitely\missing\file.xyz")).is_err());
    }
}
