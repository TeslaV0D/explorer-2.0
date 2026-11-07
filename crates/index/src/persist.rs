//! Saving and loading an index (PLAN-SEARCH.md S1.2): one file per drive,
//! written atomically (temp file + rename), checked completely when read.
//!
//! ```text
//! "EXPL2IDX" · version u32 · flags u32 · saved_at i64 · tag (u32 len + UTF-8)
//! n u32 · ordered u32 · ord_end u64 · waste u64
//! columns (little-endian, n each): parent u32 · name_off u32 · name_len u16 ·
//!   lower_off u32 · lower_len u16 · size u64 · mtime i64 · flags u8 · ext_id u16
//! lowers · names (u64 len + bytes) · ext_names (u32 count, u16 len + bytes each)
//! roots · tail · ord_off (u32 count + u32s)
//! checksum u64 over everything before it
//! ```
//! A file that is damaged, from another version or for another volume is
//! never half used: [`Index::from_bytes`] says why, the caller rebuilds.

use crate::{Index, DIRTY, EXT_NONE, EXT_OTHER, GONE, NONE, SAME, SEP};
use std::io::Write;
use std::path::Path;

const MAGIC: &[u8; 8] = b"EXPL2IDX";
/// Bump on every layout change: an old file is then rebuilt, never misread.
pub const VERSION: u32 = 1;

/// Facts stored next to the index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    /// What the index belongs to (drive root + volume serial / device id).
    pub tag: String,
    /// When it was written (Unix seconds): changes after this are looked for.
    pub saved_at: i64,
}

#[derive(Debug)]
pub enum LoadError {
    Missing,
    /// Written by another format version.
    Outdated(u32),
    /// Belongs to another volume (drive reformatted, other disk).
    OtherVolume(String),
    /// Cut off, altered or inconsistent: rebuild.
    Damaged(String),
    Io(std::io::Error),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Missing => f.write_str("kein gespeicherter Index"),
            LoadError::Outdated(v) => write!(f, "Index aus einer anderen Version ({v})"),
            LoadError::OtherVolume(t) => write!(f, "Index gehört zu einem anderen Laufwerk ({t})"),
            LoadError::Damaged(why) => write!(f, "Index beschädigt: {why}"),
            LoadError::Io(e) => write!(f, "Index nicht lesbar: {e}"),
        }
    }
}

/// Bytes from [`Index::to_bytes`] to `path`: a crash leaves either the old
/// or the new file (the bytes can be made under a read lock, written without).
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("idx.tmp");
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path)
}

fn damaged(why: impl Into<String>) -> LoadError {
    LoadError::Damaged(why.into())
}

/// Fast 64-bit checksum (8 bytes per step): finds cut-off or altered files,
/// not meant against deliberate tampering.
pub(crate) fn checksum(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ data.len() as u64;
    let (words, rest) = data.as_chunks::<8>();
    for w in words {
        h = (h ^ u64::from_le_bytes(*w)).wrapping_mul(0x0000_0100_0000_01b3);
        h ^= h >> 29;
    }
    for &b in rest {
        h = (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// File name for the index of a drive root (`C:\` → `c-1a2b….idx`);
/// Windows roots compare without case, Unix ones with.
pub fn file_name_for(root: &str, case_insensitive: bool) -> String {
    let root = if case_insensitive {
        root.to_lowercase()
    } else {
        root.to_string()
    };
    let clean: String = root
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .take(24)
        .collect();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in root.bytes() {
        h = (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{}-{h:016x}.idx", clean.trim_matches('_'))
}

struct Out(Vec<u8>);

impl Out {
    fn u16s(&mut self, v: &[u16]) {
        self.0.reserve(v.len() * 2);
        for x in v {
            self.0.extend_from_slice(&x.to_le_bytes());
        }
    }
    fn u32s(&mut self, v: &[u32]) {
        self.0.reserve(v.len() * 4);
        for x in v {
            self.0.extend_from_slice(&x.to_le_bytes());
        }
    }
    fn u64s(&mut self, v: &[u64]) {
        self.0.reserve(v.len() * 8);
        for x in v {
            self.0.extend_from_slice(&x.to_le_bytes());
        }
    }
    fn i64s(&mut self, v: &[i64]) {
        self.0.reserve(v.len() * 8);
        for x in v {
            self.0.extend_from_slice(&x.to_le_bytes());
        }
    }
    fn blob(&mut self, b: &[u8]) {
        self.0.extend_from_slice(&(b.len() as u64).to_le_bytes());
        self.0.extend_from_slice(b);
    }
    fn list(&mut self, v: &[u32]) {
        self.0.extend_from_slice(&(v.len() as u32).to_le_bytes());
        self.u32s(v);
    }
}

struct In<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> In<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], LoadError> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&e| e <= self.data.len())
            .ok_or_else(|| damaged("abgeschnitten"))?;
        let s = &self.data[self.pos..end];
        self.pos = end;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32, LoadError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().unwrap_or_default(),
        ))
    }
    fn u64(&mut self) -> Result<u64, LoadError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().unwrap_or_default(),
        ))
    }
    fn count(&mut self, max: usize) -> Result<usize, LoadError> {
        let n = self.u32()? as usize;
        if n > max {
            return Err(damaged("Anzahl zu groß"));
        }
        Ok(n)
    }
    fn u16s(&mut self, n: usize) -> Result<Vec<u16>, LoadError> {
        let b = self.take(n.checked_mul(2).ok_or_else(|| damaged("Größe"))?)?;
        Ok(b.as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect())
    }
    fn u32s(&mut self, n: usize) -> Result<Vec<u32>, LoadError> {
        let b = self.take(n.checked_mul(4).ok_or_else(|| damaged("Größe"))?)?;
        Ok(b.as_chunks::<4>()
            .0
            .iter()
            .map(|c| u32::from_le_bytes(*c))
            .collect())
    }
    fn u64s(&mut self, n: usize) -> Result<Vec<u64>, LoadError> {
        let b = self.take(n.checked_mul(8).ok_or_else(|| damaged("Größe"))?)?;
        Ok(b.as_chunks::<8>()
            .0
            .iter()
            .map(|c| u64::from_le_bytes(*c))
            .collect())
    }
    fn blob(&mut self) -> Result<Vec<u8>, LoadError> {
        let n = usize::try_from(self.u64()?).map_err(|_| damaged("Größe"))?;
        Ok(self.take(n)?.to_vec())
    }
    fn list(&mut self, max: usize) -> Result<Vec<u32>, LoadError> {
        let n = self.count(max)?;
        self.u32s(n)
    }
}

impl Index {
    /// The whole index as bytes (see the module docs for the layout).
    pub fn to_bytes(&self, tag: &str, saved_at: i64) -> Vec<u8> {
        let n = self.slots();
        let mut o = Out(Vec::with_capacity(
            n * 40 + self.lowers.len() + self.names.len() + 1024,
        ));
        o.0.extend_from_slice(MAGIC);
        o.0.extend_from_slice(&VERSION.to_le_bytes());
        o.0.extend_from_slice(&u32::from(self.case_insensitive).to_le_bytes());
        o.0.extend_from_slice(&saved_at.to_le_bytes());
        o.blob(tag.as_bytes());
        o.0.extend_from_slice(&(n as u32).to_le_bytes());
        o.0.extend_from_slice(&(self.ordered as u32).to_le_bytes());
        o.0.extend_from_slice(&(self.ord_end as u64).to_le_bytes());
        o.0.extend_from_slice(&(self.waste as u64).to_le_bytes());
        o.u32s(&self.parent);
        o.u32s(&self.name_off);
        o.u16s(&self.name_len);
        o.u32s(&self.lower_off);
        o.u16s(&self.lower_len);
        o.u64s(&self.size);
        o.i64s(&self.mtime);
        o.0.extend_from_slice(&self.flags);
        o.u16s(&self.ext_id);
        o.blob(&self.lowers);
        o.blob(&self.names);
        o.0.extend_from_slice(&(self.ext_names.len() as u32).to_le_bytes());
        for e in &self.ext_names {
            let b = &e.as_bytes()[..e.len().min(u16::MAX as usize)];
            o.0.extend_from_slice(&(b.len() as u16).to_le_bytes());
            o.0.extend_from_slice(b);
        }
        o.list(&self.roots);
        o.list(&self.tail);
        o.list(&self.ord_off);
        let sum = checksum(&o.0);
        o.0.extend_from_slice(&sum.to_le_bytes());
        o.0
    }

    /// Writes atomically: a crash leaves either the old or the new file.
    pub fn save(&self, path: &Path, tag: &str, saved_at: i64) -> std::io::Result<()> {
        write_atomic(path, &self.to_bytes(tag, saved_at))
    }

    /// Reads a saved index; `expect_tag`: the volume it must belong to.
    pub fn load(path: &Path, expect_tag: Option<&str>) -> Result<(Index, Saved), LoadError> {
        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(LoadError::Missing),
            Err(e) => return Err(LoadError::Io(e)),
        };
        let (ix, saved) = Index::from_bytes(&data)?;
        if let Some(t) = expect_tag {
            if t != saved.tag {
                return Err(LoadError::OtherVolume(saved.tag));
            }
        }
        Ok((ix, saved))
    }

    pub fn from_bytes(data: &[u8]) -> Result<(Index, Saved), LoadError> {
        if data.len() < 8 + 4 + 8 || &data[..8] != MAGIC {
            return Err(damaged("keine Indexdatei"));
        }
        let version = u32::from_le_bytes(data[8..12].try_into().unwrap_or_default());
        if version != VERSION {
            return Err(LoadError::Outdated(version));
        }
        let (body, sum) = data.split_at(data.len() - 8);
        if checksum(body) != u64::from_le_bytes(sum.try_into().unwrap_or_default()) {
            return Err(damaged("Prüfsumme falsch (abgeschnitten oder verändert)"));
        }
        Index::parse_body(body)
    }

    /// Everything after the checksum check: every offset and link is
    /// validated, so even a consistent-looking but wrong file cannot make
    /// a later search or path lookup panic or loop.
    pub(crate) fn parse_body(body: &[u8]) -> Result<(Index, Saved), LoadError> {
        let mut r = In {
            data: body,
            pos: 12,
        };
        let flags_word = r.u32()?;
        let saved_at = r.u64()? as i64;
        let tag = String::from_utf8(r.blob()?).map_err(|_| damaged("Kennung"))?;
        // an entry needs ≥ 26 bytes of columns: bounds the count before allocating
        let max = body.len() / 26 + 1;
        let n = r.count(max)?;
        let ordered = r.u32()? as usize;
        let ord_end = usize::try_from(r.u64()?).map_err(|_| damaged("Größe"))?;
        let waste = usize::try_from(r.u64()?).map_err(|_| damaged("Größe"))?;
        let parent = r.u32s(n)?;
        let name_off = r.u32s(n)?;
        let name_len = r.u16s(n)?;
        let lower_off = r.u32s(n)?;
        let lower_len = r.u16s(n)?;
        let size = r.u64s(n)?;
        let mtime: Vec<i64> = r.u64s(n)?.into_iter().map(|v| v as i64).collect();
        let flags = r.take(n)?.to_vec();
        let ext_id = r.u16s(n)?;
        let lowers = r.blob()?;
        let names = r.blob()?;
        let ext_count = r.count(u16::MAX as usize)?;
        let mut ext_names = Vec::with_capacity(ext_count);
        for _ in 0..ext_count {
            let len = u16::from_le_bytes(r.take(2)?.try_into().unwrap_or_default()) as usize;
            ext_names
                .push(String::from_utf8(r.take(len)?.to_vec()).map_err(|_| damaged("Endung"))?);
        }
        let roots = r.list(n)?;
        let tail = r.list(n)?;
        let ord_off = r.list(n)?;
        if r.pos != body.len() {
            return Err(damaged("Daten nach dem Ende"));
        }

        // ---- consistency (any failure = rebuild, never a panic later)
        if flags_word > 1 || ordered > n || ord_off.len() != ordered || ord_end > lowers.len() {
            return Err(damaged("Kopf"));
        }
        if !lowers.is_empty() && lowers[0] != SEP || std::str::from_utf8(&lowers).is_err() {
            return Err(damaged("Namenspuffer"));
        }
        let names_str = std::str::from_utf8(&names).map_err(|_| damaged("Namen"))?;
        let mut live = 0usize;
        for i in 0..n {
            // between two separators (ASCII) in a valid UTF-8 buffer = valid text
            let (lo, ll) = (lower_off[i] as usize, lower_len[i] as usize);
            if lo == 0 || lowers.get(lo - 1) != Some(&SEP) || lowers.get(lo + ll) != Some(&SEP) {
                return Err(damaged(format!("Name {i}")));
            }
            match name_off[i] {
                SAME => {
                    if name_len[i] != lower_len[i] {
                        return Err(damaged(format!("Name {i}")));
                    }
                }
                o => {
                    let (o, end) = (o as usize, o as usize + name_len[i] as usize);
                    if end > names.len()
                        || !names_str.is_char_boundary(o)
                        || !names_str.is_char_boundary(end)
                    {
                        return Err(damaged(format!("Name {i}")));
                    }
                }
            }
            if parent[i] != NONE && parent[i] as usize >= n {
                return Err(damaged(format!("Eltern {i}")));
            }
            let e = ext_id[i];
            if e != EXT_NONE && e != EXT_OTHER && e as usize >= ext_names.len() {
                return Err(damaged(format!("Endung {i}")));
            }
            if flags[i] & !(crate::DIR | GONE | DIRTY) != 0 {
                return Err(damaged(format!("Flags {i}")));
            }
            if flags[i] & GONE == 0 {
                live += 1;
                // a live entry's parent is live (removal takes the subtree)
                if parent[i] != NONE && flags[parent[i] as usize] & GONE != 0 {
                    return Err(damaged(format!("Eltern {i}")));
                }
            }
        }
        // no cycles in the parent links (path() and is_under() walk them)
        let mut state = vec![0u8; n]; // 0 new, 1 on the current walk, 2 fine
        let mut walk = Vec::new();
        for i in 0..n {
            let mut cur = i;
            while cur != NONE as usize && state[cur] == 0 {
                state[cur] = 1;
                walk.push(cur);
                cur = parent[cur] as usize;
            }
            if cur != NONE as usize && state[cur] == 1 {
                return Err(damaged("Kreis in den Ordnern"));
            }
            for w in walk.drain(..) {
                state[w] = 2;
            }
        }
        // the ordered part: strictly increasing offsets that still point at
        // the names of the clean entries (the buffer search relies on it)
        for k in 0..ordered {
            let o = ord_off[k] as usize;
            if o == 0
                || o >= ord_end
                || k > 0 && ord_off[k - 1] >= ord_off[k]
                || lowers[o - 1] != SEP
            {
                return Err(damaged("Reihenfolge"));
            }
            if flags[k] & DIRTY == 0 && lower_off[k] != ord_off[k] {
                return Err(damaged("Reihenfolge"));
            }
        }
        if roots.iter().chain(&tail).any(|&r| r as usize >= n)
            || roots.iter().any(|&r| parent[r as usize] != NONE)
        {
            return Err(damaged("Wurzeln"));
        }
        let mut ext_map = std::collections::HashMap::with_capacity(ext_names.len());
        for (k, e) in ext_names.iter().enumerate().skip(1) {
            ext_map.insert(e.clone(), k as u16);
        }
        // children lists from the parents (live entries only)
        let mut first_child = vec![NONE; n];
        let mut next_sibling = vec![NONE; n];
        for i in (0..n).rev() {
            let p = parent[i];
            if p != NONE && flags[i] & GONE == 0 {
                next_sibling[i] = first_child[p as usize];
                first_child[p as usize] = i as u32;
            }
        }
        let ix = Index {
            parent,
            first_child,
            next_sibling,
            name_off,
            name_len,
            lower_off,
            lower_len,
            size,
            mtime,
            flags,
            lowers,
            ext_id,
            ext_names,
            ext_map,
            names,
            ord_off,
            ord_end,
            roots,
            live,
            waste: waste.min(body.len()),
            case_insensitive: flags_word == 1,
            ordered,
            tail,
        };
        Ok((ix, Saved { tag, saved_at }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Meta, Now, Query, SortBy};

    fn sample() -> Index {
        let mut ix = Index::new(true);
        let d = Meta {
            is_dir: true,
            size: 0,
            mtime: 5,
        };
        let root = ix.add_root(r"C:\", d);
        let users = ix.add(root, "Users", d);
        for i in 0..300 {
            let f = ix.add(
                users,
                &format!("Bericht {i}.PDF"),
                Meta {
                    is_dir: false,
                    size: i,
                    mtime: 100 + i as i64,
                },
            );
            if i % 7 == 0 {
                ix.add(f, "x", d); // nonsense but legal: entries below a file
            }
        }
        ix.add(
            users,
            "Ünïcode ſtraße.txt",
            Meta {
                is_dir: false,
                size: 1,
                mtime: 1,
            },
        );
        ix.rebuild_order();
        // changes after the rebuild: tail, dirty, removed, waste
        let docs = ix.add(root, "Docs", d);
        ix.add(
            docs,
            "neu.md",
            Meta {
                is_dir: false,
                size: 9,
                mtime: 9,
            },
        );
        let b = ix.find(r"C:\Users\Bericht 5.PDF").unwrap();
        ix.rename(b, ix.parent(b), "Umbenannt.pdf");
        let gone = ix.find(r"C:\Users\Bericht 7.PDF").unwrap();
        ix.remove(gone);
        ix
    }

    const NOW: Now = Now {
        unix: 1_793_883_600,
        utc_offset: 3600,
    };

    fn hits(ix: &Index, q: &str) -> Vec<String> {
        let q = Query::parse(q, NOW).unwrap();
        ix.search(&q, None, SortBy::Name, false)
            .into_iter()
            .map(|i| ix.path(i))
            .collect()
    }

    #[test]
    fn roundtrip_keeps_everything() {
        let ix = sample();
        let bytes = ix.to_bytes("C:\\ serial=1234", 77);
        let (back, saved) = Index::from_bytes(&bytes).unwrap();
        assert_eq!(
            saved,
            Saved {
                tag: "C:\\ serial=1234".into(),
                saved_at: 77
            }
        );
        assert_eq!(back.len(), ix.len());
        assert_eq!(back.slots(), ix.slots());
        for q in [
            "bericht",
            "*.pdf",
            "ext:pdf",
            "umbenannt",
            "neu",
            "ſtraße",
            "straße",
            "path:users",
            "size:>100",
            "folder:",
            "x",
        ] {
            assert_eq!(hits(&back, q), hits(&ix, q), "{q}");
        }
        assert_eq!(
            back.find(r"c:\users\umbenannt.pdf"),
            ix.find(r"C:\Users\Umbenannt.pdf")
        );
        assert!(back.find(r"C:\Users\Bericht 7.PDF").is_none());
        // saving the loaded one gives the same bytes
        assert_eq!(back.to_bytes("C:\\ serial=1234", 77), bytes);
    }

    #[test]
    fn file_roundtrip_atomic_and_tag() {
        let dir = std::env::temp_dir().join(format!("e2idx-{}", std::process::id()));
        let p = dir.join(file_name_for(r"C:\", true));
        let ix = sample();
        ix.save(&p, "vol-1", 5).unwrap();
        assert!(!p.with_extension("idx.tmp").exists());
        assert!(
            matches!(Index::load(&p, Some("vol-2")), Err(LoadError::OtherVolume(t)) if t == "vol-1")
        );
        let (back, _) = Index::load(&p, Some("vol-1")).unwrap();
        assert_eq!(back.len(), ix.len());
        assert!(matches!(
            Index::load(&dir.join("nope.idx"), None),
            Err(LoadError::Missing)
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_cut_and_bit_flip_is_refused() {
        let bytes = sample().to_bytes("t", 1);
        for cut in (0..bytes.len()).step_by(7) {
            assert!(Index::from_bytes(&bytes[..cut]).is_err(), "cut {cut}");
        }
        for at in (0..bytes.len()).step_by(13) {
            let mut b = bytes.clone();
            b[at] ^= 0x20;
            assert!(Index::from_bytes(&b).is_err(), "flip at {at}");
        }
        let mut old = bytes.clone();
        old[8] = 99;
        assert!(matches!(
            Index::from_bytes(&old),
            Err(LoadError::Outdated(_))
        ));
    }

    /// Damage that keeps the checksum valid (bypassed): the structure checks
    /// alone must catch it – or leave an index that is safe to use.
    #[test]
    fn consistent_looking_damage_never_panics() {
        let bytes = sample().to_bytes("t", 1);
        let body_len = bytes.len() - 8;
        let rounds: usize = std::env::var("FUZZ_ROUNDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(4000);
        let seed: u64 = std::env::var("FUZZ_SEED")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let mut x: u64 = 0x9e37_79b9_7f4a_7c15 ^ seed.wrapping_mul(0x2545_f491_4f6c_dd1d) | 1;
        for _ in 0..rounds {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let mut b = bytes[..body_len].to_vec();
            for _ in 0..1 + x % 3 {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let at = 12 + (x as usize) % (body_len - 12);
                b[at] = (x >> 32) as u8;
            }
            if let Ok((ix, _)) = Index::parse_body(&b) {
                // usable without panics or endless loops
                for q in ["bericht", "*.pdf", "ext:pdf", "path:users", "x", "size:>5"] {
                    let q = Query::parse(q, NOW).unwrap();
                    for id in ix
                        .search(&q, None, SortBy::Name, false)
                        .into_iter()
                        .take(50)
                    {
                        let _ = ix.path(id);
                    }
                }
                let _ = ix.find(r"C:\Users\Docs");
            }
        }
    }

    #[test]
    fn file_names() {
        assert_eq!(file_name_for(r"C:\", true), file_name_for(r"c:\", true));
        assert_ne!(file_name_for(r"C:\", true), file_name_for(r"D:\", true));
        assert_ne!(file_name_for("/A", false), file_name_for("/a", false));
        let n = file_name_for("/home/a b/ü", false);
        assert!(
            n.ends_with(".idx")
                && n.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "_-.".contains(c)),
            "{n}"
        );
    }
}
