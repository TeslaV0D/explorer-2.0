//! Network places: a server (`\\nas`) lists its shares like a folder lists its
//! children; shares that need a login get the Windows credential dialog.
//! Path parsing is pure and tested on every OS; listing and login are Windows-only.

/// A shared folder on a server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Share {
    pub name: String,
    /// Comment set on the server (may be empty).
    pub remark: String,
}

fn unc_parts(s: &str) -> Option<Vec<&str>> {
    let s = s.trim();
    let rest = s
        .strip_prefix(r"\\?\UNC\")
        .or_else(|| s.strip_prefix(r"\\"))
        .or_else(|| s.strip_prefix("//").filter(|_| cfg!(windows)))?;
    if rest.starts_with(['\\', '/']) || rest.starts_with(r"?\") || rest.starts_with(r".\") {
        return None; // "\\\x", "\\?\C:\…", "\\.\pipe"
    }
    let parts: Vec<&str> = rest.split(['\\', '/']).filter(|p| !p.is_empty()).collect();
    (!parts.is_empty()).then_some(parts)
}

/// `\\nas`, `\\nas\` → `Some("nas")`; anything with a share (`\\nas\film`) or
/// a local path → `None` (pure → tested).
pub fn server_root(s: &str) -> Option<String> {
    match unc_parts(s)?.as_slice() {
        [server] => Some(server.to_string()),
        _ => None,
    }
}

/// `\\nas\film\2024` → `Some(("nas", "film"))` (pure → tested).
pub fn share_of(s: &str) -> Option<(String, String)> {
    match unc_parts(s)?.as_slice() {
        [server, share, ..] => Some((server.to_string(), share.to_string())),
        _ => None,
    }
}

/// `\\nas\film` (exactly a share root) → `Some(r"\\nas")`: going up from a
/// share opens its server, like Explorer (pure → tested).
pub fn share_root_parent(s: &str) -> Option<String> {
    match unc_parts(s)?.as_slice() {
        [server, _share] => Some(format!(r"\\{server}")),
        _ => None,
    }
}

/// Visible disk shares only: admin shares (`C$`, `ADMIN$`), `IPC$` and
/// printers are left out like in Explorer (pure → tested).
pub fn visible_share(name: &str, share_type: u32) -> bool {
    const STYPE_MASK: u32 = 0xff;
    const STYPE_DISKTREE: u32 = 0;
    const STYPE_SPECIAL: u32 = 0x8000_0000;
    share_type & STYPE_MASK == STYPE_DISKTREE
        && share_type & STYPE_SPECIAL == 0
        && !name.ends_with('$')
}

/// Why a listing failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetError {
    /// The server wants credentials (access denied / logon failure).
    LoginRequired(String),
    Other(String),
}

impl NetError {
    pub fn message(&self) -> &str {
        match self {
            NetError::LoginRequired(m) | NetError::Other(m) => m,
        }
    }
}

/// Windows error codes that a login can fix.
pub fn is_login_error(code: u32) -> bool {
    // ACCESS_DENIED, INVALID_PASSWORD, NOT_AUTHENTICATED, LOGON_FAILURE,
    // ACCOUNT_RESTRICTION, NO_SUCH_LOGON_SESSION (stale session)
    matches!(code, 5 | 86 | 1244 | 1326 | 1327 | 1312)
}

/// The login dialog exists (Windows only).
pub const LOGIN_SUPPORTED: bool = cfg!(windows);

/// Shares of `server` (blocking; run on a loader thread).
pub fn shares(server: &str) -> Result<Vec<Share>, NetError> {
    #[cfg(windows)]
    {
        win::shares(server)
    }
    #[cfg(not(windows))]
    {
        let _ = server;
        Err(NetError::Other(
            "Netzwerkfreigaben gibt es nur unter Windows".into(),
        ))
    }
}

/// Connects to `\\server\share` (or `\\server\IPC$` for the server itself)
/// with the Windows credential dialog, owned by window `hwnd` (0 = none).
/// `Ok(())` when connected, `Err` with a message when cancelled or failed.
/// Blocks until the dialog is closed → never on the UI thread.
pub fn connect(server: &str, share: Option<&str>, hwnd: isize) -> Result<(), String> {
    #[cfg(windows)]
    {
        win::connect(server, share, hwnd)
    }
    #[cfg(not(windows))]
    {
        let _ = (server, share, hwnd);
        Err("Anmelden gibt es nur unter Windows".into())
    }
}

#[cfg(windows)]
mod win {
    use super::{is_login_error, visible_share, NetError, Share};
    use windows_sys::Win32::NetworkManagement::NetManagement::NetApiBufferFree;
    use windows_sys::Win32::NetworkManagement::WNet::{
        WNetAddConnection3W, CONNECT_INTERACTIVE, CONNECT_PROMPT, NETRESOURCEW, RESOURCETYPE_ANY,
    };
    use windows_sys::Win32::Storage::FileSystem::{NetShareEnum, SHARE_INFO_1};

    const NERR_SUCCESS: u32 = 0;
    const ERROR_MORE_DATA: u32 = 234;
    const ERROR_CANCELLED: u32 = 1223;
    const MAX_PREFERRED_LENGTH: u32 = u32::MAX;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// SAFETY: `p` is NULL or a NUL-terminated UTF-16 string.
    unsafe fn from_wide(p: *const u16) -> String {
        if p.is_null() {
            return String::new();
        }
        let mut n = 0;
        while unsafe { *p.add(n) } != 0 {
            n += 1;
        }
        String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(p, n) })
    }

    pub fn message(code: u32) -> String {
        match code {
            53 | 67 | 2250 => "Netzwerkpfad nicht gefunden".into(),
            c if is_login_error(c) => "Anmeldung erforderlich".into(),
            c => std::io::Error::from_raw_os_error(c as i32).to_string(),
        }
    }

    pub fn shares(server: &str) -> Result<Vec<Share>, NetError> {
        let name = wide(&format!(r"\\{server}"));
        let mut out = Vec::new();
        let mut resume = 0u32;
        loop {
            let mut buf: *mut u8 = std::ptr::null_mut();
            let (mut read, mut total) = (0u32, 0u32);
            // SAFETY: valid NUL-terminated server name and out pointers; the
            // buffer is freed with NetApiBufferFree below.
            let rc = unsafe {
                NetShareEnum(
                    name.as_ptr(),
                    1,
                    &mut buf,
                    MAX_PREFERRED_LENGTH,
                    &mut read,
                    &mut total,
                    &mut resume,
                )
            };
            if rc != NERR_SUCCESS && rc != ERROR_MORE_DATA {
                if !buf.is_null() {
                    unsafe { NetApiBufferFree(buf as _) };
                }
                return Err(if is_login_error(rc) {
                    NetError::LoginRequired(message(rc))
                } else {
                    NetError::Other(message(rc))
                });
            }
            if !buf.is_null() {
                // SAFETY: on success `buf` holds `read` SHARE_INFO_1 records.
                let items = unsafe {
                    std::slice::from_raw_parts(buf as *const SHARE_INFO_1, read as usize)
                };
                for it in items {
                    let name = unsafe { from_wide(it.shi1_netname) };
                    if visible_share(&name, it.shi1_type) {
                        out.push(Share {
                            name,
                            remark: unsafe { from_wide(it.shi1_remark) },
                        });
                    }
                }
                unsafe { NetApiBufferFree(buf as _) };
            }
            if rc != ERROR_MORE_DATA {
                break;
            }
        }
        Ok(out)
    }

    pub fn connect(server: &str, share: Option<&str>, hwnd: isize) -> Result<(), String> {
        let mut remote = wide(&format!(r"\\{server}\{}", share.unwrap_or("IPC$")));
        // SAFETY: zeroed NETRESOURCEW is valid; all pointers we set live until the call returns.
        let mut res: NETRESOURCEW = unsafe { std::mem::zeroed() };
        res.dwType = RESOURCETYPE_ANY;
        res.lpRemoteName = remote.as_mut_ptr();
        let rc = unsafe {
            WNetAddConnection3W(
                hwnd as _,
                &res,
                std::ptr::null(),
                std::ptr::null(),
                CONNECT_INTERACTIVE | CONNECT_PROMPT,
            )
        };
        match rc {
            0 => Ok(()),
            ERROR_CANCELLED => Err("Anmeldung abgebrochen".into()),
            c => Err(message(c)),
        }
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn localhost_lists_disk_shares_without_admin_shares() {
            // every Windows machine has the hidden C$ share; it must not show up
            match super::shares("localhost") {
                Ok(list) => assert!(list.iter().all(|s| !s.name.ends_with('$')), "{list:?}"),
                // the Server service may be off on some machines
                Err(e) => eprintln!("NetShareEnum localhost: {}", e.message()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_roots() {
        assert_eq!(server_root(r"\\nas"), Some("nas".into()));
        assert_eq!(server_root(r"\\nas\"), Some("nas".into()));
        assert_eq!(server_root(r" \\192.168.0.2 "), Some("192.168.0.2".into()));
        assert_eq!(server_root(r"\\?\UNC\nas"), Some("nas".into()));
        assert_eq!(server_root(r"\\nas\film"), None);
        assert_eq!(server_root(r"C:\x"), None);
        assert_eq!(server_root(r"\\?\C:\x"), None);
        assert_eq!(server_root(r"\\.\pipe"), None);
        assert_eq!(server_root(r"\\"), None);
        assert_eq!(server_root("/home"), None);
    }

    #[test]
    fn shares_and_parents() {
        assert_eq!(
            share_of(r"\\nas\film\2024"),
            Some(("nas".into(), "film".into()))
        );
        assert_eq!(share_of(r"\\nas"), None);
        assert_eq!(share_root_parent(r"\\nas\film"), Some(r"\\nas".into()));
        assert_eq!(share_root_parent(r"\\nas\film\"), Some(r"\\nas".into()));
        assert_eq!(share_root_parent(r"\\nas\film\2024"), None);
        assert_eq!(share_root_parent(r"\\nas"), None);
    }

    #[test]
    fn only_visible_disk_shares() {
        assert!(visible_share("film", 0));
        assert!(!visible_share("C$", 0x8000_0000));
        assert!(!visible_share("ADMIN$", 0x8000_0000));
        assert!(!visible_share("IPC$", 0x8000_0003));
        assert!(!visible_share("drucker", 1), "printer");
        assert!(!visible_share("geheim$", 0), "hidden by name");
    }

    #[test]
    fn login_errors() {
        assert!(is_login_error(5));
        assert!(is_login_error(1326));
        assert!(!is_login_error(53));
    }
}
