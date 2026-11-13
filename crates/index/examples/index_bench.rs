//! Search benchmark (PLAN-SEARCH.md): a synthetic tree like a real drive,
//! queries typed by a user, best-of-N times.
//!
//!   cargo run --release -p explorer-index --example index_bench -- [entries] [--gate]
//!
//! `--gate`: exit 1 if a name query exceeds 10 ms (best of 9), `path:`/`regex:`
//! 60 ms, or the index needs more than 100 MB for 1 M entries.

use explorer_index::{Index, Meta, Now, Query, SortBy};
use std::time::Instant;

const WORDS: &[&str] = &[
    "Bericht",
    "Rechnung",
    "Urlaub",
    "Foto",
    "IMG",
    "DSC",
    "Dokument",
    "Projekt",
    "Backup",
    "Notizen",
    "Vertrag",
    "Präsentation",
    "Übersicht",
    "Kalkulation",
    "Brief",
    "Angebot",
    "screenshot",
    "video",
    "song",
    "readme",
    "index",
    "main",
    "config",
    "test",
    "lib",
    "data",
    "Steuer",
    "Bewerbung",
    "Lebenslauf",
    "Familie",
    "Geburtstag",
    "Weihnachten",
    "Garten",
    "Auto",
];
const EXTS: &[&str] = &[
    "jpg", "png", "pdf", "docx", "xlsx", "txt", "mp3", "mp4", "zip", "rs", "js", "json", "dll",
    "exe", "html", "css", "md", "log", "tmp", "ini",
];

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn pick<'a>(&mut self, v: &[&'a str]) -> &'a str {
        v[(self.next() % v.len() as u64) as usize]
    }
}

fn build(n: usize) -> Index {
    let mut r = Rng(0x1234_5678);
    let mut ix = Index::new(true);
    ix.reserve(n, n * 24);
    let now = 1_793_883_600i64;
    let root = ix.add_root(
        r"C:\",
        Meta {
            is_dir: true,
            size: 0,
            mtime: now,
        },
    );
    // (id, depth); real drives are 5-15 levels deep
    let mut dirs = vec![(root, 0u32)];
    while ix.len() < n {
        // ~1 folder per 12 files, folders cluster below recent ones
        let (parent, depth) = dirs[dirs.len() - 1 - (r.next() as usize % dirs.len().min(400))];
        if r.next().is_multiple_of(13) {
            let name = format!("{} {}", r.pick(WORDS), r.next() % 3000);
            let (parent, depth) = if depth >= 12 {
                dirs[(r.next() as usize) % dirs.len().min(64)]
            } else {
                (parent, depth)
            };
            let id = ix.add(
                parent,
                &name,
                Meta {
                    is_dir: true,
                    size: 0,
                    mtime: now - (r.next() % 2000) as i64 * 86_400,
                },
            );
            dirs.push((id, depth + 1));
        } else {
            let name = format!(
                "{}_{}{}.{}",
                r.pick(WORDS),
                r.next() % 100_000,
                if r.next().is_multiple_of(4) {
                    " (Kopie)"
                } else {
                    ""
                },
                r.pick(EXTS)
            );
            ix.add(
                parent,
                &name,
                Meta {
                    is_dir: false,
                    size: r.next() % (64 << 20),
                    mtime: now - (r.next() % (3 * 365 * 86_400)) as i64,
                },
            );
        }
    }
    ix
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let n: usize = args
        .iter()
        .find_map(|a| a.parse().ok())
        .unwrap_or(1_000_000);
    let gate = args.iter().any(|a| a == "--gate");
    let now = Now {
        unix: 1_793_883_600,
        utc_offset: 3600,
        offset_at: None,
    };
    let threads = std::thread::available_parallelism().map_or(1, |t| t.get());

    let t0 = Instant::now();
    let mut ix = build(n);
    let build_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let t1 = Instant::now();
    ix.rebuild_order();
    let order_ms = t1.elapsed().as_secs_f64() * 1000.0;
    let mb = ix.memory_bytes() as f64 / (1024.0 * 1024.0);
    println!(
        "index: {} entries, built in {build_ms:.0} ms, name order in {order_ms:.0} ms, memory {mb:.1} MB ({:.0} B/entry), {threads} threads",
        ix.len(),
        ix.memory_bytes() as f64 / ix.len() as f64
    );

    // (query, counts for the 10 ms gate)
    let queries: &[(&str, bool)] = &[
        ("b", true),
        ("bericht", true),
        ("rech 2026", true),
        ("urlaub|foto", true),
        ("*.jpg", true),
        ("img_1*.png", true),
        ("ext:jpg;png;mp4", true),
        ("size:>32mb", true),
        ("dm:thismonth", true),
        ("übersicht !kopie", true),
        ("folder:projekt", true),
        ("bewerbung ext:pdf dm:2026", true),
        ("path:garten", false),
        (r"regex:^img_\d{3}\.jpg$", false),
    ];
    let mut worst = 0.0f64;
    let mut failed = Vec::new();
    println!(
        "{:<30} {:>9} {:>10} {:>10} {:>12}",
        "query (sorted by name)", "hits", "best ms", "median ms", "resort size"
    );
    for (text, gated) in queries {
        let q = Query::parse(text, now).expect("query");
        let mut times = Vec::new();
        let mut hits = Vec::new();
        for _ in 0..9 {
            let t = Instant::now();
            hits = ix.search(&q, None, SortBy::Name, false);
            times.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        times.sort_by(f64::total_cmp);
        let t = Instant::now();
        ix.sort(&mut hits, SortBy::Size, false);
        let sort_ms = t.elapsed().as_secs_f64() * 1000.0;
        println!(
            "{:<30} {:>9} {:>10.2} {:>10.2} {:>12.2}",
            text,
            hits.len(),
            times[0],
            times[4],
            sort_ms
        );
        // name queries 10 ms; path:/regex: (whole paths, regex engine) 60 ms
        let budget = if *gated { 10.0 } else { 60.0 };
        if *gated {
            worst = worst.max(times[0]);
        }
        if times[0] > budget {
            failed.push(format!("{text}: {:.2} ms > {budget} ms", times[0]));
        }
    }
    println!("worst name query (best of 9): {worst:.2} ms (budget 10 ms)");

    // S1.2: save + load (start with a stored index: budget 1 s)
    let dir = std::env::temp_dir().join(format!("e2idx-bench-{}", std::process::id()));
    let file = dir.join(explorer_index::file_name_for(r"C:\", true));
    let t = Instant::now();
    ix.save(&file, "bench", 1).expect("save");
    let save_ms = t.elapsed().as_secs_f64() * 1000.0;
    let file_mb = std::fs::metadata(&file).map_or(0.0, |m| m.len() as f64 / (1024.0 * 1024.0));
    let mut loads = Vec::new();
    for _ in 0..3 {
        let t = Instant::now();
        let (back, _) = Index::load(&file, Some("bench")).expect("load");
        loads.push(t.elapsed().as_secs_f64() * 1000.0);
        assert_eq!(back.len(), ix.len());
    }
    loads.sort_by(f64::total_cmp);
    let _ = std::fs::remove_dir_all(&dir);
    println!(
        "persist: file {file_mb:.1} MB, save {save_ms:.0} ms, load best {:.0} ms (budget 1000 ms)",
        loads[0]
    );
    if loads[0] > 1000.0 {
        failed.push(format!("load {:.0} ms > 1000 ms", loads[0]));
    }
    if gate {
        let per_million = mb * 1_000_000.0 / ix.len() as f64;
        if per_million > 100.0 {
            failed.push(format!("memory {per_million:.1} MB per 1 M entries > 100"));
        }
        if !failed.is_empty() {
            println!("GATE FAILED: {}", failed.join("; "));
            std::process::exit(1);
        }
        println!("gate ok");
    }
}
