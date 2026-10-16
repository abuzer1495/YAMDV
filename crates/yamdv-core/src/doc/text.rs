//! Layout strings with layout ↔ source mapping (§4.3 item 4) and text extraction (§4.3 item 6,
//! §4.6 copy rules).
// Arena/segment indices in this module are produced by the builder itself and are in bounds by
// construction; byte offsets are `u32` by design (documents ≤ 256 MiB, R-OPEN-08). Decision recorded
// in ADR-0007.
#![allow(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::ops::Range;

use super::Document;
use super::detail::{Detail, NodeKind, Run, RunKind};

/// A segment of a layout string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seg {
    /// UTF-16 start in the layout string.
    pub u16_start: u32,
    /// UTF-16 length.
    pub u16_len: u32,
    /// Byte start in the layout string.
    pub byte_start: u32,
    /// Source range.
    pub src: Range<u32>,
    /// The segment text equals the source slice (character-exact mapping).
    pub verbatim: bool,
    /// Style flags.
    pub style: u16,
}

/// Layout string of one inline leaf.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParaText {
    /// Text handed to the shaper (`\n` for line breaks).
    pub text: String,
    /// Segments in order.
    pub segs: Vec<Seg>,
}

fn u(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

impl ParaText {
    /// Sub-string `r` (layout bytes, on char boundaries) with its segments clipped; verbatim
    /// segments keep an exact source mapping.
    pub fn slice_bytes(&self, r: Range<usize>) -> ParaText {
        let mut p = ParaText {
            text: self.text.get(r.clone()).unwrap_or("").to_string(),
            segs: Vec::new(),
        };
        let mut u16_pos = 0u32;
        for s in &self.segs {
            let sb = s.byte_start as usize;
            let st = self.text.get(sb..).unwrap_or("");
            let seg_text_len = st
                .char_indices()
                .scan(0u32, |n, (i, c)| {
                    if *n >= s.u16_len {
                        return None;
                    }
                    *n += c.len_utf16() as u32;
                    Some(i + c.len_utf8())
                })
                .last()
                .unwrap_or(0);
            let se = sb + seg_text_len;
            let a = sb.max(r.start);
            let b = se.min(r.end);
            if a >= b {
                continue;
            }
            let t = self.text.get(a..b).unwrap_or("");
            let len16 = t.encode_utf16().count() as u32;
            let src = if s.verbatim {
                (s.src.start + (a - sb) as u32)..(s.src.start + (b - sb) as u32)
            } else {
                s.src.clone()
            };
            p.segs.push(Seg {
                u16_start: u16_pos,
                u16_len: len16,
                byte_start: (a - r.start) as u32,
                src,
                verbatim: s.verbatim,
                style: s.style,
            });
            u16_pos += len16;
        }
        p
    }

    /// Source range covered by the segments.
    pub fn source_range(&self) -> Range<u32> {
        match (self.segs.first(), self.segs.last()) {
            (Some(a), Some(b)) => a.src.start..b.src.end,
            _ => 0..0,
        }
    }

    /// Builds the layout string from runs. Soft breaks become a space, or `\n` with
    /// `single_newline_break` (R-MD-15); hard breaks become `\n`.
    pub fn from_runs(source: &str, runs: &[Run], single_newline_break: bool) -> Self {
        let mut p = ParaText::default();
        let mut u16_pos = 0u32;
        for r in runs {
            let (t, verbatim): (&str, bool) = match r.kind {
                RunKind::Text => (r.text(source), r.owned.is_none()),
                RunKind::SoftBreak => (if single_newline_break { "\n" } else { " " }, false),
                RunKind::HardBreak => ("\n", false),
            };
            if t.is_empty() {
                continue;
            }
            let len16 = u(t.encode_utf16().count());
            p.segs.push(Seg {
                u16_start: u16_pos,
                u16_len: len16,
                byte_start: u(p.text.len()),
                src: r.range.clone(),
                verbatim,
                style: r.style,
            });
            p.text.push_str(t);
            u16_pos += len16;
        }
        p
    }

    /// Code text: runs concatenated verbatim (code blocks keep their own line breaks).
    pub fn code(source: &str, runs: &[Run]) -> Self {
        let mut p = Self::from_runs(source, runs, true);
        if p.text.ends_with('\n') {
            p.text.pop();
            if let Some(s) = p.segs.last_mut() {
                s.u16_len = s.u16_len.saturating_sub(1);
            }
        }
        p
    }

    /// UTF-16 length.
    pub fn len16(&self) -> u32 {
        self.segs.last().map_or(0, |s| s.u16_start + s.u16_len)
    }

    /// Layout position (UTF-16) → source byte offset.
    pub fn to_source(&self, idx16: u32) -> u32 {
        let Some(seg) = self.segs.iter().rev().find(|s| s.u16_start <= idx16) else {
            return self.segs.first().map_or(0, |s| s.src.start);
        };
        let off = idx16 - seg.u16_start;
        if off >= seg.u16_len {
            return seg.src.end;
        }
        if !seg.verbatim {
            return if off == 0 { seg.src.start } else { seg.src.end };
        }
        let text = &self.text[seg.byte_start as usize..];
        let mut n16 = 0u32;
        for (b, c) in text.char_indices() {
            if n16 >= off {
                return seg.src.start + u(b);
            }
            n16 += u(c.len_utf16());
        }
        seg.src.end
    }

    /// Source byte offset → layout position (UTF-16), clamped into this paragraph.
    pub fn from_source(&self, pos: u32) -> u32 {
        for seg in &self.segs {
            if pos < seg.src.start {
                return seg.u16_start;
            }
            if pos < seg.src.end {
                if !seg.verbatim {
                    return seg.u16_start;
                }
                let rel = (pos - seg.src.start) as usize;
                let text = &self.text[seg.byte_start as usize..];
                let mut n16 = 0u32;
                for (b, c) in text.char_indices() {
                    if b >= rel {
                        break;
                    }
                    n16 += u(c.len_utf16());
                }
                return seg.u16_start + n16;
            }
        }
        self.len16()
    }

    /// Substring between two UTF-16 positions.
    pub fn slice16(&self, a: u32, b: u32) -> &str {
        let to_byte = |p: u32| {
            let mut n = 0u32;
            for (i, c) in self.text.char_indices() {
                if n >= p {
                    return i;
                }
                n += u(c.len_utf16());
            }
            self.text.len()
        };
        let (x, y) = (to_byte(a.min(b)), to_byte(a.max(b)));
        &self.text[x..y]
    }
}

/// Builds the layout string of node `i` of `d`.
pub fn node_text(source: &str, d: &Detail, i: usize, single_newline_break: bool) -> ParaText {
    let runs = d.runs_of(i);
    match d.nodes.get(i).map(|n| &n.kind) {
        Some(NodeKind::Code { .. }) => ParaText::code(source, runs),
        _ => ParaText::from_runs(source, runs, single_newline_break),
    }
}

struct Piece {
    text: String,
    list_line: bool,
    group: usize,
}

fn clip<'a>(source: &'a str, r: &Range<u32>, sel: &Range<u32>) -> &'a str {
    let a = r.start.max(sel.start) as usize;
    let b = r.end.min(sel.end) as usize;
    if a >= b {
        return "";
    }
    source.get(a..b).unwrap_or("")
}

fn walk(
    doc: &Document,
    d: &Detail,
    i: usize,
    sel: &Range<u32>,
    lists: &mut Vec<(bool, u64)>,
    prefix: &mut Option<String>,
    out: &mut Vec<Piece>,
) {
    let n = &d.nodes[i];
    if n.range.end < sel.start || n.range.start > sel.end {
        return;
    }
    let indent = |lists: &Vec<(bool, u64)>| "  ".repeat(lists.len().saturating_sub(1));
    match &n.kind {
        NodeKind::List { start, .. } => {
            lists.push((start.is_some(), start.unwrap_or(1)));
            for c in d.children(i) {
                walk(doc, d, c, sel, lists, prefix, out);
                if let Some(l) = lists.last_mut() {
                    l.1 += 1;
                }
            }
            lists.pop();
        }
        NodeKind::Item { .. } => {
            let (ordered, num) = lists.last().copied().unwrap_or((false, 1));
            let marker = if ordered {
                format!("{num}. ")
            } else {
                "• ".to_string()
            };
            *prefix = Some(format!("{}{marker}", indent(lists)));
            let before = out.len();
            for c in d.children(i) {
                walk(doc, d, c, sel, lists, prefix, out);
            }
            *prefix = None;
            for p in out.iter_mut().skip(before) {
                p.list_line = true;
            }
        }
        NodeKind::TableRow { .. } => {
            // §4.6: a table row copies as tab-separated cells, one row per line.
            let mut cells = Vec::new();
            for c in d.children(i) {
                let pt = node_text(&doc.source, d, c, doc.opts.single_newline_break);
                let a = pt.from_source(sel.start);
                let b = pt.from_source(sel.end);
                cells.push(pt.slice16(a, b).to_string());
            }
            if cells.iter().any(|c| !c.is_empty()) {
                out.push(Piece {
                    text: cells.join("\t"),
                    list_line: true,
                    group: 0,
                });
            }
        }
        NodeKind::Placeholder(_) | NodeKind::Rule => {
            let t = clip(&doc.source, &n.range, sel);
            if !t.trim().is_empty() && !matches!(n.kind, NodeKind::Rule) {
                out.push(Piece {
                    text: t.trim_end_matches(['\n', '\r']).to_string(),
                    list_line: false,
                    group: 0,
                });
            }
        }
        k if k.is_inline_leaf() => {
            let pt = node_text(&doc.source, d, i, doc.opts.single_newline_break);
            let a = pt.from_source(sel.start);
            let b = pt.from_source(sel.end);
            let t = pt.slice16(a, b);
            if !t.is_empty() {
                let mut s = match prefix.take() {
                    Some(p) => p,
                    None if !lists.is_empty() => {
                        format!("{}  ", "  ".repeat(lists.len().saturating_sub(1)))
                    }
                    None => String::new(),
                };
                s.push_str(t);
                out.push(Piece {
                    text: s,
                    list_line: false,
                    group: 0,
                });
            }
        }
        _ => {
            for c in d.children(i) {
                walk(doc, d, c, sel, lists, prefix, out);
            }
        }
    }
}

/// Plain text of the source range `sel` per the §4.6 copy rules (CRLF line breaks, a blank line
/// between blocks, list items on their own lines with `• ` / `<n>. ` and 2-space indentation per
/// level, code verbatim, placeholders copy their source). Builds detail trees on demand without
/// caching them (they are released afterwards).
pub fn copy_text(doc: &Document, sel: Range<u32>) -> String {
    let mut pieces: Vec<Piece> = Vec::new();
    let first = doc.index.block_at(sel.start);
    let mut group = 0usize;
    for bi in first..doc.index.blocks.len() {
        if doc.index.blocks[bi].range.start >= sel.end {
            break;
        }
        let d = doc.detail_uncached(bi);
        for r in d.roots() {
            let before = pieces.len();
            walk(doc, &d, r, &sel, &mut Vec::new(), &mut None, &mut pieces);
            group += 1;
            for p in pieces.iter_mut().skip(before) {
                p.group = group;
            }
        }
    }
    let mut out = String::new();
    let mut prev: Option<(bool, usize)> = None;
    for p in &pieces {
        if let Some((pl, pg)) = prev {
            out.push_str(if pl && p.list_line && pg == p.group {
                "\r\n"
            } else {
                "\r\n\r\n"
            });
        }
        prev = Some((p.list_line, p.group));
        // Normalize every line break inside a piece to CRLF.
        let mut chars = p.text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\r' if chars.peek() == Some(&'\n') => {}
                '\n' => out.push_str("\r\n"),
                c => out.push(c),
            }
        }
    }
    out
}

/// Plain text of the whole document (`Ctrl+A`).
pub fn all_text(doc: &Document) -> String {
    copy_text(doc, 0..u(doc.source.len()))
}

#[cfg(test)]
mod tests {
    use super::super::ReadOpts;
    use super::*;

    fn doc(s: &str) -> Document {
        Document::new(s.to_string(), ReadOpts::default())
    }

    #[test]
    fn mapping_roundtrip() {
        let src = "Hé *wörld* `co de` 日本😀 a&amp;b\nnext";
        let mut d = doc(src);
        let det = d.detail(0);
        let pt = node_text(src, &det, 0, false);
        assert_eq!(pt.text, "Hé wörld co de 日本😀 a&b next");
        // Every verbatim character maps back and forth exactly.
        for (b, c) in src.char_indices() {
            let p16 = pt.from_source(u(b));
            let back = pt.to_source(p16);
            let seg = pt
                .segs
                .iter()
                .find(|s| s.src.start <= u(b) && u(b) < s.src.end);
            if let Some(s) = seg
                && s.verbatim
            {
                assert_eq!(back, u(b), "char {c:?} at {b}");
            }
        }
        let w = pt
            .text
            .find("wörld")
            .map(|b| u(pt.text[..b].encode_utf16().count()))
            .unwrap_or(0);
        assert_eq!(pt.to_source(w), u(src.find("wörld").unwrap_or(0)));
        let emoji16 = u(pt.text[..pt.text.find('😀').unwrap_or(0)]
            .encode_utf16()
            .count());
        assert_eq!(
            pt.to_source(emoji16 + 2),
            u(src.find(" a&amp;").unwrap_or(0))
        );
        // Owned segments map to their range ends.
        let amp = pt
            .text
            .find("a&b")
            .map(|b| u(pt.text[..b].encode_utf16().count()))
            .unwrap_or(0);
        assert!(pt.to_source(amp + 1) >= u(src.find("&amp;").unwrap_or(0)));
    }

    #[test]
    fn r_md_15_single_newline_break() {
        let src = "a\nb";
        let mut d = Document::new(
            src.into(),
            ReadOpts {
                single_newline_break: true,
                ..ReadOpts::default()
            },
        );
        let det = d.detail(0);
        assert_eq!(node_text(src, &det, 0, true).text, "a\nb");
        assert_eq!(all_text(&d), "a\r\nb");
    }

    #[test]
    fn copy_rules() {
        let src = "# Title\n\nPara *one*\nline\n\n- a\n- b\n  1. x\n  2. y\n- c\n\n5. five\n6. six\n\n```\nfn x() {\n    1\n}\n```\n\n|t|u|\n|-|-|\n|1|2|\n\n> quote\n";
        let d = doc(src);
        let t = all_text(&d);
        assert_eq!(
            t,
            "Title\r\n\r\nPara one line\r\n\r\n• a\r\n• b\r\n  1. x\r\n  2. y\r\n• c\r\n\r\n5. five\r\n6. six\r\n\r\nfn x() {\r\n    1\r\n}\r\n\r\nt\tu\r\n1\t2\r\n\r\nquote"
        );
    }

    #[test]
    fn copy_partial_selection() {
        let src = "alpha beta\n\ngamma delta\n";
        let d = doc(src);
        let a = u(src.find("beta").unwrap_or(0));
        let b = u(src.find(" delta").unwrap_or(0));
        assert_eq!(copy_text(&d, a..b), "beta\r\n\r\ngamma");
    }

    #[test]
    fn loose_list_second_paragraph_indented() {
        let src = "- a\n\n  b\n";
        assert_eq!(all_text(&doc(src)), "• a\r\n  b");
    }
}
