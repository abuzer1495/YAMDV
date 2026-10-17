//! Print and PDF through the print path (R-EXP-03; PDF via FUT-39, prompt 008 §3).
//!
//! The document is laid out at the printable width (page minus `export.pdf_margins_mm`) with the
//! printer DPI as the layout scale, paginated by `yamdv_export::paginate`, and every page is drawn
//! with the band renderer and sent to the printer DC band by band (`StretchDIBits`); no
//! full-page bitmap is allocated. Text is raster in v1 (FUT-39). "Export PDF" is the same
//! pipeline on "Microsoft Print to PDF" with the output path in `DOCINFO.lpszOutput`.
#![allow(unsafe_code)]
// Device/DIP geometry: float<->int conversions are bounded by page sizes (ADR-0007).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap
)]

use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateDCW, CreateFontW, DEVMODEW, DIB_RGB_COLORS,
    DeleteDC, DeleteObject, GetDeviceCaps, HDC, HGDIOBJ, LOGPIXELSX, LOGPIXELSY, PHYSICALHEIGHT,
    PHYSICALOFFSETX, PHYSICALOFFSETY, PHYSICALWIDTH, SRCCOPY, SelectObject, SetBkMode,
    SetTextAlign, SetTextColor, StretchDIBits, TA_BOTTOM, TA_CENTER, TRANSPARENT, TextOutW,
};
use windows::Win32::Storage::Xps::{AbortDoc, DOCINFOW, EndDoc, EndPage, StartDocW, StartPage};
use windows::core::{HSTRING, PCWSTR, w};
use yamdv_core::doc::index::BlockKind;
use yamdv_core::doc::{Document, ReadOpts};
use yamdv_core::theme::{self, Palette};
use yamdv_export::paginate::{Page, Unit, footer, paginate};
use yamdv_layout::viewer::{Content, Env, Viewer};

use crate::dwrite::{DWrite, DwShaper};
use crate::export::Job;
use crate::render::{Renderer, colorref};

/// The PDF printer driven by "Export PDF" (prompt 008 §3.4).
pub const PDF_PRINTER: &str = "Microsoft Print to PDF";
/// Raster resolution cap: pages are rendered at ≤ 300 dpi and stretched to the device.
const MAX_RENDER_DPI: f32 = 300.0;

/// Where and how to print.
#[derive(Debug, Clone, Default)]
pub struct Target {
    /// Printer name.
    pub printer: String,
    /// DEVMODE bytes from the print dialog (empty = driver defaults + paper size).
    pub devmode: Vec<u8>,
    /// 1-based inclusive page range (`None` = all).
    pub range: Option<(u32, u32)>,
    /// Output file (PDF printer only).
    pub output: Option<String>,
    /// Paper size in mm when `devmode` is empty (`export.pdf_page_size`).
    pub paper_mm: Option<(f64, f64)>,
    /// Use `export.pdf_colors` (PDF) instead of `export.print_colors` (print).
    pub pdf: bool,
}

/// Why printing failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The printer does not exist (e.g. "Microsoft Print to PDF" not installed).
    NoPrinter,
    /// Printing failed or was aborted.
    Failed,
}

/// DEVMODE with only the paper size set (the driver merges it with its defaults).
fn paper_devmode(printer: &str, mm: (f64, f64)) -> DEVMODEW {
    use windows::Win32::Graphics::Gdi::{
        DEVMODE_FIELD_FLAGS, DM_ORIENTATION, DM_PAPERLENGTH, DM_PAPERSIZE, DM_PAPERWIDTH,
    };
    let mut dm = DEVMODEW {
        dmSize: size_of::<DEVMODEW>() as u16,
        dmSpecVersion: 0x0401,
        ..Default::default()
    };
    for (d, c) in dm.dmDeviceName.iter_mut().zip(printer.encode_utf16().take(31)) {
        *d = c;
    }
    let letter = (mm.0 - 215.9).abs() < 1.0;
    let a4 = (mm.0 - 210.0).abs() < 1.0;
    let size: i16 = if letter {
        1
    } else if a4 {
        9
    } else {
        0
    };
    // Writing plain-old-data fields of the printer variant of the union.
    dm.Anonymous1.Anonymous1.dmOrientation = 1;
    dm.Anonymous1.Anonymous1.dmPaperSize = size;
    dm.Anonymous1.Anonymous1.dmPaperWidth = (mm.0 * 10.0).round() as i16;
    dm.Anonymous1.Anonymous1.dmPaperLength = (mm.1 * 10.0).round() as i16;
    dm.dmFields = DEVMODE_FIELD_FLAGS(
        DM_ORIENTATION.0
            | if size == 0 {
                DM_PAPERWIDTH.0 | DM_PAPERLENGTH.0
            } else {
                DM_PAPERSIZE.0
            },
    );
    dm
}

/// Opens a DC on `t.printer`.
fn printer_dc(t: &Target) -> Option<HDC> {
    let name = HSTRING::from(t.printer.as_str());
    let paper = t.paper_mm.map(|mm| paper_devmode(&t.printer, mm));
    let dm: Option<*const DEVMODEW> = if t.devmode.len() >= size_of::<DEVMODEW>() {
        Some(t.devmode.as_ptr().cast())
    } else {
        paper.as_ref().map(std::ptr::from_ref)
    };
    // SAFETY: valid strings; the DEVMODE buffer (dialog bytes, at least one DEVMODEW long, or
    // our own struct) outlives the call.
    let dc = unsafe { CreateDCW(w!("WINSPOOL"), &name, PCWSTR::null(), dm) };
    (!dc.is_invalid()).then_some(dc)
}

/// Whether a printer named `name` exists (a DC can be created on it).
pub fn printer_exists(name: &str) -> bool {
    let t = Target {
        printer: name.to_string(),
        ..Default::default()
    };
    match printer_dc(&t) {
        Some(dc) => {
            // SAFETY: DC created above.
            let _ = unsafe { DeleteDC(dc) };
            true
        }
        None => false,
    }
}

/// Page geometry in device pixels.
#[derive(Debug, Clone, Copy)]
struct Geo {
    dpi: f32,
    /// Content origin relative to the printable area.
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    /// Bottom margin height (footer area).
    margin: i32,
}

fn geometry(dc: HDC, margins_mm: f64) -> Geo {
    // SAFETY: valid printer DC.
    let cap = |i| unsafe { GetDeviceCaps(Some(dc), i) };
    let dpi = cap(LOGPIXELSX).max(72) as f32;
    let dpi_y = cap(LOGPIXELSY).max(72) as f32;
    let (pw, ph) = (cap(PHYSICALWIDTH), cap(PHYSICALHEIGHT));
    let (ox, oy) = (cap(PHYSICALOFFSETX), cap(PHYSICALOFFSETY));
    let mx = (margins_mm / 25.4 * f64::from(dpi)).round() as i32;
    let my = (margins_mm / 25.4 * f64::from(dpi_y)).round() as i32;
    Geo {
        dpi,
        left: (mx - ox).max(0),
        top: (my - oy).max(0),
        width: (pw - 2 * mx).max(100),
        height: (ph - 2 * my).max(100),
        margin: my,
    }
}

/// Break candidates of a laid-out block (R-EXP-03): line boundaries of paragraphs; table row
/// tops only (rows never split); none for diagrams and formulas.
fn unit_of(content: &Content, viewer: &Viewer, bi: usize, prev_bottom: f64) -> Option<Unit> {
    let bl = viewer.layout_of(bi)?;
    let top = viewer.block_top(content, bi);
    let bottom = top + f64::from(bl.height);
    let kind = match content {
        Content::Markdown(d) => d.index.blocks.get(bi).map(|b| b.kind),
        Content::Plain { .. } => None,
    };
    let src = content.source();
    let r = content.range(bi);
    let head = src
        .get(r.start as usize..(r.end as usize).min(r.start as usize + 16))
        .unwrap_or("")
        .trim_start();
    let diagram = head.starts_with("```mermaid") || head.starts_with("~~~mermaid");
    let mut breaks = Vec::new();
    match kind {
        Some(BlockKind::Math) => {}
        _ if diagram => {}
        Some(BlockKind::Table) => {
            for p in &bl.paras {
                breaks.push(top + f64::from(p.y) - 6.0);
            }
        }
        _ => {
            for p in &bl.paras {
                let n = p.shaped.lines();
                if n > 1 {
                    let pitch = f64::from(p.shaped.height()) / n as f64;
                    for k in 1..n {
                        breaks.push(top + f64::from(p.y) + pitch * k as f64);
                    }
                }
            }
        }
    }
    breaks.retain(|&b| b > top + 0.5 && b < bottom - 0.5);
    breaks.sort_by(f64::total_cmp);
    breaks.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    Some(Unit {
        top: prev_bottom.min(top),
        bottom,
        breaks,
    })
}

/// Result of a print run.
#[derive(Debug, Clone, Copy)]
pub struct Printed {
    /// Pages sent to the printer.
    pub pages: usize,
}

/// Prints `job` on `t` (COM initialized). Frees every transient buffer and trims the working set
/// once afterwards (D.4 rule 10).
pub fn print(dw: &DWrite, job: &Job, t: &Target) -> Result<Printed, Error> {
    let r = print_inner(dw, job, t);
    crate::sys::trim_once();
    r
}

#[allow(clippy::too_many_lines)]
fn print_inner(dw: &DWrite, job: &Job, t: &Target) -> Result<Printed, Error> {
    let s = crate::window::load_settings();
    let dc = printer_dc(t).ok_or(Error::NoPrinter)?;
    let g = geometry(dc, s.int("export.pdf_margins_mm").clamp(5, 40) as f64);
    let colors = s.str(if t.pdf {
        "export.pdf_colors"
    } else {
        "export.print_colors"
    });
    let pal: Palette = if colors == "light" {
        Palette {
            bg: 0xffffff,
            ..theme::palette(&job.scheme, false)
        }
    } else {
        theme::palette(&job.scheme, job.dark)
    };
    let dev_per_dip = g.dpi / 96.0;
    let rscale = g.dpi.min(MAX_RENDER_DPI) / 96.0;
    let width_dips = g.width as f32 / dev_per_dip;
    let page_h = f64::from(g.height as f32 / dev_per_dip);
    let mut content = Content::Markdown(Document::new(job.source.clone(), ReadOpts::default()));
    let mut viewer = Viewer::new(&content, width_dips, job.m);
    let images = crate::images::Store::synchronous(&job.path);
    let cache = yamdv_layout::rich::RichCache::default();
    let rich = crate::dwrite::Rich::new(dw, &cache, true, true);
    let strings = crate::window::strings(job.lang);
    let mut shaper = DwShaper(dw);
    let mut prepare = |content: &mut Content, viewer: &mut Viewer, y: f64, vh: f64| {
        viewer.scroll_to(y, vh);
        let mut env = Env {
            images: &images,
            rich: &rich,
            shaper: &mut shaper,
            m: job.m,
            pal: &pal,
            strings: &strings,
            single_newline_break: s.bool("reading.single_newline_break"),
        };
        viewer.prepare(content, &mut env, vh);
    };
    // Pass 1: lay out top to bottom (earlier tops are final), collecting break candidates.
    let mut units: Vec<Unit> = Vec::with_capacity(content.len());
    let mut next = 0usize;
    let mut guard = 0usize;
    while next < content.len() && guard < content.len() * 4 + 16 {
        guard += 1;
        let y = if next == 0 {
            0.0
        } else {
            viewer.block_top(&content, next) - 1.0
        };
        prepare(&mut content, &mut viewer, y.max(0.0), page_h);
        let mut progressed = false;
        while next < content.len() {
            let prev = units.last().map_or(0.0, |u| u.bottom);
            let Some(u) = unit_of(&content, &viewer, next, prev) else {
                break;
            };
            units.push(u);
            next += 1;
            progressed = true;
        }
        if !progressed {
            // Not laid out (empty block): skip it rather than loop.
            next += 1;
        }
    }
    let pages: Vec<Page> = paginate(&units, page_h);
    drop(units);
    let total = pages.len();
    let (from, to) = t.range.unwrap_or((1, u32::MAX));
    let name = HSTRING::from(job.stem());
    let out = t.output.as_deref().map(HSTRING::from);
    let di = DOCINFOW {
        cbSize: size_of::<DOCINFOW>() as i32,
        lpszDocName: PCWSTR(name.as_ptr()),
        lpszOutput: out.as_ref().map_or(PCWSTR::null(), |o| PCWSTR(o.as_ptr())),
        ..Default::default()
    };
    // SAFETY: valid DC and DOCINFO (strings outlive the job).
    if unsafe { StartDocW(dc, &di) } <= 0 {
        // SAFETY: DC created above.
        let _ = unsafe { DeleteDC(dc) };
        return Err(Error::Failed);
    }
    let mut ok = true;
    let mut sent = 0usize;
    let mut r = Renderer::new(dw, None, (width_dips * rscale).round() as i32, rscale)
        .map_err(|_| Error::Failed)?;
    r.set_grayscale(dw);
    let rw = r.width();
    // SAFETY: font for the footer, deleted below.
    let font = unsafe {
        CreateFontW(
            -((9.0 * g.dpi / 72.0).round() as i32),
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            windows::Win32::Graphics::Gdi::DEFAULT_CHARSET,
            windows::Win32::Graphics::Gdi::OUT_DEFAULT_PRECIS,
            windows::Win32::Graphics::Gdi::CLIP_DEFAULT_PRECIS,
            windows::Win32::Graphics::Gdi::ANTIALIASED_QUALITY,
            0,
            w!("Segoe UI"),
        )
    };
    for (k, p) in pages.iter().enumerate() {
        let n = k as u32 + 1;
        if n < from || n > to {
            continue;
        }
        let vh = p.y1 - p.y0;
        prepare(&mut content, &mut viewer, p.y0, vh);
        // SAFETY: valid DC between StartDoc and EndDoc.
        if unsafe { StartPage(dc) } <= 0 {
            ok = false;
            break;
        }
        let h_px = (vh as f32 * rscale).ceil() as i32;
        let k_dev = f64::from(dev_per_dip / rscale) * p.scale;
        let dest_w = (f64::from(rw) * k_dev).round() as i32;
        let x0 = g.left + (g.width - dest_w).max(0) / 2 * i32::from(p.scale < 1.0);
        r.render_bands(
            h_px,
            pal.text,
            &mut |band| {
                band.clear(pal.bg);
                crate::window::draw_doc(
                    band,
                    &content,
                    &viewer,
                    &pal,
                    0.0,
                    None,
                    vh,
                    Some(&images),
                    (&[], None),
                );
            },
            &mut |y, rows, bits| {
                let bmi = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: rw,
                        biHeight: -rows,
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: BI_RGB.0,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                let dy0 = (f64::from(y) * k_dev).round() as i32;
                let dy1 = (f64::from(y + rows) * k_dev).round() as i32;
                // SAFETY: `bits` holds `rows` top-down rows of `rw` 32-bit pixels.
                unsafe {
                    StretchDIBits(
                        dc,
                        x0,
                        g.top + dy0,
                        dest_w,
                        (dy1 - dy0).max(1),
                        0,
                        0,
                        rw,
                        rows,
                        Some(bits.as_ptr().cast()),
                        &bmi,
                        DIB_RGB_COLORS,
                        SRCCOPY,
                    );
                }
            },
        );
        if s.bool("export.pdf_page_numbers") {
            let text: Vec<u16> = footer(k + 1, total).encode_utf16().collect();
            // SAFETY: valid DC and font; the text buffer outlives the call.
            unsafe {
                let old = SelectObject(dc, HGDIOBJ(font.0));
                SetBkMode(dc, TRANSPARENT);
                let _ = SetTextColor(dc, colorref(pal.text_muted));
                SetTextAlign(dc, TA_CENTER | TA_BOTTOM);
                let _ = TextOutW(
                    dc,
                    g.left + g.width / 2,
                    g.top + g.height + g.margin / 2,
                    &text,
                );
                SelectObject(dc, old);
            }
        }
        // SAFETY: page started above.
        if unsafe { EndPage(dc) } <= 0 {
            ok = false;
            break;
        }
        sent += 1;
    }
    // SAFETY: valid DC; the font is ours.
    unsafe {
        if ok {
            ok = EndDoc(dc) > 0;
        } else {
            AbortDoc(dc);
        }
        let _ = DeleteObject(HGDIOBJ(font.0));
        let _ = DeleteDC(dc);
    }
    drop(r);
    drop(viewer);
    drop(content);
    if ok {
        Ok(Printed { pages: sent })
    } else {
        Err(Error::Failed)
    }
}

/// Target for "Export PDF" to `out` (paper per `export.pdf_page_size`).
pub fn pdf_target(out: &str) -> Target {
    let s = crate::window::load_settings();
    Target {
        printer: PDF_PRINTER.to_string(),
        output: Some(out.to_string()),
        paper_mm: Some(yamdv_export::paginate::paper_mm(
            s.str("export.pdf_page_size"),
            &crate::sys::user_locale(),
        )),
        pdf: true,
        ..Default::default()
    }
}

/// `yamdv --export-pdf <in.md> [-o out.pdf]` (R-EXP-05): exit 0 success, 1 error. Writes
/// `pages <n>` to stdout when a console is attached (T10).
pub fn cli_pdf(input: &str, out: Option<&str>) -> i32 {
    // SAFETY: COM for WIC on this thread.
    let _ = unsafe {
        windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        )
    };
    let Some(job) = Job::from_file(input, None, None) else {
        return 1;
    };
    let out = out.map_or_else(|| crate::export::sibling(&job.path, "pdf"), str::to_string);
    let Some(out) = std::path::absolute(&out)
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
    else {
        return 1;
    };
    let Ok(dw) = DWrite::new(&job.fonts.0, &job.fonts.1) else {
        return 1;
    };
    match print(&dw, &job, &pdf_target(&out)) {
        Ok(p) => {
            crate::console::write(
                crate::console::Stream::Out,
                &format!("pages {}\n", p.pages),
            );
            0
        }
        Err(Error::NoPrinter) => {
            crate::console::write(
                crate::console::Stream::Err,
                "yamdv: Microsoft Print to PDF is not installed\n",
            );
            1
        }
        Err(Error::Failed) => 1,
    }
}
