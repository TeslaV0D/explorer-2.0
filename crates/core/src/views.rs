//! Remembered view per folder ("Ansicht pro Ordner", like Explorer): a small
//! LRU map folder → view mode, stored as `views.ini` (one `mode<TAB>path` per
//! line, oldest first). Folders without an entry use the default view.

use crate::view::ViewMode;
use std::collections::HashMap;

/// Upper bound of remembered folders; the least recently changed drop out.
pub const MAX_FOLDERS: usize = 5000;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderViews {
    /// key → (mode, original path, stamp); higher stamp = changed later.
    map: HashMap<String, (ViewMode, String, u64)>,
    next: u64,
    /// Windows paths compare case-insensitively.
    case_insensitive: bool,
}

impl FolderViews {
    pub fn new(case_insensitive: bool) -> Self {
        FolderViews {
            case_insensitive,
            ..Default::default()
        }
    }

    fn key(&self, path: &str) -> String {
        let p = path.trim_end_matches(['\\', '/']);
        let p = if p.is_empty() { path } else { p };
        if self.case_insensitive {
            p.to_lowercase()
        } else {
            p.to_string()
        }
    }

    pub fn get(&self, path: &str) -> Option<ViewMode> {
        self.map.get(&self.key(path)).map(|(m, _, _)| *m)
    }

    pub fn set(&mut self, path: &str, mode: ViewMode) {
        let k = self.key(path);
        self.next += 1;
        self.map.insert(k, (mode, path.to_string(), self.next));
        if self.map.len() > MAX_FOLDERS {
            if let Some(oldest) = self
                .map
                .iter()
                .min_by_key(|(_, (_, _, s))| *s)
                .map(|(k, _)| k.clone())
            {
                self.map.remove(&oldest);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Tolerant: unknown modes, empty and broken lines are skipped.
    pub fn parse(text: &str, case_insensitive: bool) -> Self {
        let mut v = FolderViews::new(case_insensitive);
        for line in text.lines() {
            // a lone `\r` at the very end (hand-edited file) is no part of a path
            let line = line.strip_suffix('\r').unwrap_or(line);
            let Some((mode, path)) = line.split_once('\t') else {
                continue;
            };
            if let (Some(m), false) = (ViewMode::parse(mode.trim()), path.is_empty()) {
                v.set(path, m);
            }
        }
        v
    }

    pub fn serialize(&self) -> String {
        let mut items: Vec<_> = self.map.values().collect();
        items.sort_by_key(|(_, _, s)| *s);
        items
            .into_iter()
            .map(|(m, p, _)| format!("{}\t{}\n", m.as_str(), p))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_set_round_trip_and_case() {
        let mut v = FolderViews::new(true);
        v.set(r"C:\Bilder", ViewMode::Large);
        v.set(r"C:\Daten\", ViewMode::Details);
        assert_eq!(
            v.get(r"c:\bilder\"),
            Some(ViewMode::Large),
            "Windows: case and trailing slash"
        );
        assert_eq!(v.get(r"C:\Daten"), Some(ViewMode::Details));
        assert_eq!(v.get(r"C:\Neu"), None);
        let back = FolderViews::parse(&v.serialize(), true);
        assert_eq!(back.get(r"C:\BILDER"), Some(ViewMode::Large));
        assert_eq!(back.len(), 2);
        let unix = FolderViews::parse("large\t/home/b/Fotos\n", false);
        assert_eq!(unix.get("/home/b/fotos"), None, "Linux is case-sensitive");
        assert_eq!(unix.get("/home/b/Fotos/"), Some(ViewMode::Large));
        assert_eq!(FolderViews::new(false).get("/"), None);
    }

    #[test]
    fn tolerant_parse_and_bounded_size() {
        let v = FolderViews::parse("garbage\nxlarge\t/a\nbogus\t/b\nmedium\t\n\n", false);
        assert_eq!((v.len(), v.get("/a")), (1, Some(ViewMode::ExtraLarge)));
        let mut big = FolderViews::new(false);
        for i in 0..MAX_FOLDERS + 10 {
            big.set(&format!("/f{i}"), ViewMode::Medium);
        }
        assert_eq!(big.len(), MAX_FOLDERS);
        assert_eq!(big.get("/f0"), None, "oldest dropped");
        assert_eq!(
            big.get(&format!("/f{}", MAX_FOLDERS + 9)),
            Some(ViewMode::Medium)
        );
        // re-setting refreshes the age
        let mut v = FolderViews::new(false);
        v.set("/x", ViewMode::Large);
        v.set("/y", ViewMode::Large);
        v.set("/x", ViewMode::Medium);
        assert!(v.serialize().ends_with("medium\t/x\n"));
    }
}
