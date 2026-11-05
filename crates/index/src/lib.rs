//! Name index for the instant search (Phase 2, `docs/PLAN-SEARCH.md` S1.1).
//!
//! Plain arrays instead of a node per allocation (~1 M entries < 100 MB):
//! every entry has a parent, its lower-case name in one contiguous buffer
//! (`\n` before and after every name → one SIMD pass finds a word in all
//! names, one regex pass with `^`/`$` per name), the original name only where
//! it differs, an extension id, size, modification time and flags; children are a
//! singly linked list (first child / next sibling) for path lookups during
//! live updates. Search scans the arrays in parallel (see [`query`]).
//!
//! Ids are stable until [`Index::compact`]; removed entries are only marked.

pub mod query;

pub use query::{Now, ParseError, Query};

use std::cmp::Ordering;

/// "No entry" (parent of a root, end of a sibling list).
pub const NONE: u32 = u32::MAX;

const DIR: u8 = 1;
const GONE: u8 = 2;
/// Renamed since the name order was built: its place in `order` is stale.
const DIRTY: u8 = 4;
/// `name_off` value: the original name equals the lower-case one.
const SAME: u32 = u32::MAX;
/// Separates the names in `lowers` (`\n` in a name is stored as U+FFFD there).
pub(crate) const SEP: u8 = b'\n';
/// Extension id: no extension (folders too).
pub(crate) const EXT_NONE: u16 = 0;
/// Extension id: table full, compare the text.
pub(crate) const EXT_OTHER: u16 = u16::MAX;

/// What the index keeps about an entry besides its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Meta {
    pub is_dir: bool,
    /// Bytes (0 for folders).
    pub size: u64,
    /// Seconds since 1970 (UTC), 0 = unknown.
    pub mtime: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Name,
    Path,
    Size,
    Modified,
}

#[derive(Debug, Default, Clone)]
pub struct Index {
    parent: Vec<u32>,
    first_child: Vec<u32>,
    next_sibling: Vec<u32>,
    name_off: Vec<u32>,
    name_len: Vec<u16>,
    lower_off: Vec<u32>,
    lower_len: Vec<u16>,
    size: Vec<u64>,
    mtime: Vec<i64>,
    flags: Vec<u8>,
    /// `\nname\nname\n…` lower-case, in id order after [`Index::rebuild_order`].
    lowers: Vec<u8>,
    /// Extension of each entry as an id into `ext_names` (`ext:` compares numbers).
    ext_id: Vec<u16>,
    ext_names: Vec<String>,
    ext_map: std::collections::HashMap<String, u16>,
    /// Original names that differ from their lower-case form.
    names: Vec<u8>,
    /// `lower_off` of the ordered part at the last rebuild (the buffer search
    /// maps a hit position to an id with it; renamed entries keep stale bytes).
    ord_off: Vec<u32>,
    ord_end: usize,
    roots: Vec<u32>,
    live: usize,
    /// Arena bytes no longer referenced (renames/removals) → [`Index::compact`].
    waste: usize,
    /// Windows: names compare case-insensitively.
    case_insensitive: bool,
    /// Ids `0..ordered` are laid out in natural name order (by
    /// [`Index::rebuild_order`]): the search reads memory sequentially and
    /// its results come sorted by name for free.
    ordered: usize,
    /// Renamed since then (their place is stale; merged in at search time,
    /// like everything added after `ordered`).
    tail: Vec<u32>,
}

impl Index {
    pub fn new(case_insensitive: bool) -> Index {
        Index {
            case_insensitive,
            ..Default::default()
        }
    }

    /// Live (not removed) entries.
    pub fn len(&self) -> usize {
        self.live
    }

    pub fn is_empty(&self) -> bool {
        self.live == 0
    }

    /// Slots incl. removed ones (ids are `0..slots()`).
    pub fn slots(&self) -> usize {
        self.parent.len()
    }

    pub fn case_insensitive(&self) -> bool {
        self.case_insensitive
    }

    /// Approximate heap use.
    pub fn memory_bytes(&self) -> usize {
        self.parent.capacity() * 4 * 3
            + self.tail.capacity() * 4
            + self.name_off.capacity() * 4
            + self.name_len.capacity() * 2
            + self.lower_off.capacity() * 4
            + self.lower_len.capacity() * 2
            + self.size.capacity() * 8
            + self.mtime.capacity() * 8
            + self.flags.capacity()
            + self.lowers.capacity()
            + self.names.capacity()
            + self.ord_off.capacity() * 4
            + self.ext_id.capacity() * 2
    }

    pub fn reserve(&mut self, entries: usize, name_bytes: usize) {
        for v in [
            &mut self.parent,
            &mut self.first_child,
            &mut self.next_sibling,
        ] {
            v.reserve(entries);
        }
        self.name_off.reserve(entries);
        self.name_len.reserve(entries);
        self.lower_off.reserve(entries);
        self.lower_len.reserve(entries);
        self.size.reserve(entries);
        self.mtime.reserve(entries);
        self.flags.reserve(entries);
        self.ext_id.reserve(entries);
        self.lowers.reserve(name_bytes + entries + 1);
        self.names.reserve(name_bytes / 2);
    }

    fn push_name(&mut self, name: &str) -> (u32, u16, u32, u16) {
        // names longer than 64 KiB do not exist on any file system we index
        let name = &name[..floor_char_boundary(name, u16::MAX as usize)];
        if self.lowers.is_empty() {
            self.lowers.push(SEP);
        }
        // `\n` separates the names in `lowers` (only Unix allows it in a name)
        let mut lower = self.fold(name);
        if lower.contains('\n') {
            lower = lower.replace('\n', "\u{fffd}");
        }
        let lower = &lower[..floor_char_boundary(&lower, u16::MAX as usize)];
        let loff = self.lowers.len() as u32;
        self.lowers.extend_from_slice(lower.as_bytes());
        self.lowers.push(SEP);
        if lower.as_bytes() == name.as_bytes() {
            return (SAME, name.len() as u16, loff, lower.len() as u16);
        }
        let off = self.names.len() as u32;
        self.names.extend_from_slice(name.as_bytes());
        (off, name.len() as u16, loff, lower.len() as u16)
    }

    /// Lower-case form used for matching (always folded: search is
    /// case-insensitive unless `case:` asks otherwise).
    fn fold(&self, s: &str) -> String {
        if s.is_ascii() {
            s.to_ascii_lowercase()
        } else {
            // the long s is `s` for case-insensitive matching but stays `ſ`
            // when lower-cased (regex literal prefilter, word search)
            s.to_lowercase().replace('\u{17f}', "s")
        }
    }

    fn push(&mut self, parent: u32, name: &str, meta: Meta) -> u32 {
        let id = self.parent.len() as u32;
        let (no, nl, lo, ll) = self.push_name(name);
        self.parent.push(parent);
        self.first_child.push(NONE);
        self.next_sibling.push(NONE);
        self.name_off.push(no);
        self.name_len.push(nl);
        self.lower_off.push(lo);
        self.lower_len.push(ll);
        self.size.push(if meta.is_dir { 0 } else { meta.size });
        self.mtime.push(meta.mtime);
        self.flags.push(if meta.is_dir { DIR } else { 0 });
        self.ext_id.push(EXT_NONE);
        self.update_ext(id as usize);
        self.live += 1;
        id
    }

    /// Extension id from the current lower-case name (folders have none).
    fn update_ext(&mut self, i: usize) {
        let id = if self.flags[i] & DIR != 0 {
            EXT_NONE
        } else {
            let lower = self.lower_bytes(i);
            match memchr::memrchr(b'.', lower) {
                Some(dot) if dot > 0 && dot + 1 < lower.len() => {
                    let ext = String::from_utf8_lossy(&lower[dot + 1..]).into_owned();
                    match self.ext_map.get(&ext) {
                        Some(&e) => e,
                        None if self.ext_names.len() + 1 < EXT_OTHER as usize => {
                            if self.ext_names.is_empty() {
                                self.ext_names.push(String::new()); // id 0 = none
                            }
                            let e = self.ext_names.len() as u16;
                            self.ext_names.push(ext.clone());
                            self.ext_map.insert(ext, e);
                            e
                        }
                        None => EXT_OTHER,
                    }
                }
                _ => EXT_NONE,
            }
        };
        self.ext_id[i] = id;
    }

    pub(crate) fn ext_id(&self, i: usize) -> u16 {
        self.ext_id[i]
    }

    pub(crate) fn ext_ids(&self) -> &[u16] {
        &self.ext_id
    }

    pub(crate) fn ext_lookup(&self, ext: &str) -> Option<u16> {
        self.ext_map.get(ext).copied()
    }

    /// A drive / mount point; its name is the full path (`C:\`, `/home/a`).
    pub fn add_root(&mut self, path: &str, meta: Meta) -> u32 {
        let id = self.push(
            NONE,
            path,
            Meta {
                is_dir: true,
                ..meta
            },
        );
        self.roots.push(id);
        id
    }

    pub fn roots(&self) -> &[u32] {
        &self.roots
    }

    /// New child of `parent` (no duplicate check: the builder knows its names;
    /// live updates use [`Index::upsert`]).
    pub fn add(&mut self, parent: u32, name: &str, meta: Meta) -> u32 {
        let id = self.push(parent, name, meta);
        if let Some(p) = self.slot(parent) {
            self.next_sibling[id as usize] = self.first_child[p];
            self.first_child[p] = id;
        }
        id
    }

    /// Add or update the child `name` of `parent`.
    pub fn upsert(&mut self, parent: u32, name: &str, meta: Meta) -> u32 {
        match self.child(parent, name) {
            Some(id) => {
                self.set_meta(id, meta);
                id
            }
            None => self.add(parent, name, meta),
        }
    }

    fn slot(&self, id: u32) -> Option<usize> {
        let i = id as usize;
        (i < self.parent.len() && self.flags[i] & GONE == 0).then_some(i)
    }

    pub fn exists(&self, id: u32) -> bool {
        self.slot(id).is_some()
    }

    pub fn name(&self, id: u32) -> &str {
        // the buffers only ever receive whole `&str`s
        std::str::from_utf8(self.name_bytes(id as usize)).unwrap_or("")
    }

    pub(crate) fn lower(&self, i: usize) -> &str {
        std::str::from_utf8(self.lower_bytes(i)).unwrap_or("")
    }

    /// Lower-case name without UTF-8 validation (hot loop of the search).
    pub(crate) fn lower_bytes(&self, i: usize) -> &[u8] {
        let o = self.lower_off[i] as usize;
        &self.lowers[o..o + self.lower_len[i] as usize]
    }

    pub(crate) fn name_bytes(&self, i: usize) -> &[u8] {
        match self.name_off[i] {
            SAME => self.lower_bytes(i),
            o => &self.names[o as usize..o as usize + self.name_len[i] as usize],
        }
    }

    fn name_waste(&self, i: usize) -> usize {
        self.lower_len[i] as usize
            + 1
            + if self.name_off[i] == SAME {
                0
            } else {
                self.name_len[i] as usize
            }
    }

    /// The contiguous lower-case buffer and the start offset of each ordered
    /// entry (buffer search in [`query`]).
    pub(crate) fn ordered_names(&self) -> (&[u8], &[u32]) {
        (&self.lowers[..self.ord_end], &self.ord_off)
    }

    pub fn meta(&self, id: u32) -> Meta {
        let i = id as usize;
        Meta {
            is_dir: self.flags[i] & DIR != 0,
            size: self.size[i],
            mtime: self.mtime[i],
        }
    }

    pub fn is_dir(&self, id: u32) -> bool {
        self.flags[id as usize] & DIR != 0
    }

    pub fn parent(&self, id: u32) -> u32 {
        self.parent[id as usize]
    }

    pub fn set_meta(&mut self, id: u32, meta: Meta) {
        if let Some(i) = self.slot(id) {
            self.size[i] = if meta.is_dir { 0 } else { meta.size };
            self.mtime[i] = meta.mtime;
            self.flags[i] = (self.flags[i] & !DIR) | if meta.is_dir { DIR } else { 0 };
            self.update_ext(i);
        }
    }

    pub fn children(&self, id: u32) -> Children<'_> {
        Children {
            index: self,
            next: self.slot(id).map_or(NONE, |i| self.first_child[i]),
        }
    }

    fn names_equal(&self, id: u32, name: &str) -> bool {
        if self.case_insensitive {
            let i = id as usize;
            let lower = self.lower(i);
            lower.len() <= name.len() * 3 && lower == self.fold(name)
        } else {
            self.name(id) == name
        }
    }

    pub fn child(&self, parent: u32, name: &str) -> Option<u32> {
        self.children(parent).find(|&c| self.names_equal(c, name))
    }

    /// Separator of the root that `id` belongs to.
    fn sep_of_root(&self, root: u32) -> char {
        let r = self.name(root);
        if r.contains('\\') || r.ends_with(':') {
            '\\'
        } else {
            '/'
        }
    }

    pub fn root_of(&self, mut id: u32) -> u32 {
        while self.parent[id as usize] != NONE {
            id = self.parent[id as usize];
        }
        id
    }

    /// Full path (`C:\Users\a.txt`, `/home/a/b`).
    pub fn path(&self, id: u32) -> String {
        let mut chain = Vec::new();
        let mut cur = id;
        while cur != NONE {
            chain.push(cur);
            cur = self.parent[cur as usize];
        }
        let root = *chain.last().unwrap_or(&id);
        let sep = self.sep_of_root(root);
        let mut s = String::new();
        for (k, &n) in chain.iter().rev().enumerate() {
            if k > 0 && !s.ends_with(sep) {
                s.push(sep);
            }
            s.push_str(self.name(n));
        }
        s
    }

    /// Entry for a full path (case-insensitive on Windows), via the longest
    /// matching root.
    pub fn find(&self, path: &str) -> Option<u32> {
        let mut best: Option<(u32, usize)> = None;
        for &r in &self.roots {
            if !self.exists(r) {
                continue;
            }
            let rn = self.name(r);
            let hit = if self.case_insensitive {
                path.len() >= rn.len()
                    && path.is_char_boundary(rn.len())
                    && path[..rn.len()].eq_ignore_ascii_case(rn)
            } else {
                path.starts_with(rn)
            };
            let boundary = path.len() == rn.len()
                || rn.ends_with(['/', '\\'])
                || path[rn.len()..].starts_with(['/', '\\']);
            if hit && boundary && best.is_none_or(|(_, l)| rn.len() > l) {
                best = Some((r, rn.len()));
            }
        }
        let (mut cur, skip) = best?;
        for part in path[skip..].split(['/', '\\']).filter(|p| !p.is_empty()) {
            cur = self.child(cur, part)?;
        }
        Some(cur)
    }

    /// Removes `id` and everything below it.
    pub fn remove(&mut self, id: u32) {
        let Some(i) = self.slot(id) else { return };
        self.unlink(id);
        if self.parent[i] == NONE {
            self.roots.retain(|&r| r != id);
        }
        let mut stack = vec![id];
        while let Some(n) = stack.pop() {
            let k = n as usize;
            if self.flags[k] & GONE != 0 {
                continue;
            }
            self.flags[k] |= GONE;
            self.live -= 1;
            self.waste += self.name_waste(k);
            let mut c = self.first_child[k];
            while c != NONE {
                stack.push(c);
                c = self.next_sibling[c as usize];
            }
        }
    }

    fn unlink(&mut self, id: u32) {
        let p = self.parent[id as usize];
        if p == NONE {
            return;
        }
        let pi = p as usize;
        if self.first_child[pi] == id {
            self.first_child[pi] = self.next_sibling[id as usize];
        } else {
            let mut c = self.first_child[pi];
            while c != NONE {
                let next = self.next_sibling[c as usize];
                if next == id {
                    self.next_sibling[c as usize] = self.next_sibling[id as usize];
                    break;
                }
                c = next;
            }
        }
        self.next_sibling[id as usize] = NONE;
    }

    /// Rename and/or move (`new_parent` = current parent for a rename).
    pub fn rename(&mut self, id: u32, new_parent: u32, new_name: &str) {
        let Some(i) = self.slot(id) else { return };
        if self.slot(new_parent).is_none() || self.is_under(new_parent, id) {
            return;
        }
        self.waste += self.name_waste(i);
        let (no, nl, lo, ll) = self.push_name(new_name);
        if i < self.ordered && self.flags[i] & DIRTY == 0 {
            self.flags[i] |= DIRTY;
            self.tail.push(id);
        }
        self.name_off[i] = no;
        self.name_len[i] = nl;
        self.lower_off[i] = lo;
        self.lower_len[i] = ll;
        self.update_ext(i);
        if self.parent[i] != new_parent {
            self.unlink(id);
            self.parent[i] = new_parent;
            let p = new_parent as usize;
            self.next_sibling[i] = self.first_child[p];
            self.first_child[p] = id;
        }
    }

    /// `id` is `ancestor` or below it.
    pub fn is_under(&self, mut id: u32, ancestor: u32) -> bool {
        loop {
            if id == ancestor {
                return true;
            }
            if id == NONE {
                return false;
            }
            id = self.parent[id as usize];
        }
    }

    /// Unreferenced arena bytes + removed slots worth rebuilding for.
    pub fn needs_compact(&self) -> bool {
        let gone = self.slots() - self.live;
        gone > 4096 && gone * 4 > self.slots()
            || self.waste > (1 << 20) && self.waste * 2 > self.lowers.len() + self.names.len()
    }

    /// Rebuilds without removed entries. Returns `old id → new id` (NONE = gone).
    pub fn compact(&mut self) -> Vec<u32> {
        self.rebuild_order()
    }

    /// Lays all live entries out in natural name order (after a bulk build,
    /// on load, or when the unsorted part grows) and drops removed ones.
    /// Ids change: returns `old id → new id` (NONE = gone).
    pub fn rebuild_order(&mut self) -> Vec<u32> {
        let mut ids: Vec<u32> = (0..self.slots() as u32)
            .filter(|&i| self.flags[i as usize] & GONE == 0)
            .collect();
        self.sort_names(&mut ids);
        let mut map = vec![NONE; self.slots()];
        for (new, &old) in ids.iter().enumerate() {
            map[old as usize] = new as u32;
        }
        let mut out = Index::new(self.case_insensitive);
        out.reserve(ids.len(), self.lowers.len().saturating_sub(self.waste));
        for &old in &ids {
            let p = self.parent[old as usize];
            out.push(
                if p == NONE { NONE } else { map[p as usize] },
                self.name(old),
                self.meta(old),
            );
        }
        // children lists from the parents (a parent may now come after its child)
        for new in 0..out.slots() {
            let p = out.parent[new];
            if p != NONE {
                out.next_sibling[new] = out.first_child[p as usize];
                out.first_child[p as usize] = new as u32;
            }
        }
        out.roots = self
            .roots
            .iter()
            .map(|&r| map[r as usize])
            .filter(|&r| r != NONE)
            .collect();
        out.ordered = out.slots();
        out.ord_off = out.lower_off.clone();
        out.ord_end = out.lowers.len();
        *self = out;
        map
    }

    /// Changes since the last [`Index::rebuild_order`] (cheap to merge while small).
    pub fn unsorted(&self) -> usize {
        self.tail.len() + (self.slots() - self.ordered)
    }

    fn cmp_names(&self, a: u32, b: u32) -> Ordering {
        explorer_core::natural_cmp(self.name(a), self.name(b)).then(a.cmp(&b))
    }

    /// Sort by name, in parallel for big inputs.
    fn sort_names(&self, ids: &mut [u32]) {
        let threads = std::thread::available_parallelism()
            .map_or(1, |n| n.get())
            .min(16);
        if ids.len() < 100_000 || threads < 2 {
            ids.sort_unstable_by(|a, b| self.cmp_names(*a, *b));
            return;
        }
        let chunk = ids.len().div_ceil(threads);
        std::thread::scope(|s| {
            for part in ids.chunks_mut(chunk) {
                s.spawn(move || part.sort_unstable_by(|a, b| self.cmp_names(*a, *b)));
            }
        });
        // merge the sorted runs pairwise
        let mut run = chunk;
        let mut buf = ids.to_vec();
        while run < ids.len() {
            for (dst, src) in buf.chunks_mut(2 * run).zip(ids.chunks(2 * run)) {
                let (a, b) = src.split_at(run.min(src.len()));
                self.merge_into(a, b, dst);
            }
            ids.copy_from_slice(&buf);
            run *= 2;
        }
    }

    fn merge_into(&self, a: &[u32], b: &[u32], dst: &mut [u32]) {
        let (mut i, mut j) = (0, 0);
        for slot in dst.iter_mut() {
            let take_a =
                j >= b.len() || (i < a.len() && self.cmp_names(a[i], b[j]) != Ordering::Greater);
            if take_a {
                *slot = a[i];
                i += 1;
            } else {
                *slot = b[j];
                j += 1;
            }
        }
    }

    /// Merge two name-sorted id lists.
    pub(crate) fn merge_sorted(&self, a: Vec<u32>, b: Vec<u32>) -> Vec<u32> {
        if b.is_empty() {
            return a;
        }
        if a.is_empty() {
            return b;
        }
        let mut out = vec![0; a.len() + b.len()];
        self.merge_into(&a, &b, &mut out);
        out
    }

    pub(crate) fn ordered(&self) -> usize {
        self.ordered
    }

    /// Entries outside the sorted part: renamed ones + everything added later.
    pub(crate) fn unsorted_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.tail
            .iter()
            .copied()
            .chain(self.ordered as u32..self.slots() as u32)
    }

    /// Sort `ids` in place. Input in name order (search results) keeps
    /// names as the tie-breaker of the other keys (stable sorts).
    pub fn sort(&self, ids: &mut [u32], by: SortBy, descending: bool) {
        match by {
            SortBy::Name => {
                if !ids
                    .windows(2)
                    .all(|w| self.cmp_names(w[0], w[1]) != Ordering::Greater)
                {
                    self.sort_names(ids);
                }
            }
            SortBy::Size => ids.sort_by_key(|&i| self.size[i as usize]),
            SortBy::Modified => ids.sort_by_key(|&i| self.mtime[i as usize]),
            SortBy::Path => {
                let mut keyed: Vec<(String, u32)> =
                    ids.iter().map(|&i| (self.path(i), i)).collect();
                keyed.sort_by(|a, b| explorer_core::natural_cmp(&a.0, &b.0));
                for (slot, (_, id)) in ids.iter_mut().zip(keyed) {
                    *slot = id;
                }
            }
        }
        if descending {
            ids.reverse();
        }
    }

    /// Entries matching `q`, optionally only below `scope`, sorted.
    pub fn search(&self, q: &Query, scope: Option<u32>, by: SortBy, descending: bool) -> Vec<u32> {
        let mut hits = query::run(self, q, scope);
        if by != SortBy::Name {
            self.sort(&mut hits, by, false);
        }
        if descending {
            hits.reverse();
        }
        hits
    }

    /// Flags, sizes and times as slices (tight loops over the columns).
    pub(crate) fn columns(&self) -> (&[u8], &[u64], &[i64]) {
        (&self.flags, &self.size, &self.mtime)
    }

    pub(crate) fn flags_of(&self, i: usize) -> u8 {
        self.flags[i]
    }

    pub(crate) fn raw(&self, i: usize) -> (u8, u64, i64) {
        (self.flags[i], self.size[i], self.mtime[i])
    }
}

pub(crate) const FLAG_DIR: u8 = DIR;
pub(crate) const FLAG_GONE: u8 = GONE;
pub(crate) const FLAG_DIRTY: u8 = DIRTY;

pub struct Children<'a> {
    index: &'a Index,
    next: u32,
}

impl Iterator for Children<'_> {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.next == NONE {
            return None;
        }
        let id = self.next;
        self.next = self.index.next_sibling[id as usize];
        Some(id)
    }
}

fn floor_char_boundary(s: &str, max: usize) -> usize {
    if s.len() <= max {
        return s.len();
    }
    let mut i = max;
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Natural order helper re-exported for callers sorting paths.
pub fn natural(a: &str, b: &str) -> Ordering {
    explorer_core::natural_cmp(a, b)
}

#[cfg(test)]
mod tests;
