//! Front matter card (R-REN-13, B.6.9) and the footnotes section (R-REN-10, B.6.9).

// Pixel quantities and small counts; ranges come from the source by construction.
#![allow(
    clippy::cast_precision_loss,
    clippy::too_many_arguments,
    clippy::indexing_slicing
)]

use std::ops::Range;

use yamdv_core::doc::detail::Detail;

use crate::block::{Ctx, Env, Hot, HotKind, Item, Note, Out, base, leaf, source_text, stack};
use crate::geom::Rect;
use crate::shaper::{Family, Style};

/// `reading.front_matter` values (carried in [`crate::code::CodeOpts::front_matter`]).
pub mod mode {
    /// One summary line, click expands.
    pub const COLLAPSED: u8 = 0;
    /// Key/value grid, click collapses.
    pub const EXPANDED: u8 = 1;
    /// Not shown.
    pub const HIDDEN: u8 = 2;
}

/// Parsed front matter: top-level `key: value` (YAML) / `key = value` (TOML) pairs as source
/// ranges, or `None` pairs when the content is anything more complex (shown raw).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fields {
    /// Source range of the content between the delimiters.
    pub body: Range<u32>,
    /// (key, value) source ranges; empty when `complex`.
    pub pairs: Vec<(Range<u32>, Range<u32>)>,
    /// Number of top-level keys.
    pub count: usize,
    /// Not a flat key/value list.
    pub complex: bool,
}

fn u(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Minimal key/value extraction (R-REN-13, §4.4.4) of the front matter block at `range`.
pub fn parse(src: &str, range: &Range<u32>) -> Fields {
    let start = range.start as usize;
    let text = src.get(start..range.end as usize).unwrap_or("");
    let toml = text.starts_with("+++");
    let sep = if toml { '=' } else { ':' };
    let mut lines: Vec<(usize, &str)> = Vec::new();
    let mut off = 0;
    for l in text.split_inclusive('\n') {
        lines.push((off, l.trim_end_matches(['\n', '\r'])));
        off += l.len();
    }
    // Drop the opening and closing delimiter lines.
    let inner = if lines.len() >= 2 {
        &lines[1..lines.len() - 1]
    } else {
        &lines[..0]
    };
    let body = inner.first().map_or(start..start, |f| {
        let last = inner.last().map_or(f, |l| l);
        start + f.0..start + last.0 + last.1.len()
    });
    let mut pairs = Vec::new();
    let mut count = 0;
    let mut complex = false;
    for &(o, l) in inner {
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if l.starts_with([' ', '\t', '-', '[']) {
            complex = true;
            continue;
        }
        let Some(p) = l.find(sep) else {
            complex = true;
            continue;
        };
        let key = l[..p].trim_end();
        let val = l[p + 1..].trim();
        count += 1;
        if key.is_empty() || val.is_empty() || val.starts_with(['[', '{', '|', '>']) {
            complex = true;
            continue;
        }
        let ks = start + o;
        let vs = start + o + p + 1 + (l[p + 1..].len() - l[p + 1..].trim_start().len());
        pairs.push((u(ks)..u(ks + key.len()), u(vs)..u(vs + val.len())));
    }
    if complex {
        pairs.clear();
    }
    Fields {
        body: u(body.start)..u(body.end),
        pairs,
        count,
        complex,
    }
}

fn small(ctx: &Ctx<'_>, color: u32, code: bool) -> Style {
    Style {
        size: 13.0,
        family: if code { Family::Code } else { Family::Body },
        ..base(&ctx.m, color)
    }
}

/// Front matter card; returns its height (0 when hidden).
pub(crate) fn front_matter(
    ctx: &mut Ctx<'_>,
    src: &str,
    d: &Detail,
    i: usize,
    x: f32,
    y: f32,
    w: f32,
    out: &mut Out,
) -> f32 {
    let o = (ctx.code_opts)(ctx.block, i);
    if o.front_matter == mode::HIDDEN {
        return 0.0;
    }
    let expanded = (o.front_matter == mode::EXPANDED) != o.fold_toggled;
    let f = parse(src, &d.nodes[i].range);
    let bg_at = out.items.len();
    let (px, py) = (12.0, 8.0);
    let iw = (w - 2.0 * px - 20.0).max(1.0);
    let line = 13.0 * 1.5;
    let summary = ctx.strings.front_matter_summary(f.count);
    let mut sy = y + py;
    let s = small(ctx, ctx.pal.text_muted, false);
    let sh = crate::block::shape_text(ctx, &summary, s, Some(iw), line);
    let th = sh.height().max(line);
    out.para(x + px, sy, sh, None, None);
    // Chevron (right edge of the summary row).
    let chx = x + w - px - 6.0;
    let chy = sy + th / 2.0;
    let pts = if expanded {
        vec![
            (chx - 4.0, chy - 2.0),
            (chx, chy + 2.0),
            (chx + 4.0, chy - 2.0),
        ]
    } else {
        vec![
            (chx - 2.0, chy - 4.0),
            (chx + 2.0, chy),
            (chx - 2.0, chy + 4.0),
        ]
    };
    out.items.push(Item::Line {
        pts,
        color: ctx.pal.text_muted,
        width: 1.5,
        closed: false,
    });
    out.hot.push(Hot {
        rect: Rect::new(x, y, w, py + th),
        kind: HotKind::Fold { node: i },
    });
    sy += th;
    if expanded {
        sy += 6.0;
        if f.complex || f.pairs.is_empty() {
            let pt = source_text(src, &f.body);
            sy += leaf(
                ctx,
                pt,
                small(ctx, ctx.pal.text, true),
                line,
                x + px,
                sy,
                iw,
                out,
            );
        } else {
            // 2-column grid: keys in the code font (muted), values in text.
            let kw = (iw * 0.35).clamp(48.0, 220.0);
            for (k, v) in &f.pairs {
                let kh = leaf(
                    ctx,
                    source_text(src, k),
                    small(ctx, ctx.pal.text_muted, true),
                    line,
                    x + px,
                    sy,
                    kw - 8.0,
                    out,
                );
                let vh = leaf(
                    ctx,
                    source_text(src, v),
                    small(ctx, ctx.pal.text, false),
                    line,
                    x + px + kw,
                    sy,
                    (iw - kw).max(1.0),
                    out,
                );
                sy += kh.max(vh);
            }
        }
    }
    let h = sy + py - y;
    out.items.insert(
        bg_at,
        Item::Fill {
            rect: Rect::new(x, y, w, h),
            color: ctx.pal.code_bg,
            radius: 6.0,
        },
    );
    h
}

/// Back-link text of reference `k` (0-based): `↩`, `↩²`, `↩³`, … (R-REN-10).
pub fn backlink_label(k: u32) -> String {
    const SUP: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    let mut s = String::from("↩");
    if k > 0 {
        for c in (k + 1).to_string().chars() {
            if let Some(d) = c.to_digit(10) {
                s.push(SUP[d as usize]);
            }
        }
    }
    s
}

/// Footnotes section at `y0` (B.6.9): margin-top 32, 1 px `border_subtle` top border,
/// padding-top 16, 14 px `text_muted`, numbered, `↩` back-links in `link`. Returns the bottom.
pub(crate) fn notes_section(
    ctx: &mut Ctx<'_>,
    src: &str,
    notes: &[Note],
    y0: f32,
    w: f32,
    out: &mut Out,
) -> f32 {
    let mut y = y0 + 32.0;
    out.items.push(Item::Fill {
        rect: Rect::new(0.0, y, w, 1.0),
        color: ctx.pal.border_subtle,
        radius: 0.0,
    });
    y += 1.0 + 16.0;
    let muted = Style {
        size: 14.0,
        ..base(&ctx.m, ctx.pal.text_muted)
    };
    let pitch = 14.0 * 1.5;
    let num_w = 28.0;
    let saved = ctx.m;
    // Note content uses 14 px text (B.6.9).
    ctx.m.font_size = 14.0;
    for note in notes {
        let top = y;
        let label = format!("{}.", note.n);
        let sh = crate::block::shape_text(ctx, &label, muted, None, pitch);
        out.para(0.0, y, sh, None, None);
        let d = &note.detail;
        let kids: Vec<usize> = d
            .roots()
            .flat_map(|r| d.children(r).collect::<Vec<_>>())
            .collect();
        let env = Env {
            color: ctx.pal.text_muted,
            list_depth: 0,
            tight: true,
        };
        let cw = (w - num_w).max(1.0);
        let mut bottom = stack(ctx, src, d, &kids, num_w, y, cw, &env, out).max(y + pitch);
        // Back-links on their own row.
        let mut bx = num_w;
        for k in 0..note.refs.max(1) {
            let t = backlink_label(k);
            let st = Style {
                color: ctx.pal.link,
                ..muted
            };
            let sh = crate::block::shape_text(ctx, &t, st, None, pitch);
            let bw = sh.width();
            let bh = sh.height().max(pitch);
            out.para(bx, bottom, sh, None, None);
            out.hot.push(Hot {
                rect: Rect::new(bx, bottom, bw.max(10.0), bh),
                kind: HotKind::FootBack { n: note.n, k },
            });
            bx += bw + 8.0;
        }
        bottom += pitch;
        out.hot.push(Hot {
            rect: Rect::new(0.0, top, w, bottom - top),
            kind: HotKind::FootNote { n: note.n },
        });
        y = bottom + 8.0;
    }
    ctx.m = saved;
    y
}

/// `<details>` (R-REN-14, B.6.9): summary row with a 12 px chevron and weight 600, content
/// indented 16; closed unless `open`, toggled by clicking the summary (session state).
pub(crate) fn details(
    ctx: &mut Ctx<'_>,
    src: &str,
    d: &Detail,
    i: usize,
    open: bool,
    x: f32,
    y: f32,
    w: f32,
    env: &Env,
    out: &mut Out,
) -> f32 {
    let kids: Vec<usize> = d.children(i).collect();
    let cross = out.cross_details
        && !d.nodes[i + 1..]
            .iter()
            .any(|n| matches!(n.kind, yamdv_core::doc::detail::NodeKind::Details { .. }));
    let key = if cross {
        crate::block::CROSS_DETAILS
    } else {
        i
    };
    let toggled = (ctx.code_opts)(ctx.block, key).fold_toggled;
    let expanded = open != toggled;
    let (summary, body): (Vec<usize>, Vec<usize>) = match kids.first() {
        Some(&k)
            if matches!(
                d.nodes[k].kind,
                yamdv_core::doc::detail::NodeKind::DetailsSummary
            ) =>
        {
            (vec![k], kids[1..].to_vec())
        }
        _ => (Vec::new(), kids.clone()),
    };
    let pitch = ctx.m.pitch();
    let chy = y + pitch / 2.0;
    let chx = x + 6.0;
    let pts = if expanded {
        vec![
            (chx - 4.0, chy - 2.0),
            (chx, chy + 2.0),
            (chx + 4.0, chy - 2.0),
        ]
    } else {
        vec![
            (chx - 2.0, chy - 4.0),
            (chx + 2.0, chy),
            (chx - 2.0, chy + 4.0),
        ]
    };
    out.items.push(Item::Line {
        pts,
        color: env.color,
        width: 1.5,
        closed: false,
    });
    let mut bottom = if summary.is_empty() {
        let label = ctx
            .strings
            .placeholder(yamdv_core::doc::detail::Placeholder::Details)
            .to_string();
        let st = Style {
            weight: 600,
            ..base(&ctx.m, env.color)
        };
        let sh = crate::block::shape_text(ctx, &label, st, Some((w - 18.0).max(1.0)), pitch);
        let h = sh.height().max(pitch);
        out.para(x + 18.0, y, sh, None, None);
        y + h
    } else {
        stack(
            ctx,
            src,
            d,
            &summary,
            x + 18.0,
            y,
            (w - 18.0).max(1.0),
            env,
            out,
        )
        .max(y + pitch)
    };
    out.hot.push(Hot {
        rect: Rect::new(x, y, w, bottom - y),
        kind: HotKind::Fold { node: key },
    });
    if expanded && !body.is_empty() {
        bottom = stack(
            ctx,
            src,
            d,
            &body,
            x + 16.0,
            bottom + 8.0,
            (w - 16.0).max(1.0),
            env,
            out,
        );
    }
    bottom - y
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::block::{BlockLayout, EnStrings, Metrics, layout_block};
    use crate::code::CodeOpts;
    use crate::shaper::MockShaper;
    use crate::viewer::Content;
    use yamdv_core::doc::{Document, ReadOpts};

    fn lay(content: &mut Content, i: usize, o: CodeOpts) -> BlockLayout {
        let pal = yamdv_core::theme::palette("default", false);
        let mut sh = MockShaper;
        let mut ctx = Ctx {
            shaper: &mut sh,
            m: Metrics::default(),
            pal: &pal,
            strings: &EnStrings,
            single_newline_break: false,
            block: i,
            code_opts: &move |_, _| o,
        };
        content.layout(i, &mut ctx, 600.0)
    }

    #[test]
    fn r_ren_10_numbering_by_first_reference() {
        let src = "B[^b] then A[^a] and B again[^b].\n\n[^a]: Note A.\n\n[^b]: Note B.\n\nEnd.\n";
        let doc = Document::new(src.into(), ReadOpts::default());
        let ix = &doc.index;
        assert_eq!(ix.footnote_order.len(), 2);
        assert_eq!(ix.footnote_order[0].label, "b");
        assert_eq!(ix.footnote_order[0].refs, vec![0, 0]);
        assert_eq!(ix.footnote_number("a"), Some(2));
        let d = doc.detail_uncached(0);
        let texts: Vec<&str> = d.runs.iter().map(|r| r.text(src)).collect();
        assert!(
            texts.contains(&"[1]") && texts.contains(&"[2]"),
            "{texts:?}"
        );
    }

    #[test]
    fn r_ren_10_section_at_end_and_definitions_hidden() {
        let src = "Text[^n] and[^n].\n\n[^n]: The note.\n\nLast paragraph.\n";
        let mut c = Content::Markdown(Document::new(src.into(), ReadOpts::default()));
        // Definition block: zero height at its source position.
        let def = lay(&mut c, 1, CodeOpts::default());
        assert!(def.height.abs() < f32::EPSILON);
        let first = lay(&mut c, 0, CodeOpts::default());
        let refs: Vec<_> = first
            .hot
            .iter()
            .filter_map(|h| match h.kind {
                HotKind::FootRef { n, ord } => Some((n, ord)),
                _ => None,
            })
            .collect();
        assert_eq!(refs, vec![(1, 0), (1, 1)]);
        let last = lay(&mut c, 2, CodeOpts::default());
        let backs = last
            .hot
            .iter()
            .filter(|h| matches!(h.kind, HotKind::FootBack { n: 1, .. }))
            .count();
        assert_eq!(backs, 2);
        assert!(
            last.hot
                .iter()
                .any(|h| h.kind == HotKind::FootNote { n: 1 })
        );
        assert_eq!(backlink_label(0), "↩");
        assert_eq!(backlink_label(1), "↩²");
        assert_eq!(backlink_label(10), "↩¹¹");
    }

    #[test]
    fn r_ren_13_front_matter_fields() {
        let src = "---\ntitle: Hello\ndate: 2026-10-16\n---\n# H\n";
        let f = parse(
            src,
            &(0..u32::try_from(src.find("# H").unwrap_or(0)).unwrap_or(0)),
        );
        assert_eq!(f.count, 2);
        assert!(!f.complex);
        let (k, v) = &f.pairs[0];
        assert_eq!(&src[k.start as usize..k.end as usize], "title");
        assert_eq!(&src[v.start as usize..v.end as usize], "Hello");
        let toml = "+++\na = 1\nb = \"x\"\n+++\n";
        let t = parse(toml, &(0..u32::try_from(toml.len()).unwrap_or(0)));
        assert_eq!((t.count, t.complex), (2, false));
        let cx = "---\ntags:\n  - a\n---\n";
        let c = parse(cx, &(0..u32::try_from(cx.len()).unwrap_or(0)));
        assert!(c.complex && c.pairs.is_empty());
    }

    #[test]
    fn r_ren_13_front_matter_modes() {
        let src = "---\ntitle: Hello\ndate: x\n---\n\nBody\n";
        let doc = Document::new(src.into(), ReadOpts::default());
        let d = doc.detail_uncached(0);
        let pal = yamdv_core::theme::palette("default", false);
        let mut sh = MockShaper;
        let mut run = |o: CodeOpts| {
            let mut ctx = Ctx {
                shaper: &mut sh,
                m: Metrics::default(),
                pal: &pal,
                strings: &EnStrings,
                single_newline_break: false,
                block: 0,
                code_opts: &move |_, _| o,
            };
            layout_block(&mut ctx, src, &d, 600.0)
        };
        let collapsed = run(CodeOpts::default());
        let expanded = run(CodeOpts {
            front_matter: mode::EXPANDED,
            ..CodeOpts::default()
        });
        let toggled = run(CodeOpts {
            fold_toggled: true,
            ..CodeOpts::default()
        });
        let hidden = run(CodeOpts {
            front_matter: mode::HIDDEN,
            ..CodeOpts::default()
        });
        assert!(expanded.height > collapsed.height);
        assert!((toggled.height - expanded.height).abs() < 0.01);
        assert!(hidden.height.abs() < f32::EPSILON);
        assert!(
            collapsed
                .hot
                .iter()
                .any(|h| matches!(h.kind, HotKind::Fold { .. }))
        );
    }
}
