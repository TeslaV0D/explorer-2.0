//! Crash reports + start guard (M8.3, logic in `explorer_core::guard`).
//!
//! - Panics (any thread) and, on Windows, unhandled native exceptions write
//!   `<config>/crash/crash-<utc>-<pid>.txt`: version, cause, backtrace, the
//!   last trace lines (kept in memory even without `EXPLORER2_TRACE`).
//! - `start-guard.txt` detects starts that died before running stably; after
//!   two in a row the next start is a safe one (software renderer, no session).
//! - `gpu-probe.txt` marks a running GPU probe: if a probe takes the process
//!   down (driver crash), the GPU counts as unusable from then on.
//!
//! Test hook: `EXPLORER2_TEST_CRASH=panic|panic-later|native` (see `test_hook`).

use crate::config;
use explorer_core::guard::{self, StartRecord, Verdict};
use std::collections::VecDeque;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const RING_LINES: usize = 200;
static RING: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());
/// Config dir resolved once at install (the hook must not allocate much or lock config).
static DIR: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();

/// Every trace line also lands here (cheap: callers format anyway).
pub fn remember(line: &str) {
    if let Ok(mut r) = RING.try_lock() {
        if r.len() >= RING_LINES {
            r.pop_front();
        }
        r.push_back(line.to_string());
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

pub fn crash_dir() -> Option<PathBuf> {
    DIR.get_or_init(config::config_dir)
        .as_ref()
        .map(|d| d.join("crash"))
}

/// Writes a report; returns its path. Never panics (runs inside the panic hook).
fn write_report(cause: &str, detail: &str) -> Option<PathBuf> {
    let dir = crash_dir()?;
    let _ = std::fs::create_dir_all(&dir);
    let secs = now_secs();
    let path = dir.join(guard::report_name(secs, std::process::id()));
    let mut f = std::fs::File::create(&path).ok()?;
    let _ = writeln!(
        f,
        "Explorer 2.0 {} – Absturzbericht\nZeit: {}\nSystem: {} {}\nUrsache: {cause}\n\n{detail}\n",
        env!("CARGO_PKG_VERSION"),
        guard::utc_text(secs),
        std::env::consts::OS,
        std::env::consts::ARCH,
    );
    // try_lock: the panic may have happened while the ring was locked
    if let Ok(r) = RING.try_lock() {
        let _ = writeln!(f, "Letzte Protokollzeilen ({}):", r.len());
        for l in r.iter() {
            let _ = writeln!(f, "  {l}");
        }
    }
    let _ = f.flush();
    Some(path)
}

/// Release exes carry no symbols: a backtrace of only `<unknown>` frames says
/// nothing → one line instead of dozens.
fn backtrace_text(bt: &str) -> String {
    let useful = bt.lines().any(|l| {
        let t = l.trim();
        !t.is_empty() && !t.ends_with("<unknown>")
    });
    if useful {
        bt.to_string()
    } else {
        format!(
            "(ohne Symbole in der Release-Exe, {} Frames)",
            bt.lines().count()
        )
    }
}

/// Install the hooks as early as possible in `main`.
pub fn install() {
    let _ = crash_dir();
    std::panic::set_hook(Box::new(|info| {
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "(keine Meldung)".into());
        let loc = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        let thread = std::thread::current()
            .name()
            .unwrap_or("(ohne Namen)")
            .to_string();
        let bt = backtrace_text(&std::backtrace::Backtrace::force_capture().to_string());
        let path = write_report(
            "Panic",
            &format!("Thread: {thread}\nMeldung: {msg}\nOrt: {loc}\n\nBacktrace:\n{bt}"),
        );
        let _ = writeln!(
            std::io::stderr(),
            "explorer2: panic in thread '{thread}' at {loc}: {msg} (report: {path:?})"
        );
    }));
    #[cfg(windows)]
    win::install();
    #[cfg(unix)]
    unix::install();
}

/// SIGTERM (`kill`, session logout) is a deliberate end, not a crash: forget
/// the start record so it does not count for the start guard.
#[cfg(unix)]
mod unix {
    use std::ffi::CString;
    use std::sync::OnceLock;

    static GUARD: OnceLock<CString> = OnceLock::new();

    extern "C" fn on_term(_: libc::c_int) {
        if let Some(p) = GUARD.get() {
            // SAFETY: unlink and _exit are async-signal-safe; the path was built before.
            unsafe {
                libc::unlink(p.as_ptr());
            }
        }
        // SAFETY: async-signal-safe process exit with the usual 128 + SIGTERM.
        unsafe { libc::_exit(143) }
    }

    pub fn install() {
        let Some(p) = super::guard_file() else { return };
        let Ok(c) = CString::new(p.as_os_str().as_encoded_bytes()) else {
            return;
        };
        let _ = GUARD.set(c);
        // SAFETY: installs a handler that only calls async-signal-safe functions.
        unsafe {
            libc::signal(libc::SIGTERM, on_term as *const () as libc::sighandler_t);
        }
    }
}

#[cfg(windows)]
mod win {
    use windows_sys::Win32::System::Diagnostics::Debug::{
        SetUnhandledExceptionFilter, EXCEPTION_POINTERS,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;

    /// Last chance for native crashes (access violation, driver faults).
    unsafe extern "system" fn filter(info: *const EXCEPTION_POINTERS) -> i32 {
        // SAFETY: Windows passes valid pointers to the filter (null-checked anyway).
        let (code, addr) = unsafe {
            match info.as_ref().and_then(|i| i.ExceptionRecord.as_ref()) {
                Some(r) => (r.ExceptionCode as u32, r.ExceptionAddress as usize),
                None => (0, 0),
            }
        };
        // SAFETY: null = the exe's own module handle, no ownership involved.
        let base = unsafe { GetModuleHandleW(std::ptr::null()) } as usize;
        let rva = addr
            .checked_sub(base)
            .map_or("außerhalb der Exe".to_string(), |r| format!("exe+{r:#x}"));
        let thread = std::thread::current()
            .name()
            .unwrap_or("(ohne Namen)")
            .to_string();
        let _ = super::write_report(
            "Windows-Ausnahme",
            &format!(
                "Thread: {thread}\nCode: {code:#010x}\nAdresse: {addr:#x} ({rva})\nExe-Basis: {base:#x}"
            ),
        );
        // EXCEPTION_EXECUTE_HANDLER: end the process quietly (the report is
        // shown on the next start) instead of a "has stopped working" dialog
        1
    }

    pub fn install() {
        // SAFETY: installs a plain function pointer.
        unsafe {
            SetUnhandledExceptionFilter(Some(filter));
        }
    }

    pub fn raise_test_exception() {
        // SAFETY: a custom software exception, handled by `filter`.
        unsafe {
            windows_sys::Win32::System::Diagnostics::Debug::RaiseException(
                0xE000_2001,
                0,
                0,
                std::ptr::null(),
            );
        }
    }
}

/// Is `pid` a running Explorer 2.0 (not just any process that got the pid again)?
fn explorer2_alive(pid: u32) -> bool {
    if pid == std::process::id() {
        return true;
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, QueryFullProcessImageNameW,
            PROCESS_QUERY_LIMITED_INFORMATION,
        };
        // SAFETY: handle checked, buffers sized, handle closed.
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return false;
            }
            let mut code = 0u32;
            let running = GetExitCodeProcess(h, &mut code) != 0 && code == STILL_ACTIVE as u32;
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let name = if QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len) != 0 {
                String::from_utf16_lossy(&buf[..len as usize])
            } else {
                String::new()
            };
            CloseHandle(h);
            running && is_explorer2_image(&name)
        }
    }
    #[cfg(not(windows))]
    {
        std::fs::read_link(format!("/proc/{pid}/exe"))
            .map(|p| is_explorer2_image(&p.to_string_lossy()))
            .unwrap_or(false)
    }
}

fn is_explorer2_image(path: &str) -> bool {
    // both separators: the record may come from either platform's API
    let name = path.rsplit(['/', '\\']).next().unwrap_or("").to_lowercase();
    name.starts_with("explorer2") && !name.starts_with("explorer2-recover")
}

fn guard_file() -> Option<PathBuf> {
    config::config_dir().map(|d| d.join("start-guard.txt"))
}

fn probe_file() -> Option<PathBuf> {
    config::config_dir().map(|d| d.join("gpu-probe.txt"))
}

/// Record read from `path`, plus "its process still runs and started recently".
fn read_record(path: &Path) -> (Option<StartRecord>, bool) {
    let rec = std::fs::read_to_string(path)
        .ok()
        .and_then(|t| StartRecord::parse(&t));
    let recent = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age < Duration::from_secs(60));
    let alive = rec.is_some_and(|r| recent && explorer2_alive(r.pid));
    (rec, alive)
}

/// Before the window exists: did the last starts die? Writes our record.
pub fn begin() -> Verdict {
    let Some(path) = guard_file() else {
        return Verdict::Skip;
    };
    let (prev, alive) = read_record(&path);
    let v = guard::verdict(prev, alive);
    if let Verdict::Guard { failed } = v {
        let rec = StartRecord {
            pid: std::process::id(),
            failed,
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&path, rec.serialize());
    }
    v
}

/// Ran stably (or closed cleanly): forget this start.
pub fn stable() {
    let Some(path) = guard_file() else { return };
    let (rec, _) = read_record(&path);
    if rec.is_some_and(|r| r.pid == std::process::id()) {
        let _ = std::fs::remove_file(&path);
    }
}

/// A GPU probe that took its process down: `true` once (the marker is removed).
pub fn probe_crashed_before() -> bool {
    let Some(path) = probe_file() else {
        return false;
    };
    let (rec, alive) = read_record(&path);
    if rec.is_none() || alive {
        return false;
    }
    let _ = std::fs::remove_file(&path);
    true
}

pub fn probe_begin() {
    if let Some(p) = probe_file() {
        let rec = StartRecord {
            pid: std::process::id(),
            failed: 0,
        };
        let _ = std::fs::write(p, rec.serialize());
    }
}

pub fn probe_end() {
    if let Some(p) = probe_file() {
        let _ = std::fs::remove_file(p);
    }
}

/// New reports since the last start: marks them seen, prunes old ones.
pub fn take_new_reports() -> usize {
    let Some(dir) = crash_dir() else { return 0 };
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return 0;
    };
    let names: Vec<String> = rd
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    let mut new = 0;
    for n in &names {
        if guard::is_new_report(n) {
            new += 1;
            let _ = std::fs::rename(dir.join(n), dir.join(guard::seen_name(n)));
        }
    }
    let names: Vec<String> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    for old in guard::surplus_reports(names) {
        let _ = std::fs::remove_file(dir.join(old));
    }
    new
}

/// `EXPLORER2_TEST_CRASH`: `panic` / `native` right at start (before the
/// window, counts for the start guard), `panic-later` in a worker thread
/// once the start counts as stable (a crash during normal use).
pub fn test_hook() {
    let Ok(what) = std::env::var("EXPLORER2_TEST_CRASH") else {
        return;
    };
    match what.as_str() {
        "panic" => panic!("test crash requested (EXPLORER2_TEST_CRASH=panic)"),
        "native" => {
            #[cfg(windows)]
            win::raise_test_exception();
            // no hook on other platforms: a hard abort without report
            std::process::abort();
        }
        "panic-later" => {
            let _ = std::thread::Builder::new()
                .name("test-crash".into())
                .spawn(|| {
                    std::thread::sleep(Duration::from_secs(guard::STABLE_AFTER_SECS + 1));
                    panic!("test crash in a worker thread (EXPLORER2_TEST_CRASH=panic-later)");
                });
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backtraces() {
        assert_eq!(
            backtrace_text("   0: <unknown>\n   1: <unknown>\n"),
            "(ohne Symbole in der Release-Exe, 2 Frames)"
        );
        let real = "   0: explorer2::main\n             at src/main.rs:5\n";
        assert_eq!(backtrace_text(real), real);
    }

    #[test]
    fn image_names() {
        assert!(is_explorer2_image(
            r"C:\Users\a\AppData\Local\Programs\Explorer2\explorer2.exe"
        ));
        assert!(is_explorer2_image("/opt/x/explorer2"));
        assert!(is_explorer2_image(r"C:\x\explorer2.old-123"));
        assert!(!is_explorer2_image(r"C:\x\explorer2-recover.exe"));
        assert!(!is_explorer2_image(r"C:\Windows\svchost.exe"));
        assert!(!is_explorer2_image(""));
    }

    #[test]
    fn own_pid_is_alive_others_mostly_not() {
        assert!(explorer2_alive(std::process::id()));
        // pid 1 is never an explorer2
        assert!(!explorer2_alive(1));
    }
}
