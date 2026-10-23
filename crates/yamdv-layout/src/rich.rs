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

/// Rendered diagrams per document (S-PARSE-06).
pub const MAX_DIAGRAMS: usize = 100;
/// Rendered formulas per document (S-PARSE-05).
pub const MAX_FORMULAS: usize = 5000;

/// Entries kept per cache map before it is cleared.
const CACHE_CAP: usize = 4096;

/// Per-document cache of formula and diagram layouts (keyed by source hash) and the diagrams
/// the user chose to render anyway (R-MMD-05).
#[derive(Debug, Default)]
pub struct RichCache {
    math: RefCell<HashMap<(u64, u32, bool, u64), MathResult>>,
    scopes: RefCell<Rc<crate::macros::MacroScopes>>,
    pub(crate) diagrams: RefCell<HashMap<(u64, u32), Rc<crate::diagram::Laid>>>,
    forced: RefCell<HashSet<u64>>,
    rendered: RefCell<HashSet<u64>>,
    formulas: RefCell<HashSet<u64>>,
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
    /// Admits diagram `h` for rendering: at most [`MAX_DIAGRAMS`] distinct diagrams per document
    /// render (S-PARSE-06); later ones stay code blocks.
    pub fn admit(&self, h: u64) -> bool {
        let mut r = self.rendered.borrow_mut();
        if r.contains(&h) {
            return true;
        }
        if r.len() >= MAX_DIAGRAMS {
            return false;
        }
        r.insert(h);
        true
    }
    /// Admits formula `h`: at most [`MAX_FORMULAS`] distinct formulas per document render
    /// (S-PARSE-05); later ones stay source.
    pub fn admit_formula(&self, h: u64) -> bool {
        let mut f = self.formulas.borrow_mut();
        if f.contains(&h) {
            return true;
        }
        if f.len() >= MAX_FORMULAS {
            return false;
        }
        f.insert(h);
        true
    }
    /// Brings the macro scopes (R-MATH-07) up to date with `text` / `ix`; cheap when the
    /// defining blocks did not change.
    pub fn prepare_macros(&self, text: &str, ix: &yamdv_core::doc::index::Index) {
        let cur = self.scopes.borrow().key();
        if ix.macro_blocks.is_empty() && cur == 0 {
            return;
        }
        let key = if ix.macro_blocks.is_empty() {
            0
        } else {
            crate::macros::scope_key(text, ix)
        };
        if key != cur {
            let s = if key == 0 {
                crate::macros::MacroScopes::default()
            } else {
                crate::macros::MacroScopes::build(text, ix)
            };
            *self.scopes.borrow_mut() = Rc::new(s);
            self.math.borrow_mut().clear();
        }
    }
    /// The macro environment of top-level block `b` and its id (0 = none).
    pub fn macros_for(&self, b: usize) -> (u64, Option<Rc<yamdv_math::macros::Macros>>) {
        self.scopes.borrow().for_block(b)
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
    let h = source_hash(tex);
    if ctx.rich.cache().is_some_and(|c| !c.admit_formula(h)) {
        return None;
    }
    let (env_id, env) = ctx
        .rich
        .cache()
        .map_or((0, None), |c| c.macros_for(ctx.block));
    let key = (h, size.to_bits(), display, env_id);
    if let Some(c) = ctx.rich.cache()
        && let Some(r) = c.math.borrow().get(&key)
    {
        return Some(r.clone());
    }
    let mut env = env.map(|e| (*e).clone()).unwrap_or_default();
    let r = yamdv_math::layout::render_with(
        tex,
        &mut env,
        face.font.as_ref(),
        &face.table,
        size,
        display,
    )
    .map(Rc::new);
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
            // `\tag{…}` (R-MATH-02): right-aligned on the formula's baseline.
            let tag = mb.tag.as_ref().map(|t| Rc::new((**t).clone()));
            let tag_w = tag.as_ref().map_or(0.0, |t| t.width + size);
            let h = (mb.ascent + mb.descent).max(
                tag.as_ref()
                    .map_or(0.0, |t| mb.ascent.max(t.ascent) + mb.descent.max(t.descent)),
            );
            let asc = tag.as_ref().map_or(mb.ascent, |t| mb.ascent.max(t.ascent));
            if mb.width + tag_w <= w {
                let centered = x + (w - mb.width) / 2.0;
                // Centered unless that collides with the tag.
                let fx = centered.min(x + w - tag_w - mb.width);
                if let Some(t) = tag {
                    out.items.push(Item::Math {
                        x: x + w - t.width,
                        y: y + asc,
                        mb: t,
                        color: ctx.pal.text,
                        hscroll: None,
                    });
                }
                out.items.push(Item::Math {
                    x: fx,
                    y: y + asc,
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
                if let Some(t) = tag {
                    // Wide formulas carry the tag at the end of the scrolled content.
                    out.regions[region].content_w += tag_w;
                    out.items.push(Item::Math {
                        x: x + mb.width + size,
                        y: y + asc,
                        mb: t,
                        color: ctx.pal.text,
                        hscroll: Some(region as u16),
                    });
                }
                out.items.push(Item::Math {
                    x,
                    y: y + asc,
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
    let text = ctx
        .strings
        .rich(crate::block::RichMsg::MathError)
        .replace("{}", &e.to_string());
    let shaped = crate::block::shape_text(ctx, &text, msg, Some(w), msg.size * 1.4);
    h += 2.0;
    h += out.para(x, y + h, shaped, None, None);
    h
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
    use super::*;
    use crate::block::{EnStrings, HotKind, Item, Metrics};
    use crate::shaper::MockShaper;
    use crate::viewer::{Content, Env, Viewer};
    use yamdv_core::doc::{Document, ReadOpts};
    use yamdv_core::theme;

    struct Off(RichCache);
    impl RichHost for Off {
        fn math(&self) -> Option<&MathFace> {
            None
        }
        fn render_math(&self) -> bool {
            false
        }
        fn render_mermaid(&self) -> bool {
            false
        }
        fn cache(&self) -> Option<&RichCache> {
            Some(&self.0)
        }
    }

    fn lay(src: &str, rich: &dyn RichHost, w: f32) -> (Vec<Item>, Vec<HotKind>) {
        let mut c = Content::Markdown(Document::new(src.into(), ReadOpts::default()));
        let pal = theme::palette("default", false);
        let mut sh = MockShaper;
        let strings = EnStrings;
        let mut env = Env {
            images: &crate::img::NO_IMAGES,
            rich,
            shaper: &mut sh,
            m: Metrics::default(),
            pal: &pal,
            strings: &strings,
            single_newline_break: false,
        };
        let mut v = Viewer::new(&c, w, Metrics::default());
        v.prepare(&mut c, &mut env, 100_000.0);
        let mut items = Vec::new();
        let mut hot = Vec::new();
        for (_, _, bl) in v.visible(&c, 100_000.0) {
            items.extend(bl.items.iter().cloned());
            hot.extend(bl.hot.iter().map(|h| h.kind.clone()));
        }
        (items, hot)
    }

    fn count(items: &[Item], f: impl Fn(&Item) -> bool) -> usize {
        items.iter().filter(|i| f(i)).count()
    }
    fn is_math(i: &Item) -> bool {
        matches!(i, Item::Math { .. })
    }
    fn is_path(i: &Item) -> bool {
        matches!(i, Item::Path { .. })
    }

    #[test]
    fn inline_and_display_math() {
        let r = MockRich::default();
        let (it, _) = lay("a $x^2$ b and $y$\n\n$$\\frac{a}{b}$$\n", &r, 600.0);
        assert_eq!(count(&it, is_math), 3);
        // Without a math font the source stays (inline code style), no formulas.
        let (it, _) = lay("a $x^2$ b\n", &NO_RICH, 600.0);
        assert_eq!(count(&it, is_math), 0);
        // Cached: a second layout reuses the boxes.
        assert!(r.cache().unwrap().len().0 >= 3);
    }

    #[test]
    fn wide_display_math_scrolls() {
        let r = MockRich::default();
        let long = "x+".repeat(200);
        let (it, _) = lay(&format!("$$\n{long}y\n$$\n"), &r, 300.0);
        assert!(it.iter().any(|i| matches!(
            i,
            Item::Math {
                hscroll: Some(_),
                ..
            }
        )));
        assert!(it.iter().any(|i| matches!(i, Item::HScrollBar(_))));
    }

    #[test]
    fn math_errors_never_crash() {
        let r = MockRich::default();
        let (it, _) = lay(
            "$$\\frac{a$$\n\n$\\left($ and ```math\n\\sqrt{\n```\n",
            &r,
            600.0,
        );
        assert_eq!(count(&it, is_math), 0);
    }

    #[test]
    fn math_fence_is_display_math() {
        let r = MockRich::default();
        let (it, _) = lay("```math\n\\sum_{i=1}^n i\n```\n", &r, 600.0);
        assert_eq!(count(&it, is_math), 1);
        let (it, _) = lay(
            "```math\n\\sum_{i=1}^n i\n```\n",
            &Off(RichCache::default()),
            600.0,
        );
        assert_eq!(count(&it, is_math), 0);
    }

    #[test]
    fn mermaid_blocks() {
        let r = MockRich::default();
        let flow = "```mermaid\nflowchart LR\nA[Start] --> B{Ok?}\nB -->|yes| C\n```\n";
        let (it, _) = lay(flow, &r, 600.0);
        assert!(count(&it, is_path) > 5);
        // Scaled to fit a medium column: every path stays inside the column, no scroll region.
        let (it, _) = lay(flow, &r, 420.0);
        for i in &it {
            if let Item::Path { segs, hscroll, .. } = i {
                assert!(hscroll.is_none());
                for s in segs {
                    if let yamdv_mermaid::Seg::M(x, _) | yamdv_mermaid::Seg::L(x, _) = s {
                        assert!(*x >= -0.5 && *x <= 420.5, "{x}");
                    }
                }
            }
        }
        // BUG-08: a 9-node LR flowchart in a narrow column keeps ≥ 10 px text and scrolls.
        let wide = "```mermaid\nflowchart LR\nA[Alpha] --> B[Bravo] --> C[Charlie] --> D[Delta] \
                    --> E[Echo] --> F[Foxtrot] --> G[Golf] --> H[Hotel] --> I[India]\n```\n";
        let (it, _) = lay(wide, &r, 300.0);
        assert!(it.iter().any(|i| matches!(i, Item::HScrollBar(_))));
        assert!(it.iter().any(|i| matches!(
            i,
            Item::Path {
                hscroll: Some(_),
                ..
            }
        )));
        let seq = "```mermaid\nsequenceDiagram\nA->>B: hi\n```\n";
        assert!(count(&lay(seq, &r, 600.0).0, is_path) > 3);
        // Unsupported type and syntax error: code card, no diagram.
        for src in [
            "```mermaid\npie\n\"a\": 1\n```\n",
            "```mermaid\ngraph TD\nA -->\n```\n",
        ] {
            let (it, _) = lay(src, &r, 600.0);
            assert_eq!(count(&it, is_path), 0);
            assert!(it.iter().any(|i| matches!(i, Item::CodeToolbar { .. })));
        }
        // render_mermaid = false: code block.
        let (it, _) = lay(flow, &Off(RichCache::default()), 600.0);
        assert_eq!(count(&it, is_path), 0);
    }

    #[test]
    fn mermaid_soft_limit_and_render_anyway() {
        let mut src = String::from("```mermaid\ngraph TD\n");
        for i in 0..520 {
            src.push_str(&format!("n{i}\n"));
        }
        src.push_str("```\n");
        let r = MockRich::default();
        let (it, hot) = lay(&src, &r, 600.0);
        assert_eq!(count(&it, is_path), 0);
        let Some(HotKind::RenderAnyway { hash }) = hot
            .iter()
            .find(|h| matches!(h, HotKind::RenderAnyway { .. }))
            .cloned()
        else {
            panic!("{hot:?}")
        };
        r.cache().unwrap().force(hash);
        let (it, _) = lay(&src, &r, 600.0);
        assert!(count(&it, is_path) >= 520);
    }

    #[test]
    fn s_parse_05_formula_cap() {
        let c = RichCache::default();
        assert!((0..MAX_FORMULAS as u64).all(|h| c.admit_formula(h)));
        assert!(c.admit_formula(7));
        assert!(!c.admit_formula(u64::MAX));
    }
}
