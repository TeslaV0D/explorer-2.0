//! Checks the translation files against the sources (D-029):
//! - `lang/explorer2.pot` lists exactly the texts in the code,
//! - every `.po` belongs to a known language, has the right plural header,
//!   contains no text that is gone, keeps every placeholder,
//! - languages marked complete have every text translated.
//!
//! Out of date? `cargo run -p explorer-i18n --example extract`

use explorer_i18n::extract::{collect, placeholders, render_pot};
use explorer_i18n::{lang, po, LANGS};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

const FIX: &str = "run `cargo run -p explorer-i18n --example extract`";

#[test]
fn template_is_current() {
    let found = collect(&root()).expect("scanning the sources");
    let want = render_pot(&found);
    // line endings may be CRLF in a Windows checkout
    let have = std::fs::read_to_string(root().join("lang/explorer2.pot"))
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert!(have == want, "lang/explorer2.pot is out of date – {FIX}");
}

#[test]
fn pseudo_locale_is_current() {
    let found = collect(&root()).expect("scanning the sources");
    let want = po::write(&explorer_i18n::pseudo::catalog(&found));
    let have = std::fs::read_to_string(root().join("lang/qps/LC_MESSAGES/explorer2.po"))
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert!(have == want, "lang/qps is out of date – {FIX} -- qps");
}

#[test]
fn catalogs_are_consistent() {
    let found = collect(&root()).expect("scanning the sources");
    let dir = root().join("lang");
    let mut seen = Vec::new();
    for e in std::fs::read_dir(&dir).unwrap().flatten() {
        if !e.path().is_dir() {
            continue;
        }
        let code = e.file_name().to_string_lossy().to_string();
        let l = lang(&code).unwrap_or_else(|| panic!("lang/{code}: unknown language"));
        assert_ne!(
            code, "en",
            "English is the source language, it has no catalog"
        );
        let file = e.path().join("LC_MESSAGES/explorer2.po");
        let text =
            std::fs::read_to_string(&file).unwrap_or_else(|_| panic!("{} missing", file.display()));
        let cat = po::parse(&text).unwrap_or_else(|err| panic!("{}: {err}", file.display()));
        assert_eq!(
            cat.header_field("Plural-Forms"),
            Some(l.plural_forms),
            "{code}: Plural-Forms"
        );
        assert_eq!(
            cat.header_field("Language"),
            Some(code.as_str()),
            "{code}: Language"
        );
        let forms = explorer_i18n::plural_count(&code);
        let mut missing = Vec::new();
        for ent in &cat.entries {
            assert!(
                ent.ctx.is_none(),
                "{code}: {:?} has a context (not used)",
                ent.id
            );
            let key = explorer_i18n::extract::Msg {
                id: ent.id.clone(),
                plural: ent.id_plural.clone(),
            };
            assert!(
                found.contains_key(&key),
                "{code}: {:?} is no longer in the code – {FIX}",
                ent.id
            );
            let want = if ent.id_plural.is_some() { forms } else { 1 };
            assert_eq!(
                ent.strs.len(),
                want,
                "{code}: {:?} needs {want} form(s)",
                ent.id
            );
            if ent.is_translated() {
                let src = placeholders(ent.id_plural.as_deref().unwrap_or(&ent.id));
                for s in &ent.strs {
                    let mut p = placeholders(s);
                    // plural forms may leave out {n} (e.g. "one file" in words)
                    if ent.id_plural.is_some() {
                        p.retain(|x| x != "{n}");
                        let mut src = src.clone();
                        src.retain(|x| x != "{n}");
                        assert_eq!(p, src, "{code}: placeholders of {:?}", ent.id);
                    } else {
                        assert_eq!(p, src, "{code}: placeholders of {:?}", ent.id);
                    }
                }
            } else {
                missing.push(ent.id.clone());
            }
        }
        for m in found.keys() {
            if !cat
                .entries
                .iter()
                .any(|e| e.id == m.id && e.id_plural == m.plural)
            {
                missing.push(m.id.clone());
            }
        }
        if l.complete {
            assert!(
                missing.is_empty(),
                "{code} is marked complete, untranslated: {missing:?}"
            );
        }
        seen.push(code);
    }
    for l in LANGS.iter().filter(|l| l.complete && l.code != "en") {
        assert!(
            seen.iter().any(|c| c == l.code),
            "{} is marked complete but has no catalog",
            l.code
        );
    }
}

#[test]
fn compiled_tables_cover_every_catalog() {
    let mut on_disk: Vec<String> = std::fs::read_dir(root().join("lang"))
        .unwrap()
        .flatten()
        .filter(|e| e.path().join("LC_MESSAGES/explorer2.po").exists())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    on_disk.sort();
    let mut compiled: Vec<String> = explorer_i18n::compiled_languages()
        .into_iter()
        .map(String::from)
        .collect();
    compiled.sort();
    assert_eq!(compiled, on_disk);
}
