//! “New” menu like Windows Explorer: what can be created in a folder.
//!
//! The shell (Windows) supplies [`ShellNewRecord`]s from `HKCR\.<ext>\ShellNew`.
//! This module turns them into menu entries. It does not touch the disk and
//! does not run commands — creation lives in `explorer-ops`.
//!
//! Handler-only ShellNew entries (a COM object and nothing else) are dropped:
//! there is nothing we can create without that object.

use std::collections::HashSet;
use std::path::Path;

/// One `ShellNew` key, already read (the registry walk is platform code).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellNewRecord {
    /// Extension without the dot, lower case (`txt`).
    pub extension: String,
    /// Name shown in the menu. Empty = fall back to “New” + the extension.
    pub menu_text: String,
    /// Command line with `%1` for the new file. Empty = none.
    pub command: String,
    /// Raw `Data` value. Empty = none.
    pub data: Vec<u8>,
    /// Template file (`FileName`), absolute or relative to the ShellNew folder.
    pub file_name: String,
    /// The `NullFile` value is present (create an empty file).
    pub null_file: bool,
}

/// How a new file is made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewKind {
    Empty,
    Bytes(Vec<u8>),
    /// Copy this template (absolute path).
    Copy(String),
    /// Run this command line; `%1` is the new file.
    Command(String),
}

/// One entry of the New menu, apart from the folder (that one already exists).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSpec {
    /// Stable id: the extension (`txt`). Used as `new:<id>` in the menu.
    pub id: String,
    /// Menu label in the current language (shell names stay as Windows wrote them).
    pub label: String,
    /// File name to create (`New Text Document.txt`), before the “(2)” suffix.
    pub file_name: String,
    pub kind: NewKind,
}

/// The built-in text document, used when the shell list has no `.txt`.
pub fn text_document() -> NewSpec {
    NewSpec {
        id: "txt".into(),
        label: explorer_i18n::tr!("Text Document").into(),
        file_name: format!("{}.txt", explorer_i18n::tr!("New Text Document")),
        kind: NewKind::Empty,
    }
}

fn ok_extension(ext: &str) -> bool {
    !ext.is_empty() && ext.len() <= 20 && ext.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Characters Windows rejects in a file name, plus controls.
fn sanitize(text: &str) -> String {
    let s: String = text
        .chars()
        .map(|c| {
            if "<>:\"/\\|?*".contains(c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let s = s.trim().trim_end_matches(['.', ' ']);
    if s.is_empty() {
        explorer_i18n::tr!("New").into()
    } else {
        s.to_string()
    }
}

fn template_path(file_name: &str, root: &str) -> String {
    if file_name.is_empty() {
        return String::new();
    }
    // Windows paths show up in tests on Linux too, so don't use `Path::join`
    // (that would insert `/` into `C:\Windows\ShellNew`).
    let abs = file_name.contains(':')
        || file_name.starts_with('\\')
        || file_name.starts_with('/')
        || Path::new(file_name).is_absolute();
    if abs || root.is_empty() {
        return file_name.to_string();
    }
    let sep = if root.contains('\\') { '\\' } else { '/' };
    format!("{}{sep}{file_name}", root.trim_end_matches(['\\', '/']))
}

/// One shell record → a menu entry. `None` when there is nothing we can create
/// (no extension, or only a COM handler).
pub fn from_record(r: &ShellNewRecord, template_root: &str) -> Option<NewSpec> {
    if !ok_extension(&r.extension) {
        return None;
    }
    let kind = if !r.command.trim().is_empty() {
        NewKind::Command(r.command.trim().to_string())
    } else if !r.data.is_empty() {
        NewKind::Bytes(r.data.clone())
    } else if !r.file_name.trim().is_empty() {
        NewKind::Copy(template_path(r.file_name.trim(), template_root))
    } else if r.null_file {
        NewKind::Empty
    } else {
        return None;
    };
    let stem = if r.menu_text.trim().is_empty() {
        explorer_i18n::tr!("New").to_string()
    } else {
        format!(
            "{} {}",
            explorer_i18n::tr!("New"),
            sanitize(r.menu_text.trim())
        )
    };
    let label = if r.menu_text.trim().is_empty() {
        format!(".{}", r.extension)
    } else {
        sanitize(r.menu_text.trim())
    };
    Some(NewSpec {
        id: r.extension.clone(),
        label,
        file_name: format!("{stem}.{}", r.extension),
        kind,
    })
}

/// Menu entries from the shell list: shell items sorted by label, the built-in
/// text document first when `.txt` is not already there. The same extension
/// once (the first record wins). The folder is not included.
pub fn menu_specs(records: &[ShellNewRecord], template_root: &str) -> Vec<NewSpec> {
    let mut specs = Vec::new();
    let mut seen = HashSet::new();
    for r in records {
        if let Some(s) = from_record(r, template_root) {
            if seen.insert(s.id.clone()) {
                specs.push(s);
            }
        }
    }
    specs.sort_by(|a, b| {
        a.label
            .to_lowercase()
            .cmp(&b.label.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    if !seen.contains("txt") {
        specs.insert(0, text_document());
    }
    specs
}

/// Split a ShellNew command line into arguments. `%1` (a token, or inside one)
/// becomes `path`. When the line never mentions `%1`, `path` is appended.
/// Quotes group words; there is no shell.
pub fn command_argv(command: &str, path: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote = false;
    let mut saw = false;
    let push = |out: &mut Vec<String>, tok: &str, saw: &mut bool| {
        if tok.is_empty() {
            return;
        }
        if tok == "%1" {
            out.push(path.to_string());
            *saw = true;
        } else if tok.contains("%1") {
            out.push(tok.replace("%1", path));
            *saw = true;
        } else {
            out.push(tok.to_string());
        }
    };
    for c in command.chars() {
        match c {
            '"' => quote = !quote,
            c if c.is_whitespace() && !quote => {
                push(&mut out, &cur, &mut saw);
                cur.clear();
            }
            c => cur.push(c),
        }
    }
    push(&mut out, &cur, &mut saw);
    if !saw && !out.is_empty() {
        out.push(path.to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(ext: &str) -> ShellNewRecord {
        ShellNewRecord {
            extension: ext.into(),
            menu_text: String::new(),
            command: String::new(),
            data: Vec::new(),
            file_name: String::new(),
            null_file: true,
        }
    }

    #[test]
    fn text_document_is_used_when_the_shell_has_nothing() {
        let s = menu_specs(&[], "");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].id, "txt");
        assert_eq!(s[0].file_name, "New Text Document.txt");
        assert_eq!(s[0].kind, NewKind::Empty);
    }

    #[test]
    fn shell_txt_replaces_the_builtin_and_duplicates_drop() {
        let mut a = rec("txt");
        a.menu_text = "Text Document".into();
        a.null_file = false;
        a.data = b"hello".to_vec();
        let mut b = a.clone();
        b.command = "ignored-second".into();
        let s = menu_specs(&[a, b, rec("bmp")], "");
        assert!(s.iter().filter(|x| x.id == "txt").count() == 1);
        let txt = s.iter().find(|x| x.id == "txt").unwrap();
        assert_eq!(txt.kind, NewKind::Bytes(b"hello".to_vec()));
        assert_eq!(txt.file_name, "New Text Document.txt");
        assert!(s.iter().any(|x| x.id == "bmp"));
    }

    #[test]
    fn handler_only_and_bad_extensions_are_skipped() {
        let mut handler = rec("doc");
        handler.null_file = false;
        let mut weird = rec("a/b");
        weird.null_file = true;
        assert!(from_record(&handler, "").is_none());
        assert!(from_record(&weird, "").is_none());
        assert!(from_record(&rec(""), "").is_none());
    }

    #[test]
    fn kind_priority_and_template_path() {
        let mut r = rec("docx");
        r.command = "  writer.exe %1 ".into();
        r.data = b"x".to_vec();
        r.file_name = "winword.docx".into();
        assert!(matches!(
            from_record(&r, r"C:\Windows\ShellNew").unwrap().kind,
            NewKind::Command(_)
        ));
        r.command.clear();
        assert!(matches!(
            from_record(&r, "").unwrap().kind,
            NewKind::Bytes(_)
        ));
        r.data.clear();
        match from_record(&r, r"C:\Windows\ShellNew").unwrap().kind {
            NewKind::Copy(p) => assert_eq!(p, r"C:\Windows\ShellNew\winword.docx"),
            other => panic!("{other:?}"),
        }
        r.file_name = r"D:\Templates\a.docx".into();
        match from_record(&r, r"C:\Windows\ShellNew").unwrap().kind {
            NewKind::Copy(p) => assert_eq!(p, r"D:\Templates\a.docx"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn command_line_replaces_percent_one_without_a_shell() {
        assert_eq!(
            command_argv(r#"notepad.exe "%1""#, r"C:\a b.txt"),
            vec!["notepad.exe", r"C:\a b.txt"]
        );
        assert_eq!(
            command_argv(r#""C:\Program Files\app.exe" /n %1"#, r"D:\x.txt"),
            vec![r"C:\Program Files\app.exe", "/n", r"D:\x.txt"]
        );
        assert_eq!(
            command_argv("/bin/true", "/tmp/a.txt"),
            vec!["/bin/true", "/tmp/a.txt"]
        );
        assert!(command_argv("   ", "/tmp/a.txt").is_empty());
    }
}
