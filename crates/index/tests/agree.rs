//! The fast paths (name-ordered layout, buffer search, precomputed `path:`)
//! must find exactly what the plain per-entry check finds – also after
//! renames, moves, removals and additions since the last rebuild.

use explorer_index::{Index, Meta, Now, Query, SortBy};

const NOW: Now = Now {
    unix: 1_793_883_600,
    utc_offset: 3600,
};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

const PARTS: &[&str] = &[
    "ſtraße",
    "\u{212a}elvin",
    "IMG_123",
    "Img_9",
    "noteS",
    "bericht",
    "Bericht",
    "foto",
    "IMG",
    "img_",
    "garten",
    "Garten",
    "ü",
    "Ü",
    "straße",
    "test",
    "a",
    "b",
    "2026",
    "(Kopie)",
    " ",
    "-",
    "_",
    ".",
    "x",
    "notes",
    "ÄÖÜ",
    "e\u{301}",
];
const EXTS: &[&str] = &["txt", "jpg", "JPG", "pdf", "rs", "", "tar.gz"];

fn name(r: &mut Rng) -> String {
    let mut s: String = (0..1 + r.below(4))
        .map(|_| PARTS[r.below(PARTS.len())])
        .collect();
    let e = EXTS[r.below(EXTS.len())];
    if !e.is_empty() {
        s.push('.');
        s.push_str(e);
    }
    s
}

fn tree(seed: u64, n: usize) -> Index {
    let mut r = Rng(seed);
    let mut ix = Index::new(true);
    let root = ix.add_root(
        r"C:\",
        Meta {
            is_dir: true,
            size: 0,
            mtime: 1,
        },
    );
    let mut dirs = vec![root];
    for _ in 0..n {
        let p = dirs[r.below(dirs.len())];
        let is_dir = r.below(6) == 0;
        let meta = Meta {
            is_dir,
            size: r.next() % 5_000_000,
            mtime: NOW.unix - (r.next() % (900 * 86_400)) as i64,
        };
        let nm = name(&mut r);
        // a folder never holds the same name twice
        if ix.child(p, &nm).is_some() {
            continue;
        }
        let id = ix.add(p, &nm, meta);
        if is_dir {
            dirs.push(id);
        }
    }
    ix
}

/// Same content, two layouts: `fast` ordered (+ later changes), `plain` never ordered.
fn churn(fast: &mut Index, plain: &mut Index, seed: u64) {
    let mut r = Rng(seed);
    for _ in 0..300 {
        let slots = fast.slots() as u32;
        let id = r.below(slots as usize) as u32;
        if !fast.exists(id) || fast.parent(id) == explorer_index::NONE {
            continue;
        }
        // the same entry in `plain`, found by path
        let Some(pid) = plain.find(&fast.path(id)) else {
            continue;
        };
        match r.below(4) {
            0 => {
                let nm = name(&mut r);
                let (fp, pp) = (fast.parent(id), plain.parent(pid));
                if fast.child(fp, &nm).is_none() {
                    fast.rename(id, fp, &nm);
                    plain.rename(pid, pp, &nm);
                }
            }
            1 => {
                fast.remove(id);
                plain.remove(pid);
            }
            2 => {
                let nm = name(&mut r);
                let target = if fast.is_dir(id) { id } else { fast.parent(id) };
                let ptarget = plain.find(&fast.path(target));
                if let (Some(pt), None) = (ptarget, fast.child(target, &nm)) {
                    let meta = Meta {
                        is_dir: r.below(3) == 0,
                        size: r.next() % 9999,
                        mtime: NOW.unix,
                    };
                    fast.add(target, &nm, meta);
                    plain.add(pt, &nm, meta);
                }
            }
            _ => {
                // move into another folder
                let to = r.below(slots as usize) as u32;
                if fast.exists(to) && fast.is_dir(to) && !fast.is_under(to, id) {
                    let nm = fast.name(id).to_string();
                    if fast.child(to, &nm).is_none() {
                        if let Some(pto) = plain.find(&fast.path(to)) {
                            fast.rename(id, to, &nm);
                            plain.rename(pid, pto, &nm);
                        }
                    }
                }
            }
        }
    }
}

const QUERIES: &[&str] = &[
    r"regex:^img_\d{3}\.jpg$",
    r"regex:^img_\d",
    "regex:straße",
    "regex:strasse",
    "regex:kelvin",
    r"regex:notes$",
    "regex:bericht|notes",
    "regex:(?-i)Bericht",
    "case:regex:Bericht",
    "regex:.*",
    r"regex:\.jpg$",
    "ſtraße",
    "straße",
    "kelvin",
    "ext:jpg regex:img",
    "bericht",
    "BERICHT",
    "img_",
    "*.jpg",
    "*.JPG",
    "img*",
    "ber*t",
    "?ü*",
    "garten foto",
    "foto|garten",
    "!foto",
    "ext:jpg;pdf",
    "ext:gz",
    "ext:tar.gz",
    "file:",
    "folder:",
    "folder:garten",
    "size:>1mb",
    "size:<1kb",
    "dm:thisyear",
    "dm:<2026",
    "path:garten",
    "path:garten ext:txt",
    "!path:garten",
    r"path:garten\b",
    "path:*garten*",
    "case:Bericht",
    "case:path:Garten",
    "straße",
    "STRASSE",
    "ü",
    "\"(kopie)\"",
    "e\u{301}",
    "regex:^b.*\\.txt$",
    "regex:a|b",
    "notes 2026 !jpg",
    "test|notes ext:rs",
    "2026",
    "x",
    "a b",
    "",
];

/// Equal, else a short report of the difference.
fn same(a: Vec<String>, b: Vec<String>, what: &str) {
    if a != b {
        let only_a: Vec<&String> = a.iter().filter(|x| !b.contains(x)).take(5).collect();
        let only_b: Vec<&String> = b.iter().filter(|x| !a.contains(x)).take(5).collect();
        panic!(
            "{what}: fast {} vs plain {} hits; only fast: {only_a:?}; only plain: {only_b:?}",
            a.len(),
            b.len()
        );
    }
}

fn paths(ix: &Index, q: &str, scope: Option<u32>) -> Vec<String> {
    let query = Query::parse(q, NOW).unwrap_or_else(|e| panic!("{q}: {e}"));
    let mut v: Vec<String> = ix
        .search(&query, scope, SortBy::Name, false)
        .into_iter()
        .map(|i| ix.path(i))
        .collect();
    v.sort();
    v
}

#[test]
fn fast_paths_agree_with_the_plain_check() {
    // two trees > 50k entries (parallel scan + multi-threaded buffer search),
    // three small ones (single-threaded paths); `AGREE_SEEDS` for more
    let extra: u64 = std::env::var("AGREE_SEEDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    for seed in 1..=5 + extra {
        let size = if seed <= 2 { 52_000 } else { 6_000 };
        let mut fast = tree(seed, size);
        let mut plain = fast.clone();
        fast.rebuild_order();
        churn(&mut fast, &mut plain, seed * 77);
        assert_eq!(fast.len(), plain.len());
        for q in QUERIES {
            same(
                paths(&fast, q, None),
                paths(&plain, q, None),
                &format!("seed {seed}, query {q:?}"),
            );
        }
        // scoped to a folder
        let some_dir = (0..fast.slots() as u32)
            .find(|&i| fast.exists(i) && fast.is_dir(i) && fast.children(i).count() > 3)
            .unwrap();
        let pdir = plain.find(&fast.path(some_dir)).unwrap();
        for q in ["", "bericht", "*.jpg", "path:garten"] {
            same(
                paths(&fast, q, Some(some_dir)),
                paths(&plain, q, Some(pdir)),
                &format!("scope, {q:?}"),
            );
        }
        // results arrive sorted by name in both layouts
        let q = Query::parse("bericht", NOW).unwrap();
        let hits = fast.search(&q, None, SortBy::Name, false);
        assert!(hits.windows(2).all(
            |w| explorer_index::natural(fast.name(w[0]), fast.name(w[1]))
                != std::cmp::Ordering::Greater
        ));
        // and again after a rebuild of the changed index
        fast.rebuild_order();
        for q in QUERIES {
            same(
                paths(&fast, q, None),
                paths(&plain, q, None),
                &format!("rebuilt, seed {seed}, {q:?}"),
            );
        }
    }
}
