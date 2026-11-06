//! Persistent user settings: simple `key = value` file, robust against
//! corrupt/unknown content (falls back to defaults per key, never fails).

use crate::SortKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuProbeCache {
    Usable,
    Unusable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// "Hardwarebeschleunigung verwenden, wenn verfügbar" (browser-style).
    pub hardware_acceleration: bool,
    pub show_hidden: bool,
    /// Last GPU probe result, valid only for `gpu_probe_version`.
    pub gpu_probe: Option<GpuProbeCache>,
    pub gpu_probe_version: String,
    pub sort_key: SortKey,
    pub sort_desc: bool,
    pub view: crate::view::ViewMode,
    /// Ordneroptionen (Explorer: "Erweiterungen bei bekannten Dateitypen
    /// ausblenden" inverted, "Geschützte Systemdateien ausblenden" inverted,
    /// "Öffnen durch einfachen Klick").
    pub show_extensions: bool,
    pub show_protected: bool,
    pub single_click: bool,
    /// Inspector (details pane) shown.
    pub inspector: bool,
    /// Command palette: most recently used command ids, newest first.
    pub recent_commands: Vec<String>,
    /// Farbschema: System (follows Windows) / Hell / Dunkel.
    pub theme: crate::theme::ThemeChoice,
    /// "Windows-Akzentfarbe verwenden" (else the default blue).
    pub system_accent: bool,
    /// A second start (folder double click, Win+E) opens a tab in the
    /// running window instead of a new window (M8.3b).
    pub single_instance: bool,
    /// Search index: what to skip besides the defaults (`;`-separated paths
    /// and name patterns, see `explorer_index::Exclusions`).
    pub index_exclude: String,
    /// Index the local drives for the search (Phase 2, S1.4).
    pub index_enabled: bool,
    /// Which drives (see `index_roots`): "" = automatic, ";"-list otherwise.
    pub index_roots: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hardware_acceleration: true,
            show_hidden: false,
            gpu_probe: None,
            gpu_probe_version: String::new(),
            sort_key: SortKey::Name,
            sort_desc: false,
            view: crate::view::ViewMode::Details,
            show_extensions: true,
            show_protected: false,
            single_click: false,
            inspector: false,
            recent_commands: Vec::new(),
            theme: crate::theme::ThemeChoice::System,
            system_accent: true,
            single_instance: true,
            index_exclude: String::new(),
            index_enabled: true,
            index_roots: String::new(),
        }
    }
}

fn parse_bool(v: &str) -> Option<bool> {
    match v.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

impl Settings {
    pub fn parse(text: &str) -> Self {
        let mut s = Settings::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let (k, v) = (k.trim(), v.trim());
            match k {
                "hardware_acceleration" => {
                    if let Some(b) = parse_bool(v) {
                        s.hardware_acceleration = b;
                    }
                }
                "show_hidden" => {
                    if let Some(b) = parse_bool(v) {
                        s.show_hidden = b;
                    }
                }
                "gpu_probe" => {
                    s.gpu_probe = match v {
                        "usable" => Some(GpuProbeCache::Usable),
                        "unusable" => Some(GpuProbeCache::Unusable),
                        _ => None,
                    }
                }
                "gpu_probe_version" => s.gpu_probe_version = v.to_string(),
                "sort_key" => {
                    if let Some(key) = SortKey::parse(v) {
                        s.sort_key = key;
                    }
                }
                "sort_desc" => {
                    if let Some(b) = parse_bool(v) {
                        s.sort_desc = b;
                    }
                }
                "show_extensions" | "show_protected" | "single_click" | "inspector"
                | "system_accent" | "single_instance" | "index_enabled" => {
                    if let Some(b) = parse_bool(v) {
                        match k {
                            "show_extensions" => s.show_extensions = b,
                            "show_protected" => s.show_protected = b,
                            "inspector" => s.inspector = b,
                            "system_accent" => s.system_accent = b,
                            "single_instance" => s.single_instance = b,
                            "index_enabled" => s.index_enabled = b,
                            _ => s.single_click = b,
                        }
                    }
                }
                "recent_commands" => {
                    s.recent_commands = v
                        .split(',')
                        .map(str::trim)
                        .filter(|x| !x.is_empty())
                        .map(String::from)
                        .collect();
                }
                "index_exclude" => s.index_exclude = v.to_string(),
                "index_roots" => s.index_roots = v.to_string(),
                "theme" => {
                    if let Some(t) = crate::theme::ThemeChoice::parse(v) {
                        s.theme = t;
                    }
                }
                "view" => {
                    if let Some(m) = crate::view::ViewMode::parse(v) {
                        s.view = m;
                    }
                }
                _ => {} // unknown keys from newer/older versions are ignored
            }
        }
        s
    }

    pub fn serialize(&self) -> String {
        let probe = match self.gpu_probe {
            Some(GpuProbeCache::Usable) => "usable",
            Some(GpuProbeCache::Unusable) => "unusable",
            None => "",
        };
        format!(
            "# Explorer 2.0 settings\n\
             hardware_acceleration = {}\n\
             show_hidden = {}\n\
             gpu_probe = {probe}\n\
             gpu_probe_version = {}\n\
             sort_key = {}\n\
             sort_desc = {}\n\
             view = {}\n\
             show_extensions = {}\n\
             show_protected = {}\n\
             single_click = {}\n\
             inspector = {}\n\
             recent_commands = {}\n\
             theme = {}\n\
             system_accent = {}\n\
             single_instance = {}\n\
             index_exclude = {}\n\
             index_enabled = {}\n\
             index_roots = {}\n",
            self.hardware_acceleration,
            self.show_hidden,
            self.gpu_probe_version,
            self.sort_key.as_str(),
            self.sort_desc,
            self.view.as_str(),
            self.show_extensions,
            self.show_protected,
            self.single_click,
            self.inspector,
            self.recent_commands.join(","),
            self.theme.as_str(),
            self.system_accent,
            self.single_instance,
            self.index_exclude,
            self.index_enabled,
            self.index_roots
        )
    }

    /// Cached probe result if it was made by this app version.
    pub fn cached_probe(&self, app_version: &str) -> Option<GpuProbeCache> {
        if self.gpu_probe_version == app_version {
            self.gpu_probe
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_on_empty_or_garbage() {
        assert_eq!(Settings::parse(""), Settings::default());
        assert_eq!(
            Settings::parse(
                "\u{0}\u{1}garbage\n=== \nhardware_acceleration = maybe\nsort_key = bogus"
            ),
            Settings::default()
        );
        assert!(Settings::default().hardware_acceleration);
    }

    #[test]
    fn roundtrip() {
        let s = Settings {
            hardware_acceleration: false,
            show_hidden: true,
            gpu_probe: Some(GpuProbeCache::Unusable),
            gpu_probe_version: "1.2.3".into(),
            sort_key: SortKey::Modified,
            sort_desc: true,
            view: crate::view::ViewMode::Large,
            show_extensions: false,
            show_protected: true,
            single_click: true,
            inspector: true,
            recent_commands: vec!["paste".into(), "inspector".into()],
            theme: crate::theme::ThemeChoice::Dark,
            system_accent: false,
            single_instance: false,
            index_exclude: r"C:\Windows\WinSxS;node_modules;*.tmp".into(),
            index_enabled: false,
            index_roots: r"C:\;Z:\".into(),
        };
        assert_eq!(Settings::parse(&s.serialize()), s);
    }

    #[test]
    fn tolerant_parsing() {
        let s = Settings::parse("  # c\nHARDWARE=1\nhardware_acceleration=OFF\nunknown = x\nshow_hidden= yes \nsort_key=type");
        assert!(!s.hardware_acceleration);
        assert!(s.show_hidden);
        assert_eq!(s.sort_key, SortKey::Type);
    }

    #[test]
    fn probe_cache_invalidated_by_version() {
        let s = Settings::parse("gpu_probe = usable\ngpu_probe_version = 0.0.1");
        assert_eq!(s.cached_probe("0.0.1"), Some(GpuProbeCache::Usable));
        assert_eq!(s.cached_probe("0.0.2"), None);
    }
}
