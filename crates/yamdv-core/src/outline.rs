//! Heading palette filter (FUT-08, B.10.8) and `[TOC]` detection / entries (prompt 014 §3.4–5).
//! Pure.

use crate::doc::index::HeadingEntry;

/// A palette match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    /// Heading index.
    pub index: usize,
    /// Rank: 0 title start, 1 word start, 2 inside a word, 3 word initials.
    pub rank: u8,
    /// Matched char ranges in the title (for highlighting).
    pub ranges: Vec<(usize, usize)>,
}

fn folded(s: &str) -> Vec<char> {
    s.chars().map(|c| crate::find::fold(c, false)).collect()
}

fn word_start(t: &[char], i: usize) -> bool {
    i == 0 || t.get(i - 1).is_some_and(|c| !c.is_alphanumeric())
}

/// Ranks one title against a folded query.
fn rank(title: &str, q: &[char]) -> Option<(u8, Vec<(usize, usize)>)> {
    let t = folded(title);
    if q.is_empty() {
        return Some((0, Vec::new()));
    }
    let mut best: Option<(u8, usize)> = None;
    for i in 0..t.len().saturating_sub(q.len() - 1) {
        if t.get(i..i + q.len()) == Some(q) {
            let r = if i == 0 {
                0
            } else if word_start(&t, i) {
                1
            } else {
                2
            };
            if best.is_none_or(|b| r < b.0) {
                best = Some((r, i));
            }
        }
    }
    if let Some((r, i)) = best {
        return Some((r, vec![(i, i + q.len())]));
    }
    // Word initials ("gs" → "Getting Started").
    let starts: Vec<usize> = (0..t.len())
        .filter(|&i| word_start(&t, i) && t.get(i).is_some_and(|c| c.is_alphanumeric()))
        .collect();
    let mut ranges = Vec::new();
    let mut k = 0;
    for &s in &starts {
        if k < q.len() && t.get(s) == q.get(k) {
            ranges.push((s, s + 1));
            k += 1;
        }
    }
    (k == q.len() && q.len() > 1).then_some((3, ranges))
}

/// Filters heading titles by `query` (Turkish-aware, case-insensitive): best rank first,
/// document order within a rank; an empty query lists every heading in order.
pub fn filter(titles: &[&str], query: &str) -> Vec<Hit> {
    let q = folded(query.trim());
    let mut out: Vec<Hit> = titles
        .iter()
        .enumerate()
        .filter_map(|(index, t)| {
            rank(t, &q).map(|(rank, ranges)| Hit {
                index,
                rank,
                ranges,
            })
        })
        .collect();
    out.sort_by_key(|h| (h.rank, h.index));
    out
}

/// The paragraph is a table-of-contents marker: exactly `[TOC]` or `[[_TOC_]]`.
pub fn is_toc_marker(paragraph: &str) -> bool {
    matches!(paragraph.trim(), "[TOC]" | "[[_TOC_]]")
}

/// One `[TOC]` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocLine {
    /// Nesting (0 = shallowest heading level present).
    pub indent: u8,
    /// Heading text.
    pub text: String,
    /// Anchor id.
    pub slug: String,
    /// Heading index.
    pub heading: usize,
}

/// TOC lines for headings down to level `max_depth` (1–6), indented relative to the shallowest
/// level shown.
pub fn toc_lines(headings: &[HeadingEntry], max_depth: u8) -> Vec<TocLine> {
    let shown: Vec<(usize, &HeadingEntry)> = headings
        .iter()
        .enumerate()
        .filter(|(_, h)| h.level <= max_depth.clamp(1, 6))
        .collect();
    let top = shown.iter().map(|(_, h)| h.level).min().unwrap_or(1);
    shown
        .into_iter()
        .map(|(i, h)| TocLine {
            indent: h.level - top,
            text: h.text.clone(),
            slug: h.slug.clone(),
            heading: i,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: &[&str] = &[
        "Introduction",
        "Installation",
        "Getting Started",
        "Reinstall notes",
        "İstanbul ofisi",
        "Kurulum ışığı",
    ];

    fn idx(q: &str) -> Vec<usize> {
        filter(T, q).iter().map(|h| h.index).collect()
    }

    #[test]
    fn ranks_and_order() {
        assert_eq!(idx(""), [0, 1, 2, 3, 4, 5]);
        assert_eq!(idx("inst"), [1, 3]);
        assert_eq!(filter(T, "inst").first().map(|h| h.rank), Some(0));
        assert_eq!(filter(T, "inst").get(1).map(|h| h.rank), Some(2));
        assert_eq!(idx("started"), [2]);
        assert_eq!(filter(T, "started")[0].rank, 1);
        assert_eq!(idx("gs"), [2]);
        assert_eq!(filter(T, "gs")[0].ranges, vec![(0, 1), (8, 9)]);
        assert_eq!(idx("zzz"), Vec::<usize>::new());
    }

    #[test]
    fn turkish_folding() {
        // i / İ / ı / I are equal.
        assert_eq!(idx("istanbul"), [4]);
        assert_eq!(idx("ISTANBUL"), [4]);
        assert_eq!(idx("IŞIĞI"), [5]);
        assert_eq!(idx("ınstall"), [1, 3]);
    }

    #[test]
    fn toc_marker() {
        assert!(is_toc_marker("[TOC]"));
        assert!(is_toc_marker("  [[_TOC_]]\n"));
        assert!(!is_toc_marker("[toc]"));
        assert!(!is_toc_marker("see [TOC]"));
        assert!(!is_toc_marker("[TOC]x"));
    }

    #[test]
    fn toc_depth() {
        let h = |level: u8, t: &str| HeadingEntry {
            level,
            range: 0..0,
            text: t.into(),
            explicit_id: None,
            slug: t.to_lowercase(),
            block: 0,
        };
        let hs = vec![h(2, "A"), h(3, "B"), h(4, "C"), h(2, "D")];
        let l = toc_lines(&hs, 3);
        assert_eq!(l.len(), 3);
        assert_eq!(l.iter().map(|x| x.indent).collect::<Vec<_>>(), [0, 1, 0]);
        assert_eq!(l[2].heading, 3);
        assert_eq!(toc_lines(&hs, 6).len(), 4);
    }
}
