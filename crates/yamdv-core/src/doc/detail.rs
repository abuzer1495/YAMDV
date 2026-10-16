//! Detail trees (P-07 level 2): built lazily by re-parsing one top-level block's source slice with
//! reference definitions supplied from the document maps; kept in an LRU cache with a byte budget.
// Arena/segment indices in this module are produced by the builder itself and are in bounds by
// construction; byte offsets are `u32` by design (documents ≤ 256 MiB, R-OPEN-08). Decision recorded
// in ADR-0007.
#![allow(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::ops::Range;
use std::rc::Rc;

use pulldown_cmark::{
    BlockQuoteKind, BrokenLink, CodeBlockKind, CowStr, Event, LinkType, Parser, Tag, TagEnd,
};

use super::index::{Index, fold_label};
use super::{MAX_DEPTH, ReadOpts, parser_options};

/// GFM alert type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alert {
    /// `[!NOTE]`
    Note,
    /// `[!TIP]`
    Tip,
    /// `[!IMPORTANT]`
    Important,
    /// `[!WARNING]`
    Warning,
    /// `[!CAUTION]`
    Caution,
}

/// Interim placeholder kinds (§4.4.3, rendered as source cards until Phase 003+).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placeholder {
    /// GFM table (PHASE-003).
    Table,
    /// HTML block (PHASE-003).
    Html,
    /// Display math (PHASE-003).
    Math,
    /// Footnote definition (PHASE-003).
    Footnotes,
    /// Front matter (PHASE-003).
    FrontMatter,
}

/// Node kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    /// Paragraph; `implicit` for the inline content of tight list items / definitions.
    Paragraph {
        /// No `<p>` in the source structure (tight).
        implicit: bool,
    },
    /// Heading level 1–6.
    Heading(u8),
    /// Block quote.
    BlockQuote(Option<Alert>),
    /// Code block (runs hold the verbatim code text).
    Code {
        /// Info-string language (empty for indented code).
        lang: String,
    },
    /// List.
    List {
        /// Start number for ordered lists.
        start: Option<u64>,
        /// Tight list (no paragraph tags in its items).
        tight: bool,
    },
    /// List item; `task` = Some(checked) for task items.
    Item {
        /// Task-list state.
        task: Option<bool>,
    },
    /// Thematic break.
    Rule,
    /// Definition list.
    DefList,
    /// Definition term.
    DefTitle,
    /// Definition body.
    DefBody,
    /// GFM table (R-REN-07); `aligns` per column: 0 none, 1 left, 2 center, 3 right.
    Table {
        /// Column alignments.
        aligns: Vec<u8>,
    },
    /// Table row (`head` = the header row).
    TableRow {
        /// Header row.
        head: bool,
    },
    /// Table cell (inline leaf).
    TableCell,
    /// Interim placeholder card showing source text.
    Placeholder(Placeholder),
}

impl NodeKind {
    /// Holds inline runs directly.
    pub fn is_inline_leaf(&self) -> bool {
        matches!(
            self,
            NodeKind::Paragraph { .. }
                | NodeKind::Heading(_)
                | NodeKind::DefTitle
                | NodeKind::TableCell
                | NodeKind::Code { .. }
        )
    }
}

/// Inline style flags.
pub mod style {
    /// Strong.
    pub const STRONG: u16 = 1;
    /// Emphasis.
    pub const EM: u16 = 1 << 1;
    /// Strikethrough.
    pub const STRIKE: u16 = 1 << 2;
    /// Inline code (also inline math, PHASE-003).
    pub const CODE: u16 = 1 << 3;
    /// Link (color only in Phase 002).
    pub const LINK: u16 = 1 << 4;
    /// Superscript (rendered as normal text in 002).
    pub const SUP: u16 = 1 << 5;
    /// Subscript (rendered as normal text in 002).
    pub const SUB: u16 = 1 << 6;
    /// Muted text (inline HTML source, PHASE-003).
    pub const MUTED: u16 = 1 << 7;
    /// Image placeholder `[image: alt]` (muted italic, PHASE-003).
    pub const IMAGE: u16 = 1 << 8;
    /// Footnote reference `[label]` (superscript-styled, link color, PHASE-003).
    pub const FOOTREF: u16 = 1 << 9;
    /// Inline math source (PHASE-003).
    pub const MATH: u16 = 1 << 10;
}

/// Inline run kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunKind {
    /// Text.
    Text,
    /// Soft line break.
    SoftBreak,
    /// Hard line break.
    HardBreak,
}

/// One inline run referencing its source range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// Kind.
    pub kind: RunKind,
    /// Source byte range (absolute).
    pub range: Range<u32>,
    /// Owned text when it differs from the source slice (unescaping, entities, placeholders).
    pub owned: Option<Box<str>>,
    /// Style flags ([`style`]).
    pub style: u16,
}

impl Run {
    /// Display text of the run.
    pub fn text<'a>(&'a self, source: &'a str) -> &'a str {
        match &self.owned {
            Some(s) => s,
            None => source
                .get(self.range.start as usize..self.range.end as usize)
                .unwrap_or(""),
        }
    }
}

/// A node in the flat pre-order arena.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Kind.
    pub kind: NodeKind,
    /// Absolute source range.
    pub range: Range<u32>,
    /// Parent node index (`u32::MAX` for roots).
    pub parent: u32,
    /// One past the last descendant (children are `self+1 .. end`).
    pub end: u32,
    /// Nesting depth (0 = root).
    pub depth: u16,
    /// Inline runs in [`Detail::runs`].
    pub runs: Range<u32>,
}

/// Detail tree of one top-level block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Detail {
    /// Nodes in pre-order.
    pub nodes: Vec<Node>,
    /// Inline runs.
    pub runs: Vec<Run>,
}

const NONE: u32 = u32::MAX;

fn u(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn alert(k: BlockQuoteKind) -> Alert {
    match k {
        BlockQuoteKind::Note => Alert::Note,
        BlockQuoteKind::Tip => Alert::Tip,
        BlockQuoteKind::Important => Alert::Important,
        BlockQuoteKind::Warning => Alert::Warning,
        BlockQuoteKind::Caution => Alert::Caution,
    }
}

/// Footnote labels referenced in `s` that are defined in the document (`[^label]`).
fn footnote_labels(s: &str, index: &Index) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(p) = rest.find("[^") {
        rest = &rest[p + 2..];
        if let Some(e) = rest.find(']') {
            let label = &rest[..e];
            if !label.contains('\n')
                && !label.is_empty()
                && index.footnotes.contains_key(&fold_label(label))
                && !out.iter().any(|l: &String| l == label)
            {
                out.push(label.to_string());
            }
        }
    }
    out
}

struct Builder<'s> {
    source: &'s str,
    base: usize,
    d: Detail,
    stack: Vec<u32>,
    overflow: usize,
    implicit: Option<u32>,
    style: Vec<u16>,
    image: Option<(usize, String)>,
    image_depth: usize,
    skip: usize,
}

impl Builder<'_> {
    fn abs(&self, r: &Range<usize>) -> Range<u32> {
        u(self.base + r.start)..u(self.base + r.end)
    }

    fn push(&mut self, kind: NodeKind, range: Range<u32>) -> Option<u32> {
        if self.stack.len() >= MAX_DEPTH {
            self.overflow += 1;
            return None;
        }
        let idx = u(self.d.nodes.len());
        let parent = self.stack.last().copied().unwrap_or(NONE);
        let runs = u(self.d.runs.len())..u(self.d.runs.len());
        self.d.nodes.push(Node {
            kind,
            range,
            parent,
            end: idx + 1,
            depth: u(self.stack.len()) as u16,
            runs,
        });
        self.stack.push(idx);
        Some(idx)
    }

    fn pop(&mut self) {
        if self.overflow > 0 {
            self.overflow -= 1;
            return;
        }
        if let Some(i) = self.stack.pop() {
            let end = u(self.d.nodes.len());
            let nruns = u(self.d.runs.len());
            let n = &mut self.d.nodes[i as usize];
            n.end = end;
            if n.kind.is_inline_leaf() {
                n.runs.end = nruns;
            }
        }
    }

    fn close_implicit(&mut self) {
        if let Some(i) = self.implicit.take() {
            if self.stack.last() == Some(&i) {
                self.pop();
            }
            let n = &mut self.d.nodes[i as usize];
            let runs = &self.d.runs[n.runs.start as usize..n.runs.end as usize];
            if let (Some(f), Some(l)) = (runs.first(), runs.last()) {
                n.range = f.range.start..l.range.end;
            }
        }
    }

    /// Ensures the current node can take inline runs (opens an implicit paragraph if needed).
    fn inline_target(&mut self, at: u32) {
        let top_is_leaf = self
            .stack
            .last()
            .is_some_and(|&i| self.d.nodes[i as usize].kind.is_inline_leaf());
        if !top_is_leaf && self.overflow == 0 {
            if self.stack.len() >= MAX_DEPTH {
                // At the cap: attach to the deepest node by letting the implicit leaf exceed it.
                let idx = u(self.d.nodes.len());
                let parent = self.stack.last().copied().unwrap_or(NONE);
                let runs = u(self.d.runs.len())..u(self.d.runs.len());
                self.d.nodes.push(Node {
                    kind: NodeKind::Paragraph { implicit: true },
                    range: at..at,
                    parent,
                    end: idx + 1,
                    depth: MAX_DEPTH as u16,
                    runs,
                });
                self.stack.push(idx);
                self.implicit = Some(idx);
            } else if let Some(i) = self.push(NodeKind::Paragraph { implicit: true }, at..at) {
                self.implicit = Some(i);
            }
        }
    }

    fn run(&mut self, kind: RunKind, r: &Range<usize>, text: &str, extra: u16) {
        let range = self.abs(r);
        if let Some((_, alt)) = self.image.as_mut() {
            alt.push_str(text);
            return;
        }
        self.inline_target(range.start);
        let verbatim = self.source.get(range.start as usize..range.end as usize) == Some(text);
        let owned = if verbatim || kind != RunKind::Text {
            None
        } else {
            Some(text.into())
        };
        let style = self.style.last().copied().unwrap_or(0) | extra;
        self.d.runs.push(Run {
            kind,
            range,
            owned,
            style,
        });
        if let Some(&i) = self.stack.last() {
            self.d.nodes[i as usize].runs.end = u(self.d.runs.len());
        }
    }

    fn with_style(&mut self, f: u16) {
        let s = self.style.last().copied().unwrap_or(0) | f;
        self.style.push(s);
    }
}

impl Detail {
    /// Builds the detail tree of top-level block `i`.
    pub fn build(source: &str, index: &Index, i: usize, opts: ReadOpts) -> Self {
        let Some(block) = index.blocks.get(i) else {
            return Self::default();
        };
        let (bs, be) = (block.range.start as usize, block.range.end as usize);
        let ls = source
            .get(..bs)
            .and_then(|s| s.rfind('\n'))
            .map_or(0, |p| p + 1);
        let slice = source.get(ls..be).unwrap_or("");
        let labels = footnote_labels(slice, index);
        let mut text = String::with_capacity(slice.len() + labels.len() * 16);
        text.push_str(slice);
        if !labels.is_empty() {
            // Footnote definitions live elsewhere in the document; supply stubs so references
            // inside the slice are recognized (events past the slice are discarded).
            text.push_str("\n\n");
            for l in &labels {
                text.push_str("[^");
                text.push_str(l);
                text.push_str("]: x\n\n");
            }
        }
        let refdefs = &index.refdefs;
        let mut cb = |link: BrokenLink<'_>| {
            refdefs
                .get(&fold_label(&link.reference))
                .map(|d| (CowStr::from(d.dest.clone()), CowStr::from(d.title.clone())))
        };
        let parser =
            Parser::new_with_broken_link_callback(&text, parser_options(opts), Some(&mut cb));
        let mut b = Builder {
            source,
            base: ls,
            d: Detail::default(),
            stack: Vec::new(),
            overflow: 0,
            implicit: None,
            style: Vec::new(),
            image: None,
            image_depth: 0,
            skip: 0,
        };
        let limit = slice.len();
        for (ev, r) in parser.into_offset_iter() {
            if r.start >= limit && b.stack.is_empty() && b.skip == 0 && b.overflow == 0 {
                break;
            }
            let r = r.start.min(limit)..r.end.min(limit);
            if b.skip > 0 {
                match ev {
                    Event::Start(_) => b.skip += 1,
                    Event::End(_) => {
                        b.skip -= 1;
                        if b.skip == 0 {
                            b.pop();
                        }
                    }
                    _ => {}
                }
                continue;
            }
            match ev {
                Event::Start(tag) => {
                    let ar = b.abs(&r);
                    let block_kind = match &tag {
                        Tag::Paragraph => Some(NodeKind::Paragraph { implicit: false }),
                        Tag::Heading { level, .. } => Some(NodeKind::Heading(*level as u8)),
                        Tag::BlockQuote(k) => Some(NodeKind::BlockQuote(k.map(alert))),
                        Tag::CodeBlock(k) => Some(NodeKind::Code {
                            lang: match k {
                                CodeBlockKind::Fenced(l) => {
                                    l.split_whitespace().next().unwrap_or("").to_string()
                                }
                                CodeBlockKind::Indented => String::new(),
                            },
                        }),
                        Tag::List(start) => Some(NodeKind::List {
                            start: *start,
                            tight: true,
                        }),
                        Tag::Item => Some(NodeKind::Item { task: None }),
                        Tag::DefinitionList => Some(NodeKind::DefList),
                        Tag::DefinitionListTitle => Some(NodeKind::DefTitle),
                        Tag::DefinitionListDefinition => Some(NodeKind::DefBody),
                        Tag::Table(al) => Some(NodeKind::Table {
                            aligns: al
                                .iter()
                                .map(|a| match a {
                                    pulldown_cmark::Alignment::None => 0,
                                    pulldown_cmark::Alignment::Left => 1,
                                    pulldown_cmark::Alignment::Center => 2,
                                    pulldown_cmark::Alignment::Right => 3,
                                })
                                .collect(),
                        }),
                        Tag::TableHead => Some(NodeKind::TableRow { head: true }),
                        Tag::TableRow => Some(NodeKind::TableRow { head: false }),
                        Tag::TableCell => Some(NodeKind::TableCell),
                        Tag::HtmlBlock => Some(NodeKind::Placeholder(Placeholder::Html)),
                        Tag::FootnoteDefinition(_) => {
                            Some(NodeKind::Placeholder(Placeholder::Footnotes))
                        }
                        Tag::MetadataBlock(_) => {
                            Some(NodeKind::Placeholder(Placeholder::FrontMatter))
                        }
                        _ => None,
                    };
                    if let Some(kind) = block_kind {
                        b.close_implicit();
                        if matches!(kind, NodeKind::Paragraph { implicit: false })
                            && let Some(&p) = b.stack.last()
                        {
                            // A real paragraph in an item makes the enclosing list loose.
                            let parent = b.d.nodes[p as usize].parent;
                            if matches!(b.d.nodes[p as usize].kind, NodeKind::Item { .. })
                                && let Some(NodeKind::List { tight, .. }) =
                                    b.d.nodes.get_mut(parent as usize).map(|n| &mut n.kind)
                            {
                                *tight = false;
                            }
                        }
                        let placeholder = matches!(kind, NodeKind::Placeholder(_));
                        let pushed = b.push(kind, ar);
                        if placeholder && pushed.is_some() {
                            b.skip = 1;
                        }
                        continue;
                    }
                    match tag {
                        Tag::Emphasis => b.with_style(style::EM),
                        Tag::Strong => b.with_style(style::STRONG),
                        Tag::Strikethrough => b.with_style(style::STRIKE),
                        Tag::Superscript => b.with_style(style::SUP),
                        Tag::Subscript => b.with_style(style::SUB),
                        Tag::Link { link_type, .. } => {
                            let _ = matches!(link_type, LinkType::WikiLink { .. });
                            b.with_style(style::LINK)
                        }
                        Tag::Image { .. } => {
                            if b.image.is_none() {
                                b.image = Some((r.start, String::new()));
                            } else {
                                b.image_depth += 1;
                                b.style.push(b.style.last().copied().unwrap_or(0));
                            }
                        }
                        _ => b.style.push(b.style.last().copied().unwrap_or(0)),
                    }
                }
                Event::End(end) => match end {
                    TagEnd::Paragraph
                    | TagEnd::Heading(_)
                    | TagEnd::BlockQuote(_)
                    | TagEnd::CodeBlock
                    | TagEnd::List(_)
                    | TagEnd::Item
                    | TagEnd::DefinitionList
                    | TagEnd::DefinitionListTitle
                    | TagEnd::DefinitionListDefinition
                    | TagEnd::Table
                    | TagEnd::TableHead
                    | TagEnd::TableRow
                    | TagEnd::TableCell => {
                        b.close_implicit();
                        b.pop();
                    }
                    TagEnd::Image if b.image.is_some() && b.image_depth == 0 => {
                        if let Some((start, alt)) = b.image.take() {
                            let label = format!("[image: {alt}]");
                            b.run(RunKind::Text, &(start..r.end), &label, style::IMAGE);
                        }
                    }
                    TagEnd::Image if b.image_depth > 0 => {
                        b.image_depth -= 1;
                        b.style.pop();
                    }
                    _ => {
                        b.style.pop();
                    }
                },
                Event::Text(t) => b.run(RunKind::Text, &r, &t, 0),
                Event::Code(t) => b.run(RunKind::Text, &r, &t, style::CODE),
                Event::InlineMath(t) | Event::DisplayMath(t) => {
                    b.run(RunKind::Text, &r, &t, style::CODE | style::MATH)
                }
                Event::InlineHtml(t) | Event::Html(t) => b.run(RunKind::Text, &r, &t, style::MUTED),
                Event::FootnoteReference(l) => {
                    b.run(RunKind::Text, &r, &format!("[{l}]"), style::FOOTREF)
                }
                Event::SoftBreak => b.run(RunKind::SoftBreak, &r, "", 0),
                Event::HardBreak => b.run(RunKind::HardBreak, &r, "", 0),
                Event::Rule => {
                    b.close_implicit();
                    let ar = b.abs(&r);
                    if b.push(NodeKind::Rule, ar).is_some() {
                        b.pop();
                    }
                }
                Event::TaskListMarker(checked) => {
                    // The marker precedes the item's paragraph (loose) or text (tight).
                    let item = b
                        .stack
                        .iter()
                        .rev()
                        .copied()
                        .find(|&i| matches!(b.d.nodes[i as usize].kind, NodeKind::Item { .. }));
                    if let Some(i) = item {
                        b.d.nodes[i as usize].kind = NodeKind::Item {
                            task: Some(checked),
                        };
                    }
                }
            }
        }
        b.close_implicit();
        while !b.stack.is_empty() {
            b.pop();
        }
        // A paragraph consisting only of display math becomes a math placeholder card.
        for n in 0..b.d.nodes.len() {
            if matches!(b.d.nodes[n].kind, NodeKind::Paragraph { implicit: false }) {
                let rr = b.d.nodes[n].runs.clone();
                let runs = &b.d.runs[rr.start as usize..rr.end as usize];
                let math = runs.iter().any(|r| {
                    r.style & style::MATH != 0
                        && source
                            .get(r.range.start as usize..)
                            .is_some_and(|s| s.starts_with("$$"))
                });
                let only = runs.iter().all(|r| {
                    r.style & style::MATH != 0
                        || r.kind != RunKind::Text
                        || r.text(source).trim().is_empty()
                });
                if math && only {
                    b.d.nodes[n].kind = NodeKind::Placeholder(Placeholder::Math);
                }
            }
        }
        b.d.nodes.shrink_to_fit();
        b.d.runs.shrink_to_fit();
        b.d
    }

    /// Approximate heap bytes.
    pub fn heap_bytes(&self) -> usize {
        self.nodes.capacity() * std::mem::size_of::<Node>()
            + self
                .nodes
                .iter()
                .map(|n| {
                    if let NodeKind::Code { lang } = &n.kind {
                        lang.capacity()
                    } else {
                        0
                    }
                })
                .sum::<usize>()
            + self.runs.capacity() * std::mem::size_of::<Run>()
            + self
                .runs
                .iter()
                .map(|r| r.owned.as_ref().map_or(0, |s| s.len()))
                .sum::<usize>()
    }

    /// Root nodes (depth 0).
    pub fn roots(&self) -> impl Iterator<Item = usize> + '_ {
        let mut i = 0usize;
        std::iter::from_fn(move || {
            let n = self.nodes.get(i)?;
            let cur = i;
            i = n.end as usize;
            Some(cur)
        })
    }

    /// Direct children of node `i`.
    pub fn children(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        let end = self.nodes.get(i).map_or(0, |n| n.end as usize);
        let mut c = i + 1;
        std::iter::from_fn(move || {
            if c >= end {
                return None;
            }
            let cur = c;
            c = self.nodes[c].end as usize;
            Some(cur)
        })
    }

    /// Runs of node `i`.
    pub fn runs_of(&self, i: usize) -> &[Run] {
        self.nodes.get(i).map_or(&[], |n| {
            &self.runs[n.runs.start as usize..n.runs.end as usize]
        })
    }
}

/// Default detail-cache budget (bytes).
pub const DEFAULT_BUDGET: usize = 2 * 1024 * 1024;

/// LRU cache of detail trees with a byte budget.
#[derive(Debug)]
pub struct Cache {
    budget: usize,
    bytes: usize,
    tick: u64,
    entries: Vec<(usize, u64, usize, Rc<Detail>)>,
}

impl Cache {
    /// Empty cache.
    pub fn new(budget: usize) -> Self {
        Self {
            budget,
            bytes: 0,
            tick: 0,
            entries: Vec::new(),
        }
    }

    /// Cached bytes.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// Number of cached trees.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Drops everything (after a splice or option change).
    pub fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }

    /// Returns the tree of block `i`, building it if needed and evicting least-recently-used trees
    /// beyond the budget (the requested tree is always kept).
    pub fn get_or_build(
        &mut self,
        i: usize,
        source: &str,
        index: &Index,
        opts: ReadOpts,
    ) -> Rc<Detail> {
        self.tick += 1;
        if let Some(e) = self.entries.iter_mut().find(|e| e.0 == i) {
            e.1 = self.tick;
            return e.3.clone();
        }
        let d = Rc::new(Detail::build(source, index, i, opts));
        let size = d.heap_bytes() + 64;
        self.entries.push((i, self.tick, size, d.clone()));
        self.bytes += size;
        while self.bytes > self.budget && self.entries.len() > 1 {
            let (pos, _) = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| e.0 != i)
                .min_by_key(|(_, e)| e.1)
                .unwrap_or((0, &self.entries[0]));
            let e = self.entries.swap_remove(pos);
            self.bytes -= e.2;
        }
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(src: &str) -> (Index, Vec<Detail>) {
        let ix = Index::build(src, ReadOpts::default());
        let ds = (0..ix.blocks.len())
            .map(|i| Detail::build(src, &ix, i, ReadOpts::default()))
            .collect();
        (ix, ds)
    }

    #[test]
    fn lists_tight_loose_task() {
        let src = "- [x] done\n- [ ] todo\n  - nested\n\n3. a\n\n   b\n4. c\n";
        let (_, ds) = tree(src);
        let d = &ds[0];
        assert_eq!(
            d.nodes[0].kind,
            NodeKind::List {
                start: None,
                tight: true
            }
        );
        assert_eq!(d.nodes[1].kind, NodeKind::Item { task: Some(true) });
        assert_eq!(d.nodes[2].kind, NodeKind::Paragraph { implicit: true });
        assert_eq!(d.runs_of(2)[0].text(src), "done");
        let d = &ds[1];
        assert_eq!(
            d.nodes[0].kind,
            NodeKind::List {
                start: Some(3),
                tight: false
            }
        );
    }

    #[test]
    fn refs_resolve_across_blocks() {
        let src = "see [x] and [^n]\n\n[x]: /u\n\n[^n]: note\n";
        let (_, ds) = tree(src);
        let runs = ds[0].runs_of(0);
        assert!(
            runs.iter()
                .any(|r| r.style & style::LINK != 0 && r.text(src) == "x"),
            "{runs:?}"
        );
        assert!(
            runs.iter()
                .any(|r| r.style & style::FOOTREF != 0 && r.text(src) == "[n]"),
            "{runs:?}"
        );
    }

    #[test]
    fn placeholders_and_inline_kinds() {
        let src = "|a|\n|-|\n|1|\n\nx ![alt *t*](p) <b>h</b> $m$ ~~s~~ `c`\n\n$$\ne\n$$\n\n> [!WARNING]\n> w\n";
        let (_, ds) = tree(src);
        assert_eq!(ds[0].nodes[0].kind, NodeKind::Table { aligns: vec![0] });
        assert_eq!(ds[0].nodes[1].kind, NodeKind::TableRow { head: true });
        assert_eq!(ds[0].nodes[2].kind, NodeKind::TableCell);
        assert_eq!(ds[0].nodes.len(), 5);
        let runs = ds[1].runs_of(0);
        let find = |f: u16| {
            runs.iter()
                .find(|r| r.style & f != 0)
                .map(|r| r.text(src).to_string())
        };
        assert_eq!(find(style::IMAGE).as_deref(), Some("[image: alt t]"));
        assert_eq!(find(style::MUTED).as_deref(), Some("<b>"));
        assert_eq!(find(style::MATH).as_deref(), Some("m"));
        assert_eq!(find(style::STRIKE).as_deref(), Some("s"));
        assert_eq!(find(style::CODE).as_deref(), Some("m"));
        assert_eq!(
            ds[2].nodes[0].kind,
            NodeKind::Placeholder(Placeholder::Math)
        );
        assert_eq!(
            ds[3].nodes[0].kind,
            NodeKind::BlockQuote(Some(Alert::Warning))
        );
    }

    #[test]
    fn s_parse_01_depth_cap() {
        let src = format!("{}deep\n", "> ".repeat(200));
        let (_, ds) = tree(&src);
        let d = &ds[0];
        assert!(d.nodes.iter().all(|n| (n.depth as usize) <= MAX_DEPTH));
        assert!(d.runs.iter().any(|r| r.text(&src) == "deep"));
        let src = format!("{}x\n", "- ".repeat(120));
        let (_, ds) = tree(&src);
        assert!(ds[0].nodes.iter().all(|n| (n.depth as usize) <= MAX_DEPTH));
    }

    #[test]
    fn cache_budget_lru() {
        let src: String = (0..50)
            .map(|i| format!("paragraph number {i} with some text\n\n"))
            .collect();
        let ix = Index::build(&src, ReadOpts::default());
        let mut c = Cache::new(2000);
        for i in 0..50 {
            c.get_or_build(i, &src, &ix, ReadOpts::default());
            assert!(c.bytes() <= 2000 || c.len() == 1);
        }
        assert!(c.len() < 50);
        let a = c.get_or_build(49, &src, &ix, ReadOpts::default());
        let b = c.get_or_build(49, &src, &ix, ReadOpts::default());
        assert!(Rc::ptr_eq(&a, &b));
    }
}
