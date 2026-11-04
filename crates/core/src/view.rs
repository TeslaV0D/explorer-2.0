//! View modes of the file list: details (rows) or icons/tiles in three
//! sizes, like Explorer's "Details", "Mittelgroße/Große/Extra große Symbole".

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Details,
    Medium,
    Large,
    ExtraLarge,
}

impl ViewMode {
    /// Order for Ctrl+mouse wheel (up = bigger).
    const ORDER: [ViewMode; 4] = [
        ViewMode::Details,
        ViewMode::Medium,
        ViewMode::Large,
        ViewMode::ExtraLarge,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ViewMode::Details => "details",
            ViewMode::Medium => "medium",
            ViewMode::Large => "large",
            ViewMode::ExtraLarge => "xlarge",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ORDER.into_iter().find(|m| m.as_str() == s)
    }

    pub fn label_de(self) -> &'static str {
        match self {
            ViewMode::Details => "Details",
            ViewMode::Medium => "Mittelgroße Symbole",
            ViewMode::Large => "Große Symbole",
            ViewMode::ExtraLarge => "Extra große Symbole",
        }
    }

    pub fn is_grid(self) -> bool {
        self != ViewMode::Details
    }

    /// Edge of the image in a tile (logical px); 16 in details.
    pub fn image_px(self) -> u32 {
        match self {
            ViewMode::Details => 16,
            ViewMode::Medium => 48,
            ViewMode::Large => 96,
            ViewMode::ExtraLarge => 256,
        }
    }

    /// Tile size (width, height) in logical px (image + two text lines).
    pub fn tile(self) -> (f32, f32) {
        match self {
            ViewMode::Details => (0.0, 24.0),
            ViewMode::Medium => (92.0, 92.0),
            ViewMode::Large => (128.0, 144.0),
            ViewMode::ExtraLarge => (276.0, 304.0),
        }
    }

    /// Ctrl+wheel: one step bigger (`up`) or smaller, clamped at the ends.
    pub fn step(self, up: bool) -> Self {
        let i = Self::ORDER.iter().position(|m| *m == self).unwrap_or(0);
        let j = if up {
            (i + 1).min(Self::ORDER.len() - 1)
        } else {
            i.saturating_sub(1)
        };
        Self::ORDER[j]
    }

    /// Explorer's Ctrl+Shift+<digit> (1 extra large, 2 large, 3 medium,
    /// 6 details). `c` may be the digit or what Shift turns it into on US/DE
    /// layouts.
    pub fn from_shortcut(c: char) -> Option<Self> {
        match c {
            '1' | '!' => Some(ViewMode::ExtraLarge),
            '2' | '@' | '"' => Some(ViewMode::Large),
            '3' | '#' | '§' => Some(ViewMode::Medium),
            '6' | '^' | '&' => Some(ViewMode::Details),
            _ => None,
        }
    }
}

/// Tiles per grid row for a list `width` px wide (at least 1).
pub fn grid_columns(width: f32, tile_w: f32) -> usize {
    if tile_w <= 0.0 {
        return 1;
    }
    ((width / tile_w).floor() as usize).max(1)
}

/// Items touched by a rubber band from (x0, y0) to (x1, y1) in list content
/// coordinates (y includes the scroll offset): `cols` items of `item_w` per
/// line of `line_h`, `n` items. Details view: `cols` = 1, x is ignored.
#[allow(clippy::too_many_arguments)]
pub fn band_hits(
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    line_h: f32,
    cols: usize,
    item_w: f32,
    n: usize,
) -> Vec<usize> {
    if n == 0 || line_h <= 0.0 {
        return Vec::new();
    }
    let cols = cols.max(1);
    let (top, bottom) = (y0.min(y1).max(0.0), y0.max(y1));
    if bottom < 0.0 {
        return Vec::new();
    }
    let first_line = (top / line_h).floor() as usize;
    let last_line = (bottom / line_h).floor() as usize;
    let (c0, c1) = if cols == 1 || item_w <= 0.0 {
        (0, 0)
    } else {
        let (l, r) = (x0.min(x1).max(0.0), x0.max(x1));
        if r < 0.0 || l >= cols as f32 * item_w {
            return Vec::new();
        }
        let c0 = (l / item_w).floor() as usize;
        let c1 = ((r / item_w).floor() as usize).min(cols - 1);
        (c0, c1)
    };
    let mut out = Vec::new();
    for line in first_line..=last_line {
        for c in c0..=c1 {
            let i = line * cols + c;
            if i < n {
                out.push(i);
            }
        }
        if line * cols >= n {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip_and_steps() {
        for m in ViewMode::ORDER {
            assert_eq!(ViewMode::parse(m.as_str()), Some(m));
        }
        assert_eq!(ViewMode::parse("bogus"), None);
        assert_eq!(ViewMode::Details.step(true), ViewMode::Medium);
        assert_eq!(ViewMode::ExtraLarge.step(true), ViewMode::ExtraLarge);
        assert_eq!(ViewMode::Medium.step(false), ViewMode::Details);
        assert_eq!(ViewMode::Details.step(false), ViewMode::Details);
        assert!(!ViewMode::Details.is_grid() && ViewMode::Large.is_grid());
    }

    #[test]
    fn shortcuts_on_us_and_german_layouts() {
        assert_eq!(ViewMode::from_shortcut('1'), Some(ViewMode::ExtraLarge));
        assert_eq!(ViewMode::from_shortcut('"'), Some(ViewMode::Large));
        assert_eq!(ViewMode::from_shortcut('§'), Some(ViewMode::Medium));
        assert_eq!(ViewMode::from_shortcut('&'), Some(ViewMode::Details));
        assert_eq!(ViewMode::from_shortcut('4'), None);
    }

    #[test]
    fn rubber_band() {
        // details: rows of 24 px, x ignored
        assert_eq!(
            band_hits(5.0, 30.0, 400.0, 80.0, 24.0, 1, 0.0, 10),
            vec![1, 2, 3]
        );
        assert_eq!(
            band_hits(5.0, 80.0, 400.0, 30.0, 24.0, 1, 0.0, 10),
            vec![1, 2, 3],
            "upwards"
        );
        assert_eq!(
            band_hits(5.0, 200.0, 9.0, 400.0, 24.0, 1, 0.0, 5),
            Vec::<usize>::new(),
            "below the rows"
        );
        // grid: 4 per line, 100×120
        assert_eq!(
            band_hits(150.0, 10.0, 260.0, 130.0, 120.0, 4, 100.0, 10),
            vec![1, 2, 5, 6]
        );
        assert_eq!(
            band_hits(390.0, 10.0, 500.0, 20.0, 120.0, 4, 100.0, 10),
            vec![3],
            "clamped to the last column"
        );
        assert_eq!(
            band_hits(450.0, 10.0, 500.0, 20.0, 120.0, 4, 100.0, 10),
            Vec::<usize>::new(),
            "right of the grid"
        );
        assert_eq!(
            band_hits(0.0, 250.0, 399.0, 300.0, 120.0, 4, 100.0, 10),
            vec![8, 9],
            "last line is short"
        );
    }

    #[test]
    fn columns() {
        assert_eq!(grid_columns(500.0, 92.0), 5);
        assert_eq!(grid_columns(50.0, 92.0), 1);
        assert_eq!(grid_columns(500.0, 0.0), 1);
    }
}
