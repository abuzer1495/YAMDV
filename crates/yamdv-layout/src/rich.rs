//! Math (R-MATH-*) and Mermaid (R-MMD-*) glue between the pure engines and block layout:
//! the platform host ([`RichHost`]), the per-document cache, display/inline math layout.

// f32 pixel quantities; indices from the layout's own arenas.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::indexing_slicing
)]

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use yamdv_core::doc::text::ParaText;
use yamdv_math::font::MathFont;
use yamdv_math::{MathBox, MathError, MathTable};

use crate::block::{Ctx, HRegion, Item, Out, base, card};
use crate::geom::Rect;
use crate::shaper::{Family, Style};

/// A loaded math font with its parsed `MATH` table.
pub struct MathFace {
    /// Font.
    pub font: Box<dyn MathFont>,
    /// Parsed `MATH` table.
    pub table: MathTable,
}

impl std::fmt::Debug for MathFace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MathFace")
    }
}

impl MathFace {
    /// Wraps a font and parses its `MATH` table.
    pub fn new(font: Box<dyn MathFont>) -> Self {
        let table = MathTable::new(font.math_table(), font.units_per_em());
        Self { font, table }
    }
}

/// What the platform provides for math and diagrams.
pub trait RichHost {
    /// The math font, loaded lazily on first use (§2.1); `None` = unavailable.
    fn math(&self) -> Option<&MathFace>;
    /// `reading.render_math`.
    fn render_math(&self) -> bool;
    /// `reading.render_mermaid`.
    fn render_mermaid(&self) -> bool;
    /// Per-document cache (`None` = no caching).
    fn cache(&self) -> Option<&RichCache>;
}

/// A host without math font or cache (pure tests, fuzzing): math and diagrams render as source.
#[derive(Debug, Default)]
pub struct NoRich;

impl RichHost for NoRich {
    fn math(&self) -> Option<&MathFace> {
        None
    }
    fn render_math(&self) -> bool {
        true
    }
    fn render_mermaid(&self) -> bool {
        true
    }
    fn cache(&self) -> Option<&RichCache> {
        None
    }
}

/// Shared [`NoRich`] instance.
pub static NO_RICH: NoRich = NoRich;

/// Test host: the mock math font and a cache.
#[derive(Debug)]
pub struct MockRich {
    face: MathFace,
    cache: RichCache,
}

impl Default for MockRich {
    fn default() -> Self {
        Self {
            face: MathFace::new(Box::new(yamdv_math::font::MockFont::default())),
            cache: RichCache::default(),
        }
    }
}

impl RichHost for MockRich {
    fn math(&self) -> Option<&MathFace> {
        Some(&self.face)
    }
    fn render_math(&self) -> bool {
        true
    }
    fn render_mermaid(&self) -> bool {
        true
    }
    fn cache(&self) -> Option<&RichCache> {
        Some(&self.cache)
    }
}

/// Result of laying out one formula.
pub type MathResult = Result<Rc<MathBox>, MathError>;

/// Entries kept per cache map before it is cleared.
const CACHE_CAP: usize = 4096;

/// Per-document cache of formula and diagram layouts (keyed by source hash) and the diagrams
/// the user chose to render anyway (R-MMD-05).
#[derive(Debug, Default)]
pub struct RichCache {
    math: RefCell<HashMap<(u64, u32, bool), MathResult>>,
    pub(crate) diagrams: RefCell<HashMap<(u64, bool), Rc<crate::diagram::Laid>>>,
    forced: RefCell<HashSet<u64>>,
}

impl RichCache {
    /// Marks the diagram with source hash `h` as "render anyway".
    pub fn force(&self, h: u64) {
        self.forced.borrow_mut().insert(h);
    }
    /// Was "render anyway" chosen for `h`?
    pub fn forced(&self, h: u64) -> bool {
        self.forced.borrow().contains(&h)
    }
    /// Cached entries (formulas, diagrams).
    pub fn len(&self) -> (usize, usize) {
        (self.math.borrow().len(), self.diagrams.borrow().len())
    }
    /// Nothing cached.
    pub fn is_empty(&self) -> bool {
        self.len() == (0, 0)
    }
}

/// Stable-within-the-process hash of a source string.
pub fn source_hash(s: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// Formula size: 1.21 em of the body size (B.6.11).
pub fn math_size(body: f32) -> f32 {
    body * 1.21
}

/// Strips `$…$` / `$$…$$` delimiters (inline math runs carry their source with delimiters).
pub fn strip_delims(s: &str) -> (&str, bool) {
    let t = s.trim();
    if let Some(i) = t.strip_prefix("$$").and_then(|r| r.strip_suffix("$$")) {
        return (i, true);
    }
    if let Some(i) = t.strip_prefix('$').and_then(|r| r.strip_suffix('$')) {
        return (i, false);
    }
    (t, false)
}

/// Lays out `tex` (cached); `None` when math rendering is off or no math font is available.
pub fn math_box(ctx: &Ctx<'_>, tex: &str, size: f32, display: bool) -> Option<MathResult> {
    if !ctx.rich.render_math() {
        return None;
    }
    let face = ctx.rich.math()?;
    let key = (source_hash(tex), size.to_bits(), display);
    if let Some(c) = ctx.rich.cache()
        && let Some(r) = c.math.borrow().get(&key)
    {
        return Some(r.clone());
    }
    let r = yamdv_math::render(tex, face.font.as_ref(), &face.table, size, display).map(Rc::new);
    if let Some(c) = ctx.rich.cache() {
        let mut m = c.math.borrow_mut();
        if m.len() >= CACHE_CAP {
            m.clear();
        }
        m.insert(key, r.clone());
    }
    Some(r)
}

/// Display math block (R-MATH-05, B.6.11): centered, horizontally scrollable when too wide;
/// errors show the source in `error` and the message in small `text_muted` text (R-MATH-03).
/// Falls back to a `math` source card when math rendering is off or unavailable.
pub(crate) fn display_math(
    ctx: &mut Ctx<'_>,
    tex: &str,
    pt: ParaText,
    x: f32,
    y: f32,
    w: f32,
    out: &mut Out,
) -> f32 {
    let size = math_size(ctx.m.font_size);
    match math_box(ctx, tex, size, true) {
        None => {
            let label = ctx
                .strings
                .placeholder(yamdv_core::doc::detail::Placeholder::Math)
                .to_string();
            card(ctx, pt, Some(&label), x, y, w, out)
        }
        Some(Ok(mb)) => {
            let h = mb.ascent + mb.descent;
            if mb.width <= w {
                out.items.push(Item::Math {
                    x: x + (w - mb.width) / 2.0,
                    y: y + mb.ascent,
                    mb,
                    color: ctx.pal.text,
                    hscroll: None,
                });
                h
            } else {
                // R-MATH-05: scrolls horizontally inside the block; 6 px bar 4 px below.
                let view = Rect::new(x, y, w, h);
                let region = out.regions.len();
                out.regions.push(HRegion {
                    view,
                    content_w: mb.width,
                    track: Rect::new(x, y + h + 4.0, w, 6.0),
                });
                out.items.push(Item::PushClip(view));
                out.items.push(Item::Math {
                    x,
                    y: y + mb.ascent,
                    mb,
                    color: ctx.pal.text,
                    hscroll: Some(region as u16),
                });
                out.items.push(Item::PopClip);
                out.items.push(Item::HScrollBar(region));
                h + 10.0
            }
        }
        Some(Err(e)) => math_error(ctx, pt, &e, x, y, w, out),
    }
}

/// R-MATH-03: the source in the theme's `error` color plus the message below.
fn math_error(
    ctx: &mut Ctx<'_>,
    pt: ParaText,
    e: &MathError,
    x: f32,
    y: f32,
    w: f32,
    out: &mut Out,
) -> f32 {
    let m = ctx.m;
    let src = Style {
        family: Family::Code,
        size: m.code_size(),
        ..base(&m, ctx.pal.error)
    };
    let shaped = crate::block::shape_text(ctx, &pt.text, src, Some(w), m.code_pitch());
    let mut h = out.para(x, y, shaped, Some(pt), None);
    let msg = Style {
        size: 12.0 * m.font_size / 16.0,
        ..base(&m, ctx.pal.text_muted)
    };
    let text = format!("Math error: {e}");
    let shaped = crate::block::shape_text(ctx, &text, msg, Some(w), msg.size * 1.4);
    h += 2.0;
    h += out.para(x, y + h, shaped, None, None);
    h
}
