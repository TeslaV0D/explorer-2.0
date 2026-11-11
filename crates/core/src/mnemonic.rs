//! Access keys (mnemonics) per language, like Windows' `&` in menu texts:
//! the translation of `&Replace` decides which letter answers a dialog
//! (`E&rsetzen` → r, `&Reemplazar` → r). The text shown is the same
//! translation without `&` (a catalog test checks both stay in step).

use explorer_i18n::tr_noop;

/// What a key in the conflict dialog does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKey {
    /// Replace (files) / merge (folders) = answer 0
    Replace,
    Skip,
    KeepBoth,
    /// toggles “Do this for all remaining conflicts”
    All,
}

/// `(mnemonic source, label source, action)`; the label is what the button
/// or check box shows (`@tr` in main.slint).
pub const CONFLICT: &[(&str, &str, ConflictKey)] = &[
    (tr_noop!("&Replace"), "Replace", ConflictKey::Replace),
    (tr_noop!("Me&rge"), "Merge", ConflictKey::Replace),
    (tr_noop!("&Skip"), "Skip", ConflictKey::Skip),
    (tr_noop!("Keep &both"), "Keep both", ConflictKey::KeepBoth),
    (
        tr_noop!("Do this for &all remaining conflicts"),
        "Do this for all remaining conflicts",
        ConflictKey::All,
    ),
];

/// The letter after the first `&` (lower case), `&&` is a literal `&`.
pub fn key_of(text: &str) -> Option<char> {
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '&' {
            match chars.next() {
                Some('&') => continue,
                Some(k) => return k.to_lowercase().next(),
                None => return None,
            }
        }
    }
    None
}

/// The text without its `&` marker (`&&` → `&`).
pub fn strip(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '&' {
            if let Some(n) = chars.next() {
                out.push(n);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// A key typed in the conflict dialog → its action in the current language
/// (`dirs`: the dialog offers “Merge” instead of “Replace”).
pub fn conflict_key(typed: &str, dirs: bool) -> Option<ConflictKey> {
    let mut chars = typed.chars();
    let typed = chars.next()?.to_lowercase().next()?;
    if chars.next().is_some() {
        return None;
    }
    CONFLICT
        .iter()
        .filter(|(_, label, _)| match *label {
            "Replace" => !dirs,
            "Merge" => dirs,
            _ => true,
        })
        .find(|(m, _, _)| key_of(explorer_i18n::translate(m)) == Some(typed))
        .map(|(_, _, k)| *k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers() {
        assert_eq!(key_of("E&rsetzen"), Some('r'));
        assert_eq!(key_of("&Überspringen"), Some('ü'));
        assert_eq!(key_of("Fish && &Chips"), Some('c'));
        assert_eq!(key_of("none"), None);
        assert_eq!(key_of("trailing&"), None);
        assert_eq!(strip("Keep &both"), "Keep both");
        assert_eq!(strip("Fish && &Chips"), "Fish & Chips");
    }

    #[test]
    fn english_keys() {
        // unit tests see the English source
        assert_eq!(conflict_key("r", false), Some(ConflictKey::Replace));
        assert_eq!(conflict_key("R", true), Some(ConflictKey::Replace));
        assert_eq!(conflict_key("s", false), Some(ConflictKey::Skip));
        assert_eq!(conflict_key("b", true), Some(ConflictKey::KeepBoth));
        assert_eq!(conflict_key("a", false), Some(ConflictKey::All));
        assert_eq!(conflict_key("x", false), None);
        assert_eq!(conflict_key("rs", false), None);
        assert_eq!(conflict_key("", false), None);
        for (m, label, _) in CONFLICT {
            assert_eq!(strip(m), *label);
        }
    }
}
