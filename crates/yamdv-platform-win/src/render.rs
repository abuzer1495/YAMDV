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

use std::cell::{Cell, RefCell};

use tiny_skia::{
    Color, FillRule, GradientStop, LinearGradient, Mask, Paint, PathBuilder, PixmapMut, Point,
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
                clips: RefCell::new(Vec::new()),
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
                clips: RefCell::new(Vec::new()),
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
    /// Clip stack (P-19): effective clip rectangle (client DIPs) and its band mask.
    clips: RefCell<Vec<(Rect, Option<Mask>)>>,
}

impl Band<'_> {
    /// Pushes a clip rectangle (intersected with the current clip). Everything drawn until the
    /// matching [`Band::pop_clip`] — shapes, glyph runs, images — stays inside it.
    pub fn push_clip(&self, r: Rect) {
        let eff = match self.clip() {
            Some(c) => c.intersect(&r),
            None => r,
        };
        let mut mask = Mask::new(self.w as u32, self.h as u32);
        if let Some(m) = mask.as_mut()
            && let Some(rc) =
                tiny_skia::Rect::from_xywh(eff.x, eff.y, eff.w.max(0.0), eff.h.max(0.0))
        {
            let t =
                Transform::from_row(self.scale, 0.0, 0.0, self.scale, 0.0, -(self.top_px as f32));
            m.fill_path(&PathBuilder::from_rect(rc), FillRule::Winding, false, t);
        }
        self.clips.borrow_mut().push((eff, mask));
    }

    /// Pops the innermost clip.
    pub fn pop_clip(&self) {
        self.clips.borrow_mut().pop();
    }

    /// Current effective clip (client DIPs).
    pub fn clip(&self) -> Option<Rect> {
        self.clips.borrow().last().map(|c| c.0)
    }

    /// False when `r` lies completely outside the current clip.
    fn clip_allows(&self, r: &Rect) -> bool {
        self.clip().is_none_or(|c| c.overlaps(r))
    }

    /// Runs a tiny-skia draw with the current clip mask (if any).
    fn with_clipped(&self, f: impl FnOnce(&mut PixmapMut<'_>, Transform, Option<&Mask>)) {
        let clips = self.clips.borrow();
        let mask = clips.last().and_then(|c| c.1.as_ref());
        self.with_pixmap(|pm, t| f(pm, t, mask));
    }
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
        if !self.visible(r.y, r.h) || r.w <= 0.0 || r.h <= 0.0 || !self.clip_allows(&r) {
            return;
        }
        self.with_clipped(|pm, t, m| {
            if let Some(rc) = tiny_skia::Rect::from_xywh(r.x, r.y, r.w, r.h)
                && let Some(p) = rounded(rc, radius)
            {
                pm.fill_path(&p, &paint(color, alpha), FillRule::Winding, t, m);
            }
        });
    }

    /// Draws premultiplied pixels in band byte order (BGRA, as WIC `32bppPBGRA` delivers them)
    /// with their top-left at DIP `(x, y)`, one source pixel per device pixel (R-IMG-04).
    pub fn image(&self, x: f32, y: f32, w: u32, h: u32, px: &[u8]) {
        let (wd, hd) = (w as f32 / self.scale, h as f32 / self.scale);
        let r = Rect::new(x, y, wd, hd);
        if !self.visible(y, hd) || !self.clip_allows(&r) {
            return;
        }
        let Some(src) = tiny_skia::PixmapRef::from_bytes(px, w, h) else {
            return;
        };
        self.with_clipped(|pm, t, m| {
            let (dx, dy) = (
                (x * t.sx + t.tx).round() as i32,
                (y * t.sy + t.ty).round() as i32,
            );
            pm.draw_pixmap(
                dx,
                dy,
                src,
                &tiny_skia::PixmapPaint::default(),
                Transform::identity(),
                m,
            );
        });
    }

    /// Strokes a (rounded) rectangle with a line of `width` DIPs inside its bounds.
    pub fn stroke(&self, r: Rect, color: u32, radius: f32, width: f32) {
        if !self.visible(r.y, r.h) || !self.clip_allows(&r) {
            return;
        }
        self.with_clipped(|pm, t, m| {
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
                    m,
                );
            }
        });
    }

    /// Stroked polyline (vector icons, check marks, chevrons); `closed` joins the last point.
    pub fn polyline(&self, pts: &[(f32, f32)], color: u32, width: f32, closed: bool) {
        let (Some(x0), Some(x1), Some(y0), Some(y1)) = (
            pts.iter().map(|p| p.0).reduce(f32::min),
            pts.iter().map(|p| p.0).reduce(f32::max),
            pts.iter().map(|p| p.1).reduce(f32::min),
            pts.iter().map(|p| p.1).reduce(f32::max),
        ) else {
            return;
        };
        let bb = Rect::new(
            x0 - width,
            y0 - width,
            x1 - x0 + 2.0 * width,
            y1 - y0 + 2.0 * width,
        );
        if !self.visible(bb.y, bb.h) || !self.clip_allows(&bb) {
            return;
        }
        self.with_clipped(|pm, t, m| {
            let mut pb = PathBuilder::new();
            for (k, p) in pts.iter().enumerate() {
                if k == 0 {
                    pb.move_to(p.0, p.1);
                } else {
                    pb.line_to(p.0, p.1);
                }
            }
            if closed {
                pb.close();
            }
            if let Some(path) = pb.finish() {
                pm.stroke_path(
                    &path,
                    &paint(color, 1.0),
                    &Stroke {
                        width,
                        line_cap: tiny_skia::LineCap::Round,
                        line_join: tiny_skia::LineJoin::Round,
                        ..Default::default()
                    },
                    t,
                    m,
                );
            }
        });
    }

    /// Filled circle or ring.
    pub fn circle(&self, cx: f32, cy: f32, r: f32, color: u32, ring: Option<f32>) {
        if !self.visible(cy - r, 2.0 * r)
            || !self.clip_allows(&Rect::new(cx - r, cy - r, 2.0 * r, 2.0 * r))
        {
            return;
        }
        self.with_clipped(|pm, t, m| match ring {
            None => {
                if let Some(p) = PathBuilder::from_circle(cx, cy, r) {
                    pm.fill_path(&p, &paint(color, 1.0), FillRule::Winding, t, m);
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
                        m,
                    );
                }
            }
        });
    }

    /// Horizontal fade from transparent to `color` (left → right).
    pub fn fade_right(&self, r: Rect, color: u32) {
        if !self.visible(r.y, r.h) || !self.clip_allows(&r) {
            return;
        }
        self.with_clipped(|pm, t, m| {
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
                pm.fill_rect(rc, &p, t, m);
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
        let bbox = Rect::new(x, y, shaped.width(), shaped.height());
        match self.clip() {
            Some(c) if !c.overlaps(&bbox) => {}
            // Glyph runs are rasterized by DirectWrite straight into the DIB, which ignores GDI
            // clip regions: save the band rows, draw, and restore every pixel outside the clip.
            Some(c) if !c.contains_rect(&bbox) => {
                self.clipped_px(c, || self.text_raw(shaped, x, y))
            }
            _ => self.text_raw(shaped, x, y),
        }
    }

    /// Runs `draw`, then restores all band pixels outside `clip` (client DIPs) to their prior
    /// values. The saved copy is at most one band (≤ 512 KiB, P-12).
    fn clipped_px(&self, clip: Rect, draw: impl FnOnce()) {
        self.cpu_begin();
        let stride = self.w as usize * 4;
        let rows = self.rows.max(0) as usize;
        let len = stride * rows;
        // SAFETY: the DIB holds ≥ `rows` rows of `stride` bytes; GDI work was flushed above.
        let saved = unsafe { std::slice::from_raw_parts(self.bits, len) }.to_vec();
        draw();
        self.cpu_begin();
        let s = self.scale;
        let x0 = ((clip.x * s).floor().max(0.0) as usize).min(self.w as usize);
        let x1 = ((clip.right() * s).ceil().max(0.0) as usize).min(self.w as usize);
        let y0 = ((clip.y * s).floor() - self.top_px as f32).clamp(0.0, rows as f32) as usize;
        let y1 = ((clip.bottom() * s).ceil() - self.top_px as f32).clamp(0.0, rows as f32) as usize;
        // SAFETY: as above; no other reference to the bits exists here.
        let buf = unsafe { std::slice::from_raw_parts_mut(self.bits, len) };
        for row in 0..rows {
            let a = row * stride;
            if row < y0 || row >= y1 || x0 >= x1 {
                buf[a..a + stride].copy_from_slice(&saved[a..a + stride]);
            } else {
                buf[a..a + x0 * 4].copy_from_slice(&saved[a..a + x0 * 4]);
                buf[a + x1 * 4..a + stride].copy_from_slice(&saved[a + x1 * 4..a + stride]);
            }
        }
    }

    fn text_raw(&self, shaped: &dyn Shaped, x: f32, y: f32) {
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
