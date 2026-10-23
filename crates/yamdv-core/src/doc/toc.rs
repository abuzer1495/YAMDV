//! `[TOC]` / `[[_TOC_]]` (prompt 014 §3 item 5): a paragraph that is exactly the marker renders
//! as a "Contents" title and a nested list of links to the document's headings, down to
//! `sidebar.toc_max_depth`. The source keeps the marker; the tree is rebuilt with the detail
//! cache, so it follows heading edits.

use super::detail::{Detail, LinkDest, NO_LINK, Node, NodeKind, Run, RunKind, style};
use super::index::Index;
use std::ops::Range;

/// The paragraph text is a TOC marker.
pub fn is_marker(text: &str) -> bool {
    matches!(text.trim(), "[TOC]" | "[[_TOC_]]")
}

/// Title shown above the list.
pub const TITLE: &str = "Contents";

/// `(level, text, slug)` of the headings listed at `depth` (0 → 3).
pub fn entries(index: &Index, depth: u8) -> Vec<(u8, &str, &str)> {
    let depth = if depth == 0 { 3 } else { depth };
    index
        .headings
        .iter()
        .filter(|h| h.level <= depth)
        .map(|h| (h.level, h.text.as_str(), h.slug.as_str()))
        .collect()
}

fn u(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Detail tree of a TOC block at source `range`.
pub fn detail(index: &Index, range: Range<u32>, depth: u8) -> Detail {
    let mut d = Detail::default();
    let run = |d: &mut Detail, text: &str, st: u16, link: u32| {
        d.runs.push(Run {
            kind: RunKind::Text,
            range: range.clone(),
            owned: Some(text.into()),
            style: st,
            link,
        });
    };
    let node = |d: &mut Detail, kind: NodeKind, parent: u32, depth: u16| -> usize {
        let r = u(d.runs.len());
        d.nodes.push(Node {
            kind,
            range: range.clone(),
            parent,
            end: 0,
            depth,
            runs: r..r,
        });
        d.nodes.len() - 1
    };
    let title = node(&mut d, NodeKind::Paragraph { implicit: false }, u32::MAX, 0);
    run(&mut d, TITLE, style::STRONG, NO_LINK);
    close(&mut d, title);
    let list = entries(index, depth);
    if list.is_empty() {
        return d;
    }
    let base = list.iter().map(|e| e.0).min().unwrap_or(1);
    // Open lists and items: (level, list node, item node).
    let mut stack: Vec<(u8, usize, Option<usize>)> = Vec::new();
    for (level, text, slug) in list {
        let level = level.max(base);
        while stack.last().is_some_and(|s| s.0 > level) {
            if let Some((_, l, it)) = stack.pop() {
                if let Some(it) = it {
                    close(&mut d, it);
                }
                close(&mut d, l);
            }
        }
        // Deeper level: nest a list inside the open item (or a fresh one).
        if stack.last().is_none_or(|s| s.0 < level) {
            let (parent, dep) = match stack.last() {
                Some(&(_, l, Some(it))) => (u(it), d.nodes.get(l).map_or(0, |n| n.depth) + 2),
                Some(&(_, l, None)) => (u(l), d.nodes.get(l).map_or(0, |n| n.depth) + 1),
                None => (u32::MAX, 0),
            };
            let l = node(
                &mut d,
                NodeKind::List {
                    start: None,
                    tight: true,
                },
                parent,
                dep,
            );
            stack.push((level, l, None));
        }
        let Some(top) = stack.last_mut() else {
            continue;
        };
        if let Some(it) = top.2.take() {
            close(&mut d, it);
        }
        let l = top.1;
        let dep = d.nodes.get(l).map_or(0, |n| n.depth);
        let it = node(&mut d, NodeKind::Item { task: None }, u(l), dep + 1);
        let p = node(
            &mut d,
            NodeKind::Paragraph { implicit: true },
            u(it),
            dep + 2,
        );
        let li = u(d.links.len());
        d.links.push(LinkDest {
            dest: format!("#{slug}").into(),
            title: "".into(),
            wiki: false,
        });
        run(&mut d, text, style::LINK, li);
        close(&mut d, p);
        if let Some(top) = stack.last_mut() {
            top.2 = Some(it);
        }
    }
    while let Some((_, l, it)) = stack.pop() {
        if let Some(it) = it {
            close(&mut d, it);
        }
        close(&mut d, l);
    }
    d
}

/// Finishes node `i`: its descendants and runs end here.
fn close(d: &mut Detail, i: usize) {
    let (end, runs) = (u(d.nodes.len()), u(d.runs.len()));
    if let Some(n) = d.nodes.get_mut(i) {
        n.end = end;
        n.runs.end = if n.kind.is_inline_leaf() {
            runs
        } else {
            n.runs.start
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::{Document, ReadOpts};

    #[test]
    fn markers() {
        assert!(is_marker("[TOC]"));
        assert!(is_marker(" [[_TOC_]] "));
        assert!(!is_marker("[toc] x"));
    }

    #[test]
    fn nested_links() {
        let src = "[TOC]\n\n# A\n\n## B\n\n### C\n\n#### D\n\n## E\n";
        let d = Document::new(src.into(), ReadOpts::default());
        let t = detail(&d.index, 0..5, 3);
        let texts: Vec<&str> = t.runs.iter().map(|r| r.text(src)).collect();
        assert_eq!(texts, ["Contents", "A", "B", "C", "E"]);
        let dests: Vec<&str> = t.links.iter().map(|l| &*l.dest).collect();
        assert_eq!(dests, ["#a", "#b", "#c", "#e"]);
        // Nesting: B's list is inside A's item, C's inside B's.
        let lists = t
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::List { .. }))
            .count();
        assert_eq!(lists, 3);
        assert!(t.nodes.iter().enumerate().all(|(i, n)| n.end as usize > i));
    }
}
