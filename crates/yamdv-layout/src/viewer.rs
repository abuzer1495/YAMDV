//! Virtualized document view (R-REN-21, D.4 rules 1–2): estimates, Fenwick tree, lazy layout of
//! visible blocks ± one screen, layout LRU (≤ 3 screens), scroll anchoring, hit testing.

// Indices come from the layout's own arenas (in bounds by construction); f32/f64/u32 casts are
// pixel and UTF-16 quantities far below the type limits (documents ≤ 256 MiB).
#![allow(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::explicit_counter_loop
)]

use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;
use yamdv_core::doc::Document;
use yamdv_core::doc::index::Block;
use yamdv_core::doc::text::ParaText;
use yamdv_core::theme::Palette;

use crate::block::{self, BlockLayout, Ctx, Item, LaidPara, Metrics, Strings};
use crate::code::CodeOpts;
use crate::fenwick::Fenwick;
use crate::geom::Rect;
use crate::plain;
use crate::shaper::{Family, ParaSpec, Style, TextShaper, is_rtl};

/// Top padding (B.6.1).
pub const TOP_PAD: f64 = 48.0;
/// Bottom padding (B.6.1).
pub const BOTTOM_PAD: f64 = 120.0;

/// Detail trees of the referenced footnote definitions in note order (R-REN-10).
fn notes(d: &Document) -> Vec<block::Note> {
    let ix = &d.index;
    let mut out = Vec::with_capacity(ix.footnote_order.len());
    for (k, u) in ix.footnote_order.iter().enumerate() {
        let Some(r) = ix.footnotes.get(&u.label) else {
            continue;
        };
        // Only top-level definitions are laid out (nested ones stay hidden in their container).
        let Ok(bi) = ix.blocks.binary_search_by(|b| b.range.start.cmp(&r.start)) else {
            continue;
        };
        out.push(block::Note {
            n: u32::try_from(k + 1).unwrap_or(u32::MAX),
            refs: u32::try_from(u.refs.len()).unwrap_or(u32::MAX),
            detail: d.detail_uncached(bi),
        });
    }
    out
}

/// Document y of a footnote navigation target, if its block is laid out (see
/// [`Viewer::footnote_target`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FootTarget {
    /// Note `n` in the notes section.
    Note(u32),
    /// Reference `k` of note `n`.
    Ref(u32, u32),
}

/// What is shown: a Markdown document or plain text.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)] // one per window
pub enum Content {
    /// Markdown.
    Markdown(Document),
    /// Plain text (source + chunk ranges).
    Plain {
        /// Text.
        text: String,
        /// Chunks.
        chunks: Vec<Range<u32>>,
    },
}

impl Content {
    /// Plain-text content.
    pub fn plain(text: String) -> Self {
        let chunks = plain::chunks(&text);
        Content::Plain { text, chunks }
    }

    /// Source text.
    pub fn source(&self) -> &str {
        match self {
            Content::Markdown(d) => &d.source,
            Content::Plain { text, .. } => text,
        }
    }

    /// Number of top-level units.
    pub fn len(&self) -> usize {
        match self {
            Content::Markdown(d) => d.index.blocks.len(),
            Content::Plain { chunks, .. } => chunks.len(),
        }
    }

    /// No units.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Source range of unit `i`.
    pub fn range(&self, i: usize) -> Range<u32> {
        match self {
            Content::Markdown(d) => d.index.blocks.get(i).map_or(0..0, |b| b.range.clone()),
            Content::Plain { chunks, .. } => chunks.get(i).cloned().unwrap_or(0..0),
        }
    }

    fn blocks(&self) -> &[Block] {
        match self {
            Content::Markdown(d) => &d.index.blocks,
            Content::Plain { .. } => &[],
        }
    }

    fn gap(&self, i: usize, m: &Metrics) -> f32 {
        match self {
            Content::Markdown(_) => block::gap_before(self.blocks(), i, m),
            Content::Plain { .. } => 0.0,
        }
    }

    fn estimate(&self, i: usize, w: f32, m: &Metrics) -> f32 {
        match self {
            Content::Markdown(d) => d
                .index
                .blocks
                .get(i)
                .map_or(0.0, |b| block::estimate(b, w, m)),
            Content::Plain { text, chunks } => {
                let r = chunks.get(i).cloned().unwrap_or(0..0);
                let s = text.get(r.start as usize..r.end as usize).unwrap_or("");
                let mut lines = 0.0f32;
                for l in s.split_terminator('\n') {
                    let e = yamdv_core::doc::index::estimate(l);
                    lines += (e.half_ems as f32 * 0.5 * m.font_size / w.max(1.0))
                        .ceil()
                        .max(1.0);
                }
                lines.max(1.0) * m.pitch()
            }
        }
    }

    pub(crate) fn layout(&mut self, i: usize, ctx: &mut Ctx<'_>, w: f32) -> BlockLayout {
        match self {
            Content::Markdown(d) => {
                let det = d.detail(i);
                if i + 1 == d.index.blocks.len() && !d.index.footnote_order.is_empty() {
                    let notes = notes(d);
                    return block::layout_with_notes(ctx, &d.source, &det, &notes, w);
                }
                if d.index.html_containers.is_empty() {
                    return block::layout_block(ctx, &d.source, &det, w);
                }
                // §4.6 item 3: blocks inside a collapsed cross-block `<details>` are hidden;
                // blocks inside an alignment container take its alignment.
                let (align, dets) = d.index.container_ctx(i);
                let hidden = dets.iter().any(|&(start, open)| {
                    open == (ctx.code_opts)(start as usize, block::CROSS_DETAILS).fold_toggled
                });
                if hidden {
                    return BlockLayout {
                        height: 0.0,
                        items: Vec::new(),
                        paras: Vec::new(),
                        regions: Vec::new(),
                        hot: Vec::new(),
                    };
                }
                let opens = d.index.opens_details(i);
                block::layout_block_in(ctx, &d.source, &det, w, align, opens)
            }
            Content::Plain { text, chunks } => {
                let r = chunks.get(i).cloned().unwrap_or(0..0);
                let mut s = text.get(r.start as usize..r.end as usize).unwrap_or("");
                if s.ends_with('\n') {
                    s = &s[..s.len() - 1];
                }
                let pt = verbatim(s, r.start);
                let base = Style {
                    family: Family::Body,
                    size: ctx.m.font_size,
                    weight: 400,
                    italic: false,
                    color: ctx.pal.text,
                    strike: false,
                    underline: false,
                };
                let shaped = ctx.shaper.shape(&ParaSpec {
                    text: &pt.text,
                    base,
                    spans: vec![],
                    width: Some(w),
                    line_pitch: ctx.m.pitch(),
                    rtl: is_rtl(&pt.text),
                    tab_spaces: Some(4),
                });
                let height = shaped.height().max(ctx.m.pitch());
                BlockLayout {
                    height,
                    items: vec![Item::Para(0)],
                    paras: vec![LaidPara {
                        x: 0.0,
                        y: 0.0,
                        shaped,
                        map: Some(pt),
                        hscroll: None,
                    }],
                    regions: Vec::new(),
                    hot: Vec::new(),
                }
            }
        }
    }

    /// Copy text of a source selection (§4.6 rules).
    pub fn copy_text(&self, sel: Range<u32>) -> String {
        match self {
            Content::Markdown(d) => yamdv_core::doc::text::copy_text(d, sel),
            Content::Plain { text, .. } => plain::copy_text(text, sel),
        }
    }
}

fn verbatim(s: &str, start: u32) -> ParaText {
    let len16 = u32::try_from(s.encode_utf16().count()).unwrap_or(0);
    let end = start + u32::try_from(s.len()).unwrap_or(0);
    ParaText {
        text: s.to_string(),
        segs: vec![yamdv_core::doc::text::Seg {
            u16_start: 0,
            u16_len: len16,
            byte_start: 0,
            src: start..end,
            verbatim: true,
            style: 0,
            link: yamdv_core::doc::detail::NO_LINK,
        }],
    }
}

/// Everything layout needs besides the content.
pub struct Env<'a> {
    /// Shaper.
    pub shaper: &'a mut dyn TextShaper,
    /// Metrics.
    pub m: Metrics,
    /// Palette.
    pub pal: &'a Palette,
    /// Strings.
    pub strings: &'a dyn Strings,
    /// `reading.single_newline_break`.
    pub single_newline_break: bool,
    /// Image knowledge of the platform.
    pub images: &'a dyn crate::img::ImageHost,
}

/// Virtualized view state.
#[derive(Debug)]
pub struct Viewer {
    heights: Fenwick,
    measured: Vec<bool>,
    layouts: Vec<(usize, u64, BlockLayout)>,
    tick: u64,
    /// Column width.
    pub width: f32,
    /// Scroll offset (DIPs from the document top).
    pub scroll: f64,
    /// Top visible block and offset into it.
    anchor: (usize, f64),
    m: Metrics,
    /// Horizontal offsets per (block, region) — session state (R-CODE-03).
    hscroll: std::collections::HashMap<(usize, u16), f32>,
    /// Default code options (`reading.code_line_numbers`, `reading.code_wrap`).
    pub code_default: CodeOpts,
    /// Per code block overrides (block, node) — session state (R-CODE-02).
    code_opts: std::collections::HashMap<(usize, usize), CodeOpts>,
}

impl Viewer {
    /// Horizontal offset of region `r` of block `b`.
    pub fn hscroll(&self, b: usize, r: u16) -> f32 {
        self.hscroll.get(&(b, r)).copied().unwrap_or(0.0)
    }

    /// Sets the horizontal offset (clamped to the region); returns true if it changed.
    pub fn set_hscroll(&mut self, b: usize, r: u16, off: f32) -> bool {
        let max = self
            .layout_of(b)
            .and_then(|l| l.regions.get(usize::from(r)))
            .map_or(0.0, |g| g.max_scroll());
        let v = off.clamp(0.0, max).round();
        let old = self.hscroll(b, r);
        if (v - old).abs() < 0.5 {
            return false;
        }
        self.hscroll.insert((b, r), v);
        true
    }

    /// Horizontal region under column x / document y: (block, region, block top).
    pub fn region_at(&self, content: &Content, x: f32, y: f64) -> Option<(usize, u16, f64)> {
        if content.is_empty() {
            return None;
        }
        let bi = self.block_at_y(y);
        let bl = self.layout_of(bi)?;
        let by = self.block_top(content, bi);
        let ly = (y - by) as f32;
        bl.regions.iter().enumerate().find_map(|(i, g)| {
            let hit = Rect::new(
                g.view.x,
                g.view.y,
                g.view.w,
                g.track.bottom() - g.view.y + 4.0,
            );
            (x >= hit.x && x < hit.right() && ly >= hit.y && ly < hit.bottom())
                .then(|| (bi, u16::try_from(i).unwrap_or(0), by))
        })
    }

    /// The code toolbar whose card contains (x, y) (column x, document y): (block, block top, item).
    pub fn toolbar_at(&self, content: &Content, x: f32, y: f64) -> Option<(usize, f64, &Item)> {
        if content.is_empty() {
            return None;
        }
        let bi = self.block_at_y(y);
        let bl = self.layout_of(bi)?;
        let by = self.block_top(content, bi);
        let ly = (y - by) as f32;
        bl.items
            .iter()
            .find(|it| {
                matches!(it, Item::CodeToolbar { card, .. }
                    if x >= card.x && x < card.right() && ly >= card.y && ly < card.bottom())
            })
            .map(|it| (bi, by, it))
    }

    /// The interactive element at (x, y) (column x, document y).
    pub fn hot_at(&self, content: &Content, x: f32, y: f64) -> Option<(usize, block::HotKind)> {
        if content.is_empty() {
            return None;
        }
        let bi = self.block_at_y(y);
        let bl = self.layout_of(bi)?;
        let ly = (y - self.block_top(content, bi)) as f32;
        bl.hot
            .iter()
            .find(|h| x >= h.rect.x && x < h.rect.right() && ly >= h.rect.y && ly < h.rect.bottom())
            .map(|h| (bi, h.kind.clone()))
    }

    /// Block that holds a footnote target (for a first scroll before the exact position is known).
    pub fn footnote_block(content: &Content, t: FootTarget) -> Option<(usize, u32)> {
        let Content::Markdown(d) = content else {
            return None;
        };
        let ix = &d.index;
        match t {
            FootTarget::Note(_) => ix.blocks.len().checked_sub(1).map(|b| (b, 0)),
            FootTarget::Ref(n, k) => {
                let u = ix.footnote_order.get((n as usize).checked_sub(1)?)?;
                let b = *u.refs.get(k as usize)?;
                let ord = u.refs[..k as usize].iter().filter(|&&x| x == b).count();
                Some((b as usize, u32::try_from(ord).unwrap_or(0)))
            }
        }
    }

    /// Exact document y of a footnote target when its block layout is cached.
    pub fn footnote_target(&self, content: &Content, t: FootTarget) -> Option<f64> {
        let (b, ord) = Self::footnote_block(content, t)?;
        let bl = self.layout_of(b)?;
        let top = self.block_top(content, b);
        bl.hot
            .iter()
            .find(|h| match (t, &h.kind) {
                (FootTarget::Note(n), block::HotKind::FootNote { n: m }) => n == *m,
                (FootTarget::Ref(n, _), block::HotKind::FootRef { n: m, ord: o }) => {
                    n == *m && ord == *o
                }
                _ => false,
            })
            .map(|h| top + f64::from(h.rect.y))
    }

    /// Code options of (block, node).
    pub fn code_opts(&self, b: usize, n: usize) -> CodeOpts {
        self.code_opts
            .get(&(b, n))
            .copied()
            .unwrap_or(self.code_default)
    }

    /// Sets code options of (block, node) and drops the block's layout so it is laid out again.
    pub fn set_code_opts(&mut self, b: usize, n: usize, o: CodeOpts) {
        self.code_opts.insert((b, n), o);
        self.hscroll.retain(|k, _| k.0 != b);
        self.invalidate_block(b);
    }

    /// Drops the cached layout of block `b` (re-laid out by the next `prepare`).
    pub fn invalidate_block(&mut self, b: usize) {
        self.layouts.retain(|e| e.0 != b);
    }

    /// Typography metrics in use.
    pub fn metrics(&self) -> Metrics {
        self.m
    }

    /// New viewer with estimated heights.
    pub fn new(content: &Content, width: f32, m: Metrics) -> Self {
        let mut v = Self {
            heights: Fenwick::default(),
            measured: Vec::new(),
            layouts: Vec::new(),
            tick: 0,
            width,
            scroll: 0.0,
            anchor: (0, 0.0),
            m,
            hscroll: std::collections::HashMap::new(),
            code_default: CodeOpts::default(),
            code_opts: std::collections::HashMap::new(),
        };
        v.reestimate(content);
        v
    }

    fn reestimate(&mut self, content: &Content) {
        let n = content.len();
        let h: Vec<f64> = (0..n)
            .map(|i| f64::from(content.gap(i, &self.m) + content.estimate(i, self.width, &self.m)))
            .collect();
        self.heights = Fenwick::from_heights(&h);
        self.measured = vec![false; n];
        self.layouts.clear();
    }

    /// Total document height.
    pub fn total_height(&self) -> f64 {
        TOP_PAD + self.heights.total() + BOTTOM_PAD
    }

    /// Maximum scroll offset for a viewport height.
    pub fn max_scroll(&self, view_h: f64) -> f64 {
        (self.total_height() - view_h).max(0.0)
    }

    /// y of block `i`'s content top (after its gap).
    pub fn block_top(&self, content: &Content, i: usize) -> f64 {
        TOP_PAD + self.heights.prefix(i) + f64::from(content.gap(i, &self.m))
    }

    fn slot_top(&self, i: usize) -> f64 {
        TOP_PAD + self.heights.prefix(i)
    }

    /// Block whose slot contains document y.
    pub fn block_at_y(&self, y: f64) -> usize {
        self.heights.find((y - TOP_PAD).max(0.0))
    }

    fn update_anchor(&mut self) {
        let b = self.block_at_y(self.scroll);
        self.anchor = (b, self.scroll - self.slot_top(b));
    }

    fn restore_anchor(&mut self, view_h: f64) {
        if self.heights.is_empty() {
            self.scroll = 0.0;
            return;
        }
        let (b, off) = self.anchor;
        if self.scroll <= 0.0 && b == 0 && off <= 0.0 {
            return;
        }
        self.scroll = (self.slot_top(b) + off).clamp(0.0, self.max_scroll(view_h).max(0.0));
    }

    /// Scrolls to `y` (clamped).
    pub fn scroll_to(&mut self, y: f64, view_h: f64) {
        self.scroll = y.clamp(0.0, self.max_scroll(view_h));
        self.update_anchor();
    }

    /// Current anchor (top visible block, offset).
    pub fn anchor(&self) -> (usize, f64) {
        self.anchor
    }

    /// Width change: invalidate layouts, re-estimate, keep the anchor (proportionally inside the
    /// anchor block).
    pub fn set_width(&mut self, content: &Content, w: f32, view_h: f64) {
        if (w - self.width).abs() < 0.01 {
            return;
        }
        let (b, off) = self.anchor;
        let old_h = self.heights.get(b).max(1.0);
        self.width = w;
        self.reestimate(content);
        let frac = off / old_h;
        self.anchor = (b, frac * self.heights.get(b));
        self.restore_anchor(view_h);
    }

    /// Metrics change (zoom etc.): same as a width change.
    pub fn set_metrics(&mut self, content: &Content, m: Metrics, view_h: f64) {
        self.m = m;
        let w = self.width;
        self.width = -1.0;
        self.set_width(content, w, view_h);
    }

    /// Lays out blocks visible in `[scroll − view_h, scroll + 2·view_h]`, refining heights and
    /// keeping the anchor fixed; evicts layouts outside that window (≤ 3 screens).
    pub fn prepare(&mut self, content: &mut Content, env: &mut Env<'_>, view_h: f64) {
        if content.is_empty() {
            return;
        }
        self.update_anchor();
        let lo = (self.scroll - view_h).max(0.0);
        // Iterate until the window is covered with measured heights (refinements shift blocks).
        for _ in 0..8 {
            let hi = self.scroll + 2.0 * view_h;
            let first = self.block_at_y(lo);
            let last = self.block_at_y(hi);
            let mut changed = false;
            for i in first..=last.min(content.len() - 1) {
                if self.layouts.iter().any(|e| e.0 == i) {
                    continue;
                }
                let (def, map) = (self.code_default, &self.code_opts);
                let lookup = move |b: usize, n: usize| map.get(&(b, n)).copied().unwrap_or(def);
                let mut ctx = Ctx {
                    shaper: &mut *env.shaper,
                    m: env.m,
                    pal: env.pal,
                    strings: env.strings,
                    single_newline_break: env.single_newline_break,
                    block: i,
                    code_opts: &lookup,
                    images: env.images,
                };
                let bl = content.layout(i, &mut ctx, self.width);
                let h = f64::from(content.gap(i, &self.m) + bl.height);
                if (self.heights.get(i) - h).abs() > 0.01 {
                    self.heights.set(i, h);
                    changed = true;
                }
                if let Some(m) = self.measured.get_mut(i) {
                    *m = true;
                }
                self.tick += 1;
                self.layouts.push((i, self.tick, bl));
                if changed && i < self.anchor.0 {
                    // keep anchor: blocks above changed height
                }
            }
            self.restore_anchor(view_h);
            if !changed {
                break;
            }
        }
        let first = self.block_at_y(lo);
        let last = self.block_at_y(self.scroll + 2.0 * view_h);
        self.layouts.retain(|e| e.0 >= first && e.0 <= last);
    }

    /// Visible blocks: (index, content-top y in view coordinates, layout).
    pub fn visible<'a>(
        &'a self,
        content: &Content,
        view_h: f64,
    ) -> Vec<(usize, f64, &'a BlockLayout)> {
        let mut out = Vec::new();
        if content.is_empty() {
            return out;
        }
        let first = self.block_at_y(self.scroll);
        let last = self.block_at_y(self.scroll + view_h);
        for i in first..=last.min(content.len() - 1) {
            if let Some(e) = self.layouts.iter().find(|e| e.0 == i) {
                out.push((i, self.block_top(content, i) - self.scroll, &e.2));
            }
        }
        out
    }

    /// Drops all layouts (colors or fonts changed); heights are kept, so nothing moves.
    pub fn invalidate_layouts(&mut self) {
        self.layouts.clear();
    }

    /// Cached layouts, paragraphs and display items (`mem-stats`).
    pub fn layout_stats(&self) -> (usize, usize, usize) {
        let paras = self.layouts.iter().map(|e| e.2.paras.len()).sum();
        let items = self.layouts.iter().map(|e| e.2.items.len()).sum();
        (self.layouts.len(), paras, items)
    }

    /// Layout of block `i`, if cached.
    pub fn layout_of(&self, i: usize) -> Option<&BlockLayout> {
        self.layouts.iter().find(|e| e.0 == i).map(|e| &e.2)
    }

    /// Number of cached layouts.
    pub fn cached_layouts(&self) -> usize {
        self.layouts.len()
    }

    /// Whether block `i` has a measured height.
    pub fn is_measured(&self, i: usize) -> bool {
        self.measured.get(i).copied().unwrap_or(false)
    }

    /// Hit test at column x / document y → (block, para, UTF-16 index, source offset).
    pub fn hit(&self, content: &Content, x: f32, y: f64) -> Option<Hit> {
        if content.is_empty() {
            return None;
        }
        let bi = self.block_at_y(y);
        let bl = self.layout_of(bi)?;
        let by = self.block_top(content, bi);
        let ly = (y - by) as f32;
        let mut best: Option<(f32, usize)> = None;
        for (pi, p) in bl.paras.iter().enumerate() {
            if p.map.is_none() {
                continue;
            }
            let top = p.y;
            let bottom = p.y + p.shaped.height();
            let dist = if ly < top {
                top - ly
            } else if ly > bottom {
                ly - bottom
            } else {
                0.0
            };
            if best.is_none_or(|b| dist < b.0) {
                best = Some((dist, pi));
            }
        }
        let (_, pi) = best?;
        let p = &bl.paras[pi];
        let dx = p.hscroll.map_or(0.0, |r| self.hscroll(bi, r));
        let idx = p.shaped.hit(x - p.x + dx, ly - p.y);
        let src = p.map.as_ref().map_or(0, |m| m.to_source(idx));
        Some(Hit {
            block: bi,
            para: pi,
            idx16: idx,
            source: src,
        })
    }

    /// Word range (source) around a hit (double-click; Unicode word bounds).
    pub fn word_at(&self, h: &Hit) -> Option<Range<u32>> {
        let p = self.layout_of(h.block)?.paras.get(h.para)?;
        let map = p.map.as_ref()?;
        let (a, b) = word_bounds(&map.text, h.idx16);
        Some(map.to_source(a)..map.to_source(b))
    }

    /// Paragraph range (source) around a hit (triple-click).
    pub fn para_at(&self, h: &Hit) -> Option<Range<u32>> {
        let p = self.layout_of(h.block)?.paras.get(h.para)?;
        let map = p.map.as_ref()?;
        Some(map.to_source(0)..map.to_source(map.len16()))
    }
}

/// Hit-test result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    /// Block.
    pub block: usize,
    /// Paragraph in the block layout.
    pub para: usize,
    /// UTF-16 index in the paragraph.
    pub idx16: u32,
    /// Source byte offset.
    pub source: u32,
}

/// UTF-16 bounds of the word (Unicode word boundaries) containing `idx16`.
pub fn word_bounds(text: &str, idx16: u32) -> (u32, u32) {
    let mut p16 = 0u32;
    for (_, w) in text.split_word_bound_indices() {
        let l = u32::try_from(w.encode_utf16().count()).unwrap_or(0);
        if idx16 < p16 + l {
            return (p16, p16 + l);
        }
        p16 += l;
    }
    (p16, p16)
}

/// Selection rectangles of a source range in a laid paragraph (paragraph coordinates).
pub fn selection_rects(p: &LaidPara, sel: &Range<u32>) -> Vec<crate::Rect> {
    let Some(map) = &p.map else { return Vec::new() };
    let Some(first) = map.segs.first() else {
        return Vec::new();
    };
    let last_end = map.segs.last().map_or(first.src.end, |s| s.src.end);
    if sel.end <= first.src.start || sel.start >= last_end {
        return Vec::new();
    }
    let a = map.from_source(sel.start);
    let b = map.from_source(sel.end);
    if a >= b {
        return Vec::new();
    }
    p.shaped.range_rects(a..b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::EnStrings;
    use crate::shaper::MockShaper;
    use yamdv_core::doc::ReadOpts;
    use yamdv_core::theme;

    fn doc(n: usize) -> Content {
        let mut s = String::new();
        for i in 0..n {
            match i % 5 {
                0 => s.push_str(&format!("# Heading {i}\n\n")),
                1 => s.push_str(&format!("{}\n\n", "word ".repeat(10 + i % 40))),
                2 => s.push_str("- a\n- b\n  - c\n\n"),
                3 => s.push_str("```\ncode\nmore\n```\n\n"),
                _ => s.push_str(
                    "> quote text that wraps a few times when the column is narrow enough\n\n",
                ),
            }
        }
        Content::Markdown(Document::new(s, ReadOpts::default()))
    }

    #[test]
    fn r_ren_21_virtualized_and_anchored() {
        let mut c = doc(2000);
        let pal = theme::palette("default", false);
        let mut sh = MockShaper;
        let strings = EnStrings;
        let mut env = Env {
            images: &crate::img::NO_IMAGES,
            shaper: &mut sh,
            m: Metrics::default(),
            pal: &pal,
            strings: &strings,
            single_newline_break: false,
        };
        let mut v = Viewer::new(&c, 600.0, Metrics::default());
        let view_h = 700.0;
        v.prepare(&mut c, &mut env, view_h);
        assert!(
            v.cached_layouts() < 60,
            "only visible ± one screen is laid out"
        );
        assert!(!v.is_measured(1999));
        // Scroll top → bottom → top; at each step the top visible block keeps its on-screen
        // position across `prepare` (refinements above/below never move it).
        let mut y = 0.0;
        let mut steps = 0;
        while y < v.max_scroll(view_h) && steps < 5000 {
            y += 350.0;
            v.scroll_to(y, view_h);
            let (b, off) = v.anchor();
            let before = v.slot_top(b) + off - v.scroll;
            v.prepare(&mut c, &mut env, view_h);
            let after = v.slot_top(b) + off - v.scroll;
            assert!((before - after).abs() < 0.5, "anchor moved at step {steps}");
            assert_eq!(v.anchor().0, b);
            y = v.scroll;
            assert!(v.cached_layouts() <= 80);
            steps += 1;
        }
        assert!(v.scroll >= v.max_scroll(view_h) - 1.0);
        while v.scroll > 0.0 {
            let ny = v.scroll - 350.0;
            v.scroll_to(ny, view_h);
            let (b, off) = v.anchor();
            let before = v.slot_top(b) + off - v.scroll;
            v.prepare(&mut c, &mut env, view_h);
            let after = v.slot_top(b) + off - v.scroll;
            assert!((before - after).abs() < 0.5);
        }
        assert_eq!(v.block_at_y(0.0), 0);
    }

    #[test]
    fn resize_keeps_anchor_block() {
        let mut c = doc(500);
        let pal = theme::palette("default", false);
        let mut sh = MockShaper;
        let strings = EnStrings;
        let mut env = Env {
            images: &crate::img::NO_IMAGES,
            shaper: &mut sh,
            m: Metrics::default(),
            pal: &pal,
            strings: &strings,
            single_newline_break: false,
        };
        let mut v = Viewer::new(&c, 600.0, Metrics::default());
        v.scroll_to(20_000.0, 700.0);
        v.prepare(&mut c, &mut env, 700.0);
        let (b, _) = v.anchor();
        v.set_width(&c, 300.0, 700.0);
        v.prepare(&mut c, &mut env, 700.0);
        assert_eq!(v.anchor().0, b);
    }

    #[test]
    fn hit_word_and_selection() {
        let mut c = Content::Markdown(Document::new(
            "Hello wörld again\n".into(),
            ReadOpts::default(),
        ));
        let pal = theme::palette("default", false);
        let mut sh = MockShaper;
        let strings = EnStrings;
        let mut env = Env {
            images: &crate::img::NO_IMAGES,
            shaper: &mut sh,
            m: Metrics::default(),
            pal: &pal,
            strings: &strings,
            single_newline_break: false,
        };
        let mut v = Viewer::new(&c, 600.0, Metrics::default());
        v.prepare(&mut c, &mut env, 700.0);
        let h = v.hit(&c, 8.0 * 8.0, TOP_PAD + 5.0).expect("hit");
        assert_eq!(h.source, 9, "after \"Hello wö\"");
        let w = v.word_at(&h).expect("word");
        assert_eq!(&c.source()[w.start as usize..w.end as usize], "wörld");
        let p = &v.layout_of(0).expect("layout").paras[0];
        assert_eq!(selection_rects(p, &(6..13)).len(), 1);
        assert_eq!(c.copy_text(w), "wörld");
    }

    #[test]
    fn plain_mode_layout() {
        let t: String = (0..450).map(|i| format!("line {i}\n")).collect();
        let mut c = Content::plain(t);
        let pal = theme::palette("default", false);
        let mut sh = MockShaper;
        let strings = EnStrings;
        let mut env = Env {
            images: &crate::img::NO_IMAGES,
            shaper: &mut sh,
            m: Metrics::default(),
            pal: &pal,
            strings: &strings,
            single_newline_break: false,
        };
        let mut v = Viewer::new(&c, 600.0, Metrics::default());
        v.prepare(&mut c, &mut env, 700.0);
        let h = v.layout_of(0).expect("layout").height;
        assert!((h - 200.0 * 25.6).abs() < 1.0, "{h}");
        assert_eq!(c.copy_text(0..12), "line 0\r\nline ");
    }

    #[test]
    fn r_code_03_overflowing_code_scrolls_in_a_clipped_region() {
        let long = "x".repeat(400);
        let mut c = Content::Markdown(Document::new(
            format!("```\n{long}\nshort\n```\n"),
            ReadOpts::default(),
        ));
        let pal = theme::palette("default", false);
        let mut sh = MockShaper;
        let strings = EnStrings;
        let mut env = Env {
            images: &crate::img::NO_IMAGES,
            shaper: &mut sh,
            m: Metrics::default(),
            pal: &pal,
            strings: &strings,
            single_newline_break: false,
        };
        let mut v = Viewer::new(&c, 600.0, Metrics::default());
        v.prepare(&mut c, &mut env, 700.0);
        let l = v.layout_of(0).expect("layout");
        assert_eq!(l.regions.len(), 1);
        let g = l.regions[0];
        assert!(g.max_scroll() > 0.0);
        assert!(l.items.iter().any(|i| matches!(i, Item::PushClip(_))));
        assert!(l.items.iter().any(|i| matches!(i, Item::HScrollBar(0))));
        assert!(l.paras.iter().any(|p| p.hscroll == Some(0)));
        // Offsets clamp to the region and shift hit testing.
        let max = g.max_scroll();
        assert!(v.set_hscroll(0, 0, 1e9));
        assert!((v.hscroll(0, 0) - max.round()).abs() < 0.01);
        assert!(!v.set_hscroll(0, 0, 1e9));
        let y = f64::from(g.view.y + g.view.h - 5.0) + v.block_top(&c, 0);
        assert!(v.region_at(&c, g.view.x + 10.0, y).is_some());
        assert!(g.thumb(max).right() <= g.track.right() + 0.01);
    }
}
