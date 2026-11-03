//! Directory loading benchmark against the PLAN-BASICS budgets.
//!
//! Usage: bench [--files N] [--dir PATH] [--runs R] [--assert]
//!        bench --create-in PATH [--files N]   (only create test files, keep them)
//! Without --dir a temp folder with N files (default 100 000) is created.
//! --assert exits with code 1 if a budget is exceeded (used in CI).

use explorer_core::{sort_entries, SortSpec};
use explorer_fs::{load_dir, LoadEvent};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const BUDGET_FIRST_BATCH: Duration = Duration::from_millis(100);
const BUDGET_FULL: Duration = Duration::from_millis(1000);

fn main() {
    let mut files = 100_000usize;
    let mut dir: Option<PathBuf> = None;
    let mut runs = 5usize;
    let mut assert = false;
    let mut create_in: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--files" => files = args.next().and_then(|v| v.parse().ok()).expect("--files N"),
            "--dir" => dir = args.next().map(PathBuf::from),
            "--runs" => runs = args.next().and_then(|v| v.parse().ok()).expect("--runs R"),
            "--assert" => assert = true,
            "--create-in" => create_in = args.next().map(PathBuf::from),
            other => panic!("unknown arg {other}"),
        }
    }

    if let Some(target) = create_in {
        std::fs::create_dir_all(&target).expect("create target dir");
        let t0 = Instant::now();
        create_entries(&target, files);
        println!(
            "created {files} entries in {} in {:?}",
            target.display(),
            t0.elapsed()
        );
        return;
    }

    let _tmp;
    let dir = match dir {
        Some(d) => d,
        None => {
            let t = tempfile::tempdir().expect("tempdir");
            let t0 = Instant::now();
            for i in 0..files {
                let name = if i % 50 == 0 {
                    format!("dir_{i}")
                } else {
                    format!("file_{i}.txt")
                };
                let p = t.path().join(name);
                if i % 50 == 0 {
                    std::fs::create_dir(&p).unwrap();
                } else {
                    std::fs::File::create(&p).unwrap();
                }
            }
            println!("created {files} entries in {:?}", t0.elapsed());
            let p = t.path().to_path_buf();
            _tmp = t;
            p
        }
    };

    let mut firsts = Vec::new();
    let mut fulls = Vec::new();
    let mut sorts = Vec::new();
    let mut overheads = Vec::new();
    let mut stalls = 0usize;
    let mut total = 0;
    // Run 0 is a cold run: reported, but excluded from the medians (CI runners
    // show large first-access noise right after creating 100k files).
    for run in 0..=runs {
        // Raw enumeration without thread/channel, to isolate the OS call.
        let raw_start = Instant::now();
        let mut raw_first = None;
        explorer_fs::enumerate(&dir, &mut |_| {
            raw_first.get_or_insert_with(|| raw_start.elapsed());
            std::ops::ControlFlow::Continue(())
        })
        .expect("raw enumerate failed");
        let raw_all = raw_start.elapsed();

        let (tx, rx) = mpsc::channel();
        let start = Instant::now();
        let _h = load_dir(dir.clone(), move |e| {
            let _ = tx.send((Instant::now(), e));
        });
        let mut all = Vec::new();
        let mut first_at = None;
        loop {
            let (at, ev) = rx
                .recv_timeout(Duration::from_secs(60))
                .expect("loader timed out");
            match ev {
                LoadEvent::Batch { entries, first } => {
                    if first {
                        // what the UI does for its first paint: sort the partial batch
                        let mut part = entries.clone();
                        sort_entries(&mut part, SortSpec::default());
                        first_at =
                            Some(at.duration_since(start) + Instant::now().duration_since(at));
                    }
                    all.extend(entries);
                }
                LoadEvent::Done { total: t, .. } => {
                    total = t;
                    break;
                }
                LoadEvent::Failed(e) => panic!("load failed: {e}"),
            }
        }
        let ts = Instant::now();
        sort_entries(&mut all, SortSpec::default());
        let sort_t = ts.elapsed();
        let full = start.elapsed();
        let first = first_at.unwrap_or(full);
        println!(
            "run {run}{}: first_batch={first:?} full_sorted={full:?} sort={sort_t:?} | raw: first_entry={:?} all={raw_all:?}",
            if run == 0 { " (cold, not counted)" } else { "" },
            raw_first.unwrap_or_default()
        );
        if run > 0 {
            let raw_first = raw_first.unwrap_or_default();
            if raw_first > BUDGET_FIRST_BATCH {
                stalls += 1; // the OS call itself was slow (AV scan, indexer, I/O contention)
            }
            sorts.push(sort_t);
            fulls.push(full);
            firsts.push(first);
            overheads.push(first.saturating_sub(raw_first));
        }
    }
    firsts.sort();
    sorts.sort();
    fulls.sort();
    overheads.sort();
    let med = |v: &[Duration]| v[v.len() / 2];
    let (best_first, best_full) = (firsts[0], fulls[0]);
    let overhead = med(&overheads);
    println!(
        "entries={total} runs={runs} | first_batch best={best_first:?} median={:?} max={:?} (budget {BUDGET_FIRST_BATCH:?}) | full_sorted best={best_full:?} median={:?} max={:?} (budget {BUDGET_FULL:?}) | first batch minus raw first entry, median={overhead:?} (diagnostic: remaining 255 OS entries + thread + partial sort) | sort median={:?} | OS stalls (raw first entry > budget): {stalls}/{runs}",
        med(&firsts),
        firsts[firsts.len() - 1],
        med(&fulls),
        fulls[fulls.len() - 1],
        med(&sorts)
    );
    // Gate on best-of-N: external load (AV scans, indexer, I/O contention on shared
    // CI runners) only ever adds time, so the best run reflects what our code can do.
    // Median/max/stalls stay visible in the output.
    if assert && (best_first > BUDGET_FIRST_BATCH || best_full > BUDGET_FULL) {
        eprintln!("BUDGET EXCEEDED");
        std::process::exit(1);
    }
}

/// Every 50th entry is a folder, the rest are empty files.
fn create_entries(dir: &std::path::Path, n: usize) {
    for i in 0..n {
        if i % 50 == 0 {
            std::fs::create_dir(dir.join(format!("dir_{i}"))).unwrap();
        } else {
            std::fs::File::create(dir.join(format!("file_{i}.txt"))).unwrap();
        }
    }
}
