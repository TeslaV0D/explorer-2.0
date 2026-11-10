//! Finds translatable texts in the sources: `@tr(…)` in `.slint`,
//! `tr!(…)`/`tr_n!(…)` in Rust. Used by the `extract` example (writes the
//! `.pot`/`.po` files) and by the catalog test (checks they are current).
//!
//! Rules kept simple on purpose: the text must be a plain string literal
//! (no raw strings, no `concat!`), Slint contexts (`"ctx" => …`) are not used.

use crate::po::{self, Catalog, Entry};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Msg {
    pub id: String,
    pub plural: Option<String>,
}

/// Found texts with the files they come from (sorted, for stable output).
pub type Found = BTreeMap<Msg, Vec<String>>;

fn skip_ws(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && b[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

fn line_of(b: &[u8], i: usize) -> usize {
    b[..i.min(b.len())].iter().filter(|&&c| c == b'\n').count() + 1
}

/// Reads a `"…"` literal starting at `i` (the quote). Returns text + index after.
fn read_string(text: &str, i: usize, rust: bool) -> Result<(String, usize), String> {
    let b = text.as_bytes();
    if b.get(i) != Some(&b'"') {
        return Err("expected a string literal".into());
    }
    let mut out = String::new();
    let mut chars = text[i + 1..].char_indices();
    while let Some((off, c)) = chars.next() {
        match c {
            '"' => return Ok((out, i + 1 + off + 1)),
            '\\' => match chars.next().map(|(_, c)| c) {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') if rust => out.push('\r'),
                Some('0') if rust => out.push('\0'),
                Some('"') => out.push('"'),
                Some('\'') if rust => out.push('\''),
                Some('\\') => out.push('\\'),
                Some('u') => {
                    let mut hex = String::new();
                    if chars.next().map(|(_, c)| c) != Some('{') {
                        return Err("bad \\u escape".into());
                    }
                    for (_, c) in chars.by_ref() {
                        if c == '}' {
                            break;
                        }
                        hex.push(c);
                    }
                    let v = u32::from_str_radix(&hex, 16).map_err(|_| "bad \\u escape")?;
                    out.push(char::from_u32(v).ok_or("bad \\u escape")?);
                }
                Some('\n') if rust => {
                    // line continuation: skip the following whitespace
                    let ws = chars.clone().take_while(|(_, c)| c.is_whitespace()).count();
                    for _ in 0..ws {
                        chars.next();
                    }
                }
                Some('{') if !rust => return Err("`\\{` (interpolation) inside @tr".into()),
                other => return Err(format!("unsupported escape \\{}", other.unwrap_or(' '))),
            },
            c => out.push(c),
        }
    }
    Err("unterminated string".into())
}

/// Skips comments and string/char literals; calls `hit(i)` for every code position.
fn walk_code(
    text: &str,
    rust: bool,
    mut hit: impl FnMut(usize) -> Result<Option<usize>, String>,
) -> Result<(), String> {
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b'/' && b.get(i + 1) == Some(&b'*') {
            i = text[i + 2..].find("*/").map_or(b.len(), |e| i + 2 + e + 2);
            continue;
        }
        if c == b'"' {
            // any other string (byte strings with \x.., …): only find its end;
            // a strict read here once skipped the rest of a file silently
            i = skip_string(b, i);
            continue;
        }
        if rust
            && c == b'r'
            && (b.get(i + 1) == Some(&b'"') || b.get(i + 1) == Some(&b'#'))
            && (i == 0 || !is_ident(b[i - 1]))
        {
            // raw string r"…" / r#"…"#
            let mut j = i + 1;
            let mut hashes = 0;
            while b.get(j) == Some(&b'#') {
                hashes += 1;
                j += 1;
            }
            if b.get(j) == Some(&b'"') {
                let close = format!("\"{}", "#".repeat(hashes));
                i = text[j + 1..]
                    .find(&close)
                    .map_or(b.len(), |e| j + 1 + e + close.len());
                continue;
            }
        }
        if rust && c == b'\'' {
            // char literal ('x', '\n', '\u{..}') vs lifetime ('a)
            if b.get(i + 1) == Some(&b'\\') {
                i = text[i + 2..].find('\'').map_or(b.len(), |e| i + 2 + e + 1);
                continue;
            }
            if let Some(ch) = text[i + 1..].chars().next() {
                let after = i + 1 + ch.len_utf8();
                if b.get(after) == Some(&b'\'') {
                    i = after + 1;
                    continue;
                }
            }
        }
        if let Some(next) = hit(i)? {
            i = next;
            continue;
        }
        i += 1;
    }
    Ok(())
}

/// End of a `"…"` literal starting at `i` (after the closing quote), escapes
/// skipped as "backslash + next byte" – good for every Rust/Slint string.
fn skip_string(b: &[u8], i: usize) -> usize {
    let mut j = i + 1;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 2,
            b'"' => return j + 1,
            _ => j += 1,
        }
    }
    b.len()
}

/// How often a call of ours appears in code lines (not in `//` comments) –
/// the scanner must find exactly as many, or a text would be lost silently.
fn raw_calls(text: &str, rust: bool) -> usize {
    let pats: &[&str] = if rust {
        &["tr!(", "tr_n!(", "tr_noop!("]
    } else {
        &["@tr("]
    };
    text.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .map(|l| {
            pats.iter()
                .map(|p| {
                    l.match_indices(p)
                        .filter(|(at, _)| {
                            let before = l[..*at].bytes().last();
                            !before.is_some_and(|c| is_ident(c) || c == b'!')
                        })
                        .count()
                })
                .sum::<usize>()
        })
        .sum()
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// `@tr("text")`, `@tr("text {}", x)`, `@tr("one" | "many" % n)`.
pub fn scan_slint(text: &str) -> Result<Vec<(Msg, usize)>, String> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    walk_code(text, false, |i| {
        if !text[i..].starts_with("@tr(") {
            return Ok(None);
        }
        let at = |m: &str| format!("line {}: {m}", line_of(b, i));
        let j = skip_ws(b, i + 4);
        let (id, mut j) = read_string(text, j, false).map_err(|e| at(&e))?;
        j = skip_ws(b, j);
        if text[j..].starts_with("=>") {
            return Err(at("translation contexts are not used (D-029)"));
        }
        let mut plural = None;
        if b.get(j) == Some(&b'|') {
            let k = skip_ws(b, j + 1);
            let (p, k) = read_string(text, k, false).map_err(|e| at(&e))?;
            if b.get(skip_ws(b, k)) != Some(&b'%') {
                return Err(at("plural without `% n`"));
            }
            plural = Some(p);
            j = k;
        }
        out.push((Msg { id, plural }, line_of(b, i)));
        Ok(Some(j))
    })?;
    Ok(out)
}

/// `tr!("text")`, `tr!("text {}", x)`, `tr_n!(n, "one", "many", …)`.
pub fn scan_rust(text: &str) -> Result<Vec<(Msg, usize)>, String> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    walk_code(text, true, |i| {
        if i > 0 && (is_ident(b[i - 1]) || b[i - 1] == b'!') {
            return Ok(None);
        }
        let (plural, open) = if text[i..].starts_with("tr!(") {
            (false, 4)
        } else if text[i..].starts_with("tr_noop!(") {
            (false, 9)
        } else if text[i..].starts_with("tr_n!(") {
            (true, 6)
        } else {
            return Ok(None);
        };
        let at = |m: &str| format!("line {}: {m}", line_of(b, i));
        let mut j = i + open;
        if plural {
            // skip the count expression up to the first top-level comma
            let mut depth = 0i32;
            loop {
                match b.get(j) {
                    None => return Err(at("unterminated tr_n!")),
                    Some(b'(' | b'[' | b'{') => depth += 1,
                    Some(b')' | b']' | b'}') => depth -= 1,
                    Some(b',') if depth == 0 => break,
                    _ => {}
                }
                j += 1;
            }
            j += 1;
        }
        j = skip_ws(b, j);
        let (id, k) = read_string(text, j, true)
            .map_err(|e| at(&format!("{e} (the text must be a plain string literal)")))?;
        let mut msg = Msg { id, plural: None };
        let mut end = k;
        if plural {
            let k = skip_ws(b, k);
            if b.get(k) != Some(&b',') {
                return Err(at("tr_n! needs a singular and a plural text"));
            }
            let k = skip_ws(b, k + 1);
            let (p, k) = read_string(text, k, true).map_err(|e| at(&e))?;
            msg.plural = Some(p);
            end = k;
        }
        out.push((msg, line_of(b, i)));
        Ok(Some(end))
    })?;
    Ok(out)
}

fn walk_files(dir: &Path, ext: &str, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk_files(&p, ext, out);
        } else if p.extension().is_some_and(|x| x == ext) {
            out.push(p);
        }
    }
}

/// All texts of the workspace (`crates/*/src` + `crates/*/ui`), except this crate.
pub fn collect(root: &Path) -> Result<Found, String> {
    let mut files = Vec::new();
    let crates = root.join("crates");
    let rd = std::fs::read_dir(&crates).map_err(|e| format!("{}: {e}", crates.display()))?;
    let mut dirs: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    dirs.sort();
    for c in dirs {
        if c.file_name().is_some_and(|n| n == "i18n") {
            continue;
        }
        walk_files(&c.join("src"), "rs", &mut files);
        walk_files(&c.join("ui"), "slint", &mut files);
    }
    files.sort();
    let mut found = Found::new();
    for f in files {
        let text = std::fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
        let rel = f
            .strip_prefix(root)
            .unwrap_or(&f)
            .to_string_lossy()
            .replace('\\', "/");
        let msgs = if rel.ends_with(".slint") {
            scan_slint(&text)
        } else {
            scan_rust(&text)
        }
        .map_err(|e| format!("{rel}: {e}"))?;
        // every call in the code must have been read (never lose a text silently)
        let raw = raw_calls(&text, !rel.ends_with(".slint"));
        if raw != msgs.len() {
            return Err(format!(
                "{rel}: read {} of {raw} translation calls – one is not a plain string literal?",
                msgs.len()
            ));
        }
        for (m, _) in msgs {
            let refs = found.entry(m).or_default();
            if !refs.contains(&rel) {
                refs.push(rel.clone());
            }
        }
    }
    Ok(found)
}

const POT_HEADER: &str = "Project-Id-Version: explorer2\nMIME-Version: 1.0\nContent-Type: text/plain; charset=UTF-8\nContent-Transfer-Encoding: 8bit\n";

fn refs_comment(refs: &[String]) -> Vec<String> {
    refs.iter().map(|r| format!("#: {r}")).collect()
}

/// The template (`lang/explorer2.pot`): every text, untranslated.
pub fn render_pot(found: &Found) -> String {
    let cat = Catalog {
        header: POT_HEADER.into(),
        entries: found
            .iter()
            .map(|(m, refs)| Entry {
                ctx: None,
                id: m.id.clone(),
                id_plural: m.plural.clone(),
                strs: if m.plural.is_some() {
                    vec![String::new(); 2]
                } else {
                    vec![String::new()]
                },
                fuzzy: false,
                comments: refs_comment(refs),
            })
            .collect(),
    };
    po::write(&cat)
}

/// Header of a language's `.po` file.
pub fn po_header(code: &str) -> String {
    let forms = crate::lang(code).map_or("nplurals=2; plural=(n != 1);", |l| l.plural_forms);
    format!("{POT_HEADER}Language: {code}\nPlural-Forms: {forms}\n")
}

/// Brings a language's catalog in line with the sources: keeps existing
/// translations, adds new texts untranslated, drops texts that are gone.
pub fn merge(existing: Option<&Catalog>, found: &Found, code: &str) -> Catalog {
    let forms = crate::plural_count(code);
    let entries = found
        .iter()
        .map(|(m, refs)| {
            let old = existing.and_then(|c| {
                c.entries
                    .iter()
                    .find(|e| e.ctx.is_none() && e.id == m.id && e.id_plural == m.plural)
            });
            let want = if m.plural.is_some() { forms } else { 1 };
            let mut strs = old.map(|e| e.strs.clone()).unwrap_or_default();
            strs.resize(want, String::new());
            Entry {
                ctx: None,
                id: m.id.clone(),
                id_plural: m.plural.clone(),
                strs,
                fuzzy: old.is_some_and(|e| e.fuzzy),
                comments: refs_comment(refs),
            }
        })
        .collect();
    Catalog {
        header: po_header(code),
        entries,
    }
}

/// Placeholders of a text (`{}`, `{0}`, `{n}` …), sorted – translations must keep them.
pub fn placeholders(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(i) = rest.find('{') {
        let tail = &rest[i..];
        if let Some(after) = tail.strip_prefix("{{") {
            rest = after;
            continue;
        }
        match tail.find('}') {
            Some(e) => {
                out.push(tail[..=e].to_string());
                rest = &tail[e + 1..];
            }
            None => break,
        }
    }
    out.sort();
    out
}
