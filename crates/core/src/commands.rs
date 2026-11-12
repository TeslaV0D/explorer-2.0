//! Command registry for the command palette (Ctrl+Shift+P): every command
//! with its label (English source, translated when shown – D-029), shortcut and the condition under which it makes
//! sense, plus fuzzy matching and ranking (pure → tested).

/// When a command can run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Need {
    Always,
    /// Something selected (or focused) in the list.
    Selection,
    /// A real folder is shown (not a virtual place / server).
    Folder,
    Paste,
    Undo,
    Redo,
    /// The recycle bin is shown.
    Trash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Command {
    pub id: &'static str,
    /// English source text; shown via [`Command::title`].
    pub label: &'static str,
    /// Shortcut (English source, "" = none); shown via [`Command::shortcut`].
    pub keys: &'static str,
    pub need: Need,
}

impl Command {
    /// The label in the current language.
    pub fn title(&self) -> &'static str {
        explorer_i18n::translate(self.label)
    }
    /// The shortcut in the current language ("Strg+C" in German).
    pub fn shortcut(&self) -> &'static str {
        explorer_i18n::translate(self.keys)
    }
}

const fn c(id: &'static str, label: &'static str, keys: &'static str, need: Need) -> Command {
    Command {
        id,
        label,
        keys,
        need,
    }
}

use explorer_i18n::tr_noop;

/// All commands, in a sensible default order.
pub const ALL: &[Command] = &[
    c("open", tr_noop!("Open"), tr_noop!("Enter"), Need::Selection),
    c(
        "copy",
        tr_noop!("Copy"),
        tr_noop!("Ctrl+C"),
        Need::Selection,
    ),
    c("cut", tr_noop!("Cut"), tr_noop!("Ctrl+X"), Need::Selection),
    c("paste", tr_noop!("Paste"), tr_noop!("Ctrl+V"), Need::Paste),
    c("rename", tr_noop!("Rename"), "F2", Need::Selection),
    c(
        "delete",
        tr_noop!("Delete (to Recycle Bin)"),
        tr_noop!("Del"),
        Need::Selection,
    ),
    c(
        "delete-permanent",
        tr_noop!("Delete permanently"),
        tr_noop!("Shift+Del"),
        Need::Selection,
    ),
    c(
        "new-folder",
        tr_noop!("New folder"),
        tr_noop!("Ctrl+Shift+N"),
        Need::Folder,
    ),
    c("undo", tr_noop!("Undo"), tr_noop!("Ctrl+Z"), Need::Undo),
    c("redo", tr_noop!("Redo"), tr_noop!("Ctrl+Y"), Need::Redo),
    c(
        "properties",
        tr_noop!("Properties"),
        tr_noop!("Alt+Enter"),
        Need::Selection,
    ),
    c(
        "context-menu",
        tr_noop!("Context menu"),
        tr_noop!("Shift+F10"),
        Need::Always,
    ),
    c(
        "select-all",
        tr_noop!("Select all"),
        tr_noop!("Ctrl+A"),
        Need::Always,
    ),
    c("up", tr_noop!("Go to parent folder"), "Alt+↑", Need::Always),
    c("back", tr_noop!("Back"), "Alt+←", Need::Always),
    c("forward", tr_noop!("Forward"), "Alt+→", Need::Always),
    c("refresh", tr_noop!("Refresh"), "F5", Need::Always),
    c(
        "edit-path",
        tr_noop!("Edit address"),
        tr_noop!("Ctrl+L"),
        Need::Always,
    ),
    c(
        "new-tab",
        tr_noop!("New tab"),
        tr_noop!("Ctrl+T"),
        Need::Always,
    ),
    c(
        "close-tab",
        tr_noop!("Close tab"),
        tr_noop!("Ctrl+W"),
        Need::Always,
    ),
    c(
        "next-tab",
        tr_noop!("Next tab"),
        tr_noop!("Ctrl+Tab"),
        Need::Always,
    ),
    c(
        "prev-tab",
        tr_noop!("Previous tab"),
        tr_noop!("Ctrl+Shift+Tab"),
        Need::Always,
    ),
    c(
        "toggle-split",
        tr_noop!("Toggle two views"),
        tr_noop!("Ctrl+\\"),
        Need::Always,
    ),
    c(
        "switch-pane",
        tr_noop!("Switch to the other view"),
        "F6",
        Need::Always,
    ),
    c(
        "inspector",
        tr_noop!("Toggle details pane"),
        "Alt+P",
        Need::Always,
    ),
    c(
        "view-details",
        tr_noop!("View: Details"),
        tr_noop!("Ctrl+Shift+6"),
        Need::Always,
    ),
    c(
        "view-medium",
        tr_noop!("View: Medium icons"),
        tr_noop!("Ctrl+Shift+3"),
        Need::Always,
    ),
    c(
        "view-large",
        tr_noop!("View: Large icons"),
        tr_noop!("Ctrl+Shift+2"),
        Need::Always,
    ),
    c(
        "view-xlarge",
        tr_noop!("View: Extra large icons"),
        tr_noop!("Ctrl+Shift+1"),
        Need::Always,
    ),
    c(
        "view-all",
        tr_noop!("Use the current view for all folders"),
        "",
        Need::Always,
    ),
    c("sort-name", tr_noop!("Sort by name"), "", Need::Always),
    c(
        "sort-modified",
        tr_noop!("Sort by date modified"),
        "",
        Need::Always,
    ),
    c("sort-type", tr_noop!("Sort by type"), "", Need::Always),
    c("sort-size", tr_noop!("Sort by size"), "", Need::Always),
    c("sort-asc", tr_noop!("Sort ascending"), "", Need::Always),
    c("sort-desc", tr_noop!("Sort descending"), "", Need::Always),
    c(
        "toggle-hidden",
        tr_noop!("Toggle hidden items"),
        "",
        Need::Always,
    ),
    c(
        "toggle-extensions",
        tr_noop!("Toggle file name extensions"),
        "",
        Need::Always,
    ),
    c("go-trash", tr_noop!("Open Recycle Bin"), "", Need::Always),
    c("go-thispc", tr_noop!("Open This PC"), "", Need::Always),
    c("go-libraries", tr_noop!("Open Libraries"), "", Need::Always),
    c(
        "restore",
        tr_noop!("Restore from the Recycle Bin"),
        "",
        Need::Trash,
    ),
    c(
        "empty-trash",
        tr_noop!("Empty Recycle Bin"),
        "",
        Need::Trash,
    ),
    c("settings", tr_noop!("Settings"), "", Need::Always),
    c(
        "theme-system",
        tr_noop!("Color scheme: System default"),
        "",
        Need::Always,
    ),
    c(
        "theme-light",
        tr_noop!("Color scheme: Light"),
        "",
        Need::Always,
    ),
    c(
        "theme-dark",
        tr_noop!("Color scheme: Dark"),
        "",
        Need::Always,
    ),
    // L4: one per offered language (the native name stays in every language)
    c(
        "language-system",
        tr_noop!("Language: System default"),
        "",
        Need::Always,
    ),
    c(
        "language-en",
        tr_noop!("Language: English"),
        "",
        Need::Always,
    ),
    c(
        "language-de",
        tr_noop!("Language: Deutsch"),
        "",
        Need::Always,
    ),
    c(
        "language-es",
        tr_noop!("Language: Español"),
        "",
        Need::Always,
    ),
    c(
        "language-fr",
        tr_noop!("Language: Français"),
        "",
        Need::Always,
    ),
    c(
        "language-ru",
        tr_noop!("Language: Русский"),
        "",
        Need::Always,
    ),
    c(
        "language-uk",
        tr_noop!("Language: Українська"),
        "",
        Need::Always,
    ),
    c(
        "language-pl",
        tr_noop!("Language: Polski"),
        "",
        Need::Always,
    ),
    c(
        "language-it",
        tr_noop!("Language: Italiano"),
        "",
        Need::Always,
    ),
    c(
        "language-zh",
        tr_noop!("Language: 中文（简体）"),
        "",
        Need::Always,
    ),
    c(
        "language-ja",
        tr_noop!("Language: 日本語"),
        "",
        Need::Always,
    ),
    c(
        "toggle-folder-open",
        tr_noop!("Toggle: open folders with Explorer 2.0"),
        "",
        Need::Always,
    ),
    c(
        "toggle-context-menu",
        tr_noop!("Toggle: “Open in Explorer 2.0” in the context menu"),
        "",
        Need::Always,
    ),
    c(
        "toggle-win-e",
        tr_noop!("Toggle: Win+E opens Explorer 2.0"),
        "",
        Need::Always,
    ),
    c(
        "toggle-single-instance",
        tr_noop!("Toggle: open folders in the running window (new tab)"),
        "",
        Need::Always,
    ),
    c(
        "search",
        tr_noop!("Search (in the current folder)"),
        tr_noop!("Ctrl+F"),
        Need::Always,
    ),
    c(
        "quick-search",
        tr_noop!("Search all drives"),
        tr_noop!("Ctrl+Shift+F"),
        Need::Always,
    ),
    c(
        "search-rescan",
        tr_noop!("Re-sync search index with disk"),
        "",
        Need::Always,
    ),
    c(
        "toggle-index",
        tr_noop!("Toggle search index"),
        "",
        Need::Always,
    ),
    c(
        "install",
        tr_noop!("Install / update Explorer 2.0"),
        "",
        Need::Always,
    ),
    c(
        "integration-reset",
        tr_noop!("Reset all Windows integrations"),
        "",
        Need::Always,
    ),
];

pub fn by_id(id: &str) -> Option<&'static Command> {
    ALL.iter().find(|c| c.id == id)
}

fn fold(c: char) -> char {
    match c {
        'ä' | 'Ä' => 'a',
        'ö' | 'Ö' => 'o',
        'ü' | 'Ü' => 'u',
        'ß' => 's',
        c => c.to_ascii_lowercase(),
    }
}

/// Fuzzy match: every character of `query` (case/umlaut-insensitive) must
/// appear in `text` in order. Higher is better: consecutive characters and
/// word starts score, a match at the very start scores most. `None` = no match.
pub fn fuzzy_score(query: &str, text: &str) -> Option<i32> {
    let q: Vec<char> = query
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(fold)
        .collect();
    if q.is_empty() {
        return Some(0);
    }
    let t: Vec<char> = text.chars().map(fold).collect();
    let raw: Vec<char> = text.chars().collect();
    let mut score = 0;
    let mut qi = 0;
    let mut prev: Option<usize> = None;
    for (i, &ch) in t.iter().enumerate() {
        if qi < q.len() && ch == q[qi] {
            let word_start = i == 0 || !raw[i - 1].is_alphanumeric();
            score += 1;
            if word_start {
                score += 8;
            }
            if i == 0 {
                score += 4;
            }
            if prev.is_some_and(|p| p + 1 == i) {
                score += 5;
            }
            prev = Some(i);
            qi += 1;
        }
    }
    (qi == q.len()).then(|| score - (t.len() as i32 / 10))
}

/// Palette order: matching commands by score, recently used first among
/// equals (and on top when the query is empty).
pub fn rank<'a>(query: &str, recent: &[String], cmds: &'a [Command]) -> Vec<&'a Command> {
    let recent_pos = |id: &str| recent.iter().position(|r| r == id);
    let mut hits: Vec<(i32, usize, &Command)> = cmds
        .iter()
        .enumerate()
        // the English source matches too: findable in a language one cannot read
        .filter_map(|(i, c)| {
            fuzzy_score(query, c.title())
                .max(fuzzy_score(query, c.label))
                .map(|s| (s, i, c))
        })
        .collect();
    hits.sort_by(|a, b| {
        let ra = recent_pos(a.2.id).map(|p| 100 - p as i32).unwrap_or(0);
        let rb = recent_pos(b.2.id).map(|p| 100 - p as i32).unwrap_or(0);
        if query.trim().is_empty() {
            rb.cmp(&ra).then(a.1.cmp(&b.1))
        } else {
            (b.0 * 1000 + rb)
                .cmp(&(a.0 * 1000 + ra))
                .then(a.1.cmp(&b.1))
        }
    });
    hits.into_iter().map(|(_, _, c)| c).collect()
}

/// Remember `id` as most recently used (max `keep`, no duplicates).
pub fn push_recent(recent: &mut Vec<String>, id: &str, keep: usize) {
    recent.retain(|r| r != id);
    recent.insert(0, id.to_string());
    recent.truncate(keep);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_found() {
        for (i, a) in ALL.iter().enumerate() {
            assert!(
                ALL[i + 1..].iter().all(|b| b.id != a.id),
                "duplicate {}",
                a.id
            );
            assert_eq!(by_id(a.id), Some(a));
        }
        assert_eq!(by_id("gibtsnicht"), None);
    }

    #[test]
    fn every_offered_language_has_a_command() {
        for code in explorer_i18n::choices("") {
            let id = match code {
                "" => "language-system".to_string(),
                c => format!("language-{c}"),
            };
            assert!(by_id(&id).is_some(), "no palette command {id}");
        }
        assert!(ALL
            .iter()
            .filter(|c| c.id.starts_with("language-") && c.id != "language-system")
            .all(|c| explorer_i18n::lang(&c.id["language-".len()..]).is_some()));
    }

    #[test]
    fn fuzzy_matching() {
        assert!(fuzzy_score("neord", "Neuer Ordner").is_some());
        assert!(
            fuzzy_score("NEU ORD", "Neuer Ordner").is_some(),
            "case and spaces"
        );
        assert!(
            fuzzy_score("uber", "Übergeordneter Ordner").is_some(),
            "umlauts fold"
        );
        assert_eq!(fuzzy_score("xyz", "Neuer Ordner"), None);
        assert_eq!(
            fuzzy_score("ordner neu", "Neuer Ordner"),
            None,
            "order matters"
        );
        // word starts beat letters in the middle
        let a = fuzzy_score("ko", "Kopieren").unwrap();
        let b = fuzzy_score("ko", "Kontextmenü").unwrap();
        let c = fuzzy_score("ko", "Rückgängig oder so").unwrap_or(-99);
        assert!(a >= b && b > c);
    }

    #[test]
    fn ranking_and_recent() {
        // tests see the English source texts (no language set)
        let r = rank("del rec", &[], ALL);
        assert_eq!(r[0].id, "delete");
        assert!(rank("delet", &[], ALL)
            .iter()
            .any(|c| c.id == "delete-permanent"));
        assert_eq!(by_id("copy").unwrap().shortcut(), "Ctrl+C");
        // empty query: recently used first, then the default order
        let recent = vec!["inspector".to_string(), "paste".to_string()];
        let r = rank("", &recent, ALL);
        assert_eq!((r[0].id, r[1].id, r[2].id), ("inspector", "paste", "open"));
        let mut v = vec!["a".to_string(), "b".to_string()];
        push_recent(&mut v, "b", 2);
        push_recent(&mut v, "c", 2);
        assert_eq!(v, vec!["c".to_string(), "b".to_string()]);
    }
}
