//! Start guard + crash report helpers (M8.3), pure and testable.
//!
//! The app writes a small record (`start-guard.txt`) before the window exists
//! and deletes it once it has run stably for a few seconds (or on a clean exit).
//! A record left over from a process that is gone means that start died –
//! native crash, panic, driver hang killed by the user. After
//! [`SAFE_AFTER`] such starts in a row the next one runs in safe mode
//! (software renderer, no restored session), so a bad GPU driver or a session
//! entry that crashes the app can never lock the user out.

/// Failed starts in a row that switch the next start to safe mode.
pub const SAFE_AFTER: u32 = 2;

/// Seconds a start must survive to count as stable.
pub const STABLE_AFTER_SECS: u64 = 5;

/// Crash reports kept in the crash folder (oldest are deleted).
pub const MAX_REPORTS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartRecord {
    pub pid: u32,
    /// Failed starts before the one that wrote this record.
    pub failed: u32,
}

impl StartRecord {
    pub fn parse(text: &str) -> Option<StartRecord> {
        let (mut pid, mut failed) = (None, 0);
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            match k.trim() {
                "pid" => pid = v.trim().parse().ok(),
                "failed" => failed = v.trim().parse().unwrap_or(0),
                _ => {}
            }
        }
        Some(StartRecord {
            pid: pid?,
            failed: failed.min(1000),
        })
    }

    pub fn serialize(&self) -> String {
        format!("pid={}\nfailed={}\n", self.pid, self.failed)
    }
}

/// What this start does with the guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Another instance is starting right now: leave its record alone.
    Skip,
    /// Guard this start; `failed` starts died before it.
    Guard { failed: u32 },
}

impl Verdict {
    pub fn safe_mode(self) -> bool {
        matches!(self, Verdict::Guard { failed } if failed >= SAFE_AFTER)
    }
}

/// `prev` = record found at start, `prev_alive` = its process still runs.
pub fn verdict(prev: Option<StartRecord>, prev_alive: bool) -> Verdict {
    match prev {
        None => Verdict::Guard { failed: 0 },
        Some(_) if prev_alive => Verdict::Skip,
        Some(r) => Verdict::Guard {
            failed: r.failed.saturating_add(1),
        },
    }
}

/// `YYYYMMDD-HHMMSS` (UTC) for file names, from Unix seconds.
pub fn utc_stamp(unix_secs: u64) -> String {
    let (y, mo, d, h, mi, s) = utc_parts(unix_secs);
    format!("{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}")
}

/// `YYYY-MM-DD HH:MM:SS UTC` for report text.
pub fn utc_text(unix_secs: u64) -> String {
    let (y, mo, d, h, mi, s) = utc_parts(unix_secs);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02} UTC")
}

fn utc_parts(unix_secs: u64) -> (i64, u32, u32, u64, u64, u64) {
    let days = (unix_secs / 86_400) as i64;
    let rem = unix_secs % 86_400;
    // civil-from-days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d, rem / 3600, rem % 3600 / 60, rem % 60)
}

/// Report file name for a crash (`crash-20261105-130000-1234.txt`).
pub fn report_name(unix_secs: u64, pid: u32) -> String {
    format!("crash-{}-{pid}.txt", utc_stamp(unix_secs))
}

/// A report nobody has been told about yet (`.seen.txt` = shown once).
pub fn is_new_report(file_name: &str) -> bool {
    file_name.starts_with("crash-")
        && file_name.ends_with(".txt")
        && !file_name.ends_with(".seen.txt")
}

pub fn seen_name(file_name: &str) -> String {
    match file_name.strip_suffix(".txt") {
        Some(stem) if !stem.ends_with(".seen") => format!("{stem}.seen.txt"),
        _ => file_name.to_string(),
    }
}

/// Reports to delete so that at most [`MAX_REPORTS`] remain (names sort by time).
pub fn surplus_reports(mut names: Vec<String>) -> Vec<String> {
    names.retain(|n| n.starts_with("crash-") && n.ends_with(".txt"));
    names.sort();
    let n = names.len().saturating_sub(MAX_REPORTS);
    names.truncate(n);
    names
}

/// The notice shown after a crash / in safe mode (German UI text).
pub fn notice_de(new_reports: usize, safe_mode: bool, failed: u32) -> Option<String> {
    let crash = match new_reports {
        0 => None,
        1 => Some("Explorer 2.0 wurde beim letzten Mal unerwartet beendet.".to_string()),
        n => Some(format!("Explorer 2.0 wurde {n}-mal unerwartet beendet.")),
    };
    let safe = safe_mode.then(|| {
        format!(
            "Sicherer Start: die letzten {failed} Starts sind abgebrochen – daher Software-Darstellung \
             und keine wiederhergestellte Sitzung. Bleibt es stabil, startet Explorer 2.0 \
             beim nächsten Mal wieder normal."
        )
    });
    match (crash, safe) {
        (None, None) => None,
        (Some(c), None) => Some(format!("{c} Ein Bericht liegt im Ordner „crash“.")),
        (None, Some(s)) => Some(s),
        (Some(c), Some(s)) => Some(format!("{c} {s}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_roundtrip_and_tolerance() {
        let r = StartRecord { pid: 42, failed: 3 };
        assert_eq!(StartRecord::parse(&r.serialize()), Some(r));
        assert_eq!(StartRecord::parse(""), None);
        assert_eq!(StartRecord::parse("failed=2"), None);
        assert_eq!(
            StartRecord::parse("pid=7\nfailed=x\n\0\u{fffd}"),
            Some(StartRecord { pid: 7, failed: 0 })
        );
        assert_eq!(
            StartRecord::parse("pid=7\nfailed=4000000000").map(|r| r.failed),
            Some(1000)
        );
    }

    #[test]
    fn verdicts() {
        assert_eq!(verdict(None, false), Verdict::Guard { failed: 0 });
        let r = StartRecord { pid: 1, failed: 0 };
        assert_eq!(verdict(Some(r), true), Verdict::Skip);
        assert_eq!(verdict(Some(r), false), Verdict::Guard { failed: 1 });
        assert!(!verdict(Some(r), false).safe_mode());
        let r = StartRecord { pid: 1, failed: 1 };
        assert!(verdict(Some(r), false).safe_mode());
        assert!(!Verdict::Skip.safe_mode());
        let r = StartRecord {
            pid: 1,
            failed: u32::MAX,
        };
        assert!(verdict(Some(r), false).safe_mode());
    }

    #[test]
    fn utc_formatting() {
        assert_eq!(utc_stamp(0), "19700101-000000");
        assert_eq!(utc_text(1_762_347_600), "2025-11-05 13:00:00 UTC");
        // leap day
        assert_eq!(utc_stamp(951_782_400), "20000229-000000");
        assert_eq!(utc_stamp(1_798_761_599), "20261231-235959");
        assert_eq!(report_name(0, 12), "crash-19700101-000000-12.txt");
    }

    #[test]
    fn report_names() {
        assert!(is_new_report("crash-20261105-130000-1.txt"));
        assert!(!is_new_report("crash-20261105-130000-1.seen.txt"));
        assert!(!is_new_report("start-guard.txt"));
        assert_eq!(seen_name("crash-a.txt"), "crash-a.seen.txt");
        assert_eq!(seen_name("crash-a.seen.txt"), "crash-a.seen.txt");
        let names: Vec<String> = (0..13)
            .map(|i| format!("crash-2026110{}-0000{i:02}-1.txt", i % 3))
            .chain(["other.txt".to_string()])
            .collect();
        let gone = surplus_reports(names.clone());
        assert_eq!(gone.len(), 3);
        // the oldest go
        let mut sorted: Vec<_> = names
            .into_iter()
            .filter(|n| n.starts_with("crash"))
            .collect();
        sorted.sort();
        assert_eq!(gone, sorted[..3].to_vec());
    }

    #[test]
    fn notices() {
        assert_eq!(notice_de(0, false, 0), None);
        assert!(notice_de(1, false, 0)
            .unwrap()
            .contains("unerwartet beendet"));
        assert!(notice_de(3, false, 0).unwrap().contains("3-mal"));
        let s = notice_de(0, true, 2).unwrap();
        assert!(s.contains("Sicherer Start") && s.contains("2 Starts"));
        let both = notice_de(1, true, 2).unwrap();
        assert!(both.contains("unerwartet") && both.contains("Sicherer Start"));
    }
}
