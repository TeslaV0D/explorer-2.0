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

    /// Name shown in the tab, breadcrumb and sidebar.
    pub fn label(self) -> &'static str {
        match self {
            VirtualPlace::Trash => "Papierkorb",
            VirtualPlace::ThisPc => "Dieser PC",
            VirtualPlace::Libraries => "Bibliotheken",
        }
    }

    /// Address bar input → place: the German/English names and the Windows
    /// shell aliases Explorer understands (case-insensitive).
    pub fn from_input(s: &str) -> Option<VirtualPlace> {
        let t = s.trim().trim_end_matches(['\\', '/']).to_lowercase();
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
            VirtualPlace::Trash => ("Gelöscht am", "Ursprünglicher Ort"),
            VirtualPlace::ThisPc => ("", "Speicherplatz"),
            VirtualPlace::Libraries => ("", "Ordner"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
