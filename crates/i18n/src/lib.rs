//! Localisation (D-029): languages, the current language, translation of
//! Rust texts and plural rules.
//!
//! - Source language is **English**: `tr!("Copy")` returns the English text
//!   unless a translation exists for the current language.
//! - Translations live in `lang/<code>/LC_MESSAGES/explorer2.po` (gettext).
//!   The same files are bundled into the Slint UI (`@tr`) and compiled into a
//!   static table here by `build.rs` – no gettext runtime, no extra files.
//! - `cargo run -p explorer-i18n --example extract` refreshes
//!   `lang/explorer2.pot` and the `.po` files from the sources; a test fails
//!   when they are out of date.
//!
//! Placeholders work like Slint's `@tr`: `{}` = next argument, `{0}`/`{1}` =
//! by position, `{n}` = the count of a plural, `{{`/`}}` = literal braces.

pub mod extract;
pub mod po;
pub mod pseudo;
mod system;

pub use system::{init, system_languages};

use std::fmt::{Display, Write as _};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A supported language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lang {
    /// Folder name under `lang/`, also the Slint bundle name.
    pub code: &'static str,
    /// Name in the language itself (shown in the language list).
    pub native: &'static str,
    pub english: &'static str,
    /// gettext `Plural-Forms` header; must match [`plural_index`].
    pub plural_forms: &'static str,
    /// Every text translated: only complete languages are offered and
    /// picked automatically from the Windows language.
    pub complete: bool,
}

const TWO: &str = "nplurals=2; plural=(n != 1);";
const SLAVIC: &str =
    "nplurals=3; plural=(n%10==1 && n%100!=11 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2);";

/// All languages (D-029). `en` is the source language and always complete.
pub const LANGS: &[Lang] = &[
    Lang {
        code: "en",
        native: "English",
        english: "English",
        plural_forms: TWO,
        complete: true,
    },
    Lang {
        code: "de",
        native: "Deutsch",
        english: "German",
        plural_forms: TWO,
        complete: true,
    },
    Lang {
        code: "es",
        native: "Español",
        english: "Spanish",
        plural_forms: TWO,
        complete: true,
    },
    Lang {
        code: "fr",
        native: "Français",
        english: "French",
        plural_forms: "nplurals=2; plural=(n > 1);",
        complete: true,
    },
    Lang {
        code: "it",
        native: "Italiano",
        english: "Italian",
        plural_forms: TWO,
        complete: true,
    },
    Lang {
        code: "pl",
        native: "Polski",
        english: "Polish",
        plural_forms:
            "nplurals=3; plural=(n==1 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2);",
        complete: true,
    },
    Lang {
        code: "ru",
        native: "Русский",
        english: "Russian",
        plural_forms: SLAVIC,
        complete: true,
    },
    Lang {
        code: "uk",
        native: "Українська",
        english: "Ukrainian",
        plural_forms: SLAVIC,
        complete: true,
    },
    Lang {
        code: "zh",
        native: "中文（简体）",
        english: "Chinese (Simplified)",
        plural_forms: "nplurals=1; plural=0;",
        complete: false,
    },
    Lang {
        code: "ja",
        native: "日本語",
        english: "Japanese",
        plural_forms: "nplurals=1; plural=0;",
        complete: false,
    },
    // L5 layout test: generated from the English source (`pseudo`), never
    // offered, only via EXPLORER2_LANG=qps or `language = qps`
    Lang {
        code: pseudo::CODE,
        native: "Pseudo",
        english: "Pseudo-locale (layout test)",
        plural_forms: TWO,
        complete: false,
    },
];

/// The source language (texts in the code).
pub const SOURCE: &str = "en";

pub fn lang(code: &str) -> Option<&'static Lang> {
    LANGS.iter().find(|l| l.code == code)
}

/// The values the language setting offers (L4): `""` (the Windows language)
/// first, then every complete language by its own name; a `current` setting
/// that names a known but incomplete language stays selectable.
pub fn choices(current: &str) -> Vec<&'static str> {
    let mut langs: Vec<&'static Lang> = LANGS.iter().filter(|l| l.complete).collect();
    if let Some(l) = normalize(current).and_then(lang) {
        if !l.complete {
            langs.push(l);
        }
    }
    langs.sort_by_key(|l| l.native);
    std::iter::once("")
        .chain(langs.into_iter().map(|l| l.code))
        .collect()
}

/// Index into [`LANGS`]; starts with the source language.
static CURRENT: AtomicUsize = AtomicUsize::new(0);

/// Sets the language for [`tr!`]; unknown codes are refused (`false`).
pub fn set_language(code: &str) -> bool {
    match LANGS.iter().position(|l| l.code == code) {
        Some(i) => {
            CURRENT.store(i, Ordering::Relaxed);
            true
        }
        None => false,
    }
}

/// Code of the current language.
pub fn language() -> &'static str {
    LANGS[CURRENT.load(Ordering::Relaxed)].code
}

/// Plural form index of `n` for a language (gettext rules, see `plural_forms`).
pub fn plural_index(code: &str, n: i64) -> usize {
    let n = n.unsigned_abs();
    let few = (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100));
    match code {
        "fr" => (n > 1) as usize,
        "ru" | "uk" => {
            if n % 10 == 1 && n % 100 != 11 {
                0
            } else if few {
                1
            } else {
                2
            }
        }
        "pl" => {
            if n == 1 {
                0
            } else if few {
                1
            } else {
                2
            }
        }
        "zh" | "ja" => 0,
        _ => (n != 1) as usize,
    }
}

/// Number of plural forms of a language.
pub fn plural_count(code: &str) -> usize {
    match code {
        "ru" | "uk" | "pl" => 3,
        "zh" | "ja" => 1,
        _ => 2,
    }
}

mod catalog {
    // `SINGULAR`: language → sorted (source, translation)
    // `PLURAL`: language → sorted (singular, plural, forms)
    #![allow(clippy::type_complexity)]
    include!(concat!(env!("OUT_DIR"), "/catalog.rs"));
}

/// Languages that have a compiled catalog (for tests).
pub fn compiled_languages() -> Vec<&'static str> {
    catalog::SINGULAR.iter().map(|(c, _)| *c).collect()
}

/// Translation of a source text in the current language (or the text itself).
pub fn translate(src: &'static str) -> &'static str {
    translate_in(language(), src)
}

/// Translation of a source text in a given language (or the text itself).
pub fn translate_in(code: &str, src: &'static str) -> &'static str {
    if code == SOURCE {
        return src;
    }
    let Some((_, table)) = catalog::SINGULAR.iter().find(|(c, _)| *c == code) else {
        return src;
    };
    match table.binary_search_by(|(k, _)| (*k).cmp(src)) {
        Ok(i) => table[i].1,
        Err(_) => src,
    }
}

/// The plural form for `n` of a source text pair in the current language.
pub fn translate_plural(n: i64, singular: &'static str, plural: &'static str) -> &'static str {
    let code = language();
    let fallback = if plural_index(SOURCE, n) == 0 {
        singular
    } else {
        plural
    };
    if code == SOURCE {
        return fallback;
    }
    let Some((_, table)) = catalog::PLURAL.iter().find(|(c, _)| *c == code) else {
        return fallback;
    };
    match table.binary_search_by(|(s, p, _)| (*s, *p).cmp(&(singular, plural))) {
        Ok(i) => table[i]
            .2
            .get(plural_index(code, n))
            .copied()
            .unwrap_or(fallback),
        Err(_) => fallback,
    }
}

/// Fills placeholders: `{}` next argument, `{0}`… by position, `{n}` the
/// count, `{{`/`}}` literal braces; anything else stays as written.
pub fn format_n(template: &str, n: Option<i64>, args: &[&dyn Display]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut next = 0;
    let mut rest = template;
    while let Some(i) = rest.find(['{', '}']) {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        if tail.starts_with("{{") || tail.starts_with("}}") {
            out.push_str(&tail[..1]);
            rest = &tail[2..];
            continue;
        }
        if let Some(after) = tail.strip_prefix('}') {
            out.push('}');
            rest = after;
            continue;
        }
        let Some(end) = tail.find('}') else {
            out.push_str(tail);
            return out;
        };
        let key = &tail[1..end];
        let done = if key.is_empty() {
            let r = args.get(next).map(|a| write!(out, "{a}"));
            next += 1;
            r.is_some()
        } else if key == "n" {
            n.map(|n| write!(out, "{n}")).is_some()
        } else if let Ok(i) = key.parse::<usize>() {
            args.get(i).map(|a| write!(out, "{a}")).is_some()
        } else {
            false
        };
        if !done {
            out.push_str(&tail[..=end]);
        }
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    out
}

/// [`format_n`] without a count.
pub fn format(template: &str, args: &[&dyn Display]) -> String {
    format_n(template, None, args)
}

/// Translates a source text: `tr!("Copy")` → `&'static str`,
/// `tr!("{} selected", n)` → `String`.
#[macro_export]
macro_rules! tr {
    ($s:literal) => {
        $crate::translate($s)
    };
    ($s:literal, $($a:expr),+ $(,)?) => {
        $crate::format($crate::translate($s), &[$(&$a as &dyn ::std::fmt::Display),+])
    };
}

/// Digit-group and decimal separator of a language (like Windows' defaults:
/// `1,234.5` English, `1.234,5` German/Spanish/Italian, `1 234,5` with a
/// no-break space for French/Polish/Russian/Ukrainian).
pub fn separators(code: &str) -> (char, char) {
    match code {
        "de" | "es" | "it" => ('.', ','),
        "fr" | "pl" | "ru" | "uk" => ('\u{a0}', ','),
        _ => (',', '.'),
    }
}

/// `1234567` → `1,234,567` (current language).
pub fn format_integer(n: u64) -> String {
    let (group, _) = separators(language());
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 * 2);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(group);
        }
        out.push(c);
    }
    out
}

/// `v` with `places` decimals and the language's decimal separator (no grouping).
pub fn format_decimal(v: f64, places: usize) -> String {
    let (_, dec) = separators(language());
    let s = format!("{v:.places$}");
    if dec == '.' {
        s
    } else {
        s.replace('.', &dec.to_string())
    }
}

/// Marks a text for translation without translating it (gettext `N_`):
/// for `const` tables (command names, shortcuts). The literal stays the
/// English source; translate where it is shown: `explorer_i18n::translate(x)`.
#[macro_export]
macro_rules! tr_noop {
    ($s:literal) => {
        $s
    };
}

/// Plural: `tr_n!(count, "{n} item", "{n} items")` → `String`; further
/// arguments fill `{}`/`{0}`.
#[macro_export]
macro_rules! tr_n {
    ($n:expr, $s:literal, $p:literal $(, $a:expr)* $(,)?) => {{
        let n__: i64 = ($n) as i64;
        $crate::format_n(
            $crate::translate_plural(n__, $s, $p),
            Some(n__),
            &[$(&$a as &dyn ::std::fmt::Display),*],
        )
    }};
}

/// Maps a language tag (`de-DE`, `de_DE.UTF-8`, `zh-Hans-CN`, `pt-BR` …) to a
/// supported language code. Traditional Chinese is not mapped to `zh`
/// (Simplified) – those users get the next preferred language.
pub fn normalize(tag: &str) -> Option<&'static str> {
    let t = tag.trim().to_ascii_lowercase().replace('_', "-");
    let t = t.split(['.', '@']).next().unwrap_or("");
    let mut parts = t.split('-');
    let primary = parts.next()?;
    if primary == "zh" {
        let rest: Vec<&str> = parts.collect();
        if rest
            .iter()
            .any(|p| matches!(*p, "hant" | "tw" | "hk" | "mo"))
        {
            return None;
        }
    }
    LANGS.iter().find(|l| l.code == primary).map(|l| l.code)
}

/// Picks the language: override (`EXPLORER2_LANG`) > setting > first
/// **complete** system language > English. Explicit choices may name an
/// incomplete language (tests, translators); missing texts show in English.
pub fn resolve(setting: &str, env_override: Option<&str>, system: &[String]) -> &'static str {
    if let Some(c) = env_override.and_then(normalize) {
        return c;
    }
    if let Some(c) = normalize(setting) {
        return c;
    }
    system
        .iter()
        .filter_map(|t| normalize(t))
        .find(|c| lang(c).is_some_and(|l| l.complete))
        .unwrap_or(SOURCE)
}

/// Preferred languages from POSIX variables (gettext order): `LANGUAGE`
/// (colon list), then the first of `LC_ALL`, `LC_MESSAGES`, `LANG`.
pub fn posix_languages(get: impl Fn(&str) -> Option<String>) -> Vec<String> {
    let mut out: Vec<String> = get("LANGUAGE")
        .unwrap_or_default()
        .split(':')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    if let Some(l) = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|k| get(k).filter(|v| !v.is_empty()))
    {
        out.push(l);
    }
    out
}

#[cfg(test)]
mod tests;
