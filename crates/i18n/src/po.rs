//! Minimal gettext `.po` reader/writer (no gettext runtime needed).
//!
//! Shared with `build.rs` via `#[path]`, so it must not use anything from the
//! crate itself. Supports what our catalogs use: `msgctxt`, `msgid`,
//! `msgid_plural`, `msgstr`, `msgstr[N]`, multi-line strings, `#,` flags
//! (`fuzzy`), comments and obsolete entries (`#~`, ignored).

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Entry {
    pub ctx: Option<String>,
    pub id: String,
    pub id_plural: Option<String>,
    /// `msgstr` (one element) or `msgstr[0..]` for plurals.
    pub strs: Vec<String>,
    pub fuzzy: bool,
    /// `#.` / `#:` / `# ` lines, written back unchanged.
    pub comments: Vec<String>,
}

impl Entry {
    /// Translated and usable (not fuzzy, every form filled).
    pub fn is_translated(&self) -> bool {
        !self.fuzzy && !self.strs.is_empty() && self.strs.iter().all(|s| !s.is_empty())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Catalog {
    /// `msgstr` of the empty `msgid` (header fields, `Key: value\n`).
    pub header: String,
    pub entries: Vec<Entry>,
}

impl Catalog {
    pub fn header_field(&self, name: &str) -> Option<&str> {
        self.header.lines().find_map(|l| {
            let (k, v) = l.split_once(':')?;
            (k.trim().eq_ignore_ascii_case(name)).then(|| v.trim())
        })
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Field {
    None,
    Ctx,
    Id,
    IdPlural,
    Str(usize),
}

/// Parses a `.po` file. Errors name the line (1-based).
pub fn parse(text: &str) -> Result<Catalog, String> {
    let mut cat = Catalog::default();
    let mut cur = Entry::default();
    let mut field = Field::None;
    let mut started = false;
    let mut header_done = false;

    let mut finish = |cur: &mut Entry, started: &mut bool, cat: &mut Catalog| {
        if *started {
            let e = std::mem::take(cur);
            if e.id.is_empty() && e.ctx.is_none() && !header_done {
                cat.header = e.strs.first().cloned().unwrap_or_default();
                header_done = true;
            } else {
                cat.entries.push(e);
            }
        }
        *started = false;
    };

    for (no, raw) in text.lines().enumerate() {
        let line = raw.trim();
        let err = |m: &str| format!("line {}: {m}", no + 1);
        if line.is_empty() {
            continue;
        }
        if line.starts_with("#~") {
            continue; // obsolete entry
        }
        if let Some(flags) = line.strip_prefix("#,") {
            if field != Field::None && field != Field::Ctx {
                finish(&mut cur, &mut started, &mut cat);
                field = Field::None;
            }
            if flags.split(',').any(|f| f.trim() == "fuzzy") {
                cur.fuzzy = true;
            }
            continue;
        }
        if line.starts_with('#') {
            if field != Field::None {
                finish(&mut cur, &mut started, &mut cat);
                field = Field::None;
            }
            cur.comments.push(line.to_string());
            continue;
        }
        if line.starts_with('"') {
            let s = unquote(line).map_err(|e| err(&e))?;
            match field {
                Field::Ctx => cur.ctx.get_or_insert_with(String::new).push_str(&s),
                Field::Id => cur.id.push_str(&s),
                Field::IdPlural => cur.id_plural.get_or_insert_with(String::new).push_str(&s),
                Field::Str(i) => cur.strs[i].push_str(&s),
                Field::None => return Err(err("string without a keyword")),
            }
            continue;
        }
        let (kw, rest) = line
            .split_once(char::is_whitespace)
            .ok_or_else(|| err("keyword without a string"))?;
        let s = unquote(rest.trim()).map_err(|e| err(&e))?;
        match kw {
            "msgctxt" => {
                finish(&mut cur, &mut started, &mut cat);
                started = true;
                cur.ctx = Some(s);
                field = Field::Ctx;
            }
            "msgid" => {
                if field != Field::Ctx {
                    finish(&mut cur, &mut started, &mut cat);
                }
                started = true;
                cur.id = s;
                field = Field::Id;
            }
            "msgid_plural" => {
                cur.id_plural = Some(s);
                field = Field::IdPlural;
            }
            "msgstr" => {
                cur.strs = vec![s];
                field = Field::Str(0);
            }
            k if k.starts_with("msgstr[") && k.ends_with(']') => {
                let i: usize = k[7..k.len() - 1]
                    .parse()
                    .map_err(|_| err("bad plural index"))?;
                if i != cur.strs.len() {
                    return Err(err("plural forms out of order"));
                }
                cur.strs.push(s);
                field = Field::Str(i);
            }
            _ => return Err(err(&format!("unknown keyword {kw}"))),
        }
    }
    finish(&mut cur, &mut started, &mut cat);
    Ok(cat)
}

fn unquote(s: &str) -> Result<String, String> {
    let inner = s
        .strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .ok_or("string not in quotes")?;
    let mut out = String::with_capacity(inner.len());
    let mut it = inner.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            if c == '"' {
                return Err("unescaped quote".into());
            }
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            other => return Err(format!("unknown escape \\{}", other.unwrap_or(' '))),
        }
    }
    Ok(out)
}

/// Escapes a string for a `.po` file (without the surrounding quotes).
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

/// Writes `keyword "value"`, splitting at `\n` like gettext does.
pub fn write_field(out: &mut String, keyword: &str, value: &str) {
    // several lines (the header) → one quoted string per line
    if value.trim_end_matches('\n').contains('\n') {
        out.push_str(keyword);
        out.push_str(" \"\"\n");
        for part in value.split_inclusive('\n') {
            out.push('"');
            out.push_str(&escape(part));
            out.push_str("\"\n");
        }
    } else {
        out.push_str(keyword);
        out.push_str(" \"");
        out.push_str(&escape(value));
        out.push_str("\"\n");
    }
}

/// Serialises a catalog (header first, entries in the given order).
pub fn write(cat: &Catalog) -> String {
    let mut out = String::new();
    write_field(&mut out, "msgid", "");
    write_field(&mut out, "msgstr", &cat.header);
    for e in &cat.entries {
        out.push('\n');
        for c in &e.comments {
            out.push_str(c);
            out.push('\n');
        }
        if e.fuzzy {
            out.push_str("#, fuzzy\n");
        }
        if let Some(c) = &e.ctx {
            write_field(&mut out, "msgctxt", c);
        }
        write_field(&mut out, "msgid", &e.id);
        match &e.id_plural {
            Some(p) => {
                write_field(&mut out, "msgid_plural", p);
                for (i, s) in e.strs.iter().enumerate() {
                    write_field(&mut out, &format!("msgstr[{i}]"), s);
                }
            }
            None => write_field(&mut out, "msgstr", e.strs.first().map_or("", |s| s)),
        }
    }
    out
}
