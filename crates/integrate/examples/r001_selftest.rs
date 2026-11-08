//! CI helper for the R-001 snapshot test (not shipped): applies a test plan
//! through the real journal/apply path. The check itself is independent of
//! our code: PowerShell exports the registry before/after (`reg export`).
//!
//!   r001_selftest full <file>      every kind of change (journal reset)
//!   r001_selftest manifest         only known points (reset without journal)
//!   r001_selftest features         every M7 feature, through the features API

use explorer_integrate::{apply, Change, JournalPaths, KeyPath, RegData};
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let exe = r#""C:\Program Files\Explorer2\explorer2.exe" "%1""#;
    let verb = KeyPath::hkcu(r"Software\Classes\Directory\shell\open\command");
    let mut plan = vec![
        // stage 1 folder verb (a real integration point)
        Change::SetValue {
            key: verb.clone(),
            name: "".into(),
            data: RegData::sz(exe),
        },
        Change::SetValue {
            key: verb,
            name: "DelegateExecute".into(),
            data: RegData::sz(""),
        },
        // own namespace, nested keys, several types
        Change::SetValue {
            key: KeyPath::hkcu(r"Software\Explorer2\Selftest\deep"),
            name: "n".into(),
            data: RegData::dword(42),
        },
    ];
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["features"] => {
            let paths = JournalPaths::default_paths().expect("config dir");
            let mut reg = explorer_integrate::system_registry().expect("Windows only");
            let exe = std::path::Path::new(r"C:\Program Files\Explorer2\explorer2.exe");
            for f in explorer_integrate::features::ALL {
                match explorer_integrate::features::enable(reg.as_mut(), &paths, f, exe) {
                    Ok(n) => {
                        let _ = writeln!(std::io::stdout(), "feature {} on: {n} changes", f.id());
                    }
                    Err(e) => {
                        let _ = writeln!(std::io::stderr(), "feature {} failed: {e}", f.id());
                        std::process::exit(1);
                    }
                }
            }
            return;
        }
        ["manifest"] => {}
        ["full", file] => {
            plan.extend([
                // overwrite a pre-existing foreign value (set up by CI)
                Change::SetValue {
                    key: KeyPath::hkcu(r"Software\Explorer2R001Probe"),
                    name: "keep".into(),
                    data: RegData::sz("overwritten by explorer2"),
                },
                // user environment (+ WM_SETTINGCHANGE)
                Change::SetValue {
                    key: KeyPath::hkcu("Environment"),
                    name: "EXPLORER2_R001".into(),
                    data: RegData::expand_sz(r"%LOCALAPPDATA%\Explorer2"),
                },
                // a file (e.g. a start menu shortcut)
                Change::WriteFile {
                    path: file.into(),
                    bytes: b"explorer2 selftest".to_vec(),
                },
            ]);
        }
        _ => {
            let _ = writeln!(
                std::io::stderr(),
                "usage: r001_selftest full <file> | manifest"
            );
            std::process::exit(2);
        }
    }
    let paths = JournalPaths::default_paths().expect("config dir");
    let mut reg = explorer_integrate::system_registry().expect("Windows only");
    match apply(reg.as_mut(), &paths, &plan) {
        Ok(n) => {
            let _ = writeln!(
                std::io::stdout(),
                "applied {n} changes, journal {}",
                paths.primary.display()
            );
        }
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "apply failed: {e}");
            std::process::exit(1);
        }
    }
}
