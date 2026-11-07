//! Per-user installation (M8): program files in `%LOCALAPPDATA%\Programs\
//! Explorer2`, Start menu shortcuts, an entry in "Apps & Features".
//!
//! Shortcuts and the uninstall entry go through the journal like every other
//! integration, so the one-button reset and `explorer2-recover` remove them
//! too. The program files themselves are the app, not an integration: the
//! uninstaller deletes the folder after the reset.

use crate::features::{self, State};
use crate::model::{KeyPath, RegData};
use crate::{apply, Change, JournalPaths, Registry, ResetReport};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const APP_EXE: &str = if cfg!(windows) {
    "explorer2.exe"
} else {
    "explorer2"
};
pub const RECOVER_EXE: &str = if cfg!(windows) {
    "explorer2-recover.exe"
} else {
    "explorer2-recover"
};
pub const DISPLAY_NAME: &str = "Explorer 2.0";

/// `HKCU\…\Uninstall\Explorer2` ("Apps & Features").
pub fn uninstall_key() -> KeyPath {
    KeyPath::hkcu(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Explorer2")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallPaths {
    pub program_dir: PathBuf,
    /// Our folder in the Start menu ("Explorer 2.0").
    pub start_menu_dir: PathBuf,
}

impl InstallPaths {
    /// `EXPLORER2_INSTALL_DIR` / `EXPLORER2_STARTMENU_DIR` (tests) >
    /// `%LOCALAPPDATA%\Programs\Explorer2` and
    /// `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Explorer 2.0`.
    pub fn default_paths() -> Option<InstallPaths> {
        let env = |k: &str| std::env::var_os(k).map(PathBuf::from);
        let program_dir = env("EXPLORER2_INSTALL_DIR")
            .or_else(|| Some(env("LOCALAPPDATA")?.join("Programs").join("Explorer2")))?;
        let start_menu_dir = env("EXPLORER2_STARTMENU_DIR").or_else(|| {
            Some(
                env("APPDATA")?
                    .join(r"Microsoft\Windows\Start Menu\Programs")
                    .join(DISPLAY_NAME),
            )
        })?;
        Some(InstallPaths {
            program_dir,
            start_menu_dir,
        })
    }

    pub fn app(&self) -> PathBuf {
        self.program_dir.join(APP_EXE)
    }

    pub fn recover(&self) -> PathBuf {
        self.program_dir.join(RECOVER_EXE)
    }
}

/// A Start menu shortcut (the `.lnk` bytes are made by the caller: COM on
/// Windows, anything in tests).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcut {
    pub file: PathBuf,
    pub target: PathBuf,
    pub args: String,
    pub description: String,
}

pub fn shortcuts(p: &InstallPaths) -> Vec<Shortcut> {
    vec![
        Shortcut {
            file: p.start_menu_dir.join("Explorer 2.0.lnk"),
            target: p.app(),
            args: String::new(),
            description: "Dateien und Ordner verwalten".into(),
        },
        Shortcut {
            file: p
                .start_menu_dir
                .join("Explorer 2.0 - Windows-Standard wiederherstellen.lnk"),
            target: p.recover(),
            args: String::new(),
            description: "Alle Windows-Integrationen von Explorer 2.0 zurücksetzen".into(),
        },
    ]
}

/// Values of the "Apps & Features" entry.
pub fn uninstall_values(p: &InstallPaths, version: &str, size_kb: u32) -> Vec<Change> {
    let key = uninstall_key();
    let q = |path: &Path| format!("\"{}\"", path.display());
    let sz = |name: &str, v: String| Change::SetValue {
        key: key.clone(),
        name: name.into(),
        data: RegData::sz(&v),
    };
    let dw = |name: &str, v: u32| Change::SetValue {
        key: key.clone(),
        name: name.into(),
        data: RegData::dword(v),
    };
    vec![
        sz("DisplayName", DISPLAY_NAME.into()),
        sz("DisplayVersion", version.into()),
        sz("Publisher", DISPLAY_NAME.into()),
        sz("DisplayIcon", format!("{},0", q(&p.app()))),
        sz("InstallLocation", p.program_dir.display().to_string()),
        sz(
            "UninstallString",
            format!("{} --uninstall", q(&p.recover())),
        ),
        sz(
            "QuietUninstallString",
            format!("{} --uninstall --quiet", q(&p.recover())),
        ),
        dw("NoModify", 1),
        dw("NoRepair", 1),
        dw("EstimatedSize", size_kb),
    ]
}

/// Installed = the "Apps & Features" entry exists and the program is there.
pub fn is_installed<R: Registry + ?Sized>(reg: &R, p: &InstallPaths) -> bool {
    matches!(reg.get(&uninstall_key(), "DisplayName"), Ok(Some(_))) && p.app().exists()
}

/// Copy `src` over `dst`; a running (locked) `dst` is renamed out of the way
/// first – Windows allows renaming a running exe, not overwriting it.
fn copy_over(src: &Path, dst: &Path) -> io::Result<()> {
    if fs::read(src).ok() == fs::read(dst).ok() && dst.exists() {
        return Ok(());
    }
    if dst.exists() {
        if let Err(e) = fs::remove_file(dst) {
            let old = dst.with_extension(format!("old-{}", std::process::id()));
            fs::rename(dst, &old).map_err(|_| e)?;
        }
    }
    let tmp = dst.with_extension("tmp-install");
    fs::copy(src, &tmp)?;
    fs::rename(&tmp, dst)
}

/// Left-overs of updates over a running app (`*.old-<pid>`), removed when
/// they are no longer in use.
pub fn clean_old_files(dir: &Path) {
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if n.contains(".old-") {
                let _ = fs::remove_file(e.path());
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallReport {
    pub program_dir: PathBuf,
    pub shortcuts: usize,
    /// Integrations that now point to the installed exe.
    pub repointed: Vec<&'static str>,
}

/// Install (or update) from `source_dir` (where the running exe lives).
pub fn install<R: Registry + ?Sized>(
    reg: &mut R,
    jpaths: &JournalPaths,
    ipaths: &InstallPaths,
    source_dir: &Path,
    version: &str,
    make_shortcut: &dyn Fn(&Shortcut) -> io::Result<Vec<u8>>,
) -> Result<InstallReport, String> {
    let err = |what: &str, e: io::Error| format!("{what}: {e}");
    fs::create_dir_all(&ipaths.program_dir).map_err(|e| err("Programmordner", e))?;
    let mut size = 0u64;
    for name in [APP_EXE, RECOVER_EXE] {
        let src = source_dir.join(name);
        if !src.exists() {
            return Err(format!("{} fehlt neben dem Programm", src.display()));
        }
        let dst = ipaths.program_dir.join(name);
        if src != dst {
            copy_over(&src, &dst).map_err(|e| err(name, e))?;
        }
        size += fs::metadata(&dst).map(|m| m.len()).unwrap_or(0);
    }
    let mut changes = uninstall_values(ipaths, version, size.div_ceil(1024) as u32);
    let links = shortcuts(ipaths);
    for s in &links {
        let bytes = make_shortcut(s).map_err(|e| err("Verknüpfung", e))?;
        changes.push(Change::WriteFile {
            path: s.file.clone(),
            bytes,
        });
    }
    apply(reg, jpaths, &changes)?;
    // integrations switched on from a downloaded copy → the installed exe
    let installed = ipaths.app();
    let mut repointed = Vec::new();
    for f in features::ALL {
        // only what is clearly ours (a feature half taken over by another
        // program stays as it is)
        if matches!(features::state(reg, f, &installed), State::OnElsewhere(_)) {
            features::enable(reg, jpaths, f, &installed)?;
            repointed.push(f.id());
        }
    }
    Ok(InstallReport {
        program_dir: ipaths.program_dir.clone(),
        shortcuts: links.len(),
        repointed,
    })
}

/// Uninstall, part 1 (registry + shortcuts + every integration): the full
/// R-001 reset. Part 2 is deleting the program folder ([`remove_program_dir`]),
/// which a copy of the uninstaller outside the folder does.
pub fn uninstall<R: Registry + ?Sized>(
    reg: &mut R,
    jpaths: &JournalPaths,
    ipaths: &InstallPaths,
) -> ResetReport {
    let r = crate::reset(reg, jpaths);
    // our Start menu folder, if the reset left it empty
    let _ = fs::remove_dir(&ipaths.start_menu_dir);
    r
}

/// Only a folder that holds nothing but our program files (and update
/// left-overs) may be deleted – never a wrong path from a broken setting.
pub fn is_our_program_dir(dir: &Path) -> bool {
    let Ok(rd) = fs::read_dir(dir) else {
        return false;
    };
    let mut ours = false;
    for e in rd.flatten() {
        let n = e.file_name().to_string_lossy().to_lowercase();
        let base = n.split(".old-").next().unwrap_or(&n).to_string();
        let known = [APP_EXE, RECOVER_EXE]
            .iter()
            .any(|k| base == k.to_lowercase() || base == k.trim_end_matches(".exe"));
        if !known && !n.ends_with(".tmp-install") {
            return false;
        }
        ours = true;
    }
    ours
}

/// Delete the program folder (retries while files are still in use).
/// Refuses anything that is not clearly ours ([`is_our_program_dir`]).
pub fn remove_program_dir(dir: &Path, attempts: u32) -> io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    if !is_our_program_dir(dir) {
        return Err(io::Error::other(format!(
            "{} enthält fremde Dateien – nicht gelöscht",
            dir.display()
        )));
    }
    let mut last = Ok(());
    for i in 0..attempts.max(1) {
        match fs::remove_dir_all(dir) {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => last = Err(e),
        }
        if i + 1 < attempts {
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{load, Loaded, MemRegistry};

    fn setup() -> (tempfile::TempDir, JournalPaths, InstallPaths, PathBuf) {
        let d = tempfile::tempdir().unwrap();
        let j = JournalPaths::in_dir(&d.path().join("cfg"));
        let i = InstallPaths {
            program_dir: d.path().join("Programs").join("Explorer2"),
            start_menu_dir: d.path().join("Start").join("Explorer 2.0"),
        };
        let src = d.path().join("Downloads");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join(APP_EXE), b"app v1").unwrap();
        fs::write(src.join(RECOVER_EXE), b"recover v1").unwrap();
        (d, j, i, src)
    }

    fn lnk(s: &Shortcut) -> io::Result<Vec<u8>> {
        Ok(format!("lnk -> {}", s.target.display()).into_bytes())
    }

    #[test]
    fn install_then_uninstall_leaves_nothing() {
        let (_d, j, i, src) = setup();
        let mut reg = MemRegistry::new();
        let before = reg.clone();
        assert!(!is_installed(&reg, &i));
        let r = install(&mut reg, &j, &i, &src, "0.0.1", &lnk).unwrap();
        assert_eq!(r.shortcuts, 2);
        assert!(is_installed(&reg, &i));
        assert_eq!(fs::read(i.app()).unwrap(), b"app v1");
        assert_eq!(fs::read(i.recover()).unwrap(), b"recover v1");
        let un = reg
            .get(&uninstall_key(), "UninstallString")
            .unwrap()
            .unwrap();
        assert!(
            un.text()
                .unwrap()
                .ends_with("explorer2-recover.exe\" --uninstall")
                || !cfg!(windows)
        );
        for s in shortcuts(&i) {
            assert!(s.file.exists(), "{}", s.file.display());
        }
        // part 1: reset removes entry + shortcuts (+ integrations)
        let rep = uninstall(&mut reg, &j, &i);
        assert!(rep.ok(), "{rep:?}");
        assert!(reg.same_content(&before));
        assert!(
            !i.start_menu_dir.exists(),
            "empty Start menu folder removed"
        );
        assert_eq!(load(&j), Loaded::None);
        // part 2: program folder
        remove_program_dir(&i.program_dir, 3).unwrap();
        assert!(!i.program_dir.exists());
        assert!(remove_program_dir(&i.program_dir, 1).is_ok(), "idempotent");
    }

    #[test]
    fn update_over_an_installation_keeps_one_entry() {
        let (_d, j, i, src) = setup();
        let mut reg = MemRegistry::new();
        let before = reg.clone();
        install(&mut reg, &j, &i, &src, "0.0.1", &lnk).unwrap();
        fs::write(src.join(APP_EXE), b"app v2").unwrap();
        install(&mut reg, &j, &i, &src, "0.0.2", &lnk).unwrap();
        assert_eq!(fs::read(i.app()).unwrap(), b"app v2");
        assert_eq!(
            reg.get(&uninstall_key(), "DisplayVersion").unwrap(),
            Some(RegData::sz("0.0.2"))
        );
        uninstall(&mut reg, &j, &i);
        assert!(
            reg.same_content(&before),
            "original from before the first install"
        );
    }

    #[test]
    fn integrations_follow_the_installed_exe() {
        let (_d, j, i, src) = setup();
        let mut reg = MemRegistry::new();
        let before = reg.clone();
        // switched on while running from the download folder
        features::enable(
            &mut reg,
            &j,
            features::Feature::FolderOpen,
            &src.join(APP_EXE),
        )
        .unwrap();
        let r = install(&mut reg, &j, &i, &src, "0.0.1", &lnk).unwrap();
        assert_eq!(r.repointed, vec!["folder-open"]);
        assert_eq!(
            features::state(&reg, features::Feature::FolderOpen, &i.app()),
            State::On
        );
        assert_eq!(
            features::state(&reg, features::Feature::ContextMenu, &i.app()),
            State::Off,
            "nothing switched on that was off"
        );
        uninstall(&mut reg, &j, &i);
        assert!(
            reg.same_content(&before),
            "uninstall also removes the integrations"
        );
    }

    #[test]
    fn never_deletes_a_foreign_folder() {
        let d = tempfile::tempdir().unwrap();
        let foreign = d.path().join("Dokumente");
        fs::create_dir_all(&foreign).unwrap();
        fs::write(foreign.join(APP_EXE), b"x").unwrap();
        fs::write(foreign.join("Steuer 2025.pdf"), b"wichtig").unwrap();
        assert!(!is_our_program_dir(&foreign));
        assert!(remove_program_dir(&foreign, 1).is_err());
        assert!(foreign.join("Steuer 2025.pdf").exists());
        let empty = d.path().join("leer");
        fs::create_dir_all(&empty).unwrap();
        assert!(!is_our_program_dir(&empty), "an empty folder is not proof");
        let ours = d.path().join("Explorer2");
        fs::create_dir_all(&ours).unwrap();
        fs::write(ours.join(APP_EXE), b"x").unwrap();
        fs::write(ours.join(format!("{APP_EXE}.old-7")), b"x").unwrap();
        assert!(is_our_program_dir(&ours));
        remove_program_dir(&ours, 1).unwrap();
        assert!(!ours.exists());
    }

    #[test]
    fn missing_files_and_old_leftovers() {
        let (_d, j, i, src) = setup();
        fs::remove_file(src.join(RECOVER_EXE)).unwrap();
        let mut reg = MemRegistry::new();
        let e = install(&mut reg, &j, &i, &src, "0.0.1", &lnk).unwrap_err();
        assert!(e.contains("fehlt"), "{e}");
        assert_eq!(load(&j), Loaded::None, "nothing journaled");
        fs::create_dir_all(&i.program_dir).unwrap();
        fs::write(i.program_dir.join("explorer2.old-42"), b"x").unwrap();
        clean_old_files(&i.program_dir);
        assert!(!i.program_dir.join("explorer2.old-42").exists());
    }

    #[test]
    fn the_uninstall_entry_is_in_the_manifest() {
        assert!(crate::manifest::points().iter().any(|p| matches!(p,
            crate::manifest::Point::OwnTree(k) if k.norm() == uninstall_key().norm())));
    }
}
