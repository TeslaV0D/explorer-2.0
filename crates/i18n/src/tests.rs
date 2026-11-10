use super::*;
use crate::extract::{merge, placeholders, scan_rust, scan_slint, Found, Msg};

#[test]
fn plural_rules_match_gettext() {
    // English/German: 1 vs other
    assert_eq!([0, 1, 2, 21].map(|n| plural_index("de", n)), [1, 0, 1, 1]);
    // French: 0 and 1 are singular
    assert_eq!([0, 1, 2].map(|n| plural_index("fr", n)), [0, 0, 1]);
    // Russian/Ukrainian: 1, 21 / 2-4, 22 / 5-20, 11-14, 0
    let ru = [1, 21, 101, 2, 3, 22, 104, 0, 5, 11, 12, 14, 111, 25];
    assert_eq!(
        ru.map(|n| plural_index("ru", n)),
        [0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2]
    );
    assert_eq!(
        ru.map(|n| plural_index("uk", n)),
        ru.map(|n| plural_index("ru", n))
    );
    // Polish: only exactly 1 is singular, 21 is "many"
    let pl = [1, 21, 2, 4, 22, 12, 5, 0];
    assert_eq!(pl.map(|n| plural_index("pl", n)), [0, 2, 1, 1, 1, 2, 2, 2]);
    assert_eq!([0, 1, 7].map(|n| plural_index("ja", n)), [0, 0, 0]);
    // every rule stays inside its number of forms
    for l in LANGS {
        for n in 0..300 {
            assert!(
                plural_index(l.code, n) < plural_count(l.code),
                "{} {n}",
                l.code
            );
        }
        let header_count: usize = l
            .plural_forms
            .split(';')
            .find_map(|p| p.trim().strip_prefix("nplurals="))
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(header_count, plural_count(l.code), "{}", l.code);
    }
}

#[test]
fn placeholders_are_filled() {
    let a: &dyn Display = &"x";
    let b: &dyn Display = &7;
    assert_eq!(format("{} of {}", &[a, b]), "x of 7");
    assert_eq!(format("{1} before {0}", &[a, b]), "7 before x");
    assert_eq!(format_n("{n} files in {}", Some(3), &[a]), "3 files in x");
    assert_eq!(format("{{literal}} {}", &[a]), "{literal} x");
    // missing arguments and unknown keys stay visible instead of panicking
    assert_eq!(format("{} {} {name} {", &[a]), "x {} {name} {");
    assert_eq!(format("a } b", &[]), "a } b");
}

#[test]
fn source_language_and_macros() {
    set_language("en");
    assert_eq!(tr!("Copy"), "Copy");
    assert_eq!(tr!("{} selected", 3), "3 selected");
    assert_eq!(tr_n!(1, "{n} item", "{n} items"), "1 item");
    assert_eq!(tr_n!(2usize, "{n} item", "{n} items"), "2 items");
    assert_eq!(
        tr_n!(5, "{n} of {} items", "{n} of {} items", "C:"),
        "5 of C: items"
    );
    assert!(!set_language("xx"));
    assert_eq!(language(), "en");
}

#[test]
fn language_tags_are_normalised() {
    assert_eq!(normalize("de-DE"), Some("de"));
    assert_eq!(normalize("de_AT.UTF-8"), Some("de"));
    assert_eq!(normalize("uk_UA@euro"), Some("uk"));
    assert_eq!(normalize("EN-us"), Some("en"));
    assert_eq!(normalize("zh-CN"), Some("zh"));
    assert_eq!(normalize("zh-Hans-SG"), Some("zh"));
    assert_eq!(normalize("zh-TW"), None);
    assert_eq!(normalize("zh-Hant-HK"), None);
    assert_eq!(normalize("pt-BR"), None);
    assert_eq!(normalize("C"), None);
    assert_eq!(normalize(""), None);
}

#[test]
fn resolution_order() {
    let sys = vec![
        "zh-TW".to_string(),
        "fr-FR".to_string(),
        "de-DE".to_string(),
    ];
    assert_eq!(resolve("", Some("ru"), &sys), "ru");
    assert_eq!(resolve("pl", None, &sys), "pl");
    assert_eq!(resolve("pl", Some("bogus"), &sys), "pl");
    // automatic: the first *complete* system language, else English
    let auto = resolve("", None, &sys);
    let expect = ["fr", "de"]
        .into_iter()
        .find(|c| lang(c).unwrap().complete)
        .unwrap_or("en");
    assert_eq!(auto, expect);
    assert_eq!(resolve("", None, &[]), "en");
}

#[test]
fn posix_variables_in_gettext_order() {
    let env = |vars: &[(&str, &str)]| {
        let v: Vec<(String, String)> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        posix_languages(move |k| v.iter().find(|(n, _)| n == k).map(|(_, x)| x.clone()))
    };
    assert_eq!(env(&[("LANG", "de_DE.UTF-8")]), ["de_DE.UTF-8"]);
    assert_eq!(
        env(&[("LANGUAGE", "uk:ru"), ("LANG", "en_US")]),
        ["uk", "ru", "en_US"]
    );
    assert_eq!(env(&[("LC_ALL", "fr_FR"), ("LANG", "de_DE")]), ["fr_FR"]);
    assert_eq!(env(&[("LC_ALL", ""), ("LC_MESSAGES", "pl_PL")]), ["pl_PL"]);
    assert!(env(&[]).is_empty());
}

#[test]
fn po_round_trip() {
    let text = "msgid \"\"\nmsgstr \"\"\n\"Language: ru\\n\"\n\"Plural-Forms: nplurals=3; plural=0;\\n\"\n\n\
                #: crates/app/ui/main.slint\nmsgid \"Copy\"\nmsgstr \"Копировать\"\n\n\
                #, fuzzy\nmsgid \"Say \\\"hi\\\"\\n\"\nmsgstr \"\"\n\"Скажи \"\n\"\\\"привет\\\"\\n\"\n\n\
                msgid \"{n} file\"\nmsgid_plural \"{n} files\"\nmsgstr[0] \"{n} файл\"\nmsgstr[1] \"{n} файла\"\nmsgstr[2] \"{n} файлов\"\n\n\
                #~ msgid \"gone\"\n#~ msgstr \"weg\"\n";
    let cat = po::parse(text).unwrap();
    assert_eq!(cat.header_field("language"), Some("ru"));
    assert_eq!(cat.entries.len(), 3);
    assert_eq!(cat.entries[0].strs, ["Копировать"]);
    assert_eq!(cat.entries[0].comments, ["#: crates/app/ui/main.slint"]);
    assert!(cat.entries[1].fuzzy && !cat.entries[1].is_translated());
    assert_eq!(cat.entries[1].id, "Say \"hi\"\n");
    assert_eq!(cat.entries[1].strs, ["Скажи \"привет\"\n"]);
    assert_eq!(cat.entries[2].strs.len(), 3);
    // written and read again = the same
    assert_eq!(po::parse(&po::write(&cat)).unwrap(), cat);
    assert!(po::parse("msgid \"x\"\nmsgstr \"a\\q\"\n").is_err());
    assert!(po::parse("msgid \"x\"\nmsgstr[1] \"a\"\n").is_err());
}

#[test]
fn slint_texts_are_found() {
    let src = r#"
        // @tr("in a comment")
        Text { text: @tr("Settings"); }
        Text { text: @tr("Search in {}", root.name); }
        Text { text: @tr("{n} item" | "{n} items" % root.count); }
        Text { text: "not @tr(\"translated\")"; }
        Text { text: @tr("Line\nbreak \u{2192}"); }
    "#;
    let found = scan_slint(src).unwrap();
    let ids: Vec<_> = found
        .iter()
        .map(|(m, _)| (m.id.as_str(), m.plural.as_deref()))
        .collect();
    assert_eq!(
        ids,
        [
            ("Settings", None),
            ("Search in {}", None),
            ("{n} item", Some("{n} items")),
            ("Line\nbreak →", None)
        ]
    );
    assert_eq!(found[0].1, 3);
    assert!(scan_slint(r#"@tr("ctx" => "x")"#).is_err());
    assert!(scan_slint(r#"@tr("a \{x}")"#).is_err());
}

#[test]
fn rust_texts_are_found() {
    let src = r##"
        // tr!("comment")
        let a = tr!("Copy");
        let b = explorer_i18n::tr!("{} selected", n);
        let c = tr_n!(items.len(), "{n} item", "{n} items");
        let d = tr_n!(f(a, b), "one {0}", "many {0}", x);
        let e = "tr!(\"in a string\")";
        let f = r#"tr!("raw")"#;
        let g = 'x'; let h: &'static str = tr!("multi \
                 line");
        let i = attr!("not ours");
    "##;
    let found = scan_rust(src).unwrap();
    let ids: Vec<_> = found
        .iter()
        .map(|(m, _)| (m.id.as_str(), m.plural.as_deref()))
        .collect();
    assert_eq!(
        ids,
        [
            ("Copy", None),
            ("{} selected", None),
            ("{n} item", Some("{n} items")),
            ("one {0}", Some("many {0}")),
            ("multi line", None)
        ]
    );
    assert!(scan_rust("tr!(TEXT)").is_err());
}

#[test]
fn merge_keeps_translations_and_drops_gone_texts() {
    let mut found = Found::new();
    found.insert(
        Msg {
            id: "Copy".into(),
            plural: None,
        },
        vec!["a.rs".into()],
    );
    found.insert(
        Msg {
            id: "New".into(),
            plural: None,
        },
        vec!["b.slint".into()],
    );
    found.insert(
        Msg {
            id: "{n} file".into(),
            plural: Some("{n} files".into()),
        },
        vec!["a.rs".into()],
    );
    let old = po::parse(
        "msgid \"\"\nmsgstr \"\"\n\nmsgid \"Copy\"\nmsgstr \"Kopieren\"\n\nmsgid \"Gone\"\nmsgstr \"Weg\"\n",
    )
    .unwrap();
    let cat = merge(Some(&old), &found, "ru");
    let ids: Vec<_> = cat.entries.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["Copy", "New", "{n} file"]);
    assert_eq!(cat.entries[0].strs, ["Kopieren"]);
    assert_eq!(cat.entries[1].strs, [""]);
    assert_eq!(
        cat.entries[2].strs.len(),
        3,
        "Russian has three plural forms"
    );
    assert_eq!(cat.header_field("Language"), Some("ru"));
    assert_eq!(
        cat.header_field("Plural-Forms"),
        Some(lang("ru").unwrap().plural_forms)
    );
}

#[test]
fn placeholder_sets() {
    assert_eq!(placeholders("{} of {n} in {0} {{x}}"), ["{0}", "{n}", "{}"]);
    assert!(placeholders("none").is_empty());
}
