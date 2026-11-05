//! R-001 "Notausgang": every Windows integration goes through here.
//!
//! [`apply`] records the state before each change in the journal *first*
//! (write-ahead), then changes the registry/file. [`reset`] plays the journal
//! backwards – only where the value is still ours (a value another program
//! changed since is left alone and reported) – then sweeps the known
//! integration points ([`manifest`]) for anything recognisably ours, which
//! also covers a lost or damaged journal. The same code runs in the app
//! (settings button) and in the standalone `explorer2-recover` tool.

pub mod features;
pub mod install;
pub mod journal;
pub mod manifest;
pub mod model;
pub mod registry;
#[cfg(windows)]
mod win;

pub use journal::{load, save, Entry, Journal, JournalPaths, Loaded};
pub use model::{Hive, KeyPath, RegData};
pub use registry::{MemRegistry, Registry};
#[cfg(windows)]
pub use win::WinRegistry;

use journal::{fnv64, write_atomic};
use manifest::Point;
use std::fs;
use std::path::PathBuf;

/// One change an integration wants to make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    SetValue {
        key: KeyPath,
        name: String,
        data: RegData,
    },
    WriteFile {
        path: PathBuf,
        bytes: Vec<u8>,
    },
}

/// The real registry of this machine (`None` off Windows).
pub fn system_registry() -> Option<Box<dyn Registry>> {
    #[cfg(windows)]
    return Some(Box::new(WinRegistry));
    #[cfg(not(windows))]
    None
}

fn backup_name(path: &std::path::Path) -> String {
    format!("{:016x}.bak", fnv64(path.to_string_lossy().as_bytes()))
}

/// Apply `changes`, journaling each one before it happens. Refuses to work on
/// top of a damaged journal (the originals could get lost) – reset first.
pub fn apply<R: Registry + ?Sized>(
    reg: &mut R,
    paths: &JournalPaths,
    changes: &[Change],
) -> Result<usize, String> {
    let mut j = match load(paths) {
        Loaded::None => Journal::default(),
        Loaded::Ok(j) => j,
        Loaded::Damaged(_) => {
            return Err("Integrations-Journal beschädigt – bitte zuerst zurücksetzen".into())
        }
    };
    let save_j = |j: &Journal| save(paths, j).map_err(|e| format!("Journal schreiben: {e}"));
    let mut done = 0;
    let mut env = false;
    for c in changes {
        match c {
            Change::SetValue { key, name, data } => {
                for k in key.lineage() {
                    let exists = reg.key_exists(&k).map_err(|e| format!("{k}: {e}"))?;
                    if !exists && !j.has_key(&k) {
                        j.entries.push(Entry::Key { key: k });
                    }
                }
                let prior = reg.get(key, name).map_err(|e| format!("{key}: {e}"))?;
                match j.find_value(key, name) {
                    // already ours: the original stays the one from the first change
                    Some(i) => {
                        if let Entry::Value { ours, .. } = &mut j.entries[i] {
                            *ours = data.clone();
                        }
                    }
                    None => j.entries.push(Entry::Value {
                        key: key.clone(),
                        name: name.clone(),
                        prior,
                        ours: data.clone(),
                    }),
                }
                save_j(&j)?;
                reg.set(key, name, data)
                    .map_err(|e| format!("{key}\\{name}: {e}"))?;
                env |= key.is_environment();
            }
            Change::WriteFile { path, bytes } => {
                if j.find_file(path).is_none() {
                    let backup = if path.exists() {
                        let name = backup_name(path);
                        let old = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
                        write_atomic(&paths.backups.join(&name), &old)
                            .map_err(|e| format!("Sicherung: {e}"))?;
                        Some(name)
                    } else {
                        None
                    };
                    j.entries.push(Entry::File {
                        path: path.clone(),
                        backup,
                        ours: fnv64(bytes),
                    });
                } else if let Some(Entry::File { ours, .. }) =
                    j.find_file(path).map(|i| &mut j.entries[i])
                {
                    *ours = fnv64(bytes);
                }
                save_j(&j)?;
                write_atomic(path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
            }
        }
        done += 1;
    }
    if env {
        reg.notify_environment();
    }
    Ok(done)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalState {
    None,
    Ok,
    Damaged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResetReport {
    pub journal: JournalState,
    /// Values/files put back to their original state.
    pub restored: usize,
    pub keys_removed: usize,
    /// Removed by the sweep of the known points.
    pub swept: usize,
    /// Left alone on purpose (changed by someone else since / not empty).
    pub kept: Vec<String>,
    /// Could not be undone (e.g. access denied) – stay in the journal.
    pub errors: Vec<String>,
}

impl ResetReport {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn changed(&self) -> usize {
        self.restored + self.keys_removed + self.swept
    }

    pub fn summary_de(&self) -> String {
        let mut s = if self.changed() == 0 && self.errors.is_empty() {
            "Keine Windows-Integration aktiv – nichts zu tun.".to_string()
        } else {
            format!(
                "Windows-Standard wiederhergestellt: {} Werte/Dateien, {} Schlüssel entfernt{}.",
                self.restored + self.swept,
                self.keys_removed,
                if self.journal == JournalState::Damaged {
                    " (Journal war beschädigt, bekannte Stellen geprüft)"
                } else {
                    ""
                }
            )
        };
        if !self.kept.is_empty() {
            s.push_str(&format!(
                " {} von anderer Software geändert – belassen.",
                self.kept.len()
            ));
        }
        if !self.errors.is_empty() {
            s.push_str(&format!(
                " {} Fehler: {}",
                self.errors.len(),
                self.errors.join("; ")
            ));
        }
        s
    }
}

/// Put everything back. Idempotent; safe without a journal.
pub fn reset<R: Registry + ?Sized>(reg: &mut R, paths: &JournalPaths) -> ResetReport {
    reset_with(reg, paths, &manifest::points())
}

/// [`reset`] with an explicit list of known points (tests on a real machine
/// pass none, so they never touch a real integration of the developer).
pub fn reset_with<R: Registry + ?Sized>(
    reg: &mut R,
    paths: &JournalPaths,
    points: &[Point],
) -> ResetReport {
    revert(reg, paths, None, points)
}

/// Undo only what lies below `roots` (one integration feature); the rest
/// stays in the journal. Keys above the roots that we created go when empty.
pub fn revert_within<R: Registry + ?Sized>(
    reg: &mut R,
    paths: &JournalPaths,
    scopes: &[Scope],
) -> ResetReport {
    revert(reg, paths, Some(scopes), &[])
}

/// What one feature owns: a whole key tree, or only the values of one key
/// (e.g. the default verb in `Directory\shell`, a key shared with others).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub key: KeyPath,
    pub deep: bool,
}

impl Scope {
    pub fn tree(key: KeyPath) -> Scope {
        Scope { key, deep: true }
    }

    pub fn values_of(key: KeyPath) -> Scope {
        Scope { key, deep: false }
    }

    fn has_value_key(&self, k: &KeyPath) -> bool {
        if self.deep {
            k.is_within(&self.key)
        } else {
            k.norm() == self.key.norm()
        }
    }
}

fn revert<R: Registry + ?Sized>(
    reg: &mut R,
    paths: &JournalPaths,
    scope: Option<&[Scope]>,
    points: &[Point],
) -> ResetReport {
    let in_scope = |k: &KeyPath| scope.is_none_or(|ss| ss.iter().any(|s| s.has_value_key(k)));
    let key_in_scope =
        |k: &KeyPath| scope.is_none_or(|ss| ss.iter().any(|s| s.deep && k.is_within(&s.key)));
    // created keys above (or holding) a scope, e.g. `Directory\shell`:
    // removed only when empty
    let above_scope = |k: &KeyPath| {
        scope.is_some_and(|ss| {
            ss.iter()
                .any(|s| s.key.is_within(k) && !(s.deep && s.key.norm() == k.norm()))
        })
    };
    let (state, j) = match load(paths) {
        Loaded::None => (JournalState::None, Journal::default()),
        Loaded::Ok(j) => (JournalState::Ok, j),
        Loaded::Damaged(j) => (JournalState::Damaged, j),
    };
    let mut r = ResetReport {
        journal: state,
        restored: 0,
        keys_removed: 0,
        swept: 0,
        kept: Vec::new(),
        errors: Vec::new(),
    };
    let mut unresolved = Vec::new();
    let mut env = false;
    // values and files, newest first
    for e in j.entries.iter().rev() {
        match e {
            Entry::Key { .. } => {}
            Entry::Value { key, .. } if !in_scope(key) => unresolved.push(e.clone()),
            Entry::File { .. } if scope.is_some() => unresolved.push(e.clone()),
            Entry::Value {
                key,
                name,
                prior,
                ours,
            } => {
                let label = format!(
                    "{key}\\{}",
                    if name.is_empty() { "(Standard)" } else { name }
                );
                let cur = match reg.get(key, name) {
                    Ok(c) => c,
                    Err(err) => {
                        r.errors.push(format!("{label}: {err}"));
                        unresolved.push(e.clone());
                        continue;
                    }
                };
                if cur.as_ref() == Some(ours) {
                    let res = match prior {
                        Some(p) => reg.set(key, name, p),
                        None => reg.delete_value(key, name),
                    };
                    match res {
                        Ok(()) => {
                            r.restored += 1;
                            env |= key.is_environment();
                        }
                        Err(err) => {
                            r.errors.push(format!("{label}: {err}"));
                            unresolved.push(e.clone());
                        }
                    }
                } else if cur != *prior {
                    r.kept.push(label);
                }
            }
            Entry::File { path, backup, ours } => {
                let label = path.display().to_string();
                let cur = fs::read(path).ok();
                if cur.as_deref().map(fnv64) == Some(*ours) {
                    let res = match backup {
                        Some(b) => {
                            fs::read(paths.backups.join(b)).and_then(|old| write_atomic(path, &old))
                        }
                        None => fs::remove_file(path),
                    };
                    match res {
                        Ok(()) => r.restored += 1,
                        Err(err) => {
                            r.errors.push(format!("{label}: {err}"));
                            unresolved.push(e.clone());
                        }
                    }
                } else {
                    // not ours any more: fine if it is the original again,
                    // otherwise someone else changed it → leave it
                    let original = backup
                        .as_ref()
                        .and_then(|b| fs::read(paths.backups.join(b)).ok())
                        .map(|b| fnv64(&b));
                    if cur.as_deref().map(fnv64) != original {
                        r.kept.push(label);
                    }
                }
            }
        }
    }
    // keys we created, deepest first – only when empty
    for e in j.entries.iter().rev() {
        if let Entry::Key { key } = e {
            let above = above_scope(key) && !key_in_scope(key);
            if !key_in_scope(key) && !above {
                unresolved.push(e.clone());
                continue;
            }
            match (reg.key_exists(key), reg.key_is_empty(key)) {
                (Ok(false), _) => {}
                (Ok(true), Ok(true)) => match reg.delete_empty_key(key) {
                    Ok(()) => r.keys_removed += 1,
                    Err(err) => {
                        r.errors.push(format!("{key}: {err}"));
                        unresolved.push(e.clone());
                    }
                },
                // still holds something we failed to undo (or another feature
                // of ours) → stays in the journal
                (Ok(true), Ok(false))
                    if above || unresolved.iter().any(|u| entry_within(u, key)) =>
                {
                    unresolved.push(e.clone())
                }
                (Ok(true), Ok(false)) => r.kept.push(format!("{key} (nicht leer)")),
                (Err(err), _) | (_, Err(err)) => {
                    r.errors.push(format!("{key}: {err}"));
                    unresolved.push(e.clone());
                }
            }
        }
    }
    // belt and braces: anything still recognisably ours at the known points
    let (n, mut errs) = sweep(reg, points, false);
    r.swept = n;
    r.errors.append(&mut errs);
    if env {
        reg.notify_environment();
    }
    unresolved.reverse();
    if let Err(e) = save(
        paths,
        &Journal {
            entries: unresolved,
        },
    ) {
        r.errors.push(format!("Journal schreiben: {e}"));
    }
    r
}

fn entry_within(e: &Entry, key: &KeyPath) -> bool {
    match e {
        Entry::Key { key: k } | Entry::Value { key: k, .. } => k.is_within(key),
        Entry::File { .. } => false,
    }
}

/// Remove what is ours at the known points (or only count it: `dry_run`).
pub fn sweep<R: Registry + ?Sized>(
    reg: &mut R,
    points: &[Point],
    dry_run: bool,
) -> (usize, Vec<String>) {
    let mut n = 0;
    let mut errors = Vec::new();
    for p in points.iter().cloned() {
        match p {
            Point::Value {
                key,
                probe,
                also,
                stop,
            } => {
                let ours = matches!(reg.get(&key, probe), Ok(Some(d))
                    if d.text().is_some_and(|t| manifest::is_ours(&t)));
                if !ours {
                    continue;
                }
                n += 1;
                if dry_run {
                    continue;
                }
                for name in std::iter::once(probe).chain(also.iter().copied()) {
                    if let Err(e) = reg.delete_value(&key, name) {
                        errors.push(format!("{key}\\{name}: {e}"));
                    }
                }
                let mut k = Some(key);
                while let Some(cur) = k {
                    if cur.norm() == stop.norm() || !cur.is_within(&stop) {
                        break;
                    }
                    if !matches!(reg.key_exists(&cur), Ok(true))
                        || !matches!(reg.key_is_empty(&cur), Ok(true))
                    {
                        break;
                    }
                    if let Err(e) = reg.delete_empty_key(&cur) {
                        errors.push(format!("{cur}: {e}"));
                        break;
                    }
                    k = cur.parent();
                }
            }
            Point::OwnVerb { key, stop } => {
                if !matches!(reg.key_exists(&key), Ok(true)) {
                    continue;
                }
                n += 1;
                if dry_run {
                    continue;
                }
                if let Err(e) = reg.delete_tree(&key) {
                    errors.push(format!("{key}: {e}"));
                    continue;
                }
                let mut k = key.parent();
                while let Some(cur) = k {
                    if cur.norm() == stop.norm() || !cur.is_within(&stop) {
                        break;
                    }
                    if !matches!(reg.key_exists(&cur), Ok(true))
                        || !matches!(reg.key_is_empty(&cur), Ok(true))
                    {
                        break;
                    }
                    if let Err(e) = reg.delete_empty_key(&cur) {
                        errors.push(format!("{cur}: {e}"));
                        break;
                    }
                    k = cur.parent();
                }
            }
            Point::OwnTree(key) => {
                if !matches!(reg.key_exists(&key), Ok(true)) {
                    continue;
                }
                n += 1;
                if !dry_run {
                    if let Err(e) = reg.delete_tree(&key) {
                        errors.push(format!("{key}: {e}"));
                    }
                }
            }
        }
    }
    (n, errors)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    pub journal: JournalState,
    /// Changes recorded in the journal.
    pub entries: usize,
    /// Known points that are currently ours (also without a journal).
    pub active_points: usize,
}

impl Status {
    pub fn any(&self) -> bool {
        self.entries > 0 || self.active_points > 0 || self.journal == JournalState::Damaged
    }

    pub fn summary_de(&self) -> String {
        if !self.any() {
            return "Keine Windows-Integration aktiv.".into();
        }
        let mut s = format!(
            "{} Änderung{} im Journal",
            self.entries,
            if self.entries == 1 { "" } else { "en" }
        );
        if self.active_points > 0 {
            s.push_str(&format!(
                ", {} Integrationsstelle(n) aktiv",
                self.active_points
            ));
        }
        if self.journal == JournalState::Damaged {
            s.push_str(" – Journal beschädigt, Zurücksetzen empfohlen");
        }
        s.push('.');
        s
    }
}

pub fn status<R: Registry + ?Sized>(reg: &mut R, paths: &JournalPaths) -> Status {
    let (journal, entries) = match load(paths) {
        Loaded::None => (JournalState::None, 0),
        Loaded::Ok(j) => (JournalState::Ok, j.entries.len()),
        Loaded::Damaged(j) => (JournalState::Damaged, j.entries.len()),
    };
    Status {
        journal,
        entries,
        active_points: sweep(reg, &manifest::points(), true).0,
    }
}

#[cfg(test)]
mod tests;
