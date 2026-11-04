//! Sidebar places: quick-access folders and drives.
//!
//! `drives()` can block (disconnected network drives, spinning up optical
//! drives) → always call it off the UI thread.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceKind {
    Home,
    Desktop,
    Downloads,
    Documents,
    Pictures,
    Music,
    Videos,
}

impl PlaceKind {
    pub fn label_de(self) -> &'static str {
        match self {
            PlaceKind::Home => "Persönlicher Ordner",
            PlaceKind::Desktop => "Desktop",
            PlaceKind::Downloads => "Downloads",
            PlaceKind::Documents => "Dokumente",
            PlaceKind::Pictures => "Bilder",
            PlaceKind::Music => "Musik",
            PlaceKind::Videos => "Videos",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub kind: PlaceKind,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveKind {
    Fixed,
    Removable,
    Network,
    Optical,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drive {
    pub root: PathBuf,
    /// Volume label as set by the user ("" if none).
    pub label: String,
    pub kind: DriveKind,
    /// (free, total) bytes, if the volume is ready.
    pub space: Option<(u64, u64)>,
}

/// Explorer-style display name: "System (C:)", "Lokaler Datenträger (C:)", "USB-Laufwerk (E:)".
pub fn drive_display_name(d: &Drive) -> String {
    let root = d.root.to_string_lossy();
    let letter = root.trim_end_matches(['\\', '/']);
    if letter.is_empty() || !letter.ends_with(':') {
        // non-Windows roots ("/")
        return if d.label.is_empty() {
            "Dateisystem".to_string()
        } else {
            d.label.clone()
        };
    }
    let name = if !d.label.is_empty() {
        d.label.as_str()
    } else {
        match d.kind {
            DriveKind::Fixed => "Lokaler Datenträger",
            DriveKind::Removable => "USB-Laufwerk",
            DriveKind::Network => "Netzlaufwerk",
            DriveKind::Optical => "DVD-Laufwerk",
            DriveKind::Other => "Laufwerk",
        }
    };
    format!("{name} ({letter})")
}

/// Used fraction 0.0–1.0 for the drive bar.
pub fn used_fraction(space: Option<(u64, u64)>) -> f32 {
    match space {
        Some((free, total)) if total > 0 => {
            (1.0 - free as f64 / total as f64).clamp(0.0, 1.0) as f32
        }
        _ => 0.0,
    }
}

/// Parses `~/.config/user-dirs.dirs` (`XDG_DOWNLOAD_DIR="$HOME/Downloads"`).
pub fn parse_user_dirs(text: &str, home: &Path) -> Vec<(String, PathBuf)> {
    text.lines()
        .filter_map(|l| {
            let l = l.trim();
            let (k, v) = l.split_once('=')?;
            let k = k.strip_prefix("XDG_")?.strip_suffix("_DIR")?;
            let v = v.trim().trim_matches('"');
            let p = match v.strip_prefix("$HOME") {
                Some(rest) => home.join(rest.trim_start_matches('/')),
                None if v.starts_with('/') => PathBuf::from(v),
                None => return None,
            };
            Some((k.to_string(), p))
        })
        .collect()
}

/// Path of a `shell:` folder (None if this machine has no such folder).
pub fn known_folder_path(f: explorer_core::address::KnownFolder) -> Option<PathBuf> {
    platform::known_folder_path(f)
}

/// Quick-access folders that exist on this machine.
pub fn known_folders() -> Vec<Place> {
    let mut out = platform::known_folders();
    out.retain(|p| p.path.is_dir());
    out.dedup_by(|a, b| a.path == b.path);
    out
}

pub fn drives() -> Vec<Drive> {
    platform::drives()
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows_sys::core::GUID;
    use windows_sys::Win32::Storage::FileSystem::{
        GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
    };
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::System::Diagnostics::Debug::{
        SetThreadErrorMode, SEM_FAILCRITICALERRORS,
    };
    use windows_sys::Win32::UI::Shell::{
        FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_Music,
        FOLDERID_Pictures, FOLDERID_Profile, FOLDERID_Videos, SHGetKnownFolderPath,
    };

    const DRIVE_REMOVABLE: u32 = 2;
    const DRIVE_FIXED: u32 = 3;
    const DRIVE_REMOTE: u32 = 4;
    const DRIVE_CDROM: u32 = 5;

    fn known(id: &GUID) -> Option<PathBuf> {
        let mut p: windows_sys::core::PWSTR = std::ptr::null_mut();
        // SAFETY: valid GUID pointer and out-pointer; the returned buffer is freed with CoTaskMemFree.
        let hr = unsafe { SHGetKnownFolderPath(id, 0, std::ptr::null_mut(), &mut p) };
        let out = if hr >= 0 && !p.is_null() {
            // SAFETY: p is a NUL-terminated wide string allocated by the shell.
            let len = (0..).take_while(|&i| unsafe { *p.add(i) } != 0).count();
            let s = unsafe { std::slice::from_raw_parts(p, len) };
            Some(PathBuf::from(std::ffi::OsString::from_wide(s)))
        } else {
            None
        };
        // SAFETY: CoTaskMemFree accepts null; required even when the call failed.
        unsafe { CoTaskMemFree(p as *const _) };
        out
    }

    pub fn known_folder_path(f: explorer_core::address::KnownFolder) -> Option<PathBuf> {
        use explorer_core::address::KnownFolder as K;
        use windows_sys::Win32::UI::Shell::*;
        let id = match f {
            K::Desktop => &FOLDERID_Desktop,
            K::Downloads => &FOLDERID_Downloads,
            K::Documents => &FOLDERID_Documents,
            K::Pictures => &FOLDERID_Pictures,
            K::Music => &FOLDERID_Music,
            K::Videos => &FOLDERID_Videos,
            K::Profile => &FOLDERID_Profile,
            K::UserProfiles => &FOLDERID_UserProfiles,
            K::AppData => &FOLDERID_RoamingAppData,
            K::LocalAppData => &FOLDERID_LocalAppData,
            K::LocalAppDataLow => &FOLDERID_LocalAppDataLow,
            K::Startup => &FOLDERID_Startup,
            K::Programs => &FOLDERID_Programs,
            K::StartMenu => &FOLDERID_StartMenu,
            K::SendTo => &FOLDERID_SendTo,
            K::Recent => &FOLDERID_Recent,
            K::Templates => &FOLDERID_Templates,
            K::Favorites => &FOLDERID_Favorites,
            K::Links => &FOLDERID_Links,
            K::SavedGames => &FOLDERID_SavedGames,
            K::Screenshots => &FOLDERID_Screenshots,
            K::Fonts => &FOLDERID_Fonts,
            K::Windows => &FOLDERID_Windows,
            K::System => &FOLDERID_System,
            K::ProgramFiles => &FOLDERID_ProgramFiles,
            K::ProgramFilesX86 => &FOLDERID_ProgramFilesX86,
            K::ProgramData => &FOLDERID_ProgramData,
            K::Public => &FOLDERID_Public,
            K::CommonDesktop => &FOLDERID_PublicDesktop,
            K::CommonStartup => &FOLDERID_CommonStartup,
            K::CommonPrograms => &FOLDERID_CommonPrograms,
            K::CommonStartMenu => &FOLDERID_CommonStartMenu,
        };
        known(id)
    }

    pub fn known_folders() -> Vec<Place> {
        [
            (PlaceKind::Desktop, &FOLDERID_Desktop),
            (PlaceKind::Downloads, &FOLDERID_Downloads),
            (PlaceKind::Documents, &FOLDERID_Documents),
            (PlaceKind::Pictures, &FOLDERID_Pictures),
            (PlaceKind::Music, &FOLDERID_Music),
            (PlaceKind::Videos, &FOLDERID_Videos),
            (PlaceKind::Home, &FOLDERID_Profile),
        ]
        .into_iter()
        .filter_map(|(kind, id)| known(id).map(|path| Place { kind, path }))
        .collect()
    }

    pub fn drives() -> Vec<Drive> {
        // No "insert a disk" dialogs for empty card readers / optical drives on this thread.
        let mut old = 0;
        // SAFETY: plain Win32 call with a valid out-pointer.
        unsafe { SetThreadErrorMode(SEM_FAILCRITICALERRORS, &mut old) };
        // SAFETY: no arguments.
        let mask = unsafe { GetLogicalDrives() };
        let mut out = Vec::new();
        for i in 0..26u32 {
            if mask & (1 << i) == 0 {
                continue;
            }
            let root = format!("{}:\\", (b'A' + i as u8) as char);
            let wide: Vec<u16> = std::ffi::OsStr::new(&root)
                .encode_wide()
                .chain(Some(0))
                .collect();
            // SAFETY: NUL-terminated root path.
            let kind = match unsafe { GetDriveTypeW(wide.as_ptr()) } {
                DRIVE_FIXED => DriveKind::Fixed,
                DRIVE_REMOVABLE => DriveKind::Removable,
                DRIVE_REMOTE => DriveKind::Network,
                DRIVE_CDROM => DriveKind::Optical,
                _ => DriveKind::Other,
            };
            let mut label = [0u16; 261];
            // SAFETY: buffer size matches; optional outputs are null.
            let ok = unsafe {
                GetVolumeInformationW(
                    wide.as_ptr(),
                    label.as_mut_ptr(),
                    label.len() as u32,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                )
            };
            let label = if ok != 0 {
                let n = label.iter().position(|&c| c == 0).unwrap_or(0);
                String::from_utf16_lossy(&label[..n])
            } else {
                String::new()
            };
            let (mut free, mut total, mut _all_free) = (0u64, 0u64, 0u64);
            // SAFETY: valid out-pointers.
            let ok = unsafe {
                GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, &mut _all_free)
            };
            let space = (ok != 0 && total > 0).then_some((free, total));
            out.push(Drive {
                root: PathBuf::from(root),
                label,
                kind,
                space,
            });
        }
        // SAFETY: restore previous mode.
        unsafe { SetThreadErrorMode(old, std::ptr::null_mut()) };
        out
    }
}

#[cfg(not(windows))]
mod platform {
    use super::*;

    /// The folders that have a freedesktop counterpart; the rest is Windows-only.
    pub fn known_folder_path(f: explorer_core::address::KnownFolder) -> Option<PathBuf> {
        use explorer_core::address::KnownFolder as K;
        let home = std::env::var_os("HOME").map(PathBuf::from)?;
        let xdg = |var: &str, fallback: &str| {
            std::env::var_os(var)
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(fallback))
        };
        let place = |kind: PlaceKind| {
            known_folders()
                .into_iter()
                .find(|p| p.kind == kind)
                .map(|p| p.path)
        };
        match f {
            K::Desktop => place(PlaceKind::Desktop),
            K::Downloads => place(PlaceKind::Downloads),
            K::Documents => place(PlaceKind::Documents),
            K::Pictures => place(PlaceKind::Pictures),
            K::Music => place(PlaceKind::Music),
            K::Videos => place(PlaceKind::Videos),
            K::Profile => Some(home),
            K::AppData => Some(xdg("XDG_CONFIG_HOME", ".config")),
            K::LocalAppData => Some(xdg("XDG_DATA_HOME", ".local/share")),
            K::Templates => Some(home.join("Templates")),
            K::Fonts => Some(xdg("XDG_DATA_HOME", ".local/share").join("fonts")),
            _ => None,
        }
    }

    pub fn known_folders() -> Vec<Place> {
        let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
            return Vec::new();
        };
        let cfg = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        let dirs = std::fs::read_to_string(cfg.join("user-dirs.dirs"))
            .map(|t| parse_user_dirs(&t, &home))
            .unwrap_or_default();
        let get = |key: &str, fallback: &str| {
            dirs.iter()
                .find(|(k, _)| k == key)
                .map(|(_, p)| p.clone())
                .unwrap_or_else(|| home.join(fallback))
        };
        vec![
            Place {
                kind: PlaceKind::Desktop,
                path: get("DESKTOP", "Desktop"),
            },
            Place {
                kind: PlaceKind::Downloads,
                path: get("DOWNLOAD", "Downloads"),
            },
            Place {
                kind: PlaceKind::Documents,
                path: get("DOCUMENTS", "Documents"),
            },
            Place {
                kind: PlaceKind::Pictures,
                path: get("PICTURES", "Pictures"),
            },
            Place {
                kind: PlaceKind::Music,
                path: get("MUSIC", "Music"),
            },
            Place {
                kind: PlaceKind::Videos,
                path: get("VIDEOS", "Videos"),
            },
            Place {
                kind: PlaceKind::Home,
                path: home,
            },
        ]
    }

    pub fn drives() -> Vec<Drive> {
        vec![Drive {
            root: PathBuf::from("/"),
            label: String::new(),
            kind: DriveKind::Fixed,
            space: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(root: &str, label: &str, kind: DriveKind) -> Drive {
        Drive {
            root: root.into(),
            label: label.into(),
            kind,
            space: None,
        }
    }

    #[test]
    fn display_names() {
        assert_eq!(
            drive_display_name(&d(r"C:\", "System", DriveKind::Fixed)),
            "System (C:)"
        );
        assert_eq!(
            drive_display_name(&d(r"C:\", "", DriveKind::Fixed)),
            "Lokaler Datenträger (C:)"
        );
        assert_eq!(
            drive_display_name(&d(r"E:\", "", DriveKind::Removable)),
            "USB-Laufwerk (E:)"
        );
        assert_eq!(
            drive_display_name(&d(r"Z:\", "", DriveKind::Network)),
            "Netzlaufwerk (Z:)"
        );
        assert_eq!(
            drive_display_name(&d("/", "", DriveKind::Fixed)),
            "Dateisystem"
        );
    }

    #[test]
    fn usage_fraction() {
        assert_eq!(used_fraction(None), 0.0);
        assert_eq!(used_fraction(Some((0, 0))), 0.0);
        assert!((used_fraction(Some((25, 100))) - 0.75).abs() < 1e-6);
        assert_eq!(used_fraction(Some((200, 100))), 0.0); // clamp garbage
    }

    #[test]
    fn xdg_user_dirs() {
        let txt = "# comment\nXDG_DESKTOP_DIR=\"$HOME/Schreibtisch\"\nXDG_DOWNLOAD_DIR=\"$HOME/Downloads\"\nXDG_MUSIC_DIR=\"/data/musik\"\nXDG_BROKEN=\"x\"\nXDG_PICTURES_DIR=\"relative\"\n";
        let v = parse_user_dirs(txt, Path::new("/home/boss"));
        assert_eq!(
            v,
            [
                (
                    "DESKTOP".to_string(),
                    PathBuf::from("/home/boss/Schreibtisch")
                ),
                (
                    "DOWNLOAD".to_string(),
                    PathBuf::from("/home/boss/Downloads")
                ),
                ("MUSIC".to_string(), PathBuf::from("/data/musik")),
            ]
        );
    }

    #[test]
    fn real_places_and_drives_do_not_panic() {
        let k = known_folders();
        assert!(k.iter().all(|p| p.path.is_dir()));
        let dr = drives();
        assert!(!dr.is_empty());
        println!("places: {k:?}\ndrives: {dr:?}");
        #[cfg(windows)]
        assert!(dr
            .iter()
            .any(|d| d.root.to_string_lossy().eq_ignore_ascii_case(r"C:\") && d.space.is_some()));
    }
}
