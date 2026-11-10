//! Search language (subset of Everything's syntax) and the parallel scan.
//!
//! ```text
//! foo bar          both (AND)                 foo|bar, foo | bar   either (OR binds tighter: a b|c = a AND (b OR c))
//! !foo             not                        "a b"                phrase, taken literally (no wildcards/modifiers)
//! *.jpg  re?ort    wildcards: whole name      foo                  anywhere in the name
//! ext:jpg;png      extension (files)          file: / folder:      only files / folders (file:foo = file AND foo)
//! size:>10mb  size:1kb..2mb  size:empty|tiny|small|medium|large|huge|gigantic   (files)
//! dm:today  dm:yesterday  dm:thisweek  dm:lastmonth  dm:2026  dm:2026-11  dm:>=2026-11-01  dm:2026-01..2026-03
//! path:foo         match the full path        case:Foo             case-sensitive
//! regex:^a.*\d$    regular expression (name; with path: the full path)
//! ```

use crate::{Index, EXT_NONE, EXT_OTHER, FLAG_DIR, FLAG_DIRTY, FLAG_GONE, NONE, SEP};
use memchr::memmem::Finder;

/// "Now" for relative dates (`dm:today`): Unix seconds + the local UTC offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Now {
    pub unix: i64,
    pub utc_offset: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone)]
enum Pat {
    /// One byte anywhere (memchr: no searcher set-up per short name).
    Byte(u8),
    /// Anywhere (SIMD substring search).
    Sub(Box<Finder<'static>>),
    /// Wildcards without `?` in the middle: fast shapes.
    Prefix(Vec<u8>),
    Suffix(Vec<u8>),
    /// General wildcards (`?` = one char).
    Glob {
        chars: Vec<char>,
        bytes: Vec<u8>,
        has_q: bool,
    },
}

impl Pat {
    fn new(text: &str, wild: bool) -> Pat {
        if !wild {
            if let [b] = text.as_bytes() {
                return Pat::Byte(*b);
            }
            return Pat::Sub(Box::new(Finder::new(text.as_bytes()).into_owned()));
        }
        let stars = text.matches('*').count();
        let lit = |s: &str| !s.contains(['*', '?']);
        if !text.contains('?') {
            if stars == 1 && text.starts_with('*') && lit(&text[1..]) {
                return Pat::Suffix(text.as_bytes()[1..].to_vec());
            }
            if stars == 1 && text.ends_with('*') && lit(&text[..text.len() - 1]) {
                return Pat::Prefix(text.as_bytes()[..text.len() - 1].to_vec());
            }
            if stars == 2
                && text.len() > 2
                && text.starts_with('*')
                && text.ends_with('*')
                && lit(&text[1..text.len() - 1])
            {
                return Pat::Sub(Box::new(
                    Finder::new(&text.as_bytes()[1..text.len() - 1]).into_owned(),
                ));
            }
        }
        Pat::Glob {
            chars: text.chars().collect(),
            bytes: text.as_bytes().to_vec(),
            has_q: text.contains('?'),
        }
    }

    /// What to look for in the `\n`-separated name buffer (None = no shape
    /// the buffer search can do).
    fn buffer_needle(&self) -> Option<Vec<u8>> {
        match self {
            Pat::Byte(b) => Some(vec![*b]),
            Pat::Sub(f) => Some(f.needle().to_vec()),
            Pat::Prefix(p) => Some([&[SEP][..], p].concat()),
            Pat::Suffix(s) => Some([&s[..], &[SEP][..]].concat()),
            // a fixed start (`img_1*.png`) or the longest fixed piece
            Pat::Glob { bytes, .. } => {
                let first = bytes
                    .iter()
                    .position(|&b| b == b'*' || b == b'?')
                    .unwrap_or(bytes.len());
                if first > 0 {
                    return Some([&[SEP][..], &bytes[..first]].concat());
                }
                bytes
                    .split(|&b| b == b'*' || b == b'?')
                    .max_by_key(|piece| piece.len())
                    .filter(|piece| !piece.is_empty())
                    .map(<[u8]>::to_vec)
            }
        }
    }

    fn is_match(&self, hay: &[u8]) -> bool {
        match self {
            Pat::Byte(b) => memchr::memchr(*b, hay).is_some(),
            Pat::Sub(f) => f.find(hay).is_some(),
            Pat::Prefix(p) => hay.starts_with(p),
            Pat::Suffix(p) => hay.ends_with(p),
            Pat::Glob {
                chars,
                bytes,
                has_q,
            } => {
                // bytes are exact unless `?` meets a multi-byte character
                if !*has_q || hay.is_ascii() {
                    glob_bytes(bytes, hay)
                } else {
                    glob(chars, std::str::from_utf8(hay).unwrap_or(""))
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
enum Term {
    Any,
    Not(Box<Term>),
    All(Vec<Term>),
    Text {
        pat: Pat,
        path: bool,
        case: bool,
        /// `path:word` without separators/wildcards: precomputed per query
        /// (own name or an ancestor's contains it), see [`path_marks`].
        slot: Option<usize>,
    },
    /// `slot`: its extension ids are resolved per query, see [`Pre`].
    Ext {
        exts: Vec<String>,
        slot: usize,
    },
    /// `lo..hi` (hi exclusive), files only.
    Size(u64, u64),
    /// `lo..hi` Unix seconds (hi exclusive).
    Date(i64, i64),
    Kind {
        dir: bool,
    },
    Regex {
        re: regex::Regex,
        path: bool,
        /// Literals every match must contain (lower-case ASCII): their buffer
        /// hits are the only candidates (prefilter, like regex engines do).
        needles: Option<Vec<Vec<u8>>>,
    },
}

/// A parsed query: AND over groups, OR inside a group.
#[derive(Debug, Clone)]
pub struct Query {
    groups: Vec<Vec<Term>>,
    /// Number of precomputed `path:` terms.
    slots: usize,
    /// Number of `ext:` terms.
    ext_slots: usize,
}

impl Query {
    pub fn parse(text: &str, now: Now) -> Result<Query, ParseError> {
        let mut groups: Vec<Vec<Term>> = Vec::new();
        let mut pending_or = false;
        let mut slots = 0usize;
        let mut ext_slots = 0usize;
        for tok in tokenize(text) {
            if tok == "|" {
                pending_or = !groups.is_empty();
                continue;
            }
            let starts_or = tok.starts_with('|');
            let ends_or = tok.ends_with('|')
                && !tok.ends_with("\"|\"")
                && !tok.to_ascii_lowercase().contains("regex:");
            // a regular expression keeps its own `|` (`regex:a|b`)
            let is_regex = tok
                .to_ascii_lowercase()
                .find("regex:")
                .is_some_and(|at| !tok[..at].contains('|'));
            let parts: Vec<String> = if is_regex {
                vec![tok.trim_start_matches('|').to_string()]
            } else {
                split_top(&tok, '|')
                    .into_iter()
                    .filter(|p| !p.is_empty())
                    .collect()
            };
            if parts.is_empty() {
                pending_or = !groups.is_empty();
                continue;
            }
            let terms = parts
                .iter()
                .map(|p| parse_term(p, now, &mut slots, &mut ext_slots))
                .collect::<Result<Vec<_>, _>>()?;
            match groups.last_mut() {
                Some(g) if pending_or || starts_or => g.extend(terms),
                _ => groups.push(terms),
            }
            pending_or = ends_or;
        }
        Ok(Query {
            groups,
            slots,
            ext_slots,
        })
    }

    /// No condition at all (matches everything).
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// Only size/date/kind conditions: evaluated straight on the columns.
    fn columns_only(&self) -> bool {
        fn col(t: &Term) -> bool {
            match t {
                Term::Size(..) | Term::Date(..) | Term::Kind { .. } | Term::Any => true,
                Term::Not(t) => col(t),
                Term::All(ts) => ts.iter().all(col),
                _ => false,
            }
        }
        !self.groups.is_empty() && self.groups.iter().flatten().all(col)
    }

    /// The whole query is one plain word/prefix/suffix: its buffer hits are
    /// the result (no per-candidate check beyond removed/scope).
    fn exact_needle(&self) -> bool {
        match self.groups.as_slice() {
            [g] => match g.as_slice() {
                [Term::Text {
                    pat,
                    path: false,
                    case: false,
                    ..
                }] => match pat {
                    Pat::Byte(b) => *b != SEP,
                    Pat::Sub(f) => !f.needle().contains(&SEP),
                    Pat::Prefix(p) | Pat::Suffix(p) => !p.contains(&SEP),
                    Pat::Glob { .. } => false,
                },
                _ => false,
            },
            _ => false,
        }
    }

    /// How to find the candidates without looking at every entry: the most
    /// selective group whose every alternative a fast path can do.
    fn driver(&self) -> Option<Driver> {
        fn needles(t: &Term) -> Option<Vec<u8>> {
            match t {
                Term::Text {
                    pat,
                    path: false,
                    case: false,
                    ..
                } => pat.buffer_needle(),
                // every part must match: any part with a needle can drive
                // (`folder:projekt`, `file:foo`)
                Term::All(ts) => ts.iter().filter_map(needles).max_by_key(Vec::len),
                _ => None,
            }
        }
        // 1. words/prefixes/suffixes: one pass over the names per needle
        //    (past two needles, scanning the entries wins)
        let text = self
            .groups
            .iter()
            .filter_map(|g| g.iter().map(needles).collect::<Option<Vec<_>>>())
            .filter(|ns| ns.len() <= 2 && ns.iter().all(|n| n.iter().any(|&b| b != SEP)))
            .max_by_key(|ns| ns.iter().map(Vec::len).min().unwrap_or(0));
        if let Some(ns) = text {
            return Some(Driver::Needles(ns));
        }
        // 2. extensions: compare ids in one sequential array
        if let Some(g) = self
            .groups
            .iter()
            .find(|g| g.iter().all(|t| matches!(t, Term::Ext { .. })))
        {
            return Some(Driver::Exts(
                g.iter()
                    .filter_map(|t| match t {
                        Term::Ext { slot, .. } => Some(*slot),
                        _ => None,
                    })
                    .collect(),
            ));
        }
        // 3. a regex alone in its group: its required literals
        self.groups.iter().find_map(|g| match g.as_slice() {
            [Term::Regex {
                needles: Some(ns), ..
            }] => Some(Driver::Needles(ns.clone())),
            _ => None,
        })
    }
}

enum Driver {
    Needles(Vec<Vec<u8>>),
    Exts(Vec<usize>),
}

/// Ids whose (stale or current) lower-case name in the ordered buffer
/// contains `needle`, ascending; one hit per entry. One streaming SIMD pass
/// (`memchr`/`memmem` iterators): hits inside the name already taken cost a
/// comparison, the hit → entry step gallops forward from the last entry.
fn buffer_hits(
    buf: &[u8],
    starts: &[u32],
    needle: &[u8],
    range: std::ops::Range<usize>,
) -> Vec<u32> {
    if starts.is_empty() || needle.is_empty() || range.is_empty() {
        return Vec::new();
    }
    let lead = usize::from(needle.first() == Some(&SEP));
    let window = &buf[range.start..(range.end + needle.len()).min(buf.len())];
    let mut out = Vec::new();
    let mut cur = 0usize; // last entry known to start at or before the hit
    let mut covered = 0usize; // hits before this belong to the last pushed entry
    let mut take = |at: usize| -> bool {
        if at >= range.end {
            return false;
        }
        let q = at + lead;
        if q < covered || (q as u64) < u64::from(starts[0]) {
            return true;
        }
        let q = q as u32;
        let (mut lo, mut step) = (cur, 1usize);
        while lo + step < starts.len() && starts[lo + step] <= q {
            lo += step;
            step *= 2;
        }
        let hi = (lo + step + 1).min(starts.len());
        cur = lo + starts[lo..hi].partition_point(|&o| o <= q) - 1;
        out.push(cur as u32);
        covered = starts.get(cur + 1).map_or(usize::MAX, |&o| o as usize);
        true
    };
    if let [b] = needle {
        for rel in memchr::memchr_iter(*b, window) {
            if !take(range.start + rel) {
                break;
            }
        }
    } else {
        for rel in memchr::memmem::find_iter(window, needle) {
            if !take(range.start + rel) {
                break;
            }
        }
    }
    out
}

/// Splits at spaces outside quotes; quotes stay in the token.
fn tokenize(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in s.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                cur.push(c);
            }
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn split_top(s: &str, sep: char) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut quoted = false;
    for c in s.chars() {
        if c == '"' {
            quoted = !quoted;
        }
        if c == sep && !quoted {
            out.push(String::new());
        } else if let Some(last) = out.last_mut() {
            last.push(c);
        }
    }
    out
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    (s.len() >= prefix.len()
        && s.is_char_boundary(prefix.len())
        && s[..prefix.len()].eq_ignore_ascii_case(prefix))
    .then(|| &s[prefix.len()..])
}

fn parse_term(
    s: &str,
    now: Now,
    slots: &mut usize,
    ext_slots: &mut usize,
) -> Result<Term, ParseError> {
    if let Some(rest) = s.strip_prefix('!') {
        if !rest.is_empty() {
            return Ok(Term::Not(Box::new(parse_term(
                rest, now, slots, ext_slots,
            )?)));
        }
    }
    let (mut path, mut case) = (false, false);
    let mut terms = Vec::new();
    let mut rest = s;
    loop {
        if rest.starts_with('"') {
            break;
        }
        if let Some(r) = strip_prefix_ci(rest, "path:") {
            path = true;
            rest = r;
        } else if let Some(r) = strip_prefix_ci(rest, "case:") {
            case = true;
            rest = r;
        } else if let Some(r) = strip_prefix_ci(rest, "file:") {
            terms.push(Term::Kind { dir: false });
            rest = r;
        } else if let Some(r) = strip_prefix_ci(rest, "folder:") {
            terms.push(Term::Kind { dir: true });
            rest = r;
        } else if let Some(r) = strip_prefix_ci(rest, "ext:") {
            let exts: Vec<String> = unquote(r)
                .split(';')
                .map(|e| e.trim().trim_start_matches('.').to_lowercase())
                .filter(|e| !e.is_empty())
                .collect();
            *ext_slots += 1;
            terms.push(Term::Ext {
                exts,
                slot: *ext_slots - 1,
            });
            rest = "";
        } else if let Some(r) = strip_prefix_ci(rest, "size:") {
            let (lo, hi) = parse_size(&unquote(r))
                .ok_or_else(|| ParseError(explorer_i18n::tr!("size not understood: {0}", r)))?;
            terms.push(Term::Size(lo, hi));
            rest = "";
        } else if let Some(r) =
            strip_prefix_ci(rest, "dm:").or_else(|| strip_prefix_ci(rest, "datemodified:"))
        {
            let (lo, hi) = parse_date(&unquote(r), now)
                .ok_or_else(|| ParseError(explorer_i18n::tr!("date not understood: {0}", r)))?;
            terms.push(Term::Date(lo, hi));
            rest = "";
        } else if let Some(r) = strip_prefix_ci(rest, "regex:") {
            let src = unquote(r);
            let re = regex::RegexBuilder::new(&src)
                .case_insensitive(!case)
                .size_limit(1 << 20)
                .build()
                .map_err(|e| {
                    ParseError(explorer_i18n::tr!("invalid regular expression: {0}", e))
                })?;
            let needles = if path || case {
                None
            } else {
                regex_literals(&src)
            };
            terms.push(Term::Regex { re, path, needles });
            rest = "";
        } else {
            break;
        }
    }
    if !rest.is_empty() {
        let literal = rest.contains('"');
        let text = unquote(rest);
        let text = if case { text } else { text.to_lowercase() };
        if !text.is_empty() {
            let pat = Pat::new(&text, !literal && text.contains(['*', '?']));
            let slot = (path && !case && matches!(pat, Pat::Sub(_)) && !text.contains(['/', '\\']))
                .then(|| {
                    *slots += 1;
                    *slots - 1
                });
            terms.push(Term::Text {
                pat,
                path,
                case,
                slot,
            });
        }
    }
    Ok(match terms.len() {
        0 => Term::Any,
        1 => terms.pop().unwrap_or(Term::Any),
        _ => Term::All(terms),
    })
}

/// Prefix or suffix literals a regex match must contain, lower-cased, if
/// they are few, ASCII and long enough to be selective.
fn regex_literals(src: &str) -> Option<Vec<Vec<u8>>> {
    use regex_syntax::hir::literal::{ExtractKind, Extractor};
    let hir = regex_syntax::ParserBuilder::new().build().parse(src).ok()?;
    let pick = |kind: ExtractKind| -> Option<Vec<Vec<u8>>> {
        let mut ex = Extractor::new();
        ex.kind(kind);
        let seq = ex.extract(&hir);
        let mut lits: Vec<Vec<u8>> = seq
            .literals()?
            .iter()
            .map(|l| l.as_bytes().to_ascii_lowercase())
            .collect();
        lits.sort();
        lits.dedup();
        // ASCII only: non-ASCII letters may case-fold to forms that
        // lower-casing does not produce
        (!lits.is_empty() && lits.len() <= 2 && lits.iter().all(|l| l.len() >= 3 && l.is_ascii()))
            .then_some(lits)
    };
    let pre = pick(ExtractKind::Prefix);
    let suf = pick(ExtractKind::Suffix);
    let min = |v: &Option<Vec<Vec<u8>>>| {
        v.as_ref()
            .map_or(0, |l| l.iter().map(Vec::len).min().unwrap_or(0))
    };
    if min(&suf) > min(&pre) {
        suf
    } else {
        pre
    }
}

fn unquote(s: &str) -> String {
    s.chars().filter(|&c| c != '"').collect()
}

// ------------------------------------------------------------------ size

fn parse_bytes(s: &str) -> Option<u64> {
    let s = s.trim().to_ascii_lowercase();
    let split = s
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(s.len());
    let (num, unit) = s.split_at(split);
    let n: f64 = num.parse().ok()?;
    let mul: f64 = match unit.trim() {
        "" | "b" => 1.0,
        "k" | "kb" => 1024.0,
        "m" | "mb" => 1024.0 * 1024.0,
        "g" | "gb" => 1024.0 * 1024.0 * 1024.0,
        "t" | "tb" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    let v = n * mul;
    (v.is_finite() && (0.0..1.8e19).contains(&v)).then_some(v as u64)
}

/// `(lo, hi)` with hi exclusive.
fn parse_size(s: &str) -> Option<(u64, u64)> {
    const K: u64 = 1024;
    let named = match s.trim().to_ascii_lowercase().as_str() {
        "empty" => Some((0, 1)),
        "tiny" => Some((0, 10 * K + 1)),
        "small" => Some((10 * K + 1, 100 * K + 1)),
        "medium" => Some((100 * K + 1, K * K + 1)),
        "large" => Some((K * K + 1, 16 * K * K + 1)),
        "huge" => Some((16 * K * K + 1, 128 * K * K + 1)),
        "gigantic" => Some((128 * K * K + 1, u64::MAX)),
        _ => None,
    };
    if named.is_some() {
        return named;
    }
    ranged(s, parse_bytes, |v| (v, v.saturating_add(1)), 0, u64::MAX)
}

/// Shared `>x`, `>=x`, `<x`, `<=x`, `=x`, `a..b`, `x` handling.
fn ranged<V: Copy, T: Copy + Ord>(
    s: &str,
    parse: impl Fn(&str) -> Option<V>,
    span: impl Fn(V) -> (T, T),
    min: T,
    max: T,
) -> Option<(T, T)> {
    let s = s.trim();
    if let Some((a, b)) = s.split_once("..") {
        let (a, b) = (span(parse(a)?), span(parse(b)?));
        return (a.0 <= b.1).then_some((a.0, b.1));
    }
    let (op, v) = [">=", "<=", ">", "<", "="]
        .iter()
        .find_map(|op| s.strip_prefix(op).map(|r| (*op, r)))
        .unwrap_or(("", s));
    let (lo, hi) = span(parse(v)?);
    Some(match op {
        ">" => (hi, max),
        ">=" => (lo, max),
        "<" => (min, lo),
        "<=" => (min, hi),
        _ => (lo, hi),
    })
}

// ------------------------------------------------------------------ dates

/// Days since 1970-01-01 for a civil date (Howard Hinnant).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// A period as local days `[a, b)`.
fn period_days(s: &str, now: Now) -> Option<(i64, i64)> {
    let today = (now.unix + now.utc_offset).div_euclid(86_400);
    let (y, m, _) = civil_from_days(today);
    let month = |y: i64, m: i64| {
        (days_from_civil(y, m, 1), {
            let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
            days_from_civil(ny, nm, 1)
        })
    };
    let monday = today - (today + 3).rem_euclid(7);
    Some(match s.trim().to_ascii_lowercase().as_str() {
        "today" | "heute" => (today, today + 1),
        "yesterday" | "gestern" => (today - 1, today),
        "thisweek" => (monday, monday + 7),
        "lastweek" => (monday - 7, monday),
        "thismonth" => month(y, m),
        "lastmonth" => {
            if m == 1 {
                month(y - 1, 12)
            } else {
                month(y, m - 1)
            }
        }
        "thisyear" => (days_from_civil(y, 1, 1), days_from_civil(y + 1, 1, 1)),
        "lastyear" => (days_from_civil(y - 1, 1, 1), days_from_civil(y, 1, 1)),
        other => {
            let parts: Vec<&str> = other.split(['-', '.', '/']).collect();
            let num = |p: &str| -> Option<i64> {
                (!p.is_empty() && p.len() <= 4 && p.bytes().all(|b| b.is_ascii_digit()))
                    .then(|| p.parse().ok())
                    .flatten()
            };
            match parts.as_slice() {
                [y] if y.len() == 4 => {
                    let y = num(y)?;
                    (days_from_civil(y, 1, 1), days_from_civil(y + 1, 1, 1))
                }
                [y, m] if y.len() == 4 => {
                    let (y, m) = (num(y)?, num(m)?);
                    if !(1..=12).contains(&m) {
                        return None;
                    }
                    month(y, m)
                }
                [y, m, d] if y.len() == 4 => {
                    let (y, m, d) = (num(y)?, num(m)?, num(d)?);
                    let (a, b) = month(y, m);
                    if !(1..=12).contains(&m) || d < 1 || d > b - a {
                        return None;
                    }
                    let day = days_from_civil(y, m, d);
                    (day, day + 1)
                }
                _ => return None,
            }
        }
    })
}

fn parse_date(s: &str, now: Now) -> Option<(i64, i64)> {
    let to_utc = |days: i64| days * 86_400 - now.utc_offset;
    ranged(
        s,
        |p| period_days(p, now),
        |(a, b)| (to_utc(a), to_utc(b)),
        i64::MIN,
        i64::MAX,
    )
}

// ------------------------------------------------------------------ matching

/// `?` = one byte, `*` = any run; whole text (ASCII or no `?`).
fn glob_bytes(p: &[u8], s: &[u8]) -> bool {
    let (mut pi, mut si) = (0usize, 0usize);
    let (mut star, mut mark) = (None, 0usize);
    while si < s.len() {
        if pi < p.len() && (p[pi] == b'?' || p[pi] == s[si]) {
            pi += 1;
            si += 1;
        } else if pi < p.len() && p[pi] == b'*' {
            star = Some(pi);
            mark = si;
            pi += 1;
        } else if let Some(st) = star {
            pi = st + 1;
            mark += 1;
            si = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|&c| c == b'*')
}

/// `?` = one char, `*` = any run; whole text.
fn glob(p: &[char], s: &str) -> bool {
    let s: Vec<char> = s.chars().collect();
    let (mut pi, mut si) = (0usize, 0usize);
    let (mut star, mut mark) = (None, 0usize);
    while si < s.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == s[si]) {
            pi += 1;
            si += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = si;
            pi += 1;
        } else if let Some(st) = star {
            pi = st + 1;
            mark += 1;
            si = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|&c| c == '*')
}

/// [`Term`] on the columns of one entry (only for [`Query::columns_only`]).
fn col_match(t: &Term, flags: u8, size: u64, mtime: i64) -> bool {
    let dir = flags & FLAG_DIR != 0;
    match t {
        Term::Any => true,
        Term::Size(lo, hi) => !dir && *lo <= size && size < *hi,
        Term::Date(lo, hi) => mtime != 0 && *lo <= mtime && mtime < *hi,
        Term::Kind { dir: d } => dir == *d,
        Term::Not(t) => !col_match(t, flags, size, mtime),
        Term::All(ts) => ts.iter().all(|t| col_match(t, flags, size, mtime)),
        _ => false,
    }
}

/// Worker threads for one search (`EXPLORER2_INDEX_THREADS` for measurements).
fn threads() -> usize {
    std::env::var("EXPLORER2_INDEX_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get()))
        .clamp(1, 16)
}

/// Per-query precomputed data.
struct Pre {
    /// `path:word` terms: entry (or an ancestor) matches.
    marks: Vec<Vec<bool>>,
    /// `ext:` terms: the extension ids they accept.
    exts: Vec<Vec<u16>>,
}

struct Ctx<'a> {
    index: &'a Index,
    pre: &'a Pre,
    i: usize,
    path: Option<String>,
    lower_path: Option<String>,
}

impl Ctx<'_> {
    fn path(&mut self) -> &str {
        if self.path.is_none() {
            self.path = Some(self.index.path(self.i as u32));
        }
        self.path.as_deref().unwrap_or("")
    }
    fn lower_path(&mut self) -> &str {
        if self.lower_path.is_none() {
            let p = self.path().to_lowercase();
            self.lower_path = Some(p);
        }
        self.lower_path.as_deref().unwrap_or("")
    }
}

fn matches(t: &Term, c: &mut Ctx) -> bool {
    let is_dir = || c.index.flags_of(c.i) & FLAG_DIR != 0;
    match t {
        Term::Any => true,
        Term::Not(t) => !matches(t, c),
        Term::All(ts) => ts.iter().all(|t| matches(t, c)),
        Term::Kind { dir } => is_dir() == *dir,
        Term::Size(lo, hi) => !is_dir() && (*lo..*hi).contains(&c.index.raw(c.i).1),
        Term::Date(lo, hi) => {
            let m = c.index.raw(c.i).2;
            m != 0 && (*lo..*hi).contains(&m)
        }
        Term::Ext { exts, slot } => match c.index.ext_id(c.i) {
            EXT_NONE => false,
            // table full: compare the text
            EXT_OTHER => {
                let name = c.index.lower_bytes(c.i);
                memchr::memrchr(b'.', name).is_some_and(|dot| {
                    dot > 0 && exts.iter().any(|x| x.as_bytes() == &name[dot + 1..])
                })
            }
            id => c.pre.exts.get(*slot).is_some_and(|ids| ids.contains(&id)),
        },
        Term::Text { slot: Some(k), .. } => c.pre.marks.get(*k).is_some_and(|m| m[c.i]),
        Term::Text {
            pat, path, case, ..
        } => {
            let hay: &[u8] = match (path, case) {
                (false, false) => c.index.lower_bytes(c.i),
                (false, true) => c.index.name_bytes(c.i),
                (true, false) => {
                    c.lower_path();
                    c.lower_path.as_deref().unwrap_or("").as_bytes()
                }
                (true, true) => {
                    c.path();
                    c.path.as_deref().unwrap_or("").as_bytes()
                }
            };
            pat.is_match(hay)
        }
        Term::Regex { re, path, .. } => {
            if *path {
                let p = c.path().to_string();
                re.is_match(&p)
            } else {
                re.is_match(c.index.name(c.i as u32))
            }
        }
    }
}

fn hit(index: &Index, q: &Query, pre: &Pre, i: usize, scope: Option<u32>) -> bool {
    let mut c = Ctx {
        index,
        pre,
        i,
        path: None,
        lower_path: None,
    };
    q.groups
        .iter()
        .all(|g| g.iter().any(|t| matches(t, &mut c)))
        && scope.is_none_or(|s| index.is_under(i as u32, s))
}

/// `path:` terms without separators: per entry, does its own name or an
/// ancestor's contain the word? One pass, memoized up the tree (no paths built).
fn path_marks(index: &Index, needle: &Finder) -> Vec<bool> {
    let n = index.slots();
    // 0 = unknown, 1 = yes, 2 = no
    let mut st = vec![0u8; n];
    let mut chain = Vec::new();
    for i in 0..n {
        if st[i] != 0 {
            continue;
        }
        let mut cur = i;
        let res = loop {
            if st[cur] != 0 {
                break st[cur];
            }
            if needle.find(index.lower_bytes(cur)).is_some() {
                st[cur] = 1;
                break 1;
            }
            chain.push(cur);
            let p = index.parent(cur as u32);
            if p == NONE {
                break 2;
            }
            cur = p as usize;
        };
        for c in chain.drain(..) {
            st[c] = res;
        }
    }
    st.into_iter().map(|s| s == 1).collect()
}

fn precompute(index: &Index, q: &Query) -> Pre {
    let mut pre = Pre {
        marks: vec![Vec::new(); q.slots],
        exts: vec![Vec::new(); q.ext_slots],
    };
    fn walk(t: &Term, index: &Index, pre: &mut Pre) {
        match t {
            Term::Not(t) => walk(t, index, pre),
            Term::All(ts) => ts.iter().for_each(|t| walk(t, index, pre)),
            Term::Text {
                pat: Pat::Sub(f),
                slot: Some(k),
                ..
            } => pre.marks[*k] = path_marks(index, f),
            Term::Ext { exts, slot } => {
                // unknown extensions match nothing; a full table falls back to text
                pre.exts[*slot] = exts.iter().filter_map(|e| index.ext_lookup(e)).collect();
                pre.exts[*slot].push(EXT_OTHER);
            }
            _ => {}
        }
    }
    q.groups
        .iter()
        .flatten()
        .for_each(|t| walk(t, index, &mut pre));
    pre
}

/// Parallel sequential scan of the name-ordered part, plus the unsorted
/// rest merged in by name.
pub(crate) fn run(index: &Index, q: &Query, scope: Option<u32>) -> Vec<u32> {
    let scope = scope.filter(|&s| s != NONE && index.exists(s));
    let pre = precompute(index, q);
    let ok = |id: u32, skip_dirty: bool| {
        let f = index.flags_of(id as usize);
        f & FLAG_GONE == 0
            && !(skip_dirty && f & FLAG_DIRTY != 0)
            && hit(index, q, &pre, id as usize, scope)
    };
    let n = index.ordered();
    let threads = threads();
    let scan = |r: std::ops::Range<usize>| -> Vec<u32> {
        r.map(|i| i as u32).filter(|&id| ok(id, true)).collect()
    };
    // parallel over chunks; each chunk yields ascending ids → concatenation keeps order
    let par = |parts: usize,
               len: usize,
               f: &(dyn Fn(std::ops::Range<usize>) -> Vec<u32> + Sync)|
     -> Vec<u32> {
        let parts = parts.max(1);
        let chunk = len.div_ceil(parts).max(1);
        std::thread::scope(|s| {
            let handles: Vec<_> = (0..parts)
                .map(|t| s.spawn(move || f(t * chunk..((t + 1) * chunk).min(len))))
                .collect();
            let parts: Vec<Vec<u32>> = handles
                .into_iter()
                .map(|h| h.join().unwrap_or_default())
                .collect();
            let mut out = Vec::with_capacity(parts.iter().map(Vec::len).sum());
            for p in parts {
                out.extend_from_slice(&p);
            }
            out
        })
    };
    let exact = q.exact_needle();
    let verify = |mut cands: Vec<u32>| -> Vec<u32> {
        // one needle: chunks come in order (at most a boundary duplicate)
        if !cands.windows(2).all(|w| w[0] <= w[1]) {
            cands.sort_unstable();
        }
        cands.dedup();
        cands
            .into_iter()
            .filter(|&id| {
                (id as usize) < n
                    && if exact {
                        // stale bytes of renamed/removed entries are skipped here
                        index.flags_of(id as usize) & (FLAG_GONE | FLAG_DIRTY) == 0
                            && scope.is_none_or(|s| index.is_under(id, s))
                    } else {
                        ok(id, true)
                    }
            })
            .collect()
    };
    let driver = q.driver();
    let main: Vec<u32> = if let Some(Driver::Needles(needles)) = &driver {
        let (buf, starts) = index.ordered_names();
        let parts = if buf.len() < 1 << 20 { 1 } else { threads };
        verify(par(parts, buf.len(), &|r| {
            needles
                .iter()
                .flat_map(|nd| buffer_hits(buf, starts, nd, r.clone()))
                .collect()
        }))
    } else if let Some(Driver::Exts(slots)) = &driver {
        let mut want: Vec<u16> = slots
            .iter()
            .flat_map(|&s| pre.exts[s].iter().copied())
            .collect();
        want.sort_unstable();
        want.dedup();
        let ids = &index.ext_ids()[..n];
        let parts = if n < 50_000 { 1 } else { threads };
        // already ascending: no sort needed before the check
        par(parts, n, &|r| {
            r.filter(|&i| want.binary_search(&ids[i]).is_ok())
                .map(|i| i as u32)
                .filter(|&id| ok(id, true))
                .collect()
        })
    } else if q.columns_only() {
        let (flags, size, mtime) = index.columns();
        let (flags, size, mtime) = (&flags[..n], &size[..n], &mtime[..n]);
        let parts = if n < 50_000 { 1 } else { threads };
        // one plain condition (the common case) without the group machinery
        let single = match q.groups.as_slice() {
            [g] => match g.as_slice() {
                [t] => Some(t),
                _ => None,
            },
            _ => None,
        };
        par(parts, n, &|r| {
            let mut out = Vec::with_capacity(r.len() / 4);
            for i in r {
                let f = flags[i];
                if f & (FLAG_GONE | FLAG_DIRTY) != 0 {
                    continue;
                }
                let hit = match single {
                    Some(t) => col_match(t, f, size[i], mtime[i]),
                    None => q
                        .groups
                        .iter()
                        .all(|g| g.iter().any(|t| col_match(t, f, size[i], mtime[i]))),
                };
                if hit && scope.is_none_or(|s| index.is_under(i as u32, s)) {
                    out.push(i as u32);
                }
            }
            out
        })
    } else if n < 50_000 || threads < 2 {
        scan(0..n)
    } else {
        let chunk = n.div_ceil(threads);
        std::thread::scope(|s| {
            let handles: Vec<_> = (0..threads)
                .map(|t| s.spawn(move || scan(t * chunk..((t + 1) * chunk).min(n))))
                .collect();
            let mut out = Vec::new();
            for h in handles {
                out.extend(h.join().unwrap_or_default());
            }
            out
        })
    };
    let mut rest: Vec<u32> = index.unsorted_ids().filter(|&id| ok(id, false)).collect();
    rest.sort_unstable();
    rest.dedup();
    index.sort(&mut rest, crate::SortBy::Name, false);
    index.merge_sorted(main, rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) const NOW: Now = Now {
        // 2026-11-05 13:00 UTC, Berlin +1 h
        unix: 1_793_883_600,
        utc_offset: 3600,
    };

    #[test]
    fn dates() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(civil_from_days(days_from_civil(2026, 11, 5)), (2026, 11, 5));
        assert_eq!(
            civil_from_days((NOW.unix + NOW.utc_offset) / 86_400),
            (2026, 11, 5)
        );
        let today = parse_date("today", NOW).unwrap();
        assert_eq!(today.1 - today.0, 86_400);
        // local midnight = 23:00 UTC the day before
        assert_eq!(today.0, days_from_civil(2026, 11, 5) * 86_400 - 3600);
        assert_eq!(parse_date("2026-11-05", NOW), Some(today));
        let (a, b) = parse_date("2026-11", NOW).unwrap();
        assert_eq!((b - a) / 86_400, 30);
        assert_eq!(
            parse_date(">=2026-11-05", NOW).unwrap(),
            (today.0, i64::MAX)
        );
        assert_eq!(parse_date("<2026-11-05", NOW).unwrap(), (i64::MIN, today.0));
        assert_eq!(parse_date(">2026-11-05", NOW).unwrap(), (today.1, i64::MAX));
        let (a, b) = parse_date("2026-01..2026-03", NOW).unwrap();
        assert_eq!((b - a) / 86_400, 31 + 28 + 31);
        let w = parse_date("thisweek", NOW).unwrap();
        // 2026-11-05 is a Thursday → week starts Monday 2026-11-02
        assert_eq!(w.0, days_from_civil(2026, 11, 2) * 86_400 - 3600);
        for bad in [
            "2026-13",
            "2026-02-30",
            "x",
            "",
            "26-1-1",
            "2026-11-05..2026-01-01",
        ] {
            assert_eq!(parse_date(bad, NOW), None, "{bad}");
        }
    }

    #[test]
    fn sizes() {
        assert_eq!(parse_size("10"), Some((10, 11)));
        assert_eq!(parse_size(">10mb"), Some((10 * 1024 * 1024 + 1, u64::MAX)));
        assert_eq!(parse_size("<=1kb"), Some((0, 1025)));
        assert_eq!(parse_size("1kb..2kb"), Some((1024, 2049)));
        assert_eq!(parse_size("1.5k"), Some((1536, 1537)));
        assert_eq!(parse_size("empty"), Some((0, 1)));
        for bad in ["", "x", "10xb", ">", "-5", "1e400"] {
            assert_eq!(parse_size(bad), None, "{bad}");
        }
    }

    #[test]
    fn globbing() {
        let g = |p: &str, s: &str| glob(&p.chars().collect::<Vec<_>>(), s);
        assert!(g("*.jpg", "a.jpg"));
        assert!(!g("*.jpg", "a.jpg.txt"));
        assert!(g("re?ort*", "report 2026.pdf"));
        assert!(g("*", ""));
        assert!(g("a*b*c", "aXXbYYc"));
        assert!(!g("a*b*c", "aXXbYY"));
        assert!(g("ü?", "üx"));
    }

    #[test]
    fn grouping() {
        let q = Query::parse("a b|c", NOW).unwrap();
        assert_eq!(q.groups.len(), 2);
        assert_eq!(q.groups[1].len(), 2);
        let q = Query::parse("a | b c", NOW).unwrap();
        assert_eq!(
            q.groups.iter().map(Vec::len).collect::<Vec<_>>(),
            vec![2, 1]
        );
        let q = Query::parse("\"a b\"|c", NOW).unwrap();
        assert_eq!(q.groups.len(), 1);
        assert!(Query::parse("", NOW).unwrap().is_empty());
        assert!(Query::parse("   ", NOW).unwrap().is_empty());
        assert!(Query::parse("regex:(", NOW).is_err());
        assert!(Query::parse("size:big", NOW).is_err());
    }
}
