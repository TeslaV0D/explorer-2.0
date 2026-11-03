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

pub fn map_key(text: &str, ctrl: bool, shift: bool, alt: bool, page: isize) -> Option<KeyAction> {
    let mut chars = text.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None; // IME/multi-char input is not a list command
    }
    let page = page.max(1);
    let mv = |delta| {
        Some(KeyAction::Move {
            delta,
            extend: shift,
            focus_only: ctrl && !shift,
        })
    };
    match c {
        UP if alt => Some(KeyAction::Up),
        UP => mv(-1),
        DOWN => mv(1),
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
        RETURN => Some(KeyAction::Open),
        BACKSPACE if !ctrl && !alt => Some(KeyAction::Up),
        F5 => Some(KeyAction::Refresh),
        ' ' if ctrl => Some(KeyAction::ToggleFocused),
        'a' | 'A' if ctrl && !alt => Some(KeyAction::SelectAll),
        'l' | 'L' if ctrl && !alt => Some(KeyAction::EditPath),
        'd' | 'D' if alt && !ctrl => Some(KeyAction::EditPath),
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
        assert_eq!(map_key("x", true, false, false, 1), None); // ctrl+x = cut (M4), not typeahead
        assert_eq!(map_key("\u{F704}", false, false, false, 1), None); // F1
        assert_eq!(map_key("\u{1b}", false, false, false, 1), None); // Esc
        assert_eq!(map_key("", false, false, false, 1), None);
        assert_eq!(map_key("ab", false, false, false, 1), None);
    }
}
