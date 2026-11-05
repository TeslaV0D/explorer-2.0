//! Registry addresses and raw values.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Hive {
    CurrentUser,
    LocalMachine,
}

impl Hive {
    pub fn as_str(self) -> &'static str {
        match self {
            Hive::CurrentUser => "HKCU",
            Hive::LocalMachine => "HKLM",
        }
    }

    pub fn parse(s: &str) -> Option<Hive> {
        match s {
            "HKCU" => Some(Hive::CurrentUser),
            "HKLM" => Some(Hive::LocalMachine),
            _ => None,
        }
    }
}

/// A registry key: hive + path below it (`Software\Classes\…`, no leading
/// or trailing backslash). Windows compares keys case-insensitively → use
/// [`KeyPath::norm`] for lookups.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyPath {
    pub hive: Hive,
    pub path: String,
}

impl KeyPath {
    pub fn new(hive: Hive, path: &str) -> KeyPath {
        let path = path
            .split('\\')
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join("\\");
        KeyPath { hive, path }
    }

    pub fn hkcu(path: &str) -> KeyPath {
        KeyPath::new(Hive::CurrentUser, path)
    }

    pub fn hklm(path: &str) -> KeyPath {
        KeyPath::new(Hive::LocalMachine, path)
    }

    /// Case-insensitive identity.
    pub fn norm(&self) -> String {
        format!("{}\\{}", self.hive.as_str(), self.path.to_lowercase())
    }

    pub fn parent(&self) -> Option<KeyPath> {
        let (p, _) = self.path.rsplit_once('\\')?;
        Some(KeyPath::new(self.hive, p))
    }

    pub fn child(&self, name: &str) -> KeyPath {
        KeyPath::new(self.hive, &format!("{}\\{name}", self.path))
    }

    /// `A`, `A\B`, `A\B\C` for `A\B\C` (top-down).
    pub fn lineage(&self) -> Vec<KeyPath> {
        let parts: Vec<&str> = self.path.split('\\').collect();
        (1..=parts.len())
            .map(|n| KeyPath::new(self.hive, &parts[..n].join("\\")))
            .collect()
    }

    /// `self` is `other` or below it.
    pub fn is_within(&self, other: &KeyPath) -> bool {
        let (a, b) = (self.norm(), other.norm());
        a == b || a.starts_with(&format!("{b}\\"))
    }

    pub fn is_environment(&self) -> bool {
        self.hive == Hive::CurrentUser && self.path.eq_ignore_ascii_case("Environment")
    }
}

impl fmt::Display for KeyPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\\{}", self.hive.as_str(), self.path)
    }
}

pub const REG_SZ: u32 = 1;
pub const REG_EXPAND_SZ: u32 = 2;
pub const REG_BINARY: u32 = 3;
pub const REG_DWORD: u32 = 4;
pub const REG_MULTI_SZ: u32 = 7;

/// A registry value exactly as stored: type + raw bytes (strings are UTF-16LE
/// incl. the terminating NUL). Kept raw so a restore is bit-exact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegData {
    pub kind: u32,
    pub bytes: Vec<u8>,
}

impl RegData {
    pub fn sz(s: &str) -> RegData {
        RegData::string(REG_SZ, s)
    }

    pub fn expand_sz(s: &str) -> RegData {
        RegData::string(REG_EXPAND_SZ, s)
    }

    fn string(kind: u32, s: &str) -> RegData {
        let bytes = s
            .encode_utf16()
            .chain(std::iter::once(0))
            .flat_map(|u| u.to_le_bytes())
            .collect();
        RegData { kind, bytes }
    }

    pub fn dword(v: u32) -> RegData {
        RegData {
            kind: REG_DWORD,
            bytes: v.to_le_bytes().to_vec(),
        }
    }

    /// Text of string values (NULs → spaces for multi-strings), else `None`.
    pub fn text(&self) -> Option<String> {
        if !matches!(self.kind, REG_SZ | REG_EXPAND_SZ | REG_MULTI_SZ) {
            return None;
        }
        let units: Vec<u16> = self
            .bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect();
        let s = String::from_utf16_lossy(&units);
        Some(s.trim_end_matches('\0').replace('\0', " "))
    }
}
