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
        let idx = items
            .iter()
            .enumerate()
            .filter(|(_, i)| {
                i.original_parent.join(&i.name) == original && i.time_deleted >= since_secs
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
        let item = items.swap_remove(idx);
        trash::os_limited::restore_all([item])
            .map_err(|e| format!("Wiederherstellen fehlgeschlagen: {e}"))
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
