//! Offscreen PNG of a single block's display list (prompt 007 §4): V1-D's static HTML export
//! embeds formulas and diagrams as 2× PNG data URIs. Reuses the `--render-png` band pipeline.
#![allow(unsafe_code)]
// Pixel/DIP geometry: float<->int conversions are bounded by bitmap sizes (ADR-0007).
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use windows::Win32::Foundation::HGLOBAL;
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_ContainerFormatPng, GUID_WICPixelFormat32bppBGRA,
    IWICImagingFactory, WICBitmapEncoderNoCache,
};
use windows::Win32::System::Com::StructuredStorage::CreateStreamOnHGlobal;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, CoCreateInstance, STREAM_SEEK_CUR, STREAM_SEEK_SET,
};
use yamdv_core::theme::Palette;
use yamdv_layout::viewer::Viewer;

use crate::dwrite::DWrite;
use crate::render::Renderer;

/// Largest snapshot side in device pixels.
const MAX_SIDE: f32 = 16_384.0;

/// Encodes top-down BGRA pixels as PNG bytes through WIC (COM must be initialized).
pub fn encode_png(w: u32, h: u32, bgra: &[u8]) -> windows::core::Result<Vec<u8>> {
    if bgra.len() < (w as usize) * (h as usize) * 4 {
        return Err(windows::core::Error::empty());
    }
    // SAFETY: COM is initialized by the caller; objects are released by Drop; the pixel buffer
    // holds `w*h*4` bytes (checked above) and the read buffer is `len` bytes.
    unsafe {
        let f: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let stream = CreateStreamOnHGlobal(HGLOBAL::default(), true)?;
        let enc = f.CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null())?;
        enc.Initialize(&stream, WICBitmapEncoderNoCache)?;
        let mut frame = None;
        enc.CreateNewFrame(&mut frame, std::ptr::null_mut())?;
        let frame = frame.ok_or_else(windows::core::Error::empty)?;
        frame.Initialize(None)?;
        frame.SetSize(w, h)?;
        let mut fmt = GUID_WICPixelFormat32bppBGRA;
        frame.SetPixelFormat(&mut fmt)?;
        frame.WritePixels(h, w * 4, bgra)?;
        frame.Commit()?;
        enc.Commit()?;
        let mut len = 0u64;
        stream.Seek(0, STREAM_SEEK_CUR, Some(&mut len))?;
        stream.Seek(0, STREAM_SEEK_SET, None)?;
        let len = u32::try_from(len).map_err(|_| windows::core::Error::empty())?;
        let mut out = vec![0u8; len as usize];
        let mut read = 0u32;
        stream
            .Read(out.as_mut_ptr().cast(), len, Some(&mut read))
            .ok()?;
        out.truncate(read as usize);
        Ok(out)
    }
}

/// Renders block `bi` (already laid out in `viewer`) on `pal.bg` at `scale` device pixels per
/// DIP and returns `(width_px, height_px, png)`. `None` when the block is not laid out.
pub fn block_png(
    dw: &DWrite,
    viewer: &Viewer,
    bi: usize,
    pal: &Palette,
    scale: f32,
    images: Option<&crate::images::Store>,
) -> Option<(u32, u32, Vec<u8>)> {
    let bl = viewer.layout_of(bi)?;
    let scale = scale.clamp(0.25, 8.0);
    let w_px = (viewer.width * scale).round().clamp(1.0, MAX_SIDE) as i32;
    let h_px = (bl.height.max(1.0) * scale).round().clamp(1.0, MAX_SIDE) as i32;
    let r = Renderer::new(dw, None, w_px, scale).ok()?;
    let mut px = r.render_offscreen(h_px, pal.text, &mut |band| {
        band.clear(pal.bg);
        crate::window::draw_block(
            band,
            bl,
            &|reg| viewer.hscroll(bi, reg),
            pal,
            (0.0, 0.0),
            None,
            images,
            (&[], None),
        );
    });
    for p in px.as_chunks_mut::<4>().0 {
        p[3] = 255;
    }
    let (w, h) = (w_px as u32, h_px as u32);
    encode_png(w, h, &px).ok().map(|png| (w, h, png))
}
