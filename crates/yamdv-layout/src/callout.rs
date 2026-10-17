//! Callouts (R-MD-02/04, R-REN-06, UI_SPEC §B.6.6, B.3.3): GFM alerts and Obsidian
//! `> [!type][+-] Title` blockquotes, with vector icons and folding.

#![allow(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_arguments
)]

use yamdv_core::doc::detail::{Alert, Detail, NodeKind};
use yamdv_core::doc::text::{ParaText, node_text};
use yamdv_core::theme::CALLOUTS;

use crate::block::{Ctx, Env, Hot, HotKind, Item, Out, base, leaf, stack};
use crate::geom::Rect;
use crate::shaper::{ParaSpec, is_rtl};

/// A recognized callout.
#[derive(Debug, Clone, PartialEq)]
pub struct Callout {
    /// Visual style index into `CALLOUTS` (note, tip, important, warning, caution).
    pub style: usize,
    /// Explicit title (`None` = localized default).
    pub title: Option<String>,
    /// `Some(collapsed_by_default)` for foldable callouts.
    pub fold: Option<bool>,
    /// Remainder of the first paragraph after the `[!type]` line (Obsidian), if any.
    pub rest: Option<ParaText>,
    /// Whether the first child paragraph was consumed by the marker line.
    pub skip_first: bool,
}

/// R-MD-04 type mapping (case-insensitive) → style index; unknown types map to *note*.
pub fn style_of(t: &str) -> usize {
    match t.to_ascii_lowercase().as_str() {
        "tip" | "hint" | "success" | "check" | "done" | "example" => 1,
        "important" | "question" | "help" | "faq" => 2,
        "warning" | "attention" => 3,
        "caution" | "danger" | "error" | "failure" | "fail" | "missing" | "bug" => 4,
        _ => 0,
    }
}

fn known(t: &str) -> bool {
    matches!(
        t.to_ascii_lowercase().as_str(),
        "note" | "info" | "abstract" | "summary" | "tldr" | "todo" | "quote" | "cite"
    ) || style_of(t) != 0
}

/// Parses an Obsidian marker line `[!type]`, `[!type]+`, `[!type]-` with an optional title.
/// Returns (type, fold, title).
pub fn parse_marker(line: &str) -> Option<(&str, Option<bool>, &str)> {
    let rest = line.trim_start().strip_prefix("[!")?;
    let close = rest.find(']')?;
    let ty = &rest[..close];
    if ty.is_empty()
        || !ty
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    let mut after = &rest[close + 1..];
    let fold = if let Some(a) = after.strip_prefix('+') {
        after = a;
        Some(false)
    } else if let Some(a) = after.strip_prefix('-') {
        after = a;
        Some(true)
    } else {
        None
    };
    Some((ty, fold, after.trim()))
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// Recognizes blockquote `i` as a callout.
pub fn detect(src: &str, d: &Detail, i: usize, alert: Option<Alert>, snb: bool) -> Option<Callout> {
    if let Some(a) = alert {
        let style = match a {
            Alert::Note => 0,
            Alert::Tip => 1,
            Alert::Important => 2,
            Alert::Warning => 3,
            Alert::Caution => 4,
        };
        return Some(Callout {
            style,
            title: None,
            fold: None,
            rest: None,
            skip_first: false,
        });
    }
    let first = d.children(i).next()?;
    if !matches!(d.nodes[first].kind, NodeKind::Paragraph { .. }) {
        return None;
    }
    let pt = node_text(src, d, first, snb);
    // The marker line ends at the first source line break (a soft break is a space in `pt`).
    let r = &d.nodes[first].range;
    let para_src = src.get(r.start as usize..r.end as usize).unwrap_or("");
    let nl = para_src.find('\n');
    let (line_end, next) = match nl {
        Some(n) => {
            let u16i = pt.from_source(r.start + n as u32);
            let mut b = 0usize;
            let mut acc = 0u32;
            for ch in pt.text.chars() {
                if acc >= u16i {
                    break;
                }
                acc += ch.len_utf16() as u32;
                b += ch.len_utf8();
            }
            let skip = pt.text[b..].chars().next().map_or(0, char::len_utf8);
            (b, b + skip)
        }
        None => (pt.text.len(), pt.text.len()),
    };
    let (ty, fold, title) = parse_marker(&pt.text[..line_end])?;
    let title = if title.is_empty() {
        if known(ty) {
            None
        } else {
            Some(capitalize(ty))
        }
    } else {
        Some(title.to_string())
    };
    let style = style_of(ty);
    let title = match (title, style, ty.to_ascii_lowercase().as_str()) {
        // Aliases of a known style show their own name, like Obsidian.
        (None, _, t) if !matches!(t, "note" | "tip" | "important" | "warning" | "caution") => {
            Some(capitalize(ty))
        }
        (t, _, _) => t,
    };
    let rest = (next < pt.text.len()).then(|| pt.slice_bytes(next..pt.text.len()));
    Some(Callout {
        style,
        title,
        fold,
        rest,
        skip_first: true,
    })
}

/// 14 px vector icon at (x, y) per style (B.14 fallbacks).
fn icon(style: usize, x: f32, y: f32, color: u32, out: &mut Out) {
    let l = |pts: Vec<(f32, f32)>, closed: bool| Item::Line {
        pts,
        color,
        width: 1.5,
        closed,
    };
    let circle: Vec<(f32, f32)> = (0..16)
        .map(|k| {
            let a = k as f32 / 16.0 * std::f32::consts::TAU;
            (x + 7.0 + 6.0 * a.cos(), y + 7.0 + 6.0 * a.sin())
        })
        .collect();
    match style {
        // note: (i)
        0 => {
            out.items.push(l(circle, true));
            out.items
                .push(l(vec![(x + 7.0, y + 6.5), (x + 7.0, y + 10.0)], false));
            out.items
                .push(l(vec![(x + 7.0, y + 4.0), (x + 7.0, y + 4.2)], false));
        }
        // tip: light bulb
        1 => {
            out.items.push(l(
                vec![
                    (x + 5.0, y + 10.0),
                    (x + 3.0, y + 6.5),
                    (x + 4.0, y + 2.5),
                    (x + 7.0, y + 1.0),
                    (x + 10.0, y + 2.5),
                    (x + 11.0, y + 6.5),
                    (x + 9.0, y + 10.0),
                ],
                true,
            ));
            out.items
                .push(l(vec![(x + 5.5, y + 12.5), (x + 8.5, y + 12.5)], false));
        }
        // important: speech box with "!"
        2 => {
            out.items.push(l(
                vec![
                    (x + 1.0, y + 1.0),
                    (x + 13.0, y + 1.0),
                    (x + 13.0, y + 10.0),
                    (x + 6.0, y + 10.0),
                    (x + 3.0, y + 13.0),
                    (x + 3.0, y + 10.0),
                    (x + 1.0, y + 10.0),
                ],
                true,
            ));
            out.items
                .push(l(vec![(x + 7.0, y + 3.5), (x + 7.0, y + 6.0)], false));
            out.items
                .push(l(vec![(x + 7.0, y + 8.0), (x + 7.0, y + 8.2)], false));
        }
        // warning: triangle with "!"
        3 => {
            out.items.push(l(
                vec![
                    (x + 7.0, y + 1.0),
                    (x + 13.0, y + 13.0),
                    (x + 1.0, y + 13.0),
                ],
                true,
            ));
            out.items
                .push(l(vec![(x + 7.0, y + 5.5), (x + 7.0, y + 8.5)], false));
            out.items
                .push(l(vec![(x + 7.0, y + 10.8), (x + 7.0, y + 11.0)], false));
        }
        // caution: octagon with "!"
        _ => {
            let oct: Vec<(f32, f32)> = (0..8)
                .map(|k| {
                    let a = (k as f32 + 0.5) / 8.0 * std::f32::consts::TAU;
                    (x + 7.0 + 6.5 * a.cos(), y + 7.0 + 6.5 * a.sin())
                })
                .collect();
            out.items.push(l(oct, true));
            out.items
                .push(l(vec![(x + 7.0, y + 4.0), (x + 7.0, y + 7.5)], false));
            out.items
                .push(l(vec![(x + 7.0, y + 9.8), (x + 7.0, y + 10.0)], false));
        }
    }
}

/// Lays out a callout; returns its height.
pub(crate) fn callout(
    ctx: &mut Ctx<'_>,
    src: &str,
    d: &Detail,
    i: usize,
    c: Callout,
    x: f32,
    y: f32,
    w: f32,
    env: &Env,
    out: &mut Out,
) -> f32 {
    let m = ctx.m;
    let dark = crate::code::is_dark(ctx.pal.bg);
    let tok = CALLOUTS[c.style.min(CALLOUTS.len() - 1)];
    let (color, bg) = if dark {
        (tok.dark, tok.dark_bg)
    } else {
        (tok.light, tok.light_bg)
    };
    let collapsed = c
        .fold
        .is_some_and(|def| def != (ctx.code_opts)(ctx.block, i).fold_toggled);
    let bg_at = out.items.len();
    let cx = x + 3.0 + 16.0;
    let cw = (w - 3.0 - 32.0).max(1.0);
    let title = c.title.clone().unwrap_or_else(|| {
        let a = [
            Alert::Note,
            Alert::Tip,
            Alert::Important,
            Alert::Warning,
            Alert::Caution,
        ][c.style.min(4)];
        ctx.strings.alert(a).to_string()
    });
    let ts = crate::shaper::Style {
        size: 14.0,
        weight: 600,
        ..base(&m, color)
    };
    let chevron = if c.fold.is_some() { 20.0 } else { 0.0 };
    let tw = (cw - 20.0 - chevron).max(1.0);
    let shaped = ctx.shaper.shape(&ParaSpec {
        text: &title,
        base: ts,
        spans: vec![],
        width: Some(tw),
        line_pitch: 14.0 * 1.4,
        rtl: is_rtl(&title),
        tab_spaces: None,
        objects: Vec::new(),
    });
    let ty = y + 12.0;
    let th = shaped.height().max(14.0 * 1.4);
    icon(c.style, cx, ty + (14.0 * 1.4 - 14.0) / 2.0, color, out);
    out.para(cx + 20.0, ty, shaped, None, None);
    if c.fold.is_some() {
        let chx = cx + cw - 10.0;
        let chy = ty + th / 2.0;
        let pts = if collapsed {
            vec![
                (chx - 2.0, chy - 4.0),
                (chx + 2.0, chy),
                (chx - 2.0, chy + 4.0),
            ]
        } else {
            vec![
                (chx - 4.0, chy - 2.0),
                (chx, chy + 2.0),
                (chx + 4.0, chy - 2.0),
            ]
        };
        out.items.push(Item::Line {
            pts,
            color,
            width: 1.5,
            closed: false,
        });
        out.hot.push(Hot {
            rect: Rect::new(x, y, w, 12.0 + th),
            kind: HotKind::Fold { node: i },
        });
    }
    let mut bottom = ty + th;
    if !collapsed {
        let inner = Env {
            color: ctx.pal.text,
            list_depth: env.list_depth,
            tight: false,
        };
        let mut by = bottom + 8.0;
        let mut any = false;
        if let Some(rest) = c.rest
            && !rest.text.trim().is_empty()
        {
            by += leaf(
                ctx,
                rest,
                base(&m, ctx.pal.text),
                m.pitch(),
                cx,
                by,
                cw,
                out,
            );
            by += m.para_spacing;
            any = true;
        }
        let kids: Vec<usize> = d.children(i).skip(usize::from(c.skip_first)).collect();
        if kids.is_empty() {
            if any {
                by -= m.para_spacing;
            }
            if any {
                bottom = by;
            }
        } else {
            bottom = stack(ctx, src, d, &kids, cx, by, cw, &inner, out);
        }
    }
    let h = bottom + 12.0 - y;
    let radius = if m.rounded { 6.0 } else { 0.0 };
    out.items.insert(
        bg_at,
        Item::Fill {
            rect: Rect::new(x, y, 3.0, h),
            color,
            radius: 0.0,
        },
    );
    out.items.insert(
        bg_at,
        Item::Fill {
            rect: Rect::new(x, y, 3.0 + radius, h),
            color: bg,
            radius: 0.0,
        },
    );
    out.items.insert(
        bg_at,
        Item::Fill {
            rect: Rect::new(x, y, w, h),
            color: bg,
            radius,
        },
    );
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r_md_04_marker_and_mapping() {
        assert_eq!(parse_marker("[!info] Title"), Some(("info", None, "Title")));
        assert_eq!(parse_marker("[!bug]-"), Some(("bug", Some(true), "")));
        assert_eq!(parse_marker("[!faq]+ Q?"), Some(("faq", Some(false), "Q?")));
        assert_eq!(parse_marker("[x] no"), None);
        assert_eq!(parse_marker("[!] no"), None);
        assert_eq!(style_of("INFO"), 0);
        assert_eq!(style_of("success"), 1);
        assert_eq!(style_of("help"), 2);
        assert_eq!(style_of("attention"), 3);
        assert_eq!(style_of("danger"), 4);
        assert_eq!(style_of("whatever"), 0);
    }

    #[test]
    fn detect_obsidian_and_gfm() {
        use yamdv_core::doc::{Document, ReadOpts};
        let src = "> [!danger]- Careful\n> body text\n";
        let doc = Document::new(src.into(), ReadOpts::default());
        let d = doc.detail_uncached(0);
        let c = detect(src, &d, 0, None, false).expect("callout");
        assert_eq!(c.style, 4);
        assert_eq!(c.fold, Some(true));
        assert_eq!(c.title.as_deref(), Some("Careful"));
        assert_eq!(c.rest.map(|r| r.text).as_deref(), Some("body text"));
        let src = "> [!custom]\n> x\n";
        let doc = Document::new(src.into(), ReadOpts::default());
        let d = doc.detail_uncached(0);
        let c = detect(src, &d, 0, None, false).expect("callout");
        assert_eq!((c.style, c.title.as_deref()), (0, Some("Custom")));
        let src = "> plain quote\n";
        let doc = Document::new(src.into(), ReadOpts::default());
        let d = doc.detail_uncached(0);
        assert!(detect(src, &d, 0, None, false).is_none());
    }
}
