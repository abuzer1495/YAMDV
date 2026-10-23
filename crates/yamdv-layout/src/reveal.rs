//! F2 reveal paragraph layout (prompt 016 §3 item 1, ADR-0020 §1): the active block laid out as one
//! rendered paragraph with revealed markers in `marker` color at the surrounding size.

use yamdv_core::doc::reveal::RevealPara;

use crate::block::{self, BlockLayout, Ctx, Item, LaidPara};
use crate::shaper::{Family, ParaSpec, Span, Style, is_rtl};

/// What the viewer needs to lay out the active block in reveal mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reveal {
    /// Layout string, map (block-relative) and marker ranges.
    pub para: RevealPara,
    /// Heading level (0 = none).
    pub heading: u8,
    /// Block quote / callout (quote text color and bar).
    pub quote: bool,
}

/// Lays out `r` for the block starting at source offset `start`.
pub(crate) fn layout(ctx: &mut Ctx<'_>, r: &Reveal, start: u32, w: f32) -> BlockLayout {
    let m = ctx.m;
    let mut pt = r.para.text.clone();
    for s in &mut pt.segs {
        s.src = s.src.start + start..s.src.end + start;
    }
    let (color, size, weight) = match r.heading {
        0 if r.quote => (ctx.pal.quote_text, m.font_size, 400),
        0 => (ctx.pal.text, m.font_size, 400),
        6 => (ctx.pal.text_muted, m.heading_size(6), 600),
        l => (ctx.pal.heading, m.heading_size(l), 600),
    };
    let base = Style {
        size,
        weight,
        ..block::base(&m, color)
    };
    let mut spans = block::spans(&pt.segs, &base, &m, ctx.pal);
    for mr in &r.para.marks {
        spans.push(Span {
            range: mr.clone(),
            style: Style {
                color: ctx.pal.marker,
                family: Family::Body,
                weight: 400,
                italic: false,
                strike: false,
                underline: false,
                ..base
            },
        });
    }
    let pitch = if r.heading > 0 {
        size * 1.25
    } else {
        m.pitch()
    };
    let indent = if r.quote { 16.0 } else { 0.0 };
    let shaped = ctx.shaper.shape(&ParaSpec {
        text: &pt.text,
        base,
        spans,
        width: Some((w - indent).max(1.0)),
        line_pitch: pitch,
        rtl: is_rtl(&pt.text),
        tab_spaces: Some(4),
        objects: Vec::new(),
    });
    let height = shaped.height().max(pitch);
    let mut items = Vec::new();
    if r.quote {
        items.push(Item::Fill {
            rect: crate::geom::Rect::new(0.0, 0.0, 4.0, height),
            color: ctx.pal.quote_bar,
            radius: 0.0,
        });
    }
    items.push(Item::Para(0));
    BlockLayout {
        height,
        items,
        paras: vec![LaidPara {
            x: indent,
            y: 0.0,
            shaped,
            map: Some(pt),
            hscroll: None,
        }],
        regions: Vec::new(),
        hot: Vec::new(),
    }
}
