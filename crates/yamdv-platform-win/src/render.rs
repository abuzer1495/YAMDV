//! Backend A (ADR-0002), moved from `spikes/src/backend_a.rs` and cleaned up: DirectWrite GDI
//! interop (`IDWriteBitmapRenderTarget`) renders glyph runs into a reused 128-px band DIB,
//! tiny-skia draws shapes on the same bits, color emoji via `TranslateColorGlyphRun` (R-REN-20),
//! each band is `BitBlt`-ed to the window. Scrolling uses `ScrollWindowEx` + exposed-strip painting
//! (done by the window module).
#![allow(unsafe_code)]
// Pixel/DIP geometry: float<->int conversions are bounded by window/bitmap sizes (ADR-0007).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::indexing_slicing
)]

use std::cell::Cell;

use tiny_skia::{
    Color, FillRule, GradientStop, LinearGradient, Paint, PathBuilder, PixmapMut, Point,
    SpreadMode, Stroke, Transform,
};
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
use windows::core::{BOOL, IUnknown, Interface, Ref, implement};
use windows_numerics::Vector2;
use yamdv_layout::Rect;
use yamdv_layout::shaper::Shaped;

use crate::dwrite::{DWrite, DwShaped};

/// `0xRRGGBB` → GDI `COLORREF` (`0x00BBGGRR`).
pub fn colorref(c: u32) -> COLORREF {
    COLORREF(((c >> 16) & 0xFF) | (c & 0xFF00) | ((c & 0xFF) << 16))
}

// The DIB is BGRA while tiny-skia writes RGBA: swap R/B so the bytes land as BGRA.
fn sk(c: u32, a: f32) -> Color {
    Color::from_rgba(
        (c & 0xFF) as f32 / 255.0,
        ((c >> 8) & 0xFF) as f32 / 255.0,
        ((c >> 16) & 0xFF) as f32 / 255.0,
        a.clamp(0.0, 1.0),
    )
    .unwrap_or(Color::BLACK)
}

fn paint(c: u32, a: f32) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(sk(c, a));
    p.anti_alias = true;
    p
}

fn rounded(r: tiny_skia::Rect, rad: f32) -> Option<tiny_skia::Path> {
    if rad <= 0.0 {
        return Some(PathBuilder::from_rect(r));
    }
    let rad = rad.min(r.width() / 2.0).min(r.height() / 2.0);
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

/// Per-`Draw` context handed to the text renderer through `clientDrawingContext`.
struct DrawCtx<'a> {
    band: &'a Band<'a>,
    shaped: &'a DwShaped,
}

#[implement(IDWriteTextRenderer)]
struct TextRenderer {
    scale: Cell<f32>,
}

impl IDWritePixelSnapping_Impl for TextRenderer_Impl {
    fn IsPixelSnappingDisabled(&self, _c: *const core::ffi::c_void) -> windows::core::Result<BOOL> {
        Ok(BOOL(0))
    }
    fn GetCurrentTransform(
        &self,
        _c: *const core::ffi::c_void,
        t: *mut DWRITE_MATRIX,
    ) -> windows::core::Result<()> {
        let s = self.scale.get();
        // SAFETY: DirectWrite passes a valid out-pointer.
        unsafe {
            *t = DWRITE_MATRIX {
                m11: s,
                m12: 0.0,
                m21: 0.0,
                m22: s,
                dx: 0.0,
                dy: 0.0,
            }
        };
        Ok(())
    }
    fn GetPixelsPerDip(&self, _c: *const core::ffi::c_void) -> windows::core::Result<f32> {
        Ok(self.scale.get())
    }
}

impl IDWriteTextRenderer_Impl for TextRenderer_Impl {
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
        // SAFETY: `c` is the DrawCtx passed to IDWriteTextLayout::Draw by `Band::text`, alive for
        // the duration of the call; run/desc are valid for this callback.
        unsafe {
            let ctx = &*c.cast::<DrawCtx>();
            let band = ctx.band;
            band.gdi_begin();
            let pos = desc.as_ref().map_or(0, |d| d.textPosition);
            let color = colorref(ctx.shaped.style_at(pos).color);
            if let Some(f4) = band.r.f4.as_ref() {
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
                            band.r.brt.DrawGlyphRun(
                                b.baselineOriginX,
                                b.baselineOriginY,
                                cr.measuringMode,
                                &b.glyphRun,
                                &band.r.params,
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
            band.r
                .brt
                .DrawGlyphRun(x, y, mode, run, &band.r.params, color, None)
        }
    }
    fn DrawUnderline(
        &self,
        c: *const core::ffi::c_void,
        x: f32,
        y: f32,
        u: *const DWRITE_UNDERLINE,
        _: Ref<IUnknown>,
    ) -> windows::core::Result<()> {
        // SAFETY: see DrawGlyphRun; `u` is valid for this callback.
        unsafe {
            let ctx = &*c.cast::<DrawCtx>();
            if let Some(u) = u.as_ref() {
                ctx.band.decoration(
                    x,
                    y + u.offset,
                    u.width,
                    u.thickness.max(1.0 / ctx.band.scale),
                    ctx.shaped.base.color,
                );
            }
        }
        Ok(())
    }
    fn DrawStrikethrough(
        &self,
        c: *const core::ffi::c_void,
        x: f32,
        y: f32,
        s: *const DWRITE_STRIKETHROUGH,
        _: Ref<IUnknown>,
    ) -> windows::core::Result<()> {
        // SAFETY: see DrawGlyphRun; `s` is valid for this callback. B.6.2: strikethrough in `text`.
        unsafe {
            let ctx = &*c.cast::<DrawCtx>();
            if let Some(s) = s.as_ref() {
                ctx.band.decoration(
                    x,
                    y + s.offset,
                    s.width,
                    s.thickness.max(1.0 / ctx.band.scale),
                    ctx.band.text_color,
                );
            }
        }
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

/// Band renderer (one per window).
pub struct Renderer {
    brt: IDWriteBitmapRenderTarget,
    params: IDWriteRenderingParams,
    f4: Option<IDWriteFactory4>,
    text: IDWriteTextRenderer,
    scale_cell: windows::core::ComObject<TextRenderer>,
    w: i32,
    scale: f32,
}

impl std::fmt::Debug for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Renderer({}px @ {})", self.w, self.scale)
    }
}

/// Band buffer bytes (for `mem-stats`).
pub fn band_bytes(w: i32) -> usize {
    usize::try_from(w.max(1)).unwrap_or(0) * usize::try_from(band_rows(w)).unwrap_or(0) * 4
}

/// Band height for a width (P-12).
pub fn band_rows(w: i32) -> i32 {
    yamdv_layout::geom::band_rows(w)
}

impl Renderer {
    /// Creates the renderer. `hwnd` selects the monitor rendering parameters (ClearType per the
    /// user's settings, B.2); `None` = default parameters (headless rendering).
    pub fn new(
        dw: &DWrite,
        hwnd: Option<HWND>,
        w_px: i32,
        scale: f32,
    ) -> windows::core::Result<Self> {
        // SAFETY: valid factory; sizes are clamped to ≥ 1.
        unsafe {
            let gi = dw.factory.GetGdiInterop()?;
            let brt = gi.CreateBitmapRenderTarget(
                None,
                u32::try_from(w_px.max(1)).unwrap_or(1),
                u32::try_from(band_rows(w_px)).unwrap_or(32),
            )?;
            brt.SetPixelsPerDip(scale)?;
            let params = match hwnd {
                Some(h) => dw
                    .factory
                    .CreateMonitorRenderingParams(MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST))?,
                None => dw.factory.CreateRenderingParams()?,
            };
            let f4 = dw.factory.cast::<IDWriteFactory4>().ok();
            let scale_cell = windows::core::ComObject::new(TextRenderer {
                scale: Cell::new(scale),
            });
            let text: IDWriteTextRenderer = scale_cell.to_interface();
            Ok(Self {
                brt,
                params,
                f4,
                text,
                scale_cell,
                w: w_px.max(1),
                scale,
            })
        }
    }

    /// Width in device pixels.
    pub fn width(&self) -> i32 {
        self.w
    }

    /// Resizes the band (width only).
    pub fn resize(&mut self, w_px: i32) {
        if w_px > 0 && w_px != self.w {
            // SAFETY: valid render target.
            if unsafe {
                self.brt.Resize(
                    u32::try_from(w_px).unwrap_or(1),
                    u32::try_from(band_rows(w_px)).unwrap_or(32),
                )
            }
            .is_ok()
            {
                self.w = w_px;
            }
        }
    }

    /// Changes the DPI scale (WM_DPICHANGED) and refreshes the monitor rendering parameters.
    pub fn set_scale(&mut self, dw: &DWrite, hwnd: HWND, scale: f32) {
        self.scale = scale;
        self.scale_cell.scale.set(scale);
        // SAFETY: valid objects.
        unsafe {
            let _ = self.brt.SetPixelsPerDip(scale);
            if let Ok(p) = dw
                .factory
                .CreateMonitorRenderingParams(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST))
            {
                self.params = p;
            }
        }
    }

    fn bits(&self) -> Option<(*mut u8, i32, i32)> {
        // SAFETY: the memory DC of the render target holds a DIB section; GetObjectW fills `ds`.
        unsafe {
            let dc = self.brt.GetMemoryDC();
            let hbm = GetCurrentObject(dc, OBJ_BITMAP);
            let mut ds = DIBSECTION::default();
            if GetObjectW(
                hbm,
                i32::try_from(size_of::<DIBSECTION>()).unwrap_or(0),
                Some((&raw mut ds).cast()),
            ) == 0
                || ds.dsBm.bmBits.is_null()
            {
                return None;
            }
            Some((
                ds.dsBm.bmBits.cast::<u8>(),
                ds.dsBm.bmWidth,
                ds.dsBm.bmHeight.abs(),
            ))
        }
    }

    /// Paints `rc` (device px) band by band: `draw` gets a [`Band`] whose coordinate system is
    /// client DIPs; each band is then blitted to `hdc`.
    pub fn paint(&self, hdc: HDC, rc: RECT, text_color: u32, draw: &mut dyn FnMut(&Band<'_>)) {
        let Some((bits, w, h)) = self.bits() else {
            return;
        };
        let mut y = rc.top.max(0);
        while y < rc.bottom {
            let bh = band_rows(self.w).min(rc.bottom - y);
            let band = Band {
                r: self,
                bits,
                w,
                h,
                top_px: y,
                rows: bh,
                scale: self.scale,
                gdi: Cell::new(false),
                text_color,
            };
            draw(&band);
            band.cpu_begin();
            // SAFETY: valid DCs; the band holds `bh` valid rows.
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

    /// Renders `h_px` rows offscreen into a top-down BGRA buffer (`--render-png`).
    pub fn render_offscreen(
        &self,
        h_px: i32,
        text_color: u32,
        draw: &mut dyn FnMut(&Band<'_>),
    ) -> Vec<u8> {
        let Some((bits, w, h)) = self.bits() else {
            return Vec::new();
        };
        let stride = usize::try_from(w).unwrap_or(0) * 4;
        let mut out = vec![0u8; stride * usize::try_from(h_px.max(0)).unwrap_or(0)];
        let mut y = 0;
        while y < h_px {
            let bh = band_rows(self.w).min(h_px - y);
            let band = Band {
                r: self,
                bits,
                w,
                h,
                top_px: y,
                rows: bh,
                scale: self.scale,
                gdi: Cell::new(false),
                text_color,
            };
            draw(&band);
            band.cpu_begin();
            let rows = usize::try_from(bh).unwrap_or(0);
            let start = usize::try_from(y).unwrap_or(0) * stride;
            // SAFETY: the DIB holds `h ≥ bh` rows of `stride` bytes.
            let src = unsafe { std::slice::from_raw_parts(bits, stride * rows) };
            if let Some(dst) = out.get_mut(start..start + stride * rows) {
                dst.copy_from_slice(src);
            }
            y += bh;
        }
        out
    }
}

/// One band being painted. Coordinates are client DIPs.
pub struct Band<'a> {
    r: &'a Renderer,
    bits: *mut u8,
    w: i32,
    h: i32,
    top_px: i32,
    rows: i32,
    /// DPI scale.
    pub scale: f32,
    gdi: Cell<bool>,
    text_color: u32,
}

impl Band<'_> {
    /// DIP range covered by this band.
    pub fn dip_range(&self) -> (f32, f32) {
        (
            self.top_px as f32 / self.scale,
            (self.top_px + self.rows) as f32 / self.scale,
        )
    }

    /// Does the DIP rectangle intersect the band?
    pub fn visible(&self, y: f32, h: f32) -> bool {
        let (a, b) = self.dip_range();
        y < b && y + h > a
    }

    fn gdi_begin(&self) {
        self.gdi.set(true);
    }

    fn cpu_begin(&self) {
        if self.gdi.replace(false) {
            // SAFETY: flushes pending GDI work before the CPU touches the DIB bits.
            unsafe {
                let _ = GdiFlush();
            }
        }
    }

    fn with_pixmap(&self, f: impl FnOnce(&mut PixmapMut<'_>, Transform)) {
        self.cpu_begin();
        let len = usize::try_from(self.w).unwrap_or(0) * usize::try_from(self.h).unwrap_or(0) * 4;
        // SAFETY: `bits` points to the render target's DIB of w×h 32-bpp pixels; no other
        // reference to it exists while `f` runs (GDI work was flushed above).
        let buf = unsafe { std::slice::from_raw_parts_mut(self.bits, len) };
        let Some(mut pm) = PixmapMut::from_bytes(
            buf,
            u32::try_from(self.w).unwrap_or(0),
            u32::try_from(self.h).unwrap_or(0),
        ) else {
            return;
        };
        let t = Transform::from_row(self.scale, 0.0, 0.0, self.scale, 0.0, -(self.top_px as f32));
        f(&mut pm, t);
    }

    /// Fills the whole band.
    pub fn clear(&self, color: u32) {
        self.with_pixmap(|pm, _| pm.fill(sk(color, 1.0)));
    }

    /// Fills a (rounded) rectangle.
    pub fn fill(&self, r: Rect, color: u32, alpha: f32, radius: f32) {
        if !self.visible(r.y, r.h) || r.w <= 0.0 || r.h <= 0.0 {
            return;
        }
        self.with_pixmap(|pm, t| {
            if let Some(rc) = tiny_skia::Rect::from_xywh(r.x, r.y, r.w, r.h)
                && let Some(p) = rounded(rc, radius)
            {
                pm.fill_path(&p, &paint(color, alpha), FillRule::Winding, t, None);
            }
        });
    }

    /// Strokes a (rounded) rectangle with a line of `width` DIPs inside its bounds.
    pub fn stroke(&self, r: Rect, color: u32, radius: f32, width: f32) {
        if !self.visible(r.y, r.h) {
            return;
        }
        self.with_pixmap(|pm, t| {
            let half = width / 2.0;
            if let Some(rc) =
                tiny_skia::Rect::from_xywh(r.x + half, r.y + half, r.w - width, r.h - width)
                && let Some(p) = rounded(rc, (radius - half).max(0.0))
            {
                pm.stroke_path(
                    &p,
                    &paint(color, 1.0),
                    &Stroke {
                        width,
                        ..Default::default()
                    },
                    t,
                    None,
                );
            }
        });
    }

    /// Filled circle or ring.
    pub fn circle(&self, cx: f32, cy: f32, r: f32, color: u32, ring: Option<f32>) {
        if !self.visible(cy - r, 2.0 * r) {
            return;
        }
        self.with_pixmap(|pm, t| match ring {
            None => {
                if let Some(p) = PathBuilder::from_circle(cx, cy, r) {
                    pm.fill_path(&p, &paint(color, 1.0), FillRule::Winding, t, None);
                }
            }
            Some(w) => {
                if let Some(p) = PathBuilder::from_circle(cx, cy, r - w / 2.0) {
                    pm.stroke_path(
                        &p,
                        &paint(color, 1.0),
                        &Stroke {
                            width: w,
                            ..Default::default()
                        },
                        t,
                        None,
                    );
                }
            }
        });
    }

    /// Horizontal fade from transparent to `color` (left → right).
    pub fn fade_right(&self, r: Rect, color: u32) {
        if !self.visible(r.y, r.h) {
            return;
        }
        self.with_pixmap(|pm, t| {
            let Some(shader) = LinearGradient::new(
                Point::from_xy(r.x, 0.0),
                Point::from_xy(r.x + r.w, 0.0),
                vec![
                    GradientStop::new(0.0, sk(color, 0.0)),
                    GradientStop::new(1.0, sk(color, 1.0)),
                ],
                SpreadMode::Pad,
                Transform::identity(),
            ) else {
                return;
            };
            let p = Paint {
                shader,
                anti_alias: true,
                ..Default::default()
            };
            if let Some(rc) = tiny_skia::Rect::from_xywh(r.x, r.y, r.w, r.h) {
                pm.fill_rect(rc, &p, t, None);
            }
        });
    }

    fn decoration(&self, x: f32, y_band_dip: f32, w: f32, th: f32, color: u32) {
        // Called from text callbacks: coordinates are already band-relative DIPs.
        let top = self.top_px as f32 / self.scale;
        self.fill(Rect::new(x, y_band_dip + top, w, th), color, 1.0, 0.0);
        self.gdi_begin();
    }

    /// Draws a shaped paragraph with its origin at (x, y).
    pub fn text(&self, shaped: &dyn Shaped, x: f32, y: f32) {
        if !self.visible(y, shaped.height()) {
            return;
        }
        let Some(dw) = shaped.as_any().downcast_ref::<DwShaped>() else {
            return;
        };
        let Some(layout) = &dw.layout else { return };
        self.cpu_begin();
        let ctx = DrawCtx {
            band: self,
            shaped: dw,
        };
        let top = self.top_px as f32 / self.scale;
        // SAFETY: valid layout and renderer; `ctx` outlives the synchronous Draw call.
        unsafe {
            let _ = layout.Draw(Some((&raw const ctx).cast()), &self.r.text, x, y - top);
        }
        self.gdi_begin();
    }
}
