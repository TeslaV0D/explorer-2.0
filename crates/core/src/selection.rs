//! Explorer-style selection model (pure, index based).
//!
//! - click: select only i, anchor = focus = i
//! - ctrl+click: toggle i, anchor = focus = i
//! - shift+click / shift+arrows: range anchor..=i (replaces selection)
//! - ctrl+arrows: move focus only (selection unchanged)

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    selected: Vec<bool>,
    count: usize,
    anchor: Option<usize>,
    focus: Option<usize>,
}

impl Selection {
    pub fn new(len: usize) -> Self {
        Self {
            selected: vec![false; len],
            ..Default::default()
        }
    }

    pub fn len(&self) -> usize {
        self.selected.len()
    }

    pub fn is_empty(&self) -> bool {
        self.selected.is_empty()
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn focus(&self) -> Option<usize> {
        self.focus
    }

    pub fn is_selected(&self, i: usize) -> bool {
        self.selected.get(i).copied().unwrap_or(false)
    }

    pub fn selected_indices(&self) -> Vec<usize> {
        self.selected
            .iter()
            .enumerate()
            .filter(|(_, s)| **s)
            .map(|(i, _)| i)
            .collect()
    }

    fn set(&mut self, i: usize, v: bool) {
        if let Some(s) = self.selected.get_mut(i) {
            if *s != v {
                *s = v;
                if v {
                    self.count += 1;
                } else {
                    self.count -= 1;
                }
            }
        }
    }

    /// Rubber-band selection: exactly `base` (kept with Ctrl, else empty) plus
    /// the items inside the band; focus/anchor go to the last item hit.
    pub fn set_band(&mut self, base: &[usize], hits: &[usize]) {
        self.clear();
        for &i in base.iter().chain(hits) {
            self.set(i, true);
        }
        if let Some(&last) = hits.last() {
            self.focus = Some(last);
            self.anchor = Some(last);
        }
    }

    pub fn clear(&mut self) {
        self.selected.iter_mut().for_each(|s| *s = false);
        self.count = 0;
    }

    pub fn click(&mut self, i: usize) {
        if i >= self.len() {
            return;
        }
        self.clear();
        self.set(i, true);
        self.anchor = Some(i);
        self.focus = Some(i);
    }

    pub fn ctrl_click(&mut self, i: usize) {
        if i >= self.len() {
            return;
        }
        let v = !self.is_selected(i);
        self.set(i, v);
        self.anchor = Some(i);
        self.focus = Some(i);
    }

    pub fn shift_click(&mut self, i: usize) {
        if i >= self.len() {
            return;
        }
        let a = self.anchor.unwrap_or(i);
        self.clear();
        for k in a.min(i)..=a.max(i) {
            self.set(k, true);
        }
        self.anchor = Some(a);
        self.focus = Some(i);
    }

    pub fn select_all(&mut self) {
        let n = self.len();
        self.selected.iter_mut().for_each(|s| *s = true);
        self.count = n;
        if self.focus.is_none() && n > 0 {
            self.focus = Some(0);
            self.anchor = Some(0);
        }
    }

    /// Keyboard move to an absolute target (clamped). `extend` = Shift, `focus_only` = Ctrl.
    pub fn move_to(&mut self, target: isize, extend: bool, focus_only: bool) {
        if self.is_empty() {
            return;
        }
        let i = target.clamp(0, self.len() as isize - 1) as usize;
        if focus_only {
            self.focus = Some(i);
        } else if extend {
            self.shift_click(i);
        } else {
            self.click(i);
        }
    }

    /// Relative keyboard move (arrows, PageUp/Down). No focus yet → first/last item.
    pub fn move_by(&mut self, delta: isize, extend: bool, focus_only: bool) {
        let target = match self.focus {
            Some(f) => f as isize + delta,
            None if delta >= 0 => 0,
            None => self.len() as isize - 1,
        };
        self.move_to(target, extend, focus_only);
    }

    /// Rebuilds the selection for a new list (e.g. after auto-refresh), keeping
    /// items that still exist. `map` returns the new index of an old index.
    pub fn remap(&self, new_len: usize, map: impl Fn(usize) -> Option<usize>) -> Selection {
        let mut s = Selection::new(new_len);
        for i in self.selected_indices() {
            if let Some(j) = map(i) {
                s.set(j, true);
            }
        }
        s.focus = self.focus.and_then(&map);
        s.anchor = self.anchor.and_then(&map).or(s.focus);
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rubber_band_with_and_without_base() {
        let mut s = Selection::new(10);
        s.click(0);
        s.set_band(&[], &[3, 4]);
        assert_eq!(s.selected_indices(), vec![3, 4]);
        assert_eq!((s.focus(), s.count()), (Some(4), 2));
        s.set_band(&[0, 4], &[6]);
        assert_eq!(
            s.selected_indices(),
            vec![0, 4, 6],
            "Ctrl keeps the earlier selection"
        );
        s.set_band(&[], &[]);
        assert_eq!(s.count(), 0);
    }

    #[test]
    fn click_ctrl_shift() {
        let mut s = Selection::new(10);
        s.click(2);
        assert_eq!(s.selected_indices(), [2]);
        s.ctrl_click(5);
        assert_eq!(s.selected_indices(), [2, 5]);
        s.ctrl_click(2);
        assert_eq!(s.selected_indices(), [5]);
        // Like the Windows ListView, ctrl+click moves the anchor even when it deselects.
        s.shift_click(8); // anchor = 2
        assert_eq!(s.selected_indices(), [2, 3, 4, 5, 6, 7, 8]);
        s.click(5);
        s.shift_click(8); // anchor = 5
        assert_eq!(s.selected_indices(), [5, 6, 7, 8]);
        s.shift_click(3); // range flips around anchor
        assert_eq!(s.selected_indices(), [3, 4, 5]);
        assert_eq!(s.count(), 3);
        assert_eq!(s.focus(), Some(3));
    }

    #[test]
    fn keyboard_moves_clamp_and_extend() {
        let mut s = Selection::new(5);
        s.move_by(1, false, false); // no focus -> first
        assert_eq!(s.selected_indices(), [0]);
        s.move_by(1, true, false);
        s.move_by(1, true, false);
        assert_eq!(s.selected_indices(), [0, 1, 2]);
        s.move_by(100, false, false); // PageDown past end -> clamp
        assert_eq!(s.selected_indices(), [4]);
        s.move_by(-1, false, true); // ctrl: focus only
        assert_eq!(s.selected_indices(), [4]);
        assert_eq!(s.focus(), Some(3));
        s.move_to(0, true, false); // shift+Home from anchor 4
        assert_eq!(s.selected_indices(), [0, 1, 2, 3, 4]);
        let mut e = Selection::new(0);
        e.move_by(1, false, false);
        e.click(0);
        assert_eq!(e.count(), 0);
    }

    #[test]
    fn select_all_and_out_of_range() {
        let mut s = Selection::new(3);
        s.select_all();
        assert_eq!(s.count(), 3);
        s.click(99);
        assert_eq!(s.count(), 3);
        s.clear();
        assert_eq!(s.count(), 0);
    }

    #[test]
    fn remap_keeps_surviving_items() {
        let mut s = Selection::new(4);
        s.click(1);
        s.ctrl_click(3);
        // item 1 deleted, others shift down by one
        let r = s.remap(3, |i| match i {
            0 => Some(0),
            1 => None,
            i => Some(i - 1),
        });
        assert_eq!(r.selected_indices(), [2]);
        assert_eq!(r.focus(), Some(2));
    }
}
