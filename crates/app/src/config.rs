//! Settings file location + atomic, serialized load/modify/save.

use explorer_core::settings::Settings;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Serializes read-modify-write cycles (UI thread + background GPU re-probe).
static LOCK: Mutex<()> = Mutex::new(());

/// `EXPLORER2_CONFIG_DIR` (tests/portable) > `%APPDATA%\Explorer2` > XDG config.
pub fn config_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("EXPLORER2_CONFIG_DIR") {
        return Some(PathBuf::from(d));
    }
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Explorer2"))
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|d| d.join("explorer2"))
    }
}

pub fn settings_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("settings.ini"))
}

pub fn load_from(path: &Path) -> Settings {
    std::fs::read_to_string(path)
        .map(|t| Settings::parse(&t))
        .unwrap_or_default()
}

/// Write to a temp file and rename, so a crash never leaves a half-written file.
pub fn save_to(path: &Path, s: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("ini.tmp");
    std::fs::write(&tmp, s.serialize())?;
    std::fs::rename(&tmp, path)
}

pub fn load() -> Settings {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    settings_path().map(|p| load_from(&p)).unwrap_or_default()
}

/// Load fresh, apply `f`, save. Errors are reported, never fatal.
pub fn update(f: impl FnOnce(&mut Settings)) -> Result<Settings, String> {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = settings_path().ok_or("kein Konfigurationsordner")?;
    let mut s = load_from(&path);
    f(&mut s);
    save_to(&path, &s).map_err(|e| format!("Einstellungen nicht gespeichert: {e}"))?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_defaults_and_save_roundtrips() {
        let d = std::env::temp_dir().join(format!("explorer2-cfg-test-{}", std::process::id()));
        let p = d.join("sub").join("settings.ini");
        assert_eq!(load_from(&p), Settings::default());
        let s = Settings {
            hardware_acceleration: false,
            show_hidden: true,
            ..Default::default()
        };
        save_to(&p, &s).unwrap();
        assert_eq!(load_from(&p), s);
        assert!(!p.with_extension("ini.tmp").exists());
        let _ = std::fs::remove_dir_all(&d);
    }
}
