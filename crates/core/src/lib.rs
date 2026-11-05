//! Platform-neutral model: entries, sorting, natural ordering.
//! No I/O and no OS-specific code here, so it is unit-testable everywhere.

pub mod address;
pub mod breadcrumb;
pub mod commands;
pub mod display;
pub mod guard;
pub mod history;
pub mod inspect;
pub mod instance;
pub mod places;
pub mod renderer;
pub mod selection;
pub mod session;
pub mod settings;
pub mod theme;
pub mod typeahead;
pub mod view;
pub mod views;

use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Dir,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub kind: EntryKind,
    pub size: u64,
    /// Seconds since Unix epoch, if known.
    pub modified: Option<i64>,
    /// Hidden attribute (Windows) / dot-file (Unix).
    pub hidden: bool,
    /// System attribute (Windows only).
    pub system: bool,
    pub readonly: bool,
    /// Symlink or junction (navigable if `kind == Dir`).
    pub is_link: bool,
}

impl Entry {
    pub fn is_dir(&self) -> bool {
        self.kind == EntryKind::Dir
    }
}

/// Explorer visibility rules ("Ordneroptionen"). Default = Explorer default
/// for hidden/protected files (both hidden); extensions are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewFilter {
    /// "Ausgeblendete Elemente anzeigen" (default off).
    pub show_hidden: bool,
    /// Inverse of "Geschützte Systemdateien ausblenden" (default: hidden).
    pub show_protected_os: bool,
    /// Inverse of "Erweiterungen bei bekannten Dateitypen ausblenden"
    /// (default: shown – a file manager for people who care about names).
    pub show_extensions: bool,
}

impl Default for ViewFilter {
    fn default() -> Self {
        ViewFilter {
            show_hidden: false,
            show_protected_os: false,
            show_extensions: true,
        }
    }
}

impl ViewFilter {
    /// Protected OS files = hidden + system. System-only files stay visible, like in Explorer.
    pub fn is_visible(&self, e: &Entry) -> bool {
        if e.hidden && e.system && !self.show_protected_os {
            return false;
        }
        !e.hidden || self.show_hidden
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Size,
    Modified,
    /// File type (extension); folders are all "Dateiordner".
    Type,
}

impl SortKey {
    pub fn as_str(self) -> &'static str {
        match self {
            SortKey::Name => "name",
            SortKey::Size => "size",
            SortKey::Modified => "modified",
            SortKey::Type => "type",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "name" => Some(SortKey::Name),
            "size" => Some(SortKey::Size),
            "modified" => Some(SortKey::Modified),
            "type" => Some(SortKey::Type),
            _ => None,
        }
    }
}

/// Extension without dot ("" if none). Leading-dot names (".gitignore") have no extension,
/// like in Explorer.
pub fn extension(name: &str) -> &str {
    match name.rfind('.') {
        Some(i) if i > 0 && i + 1 < name.len() => &name[i + 1..],
        _ => "",
    }
}

/// Explorer-like type label until real shell type names arrive (M5).
pub fn type_label(e: &Entry) -> String {
    if e.is_dir() {
        return "Dateiordner".to_string();
    }
    match extension(&e.name) {
        "" => "Datei".to_string(),
        ext => format!("{}-Datei", ext.to_uppercase()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortSpec {
    pub key: SortKey,
    pub descending: bool,
    /// Directories always before files (Explorer default).
    pub dirs_first: bool,
}

impl Default for SortSpec {
    fn default() -> Self {
        Self {
            key: SortKey::Name,
            descending: false,
            dirs_first: true,
        }
    }
}

/// Natural, case-insensitive comparison: "file2" < "file10". Allocation-free,
/// with an ASCII fast path (hot loop when sorting 100k+ entries).
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (ab, bb) = (a.as_bytes(), b.as_bytes());
    let (mut i, mut j) = (0, 0);
    while i < ab.len() && j < bb.len() {
        let (x, y) = (ab[i], bb[j]);
        if x.is_ascii_digit() && y.is_ascii_digit() {
            let si = i;
            while i < ab.len() && ab[i].is_ascii_digit() {
                i += 1;
            }
            let sj = j;
            while j < bb.len() && bb[j].is_ascii_digit() {
                j += 1;
            }
            let (na, nb) = (trim_zeros(&ab[si..i]), trim_zeros(&bb[sj..j]));
            let ord = na.len().cmp(&nb.len()).then_with(|| na.cmp(nb));
            if ord != Ordering::Equal {
                return ord;
            }
        } else if x.is_ascii() && y.is_ascii() {
            let ord = x.to_ascii_lowercase().cmp(&y.to_ascii_lowercase());
            if ord != Ordering::Equal {
                return ord;
            }
            i += 1;
            j += 1;
        } else {
            // i and j are always on char boundaries (we only skip whole chars / ASCII bytes)
            let ca = a[i..].chars().next().unwrap_or_default();
            let cb = b[j..].chars().next().unwrap_or_default();
            let ord = ca.to_lowercase().cmp(cb.to_lowercase());
            if ord != Ordering::Equal {
                return ord;
            }
            i += ca.len_utf8();
            j += cb.len_utf8();
        }
    }
    (ab.len() - i).cmp(&(bb.len() - j))
}

fn trim_zeros(d: &[u8]) -> &[u8] {
    let n = d.iter().take_while(|&&c| c == b'0').count();
    &d[n..]
}

pub fn sort_entries(entries: &mut [Entry], spec: SortSpec) {
    entries.sort_unstable_by(|a, b| {
        if spec.dirs_first && a.is_dir() != b.is_dir() {
            return if a.is_dir() {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        let ord = match spec.key {
            SortKey::Name => natural_cmp(&a.name, &b.name),
            SortKey::Size => a
                .size
                .cmp(&b.size)
                .then_with(|| natural_cmp(&a.name, &b.name)),
            SortKey::Modified => a
                .modified
                .cmp(&b.modified)
                .then_with(|| natural_cmp(&a.name, &b.name)),
            SortKey::Type => {
                // Files without extension ("Datei") sort before typed files, like Explorer.
                let (ea, eb) = (extension(&a.name), extension(&b.name));
                (!ea.is_empty())
                    .cmp(&!eb.is_empty())
                    .then_with(|| natural_cmp(ea, eb))
                    .then_with(|| natural_cmp(&a.name, &b.name))
            }
        };
        let ord = if spec.descending { ord.reverse() } else { ord };
        // Deterministic order for "equal" names (a01 vs a1, File vs file) -> no flicker on reload.
        ord.then_with(|| a.name.cmp(&b.name))
    });
}

/// Sorts like [`sort_entries`], but `SortKey::Type` compares the given type
/// names (the shell's "Textdokument", "JPEG-Bild" …, as shown in the column)
/// instead of raw extensions – that is how Explorer orders the "Typ" column.
/// Names are computed once per entry (decorate-sort-undecorate).
pub fn sort_entries_with_type_names(
    entries: &mut Vec<Entry>,
    spec: SortSpec,
    mut type_name: impl FnMut(&Entry) -> String,
) {
    if spec.key != SortKey::Type {
        sort_entries(entries, spec);
        return;
    }
    let mut keyed: Vec<(String, Entry)> = entries.drain(..).map(|e| (type_name(&e), e)).collect();
    keyed.sort_unstable_by(|(ta, a), (tb, b)| {
        if spec.dirs_first && a.is_dir() != b.is_dir() {
            return if a.is_dir() {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        let ord = natural_cmp(ta, tb).then_with(|| natural_cmp(&a.name, &b.name));
        let ord = if spec.descending { ord.reverse() } else { ord };
        ord.then_with(|| a.name.cmp(&b.name))
    });
    entries.extend(keyed.into_iter().map(|(_, e)| e));
}

/// Human-readable size, Explorer-style in the list ("1 KB", "3,4 MB" uses '.').
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if v >= 100.0 || (v - v.round()).abs() < 0.05 {
        format!("{:.0} {}", v, UNITS[i])
    } else {
        format!("{:.1} {}", v, UNITS[i])
    }
}

/// Drive usage line like Explorer: "120 GB frei von 512 GB".
pub fn format_usage(free: u64, total: u64) -> String {
    format!("{} frei von {}", format_size(free), format_size(total))
}

/// Thousands separator, German style: 100000 -> "100.000".
pub fn format_count(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

/// "YYYY-MM-DD HH:MM" in UTC from Unix seconds (local time zone handling comes with the Windows adapter).
pub fn format_datetime(unix_secs: i64) -> String {
    let days = unix_secs.div_euclid(86_400);
    let rem = unix_secs.rem_euclid(86_400);
    // Howard Hinnant's civil_from_days
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}",
        rem / 3600,
        rem % 3600 / 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(name: &str, kind: EntryKind, size: u64) -> Entry {
        Entry {
            name: name.into(),
            kind,
            size,
            modified: None,
            hidden: false,
            system: false,
            readonly: false,
            is_link: false,
        }
    }

    #[test]
    fn visibility_rules() {
        let mut f = e("x", EntryKind::File, 0);
        let def = ViewFilter::default();
        assert!(def.is_visible(&f));
        f.system = true; // system-only stays visible
        assert!(def.is_visible(&f));
        f.hidden = true; // hidden+system = protected OS file
        assert!(!def.is_visible(&f));
        let show_hidden = ViewFilter {
            show_hidden: true,
            show_protected_os: false,
            ..Default::default()
        };
        assert!(!show_hidden.is_visible(&f));
        assert!(ViewFilter {
            show_hidden: true,
            show_protected_os: true,
            ..Default::default()
        }
        .is_visible(&f));
        f.system = false; // hidden only
        assert!(!def.is_visible(&f));
        assert!(show_hidden.is_visible(&f));
    }

    #[test]
    fn natural_order() {
        assert_eq!(natural_cmp("file2", "file10"), Ordering::Less);
        assert_eq!(natural_cmp("File", "file"), Ordering::Equal);
        assert_eq!(natural_cmp("a01", "a1"), Ordering::Equal);
        assert_eq!(natural_cmp("a", "ab"), Ordering::Less);
        assert_eq!(natural_cmp("ab", "a"), Ordering::Greater);
        assert_eq!(natural_cmp("", ""), Ordering::Equal);
        assert_eq!(natural_cmp("x9", "x10"), Ordering::Less);
        assert_eq!(natural_cmp("x 9", "x10"), Ordering::Less); // ' ' < '1'
        assert_eq!(natural_cmp("Ärger", "ärger"), Ordering::Equal);
        assert_eq!(natural_cmp("äb", "äc"), Ordering::Less);
        assert_eq!(natural_cmp("a🚀2", "a🚀10"), Ordering::Less);
        assert_eq!(natural_cmp("0", "00"), Ordering::Equal);
        assert_eq!(natural_cmp("v1.10", "v1.9"), Ordering::Greater);
    }

    #[test]
    fn dirs_first_and_desc() {
        let mut v = vec![
            e("b.txt", EntryKind::File, 1),
            e("zdir", EntryKind::Dir, 0),
            e("a.txt", EntryKind::File, 2),
        ];
        sort_entries(&mut v, SortSpec::default());
        assert_eq!(v[0].name, "zdir");
        assert_eq!(v[1].name, "a.txt");
        sort_entries(
            &mut v,
            SortSpec {
                key: SortKey::Size,
                descending: true,
                dirs_first: true,
            },
        );
        assert_eq!(v[1].name, "a.txt");
    }

    #[test]
    fn type_labels_and_sort() {
        assert_eq!(extension("a.TXT"), "TXT");
        assert_eq!(extension(".gitignore"), "");
        assert_eq!(extension("archive.tar.gz"), "gz");
        assert_eq!(extension("noext"), "");
        assert_eq!(extension("trailingdot."), "");
        assert_eq!(type_label(&e("x.txt", EntryKind::File, 0)), "TXT-Datei");
        assert_eq!(type_label(&e("Makefile", EntryKind::File, 0)), "Datei");
        assert_eq!(type_label(&e("src", EntryKind::Dir, 0)), "Dateiordner");
        let mut v = vec![
            e("b.txt", EntryKind::File, 0),
            e("a.png", EntryKind::File, 0),
            e("README", EntryKind::File, 0),
            e("c.TXT", EntryKind::File, 0),
            e("dir", EntryKind::Dir, 0),
        ];
        sort_entries(
            &mut v,
            SortSpec {
                key: SortKey::Type,
                descending: false,
                dirs_first: true,
            },
        );
        let names: Vec<_> = v.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["dir", "README", "a.png", "b.txt", "c.TXT"]);
        for k in [
            SortKey::Name,
            SortKey::Size,
            SortKey::Modified,
            SortKey::Type,
        ] {
            assert_eq!(SortKey::parse(k.as_str()), Some(k));
        }
    }

    #[test]
    fn equal_names_sort_deterministically() {
        let mut v = vec![
            e("a1", EntryKind::File, 0),
            e("a01", EntryKind::File, 0),
            e("A1", EntryKind::File, 0),
        ];
        let mut w = v.clone();
        w.reverse();
        sort_entries(&mut v, SortSpec::default());
        sort_entries(&mut w, SortSpec::default());
        assert_eq!(v, w);
    }

    #[test]
    fn size_formatting() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(5 * 1024 * 1024), "5 MB");
        assert!(format_size(u64::MAX).ends_with("TB"));
    }

    #[test]
    fn usage_formatting() {
        assert_eq!(
            format_usage(120 * 1024 * 1024 * 1024, 512 * 1024 * 1024 * 1024),
            "120 GB frei von 512 GB"
        );
    }

    #[test]
    fn count_formatting() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1000), "1.000");
        assert_eq!(format_count(100_000), "100.000");
        assert_eq!(format_count(1_234_567), "1.234.567");
    }

    #[test]
    fn datetime_formatting() {
        assert_eq!(format_datetime(0), "1970-01-01 00:00");
        assert_eq!(
            format_datetime(951_782_400 + 3600 * 13 + 60 * 5),
            "2000-02-29 13:05"
        );
        assert_eq!(format_datetime(-1), "1969-12-31 23:59");
        assert_eq!(format_datetime(1_793_686_008), "2026-11-03 06:06");
    }

    #[test]
    fn type_sort_uses_shown_names() {
        let mut v = vec![
            e("b.jpg", EntryKind::File, 0),
            e("a.txt", EntryKind::File, 0),
            e("Ordner", EntryKind::Dir, 0),
            e("c.jpeg", EntryKind::File, 0),
        ];
        let names = |x: &Entry| match extension(&x.name) {
            "jpg" | "jpeg" => "JPEG-Bild".to_string(),
            "txt" => "Textdokument".to_string(),
            _ => "Dateiordner".to_string(),
        };
        let spec = SortSpec {
            key: SortKey::Type,
            descending: false,
            dirs_first: true,
        };
        sort_entries_with_type_names(&mut v, spec, names);
        let order: Vec<_> = v.iter().map(|x| x.name.as_str()).collect();
        // jpg + jpeg share one type name → grouped, then by name; folders first
        assert_eq!(order, ["Ordner", "b.jpg", "c.jpeg", "a.txt"]);
        let spec = SortSpec {
            descending: true,
            ..spec
        };
        sort_entries_with_type_names(&mut v, spec, names);
        let order: Vec<_> = v.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(order, ["Ordner", "a.txt", "c.jpeg", "b.jpg"]);
        // other keys behave exactly like sort_entries
        let spec = SortSpec {
            key: SortKey::Name,
            descending: false,
            dirs_first: true,
        };
        sort_entries_with_type_names(&mut v, spec, |_| unreachable!());
        assert_eq!(v[1].name, "a.txt");
    }
}
