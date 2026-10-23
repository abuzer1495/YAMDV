//! F2 table cell editing layout (R-TBL-01, prompt 016 §3 item 8): the active table stays a
//! rendered grid; every cell is a reveal paragraph mapped to its source, so the caret goes into
//! cells (empty cells keep the row height) and the caret cell shows its inline markers.

#![allow(clippy::cast_precision_loss)]

use yamdv_core::doc::reveal::RevealPara;

use crate::block::{self, BlockLayout, Ctx, Hot, HotKind, Item, LaidPara};
use crate::geom::Rect;
use crate::shaper::{Family, ParaSpec, Span, Style, is_rtl};
use crate::table::{MIN_COL, PAD, distribute};

/// The active table: cells (block-relative maps) and column alignments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableReveal {
    /// Rows of cells; row 0 is the header.
    pub cells: Vec<Vec<RevealPara>>,
    /// Alignment per column (0 none, 1 left, 2 center, 3 right).
    pub aligns: Vec<u8>,
}

fn cell_spans(ctx: &Ctx<'_>, p: &RevealPara, base: &Style) -> Vec<Span> {
    let mut spans = block::spans(&p.text.segs, base, &ctx.m, ctx.pal);
    for mr in &p.marks {
        spans.push(Span {
            range: mr.clone(),
            style: Style {
                color: ctx.pal.marker,
                family: Family::Body,
                weight: 400,
                italic: false,
                strike: false,
                underline: false,
                ..*base
            },
        });
    }
    spans
}

/// Lays out the active table of the block starting at source offset `start`.
pub(crate) fn layout(ctx: &mut Ctx<'_>, t: &TableReveal, start: u32, w: f32) -> BlockLayout {
    let m = ctx.m;
    let ncols = t.aligns.len().max(1);
    let style = |head: bool| Style {
        weight: if head { 600 } else { 400 },
        ..block::base(&m, ctx.pal.text)
    };
    let mut cols = vec![(MIN_COL, MIN_COL); ncols];
    for (r, row) in t.cells.iter().enumerate() {
        let st = style(r == 0);
        for (c, p) in row.iter().enumerate().take(ncols) {
            if p.text.text.is_empty() {
                continue;
            }
            let full = ctx.shaper.shape(&ParaSpec {
                text: &p.text.text,
                base: st,
                spans: cell_spans(ctx, p, &st),
                width: None,
                line_pitch: m.pitch(),
                rtl: is_rtl(&p.text.text),
                tab_spaces: None,
                objects: Vec::new(),
            });
            let tok = p
                .text
                .text
                .split_whitespace()
                .max_by_key(|s| s.chars().count())
                .unwrap_or("");
            let tw = ctx
                .shaper
                .shape(&ParaSpec {
                    text: tok,
                    base: st,
                    spans: Vec::new(),
                    width: None,
                    line_pitch: m.pitch(),
                    rtl: false,
                    tab_spaces: None,
                    objects: Vec::new(),
                })
                .width();
            if let Some(e) = cols.get_mut(c) {
                e.0 = e.0.max(tw + 2.0 * PAD);
                e.1 = e.1.max(full.width() + 2.0 * PAD).max(e.0);
            }
        }
    }
    let widths = distribute(&cols, w.max(1.0));
    let total_w: f32 = widths.iter().sum();
    let mut items = Vec::new();
    let mut lines = Vec::new();
    let mut paras = Vec::new();
    let mut hot = Vec::new();
    let mut y = 0.0f32;
    for (r, row) in t.cells.iter().enumerate() {
        let st = style(r == 0);
        let mut x = 0.0f32;
        let mut rh = m.pitch();
        let mut row_cells = Vec::new();
        for c in 0..ncols {
            let cw = widths.get(c).copied().unwrap_or(MIN_COL);
            let iw = (cw - 2.0 * PAD).max(1.0);
            if let Some(p) = row.get(c) {
                let shaped = ctx.shaper.shape(&ParaSpec {
                    text: &p.text.text,
                    base: st,
                    spans: cell_spans(ctx, p, &st),
                    width: Some(iw),
                    line_pitch: m.pitch(),
                    rtl: is_rtl(&p.text.text),
                    tab_spaces: None,
                    objects: Vec::new(),
                });
                let sw = shaped.width();
                let px = match t.aligns.get(c) {
                    Some(2) => x + PAD + ((iw - sw) / 2.0).max(0.0),
                    Some(3) => x + PAD + (iw - sw).max(0.0),
                    _ => x + PAD,
                };
                rh = rh.max(shaped.height());
                let mut pt = p.text.clone();
                for s in &mut pt.segs {
                    s.src = s.src.start + start..s.src.end + start;
                }
                row_cells.push(paras.len());
                paras.push(LaidPara {
                    x: px,
                    y: y + PAD,
                    shaped,
                    map: Some(pt),
                    hscroll: None,
                });
            }
            hot.push(Hot {
                rect: Rect::new(x, y, cw, 0.0),
                kind: HotKind::Cell {
                    row: u32::try_from(r).unwrap_or(0),
                    col: u32::try_from(c).unwrap_or(0),
                },
            });
            x += cw;
        }
        let row_h = rh + 2.0 * PAD;
        for h in hot.iter_mut().rev().take(ncols) {
            h.rect.h = row_h;
        }
        if r == 0 {
            items.push(Item::Fill {
                rect: Rect::new(0.0, y, total_w, row_h),
                color: ctx.pal.table_header_bg,
                radius: 0.0,
            });
        } else {
            lines.push(Item::Fill {
                rect: Rect::new(0.0, y, total_w, 1.0),
                color: ctx.pal.table_border,
                radius: 0.0,
            });
        }
        y += row_h;
    }
    let mut cx = 0.0;
    for cw in widths.iter().take(ncols.saturating_sub(1)) {
        cx += cw;
        lines.push(Item::Fill {
            rect: Rect::new(cx, 0.0, 1.0, y),
            color: ctx.pal.table_border,
            radius: 0.0,
        });
    }
    items.extend((0..paras.len()).map(Item::Para));
    items.extend(lines);
    items.push(Item::Stroke {
        rect: Rect::new(0.0, 0.0, total_w, y),
        color: ctx.pal.table_border,
        radius: if m.rounded { 6.0 } else { 0.0 },
    });
    hot.push(Hot {
        rect: Rect::new(0.0, 0.0, total_w, y),
        kind: HotKind::Table {
            node: 0,
            rows: u32::try_from(t.cells.len()).unwrap_or(0),
            cols: u32::try_from(ncols).unwrap_or(0),
        },
    });
    BlockLayout {
        height: y,
        items,
        paras,
        regions: Vec::new(),
        hot,
    }
}
