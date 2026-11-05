//! M8.3c: the R-001 journal under random damage. The reset must never be
//! misled: a cut-off or altered journal is never taken as intact, parsing
//! never panics, and an intact journal reads back exactly.

use explorer_integrate::features::ALL;
use explorer_integrate::{apply, Journal, JournalPaths, MemRegistry};
use std::path::Path;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// A real journal: all features switched on against an empty registry.
fn real_journal() -> String {
    let dir = tempfile::tempdir().unwrap();
    let paths = JournalPaths::in_dir(dir.path());
    let mut reg = MemRegistry::new();
    let exe = Path::new(r"C:\Program Files\Explorer2\explorer2.exe");
    for f in ALL {
        apply(&mut reg, &paths, &f.changes(exe)).unwrap();
    }
    std::fs::read_to_string(&paths.primary).unwrap()
}

#[test]
fn intact_journal_reads_back() {
    let text = real_journal();
    let j = Journal::parse(&text).expect("intact");
    assert!(!j.entries.is_empty());
    assert_eq!(Journal::parse(&j.serialize()), Ok(j));
}

#[test]
fn every_cut_off_journal_is_detected() {
    let text = real_journal();
    // every char boundary: a write that stopped there (only the final line
    // break may go: content and checksum are complete without it)
    for (i, _) in text.char_indices().skip(1) {
        let cut = &text[..i];
        if cut == text.trim_end_matches('\n') {
            assert!(Journal::parse(cut).is_ok());
            continue;
        }
        assert!(
            Journal::parse(cut).is_err(),
            "cut at {i} taken as intact: {cut:?}"
        );
    }
}

#[test]
fn damaged_journals_never_pass_and_never_panic() {
    let text = real_journal();
    let chars: Vec<char> = text.chars().collect();
    let seed: u64 = std::env::var("FUZZ_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut r = Rng((0x5eed ^ seed.wrapping_mul(0x9e37_79b9_7f4a_7c15)).max(1));
    let rounds = std::env::var("FUZZ_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);
    for _ in 0..rounds {
        let mut c = chars.clone();
        let i = r.below(c.len());
        let flip = match r.below(4) {
            0 => {
                c.remove(i);
                true
            }
            1 => {
                let old = c[i];
                c[i] = char::from_u32(r.next() as u32 % 0x7f).unwrap_or('x');
                c[i] != old
            }
            2 => {
                c.insert(i, ['\t', '\n', '0', 'f', ' '][r.below(5)]);
                true
            }
            _ => {
                let j = r.below(c.len());
                c.swap(i, j);
                c[i] != c[j]
            }
        };
        let s: String = c.into_iter().collect();
        let same = s.trim_end_matches('\n') == text.trim_end_matches('\n');
        match Journal::parse(&s) {
            // a changed byte must break the checksum
            Ok(_) if flip && !same => {
                let at = s
                    .chars()
                    .zip(text.chars())
                    .position(|(a, b)| a != b)
                    .unwrap_or(s.len().min(text.len()));
                let ctx = |t: &str| {
                    t.chars()
                        .skip(at.saturating_sub(20))
                        .take(40)
                        .collect::<String>()
                };
                panic!(
                    "damaged journal taken as intact at char {at}: {:?} vs original {:?}",
                    ctx(&s),
                    ctx(&text)
                )
            }
            Ok(_) | Err(_) => {}
        }
    }
    // nothing may follow the checksum line unnoticed
    for extra in ["key\tHKCU\t41\n", "# x\n", "v1\n", "end\t0\n"] {
        assert!(
            Journal::parse(&format!("{text}{extra}")).is_err(),
            "{extra:?} after the end line"
        );
    }
    assert!(Journal::parse(&format!("{text}\n\n")).is_ok());
    // pure garbage
    for _ in 0..rounds {
        let n = r.below(300);
        let s: String = (0..n)
            .filter_map(|_| char::from_u32(r.next() as u32 % 0x11_0000))
            .collect();
        let _ = Journal::parse(&s);
        let _ = Journal::parse(&format!("{}{s}", &text[..text.len() / 2]));
    }
}
