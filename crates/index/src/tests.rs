use super::*;

const NOW: Now = Now {
    unix: 1_793_883_600, // 2026-11-05 13:00 UTC
    utc_offset: 3600,
};
const DAY: i64 = 86_400;

fn file(size: u64, mtime: i64) -> Meta {
    Meta {
        is_dir: false,
        size,
        mtime,
    }
}

fn dir() -> Meta {
    Meta {
        is_dir: true,
        size: 0,
        mtime: NOW.unix - 10 * DAY,
    }
}

/// C:\Users\Boss\{Bilder\{Urlaub.JPG, katze.png}, Docs\{Bericht 2026.pdf, notes.txt}, Ünïcode.TXT}
fn sample() -> (Index, Vec<u32>) {
    let mut ix = Index::new(true);
    let c = ix.add_root(r"C:\", dir());
    let users = ix.add(c, "Users", dir());
    let boss = ix.add(users, "Boss", dir());
    let bilder = ix.add(boss, "Bilder", dir());
    let urlaub = ix.add(
        bilder,
        "Urlaub.JPG",
        file(3 * 1024 * 1024, NOW.unix - 400 * DAY),
    );
    let katze = ix.add(bilder, "katze.png", file(200 * 1024, NOW.unix - 2 * 3600));
    let docs = ix.add(boss, "Docs", dir());
    let bericht = ix.add(docs, "Bericht 2026.pdf", file(50 * 1024, NOW.unix - DAY));
    let notes = ix.add(docs, "notes.txt", file(0, NOW.unix - 3 * DAY));
    let uni = ix.add(boss, "Ünïcode.TXT", file(10, NOW.unix));
    (
        ix,
        vec![
            c, users, boss, bilder, urlaub, katze, docs, bericht, notes, uni,
        ],
    )
}

fn names(ix: &Index, q: &str) -> Vec<String> {
    let q = Query::parse(q, NOW).unwrap();
    ix.search(&q, None, SortBy::Name, false)
        .into_iter()
        .map(|i| ix.name(i).to_string())
        .collect()
}

#[test]
fn paths_and_lookup() {
    let (ix, id) = sample();
    assert_eq!(ix.path(id[4]), r"C:\Users\Boss\Bilder\Urlaub.JPG");
    assert_eq!(ix.path(id[0]), r"C:\");
    assert_eq!(ix.find(r"c:\users\BOSS\bilder\urlaub.jpg"), Some(id[4]));
    assert_eq!(ix.find(r"C:\Users\Boss\Docs\"), Some(id[6]));
    assert_eq!(ix.find(r"C:\Users\Nobody"), None);
    assert_eq!(ix.find(r"D:\x"), None);
    assert_eq!(ix.len(), 10);
    let mut unix = Index::new(false);
    let r = unix.add_root("/home/a", dir());
    let x = unix.add(r, "X", dir());
    let y = unix.add(x, "y.txt", file(1, 1));
    assert_eq!(unix.path(y), "/home/a/X/y.txt");
    assert_eq!(unix.find("/home/a/X/y.txt"), Some(y));
    assert_eq!(unix.find("/home/a/x/y.txt"), None, "case-sensitive on Unix");
    assert_eq!(
        unix.find("/home/ab"),
        None,
        "root prefix must end at a separator"
    );
    // shorter than the root, a root's parent, or not even a path: no panic
    assert_eq!(unix.find("/nirgends"), None);
    assert_eq!(unix.find("/home"), None);
    assert_eq!(unix.find(""), None);
    assert_eq!(ix.find("C"), None);
    assert_eq!(ix.find("ü"), None);
}

#[test]
fn text_wildcards_and_logic() {
    let (ix, _) = sample();
    assert_eq!(names(&ix, "katze"), ["katze.png"]);
    assert_eq!(names(&ix, "KATZE"), ["katze.png"], "case-insensitive");
    assert_eq!(names(&ix, "*.jpg"), ["Urlaub.JPG"]);
    assert_eq!(names(&ix, "*.txt"), ["notes.txt", "Ünïcode.TXT"]);
    assert_eq!(names(&ix, "ünï"), ["Ünïcode.TXT"]);
    assert_eq!(names(&ix, "t x"), ["notes.txt", "Ünïcode.TXT"]);
    assert_eq!(names(&ix, "katze|urlaub"), ["katze.png", "Urlaub.JPG"]);
    assert_eq!(names(&ix, "katze | urlaub"), ["katze.png", "Urlaub.JPG"]);
    assert_eq!(names(&ix, "*.txt !notes"), ["Ünïcode.TXT"]);
    assert_eq!(names(&ix, "\"bericht 2026\""), ["Bericht 2026.pdf"]);
    assert_eq!(names(&ix, "bericht 2026"), ["Bericht 2026.pdf"]);
    assert_eq!(
        names(&ix, "\"*.txt\""),
        Vec::<String>::new(),
        "quoted = literal"
    );
    assert_eq!(names(&ix, "case:katze"), ["katze.png"]);
    assert_eq!(names(&ix, "case:Katze"), Vec::<String>::new());
    assert_eq!(names(&ix, "nothing-like-this"), Vec::<String>::new());
    assert_eq!(names(&ix, "").len(), 10, "empty = everything");
}

#[test]
fn filters() {
    let (ix, _) = sample();
    assert_eq!(names(&ix, "ext:jpg;png"), ["katze.png", "Urlaub.JPG"]);
    assert_eq!(names(&ix, "ext:.PDF"), ["Bericht 2026.pdf"]);
    assert_eq!(
        names(&ix, "folder:"),
        ["Bilder", "Boss", r"C:\", "Docs", "Users"]
    );
    assert_eq!(names(&ix, "folder:b"), ["Bilder", "Boss"]);
    assert_eq!(names(&ix, "file:b"), ["Bericht 2026.pdf", "Urlaub.JPG"]);
    assert_eq!(names(&ix, "size:>1mb"), ["Urlaub.JPG"]);
    assert_eq!(names(&ix, "size:empty"), ["notes.txt"]);
    assert_eq!(
        names(&ix, "size:10kb..300kb"),
        ["Bericht 2026.pdf", "katze.png"]
    );
    assert_eq!(names(&ix, "dm:today"), ["katze.png", "Ünïcode.TXT"]);
    assert_eq!(names(&ix, "dm:yesterday"), ["Bericht 2026.pdf"]);
    assert_eq!(names(&ix, "dm:<2026 file:"), ["Urlaub.JPG"]);
    assert_eq!(names(&ix, "path:bilder file:"), ["katze.png", "Urlaub.JPG"]);
    assert_eq!(names(&ix, r"path:docs\n"), ["notes.txt"]);
    assert_eq!(
        names(&ix, r"regex:^[a-z]+\.(png|txt)$"),
        ["katze.png", "notes.txt"]
    );
    assert_eq!(
        names(&ix, "!folder: !ext:txt;pdf"),
        ["katze.png", "Urlaub.JPG"]
    );
}

#[test]
fn scope_and_sorting() {
    let (ix, id) = sample();
    let q = Query::parse("file:", NOW).unwrap();
    let docs: Vec<&str> = ix
        .search(&q, Some(id[6]), SortBy::Name, false)
        .into_iter()
        .map(|i| ix.name(i))
        .collect();
    assert_eq!(docs, ["Bericht 2026.pdf", "notes.txt"]);
    let by_size: Vec<&str> = ix
        .search(&q, None, SortBy::Size, true)
        .into_iter()
        .map(|i| ix.name(i))
        .collect();
    assert_eq!(by_size[0], "Urlaub.JPG");
    assert_eq!(by_size.last(), Some(&"notes.txt"));
    let by_date: Vec<&str> = ix
        .search(&q, None, SortBy::Modified, false)
        .into_iter()
        .map(|i| ix.name(i))
        .collect();
    assert_eq!(by_date[0], "Urlaub.JPG");
    let by_path: Vec<String> = ix
        .search(&q, None, SortBy::Path, false)
        .into_iter()
        .map(|i| ix.path(i))
        .collect();
    let mut sorted = by_path.clone();
    sorted.sort_by(|a, b| natural(a, b));
    assert_eq!(by_path, sorted);
}

#[test]
fn live_updates() {
    let (mut ix, id) = sample();
    // rename
    ix.rename(id[5], id[3], "Kater.png");
    assert_eq!(names(&ix, "kat"), ["Kater.png"]);
    assert_eq!(ix.find(r"C:\Users\Boss\Bilder\kater.png"), Some(id[5]));
    // move Docs into Bilder
    ix.rename(id[6], id[3], "Docs");
    assert_eq!(ix.path(id[8]), r"C:\Users\Boss\Bilder\Docs\notes.txt");
    assert!(ix.child(id[2], "Docs").is_none());
    // never below itself
    ix.rename(id[3], id[6], "Loop");
    assert_eq!(ix.name(id[3]), "Bilder");
    // upsert = update, no duplicate
    let n = ix.len();
    let again = ix.upsert(id[3], "KATER.PNG", file(7, 7));
    assert_eq!(again, id[5]);
    assert_eq!(ix.len(), n);
    assert_eq!(ix.meta(id[5]).size, 7);
    // remove a folder = its subtree
    ix.remove(id[3]);
    assert_eq!(ix.len(), n - 6);
    assert_eq!(names(&ix, "file:"), ["Ünïcode.TXT"]);
    assert!(ix.find(r"C:\Users\Boss\Bilder").is_none());
    ix.remove(id[3]); // twice is harmless
    assert_eq!(ix.len(), n - 6);
}

#[test]
fn compact_keeps_everything_live() {
    let (mut ix, id) = sample();
    ix.rename(id[6], id[3], "Dokumente"); // moved below an older folder
    let extra = ix.add(id[6], "neu.md", file(1, NOW.unix));
    ix.rename(id[3], id[1], "Fotos"); // moved up
    ix.remove(id[9]);
    let before: Vec<String> = {
        let mut v: Vec<String> = (0..ix.slots() as u32)
            .filter(|&i| ix.exists(i))
            .map(|i| ix.path(i))
            .collect();
        v.sort();
        v
    };
    let map = ix.compact();
    let mut after: Vec<String> = (0..ix.slots() as u32).map(|i| ix.path(i)).collect();
    after.sort();
    assert_eq!(before, after);
    assert_eq!(ix.slots(), ix.len());
    assert_eq!(map[id[9] as usize], NONE);
    assert_eq!(
        ix.path(map[extra as usize]),
        r"C:\Users\Fotos\Dokumente\neu.md"
    );
    assert_eq!(names(&ix, "neu"), ["neu.md"]);
}

#[test]
fn many_entries_use_threads_and_agree() {
    let mut ix = Index::new(false);
    let r = ix.add_root("/data", dir());
    for d in 0..300 {
        let p = ix.add(r, &format!("dir{d}"), dir());
        for f in 0..300 {
            ix.add(
                p,
                &format!("file{f}.{}", ["txt", "jpg", "rs"][f % 3]),
                file(f as u64, 1),
            );
        }
    }
    assert!(ix.slots() > 50_000);
    let q = Query::parse("file1*.rs", NOW).unwrap();
    let hits = ix.search(&q, None, SortBy::Name, false);
    // file1.., file1x.., file1xx.. with f % 3 == 2 → counted directly
    let expected = (0..300)
        .filter(|f| f % 3 == 2 && f.to_string().starts_with('1'))
        .count()
        * 300;
    assert_eq!(hits.len(), expected);
}
