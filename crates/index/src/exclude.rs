//! What the index builder skips (PLAN-SEARCH.md S1.2).
//!
//! One entry per line or `;`: with a separator it is a **path** (that folder
//! and everything below it: `C:\Windows\WinSxS`, `/proc`), without one a
//! **name** pattern for any file or folder (`node_modules`, `*.tmp`, `~$*`).
//! Windows compares without case.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Exclusions {
    /// Normalised path prefixes (no trailing separator; lower-case on Windows).
    paths: Vec<String>,
    /// Name patterns (`*`, `?`; lower-case on Windows).
    names: Vec<String>,
    case_insensitive: bool,
}

/// Always skipped: system folders nobody searches and that only cost time.
pub fn defaults(windows: bool) -> &'static str {
    if windows {
        "$Recycle.Bin;System Volume Information;$WinREAgent"
    } else {
        "/proc;/sys;/dev;/run;/snap"
    }
}

fn norm(s: &str, ci: bool) -> String {
    let s = s.trim().trim_end_matches(['\\', '/']);
    let s = if ci {
        s.replace('/', "\\")
    } else {
        s.to_string()
    };
    if ci {
        s.to_lowercase()
    } else {
        s
    }
}

impl Exclusions {
    pub fn parse(text: &str, case_insensitive: bool) -> Exclusions {
        let mut ex = Exclusions {
            case_insensitive,
            ..Default::default()
        };
        for item in text.split([';', '\n', '\r']) {
            let item = item.trim().trim_matches('"');
            if item.is_empty() {
                continue;
            }
            if item.contains(['\\', '/']) {
                let p = norm(item, case_insensitive);
                if !p.is_empty() && !ex.paths.contains(&p) {
                    ex.paths.push(p);
                }
            } else {
                let n = if case_insensitive {
                    item.to_lowercase()
                } else {
                    item.to_string()
                };
                if !ex.names.contains(&n) {
                    ex.names.push(n);
                }
            }
        }
        ex
    }

    /// The defaults plus the user's list.
    pub fn with_defaults(user: &str, windows: bool) -> Exclusions {
        Exclusions::parse(&format!("{};{user}", defaults(windows)), windows)
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty() && self.names.is_empty()
    }

    /// Skip `name` at `path` (and, for a folder, everything below it)?
    pub fn excludes(&self, path: &str, name: &str) -> bool {
        if !self.names.is_empty() {
            let n = if self.case_insensitive {
                name.to_lowercase()
            } else {
                name.to_string()
            };
            if self.names.iter().any(|p| glob(p, &n)) {
                return true;
            }
        }
        if !self.paths.is_empty() {
            let p = norm(path, self.case_insensitive);
            let sep = if self.case_insensitive { '\\' } else { '/' };
            if self
                .paths
                .iter()
                .any(|x| p == *x || p.starts_with(x.as_str()) && p[x.len()..].starts_with(sep))
            {
                return true;
            }
        }
        false
    }

    /// One line per entry, for the settings text field.
    pub fn to_text(&self) -> String {
        self.paths
            .iter()
            .chain(&self.names)
            .cloned()
            .collect::<Vec<_>>()
            .join(";")
    }
}

/// `*` / `?` over chars, whole name.
fn glob(p: &str, s: &str) -> bool {
    if !p.contains(['*', '?']) {
        return p == s;
    }
    let (p, s): (Vec<char>, Vec<char>) = (p.chars().collect(), s.chars().collect());
    let (mut pi, mut si, mut star, mut mark) = (0, 0, None, 0);
    while si < s.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == s[si]) {
            pi += 1;
            si += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = si;
            pi += 1;
        } else if let Some(st) = star {
            pi = st + 1;
            mark += 1;
            si = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|&c| c == '*')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_paths_and_names() {
        let ex = Exclusions::with_defaults(r"C:\Windows\WinSxS; node_modules ; *.TMP;~$*", true);
        assert!(ex.excludes(r"C:\Windows\WinSxS", "WinSxS"));
        assert!(ex.excludes(r"c:\windows\winsxs\amd64_x", "amd64_x"));
        assert!(ex.excludes(r"C:\Windows\WinSxS\", "WinSxS"));
        assert!(
            !ex.excludes(r"C:\Windows\WinSxSBackup", "WinSxSBackup"),
            "prefix only at a separator"
        );
        assert!(!ex.excludes(r"C:\Windows", "Windows"));
        assert!(ex.excludes(r"D:\code\app\node_modules", "node_modules"));
        assert!(ex.excludes(r"D:\a\b.tmp", "b.tmp"));
        assert!(ex.excludes(r"D:\a\~$Bericht.docx", "~$Bericht.docx"));
        assert!(
            ex.excludes(r"C:\$Recycle.Bin", "$RECYCLE.BIN"),
            "defaults, any case"
        );
        assert!(ex.excludes(r"E:\System Volume Information", "System Volume Information"));
        assert!(!ex.excludes(r"D:\a\Bericht.docx", "Bericht.docx"));
        // forward slashes typed by the user still mean the same folder
        let ex = Exclusions::parse("c:/temp/", true);
        assert!(ex.excludes(r"C:\Temp\x", "x"));
    }

    #[test]
    fn unix_is_case_sensitive() {
        let ex = Exclusions::with_defaults("/home/a/.cache;target", false);
        assert!(ex.excludes("/proc/1/fd", "fd"));
        assert!(ex.excludes("/home/a/.cache", ".cache"));
        assert!(!ex.excludes("/home/a/.Cache", ".Cache"));
        assert!(ex.excludes("/x/target", "target"));
        assert!(!ex.excludes("/x/Target", "Target"));
        assert!(!ex.excludes("/home/a/.cachex", ".cachex"));
    }

    #[test]
    fn text_roundtrip_and_junk() {
        let ex = Exclusions::parse(r#" "C:\A\";;x*;x*;  ;C:\a"#, true);
        assert_eq!(ex.to_text(), r"c:\a;x*");
        assert_eq!(Exclusions::parse(&ex.to_text(), true), ex);
        assert!(Exclusions::parse(" ; \n ", true).is_empty());
        for junk in ["\0", "\\", "/", "*", "?", "**?*", "ü*ß", "C:"] {
            let ex = Exclusions::parse(junk, true);
            let _ = ex.excludes(r"C:\x\y", "y");
        }
    }
}
