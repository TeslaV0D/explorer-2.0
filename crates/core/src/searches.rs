//! Recently used searches (`search-history.txt`, one per line, newest first).

/// How many are kept.
pub const KEEP: usize = 20;

/// Tolerant: blank lines and repeats are dropped, at most [`KEEP`].
pub fn parse(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in text.lines() {
        let q = line.trim();
        if !q.is_empty() && !out.iter().any(|o| o == q) {
            out.push(q.to_string());
            if out.len() == KEEP {
                break;
            }
        }
    }
    out
}

pub fn serialize(list: &[String]) -> String {
    let mut s = String::new();
    for q in list.iter().take(KEEP) {
        // a query is one line (the search field is single-line anyway)
        s.push_str(&q.replace(['\r', '\n'], " "));
        s.push('\n');
    }
    s
}

/// `query` was used: to the front (once), at most [`KEEP`].
pub fn push(list: &mut Vec<String>, query: &str) {
    let q = query.trim();
    if q.is_empty() {
        return;
    }
    list.retain(|o| o != q);
    list.insert(0, q.to_string());
    list.truncate(KEEP);
}

/// Up/Down in the search field: `cursor` = how far back (0 = not browsing).
/// `older` = Up. Returns the new cursor and the query to show (None: leave
/// the field as it is – Down at the newest goes back to the list).
pub fn step(list: &[String], cursor: usize, older: bool) -> (usize, Option<String>) {
    if older {
        if cursor < list.len() {
            (cursor + 1, Some(list[cursor].clone()))
        } else {
            (cursor, None)
        }
    } else if cursor > 1 {
        (cursor - 1, Some(list[cursor - 2].clone()))
    } else {
        (0, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_tolerance() {
        let mut l = Vec::new();
        push(&mut l, "bericht");
        push(&mut l, " ext:pdf ");
        push(&mut l, "bericht"); // to the front, once
        push(&mut l, "   ");
        assert_eq!(l, ["bericht", "ext:pdf"]);
        assert_eq!(parse(&serialize(&l)), l);
        assert_eq!(parse("\n a \n\na\nb\r\n"), ["a", "b"]);
        let many: Vec<String> = (0..50).map(|i| i.to_string()).collect();
        assert_eq!(parse(&serialize(&many)).len(), KEEP);
        for i in 0..30 {
            push(&mut l, &format!("q{i}"));
        }
        assert_eq!(l.len(), KEEP);
        assert_eq!(serialize(&["a\nb".to_string()]), "a b\n");
    }

    #[test]
    fn up_and_down_like_a_shell() {
        let l: Vec<String> = ["neu", "mittel", "alt"].map(String::from).to_vec();
        let (c, q) = step(&l, 0, true);
        assert_eq!((c, q.as_deref()), (1, Some("neu")));
        let (c, q) = step(&l, c, true);
        assert_eq!((c, q.as_deref()), (2, Some("mittel")));
        let (c, q) = step(&l, c, true);
        let (c, q2) = step(&l, c, true); // at the oldest: stays
        assert_eq!((q.as_deref(), c, q2), (Some("alt"), 3, None));
        let (c, q) = step(&l, c, false);
        assert_eq!((c, q.as_deref()), (2, Some("mittel")));
        let (c, _) = step(&l, c, false);
        let (c, q) = step(&l, c, false); // past the newest: back to the list
        assert_eq!((c, q), (0, None));
        assert_eq!(step(&[], 0, true), (0, None));
    }
}
