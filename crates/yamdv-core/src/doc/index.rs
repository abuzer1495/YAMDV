//! Top-level block index and document maps (P-07 level 1).
// Arena/segment indices in this module are produced by the builder itself and are in bounds by
// construction; byte offsets are `u32` by design (documents ≤ 256 MiB, R-OPEN-08). Decision recorded
// in ADR-0007.
#![allow(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::collections::HashMap;
use std::ops::Range;

use pulldown_cmark::{Event, MetadataBlockKind, Parser, Tag, TagEnd};

use super::slug::{Slugger, github_slug};
use super::{ReadOpts, parser_options};

/// Kind of a top-level block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// Paragraph.
    Paragraph,
    /// ATX/Setext heading with level 1–6.
    Heading(u8),
    /// Block quote; `alert` is true for GFM alerts (`> [!NOTE]`).
    BlockQuote {
        /// GFM alert.
        alert: bool,
    },
    /// Fenced or indented code.
    Code,
    /// HTML block.
    Html,
    /// Ordered or bullet list.
    List,
    /// Thematic break.
    Rule,
    /// GFM table.
    Table,
    /// Footnote definition.
    FootnoteDef,
    /// Definition list.
    DefList,
    /// Paragraph consisting only of display math.
    Math,
    /// YAML (`---`) or TOML (`+++`) front matter.
    FrontMatter,
}

/// Height-estimate inputs (§4.4.4) gathered from the source slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Estimate {
    /// Width units: 1 per narrow character, 2 per wide (CJK/emoji) character (½ em each).
    pub half_ems: u32,
    /// Source lines.
    pub lines: u32,
}

/// One top-level block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// Kind.
    pub kind: BlockKind,
    /// Source byte range.
    pub range: Range<u32>,
    /// Estimate inputs.
    pub est: Estimate,
}

/// A heading anywhere in the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadingEntry {
    /// Level 1–6.
    pub level: u8,
    /// Source range.
    pub range: Range<u32>,
    /// Plain text.
    pub text: String,
    /// Explicit `{#id}` (R-MD-10), if any.
    pub explicit_id: Option<String>,
    /// Final anchor id (explicit id or unique GitHub slug).
    pub slug: String,
    /// Index of the containing top-level block.
    pub block: u32,
}

/// A link reference definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefDef {
    /// Destination.
    pub dest: String,
    /// Title.
    pub title: String,
    /// Source span.
    pub span: Range<u32>,
}

/// Top-level index + maps.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Index {
    /// Top-level blocks in source order, non-overlapping.
    pub blocks: Vec<Block>,
    /// All headings in source order.
    pub headings: Vec<HeadingEntry>,
    /// Link reference definitions keyed by case-folded label.
    pub refdefs: HashMap<String, RefDef>,
    /// Footnote definitions keyed by case-folded label → source range.
    pub footnotes: HashMap<String, Range<u32>>,
    /// Front matter range.
    pub front_matter: Option<Range<u32>>,
}

/// Case-folds a reference label (whitespace collapsed).
pub fn fold_label(label: &str) -> String {
    label
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Is `c` a wide character for estimates (CJK, Hangul, kana, fullwidth, emoji)?
pub fn is_wide(c: char) -> bool {
    matches!(c as u32,
        0x1100..=0x115F | 0x2E80..=0x303E | 0x3041..=0x33FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF
        | 0xA000..=0xA4CF | 0xAC00..=0xD7A3 | 0xF900..=0xFAFF | 0xFE30..=0xFE4F | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6 | 0x1F300..=0x1F64F | 0x1F900..=0x1F9FF | 0x20000..=0x3FFFD)
}

/// Estimate inputs of a slice.
pub fn estimate(s: &str) -> Estimate {
    let mut half_ems = 0u32;
    let mut lines = 1u32;
    for c in s.chars() {
        if c == '\n' {
            lines += 1;
        } else {
            half_ems = half_ems.saturating_add(if is_wide(c) { 2 } else { 1 });
        }
    }
    if s.ends_with('\n') {
        lines -= 1;
    }
    Estimate {
        half_ems,
        lines: lines.max(1),
    }
}

fn u(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Output of one scan.
#[derive(Debug, Default)]
struct Scan {
    blocks: Vec<Block>,
    headings: Vec<HeadingEntry>,
    footnotes: HashMap<String, Range<u32>>,
    front_matter: Option<Range<u32>>,
}

/// Scans `text[base..]` and collects top-level blocks (absolute ranges) until `stop` returns true
/// for a completed block (that block is not included).
fn scan(text: &str, base: usize, opts: ReadOpts, mut stop: impl FnMut(&Block) -> bool) -> Scan {
    let slice = text.get(base..).unwrap_or("");
    let mut out = Scan::default();
    let mut depth = 0usize;
    let mut kind = BlockKind::Paragraph;
    let mut start = 0usize;
    let mut only_math = false;
    let mut saw_math = false;
    let mut heading: Option<(u8, Range<usize>, String, Option<String>)> = None;
    let finish = |out: &mut Scan,
                  kind: BlockKind,
                  r: Range<usize>,
                  stop: &mut dyn FnMut(&Block) -> bool|
     -> bool {
        let abs = base + r.start..base + r.end;
        let est = estimate(text.get(abs.clone()).unwrap_or(""));
        let b = Block {
            kind,
            range: u(abs.start)..u(abs.end),
            est,
        };
        if stop(&b) {
            return true;
        }
        out.blocks.push(b);
        false
    };
    for (ev, r) in Parser::new_ext(slice, parser_options(opts)).into_offset_iter() {
        match ev {
            Event::Start(tag) => {
                if let Tag::Heading { level, id, .. } = &tag {
                    heading = Some((
                        *level as u8,
                        r.clone(),
                        String::new(),
                        id.as_ref().map(|s| s.to_string()),
                    ));
                }
                if let Tag::FootnoteDefinition(label) = &tag {
                    out.footnotes
                        .entry(fold_label(label))
                        .or_insert(u(base + r.start)..u(base + r.end));
                }
                if depth == 0 {
                    start = r.start;
                    only_math = false;
                    saw_math = false;
                    kind = match &tag {
                        Tag::Paragraph => {
                            only_math = true;
                            BlockKind::Paragraph
                        }
                        Tag::Heading { level, .. } => BlockKind::Heading(*level as u8),
                        Tag::BlockQuote(k) => BlockKind::BlockQuote { alert: k.is_some() },
                        Tag::CodeBlock(_) => BlockKind::Code,
                        Tag::HtmlBlock => BlockKind::Html,
                        Tag::List(_) => BlockKind::List,
                        Tag::Table(_) => BlockKind::Table,
                        Tag::FootnoteDefinition(_) => BlockKind::FootnoteDef,
                        Tag::DefinitionList => BlockKind::DefList,
                        Tag::MetadataBlock(m) => {
                            let _ = matches!(m, MetadataBlockKind::YamlStyle);
                            BlockKind::FrontMatter
                        }
                        _ => BlockKind::Paragraph,
                    };
                }
                depth += 1;
            }
            Event::End(end) => {
                depth = depth.saturating_sub(1);
                if matches!(end, TagEnd::Heading(_))
                    && let Some((level, hr, text_acc, id)) = heading.take()
                {
                    out.headings.push(HeadingEntry {
                        level,
                        range: u(base + hr.start)..u(base + hr.end),
                        text: text_acc,
                        explicit_id: id,
                        slug: String::new(),
                        block: u(out.blocks.len()),
                    });
                }
                if depth == 0 {
                    let mut k = kind;
                    if k == BlockKind::Paragraph && only_math && saw_math {
                        k = BlockKind::Math;
                    }
                    if matches!(end, TagEnd::MetadataBlock(_)) {
                        out.front_matter = Some(u(base + start)..u(base + r.end));
                    }
                    if finish(&mut out, k, start..r.end, &mut stop) {
                        out.headings
                            .retain(|h| (h.block as usize) < out.blocks.len());
                        return out;
                    }
                }
            }
            Event::Rule if depth == 0 => {
                if finish(&mut out, BlockKind::Rule, r, &mut stop) {
                    return out;
                }
            }
            other => {
                if let Some(h) = heading.as_mut() {
                    match &other {
                        Event::Text(t) | Event::Code(t) | Event::InlineMath(t) => h.2.push_str(t),
                        Event::SoftBreak | Event::HardBreak => h.2.push(' '),
                        _ => {}
                    }
                }
                match other {
                    Event::DisplayMath(_) => saw_math = true,
                    Event::Text(t) if t.trim().is_empty() => {}
                    Event::SoftBreak | Event::HardBreak => {}
                    _ => only_math = false,
                }
            }
        }
    }
    out
}

/// Clamps block ends so ranges never overlap (pulldown-cmark reports definition-list ends that
/// swallow the next block).
fn clamp(blocks: &mut [Block]) {
    for i in 1..blocks.len() {
        let next_start = blocks[i].range.start;
        let prev = &mut blocks[i - 1];
        if prev.range.end > next_start {
            prev.range.end = next_start.max(prev.range.start);
        }
    }
}

fn assign_slugs(headings: &mut [HeadingEntry]) {
    let mut s = Slugger::default();
    for h in headings {
        h.slug = match &h.explicit_id {
            Some(id) => {
                s.unique(id);
                id.clone()
            }
            None => s.unique(&github_slug(&h.text)),
        };
    }
}

fn collect_refdefs(text: &str, opts: ReadOpts) -> HashMap<String, RefDef> {
    let parser = Parser::new_ext(text, parser_options(opts));
    let mut map = HashMap::new();
    for (k, d) in parser.reference_definitions().iter() {
        map.insert(
            fold_label(k),
            RefDef {
                dest: d.dest.to_string(),
                title: d.title.as_ref().map(|t| t.to_string()).unwrap_or_default(),
                span: u(d.span.start)..u(d.span.end),
            },
        );
    }
    map
}

/// Result of [`Index::splice`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Splice {
    /// Old top-level block indices that were replaced.
    pub replaced: Range<usize>,
    /// Number of new blocks inserted at `replaced.start`.
    pub inserted: usize,
    /// Reference/footnote definitions changed → inlines must be re-resolved.
    pub refs_changed: bool,
}

impl Index {
    /// Builds the index of `text` with one full parse (+ the refdef map of the same parse).
    pub fn build(text: &str, opts: ReadOpts) -> Self {
        let mut s = scan(text, 0, opts, |_| false);
        clamp(&mut s.blocks);
        assign_slugs(&mut s.headings);
        Self {
            blocks: s.blocks,
            headings: s.headings,
            refdefs: collect_refdefs(text, opts),
            footnotes: s.footnotes,
            front_matter: s.front_matter,
        }
    }

    /// Index of the top-level block containing byte `pos` or the first block after it.
    pub fn block_at(&self, pos: u32) -> usize {
        self.blocks.partition_point(|b| b.range.end <= pos)
    }

    /// Approximate heap use.
    pub fn heap_bytes(&self) -> usize {
        self.blocks.capacity() * std::mem::size_of::<Block>()
            + self
                .headings
                .iter()
                .map(|h| h.text.capacity() + h.slug.capacity() + 96)
                .sum::<usize>()
            + self
                .refdefs
                .iter()
                .map(|(k, v)| k.len() + v.dest.len() + v.title.len() + 80)
                .sum::<usize>()
            + self.footnotes.keys().map(|k| k.len() + 40).sum::<usize>()
    }

    /// Incremental update (designed for Phase 006, ADR-0007). `new_text` is the full text after
    /// replacing the old byte range `edit` by `new_len` bytes. Re-parses from the top-level block
    /// before the edit until block boundaries re-synchronize, then splices and shifts.
    pub fn splice(
        &mut self,
        new_text: &str,
        edit: Range<usize>,
        new_len: usize,
        opts: ReadOpts,
    ) -> Splice {
        let delta = new_len as i64 - (edit.end - edit.start) as i64;
        let first = self.block_at(u(edit.start)).saturating_sub(1);
        let base = self
            .blocks
            .get(first)
            .map_or(edit.start.min(new_text.len()), |b| {
                (b.range.start as usize).min(edit.start)
            });
        // Line start of `base` in the new text (unchanged before the edit).
        let base = new_text
            .get(..base)
            .and_then(|s| s.rfind('\n'))
            .map_or(0, |p| p + 1);
        let first = self.block_at(u(base));
        let old_blocks = &self.blocks;
        let edit_end_new = edit.start as i64 + new_len as i64;
        let mut resync: Option<usize> = None;
        let mut j = first;
        let s = scan(new_text, base, opts, |nb| {
            if (nb.range.start as i64) < edit_end_new {
                return false;
            }
            while j < old_blocks.len()
                && (old_blocks[j].range.start as i64 + delta) < nb.range.start as i64
            {
                j += 1;
            }
            if let Some(ob) = old_blocks.get(j)
                && ob.range.start as usize >= edit.end
                && ob.range.start as i64 + delta == nb.range.start as i64
                && ob.kind == nb.kind
            {
                resync = Some(j);
                return true;
            }
            false
        });
        let end = resync.unwrap_or(self.blocks.len());
        let shift = |r: &Range<u32>| {
            u((r.start as i64 + delta) as usize)..u((r.end as i64 + delta) as usize)
        };
        let mut new_blocks = s.blocks;
        let inserted = new_blocks.len();
        let tail: Vec<Block> = self.blocks[end..]
            .iter()
            .map(|b| Block {
                range: shift(&b.range),
                ..b.clone()
            })
            .collect();
        new_blocks.extend(tail);
        let mut all: Vec<Block> = self.blocks[..first].to_vec();
        all.extend(new_blocks);
        clamp(&mut all);
        // Headings: keep those before `first`, insert new, shift those from `end`.
        let mut headings: Vec<HeadingEntry> = self
            .headings
            .iter()
            .filter(|h| (h.block as usize) < first)
            .cloned()
            .collect();
        headings.extend(s.headings.into_iter().map(|mut h| {
            h.block += u(first);
            h
        }));
        let block_delta = first as i64 + inserted as i64 - end as i64;
        headings.extend(
            self.headings
                .iter()
                .filter(|h| h.block as usize >= end)
                .map(|h| HeadingEntry {
                    range: shift(&h.range),
                    block: u((h.block as i64 + block_delta) as usize),
                    ..h.clone()
                }),
        );
        assign_slugs(&mut headings);
        let old_len = (new_text.len() as i64 - delta) as usize;
        let old_region = base..self
            .blocks
            .get(end)
            .map_or(old_len, |b| b.range.start as usize);
        let new_region_end = all
            .get(first + inserted)
            .map_or(new_text.len(), |b| b.range.start as usize);
        let new_region = new_text.get(base..new_region_end).unwrap_or("");
        let hits = |r: &Range<u32>| {
            (r.start as usize) < old_region.end && r.end as usize > old_region.start
        };
        let refs_changed = new_region.contains("]:")
            || self.refdefs.values().any(|d| hits(&d.span))
            || self.footnotes.values().any(hits);
        self.blocks = all;
        self.headings = headings;
        if refs_changed {
            // Re-resolution pass: rebuild the global maps.
            let full = Index::build(new_text, opts);
            self.refdefs = full.refdefs;
            self.footnotes = full.footnotes;
            self.front_matter = full.front_matter;
            // Heading texts depend on reference resolution.
            self.headings = full.headings;
        } else {
            for d in self.refdefs.values_mut() {
                if d.span.start as usize >= edit.end {
                    d.span = shift(&d.span);
                }
            }
            for r in self.footnotes.values_mut() {
                if r.start as usize >= edit.end {
                    *r = shift(r);
                }
            }
            if let Some(fm) = &self.front_matter
                && fm.start as usize >= edit.end
            {
                self.front_matter = Some(shift(fm));
            }
        }
        Splice {
            replaced: first..end,
            inserted,
            refs_changed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(t: &str) -> Vec<BlockKind> {
        Index::build(t, ReadOpts::default())
            .blocks
            .iter()
            .map(|b| b.kind)
            .collect()
    }

    #[test]
    fn p07_block_kinds() {
        use BlockKind::*;
        let t = "---\na: 1\n---\n# T\n\npara\n\n> q\n\n> [!TIP]\n> x\n\n```rs\nc\n```\n\n<div>\nx\n</div>\n\n- a\n- b\n\n***\n\n|a|b|\n|-|-|\n|1|2|\n\n[^n]: note\n\nterm\n: def\n\n$$\nx\n$$\n";
        assert_eq!(
            kinds(t),
            vec![
                FrontMatter,
                Heading(1),
                Paragraph,
                BlockQuote { alert: false },
                BlockQuote { alert: true },
                Code,
                Html,
                List,
                Rule,
                Table,
                FootnoteDef,
                DefList,
                Math
            ]
        );
        let ix = Index::build(t, ReadOpts::default());
        assert_eq!(ix.front_matter, Some(0..12));
        assert!(ix.footnotes.contains_key("n"));
        for w in ix.blocks.windows(2) {
            assert!(w[0].range.end <= w[1].range.start);
        }
    }

    #[test]
    fn r_md_10_heading_map() {
        let ix = Index::build(
            "# A b\n\n## A b\n\n> # In quote\n\n### X {#custom}\n\nSetext\n===\n",
            ReadOpts::default(),
        );
        let s: Vec<_> = ix
            .headings
            .iter()
            .map(|h| (h.level, h.slug.as_str(), h.block))
            .collect();
        assert_eq!(
            s,
            vec![
                (1, "a-b", 0),
                (2, "a-b-1", 1),
                (1, "in-quote", 2),
                (3, "custom", 3),
                (1, "setext", 4)
            ]
        );
    }

    #[test]
    fn refdefs_map() {
        let ix = Index::build("[x]\n\n[X]: /u \"t\"\n", ReadOpts::default());
        assert_eq!(
            ix.refdefs
                .get("x")
                .map(|d| (d.dest.as_str(), d.title.as_str())),
            Some(("/u", "t"))
        );
        assert_eq!(ix.blocks.len(), 1);
    }

    #[test]
    fn estimates() {
        assert_eq!(
            estimate("ab\ncd\n"),
            Estimate {
                half_ems: 4,
                lines: 2
            }
        );
        assert_eq!(
            estimate("日本"),
            Estimate {
                half_ems: 4,
                lines: 1
            }
        );
    }

    fn check_splice(old: &str, edit: Range<usize>, ins: &str) {
        let mut new = old.to_string();
        new.replace_range(edit.clone(), ins);
        let mut ix = Index::build(old, ReadOpts::default());
        ix.splice(&new, edit.clone(), ins.len(), ReadOpts::default());
        let full = Index::build(&new, ReadOpts::default());
        assert_eq!(
            ix.blocks, full.blocks,
            "blocks after splice {edit:?} {ins:?} in {old:?}"
        );
        assert_eq!(ix.headings, full.headings, "headings");
        assert_eq!(ix.refdefs, full.refdefs, "refdefs");
        assert_eq!(ix.footnotes, full.footnotes, "footnotes");
        assert_eq!(ix.front_matter, full.front_matter, "front matter");
    }

    #[test]
    fn splice_synthetic_edits() {
        let doc = "# One\n\npara one\nline\n\n- a\n- b\n\n```\ncode\n```\n\n## Two\n\n> quote\n\n[r]: /x\n\ntext [r]\n\n# One\n";
        let inserts = [
            "",
            "x",
            "\n",
            "\n\n",
            "# H\n\n",
            "```\n",
            "- c\n",
            "> ",
            "[r]: /y\n\n",
            "|a|\n|-|\n",
            "***\n",
        ];
        let mut edits = 0;
        for start in (0..doc.len()).step_by(3) {
            for len in [0usize, 1, 5] {
                let end = (start + len).min(doc.len());
                if !doc.is_char_boundary(start) || !doc.is_char_boundary(end) {
                    continue;
                }
                for ins in inserts {
                    check_splice(doc, start..end, ins);
                    edits += 1;
                }
            }
        }
        assert!(edits > 500);
    }

    #[test]
    fn splice_resyncs_locally() {
        let doc = "a\n\nb\n\nc\n\nd\n";
        let mut ix = Index::build(doc, ReadOpts::default());
        let new = "a\n\nbX\n\nc\n\nd\n";
        let r = ix.splice(new, 4..4, 1, ReadOpts::default());
        assert!(r.replaced.end - r.replaced.start <= 2, "{r:?}");
        assert!(!r.refs_changed);
    }
}
