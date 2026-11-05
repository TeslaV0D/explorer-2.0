//! R-001 against the in-memory registry: "integration on → reset → registry
//! identical to the snapshot before", plus every failure mode we can think of.

use super::*;
use crate::model::RegData;
use std::io;

fn verb_key() -> KeyPath {
    KeyPath::hkcu(r"Software\Classes\Directory\shell\open\command")
}

/// A machine with some pre-existing state that is not ours.
fn machine() -> MemRegistry {
    let mut reg = MemRegistry::new();
    reg.set(
        &KeyPath::hkcu(r"Software\Classes\.txt"),
        "",
        &RegData::sz("txtfile"),
    )
    .unwrap();
    reg.set(
        &KeyPath::hkcu("Environment"),
        "Path",
        &RegData::expand_sz(r"%USERPROFILE%\bin"),
    )
    .unwrap();
    reg.set(
        &KeyPath::hklm(r"Software\Vendor\App"),
        "Setting",
        &RegData::dword(1),
    )
    .unwrap();
    reg
}

fn stage1(exe: &str) -> Vec<Change> {
    let cmd = format!(r#""{exe}" "%1""#);
    vec![
        Change::SetValue {
            key: verb_key(),
            name: "".into(),
            data: RegData::sz(&cmd),
        },
        Change::SetValue {
            key: verb_key(),
            name: "DelegateExecute".into(),
            data: RegData::sz(""),
        },
        // overwrites an existing value of the user
        Change::SetValue {
            key: KeyPath::hkcu("Environment"),
            name: "Path".into(),
            data: RegData::expand_sz(r"%USERPROFILE%\bin;C:\Explorer2"),
        },
        Change::SetValue {
            key: KeyPath::hkcu(r"Software\Explorer2\Selftest"),
            name: "n".into(),
            data: RegData::dword(42),
        },
    ]
}

fn dir() -> (tempfile::TempDir, JournalPaths) {
    let d = tempfile::tempdir().unwrap();
    let p = JournalPaths::in_dir(d.path());
    (d, p)
}

const EXE: &str = r"C:\Program Files\Explorer2\explorer2.exe";

#[test]
fn apply_then_reset_restores_the_exact_snapshot() {
    let (_d, paths) = dir();
    let mut reg = machine();
    let before = reg.clone();
    assert_eq!(apply(&mut reg, &paths, &stage1(EXE)), Ok(4));
    assert!(!reg.same_content(&before));
    assert_eq!(reg.env_notifications, 1, "PATH change is announced");
    assert!(paths.primary.exists() && paths.mirror.exists());
    let r = reset(&mut reg, &paths);
    assert!(r.ok(), "{r:?}");
    assert!(reg.same_content(&before), "registry identical to before");
    assert_eq!(r.journal, JournalState::Ok);
    assert_eq!(r.restored, 4);
    assert!(
        r.keys_removed >= 4,
        "Directory, shell, open, command, Explorer2 …"
    );
    assert!(r.kept.is_empty());
    assert_eq!(reg.env_notifications, 2, "restored PATH is announced too");
    assert!(
        !paths.primary.exists() && !paths.mirror.exists(),
        "journal gone"
    );
    // idempotent
    let again = reset(&mut reg, &paths);
    assert_eq!(again.changed(), 0);
    assert!(again
        .summary_de()
        .starts_with("Keine Windows-Integration aktiv"));
    assert!(reg.same_content(&before));
}

#[test]
fn applying_twice_keeps_the_first_original() {
    let (_d, paths) = dir();
    let mut reg = machine();
    let before = reg.clone();
    apply(&mut reg, &paths, &stage1(EXE)).unwrap();
    apply(&mut reg, &paths, &stage1(r"D:\neu\explorer2.exe")).unwrap();
    let j = match load(&paths) {
        Loaded::Ok(j) => j,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        j.entries
            .iter()
            .filter(|e| matches!(e, Entry::Value { .. }))
            .count(),
        4,
        "no duplicate entries"
    );
    reset(&mut reg, &paths);
    assert!(reg.same_content(&before));
}

#[test]
fn values_changed_by_others_are_left_alone() {
    let (_d, paths) = dir();
    let mut reg = machine();
    apply(&mut reg, &paths, &stage1(EXE)).unwrap();
    // another program takes over the verb and adds to a key we created
    reg.set(&verb_key(), "", &RegData::sz("other.exe %1"))
        .unwrap();
    reg.set(
        &KeyPath::hkcu(r"Software\Classes\Directory\shell"),
        "Foreign",
        &RegData::sz("x"),
    )
    .unwrap();
    let r = reset(&mut reg, &paths);
    assert!(r.ok(), "{r:?}");
    assert_eq!(
        reg.get(&verb_key(), "").unwrap(),
        Some(RegData::sz("other.exe %1"))
    );
    assert!(r
        .kept
        .iter()
        .any(|k| k.contains(r"shell\open\command\(Standard)")));
    assert!(r.kept.iter().any(|k| k.contains("nicht leer")));
    assert!(r.summary_de().contains("von anderer Software geändert"));
    // what was still ours is back
    assert_eq!(
        reg.get(&KeyPath::hkcu("Environment"), "Path").unwrap(),
        Some(RegData::expand_sz(r"%USERPROFILE%\bin"))
    );
}

#[test]
fn primary_journal_damaged_uses_the_mirror() {
    let (_d, paths) = dir();
    let mut reg = machine();
    let before = reg.clone();
    apply(&mut reg, &paths, &stage1(EXE)).unwrap();
    let mut s = std::fs::read_to_string(&paths.primary).unwrap();
    s.truncate(s.len() / 2);
    std::fs::write(&paths.primary, s).unwrap();
    let r = reset(&mut reg, &paths);
    assert_eq!(r.journal, JournalState::Ok);
    assert!(reg.same_content(&before));
}

#[test]
fn without_any_journal_the_sweep_removes_what_is_ours() {
    let (_d, paths) = dir();
    let mut reg = machine();
    // a foreign override at another known point must survive
    let foreign = KeyPath::hkcu(r"Software\Classes\Folder\shell\open\command");
    reg.set(&foreign, "", &RegData::sz("dopus.exe %1")).unwrap();
    let before = reg.clone();
    // only manifest points (Directory verb + own tree): fully recoverable
    let changes: Vec<Change> = stage1(EXE)
        .into_iter()
        .filter(|c| !matches!(c, Change::SetValue { key, .. } if key.is_environment()))
        .collect();
    apply(&mut reg, &paths, &changes).unwrap();
    std::fs::remove_file(&paths.primary).unwrap();
    std::fs::write(&paths.mirror, "garbage\n").unwrap();
    let st = status(&mut reg, &paths);
    assert_eq!(st.journal, JournalState::Damaged);
    assert_eq!(st.active_points, 2, "Directory verb + Software\\Explorer2");
    assert!(st.summary_de().contains("beschädigt"));
    let r = reset(&mut reg, &paths);
    assert_eq!(r.journal, JournalState::Damaged);
    assert_eq!(r.swept, 2);
    assert!(reg.same_content(&before), "{r:?}");
    assert_eq!(
        reg.get(&foreign, "").unwrap(),
        Some(RegData::sz("dopus.exe %1"))
    );
    assert!(
        !paths.mirror.exists(),
        "damaged journal removed after reset"
    );
}

#[test]
fn apply_refuses_on_a_damaged_journal() {
    let (_d, paths) = dir();
    std::fs::create_dir_all(paths.primary.parent().unwrap()).unwrap();
    std::fs::write(&paths.primary, "v1\nvalue\tHKCU\tzz\n").unwrap();
    let mut reg = machine();
    let before = reg.clone();
    assert!(apply(&mut reg, &paths, &stage1(EXE))
        .unwrap_err()
        .contains("beschädigt"));
    assert!(reg.same_content(&before));
}

/// Fails `set` on HKLM (no admin) – or every `set` after `fail_after` calls.
struct Flaky {
    inner: MemRegistry,
    deny_hklm: bool,
    sets_left: Option<usize>,
}

impl Registry for Flaky {
    fn get(&self, k: &KeyPath, n: &str) -> io::Result<Option<RegData>> {
        self.inner.get(k, n)
    }
    fn set(&mut self, k: &KeyPath, n: &str, d: &RegData) -> io::Result<()> {
        if self.deny_hklm && k.hive == Hive::LocalMachine {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied"));
        }
        if let Some(left) = &mut self.sets_left {
            if *left == 0 {
                return Err(io::Error::other("crash"));
            }
            *left -= 1;
        }
        self.inner.set(k, n, d)
    }
    fn delete_value(&mut self, k: &KeyPath, n: &str) -> io::Result<()> {
        if self.deny_hklm && k.hive == Hive::LocalMachine {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied"));
        }
        self.inner.delete_value(k, n)
    }
    fn key_exists(&self, k: &KeyPath) -> io::Result<bool> {
        self.inner.key_exists(k)
    }
    fn key_is_empty(&self, k: &KeyPath) -> io::Result<bool> {
        self.inner.key_is_empty(k)
    }
    fn delete_empty_key(&mut self, k: &KeyPath) -> io::Result<()> {
        if self.deny_hklm && k.hive == Hive::LocalMachine {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied"));
        }
        self.inner.delete_empty_key(k)
    }
    fn delete_tree(&mut self, k: &KeyPath) -> io::Result<()> {
        self.inner.delete_tree(k)
    }
}

#[test]
fn a_crash_between_journal_and_change_is_harmless() {
    let (_d, paths) = dir();
    let mut reg = Flaky {
        inner: machine(),
        deny_hklm: false,
        sets_left: Some(2),
    };
    let before = reg.inner.clone();
    // 3rd change "crashes": journaled, not executed
    assert!(apply(&mut reg, &paths, &stage1(EXE)).is_err());
    reg.sets_left = None;
    let r = reset(&mut reg, &paths);
    assert!(r.ok(), "{r:?}");
    assert!(r.kept.is_empty(), "unexecuted change = original = fine");
    assert!(reg.inner.same_content(&before));
}

#[test]
fn errors_stay_in_the_journal_for_the_next_try() {
    let (_d, paths) = dir();
    let ifeo = KeyPath::hklm(
        r"Software\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\explorer.exe",
    );
    let mut reg = Flaky {
        inner: machine(),
        deny_hklm: false,
        sets_left: None,
    };
    reg.inner.create_key(&KeyPath::hklm(
        r"Software\Microsoft\Windows NT\CurrentVersion\Image File Execution Options",
    ));
    let before = reg.inner.clone();
    apply(
        &mut reg,
        &paths,
        &[Change::SetValue {
            key: ifeo.clone(),
            name: "Debugger".into(),
            data: RegData::sz(r"C:\x\explorer2-proxy.exe"),
        }],
    )
    .unwrap();
    // reset without admin rights
    reg.deny_hklm = true;
    let r = reset(&mut reg, &paths);
    assert!(!r.ok());
    assert!(r.summary_de().contains("Fehler"));
    match load(&paths) {
        Loaded::Ok(j) => assert!(!j.is_empty(), "kept for retry"),
        other => panic!("{other:?}"),
    }
    // with admin rights
    reg.deny_hklm = false;
    let r = reset(&mut reg, &paths);
    assert!(r.ok(), "{r:?}");
    assert!(reg.inner.same_content(&before));
    assert_eq!(load(&paths), Loaded::None);
}

#[test]
fn files_are_restored_or_removed() {
    let (d, paths) = dir();
    let mut reg = machine();
    let new = d
        .path()
        .join("Startmenü")
        .join("Explorer 2.0 – Wiederherstellen.lnk");
    let old = d.path().join("vorhanden.txt");
    std::fs::write(&old, b"original").unwrap();
    let changed = d.path().join("geaendert.txt");
    apply(
        &mut reg,
        &paths,
        &[
            Change::WriteFile {
                path: new.clone(),
                bytes: b"lnk".to_vec(),
            },
            Change::WriteFile {
                path: old.clone(),
                bytes: b"ours".to_vec(),
            },
            Change::WriteFile {
                path: changed.clone(),
                bytes: b"ours".to_vec(),
            },
        ],
    )
    .unwrap();
    assert_eq!(std::fs::read(&old).unwrap(), b"ours");
    std::fs::write(&changed, b"user edited").unwrap();
    let r = reset(&mut reg, &paths);
    assert!(r.ok(), "{r:?}");
    assert!(!new.exists());
    assert_eq!(std::fs::read(&old).unwrap(), b"original");
    assert_eq!(
        std::fs::read(&changed).unwrap(),
        b"user edited",
        "left alone"
    );
    assert_eq!(r.kept.len(), 1);
    assert!(!paths.backups.exists(), "backups cleaned up");
}

#[test]
fn journal_format_round_trip_and_damage_detection() {
    let j = Journal {
        entries: vec![
            Entry::Key {
                key: KeyPath::hkcu(r"Software\Classes\Ä Ordner"),
            },
            Entry::Value {
                key: KeyPath::hklm(r"Software\X"),
                name: "".into(),
                prior: None,
                ours: RegData::sz("tab\there\nnewline"),
            },
            Entry::Value {
                key: KeyPath::hkcu("Environment"),
                name: "Path".into(),
                prior: Some(RegData {
                    kind: 99,
                    bytes: vec![],
                }),
                ours: RegData::dword(3),
            },
            Entry::File {
                path: PathBuf::from("/tmp/ä b.txt"),
                backup: Some("1.bak".into()),
                ours: 0xdead_beef,
            },
            Entry::File {
                path: PathBuf::from(r"C:\neu.lnk"),
                backup: None,
                ours: 1,
            },
        ],
    };
    let s = j.serialize();
    assert_eq!(Journal::parse(&s), Ok(j.clone()));
    // a flipped bit in a data field → checksum mismatch, entries salvaged
    let tampered = s.replacen("value\tHKLM", "value\tHKCU", 1);
    let salvaged = Journal::parse(&tampered).unwrap_err();
    assert_eq!(salvaged.entries.len(), 5);
    // cut off → no end line
    let cut = &s[..s.len() - 10];
    assert!(Journal::parse(cut).is_err());
}

#[test]
fn status_reports_what_is_active() {
    let (_d, paths) = dir();
    let mut reg = machine();
    let st = status(&mut reg, &paths);
    assert!(!st.any());
    assert_eq!(st.summary_de(), "Keine Windows-Integration aktiv.");
    apply(&mut reg, &paths, &stage1(EXE)).unwrap();
    let st = status(&mut reg, &paths);
    assert!(st.any());
    assert!(st.entries >= 4);
    assert_eq!(st.active_points, 2);
}

#[test]
fn manifest_points_are_sane() {
    for p in manifest::points() {
        match p {
            manifest::Point::Value { key, stop, .. } => {
                assert!(key.is_within(&stop) && key.norm() != stop.norm(), "{key}");
                assert_eq!(key.hive, Hive::CurrentUser, "HKCU only so far: {key}");
            }
            manifest::Point::OwnTree(k) => {
                assert!(k.path.to_lowercase().contains("explorer2"), "{k}")
            }
        }
    }
    assert!(manifest::is_ours(r#""C:\Tools\Explorer2.EXE" "%1""#));
    assert!(!manifest::is_ours("explorer.exe /e"));
}

#[test]
fn key_paths() {
    let k = KeyPath::hkcu(r"\Software\\Classes\X\");
    assert_eq!(k.path, r"Software\Classes\X");
    assert_eq!(k.lineage().len(), 3);
    assert_eq!(k.parent().unwrap().path, r"Software\Classes");
    assert!(k.is_within(&KeyPath::hkcu(r"software\classes")));
    assert!(!KeyPath::hkcu(r"Software\ClassesX").is_within(&KeyPath::hkcu(r"Software\Classes")));
    assert_eq!(RegData::sz("a").text().as_deref(), Some("a"));
    assert_eq!(RegData::dword(1).text(), None);
}
