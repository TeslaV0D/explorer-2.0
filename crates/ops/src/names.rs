//! File-name rules (pure): validation and Explorer-style generated names.

/// Characters Windows forbids in file names.
const WINDOWS_FORBIDDEN: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Validates a new name (message in the current language on error). `windows` selects the
/// Windows rules; on other platforms only `/`, NUL, "." and ".." are invalid.
pub fn validate_name_for(name: &str, windows: bool) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err(explorer_i18n::tr!("The name can't be empty.").into());
    }
    if name == "." || name == ".." {
        return Err(explorer_i18n::tr!("“.” and “..” are not allowed as a name.").into());
    }
    if name.chars().count() > 255 {
        return Err(explorer_i18n::tr!("The name is too long (at most 255 characters).").into());
    }
    if name.contains('\0') || name.contains('/') {
        return Err(explorer_i18n::tr!("The name contains an invalid character.").into());
    }
    if windows {
        if let Some(c) = name
            .chars()
            .find(|c| WINDOWS_FORBIDDEN.contains(c) || c.is_control())
        {
            let shown = if c.is_control() {
                explorer_i18n::tr!("control characters").to_string()
            } else {
                explorer_i18n::tr!("“{0}”", c)
            };
            return Err(explorer_i18n::tr!(
                "A file name can't contain {0}: \\ / : * ? \" < > |",
                shown
            ));
        }
        if name.ends_with('.') || name.ends_with(' ') {
            return Err(explorer_i18n::tr!("The name can't end with a period or a space.").into());
        }
        let stem = name.split('.').next().unwrap_or(name).trim_end();
        if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(stem)) {
            return Err(explorer_i18n::tr!(
                "“{0}” is a reserved device name on Windows.",
                stem
            ));
        }
    }
    Ok(())
}

pub fn validate_name(name: &str) -> Result<(), String> {
    validate_name_for(name, cfg!(windows))
}

/// Splits "a.tar.gz" → ("a.tar", ".gz"); folders and dot-files have no extension.
fn split_ext(name: &str, is_dir: bool) -> (&str, &str) {
    if is_dir {
        return (name, "");
    }
    match name.rfind('.') {
        Some(i) if i > 0 && i + 1 < name.len() => (&name[..i], &name[i..]),
        _ => (name, ""),
    }
}

/// Explorer copy name in the same folder: "Report - Copy.docx" (German
/// "Bericht - Kopie.docx"), then "… - Copy (2).docx", … `exists` tells whether
/// a name is taken. Like Explorer, the word follows the UI language.
pub fn copy_name(name: &str, is_dir: bool, exists: impl Fn(&str) -> bool) -> String {
    let (stem, ext) = split_ext(name, is_dir);
    let first = explorer_i18n::tr!("{0} - Copy{1}", stem, ext);
    if !exists(&first) {
        return first;
    }
    (2..)
        .map(|n| explorer_i18n::tr!("{0} - Copy ({1}){2}", stem, n, ext))
        .find(|c| !exists(c))
        .expect("infinite iterator")
}

/// "Keep both" name on conflicts: "Bericht (2).docx", "Bericht (3).docx", …
pub fn unique_name(name: &str, is_dir: bool, exists: impl Fn(&str) -> bool) -> String {
    if !exists(name) {
        return name.to_string();
    }
    let (stem, ext) = split_ext(name, is_dir);
    (2..)
        .map(|n| format!("{stem} ({n}){ext}"))
        .find(|c| !exists(c))
        .expect("infinite iterator")
}

/// Name of a new folder in the current language ("New folder", German "Neuer Ordner").
pub fn new_folder_name() -> &'static str {
    explorer_i18n::tr!("New folder")
}

/// Byte length of the part the rename box pre-selects (name without extension), like Explorer.
pub fn rename_selection_len(name: &str, is_dir: bool) -> usize {
    split_ext(name, is_dir).0.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_rules() {
        assert!(validate_name_for("Bericht 2024.docx", true).is_ok());
        assert!(validate_name_for("Ärger – ok (1).txt", true).is_ok());
        for bad in [
            "", "   ", ".", "..", "a/b", "a\\b", "a:b", "a*b", "a?b", "a\"b", "a<b", "a>b", "a|b",
            "x\u{7}", "name.", "name ",
        ] {
            assert!(
                validate_name_for(bad, true).is_err(),
                "should reject {bad:?}"
            );
        }
        for reserved in ["CON", "con", "Nul.txt", "COM1", "lpt9.log", "AUX .txt"] {
            assert!(
                validate_name_for(reserved, true).is_err(),
                "reserved {reserved:?}"
            );
        }
        assert!(validate_name_for("CONSOLE.txt", true).is_ok());
        assert!(validate_name_for(&"x".repeat(256), true).is_err());
    }

    #[test]
    fn unix_rules_are_lenient() {
        assert!(validate_name_for("a:b*?", false).is_ok());
        assert!(validate_name_for("CON", false).is_ok());
        assert!(validate_name_for("a/b", false).is_err());
        assert!(validate_name_for("..", false).is_err());
    }

    #[test]
    fn copy_names() {
        let taken = ["Bericht - Copy.docx", "Bericht - Copy (2).docx"];
        assert_eq!(
            copy_name("Bericht.docx", false, |_| false),
            "Bericht - Copy.docx"
        );
        assert_eq!(
            copy_name("Bericht.docx", false, |n| taken.contains(&n)),
            "Bericht - Copy (3).docx"
        );
        assert_eq!(
            copy_name("Projekt.v2", true, |_| false),
            "Projekt.v2 - Copy"
        );
        assert_eq!(
            copy_name(".gitignore", false, |_| false),
            ".gitignore - Copy"
        );
        assert_eq!(
            copy_name("archiv.tar.gz", false, |_| false),
            "archiv.tar - Copy.gz"
        );
    }

    #[test]
    fn unique_names() {
        assert_eq!(unique_name("a.txt", false, |_| false), "a.txt");
        assert_eq!(unique_name("a.txt", false, |n| n == "a.txt"), "a (2).txt");
        assert_eq!(
            unique_name("a.txt", false, |n| n == "a.txt" || n == "a (2).txt"),
            "a (3).txt"
        );
        assert_eq!(
            unique_name(new_folder_name(), true, |n| n == new_folder_name()),
            "New folder (2)"
        );
    }

    #[test]
    fn rename_selection() {
        assert_eq!(rename_selection_len("Bericht.docx", false), "Bericht".len());
        assert_eq!(rename_selection_len("Ärger.txt", false), "Ärger".len()); // bytes, not chars
        assert_eq!(rename_selection_len("Ordner.v2", true), "Ordner.v2".len());
        assert_eq!(rename_selection_len(".bashrc", false), ".bashrc".len());
    }
}
