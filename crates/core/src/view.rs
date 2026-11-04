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
    fn columns() {
        assert_eq!(grid_columns(500.0, 92.0), 5);
        assert_eq!(grid_columns(50.0, 92.0), 1);
        assert_eq!(grid_columns(500.0, 0.0), 1);
    }
}
