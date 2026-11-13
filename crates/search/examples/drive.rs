//! Index a real drive with the search service and measure what the user
//! feels (PLAN-SEARCH.md budgets):
//!
//! 1. first build (no saved index), entries per second → time for 1 M
//! 2. live: a folder with files appears / disappears → seen in a search
//! 3. restart with the saved index: load + compare with the disk
//!
//! `cargo run --release -p explorer-search --example drive -- [root] [--live <dir>] [--gate]`
//! Defaults: Windows `C:\` (live folder %TEMP%), elsewhere $HOME.
//! `--gate`: exit 1 if loading takes ≥ 1 s, a live change ≥ 1 s (best of 3),
//! or the first build would need more than 2 min for 1 M entries.
//!
//! `--probe <dir> --secs N [--threads N]`: only walk `dir` for at most N
//! seconds and print entries/s next to the process CPU time (cold disk vs
//! CPU vs priority; `EXPLORER2_INDEX_PRIORITY=normal` for the comparison).

use explorer_index::{Exclusions, Now, Query, SortBy};
use explorer_search::{Config, Origin, Phase, Search, Volume};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn wait_ready(v: &Volume, limit: Duration) -> Duration {
    let t0 = Instant::now();
    let mut last = Instant::now();
    while v.status().phase != Phase::Ready {
        if t0.elapsed() > limit {
            eprintln!("not ready after {limit:?}: {:?}", v.status());
            std::process::exit(2);
        }
        if last.elapsed() > Duration::from_secs(10) {
            last = Instant::now();
            let s = v.status();
            println!(
                "  … {:?}, {} entries, {:.0} s",
                s.phase,
                s.entries,
                t0.elapsed().as_secs_f64()
            );
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    t0.elapsed()
}

fn hits(v: &Volume, text: &str) -> usize {
    let q = Query::parse(
        text,
        Now {
            unix: 0,
            utc_offset: 0,
            offset_at: None,
        },
    )
    .expect("query");
    v.search(&q, None, SortBy::Name, false).ids.len()
}

fn wait_hits(v: &Volume, text: &str, want: usize) -> Option<f64> {
    let t0 = Instant::now();
    while hits(v, text) != want {
        if t0.elapsed() > Duration::from_secs(30) {
            return None;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Some(t0.elapsed().as_secs_f64() * 1000.0)
}

/// User + kernel CPU time of this process in ms.
fn cpu_ms() -> Option<u64> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::FILETIME;
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
        let z = || FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let (mut a, mut b, mut k, mut u) = (z(), z(), z(), z());
        // SAFETY: pseudo handle, four valid out pointers.
        let ok = unsafe { GetProcessTimes(GetCurrentProcess(), &mut a, &mut b, &mut k, &mut u) };
        let t = |f: &FILETIME| (u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime);
        (ok != 0).then(|| (t(&k) + t(&u)) / 10_000)
    }
    #[cfg(not(windows))]
    {
        // fields 14 + 15 of /proc/self/stat, in clock ticks (100 Hz)
        let s = std::fs::read_to_string("/proc/self/stat").ok()?;
        let rest = &s[s.rfind(')')? + 2..];
        let f: Vec<&str> = rest.split(' ').collect();
        let ticks: u64 = f.get(11)?.parse::<u64>().ok()? + f.get(12)?.parse::<u64>().ok()?;
        Some(ticks * 10)
    }
}

fn probe(dir: &std::path::Path, secs: u64, threads: Option<usize>) {
    use explorer_search::walk::{build, Walker};
    use std::sync::atomic::{AtomicBool, Ordering};
    let ex = Exclusions::with_defaults("", cfg!(windows));
    let stop = AtomicBool::new(false);
    let mut w = Walker::new(&ex);
    if let Some(t) = threads {
        w.threads = t;
    }
    w.cancel = Some(&stop);
    let cpu0 = cpu_ms();
    let t0 = Instant::now();
    let (_, rep) = std::thread::scope(|s| {
        s.spawn(|| {
            let end = t0 + Duration::from_secs(secs);
            while Instant::now() < end && !stop.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(50));
            }
            stop.store(true, Ordering::Relaxed);
        });
        let r = build(dir, &w);
        stop.store(true, Ordering::Relaxed);
        r
    });
    let wall = t0.elapsed().as_millis().max(1) as u64;
    let cpu = match (cpu0, cpu_ms()) {
        (Some(a), Some(b)) => format!(
            "{} ms ({:.0} % of one core)",
            b - a,
            (b - a) as f64 * 100.0 / wall as f64
        ),
        _ => "n/a".into(),
    };
    println!(
        "probe {}: {} threads, priority {}, {} entries in {} folders ({} unreadable) in {wall} ms{} → {:.0} entries/s, {:.0} folders/s; CPU {cpu}",
        dir.display(),
        w.threads,
        if cfg!(windows) {
            std::env::var("EXPLORER2_INDEX_PRIORITY").unwrap_or_else(|_| "below normal".into())
        } else {
            "unchanged".into()
        },
        rep.entries,
        rep.dirs,
        rep.errors,
        if rep.cancelled { " (time limit)" } else { " (complete)" },
        rep.entries as f64 * 1000.0 / wall as f64,
        rep.dirs as f64 * 1000.0 / wall as f64,
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    if let Some(dir) = value("--probe") {
        let secs = value("--secs").and_then(|s| s.parse().ok()).unwrap_or(40);
        let threads = value("--threads").and_then(|s| s.parse().ok());
        probe(std::path::Path::new(&dir), secs, threads);
        return;
    }
    let gate = args.iter().any(|a| a == "--gate");
    let flag = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .map(PathBuf::from)
    };
    let home = || std::env::var_os("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from);
    let root = args
        .iter()
        .enumerate()
        .find(|(i, a)| !a.starts_with("--") && (*i == 0 || args[i - 1] != "--live"))
        .map(|(_, a)| PathBuf::from(a))
        .unwrap_or_else(|| {
            if cfg!(windows) {
                PathBuf::from(r"C:\")
            } else {
                home()
            }
        });
    let live_dir = flag("--live").unwrap_or_else(|| {
        if cfg!(windows) {
            std::env::temp_dir()
        } else {
            home()
        }
    });
    let store = std::env::temp_dir().join(format!("explorer2-drive-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&store);
    let ex = Exclusions::with_defaults("", cfg!(windows));
    let cfg = || Config::new(store.clone(), ex.clone());
    let mut failed: Vec<String> = Vec::new();
    println!(
        "root {}, live folder {}",
        root.display(),
        live_dir.display()
    );
    println!("volume tag: {}", explorer_fs::places::volume_tag(&root));

    // 1. first build
    let s = Search::start(std::slice::from_ref(&root), cfg(), |_| {});
    let v = &s.volumes()[0];
    let ready = wait_ready(v, Duration::from_secs(600));
    let st = v.status();
    let rep = st.build.unwrap_or_default();
    let mem = v.read(|ix| ix.memory_bytes()).unwrap_or(0) as f64 / 1048576.0;
    let per_m = rep.ms as f64 * 1_000_000.0 / rep.entries.max(1) as f64 / 1000.0;
    println!(
        "first build: {} entries, {} folders, {} unreadable, walk {} ms, ready after {:.0} ms ({:.0} entries/s → {per_m:.0} s per 1 M), memory {mem:.1} MB, live updates: {}",
        rep.entries,
        rep.dirs,
        rep.errors,
        rep.ms,
        ready.as_secs_f64() * 1000.0,
        rep.entries as f64 * 1000.0 / rep.ms.max(1) as f64,
        if st.live {
            "yes".to_string()
        } else {
            format!("NO ({:?})", st.watch_error)
        },
    );
    if !matches!(st.origin, Some(Origin::Built { .. })) {
        failed.push(format!("expected a fresh build: {:?}", st.origin));
    }
    if per_m > 120.0 {
        failed.push(format!("first build {per_m:.0} s per 1 M entries > 120 s"));
    }
    if let Some(e) = &st.save_error {
        failed.push(format!("save: {e}"));
    }

    // 2. live changes, best of 3 (a busy VM stalls now and then)
    let mut best_add = f64::MAX;
    let mut best_del = f64::MAX;
    if st.live {
        for round in 0..3 {
            let tag = format!("e2live{}x{round}", std::process::id());
            let dir = live_dir.join(&tag);
            let t = Instant::now();
            std::fs::create_dir_all(&dir).expect("live folder");
            for i in 0..100 {
                std::fs::write(dir.join(format!("{tag}_{i}.txt")), b"x").expect("file");
            }
            let made = t.elapsed().as_secs_f64() * 1000.0;
            // 100 files + the folder (its name contains the tag too)
            let add = wait_hits(v, &tag, 101);
            std::fs::remove_dir_all(&dir).expect("remove");
            let del = wait_hits(v, &tag, 0);
            println!(
                "live round {round}: created in {made:.0} ms, visible after {} ms, removal after {} ms",
                add.map_or("—".into(), |a| format!("{a:.0}")),
                del.map_or("—".into(), |d| format!("{d:.0}")),
            );
            best_add = best_add.min(add.unwrap_or(f64::MAX));
            best_del = best_del.min(del.unwrap_or(f64::MAX));
        }
        let ms = |v: f64| {
            if v == f64::MAX {
                "never".to_string()
            } else {
                format!("{v:.0} ms")
            }
        };
        println!(
            "live best: new {}, removed {} (budget 1000 ms); applied changes {}, last full check {:?}",
            ms(best_add),
            ms(best_del),
            v.status().live_changes,
            v.status().refresh
        );
        if best_add >= 1000.0 || best_del >= 1000.0 {
            failed.push(format!(
                "live change {}/{} ≥ 1000 ms",
                ms(best_add),
                ms(best_del)
            ));
        }
    } else {
        failed.push("no live updates".into());
    }
    let t = Instant::now();
    s.shutdown();
    println!(
        "shutdown (incl. save) {:.0} ms",
        t.elapsed().as_secs_f64() * 1000.0
    );
    let file_mb = std::fs::read_dir(&store)
        .map(|d| {
            d.flatten()
                .filter_map(|e| e.metadata().ok())
                .map(|m| m.len())
                .sum::<u64>()
        })
        .unwrap_or(0) as f64
        / 1048576.0;

    // 3. next start: load + compare
    let s = Search::start(std::slice::from_ref(&root), cfg(), |_| {});
    let v = &s.volumes()[0];
    let ready = wait_ready(v, Duration::from_secs(600));
    let st = v.status();
    let load_ms = match st.origin {
        Some(Origin::Loaded { ms }) => ms,
        ref o => {
            failed.push(format!("saved index not used: {o:?}"));
            0
        }
    };
    let r = st.refresh.unwrap_or_default();
    println!(
        "next start: file {file_mb:.1} MB, loaded (searchable) after {load_ms} ms, compared with the disk after {:.0} ms: {} folders checked, {} read again, +{} −{} ~{}, {} errors",
        ready.as_secs_f64() * 1000.0,
        r.dirs_checked,
        r.dirs_read,
        r.added,
        r.removed,
        r.updated,
        r.errors,
    );
    if load_ms >= 1000 {
        failed.push(format!("load {load_ms} ms ≥ 1000 ms"));
    }
    drop(s);
    let _ = std::fs::remove_dir_all(&store);
    if gate && !failed.is_empty() {
        eprintln!("FAILED: {}", failed.join("; "));
        std::process::exit(1);
    }
    if !failed.is_empty() {
        println!("(would fail with --gate: {})", failed.join("; "));
    }
}
