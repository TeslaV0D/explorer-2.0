//! What the system uses: dark or light apps, accent colour – plus live
//! change notification and the dark title bar.
//!
//! Windows reads `HKCU\…\Themes\Personalize\AppsUseLightTheme` and
//! `HKCU\…\Explorer\Accent\AccentPalette` (fallback `DWM\AccentColor`).
//! Overrides for tests and other platforms: `EXPLORER2_SYSTEM_THEME=dark|light`,
//! `EXPLORER2_ACCENT=#rrggbb`; without them Linux reports `GTK_THEME=…:dark`
//! as dark and has no accent.

use explorer_core::theme::{parse_hex, Accent, SystemTheme};

/// Current system theme (cheap: a few registry reads).
pub fn system_theme() -> SystemTheme {
    #[cfg(windows)]
    let mut t = crate::win_theme::read();
    #[cfg(not(windows))]
    let mut t = SystemTheme {
        dark: std::env::var("GTK_THEME").is_ok_and(|v| v.to_ascii_lowercase().ends_with(":dark")),
        accent: None,
    };
    if let Ok(v) = std::env::var("EXPLORER2_SYSTEM_THEME") {
        match v.trim() {
            "dark" => t.dark = true,
            "light" => t.dark = false,
            _ => {}
        }
    }
    if let Some(c) = std::env::var("EXPLORER2_ACCENT")
        .ok()
        .and_then(|v| parse_hex(&v))
    {
        t.accent = Some(Accent::plain(c));
    }
    t
}

/// Calls `changed` (on a background thread) whenever Windows' colour
/// settings may have changed. Elsewhere a no-op. The callback should re-read
/// [`system_theme`] and compare.
pub fn watch_system_theme(changed: impl Fn() + Send + 'static) {
    #[cfg(windows)]
    crate::win_theme::watch(changed);
    #[cfg(not(windows))]
    let _ = changed;
}

/// Dark or light native popup menus (the shell's context menu under
/// "Weitere Optionen"). Uses uxtheme's undocumented SetPreferredAppMode
/// (ordinal 135) + FlushMenuThemes (136), like many apps do; if they are
/// missing the menu simply stays light. Elsewhere a no-op.
pub fn set_dark_menus(dark: bool) -> bool {
    #[cfg(windows)]
    return crate::win_theme::dark_menus(dark);
    #[cfg(not(windows))]
    {
        let _ = dark;
        false
    }
}

/// Dark or light title bar (Windows 10 20H1+ / 11); elsewhere a no-op.
pub fn set_dark_title_bar(hwnd: isize, dark: bool) -> Result<(), String> {
    #[cfg(windows)]
    return crate::win_theme::title_bar(hwnd, dark);
    #[cfg(not(windows))]
    {
        let _ = (hwnd, dark);
        Ok(())
    }
}
