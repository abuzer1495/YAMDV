//! Developer tools (feature `dev-tools`, never in release): `--render-png` renders a document
//! headlessly through the real pipeline (offscreen band rendering into a full bitmap, PNG via WIC)
//! for the golden images (Appendix H §H.4).
#![allow(unsafe_code)]
// Pixel/DIP geometry: float<->int conversions are bounded by window/bitmap sizes (ADR-0007).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::indexing_slicing
)]

use windows::Win32::Foundation::GENERIC_WRITE;
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_ContainerFormatPng, GUID_WICPixelFormat32bppBGRA,
    IWICImagingFactory, WICBitmapEncoderNoCache,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::core::HSTRING;
use yamdv_core::doc::{Document, ReadOpts};
use yamdv_core::theme;
use yamdv_layout::block::Metrics;
use yamdv_layout::viewer::{Content, Env, Viewer};

use crate::dwrite::{DWrite, DwShaper};
use crate::render::Renderer;

/// `--render-png` options.
#[derive(Debug, Clone)]
pub struct RenderPng {
    /// Input document.
    pub input: String,
    /// Output PNG.
    pub output: String,
    /// Window width in DIPs (column per B.6.1).
    pub width: f32,
    /// DPI scale.
    pub scale: f32,
    /// Dark palette.
    pub dark: bool,
}

/// Writes BGRA pixels as PNG through WIC.
pub fn write_png(path: &str, w: u32, h: u32, bgra: &[u8]) -> windows::core::Result<()> {
    // SAFETY: COM initialized by the caller; all objects are released by Drop; the pixel buffer
    // is `w*h*4` bytes.
    unsafe {
        let f: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let stream = f.CreateStream()?;
        stream.InitializeFromFilename(&HSTRING::from(path), GENERIC_WRITE.0)?;
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
    }
    Ok(())
}

/// Renders `o.input` to `o.output`. Returns the process exit code.
pub fn render_png(o: &RenderPng) -> i32 {
    // SAFETY: COM for WIC on this thread.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let Ok(bytes) = std::fs::read(&o.input) else {
        return 1;
    };
    let d = yamdv_core::decode::decode(&bytes);
    let Ok(dw) = DWrite::new("Segoe UI", "Cascadia Mono") else {
        return 1;
    };
    let m = Metrics::default();
    let (cx, colw) = crate::window::column(o.width, 0.65);
    let plain = yamdv_core::path::extension(&o.input) == "txt";
    let mut content = if plain {
        Content::plain(d.text)
    } else {
        Content::Markdown(Document::new(d.text, ReadOpts::default()))
    };
    let pal = theme::palette("default", o.dark);
    let strings = crate::window::strings(yamdv_core::i18n::Lang::En);
    let mut viewer = Viewer::new(&content, colw, m);
    let images = crate::images::Store::synchronous(
        &std::path::absolute(&o.input)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| o.input.clone()),
    );
    let mut shaper = DwShaper(&dw);
    // Lay out everything (the viewport is the whole document); repeat until heights settle.
    let mut total = viewer.total_height();
    for _ in 0..16 {
        let mut env = Env {
            images: &images,
            shaper: &mut shaper,
            m,
            pal: &pal,
            strings: &strings,
            single_newline_break: false,
        };
        viewer.prepare(&mut content, &mut env, total + 1.0);
        let t = viewer.total_height();
        if (t - total).abs() < 0.5 {
            break;
        }
        total = t;
    }
    let w_px = (o.width * o.scale).round() as i32;
    let h_px = ((total as f32) * o.scale).round().min(32_000.0) as i32;
    let Ok(r) = Renderer::new(&dw, None, w_px, o.scale) else {
        return 1;
    };
    let px = r.render_offscreen(h_px, pal.text, &mut |band| {
        band.clear(pal.bg);
        crate::window::draw_doc(
            band,
            &content,
            &viewer,
            &pal,
            cx,
            None,
            total + 1.0,
            Some(&images),
        );
    });
    let mut px = px;
    for p in px.as_chunks_mut::<4>().0 {
        p[3] = 255;
    }
    match write_png(
        &o.output,
        u32::try_from(w_px).unwrap_or(0),
        u32::try_from(h_px).unwrap_or(0),
        &px,
    ) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}
