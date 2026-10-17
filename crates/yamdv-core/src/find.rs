//! Find in document (B.9, prompt 005 §3.8): case folding with Turkish i/İ/ı/I equivalence,
//! all matches, and wrap-around navigation. Pure.

/// Folds one char for matching. Case-insensitive mode maps i, İ, ı and I to `i`.
fn fold(c: char, case: bool) -> char {
    if case {
        return c;
    }
    match c {
        'I' | 'İ' | 'ı' | 'i' => 'i',
        _ => c.to_lowercase().next().unwrap_or(c),
    }
}

/// Byte ranges of every non-overlapping match of `needle` in `hay`.
pub fn find_all(hay: &str, needle: &str, match_case: bool) -> Vec<(usize, usize)> {
    let n: Vec<char> = needle.chars().map(|c| fold(c, match_case)).collect();
    let Some(&first) = n.first() else {
        return Vec::new();
    };
    let h: Vec<(usize, char)> = hay
        .char_indices()
        .map(|(i, c)| (i, fold(c, match_case)))
        .collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i + n.len() <= h.len() {
        let hit = h.get(i).is_some_and(|x| x.1 == first)
            && n.iter()
                .enumerate()
                .all(|(k, c)| h.get(i + k).is_some_and(|x| x.1 == *c));
        if hit {
            let start = h.get(i).map_or(0, |x| x.0);
            let end = h.get(i + n.len()).map_or(hay.len(), |x| x.0);
            out.push((start, end));
            i += n.len();
        } else {
            i += 1;
        }
    }
    out
}

/// Next match index after moving `dir` (±1) from `cur`, wrapping (Enter / Shift+Enter, Ctrl+G).
pub fn step(cur: Option<usize>, count: usize, dir: i32) -> Option<usize> {
    if count == 0 {
        return None;
    }
    Some(match cur {
        None if dir < 0 => count - 1,
        None => 0,
        Some(c) if dir < 0 => (c + count - 1) % count,
        Some(c) => (c + 1) % count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn b9_turkish_i_equivalence() {
        let t = "İstanbul ısı Iğdır iyi";
        assert_eq!(find_all(t, "i", false).len(), 7);
        assert_eq!(find_all(t, "ISI", false), vec![(10, 15)]);
        assert_eq!(find_all(t, "İstanbul", true).len(), 1);
        assert_eq!(find_all(t, "istanbul", true).len(), 0);
    }

    #[test]
    fn b9_matches_and_wrap() {
        assert_eq!(find_all("aaaa", "aa", false), vec![(0, 2), (2, 4)]);
        assert!(find_all("abc", "", false).is_empty());
        assert_eq!(find_all("Ab ab", "AB", false).len(), 2);
        assert_eq!(step(None, 3, 1), Some(0));
        assert_eq!(step(Some(2), 3, 1), Some(0));
        assert_eq!(step(Some(0), 3, -1), Some(2));
        assert_eq!(step(None, 0, 1), None);
    }
}
