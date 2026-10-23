//! F2 inline reveal (prompt 016 §3 item 1, ADR-0020): the active block's text is shown rendered,
//! with Markdown markers visible only near the caret. Pure: builds a layout string + source map
//! ([`ParaText`]) from the parser runs of the block text; the layout crate turns it into a paragraph.
//!
//! Offsets are relative to the block text (the caller shifts them by the block start).
// Offsets are `u32` by design (ADR-0007); indices come from the builder and are in bounds.
#![allow(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::ops::Range;

use super::ReadOpts;
use super::detail::{Alert, Detail, NO_LINK, NodeKind, RunKind, style};
use super::index::{BlockKind, Index};
use super::text::{ParaText, Seg};

/// Inline style flags that make a run part of a construct (its markers reveal together).
const INLINE: u16 = style::STRONG
    | style::EM
    | style::STRIKE
    | style::CODE
    | style::LINK
    | style::SUP
    | style::SUB
    | style::MUTED
    | style::IMAGE
    | style::FOOTREF
    | style::MATH
    | style::HIGHLIGHT
    | style::UNDERLINE
    | style::KBD
    | style::ABBR;

/// One piece of the block text in source order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    /// Rendered text: verbatim source slice with style flags.
    Text {
        /// Source range.
        range: Range<u32>,
        /// Style flags.
        style: u16,
        /// Link index.
        link: u32,
    },
    /// A run whose display differs from its source (entity, escape, emoji, image chip):
    /// shows `display` when hidden, its source when revealed.
    Atom {
        /// Source range.
        range: Range<u32>,
        /// Hidden display.
        display: String,
        /// Style flags.
        style: u16,
        /// Construct.
        group: u32,
    },
    /// Source bytes no run covers (markers).
    Mark {
        /// Source range.
        range: Range<u32>,
        /// Construct (inline) or line number (line markers).
        group: u32,
        /// Line marker (`## `, `> `, `- `, …): revealed with the caret on its line.
        line: bool,
        /// Display while hidden (`• `, `☐ `, callout title …; empty = nothing).
        display: String,
    },
    /// A line break between lines (always shown).
    Break {
        /// Source range of the `\n` (or `\r\n`).
        range: Range<u32>,
    },
}

/// Analysis of one block's text for F2 reveal.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Analysis {
    /// Pieces in source order (they tile the text).
    pub pieces: Vec<Piece>,
    /// Construct extents (source ranges), indexed by group.
    pub groups: Vec<Range<u32>>,
    /// Line start offsets.
    pub lines: Vec<u32>,
    /// Heading level (0 = none).
    pub heading: u8,
    /// Block quote / callout.
    pub quote: bool,
}

/// The block kinds that use inline reveal; everything else keeps block-level reveal (P-47).
pub fn inline_kind(k: BlockKind) -> bool {
    matches!(
        k,
        BlockKind::Paragraph
            | BlockKind::Heading(_)
            | BlockKind::BlockQuote { .. }
            | BlockKind::List
    )
}

fn u(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn alert_title(a: Alert) -> &'static str {
    match a {
        Alert::Note => "ⓘ Note",
        Alert::Tip => "💡 Tip",
        Alert::Important => "❗ Important",
        Alert::Warning => "⚠ Warning",
        Alert::Caution => "⛔ Caution",
    }
}

/// Hidden display of a line marker (`- ` → `• `, `- [ ] ` → `☐ `, `1. ` stays, `## `/`> ` vanish).
fn line_display(s: &str, alert: Option<Alert>) -> String {
    let indent: String = s.chars().take_while(|c| *c == ' ').collect();
    let t = s.trim_start_matches(' ');
    // Strip quote markers.
    let mut t = t;
    while let Some(r) = t.strip_prefix('>') {
        t = r.trim_start_matches(' ');
    }
    if let Some(a) = alert
        && t.trim_end().starts_with("[!")
    {
        return alert_title(a).to_string();
    }
    let b = t.as_bytes();
    if let Some(&c) = b.first()
        && matches!(c, b'-' | b'*' | b'+')
        && b.get(1) == Some(&b' ')
    {
        let rest = t[2..].trim_start();
        if rest.starts_with("[ ]") {
            return format!("{indent}☐ ");
        }
        if rest.starts_with("[x]") || rest.starts_with("[X]") {
            return format!("{indent}☑ ");
        }
        return format!("{indent}• ");
    }
    let digits = t.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 0 && matches!(b.get(digits), Some(b'.' | b')')) {
        return format!("{indent}{} ", &t[..=digits]);
    }
    if t.starts_with('#') {
        return String::new();
    }
    indent
}

/// Analyses `text` (one block). `None` when it is not a single inline-reveal block (the caller
/// keeps block-level reveal: source in place).
pub fn analyze(text: &str, opts: ReadOpts) -> Option<Analysis> {
    if text.is_empty() {
        return Some(Analysis {
            lines: vec![0],
            ..Analysis::default()
        });
    }
    let idx = Index::build(text, opts);
    if idx.blocks.len() != 1 || !inline_kind(idx.blocks.first()?.kind) {
        return None;
    }
    if super::toc::is_marker(text) {
        return None;
    }
    let d = Detail::build(text, &idx, 0, opts);
    Some(from_detail(text, &d))
}

fn from_detail(text: &str, d: &Detail) -> Analysis {
    let mut a = Analysis::default();
    let mut alert = None;
    for n in &d.nodes {
        match &n.kind {
            NodeKind::Heading(l) if a.heading == 0 => a.heading = *l,
            NodeKind::BlockQuote(al) => {
                a.quote = true;
                alert = alert.or(*al);
            }
            _ => {}
        }
    }
    a.lines.push(0);
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            a.lines.push(u(i + 1));
        }
    }
    // Runs sorted by start; gaps between them become marks / breaks.
    let mut runs: Vec<&super::detail::Run> = d
        .runs
        .iter()
        .filter(|r| r.range.end as usize <= text.len() && r.range.start < r.range.end)
        .collect();
    runs.sort_by_key(|r| r.range.start);
    let mut raw: Vec<Piece> = Vec::new();
    let mut pos = 0u32;
    let gap = |raw: &mut Vec<Piece>, s: u32, e: u32| {
        let mut p = s;
        let bytes = text.as_bytes();
        while p < e {
            // Split at line breaks.
            let nl = (p..e).find(|&i| bytes[i as usize] == b'\n');
            match nl {
                Some(n) => {
                    let mut ms = n;
                    if ms > p && bytes[(ms - 1) as usize] == b'\r' {
                        ms -= 1;
                    }
                    if ms > p {
                        raw.push(Piece::Mark {
                            range: p..ms,
                            group: 0,
                            line: false,
                            display: String::new(),
                        });
                    }
                    raw.push(Piece::Break { range: ms..n + 1 });
                    p = n + 1;
                }
                None => {
                    raw.push(Piece::Mark {
                        range: p..e,
                        group: 0,
                        line: false,
                        display: String::new(),
                    });
                    p = e;
                }
            }
        }
    };
    for r in runs {
        let (s, e) = (r.range.start, r.range.end);
        if s < pos {
            continue;
        }
        gap(&mut raw, pos, s);
        let src = &text[s as usize..e as usize];
        if r.kind != RunKind::Text {
            // Soft / hard breaks keep the source text (a `\n`, maybe with the hard-break spaces).
            raw.push(Piece::Break { range: s..e });
        } else if let Some(owned) = &r.owned {
            // Inline code `` `x` ``: split the delimiters off as marks, keep the body verbatim.
            match src.find(&**owned) {
                Some(k) if !owned.is_empty() && src[k + owned.len()..].len() == k && k > 0 => {
                    let (b0, b1) = (s + u(k), s + u(k + owned.len()));
                    gap(&mut raw, s, b0);
                    raw.push(Piece::Text {
                        range: b0..b1,
                        style: r.style,
                        link: r.link,
                    });
                    gap(&mut raw, b1, e);
                }
                _ => raw.push(Piece::Atom {
                    range: s..e,
                    display: owned.to_string(),
                    style: r.style,
                    group: 0,
                }),
            }
        } else if r.style & style::MATH != 0 {
            let open = src.bytes().take_while(|b| *b == b'$').count();
            let close = src.bytes().rev().take_while(|b| *b == b'$').count();
            if open > 0 && open == close && open * 2 < src.len() {
                gap(&mut raw, s, s + u(open));
                raw.push(Piece::Text {
                    range: s + u(open)..e - u(close),
                    style: r.style,
                    link: r.link,
                });
                gap(&mut raw, e - u(close), e);
            } else {
                raw.push(Piece::Text {
                    range: s..e,
                    style: r.style,
                    link: r.link,
                });
            }
        } else {
            raw.push(Piece::Text {
                range: s..e,
                style: r.style,
                link: r.link,
            });
        }
        pos = e;
    }
    gap(&mut raw, pos, u(text.len()));
    // Line markers: a mark at the start of a line (before the line's first text).
    let mut at_line_start = true;
    for p in &mut raw {
        match p {
            Piece::Break { .. } => at_line_start = true,
            Piece::Mark {
                range,
                line,
                group,
                display,
            } if at_line_start => {
                let li = a
                    .lines
                    .partition_point(|&l| l <= range.start)
                    .saturating_sub(1);
                *line = true;
                *group = u(li);
                *display = line_display(&text[range.start as usize..range.end as usize], alert);
                at_line_start = false;
            }
            _ => at_line_start = false,
        }
    }
    // Constructs: maximal stretches of inline-styled text, atoms and non-line marks on one line.
    let mut cur: Option<u32> = None;
    for p in &mut raw {
        let joins = match p {
            Piece::Text { style, .. } => *style & INLINE != 0,
            Piece::Atom { .. } => true,
            Piece::Mark { line, .. } => !*line,
            Piece::Break { .. } => false,
        };
        if !joins {
            cur = None;
            continue;
        }
        let r = match p {
            Piece::Text { range, .. } | Piece::Atom { range, .. } | Piece::Mark { range, .. } => {
                range.clone()
            }
            Piece::Break { range } => range.clone(),
        };
        let g = match cur {
            Some(g) => {
                if let Some(x) = a.groups.get_mut(g as usize) {
                    x.end = r.end;
                }
                g
            }
            None => {
                a.groups.push(r.clone());
                u(a.groups.len() - 1)
            }
        };
        cur = Some(g);
        match p {
            Piece::Atom { group, .. } => *group = g,
            Piece::Mark { group, .. } => *group = g,
            _ => {}
        }
    }
    a.pieces = raw;
    a
}

/// Line index of offset `p`.
fn line_of(a: &Analysis, p: u32) -> u32 {
    u(a.lines.partition_point(|&l| l <= p).saturating_sub(1))
}

/// Reveal flags per piece for a selection (both ends count; ADR-0020 §2).
pub fn reveal_set(a: &Analysis, sel: Range<u32>) -> Vec<bool> {
    let ends = [sel.start, sel.end];
    let lines = [line_of(a, sel.start), line_of(a, sel.end)];
    let group_on = |g: u32| {
        a.groups
            .get(g as usize)
            .is_some_and(|r| ends.iter().any(|&c| r.start <= c && c <= r.end))
    };
    a.pieces
        .iter()
        .map(|p| match p {
            Piece::Mark {
                line: true, group, ..
            } => lines.contains(group),
            Piece::Mark { group, .. } | Piece::Atom { group, .. } => group_on(*group),
            _ => false,
        })
        .collect()
}

/// The reveal paragraph: layout string + map, marker ranges (UTF-16) and the revealed marker texts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RevealPara {
    /// Layout string and source map (block-relative source offsets).
    pub text: ParaText,
    /// UTF-16 ranges drawn in `marker` color.
    pub marks: Vec<Range<u32>>,
    /// Source text of every revealed marker, in order (dev-tools probe).
    pub shown: Vec<String>,
}

/// Builds the reveal paragraph for `shown` flags (from [`reveal_set`]).
pub fn build(a: &Analysis, text: &str, shown: &[bool]) -> RevealPara {
    let mut out = RevealPara::default();
    let push = |out: &mut RevealPara,
                s: &str,
                src: Range<u32>,
                verbatim: bool,
                st: u16,
                link: u32,
                mark: bool| {
        if s.is_empty() {
            return;
        }
        let u16_start = out.text.len16();
        let len16 = u(s.encode_utf16().count());
        out.text.segs.push(Seg {
            u16_start,
            u16_len: len16,
            byte_start: u(out.text.text.len()),
            src,
            verbatim,
            style: st,
            link,
        });
        out.text.text.push_str(s);
        if mark {
            out.marks.push(u16_start..u16_start + len16);
        }
    };
    for (i, p) in a.pieces.iter().enumerate() {
        let on = shown.get(i).copied().unwrap_or(false);
        match p {
            Piece::Text { range, style, link } => {
                let s = &text[range.start as usize..range.end as usize];
                push(&mut out, s, range.clone(), true, *style, *link, false);
            }
            Piece::Break { range } => {
                push(&mut out, "\n", range.clone(), false, 0, NO_LINK, false);
            }
            Piece::Atom {
                range,
                display,
                style,
                ..
            } => {
                if on {
                    let s = &text[range.start as usize..range.end as usize];
                    out.shown.push(s.to_string());
                    push(&mut out, s, range.clone(), true, 0, NO_LINK, true);
                } else {
                    push(
                        &mut out,
                        display,
                        range.clone(),
                        false,
                        *style,
                        NO_LINK,
                        false,
                    );
                }
            }
            Piece::Mark { range, display, .. } => {
                let s = &text[range.start as usize..range.end as usize];
                if on {
                    out.shown.push(s.to_string());
                    push(&mut out, s, range.clone(), true, 0, NO_LINK, true);
                } else if !display.is_empty() {
                    push(&mut out, display, range.clone(), false, 0, NO_LINK, false);
                }
            }
        }
    }
    if out.text.segs.is_empty() {
        // Empty block: one empty verbatim segment so the caret maps.
        out.text.segs.push(Seg {
            u16_start: 0,
            u16_len: 0,
            byte_start: 0,
            src: 0..0,
            verbatim: true,
            style: 0,
            link: NO_LINK,
        });
    }
    out
}

/// Caret snapping (ADR-0020 §3): a position strictly inside a hidden mark or atom moves to its
/// end when moving forward, to its start otherwise.
pub fn snap(a: &Analysis, shown: &[bool], pos: u32, forward: bool) -> u32 {
    for (i, p) in a.pieces.iter().enumerate() {
        let r = match p {
            Piece::Mark { range, .. } | Piece::Atom { range, .. } => range,
            _ => continue,
        };
        if r.start < pos && pos < r.end && !shown.get(i).copied().unwrap_or(false) {
            return if forward { r.end } else { r.start };
        }
    }
    pos
}

#[cfg(test)]
#[path = "tests_reveal.rs"]
mod tests;
