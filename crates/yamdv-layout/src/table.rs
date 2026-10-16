//! GFM tables (R-REN-07, UI_SPEC §B.6.8, prompt 003 §4.3).
//!
//! Auto layout: per column `min` = widest unbreakable token (≥ 48 px incl. padding) and `max` =
//! unwrapped content; columns get `max` when everything fits, otherwise `min` plus the remaining
//! width shared in proportion to `max − min` (CSS auto layout). When even the minimums do not fit
//! the widths are scaled down; the shaper's emergency breaking keeps text inside its column.

#![allow(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::too_many_arguments
)]

use yamdv_core::doc::detail::{Detail, NodeKind};
use yamdv_core::doc::text::node_text;

use crate::block::{Ctx, Item, Out, base, card, leaf_aligned, source_text, spans};
use crate::geom::Rect;
use crate::shaper::{ParaSpec, is_rtl};

/// S-PARSE-02 limits: more columns or cells render the source in a "table too large" card.
pub const MAX_COLS: usize = 1_000;
/// See [`MAX_COLS`].
pub const MAX_CELLS: usize = 100_000;
/// Minimum column width including padding (B.6.8).
pub const MIN_COL: f32 = 48.0;
/// Cell padding (each side).
pub const PAD: f32 = 8.0;
/// Rows sampled for column measurement (larger tables wrap in later rows).
pub const MEASURE_ROWS: usize = 1_000;

/// CSS-like auto distribution of `avail` among columns with `(min, max)` widths.
pub fn distribute(cols: &[(f32, f32)], avail: f32) -> Vec<f32> {
    let sum_max: f32 = cols.iter().map(|c| c.1).sum();
    let sum_min: f32 = cols.iter().map(|c| c.0).sum();
    if sum_max <= avail {
        return cols.iter().map(|c| c.1).collect();
    }
    if sum_min >= avail {
        // Water-filling: narrow columns keep their minimum, the widest share what is left (their
        // long tokens break anywhere).
        let mut order: Vec<usize> = (0..cols.len()).collect();
        order.sort_by(|&a, &b| cols[a].0.total_cmp(&cols[b].0));
        let mut out = vec![0.0; cols.len()];
        let mut left = avail;
        for (k, &c) in order.iter().enumerate() {
            let share = left / (cols.len() - k) as f32;
            out[c] = cols[c].0.min(share).max(1.0);
            left -= out[c];
        }
        return out;
    }
    let extra = avail - sum_min;
    let span = sum_max - sum_min;
    cols.iter()
        .map(|c| c.0 + extra * (c.1 - c.0) / span.max(f32::EPSILON))
        .collect()
}

fn mix(a: u32, b: u32) -> u32 {
    let ch = |s: u32| (((a >> s) & 0xff) + ((b >> s) & 0xff)) / 2;
    (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

/// Lays out table node `i`; returns its height.
pub(crate) fn table(
    ctx: &mut Ctx<'_>,
    src: &str,
    d: &Detail,
    i: usize,
    aligns: &[u8],
    x: f32,
    y: f32,
    w: f32,
    out: &mut Out,
) -> f32 {
    let m = ctx.m;
    let rows: Vec<(bool, Vec<usize>)> = d
        .children(i)
        .filter_map(|r| match d.nodes[r].kind {
            NodeKind::TableRow { head } => Some((head, d.children(r).collect())),
            _ => None,
        })
        .collect();
    let ncols = aligns
        .len()
        .max(rows.iter().map(|r| r.1.len()).max().unwrap_or(0));
    let cells: usize = rows.iter().map(|r| r.1.len()).sum();
    if ncols == 0 || ncols > MAX_COLS || cells > MAX_CELLS {
        let pt = source_text(src, &d.nodes[i].range);
        return card(ctx, pt, Some("table too large"), x, y, w, out);
    }
    // Measure.
    let mut cols = vec![(MIN_COL, MIN_COL); ncols];
    for (head, cs) in rows.iter().take(MEASURE_ROWS) {
        let mut style = base(&m, ctx.pal.text);
        if *head {
            style.weight = 600;
        }
        for (c, &cell) in cs.iter().enumerate() {
            let pt = node_text(src, d, cell, ctx.single_newline_break);
            if pt.text.is_empty() {
                continue;
            }
            let sp = spans(&pt.segs, &style, &m, ctx.pal);
            let full = ctx.shaper.shape(&ParaSpec {
                text: &pt.text,
                base: style,
                spans: sp,
                width: None,
                line_pitch: m.pitch(),
                rtl: is_rtl(&pt.text),
                tab_spaces: None,
            });
            let token = pt
                .text
                .split_whitespace()
                .max_by_key(|t| t.chars().count())
                .unwrap_or("");
            let tok = ctx.shaper.shape(&ParaSpec {
                text: token,
                base: style,
                spans: vec![],
                width: None,
                line_pitch: m.pitch(),
                rtl: false,
                tab_spaces: None,
            });
            let e = &mut cols[c];
            e.0 = e.0.max(tok.width() + 2.0 * PAD);
            e.1 = e.1.max(full.width() + 2.0 * PAD);
        }
    }
    for c in &mut cols {
        c.1 = c.1.max(c.0);
    }
    let widths = distribute(&cols, w.max(1.0));
    let total_w: f32 = widths.iter().sum();
    let radius = if m.rounded { 6.0 } else { 0.0 };
    let zebra = if ctx.pal.zebra_bg != 0 {
        ctx.pal.zebra_bg
    } else {
        mix(ctx.pal.bg, ctx.pal.table_header_bg)
    };
    let bg_at = out.items.len();
    let mut bgs: Vec<Item> = Vec::new();
    let mut lines: Vec<Item> = Vec::new();
    let mut yy = y;
    let mut body = 0usize;
    for (r, (head, cs)) in rows.iter().enumerate() {
        let mut style = base(&m, ctx.pal.text);
        if *head {
            style.weight = 600;
        }
        let mut cx = x;
        let mut rh = m.pitch();
        for (c, &cell) in cs.iter().enumerate().take(ncols) {
            let cw = widths[c];
            let pt = node_text(src, d, cell, ctx.single_newline_break);
            let align = aligns.get(c).copied().unwrap_or(0);
            let h = leaf_aligned(
                ctx,
                pt,
                style,
                m.pitch(),
                cx + PAD,
                yy + PAD,
                (cw - 2.0 * PAD).max(1.0),
                align,
                out,
            );
            rh = rh.max(h);
            cx += cw;
        }
        let row_h = rh + 2.0 * PAD;
        if *head {
            bgs.push(Item::Fill {
                rect: Rect::new(x, yy, total_w, row_h),
                color: ctx.pal.table_header_bg,
                radius,
            });
            if radius > 0.0 && row_h > radius {
                bgs.push(Item::Fill {
                    rect: Rect::new(x, yy + radius, total_w, row_h - radius),
                    color: ctx.pal.table_header_bg,
                    radius: 0.0,
                });
            }
        } else {
            body += 1;
            if m.table_zebra && body.is_multiple_of(2) {
                bgs.push(Item::Fill {
                    rect: Rect::new(x, yy, total_w, row_h),
                    color: zebra,
                    radius: 0.0,
                });
            }
        }
        if r > 0 {
            lines.push(Item::Fill {
                rect: Rect::new(x, yy, total_w, 1.0),
                color: ctx.pal.table_border,
                radius: 0.0,
            });
        }
        yy += row_h;
    }
    let h = yy - y;
    let mut cx = x;
    for cw in widths.iter().take(ncols.saturating_sub(1)) {
        cx += cw;
        lines.push(Item::Fill {
            rect: Rect::new(cx, y, 1.0, h),
            color: ctx.pal.table_border,
            radius: 0.0,
        });
    }
    // Backgrounds go under the cell text; grid lines and the outer border on top.
    for (k, b) in bgs.into_iter().enumerate() {
        out.items.insert(bg_at + k, b);
    }
    // Inserting non-Para items does not change para indices (they index `out.paras`).
    out.items.extend(lines);
    out.items.push(Item::Stroke {
        rect: Rect::new(x, y, total_w, h),
        color: ctx.pal.table_border,
        radius,
    });
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::block::{EnStrings, Metrics, layout_block};
    use crate::shaper::MockShaper;
    use yamdv_core::doc::{Document, ReadOpts};

    fn lay(src: &str, w: f32, m: Metrics) -> crate::block::BlockLayout {
        let doc = Document::new(src.into(), ReadOpts::default());
        let d = doc.detail_uncached(0);
        let pal = yamdv_core::theme::palette("default", false);
        let mut sh = MockShaper;
        let mut ctx = Ctx {
            shaper: &mut sh,
            m,
            pal: &pal,
            strings: &EnStrings,
            single_newline_break: false,
            block: 0,
            code_opts: &|_, _| crate::code::CodeOpts::default(),
        };
        layout_block(&mut ctx, src, &d, w)
    }

    #[test]
    fn r_ren_07_cells_stay_inside_columns() {
        let long = "x".repeat(400);
        let src = format!(
            "| a | b | c |\n|:-|:-:|-:|\n| **bold** `code` | {long} | 1 |\n| r | s | t |\n"
        );
        let bl = lay(&src, 600.0, Metrics::default());
        assert!(bl.paras.len() >= 9);
        for p in bl.paras.iter().filter(|p| p.shaped.width() < 1000.0) {
            assert!(
                p.x >= 0.0 && p.x + p.shaped.width() <= 600.0 + 0.5,
                "{} {}",
                p.x,
                p.shaped.width()
            );
        }
        let header = bl
            .items
            .iter()
            .any(|i| matches!(i, Item::Fill { color, .. } if *color == 0xf5f5f5));
        assert!(header, "header background");
        // Right-aligned column: the `1` ends at the column's right padding.
        let max_x = bl
            .paras
            .iter()
            .map(|p| p.x + p.shaped.width())
            .fold(0.0f32, f32::max);
        assert!(max_x <= 600.0);
    }

    #[test]
    fn zebra_only_when_enabled() {
        let src = "|a|\n|-|\n|1|\n|2|\n|3|\n";
        let count = |m: Metrics| {
            lay(src, 400.0, m)
                .items
                .iter()
                .filter(|i| matches!(i, Item::Fill { rect, radius, .. } if *radius == 0.0 && rect.h > 2.0))
                .count()
        };
        let off = count(Metrics::default());
        let on = count(Metrics {
            table_zebra: true,
            ..Metrics::default()
        });
        assert_eq!(on, off + 1, "one even body row");
    }

    #[test]
    fn s_parse_02_too_many_columns_is_a_card() {
        let n = MAX_COLS + 1;
        let src = format!("|{}\n|{}\n", "a|".repeat(n), "-|".repeat(n));
        let bl = lay(&src, 600.0, Metrics::default());
        assert!(bl.paras.len() <= 2, "source card, not cells");
    }

    #[test]
    fn fits_uses_max() {
        assert_eq!(
            distribute(&[(48.0, 60.0), (48.0, 100.0)], 500.0),
            vec![60.0, 100.0]
        );
    }

    #[test]
    fn shares_extra_by_slack() {
        let w = distribute(&[(50.0, 150.0), (50.0, 350.0)], 300.0);
        assert!(
            (w[0] - 100.0).abs() < 0.01 && (w[1] - 200.0).abs() < 0.01,
            "{w:?}"
        );
        assert!((w.iter().sum::<f32>() - 300.0).abs() < 0.01);
    }

    #[test]
    fn scales_when_minimums_overflow() {
        let w = distribute(&[(200.0, 300.0), (200.0, 300.0)], 200.0);
        assert!((w.iter().sum::<f32>() - 200.0).abs() < 0.01, "{w:?}");
        let w = distribute(&[(48.0, 48.0), (3000.0, 3000.0)], 600.0);
        assert_eq!(w, vec![48.0, 552.0]);
    }
}
