//! Which drives the search index covers (setting `index_roots`).
//!
//! Empty = automatic: the local fixed drives (Linux: the home folder).
//! Otherwise a `;`-separated list. A listed drive that is not there right
//! now (USB stick pulled) is skipped, not an error.

/// A drive (or folder) that can be indexed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub path: String,
    /// "Lokaler Datenträger (C:)", "Netzlaufwerk (Z:)" …
    pub label: String,
    /// Part of the automatic choice (local fixed drives).
    pub auto: bool,
}

fn norm(p: &str, ci: bool) -> String {
    let t = p.trim();
    let t = if t.len() > 1 {
        t.trim_end_matches(['/', '\\'])
    } else {
        t
    };
    // "C:" and "C:\" are the same drive; "/" stays "/"
    let t = if t.is_empty() { p.trim() } else { t };
    if ci {
        t.to_lowercase()
    } else {
        t.to_string()
    }
}

fn listed(setting: &str) -> Vec<&str> {
    setting
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

/// The paths to index now. `ci`: paths compare without case (Windows).
pub fn chosen(setting: &str, candidates: &[Candidate], ci: bool) -> Vec<String> {
    // only an empty setting is "automatic"; ";" is an explicit "none"
    let automatic = setting.trim().is_empty();
    let list = listed(setting);
    candidates
        .iter()
        .filter(|c| {
            if automatic {
                c.auto
            } else {
                list.iter().any(|l| norm(l, ci) == norm(&c.path, ci))
            }
        })
        .map(|c| c.path.clone())
        .collect()
}

/// The setting after switching `path` on/off. Ends as `""` (automatic)
/// when the choice equals the automatic one, so drives added later are
/// picked up again.
pub fn toggle(setting: &str, candidates: &[Candidate], path: &str, on: bool, ci: bool) -> String {
    let mut now = chosen(setting, candidates, ci);
    // listed drives that are absent right now stay in the list
    let absent: Vec<String> = listed(setting)
        .into_iter()
        .filter(|l| !candidates.iter().any(|c| norm(&c.path, ci) == norm(l, ci)))
        .map(String::from)
        .collect();
    now.retain(|p| norm(p, ci) != norm(path, ci));
    if on {
        now.push(path.trim().to_string());
    }
    let auto: Vec<String> = chosen("", candidates, ci);
    let same_as_auto = absent.is_empty()
        && now.len() == auto.len()
        && auto
            .iter()
            .all(|a| now.iter().any(|n| norm(n, ci) == norm(a, ci)));
    if same_as_auto {
        return String::new();
    }
    // keep the order of the candidates (stable setting text)
    let mut out: Vec<String> = candidates
        .iter()
        .filter(|c| now.iter().any(|n| norm(n, ci) == norm(&c.path, ci)))
        .map(|c| c.path.clone())
        .collect();
    out.extend(absent);
    if out.is_empty() {
        // nothing chosen: an explicit empty choice (not "automatic")
        return ";".into();
    }
    out.join(";")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cands() -> Vec<Candidate> {
        let c = |p: &str, auto| Candidate {
            path: p.into(),
            label: p.into(),
            auto,
        };
        vec![
            c(r"C:\", true),
            c(r"D:\", true),
            c(r"E:\", false),
            c(r"Z:\", false),
        ]
    }

    #[test]
    fn automatic_is_the_fixed_drives() {
        assert_eq!(chosen("", &cands(), true), [r"C:\", r"D:\"]);
        assert_eq!(chosen("  ", &cands(), true), [r"C:\", r"D:\"]);
        assert!(chosen(" ; ", &cands(), true).is_empty(), "explicitly none");
    }

    #[test]
    fn switching_drives_on_and_off() {
        let c = cands();
        // network drive Z: added
        let s = toggle("", &c, r"Z:\", true, true);
        assert_eq!(s, r"C:\;D:\;Z:\");
        assert_eq!(chosen(&s, &c, true), [r"C:\", r"D:\", r"Z:\"]);
        // D: off (written in another case / without backslash)
        let s = toggle(&s, &c, "d:", false, true);
        assert_eq!(chosen(&s, &c, true), [r"C:\", r"Z:\"]);
        // back to exactly the automatic choice: automatic again
        let s = toggle(&s, &c, r"Z:\", false, true);
        let s = toggle(&s, &c, r"D:\", true, true);
        assert_eq!(s, "");
        // everything off: explicitly nothing (not automatic)
        let s = toggle("", &c, r"C:\", false, true);
        let s = toggle(&s, &c, r"D:\", false, true);
        assert_eq!(s, ";");
        assert!(chosen(&s, &c, true).is_empty());
    }

    #[test]
    fn absent_drives_are_kept_but_not_indexed() {
        let c = cands();
        // F: (USB stick) listed but not plugged in
        let s = r"C:\;F:\";
        assert_eq!(chosen(s, &c, true), [r"C:\"]);
        let s = toggle(s, &c, r"D:\", true, true);
        assert_eq!(s, r"C:\;D:\;F:\");
    }

    #[test]
    fn unix_paths_keep_their_case() {
        let c = vec![
            Candidate {
                path: "/home/boss".into(),
                label: "Persönlicher Ordner".into(),
                auto: true,
            },
            Candidate {
                path: "/".into(),
                label: "Dateisystem".into(),
                auto: false,
            },
        ];
        assert_eq!(chosen("", &c, false), ["/home/boss"]);
        assert_eq!(chosen("/HOME/boss", &c, false), Vec::<String>::new());
        let s = toggle("", &c, "/", true, false);
        assert_eq!(s, "/home/boss;/");
        assert_eq!(chosen(&s, &c, false), ["/home/boss", "/"]);
    }
}
