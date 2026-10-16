//! The §4.7 reference scene: built once with DirectWrite, drawn by every backend.
//!
//! All geometry is in DIPs relative to the top-left of one scene instance; the instance is
//! repeated `repeat` times vertically. Colors are `0xRRGGBB`.

use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_ITALIC,
    DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_BOLD,
    DWRITE_FONT_WEIGHT_NORMAL, DWRITE_HIT_TEST_METRICS, DWRITE_LINE_SPACING_METHOD_UNIFORM,
    DWRITE_TEXT_METRICS, DWRITE_TEXT_RANGE, DWriteCreateFactory, IDWriteFactory,
    IDWriteFontCollection, IDWriteTextLayout,
};
use windows::core::{BOOL, HSTRING, w};

use crate::app::Log;

pub const X0: f32 = 40.0;
pub const COL_W: f32 = 760.0;
pub const TEXT: u32 = 0x333333;
pub const LINK: u32 = 0x2563eb;
pub const CODE_BG: u32 = 0xf5f5f5;
pub const CODE_BORDER: u32 = 0xe0e0e0;
pub const RULE: u32 = 0xdddddd;
pub const BG: u32 = 0xffffff;
pub const SEL: u32 = 0x2563eb;
pub const SEL_ALPHA: f32 = 0.25;
pub const TABLE_ROW_H: f32 = 36.0;
pub const TABLE_COL_W: f32 = 190.0;
pub const PRIM_H: f32 = 150.0;
pub const IMG_X: f32 = 520.0;
pub const IMG_W: f32 = 200.0;
pub const IMG_H: f32 = 150.0;

const HEADING: &str = "Reference scene — YAMDV backend spike";
const PARA: &str = "YAMDV is a reading-first viewer. Türkçe: ğüşıöçİ ĞÜŞIÖÇ — CJK: 中文 日本語 한국어 — \
Arabic: مرحبا بالعالم — Emoji: 👍🏽 🚀 👩‍💻 — with a bold span, an italic span and a \
link-colored span, wrapped at 760 px with font fallback across scripts in one paragraph.";
const CODE: &str = "fn main() {\n    // print a greeting\n    let name = \"Aurora\";\n    greet(name, 42);\n}\nfn greet(n: &str, x: u32) { println!(\"{n} {x}\"); }";
const CELLS: [&str; 12] = [
    "Scheme", "Light", "Dark", "Notes", "Default", "#ffffff", "#1e1e1e", "base", "GitHub",
    "#ffffff", "#0d1117", "extras",
];

/// A colored UTF-16 range of a layout.
#[derive(Clone, Copy)]
pub struct ColorRange {
    pub start: u32,
    pub len: u32,
    pub color: u32,
}

/// A text layout plus its position and color ranges.
pub struct Text {
    pub layout: IDWriteTextLayout,
    pub x: f32,
    pub y: f32,
    pub color: u32,
    pub ranges: Vec<ColorRange>,
}

impl Text {
    /// Color of the run starting at UTF-16 position `pos`.
    pub fn color_at(&self, pos: u32) -> u32 {
        self.ranges
            .iter()
            .find(|r| pos >= r.start && pos < r.start + r.len)
            .map_or(self.color, |r| r.color)
    }
}

/// Simple rectangle (DIPs).
#[derive(Clone, Copy, Default)]
pub struct R {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// The built scene.
pub struct Scene {
    pub dw: IDWriteFactory,
    pub scale: f32,
    pub repeat: u32,
    pub texts: Vec<Text>,
    pub selection: Vec<R>,
    pub code_box: R,
    pub table: R,
    pub prim_y: f32,
    pub height: f32,
    /// Decoded image, PBGRA, device pixels.
    pub img_w: u32,
    pub img_h: u32,
    pub img: Vec<u8>,
}

fn u16_range(text: &str, sub: &str) -> (u32, u32) {
    let b = text.find(sub).unwrap_or(0);
    let start = text[..b].encode_utf16().count() as u32;
    (start, sub.encode_utf16().count() as u32)
}

fn tr((start, len): (u32, u32)) -> DWRITE_TEXT_RANGE {
    DWRITE_TEXT_RANGE {
        startPosition: start,
        length: len,
    }
}

impl Scene {
    /// Total scrollable height in device pixels.
    pub fn total_px(&self) -> i32 {
        (self.height * self.repeat as f32 * self.scale).ceil() as i32
    }

    /// Instance height in device pixels (fractional).
    pub fn inst_px(&self) -> f32 {
        self.height * self.scale
    }

    /// Indices of instances intersecting device rows `[top, bottom)`.
    pub fn visible(&self, top: f32, bottom: f32) -> std::ops::Range<u32> {
        let h = self.inst_px();
        let a = (top / h).floor().max(0.0) as u32;
        let b = ((bottom / h).ceil().max(0.0) as u32).min(self.repeat);
        a.min(b)..b
    }

    /// Builds the scene: text layouts, geometry and the WIC-decoded image.
    pub fn new(scale: f32, repeat: u32, log: &mut Log) -> windows::core::Result<Self> {
        unsafe {
            let dw: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            log.pws("dwrite_factory");
            let mut coll: Option<IDWriteFontCollection> = None;
            dw.GetSystemFontCollection(&mut coll, false)?;
            let mono = {
                let mut idx = 0;
                let mut exists = BOOL(0);
                if let Some(c) = &coll {
                    let _ = c.FindFamilyName(w!("Cascadia Mono"), &mut idx, &mut exists);
                }
                if exists.as_bool() {
                    "Cascadia Mono"
                } else {
                    "Consolas"
                }
            };
            log.kv("font.mono", mono);
            let fmt = |family: &str, weight: DWRITE_FONT_WEIGHT, size: f32| {
                dw.CreateTextFormat(
                    &HSTRING::from(family),
                    None,
                    weight,
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    size,
                    w!("en-us"),
                )
            };
            let layout = |s: &str,
                          f: &windows::Win32::Graphics::DirectWrite::IDWriteTextFormat,
                          width: f32| {
                let v: Vec<u16> = s.encode_utf16().collect();
                dw.CreateTextLayout(&v, f, width, 10_000.0)
            };
            let height_of = |l: &IDWriteTextLayout| -> f32 {
                let mut m = DWRITE_TEXT_METRICS::default();
                let _ = l.GetMetrics(&mut m);
                m.height
            };

            let mut texts = Vec::new();
            let mut y = 24.0;

            // Heading.
            let hf = fmt("Segoe UI", DWRITE_FONT_WEIGHT(600), 32.0)?;
            let hl = layout(HEADING, &hf, COL_W)?;
            let hh = height_of(&hl);
            texts.push(Text {
                layout: hl,
                x: X0,
                y,
                color: TEXT,
                ranges: vec![],
            });
            y += hh + 16.0;

            // Paragraph: mixed scripts, bold/italic/link, line height 1.6.
            let pf = fmt("Segoe UI", DWRITE_FONT_WEIGHT_NORMAL, 16.0)?;
            pf.SetLineSpacing(DWRITE_LINE_SPACING_METHOD_UNIFORM, 25.6, 25.6 * 0.8)?;
            let pl = layout(PARA, &pf, COL_W)?;
            pl.SetFontWeight(DWRITE_FONT_WEIGHT_BOLD, tr(u16_range(PARA, "bold span")))?;
            pl.SetFontStyle(DWRITE_FONT_STYLE_ITALIC, tr(u16_range(PARA, "italic span")))?;
            let (ls, ll) = u16_range(PARA, "link-colored span");
            let (ss, sl) = u16_range(PARA, "reading-first viewer. Türkçe");
            let mut count = 0;
            let _ = pl.HitTestTextRange(ss, sl, 0.0, 0.0, None, &mut count);
            let mut hits = vec![DWRITE_HIT_TEST_METRICS::default(); count as usize];
            pl.HitTestTextRange(ss, sl, X0, y, Some(&mut hits), &mut count)?;
            let selection = hits
                .iter()
                .map(|m| R {
                    x: m.left,
                    y: m.top,
                    w: m.width,
                    h: m.height,
                })
                .collect();
            let ph = height_of(&pl);
            texts.push(Text {
                layout: pl,
                x: X0,
                y,
                color: TEXT,
                ranges: vec![ColorRange {
                    start: ls,
                    len: ll,
                    color: LINK,
                }],
            });
            y += ph + 16.0;

            // Code block: rounded box, monospace, four token colors.
            let cf = fmt(mono, DWRITE_FONT_WEIGHT_NORMAL, 14.0)?;
            cf.SetLineSpacing(DWRITE_LINE_SPACING_METHOD_UNIFORM, 22.4, 22.4 * 0.8)?;
            let cl = layout(CODE, &cf, COL_W - 32.0)?;
            let mut ranges = Vec::new();
            let mut add = |sub: &str, nth: usize, color: u32| {
                let mut from = 0;
                for _ in 0..nth {
                    from += CODE[from..].find(sub).map_or(0, |i| i + sub.len());
                }
                let b = from + CODE[from..].find(sub).unwrap_or(0);
                let start = CODE[..b].encode_utf16().count() as u32;
                ranges.push(ColorRange {
                    start,
                    len: sub.encode_utf16().count() as u32,
                    color,
                });
            };
            const KW: u32 = 0xd73a49;
            const STR: u32 = 0x032f62;
            const COM: u32 = 0x6a737d;
            const FUN: u32 = 0x6f42c1;
            add("fn", 0, KW);
            add("fn", 1, KW);
            add("let", 0, KW);
            add("main", 0, FUN);
            add("greet", 0, FUN);
            add("greet", 1, FUN);
            add("println!", 0, FUN);
            add("// print a greeting", 0, COM);
            add("\"Aurora\"", 0, STR);
            add("\"{n} {x}\"", 0, STR);
            let ch = height_of(&cl);
            let code_box = R {
                x: X0,
                y,
                w: COL_W,
                h: ch + 32.0,
            };
            texts.push(Text {
                layout: cl,
                x: X0 + 16.0,
                y: y + 16.0,
                color: 0x24292e,
                ranges,
            });
            y += code_box.h + 16.0;

            // Table 4×3 with bold header.
            let tf = fmt("Segoe UI", DWRITE_FONT_WEIGHT_NORMAL, 14.0)?;
            let table = R {
                x: X0,
                y,
                w: TABLE_COL_W * 4.0,
                h: TABLE_ROW_H * 3.0,
            };
            for (i, c) in CELLS.iter().enumerate() {
                let (row, col) = (i / 4, i % 4);
                let l = layout(c, &tf, TABLE_COL_W - 16.0)?;
                if row == 0 {
                    l.SetFontWeight(
                        DWRITE_FONT_WEIGHT_BOLD,
                        DWRITE_TEXT_RANGE {
                            startPosition: 0,
                            length: 100,
                        },
                    )?;
                }
                texts.push(Text {
                    layout: l,
                    x: X0 + col as f32 * TABLE_COL_W + 8.0,
                    y: y + row as f32 * TABLE_ROW_H + 8.0,
                    color: TEXT,
                    ranges: vec![],
                });
            }
            y += table.h + 24.0;
            let prim_y = y;
            y += PRIM_H + 24.0;
            log.pws("text_layouts");

            let img_w = (IMG_W * scale).round() as u32;
            let img_h = (IMG_H * scale).round() as u32;
            let img =
                crate::img::decode_png(&crate::img::generate_png(400, 300), img_w, img_h, log)?;

            Ok(Self {
                dw,
                scale,
                repeat,
                texts,
                selection,
                code_box,
                table,
                prim_y,
                height: y,
                img_w,
                img_h,
                img,
            })
        }
    }
}
