//! One instance per user/config (M8.3b), pure parts: endpoint name and the
//! tiny line protocol between a second start and the running window.
//!
//! ```text
//! client → EXPLORER2/1 open C:\Users\Boss\Downloads\n   (or "EXPLORER2/1 open\n")
//! server → ok\n        (only after the window has opened the tab)
//! ```
//! No answer in time = the running instance hangs → the second start opens
//! its own window instead (nobody is ever left without a window).

pub const MAGIC: &str = "EXPLORER2/1";
/// Longest request line accepted (a path, generously).
pub const MAX_LINE: usize = 64 * 1024;
/// Client: wait this long for "ok", then start an own window.
pub const CLIENT_WAIT_MS: u64 = 4000;
/// Server: wait this long for the UI to open the tab.
pub const SERVER_WAIT_MS: u64 = 3000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Open a folder in a new tab (`None` = new tab like Ctrl+T), window to front.
    Open(Option<String>),
}

impl Request {
    pub fn encode(&self) -> String {
        match self {
            Request::Open(Some(p)) => format!("{MAGIC} open {p}\n"),
            Request::Open(None) => format!("{MAGIC} open\n"),
        }
    }

    /// `None` for anything that is not exactly our protocol.
    pub fn decode(line: &str) -> Option<Request> {
        let line = line.strip_suffix('\n').unwrap_or(line);
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.len() > MAX_LINE || line.contains(['\n', '\0']) {
            return None;
        }
        let rest = line.strip_prefix(MAGIC)?.strip_prefix(' ')?;
        if rest == "open" {
            return Some(Request::Open(None));
        }
        let path = rest.strip_prefix("open ")?;
        (!path.trim().is_empty()).then(|| Request::Open(Some(path.to_string())))
    }
}

pub const REPLY_OK: &str = "ok\n";

/// FNV-1a 64: stable across versions/platforms (std's hasher is not).
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Name part of the pipe/socket: one per config dir (tests and portable
/// copies with their own `EXPLORER2_CONFIG_DIR` never talk to each other) and
/// per Windows session / Unix user. Windows paths compare case-insensitively.
pub fn endpoint_id(config_dir: &str, windows: bool, session_or_uid: u32) -> String {
    let dir = if windows {
        config_dir.trim_end_matches(['\\', '/']).to_lowercase()
    } else {
        config_dir.trim_end_matches('/').to_string()
    };
    format!("explorer2-{session_or_uid}-{:016x}", fnv1a(dir.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for r in [
            Request::Open(None),
            Request::Open(Some(r"C:\Users\Boss\Neuer Ordner (2)".into())),
            Request::Open(Some("/home/a b/ü".into())),
        ] {
            assert_eq!(Request::decode(&r.encode()), Some(r.clone()));
            assert_eq!(Request::decode(&r.encode().replace('\n', "\r\n")), Some(r));
        }
    }

    #[test]
    fn rejects_foreign_input() {
        for bad in [
            "",
            "open C:\\",
            "EXPLORER2/2 open C:\\",
            "EXPLORER2/1",
            "EXPLORER2/1 delete C:\\",
            "EXPLORER2/1 open ",
            "EXPLORER2/1 open C:\\\nEXPLORER2/1 open D:\\",
            "EXPLORER2/1 open C:\\\0x",
            "GET / HTTP/1.1",
        ] {
            assert_eq!(Request::decode(bad), None, "{bad:?}");
        }
        let long = format!("{MAGIC} open {}", "a".repeat(MAX_LINE));
        assert_eq!(Request::decode(&long), None);
    }

    #[test]
    fn endpoint_ids() {
        let a = endpoint_id(r"C:\Users\Boss\AppData\Roaming\Explorer2", true, 1);
        assert_eq!(
            a,
            endpoint_id(r"c:\users\boss\appdata\roaming\explorer2\", true, 1)
        );
        assert_ne!(
            a,
            endpoint_id(r"C:\Users\Boss\AppData\Roaming\Explorer2", true, 2)
        );
        assert_ne!(a, endpoint_id(r"C:\test\cfg", true, 1));
        // Unix paths are case-sensitive
        assert_ne!(
            endpoint_id("/home/a/.config/explorer2", false, 1000),
            endpoint_id("/home/a/.config/Explorer2", false, 1000)
        );
        assert!(a.starts_with("explorer2-1-") && a.len() == "explorer2-1-".len() + 16);
        // stable value (FNV-1a) – the name must not change between versions
        assert_eq!(
            endpoint_id("/x", false, 0),
            format!("explorer2-0-{:016x}", fnv1a(b"/x"))
        );
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
    }
}
