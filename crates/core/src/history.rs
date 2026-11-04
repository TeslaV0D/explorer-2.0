//! Per-tab back/forward history (Alt+←/→, mouse side buttons).

use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct History {
    back: Vec<PathBuf>,
    forward: Vec<PathBuf>,
}

/// Enough for real use, bounded so a long session can't grow without limit.
pub const MAX_ENTRIES: usize = 100;

impl History {
    /// A new location is visited from `current`: `current` goes to the back
    /// stack, the forward stack is discarded (browser semantics).
    pub fn visit(&mut self, current: PathBuf) {
        if self.back.last() != Some(&current) {
            self.back.push(current);
            if self.back.len() > MAX_ENTRIES {
                self.back.remove(0);
            }
        }
        self.forward.clear();
    }

    pub fn can_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    /// Returns the target to go back to; `current` moves to the forward stack.
    /// Visited places, most recent first (for "recent folders").
    pub fn recent(&self) -> impl Iterator<Item = &PathBuf> {
        self.back.iter().rev().chain(self.forward.iter().rev())
    }

    pub fn back(&mut self, current: PathBuf) -> Option<PathBuf> {
        let target = self.back.pop()?;
        self.forward.push(current);
        Some(target)
    }

    pub fn forward(&mut self, current: PathBuf) -> Option<PathBuf> {
        let target = self.forward.pop()?;
        self.back.push(current);
        Some(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn browser_semantics() {
        let mut h = History::default();
        assert!(!h.can_back() && !h.can_forward());
        h.visit(p("/a")); // a -> b
        h.visit(p("/b")); // b -> c
        assert_eq!(h.back(p("/c")), Some(p("/b")));
        assert_eq!(h.back(p("/b")), Some(p("/a")));
        assert_eq!(h.back(p("/a")), None);
        assert!(h.can_forward());
        assert_eq!(h.forward(p("/a")), Some(p("/b")));
        // a new visit drops the forward stack
        h.visit(p("/b"));
        assert!(!h.can_forward());
        assert_eq!(h.back(p("/x")), Some(p("/b")));
    }

    #[test]
    fn no_duplicate_consecutive_entries_and_bounded() {
        let mut h = History::default();
        h.visit(p("/a"));
        h.visit(p("/a"));
        assert_eq!(h.back(p("/b")), Some(p("/a")));
        assert!(!h.can_back());
        for i in 0..(MAX_ENTRIES + 50) {
            h.visit(PathBuf::from(format!("/{i}")));
        }
        let mut n = 0;
        let mut cur = p("/end");
        while let Some(t) = h.back(cur.clone()) {
            cur = t;
            n += 1;
        }
        assert_eq!(n, MAX_ENTRIES);
        assert_eq!(cur, PathBuf::from(format!("/{}", 50)));
    }
}
