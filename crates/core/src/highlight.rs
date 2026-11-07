//! Which part of a name a search matched (shown bold in the results).

/// The literal pieces of the plain words of `query`, longest first:
/// `bericht *.pdf` → `["bericht", ".pdf"]`. Words with a function
/// (`ext:pdf`, `path:…`, `regex:…`), negations (`!x`) and `|` are skipped:
/// they say nothing about which letters of the name matched.
pub fn pieces(query: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for tok in query.split_whitespace() {
        let t = tok.trim_matches('"');
        if t.is_empty() || t.starts_with('!') || t.contains(':') || t == "|" {
            continue;
        }
        for part in t.split(['*', '?', '|', '"']) {
            if !part.is_empty() && !out.iter().any(|o| o == part) {
                out.push(part.to_string());
            }
        }
    }
    out.sort_by_key(|p| std::cmp::Reverse(p.chars().count()));
    out
}

fn same(a: char, b: char) -> bool {
    a == b || a.to_lowercase().eq(b.to_lowercase())
}

/// Byte range of the first place in `name` where one of `pieces` occurs
/// (no case; longest piece first). `None`: nothing to mark.
pub fn find(name: &str, pieces: &[String]) -> Option<(usize, usize)> {
    let chars: Vec<(usize, char)> = name.char_indices().collect();
    for p in pieces {
        let pc: Vec<char> = p.chars().collect();
        if pc.is_empty() || pc.len() > chars.len() {
            continue;
        }
        for start in 0..=chars.len() - pc.len() {
            if pc
                .iter()
                .enumerate()
                .all(|(k, &c)| same(chars[start + k].1, c))
            {
                let from = chars[start].0;
                let to = chars.get(start + pc.len()).map_or(name.len(), |c| c.0);
                return Some((from, to));
            }
        }
    }
    None
}

/// `name` split around the match: (before, match, after); `None`: no match.
pub fn split<'a>(name: &'a str, pieces: &[String]) -> Option<(&'a str, &'a str, &'a str)> {
    let (a, b) = find(name, pieces)?;
    Some((&name[..a], &name[a..b], &name[b..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pieces_of_a_query() {
        assert_eq!(pieces("bericht *.pdf"), ["bericht", ".pdf"]);
        assert_eq!(pieces("ext:pdf !alt path:x regex:^a"), Vec::<String>::new());
        assert_eq!(pieces("a | bb"), ["bb", "a"]);
        assert_eq!(
            pieces("\"zwei worte\" foo*bar"),
            ["worte", "zwei", "foo", "bar"]
        );
        assert_eq!(pieces("x x X"), ["x", "X"]);
    }

    #[test]
    fn marks_the_first_match_without_case() {
        let p = pieces("bericht");
        assert_eq!(
            split("Jahresbericht 2026.pdf", &p),
            Some(("Jahres", "bericht", " 2026.pdf"))
        );
        assert_eq!(split("BERICHT", &p), Some(("", "BERICHT", "")));
        assert_eq!(split("nichts", &p), None);
        // umlauts and multi-byte characters keep their boundaries
        let p = pieces("größe");
        assert_eq!(split("Übergröße.txt", &p), Some(("Über", "größe", ".txt")));
        assert_eq!(split("ÜBERGRÖSSE", &p), None); // ß ≠ SS here (shown unmarked)
        let p = pieces("ü");
        assert_eq!(split("Ü", &p), Some(("", "Ü", "")));
        // longest piece wins
        let p = pieces("a bericht");
        assert_eq!(split("a bericht", &p), Some(("a ", "bericht", "")));
        // a piece longer than the name, empty name
        assert_eq!(split("ab", &pieces("abc")), None);
        assert_eq!(split("", &pieces("a")), None);
    }
}
