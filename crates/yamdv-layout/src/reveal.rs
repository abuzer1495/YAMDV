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
    /// Sole image of the paragraph, laid out under its source line.
    pub image: Option<yamdv_core::doc::detail::ImageRef>,
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
    let mut height = shaped.height().max(pitch);
    let mut items = Vec::new();
    if r.quote {
        items.push(Item::Fill {
            rect: crate::geom::Rect::new(0.0, 0.0, 4.0, height),
            color: ctx.pal.quote_bar,
            radius: 0.0,
        });
    }
    items.push(Item::Para(0));
    let mut out = empty_out();
    out.items = items;
    out.paras.push(LaidPara {
        x: indent,
        y: 0.0,
        shaped,
        map: Some(pt),
        hscroll: None,
    });
    if let Some(img) = &r.image {
        height += 8.0;
        height += crate::img::block(ctx, img, 0.0, height, w, &mut out);
    }
    BlockLayout {
        height,
        items: out.items,
        paras: out.paras,
        regions: out.regions,
        hot: out.hot,
    }
}

fn empty_out() -> crate::block::Out {
    crate::block::Out {
        items: Vec::new(),
        paras: Vec::new(),
        regions: Vec::new(),
        hot: Vec::new(),
        align: 0,
        cross_details: false,
    }
}

/// Live preview kind under a block source (prompt 016 §3 item 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewKind {
    /// `$$ … $$` or ```` ```math ````.
    Math,
    /// ```` ```mermaid ````.
    Mermaid,
}

/// A live preview: what to render under the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    /// Kind.
    pub kind: PreviewKind,
    /// TeX or Mermaid source (last good for Mermaid).
    pub text: String,
}

/// Lays out the F2 active block: the inline reveal paragraph, or the block-level source in
/// place (code: monospace + live highlight, fence lines in `marker`; math / Mermaid: plus the
/// live preview card below).
pub(crate) fn source_block(
    ctx: &mut Ctx<'_>,
    sb: &crate::viewer::SourceBlock,
    w: f32,
) -> BlockLayout {
    if let Some(t) = &sb.table {
        return crate::table_edit::layout(ctx, t, sb.start, w);
    }
    if let Some(r) = &sb.reveal {
        return layout(ctx, r, sb.start, w);
    }
    let fam = if sb.code_font {
        Family::Code
    } else {
        Family::Body
    };
    let spans = if sb.code_font {
        fence_spans(ctx, &sb.text)
    } else {
        Vec::new()
    };
    let bg = ctx.pal.code_bg;
    let mut bl =
        crate::viewer::verbatim_spans(ctx, &sb.text, sb.start, Some(w), fam, Some(bg), spans);
    if let Some(p) = &sb.preview {
        preview(ctx, &mut bl, p, sb.start, w);
    }
    bl
}

/// Highlight + marker spans of a fenced code block's source (UTF-16 ranges).
fn fence_spans(ctx: &Ctx<'_>, text: &str) -> Vec<Span> {
    let mut out = Vec::new();
    let first = text.split('\n').next().unwrap_or("");
    let t = first.trim_start();
    let fence = if t.starts_with("```") {
        '`'
    } else if t.starts_with("~~~") {
        '~'
    } else if t.starts_with("$$") {
        '$'
    } else {
        return out;
    };
    let dark = crate::code::is_dark(ctx.pal.bg);
    let info = t.trim_start_matches(fence).trim();
    let lang = if fence == '$' {
        None
    } else {
        yamdv_highlight::lang_for(info)
    };
    let base = Style {
        family: Family::Code,
        size: ctx.m.code_size(),
        weight: 400,
        italic: false,
        color: ctx.pal.text,
        strike: false,
        underline: false,
    };
    let marker = Style {
        color: ctx.pal.marker,
        ..base
    };
    let lines: Vec<&str> = text.split('\n').collect();
    let n = lines.len();
    let mut st = yamdv_highlight::State::NORMAL;
    let mut u = 0u32;
    for (k, l) in lines.iter().enumerate() {
        let l16 = u32::try_from(l.encode_utf16().count()).unwrap_or(0);
        let lt = l.trim();
        let closing = k > 0
            && k + 1 >= n.saturating_sub(1)
            && !lt.is_empty()
            && lt.chars().all(|c| c == fence);
        if k == 0 || closing {
            if l16 > 0 {
                out.push(Span {
                    range: u..u + l16,
                    style: marker,
                });
            }
        } else if let Some(lang) = lang {
            let body = l.strip_suffix('\r').unwrap_or(l);
            let (toks, next) = yamdv_highlight::highlight_line(lang, st, body);
            st = next;
            for (r, tok) in toks {
                let (color, italic, bold) = crate::code::syntax_style(tok, dark);
                let a =
                    u + u32::try_from(body.get(..r.start).map_or(0, |s| s.encode_utf16().count()))
                        .unwrap_or(0);
                let b =
                    u + u32::try_from(body.get(..r.end).map_or(0, |s| s.encode_utf16().count()))
                        .unwrap_or(0);
                if b > a {
                    out.push(Span {
                        range: a..b,
                        style: Style {
                            color,
                            italic,
                            weight: if bold { 600 } else { 400 },
                            ..base
                        },
                    });
                }
            }
        }
        u += l16 + 1;
    }
    out
}

/// Appends the live preview card under the source.
fn preview(ctx: &mut Ctx<'_>, bl: &mut BlockLayout, p: &Preview, start: u32, w: f32) {
    const GAP: f32 = 8.0;
    const PAD: f32 = 8.0;
    let y0 = bl.height + GAP;
    let mut out = empty_out();
    let pt = crate::viewer::verbatim(&p.text, start);
    let h = match p.kind {
        PreviewKind::Math => crate::rich::display_math(
            ctx,
            &p.text,
            pt,
            PAD,
            y0 + PAD,
            (w - 2.0 * PAD).max(1.0),
            &mut out,
        ),
        PreviewKind::Mermaid => crate::diagram::block(
            ctx,
            usize::MAX - 1,
            "mermaid",
            pt,
            PAD,
            y0 + PAD,
            (w - 2.0 * PAD).max(1.0),
            &mut out,
        ),
    };
    bl.items.push(Item::Fill {
        rect: crate::geom::Rect::new(0.0, y0, w, h + 2.0 * PAD),
        color: ctx.pal.border_subtle,
        radius: 4.0,
    });
    bl.items.push(Item::Fill {
        rect: crate::geom::Rect::new(1.0, y0 + 1.0, w - 2.0, h + 2.0 * PAD - 2.0),
        color: ctx.pal.bg,
        radius: 4.0,
    });
    let base = bl.paras.len();
    for it in out.items {
        bl.items.push(match it {
            Item::Para(k) => Item::Para(k + base),
            other => other,
        });
    }
    bl.paras.extend(out.paras);
    bl.regions.extend(out.regions);
    bl.hot.extend(out.hot);
    bl.height = y0 + h + 2.0 * PAD;
}
