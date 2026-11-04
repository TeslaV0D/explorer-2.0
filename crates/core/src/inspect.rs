//! Inspector helpers (pure → tested on every OS): picture size from the file
//! header, a text preview, attribute labels and selection totals.

use crate::Entry;

/// Width × height from the first bytes of a PNG, GIF, BMP, JPEG or WebP
/// file (`None` for anything else or a broken header). 64 KB are enough
/// for every JPEG with sane metadata.
pub fn image_size(b: &[u8]) -> Option<(u32, u32)> {
    let be16 =
        |i: usize| -> Option<u32> { Some(u16::from_be_bytes([*b.get(i)?, *b.get(i + 1)?]) as u32) };
    let le16 =
        |i: usize| -> Option<u32> { Some(u16::from_le_bytes([*b.get(i)?, *b.get(i + 1)?]) as u32) };
    let be32 =
        |i: usize| -> Option<u32> { Some(u32::from_be_bytes(b.get(i..i + 4)?.try_into().ok()?)) };
    let le32 =
        |i: usize| -> Option<u32> { Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?)) };
    if b.starts_with(b"\x89PNG\r\n\x1a\n") && b.get(12..16) == Some(b"IHDR") {
        return Some((be32(16)?, be32(20)?));
    }
    if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") {
        return Some((le16(6)?, le16(8)?));
    }
    if b.starts_with(b"BM") {
        let w = le32(18)? as i32;
        let h = le32(22)? as i32; // negative = top-down
        return (w > 0 && h != 0).then_some((w as u32, h.unsigned_abs()));
    }
    if b.starts_with(b"RIFF") && b.get(8..12) == Some(b"WEBP") {
        return match b.get(12..16)? {
            b"VP8X" => Some(((le32(24)? & 0xff_ffff) + 1, (le32(27)? & 0xff_ffff) + 1)),
            b"VP8 " => Some((le16(26)? & 0x3fff, le16(28)? & 0x3fff)),
            b"VP8L" => {
                let v = le32(21)?;
                Some(((v & 0x3fff) + 1, ((v >> 14) & 0x3fff) + 1))
            }
            _ => None,
        };
    }
    if b.starts_with(&[0xff, 0xd8]) {
        // walk the JPEG segments to the first start-of-frame
        let mut i = 2;
        while i + 4 <= b.len() {
            if b[i] != 0xff {
                return None;
            }
            let marker = b[i + 1];
            if marker == 0xff {
                i += 1; // fill byte
                continue;
            }
            let len = be16(i + 2)? as usize;
            let sof = matches!(marker, 0xc0..=0xcf) && !matches!(marker, 0xc4 | 0xc8 | 0xcc);
            if sof {
                return Some((be16(i + 7)?, be16(i + 5)?));
            }
            if len < 2 {
                return None;
            }
            i += 2 + len;
        }
    }
    None
}

/// First lines of a text file, or `None` if the bytes do not look like text
/// (NUL bytes, invalid UTF-8 beyond a cut-off character at the end).
pub fn text_preview(b: &[u8], max_lines: usize, max_chars: usize) -> Option<String> {
    if b.is_empty() {
        return Some(String::new());
    }
    if b.contains(&0) {
        return None;
    }
    let b = b.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(b); // UTF-8 BOM
    let s = match std::str::from_utf8(b) {
        Ok(s) => s,
        // the read may have cut a multi-byte character at the end
        Err(e) if e.error_len().is_none() => std::str::from_utf8(&b[..e.valid_up_to()]).ok()?,
        Err(_) => return None,
    };
    let mut out = String::new();
    for (n, line) in s.lines().enumerate() {
        if n >= max_lines {
            break;
        }
        let line: String = line.chars().take(max_chars).collect();
        if n > 0 {
            out.push('\n');
        }
        out.push_str(line.trim_end());
    }
    Some(out)
}

/// "Schreibgeschützt, Versteckt, System" (Explorer's attribute names).
pub fn attributes_de(e: &Entry) -> String {
    let mut v = Vec::new();
    if e.readonly {
        v.push("Schreibgeschützt");
    }
    if e.hidden {
        v.push("Versteckt");
    }
    if e.system {
        v.push("System");
    }
    if e.is_link {
        v.push("Verknüpfung/Link");
    }
    if v.is_empty() {
        "–".into()
    } else {
        v.join(", ")
    }
}

/// Selection summary: (files, folders, bytes of the files).
pub fn totals<'a>(entries: impl IntoIterator<Item = &'a Entry>) -> (usize, usize, u64) {
    entries.into_iter().fold((0, 0, 0), |(f, d, b), e| {
        if e.is_dir() {
            (f, d + 1, b)
        } else {
            (f + 1, d, b + e.size)
        }
    })
}

/// "3 Dateien, 1 Ordner".
pub fn count_de(files: usize, folders: usize) -> String {
    let f = match files {
        0 => None,
        1 => Some("1 Datei".to_string()),
        n => Some(format!("{} Dateien", crate::format_count(n))),
    };
    let d = match folders {
        0 => None,
        1 => Some("1 Ordner".to_string()),
        n => Some(format!("{} Ordner", crate::format_count(n))),
    };
    match (f, d) {
        (Some(f), Some(d)) => format!("{f}, {d}"),
        (Some(x), None) | (None, Some(x)) => x,
        (None, None) => "leer".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EntryKind;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut b = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        b.extend_from_slice(&w.to_be_bytes());
        b.extend_from_slice(&h.to_be_bytes());
        b
    }

    #[test]
    fn picture_sizes() {
        assert_eq!(image_size(&png(1920, 1080)), Some((1920, 1080)));
        assert_eq!(image_size(b"GIF89a\x40\x01\xf0\x00"), Some((320, 240)));
        let mut bmp = vec![0u8; 26];
        bmp[0..2].copy_from_slice(b"BM");
        bmp[18..22].copy_from_slice(&64i32.to_le_bytes());
        bmp[22..26].copy_from_slice(&(-32i32).to_le_bytes());
        assert_eq!(image_size(&bmp), Some((64, 32)), "top-down BMP");
        // JPEG: SOI, APP0 (len 4), SOF0 with height 600, width 800
        let jpg = [
            0xff, 0xd8, 0xff, 0xe0, 0x00, 0x04, 0x00, 0x00, 0xff, 0xc0, 0x00, 0x11, 0x08, 0x02,
            0x58, 0x03, 0x20,
        ];
        assert_eq!(image_size(&jpg), Some((800, 600)));
        // WebP lossless: 100×50 → (99, 49) packed
        let v: u32 = 99 | (49 << 14);
        let mut webp = b"RIFF\0\0\0\0WEBPVP8L\0\0\0\0\x2f".to_vec();
        webp.extend_from_slice(&v.to_le_bytes());
        assert_eq!(image_size(&webp), Some((100, 50)));
        assert_eq!(image_size(b"hello"), None);
        assert_eq!(image_size(&png(1, 1)[..20]), None, "cut header");
        assert_eq!(image_size(&[0xff, 0xd8, 0x00]), None);
    }

    #[test]
    fn text_previews() {
        assert_eq!(text_preview(b"a\nb\r\nc\n", 2, 80).as_deref(), Some("a\nb"));
        assert_eq!(
            text_preview("\u{feff}Grüße".as_bytes(), 5, 80).as_deref(),
            Some("Grüße")
        );
        assert_eq!(text_preview(b"bin\0ary", 5, 80), None);
        assert_eq!(
            text_preview(&[0x61, 0xc3], 5, 80).as_deref(),
            Some("a"),
            "cut UTF-8 at the end"
        );
        assert_eq!(text_preview(&[0x61, 0xff, 0x61], 5, 80), None, "not UTF-8");
        assert_eq!(text_preview(b"abcdef", 5, 3).as_deref(), Some("abc"));
    }

    #[test]
    fn labels_and_totals() {
        let e = |name: &str, dir: bool, size: u64| Entry {
            name: name.into(),
            kind: if dir { EntryKind::Dir } else { EntryKind::File },
            size,
            modified: None,
            hidden: false,
            system: false,
            readonly: false,
            is_link: false,
        };
        let mut ro = e("a", false, 1);
        ro.readonly = true;
        ro.hidden = true;
        assert_eq!(attributes_de(&ro), "Schreibgeschützt, Versteckt");
        assert_eq!(attributes_de(&e("b", false, 0)), "–");
        let all = [e("a", false, 10), e("b", false, 5), e("d", true, 4096)];
        assert_eq!(totals(&all), (2, 1, 15), "folder sizes are not counted");
        assert_eq!(count_de(2, 1), "2 Dateien, 1 Ordner");
        assert_eq!(count_de(1, 0), "1 Datei");
        assert_eq!(count_de(0, 0), "leer");
    }
}
