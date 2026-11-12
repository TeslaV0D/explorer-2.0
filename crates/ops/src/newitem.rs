//! Create one file from a “New” menu entry (empty, bytes, a template copy,
//! or a ShellNew command). The folder entry stays [`crate::simple::new_folder`].

use explorer_core::newmenu::{command_argv, NewKind, NewSpec};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::names::{unique_name, validate_name};

/// Redo stores at most this much. A larger file can still be undone (it is
/// moved to the Recycle Bin) but not recreated from memory.
const REDO_LIMIT: u64 = 256 * 1024;

/// How long a ShellNew command may block the caller before we carry on and
/// leave the process running (an editor must not freeze the window).
const COMMAND_WAIT: Duration = Duration::from_secs(3);

/// A file that was just created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Created {
    pub path: PathBuf,
    /// Bytes to write again on redo. `None` = no redo (too large, or the
    /// command was still running and the file was not ours to snapshot).
    pub redo: Option<Vec<u8>>,
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

/// Bytes to recreate `path`, or `None` when it is missing or over [`REDO_LIMIT`].
fn redo_of(path: &Path) -> Option<Vec<u8>> {
    let len = std::fs::metadata(path).ok()?.len();
    if len > REDO_LIMIT {
        return None;
    }
    Some(std::fs::read(path).unwrap_or_default())
}

/// Creates `spec` in `dir` under a free name (`name`, then `name (2)`, …).
pub fn create(dir: &Path, spec: &NewSpec) -> Result<Created, String> {
    validate_name(&spec.file_name)?;
    let name = unique_name(&spec.file_name, false, |n| {
        std::fs::symlink_metadata(dir.join(n)).is_ok()
    });
    validate_name(&name)?;
    let path = dir.join(&name);
    match &spec.kind {
        NewKind::Empty => {
            write_new(&path, b"")?;
            Ok(Created {
                path,
                redo: Some(Vec::new()),
            })
        }
        NewKind::Bytes(b) => {
            write_new(&path, b)?;
            let redo = if (b.len() as u64) <= REDO_LIMIT {
                Some(b.clone())
            } else {
                None
            };
            Ok(Created { path, redo })
        }
        NewKind::Copy(template) => {
            // a missing template becomes an empty file (the menu entry still works)
            if std::fs::copy(template, &path).is_err() {
                write_new(&path, b"")?;
            }
            let redo = redo_of(&path);
            Ok(Created { path, redo })
        }
        NewKind::Command(line) => run_command(dir, &path, line),
    }
}

fn run_command(dir: &Path, path: &Path, line: &str) -> Result<Created, String> {
    let shown = path.to_string_lossy();
    let argv = command_argv(line, &shown);
    let (program, args) = argv
        .split_first()
        .ok_or_else(|| explorer_i18n::tr!("The command for this item is empty.").to_string())?;
    let mut child = Command::new(program)
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            explorer_i18n::tr!(
                "Could not run the command for this item: {0}",
                e.to_string()
            )
        })?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() >= COMMAND_WAIT => {
                // Still running (an editor). Don't invent an empty file next
                // to whatever it is about to write.
                if path.exists() {
                    return Ok(Created {
                        path: path.to_path_buf(),
                        redo: redo_of(path),
                    });
                }
                return Err(
                    explorer_i18n::tr!("The command for this item is still running.").to_string(),
                );
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(30)),
            Err(e) => return Err(e.to_string()),
        }
    }
    if !path.exists() {
        write_new(path, b"")?;
    }
    Ok(Created {
        path: path.to_path_buf(),
        redo: redo_of(path),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use explorer_core::newmenu::{NewKind, NewSpec};

    fn spec(name: &str, kind: NewKind) -> NewSpec {
        NewSpec {
            id: "txt".into(),
            label: "Text".into(),
            file_name: name.into(),
            kind,
        }
    }

    #[test]
    fn empty_then_a_second_name() {
        let dir = tempfile::tempdir().unwrap();
        let a = create(dir.path(), &spec("Note.txt", NewKind::Empty)).unwrap();
        assert_eq!(std::fs::read(&a.path).unwrap(), b"");
        assert_eq!(a.redo.as_deref(), Some(&[][..]));
        let b = create(dir.path(), &spec("Note.txt", NewKind::Empty)).unwrap();
        assert_ne!(a.path, b.path);
        assert!(b.path.ends_with("Note (2).txt"));
    }

    #[test]
    fn bytes_and_a_template_copy() {
        let dir = tempfile::tempdir().unwrap();
        let a = create(dir.path(), &spec("A.bin", NewKind::Bytes(b"abc".to_vec()))).unwrap();
        assert_eq!(std::fs::read(&a.path).unwrap(), b"abc");
        let template = dir.path().join("template.dat");
        std::fs::write(&template, b"from-template").unwrap();
        let b = create(
            dir.path(),
            &spec(
                "B.dat",
                NewKind::Copy(template.to_string_lossy().into_owned()),
            ),
        )
        .unwrap();
        assert_eq!(std::fs::read(&b.path).unwrap(), b"from-template");
        let missing = create(
            dir.path(),
            &spec("C.dat", NewKind::Copy("/no/such/template".into())),
        )
        .unwrap();
        assert_eq!(std::fs::read(&missing.path).unwrap(), b"");
    }

    #[cfg(unix)]
    #[test]
    fn command_that_exits_without_a_file_leaves_an_empty_one() {
        let dir = tempfile::tempdir().unwrap();
        let created = create(
            dir.path(),
            &spec("D.txt", NewKind::Command("/bin/true".into())),
        )
        .unwrap();
        assert!(created.path.exists());
        assert_eq!(std::fs::read(&created.path).unwrap(), b"");
    }
}
