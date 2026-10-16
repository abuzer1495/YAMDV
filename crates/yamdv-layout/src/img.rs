//! Block images (R-IMG-03/04/06): a paragraph that holds exactly one image becomes a block
//! image — the decoded picture at display size, or the full R-IMG-06 placeholder. Inline
//! images keep the P-25 chip. Pixels are owned by the platform through [`ImageHost`].

use crate::Rect;
use crate::block::{Ctx, Hot, HotKind, Item, Out, base, shape_text};
use crate::shaper::Style;
use yamdv_core::doc::detail::{Detail, ImageRef, style as st};
use yamdv_core::image::{self, Placeholder};

/// What the platform knows about an image source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImgState {
    /// Not probed yet: layout uses the 4:3 estimate.
    Pending,
    /// Header probe done: intrinsic size in image pixels.
    Ready {
        /// Width.
        w: u32,
        /// Height.
        h: u32,
    },
    /// Shown as a placeholder.
    Fail(Placeholder),
}

/// Image knowledge of the platform (resolution, probe, decode). `state` may queue work.
pub trait ImageHost {
    /// State of `src` (as written in the document).
    fn state(&self, src: &str) -> ImgState;
}

/// A host that knows nothing (pure tests, fuzzing): every image stays pending.
#[derive(Debug, Default)]
pub struct NoImages;

impl ImageHost for NoImages {
    fn state(&self, _: &str) -> ImgState {
        ImgState::Pending
    }
}

/// Shared [`NoImages`] instance.
pub static NO_IMAGES: NoImages = NoImages;

/// Placeholder card height.
pub const PLACEHOLDER_H: f32 = 64.0;

/// The single image of paragraph node `i` when the paragraph holds nothing else.
pub fn sole_image<'d>(src: &str, d: &'d Detail, i: usize) -> Option<&'d ImageRef> {
    let node = d.nodes.get(i)?;
    let (a, b) = (node.runs.start as usize, node.runs.end as usize);
    let runs = d.runs.get(a..b)?;
    let mut found = None;
    for (k, r) in runs.iter().enumerate() {
        if r.style & st::IMAGE != 0 {
            if found.is_some() {
                return None;
            }
            found = Some(a + k);
        } else {
            let t = r
                .owned
                .as_deref()
                .or_else(|| src.get(r.range.start as usize..r.range.end as usize))
                .unwrap_or("");
            if !t.trim().is_empty() {
                return None;
            }
        }
    }
    let first = found?;
    // The k-th IMAGE run of the tree is `images[k]`.
    let k = d
        .runs
        .iter()
        .take(first)
        .filter(|r| r.style & st::IMAGE != 0)
        .count();
    d.images.get(k)
}

/// Display size in logical px for `r` with state `s` inside `column` (R-IMG-03).
pub fn size(r: &ImageRef, s: &ImgState, column: f32) -> (f32, f32) {
    #[allow(clippy::cast_precision_loss)]
    let req = image::Req {
        w: r.w.map(|v| v as f32),
        h: r.h.map(|v| v as f32),
    };
    match (s, req.w, req.h) {
        (ImgState::Ready { w, h }, ..) => image::display_size(*w, *h, req, column),
        // Both sizes given: exact without a probe.
        (_, Some(_), Some(_)) => image::display_size(1, 1, req, column),
        _ => image::estimate(column),
    }
}

/// Lays out a block image at (`x`, `y`) in a column of width `w`; returns the height.
pub(crate) fn block(ctx: &mut Ctx<'_>, r: &ImageRef, x: f32, y: f32, w: f32, out: &mut Out) -> f32 {
    let s = ctx.images.state(&r.src);
    if let ImgState::Fail(p) = &s {
        return placeholder(ctx, r, p, x, y, w, out);
    }
    let (iw, ih) = size(r, &s, w);
    // Centered when narrower than the column.
    let ix = x + ((w - iw) / 2.0).max(0.0);
    let rect = Rect::new(ix, y, iw, ih);
    out.items.push(Item::Image {
        rect,
        src: r.src.clone(),
        ready: matches!(s, ImgState::Ready { .. }),
    });
    out.hot.push(Hot {
        rect,
        kind: HotKind::Image { src: r.src.clone() },
    });
    ih
}

fn icon(p: &Placeholder) -> &'static str {
    match p {
        Placeholder::Remote => "\u{1F310}",
        Placeholder::Network => "\u{26D4}",
        Placeholder::TooLarge => "\u{2196}",
        Placeholder::Unsupported | Placeholder::Invalid | Placeholder::InvalidPath => "\u{26A0}",
        Placeholder::Missing => "\u{25A3}",
    }
}

/// R-IMG-06 block placeholder: icon, message per reason, alt text and path/URL, and
/// [Copy URL] for remote images.
fn placeholder(
    ctx: &mut Ctx<'_>,
    r: &ImageRef,
    p: &Placeholder,
    x: f32,
    y: f32,
    w: f32,
    out: &mut Out,
) -> f32 {
    let m = ctx.m;
    let remote = *p == Placeholder::Remote;
    let tw = (w - 64.0 - if remote { 112.0 } else { 0.0 }).max(40.0);
    let mut msg = ctx.strings.image_message(p).to_string();
    if !r.alt.trim().is_empty() {
        msg.push_str(" \u{2014} ");
        msg.push_str(&r.alt);
    }
    let ms = Style {
        weight: 600,
        ..base(&m, ctx.pal.text)
    };
    let t = shape_text(ctx, &msg, ms, Some(tw), m.pitch());
    let th = t.height();
    let h = (th + 16.0 + 24.0).max(PLACEHOLDER_H);
    let rect = Rect::new(x, y, w, h);
    out.items.push(Item::Fill {
        rect,
        color: ctx.pal.code_bg,
        radius: 8.0,
    });
    out.items.push(Item::Stroke {
        rect,
        color: ctx.pal.code_border,
        radius: 8.0,
    });
    let ic = Style {
        size: 20.0,
        ..base(&m, ctx.pal.text_muted)
    };
    let i = shape_text(ctx, icon(p), ic, None, 28.0);
    let ih = i.height();
    out.para(x + 16.0, y + (h - ih) / 2.0, i, None, None);
    out.para(x + 52.0, y + 10.0, t, None, None);
    let shown: String = if r.src.starts_with("data:") {
        "data:".to_string()
    } else {
        r.src.chars().take(200).collect()
    };
    let ps = Style {
        size: 12.0,
        ..base(&m, ctx.pal.text_muted)
    };
    let pth = shape_text(ctx, &shown, ps, Some(tw), 16.0);
    out.para(x + 52.0, y + 10.0 + th + 2.0, pth, None, None);
    if remote {
        let b = Rect::new(x + w - 104.0, y + (h - 28.0) / 2.0, 88.0, 28.0);
        out.items.push(Item::Stroke {
            rect: b,
            color: ctx.pal.border_subtle,
            radius: 6.0,
        });
        let bs = Style {
            size: 12.0,
            ..base(&m, ctx.pal.text)
        };
        let label = ctx.strings.copy_url().to_string();
        let l = shape_text(ctx, &label, bs, None, 16.0);
        let (lw, lh) = (l.width(), l.height());
        out.para(
            b.x + (b.w - lw) / 2.0,
            b.y + (b.h - lh) / 2.0,
            l,
            None,
            None,
        );
        out.hot.push(Hot {
            rect: b,
            kind: HotKind::CopyUrl { url: r.src.clone() },
        });
    }
    out.hot.push(Hot {
        rect,
        kind: HotKind::ImagePlaceholder {
            src: r.src.clone(),
            reason: p.clone(),
        },
    });
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::{EnStrings, Metrics, layout_block};
    use crate::shaper::MockShaper;
    use yamdv_core::doc::{Document, ReadOpts};

    struct Fixed(ImgState);
    impl ImageHost for Fixed {
        fn state(&self, _: &str) -> ImgState {
            self.0.clone()
        }
    }

    fn lay(src: &str, host: &dyn ImageHost) -> crate::block::BlockLayout {
        let doc = Document::new(src.into(), ReadOpts::default());
        let d = doc.detail_uncached(0);
        let pal = yamdv_core::theme::palette("default", false);
        let mut sh = MockShaper;
        let mut ctx = Ctx {
            images: host,
            shaper: &mut sh,
            m: Metrics::default(),
            pal: &pal,
            strings: &EnStrings,
            single_newline_break: false,
            block: 0,
            code_opts: &|_, _| crate::code::CodeOpts::default(),
        };
        layout_block(&mut ctx, src, &d, 600.0)
    }

    fn image_rect(bl: &crate::block::BlockLayout) -> Option<Rect> {
        bl.items.iter().find_map(|i| match i {
            Item::Image { rect, .. } => Some(*rect),
            _ => None,
        })
    }

    #[test]
    fn r_img_03_probed_size_scaled_and_centered() {
        let bl = lay("![a](x.png)\n", &Fixed(ImgState::Ready { w: 300, h: 100 }));
        let r = image_rect(&bl).unwrap();
        assert_eq!((r.x, r.w, r.h), (150.0, 300.0, 100.0));
        let bl = lay("![a](x.png)\n", &Fixed(ImgState::Ready { w: 1200, h: 600 }));
        let r = image_rect(&bl).unwrap();
        assert_eq!((r.w, r.h), (600.0, 300.0));
        let bl = lay(
            "![[x.png|200]]\n",
            &Fixed(ImgState::Ready { w: 400, h: 400 }),
        );
        assert_eq!(image_rect(&bl).unwrap().w, 200.0);
        let bl = lay("![a](x.png)\n", &NO_IMAGES);
        assert_eq!(image_rect(&bl).unwrap().h, 450.0);
    }

    #[test]
    fn r_img_06_block_placeholder_and_inline_chip() {
        let bl = lay(
            "![logo](https://x/y.png)\n",
            &Fixed(ImgState::Fail(Placeholder::Remote)),
        );
        assert!(image_rect(&bl).is_none());
        assert!(bl.height >= PLACEHOLDER_H);
        assert!(
            bl.hot.iter().any(
                |h| matches!(&h.kind, HotKind::CopyUrl { url } if &**url == "https://x/y.png")
            )
        );
        let bl = lay(
            "![a](x.png)\n",
            &Fixed(ImgState::Fail(Placeholder::InvalidPath)),
        );
        assert!(
            !bl.hot
                .iter()
                .any(|h| matches!(h.kind, HotKind::CopyUrl { .. }))
        );
        // Inline image (text around it) stays a chip.
        let bl = lay(
            "see ![b](x.png) here\n",
            &Fixed(ImgState::Ready { w: 10, h: 10 }),
        );
        assert!(image_rect(&bl).is_none());
    }
}
