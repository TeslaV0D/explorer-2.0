//! M8.3c: every parser and text helper fed with random and mutated input.
//! Properties: never panics; what was parsed serializes and parses back to
//! the same (settings/session/views survive a save + load unchanged).
//! Deterministic (fixed seeds) → a failure is reproducible.

use explorer_core::guard::StartRecord;
use explorer_core::instance::Request;
use explorer_core::session::Session;
use explorer_core::settings::Settings;
use explorer_core::views::FolderViews;

/// `FUZZ_ROUNDS` / `FUZZ_SEED` for a longer local run (CI: the defaults).
fn rounds() -> usize {
    std::env::var("FUZZ_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4000)
}

fn seed(base: u64) -> u64 {
    let extra: u64 = std::env::var("FUZZ_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    (base ^ extra.wrapping_mul(0x9e37_79b9_7f4a_7c15)).max(1)
}

/// xorshift64*: small, deterministic, good enough for test input.
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

/// Pieces that hit the parsers' structure (keys, separators, odd characters).
const PIECES: &[&str] = &[
    "=",
    " = ",
    "\n",
    "\r\n",
    "[pane]",
    "#",
    " ",
    "\t",
    "\\",
    "/",
    ":",
    "\"",
    "%",
    "~",
    ".",
    "..",
    "\0",
    "ü",
    "€",
    "😀",
    "e\u{301}",
    "\u{feff}",
    "true",
    "false",
    "1",
    "0",
    "-1",
    "99999999999",
    "tab",
    "pane",
    "split",
    "active_tab",
    "active_pane",
    "hardware_acceleration",
    "show_hidden",
    "sort_key",
    "name",
    "size",
    "view",
    "large",
    "theme",
    "dark",
    "recent_commands",
    ",",
    "single_instance",
    "gpu_probe",
    "unusable",
    "pid",
    "failed",
    "EXPLORER2/1",
    " open ",
    "open",
    "C:",
    "C:\\",
    "\\\\nas\\share",
    "%USERPROFILE%",
    "%",
    "~/",
    "shell:Downloads",
    "::papierkorb",
    "::bibliothek:",
    "a.txt",
    ".hidden",
    "x.tar.gz",
    "\u{202e}",
    "|",
    "*",
    "?",
];

fn random_text(r: &mut Rng) -> String {
    let n = r.below(40);
    let mut s = String::new();
    for _ in 0..n {
        match r.below(4) {
            0 => {
                // raw random char (any plane, surrogates excluded by from_u32)
                if let Some(c) = char::from_u32(r.next() as u32 % 0x11_0000) {
                    s.push(c);
                }
            }
            _ => s.push_str(PIECES[r.below(PIECES.len())]),
        }
    }
    s
}

/// A valid text with random damage: cut at a char boundary, chars replaced,
/// lines swapped or duplicated.
fn mutate(r: &mut Rng, valid: &str) -> String {
    let mut chars: Vec<char> = valid.chars().collect();
    for _ in 0..1 + r.below(4) {
        if chars.is_empty() {
            break;
        }
        let i = r.below(chars.len());
        match r.below(5) {
            0 => chars.truncate(i),
            1 => chars[i] = PIECES[r.below(PIECES.len())].chars().next().unwrap_or('x'),
            2 => {
                chars.remove(i);
            }
            3 => {
                let piece: Vec<char> = PIECES[r.below(PIECES.len())].chars().collect();
                chars.splice(i..i, piece);
            }
            _ => {
                let j = r.below(chars.len());
                chars.swap(i, j);
            }
        }
    }
    chars.into_iter().collect()
}

fn inputs(base: u64, valid: &[String]) -> Vec<String> {
    let mut r = Rng(seed(base));
    (0..rounds())
        .map(|i| {
            if i % 2 == 0 || valid.is_empty() {
                random_text(&mut r)
            } else {
                let k = r.below(valid.len());
                mutate(&mut r, &valid[k])
            }
        })
        .collect()
}

#[test]
fn settings_survive_any_file() {
    let valid = vec![
        Settings::default().serialize(),
        Settings {
            show_hidden: true,
            recent_commands: vec!["paste".into(), "inspector".into()],
            single_instance: false,
            ..Default::default()
        }
        .serialize(),
    ];
    for text in inputs(1, &valid) {
        let s = Settings::parse(&text);
        assert_eq!(Settings::parse(&s.serialize()), s, "input: {text:?}");
    }
}

#[test]
fn session_survives_any_file() {
    let valid = vec![
        "split = true\nactive_pane = 1\n[pane]\nactive_tab = 1\ntab = C:\\a\ntab = C:\\b\n[pane]\ntab = /home/x\n".to_string(),
    ];
    for text in inputs(2, &valid) {
        if let Some(s) = Session::parse(&text) {
            assert!(!s.panes.is_empty() && s.panes.len() <= explorer_core::session::MAX_PANES);
            assert_eq!(Session::parse(&s.serialize()), Some(s), "input: {text:?}");
        }
    }
}

#[test]
fn folder_views_survive_any_file() {
    let mut fv = FolderViews::default();
    fv.set(
        r"C:\Users\Boss\Bilder",
        explorer_core::view::ViewMode::Large,
    );
    let valid = vec![fv.serialize()];
    for ci in [true, false] {
        for text in inputs(3 + u64::from(ci), &valid) {
            let v = FolderViews::parse(&text, ci);
            assert_eq!(FolderViews::parse(&v.serialize(), ci), v, "input: {text:?}");
        }
    }
}

#[test]
fn start_guard_and_instance_protocol_never_panic() {
    let valid = vec![
        StartRecord { pid: 42, failed: 1 }.serialize(),
        Request::Open(Some(r"C:\x y".into())).encode(),
        Request::Open(None).encode(),
    ];
    for text in inputs(5, &valid) {
        if let Some(r) = StartRecord::parse(&text) {
            assert_eq!(StartRecord::parse(&r.serialize()), Some(r));
        }
        if let Some(req) = Request::decode(&text) {
            assert_eq!(Request::decode(&req.encode()), Some(req), "input: {text:?}");
        }
    }
}

#[test]
fn text_helpers_never_panic() {
    use explorer_core::{address, breadcrumb, commands, display, inspect, theme};
    let mut r = Rng(seed(6));
    for text in inputs(7, &[]) {
        let _ = address::launch_arg(&text);
        let _ = address::expand_env(&text, |k| (k.len() % 2 == 0).then(|| text.clone()));
        let _ = address::expand_home(&text, Some("/home/boss"));
        let _ = address::shell_folder(&text);
        let crumbs = breadcrumb::crumbs(&text);
        let _ = breadcrumb::fit(crumbs, r.below(80));
        let _ = commands::fuzzy_score(&text, "Ordner im offenen Fenster öffnen");
        let _ = commands::fuzzy_score("ordner", &text);
        let _ = commands::rank(&text, std::slice::from_ref(&text), commands::ALL);
        for is_dir in [true, false] {
            let shown = display::display_name(&text, is_dir, false, |_| true);
            let hidden = display::hidden_ext(&text, shown);
            let _ = display::rename_result(&text, hidden);
        }
        let _ = theme::parse_hex(&text);
        // search history file: tolerant, stable after one round trip
        let h = explorer_core::searches::parse(&text);
        assert!(h.len() <= explorer_core::searches::KEEP);
        assert_eq!(
            explorer_core::searches::parse(&explorer_core::searches::serialize(&h)),
            h
        );
        // search highlight: never panics, the three parts are the name again
        let other = random_text(&mut r);
        for (name, query) in [(&text, &other), (&other, &text), (&text, &text)] {
            let p = explorer_core::highlight::pieces(query);
            if let Some((a, b, c)) = explorer_core::highlight::split(name, &p) {
                assert_eq!(format!("{a}{b}{c}"), *name);
                assert!(!b.is_empty());
            }
        }
        let _ = explorer_core::guard::notice_de(r.below(5), r.below(2) == 0, r.below(9) as u32);
        let mut bytes = text.into_bytes();
        bytes.extend((0..r.below(64)).map(|_| r.next() as u8));
        let _ = inspect::image_size(&bytes);
        let _ = inspect::text_preview(&bytes, r.below(30), r.below(200));
    }
}
