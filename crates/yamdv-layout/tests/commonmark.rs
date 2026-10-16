//! CommonMark conformance through the full pipeline (§4.8): own vectors (≥ 150) are run through
//! load → index → detail trees → mock layout → display list and checked against pulldown-cmark's
//! own events of a full parse (block kinds, nesting, source ranges, inline text).
//!
//! The official spec examples are **not** committed (H12). Set `YAMDV_COMMONMARK_JSON` to a local
//! copy of `spec.json` to run them through the same pipeline once (`-- --ignored`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::print_stdout,
    missing_docs
)]

use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use yamdv_core::doc::detail::{NodeKind, Placeholder};
use yamdv_core::doc::text::node_text;
use yamdv_core::doc::{Document, MAX_DEPTH, ReadOpts, parser_options};
use yamdv_layout::block::{Ctx, EnStrings, Item, Metrics, layout_block};
use yamdv_layout::shaper::MockShaper;

#[derive(Debug, PartialEq, Eq, Clone)]
struct B {
    depth: usize,
    kind: String,
    start: usize,
}

fn tag_kind(t: &Tag<'_>) -> Option<&'static str> {
    Some(match t {
        Tag::Paragraph => "p",
        Tag::Heading { .. } => "h",
        Tag::BlockQuote(_) => "quote",
        Tag::CodeBlock(_) => "code",
        Tag::HtmlBlock => "html",
        Tag::List(_) => "list",
        Tag::Item => "item",
        Tag::FootnoteDefinition(_) => "footnotes",
        Tag::DefinitionList => "dl",
        Tag::DefinitionListTitle => "dt",
        Tag::DefinitionListDefinition => "dd",
        Tag::Table(_) => "table",
        Tag::MetadataBlock(_) => "front",
        _ => return None,
    })
}

/// Reference block structure + inline text from one full parse.
fn reference(src: &str) -> (Vec<B>, String) {
    let evs: Vec<_> = Parser::new_ext(src, parser_options(ReadOpts::default()))
        .into_offset_iter()
        .collect();
    // GitHub footnote numbering: defined labels in order of first reference.
    let fold = |l: &str| yamdv_core::doc::index::fold_label(l);
    let defined: Vec<String> = evs
        .iter()
        .filter_map(|(e, _)| match e {
            Event::Start(Tag::FootnoteDefinition(l)) => Some(fold(l)),
            _ => None,
        })
        .collect();
    let mut order: Vec<String> = Vec::new();
    for (e, _) in &evs {
        if let Event::FootnoteReference(l) = e {
            let f = fold(l);
            if defined.contains(&f) && !order.contains(&f) {
                order.push(f);
            }
        }
    }
    let mut blocks = Vec::new();
    let mut text = String::new();
    let mut depth = 0usize;
    let mut skip = 0usize;
    let mut image = 0usize;
    let mut img_src = String::new();
    let mut img_at = 0usize;
    let mut i = 0;
    while i < evs.len() {
        let (ev, r) = &evs[i];
        i += 1;
        if skip > 0 {
            match ev {
                Event::Start(_) => skip += 1,
                Event::End(_) => {
                    skip -= 1;
                    if skip == 0 {
                        depth -= 1;
                    }
                }
                _ => {}
            }
            continue;
        }
        match ev {
            Event::Start(t) => {
                if let Some(k) = tag_kind(t) {
                    let mut k = k.to_string();
                    // A paragraph consisting only of display math is a math card.
                    if k == "p" {
                        let mut j = i;
                        let mut only = true;
                        let mut math = false;
                        while !matches!(evs[j].0, Event::End(TagEnd::Paragraph)) {
                            match &evs[j].0 {
                                Event::DisplayMath(_)
                                    if src[evs[j].1.clone()].starts_with("$$") =>
                                {
                                    math = true
                                }
                                Event::Text(t) if t.trim().is_empty() => {}
                                Event::SoftBreak | Event::HardBreak => {}
                                _ => only = false,
                            }
                            j += 1;
                        }
                        if math && only {
                            k = "math".into();
                        }
                    }
                    if depth < MAX_DEPTH {
                        blocks.push(B {
                            depth,
                            kind: k.clone(),
                            start: r.start,
                        });
                    }
                    depth += 1;
                    if ["html", "front", "math"].contains(&k.as_str()) {
                        skip = 1;
                    }
                } else if let Tag::Image { dest_url, .. } = t {
                    if image == 0 {
                        img_src = dest_url.to_string();
                        img_at = text.len();
                    }
                    image += 1;
                }
            }
            Event::End(t) => {
                if matches!(t, TagEnd::Image) {
                    image -= 1;
                    if image == 0 {
                        let alt = text.split_off(img_at);
                        text.push_str(&yamdv_core::doc::detail::image_label(&img_src, &alt));
                    }
                } else if !matches!(
                    t,
                    TagEnd::Emphasis
                        | TagEnd::Strong
                        | TagEnd::Strikethrough
                        | TagEnd::Superscript
                        | TagEnd::Subscript
                        | TagEnd::Link
                        | TagEnd::TableHead
                        | TagEnd::TableRow
                        | TagEnd::TableCell
                ) {
                    depth -= 1;
                }
            }
            Event::Text(t)
            | Event::Code(t)
            | Event::InlineMath(t)
            | Event::DisplayMath(t)
            | Event::Html(t) => text.push_str(t),
            Event::InlineHtml(t) => {
                // Our renderer shows sanitized inline HTML (R-MD-14): text, breaks, images.
                let mut budget = yamdv_core::html::MAX_TAGS;
                for tok in yamdv_core::html::sanitize(t, &mut budget) {
                    match tok {
                        yamdv_core::html::Tok::Text { text: tt, .. } => text.push_str(&tt),
                        yamdv_core::html::Tok::Open { tag, attrs, .. } if tag == "img" => {
                            let alt = yamdv_core::html::attr(&attrs, "alt").unwrap_or("");
                            let src = yamdv_core::html::attr(&attrs, "src").unwrap_or("");
                            text.push_str(&yamdv_core::doc::detail::image_label(src, alt));
                        }
                        yamdv_core::html::Tok::Unsupported(_) => text.push_str("[svg]"),
                        _ => {}
                    }
                }
            }
            Event::FootnoteReference(l) => match order.iter().position(|o| *o == fold(l)) {
                Some(p) => text.push_str(&format!("[{}]", p + 1)),
                None => text.push_str(&format!("[^{l}]")),
            },
            Event::Rule if depth < MAX_DEPTH => {
                blocks.push(B {
                    depth,
                    kind: "hr".into(),
                    start: r.start,
                });
            }
            _ => {}
        }
    }
    (blocks, text)
}

fn node_kind(k: &NodeKind) -> Option<&'static str> {
    Some(match k {
        NodeKind::Paragraph { implicit: true } => return None,
        NodeKind::Paragraph { implicit: false } => "p",
        NodeKind::Heading(_) => "h",
        NodeKind::BlockQuote(_) => "quote",
        NodeKind::Code { .. } => "code",
        NodeKind::List { .. } => "list",
        NodeKind::Item { .. } => "item",
        NodeKind::Rule => "hr",
        NodeKind::DefList => "dl",
        NodeKind::DefTitle => "dt",
        NodeKind::DefBody => "dd",
        NodeKind::Placeholder(Placeholder::Table) | NodeKind::Table { .. } => "table",
        NodeKind::TableRow { .. } | NodeKind::TableCell => return None,
        NodeKind::Placeholder(Placeholder::Html) | NodeKind::HtmlBlock => "html",
        NodeKind::Align(_)
        | NodeKind::Details { .. }
        | NodeKind::DetailsSummary
        | NodeKind::Unsupported
        | NodeKind::Placeholder(Placeholder::Details) => return None,
        NodeKind::Placeholder(Placeholder::Math) => "math",
        NodeKind::Placeholder(Placeholder::Footnotes) | NodeKind::FootnoteDef { .. } => "footnotes",
        NodeKind::Placeholder(Placeholder::FrontMatter) => "front",
    })
}

fn squash(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Runs one input through the pipeline; returns (block count, para items).
fn pipeline(src: &str, check: bool) -> (usize, usize) {
    let mut doc = Document::new(src.to_string(), ReadOpts::default());
    let pal = yamdv_core::theme::palette("default", false);
    let mut shaper = MockShaper;
    let strings = EnStrings;
    let mut ours = Vec::new();
    let mut text = String::new();
    let mut paras = 0;
    let n = doc.index.blocks.len();
    for i in 0..n {
        let d = doc.detail(i);
        for (k, node) in d.nodes.iter().enumerate() {
            // Content of sanitized HTML blocks is ours only (the reference treats them opaquely).
            let mut p = node.parent;
            let mut in_html = false;
            while p != u32::MAX {
                if matches!(d.nodes[p as usize].kind, NodeKind::HtmlBlock) {
                    in_html = true;
                    break;
                }
                p = d.nodes[p as usize].parent;
            }
            if in_html {
                continue;
            }
            if let Some(kind) = node_kind(&node.kind) {
                let depth = (0..)
                    .scan(node.parent, |p, _| {
                        if *p == u32::MAX {
                            None
                        } else {
                            let cur = *p;
                            *p = d.nodes[cur as usize].parent;
                            Some(cur)
                        }
                    })
                    .filter(|&p| node_kind(&d.nodes[p as usize].kind).is_some())
                    .count();
                ours.push(B {
                    depth,
                    kind: kind.into(),
                    start: node.range.start as usize,
                });
            }
            if node.kind.is_inline_leaf() || matches!(node.kind, NodeKind::Paragraph { .. }) {
                text.push_str(&node_text(src, &d, k, false).text);
            }
            for r in d.runs_of(k) {
                assert!(
                    r.range.start <= r.range.end && r.range.end as usize <= src.len(),
                    "run range in bounds: {src:?}"
                );
            }
        }
        let mut ctx = Ctx {
            images: &yamdv_layout::img::NO_IMAGES,
            shaper: &mut shaper,
            m: Metrics::default(),
            pal: &pal,
            strings: &strings,
            single_newline_break: false,
            block: 0,
            code_opts: &|_, _| yamdv_layout::code::CodeOpts::default(),
        };
        let bl = layout_block(&mut ctx, src, &d, 600.0);
        assert!(bl.height.is_finite() && bl.height >= 0.0);
        paras += bl
            .items
            .iter()
            .filter(|i| matches!(i, Item::Para(_)))
            .count();
    }
    if check {
        let (rb, rt) = reference(src);
        assert_eq!(ours, rb, "block structure for {src:?}");
        assert_eq!(squash(&text), squash(&rt), "inline text for {src:?}");
    }
    (n, paras)
}

fn vectors() -> Vec<String> {
    let raw = include_str!("vectors.txt");
    raw.split("\n~~~~~~\n")
        .skip(1)
        .map(|v| v.replace('→', "\t"))
        .collect()
}

#[test]
fn own_vectors_match_pulldown() {
    let v = vectors();
    assert!(v.len() >= 150, "{} vectors", v.len());
    let mut blocks = 0;
    for src in &v {
        blocks += pipeline(src, true).0;
    }
    assert!(blocks >= v.len());
}

#[test]
#[ignore = "needs a local copy of the CommonMark spec.json (never committed)"]
fn official_spec_examples() {
    let Ok(path) = std::env::var("YAMDV_COMMONMARK_JSON") else {
        return;
    };
    let json = std::fs::read_to_string(path).unwrap();
    // Minimal extraction of "markdown": "..." strings (JSON string unescape).
    let mut n = 0;
    let mut mismatches = 0;
    let mut rest = json.as_str();
    while let Some(p) = rest.find("\"markdown\": \"") {
        rest = &rest[p + 13..];
        let mut s = String::new();
        let mut chars = rest.char_indices();
        let mut end = 0;
        while let Some((i, c)) = chars.next() {
            match c {
                '"' => {
                    end = i;
                    break;
                }
                '\\' => match chars.next().map(|x| x.1) {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('u') => {
                        let h: String = (0..4).filter_map(|_| chars.next().map(|x| x.1)).collect();
                        s.push(
                            char::from_u32(u32::from_str_radix(&h, 16).unwrap())
                                .unwrap_or('\u{FFFD}'),
                        );
                    }
                    Some(o) => s.push(o),
                    None => {}
                },
                c => s.push(c),
            }
        }
        rest = &rest[end..];
        pipeline(&s, false);
        if std::panic::catch_unwind(|| pipeline(&s, true)).is_err() {
            mismatches += 1;
        }
        n += 1;
    }
    println!(
        "{n} examples, 0 panics/hangs; structure/text check vs a full pulldown-cmark parse: {mismatches} differ"
    );
    assert!(n > 600);
}
