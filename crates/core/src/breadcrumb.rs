//! Address-bar breadcrumb segments from a path string (Windows + Unix forms).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crumb {
    pub label: String,
    /// Full path this segment navigates to.
    pub path: String,
}

/// Splits a path into clickable segments: `C:\Users\Boss` → `C:\`, `Users`, `Boss`;
/// `\\nas\share\x` → `\\nas`, `share`, `x`; `/home/u` → `/`, `home`, `u`.
pub fn crumbs(path: &str) -> Vec<Crumb> {
    let p = path
        .strip_prefix(r"\\?\UNC\")
        .map(|r| format!(r"\\{r}"))
        .unwrap_or_else(|| path.strip_prefix(r"\\?\").unwrap_or(path).to_string());
    // A drive (`C:/Users`) or a UNC path (`//nas/share`) may use either
    // separator. Split on one of them, or the rest stays one segment and
    // the accumulated path mixes `\` and `/`.
    let windows =
        p.contains('\\') || p.starts_with("//") || (p.len() >= 2 && p.as_bytes()[1] == b':');
    let p = if windows { p.replace('/', "\\") } else { p };
    let sep = if windows { '\\' } else { '/' };
    let (root, rest): (String, &str) = if let Some(unc) = p.strip_prefix(r"\\") {
        // the server is its own segment (it lists its shares), like Explorer
        let server = unc.split('\\').next().unwrap_or("");
        (
            format!(r"\\{server}"),
            &p[(2 + server.len()).min(p.len())..],
        )
    } else if windows && p.len() >= 2 && p.as_bytes()[1] == b':' {
        (format!(r"{}\", p[..2].to_ascii_uppercase()), &p[2..])
    } else if let Some(r) = p.strip_prefix('/') {
        ("/".to_string(), r)
    } else {
        (String::new(), p.as_str())
    };
    let mut out = Vec::new();
    let mut acc = root.clone();
    if !root.is_empty() {
        out.push(Crumb {
            label: root.clone(),
            path: root.clone(),
        });
    }
    for part in rest.split(sep).filter(|s| !s.is_empty()) {
        if !acc.is_empty() && !acc.ends_with(sep) {
            acc.push(sep);
        }
        acc.push_str(part);
        out.push(Crumb {
            label: part.to_string(),
            path: acc.clone(),
        });
    }
    out
}

/// Keeps the last segments that fit into `max_chars` (labels + separators);
/// the root is replaced by "…" pointing at the first hidden parent.
pub fn fit(crumbs: Vec<Crumb>, max_chars: usize) -> Vec<Crumb> {
    let total: usize = crumbs.iter().map(|c| c.label.chars().count() + 3).sum();
    if total <= max_chars || crumbs.len() <= 1 {
        return crumbs;
    }
    let mut kept = Vec::new();
    let mut used = 4; // "…" + separator
    for c in crumbs.iter().rev() {
        let w = c.label.chars().count() + 3;
        if used + w > max_chars && !kept.is_empty() {
            break;
        }
        used += w;
        kept.push(c.clone());
    }
    kept.reverse();
    let hidden_idx = crumbs.len() - kept.len();
    if hidden_idx == 0 {
        return kept;
    }
    let mut out = vec![Crumb {
        label: "…".into(),
        path: crumbs[hidden_idx - 1].path.clone(),
    }];
    out.extend(kept);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(v: &[Crumb]) -> Vec<&str> {
        v.iter().map(|c| c.label.as_str()).collect()
    }

    #[test]
    fn windows_paths() {
        let c = crumbs(r"C:\Users\Boss\Docs");
        assert_eq!(labels(&c), [r"C:\", "Users", "Boss", "Docs"]);
        assert_eq!(c[2].path, r"C:\Users\Boss");
        assert_eq!(c[0].path, r"C:\");
        assert_eq!(labels(&crumbs(r"d:\")), [r"D:\"]);
        assert_eq!(
            labels(&crumbs(r"\\?\C:\very\long")),
            [r"C:\", "very", "long"]
        );
    }

    #[test]
    fn unc_paths() {
        let c = crumbs(r"\\nas\share\film\2024");
        assert_eq!(labels(&c), [r"\\nas", "share", "film", "2024"]);
        assert_eq!(c[1].path, r"\\nas\share");
        assert_eq!(c[3].path, r"\\nas\share\film\2024");
        assert_eq!(labels(&crumbs(r"\\nas")), [r"\\nas"]);
        assert_eq!(labels(&crumbs(r"\\nas\")), [r"\\nas"]);
        assert_eq!(
            labels(&crumbs(r"\\?\UNC\nas\share\x")),
            [r"\\nas", "share", "x"]
        );
    }

    #[test]
    fn forward_slashes_on_windows_paths() {
        let c = crumbs("C:/Users/Boss");
        assert_eq!(labels(&c), [r"C:\", "Users", "Boss"]);
        assert_eq!(c[0].path, r"C:\");
        assert_eq!(c[1].path, r"C:\Users");
        assert_eq!(c[2].path, r"C:\Users\Boss");
        let u = crumbs("//nas/share/x");
        assert_eq!(labels(&u), [r"\\nas", "share", "x"]);
        assert_eq!(u[1].path, r"\\nas\share");
        assert_eq!(u[2].path, r"\\nas\share\x");
        let mixed = crumbs(r"\\nas/share\x");
        assert_eq!(labels(&mixed), [r"\\nas", "share", "x"]);
    }

    #[test]
    fn unix_paths() {
        let c = crumbs("/home/boss/");
        assert_eq!(labels(&c), ["/", "home", "boss"]);
        assert_eq!(c[2].path, "/home/boss");
        assert_eq!(c[1].path, "/home");
    }

    #[test]
    fn fit_truncates_from_left() {
        let c = crumbs(r"C:\aaaaaaaaaa\bbbbbbbbbb\cccccccccc\dddddddddd");
        let f = fit(c.clone(), 30);
        assert_eq!(f[0].label, "…");
        assert_eq!(labels(&f)[1..], ["cccccccccc", "dddddddddd"]);
        assert_eq!(f[0].path, r"C:\aaaaaaaaaa\bbbbbbbbbb");
        assert_eq!(fit(c.clone(), 1000), c);
        // a single overlong segment is always kept
        let one = fit(crumbs(r"C:\xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"), 10);
        assert_eq!(one.last().unwrap().label.len(), 40);
    }
}
