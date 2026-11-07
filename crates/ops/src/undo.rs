//! Undo/redo for file jobs. A finished job's `Report` becomes an undo entry;
//! undo reverts its effects in reverse order:
//! - created items (copies, new folders) → moved to the trash (recoverable),
//! - moves/renames → moved back,
//! - trashed items → restored from the trash,
//! - permanent deletes → cannot be undone (entry is marked).

use crate::exec::{Done, JobSpec, Report};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revert {
    TrashCreated(PathBuf),
    MoveBack {
        from: PathBuf,
        to: PathBuf,
    },
    RestoreTrashed {
        original: PathBuf,
        since: SystemTime,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoEntry {
    /// German label: "Kopieren rückgängig machen" etc.
    pub label: String,
    pub reverts: Vec<Revert>,
    /// Some permanently deleted items cannot come back.
    pub incomplete: bool,
    /// What redo runs again (None for renames/new folders handled by `redo_rename`).
    pub redo: Option<Redo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Redo {
    Job(JobSpec),
    Rename { from: PathBuf, to: PathBuf },
    NewFolder(PathBuf),
}

pub fn entry_from_report(label: &str, report: &Report, redo: Option<Redo>) -> Option<UndoEntry> {
    let mut reverts = Vec::new();
    let mut incomplete = false;
    for d in report.done.iter().rev() {
        match d {
            Done::Created(p) => reverts.push(Revert::TrashCreated(p.clone())),
            Done::Moved { from, to } => reverts.push(Revert::MoveBack {
                from: to.clone(),
                to: from.clone(),
            }),
            Done::Trashed { original, at } => reverts.push(Revert::RestoreTrashed {
                original: original.clone(),
                since: *at,
            }),
            Done::Deleted(_) => incomplete = true,
        }
    }
    if reverts.is_empty() {
        return None;
    }
    Some(UndoEntry {
        label: label.to_string(),
        reverts,
        incomplete,
        redo,
    })
}

/// Trash items as they were *before* an undo started. Restores only pick from
/// this snapshot: an undo that first trashes a replacement (same path) must not
/// "restore" that replacement instead of the original.
pub struct TrashSnapshot(Result<Vec<trash::TrashItem>, String>);

impl TrashSnapshot {
    pub fn take() -> Self {
        TrashSnapshot(
            trash::os_limited::list().map_err(|e| format!("Papierkorb nicht lesbar: {e}")),
        )
    }

    /// Restores `original`: newest matching item deleted at/after `since`.
    pub fn restore(&mut self, original: &Path, since: SystemTime) -> Result<(), String> {
        let items = self.0.as_mut().map_err(|e| e.clone())?;
        let since_secs = since
            .checked_sub(Duration::from_secs(2)) // trash timestamps have second precision
            .unwrap_or(since)
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let want_parent = original.parent().map(canonical).unwrap_or_default();
        let idx = items
            .iter()
            .enumerate()
            .filter(|(_, i)| {
                i.time_deleted >= since_secs
                    && same_location(&i.original_parent, &i.name, &i.id, original, &want_parent)
            })
            .max_by_key(|(_, i)| i.time_deleted)
            .map(|(n, _)| n)
            .ok_or_else(|| format!("Im Papierkorb nicht gefunden: {}", original.display()))?;
        if original.exists() {
            return Err(format!(
                "Am Originalort existiert bereits ein Element: {}",
                original.display()
            ));
        }
        let mut item = items.swap_remove(idx);
        // Restore under the real name (Windows reports a display name that may
        // lack the extension, and the restore would use it as the target name).
        if let Some(n) = original.file_name() {
            item.name = n.to_os_string();
        }
        trash::os_limited::restore_all([item])
            .map_err(|e| format!("Wiederherstellen fehlgeschlagen: {e}"))
    }
}

/// Canonical form of an existing folder (resolves 8.3 short names like
/// `RUNNER~1`, symlinks, `.`/`..`); falls back to the path itself.
fn canonical(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

fn eq_name(a: &str, b: &str, windows: bool) -> bool {
    if windows {
        a.to_lowercase() == b.to_lowercase()
    } else {
        a == b
    }
}

/// Does the trash item's name denote `original_name`? On Windows the recycle bin
/// reports the *display* name, which hides known extensions by default
/// ("weg" for "weg.txt"); its id (`…\$Recycle.Bin\…\$R1A2B3C.txt`) keeps the
/// extension, so stem + id extension identify the file (pure → tested).
pub fn trash_name_matches(
    item_name: &str,
    item_id: &str,
    original_name: &str,
    windows: bool,
) -> bool {
    if eq_name(item_name, original_name, windows) {
        return true;
    }
    if !windows {
        return false;
    }
    let orig = Path::new(original_name);
    let (Some(stem), Some(ext)) = (orig.file_stem(), orig.extension()) else {
        return false;
    };
    let id_ext = Path::new(item_id)
        .extension()
        .map(|e| e.to_string_lossy().into_owned());
    eq_name(item_name, &stem.to_string_lossy(), true)
        && id_ext.is_some_and(|e| eq_name(&e, &ext.to_string_lossy(), true))
}

/// Does a trash item (parent + name as recorded by the OS) denote `original`?
/// Windows records the long path while ours may contain short names, and
/// compares case-insensitively.
fn same_location(
    item_parent: &Path,
    item_name: &std::ffi::OsStr,
    item_id: &std::ffi::OsStr,
    original: &Path,
    want_parent: &Path,
) -> bool {
    if item_parent.join(item_name) == original {
        return true;
    }
    let Some(name) = original.file_name() else {
        return false;
    };
    if !trash_name_matches(
        &item_name.to_string_lossy(),
        &item_id.to_string_lossy(),
        &name.to_string_lossy(),
        cfg!(windows),
    ) {
        return false;
    }
    let have = canonical(item_parent);
    if cfg!(windows) {
        have.to_string_lossy().to_lowercase() == want_parent.to_string_lossy().to_lowercase()
    } else {
        have == want_parent
    }
}

/// Executes an undo entry. Continues after errors and reports all of them.
pub fn undo(entry: &UndoEntry) -> Vec<String> {
    let needs_trash = entry
        .reverts
        .iter()
        .any(|r| matches!(r, Revert::RestoreTrashed { .. }));
    let mut snapshot = needs_trash.then(TrashSnapshot::take);
    let mut errors = Vec::new();
    for r in &entry.reverts {
        let res = match r {
            Revert::TrashCreated(p) if !p.exists() => Ok(()),
            Revert::TrashCreated(p) => {
                trash::delete(p).map_err(|e| format!("{}: {e}", p.display()))
            }
            Revert::MoveBack { from, to } => {
                if to.exists() {
                    Err(format!(
                        "Zurückverschieben nicht möglich, Ziel existiert: {}",
                        to.display()
                    ))
                } else {
                    if let Some(parent) = to.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    std::fs::rename(from, to).map_err(|e| format!("{}: {e}", from.display()))
                }
            }
            Revert::RestoreTrashed { original, since } => match snapshot.as_mut() {
                Some(s) => s.restore(original, *since),
                None => Err("Papierkorb nicht verfügbar".into()),
            },
        };
        if let Err(e) = res {
            errors.push(e);
        }
    }
    errors
}

/// Bounded undo/redo stacks (newest last).
#[derive(Debug, Default)]
pub struct UndoStack {
    undo: Vec<UndoEntry>,
    redo: Vec<UndoEntry>,
}

pub const MAX_UNDO: usize = 50;

impl UndoStack {
    pub fn push(&mut self, e: UndoEntry) {
        self.undo.push(e);
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn pop_undo(&mut self) -> Option<UndoEntry> {
        self.undo.pop()
    }

    pub fn push_redo(&mut self, e: UndoEntry) {
        self.redo.push(e);
    }

    pub fn pop_redo(&mut self) -> Option<UndoEntry> {
        self.redo.pop()
    }

    /// After a redo the new result goes back onto the undo stack without clearing redo.
    pub fn push_after_redo(&mut self, e: UndoEntry) {
        self.undo.push(e);
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod location_tests {
    use super::*;

    #[test]
    fn matches_through_symlinked_or_dotted_parent() {
        let d = tempfile::tempdir().unwrap();
        let real = d.path().join("real");
        std::fs::create_dir(&real).unwrap();
        let dotted = real.join("..").join("real");
        let want = canonical(&dotted);
        assert!(same_location(
            &real,
            "a.txt".as_ref(),
            "".as_ref(),
            &dotted.join("a.txt"),
            &want
        ));
        assert!(!same_location(
            &real,
            "b.txt".as_ref(),
            "".as_ref(),
            &dotted.join("a.txt"),
            &want
        ));
        let other = d.path().join("other");
        std::fs::create_dir(&other).unwrap();
        assert!(!same_location(
            &other,
            "a.txt".as_ref(),
            "".as_ref(),
            &dotted.join("a.txt"),
            &want
        ));
    }

    #[test]
    fn windows_display_names_without_extension() {
        let id = r"C:\$Recycle.Bin\S-1-5-21\$RAB12CD.txt";
        assert!(trash_name_matches("weg.txt", id, "weg.txt", true));
        assert!(
            trash_name_matches("weg", id, "weg.txt", true),
            "extension hidden"
        );
        assert!(
            trash_name_matches("WEG", id, "weg.txt", true),
            "case-insensitive"
        );
        assert!(!trash_name_matches(
            "weg",
            r"C:\$Recycle.Bin\S\$RAB12CD.md",
            "weg.txt",
            true
        ));
        assert!(
            !trash_name_matches("weg", id, "weg.txt", false),
            "only on Windows"
        );
        assert!(trash_name_matches(
            "Ordner",
            r"C:\$Recycle.Bin\S\$RAB12CD",
            "Ordner",
            true
        ));
        assert!(!trash_name_matches("andere", id, "weg.txt", true));
    }
}
