//! SVG decode on the image worker (ADR-0021): `yamdv-svg` renders shapes at display size;
//! text runs are drawn here with DirectWrite glyph runs (system fonts only, S-RES-06) as a
//! grayscale coverage mask composited in the run's color.

use std::sync::atomic::{AtomicU32, Ordering};

use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_METRICS, DWRITE_FONT_STRETCH_NORMAL,
    DWRITE_FONT_STYLE_ITALIC, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_BOLD,
    DWRITE_FONT_WEIGHT_NORMAL, DWRITE_GLYPH_METRICS, DWRITE_GLYPH_RUN,
    DWRITE_MEASURING_MODE_NATURAL, DWRITE_PIXEL_GEOMETRY_FLAT,
    DWRITE_RENDERING_MODE_NATURAL_SYMMETRIC, DWriteCreateFactory, IDWriteFactory,
    IDWriteFontCollection, IDWriteFontFace,
};
use windows::Win32::Graphics::Gdi::{DIBSECTION, GetCurrentObject, GetObjectW, OBJ_BITMAP};
use windows::core::{BOOL, HSTRING};
use yamdv_core::image::Placeholder;
use yamdv_svg::{Anchor, TextRun};

use super::Bitmap;

/// `currentColor` for SVG images: the document text color (0xRRGGBB), set by the window.
static TEXT_COLOR: AtomicU32 = AtomicU32::new(0x0020_2020);

/// Sets the SVG `currentColor` (document text color, 0xRRGGBB).
pub fn set_text_color(rgb: u32) {
    TEXT_COLOR.store(rgb & 0x00ff_ffff, Ordering::Relaxed);
}

/// The current SVG `currentColor`.
pub fn text_color() -> u32 {
    TEXT_COLOR.load(Ordering::Relaxed)
}

/// Whether `bytes` look like SVG (same rule as the header probe).
pub fn is_svg(bytes: &[u8]) -> bool {
    matches!(
        yamdv_core::image::probe(bytes.get(..bytes.len().min(512)).unwrap_or(&[])),
        Some(p) if p.format == yamdv_core::image::Format::Svg
    )
}

fn err(e: yamdv_svg::Error) -> Placeholder {
    match e {
        yamdv_svg::Error::TooLarge | yamdv_svg::Error::TooMany | yamdv_svg::Error::TooDeep => {
            Placeholder::TooLarge
        }
        _ => Placeholder::Invalid,
    }
}

/// Intrinsic size (rounded CSS px).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn size(bytes: &[u8]) -> Result<(u32, u32), Placeholder> {
    let s = std::str::from_utf8(bytes).map_err(|_| Placeholder::Invalid)?;
    let (w, h) = yamdv_svg::size(s).map_err(err)?;
    Ok((w.round().max(1.0) as u32, h.round().max(1.0) as u32))
}

/// Renders `bytes` at `w × h` device px (premultiplied BGRA).
pub fn decode(bytes: &[u8], w: u32, h: u32) -> Result<Bitmap, Placeholder> {
    let s = std::str::from_utf8(bytes).map_err(|_| Placeholder::Invalid)?;
    let c = text_color();
    let [_, r, g, b] = c.to_be_bytes();
    let img = yamdv_svg::render(s, w, h, [r, g, b, 255]).map_err(err)?;
    let mut px = img.rgba;
    for p in px.as_chunks_mut::<4>().0 {
        p.swap(0, 2);
    }
    let mut bmp = Bitmap {
        w: img.w,
        h: img.h,
        px,
        svg_color: Some(c),
    };
    if !img.texts.is_empty() {
        draw_texts(&mut bmp, &img.texts);
    }
    Ok(bmp)
}

/// A run's face, glyph ids and advances (px).
type Shaped = (Face, Vec<u16>, Vec<f32>);

struct Face {
    face: IDWriteFontFace,
    upem: f32,
    ascent: f32,
    descent: f32,
}

fn face(coll: &IDWriteFontCollection, run: &TextRun) -> Option<Face> {
    let fam = match run.family.to_ascii_lowercase().as_str() {
        "" | "sans-serif" | "system-ui" | "ui-sans-serif" => "Segoe UI",
        "serif" | "ui-serif" => "Times New Roman",
        "monospace" | "ui-monospace" => "Consolas",
        "cursive" => "Comic Sans MS",
        _ => run.family.as_str(),
    };
    // SAFETY: valid collection; out-pointers on the stack; COM objects released by Drop.
    unsafe {
        let (mut idx, mut exists) = (0u32, BOOL(0));
        coll.FindFamilyName(&HSTRING::from(fam), &mut idx, &mut exists)
            .ok()?;
        if !exists.as_bool() {
            coll.FindFamilyName(&HSTRING::from("Segoe UI"), &mut idx, &mut exists)
                .ok()?;
            if !exists.as_bool() {
                idx = 0;
            }
        }
        let font = coll
            .GetFontFamily(idx)
            .ok()?
            .GetFirstMatchingFont(
                if run.bold {
                    DWRITE_FONT_WEIGHT_BOLD
                } else {
                    DWRITE_FONT_WEIGHT_NORMAL
                },
                DWRITE_FONT_STRETCH_NORMAL,
                if run.italic {
                    DWRITE_FONT_STYLE_ITALIC
                } else {
                    DWRITE_FONT_STYLE_NORMAL
                },
            )
            .ok()?;
        let face = font.CreateFontFace().ok()?;
        let mut fm = DWRITE_FONT_METRICS::default();
        face.GetMetrics(&mut fm);
        let upem = f32::from(fm.designUnitsPerEm.max(1));
        Some(Face {
            face,
            upem,
            ascent: f32::from(fm.ascent),
            descent: f32::from(fm.descent),
        })
    }
}

/// Glyph ids and advances (px) of `run` (cmap lookup, no shaping: SVG text is single-line runs).
fn glyphs(fc: &Face, run: &TextRun) -> (Vec<u16>, Vec<f32>) {
    let cps: Vec<u32> = run.text.chars().take(4096).map(u32::from).collect();
    let mut ids = vec![0u16; cps.len()];
    let mut m = vec![DWRITE_GLYPH_METRICS::default(); cps.len()];
    let n = u32::try_from(cps.len()).unwrap_or(0);
    // SAFETY: input and output buffers have `n` elements.
    unsafe {
        let _ = fc.face.GetGlyphIndices(cps.as_ptr(), n, ids.as_mut_ptr());
        let _ = fc
            .face
            .GetDesignGlyphMetrics(ids.as_ptr(), n, m.as_mut_ptr(), false);
    }
    #[allow(clippy::cast_precision_loss)]
    let adv = m
        .iter()
        .map(|g| g.advanceWidth as f32 * run.size / fc.upem)
        .collect();
    (ids, adv)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn draw_texts(bmp: &mut Bitmap, runs: &[TextRun]) {
    // SAFETY: plain factory creation (shared factory).
    let Ok(f) = (unsafe { DWriteCreateFactory::<IDWriteFactory>(DWRITE_FACTORY_TYPE_SHARED) })
    else {
        return;
    };
    let mut coll: Option<IDWriteFontCollection> = None;
    // SAFETY: valid out-pointer.
    if unsafe { f.GetSystemFontCollection(&mut coll, false) }.is_err() {
        return;
    }
    let Some(coll) = coll else { return };
    // SAFETY: plain factory call.
    let Ok(params) = (unsafe {
        f.CreateCustomRenderingParams(
            1.0,
            0.0,
            0.0,
            DWRITE_PIXEL_GEOMETRY_FLAT,
            DWRITE_RENDERING_MODE_NATURAL_SYMMETRIC,
        )
    }) else {
        return;
    };
    // SAFETY: plain factory call.
    let Ok(gi) = (unsafe { f.GetGdiInterop() }) else {
        return;
    };
    // Shape every run, then place chunks (a run plus the runs continuing it) by their anchor.
    let shaped: Vec<Option<Shaped>> = runs
        .iter()
        .map(|r| {
            let fc = face(&coll, r)?;
            let (ids, adv) = glyphs(&fc, r);
            Some((fc, ids, adv))
        })
        .collect();
    let width = |i: usize| -> f32 {
        shaped
            .get(i)
            .and_then(Option::as_ref)
            .map_or(0.0, |s| s.2.iter().sum())
    };
    let mut pens = vec![(0.0f32, 0.0f32); runs.len()];
    let mut i = 0;
    while i < runs.len() {
        let mut j = i + 1;
        while runs.get(j).is_some_and(|r| r.continues) {
            j += 1;
        }
        let total: f32 = (i..j).map(width).sum();
        let Some(head) = runs.get(i) else { break };
        let shift = match head.anchor {
            Anchor::Start => 0.0,
            Anchor::Middle => total / 2.0,
            Anchor::End => total,
        };
        let mut x = head.x - shift;
        for (k, p) in pens.iter_mut().enumerate().take(j).skip(i) {
            *p = (x, head.y);
            x += width(k);
        }
        i = j;
    }
    for ((run, sh), &(x, y)) in runs.iter().zip(&shaped).zip(&pens) {
        let Some((fc, ids, adv)) = sh else { continue };
        if ids.is_empty() || run.size < 1.0 || run.size > 2048.0 || run.color[3] == 0 {
            continue;
        }
        let asc = fc.ascent * run.size / fc.upem;
        let desc = fc.descent * run.size / fc.upem;
        let pad = 2.0 + run.size * 0.25;
        let (x0, y0) = ((x - pad).floor(), (y - asc - pad).floor());
        let bw = (adv.iter().sum::<f32>() + 2.0 * pad).ceil();
        let bh = (asc + desc + 2.0 * pad).ceil();
        // Clip the run box to the bitmap.
        let (cx0, cy0) = (x0.max(0.0), y0.max(0.0));
        let (cx1, cy1) = ((x0 + bw).min(bmp.w as f32), (y0 + bh).min(bmp.h as f32));
        if cx1 <= cx0 || cy1 <= cy0 || bw > 16_384.0 || bh > 4096.0 {
            continue;
        }
        let (bwu, bhu) = (bw as u32, bh as u32);
        // SAFETY: valid interop object; sizes ≥ 1; the glyph buffers outlive DrawGlyphRun; the
        // DIB pointer is valid for `w*h*4` bytes while `brt` lives.
        unsafe {
            let Ok(brt) = gi.CreateBitmapRenderTarget(None, bwu.max(1), bhu.max(1)) else {
                continue;
            };
            let _ = brt.SetPixelsPerDip(1.0);
            let dc = brt.GetMemoryDC();
            let hbm = GetCurrentObject(dc, OBJ_BITMAP);
            let mut ds = DIBSECTION::default();
            if GetObjectW(
                hbm,
                i32::try_from(size_of::<DIBSECTION>()).unwrap_or(0),
                Some((&raw mut ds).cast()),
            ) == 0
                || ds.dsBm.bmBits.is_null()
            {
                continue;
            }
            let (dw, dh) = (
                usize::try_from(ds.dsBm.bmWidth).unwrap_or(0),
                usize::try_from(ds.dsBm.bmHeight.abs()).unwrap_or(0),
            );
            let bits = std::slice::from_raw_parts_mut(ds.dsBm.bmBits.cast::<u8>(), dw * dh * 4);
            bits.fill(0);
            let mut gr = DWRITE_GLYPH_RUN {
                fontFace: std::mem::ManuallyDrop::new(Some(fc.face.clone())),
                fontEmSize: run.size,
                glyphCount: u32::try_from(ids.len()).unwrap_or(0),
                glyphIndices: ids.as_ptr(),
                glyphAdvances: adv.as_ptr(),
                glyphOffsets: std::ptr::null(),
                isSideways: BOOL(0),
                bidiLevel: 0,
            };
            let _ = brt.DrawGlyphRun(
                x - x0,
                y - y0,
                DWRITE_MEASURING_MODE_NATURAL,
                &gr,
                &params,
                COLORREF(0x00ff_ffff),
                None,
            );
            std::mem::ManuallyDrop::drop(&mut gr.fontFace);
            let bits = std::slice::from_raw_parts(ds.dsBm.bmBits.cast::<u8>(), dw * dh * 4);
            composite(bmp, bits, dw, (x0 as i64, y0 as i64), dh, run.color);
        }
    }
}

/// Source-over of `color` through the green-channel coverage of `mask` (BGRX, `mw` wide) at
/// `at` into the premultiplied BGRA bitmap.
fn composite(bmp: &mut Bitmap, mask: &[u8], mw: usize, at: (i64, i64), mh: usize, color: [u8; 4]) {
    let (bw, bh) = (i64::from(bmp.w), i64::from(bmp.h));
    let a = u32::from(color[3]);
    for my in 0..mh {
        let y = at.1 + my as i64;
        if y < 0 || y >= bh {
            continue;
        }
        for mx in 0..mw {
            let x = at.0 + mx as i64;
            if x < 0 || x >= bw {
                continue;
            }
            let cov = u32::from(mask.get((my * mw + mx) * 4 + 1).copied().unwrap_or(0));
            if cov == 0 {
                continue;
            }
            let sa = cov * a / 255;
            let Ok(o) = usize::try_from((y * bw + x) * 4) else {
                continue;
            };
            let Some(d) = bmp.px.get_mut(o..o + 4) else {
                continue;
            };
            let inv = 255 - sa;
            let src = [color[2], color[1], color[0]];
            for c in 0..3 {
                if let (Some(dc), Some(&s)) = (d.get_mut(c), src.get(c)) {
                    *dc = u8::try_from((u32::from(s) * sa + u32::from(*dc) * inv) / 255)
                        .unwrap_or(255);
                }
            }
            if let Some(da) = d.get_mut(3) {
                *da = u8::try_from(sa + u32::from(*da) * inv / 255).unwrap_or(255);
            }
        }
    }
}
