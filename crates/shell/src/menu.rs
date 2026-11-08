//! Native context menu (incl. third-party shell extensions) and the
//! properties dialog.
//!
//! Some verbs are handled by the app itself instead of the shell, so they get
//! our progress panel, conflict dialog and undo (`OwnVerb`); everything else
//! (Open with, Send to, 7-Zip, Git, antivirus, …) is executed by the shell.

use std::path::{Path, PathBuf};

/// Verbs the app performs itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnVerb {
    Rename,
    Delete,
    Cut,
    Copy,
    Paste,
    NewFolder,
}

/// Maps a shell verb (`GCS_VERBW`, language-independent) to our own action (pure → tested).
pub fn own_verb(verb: &str) -> Option<OwnVerb> {
    match verb.to_ascii_lowercase().as_str() {
        "rename" => Some(OwnVerb::Rename),
        "delete" => Some(OwnVerb::Delete),
        "cut" => Some(OwnVerb::Cut),
        "copy" => Some(OwnVerb::Copy),
        "paste" => Some(OwnVerb::Paste),
        "newfolder" => Some(OwnVerb::NewFolder),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuOutcome {
    /// Closed without a choice.
    Dismissed,
    /// The app should do this itself.
    Own(OwnVerb),
    /// The shell executed the command (verb for traces, may be empty).
    Shell(String),
    Failed(String),
}

/// One menu entry (for tests / diagnostics): label and language-independent verb.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuEntry {
    pub label: String,
    pub verb: String,
}

/// Shows the shell context menu for `items` (all in folder `dir`) at the
/// screen position `at` (physical pixels); `items` empty = folder background.
/// Blocks until the menu closes (modal, like in every Windows app).
pub fn show_context_menu(
    hwnd: isize,
    dir: &Path,
    items: &[PathBuf],
    at: (i32, i32),
) -> MenuOutcome {
    #[cfg(windows)]
    {
        crate::win_menu::show(hwnd, dir, items, at)
    }
    #[cfg(not(windows))]
    {
        let _ = (hwnd, dir, items, at);
        MenuOutcome::Failed("Kontextmenü gibt es nur unter Windows.".into())
    }
}

/// Builds the menu without showing it and lists its top-level entries
/// (lets CI check that the shell menu works, incl. verbs).
pub fn menu_entries(dir: &Path, items: &[PathBuf]) -> Result<Vec<MenuEntry>, String> {
    #[cfg(windows)]
    {
        crate::win_menu::entries(dir, items)
    }
    #[cfg(not(windows))]
    {
        let _ = (dir, items);
        Err("nur unter Windows".into())
    }
}

/// Window-client logical coordinates → screen pixels (for menus at a row).
pub fn client_to_screen(hwnd: isize, x: f32, y: f32, scale: f32) -> (i32, i32) {
    let (px, py) = ((x * scale).round() as i32, (y * scale).round() as i32);
    #[cfg(windows)]
    {
        crate::win_menu::client_to_screen(hwnd, px, py)
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        (px, py)
    }
}

/// Opens the Explorer properties dialog for `items` (Alt+Enter).
pub fn show_properties(hwnd: isize, items: &[PathBuf]) -> Result<(), String> {
    #[cfg(windows)]
    {
        crate::win_menu::properties(hwnd, items)
    }
    #[cfg(not(windows))]
    {
        let _ = (hwnd, items);
        Err("Eigenschaften gibt es nur unter Windows.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbs_we_handle_ourselves() {
        assert_eq!(own_verb("rename"), Some(OwnVerb::Rename));
        assert_eq!(own_verb("Delete"), Some(OwnVerb::Delete));
        assert_eq!(own_verb("copy"), Some(OwnVerb::Copy));
        assert_eq!(own_verb("cut"), Some(OwnVerb::Cut));
        assert_eq!(own_verb("paste"), Some(OwnVerb::Paste));
        assert_eq!(own_verb("NewFolder"), Some(OwnVerb::NewFolder));
        assert_eq!(own_verb("open"), None);
        assert_eq!(own_verb("properties"), None, "shell dialog");
        assert_eq!(own_verb(""), None);
    }
}
