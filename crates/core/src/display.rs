//! How names are shown: Explorer's "Erweiterungen bei bekannten Dateitypen
//! ausblenden". Pure → tested; whether a type is known comes from the caller
//! (Windows: registered in HKCR).

/// Extension Explorer would hide: files only, after the last dot, not for
/// leading-dot names (".gitignore") or names ending in a dot.
pub fn hideable_ext(name: &str, is_dir: bool) -> Option<&str> {
    if is_dir {
        return None;
    }
    match name.rfind('.') {
        Some(i) if i > 0 && i + 1 < name.len() => Some(&name[i + 1..]),
        _ => None,
    }
}

/// Name as shown in the list.
pub fn display_name(
    name: &str,
    is_dir: bool,
    show_extensions: bool,
    mut known: impl FnMut(&str) -> bool,
) -> &str {
    if show_extensions {
        return name;
    }
    match hideable_ext(name, is_dir) {
        Some(ext) if known(ext) => &name[..name.len() - ext.len() - 1],
        _ => name,
    }
}

/// The extension hidden in the list for `name` shown as `shown` (None if the
/// full name is shown).
pub fn hidden_ext<'a>(name: &'a str, shown: &str) -> Option<&'a str> {
    name.strip_prefix(shown)?
        .strip_prefix('.')
        .filter(|e| !e.is_empty())
}

/// Final name after inline rename: with a hidden extension the user only
/// edits the stem and the extension is kept (typing "x.txt" for "a" = a.txt
/// gives "x.txt.txt", exactly like Explorer).
pub fn rename_result(typed: &str, hidden_ext: Option<&str>) -> String {
    match hidden_ext {
        Some(ext) => format!("{typed}.{ext}"),
        None => typed.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_like_explorer() {
        let all = |_: &str| true;
        let none = |_: &str| false;
        assert_eq!(display_name("Bericht.docx", false, false, all), "Bericht");
        assert_eq!(
            display_name("Bericht.docx", false, true, all),
            "Bericht.docx"
        );
        assert_eq!(
            display_name("archiv.tar.gz", false, false, all),
            "archiv.tar"
        );
        assert_eq!(
            display_name("x.unbekannt", false, false, none),
            "x.unbekannt",
            "unknown types keep it"
        );
        assert_eq!(display_name(".gitignore", false, false, all), ".gitignore");
        assert_eq!(display_name("ende.", false, false, all), "ende.");
        assert_eq!(
            display_name("Ordner.alt", true, false, all),
            "Ordner.alt",
            "folders never"
        );
    }

    #[test]
    fn rename_keeps_a_hidden_extension() {
        assert_eq!(hidden_ext("Bericht.docx", "Bericht"), Some("docx"));
        assert_eq!(hidden_ext("Bericht.docx", "Bericht.docx"), None);
        assert_eq!(hidden_ext("a", "a"), None);
        assert_eq!(rename_result("Neu", Some("docx")), "Neu.docx");
        assert_eq!(rename_result("Neu.txt", Some("txt")), "Neu.txt.txt");
        assert_eq!(rename_result("Neu.txt", None), "Neu.txt");
    }
}
