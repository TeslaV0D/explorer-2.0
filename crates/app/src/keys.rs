//! Keyboard mapping for the file list (pure → unit-tested).
//! Key texts are Slint's special code points (see i-slint-common key_codes).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    /// Relative move (arrows, PageUp/PageDown).
    Move {
        delta: isize,
        extend: bool,
        focus_only: bool,
    },
    /// Absolute move to first (`false`) / last (`true`) item.
    Edge {
        end: bool,
        extend: bool,
        focus_only: bool,
    },
    SelectAll,
    ToggleFocused,
    Open,
    Up,
    Refresh,
    /// Ctrl+L / Alt+D: edit the address bar (Explorer shortcut).
    EditPath,
    /// Ctrl+F / Ctrl+E / F3: the search field of the pane (S1.4b).
    FocusSearch,
    /// Alt+← / Alt+→ (mouse side buttons are handled in the UI).
    Back,
    Forward,
    /// Ctrl+T / Ctrl+W / Ctrl+Tab / Ctrl+Shift+Tab.
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    /// F6: other pane (split view).
    SwitchPane,
    /// Ctrl+\: split view on/off.
    ToggleSplit,
    /// File operations (M4).
    Copy,
    Cut,
    Paste,
    /// Del → trash, Shift+Del → permanent (with confirmation).
    Delete {
        permanent: bool,
    },
    /// F2
    Rename,
    /// Ctrl+Shift+N
    NewFolder,
    Undo,
    Redo,
    /// Esc: close dialogs / clear cut marks.
    Escape,
    /// Shift+F10 / menu key: context menu of the selection.
    ContextMenu,
    /// Alt+Enter: properties dialog.
    Properties,
    /// Alt+P: inspector (Explorer's preview pane shortcut).
    ToggleInspector,
    /// Ctrl+Shift+P: command palette.
    Palette,
    /// Ctrl+Shift+1/2/3/6: icon sizes / details (Explorer).
    View(explorer_core::view::ViewMode),
    TypeChar(char),
}

const UP: char = '\u{F700}';
const DOWN: char = '\u{F701}';
const PAGE_UP: char = '\u{F72C}';
const PAGE_DOWN: char = '\u{F72D}';
const HOME: char = '\u{F729}';
const END: char = '\u{F72B}';
const RETURN: char = '\n';
const BACKSPACE: char = '\u{8}';
const F5: char = '\u{F708}';
const F6: char = '\u{F709}';
const LEFT: char = '\u{F702}';
const RIGHT: char = '\u{F703}';
const TAB: char = '\t';
const BACKTAB: char = '\u{19}';
const DELETE: char = '\u{7F}';
const F2: char = '\u{F705}';
const F3: char = '\u{F706}';
const INSERT: char = '\u{F727}';
const ESCAPE: char = '\u{1B}';
const F10: char = '\u{F70D}';
const MENU: char = '\u{F735}';

/// Details view (one item per row).
#[cfg(test)]
pub fn map_key(text: &str, ctrl: bool, shift: bool, alt: bool, page: isize) -> Option<KeyAction> {
    map_key_grid(text, ctrl, shift, alt, page, 1)
}

/// `cols` items per row (icon view): ↑/↓ move a whole row, ←/→ one item,
/// PageUp/PageDown `page` rows.
pub fn map_key_grid(
    text: &str,
    ctrl: bool,
    shift: bool,
    alt: bool,
    page: isize,
    cols: isize,
) -> Option<KeyAction> {
    let cols = cols.max(1);
    let mut chars = text.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None; // IME/multi-char input is not a list command
    }
    let page = page.max(1) * cols;
    let mv = |delta| {
        Some(KeyAction::Move {
            delta,
            extend: shift,
            focus_only: ctrl && !shift,
        })
    };
    match c {
        UP if alt => Some(KeyAction::Up),
        LEFT if alt => Some(KeyAction::Back),
        RIGHT if alt => Some(KeyAction::Forward),
        TAB | BACKTAB if ctrl => Some(if shift || c == BACKTAB {
            KeyAction::PrevTab
        } else {
            KeyAction::NextTab
        }),
        F6 => Some(KeyAction::SwitchPane),
        c if ctrl && shift && !alt && explorer_core::view::ViewMode::from_shortcut(c).is_some() => {
            explorer_core::view::ViewMode::from_shortcut(c).map(KeyAction::View)
        }
        UP => mv(-cols),
        DOWN => mv(cols),
        LEFT if cols > 1 => mv(-1),
        RIGHT if cols > 1 => mv(1),
        PAGE_UP => mv(-page),
        PAGE_DOWN => mv(page),
        HOME => Some(KeyAction::Edge {
            end: false,
            extend: shift,
            focus_only: ctrl && !shift,
        }),
        END => Some(KeyAction::Edge {
            end: true,
            extend: shift,
            focus_only: ctrl && !shift,
        }),
        RETURN if alt => Some(KeyAction::Properties),
        RETURN => Some(KeyAction::Open),
        F10 if shift => Some(KeyAction::ContextMenu),
        MENU => Some(KeyAction::ContextMenu),
        BACKSPACE if !ctrl && !alt => Some(KeyAction::Up),
        F5 => Some(KeyAction::Refresh),
        ' ' if ctrl => Some(KeyAction::ToggleFocused),
        'a' | 'A' if ctrl && !alt => Some(KeyAction::SelectAll),
        'l' | 'L' if ctrl && !alt => Some(KeyAction::EditPath),
        'f' | 'F' | 'e' | 'E' if ctrl && !alt && !shift => Some(KeyAction::FocusSearch),
        F3 if !ctrl && !alt => Some(KeyAction::FocusSearch),
        't' | 'T' if ctrl && !alt => Some(KeyAction::NewTab),
        'w' | 'W' if ctrl && !alt => Some(KeyAction::CloseTab),
        '\\' if ctrl && !alt => Some(KeyAction::ToggleSplit),
        'c' | 'C' if ctrl && !alt && !shift => Some(KeyAction::Copy),
        'x' | 'X' if ctrl && !alt && !shift => Some(KeyAction::Cut),
        'v' | 'V' if ctrl && !alt && !shift => Some(KeyAction::Paste),
        INSERT if ctrl && !shift => Some(KeyAction::Copy),
        INSERT if shift && !ctrl => Some(KeyAction::Paste),
        DELETE if shift && !ctrl && !alt => Some(KeyAction::Delete { permanent: true }),
        DELETE if !ctrl && !alt => Some(KeyAction::Delete { permanent: false }),
        F2 if !ctrl && !alt => Some(KeyAction::Rename),
        'n' | 'N' if ctrl && shift && !alt => Some(KeyAction::NewFolder),
        'z' | 'Z' if ctrl && !alt && !shift => Some(KeyAction::Undo),
        'y' | 'Y' if ctrl && !alt && !shift => Some(KeyAction::Redo),
        'z' | 'Z' if ctrl && shift && !alt => Some(KeyAction::Redo),
        ESCAPE => Some(KeyAction::Escape),
        'd' | 'D' if alt && !ctrl => Some(KeyAction::EditPath),
        'p' | 'P' if alt && !ctrl => Some(KeyAction::ToggleInspector),
        'p' | 'P' if ctrl && shift && !alt => Some(KeyAction::Palette),
        // printable text → type-to-select (no private-use specials, no control chars)
        c if !ctrl && !alt && !c.is_control() && !('\u{E000}'..='\u{F8FF}').contains(&c) => {
            Some(KeyAction::TypeChar(c))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_navigation_and_view_shortcuts() {
        let mv = |d, e| {
            Some(KeyAction::Move {
                delta: d,
                extend: e,
                focus_only: false,
            })
        };
        assert_eq!(
            map_key_grid("\u{F701}", false, false, false, 3, 5),
            mv(5, false)
        );
        assert_eq!(
            map_key_grid("\u{F700}", false, true, false, 3, 5),
            mv(-5, true)
        );
        assert_eq!(
            map_key_grid("\u{F703}", false, false, false, 3, 5),
            mv(1, false)
        );
        assert_eq!(
            map_key_grid("\u{F702}", false, false, false, 3, 5),
            mv(-1, false)
        );
        assert_eq!(
            map_key_grid("\u{F72D}", false, false, false, 3, 5),
            mv(15, false)
        );
        assert_eq!(
            map_key_grid("\u{F702}", false, false, false, 3, 1),
            None,
            "details: ← nothing"
        );
        use explorer_core::view::ViewMode;
        assert_eq!(
            map_key("1", true, true, false, 1),
            Some(KeyAction::View(ViewMode::ExtraLarge))
        );
        assert_eq!(
            map_key("\"", true, true, false, 1),
            Some(KeyAction::View(ViewMode::Large))
        );
        assert_eq!(
            map_key("6", true, true, false, 1),
            Some(KeyAction::View(ViewMode::Details))
        );
        assert_eq!(
            map_key("1", false, false, false, 1),
            Some(KeyAction::TypeChar('1'))
        );
    }

    #[test]
    fn navigation_keys() {
        assert_eq!(
            map_key("\u{F701}", false, false, false, 20),
            Some(KeyAction::Move {
                delta: 1,
                extend: false,
                focus_only: false
            })
        );
        assert_eq!(
            map_key("\u{F700}", false, true, false, 20),
            Some(KeyAction::Move {
                delta: -1,
                extend: true,
                focus_only: false
            })
        );
        assert_eq!(
            map_key("\u{F701}", true, false, false, 20),
            Some(KeyAction::Move {
                delta: 1,
                extend: false,
                focus_only: true
            })
        );
        assert_eq!(
            map_key("\u{F72D}", false, false, false, 20),
            Some(KeyAction::Move {
                delta: 20,
                extend: false,
                focus_only: false
            })
        );
        assert_eq!(
            map_key("\u{F72C}", false, false, false, 0),
            Some(KeyAction::Move {
                delta: -1,
                extend: false,
                focus_only: false
            })
        );
        assert_eq!(
            map_key("\u{F72B}", false, true, false, 20),
            Some(KeyAction::Edge {
                end: true,
                extend: true,
                focus_only: false
            })
        );
        assert_eq!(
            map_key("\u{F729}", false, false, false, 20),
            Some(KeyAction::Edge {
                end: false,
                extend: false,
                focus_only: false
            })
        );
    }

    #[test]
    fn commands() {
        assert_eq!(map_key("\n", false, false, false, 1), Some(KeyAction::Open));
        assert_eq!(
            map_key("\u{8}", false, false, false, 1),
            Some(KeyAction::Up)
        );
        assert_eq!(
            map_key("\u{F700}", false, false, true, 1),
            Some(KeyAction::Up)
        );
        assert_eq!(
            map_key("a", true, false, false, 1),
            Some(KeyAction::SelectAll)
        );
        assert_eq!(
            map_key(" ", true, false, false, 1),
            Some(KeyAction::ToggleFocused)
        );
        assert_eq!(
            map_key("\u{F708}", false, false, false, 1),
            Some(KeyAction::Refresh)
        );
        assert_eq!(
            map_key("l", true, false, false, 1),
            Some(KeyAction::EditPath)
        );
        assert_eq!(
            map_key("d", false, false, true, 1),
            Some(KeyAction::EditPath)
        );
        assert_eq!(
            map_key("d", false, false, false, 1),
            Some(KeyAction::TypeChar('d'))
        );
    }

    #[test]
    fn tab_pane_history_keys() {
        assert_eq!(
            map_key("\u{F702}", false, false, true, 1),
            Some(KeyAction::Back)
        );
        assert_eq!(
            map_key("\u{F703}", false, false, true, 1),
            Some(KeyAction::Forward)
        );
        assert_eq!(map_key("\u{F702}", false, false, false, 1), None); // plain Left: nothing in details view
        assert_eq!(map_key("t", true, false, false, 1), Some(KeyAction::NewTab));
        assert_eq!(
            map_key("w", true, false, false, 1),
            Some(KeyAction::CloseTab)
        );
        assert_eq!(
            map_key("\t", true, false, false, 1),
            Some(KeyAction::NextTab)
        );
        assert_eq!(
            map_key("\t", true, true, false, 1),
            Some(KeyAction::PrevTab)
        );
        assert_eq!(
            map_key("\u{19}", true, true, false, 1),
            Some(KeyAction::PrevTab)
        );
        assert_eq!(map_key("\t", false, false, false, 1), None); // plain Tab = focus navigation
        assert_eq!(
            map_key("\u{F709}", false, false, false, 1),
            Some(KeyAction::SwitchPane)
        );
        assert_eq!(
            map_key("\\", true, false, false, 1),
            Some(KeyAction::ToggleSplit)
        );
        assert_eq!(
            map_key("t", false, false, false, 1),
            Some(KeyAction::TypeChar('t'))
        );
    }

    #[test]
    fn typeahead_chars() {
        assert_eq!(
            map_key("b", false, false, false, 1),
            Some(KeyAction::TypeChar('b'))
        );
        assert_eq!(
            map_key("Ä", false, true, false, 1),
            Some(KeyAction::TypeChar('Ä'))
        );
        assert_eq!(
            map_key(" ", false, false, false, 1),
            Some(KeyAction::TypeChar(' '))
        );
        assert_eq!(map_key("x", true, false, false, 1), Some(KeyAction::Cut)); // not typeahead
        assert_eq!(map_key("\u{F704}", false, false, false, 1), None); // F1
        assert_eq!(
            map_key("\u{1b}", false, false, false, 1),
            Some(KeyAction::Escape)
        );
        assert_eq!(map_key("", false, false, false, 1), None);
        assert_eq!(map_key("ab", false, false, false, 1), None);
    }

    #[test]
    fn file_operation_keys() {
        let k = |t, c, sh| map_key(t, c, sh, false, 1);
        assert_eq!(k("c", true, false), Some(KeyAction::Copy));
        assert_eq!(k("v", true, false), Some(KeyAction::Paste));
        assert_eq!(k("\u{F727}", true, false), Some(KeyAction::Copy));
        assert_eq!(k("\u{F727}", false, true), Some(KeyAction::Paste));
        assert_eq!(
            k("\u{7F}", false, false),
            Some(KeyAction::Delete { permanent: false })
        );
        assert_eq!(
            k("\u{7F}", false, true),
            Some(KeyAction::Delete { permanent: true })
        );
        assert_eq!(k("\u{F705}", false, false), Some(KeyAction::Rename));
        assert_eq!(k("N", true, true), Some(KeyAction::NewFolder));
        assert_eq!(k("n", true, true), Some(KeyAction::NewFolder));
        assert_eq!(k("n", true, false), None, "Ctrl+N = new window (later)");
        assert_eq!(k("z", true, false), Some(KeyAction::Undo));
        assert_eq!(k("y", true, false), Some(KeyAction::Redo));
        assert_eq!(k("Z", true, true), Some(KeyAction::Redo));
        assert_eq!(k("\u{F70D}", false, true), Some(KeyAction::ContextMenu));
        assert_eq!(k("\u{F735}", false, false), Some(KeyAction::ContextMenu));
        assert_eq!(
            map_key("\n", false, false, true, 1),
            Some(KeyAction::Properties)
        );
        assert_eq!(
            k("\u{F70D}", false, false),
            None,
            "plain F10 = menu bar (later)"
        );
    }
}
