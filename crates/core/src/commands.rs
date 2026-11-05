//! Command registry for the command palette (Ctrl+Shift+P): every command
//! with its German label, shortcut and the condition under which it makes
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
    pub label: &'static str,
    /// Shortcut as shown ("" = none).
    pub keys: &'static str,
    pub need: Need,
}

const fn c(id: &'static str, label: &'static str, keys: &'static str, need: Need) -> Command {
    Command {
        id,
        label,
        keys,
        need,
    }
}

/// All commands, in a sensible default order.
pub const ALL: &[Command] = &[
    c("open", "Öffnen", "Eingabe", Need::Selection),
    c("copy", "Kopieren", "Strg+C", Need::Selection),
    c("cut", "Ausschneiden", "Strg+X", Need::Selection),
    c("paste", "Einfügen", "Strg+V", Need::Paste),
    c("rename", "Umbenennen", "F2", Need::Selection),
    c("delete", "Löschen (Papierkorb)", "Entf", Need::Selection),
    c(
        "delete-permanent",
        "Endgültig löschen",
        "Umschalt+Entf",
        Need::Selection,
    ),
    c(
        "new-folder",
        "Neuer Ordner",
        "Strg+Umschalt+N",
        Need::Folder,
    ),
    c("undo", "Rückgängig", "Strg+Z", Need::Undo),
    c("redo", "Wiederholen", "Strg+Y", Need::Redo),
    c(
        "properties",
        "Eigenschaften",
        "Alt+Eingabe",
        Need::Selection,
    ),
    c("context-menu", "Kontextmenü", "Umschalt+F10", Need::Always),
    c("select-all", "Alles auswählen", "Strg+A", Need::Always),
    c("up", "Übergeordneter Ordner", "Alt+↑", Need::Always),
    c("back", "Zurück", "Alt+←", Need::Always),
    c("forward", "Vorwärts", "Alt+→", Need::Always),
    c("refresh", "Aktualisieren", "F5", Need::Always),
    c("edit-path", "Adresse bearbeiten", "Strg+L", Need::Always),
    c("new-tab", "Neuer Tab", "Strg+T", Need::Always),
    c("close-tab", "Tab schließen", "Strg+W", Need::Always),
    c("next-tab", "Nächster Tab", "Strg+Tab", Need::Always),
    c(
        "prev-tab",
        "Vorheriger Tab",
        "Strg+Umschalt+Tab",
        Need::Always,
    ),
    c(
        "toggle-split",
        "Zwei Ansichten ein/aus",
        "Strg+\\",
        Need::Always,
    ),
    c(
        "switch-pane",
        "Zur anderen Ansicht wechseln",
        "F6",
        Need::Always,
    ),
    c(
        "inspector",
        "Details-Bereich ein/aus",
        "Alt+P",
        Need::Always,
    ),
    c(
        "view-details",
        "Ansicht: Details",
        "Strg+Umschalt+6",
        Need::Always,
    ),
    c(
        "view-medium",
        "Ansicht: Mittelgroße Symbole",
        "Strg+Umschalt+3",
        Need::Always,
    ),
    c(
        "view-large",
        "Ansicht: Große Symbole",
        "Strg+Umschalt+2",
        Need::Always,
    ),
    c(
        "view-xlarge",
        "Ansicht: Extra große Symbole",
        "Strg+Umschalt+1",
        Need::Always,
    ),
    c(
        "view-all",
        "Aktuelle Ansicht für alle Ordner",
        "",
        Need::Always,
    ),
    c("sort-name", "Sortieren nach Name", "", Need::Always),
    c(
        "sort-modified",
        "Sortieren nach Änderungsdatum",
        "",
        Need::Always,
    ),
    c("sort-type", "Sortieren nach Typ", "", Need::Always),
    c("sort-size", "Sortieren nach Größe", "", Need::Always),
    c("sort-asc", "Aufsteigend sortieren", "", Need::Always),
    c("sort-desc", "Absteigend sortieren", "", Need::Always),
    c(
        "toggle-hidden",
        "Ausgeblendete Elemente ein/aus",
        "",
        Need::Always,
    ),
    c(
        "toggle-extensions",
        "Dateinamenerweiterungen ein/aus",
        "",
        Need::Always,
    ),
    c("go-trash", "Papierkorb öffnen", "", Need::Always),
    c("go-thispc", "Dieser PC öffnen", "", Need::Always),
    c("go-libraries", "Bibliotheken öffnen", "", Need::Always),
    c(
        "restore",
        "Aus dem Papierkorb wiederherstellen",
        "",
        Need::Trash,
    ),
    c("empty-trash", "Papierkorb leeren", "", Need::Trash),
    c("settings", "Einstellungen", "", Need::Always),
    c("theme-system", "Farbschema: Wie Windows", "", Need::Always),
    c("theme-light", "Farbschema: Hell", "", Need::Always),
    c("theme-dark", "Farbschema: Dunkel", "", Need::Always),
    c(
        "toggle-folder-open",
        "Ordner mit Explorer 2.0 öffnen ein/aus",
        "",
        Need::Always,
    ),
    c(
        "toggle-context-menu",
        "„In Explorer 2.0 öffnen“ im Kontextmenü ein/aus",
        "",
        Need::Always,
    ),
    c(
        "toggle-win-e",
        "Win+E öffnet Explorer 2.0 ein/aus",
        "",
        Need::Always,
    ),
    c(
        "toggle-single-instance",
        "Ordner im offenen Fenster öffnen (neuer Tab) ein/aus",
        "",
        Need::Always,
    ),
    c(
        "install",
        "Explorer 2.0 installieren / aktualisieren",
        "",
        Need::Always,
    ),
    c(
        "integration-reset",
        "Alle Windows-Integrationen zurücksetzen",
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
        .filter_map(|(i, c)| fuzzy_score(query, c.label).map(|s| (s, i, c)))
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
        let r = rank("lösch", &[], ALL);
        assert_eq!(r[0].id, "delete");
        assert!(r.iter().any(|c| c.id == "delete-permanent"));
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
