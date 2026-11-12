//! The recycle bin as a folder view: list, restore, delete permanently, empty.
//! Works on the Windows recycle bin and the freedesktop trash (Linux).

use explorer_core::{Entry, EntryKind};
use std::collections::HashMap;
use std::path::PathBuf;

/// One item in the trash, ready for the list.
#[derive(Debug, Clone)]
pub struct TrashEntry {
    /// `entry.name` is unique within the listing (see `unique_names`).
    pub entry: Entry,
    /// Folder it was deleted from (shown as "Ursprünglicher Ort").
    pub original_parent: PathBuf,
    /// Real file name (with extension) to restore under.
    pub real_name: String,
    pub item: trash::TrashItem,
}

/// The real name of a trash item. Windows reports the *display* name, which
/// hides known extensions ("Bericht" for "Bericht.docx"); the `$R…` file in
/// the recycle bin keeps the extension (pure → tested).
pub fn real_name(display: &str, id: &str, windows: bool) -> String {
    if !windows {
        return display.to_string();
    }
    // file name of the id with either separator (the id is a Windows path)
    let file = id.rsplit(['\\', '/']).next().unwrap_or(id);
    let ext = match file.rfind('.') {
        Some(i) if i > 0 && i + 1 < file.len() => file[i + 1..].to_string(),
        _ => return display.to_string(),
    };
    let suffix = format!(".{}", ext.to_lowercase());
    if display.to_lowercase().ends_with(&suffix) {
        display.to_string()
    } else {
        format!("{display}.{ext}")
    }
}

/// Same name deleted twice (e.g. from two folders): list them as
/// "a.txt", "a.txt (2)" so every row stays addressable (pure → tested).
pub fn unique_names(names: &[String]) -> Vec<String> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    names
        .iter()
        .map(|n| {
            let c = seen.entry(n.to_lowercase()).or_insert(0);
            *c += 1;
            if *c == 1 {
                n.clone()
            } else {
                format!("{n} ({})", c)
            }
        })
        .collect()
}

/// Everything in the trash (blocking: run on a loader thread).
pub fn list() -> Result<Vec<TrashEntry>, String> {
    let items = trash::os_limited::list()
        .map_err(|e| explorer_i18n::tr!("Recycle Bin not readable: {0}", e))?;
    let reals: Vec<String> = items
        .iter()
        .map(|i| {
            real_name(
                &i.name.to_string_lossy(),
                &i.id.to_string_lossy(),
                cfg!(windows),
            )
        })
        .collect();
    let shown = unique_names(&reals);
    Ok(items
        .into_iter()
        .zip(reals)
        .zip(shown)
        .map(|((item, real), shown)| {
            let (is_dir, size) = match trash::os_limited::metadata(&item).map(|m| m.size) {
                Ok(trash::TrashItemSize::Bytes(b)) => (false, b),
                Ok(trash::TrashItemSize::Entries(_)) => (true, 0),
                Err(_) => (false, 0),
            };
            TrashEntry {
                entry: Entry {
                    name: shown,
                    kind: if is_dir {
                        EntryKind::Dir
                    } else {
                        EntryKind::File
                    },
                    size,
                    modified: Some(item.time_deleted),
                    hidden: false,
                    system: false,
                    readonly: false,
                    is_link: false,
                },
                original_parent: item.original_parent.clone(),
                real_name: real,
                item,
            }
        })
        .collect())
}

fn with_real_names(entries: &[TrashEntry]) -> Vec<trash::TrashItem> {
    entries
        .iter()
        .map(|e| {
            let mut item = e.item.clone();
            item.name = e.real_name.clone().into(); // restore under the real name
            item
        })
        .collect()
}

/// Puts the items back where they came from. Fails (and restores nothing) if
/// one of the original paths exists again; the message names it.
pub fn restore(entries: &[TrashEntry]) -> Result<(), String> {
    for e in entries {
        let target = e.original_parent.join(&e.real_name);
        if target.exists() {
            return Err(explorer_i18n::tr!(
                "An item already exists at the original location: {0}",
                target.display()
            ));
        }
        // the original folder may have been deleted meanwhile
        let _ = std::fs::create_dir_all(&e.original_parent);
    }
    trash::os_limited::restore_all(with_real_names(entries))
        .map_err(|e| explorer_i18n::tr!("Restore failed: {0}", e))
}

/// Deletes the items for good (cannot be undone).
pub fn purge(entries: &[TrashEntry]) -> Result<(), String> {
    trash::os_limited::purge_all(entries.iter().map(|e| e.item.clone()))
        .map_err(|e| explorer_i18n::tr!("Delete permanently failed: {0}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_real_names_get_their_extension_back() {
        let id = r"C:\$Recycle.Bin\S-1-5\$RAB12CD.docx";
        assert_eq!(real_name("Bericht", id, true), "Bericht.docx");
        assert_eq!(real_name("Bericht.docx", id, true), "Bericht.docx");
        assert_eq!(real_name("BERICHT.DOCX", id, true), "BERICHT.DOCX");
        assert_eq!(
            real_name("Ordner", r"C:\$Recycle.Bin\S\$RAB12CD", true),
            "Ordner"
        );
        assert_eq!(
            real_name("Bericht", id, false),
            "Bericht",
            "only on Windows"
        );
    }

    #[test]
    fn duplicate_names_become_addressable() {
        let n = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            unique_names(&n(&["a.txt", "b.txt", "A.txt", "a.txt"])),
            n(&["a.txt", "b.txt", "A.txt (2)", "a.txt (3)"])
        );
    }
}
