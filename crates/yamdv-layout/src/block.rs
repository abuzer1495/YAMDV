//! Block layout (UI_SPEC §B.6) producing a display list per top-level block.
//!
//! Interim renderings of constructs owned by later phases are marked `PHASE-NNN`.

// Indices come from the layout's own arenas (in bounds by construction); f32/f64/u32 casts are
// pixel and UTF-16 quantities far below the type limits (documents ≤ 256 MiB).
#![allow(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::explicit_counter_loop
)]

use yamdv_core::doc::detail::{Alert, Detail, NodeKind, Placeholder, style as st};
use yamdv_core::doc::index::{Block, BlockKind};
use yamdv_core::doc::text::{ParaText, Seg, node_text};
use yamdv_core::theme::Palette;

use crate::geom::Rect;
use crate::shaper::{Family, ParaSpec, Shaped, Span, Style, TextShaper, is_rtl};

/// Typography settings (B.2, B.8.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// `typography.font_size`.
    pub font_size: f32,
    /// `typography.line_height`.
    pub line_height: f32,
    /// `typography.paragraph_spacing`.
    pub para_spacing: f32,
    /// `appearance.table_zebra` (R-THEME-09).
    pub table_zebra: bool,
    /// `appearance.corner_style == rounded`.
    pub rounded: bool,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            line_height: 1.6,
            para_spacing: 16.0,
            table_zebra: false,
            rounded: true,
        }
    }
}

impl Metrics {
    /// Body line pitch.
    pub fn pitch(&self) -> f32 {
        self.font_size * self.line_height
    }
    /// Code font size (0.875 em).
    pub fn code_size(&self) -> f32 {
        self.font_size * 0.875
    }
    /// Code line pitch (1.6).
    pub fn code_pitch(&self) -> f32 {
        self.code_size() * 1.6
    }
    /// Heading size for level 1–6.
    pub fn heading_size(&self, level: u8) -> f32 {
        const EM: [f32; 6] = [2.0, 1.5, 1.25, 1.125, 1.0, 0.875];
        self.font_size
            * EM.get(usize::from(level.clamp(1, 6)) - 1)
                .copied()
                .unwrap_or(1.0)
    }
}

/// Code-block geometry (B.6.7).
pub const CODE_HEADER: f32 = 28.0;
/// Padding before the first code line.
pub const CODE_PAD_TOP: f32 = 16.0;
/// Left/right code padding.
pub const CODE_PAD_X: f32 = 32.0;
/// Bottom code padding.
pub const CODE_PAD_BOTTOM: f32 = 16.0;

/// Bullet shape by depth (B.6.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bullet {
    /// Filled disc (depth 1).
    Disc,
    /// Ring (depth 2).
    Ring,
    /// Square (depth 3+).
    Square,
}

/// Display item (coordinates relative to the block's top-left at the column origin).
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// Filled (rounded) rectangle.
    Fill {
        /// Rectangle.
        rect: Rect,
        /// Color.
        color: u32,
        /// Corner radius.
        radius: f32,
    },
    /// 1-px stroked rounded rectangle.
    Stroke {
        /// Rectangle.
        rect: Rect,
        /// Color.
        color: u32,
        /// Corner radius.
        radius: f32,
    },
    /// Stroked rounded rectangle with a given line width (task boxes).
    StrokeW {
        /// Rectangle.
        rect: Rect,
        /// Color.
        color: u32,
        /// Corner radius.
        radius: f32,
        /// Line width.
        width: f32,
    },
    /// Stroked polyline (vector icons, check marks, chevrons).
    Line {
        /// Points.
        pts: Vec<(f32, f32)>,
        /// Color.
        color: u32,
        /// Line width.
        width: f32,
        /// Close the path.
        closed: bool,
    },
    /// List bullet.
    Bullet {
        /// Center x.
        cx: f32,
        /// Center y.
        cy: f32,
        /// Size (diameter / side).
        size: f32,
        /// Shape.
        shape: Bullet,
        /// Color.
        color: u32,
    },
    /// Horizontal fade from transparent to `color` (overflow hint).
    FadeRight {
        /// Rectangle.
        rect: Rect,
        /// Color.
        color: u32,
    },
    /// Paragraph `paras[i]`.
    Para(usize),
    /// Push a clip rectangle (P-19).
    PushClip(Rect),
    /// Pop the innermost clip.
    PopClip,
    /// Block image (R-IMG-03/04): the platform draws the decoded pixels of `src` into `rect`
    /// (`ready` = the size comes from the probe, not the 4:3 estimate).
    Image {
        /// Display rectangle (logical px).
        rect: Rect,
        /// Source as written.
        src: Box<str>,
        /// Probed size.
        ready: bool,
    },
    /// Scrollbar of region `regions[i]` (drawn with the current offset).
    HScrollBar(usize),
    /// Code block header toolbar (drawn by the platform on hover / focus, B.6.7).
    CodeToolbar {
        /// Detail node of the code block.
        node: usize,
        /// The whole card.
        card: Rect,
        /// Language label.
        label: String,
        /// Label rectangle.
        label_rect: Rect,
        /// [copy, line numbers, wrap] buttons.
        buttons: [Rect; 3],
        /// Current options.
        opts: crate::code::CodeOpts,
        /// Source range of the code text (copy).
        raw: std::ops::Range<u32>,
    },
}

/// Interactive element kinds (hit testing, focus, activation).
#[derive(Debug, Clone, PartialEq)]
pub enum HotKind {
    /// Code block: copy.
    CodeCopy {
        /// Detail node.
        node: usize,
    },
    /// Code block: toggle line numbers.
    CodeLines {
        /// Detail node.
        node: usize,
    },
    /// Code block: toggle wrap.
    CodeWrap {
        /// Detail node.
        node: usize,
    },
    /// Foldable callout / collapsible container title row (R-REN-06); front matter card.
    Fold {
        /// Detail node.
        node: usize,
    },
    /// Footnote reference `[n]` (R-REN-10): activation scrolls to note `n`. `ord` counts the
    /// references to the same note earlier in this block.
    FootRef {
        /// Note number (1-based).
        n: u32,
        /// Occurrence within the block.
        ord: u32,
    },
    /// Footnote back-link `↩` / `↩²` …: activation scrolls to reference `k` (0-based) of note `n`.
    FootBack {
        /// Note number.
        n: u32,
        /// Reference index in [`yamdv_core::doc::index::FootnoteUse::refs`].
        k: u32,
    },
    /// Link text (R-LINK-*): index into the block's `Detail::links`.
    Link {
        /// Link index.
        link: u32,
    },
    /// Target rectangle of note `n` in the notes section (not clickable; navigation anchor).
    FootNote {
        /// Note number.
        n: u32,
    },
    /// A block image (tooltip: alt/path; not focusable).
    Image {
        /// Source as written.
        src: Box<str>,
    },
    /// R-IMG-06 placeholder card (tooltip with the details).
    ImagePlaceholder {
        /// Source as written.
        src: Box<str>,
        /// Reason.
        reason: yamdv_core::image::Placeholder,
    },
    /// [Copy URL] button of a remote-image placeholder.
    CopyUrl {
        /// URL.
        url: Box<str>,
    },
}

/// An interactive rectangle in block coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct Hot {
    /// Rectangle.
    pub rect: Rect,
    /// What it is.
    pub kind: HotKind,
}

/// A shaped paragraph placed in the block.
#[derive(Debug)]
pub struct LaidPara {
    /// Origin.
    pub x: f32,
    /// Origin.
    pub y: f32,
    /// Shaped text.
    pub shaped: Box<dyn Shaped>,
    /// Source mapping (`None` for decorations such as list numbers and labels).
    pub map: Option<ParaText>,
    /// Horizontal scroll region of the block this paragraph scrolls with (code blocks).
    pub hscroll: Option<u16>,
}

/// A horizontally scrollable region of a block (R-CODE-03, B.6.7).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HRegion {
    /// Visible area (block coordinates); content is clipped to it.
    pub view: Rect,
    /// Content width.
    pub content_w: f32,
    /// Scrollbar track (block coordinates), 6 px tall.
    pub track: Rect,
}

impl HRegion {
    /// Maximum scroll offset.
    pub fn max_scroll(&self) -> f32 {
        (self.content_w - self.view.w).max(0.0)
    }
    /// Scrollbar thumb for an offset (block coordinates).
    pub fn thumb(&self, off: f32) -> Rect {
        let t = self.track;
        if self.content_w <= 0.0 {
            return t;
        }
        let w = (t.w * self.view.w / self.content_w).clamp(24.0_f32.min(t.w), t.w);
        let max = self.max_scroll();
        let x = if max > 0.0 {
            t.x + (t.w - w) * (off / max).clamp(0.0, 1.0)
        } else {
            t.x
        };
        Rect::new(x, t.y, w, t.h)
    }
}

/// Layout of one top-level block.
#[derive(Debug, Default)]
pub struct BlockLayout {
    /// Content height (without the gap before the block).
    pub height: f32,
    /// Display items in paint order.
    pub items: Vec<Item>,
    /// Paragraphs.
    pub paras: Vec<LaidPara>,
    /// Horizontal scroll regions.
    pub regions: Vec<HRegion>,
    /// Interactive elements in document order.
    pub hot: Vec<Hot>,
}

/// Localized strings layout needs.
pub trait Strings {
    /// GFM alert title (`Note`, `Tip`, …).
    fn alert(&self, a: Alert) -> &str;
    /// Placeholder card label (`table`, `html`, …).
    fn placeholder(&self, p: Placeholder) -> &str;
    /// R-IMG-06 placeholder message.
    fn image_message(&self, p: &yamdv_core::image::Placeholder) -> &str {
        p.message()
    }
    /// Label of the [Copy URL] button.
    fn copy_url(&self) -> &str {
        "Copy URL"
    }
    /// Collapsed front matter summary (R-REN-13): "Front matter · N fields".
    fn front_matter_summary(&self, n: usize) -> String {
        if n == 1 {
            "Front matter · 1 field".to_string()
        } else {
            format!("Front matter · {n} fields")
        }
    }
}

/// English fallback strings (tests).
#[derive(Debug, Default)]
pub struct EnStrings;

impl Strings for EnStrings {
    fn alert(&self, a: Alert) -> &str {
        match a {
            Alert::Note => "Note",
            Alert::Tip => "Tip",
            Alert::Important => "Important",
            Alert::Warning => "Warning",
            Alert::Caution => "Caution",
        }
    }
    fn placeholder(&self, p: Placeholder) -> &str {
        placeholder_label(p)
    }
}

/// Untranslated technical label of a placeholder (§4.4.3).
pub fn placeholder_label(p: Placeholder) -> &'static str {
    match p {
        Placeholder::Table => "table",
        Placeholder::Html => "html",
        Placeholder::Math => "math",
        Placeholder::Footnotes => "footnotes",
        Placeholder::FrontMatter => "front matter",
        Placeholder::Details => "details",
    }
}

/// Layout context.
pub struct Ctx<'a> {
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
    /// Top-level block being laid out.
    pub block: usize,
    /// Code options for (block, node).
    pub code_opts: &'a dyn Fn(usize, usize) -> crate::code::CodeOpts,
    /// Image knowledge of the platform.
    pub images: &'a dyn crate::img::ImageHost,
}

/// Vertical margins (top, bottom) of a top-level block kind (B.6.2–B.6.9).
pub fn margins(kind: BlockKind, m: &Metrics) -> (f32, f32) {
    match kind {
        BlockKind::Heading(1) => (32.0, 16.0),
        BlockKind::Heading(2) => (24.0, 16.0),
        BlockKind::Heading(3) => (24.0, 12.0),
        BlockKind::Heading(_) => (20.0, 8.0),
        BlockKind::BlockQuote { .. } | BlockKind::Code => (16.0, 16.0),
        BlockKind::Rule => (24.0, 24.0),
        // R-REN-10: definitions vanish from their source position.
        BlockKind::FootnoteDef => (0.0, 0.0),
        // B.6.9: front matter card margin 0 0 16.
        BlockKind::FrontMatter => (0.0, 16.0),
        BlockKind::Table | BlockKind::Html | BlockKind::Math => (16.0, 16.0),
        BlockKind::Paragraph | BlockKind::List | BlockKind::DefList => (0.0, m.para_spacing),
    }
}

/// Gap before top-level block `i` (collapsed margins; H1 has no top margin as first block).
pub fn gap_before(blocks: &[Block], i: usize, m: &Metrics) -> f32 {
    let Some(b) = blocks.get(i) else { return 0.0 };
    let (mt, _) = margins(b.kind, m);
    match i.checked_sub(1).and_then(|p| blocks.get(p)) {
        None => 0.0,
        Some(prev) => mt.max(margins(prev.kind, m).1),
    }
}

/// Height estimate of a not-yet-laid-out block (§4.4.4, documented in ADR-0007):
/// text blocks `max(lines, ceil(half_ems × 0.5 em / width)) × pitch`; code and placeholders
/// `lines × code_pitch + paddings`; rules 2 px.
pub fn estimate(b: &Block, width: f32, m: &Metrics) -> f32 {
    if let Some(h) = estimate_special(b, m) {
        return h;
    }
    let width = width.max(1.0);
    let wrapped = |size: f32, pitch: f32| {
        let w = b.est.half_ems as f32 * 0.5 * size;
        (w / width).ceil().max(b.est.lines as f32).max(1.0) * pitch
    };
    match b.kind {
        BlockKind::Heading(l) => wrapped(m.heading_size(l), m.heading_size(l) * 1.25),
        BlockKind::Code
        | BlockKind::Table
        | BlockKind::Html
        | BlockKind::Math
        | BlockKind::FootnoteDef
        | BlockKind::FrontMatter => {
            let lines = if b.kind == BlockKind::Code {
                b.est.lines.saturating_sub(2).max(1)
            } else {
                b.est.lines
            };
            lines as f32 * m.code_pitch() + CODE_HEADER + CODE_PAD_TOP + CODE_PAD_BOTTOM
        }
        BlockKind::Rule => 2.0,
        _ => wrapped(m.font_size, m.pitch()),
    }
}

/// Height estimate including the special cases that need no source scan.
fn estimate_special(b: &Block, m: &Metrics) -> Option<f32> {
    match b.kind {
        BlockKind::FootnoteDef => Some(0.0),
        BlockKind::FrontMatter => Some(13.0 * 1.5 + 16.0),
        _ => {
            let _ = m;
            None
        }
    }
}

pub(crate) fn base(m: &Metrics, color: u32) -> Style {
    Style {
        family: Family::Body,
        size: m.font_size,
        weight: 400,
        italic: false,
        color,
        strike: false,
        underline: false,
    }
}

pub(crate) fn spans(segs: &[Seg], b: &Style, m: &Metrics, pal: &Palette) -> Vec<Span> {
    let mut out = Vec::new();
    for s in segs {
        if s.style == 0 {
            continue;
        }
        let mut t = *b;
        let f = s.style;
        if f & st::STRONG != 0 {
            t.weight = 600;
        }
        if f & st::EM != 0 {
            t.italic = true;
        }
        if f & st::STRIKE != 0 {
            t.strike = true;
        }
        if f & st::CODE != 0 {
            // PHASE-003: inline math also uses the inline-code style until math rendering lands.
            t.family = Family::Code;
            t.size = b.size * 0.875;
        }
        if f & st::LINK != 0 {
            t.color = pal.link;
        }
        if f & (st::MUTED | st::IMAGE) != 0 {
            t.color = pal.text_muted;
        }
        if f & st::IMAGE != 0 {
            // P-25 compact image chip: 12 px glyph + alt text in `text_muted`.
            t.size = 12.0 * b.size / 14.0;
            t.weight = 400;
            t.italic = false;
        }
        if f & (st::SUP | st::SUB) != 0 {
            // R-MD-06: 0.75 em. PARTIAL: no baseline shift until inline objects (§4.1.3).
            t.size = b.size * 0.75;
        }
        if f & (st::UNDERLINE | st::ABBR) != 0 {
            t.underline = true;
        }
        if f & st::KBD != 0 {
            // B.6.3 key cap: code font on the inline-code background.
            t.family = Family::Code;
            t.size = b.size * 0.875;
        }
        if f & st::HIGHLIGHT != 0 && pal.highlight_text != 0 {
            t.color = pal.highlight_text;
        }
        if f & st::FOOTREF != 0 {
            // PHASE-003: superscript-styled footnote reference without navigation.
            t.color = pal.link;
            t.size = b.size * 0.75;
        }
        let _ = m;
        out.push(Span {
            range: s.u16_start..s.u16_start + s.u16_len,
            style: t,
        });
    }
    out
}

pub(crate) struct Out {
    pub(crate) items: Vec<Item>,
    pub(crate) paras: Vec<LaidPara>,
    pub(crate) regions: Vec<HRegion>,
    pub(crate) hot: Vec<Hot>,
    /// Horizontal alignment of inline leaves inside an HTML alignment container (0 = start).
    pub(crate) align: u8,
    /// This block opens a `<details>` closed in a later block: its last details node toggles
    /// with the fold key `usize::MAX` (§4.6 item 3).
    pub(crate) cross_details: bool,
}

/// Fold key (node) of a cross-block `<details>` toggle.
pub const CROSS_DETAILS: usize = usize::MAX;

impl Out {
    pub(crate) fn para(
        &mut self,
        x: f32,
        y: f32,
        shaped: Box<dyn Shaped>,
        map: Option<ParaText>,
        hscroll: Option<u16>,
    ) -> f32 {
        let h = shaped.height();
        self.paras.push(LaidPara {
            x,
            y,
            shaped,
            map,
            hscroll,
        });
        self.items.push(Item::Para(self.paras.len() - 1));
        h
    }
}

pub(crate) fn shape_text(
    ctx: &mut Ctx<'_>,
    text: &str,
    style: Style,
    width: Option<f32>,
    pitch: f32,
) -> Box<dyn Shaped> {
    ctx.shaper.shape(&ParaSpec {
        text,
        base: style,
        spans: vec![],
        width,
        line_pitch: pitch,
        rtl: is_rtl(text),
        tab_spaces: None,
    })
}

/// Inline leaf → shaped paragraph with inline-code backgrounds.
pub(crate) fn leaf(
    ctx: &mut Ctx<'_>,
    pt: ParaText,
    style: Style,
    pitch: f32,
    x: f32,
    y: f32,
    w: f32,
    out: &mut Out,
) -> f32 {
    let a = out.align;
    leaf_aligned(ctx, pt, style, pitch, x, y, w, a, out)
}

/// [`leaf`] with a horizontal alignment of the shaped box inside `w` (0/1 start, 2 center,
/// 3 end; used by table cells).
pub(crate) fn leaf_aligned(
    ctx: &mut Ctx<'_>,
    pt: ParaText,
    style: Style,
    pitch: f32,
    x: f32,
    y: f32,
    w: f32,
    align: u8,
    out: &mut Out,
) -> f32 {
    let sp = spans(&pt.segs, &style, &ctx.m, ctx.pal);
    let rtl = is_rtl(&pt.text);
    let shaped = ctx.shaper.shape(&ParaSpec {
        text: &pt.text,
        base: style,
        spans: sp.clone(),
        width: Some(w),
        line_pitch: pitch,
        rtl,
        tab_spaces: None,
    });
    let x = match align {
        2 => x + ((w - shaped.width()) / 2.0).max(0.0),
        3 => x + (w - shaped.width()).max(0.0),
        _ => x,
    };
    // B.6.3: inline code background, padding 0.15 em × 0.4 em, radius 4 — drawn before the text.
    // The horizontal padding is part of the shaped advance (character spacing, F-13/P-21), so the
    // background is exactly the range rectangle and never overlaps neighbouring glyphs.
    // R-MD-05: highlight background (radius 2), behind the text like inline code.
    for (s, seg) in sp.iter().zip(pt.segs.iter().filter(|g| g.style != 0)) {
        if seg.style & st::HIGHLIGHT != 0 {
            for r in shaped.range_rects(s.range.clone()) {
                out.items.push(Item::Fill {
                    rect: Rect::new(x + r.x, y + r.y, r.w, r.h),
                    color: ctx.pal.highlight_bg,
                    radius: 2.0,
                });
            }
        }
    }
    // P-25: image chip — line-height tall, radius 4, `code_bg` fill behind glyph + alt text.
    for (s, seg) in sp.iter().zip(pt.segs.iter().filter(|g| g.style != 0)) {
        if seg.style & st::IMAGE != 0 {
            for r in shaped.range_rects(s.range.clone()) {
                out.items.push(Item::Fill {
                    rect: Rect::new(x + r.x - 3.0, y + r.y, r.w + 6.0, r.h),
                    color: ctx.pal.code_bg,
                    radius: 4.0,
                });
            }
        }
    }
    for s in &sp {
        if s.style.family == Family::Code {
            let em = s.style.size / 0.875;
            for r in shaped.range_rects(s.range.clone()) {
                let ih = s.style.size * 1.3 + 0.3 * em;
                let iy = r.y + (r.h - ih) / 2.0;
                out.items.push(Item::Fill {
                    rect: Rect::new(x + r.x, y + iy, r.w, ih),
                    color: ctx.pal.inline_code_bg,
                    radius: 4.0,
                });
            }
        }
    }
    // B.6.3 `<kbd>` key cap: 1 px border with a 2 px bottom border around the code background.
    for (s, seg) in sp.iter().zip(pt.segs.iter().filter(|g| g.style != 0)) {
        if seg.style & st::KBD == 0 {
            continue;
        }
        let em = s.style.size / 0.875;
        for r in shaped.range_rects(s.range.clone()) {
            let ih = s.style.size * 1.3 + 0.3 * em;
            let iy = r.y + (r.h - ih) / 2.0;
            let rect = Rect::new(x + r.x, y + iy, r.w, ih);
            out.items.push(Item::Stroke {
                rect,
                color: ctx.pal.code_border,
                radius: 4.0,
            });
            out.items.push(Item::Fill {
                rect: Rect::new(
                    rect.x + 2.0,
                    rect.bottom() - 1.0,
                    (rect.w - 4.0).max(0.0),
                    1.0,
                ),
                color: ctx.pal.code_border,
                radius: 0.0,
            });
        }
    }
    // R-LINK-*: link text is activatable (one hot rect per line fragment).
    for (s, seg) in sp.iter().zip(pt.segs.iter().filter(|g| g.style != 0)) {
        if seg.link == yamdv_core::doc::detail::NO_LINK {
            continue;
        }
        for r in shaped.range_rects(s.range.clone()) {
            out.hot.push(Hot {
                rect: Rect::new(x + r.x, y + r.y, r.w, r.h),
                kind: HotKind::Link { link: seg.link },
            });
        }
    }
    // R-REN-10: footnote references are activatable.
    for (s, seg) in sp.iter().zip(pt.segs.iter().filter(|g| g.style != 0)) {
        if seg.style & yamdv_core::doc::detail::style::FOOTREF == 0 {
            continue;
        }
        let label = pt.slice16(s.range.start, s.range.end);
        let Ok(n) = label.trim_matches(['[', ']']).parse::<u32>() else {
            continue;
        };
        let ord = u32::try_from(
            out.hot
                .iter()
                .filter(|h| matches!(h.kind, HotKind::FootRef { n: m, .. } if m == n))
                .count(),
        )
        .unwrap_or(0);
        if let Some(r) = shaped.range_rects(s.range.clone()).first() {
            out.hot.push(Hot {
                rect: Rect::new(x + r.x, y + r.y, r.w.max(6.0), r.h),
                kind: HotKind::FootRef { n, ord },
            });
        }
    }
    out.para(x, y, shaped, Some(pt), None)
}

/// Code-like card: code blocks and interim placeholders (PHASE-003).
#[allow(clippy::too_many_arguments)]
pub(crate) fn card(
    ctx: &mut Ctx<'_>,
    pt: ParaText,
    label: Option<&str>,
    x: f32,
    y: f32,
    w: f32,
    out: &mut Out,
) -> f32 {
    let m = ctx.m;
    let code = Style {
        family: Family::Code,
        size: m.code_size(),
        ..base(&m, ctx.pal.text)
    };
    let shaped = ctx.shaper.shape(&ParaSpec {
        text: &pt.text,
        base: code,
        spans: vec![],
        width: None,
        line_pitch: m.code_pitch(),
        rtl: false,
        tab_spaces: Some(4),
    });
    let text_h = shaped.height().max(m.code_pitch());
    let h = CODE_HEADER + CODE_PAD_TOP + text_h + CODE_PAD_BOTTOM;
    let rect = Rect::new(x, y, w, h);
    out.items.push(Item::Fill {
        rect,
        color: ctx.pal.code_bg,
        radius: 8.0,
    });
    if let Some(label) = label {
        let ls = Style {
            size: 11.0,
            color: ctx.pal.text_muted,
            ..base(&m, ctx.pal.text_muted)
        };
        let l = shape_text(ctx, label, ls, None, 11.0 * 1.4);
        let lw = l.width();
        let lh = l.height();
        out.para(
            x + w - 8.0 - lw,
            y + (CODE_HEADER - lh) / 2.0 + 4.0,
            l,
            None,
            None,
        );
    }
    let inner = Rect::new(
        x + CODE_PAD_X,
        y + CODE_HEADER + CODE_PAD_TOP,
        (w - 2.0 * CODE_PAD_X).max(0.0),
        text_h,
    );
    let overflow = shaped.width() > inner.w;
    if overflow {
        // R-CODE-03 / P-19: the code scrolls horizontally inside the card, clipped to the card
        // interior; a 6 px scrollbar sits in the bottom padding while overflowing (B.6.7).
        let view = Rect::new(
            x + 1.0,
            inner.y - CODE_PAD_TOP,
            w - 2.0,
            text_h + CODE_PAD_TOP,
        );
        let region = out.regions.len();
        out.regions.push(HRegion {
            view,
            content_w: shaped.width() + 2.0 * CODE_PAD_X,
            track: Rect::new(
                inner.x,
                inner.bottom() + (CODE_PAD_BOTTOM - 6.0) / 2.0,
                inner.w,
                6.0,
            ),
        });
        out.items.push(Item::PushClip(view));
        out.para(inner.x, inner.y, shaped, Some(pt), Some(region as u16));
        out.items.push(Item::PopClip);
        out.items.push(Item::HScrollBar(region));
    } else {
        out.para(inner.x, inner.y, shaped, Some(pt), None);
    }
    out.items.push(Item::Stroke {
        rect,
        color: ctx.pal.code_border,
        radius: 8.0,
    });
    h
}

pub(crate) fn source_text(src: &str, r: &std::ops::Range<u32>) -> ParaText {
    let mut t = src.get(r.start as usize..r.end as usize).unwrap_or("");
    while t.ends_with('\n') || t.ends_with('\r') {
        t = &t[..t.len() - 1];
    }
    let len16 = u32::try_from(t.encode_utf16().count()).unwrap_or(0);
    let end = r.start + u32::try_from(t.len()).unwrap_or(0);
    ParaText {
        text: t.to_string(),
        segs: vec![Seg {
            u16_start: 0,
            u16_len: len16,
            byte_start: 0,
            src: r.start..end,
            verbatim: true,
            style: 0,
            link: yamdv_core::doc::detail::NO_LINK,
        }],
    }
}

fn node_margins(kind: &NodeKind, m: &Metrics, tight: bool) -> (f32, f32) {
    match kind {
        NodeKind::Paragraph { .. } if tight => (0.0, 0.0),
        NodeKind::Paragraph { .. } | NodeKind::DefBody => (0.0, m.para_spacing),
        NodeKind::DefTitle => (0.0, 0.0),
        NodeKind::Heading(l) => margins(BlockKind::Heading(*l), m),
        NodeKind::BlockQuote(_)
        | NodeKind::Code { .. }
        | NodeKind::Placeholder(_)
        | NodeKind::Table { .. } => (16.0, 16.0),
        NodeKind::TableRow { .. } | NodeKind::TableCell | NodeKind::FootnoteDef { .. } => {
            (0.0, 0.0)
        }
        NodeKind::Rule => (24.0, 24.0),
        NodeKind::List { .. } | NodeKind::DefList => {
            (0.0, if tight { 0.0 } else { m.para_spacing })
        }
        NodeKind::Item { .. } | NodeKind::HtmlBlock | NodeKind::Align(_) => (0.0, 0.0),
        NodeKind::Details { .. } => (8.0, 8.0),
        NodeKind::DetailsSummary | NodeKind::Unsupported => (0.0, m.para_spacing),
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Env {
    pub(crate) color: u32,
    pub(crate) list_depth: usize,
    pub(crate) tight: bool,
}

/// Lays out `children` vertically; returns the bottom y (without the last margin).
#[allow(clippy::too_many_arguments)]
pub(crate) fn stack(
    ctx: &mut Ctx<'_>,
    src: &str,
    d: &Detail,
    children: &[usize],
    x: f32,
    y0: f32,
    w: f32,
    env: &Env,
    out: &mut Out,
) -> f32 {
    let mut y = y0;
    let mut prev_mb: Option<f32> = None;
    for &c in children {
        let (mt, mb) = node_margins(&d.nodes[c].kind, &ctx.m, env.tight);
        if let Some(p) = prev_mb {
            y += mt.max(p);
        }
        y += node(ctx, src, d, c, x, y, w, env, out);
        prev_mb = Some(mb);
    }
    y
}

/// Lays out node `i` at (x, y) with width `w`; returns its height.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn node(
    ctx: &mut Ctx<'_>,
    src: &str,
    d: &Detail,
    i: usize,
    x: f32,
    y: f32,
    w: f32,
    env: &Env,
    out: &mut Out,
) -> f32 {
    let m = ctx.m;
    let kids: Vec<usize> = d.children(i).collect();
    match &d.nodes[i].kind {
        NodeKind::Paragraph { .. } if crate::img::sole_image(src, d, i).is_some() => {
            match crate::img::sole_image(src, d, i) {
                Some(r) => crate::img::block(ctx, r, x, y, w, out),
                None => 0.0,
            }
        }
        NodeKind::Paragraph { .. } | NodeKind::DefTitle => {
            let pt = node_text(src, d, i, ctx.single_newline_break);
            let mut s = base(&m, env.color);
            if matches!(d.nodes[i].kind, NodeKind::DefTitle) {
                s.weight = 600;
            }
            leaf(ctx, pt, s, m.pitch(), x, y, w, out)
        }
        NodeKind::Heading(l) => {
            let pt = node_text(src, d, i, ctx.single_newline_break);
            let color = if *l == 6 {
                ctx.pal.text_muted
            } else {
                ctx.pal.heading
            };
            let size = m.heading_size(*l);
            let s = Style {
                size,
                weight: 600,
                ..base(&m, color)
            };
            leaf(ctx, pt, s, size * 1.25, x, y, w, out)
        }
        NodeKind::Code { lang } => {
            let pt = node_text(src, d, i, true);
            crate::code::code_block(ctx, i, lang, pt, x, y, w, out)
        }
        NodeKind::Table { aligns } => crate::table::table(ctx, src, d, i, aligns, x, y, w, out),
        NodeKind::TableRow { .. } | NodeKind::TableCell => 0.0,
        // R-REN-10: laid out in the notes section (see `layout_with_notes`).
        NodeKind::FootnoteDef { .. } => 0.0,
        // R-MD-14: sanitized HTML maps onto the ordinary node kinds.
        NodeKind::HtmlBlock => stack(ctx, src, d, &kids, x, y, w, env, out) - y,
        NodeKind::Align(a) => {
            let saved = out.align;
            out.align = *a;
            let h = stack(ctx, src, d, &kids, x, y, w, env, out) - y;
            out.align = saved;
            h
        }
        NodeKind::Details { open } => {
            crate::front::details(ctx, src, d, i, *open, x, y, w, env, out)
        }
        NodeKind::DetailsSummary => {
            let pt = node_text(src, d, i, ctx.single_newline_break);
            let s = Style {
                weight: 600,
                ..base(&m, env.color)
            };
            leaf(ctx, pt, s, m.pitch(), x, y, w, out)
        }
        NodeKind::Unsupported => {
            let label = ctx.strings.placeholder(Placeholder::Html).to_string();
            let pt = source_text(src, &d.nodes[i].range);
            card(ctx, pt, Some(&label), x, y, w, out)
        }
        NodeKind::Placeholder(Placeholder::FrontMatter) => {
            crate::front::front_matter(ctx, src, d, i, x, y, w, out)
        }
        NodeKind::Placeholder(p) => {
            // PHASE-003: tables, HTML, math, footnotes, front matter are shown as source cards.
            let label = ctx.strings.placeholder(*p).to_string();
            let pt = source_text(src, &d.nodes[i].range);
            card(ctx, pt, Some(&label), x, y, w, out)
        }
        NodeKind::Rule => {
            out.items.push(Item::Fill {
                rect: Rect::new(x, y, w, 2.0),
                color: ctx.pal.border_subtle,
                radius: 0.0,
            });
            2.0
        }
        NodeKind::BlockQuote(alert) => {
            if let Some(c) = crate::callout::detect(src, d, i, *alert, ctx.single_newline_break) {
                return crate::callout::callout(ctx, src, d, i, c, x, y, w, env, out);
            }
            let bar_at = out.items.len();
            let cx = x + 3.0 + 16.0;
            let cw = (w - 19.0).max(1.0);
            let inner = Env {
                color: ctx.pal.quote_text,
                list_depth: env.list_depth,
                tight: false,
            };
            let bottom = stack(ctx, src, d, &kids, cx, y, cw, &inner, out);
            let h = (bottom - y).max(m.pitch());
            out.items.insert(
                bar_at,
                Item::Fill {
                    rect: Rect::new(x, y, 3.0, h),
                    color: ctx.pal.quote_bar,
                    radius: 0.0,
                },
            );
            // Para indices are unaffected by inserting a non-Para item.
            h
        }
        NodeKind::List { start, tight } => {
            let depth = env.list_depth + 1;
            let mut n = start.unwrap_or(1);
            let mut yy = y;
            for (k, &item) in kids.iter().enumerate() {
                if k > 0 && !tight {
                    yy += 8.0;
                }
                let content_x = x + 40.0;
                let cw = (w - 40.0).max(1.0);
                let task = if let NodeKind::Item { task } = d.nodes[item].kind {
                    task
                } else {
                    None
                };
                // Marker sits right-aligned in the 40 px box with an 8 px gap (B.6.4).
                let marker_right = content_x - 8.0;
                match (task, start) {
                    (Some(done), _) => {
                        // B.6.4: vector 16 × 16 box, radius 4, 1.5 px border; checked → accent
                        // fill with a white check. Not clickable (P-16/P-17).
                        let r = Rect::new(
                            marker_right - 16.0,
                            yy + (m.pitch() - 16.0) / 2.0,
                            16.0,
                            16.0,
                        );
                        if done {
                            out.items.push(Item::Fill {
                                rect: r,
                                color: ctx.pal.accent,
                                radius: 4.0,
                            });
                            out.items.push(Item::Line {
                                pts: vec![
                                    (r.x + 4.0, r.y + 8.5),
                                    (r.x + 7.0, r.y + 11.5),
                                    (r.x + 12.0, r.y + 5.0),
                                ],
                                color: 0xffffff,
                                width: 2.0,
                                closed: false,
                            });
                        } else {
                            let border = if crate::code::is_dark(ctx.pal.bg) {
                                0x6a6a6a
                            } else {
                                0xb0b0b0
                            };
                            out.items.push(Item::StrokeW {
                                rect: r,
                                color: border,
                                radius: 4.0,
                                width: 1.5,
                            });
                        }
                    }
                    (None, Some(_)) => {
                        let s = base(&m, env.color);
                        let sh = shape_text(ctx, &format!("{n}."), s, None, m.pitch());
                        let mw = sh.width();
                        out.para(marker_right - mw, yy, sh, None, None);
                    }
                    (None, None) => {
                        let size = 0.35 * m.font_size;
                        let shape = match depth {
                            1 => Bullet::Disc,
                            2 => Bullet::Ring,
                            _ => Bullet::Square,
                        };
                        out.items.push(Item::Bullet {
                            cx: marker_right - size / 2.0,
                            cy: yy + m.pitch() / 2.0,
                            size,
                            shape,
                            color: env.color,
                        });
                    }
                }
                let ikids: Vec<usize> = d.children(item).collect();
                let ienv = Env {
                    color: env.color,
                    list_depth: depth,
                    tight: *tight,
                };
                let bottom = stack(ctx, src, d, &ikids, content_x, yy, cw, &ienv, out);
                yy = bottom.max(yy + m.pitch());
                n += 1;
            }
            yy - y
        }
        NodeKind::Item { .. } => stack(ctx, src, d, &kids, x, y, w, env, out) - y,
        NodeKind::DefList => {
            stack(
                ctx,
                src,
                d,
                &kids,
                x,
                y,
                w,
                &Env {
                    tight: true,
                    ..*env
                },
                out,
            ) - y
        }
        NodeKind::DefBody => {
            let inner = Env {
                tight: true,
                ..*env
            };
            stack(
                ctx,
                src,
                d,
                &kids,
                x + 24.0,
                y,
                (w - 24.0).max(1.0),
                &inner,
                out,
            ) - y
        }
    }
}

/// Lays out a top-level block from its detail tree at column width `w`.
pub fn layout_block(ctx: &mut Ctx<'_>, src: &str, d: &Detail, w: f32) -> BlockLayout {
    layout_block_in(ctx, src, d, w, 0, false)
}

/// [`layout_block`] inside cross-block HTML containers: initial alignment and whether the
/// block opens a cross-block `<details>`.
pub fn layout_block_in(
    ctx: &mut Ctx<'_>,
    src: &str,
    d: &Detail,
    w: f32,
    align: u8,
    cross_details: bool,
) -> BlockLayout {
    let mut out = Out {
        items: Vec::new(),
        paras: Vec::new(),
        regions: Vec::new(),
        hot: Vec::new(),
        align,
        cross_details,
    };
    let roots: Vec<usize> = d.roots().collect();
    let env = Env {
        color: ctx.pal.text,
        list_depth: 0,
        tight: false,
    };
    let bottom = stack(ctx, src, d, &roots, 0.0, 0.0, w, &env, &mut out);
    BlockLayout {
        height: bottom.max(0.0),
        items: out.items,
        paras: out.paras,
        regions: out.regions,
        hot: out.hot,
    }
}

/// One note of the footnotes section: number, reference count, detail tree of its definition.
pub struct Note {
    /// Note number (1-based).
    pub n: u32,
    /// Number of references (back-links `↩`, `↩²`, …).
    pub refs: u32,
    /// Detail tree of the top-level block holding the definition.
    pub detail: Detail,
}

/// [`layout_block`] plus the footnotes section appended (the last block of a document with
/// referenced footnotes, R-REN-10 / B.6.9).
pub fn layout_with_notes(
    ctx: &mut Ctx<'_>,
    src: &str,
    d: &Detail,
    notes: &[Note],
    w: f32,
) -> BlockLayout {
    let mut bl = layout_block(ctx, src, d, w);
    if notes.is_empty() {
        return bl;
    }
    let mut out = Out {
        items: std::mem::take(&mut bl.items),
        paras: std::mem::take(&mut bl.paras),
        regions: std::mem::take(&mut bl.regions),
        hot: std::mem::take(&mut bl.hot),
        align: 0,
        cross_details: false,
    };
    let bottom = crate::front::notes_section(ctx, src, notes, bl.height, w, &mut out);
    BlockLayout {
        height: bottom,
        items: out.items,
        paras: out.paras,
        regions: out.regions,
        hot: out.hot,
    }
}
