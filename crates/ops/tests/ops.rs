//! Integration tests on real file systems. Data safety is the point here:
//! every test also checks that nothing was lost or left half-written.

use explorer_ops::exec::{run, Choice, Conflict, Control, Done, Event, JobSpec, Kind, Report};
use explorer_ops::simple::{new_folder, rename};
use explorer_ops::undo::{entry_from_report, undo};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

/// Tests that use the (process-global) trash run one at a time.
static TRASH_LOCK: Mutex<()> = Mutex::new(());

fn tmp() -> tempfile::TempDir {
    // Under the home dir so the freedesktop trash can be used (same file system).
    let base = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join(".explorer2-ops-tests");
    fs::create_dir_all(&base).unwrap();
    tempfile::tempdir_in(base).unwrap()
}

fn write(p: &Path, content: &[u8]) {
    if let Some(d) = p.parent() {
        fs::create_dir_all(d).unwrap();
    }
    fs::write(p, content).unwrap();
}

fn read(p: &Path) -> Vec<u8> {
    fs::read(p).unwrap()
}

/// All files below `root` with content, relative paths (sorted) – to compare trees.
fn tree(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(base: &Path, p: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for e in fs::read_dir(p).unwrap().flatten() {
            let path = e.path();
            let rel = path
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if path.is_dir() {
                out.push((format!("{rel}/"), Vec::new()));
                walk(base, &path, out);
            } else {
                out.push((rel, fs::read(&path).unwrap()));
            }
        }
    }
    let mut v = Vec::new();
    walk(root, root, &mut v);
    v.sort();
    v
}

fn no_partials(root: &Path) -> bool {
    tree(root).iter().all(|(n, _)| !n.ends_with(".e2part"))
}

fn job(kind: Kind, sources: &[&Path], dest: Option<&Path>) -> JobSpec {
    JobSpec {
        kind,
        sources: sources.iter().map(|p| p.to_path_buf()).collect(),
        dest: dest.map(Path::to_path_buf),
    }
}

/// Runs a job synchronously; conflicts are answered from `answers` (in order).
fn exec(spec: &JobSpec, answers: &[(Choice, bool)]) -> (Report, usize) {
    let ctl = Control::default();
    let mut asked = 0;
    let mut answers = answers.iter().copied();
    let report = run(spec, &ctl, &mut |_e: Event| {}, &mut |_c: &Conflict| {
        asked += 1;
        answers.next().expect("unexpected conflict question")
    });
    (report, asked)
}

// ---------------------------------------------------------------- copy

#[test]
fn copy_file_keeps_content_and_mtime() {
    let d = tmp();
    let (src, dst) = (d.path().join("src/a.txt"), d.path().join("dst"));
    write(&src, b"hello world");
    fs::create_dir(&dst).unwrap();
    let old = SystemTime::now() - Duration::from_secs(3600 * 24 * 30);
    fs::File::options()
        .write(true)
        .open(&src)
        .unwrap()
        .set_modified(old)
        .unwrap();
    let (r, asked) = exec(&job(Kind::Copy, &[&src], Some(&dst)), &[]);
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert_eq!(asked, 0);
    assert_eq!(read(&dst.join("a.txt")), b"hello world");
    assert_eq!(read(&src), b"hello world", "source untouched");
    let m = fs::metadata(dst.join("a.txt")).unwrap().modified().unwrap();
    assert!(
        m.duration_since(old).unwrap_or_default() < Duration::from_secs(2),
        "mtime preserved"
    );
    assert_eq!(r.done, vec![Done::Created(dst.join("a.txt"))]);
    assert!(no_partials(d.path()));
}

#[test]
fn copy_tree_is_identical_and_one_undo_item() {
    let d = tmp();
    let src = d.path().join("Projekt");
    write(&src.join("a.txt"), b"a");
    write(&src.join("sub/b.bin"), &vec![7u8; 3 << 20]); // 3 MiB: several chunks
    write(&src.join("sub/deep/Ärger 日本.md"), b"unicode");
    fs::create_dir_all(src.join("empty")).unwrap();
    let dst = d.path().join("dst");
    fs::create_dir(&dst).unwrap();
    let (r, _) = exec(&job(Kind::Copy, &[&src], Some(&dst)), &[]);
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert_eq!(tree(&dst.join("Projekt")), tree(&src));
    assert_eq!(
        r.done,
        vec![Done::Created(dst.join("Projekt"))],
        "a new folder is one undo item"
    );
}

#[test]
fn copy_into_same_folder_creates_kopie_names() {
    let d = tmp();
    let src = d.path().join("Bericht.docx");
    write(&src, b"x");
    exec(&job(Kind::Copy, &[&src], Some(d.path())), &[]);
    exec(&job(Kind::Copy, &[&src], Some(d.path())), &[]);
    assert!(d.path().join("Bericht - Kopie.docx").is_file());
    assert!(d.path().join("Bericht - Kopie (2).docx").is_file());
}

#[test]
fn conflict_skip_keep_both_and_apply_all() {
    let d = tmp();
    let (src, dst) = (d.path().join("src"), d.path().join("dst"));
    for n in ["a.txt", "b.txt", "c.txt"] {
        write(&src.join(n), b"new");
        write(&dst.join(n), b"old");
    }
    // Skip: target untouched
    let (r, asked) = exec(
        &job(Kind::Copy, &[&src.join("a.txt")], Some(&dst)),
        &[(Choice::Skip, false)],
    );
    assert_eq!((asked, r.skipped), (1, 1));
    assert_eq!(read(&dst.join("a.txt")), b"old");
    // Keep both: "a (2).txt"
    exec(
        &job(Kind::Copy, &[&src.join("a.txt")], Some(&dst)),
        &[(Choice::KeepBoth, false)],
    );
    assert_eq!(read(&dst.join("a (2).txt")), b"new");
    assert_eq!(read(&dst.join("a.txt")), b"old");
    // apply_all: asked once for three conflicts
    let (_, asked) = exec(
        &job(
            Kind::Copy,
            &[&src.join("a.txt"), &src.join("b.txt"), &src.join("c.txt")],
            Some(&dst),
        ),
        &[(Choice::KeepBoth, true)],
    );
    assert_eq!(asked, 1);
    assert!(dst.join("b (2).txt").is_file() && dst.join("c (2).txt").is_file());
}

#[test]
fn conflict_replace_moves_old_to_trash_and_undo_restores_it() {
    let _g = TRASH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let d = tmp();
    let (src, dst) = (d.path().join("src/a.txt"), d.path().join("dst"));
    write(&src, b"new");
    write(&dst.join("a.txt"), b"old");
    let (r, _) = exec(
        &job(Kind::Copy, &[&src], Some(&dst)),
        &[(Choice::Replace, false)],
    );
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert_eq!(read(&dst.join("a.txt")), b"new");
    assert!(
        matches!(r.done[0], Done::Trashed { .. }),
        "old file went to the trash first: {:?}",
        r.done
    );
    let e = entry_from_report("Kopieren", &r, None).unwrap();
    assert_eq!(undo(&e), Vec::<String>::new());
    assert_eq!(
        read(&dst.join("a.txt")),
        b"old",
        "undo of replace brings the old file back"
    );
}

#[test]
fn cancel_choice_stops_the_job() {
    let d = tmp();
    let (src, dst) = (d.path().join("src"), d.path().join("dst"));
    write(&src.join("a.txt"), b"1");
    write(&src.join("b.txt"), b"2");
    write(&dst.join("a.txt"), b"old");
    write(&dst.join("b.txt"), b"old");
    let (r, asked) = exec(
        &job(
            Kind::Copy,
            &[&src.join("a.txt"), &src.join("b.txt")],
            Some(&dst),
        ),
        &[(Choice::Cancel, false)],
    );
    assert!(r.cancelled);
    assert_eq!(asked, 1);
    assert_eq!(read(&dst.join("b.txt")), b"old");
}

#[test]
fn replace_on_folders_merges_them() {
    let d = tmp();
    let (src, dst) = (d.path().join("src/Fotos"), d.path().join("dst/Fotos"));
    write(&src.join("neu.jpg"), b"n");
    write(&src.join("gleich.jpg"), b"src");
    write(&dst.join("alt.jpg"), b"a");
    write(&dst.join("gleich.jpg"), b"dst");
    // 1st question: folder "Fotos" (→ Replace = merge), 2nd: file "gleich.jpg" (→ Skip)
    let (r, asked) = exec(
        &job(Kind::Copy, &[&src], Some(&d.path().join("dst"))),
        &[(Choice::Replace, false), (Choice::Skip, false)],
    );
    assert_eq!(asked, 2);
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert_eq!(read(&dst.join("alt.jpg")), b"a");
    assert_eq!(read(&dst.join("neu.jpg")), b"n");
    assert_eq!(read(&dst.join("gleich.jpg")), b"dst");
    assert_eq!(
        r.done,
        vec![Done::Created(dst.join("neu.jpg"))],
        "merge records items individually"
    );
}

#[test]
fn missing_source_is_reported_and_others_continue() {
    let d = tmp();
    let ok = d.path().join("ok.txt");
    write(&ok, b"ok");
    let dst = d.path().join("dst");
    fs::create_dir(&dst).unwrap();
    let (r, _) = exec(
        &job(
            Kind::Copy,
            &[&d.path().join("missing.txt"), &ok],
            Some(&dst),
        ),
        &[],
    );
    assert_eq!(r.errors.len(), 1);
    assert!(dst.join("ok.txt").is_file());
}

#[test]
fn cancel_during_large_copy_leaves_nothing_behind() {
    let d = tmp();
    let src = d.path().join("big.bin");
    write(&src, &vec![1u8; 16 << 20]); // 16 MiB
    let dst = d.path().join("dst");
    fs::create_dir(&dst).unwrap();
    let ctl = Control::default();
    let cancel = ctl.cancel.clone();
    let report = run(
        &job(Kind::Copy, &[&src], Some(&dst)),
        &ctl,
        &mut |e| {
            if let Event::Progress(_) = e {
                cancel.store(true, Ordering::Relaxed); // cancel once copying has started
            }
        },
        &mut |_| unreachable!(),
    );
    assert!(report.cancelled);
    assert!(!dst.join("big.bin").exists(), "no partial target");
    assert!(no_partials(d.path()), "no temp file left");
    assert_eq!(fs::metadata(&src).unwrap().len(), 16 << 20, "source intact");
}

// ---------------------------------------------------------------- move

#[test]
fn move_file_and_folder_and_undo() {
    let d = tmp();
    let f = d.path().join("a.txt");
    let dir = d.path().join("Ordner");
    write(&f, b"a");
    write(&dir.join("x/y.txt"), b"y");
    let before = tree(&dir);
    let dst = d.path().join("dst");
    fs::create_dir(&dst).unwrap();
    let (r, _) = exec(&job(Kind::Move, &[&f, &dir], Some(&dst)), &[]);
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert!(!f.exists() && !dir.exists());
    assert_eq!(read(&dst.join("a.txt")), b"a");
    assert_eq!(tree(&dst.join("Ordner")), before);
    assert_eq!(r.done.len(), 2, "one entry per moved item: {:?}", r.done);
    let e = entry_from_report("Verschieben", &r, None).unwrap();
    assert_eq!(undo(&e), Vec::<String>::new());
    assert_eq!(read(&f), b"a");
    assert_eq!(tree(&dir), before);
    assert!(!dst.join("a.txt").exists());
}

#[test]
fn move_into_own_subfolder_is_rejected() {
    let d = tmp();
    let dir = d.path().join("A");
    write(&dir.join("B/c.txt"), b"c");
    let before = tree(d.path());
    let (r, _) = exec(&job(Kind::Move, &[&dir], Some(&dir.join("B"))), &[]);
    assert_eq!(r.errors.len(), 1);
    assert!(r.errors[0].1.contains("Unterordner"));
    assert_eq!(tree(d.path()), before, "nothing changed");
    let (r, _) = exec(&job(Kind::Copy, &[&dir], Some(&dir)), &[]);
    assert_eq!(r.errors.len(), 1, "copy into itself rejected too");
}

#[test]
fn move_into_same_folder_is_a_noop() {
    let d = tmp();
    let f = d.path().join("a.txt");
    write(&f, b"a");
    let (r, _) = exec(&job(Kind::Move, &[&f], Some(d.path())), &[]);
    assert_eq!((r.skipped, r.done.len()), (1, 0));
    assert!(f.exists());
}

// ---------------------------------------------------------------- delete / trash

#[test]
fn trash_and_undo_restore() {
    let _g = TRASH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let d = tmp();
    let f = d.path().join("weg.txt");
    let dir = d.path().join("WegOrdner");
    write(&f, b"bitte zurueck");
    write(&dir.join("inner.txt"), b"i");
    let (r, _) = exec(&job(Kind::Trash, &[&f, &dir], None), &[]);
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert!(!f.exists() && !dir.exists());
    let e = entry_from_report("Löschen", &r, None).unwrap();
    assert!(!e.incomplete);
    assert_eq!(undo(&e), Vec::<String>::new());
    assert_eq!(read(&f), b"bitte zurueck");
    assert_eq!(read(&dir.join("inner.txt")), b"i");
}

#[test]
fn undo_copy_moves_copy_to_trash() {
    let _g = TRASH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let d = tmp();
    let src = d.path().join("a.txt");
    write(&src, b"a");
    let dst = d.path().join("dst");
    fs::create_dir(&dst).unwrap();
    let (r, _) = exec(&job(Kind::Copy, &[&src], Some(&dst)), &[]);
    let e = entry_from_report("Kopieren", &r, None).unwrap();
    assert_eq!(undo(&e), Vec::<String>::new());
    assert!(!dst.join("a.txt").exists());
    assert!(src.exists(), "original untouched");
}

#[test]
fn permanent_delete_is_marked_not_undoable() {
    let d = tmp();
    let f = d.path().join("a.txt");
    write(&f, b"a");
    let (r, _) = exec(&job(Kind::DeletePermanent, &[&f], None), &[]);
    assert!(!f.exists());
    assert_eq!(r.done, vec![Done::Deleted(f)]);
    assert!(
        entry_from_report("Löschen", &r, None).is_none(),
        "nothing to revert"
    );
}

// ---------------------------------------------------------------- rename / new folder

#[test]
fn rename_rules() {
    let d = tmp();
    let a = d.path().join("a.txt");
    write(&a, b"a");
    write(&d.path().join("b.txt"), b"b");
    assert!(rename(&a, "b.txt").unwrap_err().contains("bereits"));
    assert!(rename(&a, "").is_err());
    let c = rename(&a, "c.txt").unwrap();
    assert_eq!(read(&c), b"a");
    let upper = rename(&c, "C.txt").unwrap(); // case-only change
    assert!(upper.ends_with("C.txt"));
    assert_eq!(
        rename(&upper, "C.txt").unwrap(),
        upper,
        "same name is a no-op"
    );
}

#[test]
fn new_folder_names_are_unique() {
    let d = tmp();
    assert!(new_folder(d.path()).unwrap().ends_with("Neuer Ordner"));
    assert!(new_folder(d.path()).unwrap().ends_with("Neuer Ordner (2)"));
}

// ---------------------------------------------------------------- async handle

#[test]
fn spawned_job_asks_and_waits_for_answer_and_can_pause() {
    use std::sync::mpsc;
    let d = tmp();
    let (src, dst) = (d.path().join("src/a.txt"), d.path().join("dst"));
    write(&src, b"new");
    write(&dst.join("a.txt"), b"old");
    let (tx, rx) = mpsc::channel();
    let h = explorer_ops::spawn(job(Kind::Copy, &[&src], Some(&dst)), move |e| {
        let _ = tx.send(e);
    });
    // the conflict question arrives; the job waits for the answer
    let conflict = loop {
        match rx.recv_timeout(Duration::from_secs(10)).expect("no event") {
            Event::Conflict(c) => break c,
            _ => continue,
        }
    };
    assert_eq!(conflict.target, dst.join("a.txt"));
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(
        read(&dst.join("a.txt")),
        b"old",
        "nothing happens before the answer"
    );
    h.set_paused(true);
    h.answer(Choice::KeepBoth, false);
    // paused: no Finished within 300 ms
    let early = rx.recv_timeout(Duration::from_millis(300));
    assert!(
        !matches!(early, Ok(Event::Finished(_))),
        "must not finish while paused"
    );
    h.set_paused(false);
    let report = loop {
        match rx.recv_timeout(Duration::from_secs(10)).expect("no finish") {
            Event::Finished(r) => break r,
            _ => continue,
        }
    };
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert_eq!(read(&dst.join("a (2).txt")), b"new");
    for _ in 0..200 {
        if h.is_finished() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(h.is_finished());
}

// ---------------------------------------------------------------- platform specific

/// Windows: a file opened without sharing cannot be copied → clear error, others continue.
#[cfg(windows)]
#[test]
fn locked_file_is_reported() {
    use std::os::windows::fs::OpenOptionsExt;
    let d = tmp();
    let locked = d.path().join("locked.txt");
    let ok = d.path().join("ok.txt");
    write(&locked, b"l");
    write(&ok, b"o");
    let _hold = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(&locked)
        .unwrap();
    let dst = d.path().join("dst");
    fs::create_dir(&dst).unwrap();
    let (r, _) = exec(&job(Kind::Copy, &[&locked, &ok], Some(&dst)), &[]);
    assert_eq!(r.errors.len(), 1, "{:?}", r.errors);
    assert!(r.errors[0].1.contains("anderen Prozess"), "{:?}", r.errors);
    assert!(dst.join("ok.txt").is_file());
    assert!(!dst.join("locked.txt").exists() && no_partials(&dst));
}

/// Disk full: run in CI against a tiny tmpfs (`EXPLORER2_SMALL_FS=/mnt/small`).
#[test]
#[ignore = "needs a small file system, see CI"]
fn disk_full_reports_error_and_leaves_no_partial_file() {
    let Some(small) = std::env::var_os("EXPLORER2_SMALL_FS").map(PathBuf::from) else {
        return;
    };
    let d = tmp();
    let src = d.path().join("big.bin");
    write(&src, &vec![9u8; 4 << 20]); // 4 MiB into a 1 MiB file system
    let dst = small.join(format!("t{}", std::process::id()));
    fs::create_dir_all(&dst).unwrap();
    let (r, _) = exec(&job(Kind::Copy, &[&src], Some(&dst)), &[]);
    assert_eq!(r.errors.len(), 1, "{:?}", r.errors);
    assert!(r.errors[0].1.contains("Speicherplatz"), "{:?}", r.errors);
    assert!(!dst.join("big.bin").exists());
    assert!(no_partials(&dst));
    let _ = fs::remove_dir_all(&dst);
}

// ---------------------------------------------------------------- recycle bin view

#[test]
fn trash_view_lists_restores_and_purges() {
    use explorer_ops::trashview;
    let _g = TRASH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let d = tmp();
    let f = d.path().join("papierkorb-test.txt");
    write(&f, b"bitte wiederherstellen");
    let (r, _) = exec(&job(Kind::Trash, &[&f], None), &[]);
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let find = || {
        trashview::list().unwrap().into_iter().find(|e| {
            e.real_name == "papierkorb-test.txt" && same_dir(&e.original_parent, d.path())
        })
    };
    let e = find().expect("trashed file listed with its original folder");
    assert_eq!(e.entry.size, 22);
    assert!(e.entry.modified.is_some(), "deletion time shown");
    trashview::restore(std::slice::from_ref(&e)).unwrap();
    assert_eq!(
        read(&f),
        b"bitte wiederherstellen",
        "restored under its real name"
    );
    assert!(find().is_none());

    // restore refuses to overwrite a file that exists again
    exec(&job(Kind::Trash, &[&f], None), &[]);
    write(&f, b"neu");
    let e = find().unwrap();
    assert!(trashview::restore(std::slice::from_ref(&e))
        .unwrap_err()
        .contains("existiert bereits"));
    assert_eq!(read(&f), b"neu", "nothing overwritten");
    trashview::purge(std::slice::from_ref(&e)).unwrap();
    assert!(find().is_none(), "purged for good");
}

/// Original folders may be reported as long names while temp paths use 8.3 names (Windows).
fn same_dir(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}
