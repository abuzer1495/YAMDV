//! HTML fragment writer for the clipboard (§4.9, R-REN-16): everything escaped, only sanitized
//! constructs, images only as their alt text, links only for validated http/https/mailto URLs.

// Node and block indices come from the trees' own arenas (in bounds by construction).
#![allow(clippy::indexing_slicing)]

use std::fmt::Write as _;
use std::ops::Range;

use yamdv_core::doc::Document;
use yamdv_core::doc::detail::{Detail, NodeKind, RunKind, style};
use yamdv_core::link::{Target, classify};

/// Escapes text for HTML element content and attribute values.
pub fn escape(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
}

fn clip(src: &str, r: &Range<u32>, sel: &Range<u32>, text: &str, verbatim: bool) -> Option<String> {
    if r.end <= sel.start || r.start >= sel.end {
        // Zero-width runs (breaks, HTML images) inside the selection still count.
        return (r.start == r.end && r.start >= sel.start && r.start < sel.end)
            .then(|| text.to_string());
    }
    if !verbatim {
        return Some(text.to_string());
    }
    let a = r.start.max(sel.start) as usize;
    let b = r.end.min(sel.end) as usize;
    src.get(a..b).map(str::to_string)
}

fn inline(doc: &Document, d: &Detail, i: usize, sel: &Range<u32>, out: &mut String) -> bool {
    let mut any = false;
    for r in d.runs_of(i) {
        let text = match r.kind {
            RunKind::Text => r.text(&doc.source),
            RunKind::SoftBreak => " ",
            RunKind::HardBreak => "\n",
        };
        let Some(t) = clip(&doc.source, &r.range, sel, text, r.owned.is_none()) else {
            continue;
        };
        any = true;
        if r.kind == RunKind::HardBreak {
            out.push_str("<br>");
            continue;
        }
        let mut close: Vec<&str> = Vec::new();
        let f = r.style;
        if f & style::LINK != 0
            && let Some(l) = d.links.get(r.link as usize)
            && let Target::External(u) = classify(&l.dest, l.wiki)
        {
            out.push_str("<a href=\"");
            escape(&u.serialize(), out);
            out.push_str("\">");
            close.push("</a>");
        }
        for (flag, open, end) in [
            (style::STRONG, "<strong>", "</strong>"),
            (style::EM, "<em>", "</em>"),
            (style::STRIKE, "<del>", "</del>"),
            (style::CODE | style::KBD, "<code>", "</code>"),
            (style::HIGHLIGHT, "<mark>", "</mark>"),
            (style::SUP, "<sup>", "</sup>"),
            (style::SUB, "<sub>", "</sub>"),
            (style::UNDERLINE, "<u>", "</u>"),
        ] {
            if f & flag != 0 {
                out.push_str(open);
                close.push(end);
            }
        }
        if f & style::IMAGE != 0 {
            // Images only as alt text: "▣\u{2009}alt" → "alt" (S-RES-05: nothing embedded).
            let alt = t.strip_prefix("\u{25A3}\u{2009}").unwrap_or(&t);
            escape(alt, out);
        } else {
            escape(&t, out);
        }
        for c in close.iter().rev() {
            out.push_str(c);
        }
    }
    any
}

fn node(doc: &Document, d: &Detail, i: usize, sel: &Range<u32>, out: &mut String) {
    let n = &d.nodes[i];
    if (n.range.end <= sel.start || n.range.start >= sel.end)
        && !matches!(n.kind, NodeKind::HtmlBlock | NodeKind::Align(_))
    {
        return;
    }
    let kids = |out: &mut String| {
        for c in d.children(i) {
            node(doc, d, c, sel, out);
        }
    };
    let wrap_inline = |tag: &str, out: &mut String| {
        let mut buf = String::new();
        if inline(doc, d, i, sel, &mut buf) && !buf.is_empty() {
            let _ = write!(out, "<{tag}>{buf}</{tag}>");
        }
    };
    match &n.kind {
        NodeKind::Paragraph { implicit: true } => {
            let _ = inline(doc, d, i, sel, out);
        }
        NodeKind::Paragraph { .. } => wrap_inline("p", out),
        NodeKind::Heading(l) => wrap_inline(&format!("h{}", (*l).clamp(1, 6)), out),
        NodeKind::DefTitle => wrap_inline("dt", out),
        NodeKind::DetailsSummary => wrap_inline("p", out),
        NodeKind::TableCell => wrap_inline("td", out),
        NodeKind::Code { .. } => {
            let mut buf = String::new();
            if inline(doc, d, i, sel, &mut buf) {
                let _ = write!(out, "<pre><code>{buf}</code></pre>");
            }
        }
        NodeKind::BlockQuote(_) => {
            out.push_str("<blockquote>");
            kids(out);
            out.push_str("</blockquote>");
        }
        NodeKind::List { start, .. } => {
            let tag = if start.is_some() { "ol" } else { "ul" };
            let _ = write!(out, "<{tag}>");
            kids(out);
            let _ = write!(out, "</{tag}>");
        }
        NodeKind::Item { .. } => {
            out.push_str("<li>");
            kids(out);
            out.push_str("</li>");
        }
        NodeKind::DefList => {
            out.push_str("<dl>");
            kids(out);
            out.push_str("</dl>");
        }
        NodeKind::DefBody => {
            out.push_str("<dd>");
            kids(out);
            out.push_str("</dd>");
        }
        NodeKind::Table { .. } => {
            out.push_str("<table>");
            kids(out);
            out.push_str("</table>");
        }
        NodeKind::TableRow { .. } => {
            out.push_str("<tr>");
            kids(out);
            out.push_str("</tr>");
        }
        NodeKind::Rule => out.push_str("<hr>"),
        NodeKind::Placeholder(_) => {
            let a = n.range.start.max(sel.start) as usize;
            let b = n.range.end.min(sel.end) as usize;
            out.push_str("<pre>");
            escape(doc.source.get(a..b).unwrap_or(""), out);
            out.push_str("</pre>");
        }
        NodeKind::FootnoteDef { .. } | NodeKind::Unsupported => {}
        NodeKind::HtmlBlock | NodeKind::Align(_) | NodeKind::Details { .. } => kids(out),
    }
}

/// HTML fragment for the selection `sel` (source byte range).
pub fn fragment(doc: &Document, sel: Range<u32>) -> String {
    let mut out = String::new();
    let first = doc.index.block_at(sel.start);
    for bi in first..doc.index.blocks.len() {
        if doc.index.blocks[bi].range.start >= sel.end {
            break;
        }
        let d = doc.detail_uncached(bi);
        for r in d.roots() {
            node(doc, &d, r, &sel, &mut out);
        }
    }
    out
}

/// Wraps a fragment in the `CF_HTML` clipboard format (offsets in UTF-8 bytes).
pub fn cf_html(fragment: &str) -> String {
    const HEADER: &str = "Version:0.9\r\nStartHTML:0000000000\r\nEndHTML:0000000000\r\nStartFragment:0000000000\r\nEndFragment:0000000000\r\n";
    let pre = "<html><body>\r\n<!--StartFragment-->";
    let post = "<!--EndFragment-->\r\n</body></html>";
    let start_html = HEADER.len();
    let start_frag = start_html + pre.len();
    let end_frag = start_frag + fragment.len();
    let end_html = end_frag + post.len();
    format!(
        "Version:0.9\r\nStartHTML:{start_html:010}\r\nEndHTML:{end_html:010}\r\nStartFragment:{start_frag:010}\r\nEndFragment:{end_frag:010}\r\n{pre}{fragment}{post}"
    )
}

/// "Copy as Markdown" (§4.9): the source slice covering `sel`, expanded to whole inline
/// constructs (every run the selection touches) and to whole lines for block-level edges.
pub fn markdown_slice(doc: &Document, sel: Range<u32>) -> String {
    let (mut a, mut b) = (sel.start, sel.end);
    let first = doc.index.block_at(sel.start);
    for bi in first..doc.index.blocks.len() {
        if doc.index.blocks[bi].range.start >= sel.end {
            break;
        }
        let d = doc.detail_uncached(bi);
        // Expand to the enclosing inline leaf node's construct boundaries: a run that is cut
        // extends the range to the start/end of its leaf's source (markers included).
        for n in &d.nodes {
            if !n.kind.is_inline_leaf() {
                continue;
            }
            let r = &n.range;
            if r.start < a && r.end > a {
                let cut = d
                    .runs
                    .iter()
                    .find(|x| x.range.start <= a && x.range.end > a);
                if cut.is_some_and(|x| x.style != 0) {
                    a = construct_start(&doc.source, r.start, a);
                }
            }
            if r.start < b && r.end > b {
                let cut = d
                    .runs
                    .iter()
                    .find(|x| x.range.start < b && x.range.end >= b);
                if cut.is_some_and(|x| x.style != 0) {
                    b = construct_end(&doc.source, b, r.end);
                }
            }
        }
    }
    doc.source
        .get(a as usize..b as usize)
        .unwrap_or("")
        .to_string()
}

/// Start of the inline construct around `pos`: the nearest preceding marker character run
/// (`*`, `_`, `~`, `` ` ``, `=`, `[`, `!`) within the leaf.
fn construct_start(src: &str, leaf_start: u32, pos: u32) -> u32 {
    let bytes = src.as_bytes();
    let mut p = pos as usize;
    while p > leaf_start as usize {
        let c = bytes.get(p - 1).copied().unwrap_or(b' ');
        if matches!(c, b'*' | b'_' | b'~' | b'`' | b'=' | b'[' | b'!' | b'^') {
            // Include the whole marker run.
            while p > leaf_start as usize
                && matches!(
                    bytes.get(p - 1),
                    Some(b'*' | b'_' | b'~' | b'`' | b'=' | b'[' | b'!' | b'^')
                )
            {
                p -= 1;
            }
            return u32::try_from(p).unwrap_or(pos);
        }
        if c == b' ' || c == b'\n' {
            break;
        }
        p -= 1;
    }
    pos.min(u32::try_from(p).unwrap_or(pos))
        .max(leaf_start)
        .min(pos)
}

fn construct_end(src: &str, pos: u32, leaf_end: u32) -> u32 {
    let bytes = src.as_bytes();
    let mut p = pos as usize;
    while p < leaf_end as usize {
        let c = bytes.get(p).copied().unwrap_or(b' ');
        if matches!(c, b'*' | b'_' | b'~' | b'`' | b'=' | b']' | b')' | b'^') {
            while p < leaf_end as usize
                && matches!(
                    bytes.get(p),
                    Some(b'*' | b'_' | b'~' | b'`' | b'=' | b']' | b')' | b'^')
                )
            {
                p += 1;
            }
            return u32::try_from(p).unwrap_or(pos);
        }
        if c == b' ' || c == b'\n' {
            break;
        }
        p += 1;
    }
    pos
}

#[cfg(test)]
mod tests {
    use super::*;
    use yamdv_core::doc::ReadOpts;

    fn doc(s: &str) -> Document {
        Document::new(s.to_string(), ReadOpts::default())
    }

    fn all(d: &Document) -> Range<u32> {
        0..u32::try_from(d.source.len()).unwrap_or(0)
    }

    #[test]
    fn fragment_escapes_and_sanitizes() {
        let d = doc(
            "# T \\<x>\n\nA **b** [l](https://e.com/?a=1&b=2) [j](javascript:x) ![alt & co](p.png)\n\n- one\n- two\n\n<script>bad()</script>\n",
        );
        let h = fragment(&d, all(&d));
        assert!(h.starts_with("<h1>T &lt;x&gt;</h1>"), "{h}");
        assert!(h.contains("<strong>b</strong>"), "{h}");
        assert!(
            h.contains("<a href=\"https://e.com/?a=1&amp;b=2\">l</a>"),
            "{h}"
        );
        assert!(!h.contains("javascript"), "{h}");
        assert!(h.contains("alt &amp; co") && !h.contains("<img"), "{h}");
        assert!(h.contains("<ul><li>one</li><li>two</li></ul>"), "{h}");
        assert!(!h.contains("bad()"), "{h}");
    }

    #[test]
    fn cf_html_offsets() {
        let c = cf_html("<b>é</b>");
        let num = |k: &str| -> usize {
            let p = c.find(k).unwrap_or(0) + k.len();
            c[p..p + 10].parse().unwrap_or(0)
        };
        assert_eq!(&c[num("StartFragment:")..num("EndFragment:")], "<b>é</b>");
        assert!(c[num("StartHTML:")..].starts_with("<html>"));
        assert_eq!(num("EndHTML:"), c.len());
    }

    #[test]
    fn copy_as_markdown_expands_constructs() {
        let d = doc("Say **bold words** now.\n");
        let s = d.source.find("old").unwrap_or(0);
        let e = d.source.find(" words").unwrap_or(0);
        let sel = u32::try_from(s).unwrap_or(0)..u32::try_from(e).unwrap_or(0);
        assert_eq!(markdown_slice(&d, sel), "**bold");
        let s2 = u32::try_from(d.source.find("bold").unwrap_or(0)).unwrap_or(0);
        let e2 = u32::try_from(d.source.find(" now").unwrap_or(0)).unwrap_or(0);
        assert_eq!(markdown_slice(&d, s2..e2), "**bold words**");
    }
}
