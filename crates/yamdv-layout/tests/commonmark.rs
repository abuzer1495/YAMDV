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
    let mut blocks = Vec::new();
    let mut text = String::new();
    let mut depth = 0usize;
    let mut skip = 0usize;
    let mut image = 0usize;
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
                    if ["html", "footnotes", "front", "math"].contains(&k.as_str()) {
                        skip = 1;
                    }
                } else if matches!(t, Tag::Image { .. }) {
                    if image == 0 {
                        text.push_str("[image: ");
                    }
                    image += 1;
                }
            }
            Event::End(t) => {
                if matches!(t, TagEnd::Image) {
                    image -= 1;
                    if image == 0 {
                        text.push(']');
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
            | Event::InlineHtml(t)
            | Event::Html(t) => text.push_str(t),
            Event::FootnoteReference(l) => {
                text.push('[');
                text.push_str(l);
                text.push(']');
            }
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
        NodeKind::Placeholder(Placeholder::Html) => "html",
        NodeKind::Placeholder(Placeholder::Math) => "math",
        NodeKind::Placeholder(Placeholder::Footnotes) => "footnotes",
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
