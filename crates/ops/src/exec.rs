//! Job execution: copy / move / trash / permanent delete with progress,
//! pause, cancel, conflict questions and an undo-able report.
//!
//! Safety rules (data must never be lost by a failed or cancelled job):
//! - copies go to a temp file next to the target and are renamed into place
//!   only when complete → no half-written file under the real name;
//! - a move across volumes deletes the source only after its copy succeeded
//!   and was flushed to disk;
//! - "Replace" moves the old target to the trash first (so it can be undone);
//! - an error on one item is recorded and the job continues with the next.

use crate::names::{copy_name, unique_name};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant, SystemTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Copy,
    Move,
    Trash,
    DeletePermanent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobSpec {
    pub kind: Kind,
    pub sources: Vec<PathBuf>,
    /// Target folder (copy/move).
    pub dest: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Replace,
    Skip,
    KeepBoth,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub source: PathBuf,
    pub target: PathBuf,
    pub source_is_dir: bool,
    pub target_is_dir: bool,
    pub source_size: u64,
    pub target_size: u64,
    pub source_mtime: Option<SystemTime>,
    pub target_mtime: Option<SystemTime>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Progress {
    pub done_items: u64,
    pub total_items: u64,
    pub done_bytes: u64,
    pub total_bytes: u64,
    pub current: PathBuf,
}

/// What a job did, in order – the basis for undo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Done {
    /// A new item (copy result or folder created while merging).
    Created(PathBuf),
    Moved {
        from: PathBuf,
        to: PathBuf,
    },
    /// Moved to the trash (`at` = time just before, to find it again for restore).
    Trashed {
        original: PathBuf,
        at: SystemTime,
    },
    /// Permanently deleted – cannot be undone.
    Deleted(PathBuf),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub done: Vec<Done>,
    pub errors: Vec<(PathBuf, String)>,
    pub skipped: u64,
    pub cancelled: bool,
}

#[derive(Debug)]
pub enum Event {
    Started { total_items: u64, total_bytes: u64 },
    Progress(Progress),
    Conflict(Conflict),
    Finished(Report),
}

/// Shared control flags (UI thread ↔ worker).
#[derive(Debug, Default, Clone)]
pub struct Control {
    pub cancel: Arc<AtomicBool>,
    pub pause: Arc<AtomicBool>,
}

impl Control {
    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// Blocks while paused; returns false if cancelled meanwhile.
    fn checkpoint(&self) -> bool {
        while self.pause.load(Ordering::Relaxed) && !self.cancelled() {
            std::thread::sleep(Duration::from_millis(50));
        }
        !self.cancelled()
    }
}

/// Handle to a running job; dropping it does NOT cancel (jobs finish in the background).
pub struct JobHandle {
    pub control: Control,
    answers: mpsc::Sender<(Choice, bool)>,
    finished: Arc<AtomicBool>,
}

impl JobHandle {
    pub fn cancel(&self) {
        self.control.cancel.store(true, Ordering::Relaxed);
        let _ = self.answers.send((Choice::Cancel, true)); // unblock a pending conflict question
    }

    pub fn set_paused(&self, paused: bool) {
        self.control.pause.store(paused, Ordering::Relaxed);
    }

    /// Answer to the last `Event::Conflict`; `apply_all` reuses it for later conflicts.
    pub fn answer(&self, choice: Choice, apply_all: bool) {
        let _ = self.answers.send((choice, apply_all));
    }

    /// True once the worker thread is done (after `Event::Finished` was emitted).
    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Acquire)
    }
}

/// Runs a job on a background thread; events go to `sink` (called on that thread).
pub fn spawn(spec: JobSpec, mut sink: impl FnMut(Event) + Send + 'static) -> JobHandle {
    let control = Control::default();
    let (tx, rx) = mpsc::channel::<(Choice, bool)>();
    let ctl = control.clone();
    let finished = Arc::new(AtomicBool::new(false));
    let fin = finished.clone();
    std::thread::Builder::new()
        .name("file-job".into())
        .spawn(move || {
            let sink_cell = std::cell::RefCell::new(&mut sink);
            let mut ask = |c: &Conflict| -> (Choice, bool) {
                (sink_cell.borrow_mut())(Event::Conflict(c.clone()));
                loop {
                    match rx.recv_timeout(Duration::from_millis(200)) {
                        Ok(a) => return a,
                        Err(mpsc::RecvTimeoutError::Timeout) if ctl.cancelled() => {
                            return (Choice::Cancel, true)
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        Err(mpsc::RecvTimeoutError::Disconnected) => return (Choice::Cancel, true),
                    }
                }
            };
            let mut emit = |e: Event| (sink_cell.borrow_mut())(e);
            let report = run(&spec, &ctl, &mut emit, &mut ask);
            (sink_cell.borrow_mut())(Event::Finished(report));
            fin.store(true, Ordering::Release);
        })
        .expect("spawning file job failed");
    JobHandle {
        control,
        answers: tx,
        finished,
    }
}

// ---------------------------------------------------------------- engine

struct Ctx<'a> {
    ctl: &'a Control,
    emit: &'a mut dyn FnMut(Event),
    ask: &'a mut dyn FnMut(&Conflict) -> (Choice, bool),
    remembered: Option<Choice>,
    report: Report,
    progress: Progress,
    last_emit: Instant,
    kind: Kind,
}

impl Ctx<'_> {
    fn tick(&mut self, force: bool) {
        if force || self.last_emit.elapsed() >= Duration::from_millis(50) {
            self.last_emit = Instant::now();
            (self.emit)(Event::Progress(self.progress.clone()));
        }
    }

    fn error(&mut self, path: &Path, msg: impl Into<String>) {
        self.report.errors.push((path.to_path_buf(), msg.into()));
    }

    fn decide(&mut self, c: Conflict) -> Choice {
        if let Some(r) = self.remembered {
            return r;
        }
        let (choice, all) = (self.ask)(&c);
        if all {
            self.remembered = Some(choice);
        }
        if choice == Choice::Cancel {
            self.ctl.cancel.store(true, Ordering::Relaxed);
        }
        choice
    }
}

/// Synchronous engine (used by `spawn`; tests call it directly with scripted answers).
pub fn run(
    spec: &JobSpec,
    ctl: &Control,
    emit: &mut dyn FnMut(Event),
    ask: &mut dyn FnMut(&Conflict) -> (Choice, bool),
) -> Report {
    let (total_items, total_bytes) = measure(&spec.sources, ctl);
    emit(Event::Started {
        total_items,
        total_bytes,
    });
    let mut cx = Ctx {
        ctl,
        emit,
        ask,
        remembered: None,
        report: Report::default(),
        progress: Progress {
            total_items,
            total_bytes,
            ..Default::default()
        },
        // first progress event right away (immediate feedback), then every 50 ms
        last_emit: Instant::now()
            .checked_sub(Duration::from_secs(1))
            .unwrap_or_else(Instant::now),
        kind: spec.kind,
    };
    match spec.kind {
        Kind::Copy | Kind::Move => match &spec.dest {
            Some(dest) => transfer_all(&mut cx, &spec.sources, dest),
            None => cx.error(Path::new(""), "Kein Ziel angegeben."),
        },
        Kind::Trash => trash_all(&mut cx, &spec.sources),
        Kind::DeletePermanent => delete_all(&mut cx, &spec.sources),
    }
    cx.report.cancelled = ctl.cancelled();
    cx.tick(true);
    cx.report
}

/// Counts items and bytes for the progress bar (cancellable).
fn measure(sources: &[PathBuf], ctl: &Control) -> (u64, u64) {
    fn walk(p: &Path, ctl: &Control, items: &mut u64, bytes: &mut u64) {
        if ctl.cancelled() {
            return;
        }
        let Ok(m) = fs::symlink_metadata(p) else {
            return;
        };
        *items += 1;
        if m.is_dir() {
            if let Ok(rd) = fs::read_dir(p) {
                for e in rd.flatten() {
                    walk(&e.path(), ctl, items, bytes);
                }
            }
        } else {
            *bytes += m.len();
        }
    }
    let (mut items, mut bytes) = (0, 0);
    for s in sources {
        walk(s, ctl, &mut items, &mut bytes);
    }
    (items, bytes)
}

fn is_inside(child: &Path, parent: &Path) -> bool {
    match (fs::canonicalize(child), fs::canonicalize(parent)) {
        (Ok(c), Ok(p)) => c.starts_with(p),
        _ => false,
    }
}

fn exists(p: &Path) -> bool {
    fs::symlink_metadata(p).is_ok()
}

fn is_cross_device(e: &io::Error) -> bool {
    // EXDEV (18) on Unix, ERROR_NOT_SAME_DEVICE (17) on Windows.
    e.kind() == io::ErrorKind::CrossesDevices
        || matches!(e.raw_os_error(), Some(18) if cfg!(unix))
        || matches!(e.raw_os_error(), Some(17) if cfg!(windows))
}

fn msg(e: &io::Error) -> String {
    match e.kind() {
        io::ErrorKind::NotFound => explorer_i18n::tr!("Not found").into(),
        io::ErrorKind::PermissionDenied => explorer_i18n::tr!("Access denied").into(),
        io::ErrorKind::StorageFull => explorer_i18n::tr!("Not enough disk space").into(),
        io::ErrorKind::AlreadyExists => explorer_i18n::tr!("Already exists").into(),
        io::ErrorKind::Interrupted => explorer_i18n::tr!("Cancelled.").into(),
        _ if e.raw_os_error() == Some(32) && cfg!(windows) => {
            explorer_i18n::tr!("The file is being used by another process").into()
        }
        _ if e.raw_os_error() == Some(28) && cfg!(unix) => {
            explorer_i18n::tr!("Not enough disk space").into()
        }
        _ => e.to_string(),
    }
}

fn transfer_all(cx: &mut Ctx, sources: &[PathBuf], dest: &Path) {
    for src in sources {
        if !cx.ctl.checkpoint() {
            return;
        }
        let Some(name) = src.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            cx.error(src, explorer_i18n::tr!("Invalid path"));
            continue;
        };
        let src_is_dir = fs::symlink_metadata(src)
            .map(|m| m.is_dir())
            .unwrap_or(false);
        if src_is_dir && (is_inside(dest, src)) {
            cx.error(
                src,
                explorer_i18n::tr!("The destination folder is a subfolder of the source folder."),
            );
            continue;
        }
        let same_folder = src
            .parent()
            .map(|p| fs::canonicalize(p).ok() == fs::canonicalize(dest).ok())
            .unwrap_or(false);
        let target = if same_folder {
            match cx.kind {
                // moving into its own folder is a no-op
                Kind::Move => {
                    cx.report.skipped += 1;
                    continue;
                }
                // copy in place → "Name - Kopie"
                _ => dest.join(copy_name(&name, src_is_dir, |n| exists(&dest.join(n)))),
            }
        } else {
            dest.join(&name)
        };
        transfer(cx, src, &target);
    }
}

/// Copies or moves `src` to `target` (recursively for folders).
fn transfer(cx: &mut Ctx, src: &Path, target: &Path) {
    if !cx.ctl.checkpoint() {
        return;
    }
    let meta = match fs::symlink_metadata(src) {
        Ok(m) => m,
        Err(e) => return cx.error(src, msg(&e)),
    };
    let src_is_dir = meta.is_dir();
    cx.progress.current = src.to_path_buf();
    let mut target = target.to_path_buf();
    let mut merge = false;
    if let Ok(tm) = fs::symlink_metadata(&target) {
        let c = Conflict {
            source: src.to_path_buf(),
            target: target.clone(),
            source_is_dir: src_is_dir,
            target_is_dir: tm.is_dir(),
            source_size: meta.len(),
            target_size: tm.len(),
            source_mtime: meta.modified().ok(),
            target_mtime: tm.modified().ok(),
        };
        match cx.decide(c) {
            Choice::Cancel => return,
            Choice::Skip => {
                let (items, bytes) = measure(&[src.to_path_buf()], cx.ctl);
                cx.report.skipped += items;
                cx.progress.done_items += items;
                cx.progress.done_bytes += bytes;
                return cx.tick(false);
            }
            Choice::KeepBoth => {
                let dir = target.parent().unwrap_or(Path::new("")).to_path_buf();
                let name = target
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                target = dir.join(unique_name(&name, src_is_dir, |n| exists(&dir.join(n))));
            }
            Choice::Replace => {
                if src_is_dir && tm.is_dir() {
                    merge = true; // folders are merged, conflicts inside are asked one by one
                } else if src_is_dir != tm.is_dir() {
                    return cx.error(
                        src,
                        explorer_i18n::tr!("A folder and a file can't replace each other."),
                    );
                } else {
                    // Keep the old file recoverable: move it to the trash before replacing.
                    let at = SystemTime::now();
                    if let Err(e) = trash::delete(&target) {
                        return cx.error(
                            &target,
                            explorer_i18n::tr!(
                                "Couldn't move the existing item to the Recycle Bin: {0}",
                                e
                            ),
                        );
                    }
                    cx.report.done.push(Done::Trashed {
                        original: target.clone(),
                        at,
                    });
                }
            }
        }
    }

    if src_is_dir {
        if cx.kind == Kind::Move && !merge {
            match fs::rename(src, &target) {
                Ok(()) => {
                    let (items, bytes) = measure(&[target.clone()], cx.ctl);
                    cx.progress.done_items += items;
                    cx.progress.done_bytes += bytes;
                    cx.report.done.push(Done::Moved {
                        from: src.to_path_buf(),
                        to: target,
                    });
                    return cx.tick(false);
                }
                Err(e) if is_cross_device(&e) => {} // fall through: copy + delete
                Err(e) => return cx.error(src, msg(&e)),
            }
        }
        if !merge {
            if let Err(e) = fs::create_dir(&target) {
                return cx.error(&target, msg(&e));
            }
        }
        cx.progress.done_items += 1;
        let children: Vec<PathBuf> = match fs::read_dir(src) {
            Ok(rd) => rd.flatten().map(|e| e.path()).collect(),
            Err(e) => return cx.error(src, msg(&e)),
        };
        let mark = cx.report.done.len();
        let (errors_before, skipped_before) = (cx.report.errors.len(), cx.report.skipped);
        for child in children {
            if cx.ctl.cancelled() {
                break;
            }
            let name = child
                .file_name()
                .map(|n| n.to_os_string())
                .unwrap_or_default();
            transfer(cx, &child, &target.join(&name));
        }
        let clean = cx.report.errors.len() == errors_before
            && cx.report.skipped == skipped_before
            && !cx.ctl.cancelled();
        match (cx.kind, merge) {
            // A freshly created copy is one undo item (also when only partly copied).
            (Kind::Copy, false) => {
                cx.report.done.truncate(mark);
                cx.report.done.push(Done::Created(target.clone()));
            }
            // Merged into an existing folder: every new item is recorded on its own.
            (Kind::Copy, true) => {}
            (Kind::Move, _) if clean => match fs::remove_dir(src) {
                Ok(()) if !merge => {
                    cx.report.done.truncate(mark);
                    cx.report.done.push(Done::Moved {
                        from: src.to_path_buf(),
                        to: target.clone(),
                    });
                }
                Ok(()) => {}
                Err(e) => cx.error(src, msg(&e)),
            },
            // Partly moved (errors/cancel): the new folder must be undone *after* its
            // children went back → record it before them (undo runs in reverse order).
            (Kind::Move, false) => cx.report.done.insert(mark, Done::Created(target.clone())),
            _ => {}
        }
        return cx.tick(false);
    }

    // ---- file or symlink
    if cx.kind == Kind::Move {
        match fs::rename(src, &target) {
            Ok(()) => {
                cx.progress.done_items += 1;
                cx.progress.done_bytes += meta.len();
                cx.report.done.push(Done::Moved {
                    from: src.to_path_buf(),
                    to: target,
                });
                return cx.tick(false);
            }
            Err(e) if is_cross_device(&e) => {}
            Err(e) => return cx.error(src, msg(&e)),
        }
    }
    match copy_one(cx, src, &target, &meta) {
        Ok(()) => {
            cx.progress.done_items += 1;
            if cx.kind == Kind::Move {
                // cross-volume move: source goes only after the copy is complete
                if let Err(e) = fs::remove_file(src) {
                    cx.error(
                        src,
                        explorer_i18n::tr!("Copied, but the source was not deleted: {0}", msg(&e)),
                    );
                    cx.report.done.push(Done::Created(target));
                } else {
                    cx.report.done.push(Done::Moved {
                        from: src.to_path_buf(),
                        to: target,
                    });
                }
            } else {
                cx.report.done.push(Done::Created(target));
            }
        }
        Err(e) if cx.ctl.cancelled() => {
            let _ = e;
        }
        Err(e) => cx.error(src, msg(&e)),
    }
    cx.tick(false);
}

fn temp_name(target: &Path) -> PathBuf {
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    target.with_file_name(format!(".{name}.e2part"))
}

/// Copies one file via a temp file and renames it into place.
fn copy_one(cx: &mut Ctx, src: &Path, target: &Path, meta: &fs::Metadata) -> io::Result<()> {
    if meta.file_type().is_symlink() {
        #[cfg(unix)]
        {
            let link = fs::read_link(src)?;
            return std::os::unix::fs::symlink(link, target);
        }
    }
    let tmp = temp_name(target);
    let res = copy_data(cx, src, &tmp, meta)
        .and_then(|()| {
            // A move deletes the source afterwards → the copy must be on disk first.
            // Plain copies skip the flush (the source stays; a flush per file would
            // make copying many small files several times slower).
            if cx.kind == Kind::Move {
                fs::OpenOptions::new().write(true).open(&tmp)?.sync_all()?;
            }
            Ok(())
        })
        .and_then(|()| fs::rename(&tmp, target));
    if res.is_err() {
        let _ = fs::remove_file(&tmp); // never leave partial data behind
    }
    res
}

#[cfg(not(windows))]
fn copy_data(cx: &mut Ctx, src: &Path, tmp: &Path, meta: &fs::Metadata) -> io::Result<()> {
    use std::io::{Read, Write};
    let mut input = fs::File::open(src)?;
    let mut output = fs::File::create(tmp)?;
    let mut buf = vec![0u8; 1 << 20];
    loop {
        if !cx.ctl.checkpoint() {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "abgebrochen"));
        }
        let n = input.read(&mut buf)?;
        if n == 0 {
            break;
        }
        output.write_all(&buf[..n])?;
        cx.progress.done_bytes += n as u64;
        cx.tick(false);
    }
    drop(output);
    fs::set_permissions(tmp, meta.permissions())?;
    if let Ok(m) = meta.modified() {
        let _ = fs::File::options()
            .write(true)
            .open(tmp)
            .and_then(|f| f.set_modified(m));
    }
    Ok(())
}

#[cfg(windows)]
fn copy_data(cx: &mut Ctx, src: &Path, tmp: &Path, _meta: &fs::Metadata) -> io::Result<()> {
    crate::win_copy::copy_file(src, tmp, &mut |copied| {
        if !cx.ctl.checkpoint() {
            return false;
        }
        cx.progress.done_bytes += copied;
        cx.tick(false);
        true
    })
}

fn trash_all(cx: &mut Ctx, sources: &[PathBuf]) {
    for src in sources {
        if !cx.ctl.checkpoint() {
            return;
        }
        cx.progress.current = src.clone();
        let (items, bytes) = measure(std::slice::from_ref(src), cx.ctl);
        let at = SystemTime::now();
        match trash::delete(src) {
            Ok(()) => cx.report.done.push(Done::Trashed {
                original: src.clone(),
                at,
            }),
            Err(e) => cx.error(src, explorer_i18n::tr!("Recycle Bin: {0}", e)),
        }
        cx.progress.done_items += items;
        cx.progress.done_bytes += bytes;
        cx.tick(false);
    }
}

fn delete_all(cx: &mut Ctx, sources: &[PathBuf]) {
    for src in sources {
        if !cx.ctl.checkpoint() {
            return;
        }
        cx.progress.current = src.clone();
        let (items, bytes) = measure(std::slice::from_ref(src), cx.ctl);
        let res = match fs::symlink_metadata(src) {
            Ok(m) if m.is_dir() => fs::remove_dir_all(src),
            Ok(_) => fs::remove_file(src),
            Err(e) => Err(e),
        };
        match res {
            Ok(()) => cx.report.done.push(Done::Deleted(src.clone())),
            Err(e) => cx.error(src, msg(&e)),
        }
        cx.progress.done_items += items;
        cx.progress.done_bytes += bytes;
        cx.tick(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cancelled_copy_is_not_a_german_word() {
        let e = io::Error::new(io::ErrorKind::Interrupted, "abgebrochen");
        assert_eq!(msg(&e), "Cancelled.");
    }
}
