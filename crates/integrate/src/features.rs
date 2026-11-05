//! The Windows integrations (M7), each switchable on its own. All go through
//! the journal (`apply` / `revert_within`), all their places are in the
//! manifest – so the one-button reset and `explorer2-recover` cover them.
//!
//! Only real file-system folders are taken over (`Directory`, `Drive`), never
//! `Folder`: that class also opens virtual folders (Control Panel, Recycle
//! Bin, …) which must keep working with Windows' own Explorer.
//!
//! "Open folders" adds our own verb `explorer2open` and makes it the
//! default verb of the class (`Directory\shell` @ = `explorer2open`; Windows
//! ships `none` there, which falls through to Explorer). Windows' own `open`
//! verb is never touched, and the sweep recognises our verb by its name.

use crate::model::{KeyPath, RegData};
use crate::{apply, revert_within, Change, JournalPaths, Registry, ResetReport, Scope};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    /// Double click on a folder / drive and "open folder" of other programs.
    FolderOpen,
    /// "In Explorer 2.0 öffnen" in the context menu of folders, drives and
    /// the folder background.
    ContextMenu,
}

pub const ALL: [Feature; 2] = [Feature::FolderOpen, Feature::ContextMenu];

const CLASSES: &str = r"Software\Classes";
/// Our verb for "open folders" (its name contains the manifest marker).
pub const OPEN_VERB: &str = "explorer2open";

fn classes(rest: &str) -> KeyPath {
    KeyPath::hkcu(&format!(r"{CLASSES}\{rest}"))
}

/// `"C:\…\explorer2.exe" "%1"` (the exe path is always quoted).
pub fn command(exe: &Path, arg: &str) -> String {
    format!("\"{}\" \"{arg}\"", exe.display())
}

impl Feature {
    pub fn id(self) -> &'static str {
        match self {
            Feature::FolderOpen => "folder-open",
            Feature::ContextMenu => "context-menu",
        }
    }

    pub fn parse(s: &str) -> Option<Feature> {
        ALL.into_iter().find(|f| f.id() == s)
    }

    pub fn label_de(self) -> &'static str {
        match self {
            Feature::FolderOpen => "Ordner mit Explorer 2.0 öffnen",
            Feature::ContextMenu => "„In Explorer 2.0 öffnen“ im Kontextmenü",
        }
    }

    /// What this feature owns (for switching it off on its own).
    pub fn scopes(self) -> Vec<Scope> {
        match self {
            Feature::FolderOpen => ["Directory", "Drive"]
                .into_iter()
                .flat_map(|c| {
                    [
                        Scope::values_of(classes(&format!(r"{c}\shell"))),
                        Scope::tree(classes(&format!(r"{c}\shell\{OPEN_VERB}"))),
                    ]
                })
                .collect(),
            Feature::ContextMenu => ["Directory", r"Directory\Background", "Drive"]
                .into_iter()
                .map(|c| Scope::tree(classes(&format!(r"{c}\shell\explorer2"))))
                .collect(),
        }
    }

    pub fn changes(self, exe: &Path) -> Vec<Change> {
        let set = |key: KeyPath, name: &str, data: RegData| Change::SetValue {
            key,
            name: name.into(),
            data,
        };
        match self {
            Feature::FolderOpen => ["Directory", "Drive"]
                .into_iter()
                .flat_map(|class| {
                    let shell = classes(&format!(r"{class}\shell"));
                    let verb = shell.child(OPEN_VERB);
                    [
                        set(verb.clone(), "", RegData::sz("Öffnen (Explorer 2.0)")),
                        set(
                            verb.clone(),
                            "Icon",
                            RegData::sz(&format!("\"{}\",0", exe.display())),
                        ),
                        set(verb.child("command"), "", RegData::sz(&command(exe, "%1"))),
                        // default verb of the class (double click, ShellExecute)
                        set(shell, "", RegData::sz(OPEN_VERB)),
                    ]
                })
                .collect(),
            Feature::ContextMenu => [
                ("Directory", "%1"),
                (r"Directory\Background", "%V"),
                ("Drive", "%1"),
            ]
            .into_iter()
            .flat_map(|(class, arg)| {
                let verb = classes(&format!(r"{class}\shell\explorer2"));
                [
                    set(verb.clone(), "", RegData::sz("In Explorer 2.0 öffnen")),
                    set(
                        verb.clone(),
                        "Icon",
                        RegData::sz(&format!("\"{}\",0", exe.display())),
                    ),
                    set(verb.child("command"), "", RegData::sz(&command(exe, arg))),
                ]
            })
            .collect(),
        }
    }
}

/// State of a feature on this machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Off,
    /// On and pointing to `exe`.
    On,
    /// On, but registered for another exe path (`String`) – e.g. the app was
    /// moved. Still ours; [`enable`] re-points it.
    OnElsewhere(String),
    /// Some values present, others missing or changed by someone else.
    Partial,
}

pub fn state<R: Registry + ?Sized>(reg: &R, f: Feature, exe: &Path) -> State {
    let changes = f.changes(exe);
    let mut exact = 0;
    let mut ours = 0;
    let mut other_exe = None;
    for c in &changes {
        if let Change::SetValue { key, name, data } = c {
            match reg.get(key, name) {
                Ok(Some(cur)) if &cur == data => exact += 1,
                Ok(Some(cur)) => {
                    let t = cur.text().unwrap_or_default();
                    if crate::manifest::is_ours(&t) {
                        ours += 1;
                        other_exe.get_or_insert(t);
                    }
                }
                _ => {}
            }
        }
    }
    let n = changes.len();
    if exact == n {
        State::On
    } else if exact + ours == n && ours > 0 {
        State::OnElsewhere(other_exe.unwrap_or_default())
    } else if exact == 0 && ours == 0 {
        State::Off
    } else {
        State::Partial
    }
}

/// Switch on (or re-point to `exe`); journaled.
pub fn enable<R: Registry + ?Sized>(
    reg: &mut R,
    paths: &JournalPaths,
    f: Feature,
    exe: &Path,
) -> Result<usize, String> {
    apply(reg, paths, &f.changes(exe))
}

/// Switch off: exactly the state before, for this feature only.
pub fn disable<R: Registry + ?Sized>(reg: &mut R, paths: &JournalPaths, f: Feature) -> ResetReport {
    revert_within(reg, paths, &f.scopes())
}
