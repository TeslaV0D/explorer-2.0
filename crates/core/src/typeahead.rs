//! Explorer type-to-select: typed letters jump to the first matching name;
//! repeating the same single letter cycles through matches; buffer resets after a pause.

use std::time::{Duration, Instant};

pub const RESET_AFTER: Duration = Duration::from_millis(1000);

#[derive(Debug, Default)]
pub struct TypeAhead {
    buffer: String,
    last: Option<Instant>,
}

impl TypeAhead {
    /// Feeds one typed char and returns the index to focus, if any.
    /// `current` = focused index, `names` = displayed order.
    pub fn feed<'a>(
        &mut self,
        ch: char,
        now: Instant,
        current: Option<usize>,
        names: impl Fn(usize) -> &'a str,
        len: usize,
    ) -> Option<usize> {
        if self
            .last
            .is_none_or(|t| now.duration_since(t) > RESET_AFTER)
        {
            self.buffer.clear();
        }
        self.last = Some(now);
        self.buffer.extend(ch.to_lowercase());
        if len == 0 {
            return None;
        }
        let first = self.buffer.chars().next().unwrap_or(ch);
        let cycling = self.buffer.chars().all(|c| c == first);
        let (needle, start) = if cycling {
            // same letter again → next match after current
            (first.to_string(), current.map(|c| c + 1).unwrap_or(0))
        } else {
            (self.buffer.clone(), current.unwrap_or(0))
        };
        (0..len)
            .map(|k| (start + k) % len)
            .find(|&i| names(i).to_lowercase().starts_with(&needle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMES: [&str; 6] = ["Apfel", "ausgang", "Banane", "baum", "Birne", "Zebra"];

    fn feed(t: &mut TypeAhead, c: char, at: Instant, cur: Option<usize>) -> Option<usize> {
        t.feed(c, at, cur, |i| NAMES[i], NAMES.len())
    }

    #[test]
    fn prefix_and_cycle() {
        let t0 = Instant::now();
        let ms = Duration::from_millis;
        let mut t = TypeAhead::default();
        assert_eq!(feed(&mut t, 'b', t0, None), Some(2));
        assert_eq!(feed(&mut t, 'i', t0 + ms(100), Some(2)), Some(4)); // "bi" -> Birne
                                                                       // pause -> new search; repeated 'b' cycles Banane -> baum -> Birne -> Banane
        assert_eq!(feed(&mut t, 'b', t0 + ms(2000), Some(4)), Some(2));
        assert_eq!(feed(&mut t, 'b', t0 + ms(2100), Some(2)), Some(3));
        assert_eq!(feed(&mut t, 'b', t0 + ms(2200), Some(3)), Some(4));
        assert_eq!(feed(&mut t, 'b', t0 + ms(2300), Some(4)), Some(2));
        assert_eq!(feed(&mut t, 'q', t0 + ms(5000), Some(2)), None);
        assert_eq!(feed(&mut t, 'Z', t0 + ms(9000), None), Some(5));
    }

    #[test]
    fn empty_list() {
        let mut t = TypeAhead::default();
        assert_eq!(t.feed('a', Instant::now(), None, |_| "", 0), None);
    }
}
