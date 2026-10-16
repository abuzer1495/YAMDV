//! Spike A: DirectWrite → GDI interop (`IDWriteBitmapRenderTarget`) into a banded 32-bpp DIB,
//! shapes/images via tiny-skia on the same DIB bits, color emoji via `TranslateColorGlyphRun`,
//! `BitBlt` to the window, scrolling via `ScrollWindowEx` (§4.7 candidate A).

use windows::Win32::Foundation::{COLORREF, DWRITE_E_NOCOLOR, HWND, RECT};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_GLYPH_IMAGE_FORMATS_CFF, DWRITE_GLYPH_IMAGE_FORMATS_COLR,
    DWRITE_GLYPH_IMAGE_FORMATS_TRUETYPE, DWRITE_GLYPH_RUN, DWRITE_GLYPH_RUN_DESCRIPTION,
    DWRITE_MATRIX, DWRITE_MEASURING_MODE, DWRITE_STRIKETHROUGH, DWRITE_UNDERLINE,
    IDWriteBitmapRenderTarget, IDWriteFactory4, IDWriteInlineObject, IDWritePixelSnapping_Impl,
    IDWriteRenderingParams, IDWriteTextRenderer, IDWriteTextRenderer_Impl,
};
use windows::Win32::Graphics::Gdi::{
    BitBlt, DIBSECTION, GdiFlush, GetCurrentObject, GetObjectW, HDC, MONITOR_DEFAULTTONEAREST,
    MonitorFromWindow, OBJ_BITMAP, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE;
use windows::core::{BOOL, IUnknown, Interface, Ref, implement};
use windows_numerics::Vector2;

use tiny_skia::{
    Color, FillRule, Paint, PathBuilder, PixmapMut, PixmapPaint, PixmapRef, Rect, Stroke, Transform,
};

use crate::app::{Log, ScrollMode};
use crate::scene::{self, Scene, Text};

const BAND: i32 = 128;

/// Per-`Draw` context handed to the renderer through `clientDrawingContext`.
struct DrawCtx<'a> {
    brt: &'a IDWriteBitmapRenderTarget,
    params: &'a IDWriteRenderingParams,
    f4: Option<&'a IDWriteFactory4>,
    text: &'a Text,
}

fn colorref(c: u32) -> COLORREF {
    COLORREF(((c >> 16) & 0xFF) | (c & 0xFF00) | ((c & 0xFF) << 16))
}

#[implement(IDWriteTextRenderer)]
struct Renderer {
    scale: f32,
}

impl IDWritePixelSnapping_Impl for Renderer_Impl {
    fn IsPixelSnappingDisabled(&self, _c: *const core::ffi::c_void) -> windows::core::Result<BOOL> {
        Ok(BOOL(0))
    }
    fn GetCurrentTransform(
        &self,
        _c: *const core::ffi::c_void,
        t: *mut DWRITE_MATRIX,
    ) -> windows::core::Result<()> {
        unsafe {
            *t = DWRITE_MATRIX {
                m11: self.scale,
                m12: 0.0,
                m21: 0.0,
                m22: self.scale,
                dx: 0.0,
                dy: 0.0,
            };
        }
        Ok(())
    }
    fn GetPixelsPerDip(&self, _c: *const core::ffi::c_void) -> windows::core::Result<f32> {
        Ok(self.scale)
    }
}

impl IDWriteTextRenderer_Impl for Renderer_Impl {
    fn DrawGlyphRun(
        &self,
        c: *const core::ffi::c_void,
        x: f32,
        y: f32,
        mode: DWRITE_MEASURING_MODE,
        run: *const DWRITE_GLYPH_RUN,
        desc: *const DWRITE_GLYPH_RUN_DESCRIPTION,
        _effect: Ref<IUnknown>,
    ) -> windows::core::Result<()> {
        unsafe {
            let ctx = &*c.cast::<DrawCtx>();
            let pos = desc.as_ref().map_or(0, |d| d.textPosition);
            let color = colorref(ctx.text.color_at(pos));
            if let Some(f4) = ctx.f4 {
                match f4.TranslateColorGlyphRun(
                    Vector2 { X: x, Y: y },
                    run,
                    Some(desc),
                    DWRITE_GLYPH_IMAGE_FORMATS_TRUETYPE
                        | DWRITE_GLYPH_IMAGE_FORMATS_CFF
                        | DWRITE_GLYPH_IMAGE_FORMATS_COLR,
                    mode,
                    None,
                    0,
                ) {
                    Ok(en) => {
                        while en.MoveNext()?.as_bool() {
                            let cr = &*en.GetCurrentRun()?;
                            let b = &cr.Base;
                            let col = if b.paletteIndex == 0xFFFF {
                                color
                            } else {
                                let rc = b.runColor;
                                COLORREF(
                                    (rc.r * 255.0) as u32
                                        | (((rc.g * 255.0) as u32) << 8)
                                        | (((rc.b * 255.0) as u32) << 16),
                                )
                            };
                            ctx.brt.DrawGlyphRun(
                                b.baselineOriginX,
                                b.baselineOriginY,
                                cr.measuringMode,
                                &b.glyphRun,
                                ctx.params,
                                col,
                                None,
                            )?;
                        }
                        return Ok(());
                    }
                    Err(e) if e.code() == DWRITE_E_NOCOLOR => {}
                    Err(e) => return Err(e),
                }
            }
            ctx.brt
                .DrawGlyphRun(x, y, mode, run, ctx.params, color, None)
        }
    }
    fn DrawUnderline(
        &self,
        _: *const core::ffi::c_void,
        _: f32,
        _: f32,
        _: *const DWRITE_UNDERLINE,
        _: Ref<IUnknown>,
    ) -> windows::core::Result<()> {
        Ok(())
    }
    fn DrawStrikethrough(
        &self,
        _: *const core::ffi::c_void,
        _: f32,
        _: f32,
        _: *const DWRITE_STRIKETHROUGH,
        _: Ref<IUnknown>,
    ) -> windows::core::Result<()> {
        Ok(())
    }
    fn DrawInlineObject(
        &self,
        _: *const core::ffi::c_void,
        _: f32,
        _: f32,
        _: Ref<IDWriteInlineObject>,
        _: BOOL,
        _: BOOL,
        _: Ref<IUnknown>,
    ) -> windows::core::Result<()> {
        Ok(())
    }
}

// The DIB is BGRA; tiny-skia writes RGBA. Swap R/B in every color so bytes land as BGRA.
fn sk(c: u32, a: f32) -> Color {
    Color::from_rgba(
        (c & 0xFF) as f32 / 255.0,
        ((c >> 8) & 0xFF) as f32 / 255.0,
        ((c >> 16) & 0xFF) as f32 / 255.0,
        a,
    )
    .unwrap_or(Color::BLACK)
}

fn paint(c: u32, a: f32) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(sk(c, a));
    p.anti_alias = true;
    p
}

pub struct BackendA {
    brt: IDWriteBitmapRenderTarget,
    params: IDWriteRenderingParams,
    f4: Option<IDWriteFactory4>,
    renderer: IDWriteTextRenderer,
    hwnd: HWND,
    w: i32,
    scale: f32,
}

impl BackendA {
    fn make_target(s: &Scene, w: i32) -> windows::core::Result<IDWriteBitmapRenderTarget> {
        unsafe {
            let gi = s.dw.GetGdiInterop()?;
            let brt = gi.CreateBitmapRenderTarget(None, w.max(1) as u32, BAND as u32)?;
            brt.SetPixelsPerDip(s.scale)?;
            Ok(brt)
        }
    }

    fn bits(&self) -> Option<(&'static mut [u8], i32, i32)> {
        unsafe {
            let dc = self.brt.GetMemoryDC();
            let hbm = GetCurrentObject(dc, OBJ_BITMAP);
            let mut ds = DIBSECTION::default();
            if GetObjectW(
                hbm,
                size_of::<DIBSECTION>() as i32,
                Some((&raw mut ds).cast()),
            ) == 0
                || ds.dsBm.bmBits.is_null()
            {
                return None;
            }
            let (w, h) = (ds.dsBm.bmWidth, ds.dsBm.bmHeight.abs());
            let len = (ds.dsBm.bmWidthBytes * h) as usize;
            Some((
                std::slice::from_raw_parts_mut(ds.dsBm.bmBits.cast::<u8>(), len),
                w,
                h,
            ))
        }
    }

    /// Draws the rows `[top, top + rows)` (global device px) into the band.
    fn draw_band(&self, s: &Scene, top: f32, rows: i32) {
        let Some((buf, w, h)) = self.bits() else {
            return;
        };
        let Some(mut pm) = PixmapMut::from_bytes(buf, w as u32, h as u32) else {
            return;
        };
        pm.fill(sk(scene::BG, 1.0));
        let img = PixmapRef::from_bytes(&s.img, s.img_w, s.img_h);
        let vis = s.visible(top, top + rows as f32);
        for i in vis.clone() {
            let off = i as f32 * s.inst_px() - top;
            let t = Transform::from_row(s.scale, 0.0, 0.0, s.scale, 0.0, off);
            draw_shapes(&mut pm, s, t);
            if let Some(img) = img {
                let x = ((scene::X0 + scene::IMG_X) * s.scale).round() as i32;
                let y = (s.prim_y * s.scale + off).round() as i32;
                pm.draw_pixmap(
                    x,
                    y,
                    img,
                    &PixmapPaint::default(),
                    Transform::identity(),
                    None,
                );
            }
        }
        unsafe {
            let _ = GdiFlush();
            for i in vis {
                let off_dip = (i as f32 * s.inst_px() - top) / s.scale;
                for t in &s.texts {
                    let ctx = DrawCtx {
                        brt: &self.brt,
                        params: &self.params,
                        f4: self.f4.as_ref(),
                        text: t,
                    };
                    let _ = t.layout.Draw(
                        Some((&raw const ctx).cast()),
                        &self.renderer,
                        t.x,
                        t.y + off_dip,
                    );
                }
            }
        }
    }
}

fn draw_shapes(pm: &mut PixmapMut, s: &Scene, t: Transform) {
    let rect = |x: f32, y: f32, w: f32, h: f32| Rect::from_xywh(x, y, w, h);
    for r in &s.selection {
        if let Some(rc) = rect(r.x, r.y, r.w, r.h) {
            pm.fill_rect(rc, &paint(scene::SEL, scene::SEL_ALPHA), t, None);
        }
    }
    let c = s.code_box;
    if let Some(rc) = rect(c.x + 0.5, c.y + 0.5, c.w - 1.0, c.h - 1.0)
        && let Some(path) = rounded(rc, 8.0)
    {
        pm.fill_path(
            &path,
            &paint(scene::CODE_BG, 1.0),
            FillRule::Winding,
            t,
            None,
        );
        pm.stroke_path(
            &path,
            &paint(scene::CODE_BORDER, 1.0),
            &Stroke {
                width: 1.0,
                ..Default::default()
            },
            t,
            None,
        );
    }
    let tb = s.table;
    if let Some(rc) = rect(tb.x, tb.y, tb.w, scene::TABLE_ROW_H) {
        pm.fill_rect(rc, &paint(scene::CODE_BG, 1.0), t, None);
    }
    let mut pb = PathBuilder::new();
    for row in 0..=3 {
        let y = tb.y + row as f32 * scene::TABLE_ROW_H + 0.5;
        pb.move_to(tb.x, y);
        pb.line_to(tb.x + tb.w, y);
    }
    for col in 0..=4 {
        let x = tb.x + col as f32 * scene::TABLE_COL_W + 0.5;
        pb.move_to(x, tb.y);
        pb.line_to(x, tb.y + tb.h);
    }
    if let Some(p) = pb.finish() {
        pm.stroke_path(
            &p,
            &paint(scene::RULE, 1.0),
            &Stroke {
                width: 1.0,
                ..Default::default()
            },
            t,
            None,
        );
    }
    let (x0, py) = (scene::X0, s.prim_y);
    if let Some(rc) = rect(x0, py, 3.0, scene::PRIM_H) {
        pm.fill_rect(rc, &paint(scene::RULE, 1.0), t, None);
    }
    if let Some(p) = PathBuilder::from_circle(x0 + 40.0, py + 20.0, 6.0) {
        pm.fill_path(&p, &paint(scene::LINK, 1.0), FillRule::Winding, t, None);
    }
    let s15 = Stroke {
        width: 1.5,
        ..Default::default()
    };
    let mut pb = PathBuilder::new();
    pb.move_to(x0 + 20.0, py + 40.0);
    pb.line_to(x0 + 200.0, py + 140.0);
    if let Some(p) = pb.finish() {
        pm.stroke_path(&p, &paint(scene::TEXT, 1.0), &s15, t, None);
    }
    let mut pb = PathBuilder::new();
    pb.move_to(x0 + 200.0, py + 140.0);
    pb.line_to(x0 + 186.0, py + 137.0);
    pb.line_to(x0 + 196.0, py + 127.0);
    pb.close();
    if let Some(p) = pb.finish() {
        pm.fill_path(&p, &paint(scene::TEXT, 1.0), FillRule::Winding, t, None);
    }
    let mut pb = PathBuilder::new();
    pb.move_to(x0 + 240.0, py + 130.0);
    pb.cubic_to(
        x0 + 280.0,
        py + 10.0,
        x0 + 380.0,
        py + 150.0,
        x0 + 460.0,
        py + 30.0,
    );
    if let Some(p) = pb.finish() {
        pm.stroke_path(&p, &paint(scene::LINK, 1.0), &s15, t, None);
    }
}

fn rounded(r: Rect, rad: f32) -> Option<tiny_skia::Path> {
    let (l, t, rr, b) = (r.left(), r.top(), r.right(), r.bottom());
    let k = rad * 0.552_284_8;
    let mut pb = PathBuilder::new();
    pb.move_to(l + rad, t);
    pb.line_to(rr - rad, t);
    pb.cubic_to(rr - rad + k, t, rr, t + rad - k, rr, t + rad);
    pb.line_to(rr, b - rad);
    pb.cubic_to(rr, b - rad + k, rr - rad + k, b, rr - rad, b);
    pb.line_to(l + rad, b);
    pb.cubic_to(l + rad - k, b, l, b - rad + k, l, b - rad);
    pb.line_to(l, t + rad);
    pb.cubic_to(l, t + rad - k, l + rad - k, t, l + rad, t);
    pb.close();
    pb.finish()
}

impl crate::app::Backend for BackendA {
    const NAME: &'static str = "A (DWrite GDI interop + tiny-skia)";
    const MODE: ScrollMode = ScrollMode::Blit;
    const EX_STYLE: WINDOW_EX_STYLE = WINDOW_EX_STYLE(0);

    fn create(
        hwnd: HWND,
        s: &Scene,
        w: u32,
        _h: u32,
        log: &mut Log,
    ) -> windows::core::Result<Self> {
        unsafe {
            let brt = Self::make_target(s, w as i32)?;
            let params = s
                .dw
                .CreateMonitorRenderingParams(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST))?;
            let f4 = s.dw.cast::<IDWriteFactory4>().ok();
            log.kv("dwrite.factory4", f4.is_some());
            log.pws("gdi_interop");
            let renderer: IDWriteTextRenderer = Renderer { scale: s.scale }.into();
            Ok(Self {
                brt,
                params,
                f4,
                renderer,
                hwnd,
                w: w as i32,
                scale: s.scale,
            })
        }
    }

    fn resize(&mut self, w: u32, _h: u32) {
        let w = w as i32;
        if w > 0 && w != self.w {
            unsafe {
                if self.brt.Resize(w as u32, BAND as u32).is_ok() {
                    self.w = w;
                }
            }
        }
    }

    fn paint(&mut self, s: &Scene, scroll: i32, rc: RECT, hdc: HDC) {
        let _ = (self.hwnd, self.scale);
        let mut y = rc.top;
        while y < rc.bottom {
            let bh = BAND.min(rc.bottom - y);
            self.draw_band(s, (scroll + y) as f32, bh);
            unsafe {
                let _ = BitBlt(
                    hdc,
                    0,
                    y,
                    self.w,
                    bh,
                    Some(self.brt.GetMemoryDC()),
                    0,
                    0,
                    SRCCOPY,
                );
            }
            y += bh;
        }
    }
}
