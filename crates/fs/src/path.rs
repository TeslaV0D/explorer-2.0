//! Pure path helpers (platform-neutral so they are unit-tested on every OS).

/// Converts an absolute Windows path to its verbatim form (`\\?\C:\…`,
/// `\\?\UNC\server\share\…`) so Win32 APIs accept paths longer than 260 chars.
/// Verbatim paths skip normalization, so `.`/`..` and `/` are resolved here.
/// Relative paths are returned unchanged (callers absolutize first).
pub fn to_verbatim(path: &str) -> String {
    if path.starts_with(r"\\?\") || path.starts_with(r"\\.\") {
        return path.to_string();
    }
    let p = path.replace('/', "\\");
    let (prefix, rest) = if let Some(unc) = p.strip_prefix(r"\\") {
        // \\server\share\rest  -> keep server+share as root
        let mut parts = unc.splitn(3, '\\');
        let server = parts.next().unwrap_or_default();
        let share = parts.next().unwrap_or_default();
        if server.is_empty() || share.is_empty() {
            return path.to_string();
        }
        (
            format!(r"\\?\UNC\{server}\{share}"),
            parts.next().unwrap_or_default().to_string(),
        )
    } else if p.len() >= 2 && p.as_bytes()[1] == b':' && p.as_bytes()[0].is_ascii_alphabetic() {
        let rest = &p[2..];
        if !rest.is_empty() && !rest.starts_with('\\') {
            return path.to_string(); // drive-relative ("C:foo") – not absolute
        }
        (
            format!(r"\\?\{}:", p[..1].to_ascii_uppercase()),
            rest.to_string(),
        )
    } else {
        return path.to_string();
    };
    let mut comps: Vec<&str> = Vec::new();
    for c in rest.split('\\') {
        match c {
            "" | "." => {}
            ".." => {
                comps.pop();
            }
            c => comps.push(c),
        }
    }
    let mut out = prefix;
    out.push('\\');
    out.push_str(&comps.join("\\"));
    out
}

/// Windows FILETIME (100 ns ticks since 1601-01-01) to Unix seconds.
pub fn filetime_to_unix(ticks: u64) -> Option<i64> {
    const EPOCH_DIFF: i64 = 116_444_736_000_000_000;
    if ticks == 0 {
        return None;
    }
    Some((ticks as i64 - EPOCH_DIFF).div_euclid(10_000_000))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbatim_drive_paths() {
        assert_eq!(to_verbatim(r"C:\Users\Boss"), r"\\?\C:\Users\Boss");
        assert_eq!(to_verbatim(r"c:\"), r"\\?\C:\");
        assert_eq!(to_verbatim("C:/a/b/../c/./d/"), r"\\?\C:\a\c\d");
        assert_eq!(to_verbatim(r"C:\..\.."), r"\\?\C:\");
    }

    #[test]
    fn verbatim_unc_paths() {
        assert_eq!(to_verbatim(r"\\nas\share\dir"), r"\\?\UNC\nas\share\dir");
        assert_eq!(to_verbatim(r"\\nas\share"), r"\\?\UNC\nas\share\");
    }

    #[test]
    fn verbatim_passthrough() {
        assert_eq!(to_verbatim(r"\\?\C:\x"), r"\\?\C:\x");
        assert_eq!(to_verbatim(r"\\.\pipe\x"), r"\\.\pipe\x");
        assert_eq!(to_verbatim("relative\\x"), "relative\\x");
        assert_eq!(to_verbatim("C:foo"), "C:foo");
        assert_eq!(to_verbatim(r"\\server"), r"\\server");
    }

    #[test]
    fn filetime_conversion() {
        assert_eq!(filetime_to_unix(0), None);
        assert_eq!(filetime_to_unix(116_444_736_000_000_000), Some(0));
        assert_eq!(
            filetime_to_unix(116_444_736_000_000_000 + 10_000_000 * 86_400),
            Some(86_400)
        );
        assert_eq!(filetime_to_unix(116_444_735_990_000_000), Some(-1));
    }
}
