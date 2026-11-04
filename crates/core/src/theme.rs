//! Colour schemes (light / dark / follow Windows) and the accent colour.
//!
//! Every colour of the UI comes from [`tokens`]: the light set is the
//! original look, the dark one mirrors it. Colours that belong to the
//! selection family (selection, hover, current place, drop target) take the
//! hue of the accent; text and outline accents are pushed to readable
//! contrast. Pure → tested on every OS (incl. WCAG contrast of every pair
//! that is drawn on top of each other).

pub type Rgb = [u8; 3];
/// RGBA, alpha 255 = opaque.
pub type Rgba = [u8; 4];

/// The setting "Farbschema".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeChoice {
    /// Follow Windows ("Standard-App-Modus"), live.
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeChoice {
    pub fn as_str(self) -> &'static str {
        match self {
            ThemeChoice::System => "system",
            ThemeChoice::Light => "light",
            ThemeChoice::Dark => "dark",
        }
    }

    pub fn parse(s: &str) -> Option<ThemeChoice> {
        match s.trim().to_ascii_lowercase().as_str() {
            "system" | "auto" => Some(ThemeChoice::System),
            "light" | "hell" => Some(ThemeChoice::Light),
            "dark" | "dunkel" => Some(ThemeChoice::Dark),
            _ => None,
        }
    }

    /// Dark or not, given what the system currently uses.
    pub fn is_dark(self, system_dark: bool) -> bool {
        match self {
            ThemeChoice::System => system_dark,
            ThemeChoice::Light => false,
            ThemeChoice::Dark => true,
        }
    }
}

/// An accent colour: the base plus (when Windows provides them) the shades
/// Windows itself uses on light (`on_light` = "Dark 1") and dark (`on_dark` =
/// "Light 2") backgrounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Accent {
    pub base: Rgb,
    pub on_light: Option<Rgb>,
    pub on_dark: Option<Rgb>,
}

impl Accent {
    pub fn plain(base: Rgb) -> Accent {
        Accent {
            base,
            on_light: None,
            on_dark: None,
        }
    }
}

/// What the system reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SystemTheme {
    pub dark: bool,
    pub accent: Option<Accent>,
}

/// The accent the UI used before M6.4 (= Windows 11 "Dark 1" of the default
/// blue, readable on white).
pub const DEFAULT_ACCENT: Rgb = [0x00, 0x67, 0xc0];
/// Default blue on dark backgrounds (Windows 11 "Light 2").
pub const DEFAULT_ACCENT_ON_DARK: Rgb = [0x4c, 0xc2, 0xff];

/// `HKCU\…\Explorer\Accent\AccentPalette`: 8 colours × RGBA, from light to
/// dark: Light 3, Light 2, Light 1, Accent, Dark 1, Dark 2, Dark 3, (unused).
pub fn accent_from_palette(bytes: &[u8]) -> Option<Accent> {
    if bytes.len() < 28 {
        return None;
    }
    let px = |i: usize| -> Rgb { [bytes[i * 4], bytes[i * 4 + 1], bytes[i * 4 + 2]] };
    Some(Accent {
        base: px(3),
        on_light: Some(px(4)),
        on_dark: Some(px(1)),
    })
}

/// DWORD colours in the registry (`AccentColorMenu`, `DWM\AccentColor`) are
/// stored as 0xAABBGGRR.
pub fn rgb_from_abgr(v: u32) -> Rgb {
    [
        (v & 0xff) as u8,
        ((v >> 8) & 0xff) as u8,
        ((v >> 16) & 0xff) as u8,
    ]
}

/// "#0067c0" / "0067c0" → colour.
pub fn parse_hex(s: &str) -> Option<Rgb> {
    let h = s.trim().trim_start_matches('#');
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let b = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
    Some([b(0)?, b(2)?, b(4)?])
}

pub fn hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

// ---------- colour maths ----------

fn lin(c: u8) -> f64 {
    let c = c as f64 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// WCAG relative luminance.
pub fn luminance(c: Rgb) -> f64 {
    0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])
}

/// WCAG contrast ratio (1 … 21).
pub fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// RGB → (hue°, saturation, lightness).
fn to_hsl(c: Rgb) -> (f64, f64, f64) {
    let [r, g, b] = c.map(|v| v as f64 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d < 1e-9 {
        return (0.0, 0.0, l);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, s.min(1.0), l)
}

fn from_hsl(h: f64, s: f64, l: f64) -> Rgb {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h.rem_euclid(360.0) / 60.0;
    let x = c * (1.0 - (hp.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    [r, g, b].map(|v| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8)
}

/// Re-colour a colour designed for the default blue to the hue (and
/// saturation) of `accent`, keeping its lightness: a light-blue selection
/// becomes light-green for a green accent, grey for a grey one.
///
/// The luminance (not the HSL lightness) is kept: a yellow at the lightness
/// of the light-blue selection would be almost white.
pub fn tint(designed: Rgb, accent: Rgb) -> Rgb {
    if accent == DEFAULT_ACCENT {
        return designed;
    }
    let (h0, s0, _) = to_hsl(DEFAULT_ACCENT);
    let (ha, sa, _) = to_hsl(accent);
    let (h, s, _) = to_hsl(designed);
    let (h, s) = (h + (ha - h0), (s * sa / s0).min(1.0));
    let want = luminance(designed);
    // luminance grows with the lightness at fixed hue/saturation → bisect
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for _ in 0..30 {
        let mid = (lo + hi) / 2.0;
        if luminance(from_hsl(h, s, mid)) < want {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    from_hsl(h, s, (lo + hi) / 2.0)
}

/// Move the lightness of `c` (darker on light, lighter on dark backgrounds)
/// until it reaches `min` contrast against `bg`.
pub fn readable(c: Rgb, bg: Rgb, min: f64) -> Rgb {
    let lighten = luminance(bg) < 0.18;
    let (h, s, mut l) = to_hsl(c);
    let mut out = c;
    for _ in 0..100 {
        if contrast(out, bg) >= min {
            break;
        }
        l = if lighten {
            (l + 0.01).min(1.0)
        } else {
            (l - 0.01).max(0.0)
        };
        out = from_hsl(h, s, l);
    }
    out
}

fn op(c: Rgb) -> Rgba {
    [c[0], c[1], c[2], 255]
}

fn rgb(v: u32) -> Rgb {
    [(v >> 16) as u8, (v >> 8) as u8, v as u8]
}

/// Every colour the UI draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tokens {
    pub dark: bool,
    /// File list background.
    pub content: Rgba,
    /// Raised surfaces: dialogs, popups, address field, current tab, previews.
    pub card: Rgba,
    /// Window frame: tool bar, sidebar, status bar.
    pub chrome: Rgba,
    /// Side panels: inspector, operations.
    pub panel: Rgba,
    /// Column header.
    pub header: Rgba,
    pub tabs_active: Rgba,
    pub tabs_idle: Rgba,
    pub line: Rgba,
    pub line_strong: Rgba,
    /// Border of inputs and dialogs.
    pub outline: Rgba,
    pub hover: Rgba,
    pub hover_strong: Rgba,
    pub row_hover: Rgba,
    pub accent_hover: Rgba,
    /// Current place in the sidebar, active view button.
    pub current: Rgba,
    pub selection: Rgba,
    /// Selection while the list has no focus.
    pub selection_idle: Rgba,
    pub drop: Rgba,
    /// Focus border, active pane, progress, radio dot.
    pub accent: Rgba,
    /// Rubber band fill.
    pub band: Rgba,
    pub text: Rgba,
    pub text2: Rgba,
    pub text3: Rgba,
    pub text4: Rgba,
    pub muted: Rgba,
    pub faint: Rgba,
    pub disabled: Rgba,
    pub danger: Rgba,
    pub warning: Rgba,
    /// Empty part of meters and progress bars.
    pub track: Rgba,
    pub folder: Rgba,
    pub file: Rgba,
    pub drive: Rgba,
    pub scrim: Rgba,
    pub scrim_soft: Rgba,
    pub shadow: Rgba,
    pub shadow_soft: Rgba,
}

/// The colour set for `dark` and an optional accent (`None` = default blue).
pub fn tokens(dark: bool, accent: Option<&Accent>) -> Tokens {
    let base = accent.map(|a| a.base);
    let tinted = |c: u32| -> Rgba {
        let c = rgb(c);
        op(base.map_or(c, |a| tint(c, a)))
    };
    let solid = |c: u32| op(rgb(c));
    let alpha = |c: u32, a: u8| {
        let [r, g, b] = rgb(c);
        [r, g, b, a]
    };
    if !dark {
        let card = rgb(0xffffff);
        let acc = match accent {
            None => DEFAULT_ACCENT,
            Some(a) => readable(a.on_light.unwrap_or(a.base), rgb(0xfafafa), 4.5),
        };
        Tokens {
            dark,
            content: solid(0xfafafa),
            card: op(card),
            chrome: solid(0xf3f4f6),
            panel: solid(0xf7f8fa),
            header: solid(0xf4f5f7),
            tabs_active: solid(0xe4e8ee),
            tabs_idle: solid(0xeceef1),
            line: solid(0xe1e4e8),
            line_strong: solid(0xd0d4d9),
            outline: solid(0xc8ccd0),
            hover: solid(0xe9ecf0),
            hover_strong: solid(0xd9dde2),
            row_hover: tinted(0xeef3f9),
            accent_hover: tinted(0xe5f1fb),
            current: tinted(0xdde8f5),
            selection: tinted(0xcce8ff),
            selection_idle: solid(0xe5e5e5),
            drop: tinted(0xb3dcff),
            accent: op(acc),
            band: [acc[0], acc[1], acc[2], 0x22],
            text: solid(0x222222),
            text2: solid(0x333333),
            text3: solid(0x444444),
            text4: solid(0x555555),
            muted: solid(0x666666),
            faint: solid(0x888888),
            disabled: solid(0x999999),
            danger: solid(0xc0262d),
            warning: solid(0xc9a227),
            track: solid(0xdde1e6),
            folder: solid(0xe8b730),
            file: solid(0x9aa5b1),
            drive: solid(0x7a8794),
            scrim: alpha(0, 0x30),
            scrim_soft: alpha(0, 0x18),
            shadow: alpha(0, 0x40),
            shadow_soft: alpha(0, 0x22),
        }
    } else {
        let card = rgb(0x2b2b2b);
        let acc = match accent {
            None => DEFAULT_ACCENT_ON_DARK,
            Some(a) => readable(a.on_dark.unwrap_or(a.base), card, 4.5),
        };
        Tokens {
            dark,
            content: solid(0x191919),
            card: op(card),
            chrome: solid(0x202020),
            panel: solid(0x242424),
            header: solid(0x222222),
            tabs_active: solid(0x1e1e1e),
            tabs_idle: solid(0x1a1a1a),
            line: solid(0x333333),
            line_strong: solid(0x3d3d3d),
            outline: solid(0x4a4a4a),
            hover: solid(0x2f2f2f),
            hover_strong: solid(0x3c3c3c),
            row_hover: tinted(0x232a31),
            accent_hover: tinted(0x263545),
            current: tinted(0x2b3e52),
            selection: tinted(0x1c3f63),
            selection_idle: solid(0x3a3a3a),
            drop: tinted(0x22507c),
            accent: op(acc),
            band: [acc[0], acc[1], acc[2], 0x33],
            text: solid(0xeeeeee),
            text2: solid(0xe0e0e0),
            text3: solid(0xd0d0d0),
            text4: solid(0xc2c2c2),
            muted: solid(0xa8a8a8),
            faint: solid(0x8c8c8c),
            disabled: solid(0x6e6e6e),
            danger: solid(0xff99a4),
            warning: solid(0xe0b84a),
            track: solid(0x3a3a3a),
            folder: solid(0xe8b730),
            file: solid(0x8a95a1),
            drive: solid(0x8f9ba8),
            scrim: alpha(0, 0x70),
            scrim_soft: alpha(0, 0x50),
            shadow: alpha(0, 0xa0),
            shadow_soft: alpha(0, 0x60),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(x: Rgba) -> Rgb {
        [x[0], x[1], x[2]]
    }

    /// Accents a user can pick in Windows (incl. the awkward ones).
    const ACCENTS: &[u32] = &[
        0x0078d4, 0x0067c0, 0xffb900, 0xe81123, 0x107c10, 0x744da9, 0x767676, 0x00b7c3, 0xffffff,
        0x000000, 0xff8c00, 0xc30052,
    ];

    #[test]
    fn choice_round_trip() {
        for c in [ThemeChoice::System, ThemeChoice::Light, ThemeChoice::Dark] {
            assert_eq!(ThemeChoice::parse(c.as_str()), Some(c));
        }
        assert_eq!(ThemeChoice::parse(" Dunkel "), Some(ThemeChoice::Dark));
        assert_eq!(ThemeChoice::parse("lila"), None);
        assert_eq!(ThemeChoice::default(), ThemeChoice::System);
        assert!(ThemeChoice::System.is_dark(true));
        assert!(!ThemeChoice::System.is_dark(false));
        assert!(!ThemeChoice::Light.is_dark(true));
        assert!(ThemeChoice::Dark.is_dark(false));
    }

    #[test]
    fn registry_formats() {
        assert_eq!(rgb_from_abgr(0xffc06700), [0x00, 0x67, 0xc0]);
        assert_eq!(parse_hex("#0067C0"), Some([0x00, 0x67, 0xc0]));
        assert_eq!(parse_hex("4cc2ff"), Some([0x4c, 0xc2, 0xff]));
        assert_eq!(parse_hex("#12345"), None);
        assert_eq!(parse_hex("#gggggg"), None);
        assert_eq!(hex([0x00, 0x67, 0xc0]), "#0067c0");
        // Windows 11 default blue palette
        let mut pal = Vec::new();
        for v in [
            0x99ebff, 0x4cc2ff, 0x0091f8, 0x0078d4, 0x005fb8, 0x003d92, 0x001968, 0x000000,
        ] {
            let [r, g, b] = rgb(v);
            pal.extend_from_slice(&[r, g, b, 0]);
        }
        let a = accent_from_palette(&pal).unwrap();
        assert_eq!(a.base, [0x00, 0x78, 0xd4]);
        assert_eq!(a.on_light, Some([0x00, 0x5f, 0xb8]));
        assert_eq!(a.on_dark, Some([0x4c, 0xc2, 0xff]));
        assert_eq!(accent_from_palette(&pal[..20]), None);
    }

    #[test]
    fn hsl_round_trip_and_tint() {
        for v in ACCENTS.iter().chain(&[0xcce8ff, 0xeef3f9, 0x1c3f63]) {
            let x = rgb(*v);
            let (h, s, l) = to_hsl(x);
            assert_eq!(from_hsl(h, s, l), x, "{}", hex(x));
        }
        // the default accent leaves the designed colours alone
        assert_eq!(tint(rgb(0xcce8ff), DEFAULT_ACCENT), rgb(0xcce8ff));
        // a grey accent gives a grey selection, green gives green
        let g = tint(rgb(0xcce8ff), rgb(0x767676));
        assert!(g[0] == g[1] && g[1] == g[2], "{g:?}");
        let gr = tint(rgb(0xcce8ff), rgb(0x107c10));
        assert!(gr[1] > gr[0] && gr[1] > gr[2], "{gr:?}");
    }

    #[test]
    fn light_without_accent_is_the_original_look() {
        let t = tokens(false, None);
        assert_eq!(c(t.selection), rgb(0xcce8ff), "E2E [19] measures this");
        assert_eq!(c(t.accent), DEFAULT_ACCENT);
        assert_eq!(c(t.content), rgb(0xfafafa));
        assert!(!t.dark && tokens(true, None).dark);
    }

    fn check_contrast(t: &Tokens, what: &str) {
        let bgs = [
            ("content", t.content),
            ("card", t.card),
            ("chrome", t.chrome),
            ("panel", t.panel),
            ("header", t.header),
            ("tabs_active", t.tabs_active),
            ("hover", t.hover),
            ("row_hover", t.row_hover),
            ("accent_hover", t.accent_hover),
            ("current", t.current),
            ("selection", t.selection),
            ("selection_idle", t.selection_idle),
            ("drop", t.drop),
        ];
        // surfaces carry every text colour, rows (hover/selection/…) only
        // the name (text) and the detail columns (text4)
        for (i, (name, bg)) in bgs.into_iter().enumerate() {
            let bg = c(bg);
            let surface = i < 6;
            for (tn, fg, min) in [
                ("text", t.text, 7.0),
                ("text4", t.text4, 4.5),
                ("muted", t.muted, if surface { 4.5 } else { 0.0 }),
                ("faint", t.faint, if surface { 2.5 } else { 0.0 }),
            ] {
                let r = contrast(c(fg), bg);
                assert!(r >= min, "{what}: {tn} on {name} = {r:.2} < {min}");
            }
        }
        // borders and markers in the accent colour must stand out (WCAG 1.4.11: 3:1)
        for (name, bg) in [
            ("content", t.content),
            ("card", t.card),
            ("tabs_active", t.tabs_active),
        ] {
            let r = contrast(c(t.accent), c(bg));
            assert!(r >= 3.0, "{what}: accent on {name} = {r:.2}");
        }
        let r = contrast(c(t.danger), c(t.chrome));
        assert!(r >= 4.5, "{what}: error text = {r:.2}");
        // the selection must be visible against the list
        let r = contrast(c(t.selection), c(t.content));
        assert!(r >= 1.15, "{what}: selection vs list = {r:.2}");
    }

    #[test]
    fn every_scheme_and_accent_is_readable() {
        for dark in [false, true] {
            check_contrast(&tokens(dark, None), &format!("dark={dark} default"));
            for v in ACCENTS {
                let a = Accent::plain(rgb(*v));
                check_contrast(
                    &tokens(dark, Some(&a)),
                    &format!("dark={dark} accent={}", hex(a.base)),
                );
            }
        }
    }

    #[test]
    fn readable_moves_the_right_way() {
        let white = rgb(0xffffff);
        let y = readable(rgb(0xffb900), white, 4.5);
        assert!(contrast(y, white) >= 4.5 && luminance(y) < luminance(rgb(0xffb900)));
        let dark = rgb(0x2b2b2b);
        let b = readable(rgb(0x000000), dark, 4.5);
        assert!(contrast(b, dark) >= 4.5);
        // already fine → untouched
        assert_eq!(readable(DEFAULT_ACCENT, white, 4.5), DEFAULT_ACCENT);
    }
}
