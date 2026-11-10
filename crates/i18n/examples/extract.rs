//! Refreshes the translation files from the sources (D-029):
//! writes `lang/explorer2.pot` and brings every `lang/<code>/LC_MESSAGES/explorer2.po`
//! up to date (new texts untranslated, removed texts dropped, translations kept).
//!
//!   cargo run -p explorer-i18n --example extract            # all existing catalogs
//!   cargo run -p explorer-i18n --example extract -- fr ja   # also create these

use explorer_i18n::extract::{collect, merge, render_pot};
use explorer_i18n::po;
use std::io::Write as _;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let found = match collect(&root) {
        Ok(f) => f,
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "error: {e}");
            std::process::exit(1);
        }
    };
    let lang = root.join("lang");
    std::fs::create_dir_all(&lang).unwrap();
    std::fs::write(lang.join("explorer2.pot"), render_pot(&found)).unwrap();

    let mut codes: Vec<String> = std::fs::read_dir(&lang)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    for extra in std::env::args().skip(1) {
        if explorer_i18n::lang(&extra).is_none() || extra == explorer_i18n::SOURCE {
            let _ = writeln!(std::io::stderr(), "error: {extra} is not a target language");
            std::process::exit(1);
        }
        if !codes.contains(&extra) {
            codes.push(extra);
        }
    }
    codes.sort();
    for code in codes {
        let file = lang.join(&code).join("LC_MESSAGES").join("explorer2.po");
        let old = std::fs::read_to_string(&file)
            .ok()
            .map(|t| po::parse(&t).unwrap_or_else(|e| panic!("{}: {e}", file.display())));
        let cat = merge(old.as_ref(), &found, &code);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, po::write(&cat)).unwrap();
        let done = cat.entries.iter().filter(|e| e.is_translated()).count();
        let _ = writeln!(
            std::io::stdout(),
            "{code}: {done}/{} translated",
            cat.entries.len()
        );
    }
    let _ = writeln!(std::io::stdout(), "texts: {}", found.len());
}
