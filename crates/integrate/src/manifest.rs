//! Every integration point Explorer 2.0 may ever touch, for the reset without
//! a (readable) journal. Windows' default for all of them is "no HKCU
//! override", so a value is removed when it is recognisably ours (its text
//! names `explorer2`), then empty keys are pruned up to `stop`.
//!
//! **Every new integration (M7+) must add its points here** – the test
//! `manifest_covers_known_points` and the CI snapshot test rely on it.

use crate::model::KeyPath;

/// Text that identifies our values (the command line names our exe).
pub const MARKER: &str = "explorer2";

#[derive(Debug, Clone)]
pub enum Point {
    /// `probe` value of `key` is ours if its text contains [`MARKER`]; then it
    /// and `also` are deleted and empty keys pruned up to (not incl.) `stop`.
    Value {
        key: KeyPath,
        probe: &'static str,
        also: &'static [&'static str],
        stop: KeyPath,
    },
    /// A shell verb only we use (`…\shell\explorer2`): removed with
    /// everything below, then empty parents pruned up to (not incl.) `stop`.
    OwnVerb { key: KeyPath, stop: KeyPath },
    /// A key that only we use: removed with everything below.
    OwnTree(KeyPath),
}

fn verb(class: &str, verb: &str) -> Point {
    Point::Value {
        key: KeyPath::hkcu(&format!(r"Software\Classes\{class}\shell\{verb}\command")),
        probe: "",
        also: &["DelegateExecute"],
        stop: KeyPath::hkcu(r"Software\Classes"),
    }
}

/// The known points (see WINDOWS-INTEGRATION.md, stages 1–2).
pub fn points() -> Vec<Point> {
    let mut v = Vec::new();
    // stage 1: folder verbs ("open"/"explore" of folders, directories, drives)
    for class in ["Folder", "Directory", "Drive"] {
        for vb in ["open", "explore"] {
            v.push(verb(class, vb));
        }
    }
    // M7.1 "open folders": own default verb + the class' default verb value
    for class in ["Directory", "Drive"] {
        v.push(Point::OwnVerb {
            key: KeyPath::hkcu(&format!(r"Software\Classes\{class}\shell\explorer2open")),
            stop: KeyPath::hkcu(r"Software\Classes"),
        });
        v.push(Point::Value {
            key: KeyPath::hkcu(&format!(r"Software\Classes\{class}\shell")),
            probe: "",
            also: &[],
            stop: KeyPath::hkcu(r"Software\Classes"),
        });
    }
    // own context menu entries ("In Explorer 2.0 öffnen")
    for class in ["Directory", r"Directory\Background", "Drive"] {
        v.push(Point::OwnVerb {
            key: KeyPath::hkcu(&format!(r"Software\Classes\{class}\shell\explorer2")),
            stop: KeyPath::hkcu(r"Software\Classes"),
        });
    }
    // stage 2: Win+E ("This PC" opens a new window)
    v.push(verb(
        r"CLSID\{52205FD8-5DFB-447D-801A-D0B52F2E83E1}",
        "opennewwindow",
    ));
    // autostart (D-028): one value in Run; the key itself belongs to Windows
    v.push(Point::Value {
        key: KeyPath::hkcu(r"Software\Microsoft\Windows\CurrentVersion\Run"),
        probe: "Explorer2",
        also: &[],
        // `Software` already exists; stopping at CurrentVersion would leave
        // the empty keys we created on the way down
        stop: KeyPath::hkcu("Software"),
    });
    // our own namespace (selftest, future per-user state)
    v.push(Point::OwnTree(KeyPath::hkcu(r"Software\Explorer2")));
    // M8: the "Apps & Features" entry of the installation
    v.push(Point::OwnTree(crate::install::uninstall_key()));
    v
}

/// Case-insensitive "is this value ours".
pub fn is_ours(text: &str) -> bool {
    text.to_lowercase().contains(MARKER)
}
