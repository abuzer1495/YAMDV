//! Animated GIF playback (R-IMG-01, FUT-20): WIC frames composited by
//! `yamdv_core::image::gif` and scaled to display size. Holds the logical-screen canvas plus
//! one display-size frame; frames are decoded one at a time on the UI thread.

use std::sync::atomic::{AtomicU64, Ordering};

use windows::Win32::Graphics::Imaging::{
    GUID_ContainerFormatGif, GUID_WICPixelFormat32bppPBGRA, IWICBitmapDecoder, IWICImagingFactory,
    IWICMetadataQueryReader, WICBitmapDitherTypeNone, WICBitmapInterpolationModeFant,
    WICBitmapPaletteTypeCustom,
};
use windows::Win32::System::Com::StructuredStorage::{
    PROPVARIANT, PropVariantClear, PropVariantToUInt32WithDefault,
};
use windows::core::{PCWSTR, w};
use yamdv_core::image::gif::{Canvas, Disposal, MAX_FRAMES, Player, Rect, delay_ms};
use yamdv_core::image::{MAX_DIM, Source};

use super::{Bitmap, open_decoder};

/// Frames advanced since start (driver `gif_frames`).
pub static FRAMES: AtomicU64 = AtomicU64::new(0);

/// Monotonic ms clock for [`Player`].
pub fn now_ms() -> u64 {
    // SAFETY: no preconditions.
    unsafe { windows::Win32::System::SystemInformation::GetTickCount64() }
}

fn md(q: &IWICMetadataQueryReader, name: PCWSTR) -> u32 {
    let mut v = PROPVARIANT::default();
    // SAFETY: `v` is a valid out PROPVARIANT, cleared after reading.
    unsafe {
        if q.GetMetadataByName(name, &mut v).is_err() {
            return 0;
        }
        let x = PropVariantToUInt32WithDefault(&v, 0);
        let _ = PropVariantClear(&mut v);
        x
    }
}

/// Per-frame rectangle, delay (ms) and disposal.
type Meta = (Rect, u32, Disposal);

/// One playing GIF at a fixed display size.
pub struct Anim {
    dec: IWICBitmapDecoder,
    /// Keeps a `data:` payload alive for the decoder's memory stream (dropped after `dec`).
    _src: Source,
    meta: Vec<Meta>,
    canvas: Canvas,
    /// Frame clock.
    pub player: Player,
    /// The current frame at display size.
    pub out: Bitmap,
}

impl std::fmt::Debug for Anim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Anim")
            .field("frames", &self.meta.len())
            .field("idx", &self.player.idx)
            .finish()
    }
}

impl Anim {
    /// Opens `src` for `w × h` display px; `None` unless it is a GIF with ≥ 2 frames.
    pub fn open(f: &IWICImagingFactory, src: &Source, w: u32, h: u32, now: u64) -> Option<Self> {
        let src = src.clone();
        let dec = open_decoder(f, &src).ok()?;
        // SAFETY: COM objects of this thread; all calls are plain getters.
        let (meta, cw, ch) = unsafe {
            if dec.GetContainerFormat().ok()? != GUID_ContainerFormatGif {
                return None;
            }
            let n = (dec.GetFrameCount().ok()? as usize).min(MAX_FRAMES);
            if n < 2 {
                return None;
            }
            let (mut cw, mut ch) = (0, 0);
            if let Ok(q) = dec.GetMetadataQueryReader() {
                cw = md(&q, w!("/logscrdesc/Width"));
                ch = md(&q, w!("/logscrdesc/Height"));
            }
            let mut meta = Vec::with_capacity(n);
            for i in 0..n {
                let fr = dec.GetFrame(u32::try_from(i).ok()?).ok()?;
                let (mut fw, mut fh) = (0, 0);
                fr.GetSize(&mut fw, &mut fh).ok()?;
                let (x, y, d, disp) = match fr.GetMetadataQueryReader() {
                    Ok(q) => (
                        md(&q, w!("/imgdesc/Left")),
                        md(&q, w!("/imgdesc/Top")),
                        md(&q, w!("/grctlext/Delay")),
                        md(&q, w!("/grctlext/Disposal")),
                    ),
                    Err(_) => (0, 0, 0, 0),
                };
                let r = Rect { x, y, w: fw, h: fh };
                if cw == 0 || ch == 0 {
                    (cw, ch) = (x + fw, y + fh);
                }
                meta.push((
                    r,
                    delay_ms(u16::try_from(d).unwrap_or(u16::MAX)),
                    Disposal::from_gif(u8::try_from(disp).unwrap_or(0)),
                ));
            }
            (meta, cw, ch)
        };
        if cw == 0 || ch == 0 || cw > MAX_DIM || ch > MAX_DIM {
            return None;
        }
        let first = meta.first()?.1;
        let (w, h) = (w.clamp(1, MAX_DIM), h.clamp(1, MAX_DIM));
        let mut a = Self {
            dec,
            _src: src,
            player: Player::new(meta.len(), first, now),
            meta,
            canvas: Canvas::new(cw, ch),
            out: Bitmap {
                w,
                h,
                px: vec![0; (w as usize) * (h as usize) * 4],
                svg_color: None,
            },
        };
        a.compose(f, 0)?;
        Some(a)
    }

    /// Bytes held (canvas + saved region + display frame).
    pub fn bytes(&self) -> usize {
        self.canvas.px.len() + self.canvas.extra_bytes() + self.out.px.len()
    }

    fn compose(&mut self, f: &IWICImagingFactory, i: usize) -> Option<()> {
        if i == 0 {
            self.canvas.reset();
        }
        let &(r, _, disp) = self.meta.get(i)?;
        // SAFETY: COM objects of this thread; buffers are sized `w*h*4` with stride `w*4`.
        unsafe {
            let fr = self.dec.GetFrame(u32::try_from(i).ok()?).ok()?;
            let cv = f.CreateFormatConverter().ok()?;
            cv.Initialize(
                &fr,
                &GUID_WICPixelFormat32bppPBGRA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )
            .ok()?;
            let mut px = vec![0u8; (r.w as usize) * (r.h as usize) * 4];
            cv.CopyPixels(std::ptr::null(), r.w * 4, &mut px).ok()?;
            self.canvas.draw(r, disp, &px);
            let (cw, ch) = (self.canvas.w, self.canvas.h);
            let bmp = f
                .CreateBitmapFromMemory(
                    cw,
                    ch,
                    &GUID_WICPixelFormat32bppPBGRA,
                    cw * 4,
                    &self.canvas.px,
                )
                .ok()?;
            let sc = f.CreateBitmapScaler().ok()?;
            sc.Initialize(&bmp, self.out.w, self.out.h, WICBitmapInterpolationModeFant)
                .ok()?;
            sc.CopyPixels(std::ptr::null(), self.out.w * 4, &mut self.out.px)
                .ok()?;
        }
        if i > 0 {
            FRAMES.fetch_add(1, Ordering::Relaxed);
        }
        Some(())
    }

    /// Advances when the next frame is due; returns whether the picture changed.
    pub fn tick(&mut self, f: &IWICImagingFactory, now: u64) -> bool {
        let meta = &self.meta;
        match self.player.tick(now, |k| meta.get(k).map_or(100, |m| m.1)) {
            Some(i) => self.compose(f, i).is_some(),
            None => false,
        }
    }

    /// After a pause: the current frame shows its delay again from `now` at most.
    pub fn resume(&mut self, now: u64) {
        let d = self.meta.get(self.player.idx).map_or(100, |m| m.1);
        self.player.resume(now, d);
    }
}
