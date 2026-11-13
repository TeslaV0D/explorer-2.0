//! One window per user (M8.3b): a second start hands its folder to the
//! running instance (new tab, window to the front) and exits. Protocol and
//! endpoint name in `explorer_core::instance`.
//!
//! - Windows: named pipe `\\.\pipe\explorer2-<session>-<hash>` (local clients
//!   only, default ACL = same user).
//! - Unix: socket `$TMPDIR/explorer2-<uid>-<hash>.sock` (mode 0600).
//!
//! The answer comes only after the UI thread really opened the tab; a hung
//! instance answers nothing → the second start opens its own window.
//! `--new-instance` / `EXPLORER2_NEW_INSTANCE=1` / setting off: no hand-over.

use crate::{nav, MainWindow};
use explorer_core::instance::{self as proto, Request};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandOver {
    /// The running window took it: this process exits.
    Done,
    /// Nobody listens: this process becomes the instance.
    NoInstance,
    /// Someone listens but did not answer (hangs): own window.
    NoAnswer,
}

fn endpoint() -> Option<String> {
    let dir = crate::config::config_dir()?;
    Some(proto::endpoint_id(
        &dir.to_string_lossy(),
        cfg!(windows),
        session_or_uid(),
    ))
}

#[cfg(windows)]
fn session_or_uid() -> u32 {
    let mut s = 0u32;
    // SAFETY: plain out-parameter call.
    unsafe {
        windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId(std::process::id(), &mut s);
    }
    s
}

#[cfg(unix)]
fn session_or_uid() -> u32 {
    // SAFETY: getuid has no preconditions.
    unsafe { libc::getuid() }
}

#[cfg(windows)]
fn pipe_path(id: &str) -> String {
    format!(r"\\.\pipe\{id}")
}

#[cfg(unix)]
pub fn socket_path(id: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{id}.sock"))
}

/// Whether this start may hand over at all.
pub fn wanted(args: &[String]) -> bool {
    !args.iter().any(|a| a == "--new-instance")
        && std::env::var_os("EXPLORER2_NEW_INSTANCE").is_none()
        && crate::config::load().single_instance
}

/// Second start: try to give `start` to the running window.
pub fn hand_over(start: Option<&Path>) -> HandOver {
    let Some(id) = endpoint() else {
        return HandOver::NoInstance;
    };
    let req = Request::Open(start.map(|p| p.to_string_lossy().into_owned())).encode();
    // the window to come to the front must be allowed to (we are the one
    // the user just started, so we have the right to pass on)
    #[cfg(windows)]
    // SAFETY: plain call; failure only means the window may flash instead.
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(
            windows_sys::Win32::UI::WindowsAndMessaging::ASFW_ANY,
        );
    }
    let Some(mut stream) = connect(&id) else {
        return HandOver::NoInstance;
    };
    // the exchange runs in a thread: a hung server must not hang us
    let (tx, rx) = mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("instance-client".into())
        .spawn(move || {
            let ok = stream.write_all(req.as_bytes()).is_ok() && stream.flush().is_ok() && {
                let mut line = String::new();
                let mut r = BufReader::new(&mut stream).take(64);
                r.read_line(&mut line).is_ok() && line == proto::REPLY_OK
            };
            let _ = tx.send(ok);
        });
    match rx.recv_timeout(Duration::from_millis(proto::CLIENT_WAIT_MS)) {
        Ok(true) => HandOver::Done,
        _ => HandOver::NoAnswer,
    }
}

#[cfg(windows)]
fn connect(id: &str) -> Option<std::fs::File> {
    let path = pipe_path(id);
    for _ in 0..20 {
        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
        {
            Ok(f) => return Some(f),
            // all pipe instances busy for a moment
            Err(e)
                if e.raw_os_error()
                    == Some(windows_sys::Win32::Foundation::ERROR_PIPE_BUSY as i32) =>
            {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => return None,
        }
    }
    None
}

#[cfg(unix)]
fn connect(id: &str) -> Option<std::os::unix::net::UnixStream> {
    std::os::unix::net::UnixStream::connect(socket_path(id)).ok()
}

/// One request from a second start: open the tab on the UI thread, answer
/// only when that happened.
fn handle(stream: &mut (impl Read + Write), weak: &slint::Weak<MainWindow>) {
    let mut line = String::new();
    {
        let mut r = BufReader::new(&mut *stream).take(proto::MAX_LINE as u64 + 2);
        if r.read_line(&mut line).is_err() {
            return;
        }
    }
    let Some(Request::Open(path)) = Request::decode(&line) else {
        nav::trace_line("instance: ignored a malformed request");
        return;
    };
    let path = path.map(PathBuf::from).and_then(|p| {
        // same rule as a path on the command line
        explorer_fs::nearest_existing_ancestor(&p)
    });
    let (tx, rx) = mpsc::channel();
    let weak = weak.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(w) = weak.upgrade() {
            nav::trace_line(&format!(
                "instance: open {}",
                path.as_ref()
                    .map_or("(new tab)".to_string(), |p| p.display().to_string())
            ));
            nav::open_tab_at(&w, path);
            raise(&w);
            let _ = tx.send(());
        }
    });
    if rx
        .recv_timeout(Duration::from_millis(proto::SERVER_WAIT_MS))
        .is_ok()
    {
        let _ = stream.write_all(proto::REPLY_OK.as_bytes());
        let _ = stream.flush();
    }
}

/// Window to the front. Also undoes a hide (tray / `--background`).
pub fn raise(win: &MainWindow) {
    use slint::ComponentHandle;
    let _ = win.show();
    win.window().set_minimized(false);
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
        };
        let hwnd = crate::fileops::window_handle(win) as windows_sys::Win32::Foundation::HWND;
        if !hwnd.is_null() {
            // SAFETY: our own window handle.
            unsafe {
                let cmd = if IsIconic(hwnd) != 0 {
                    SW_RESTORE
                } else {
                    SW_SHOW
                };
                ShowWindow(hwnd, cmd);
                let ok = SetForegroundWindow(hwnd) != 0;
                nav::trace_line(&format!("instance: raised foreground={ok}"));
            }
        }
    }
    #[cfg(not(windows))]
    {
        nav::trace_line("instance: raised");
    }
    // A hide dropped the folder listings. The window is visible again: load them.
    nav::on_window_shown(win);
}

/// The running window listens for second starts (background thread). If
/// another instance still holds the name (an old one closing, or a start with
/// `--new-instance`), it keeps trying and takes over once that one is gone.
pub fn serve(win: &MainWindow) {
    let Some(id) = endpoint() else { return };
    let weak = slint::ComponentHandle::as_weak(win);
    let _ = std::thread::Builder::new()
        .name("instance-server".into())
        .spawn(move || server_loop(&id, &weak));
}

#[cfg(windows)]
fn server_loop(id: &str, weak: &slint::Weak<MainWindow>) {
    use std::os::windows::io::FromRawHandle;
    use windows_sys::Win32::Foundation::{
        GetLastError, ERROR_PIPE_CONNECTED, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        FlushFileBuffers, FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX,
    };
    use windows_sys::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE,
        PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    };
    let name: Vec<u16> = pipe_path(id).encode_utf16().chain([0]).collect();
    let create = |first: bool| {
        // SAFETY: valid NUL-terminated name, default security, no overlapped I/O.
        let h = unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                PIPE_ACCESS_DUPLEX
                    | if first {
                        FILE_FLAG_FIRST_PIPE_INSTANCE
                    } else {
                        0
                    },
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                PIPE_UNLIMITED_INSTANCES,
                4096,
                4096,
                0,
                std::ptr::null(),
            )
        };
        (h != INVALID_HANDLE_VALUE).then_some(h)
    };
    // become the owner of the name (the first instance)
    let mut waited = false;
    let mut next = loop {
        if let Some(h) = create(true) {
            break h;
        }
        if !waited {
            nav::trace_line("instance: another instance owns the name, waiting");
            waited = true;
        }
        std::thread::sleep(Duration::from_secs(2));
    };
    nav::trace_line(&format!("instance: serving {}", pipe_path(id)));
    loop {
        let cur = next;
        // SAFETY: valid pipe handle, synchronous connect.
        let connected = unsafe {
            ConnectNamedPipe(cur, std::ptr::null_mut()) != 0
                || GetLastError() == ERROR_PIPE_CONNECTED
        };
        // the next instance exists before this one closes → the name never vanishes
        next = loop {
            if let Some(h) = create(false) {
                break h;
            }
            std::thread::sleep(Duration::from_millis(200));
        };
        // SAFETY: we own `cur`; the File closes it when dropped.
        let mut f = unsafe { std::fs::File::from_raw_handle(cur as _) };
        if connected {
            handle(&mut f, weak);
            // SAFETY: valid handle; lets the client read the answer before we close.
            unsafe {
                FlushFileBuffers(cur);
                DisconnectNamedPipe(cur);
            }
        }
        drop(f);
    }
}

#[cfg(unix)]
fn server_loop(id: &str, weak: &slint::Weak<MainWindow>) {
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::{UnixListener, UnixStream};
    let path = socket_path(id);
    let mut waited = false;
    let listener = loop {
        match UnixListener::bind(&path) {
            Ok(l) => break l,
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                if UnixStream::connect(&path).is_err() {
                    // left over from a crashed instance
                    let _ = std::fs::remove_file(&path);
                    continue;
                }
                if !waited {
                    nav::trace_line("instance: another instance owns the name, waiting");
                    waited = true;
                }
                std::thread::sleep(Duration::from_secs(2));
            }
            Err(e) => {
                nav::trace_line(&format!("instance: cannot listen: {e}"));
                return;
            }
        }
    };
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    crate::crash::own_socket(&path);
    nav::trace_line(&format!("instance: serving {}", path.display()));
    for s in listener.incoming() {
        let Ok(mut s) = s else { continue };
        let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
        handle(&mut s, weak);
    }
}

/// Clean exit: give the name free right away (Windows frees it with the process).
pub fn shutdown() {
    #[cfg(unix)]
    crate::crash::release_socket();
}
