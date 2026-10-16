//! Code blocks (R-CODE-01…06, B.6.7, B.3.4): syntax colors, line numbers, wrap, diff rows,
//! header toolbar geometry, horizontal scroll region, chunking of very long blocks.
// Line/chunk indices are produced by `lines_of` and the chunk loop and are in bounds by
// construction; UTF-16 counts fit u32 (documents ≤ 256 MiB, ADR-0007).
#![allow(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use yamdv_core::doc::text::ParaText;
use yamdv_highlight::{Lang, Tok, highlight_line, lang_for};

use crate::block::{
    CODE_HEADER, CODE_PAD_BOTTOM, CODE_PAD_TOP, CODE_PAD_X, Ctx, HRegion, Hot, HotKind, Item, Out,
};
use crate::geom::Rect;
use crate::shaper::{Family, ParaSpec, Span, Style};

/// Per-block code options (session state; defaults from `reading.code_line_numbers` and
/// `reading.code_wrap`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodeOpts {
    /// Line numbers shown.
    pub line_numbers: bool,
    /// Soft wrap.
    pub wrap: bool,
    /// Fold state toggled away from the node's default (callouts, details; session state).
    pub fold_toggled: bool,
    /// `reading.front_matter` (see [`crate::front::mode`]); the same for every node.
    pub front_matter: u8,
}

/// Lines per shaped chunk; blocks longer than this are split so no single text layout grows
/// without bound (R-CODE-06) and off-screen chunks are culled when drawing.
pub const CHUNK_LINES: usize = 500;

/// Toolbar button size (B.6.7).
pub const BUTTON: f32 = 24.0;

/// Syntax color, italic, bold for a token class (B.3.4).
pub fn syntax_style(t: Tok, dark: bool) -> (u32, bool, bool) {
    let pick = |l: u32, d: u32| if dark { d } else { l };
    match t {
        Tok::Text | Tok::Operator | Tok::Punct => (pick(0x24292e, 0xc9d1d9), false, false),
        Tok::Comment => (pick(0x6a737d, 0x8b949e), true, false),
        Tok::Keyword | Tok::Meta => (pick(0xd73a49, 0xff7b72), false, false),
        Tok::Str | Tok::Regex => (pick(0x032f62, 0xa5d6ff), false, false),
        Tok::Number | Tok::Constant | Tok::Attribute => (pick(0x005cc5, 0x79c0ff), false, false),
        Tok::Function => (pick(0x6f42c1, 0xd2a8ff), false, false),
        Tok::Type | Tok::Variable => (pick(0xe36209, 0xffa657), false, false),
        Tok::Tag => (pick(0x22863a, 0x7ee787), false, false),
        Tok::DiffAdd => (pick(0x22863a, 0xaff5b4), false, false),
        Tok::DiffDel => (pick(0xb31d28, 0xffdcd7), false, false),
        Tok::DiffMeta => (pick(0x7a7a7a, 0x8b949e), false, false),
        Tok::Heading => (pick(0x005cc5, 0x79c0ff), false, true),
    }
}

/// Diff row backgrounds (B.3.4).
fn diff_bg(t: Tok, dark: bool) -> Option<u32> {
    match (t, dark) {
        (Tok::DiffAdd, false) => Some(0xe6ffec),
        (Tok::DiffAdd, true) => Some(0x12261e),
        (Tok::DiffDel, false) => Some(0xffebe9),
        (Tok::DiffDel, true) => Some(0x25171c),
        _ => None,
    }
}

/// Is the palette dark? (relative luminance of the page background).
pub fn is_dark(bg: u32) -> bool {
    yamdv_core::theme::luminance(bg) < 0.4
}

/// One logical line of the block: byte range and UTF-16 range in the block text.
struct Line {
    bytes: std::ops::Range<usize>,
    u16s: std::ops::Range<u32>,
}

fn lines_of(text: &str) -> Vec<Line> {
    let mut v = Vec::new();
    let (mut b, mut u) = (0usize, 0u32);
    for l in text.split('\n') {
        let n16 = l.encode_utf16().count() as u32;
        v.push(Line {
            bytes: b..b + l.len(),
            u16s: u..u + n16,
        });
        b += l.len() + 1;
        u += n16 + 1;
    }
    v
}

/// Lays out a code block card; returns its height.
#[allow(clippy::too_many_arguments)]
pub(crate) fn code_block(
    ctx: &mut Ctx<'_>,
    node: usize,
    info: &str,
    pt: ParaText,
    x: f32,
    y: f32,
    w: f32,
    out: &mut Out,
) -> f32 {
    let m = ctx.m;
    let dark = is_dark(ctx.pal.bg);
    let opts = (ctx.code_opts)(ctx.block, node);
    let lang: Option<Lang> = lang_for(info);
    let label = info.split_whitespace().next().unwrap_or("").to_string();
    let (text_color, _, _) = syntax_style(Tok::Text, dark);
    let code = Style {
        family: Family::Code,
        size: m.code_size(),
        weight: 400,
        italic: false,
        color: text_color,
        strike: false,
        underline: false,
    };
    let pitch = m.code_pitch();
    let all = lines_of(&pt.text);
    let n_lines = all.len().max(1);
    // Gutter for line numbers: digits × digit width + 16; text starts 12 px after it.
    let digit_w = {
        let s = ctx.shaper.shape(&ParaSpec {
            text: "0000000000",
            base: code,
            spans: vec![],
            width: None,
            line_pitch: pitch,
            rtl: false,
            tab_spaces: None,
        });
        s.width() / 10.0
    };
    let digits = n_lines.to_string().len().max(2);
    let gutter = if opts.line_numbers {
        digits as f32 * digit_w + 16.0
    } else {
        0.0
    };
    let text_x = x
        + CODE_PAD_X
        + if opts.line_numbers {
            gutter + 12.0 - CODE_PAD_X / 2.0
        } else {
            0.0
        };
    let text_w = (x + w - CODE_PAD_X - text_x).max(40.0);
    let top = y + CODE_HEADER + CODE_PAD_TOP;

    // Highlight (per-line states) → spans per chunk; diff rows.
    let mut state = yamdv_highlight::State::NORMAL;
    let mut line_tok: Vec<Option<Tok>> = Vec::with_capacity(all.len());
    let mut chunks: Vec<(usize, usize, Vec<Span>)> = Vec::new(); // (first line, end line, spans)
    let mut first = 0;
    while first < all.len() {
        let end = (first + CHUNK_LINES).min(all.len());
        let base_u16 = all[first].u16s.start;
        let mut spans = Vec::new();
        for l in &all[first..end] {
            let s = pt.text.get(l.bytes.clone()).unwrap_or("");
            let mut kind = None;
            if let Some(lang) = lang {
                let (toks, next) = highlight_line(lang, state, s);
                state = next;
                // Byte → UTF-16 offsets inside the line.
                let mut u = l.u16s.start - base_u16;
                let mut last = 0usize;
                for (r, t) in toks {
                    if let Some(pre) = s.get(last..r.start) {
                        u += pre.encode_utf16().count() as u32;
                    }
                    let len = s
                        .get(r.clone())
                        .map_or(0, |t| t.encode_utf16().count() as u32);
                    if matches!(t, Tok::DiffAdd | Tok::DiffDel | Tok::DiffMeta) {
                        kind = Some(t);
                    }
                    if t != Tok::Text && len > 0 {
                        let (color, italic, bold) = syntax_style(t, dark);
                        spans.push(Span {
                            range: u..u + len,
                            style: Style {
                                color,
                                italic,
                                weight: if bold { 600 } else { 400 },
                                ..code
                            },
                        });
                    }
                    u += len;
                    last = r.end;
                }
            }
            line_tok.push(kind);
        }
        chunks.push((first, end, spans));
        first = end;
    }

    // Shape chunks; collect visual rows per logical line (wrap).
    let mut rows: Vec<usize> = Vec::with_capacity(all.len());
    let mut shaped_chunks = Vec::with_capacity(chunks.len());
    let mut yy = top;
    let mut max_w = 0.0f32;
    for (a, b, spans) in chunks {
        let start = all[a].bytes.start;
        let end = all[b - 1].bytes.end;
        let text = pt.text.get(start..end).unwrap_or("").to_string();
        let shaped = ctx.shaper.shape(&ParaSpec {
            text: &text,
            base: code,
            spans,
            width: opts.wrap.then_some(text_w),
            line_pitch: pitch,
            rtl: false,
            tab_spaces: Some(4),
        });
        let base_u16 = all[a].u16s.start;
        for l in &all[a..b] {
            let r = if opts.wrap {
                let rr = (l.u16s.start - base_u16)
                    ..(l.u16s.end - base_u16).max(l.u16s.start - base_u16 + 1);
                let mut ys: Vec<i32> = shaped
                    .range_rects(rr)
                    .iter()
                    .map(|r| (r.y / pitch).round() as i32)
                    .collect();
                ys.dedup();
                ys.len().max(1)
            } else {
                1
            };
            rows.push(r);
        }
        let h = shaped.height().max(pitch * (b - a) as f32);
        max_w = max_w.max(shaped.width());
        // Sub-ParaText for this chunk keeps the source mapping for selection/copy.
        let sub = pt.slice_bytes(start..end);
        shaped_chunks.push((yy, shaped, sub));
        yy += h;
    }
    let text_h = (yy - top).max(pitch);
    let h = CODE_HEADER + CODE_PAD_TOP + text_h + CODE_PAD_BOTTOM;
    let rect = Rect::new(x, y, w, h);
    out.items.push(Item::Fill {
        rect,
        color: ctx.pal.code_bg,
        radius: 8.0,
    });

    // Diff row backgrounds (full card width).
    let mut ry = top;
    for (i, r) in rows.iter().enumerate() {
        let rh = *r as f32 * pitch;
        if let Some(bg) = line_tok
            .get(i)
            .copied()
            .flatten()
            .and_then(|t| diff_bg(t, dark))
        {
            out.items.push(Item::Fill {
                rect: Rect::new(x + 1.0, ry, w - 2.0, rh),
                color: bg,
                radius: 0.0,
            });
        }
        ry += rh;
    }

    // Line numbers (drawn, never copied: the paragraph has no source map).
    if opts.line_numbers {
        let mut s = String::new();
        for (i, r) in rows.iter().enumerate() {
            let n = (i + 1).to_string();
            s.push_str(&" ".repeat(digits.saturating_sub(n.len())));
            s.push_str(&n);
            for _ in 0..*r {
                s.push('\n');
            }
        }
        s.pop();
        let ln = Style {
            color: ctx.pal.line_number,
            ..code
        };
        let shaped = ctx.shaper.shape(&ParaSpec {
            text: &s,
            base: ln,
            spans: vec![],
            width: None,
            line_pitch: pitch,
            rtl: false,
            tab_spaces: None,
        });
        out.para(x + CODE_PAD_X / 2.0 + 8.0, top, shaped, None, None);
    }

    // Code text, clipped and horizontally scrollable when it overflows (R-CODE-03).
    let overflow = !opts.wrap && max_w > text_w;
    let view_x = if opts.line_numbers {
        text_x - 4.0
    } else {
        x + 1.0
    };
    if overflow {
        let view = Rect::new(
            view_x,
            top - CODE_PAD_TOP,
            x + w - 1.0 - view_x,
            text_h + CODE_PAD_TOP,
        );
        let region = out.regions.len();
        out.regions.push(HRegion {
            view,
            content_w: max_w + (text_x - view_x) + CODE_PAD_X,
            track: Rect::new(
                x + CODE_PAD_X,
                top + text_h + (CODE_PAD_BOTTOM - 6.0) / 2.0,
                w - 2.0 * CODE_PAD_X,
                6.0,
            ),
        });
        out.items.push(Item::PushClip(view));
        for (cy, shaped, sub) in shaped_chunks {
            out.para(text_x, cy, shaped, Some(sub), Some(region as u16));
        }
        out.items.push(Item::PopClip);
        out.items.push(Item::HScrollBar(region));
    } else {
        for (cy, shaped, sub) in shaped_chunks {
            out.para(text_x, cy, shaped, Some(sub), None);
        }
    }

    // Header toolbar (drawn on hover by the platform; B.6.7): [copy] [line numbers] [wrap] label.
    let label_w = if label.is_empty() {
        0.0
    } else {
        let s = ctx.shaper.shape(&ParaSpec {
            text: &label,
            base: Style {
                family: Family::Body,
                size: 11.0,
                ..code
            },
            spans: vec![],
            width: None,
            line_pitch: 11.0 * 1.4,
            rtl: false,
            tab_spaces: None,
        });
        s.width()
    };
    let label_rect = Rect::new(
        x + w - 8.0 - label_w,
        y + (CODE_HEADER - 15.4) / 2.0 + 2.0,
        label_w,
        15.4,
    );
    let mut bx = label_rect.x - if label_w > 0.0 { 8.0 } else { 0.0 } - 3.0 * BUTTON - 8.0;
    let by = y + (CODE_HEADER - BUTTON) / 2.0 + 2.0;
    let mut buttons = [Rect::default(); 3];
    for (k, kind) in [
        HotKind::CodeCopy { node },
        HotKind::CodeLines { node },
        HotKind::CodeWrap { node },
    ]
    .into_iter()
    .enumerate()
    {
        let r = Rect::new(bx, by, BUTTON, BUTTON);
        buttons[k] = r;
        out.hot.push(Hot { rect: r, kind });
        bx += BUTTON + 4.0;
    }
    out.items.push(Item::CodeToolbar {
        node,
        card: rect,
        label,
        label_rect,
        buttons,
        opts,
        raw: pt.source_range(),
    });
    out.items.push(Item::Stroke {
        rect,
        color: ctx.pal.code_border,
        radius: 8.0,
    });
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_map_bytes_and_utf16() {
        let l = lines_of("añ\n😀b\n");
        assert_eq!(l.len(), 3);
        assert_eq!(l[0].bytes, 0..3);
        assert_eq!(l[0].u16s, 0..2);
        assert_eq!(l[1].u16s, 3..6);
        assert_eq!(l[2].bytes, 10..10);
    }

    #[test]
    fn syntax_palette_matches_b34() {
        assert_eq!(syntax_style(Tok::Keyword, false).0, 0xd73a49);
        assert!(syntax_style(Tok::Comment, true).1);
        assert!(syntax_style(Tok::Heading, false).2);
        assert_eq!(diff_bg(Tok::DiffAdd, false), Some(0xe6ffec));
    }
}
