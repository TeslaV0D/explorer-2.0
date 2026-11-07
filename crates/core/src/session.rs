//! Session (open panes/tabs) persisted across restarts. Line format, tolerant:
//! unknown or broken lines are skipped, an unusable file yields `None`.
//!
//! ```text
//! split = true
//! active_pane = 1
//! [pane]
//! active_tab = 0
//! tab = C:\Users\Boss
//! ```

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneSession {
    pub tabs: Vec<String>,
    pub active_tab: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub panes: Vec<PaneSession>,
    pub active_pane: usize,
    pub split: bool,
}

pub const MAX_PANES: usize = 2;
pub const MAX_TABS: usize = 64;

impl Session {
    pub fn parse(text: &str) -> Option<Session> {
        let mut panes: Vec<PaneSession> = Vec::new();
        let (mut split, mut active_pane) = (false, 0usize);
        for line in text.lines() {
            let line = line.trim();
            if line == "[pane]" {
                if panes.len() < MAX_PANES {
                    panes.push(PaneSession {
                        tabs: Vec::new(),
                        active_tab: 0,
                    });
                }
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let (k, v) = (k.trim(), v.trim());
            match (k, panes.last_mut()) {
                ("split", _) => split = v == "true",
                ("active_pane", _) => active_pane = v.parse().unwrap_or(0),
                // a stray `\r` cannot come from `serialize` (it skips such paths)
                ("tab", Some(p))
                    if !v.is_empty() && !v.contains('\r') && p.tabs.len() < MAX_TABS =>
                {
                    p.tabs.push(v.to_string())
                }
                ("active_tab", Some(p)) => p.active_tab = v.parse().unwrap_or(0),
                _ => {}
            }
        }
        panes.retain(|p| !p.tabs.is_empty());
        if panes.is_empty() {
            return None;
        }
        for p in &mut panes {
            p.active_tab = p.active_tab.min(p.tabs.len() - 1);
        }
        let split = split && panes.len() == 2;
        if !split {
            panes.truncate(1);
        }
        let active_pane = active_pane.min(panes.len() - 1);
        Some(Session {
            panes,
            active_pane,
            split,
        })
    }

    pub fn serialize(&self) -> String {
        let mut s = format!(
            "# Explorer 2.0 session\nsplit = {}\nactive_pane = {}\n",
            self.split, self.active_pane
        );
        for p in &self.panes {
            s.push_str(&format!("[pane]\nactive_tab = {}\n", p.active_tab));
            for t in &p.tabs {
                // paths with line breaks cannot be represented → skipped
                if !t.contains(['\n', '\r']) {
                    s.push_str(&format!("tab = {t}\n"));
                }
            }
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let s = Session {
            panes: vec![
                PaneSession {
                    tabs: vec![r"C:\Users\Boss".into(), r"D:\Film = Neu".into()],
                    active_tab: 1,
                },
                PaneSession {
                    tabs: vec!["/home/boss".into()],
                    active_tab: 0,
                },
            ],
            active_pane: 1,
            split: true,
        };
        assert_eq!(Session::parse(&s.serialize()), Some(s));
    }

    #[test]
    fn tolerant_and_clamped() {
        assert_eq!(Session::parse(""), None);
        assert_eq!(Session::parse("garbage\n[pane]\nactive_tab = 9\n"), None); // no tabs
        let s = Session::parse(
            "split = true\nactive_pane = 7\n[pane]\nactive_tab = 9\ntab = /a\ntab = /b\n",
        )
        .unwrap();
        assert_eq!(s.panes[0].active_tab, 1);
        assert!(!s.split, "split needs two panes");
        assert_eq!(s.active_pane, 0);
        // a third pane is ignored
        let s =
            Session::parse("split=true\n[pane]\ntab=/a\n[pane]\ntab=/b\n[pane]\ntab=/c\n").unwrap();
        assert_eq!(s.panes.len(), 2);
        assert!(s.split);
    }

    #[test]
    fn not_split_keeps_only_first_pane() {
        let s = Session::parse("split=false\n[pane]\ntab=/a\n[pane]\ntab=/b\n").unwrap();
        assert_eq!(s.panes.len(), 1);
    }
}
