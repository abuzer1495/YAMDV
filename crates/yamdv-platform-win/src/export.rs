//! Export (prompt 008 §2): the static single-file HTML document (R-EXP-02 per FUT-21) with its
//! platform host (decoded local images, formulas and diagrams as 2× PNG through the V1-C
//! block-PNG pipeline), table CSV files (R-EXP-04) and the headless CLI (R-EXP-05).
//! Exports are written atomically, only to the user-chosen path (S-EXP-01).
#![allow(unsafe_code)]
// Pixel/DIP geometry: float<->int conversions are bounded by bitmap sizes (ADR-0007).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use yamdv_core::doc::detail::ImageRef;
use yamdv_core::doc::{Document, ReadOpts};
use yamdv_core::i18n::Lang;
use yamdv_core::image::{self, Format, Source};
use yamdv_core::theme::Palette;
use yamdv_export::html::{DocOpts, ExportHost, Png};
use yamdv_layout::block::Metrics;
use yamdv_layout::viewer::{Content, Env, Viewer};

use crate::dwrite::{DWrite, DwShaper};
use crate::render::Renderer;

/// Column width (DIPs) used for diagrams in the exported HTML (`main` max-width minus padding).
const HTML_COLUMN: f32 = 812.0;
/// Raster scale of formulas and diagrams (prompt 008 §2.1: 2× PNG).
const SCALE: f32 = 2.0;

/// What to export: the document text and the look it is exported with.
#[derive(Debug, Clone)]
pub struct Job {
    /// Markdown source.
    pub source: String,
    /// Absolute document path (image resolution, default output names).
    pub path: String,
    /// `appearance.scheme`.
    pub scheme: String,
    /// Dark mode.
    pub dark: bool,
    /// Typography.
    pub m: Metrics,
    /// Body and code font families.
    pub fonts: (String, String),
    /// UI language (placeholder labels).
    pub lang: Lang,
    /// `--mode` was given on the command line: it sets the export palette directly.
    pub mode_forced: bool,
}

impl Job {
    /// Job for a file with the settings in `settings.ini`; `scheme`/`dark` override them.
    pub fn from_file(path: &str, scheme: Option<&str>, dark: Option<bool>) -> Option<Self> {
        let abs = std::path::absolute(path)
            .ok()?
            .to_string_lossy()
            .into_owned();
        let bytes = std::fs::read(yamdv_core::path::long_form(&abs)).ok()?;
        if bytes.len() > 256 * 1024 * 1024 {
            return None;
        }
        let d = yamdv_core::decode::decode(&bytes);
        Some(Self::from_source(abs, d.text, scheme, dark))
    }

    /// Job for `source` (document at the absolute `abs`) with the settings in `settings.ini`;
    /// `scheme`/`dark` override them.
    pub fn from_source(
        abs: String,
        source: String,
        scheme: Option<&str>,
        dark: Option<bool>,
    ) -> Self {
        let s = crate::window::load_settings();
        let mode_forced = dark.is_some();
        let mode = s.str("appearance.mode");
        let dark = dark.unwrap_or_else(|| match mode {
            "dark" => true,
            "light" => false,
            _ => crate::theme::app_mode() == crate::theme::Mode::Dark,
        });
        Self {
            source,
            path: abs,
            scheme: scheme.map_or_else(|| s.str("appearance.scheme").to_string(), str::to_string),
            dark,
            m: Metrics {
                font_size: s.int("typography.font_size") as f32,
                line_height: s.float("typography.line_height") as f32,
                para_spacing: s.int("typography.paragraph_spacing") as f32,
                table_zebra: s.bool("appearance.table_zebra"),
                rounded: s.str("appearance.corner_style") != "sharp",
            },
            fonts: (
                s.str("typography.body_font").to_string(),
                s.str("typography.code_font").to_string(),
            ),
            lang: yamdv_core::i18n::resolve(
                s.str("general.language"),
                crate::sys::ui_language_tag(),
            ),
            mode_forced,
        }
    }

    /// File stem of the document (`<doc stem>.html` / `.pdf` / `.csv`).
    pub fn stem(&self) -> String {
        stem(&self.path)
    }
}

/// File stem of `path` (`document` when it has none).
pub fn stem(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "document".into())
}

/// Lays out every block of `content` (repeating until the heights settle).
#[allow(clippy::too_many_arguments)]
pub(crate) fn layout_all(
    dw: &DWrite,
    content: &mut Content,
    viewer: &mut Viewer,
    pal: &Palette,
    images: &crate::images::Store,
    rich: &dyn yamdv_layout::rich::RichHost,
    lang: Lang,
    m: Metrics,
) {
    let strings = crate::window::strings(lang);
    let mut shaper = DwShaper(dw);
    let mut total = viewer.total_height();
    for _ in 0..16 {
        let mut env = Env {
            images,
            rich,
            shaper: &mut shaper,
            m,
            pal,
            strings: &strings,
            single_newline_break: false,
        };
        viewer.prepare(content, &mut env, total + 1.0);
        let t = viewer.total_height();
        if (t - total).abs() < 0.5 {
            break;
        }
        total = t;
    }
}

/// The platform host of the HTML export.
struct Host<'a> {
    dw: &'a DWrite,
    doc_path: String,
    pal: Palette,
    m: Metrics,
    lang: Lang,
    wic: Option<windows::Win32::Graphics::Imaging::IWICImagingFactory>,
}

fn mime(f: Format) -> Option<&'static str> {
    Some(match f {
        Format::Png => "image/png",
        Format::Jpeg => "image/jpeg",
        Format::Gif => "image/gif",
        _ => return None,
    })
}

impl ExportHost for Host<'_> {
    fn image(&self, img: &ImageRef) -> Option<(&'static str, Vec<u8>, u32, u32)> {
        let wic = self.wic.as_ref()?;
        // Same policy as the reading view (S-RES-*): lexical refusals, then FS checks.
        let src =
            image::resolve_policy(&img.src, &self.doc_path, &mut crate::images::WinFs).ok()?;
        let yamdv_layout::img::ImgState::Ready { w, h } = crate::images::Store::probe_source(&src)
        else {
            return None;
        };
        let bytes = match &src {
            Source::File(p) => {
                let len = std::fs::metadata(p).ok()?.len();
                if len > 64 * 1024 * 1024 {
                    return None;
                }
                std::fs::read(p).ok()?
            }
            Source::Data(d) => d.clone(),
        };
        let fmt = image::probe(bytes.get(..bytes.len().min(65_536))?)?.format;
        let (cw, ch) = match (img.w, img.h) {
            (Some(a), Some(b)) => (a, b),
            (Some(a), None) => (
                a,
                (u64::from(a) * u64::from(h) / u64::from(w.max(1))) as u32,
            ),
            (None, Some(b)) => (
                (u64::from(b) * u64::from(w) / u64::from(h.max(1))) as u32,
                b,
            ),
            (None, None) => (w, h),
        };
        // S-RES-05: only images that actually decode are embedded.
        let probe = crate::images::decode(wic, &src, w.min(64), h.min(64)).ok()?;
        drop(probe);
        if let Some(m) = mime(fmt) {
            return Some((m, bytes, cw, ch));
        }
        // BMP/TIFF/ICO/WebP: re-encoded as PNG at their natural size (capped).
        let (dw_, dh) = (w.min(4096), h.min(4096));
        let bmp = crate::images::decode(wic, &src, dw_, dh).ok()?;
        let mut px = bmp.px;
        unpremultiply(&mut px);
        let png = crate::snapshot::encode_png(bmp.w, bmp.h, &px).ok()?;
        Some(("image/png", png, cw, ch))
    }

    fn math(&self, tex: &str, display: bool) -> Option<Png> {
        let face = self.dw.math_face()?;
        let size = yamdv_layout::rich::math_size(self.m.font_size);
        let mb = yamdv_math::render(tex, face.font.as_ref(), &face.table, size, display).ok()?;
        let pad = 2.0f32;
        let w_px = ((mb.width + 2.0 * pad) * SCALE).ceil().clamp(1.0, 16_384.0) as i32;
        let h_px = ((mb.ascent + mb.descent + 2.0 * pad) * SCALE)
            .ceil()
            .clamp(1.0, 16_384.0) as i32;
        let r = Renderer::new(self.dw, None, w_px, SCALE).ok()?;
        let pal = self.pal;
        let mut px = r.render_offscreen(h_px, pal.text, &mut |band| {
            band.clear(pal.bg);
            band.math(&mb, pad, pad + mb.ascent, pal.text);
        });
        opaque(&mut px);
        let (w, h) = (w_px as u32, h_px as u32);
        let bytes = crate::snapshot::encode_png(w, h, &px).ok()?;
        Some(Png {
            w,
            h,
            descent: (mb.descent + pad).round() as u32,
            bytes,
        })
    }

    fn diagram(&self, source: &str) -> Option<Png> {
        let md = format!("```mermaid\n{source}\n```\n");
        let mut content = Content::Markdown(Document::new(md, ReadOpts::default()));
        let mut viewer = Viewer::new(&content, HTML_COLUMN, self.m);
        let images = crate::images::Store::synchronous(&self.doc_path);
        let cache = yamdv_layout::rich::RichCache::default();
        let rich = crate::dwrite::Rich::new(self.dw, &cache, true, true);
        layout_all(
            self.dw,
            &mut content,
            &mut viewer,
            &self.pal,
            &images,
            &rich,
            self.lang,
            self.m,
        );
        let (w, h, bytes) =
            crate::snapshot::block_png(self.dw, &viewer, 0, &self.pal, SCALE, None)?;
        Some(Png {
            w,
            h,
            descent: 0,
            bytes,
        })
    }
}

fn opaque(px: &mut [u8]) {
    for p in px.as_chunks_mut::<4>().0 {
        p[3] = 255;
    }
}

fn unpremultiply(px: &mut [u8]) {
    for p in px.as_chunks_mut::<4>().0 {
        let a = u32::from(p[3]);
        if a != 0 && a != 255 {
            for c in &mut p[..3] {
                *c = ((u32::from(*c) * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
}

/// The exported HTML text of `job` (COM must be initialized on this thread).
pub fn html_text(dw: &DWrite, job: &Job) -> String {
    html_text_with(dw, job, &mut |_, _| true).unwrap_or_default()
}

/// [`html_text`] with a per-block progress callback (`false` cancels: `None`).
pub fn html_text_with(
    dw: &DWrite,
    job: &Job,
    progress: &mut dyn FnMut(usize, usize) -> bool,
) -> Option<String> {
    let s = crate::window::load_settings();
    // BUG-10: `export.html_colors` picks the palette; an explicit `--mode` sets it directly.
    let (pal, dark) = if job.mode_forced {
        let dark = job.dark;
        let base = crate::window::export_palette(&job.scheme, "theme", dark).0;
        (base, dark)
    } else {
        crate::window::export_palette(&job.scheme, s.str("export.html_colors"), job.dark)
    };
    let host = Host {
        dw,
        doc_path: job.path.clone(),
        pal,
        m: job.m,
        lang: job.lang,
        wic: crate::images::factory(),
    };
    let doc = Document::new(job.source.clone(), ReadOpts::default());
    yamdv_export::html::document_with(
        &doc,
        &host,
        &DocOpts {
            title: doc
                .index
                .headings
                .first()
                .map_or_else(|| job.stem(), |h| h.text.clone()),
            pal,
            dark,
            toc: s.bool("export.html_include_toc"),
        },
        progress,
    )
}

/// Writes the HTML export of `job` to `out` atomically (S-EXP-01).
pub fn export_html(dw: &DWrite, job: &Job, out: &str) -> bool {
    let text = html_text(dw, job);
    let ok = crate::shell::write_atomic(out, &text);
    drop(text);
    crate::sys::trim_once();
    ok
}

/// HTML export in the worker (BUG-11): progress per block, Cancel between blocks, then an atomic
/// write (nothing is left behind on Cancel or failure).
pub fn export_html_with(
    dw: &DWrite,
    job: &Job,
    out: &str,
    mut link: Option<&mut crate::job::Link>,
) -> Result<(), crate::print::Error> {
    use yamdv_core::progress::Phase;
    let mut report = |n: usize, total: usize| {
        link.as_deref_mut().is_none_or(|l| {
            l.report(
                Phase::Preparing,
                u32::try_from(n).unwrap_or(u32::MAX),
                u32::try_from(total).unwrap_or(u32::MAX),
            )
        })
    };
    let Some(text) = html_text_with(dw, job, &mut report) else {
        return Err(crate::print::Error::Cancelled);
    };
    if link
        .as_deref()
        .is_some_and(|l| crate::job::is_set(l.cancel_handle()))
    {
        return Err(crate::print::Error::Cancelled);
    }
    let ok = crate::shell::write_atomic(out, &text);
    drop(text);
    crate::sys::trim_once();
    if ok {
        Ok(())
    } else {
        let _ = std::fs::remove_file(format!("{out}.tmp"));
        Err(crate::print::Error::Failed)
    }
}

/// Copy/Export table as CSV (R-EXP-04): the table of block `bi` at source offset `at`.
pub fn table_csv(doc: &Document, bi: usize, at: u32) -> Option<String> {
    let rows = yamdv_export::csv::table_at(doc, bi, at)?;
    let guard = crate::window::load_settings().bool("export.csv_formula_guard");
    Some(yamdv_export::csv::to_csv(&rows, guard))
}

/// Exit codes of the headless CLI (R-EXP-05).
pub const EXIT_ERROR: i32 = 1;

/// `yamdv --export-html <in.md> [-o out.html] [--theme <scheme>] [--mode light|dark]`.
pub fn cli_html(input: &str, out: Option<&str>, scheme: Option<&str>, dark: Option<bool>) -> i32 {
    // SAFETY: COM for WIC on this thread.
    let _ = unsafe {
        windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        )
    };
    let Some(job) = Job::from_file(input, scheme, dark) else {
        return EXIT_ERROR;
    };
    let out = out.map_or_else(|| sibling(&job.path, "html"), str::to_string);
    let Ok(dw) = DWrite::new(&job.fonts.0, &job.fonts.1) else {
        return EXIT_ERROR;
    };
    if export_html(&dw, &job, &out) {
        0
    } else {
        EXIT_ERROR
    }
}

/// `<dir>\<stem>.<ext>` next to the document.
pub fn sibling(path: &str, ext: &str) -> String {
    let p = std::path::Path::new(path);
    p.with_file_name(format!("{}.{ext}", stem(path)))
        .to_string_lossy()
        .into_owned()
}
