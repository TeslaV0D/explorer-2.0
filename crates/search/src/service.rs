//! The search service (PLAN-SEARCH.md S1.3): one maintainer thread per drive
//! loads (or builds) its index, brings it up to date, applies the watcher's
//! changes and saves it now and then; searches read it at any time.
//!
//! Locking: the maintainer is the only writer and holds the write lock only
//! for short slices (`Config::slice`), so a search waits at most that long.
//! Reordering and saving happen under the read lock (searches go on).
//!
//! Saved state is only ever exact: an index whose last change was cut off
//! (app closed in the middle of adding a big folder) is not saved; the
//! previous file stays and the next start's refresh finds the difference.

use crate::live::{inside, plan, recheck_target, run, Ran, RECHECK_NEW};
use crate::refresh::{Refresh, RefreshReport};
use crate::walk::{build, Progress, Report, Walker};
use crate::watch::{watch_tree, Batch, Pending};
use explorer_index::{persist, Exclusions, Index, Query, SortBy};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// How the service works (defaults in [`Config::new`]).
#[derive(Debug, Clone)]
pub struct Config {
    /// Where the index files live (one per drive).
    pub dir: PathBuf,
    pub exclusions: Exclusions,
    /// Reader threads for building (None: by CPU count).
    pub threads: Option<usize>,
    /// Apply changes this long after the last event …
    pub quiet: Duration,
    /// … but no later than this after the first.
    pub max_wait: Duration,
    /// Save a changed index at most this often (and when stopping).
    pub save_every: Duration,
    /// Lay the index out again from this many unsorted entries (at least
    /// 5 % of the index): searches stay fast.
    pub reorder_min: usize,
    /// Longest hold of the write lock.
    pub slice: Duration,
}

impl Config {
    pub fn new(dir: PathBuf, exclusions: Exclusions) -> Config {
        Config {
            dir,
            exclusions,
            threads: None,
            quiet: Duration::from_millis(200),
            max_wait: Duration::from_millis(600),
            save_every: Duration::from_secs(300),
            reorder_min: 20_000,
            slice: Duration::from_millis(15),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Waiting,
    Loading,
    /// First build: `Status::entries` grows.
    Building,
    /// Loaded index searchable, being compared with the disk.
    Refreshing,
    Ready,
    Stopped,
}

/// Where the index came from at start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    Loaded {
        ms: u64,
    },
    /// No usable saved index (`why`, German, for the status line).
    Built {
        why: String,
        ms: u64,
    },
}

#[derive(Debug, Clone, Default)]
pub struct Status {
    pub phase: Phase,
    pub entries: usize,
    pub origin: Option<Origin>,
    pub build: Option<Report>,
    /// The last comparison with the disk (start, lost events, `rescan`).
    pub refresh: Option<RefreshReport>,
    /// Live updates running; if not, `watch_error` says why.
    pub live: bool,
    pub watch_error: Option<String>,
    pub save_error: Option<String>,
    /// `saved_at` of the last successful save (Unix seconds).
    pub saved: Option<i64>,
    /// Changes applied by the watcher (units of work that changed something).
    pub live_changes: u64,
}

/// Search results, valid for one [`Volume::generation`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hits {
    pub generation: u64,
    pub ids: Vec<u32>,
}

/// One indexed drive (or folder).
pub struct Volume {
    root: PathBuf,
    file: PathBuf,
    index: RwLock<Option<Index>>,
    /// Bumped whenever ids change meaning (new index, reordered).
    generation: AtomicU64,
    status: Mutex<Status>,
    pending: Arc<Pending>,
}

impl Volume {
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn index_file(&self) -> &Path {
        &self.file
    }

    pub fn status(&self) -> Status {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    /// Read access (None before the index is loaded or built).
    pub fn read<R>(&self, f: impl FnOnce(&Index) -> R) -> Option<R> {
        self.index
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(f)
    }

    /// `scope`: only below this folder (a folder above the root = all).
    pub fn search(&self, q: &Query, scope: Option<&Path>, by: SortBy, descending: bool) -> Hits {
        let g = self.index.read().unwrap_or_else(|e| e.into_inner());
        // read under the lock: a writer bumps it while holding the write lock
        let generation = self.generation();
        let Some(ix) = g.as_ref() else {
            return Hits {
                generation,
                ids: Vec::new(),
            };
        };
        let scope_id = match scope {
            None => None,
            Some(p) => {
                let s = p.to_string_lossy();
                let root = self.root.to_string_lossy();
                if same_path(&s, &root) || inside(&root, &s) {
                    None
                } else {
                    match ix.find(&s) {
                        Some(id) => Some(id),
                        None => {
                            return Hits {
                                generation,
                                ids: Vec::new(),
                            }
                        }
                    }
                }
            }
        };
        Hits {
            generation,
            ids: ix.search(q, scope_id, by, descending),
        }
    }

    /// Full path of a hit (None if the index changed meanwhile: search again).
    pub fn path_of(&self, generation: u64, id: u32) -> Option<String> {
        let g = self.index.read().unwrap_or_else(|e| e.into_inner());
        if self.generation() != generation {
            return None;
        }
        g.as_ref()
            .filter(|ix| (id as usize) < ix.slots() && ix.exists(id))
            .map(|ix| ix.path(id))
    }

    /// Compare the whole drive with the disk again (menu, lost events).
    pub fn rescan(&self) {
        self.pending.overflow();
    }

    fn update(&self, f: impl FnOnce(&mut Status)) {
        f(&mut self.status.lock().unwrap_or_else(|e| e.into_inner()));
    }

    fn write<R>(&self, f: impl FnOnce(&mut Index) -> R) -> Option<R> {
        self.index
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .as_mut()
            .map(f)
    }

    /// A new index (other ids): swap + new generation under the write lock.
    fn publish(&self, ix: Index) {
        let len = ix.len();
        let mut g = self.index.write().unwrap_or_else(|e| e.into_inner());
        *g = Some(ix);
        self.generation.fetch_add(1, Ordering::AcqRel);
        drop(g);
        self.update(|s| s.entries = len);
    }
}

/// The running service; dropping it stops (and saves) every drive.
pub struct Search {
    volumes: Vec<Arc<Volume>>,
    threads: Vec<JoinHandle<()>>,
    cancel: Arc<AtomicBool>,
}

type OnChange = Arc<dyn Fn(usize) + Send + Sync>;

impl Search {
    /// Starts one maintainer per root. `on_change(volume index)` is called
    /// (from that thread) whenever results may differ: search again.
    pub fn start(
        roots: &[PathBuf],
        cfg: Config,
        on_change: impl Fn(usize) + Send + Sync + 'static,
    ) -> Search {
        let cfg = Arc::new(cfg);
        let on_change: OnChange = Arc::new(on_change);
        let cancel = Arc::new(AtomicBool::new(false));
        let mut volumes = Vec::new();
        let mut threads = Vec::new();
        for (i, root) in roots.iter().enumerate() {
            let name = persist::file_name_for(&root.to_string_lossy(), crate::CASE_INSENSITIVE);
            let vol = Arc::new(Volume {
                root: root.clone(),
                file: cfg.dir.join(name),
                index: RwLock::new(None),
                generation: AtomicU64::new(0),
                status: Mutex::new(Status::default()),
                pending: Arc::new(Pending::new(cfg.quiet, cfg.max_wait)),
            });
            volumes.push(vol.clone());
            let (cfg, cancel, on_change) = (cfg.clone(), cancel.clone(), on_change.clone());
            let spawned = std::thread::Builder::new()
                .name(format!("index-{i}"))
                .spawn(move || maintain(&vol, i, &cfg, &cancel, &on_change));
            if let Ok(t) = spawned {
                threads.push(t);
            }
        }
        Search {
            volumes,
            threads,
            cancel,
        }
    }

    pub fn volumes(&self) -> &[Arc<Volume>] {
        &self.volumes
    }

    /// Stops every maintainer and waits for the final saves.
    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        for v in &self.volumes {
            v.pending.stop();
        }
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

impl Drop for Search {
    fn drop(&mut self) {
        self.stop();
    }
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

fn same_path(a: &str, b: &str) -> bool {
    let trim = |s: &str| s.trim_end_matches(['/', '\\']).to_string();
    if crate::CASE_INSENSITIVE {
        trim(a).to_lowercase() == trim(b).to_lowercase()
    } else {
        trim(a) == trim(b)
    }
}

/// Seconds before the last exact moment that are looked at again on the
/// next start (events arrive a little late; clocks of network drives drift).
const SAVE_MARGIN: i64 = 10;

/// See [`RECHECK_NEW`].
const RECHECK_AFTER: Duration = Duration::from_secs(1);

fn maintain(vol: &Volume, i: usize, cfg: &Config, cancel: &AtomicBool, on_change: &OnChange) {
    crate::background_priority();
    let changed = || on_change(i);
    let mut w = Walker::new(&cfg.exclusions);
    if let Some(t) = cfg.threads {
        w.threads = t.max(1);
    }
    w.cancel = Some(cancel);
    // Watcher start costs time (Linux: one inotify watch per folder), so a
    // saved index is made searchable first. Nothing is missed: the watcher
    // runs before the comparison with the disk (load) or the walk (build).
    let start_watch = || match watch_tree(&vol.root, vol.pending.clone()) {
        Ok(wt) => {
            vol.update(|s| s.live = true);
            Some(wt)
        }
        Err(e) => {
            vol.update(|s| s.watch_error = Some(e.to_string()));
            None
        }
    };
    let tag = explorer_fs::places::volume_tag(&vol.root);
    let root_text = vol.root.to_string_lossy().into_owned();
    vol.update(|s| s.phase = Phase::Loading);
    changed();

    let t0 = Instant::now();
    let loaded = match Index::load(&vol.file, Some(&tag)) {
        Ok((ix, saved))
            if ix.roots().len() == 1 && same_path(&ix.path(ix.roots()[0]), &root_text) =>
        {
            Ok((ix, saved))
        }
        Ok(_) => Err("Index gehört zu einem anderen Ordner".to_string()),
        Err(e) => Err(e.to_string()),
    };
    // the index matches the disk up to this moment (Unix seconds)
    let mut exact_at;
    let mut dirty;
    let watcher;
    match loaded {
        Ok((ix, saved)) => {
            let ms = t0.elapsed().as_millis() as u64;
            vol.publish(ix);
            vol.update(|s| {
                s.origin = Some(Origin::Loaded { ms });
                s.phase = Phase::Refreshing;
            });
            changed();
            watcher = start_watch();
            exact_at = now_unix();
            let Some(rep) = refresh_in_slices(vol, &w, cfg, None, saved.saved_at) else {
                return; // stopped: the saved file stays as it was
            };
            dirty = rep.changed();
            vol.update(|s| s.refresh = Some(rep));
        }
        Err(why) => {
            watcher = start_watch();
            exact_at = now_unix();
            vol.update(|s| s.phase = Phase::Building);
            let progress = |p: Progress| {
                vol.update(|s| s.entries = p.entries);
                changed();
            };
            let mut bw = w;
            bw.progress = Some(&progress);
            let (ix, rep) = build(&vol.root, &bw);
            if rep.cancelled {
                vol.update(|s| s.phase = Phase::Stopped);
                return;
            }
            vol.publish(ix);
            vol.update(|s| {
                s.origin = Some(Origin::Built { why, ms: rep.ms });
                s.build = Some(rep);
            });
            dirty = true;
        }
    }
    let entries = vol.read(Index::len).unwrap_or(0);
    vol.update(|s| {
        s.phase = Phase::Ready;
        s.entries = entries;
    });
    changed();
    let mut last_save = Instant::now();
    if dirty {
        save(vol, &tag, exact_at);
        dirty = false;
    }

    // folders to compare again (path, when, applied at) – see `RECHECK_NEW`
    let mut rechecks: Vec<(PathBuf, Instant, i64)> = Vec::new();
    loop {
        let wait = rechecks
            .iter()
            .map(|r| r.1.saturating_duration_since(Instant::now()))
            .min()
            .unwrap_or(Duration::MAX)
            .min(Duration::from_millis(500));
        vol.pending.wait(wait);
        if vol.pending.stopped() {
            // changes still waiting are not applied (a big one could be cut
            // off): they happened after `exact_at`, the next start finds them
            break;
        }
        if let Some(batch) = vol.pending.take(false) {
            let drained = now_unix();
            let Some(did) = apply_batch(vol, &w, cfg, batch, exact_at, &mut rechecks) else {
                vol.update(|s| s.phase = Phase::Stopped);
                return; // cut off by stopping: never save a partial change
            };
            exact_at = drained;
            if did {
                dirty = true;
                let entries = vol.read(Index::len).unwrap_or(0);
                vol.update(|s| s.entries = entries);
                changed();
            }
        }
        let now = Instant::now();
        if rechecks.iter().any(|r| r.1 <= now) {
            let (due, later): (Vec<_>, Vec<_>) = rechecks.drain(..).partition(|r| r.1 <= now);
            rechecks = later;
            let mut did = false;
            for (dir, _, since) in due {
                let start =
                    vol.read(|ix| ix.find(&dir.to_string_lossy()).filter(|&id| ix.is_dir(id)));
                let Some(Some(start)) = start else { continue };
                let Some(rep) = refresh_in_slices(vol, &w, cfg, Some(start), since) else {
                    vol.update(|s| s.phase = Phase::Stopped);
                    return;
                };
                did |= rep.changed();
            }
            if did {
                dirty = true;
                let entries = vol.read(Index::len).unwrap_or(0);
                vol.update(|s| s.entries = entries);
                changed();
            }
        }
        if maybe_reorder(vol, cfg) {
            changed();
        }
        if dirty && last_save.elapsed() >= cfg.save_every {
            save(vol, &tag, exact_at);
            last_save = Instant::now();
            dirty = false;
        }
    }
    if dirty {
        save(vol, &tag, exact_at);
    }
    vol.update(|s| s.phase = Phase::Stopped);
    drop(watcher);
}

/// From `start` (None: the root) down. None: stopped before the end.
fn refresh_in_slices(
    vol: &Volume,
    w: &Walker,
    cfg: &Config,
    start: Option<u32>,
    since: i64,
) -> Option<RefreshReport> {
    let start = match start {
        Some(s) => s,
        None => vol.read(|ix| ix.roots().first().copied())??,
    };
    let mut r = Refresh::new(start, since);
    while !vol.write(|ix| r.step(ix, w, Some(Instant::now() + cfg.slice)))? {
        std::thread::yield_now(); // let waiting searches in
    }
    let rep = r.report();
    (!rep.cancelled).then_some(rep)
}

/// Some(changed), or None when stopping cut it off.
fn apply_batch(
    vol: &Volume,
    w: &Walker,
    cfg: &Config,
    batch: Batch,
    exact_at: i64,
    rechecks: &mut Vec<(PathBuf, Instant, i64)>,
) -> Option<bool> {
    let mut did = false;
    let applied_at = now_unix();
    if batch.overflow {
        let rep = refresh_in_slices(vol, w, cfg, None, exact_at)?;
        did |= rep.changed();
        vol.update(|s| s.refresh = Some(rep));
    }
    let work = plan(batch.paths);
    let mut next = 0;
    while next < work.len() {
        let changes = vol.write(|ix| {
            let Some(&root) = ix.roots().first() else {
                next = work.len();
                return 0;
            };
            let deadline = Instant::now() + cfg.slice;
            let mut n = 0;
            while next < work.len() {
                let ran = run(ix, root, &work[next], w);
                n += u64::from(ran != Ran::Unchanged);
                if ran == Ran::Added && RECHECK_NEW {
                    if let Some(d) = recheck_target(&work[next]) {
                        if !rechecks.iter().any(|r| r.0 == d) {
                            rechecks.push((d, Instant::now() + RECHECK_AFTER, applied_at));
                        }
                    }
                }
                next += 1;
                if Instant::now() >= deadline {
                    break;
                }
            }
            n
        })?;
        if changes > 0 {
            did = true;
            vol.update(|s| s.live_changes += changes);
        }
        if w.cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return None;
        }
        std::thread::yield_now();
    }
    Some(did)
}

/// Many changes since the last layout: lay out a copy while searches go on,
/// then swap it in (ids change: new generation). True if it happened.
fn maybe_reorder(vol: &Volume, cfg: &Config) -> bool {
    let want = vol
        .read(|ix| ix.unsorted() >= cfg.reorder_min.max(ix.len() / 20) || ix.needs_compact())
        .unwrap_or(false);
    if !want {
        return false;
    }
    // only this thread writes: nothing changes between copy and swap
    let Some((fresh, _)) = vol.read(Index::reordered) else {
        return false;
    };
    vol.publish(fresh);
    true
}

fn save(vol: &Volume, tag: &str, exact_at: i64) {
    let Some(bytes) = vol.read(|ix| ix.to_bytes(tag, exact_at - SAVE_MARGIN)) else {
        return;
    };
    let res = persist::write_atomic(&vol.file, &bytes);
    vol.update(|s| match res {
        Ok(()) => {
            s.save_error = None;
            s.saved = Some(exact_at - SAVE_MARGIN);
        }
        Err(e) => s.save_error = Some(e.to_string()),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{snapshot, tree};
    use explorer_index::Now;
    use std::sync::atomic::AtomicUsize;

    fn wait_for(what: &str, secs: u64, mut ok: impl FnMut() -> bool) {
        let t0 = Instant::now();
        while !ok() {
            assert!(
                t0.elapsed() < Duration::from_secs(secs),
                "timed out waiting for {what}"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Waits until the index equals `want`; on timeout shows the difference.
    fn wait_same(v: &Volume, want: &[(String, bool, u64)], what: &str) {
        let t0 = Instant::now();
        loop {
            let now = current(v);
            if now == want {
                return;
            }
            if t0.elapsed() > Duration::from_secs(10) {
                let missing: Vec<_> = want.iter().filter(|e| !now.contains(e)).collect();
                let extra: Vec<_> = now.iter().filter(|e| !want.contains(e)).collect();
                panic!(
                    "{what}: missing {missing:?}, extra {extra:?}, status {:?}",
                    v.status()
                );
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn fast(dir: &Path, ex: Exclusions) -> Config {
        let mut c = Config::new(dir.to_path_buf(), ex);
        c.quiet = Duration::from_millis(50);
        c.max_wait = Duration::from_millis(200);
        c
    }

    fn fresh(root: &Path, ex: &Exclusions) -> Vec<(String, bool, u64)> {
        snapshot(&build(root, &Walker::new(ex)).0, root)
    }

    fn current(v: &Volume) -> Vec<(String, bool, u64)> {
        v.read(|ix| snapshot(ix, v.root())).unwrap_or_default()
    }

    fn q(text: &str) -> Query {
        Query::parse(
            text,
            Now {
                unix: 0,
                utc_offset: 0,
            },
        )
        .unwrap()
    }

    fn ready(s: &Search) -> &Volume {
        let v = &s.volumes()[0];
        wait_for("ready", 20, || v.status().phase == Phase::Ready);
        v
    }

    #[test]
    fn builds_saves_and_next_time_loads_and_refreshes() {
        let (d, store) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let root = d.path().to_path_buf();
        tree(&root);
        let ex = Exclusions::parse("node_modules", crate::CASE_INSENSITIVE);
        let calls = Arc::new(AtomicUsize::new(0));
        let c = calls.clone();
        let s = Search::start(
            std::slice::from_ref(&root),
            fast(store.path(), ex.clone()),
            move |_| {
                c.fetch_add(1, Ordering::Relaxed);
            },
        );
        let v = ready(&s);
        let st = v.status();
        assert!(matches!(st.origin, Some(Origin::Built { .. })), "{st:?}");
        assert!(st.live, "{st:?}");
        assert_eq!(current(v), fresh(&root, &ex));
        // saved right after the build (searchable before that)
        wait_for("first save", 20, || v.status().saved.is_some());
        assert!(v.index_file().exists());
        let hits = v.search(&q("tief"), None, SortBy::Name, false);
        assert_eq!(hits.ids.len(), 1);
        let p = v.path_of(hits.generation, hits.ids[0]).unwrap();
        assert!(p.ends_with("tief.pdf"), "{p}");
        // scopes: below a folder, a folder above the root, another place
        assert_eq!(
            v.search(&q("txt"), Some(&root.join("a")), SortBy::Name, false)
                .ids
                .len(),
            1
        );
        assert_eq!(
            v.search(&q("tief"), root.parent(), SortBy::Name, false)
                .ids
                .len(),
            1
        );
        assert!(v
            .search(
                &q("tief"),
                Some(Path::new("/nirgends")),
                SortBy::Name,
                false
            )
            .ids
            .is_empty());
        assert!(calls.load(Ordering::Relaxed) > 0);
        s.shutdown();

        // closed: changes on disk
        std::fs::write(root.join("a/offline.txt"), b"neu").unwrap();
        std::fs::remove_dir_all(root.join("a/b")).unwrap();
        let s = Search::start(
            std::slice::from_ref(&root),
            fast(store.path(), ex.clone()),
            |_| {},
        );
        let v = ready(&s);
        let st = v.status();
        assert!(matches!(st.origin, Some(Origin::Loaded { .. })), "{st:?}");
        assert!(st.refresh.is_some_and(|r| r.changed()), "{st:?}");
        assert_eq!(current(v), fresh(&root, &ex));
    }

    #[test]
    fn live_changes_arrive() {
        let (d, store) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let root = d.path().to_path_buf();
        tree(&root);
        let ex = Exclusions::parse("node_modules;*.tmp", crate::CASE_INSENSITIVE);
        let s = Search::start(
            std::slice::from_ref(&root),
            fast(store.path(), ex.clone()),
            |_| {},
        );
        let v = ready(&s);
        std::fs::write(root.join("a/neu.txt"), b"neu").unwrap();
        std::fs::rename(root.join("a/b"), root.join("Leer/b2")).unwrap();
        std::fs::remove_file(root.join("oben.txt")).unwrap();
        std::fs::write(root.join("weg.tmp"), b"ausgeschlossen").unwrap();
        // a burst in one folder (one read instead of a lookup per file)
        std::fs::create_dir(root.join("viele")).unwrap();
        for i in 0..crate::live::BULK * 3 {
            std::fs::write(root.join(format!("viele/f{i}.dat")), b"x").unwrap();
        }
        // a folder with content moved in from outside the root
        let outside = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(outside.path().join("pack/tief")).unwrap();
        std::fs::write(outside.path().join("pack/tief/drin.md"), b"d").unwrap();
        std::fs::rename(outside.path().join("pack"), root.join("pack")).unwrap();
        let want = fresh(&root, &ex);
        wait_same(v, &want, "live changes");
        assert!(v.status().live_changes > 0);
        // in place: size changes
        std::fs::write(root.join("a/1.txt"), b"viel laenger als vorher").unwrap();
        let want = fresh(&root, &ex);
        wait_same(v, &want, "in-place change");
    }

    #[test]
    fn damaged_foreign_or_other_root_index_is_rebuilt() {
        let (d, store) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let root = d.path().to_path_buf();
        tree(&root);
        let ex = Exclusions::default();
        let name = persist::file_name_for(&root.to_string_lossy(), crate::CASE_INSENSITIVE);
        let file = store.path().join(name);
        let tag = explorer_fs::places::volume_tag(&root);
        let other_root = tempfile::tempdir().unwrap();
        let (other, _) = build(other_root.path(), &Walker::new(&ex));
        for (bytes, expect) in [
            (b"kaputt".to_vec(), "beschädigt"),
            (
                build(&root, &Walker::new(&ex)).0.to_bytes("anderes", 0),
                "anderen Laufwerk",
            ),
            (other.to_bytes(&tag, 0), "anderen Ordner"),
        ] {
            std::fs::write(&file, &bytes).unwrap();
            let s = Search::start(
                std::slice::from_ref(&root),
                fast(store.path(), ex.clone()),
                |_| {},
            );
            let v = ready(&s);
            match v.status().origin {
                Some(Origin::Built { why, .. }) => assert!(why.contains(expect), "{why}"),
                o => panic!("{o:?}"),
            }
            assert_eq!(current(v), fresh(&root, &ex));
            drop(s);
            // and replaced by a good one
            assert!(Index::load(&file, Some(&tag)).is_ok());
        }
    }

    #[test]
    fn reorder_changes_generation_and_rescan_finds_everything() {
        let (d, store) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let root = d.path().to_path_buf();
        tree(&root);
        let ex = Exclusions::default();
        let mut cfg = fast(store.path(), ex.clone());
        cfg.reorder_min = 1;
        let s = Search::start(std::slice::from_ref(&root), cfg, |_| {});
        let v = ready(&s);
        let before = v.search(&q("tief"), None, SortBy::Name, false);
        std::fs::write(root.join("zz-neu.txt"), b"z").unwrap();
        let want = fresh(&root, &ex);
        wait_for("change + reorder", 10, || {
            current(v) == want && v.generation() > before.generation
        });
        assert_eq!(v.read(Index::unsorted), Some(0));
        assert_eq!(v.path_of(before.generation, before.ids[0]), None);
        let now = v.search(&q("tief"), None, SortBy::Name, false);
        assert!(v.path_of(now.generation, now.ids[0]).is_some());
        // a full comparison on request (also what lost events trigger)
        let checks = v.status().refresh.map_or(0, |r| r.dirs_checked);
        assert_eq!(checks, 0, "built, never refreshed");
        v.rescan();
        wait_for("rescan", 10, || v.status().refresh.is_some());
        assert!(v.status().refresh.unwrap().dirs_checked >= 7);
        assert_eq!(current(v), want);
    }

    #[test]
    fn stopping_right_away_is_quick_and_clean() {
        let (d, store) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        tree(d.path());
        let t0 = Instant::now();
        let s = Search::start(
            &[d.path().to_path_buf()],
            fast(store.path(), Exclusions::default()),
            |_| {},
        );
        s.shutdown();
        assert!(t0.elapsed() < Duration::from_secs(5));
        // a root that does not exist: no panic, nothing searchable
        let s = Search::start(
            &[d.path().join("gibt-es-nicht")],
            fast(store.path(), Exclusions::default()),
            |_| {},
        );
        let v = &s.volumes()[0];
        wait_for("ready or stopped", 10, || {
            matches!(v.status().phase, Phase::Ready)
        });
        // (not "x": the root's name is the random temp path)
        assert!(v
            .search(&q("tief"), None, SortBy::Name, false)
            .ids
            .is_empty());
    }
}
