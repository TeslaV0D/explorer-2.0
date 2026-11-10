//! Compiles `lang/<code>/LC_MESSAGES/explorer2.po` into static tables
//! (`OUT_DIR/catalog.rs`) so Rust texts are translated without gettext.
//! Only translated, non-fuzzy entries are kept; the rest falls back to English.

#[path = "src/po.rs"]
#[allow(dead_code)]
mod po;

use std::fmt::Write as _;
use std::path::Path;

fn main() {
    let lang = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lang");
    println!("cargo:rerun-if-changed={}", lang.display());
    let mut dirs: Vec<_> = std::fs::read_dir(&lang)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default();
    dirs.sort();

    let mut singular =
        String::from("pub(crate) static SINGULAR: &[(&str, &[(&str, &str)])] = &[\n");
    let mut plural =
        String::from("pub(crate) static PLURAL: &[(&str, &[(&str, &str, &[&str])])] = &[\n");
    for dir in dirs {
        let code = dir.file_name().unwrap().to_string_lossy().to_string();
        let file = dir.join("LC_MESSAGES").join("explorer2.po");
        println!("cargo:rerun-if-changed={}", file.display());
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let cat = po::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        let mut one: Vec<(&str, &str)> = Vec::new();
        let mut many: Vec<(&str, &str, &[String])> = Vec::new();
        for e in cat
            .entries
            .iter()
            .filter(|e| e.is_translated() && e.ctx.is_none())
        {
            match &e.id_plural {
                Some(p) => many.push((&e.id, p, &e.strs)),
                None => one.push((&e.id, &e.strs[0])),
            }
        }
        one.sort();
        one.dedup_by(|a, b| a.0 == b.0);
        many.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
        let _ = write!(singular, "    ({code:?}, &[");
        for (k, v) in one {
            let _ = write!(singular, "({k:?}, {v:?}),");
        }
        singular.push_str("]),\n");
        let _ = write!(plural, "    ({code:?}, &[");
        for (s, p, forms) in many {
            let _ = write!(plural, "({s:?}, {p:?}, &[");
            for f in forms {
                let _ = write!(plural, "{f:?},");
            }
            plural.push_str("]),");
        }
        plural.push_str("]),\n");
    }
    singular.push_str("];\n");
    plural.push_str("];\n");
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("catalog.rs");
    std::fs::write(out, singular + &plural).unwrap();
}
