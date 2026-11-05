//! Random query text: parsing never panics, and every query that parses
//! finds the same entries on the fast and the plain path.

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

const PIECES: &[&str] = &[
    " ",
    "  ",
    "|",
    " | ",
    "!",
    "\"",
    "*",
    "?",
    ":",
    ";",
    ".",
    "..",
    ">",
    "<",
    ">=",
    "<=",
    "=",
    "\\",
    "/",
    "path:",
    "case:",
    "file:",
    "folder:",
    "ext:",
    "size:",
    "dm:",
    "datemodified:",
    "regex:",
    "PATH:",
    "jpg",
    "txt",
    "1kb",
    "2mb",
    "10",
    "empty",
    "huge",
    "today",
    "2026",
    "2026-11",
    "2026-11-05",
    "thisweek",
    "a",
    "b",
    "bericht",
    "ü",
    "Ü",
    "ß",
    "(",
    ")",
    "[",
    "]",
    "{",
    "}",
    "^",
    "$",
    "+",
    "\\d",
    ".*",
    "\0",
    "😀",
    "e\u{301}",
    "\t",
    "\n",
];

fn random_query(r: &mut Rng) -> String {
    (0..r.below(8))
        .map(|_| {
            if r.below(6) == 0 {
                char::from_u32(r.next() as u32 % 0x11_0000)
                    .map(String::from)
                    .unwrap_or_default()
            } else {
                PIECES[r.below(PIECES.len())].to_string()
            }
        })
        .collect()
}

fn small_tree() -> Index {
    let mut ix = Index::new(true);
    let root = ix.add_root(
        r"C:\",
        Meta {
            is_dir: true,
            size: 0,
            mtime: 1,
        },
    );
    let mut r = Rng(99);
    let mut dirs = vec![root];
    for i in 0..3000 {
        let p = dirs[r.below(dirs.len())];
        let nm = format!(
            "{}{}{}",
            PIECES[r.below(PIECES.len())].replace(['\\', '/', '\0'], "_"),
            i,
            [".jpg", ".TXT", "", ".tar.gz", ".ü"][r.below(5)]
        );
        let meta = Meta {
            is_dir: r.below(5) == 0,
            size: r.next() % 3_000_000,
            mtime: NOW.unix - (r.next() % (800 * 86_400)) as i64,
        };
        let id = ix.add(p, &nm, meta);
        if meta.is_dir {
            dirs.push(id);
        }
    }
    ix
}

#[test]
fn random_queries() {
    let plain = small_tree();
    let mut fast = plain.clone();
    fast.rebuild_order();
    let rounds: usize = std::env::var("FUZZ_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);
    let seed: u64 = std::env::var("FUZZ_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut r = Rng(0xf00d ^ seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
    let mut parsed = 0;
    for _ in 0..rounds {
        let text = random_query(&mut r);
        let Ok(q) = Query::parse(&text, NOW) else {
            continue;
        };
        parsed += 1;
        let paths = |ix: &Index| {
            let mut v: Vec<String> = ix
                .search(&q, None, SortBy::Name, false)
                .into_iter()
                .map(|i| ix.path(i))
                .collect();
            v.sort();
            v
        };
        assert_eq!(paths(&fast), paths(&plain), "query {text:?}");
    }
    assert!(parsed > rounds / 3, "too few queries parsed: {parsed}");
}
