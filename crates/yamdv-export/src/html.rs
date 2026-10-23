//! HTML writer (§4.9, R-REN-16, R-EXP-02 per FUT-21). One writer, two modes: the clipboard
//! fragment (images only as alt text) and the standalone single-file document ([`document`]:
//! inline CSS, TOC, highlighted code, callouts, footnotes, task boxes, `<details>`, images and
//! rasterized math/diagrams as `data:` URIs, no script). Everything is escaped; links only for
//! validated http/https/mailto URLs (plus in-document anchors in the document mode).

// Node and block indices come from the trees' own arenas (in bounds by construction).
#![allow(clippy::indexing_slicing)]

use std::collections::HashMap;
use std::fmt::Write as _;
use std::ops::Range;

use yamdv_core::doc::Document;
use yamdv_core::doc::detail::{Detail, ImageRef, NodeKind, Placeholder, RunKind, style};
use yamdv_core::link::{Target, classify};
use yamdv_core::theme::{CALLOUTS, Palette};
use yamdv_highlight::{Tok, highlight_block, lang_for};

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

/// A rendered raster for the static HTML export (FUT-21): PNG bytes at 2× device pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Png {
    /// Width in device pixels (CSS width = `w / 2`).
    pub w: u32,
    /// Height in device pixels (CSS height = `h / 2`).
    pub h: u32,
    /// Depth below the baseline in CSS px (inline formulas: `vertical-align`).
    pub descent: u32,
    /// PNG file bytes.
    pub bytes: Vec<u8>,
}

/// What the platform provides for the static HTML export (R-EXP-02 per FUT-21).
pub trait ExportHost {
    /// A local image that decoded successfully (S-RES-05): `(mime, file bytes, css w, css h)`.
    /// `None` = not embedded (shown as its alt text).
    fn image(&self, img: &ImageRef) -> Option<(&'static str, Vec<u8>, u32, u32)>;
    /// A formula (`tex` without delimiters) as a 2× PNG.
    fn math(&self, tex: &str, display: bool) -> Option<Png>;
    /// A Mermaid diagram (the fenced source) as a 2× PNG.
    fn diagram(&self, source: &str) -> Option<Png>;
}

/// Options of the standalone document export.
#[derive(Debug, Clone)]
pub struct DocOpts {
    /// Document title (`<title>`).
    pub title: String,
    /// Palette of the current scheme and mode.
    pub pal: Palette,
    /// Dark mode (syntax and callout colors).
    pub dark: bool,
    /// `export.html_include_toc`.
    pub toc: bool,
}

/// Writer state: the clipboard fragment (`host == None`) or the standalone document.
struct Wr<'a> {
    host: Option<&'a dyn ExportHost>,
    /// Image runs seen in the current detail tree (index into `Detail::images`).
    img: usize,
    /// Heading anchors by source start.
    slugs: HashMap<u32, String>,
    /// Footnote bodies `(number, html)` collected for the notes section.
    notes: Vec<(u32, String)>,
    /// Footnote reference ordinal per number (back-link ids).
    refs: HashMap<u32, u32>,
}

impl Wr<'_> {
    fn full(&self) -> bool {
        self.host.is_some()
    }
}

/// `data:` URI of `bytes` (RFC 2397, base64).
pub fn data_uri(mime: &str, bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(bytes.len() * 4 / 3 + 32);
    s.push_str("data:");
    s.push_str(mime);
    s.push_str(";base64,");
    for c in bytes.chunks(3) {
        let b = [
            c.first().copied().unwrap_or(0),
            c.get(1).copied().unwrap_or(0),
            c.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        s.push(char::from(T[(n >> 18) as usize & 63]));
        s.push(char::from(T[(n >> 12) as usize & 63]));
        s.push(if c.len() > 1 {
            char::from(T[(n >> 6) as usize & 63])
        } else {
            '='
        });
        s.push(if c.len() > 2 {
            char::from(T[n as usize & 63])
        } else {
            '='
        });
    }
    s
}

fn png_img(p: &Png, alt: &str, class: &str, out: &mut String) {
    let _ = write!(
        out,
        "<img class=\"{class}\" src=\"{}\" alt=\"",
        data_uri("image/png", &p.bytes)
    );
    escape(alt, out);
    let _ = write!(
        out,
        "\" style=\"width:{}px;height:{}px",
        p.w.div_ceil(2),
        p.h.div_ceil(2)
    );
    if class == "math" {
        let _ = write!(out, ";vertical-align:-{}px", p.descent);
    }
    out.push_str("\">");
}

fn inline(
    doc: &Document,
    d: &Detail,
    i: usize,
    sel: &Range<u32>,
    x: &mut Wr<'_>,
    out: &mut String,
) -> bool {
    let mut any = false;
    for r in d.runs_of(i) {
        let text = match r.kind {
            RunKind::Text => r.text(&doc.source),
            RunKind::SoftBreak => " ",
            RunKind::HardBreak => "\n",
        };
        let is_img = r.style & style::IMAGE != 0;
        let img_ix = x.img;
        if is_img {
            x.img += 1;
        }
        let Some(t) = clip(&doc.source, &r.range, sel, text, r.owned.is_none()) else {
            continue;
        };
        any = true;
        if r.kind == RunKind::HardBreak {
            out.push_str("<br>");
            continue;
        }
        let f = r.style;
        if let Some(h) = x.host {
            if f & style::MATH != 0 {
                let (tex, display) = yamdv_layout::rich::strip_delims(&t);
                if let Some(p) = h.math(tex, display) {
                    png_img(&p, tex, "math", out);
                    continue;
                }
            }
            if f & style::FOOTREF != 0
                && let Ok(n) = t.trim_matches(['[', ']']).parse::<u32>()
            {
                let k = x.refs.entry(n).or_insert(0);
                *k += 1;
                let id = if *k == 1 {
                    format!("fnref-{n}")
                } else {
                    format!("fnref-{n}-{k}")
                };
                let _ = write!(
                    out,
                    "<sup class=\"fnref\" id=\"{id}\"><a href=\"#fn-{n}\">{n}</a></sup>"
                );
                continue;
            }
            if is_img
                && let Some(im) = d.images.get(img_ix)
                && let Some((mime, bytes, w, hh)) = h.image(im)
            {
                let _ = write!(out, "<img src=\"{}\" alt=\"", data_uri(mime, &bytes));
                escape(&im.alt, out);
                let _ = write!(
                    out,
                    "\" style=\"max-width:100%;width:{w}px;height:auto\" data-h=\"{hh}\">"
                );
                continue;
            }
        }
        let mut close: Vec<&str> = Vec::new();
        if f & style::LINK != 0
            && let Some(l) = d.links.get(r.link as usize)
        {
            match classify(&l.dest, l.wiki) {
                Target::External(u) => {
                    out.push_str("<a href=\"");
                    escape(&u.serialize(), out);
                    if x.full() {
                        out.push_str("\" rel=\"noopener noreferrer\">");
                    } else {
                        out.push_str("\">");
                    }
                    close.push("</a>");
                }
                Target::Anchor(a) if x.full() => {
                    out.push_str("<a href=\"#");
                    escape(&a, out);
                    out.push_str("\">");
                    close.push("</a>");
                }
                _ => {}
            }
        }
        for (flag, open, end) in [
            (style::STRONG, "<strong>", "</strong>"),
            (style::EM, "<em>", "</em>"),
            (style::STRIKE, "<del>", "</del>"),
            (style::CODE, "<code>", "</code>"),
            (style::KBD, "<kbd>", "</kbd>"),
            (style::HIGHLIGHT, "<mark>", "</mark>"),
            (style::SUP, "<sup>", "</sup>"),
            (style::SUB, "<sub>", "</sub>"),
            (style::UNDERLINE, "<u>", "</u>"),
        ] {
            if f & flag != 0 {
                let open = if !x.full() && flag == style::KBD {
                    "<code>"
                } else {
                    open
                };
                let end = if !x.full() && flag == style::KBD {
                    "</code>"
                } else {
                    end
                };
                out.push_str(open);
                close.push(end);
            }
        }
        if is_img {
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

/// Plain text of a node's runs (code blocks, math sources).
fn run_text(doc: &Document, d: &Detail, i: usize) -> String {
    d.runs_of(i)
        .iter()
        .map(|r| match r.kind {
            RunKind::Text => r.text(&doc.source),
            RunKind::SoftBreak | RunKind::HardBreak => "\n",
        })
        .collect()
}

/// CSS class of a syntax token (`t0` … in [`TOKS`] order).
const TOKS: [Tok; 19] = [
    Tok::Text,
    Tok::Keyword,
    Tok::Str,
    Tok::Regex,
    Tok::Number,
    Tok::Comment,
    Tok::Function,
    Tok::Type,
    Tok::Constant,
    Tok::Operator,
    Tok::Punct,
    Tok::Tag,
    Tok::Attribute,
    Tok::Variable,
    Tok::Meta,
    Tok::DiffAdd,
    Tok::DiffDel,
    Tok::DiffMeta,
    Tok::Heading,
];

fn code_block(doc: &Document, d: &Detail, i: usize, lang: &str, x: &mut Wr<'_>, out: &mut String) {
    let text = run_text(doc, d, i);
    let text = text.strip_suffix('\n').unwrap_or(&text);
    let l = lang
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if let Some(h) = x.host {
        if l == "mermaid"
            && let Some(p) = h.diagram(text)
        {
            out.push_str("<figure class=\"diagram\">");
            png_img(&p, "Mermaid diagram", "diagram", out);
            out.push_str("</figure>");
            return;
        }
        if l == "math"
            && let Some(p) = h.math(text, true)
        {
            out.push_str("<div class=\"display-math\">");
            png_img(&p, text, "display", out);
            out.push_str("</div>");
            return;
        }
    }
    let Some(lg) = lang_for(lang).filter(|_| x.full()) else {
        out.push_str("<pre><code>");
        escape(text, out);
        out.push_str("</code></pre>");
        return;
    };
    out.push_str("<pre><code>");
    for (k, (line, toks)) in text.split('\n').zip(highlight_block(lg, text)).enumerate() {
        if k > 0 {
            out.push('\n');
        }
        let line = line.strip_suffix('\r').unwrap_or(line);
        for (r, t) in toks {
            let Some(s) = line.get(r) else { continue };
            let c = TOKS.iter().position(|x| *x == t).unwrap_or(0);
            if c == 0 {
                escape(s, out);
            } else {
                let _ = write!(out, "<span class=\"t{c}\">");
                escape(s, out);
                out.push_str("</span>");
            }
        }
    }
    out.push_str("</code></pre>");
}

fn node(doc: &Document, d: &Detail, i: usize, sel: &Range<u32>, x: &mut Wr<'_>, out: &mut String) {
    let n = &d.nodes[i];
    if (n.range.end <= sel.start || n.range.start >= sel.end)
        && !matches!(n.kind, NodeKind::HtmlBlock | NodeKind::Align(_))
    {
        return;
    }
    let kids = |x: &mut Wr<'_>, out: &mut String| {
        for c in d.children(i) {
            node(doc, d, c, sel, x, out);
        }
    };
    let wrap_inline = |tag: &str, attrs: &str, x: &mut Wr<'_>, out: &mut String| {
        let mut buf = String::new();
        if inline(doc, d, i, sel, x, &mut buf) && !buf.is_empty() {
            let _ = write!(out, "<{tag}{attrs}>{buf}</{tag}>");
        }
    };
    let full = x.full();
    match &n.kind {
        NodeKind::Paragraph { implicit: true } => {
            let _ = inline(doc, d, i, sel, x, out);
        }
        NodeKind::Paragraph { .. } => wrap_inline("p", "", x, out),
        NodeKind::Heading(l) => {
            let id = x
                .slugs
                .get(&n.range.start)
                .filter(|_| full)
                .map(|s| {
                    let mut a = String::from(" id=\"");
                    escape(s, &mut a);
                    a.push('"');
                    a
                })
                .unwrap_or_default();
            wrap_inline(&format!("h{}", (*l).clamp(1, 6)), &id, x, out);
        }
        NodeKind::DefTitle => wrap_inline("dt", "", x, out),
        NodeKind::DetailsSummary if full => wrap_inline("summary", "", x, out),
        NodeKind::DetailsSummary => wrap_inline("p", "", x, out),
        NodeKind::TableCell => wrap_inline("td", "", x, out),
        NodeKind::Code { lang } if full => code_block(doc, d, i, lang, x, out),
        NodeKind::Code { .. } => {
            let mut buf = String::new();
            if inline(doc, d, i, sel, x, &mut buf) {
                let _ = write!(out, "<pre><code>{buf}</code></pre>");
            }
        }
        NodeKind::BlockQuote(alert) if full => {
            if let Some(c) = yamdv_layout::callout::detect(&doc.source, d, i, *alert, false) {
                let name = CALLOUTS.get(c.style).map_or("note", |k| k.name);
                let _ = write!(
                    out,
                    "<div class=\"callout {name}\"><p class=\"callout-title\">"
                );
                let title = c.title.clone().unwrap_or_else(|| {
                    let mut t = name.to_string();
                    if let Some(f) = t.get_mut(..1) {
                        f.make_ascii_uppercase();
                    }
                    t
                });
                escape(&title, out);
                out.push_str("</p>");
                if let Some(rest) = &c.rest {
                    out.push_str("<p>");
                    escape(&rest.text, out);
                    out.push_str("</p>");
                }
                for (k, ch) in d.children(i).enumerate() {
                    if k == 0 && c.skip_first {
                        continue;
                    }
                    node(doc, d, ch, sel, x, out);
                }
                out.push_str("</div>");
            } else {
                out.push_str("<blockquote>");
                kids(x, out);
                out.push_str("</blockquote>");
            }
        }
        NodeKind::BlockQuote(_) => {
            out.push_str("<blockquote>");
            kids(x, out);
            out.push_str("</blockquote>");
        }
        NodeKind::List { start, .. } => {
            match start {
                Some(s) if full && *s != 1 => {
                    let _ = write!(out, "<ol start=\"{s}\">");
                }
                Some(_) => out.push_str("<ol>"),
                None => out.push_str("<ul>"),
            }
            kids(x, out);
            out.push_str(if start.is_some() { "</ol>" } else { "</ul>" });
        }
        NodeKind::Item { task } => {
            match task {
                Some(c) if full => {
                    let _ = write!(
                        out,
                        "<li class=\"task\"><input type=\"checkbox\" disabled{}> ",
                        if *c { " checked" } else { "" }
                    );
                }
                _ => out.push_str("<li>"),
            }
            kids(x, out);
            out.push_str("</li>");
        }
        NodeKind::DefList => {
            out.push_str("<dl>");
            kids(x, out);
            out.push_str("</dl>");
        }
        NodeKind::DefBody => {
            out.push_str("<dd>");
            kids(x, out);
            out.push_str("</dd>");
        }
        NodeKind::Table { aligns } if full => {
            out.push_str("<table>");
            for row in d.children(i) {
                let head = matches!(d.nodes[row].kind, NodeKind::TableRow { head: true });
                out.push_str("<tr>");
                for (col, cell) in d.children(row).enumerate() {
                    let tag = if head { "th" } else { "td" };
                    let al = match aligns.get(col) {
                        Some(1) => " style=\"text-align:left\"",
                        Some(2) => " style=\"text-align:center\"",
                        Some(3) => " style=\"text-align:right\"",
                        _ => "",
                    };
                    let mut buf = String::new();
                    let _ = inline(doc, d, cell, sel, x, &mut buf);
                    let _ = write!(out, "<{tag}{al}>{buf}</{tag}>");
                }
                out.push_str("</tr>");
            }
            out.push_str("</table>");
        }
        NodeKind::Table { .. } => {
            out.push_str("<table>");
            kids(x, out);
            out.push_str("</table>");
        }
        NodeKind::TableRow { .. } => {
            out.push_str("<tr>");
            kids(x, out);
            out.push_str("</tr>");
        }
        NodeKind::Rule => out.push_str("<hr>"),
        NodeKind::Placeholder(Placeholder::Math) if full => {
            let mut done = false;
            for r in d.runs_of(i) {
                if r.style & style::MATH == 0 {
                    continue;
                }
                let (tex, _) = yamdv_layout::rich::strip_delims(r.text(&doc.source));
                let tex = tex.trim();
                if let Some(p) = x.host.and_then(|h| h.math(tex, true)) {
                    out.push_str("<div class=\"display-math\">");
                    png_img(&p, tex, "display", out);
                    out.push_str("</div>");
                    done = true;
                }
            }
            if !done {
                out.push_str("<pre>");
                escape(
                    doc.source
                        .get(n.range.start as usize..n.range.end as usize)
                        .unwrap_or(""),
                    out,
                );
                out.push_str("</pre>");
            }
        }
        NodeKind::Placeholder(_) => {
            let a = n.range.start.max(sel.start) as usize;
            let b = n.range.end.min(sel.end) as usize;
            out.push_str("<pre>");
            escape(doc.source.get(a..b).unwrap_or(""), out);
            out.push_str("</pre>");
        }
        NodeKind::FootnoteDef { number } if full && *number > 0 => {
            let mut buf = String::new();
            for c in d.children(i) {
                node(doc, d, c, sel, x, &mut buf);
            }
            x.notes.push((*number, buf));
        }
        NodeKind::FootnoteDef { .. } | NodeKind::Unsupported => {}
        NodeKind::Details { open } if full => {
            out.push_str(if *open { "<details open>" } else { "<details>" });
            kids(x, out);
            out.push_str("</details>");
        }
        NodeKind::HtmlBlock | NodeKind::Align(_) | NodeKind::Details { .. } => kids(x, out),
    }
}

fn blocks(doc: &Document, sel: &Range<u32>, x: &mut Wr<'_>, out: &mut String) {
    let _ = blocks_with(doc, sel, x, out, &mut |_, _| true);
}

/// [`blocks`] with a progress callback `(done, total)` before each block; `false` from the
/// callback stops (cancel, R-EXP-07) and returns `false`.
fn blocks_with(
    doc: &Document,
    sel: &Range<u32>,
    x: &mut Wr<'_>,
    out: &mut String,
    progress: &mut dyn FnMut(usize, usize) -> bool,
) -> bool {
    let first = doc.index.block_at(sel.start);
    let total = doc.index.blocks.len();
    for bi in first..total {
        if doc.index.blocks[bi].range.start >= sel.end {
            break;
        }
        if !progress(bi - first, total - first) {
            return false;
        }
        let d = doc.detail_uncached(bi);
        x.img = 0;
        for r in d.roots() {
            node(doc, &d, r, sel, x, out);
        }
    }
    true
}

/// HTML fragment for the selection `sel` (source byte range).
pub fn fragment(doc: &Document, sel: Range<u32>) -> String {
    let mut out = String::new();
    let mut x = Wr {
        host: None,
        img: 0,
        slugs: HashMap::new(),
        notes: Vec::new(),
        refs: HashMap::new(),
    };
    blocks(doc, &sel, &mut x, &mut out);
    out
}

/// The Content-Security-Policy of exported HTML (S-EXP-02 without scripts, FUT-21).
pub const EXPORT_CSP: &str = "default-src 'none'; img-src data:; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'";

fn hex(c: u32) -> String {
    format!("#{:06x}", c & 0xff_ffff)
}

/// Inline CSS from the scheme tokens (B.3) in the current mode.
pub fn css(p: &Palette, dark: bool) -> String {
    let mut s = String::new();
    let _ = write!(
        s,
        "body{{margin:0;background:{bg};color:{tx};font:16px/1.6 'Segoe UI',system-ui,sans-serif}}\
main{{max-width:860px;margin:0 auto;padding:32px 24px}}\
h1,h2,h3,h4,h5{{color:{hd};line-height:1.25;margin:1.4em 0 .5em}}h6{{color:{mu}}}\
h1{{font-size:2em;border-bottom:1px solid {bs};padding-bottom:.3em}}h2{{font-size:1.5em;border-bottom:1px solid {bs};padding-bottom:.3em}}\
a{{color:{ln}}}blockquote{{margin:0 0 1em;padding:0 1em;border-left:3px solid {qb};color:{qt}}}\
code,kbd{{font-family:'Cascadia Mono',Consolas,monospace;font-size:.9em;background:{ic};border-radius:3px;padding:.1em .3em}}\
kbd{{border:1px solid {cb};border-bottom-width:2px}}\
pre{{background:{cg};border:1px solid {cb};border-radius:6px;padding:12px 16px;overflow:auto}}pre code{{background:none;padding:0}}\
table{{border-collapse:collapse;margin:0 0 1em}}th,td{{border:1px solid {tb};padding:6px 12px}}th{{background:{th}}}tr:nth-child(even) td{{background:{zb}}}\
hr{{border:0;border-top:2px solid {bs}}}mark{{background:{hb};color:{ht}}}\
li.task{{list-style:none}}li.task input{{margin:0 .4em 0 -1.4em}}\
img{{max-width:100%}}.display-math,figure.diagram{{text-align:center;margin:1em 0;overflow-x:auto}}figure.diagram img{{max-width:none}}\
nav.toc{{border:1px solid {bs};border-radius:6px;padding:8px 16px;margin-bottom:24px}}nav.toc ul{{margin:0;padding-left:1.2em}}\
.footnotes{{border-top:1px solid {bs};margin-top:2em;font-size:.9em}}.fnref a{{text-decoration:none}}\
.callout{{border-left:4px solid;border-radius:4px;padding:8px 16px;margin:0 0 1em}}.callout-title{{font-weight:600;margin:.2em 0}}\
details{{margin:0 0 1em}}summary{{cursor:pointer;font-weight:600}}",
        bg = hex(p.bg),
        tx = hex(p.text),
        hd = hex(p.heading),
        mu = hex(p.text_muted),
        bs = hex(p.border_subtle),
        ln = hex(p.link),
        qb = hex(p.quote_bar),
        qt = hex(p.quote_text),
        ic = hex(p.inline_code_bg),
        cg = hex(p.code_bg),
        cb = hex(p.code_border),
        tb = hex(p.table_border),
        th = hex(p.table_header_bg),
        zb = hex(p.zebra()),
        hb = hex(p.highlight_bg),
        ht = hex(if p.highlight_text == 0 {
            p.text
        } else {
            p.highlight_text
        }),
    );
    for c in &CALLOUTS {
        let (fg, bg) = if dark {
            (c.dark, c.dark_bg)
        } else {
            (c.light, c.light_bg)
        };
        let _ = write!(
            s,
            ".callout.{n}{{border-color:{f};background:{b}}}.callout.{n} .callout-title{{color:{f}}}",
            n = c.name,
            f = hex(fg),
            b = hex(bg)
        );
    }
    for (k, t) in TOKS.iter().enumerate().skip(1) {
        let (c, it, bold) = yamdv_layout::code::syntax_style(*t, dark);
        let _ = write!(s, ".t{k}{{color:{}", hex(c));
        if it {
            s.push_str(";font-style:italic");
        }
        if bold {
            s.push_str(";font-weight:600");
        }
        s.push('}');
    }
    s
}

/// Standalone single-file HTML document (R-EXP-02 static per FUT-21): inline CSS, TOC, no script,
/// CSP meta, images/math/diagrams as `data:` URIs from `host`.
pub fn document(doc: &Document, host: &dyn ExportHost, o: &DocOpts) -> String {
    document_with(doc, host, o, &mut |_, _| true).unwrap_or_default()
}

/// [`document`] reporting `(blocks done, total)` to `progress` (BUG-11: the export progress card
/// counts blocks); `None` when `progress` returned `false` (cancelled).
pub fn document_with(
    doc: &Document,
    host: &dyn ExportHost,
    o: &DocOpts,
    progress: &mut dyn FnMut(usize, usize) -> bool,
) -> Option<String> {
    let mut x = Wr {
        host: Some(host),
        img: 0,
        slugs: doc
            .index
            .headings
            .iter()
            .map(|h| (h.range.start, h.slug.clone()))
            .collect(),
        notes: Vec::new(),
        refs: HashMap::new(),
    };
    let mut body = String::new();
    let len = u32::try_from(doc.source.len()).unwrap_or(u32::MAX);
    if !blocks_with(doc, &(0..len), &mut x, &mut body, progress) {
        return None;
    }
    let total = doc.index.blocks.len();
    if !progress(total, total) {
        return None;
    }
    let mut out = String::with_capacity(body.len() + 8192);
    out.push_str("<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\">\n<meta http-equiv=\"Content-Security-Policy\" content=\"");
    out.push_str(EXPORT_CSP);
    out.push_str("\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<meta name=\"generator\" content=\"YAMDV\">\n<title>");
    escape(&o.title, &mut out);
    out.push_str("</title>\n<style>");
    out.push_str(&css(&o.pal, o.dark));
    out.push_str("</style>\n</head><body><main>\n");
    if o.toc && !doc.index.headings.is_empty() {
        out.push_str("<nav class=\"toc\"><ul>");
        let base = doc
            .index
            .headings
            .iter()
            .map(|h| h.level)
            .min()
            .unwrap_or(1);
        for h in &doc.index.headings {
            let _ = write!(
                out,
                "<li style=\"margin-left:{}em\"><a href=\"#",
                u32::from(h.level.saturating_sub(base))
            );
            escape(&h.slug, &mut out);
            out.push_str("\">");
            escape(&h.text, &mut out);
            out.push_str("</a></li>");
        }
        out.push_str("</ul></nav>\n");
    }
    out.push_str(&body);
    if !x.notes.is_empty() {
        x.notes.sort_by_key(|n| n.0);
        out.push_str("\n<section class=\"footnotes\"><ol>");
        for (n, html) in &x.notes {
            let _ = write!(
                out,
                "<li id=\"fn-{n}\">{html} <a href=\"#fnref-{n}\" class=\"back\">\u{21a9}</a></li>"
            );
        }
        out.push_str("</ol></section>");
    }
    out.push_str("\n</main></body></html>\n");
    Some(out)
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

    /// Test host: every image "decodes", formulas and diagrams are 4×2 px PNG stand-ins.
    pub(crate) struct Mock;
    impl ExportHost for Mock {
        fn image(&self, img: &ImageRef) -> Option<(&'static str, Vec<u8>, u32, u32)> {
            (!img.src.contains("missing")).then(|| ("image/png", vec![1, 2, 3], 10, 5))
        }
        fn math(&self, tex: &str, _display: bool) -> Option<Png> {
            (!tex.contains("\\bad")).then(|| Png {
                w: 4,
                h: 2,
                descent: 3,
                bytes: vec![0x89, b'P'],
            })
        }
        fn diagram(&self, _source: &str) -> Option<Png> {
            Some(Png {
                w: 8,
                h: 4,
                descent: 0,
                bytes: vec![9],
            })
        }
    }

    pub(crate) fn opts() -> DocOpts {
        DocOpts {
            title: "T <x>".into(),
            pal: yamdv_core::theme::palette("default", false),
            dark: false,
            toc: true,
        }
    }

    /// BUG-10: `colors.md` exported with a dark UI and `html_colors=light`.
    #[test]
    fn bug_10_colors_md_dark_ui_light_export() {
        use yamdv_core::theme::{self, ExportColors};
        let src = include_str!("../../../tests/fixtures/colors.md");
        let d = doc(src);
        for scheme in theme::SCHEMES {
            let c = ExportColors::parse("light");
            let dark = c.dark(true);
            assert!(!dark);
            let pal = theme::export_palette(theme::palette(scheme, dark), dark);
            let o = DocOpts {
                pal,
                dark,
                ..opts()
            };
            let mut seen = Vec::new();
            let h = document_with(&d, &Mock, &o, &mut |n, t| {
                seen.push((n, t));
                true
            })
            .expect("not cancelled");
            let style = &h[h.find("<style>").unwrap_or(0)..h.find("</style>").unwrap_or(0)];
            assert!(
                style.starts_with("<style>body{margin:0;background:#ffffff;"),
                "{scheme}"
            );
            // No dark-palette token value (that differs from the light palette) is used.
            let l = theme::palette(scheme, false);
            let k = theme::palette(scheme, true);
            let pairs = [
                (k.bg, l.bg),
                (k.text, l.text),
                (k.code_bg, l.code_bg),
                (k.inline_code_bg, l.inline_code_bg),
                (k.table_header_bg, l.table_header_bg),
                (k.quote_text, l.quote_text),
                (k.code_border, l.code_border),
                (k.table_border, l.table_border),
                (k.highlight_bg, l.highlight_bg),
            ];
            let light_css = css(&pal, false);
            for (dv, lv) in pairs {
                let hx = hex(dv);
                if dv != lv && !light_css.contains(&hx) {
                    assert!(
                        !style.contains(&hx),
                        "{scheme}: dark token {hx} in export CSS"
                    );
                }
            }
            for c in &CALLOUTS {
                assert!(!style.contains(&hex(c.dark_bg)) || c.dark_bg == c.light_bg);
            }
            // Progress is monotonic and ends at the total.
            assert!(seen.windows(2).all(|w| w[0].0 <= w[1].0));
            assert_eq!(seen.last().map(|p| p.0 == p.1), Some(true));
        }
        // Cancel stops the export.
        let mut n = 0;
        assert!(
            document_with(&d, &Mock, &opts(), &mut |_, _| {
                n += 1;
                n < 3
            })
            .is_none()
        );
    }

    #[test]
    fn toc_marker_exports_anchored_list() {
        let d = doc("[TOC]\n\n# Intro\n\n## Installation\n\nText.\n");
        let h = document(&d, &Mock, &opts());
        assert!(h.contains("<strong>Contents</strong>"), "{h}");
        assert!(h.contains("href=\"#installation\""), "{h}");
        assert!(h.contains("<h2 id=\"installation\">"), "{h}");
        assert!(!h.contains("[TOC]"), "{h}");
    }

    #[test]
    fn r_exp_02_static_document() {
        let d = doc(concat!(
            "# Title\n\n## Sub & more\n\nText $E=mc^2$ and ![pic](a.png) ![gone](missing.png) [x](https://e.com) [s](#title).[^1]\n\n",
            "```rust\nfn main() { let x = 1; }\n```\n\n",
            "> [!NOTE]\n> Be careful.\n\n",
            "- [x] done\n- [ ] todo\n\n",
            "| L | R |\n|:--|--:|\n| a | b |\n\n",
            "$$\n\\frac{a}{b}\n$$\n\n",
            "```mermaid\ngraph TD\nA-->B\n```\n\n",
            "<details open><summary>More</summary>\n\nHidden\n\n</details>\n\n",
            "[^1]: The note.\n"
        ));
        let h = document(&d, &Mock, &opts());
        assert!(h.starts_with("<!DOCTYPE html>"), "{h}");
        assert!(h.contains(&format!("content=\"{EXPORT_CSP}\"")));
        // BUG-08: wide diagrams scroll inside their figure instead of shrinking.
        assert!(h.contains("figure.diagram img{max-width:none}"));
        assert!(!h.to_ascii_lowercase().contains("<script"));
        assert!(h.contains("<title>T &lt;x&gt;</title>"));
        assert!(
            h.contains("<nav class=\"toc\">") && h.contains("href=\"#sub--more\""),
            "{h}"
        );
        assert!(h.contains("<h2 id=\"sub--more\">"), "{h}");
        assert!(h.contains("<span class=\"t1\">fn</span>"), "{h}");
        assert!(
            h.contains("class=\"callout note\"") && h.contains("Be careful"),
            "{h}"
        );
        assert!(
            h.contains("<input type=\"checkbox\" disabled checked>"),
            "{h}"
        );
        assert!(h.contains("<input type=\"checkbox\" disabled>"), "{h}");
        assert!(h.contains("<th style=\"text-align:left\">L</th>"), "{h}");
        assert!(h.contains("<td style=\"text-align:right\">b</td>"), "{h}");
        assert!(
            h.contains("src=\"data:image/png;base64,AQID\" alt=\"pic\""),
            "{h}"
        );
        assert!(h.contains("gone") && !h.contains("missing.png"), "{h}");
        assert!(
            h.contains("class=\"math\"") && h.contains("alt=\"E=mc^2\""),
            "{h}"
        );
        assert!(h.contains("vertical-align:-3px"), "{h}");
        assert!(h.contains("alt=\"\\frac{a}{b}\""), "{h}");
        assert!(h.contains("alt=\"Mermaid diagram\""), "{h}");
        assert!(h.contains("<details open><summary>More</summary>"), "{h}");
        assert!(
            h.contains("href=\"https://e.com\" rel=\"noopener noreferrer\""),
            "{h}"
        );
        assert!(h.contains("<a href=\"#title\">s</a>"), "{h}");
        assert!(
            h.contains("<sup class=\"fnref\" id=\"fnref-1\"><a href=\"#fn-1\">1</a></sup>"),
            "{h}"
        );
        assert!(
            h.contains("<li id=\"fn-1\">") && h.contains("href=\"#fnref-1\""),
            "{h}"
        );
        for (i, _) in h.match_indices("src=\"") {
            assert!(h[i + 5..].starts_with("data:image/"), "{}", &h[i..i + 40]);
        }
    }

    #[test]
    fn data_uri_base64() {
        assert_eq!(data_uri("image/png", b"Man"), "data:image/png;base64,TWFu");
        assert_eq!(data_uri("image/png", b"Ma"), "data:image/png;base64,TWE=");
        assert_eq!(data_uri("image/png", b"M"), "data:image/png;base64,TQ==");
    }
}
