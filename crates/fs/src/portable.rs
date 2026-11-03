//! Portable enumeration via `std::fs` (non-Windows).

use crate::FsError;
use explorer_core::{Entry, EntryKind};
use std::ops::ControlFlow;
use std::path::Path;
use std::time::UNIX_EPOCH;

pub fn enumerate(dir: &Path, f: &mut dyn FnMut(Entry) -> ControlFlow<()>) -> Result<(), FsError> {
    for item in std::fs::read_dir(dir)? {
        let Ok(item) = item else { continue };
        let Ok(ft) = item.file_type() else { continue };
        let is_link = ft.is_symlink();
        // Follow links for kind/size; a dangling link is shown as a file.
        let meta = if is_link {
            std::fs::metadata(item.path()).or_else(|_| item.metadata())
        } else {
            item.metadata()
        };
        let Ok(meta) = meta else { continue };
        let name = item.file_name().to_string_lossy().into_owned();
        let is_dir = meta.is_dir();
        let entry = Entry {
            hidden: name.starts_with('.'),
            system: false,
            readonly: meta.permissions().readonly(),
            is_link,
            kind: if is_dir {
                EntryKind::Dir
            } else {
                EntryKind::File
            },
            size: if is_dir { 0 } else { meta.len() },
            modified: meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64),
            name,
        };
        if f(entry).is_break() {
            break;
        }
    }
    Ok(())
}
