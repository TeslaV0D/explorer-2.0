//! Windows libraries (Dokumente, Bilder, Musik, Videos, own ones): a library
//! is a named set of folders stored in `…\Libraries\<stem>.library-ms`.
//! Read through `IShellLibrary`; other platforms have none.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Library {
    /// File stem of the `.library-ms` file ("Documents"): stable id.
    pub stem: String,
    /// Localized name ("Dokumente").
    pub display: String,
    /// Member folders (file system ones only), in the library's order.
    pub folders: Vec<PathBuf>,
    /// The `.library-ms` file (its icon is the library icon).
    pub file: PathBuf,
}

/// The library behind a `shell:…Library` name (pure → tested).
pub fn stem_for_shell_name(name: &str) -> Option<&'static str> {
    let n: String = name
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect();
    Some(match n.as_str() {
        "documentslibrary" => "Documents",
        "musiclibrary" => "Music",
        "pictureslibrary" => "Pictures",
        "videoslibrary" => "Videos",
        "cameraroll" | "camerarolllibrary" => "Camera Roll",
        "savedpictureslibrary" => "Saved Pictures",
        _ => return None,
    })
}

/// All libraries of the user (blocking; loader thread).
pub fn libraries() -> Vec<Library> {
    #[cfg(windows)]
    {
        crate::win_lib::libraries()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// One library by stem (blocking; loader thread).
pub fn library(stem: &str) -> Option<Library> {
    #[cfg(windows)]
    {
        crate::win_lib::library(stem)
    }
    #[cfg(not(windows))]
    {
        let _ = stem;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_library_names() {
        assert_eq!(stem_for_shell_name("DocumentsLibrary"), Some("Documents"));
        assert_eq!(stem_for_shell_name("pictures library"), Some("Pictures"));
        assert_eq!(stem_for_shell_name("Downloads"), None);
    }

    #[test]
    fn no_libraries_without_windows() {
        if !cfg!(windows) {
            assert!(libraries().is_empty());
            assert_eq!(library("Documents"), None);
        }
    }
}
