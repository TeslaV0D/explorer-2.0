//! The integration journal: for every change we make, the state before it.
//!
//! Written *before* the change (write-ahead), atomically (temp + flush +
//! rename), twice (primary + mirror on another path), with a checksum line.
//! A damaged primary falls back to the mirror; if both are damaged, the
//! readable entries are salvaged and the reset additionally sweeps the
//! known integration points (`manifest`).
//!
//! Text format (all names/data hex-encoded, so no escaping problems):
//! ```text
//! # Explorer 2.0 – integration journal (R-001), do not edit
//! v1
//! key   HKCU <path>                           (created by us)
//! value HKCU <path> <name> <prior|-> <ours>    (data = <type>:<hex>)
//! file  <path> <backup|-> <fnv64 of our bytes>
//! end   <fnv64 of all lines above>
//! ```

use crate::model::{Hive, KeyPath, RegData};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// A key that did not exist and was created by us.
    Key { key: KeyPath },
    /// A value we set: what was there before (`None` = absent) and ours.
    Value {
        key: KeyPath,
        name: String,
        prior: Option<RegData>,
        ours: RegData,
    },
    /// A file we wrote: backup of the previous content (`None` = absent)
    /// and the hash of our content.
    File {
        path: PathBuf,
        backup: Option<String>,
        ours: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Journal {
    pub entries: Vec<Entry>,
}

/// Where the journal lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalPaths {
    pub primary: PathBuf,
    pub mirror: PathBuf,
    /// Backups of files we overwrote.
    pub backups: PathBuf,
}

impl JournalPaths {
    /// All in one directory (tests, portable mode).
    pub fn in_dir(dir: &Path) -> JournalPaths {
        JournalPaths {
            primary: dir.join("integration-journal.txt"),
            mirror: dir.join("integration-journal.mirror.txt"),
            backups: dir.join("integration-backup"),
        }
    }

    /// `EXPLORER2_CONFIG_DIR` > `%APPDATA%\Explorer2` (mirror in
    /// `%LOCALAPPDATA%\Explorer2`) > XDG config – same directory as the app's
    /// settings.ini.
    pub fn default_paths() -> Option<JournalPaths> {
        if let Some(d) = std::env::var_os("EXPLORER2_CONFIG_DIR") {
            return Some(JournalPaths::in_dir(Path::new(&d)));
        }
        #[cfg(windows)]
        {
            let app = PathBuf::from(std::env::var_os("APPDATA")?).join("Explorer2");
            let mut p = JournalPaths::in_dir(&app);
            if let Some(local) = std::env::var_os("LOCALAPPDATA") {
                p.mirror = PathBuf::from(local)
                    .join("Explorer2")
                    .join("integration-journal.mirror.txt");
            }
            Some(p)
        }
        #[cfg(not(windows))]
        {
            let dir = std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?
                .join("explorer2");
            Some(JournalPaths::in_dir(&dir))
        }
    }
}

/// Result of reading the journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Loaded {
    /// No journal: nothing was changed (or everything was reset).
    None,
    Ok(Journal),
    /// Primary and mirror damaged: what could be read.
    Damaged(Journal),
}

pub fn fnv64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn hex(b: &[u8]) -> String {
    if b.is_empty() {
        return "~".into(); // empty field marker (split keeps it visible)
    }
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if s == "~" {
        return Some(Vec::new());
    }
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

fn text(s: &str) -> Option<String> {
    String::from_utf8(unhex(s)?).ok()
}

fn data(d: &Option<RegData>) -> String {
    match d {
        None => "-".into(),
        Some(d) => format!("{}:{}", d.kind, hex(&d.bytes)),
    }
}

fn undata(s: &str) -> Option<Option<RegData>> {
    if s == "-" {
        return Some(None);
    }
    let (k, b) = s.split_once(':')?;
    Some(Some(RegData {
        kind: k.parse().ok()?,
        bytes: unhex(b)?,
    }))
}

const HEADER: &str = "# Explorer 2.0 - integration journal (R-001), do not edit\nv1\n";

impl Journal {
    pub fn serialize(&self) -> String {
        let mut s = String::from(HEADER);
        for e in &self.entries {
            let line = match e {
                Entry::Key { key } => {
                    format!("key\t{}\t{}", key.hive.as_str(), hex(key.path.as_bytes()))
                }
                Entry::Value {
                    key,
                    name,
                    prior,
                    ours,
                } => format!(
                    "value\t{}\t{}\t{}\t{}\t{}",
                    key.hive.as_str(),
                    hex(key.path.as_bytes()),
                    hex(name.as_bytes()),
                    data(prior),
                    data(&Some(ours.clone()))
                ),
                Entry::File { path, backup, ours } => format!(
                    "file\t{}\t{}\t{ours:016x}",
                    hex(path.to_string_lossy().as_bytes()),
                    backup
                        .as_deref()
                        .map_or("-".to_string(), |b| hex(b.as_bytes()))
                ),
            };
            s.push_str(&line);
            s.push('\n');
        }
        let sum = fnv64(s.as_bytes());
        s.push_str(&format!("end\t{sum:016x}\n"));
        s
    }

    fn parse_line(line: &str) -> Option<Entry> {
        let f: Vec<&str> = line.split('\t').collect();
        match f.as_slice() {
            ["key", hive, path] => Some(Entry::Key {
                key: KeyPath::new(Hive::parse(hive)?, &text(path)?),
            }),
            ["value", hive, path, name, prior, ours] => Some(Entry::Value {
                key: KeyPath::new(Hive::parse(hive)?, &text(path)?),
                name: text(name)?,
                prior: undata(prior)?,
                ours: undata(ours)??,
            }),
            ["file", path, backup, ours] => Some(Entry::File {
                path: PathBuf::from(text(path)?),
                backup: if *backup == "-" {
                    None
                } else {
                    Some(text(backup)?)
                },
                ours: u64::from_str_radix(ours, 16).ok()?,
            }),
            _ => None,
        }
    }

    /// `Ok` = intact (checksum matches), `Err` = what could be salvaged.
    pub fn parse(s: &str) -> Result<Journal, Journal> {
        let mut j = Journal::default();
        let mut intact = s.starts_with(HEADER);
        let mut ended = false;
        let mut offset = 0;
        for line in s.split_inclusive('\n') {
            let body = line.trim_end_matches(['\n', '\r']);
            if let Some(sum) = body.strip_prefix("end\t") {
                ended = u64::from_str_radix(sum, 16).ok() == Some(fnv64(&s.as_bytes()[..offset]));
                offset += line.len();
                continue;
            }
            offset += line.len();
            if body.is_empty() || body.starts_with('#') || body == "v1" {
                continue;
            }
            match Journal::parse_line(body) {
                Some(e) => j.entries.push(e),
                None => intact = false,
            }
        }
        if intact && ended {
            Ok(j)
        } else {
            Err(j)
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn find_value(&self, key: &KeyPath, name: &str) -> Option<usize> {
        self.entries.iter().position(|e| {
            matches!(e, Entry::Value { key: k, name: n, .. }
                if k.norm() == key.norm() && n.eq_ignore_ascii_case(name))
        })
    }

    pub fn has_key(&self, key: &KeyPath) -> bool {
        self.entries
            .iter()
            .any(|e| matches!(e, Entry::Key { key: k } if k.norm() == key.norm()))
    }

    pub fn find_file(&self, path: &Path) -> Option<usize> {
        self.entries
            .iter()
            .position(|e| matches!(e, Entry::File { path: p, .. } if p == path))
    }
}

/// Write `bytes` to `path` atomically: temp file next to it, flushed to the
/// disk, then renamed over the target.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp-write");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)
}

fn read(path: &Path) -> Option<String> {
    fs::read(path)
        .ok()
        .map(|b| String::from_utf8_lossy(&b).into_owned())
}

pub fn load(paths: &JournalPaths) -> Loaded {
    let (p, m) = (read(&paths.primary), read(&paths.mirror));
    if p.is_none() && m.is_none() {
        return Loaded::None;
    }
    let primary = p.map(|s| Journal::parse(&s));
    if let Some(Ok(j)) = primary {
        return Loaded::Ok(j);
    }
    let mirror = m.map(|s| Journal::parse(&s));
    if let Some(Ok(j)) = mirror {
        return Loaded::Ok(j);
    }
    // both unusable: salvage the bigger readable part
    let salvaged = [primary, mirror]
        .into_iter()
        .flatten()
        .map(|r| r.unwrap_or_else(|j| j))
        .max_by_key(|j| j.entries.len())
        .unwrap_or_default();
    Loaded::Damaged(salvaged)
}

/// Mirror first, then primary: at every moment one intact copy exists.
/// An empty journal removes both files (and the backups).
pub fn save(paths: &JournalPaths, j: &Journal) -> io::Result<()> {
    if j.is_empty() {
        for p in [&paths.primary, &paths.mirror] {
            match fs::remove_file(p) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
                _ => {}
            }
        }
        let _ = fs::remove_dir_all(&paths.backups);
        return Ok(());
    }
    let s = j.serialize();
    write_atomic(&paths.mirror, s.as_bytes())?;
    write_atomic(&paths.primary, s.as_bytes())
}
