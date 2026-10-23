//! Mermaid blocks (R-MMD-01/02/04/05, B.6.12, B.3.6): parse, lay out (cached per source hash and
//! width), scale down to fit, and emit vector paths plus shaped text into the display list.

use std::rc::Rc;
use std::time::{Duration, Instant};

use yamdv_core::doc::text::ParaText;
use yamdv_mermaid::{Diagram, Drawing, Failure, Ink, Measure, Prim, Role, Seg};

use crate::Rect;
use crate::block::{Ctx, Hot, HotKind, Item, Out, RichMsg, base};
use crate::code::is_dark;
use crate::shaper::{Family, ParaSpec, Shaped, Style, TextShaper};

/// Soft limits (placeholder + [Render anyway]) and hard limits (S-PARSE-06).
pub const SOFT_NODES: usize = 500;
/// Soft edge limit.
pub const SOFT_EDGES: usize = 1500;
/// Hard node limit.
pub const HARD_NODES: usize = 2000;
/// Hard edge limit.
pub const HARD_EDGES: usize = 6000;
/// Layout time budget (S-PARSE-03).
pub const BUDGET: Duration = Duration::from_millis(200);
/// Card padding (B.6.12).
const PAD: f32 = 24.0;
/// Diagram text size at scale 1 (layered / sequence labels).
const BASE_TEXT: f32 = 14.0;
/// BUG-08: diagram text never shrinks below this at 100 % zoom.
pub const MIN_TEXT_PX: f32 = 10.0;

/// Scale for a drawing `dr_w` wide at scale `s` to fit `inner_w` — never up, never below the
/// minimum text size (BUG-08); a wider result scrolls horizontally like a code block.
pub fn fit_scale(s: f32, inner_w: f32, dr_w: f32) -> f32 {
    let min = MIN_TEXT_PX / BASE_TEXT;
    if dr_w <= inner_w || dr_w <= 0.0 {
        return s;
    }
    (s * inner_w / dr_w).clamp(min.min(s), s)
}

#[derive(Debug, Clone)]
struct TextSpec {
    s: String,
    size: f32,
    max_w: f32,
    bold: bool,
}

/// A laid-out diagram (cached).
#[derive(Debug)]
pub struct Laid {
    drawing: Drawing,
    texts: Vec<TextSpec>,
    slow: bool,
}

struct Meas<'a> {
    shaper: &'a mut dyn TextShaper,
    texts: Vec<TextSpec>,
}

fn shape(shaper: &mut dyn TextShaper, t: &TextSpec, color: u32) -> Box<dyn Shaped> {
    shaper.shape(&ParaSpec {
        text: &t.s,
        base: Style {
            family: Family::Body,
            size: t.size,
            weight: if t.bold { 600 } else { 400 },
            italic: false,
            color,
            strike: false,
            underline: false,
        },
        spans: vec![],
        width: Some(t.max_w.max(1.0)),
        line_pitch: t.size * 1.4,
        rtl: false,
        tab_spaces: None,
        objects: Vec::new(),
    })
}

impl Measure for Meas<'_> {
    fn text(&mut self, s: &str, size: f32, max_w: f32, bold: bool) -> (u32, f32, f32) {
        let spec = TextSpec {
            s: s.to_string(),
            size,
            max_w,
            bold,
        };
        let sh = shape(self.shaper, &spec, 0);
        self.texts.push(spec);
        let id = u32::try_from(self.texts.len() - 1).unwrap_or(u32::MAX);
        (id, sh.width(), sh.height())
    }
}

/// B.3.6 pie / mindmap series palette.
const SERIES: [u32; 10] = [
    0x60A5FA, 0x34D399, 0xFBBF24, 0xF87171, 0xA78BFA, 0xF472B6, 0x22D3EE, 0xA3E635, 0xFB923C,
    0x94A3B8,
];

/// B.3.6 palette.
fn color(ink: Ink, dark: bool, card: u32) -> u32 {
    let r = match ink {
        Ink::Rgb(c) => return c,
        Ink::Role(r) => r,
    };
    match (r, dark) {
        (Role::NodeFill, false) => 0xEFF6FF,
        (Role::NodeFill, true) => 0x1E3A5F,
        (Role::NodeStroke, false) => 0x93C5FD,
        (Role::NodeStroke, true) => 0x60A5FA,
        (Role::Text, false) => 0x333333,
        (Role::Text, true) => 0xE5E7EB,
        (Role::Edge, _) => 0x94A3B8,
        (Role::LabelBg, _) => card,
        (Role::ClusterFill, false) => 0xF8FAFC,
        (Role::ClusterFill, true) => 0x262B33,
        (Role::ClusterStroke, false) => 0xCBD5E1,
        (Role::ClusterStroke, true) | (Role::Frame, true) => 0x475569,
        (Role::NoteFill, false) => 0xFFF8C5,
        (Role::NoteFill, true) => 0x3B3415,
        (Role::NoteStroke, false) => 0xE3C34D,
        (Role::NoteStroke, true) => 0xA88F2A,
        (Role::Lifeline, false) | (Role::Frame, false) => 0x94A3B8,
        (Role::Lifeline, true) => 0x64748B,
        (Role::Activation, false) => 0xE2E8F0,
        (Role::Activation, true) => 0x334155,
        (Role::GanttBar, false) => 0x93C5FD,
        (Role::GanttBar, true) => 0x3B82F6,
        (Role::GanttDone, false) => 0xCBD5E1,
        (Role::GanttDone, true) => 0x475569,
        (Role::GanttActive, _) => 0x60A5FA,
        (Role::GanttCrit, false) => 0xF87171,
        (Role::GanttCrit, true) => 0xEF4444,
        (Role::GanttMilestone, false) => 0x2563EB,
        (Role::GanttMilestone, true) => 0x93C5FD,
        (Role::Grid, false) => 0xE5E7EB,
        (Role::Grid, true) => 0x374151,
        (Role::Today, false) => 0xEF4444,
        (Role::Today, true) => 0xF87171,
        (Role::Series(i), _) => SERIES
            .get(usize::from(i) % SERIES.len())
            .copied()
            .unwrap_or(0x94A3B8),
    }
}

fn lay(ctx: &mut Ctx<'_>, d: &Diagram, src: &str, inner_w: f32) -> Laid {
    let t0 = Instant::now();
    let now = ctx.rich.now_local();
    let init = yamdv_mermaid::theme::init(src);
    let s0 = init.as_ref().and_then(|i| i.font_scale).unwrap_or(1.0);
    let mut m = Meas {
        shaper: &mut *ctx.shaper,
        texts: Vec::new(),
    };
    let mut dr = d.layout_at(&mut m, s0, now);
    // R-MMD-02: scaled down to fit the column, never up (text is re-laid at the smaller size).
    let mut s = s0;
    for _ in 0..2 {
        if dr.complex || dr.w <= inner_w || dr.w <= 0.0 {
            break;
        }
        let ns = fit_scale(s, inner_w, dr.w);
        if (ns - s).abs() < 1e-4 {
            break;
        }
        s = ns;
        m.texts.clear();
        dr = d.layout_at(&mut m, s, now);
    }
    if let Some(i) = &init {
        i.apply(&mut dr);
    }
    Laid {
        drawing: dr,
        texts: m.texts,
        slow: t0.elapsed() > BUDGET,
    }
}

/// A ```` ```mermaid ```` block; returns its height.
#[allow(clippy::too_many_arguments)]
pub(crate) fn block(
    ctx: &mut Ctx<'_>,
    node: usize,
    info: &str,
    pt: ParaText,
    x: f32,
    y: f32,
    w: f32,
    out: &mut Out,
) -> f32 {
    let hash = crate::rich::source_hash(&pt.text);
    let d = match yamdv_mermaid::parse(&pt.text) {
        Ok(d) => d,
        Err(Failure::Unsupported(_)) => {
            let msg = ctx.strings.rich(RichMsg::Unsupported).to_string();
            let color = ctx.pal.text_muted;
            return with_note(ctx, node, info, pt, (x, y, w), &msg, color, out);
        }
        Err(Failure::Syntax(e)) => {
            let msg = ctx
                .strings
                .rich(RichMsg::MermaidError)
                .replace("{l}", &e.line.to_string())
                .replace("{c}", &e.col.to_string())
                .replace("{m}", &e.msg);
            let color = ctx.pal.error;
            return with_note(ctx, node, info, pt, (x, y, w), &msg, color, out);
        }
    };
    let cache = ctx.rich.cache();
    if cache.is_some_and(|c| !c.admit(hash)) {
        return crate::code::code_card(ctx, node, info, pt, x, y, w, out);
    }
    let forced = cache.is_some_and(|c| c.forced(hash));
    let (nodes, edges) = d.counts();
    let too_large = || {
        ctx.strings
            .rich(RichMsg::TooLarge)
            .replace("{}", &nodes.to_string())
    };
    if nodes > HARD_NODES || edges > HARD_EDGES {
        let msg = too_large();
        return placeholder(ctx, &msg, None, x, y, w, out);
    }
    if (nodes > SOFT_NODES || edges > SOFT_EDGES) && !forced {
        let msg = too_large();
        return placeholder(ctx, &msg, Some(hash), x, y, w, out);
    }
    let inner_w = (w - 2.0 * PAD).max(40.0);
    let key = (hash, inner_w.to_bits());
    let laid = match ctx
        .rich
        .cache()
        .and_then(|c| c.diagrams.borrow().get(&key).cloned())
    {
        Some(l) => l,
        None => {
            let l = Rc::new(lay(ctx, &d, &pt.text, inner_w));
            if let Some(c) = ctx.rich.cache() {
                let mut m = c.diagrams.borrow_mut();
                if m.len() >= 512 {
                    m.clear();
                }
                m.insert(key, l.clone());
            }
            l
        }
    };
    if laid.drawing.complex || (laid.slow && !forced) {
        let msg = ctx.strings.rich(RichMsg::TooComplex).to_string();
        let hot = (!laid.drawing.complex).then_some(hash);
        return placeholder(ctx, &msg, hot, x, y, w, out);
    }
    emit(ctx, &laid, (node, x, y, w), out)
}

fn emit(
    ctx: &mut Ctx<'_>,
    laid: &Laid,
    (node, x, y, w): (usize, f32, f32, f32),
    out: &mut Out,
) -> f32 {
    let dr = &laid.drawing;
    let h = dr.h + 2.0 * PAD;
    let card = Rect::new(x, y, w, h);
    let (bg, border) = (ctx.pal.code_bg, ctx.pal.code_border);
    let dark = is_dark(ctx.pal.bg);
    out.items.push(Item::Fill {
        rect: card,
        color: bg,
        radius: 8.0,
    });
    // BUG-08: wider than the card (minimum text size) → clipped, horizontal scroll region.
    let wide = dr.w > w - 2.0 * PAD + 0.5;
    let region = if wide {
        let view = Rect::new(x, y, w, h);
        out.regions.push(crate::block::HRegion {
            view,
            content_w: dr.w + 2.0 * PAD,
            track: Rect::new(x + 8.0, y + h - 10.0, (w - 16.0).max(1.0), 6.0),
        });
        out.items.push(Item::PushClip(view));
        Some(u16::try_from(out.regions.len() - 1).unwrap_or(u16::MAX))
    } else {
        None
    };
    let ox = if wide {
        x + PAD
    } else {
        x + ((w - dr.w) / 2.0).max(0.0)
    };
    let oy = y + PAD;
    let col = |i: Ink| color(i, dark, bg);
    for p in &dr.prims {
        match p {
            Prim::Path(p) => {
                let segs = p
                    .segs
                    .iter()
                    .map(|s| match *s {
                        Seg::M(a, b) => Seg::M(a + ox, b + oy),
                        Seg::L(a, b) => Seg::L(a + ox, b + oy),
                        Seg::C(a, b, c, d, e, f) => {
                            Seg::C(a + ox, b + oy, c + ox, d + oy, e + ox, f + oy)
                        }
                        Seg::Z => Seg::Z,
                    })
                    .collect();
                out.items.push(Item::Path {
                    segs,
                    fill: p.fill.map(col),
                    stroke: p.stroke.map(col),
                    width: p.width,
                    dash: p.dash,
                    hscroll: region,
                });
            }
            Prim::Text(t) => {
                let Some(spec) = usize::try_from(t.id).ok().and_then(|i| laid.texts.get(i)) else {
                    continue;
                };
                let sh = shape(&mut *ctx.shaper, spec, col(t.color));
                out.para(ox + t.x, oy + t.y, sh, None, region);
            }
        }
    }
    if let Some(r) = region {
        out.items.push(Item::PopClip);
        out.items.push(Item::HScrollBar(usize::from(r)));
    }
    out.items.push(Item::Stroke {
        rect: card,
        color: border,
        radius: 8.0,
    });
    out.hot.push(crate::block::Hot {
        rect: card,
        kind: crate::block::HotKind::Diagram { node },
    });
    h
}

/// The source as a code card plus a message line below it (R-MMD-01 badge, R-MMD-04 error).
#[allow(clippy::too_many_arguments)]
fn with_note(
    ctx: &mut Ctx<'_>,
    node: usize,
    info: &str,
    pt: ParaText,
    (x, y, w): (f32, f32, f32),
    msg: &str,
    color: u32,
    out: &mut Out,
) -> f32 {
    let mut h = crate::code::code_card(ctx, node, info, pt, x, y, w, out);
    let m = ctx.m;
    let st = Style {
        size: 12.0 * m.font_size / 16.0,
        ..base(&m, color)
    };
    let sh = crate::block::shape_text(ctx, msg, st, Some(w), st.size * 1.4);
    h += 4.0;
    h += out.para(x, y + h, sh, None, None);
    h
}

/// A card with a centered message and an optional [Render anyway] button.
fn placeholder(
    ctx: &mut Ctx<'_>,
    msg: &str,
    hot: Option<u64>,
    x: f32,
    y: f32,
    w: f32,
    out: &mut Out,
) -> f32 {
    let m = ctx.m;
    let st = base(&m, ctx.pal.text_muted);
    let sh = crate::block::shape_text(ctx, msg, st, Some((w - 2.0 * PAD).max(10.0)), m.pitch());
    let th = sh.height();
    let tw = sh.width();
    let btn_h = if hot.is_some() { 28.0 + 12.0 } else { 0.0 };
    let h = PAD + th + btn_h + PAD;
    let card = Rect::new(x, y, w, h);
    out.items.push(Item::Fill {
        rect: card,
        color: ctx.pal.code_bg,
        radius: 8.0,
    });
    out.para(x + (w - tw) / 2.0, y + PAD, sh, None, None);
    if let Some(hash) = hot {
        let label = ctx.strings.rich(RichMsg::RenderAnyway).to_string();
        let bs = Style {
            size: 13.0 * m.font_size / 16.0,
            ..base(&m, ctx.pal.accent)
        };
        let lb = crate::block::shape_text(ctx, &label, bs, None, 28.0);
        let bw = lb.width() + 24.0;
        let r = Rect::new(x + (w - bw) / 2.0, y + PAD + th + 12.0, bw, 28.0);
        out.items.push(Item::Stroke {
            rect: r,
            color: ctx.pal.accent,
            radius: 4.0,
        });
        out.para(r.x + 12.0, r.y, lb, None, None);
        out.hot.push(Hot {
            rect: r,
            kind: HotKind::RenderAnyway { hash },
        });
    }
    out.items.push(Item::Stroke {
        rect: card,
        color: ctx.pal.code_border,
        radius: 8.0,
    });
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_scale_bug08() {
        // Fits: unchanged.
        assert_eq!(fit_scale(1.0, 600.0, 500.0), 1.0);
        // Moderately wide: scaled to fit.
        assert!((fit_scale(1.0, 600.0, 750.0) - 0.8).abs() < 1e-6);
        // Very wide: clamped so 14 px text stays at 10 px.
        let s = fit_scale(1.0, 300.0, 1800.0);
        assert!((s * BASE_TEXT - MIN_TEXT_PX).abs() < 1e-4);
        // Never up, and never below a scale already under the minimum.
        assert_eq!(fit_scale(0.5, 300.0, 1800.0), 0.5);
    }
}
