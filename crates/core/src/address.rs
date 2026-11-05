//! Address bar input like Explorer understands it: `%APPDATA%\x`,
//! `shell:Downloads`, `shell:Common Startup`, and on Unix `~/x`.
//! Pure parsing (tested on every OS); resolving a known folder to a path is
//! done by `explorer_fs::places::known_folder_path`.

/// Folders reachable via `shell:<name>` (Windows canonical names).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnownFolder {
    Desktop,
    Downloads,
    Documents,
    Pictures,
    Music,
    Videos,
    Profile,
    UserProfiles,
    AppData,
    LocalAppData,
    LocalAppDataLow,
    Startup,
    Programs,
    StartMenu,
    SendTo,
    Recent,
    Templates,
    Favorites,
    Links,
    SavedGames,
    Screenshots,
    Fonts,
    Windows,
    System,
    ProgramFiles,
    ProgramFilesX86,
    ProgramData,
    Public,
    CommonDesktop,
    CommonStartup,
    CommonPrograms,
    CommonStartMenu,
}

impl KnownFolder {
    /// `shell:` names (lower case, spaces removed) → folder.
    const NAMES: &'static [(&'static str, KnownFolder)] = &[
        ("desktop", KnownFolder::Desktop),
        ("downloads", KnownFolder::Downloads),
        ("personal", KnownFolder::Documents),
        ("documents", KnownFolder::Documents),
        ("mypictures", KnownFolder::Pictures),
        ("pictures", KnownFolder::Pictures),
        ("mymusic", KnownFolder::Music),
        ("music", KnownFolder::Music),
        ("myvideo", KnownFolder::Videos),
        ("videos", KnownFolder::Videos),
        ("profile", KnownFolder::Profile),
        ("userprofiles", KnownFolder::UserProfiles),
        ("appdata", KnownFolder::AppData),
        ("localappdata", KnownFolder::LocalAppData),
        ("localappdatalow", KnownFolder::LocalAppDataLow),
        ("startup", KnownFolder::Startup),
        ("programs", KnownFolder::Programs),
        ("startmenu", KnownFolder::StartMenu),
        ("sendto", KnownFolder::SendTo),
        ("recent", KnownFolder::Recent),
        ("templates", KnownFolder::Templates),
        ("favorites", KnownFolder::Favorites),
        ("links", KnownFolder::Links),
        ("savedgames", KnownFolder::SavedGames),
        ("screenshots", KnownFolder::Screenshots),
        ("fonts", KnownFolder::Fonts),
        ("windows", KnownFolder::Windows),
        ("system", KnownFolder::System),
        ("programfiles", KnownFolder::ProgramFiles),
        ("programfilesx86", KnownFolder::ProgramFilesX86),
        ("commonappdata", KnownFolder::ProgramData),
        ("public", KnownFolder::Public),
        ("commondesktop", KnownFolder::CommonDesktop),
        ("commonstartup", KnownFolder::CommonStartup),
        ("commonprograms", KnownFolder::CommonPrograms),
        ("commonstartmenu", KnownFolder::CommonStartMenu),
    ];

    pub fn from_name(name: &str) -> Option<KnownFolder> {
        let n: String = name
            .chars()
            .filter(|c| !c.is_whitespace())
            .flat_map(char::to_lowercase)
            .collect();
        Self::NAMES.iter().find(|(k, _)| *k == n).map(|(_, f)| *f)
    }
}

/// `shell:Downloads\Fotos` → `(Downloads, "Fotos")`; `Ok(None)` if the input
/// is no `shell:` address; `Err(name)` for an unknown name. Names handled
/// elsewhere (recycle bin, `shell:::{CLSID}`) are `Ok(None)` too.
pub fn shell_folder(input: &str) -> Result<Option<(KnownFolder, String)>, String> {
    let t = input.trim();
    let Some(rest) = t
        .get(..6)
        .filter(|p| p.eq_ignore_ascii_case("shell:"))
        .map(|_| &t[6..])
    else {
        return Ok(None);
    };
    if rest.starts_with("::") {
        return Ok(None); // CLSID form, see VirtualPlace::from_input
    }
    let (name, sub) = match rest.find(['\\', '/']) {
        Some(i) => (&rest[..i], rest[i + 1..].trim_matches(['\\', '/'])),
        None => (rest, ""),
    };
    match KnownFolder::from_name(name) {
        Some(f) => Ok(Some((f, sub.to_string()))),
        None if crate::places::VirtualPlace::from_input(t).is_some() => Ok(None),
        None => Err(name.to_string()),
    }
}

/// Expands `%NAME%` with `lookup`; unknown names and a lone `%` stay as
/// typed (like the Windows shell).
pub fn expand_env(s: &str, lookup: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) if end > 0 => {
                let name = &after[..end];
                match lookup(name) {
                    Some(v) => out.push_str(&v),
                    None => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            _ => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// `~` / `~/x` → home (Unix shells; Windows has no such convention).
pub fn expand_home(s: &str, home: Option<&str>) -> String {
    match (home, s.strip_prefix('~')) {
        (Some(h), Some(r)) if r.is_empty() || r.starts_with('/') => format!("{h}{r}"),
        _ => s.to_string(),
    }
}

/// A folder handed over on the command line by Windows (folder verbs:
/// `"explorer2.exe" "%1"`). The C runtime's quoting rules turn the drive
/// root `"C:\"` into `C:"` (backslash + quote = literal quote) → undo that,
/// strip surrounding quotes, give a bare drive (`C:`) its root.
pub fn launch_arg(s: &str) -> String {
    let mut t = s.trim();
    t = t.strip_prefix('"').unwrap_or(t);
    let mut out = t.trim_end_matches('"').to_string();
    let b = out.as_bytes();
    if b.len() == 2 && b[1] == b':' && b[0].is_ascii_alphabetic() {
        out.push('\\');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_names() {
        assert_eq!(
            shell_folder("shell:Downloads"),
            Ok(Some((KnownFolder::Downloads, String::new())))
        );
        assert_eq!(
            shell_folder(r" SHELL:common startup\x\ "),
            Ok(Some((KnownFolder::CommonStartup, "x".into())))
        );
        assert_eq!(
            shell_folder("shell:My Pictures/2024/Mai"),
            Ok(Some((KnownFolder::Pictures, "2024/Mai".into())))
        );
        assert_eq!(
            shell_folder("shell:Personal").unwrap().unwrap().0,
            KnownFolder::Documents
        );
        assert_eq!(shell_folder("shell:gibtsnicht"), Err("gibtsnicht".into()));
        assert_eq!(shell_folder(r"C:\shell:x"), Ok(None));
        assert_eq!(
            shell_folder("shell:RecycleBinFolder"),
            Ok(None),
            "virtual place"
        );
        assert_eq!(
            shell_folder("shell:::{645FF040-5081-101B-9F08-00AA002F954E}"),
            Ok(None)
        );
    }

    #[test]
    fn env_expansion() {
        let look = |n: &str| match n.to_ascii_uppercase().as_str() {
            "APPDATA" => Some(r"C:\Users\b\AppData\Roaming".to_string()),
            "X" => Some("1".to_string()),
            _ => None,
        };
        assert_eq!(
            expand_env(r"%AppData%\Explorer2", look),
            r"C:\Users\b\AppData\Roaming\Explorer2"
        );
        assert_eq!(expand_env("%x%%x%", look), "11");
        assert_eq!(expand_env("100% sicher", look), "100% sicher");
        assert_eq!(expand_env("%NOPE%\\a", look), "%NOPE%\\a");
        assert_eq!(expand_env("a%%b", look), "a%%b");
        assert_eq!(expand_env("ende%", look), "ende%");
    }

    #[test]
    fn home_expansion() {
        assert_eq!(expand_home("~", Some("/home/b")), "/home/b");
        assert_eq!(expand_home("~/x", Some("/home/b")), "/home/b/x");
        assert_eq!(expand_home("~other", Some("/home/b")), "~other");
        assert_eq!(expand_home("/a/~", Some("/home/b")), "/a/~");
        assert_eq!(expand_home("~", None), "~");
    }

    #[test]
    fn launch_args_from_windows() {
        assert_eq!(launch_arg(r#"C:""#), r"C:\", "\"C:\\\" after CRT parsing");
        assert_eq!(launch_arg("D:"), r"D:\");
        assert_eq!(launch_arg(r#""C:\Users\a b""#), r"C:\Users\a b");
        assert_eq!(launch_arg(r"C:\Users\a b"), r"C:\Users\a b");
        assert_eq!(launch_arg(r"\\nas\share"), r"\\nas\share");
        assert_eq!(launch_arg("  /home/x  "), "/home/x");
        assert_eq!(launch_arg(""), "");
    }
}
