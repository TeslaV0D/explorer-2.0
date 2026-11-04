//! Icon keys and the id registry.
//!
//! Most files share the icon of their extension, so the key is the extension
//! (a 100 000-file folder needs a handful of icons). Some types carry their
//! own icon per file (programs, shortcuts, icon files, custom folders) – those
//! get a key per path and are only loaded when visible.
//!
//! The registry hands out stable numeric ids (thread-safe, the loader thread
//! assigns them while preparing rows); the UI keeps one image per id, so an
//! icon arriving later updates every row using it without touching the rows.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IconKey {
    /// Plain folder.
    Folder,
    /// Files without own icon: lower-case extension ("" = no extension).
    Ext(String),
    /// Item with an individual icon (exe, lnk, ico, custom folder …).
    File(PathBuf),
}

impl IconKey {
    /// Per-file keys are expensive (one shell call per file) → load lazily.
    pub fn is_individual(&self) -> bool {
        matches!(self, IconKey::File(_))
    }
}

/// Extensions whose icon depends on the file itself (Explorer behaviour).
const INDIVIDUAL: &[&str] = &[
    "exe",
    "lnk",
    "ico",
    "url",
    "cur",
    "ani",
    "scr",
    "msc",
    "cpl",
    "appref-ms",
    "website",
];

/// Icon key for an entry named `name` in folder `dir`. Folders with the
/// read-only or system attribute may carry a custom icon (desktop.ini), so
/// they are looked up individually, like Explorer does.
pub fn icon_key(dir: &Path, name: &str, is_dir: bool, readonly_or_system: bool) -> IconKey {
    if is_dir {
        return if readonly_or_system {
            IconKey::File(dir.join(name))
        } else {
            IconKey::Folder
        };
    }
    let ext = match name.rfind('.') {
        Some(i) if i > 0 && i + 1 < name.len() => name[i + 1..].to_ascii_lowercase(),
        _ => String::new(),
    };
    if INDIVIDUAL.contains(&ext.as_str()) {
        IconKey::File(dir.join(name))
    } else {
        IconKey::Ext(ext)
    }
}

#[derive(Default)]
struct Inner {
    ids: HashMap<IconKey, u32>,
    keys: Vec<IconKey>,
}

/// Key ↔ id mapping shared between the loader thread and the UI.
/// Id 0 is reserved for "no icon" (the UI's fallback).
#[derive(Clone, Default)]
pub struct IconRegistry(Arc<Mutex<Inner>>);

impl IconRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Id for `key` (≥ 1), allocating a new one on first sight.
    pub fn id_for(&self, key: IconKey) -> u32 {
        let mut g = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(&id) = g.ids.get(&key) {
            return id;
        }
        g.keys.push(key.clone());
        let id = g.keys.len() as u32;
        g.ids.insert(key, id);
        id
    }

    pub fn key(&self, id: u32) -> Option<IconKey> {
        let g = self.0.lock().unwrap_or_else(|e| e.into_inner());
        id.checked_sub(1)
            .and_then(|i| g.keys.get(i as usize).cloned())
    }

    /// Number of ids handed out so far (ids are `1..=len`).
    pub fn len(&self) -> usize {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Straight (non-premultiplied) RGBA pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Icon {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconSize {
    /// 16 px (100 % scaling).
    Small,
    /// 32 px (for 125 %+ scaling, drawn at 16 logical px).
    Large,
}

impl IconSize {
    pub fn for_scale(scale: f32) -> Self {
        if scale > 1.1 {
            IconSize::Large
        } else {
            IconSize::Small
        }
    }
}

/// Asks the shell for the icon of `key`. Blocking (shell calls can take a
/// while, e.g. on network paths) → call from a background thread that has
/// COM initialized (`init_thread`). `None` on other platforms or failure.
pub fn load_icon(key: &IconKey, size: IconSize) -> Option<Icon> {
    #[cfg(windows)]
    {
        crate::win::load_icon(key, size)
    }
    #[cfg(not(windows))]
    {
        let _ = (key, size);
        None
    }
}

/// Prepares a worker thread for shell calls (COM apartment on Windows).
pub fn init_thread() {
    #[cfg(windows)]
    crate::win::init_thread();
}

/// Converts a 32-bit icon bitmap (BGRA, top-down) to straight RGBA.
/// Old-style icons have no alpha channel (all 0); then the AND mask decides:
/// black mask pixel = opaque, white = transparent (pure → tested).
pub fn bgra_to_rgba(color: &[u8], mask: Option<&[u8]>) -> Vec<u8> {
    let has_alpha = color.as_chunks::<4>().0.iter().any(|p| p[3] != 0);
    let mut out = Vec::with_capacity(color.len());
    for (i, p) in color.as_chunks::<4>().0.iter().enumerate() {
        let a = if has_alpha {
            p[3]
        } else {
            match mask.and_then(|m| m.get(i * 4)) {
                Some(&m) if m != 0 => 0,
                _ => 255,
            }
        };
        out.extend_from_slice(&[p[2], p[1], p[0], a]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_conversion() {
        // with alpha: channels swapped, alpha kept
        assert_eq!(bgra_to_rgba(&[1, 2, 3, 128], None), vec![3, 2, 1, 128]);
        // without alpha: mask black (0) = opaque, white (255) = transparent
        let color = [10, 20, 30, 0, 40, 50, 60, 0];
        let mask = [0, 0, 0, 0, 255, 255, 255, 0];
        assert_eq!(
            bgra_to_rgba(&color, Some(&mask)),
            vec![30, 20, 10, 255, 60, 50, 40, 0]
        );
        // without alpha and mask: opaque
        assert_eq!(bgra_to_rgba(&[1, 2, 3, 0], None), vec![3, 2, 1, 255]);
    }

    #[test]
    fn keys_follow_explorer_rules() {
        let d = Path::new("/d");
        assert_eq!(
            icon_key(d, "a.TXT", false, false),
            IconKey::Ext("txt".into())
        );
        assert_eq!(
            icon_key(d, "README", false, false),
            IconKey::Ext(String::new())
        );
        assert_eq!(
            icon_key(d, ".gitignore", false, false),
            IconKey::Ext(String::new())
        );
        assert_eq!(icon_key(d, "x.", false, false), IconKey::Ext(String::new()));
        assert_eq!(
            icon_key(d, "Setup.EXE", false, false),
            IconKey::File(d.join("Setup.EXE"))
        );
        assert_eq!(
            icon_key(d, "Link.lnk", false, false),
            IconKey::File(d.join("Link.lnk"))
        );
        assert_eq!(icon_key(d, "Fotos", true, false), IconKey::Folder);
        assert_eq!(
            icon_key(d, "Custom", true, true),
            IconKey::File(d.join("Custom"))
        );
        assert!(icon_key(d, "a.ico", false, false).is_individual());
        assert!(!icon_key(d, "a.png", false, false).is_individual());
    }

    #[test]
    fn registry_ids_are_stable_and_start_at_one() {
        let r = IconRegistry::new();
        assert!(r.is_empty());
        let a = r.id_for(IconKey::Ext("txt".into()));
        let b = r.id_for(IconKey::Folder);
        assert_eq!((a, b), (1, 2));
        assert_eq!(r.id_for(IconKey::Ext("txt".into())), 1, "same key, same id");
        assert_eq!(r.key(2), Some(IconKey::Folder));
        assert_eq!(r.key(0), None, "0 = no icon");
        assert_eq!(r.key(3), None);
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn registry_is_shared_across_threads() {
        let r = IconRegistry::new();
        let handles: Vec<_> = (0..4)
            .map(|t| {
                let r = r.clone();
                std::thread::spawn(move || {
                    (0..100)
                        .map(|i| r.id_for(IconKey::Ext(format!("e{}", (i + t) % 50))))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(r.len(), 50, "no duplicate ids under contention");
    }

    #[test]
    fn icon_size_from_scale() {
        assert_eq!(IconSize::for_scale(1.0), IconSize::Small);
        assert_eq!(IconSize::for_scale(1.25), IconSize::Large);
        assert_eq!(IconSize::for_scale(2.0), IconSize::Large);
    }

    #[test]
    fn no_icons_without_shell() {
        if !crate::SHELL_SUPPORTED {
            assert_eq!(load_icon(&IconKey::Folder, IconSize::Small), None);
        }
    }
}
