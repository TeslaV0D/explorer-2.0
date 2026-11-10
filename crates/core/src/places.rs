//! Virtual places that are not folders on disk: the recycle bin and
//! "Dieser PC". They travel through tabs, history and the session as
//! placeholder paths (`::papierkorb`), which can never collide with a real
//! path (`:` is not allowed in Windows names, and the form is not absolute).

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VirtualPlace {
    Trash,
    ThisPc,
    Libraries,
}

const TRASH: &str = "::papierkorb";
const THIS_PC: &str = "::dieser-pc";
const LIBRARIES: &str = "::bibliotheken";
/// Prefix of one library: `::bibliothek:Documents` (stem of its .library-ms).
const LIBRARY: &str = "::bibliothek:";

/// Prefix of a search: `::suche:<scope>\u{1f}<query>` (S1.4b).
const SEARCH: &str = "::suche:";
const SEARCH_SEP: char = '\u{1f}';

/// Placeholder of a search for `query` below `scope` (None: every indexed
/// drive). Lives in the tab's history like a folder: Back returns to it.
pub fn search_path(scope: Option<&Path>, query: &str) -> PathBuf {
    let scope = scope
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    // the separator and line breaks never come from typing; keep the path one line
    let query: String = query
        .chars()
        .map(|c| {
            if c == SEARCH_SEP || c == '\r' || c == '\n' {
                ' '
            } else {
                c
            }
        })
        .collect();
    PathBuf::from(format!("{SEARCH}{scope}{SEARCH_SEP}{query}"))
}

/// `::suche:…` → (scope, query).
pub fn search_of(p: &Path) -> Option<(Option<PathBuf>, String)> {
    let rest = p.to_str()?.strip_prefix(SEARCH)?;
    let (scope, query) = rest.split_once(SEARCH_SEP)?;
    let scope = (!scope.is_empty()).then(|| PathBuf::from(scope));
    Some((scope, query.to_string()))
}

/// Any placeholder path (virtual place or library) – file operations must
/// never act on these.
pub fn is_placeholder(p: &Path) -> bool {
    p.to_str().is_some_and(|s| s.starts_with("::"))
}

/// Placeholder path of one library.
pub fn library_path(stem: &str) -> PathBuf {
    PathBuf::from(format!("{LIBRARY}{stem}"))
}

/// `::bibliothek:Documents` → `Some("Documents")`.
pub fn library_of(p: &Path) -> Option<&str> {
    p.to_str()?.strip_prefix(LIBRARY).filter(|s| !s.is_empty())
}

impl VirtualPlace {
    pub const ALL: [VirtualPlace; 3] = [
        VirtualPlace::Trash,
        VirtualPlace::ThisPc,
        VirtualPlace::Libraries,
    ];

    /// Placeholder path stored in tabs, history and session.ini.
    pub fn path(self) -> PathBuf {
        PathBuf::from(match self {
            VirtualPlace::Trash => TRASH,
            VirtualPlace::ThisPc => THIS_PC,
            VirtualPlace::Libraries => LIBRARIES,
        })
    }

    pub fn from_path(p: &Path) -> Option<VirtualPlace> {
        match p.to_str()? {
            TRASH => Some(VirtualPlace::Trash),
            THIS_PC => Some(VirtualPlace::ThisPc),
            LIBRARIES => Some(VirtualPlace::Libraries),
            _ => None,
        }
    }

    /// Name shown in the tab, breadcrumb and sidebar (current language).
    pub fn label(self) -> &'static str {
        use explorer_i18n::tr;
        match self {
            VirtualPlace::Trash => tr!("Recycle Bin"),
            VirtualPlace::ThisPc => tr!("This PC"),
            VirtualPlace::Libraries => tr!("Libraries"),
        }
    }

    /// Address bar input → place: the name in the current language, the
    /// German/English names and the Windows shell aliases Explorer understands
    /// (case-insensitive).
    pub fn from_input(s: &str) -> Option<VirtualPlace> {
        let t = s.trim().trim_end_matches(['\\', '/']).to_lowercase();
        if let Some(v) = [
            VirtualPlace::Trash,
            VirtualPlace::ThisPc,
            VirtualPlace::Libraries,
        ]
        .into_iter()
        .find(|v| v.label().to_lowercase() == t)
        {
            return Some(v);
        }
        match t.as_str() {
            "::papierkorb"
            | "papierkorb"
            | "recycle bin"
            | "shell:recyclebinfolder"
            | "shell:::{645ff040-5081-101b-9f08-00aa002f954e}" => Some(VirtualPlace::Trash),
            "::dieser-pc"
            | "dieser pc"
            | "this pc"
            | "computer"
            | "shell:mycomputerfolder"
            | "shell:::{20d04fe0-3aea-1069-a2d8-08002b30309d}" => Some(VirtualPlace::ThisPc),
            "::bibliotheken"
            | "bibliotheken"
            | "libraries"
            | "shell:libraries"
            | "shell:::{031e4825-7b94-4dc3-b131-e946b44c8dd5}" => Some(VirtualPlace::Libraries),
            _ => None,
        }
    }

    /// Column headers in this place: (date column, type column).
    pub fn columns(self) -> (&'static str, &'static str) {
        match self {
            VirtualPlace::Trash => (
                explorer_i18n::tr!("Date deleted"),
                explorer_i18n::tr!("Original location"),
            ),
            VirtualPlace::ThisPc => ("", explorer_i18n::tr!("Storage")),
            VirtualPlace::Libraries => ("", explorer_i18n::tr!("Folder")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_paths_round_trip() {
        let p = search_path(Some(Path::new(r"C:\Users\Boss")), "ext:pdf bericht");
        assert!(is_placeholder(&p));
        assert_eq!(VirtualPlace::from_path(&p), None);
        assert_eq!(library_of(&p), None);
        assert_eq!(
            search_of(&p),
            Some((
                Some(PathBuf::from(r"C:\Users\Boss")),
                "ext:pdf bericht".into()
            ))
        );
        let all = search_path(None, "a\u{1f}b\nc");
        assert_eq!(search_of(&all), Some((None, "a b c".into())));
        assert_eq!(search_of(Path::new("::suche:")), None);
        assert_eq!(search_of(Path::new("/home/a")), None);
        // a scope with the prefix text inside is still split at the separator
        let odd = search_path(Some(Path::new("/x/::suche:y")), "q");
        assert_eq!(
            search_of(&odd),
            Some((Some(PathBuf::from("/x/::suche:y")), "q".into()))
        );
    }

    #[test]
    fn placeholder_paths_round_trip() {
        for p in VirtualPlace::ALL {
            assert_eq!(VirtualPlace::from_path(&p.path()), Some(p));
            assert!(!p.path().is_absolute(), "never a real absolute path");
        }
        assert_eq!(VirtualPlace::from_path(Path::new("/tmp")), None);
        assert_eq!(
            VirtualPlace::from_path(Path::new("papierkorb")),
            None,
            "a folder named like it"
        );
    }

    #[test]
    fn library_placeholders() {
        let p = library_path("Documents");
        assert_eq!(library_of(&p), Some("Documents"));
        assert!(is_placeholder(&p) && !p.is_absolute());
        assert_eq!(library_of(Path::new("::bibliothek:")), None);
        assert_eq!(library_of(&VirtualPlace::Libraries.path()), None);
        assert!(is_placeholder(&VirtualPlace::Trash.path()));
        assert!(!is_placeholder(Path::new("/x")) && !is_placeholder(Path::new(r"C:\x")));
    }

    #[test]
    fn address_bar_aliases() {
        assert_eq!(
            VirtualPlace::from_input("Papierkorb"),
            Some(VirtualPlace::Trash)
        );
        assert_eq!(
            VirtualPlace::from_input(" shell:RecycleBinFolder "),
            Some(VirtualPlace::Trash)
        );
        assert_eq!(
            VirtualPlace::from_input("shell:::{645FF040-5081-101B-9F08-00AA002F954E}"),
            Some(VirtualPlace::Trash)
        );
        assert_eq!(
            VirtualPlace::from_input("Dieser PC"),
            Some(VirtualPlace::ThisPc)
        );
        assert_eq!(
            VirtualPlace::from_input("shell:MyComputerFolder\\"),
            Some(VirtualPlace::ThisPc)
        );
        assert_eq!(
            VirtualPlace::from_input("shell:Libraries"),
            Some(VirtualPlace::Libraries)
        );
        assert_eq!(
            VirtualPlace::from_input("Bibliotheken"),
            Some(VirtualPlace::Libraries)
        );
        assert_eq!(VirtualPlace::from_input("C:\\Papierkorb"), None);
        assert_eq!(VirtualPlace::from_input("/home"), None);
    }
}
