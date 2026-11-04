//! Drag & drop with other applications (OLE on Windows).
//!
//! The decision logic is platform-neutral and tested; the Windows part
//! (`win_dnd`) only moves data between OLE and these types.

use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropEffect {
    None,
    Copy,
    Move,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DropKeys {
    pub ctrl: bool,
    pub shift: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPhase {
    /// Cursor entered or moved: answer with the effect to show.
    Over,
    /// Released: perform the operation, answer with what was done.
    Drop,
    /// Left the window / cancelled.
    Leave,
}

/// What a drop handler is asked (on the UI thread).
#[derive(Debug, Clone)]
pub struct DropEvent<'a> {
    pub phase: DropPhase,
    pub paths: &'a [PathBuf],
    /// Screen position in physical pixels.
    pub screen: (i32, i32),
    pub keys: DropKeys,
    pub allow_copy: bool,
    pub allow_move: bool,
}

/// Volume of a path: drive/UNC prefix (case-insensitive) or the root on Unix.
fn volume(p: &Path) -> String {
    match p.components().next() {
        Some(Component::Prefix(pre)) => pre.as_os_str().to_string_lossy().to_lowercase(),
        Some(Component::RootDir) => "/".into(),
        _ => String::new(),
    }
}

pub fn same_volume(a: &Path, b: &Path) -> bool {
    volume(a) == volume(b)
}

/// Explorer's default: same drive → move, other drive → copy; Ctrl forces
/// copy, Shift forces move; limited to what the source allows (pure → tested).
pub fn default_effect(
    sources: &[PathBuf],
    dest: &Path,
    keys: DropKeys,
    allow_copy: bool,
    allow_move: bool,
) -> DropEffect {
    if sources.is_empty() {
        return DropEffect::None;
    }
    let wanted = match (keys.ctrl, keys.shift) {
        (true, false) => DropEffect::Copy,
        (false, true) => DropEffect::Move,
        _ if sources.iter().all(|s| same_volume(s, dest)) => DropEffect::Move,
        _ => DropEffect::Copy,
    };
    match wanted {
        DropEffect::Move if allow_move => DropEffect::Move,
        DropEffect::Copy if allow_copy => DropEffect::Copy,
        // fall back to what the source allows
        _ if allow_copy => DropEffect::Copy,
        _ if allow_move => DropEffect::Move,
        _ => DropEffect::None,
    }
}

/// Lets other apps drop files on window `hwnd`; `handler` runs on the UI
/// thread (OLE calls back on the registering thread). Replaces the window's
/// previous drop target (winit registers its own).
pub fn register_drop_target(
    hwnd: isize,
    handler: impl Fn(&DropEvent) -> DropEffect + 'static,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        crate::win_dnd::register(hwnd, Box::new(handler))
    }
    #[cfg(not(windows))]
    {
        let _ = (hwnd, handler);
        Err("OLE drag & drop only on Windows".into())
    }
}

/// Starts a system drag of `items` (all in one folder) from window `hwnd`.
/// Blocks until dropped (modal). Returns what the target did.
pub fn start_drag(hwnd: isize, items: &[PathBuf]) -> Result<DropEffect, String> {
    #[cfg(windows)]
    {
        crate::win_dnd::start_drag(hwnd, items)
    }
    #[cfg(not(windows))]
    {
        let _ = (hwnd, items);
        Err("OLE drag & drop only on Windows".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn explorer_default_effects() {
        let none = DropKeys::default();
        let ctrl = DropKeys {
            ctrl: true,
            shift: false,
        };
        let shift = DropKeys {
            ctrl: false,
            shift: true,
        };
        #[cfg(windows)]
        let (src, same, other) = (vec![p(r"C:\a\x.txt")], p(r"c:\b"), p(r"D:\b"));
        #[cfg(not(windows))]
        let (src, same, other) = (vec![p("/a/x.txt")], p("/b"), p("relative"));
        assert_eq!(
            default_effect(&src, &same, none, true, true),
            DropEffect::Move,
            "same volume"
        );
        assert_eq!(
            default_effect(&src, &other, none, true, true),
            DropEffect::Copy,
            "other volume"
        );
        assert_eq!(
            default_effect(&src, &same, ctrl, true, true),
            DropEffect::Copy
        );
        assert_eq!(
            default_effect(&src, &other, shift, true, true),
            DropEffect::Move
        );
        // source only allows copy (e.g. a browser download, a read-only medium)
        assert_eq!(
            default_effect(&src, &same, shift, true, false),
            DropEffect::Copy
        );
        assert_eq!(
            default_effect(&src, &same, none, false, false),
            DropEffect::None
        );
        assert_eq!(
            default_effect(&[], &same, none, true, true),
            DropEffect::None
        );
    }

    #[cfg(windows)]
    #[test]
    fn volumes_on_windows() {
        assert!(same_volume(Path::new(r"C:\x"), Path::new(r"c:\y\z")));
        assert!(!same_volume(Path::new(r"C:\x"), Path::new(r"D:\x")));
        assert!(same_volume(
            Path::new(r"\\srv\share\a"),
            Path::new(r"\\SRV\share\b")
        ));
        assert!(!same_volume(
            Path::new(r"\\srv\a\x"),
            Path::new(r"\\srv\b\x")
        ));
    }
}
