//! DirectWrite implementation of [`TextShaper`] (§4.4.1): one `IDWriteTextLayout` per paragraph
//! with styled runs, uniform line spacing (fallback fonts never change the line pitch), paragraph
//! direction, code tab stops and hit testing.
#![allow(unsafe_code)]
// Pixel/DIP geometry: float<->int conversions are bounded by window/bitmap sizes (ADR-0007).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::indexing_slicing
)]

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;

use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_ITALIC,
    DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT, DWRITE_HIT_TEST_METRICS,
    DWRITE_LINE_SPACING_METHOD_UNIFORM, DWRITE_READING_DIRECTION_RIGHT_TO_LEFT,
    DWRITE_TEXT_METRICS, DWRITE_TEXT_RANGE, DWRITE_WORD_WRAPPING_EMERGENCY_BREAK,
    DWRITE_WORD_WRAPPING_NO_WRAP, DWriteCreateFactory, IDWriteFactory, IDWriteFontCollection,
    IDWriteTextFormat, IDWriteTextLayout, IDWriteTextLayout1,
};
use windows::core::{BOOL, HSTRING, IUnknown, Interface, implement, w};
use yamdv_layout::Rect;
use yamdv_layout::shaper::{Family, ParaSpec, Shaped, Style, TextShaper};

/// Marker drawing effect: makes DirectWrite split glyph runs at style boundaries so the renderer
/// can color each run (the color itself is looked up by text position).
#[implement()]
struct RunBreak;

/// Shared DirectWrite state.
pub struct DWrite {
    /// Factory.
    pub factory: IDWriteFactory,
    body: HSTRING,
    code: HSTRING,
    formats: RefCell<HashMap<(bool, u32, u16, bool), IDWriteTextFormat>>,
    space: RefCell<HashMap<(bool, u32), f32>>,
    brk: IUnknown,
}

impl std::fmt::Debug for DWrite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DWrite")
    }
}

fn family_exists(coll: &IDWriteFontCollection, name: &HSTRING) -> bool {
    let mut idx = 0u32;
    let mut exists = BOOL(0);
    // SAFETY: valid collection; out-pointers on the stack.
    unsafe { coll.FindFamilyName(name, &mut idx, &mut exists) }.is_ok() && exists.as_bool()
}

impl DWrite {
    /// Creates the shared factory; resolves the body font and the code-font fallback chain
    /// (`code_font` → Cascadia Mono → Consolas → Courier New, B.2).
    pub fn new(body_font: &str, code_font: &str) -> windows::core::Result<Self> {
        // SAFETY: plain factory creation.
        let factory: IDWriteFactory = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? };
        let mut coll: Option<IDWriteFontCollection> = None;
        // SAFETY: valid out-pointer.
        unsafe { factory.GetSystemFontCollection(&mut coll, false)? };
        let pick = |cands: &[&str]| -> HSTRING {
            for c in cands {
                let h = HSTRING::from(*c);
                if coll.as_ref().is_some_and(|k| family_exists(k, &h)) {
                    return h;
                }
            }
            HSTRING::from(cands.last().copied().unwrap_or("Segoe UI"))
        };
        let body = pick(&[body_font, "Segoe UI"]);
        let code = pick(&[code_font, "Cascadia Mono", "Consolas", "Courier New"]);
        Ok(Self {
            factory,
            body,
            code,
            formats: RefCell::new(HashMap::new()),
            space: RefCell::new(HashMap::new()),
            brk: RunBreak.into(),
        })
    }

    fn format(&self, s: &Style) -> windows::core::Result<IDWriteTextFormat> {
        let code = s.family == Family::Code;
        let key = (code, s.size.to_bits(), s.weight, s.italic);
        if let Some(f) = self.formats.borrow().get(&key) {
            return Ok(f.clone());
        }
        let fam = if code { &self.code } else { &self.body };
        // SAFETY: valid factory and strings.
        let f = unsafe {
            self.factory.CreateTextFormat(
                fam,
                None,
                DWRITE_FONT_WEIGHT(i32::from(s.weight)),
                if s.italic {
                    DWRITE_FONT_STYLE_ITALIC
                } else {
                    DWRITE_FONT_STYLE_NORMAL
                },
                DWRITE_FONT_STRETCH_NORMAL,
                s.size,
                w!("en-us"),
            )?
        };
        self.formats.borrow_mut().insert(key, f.clone());
        Ok(f)
    }

    fn space_advance(&self, s: &Style) -> f32 {
        let key = (s.family == Family::Code, s.size.to_bits());
        if let Some(v) = self.space.borrow().get(&key) {
            return *v;
        }
        let v = self
            .format(s)
            .and_then(|f| {
                let t: Vec<u16> = "    ".encode_utf16().collect();
                // SAFETY: valid factory/format; the text buffer outlives the call.
                let l = unsafe { self.factory.CreateTextLayout(&t, &f, 10_000.0, 100.0)? };
                let mut m = DWRITE_TEXT_METRICS::default();
                // SAFETY: valid layout and out-pointer.
                unsafe { l.GetMetrics(&mut m)? };
                Ok(m.widthIncludingTrailingWhitespace / 4.0)
            })
            .unwrap_or(s.size * 0.55);
        self.space.borrow_mut().insert(key, v);
        v
    }

    /// Font warm-up task (F-10): creates the text format for `s` and lays out `sample`, which
    /// loads the font faces (and fallback fonts for non-Latin samples) into the DWrite caches.
    pub fn warm(&self, s: &Style, sample: &str) {
        let _ = self.space_advance(s);
        if let Ok(f) = self.format(s) {
            let t: Vec<u16> = sample.encode_utf16().collect();
            // SAFETY: valid factory/format; the text buffer outlives the calls.
            unsafe {
                if let Ok(l) = self.factory.CreateTextLayout(&t, &f, 1000.0, 100.0) {
                    let mut m = DWRITE_TEXT_METRICS::default();
                    let _ = l.GetMetrics(&mut m);
                }
            }
        }
    }

    /// Text width/height of a simple string (UI measurement).
    pub fn measure(&self, text: &str, size: f32, weight: u16, max_w: f32) -> (f32, f32) {
        let s = Style {
            family: Family::Body,
            size,
            weight,
            italic: false,
            color: 0,
            strike: false,
            underline: false,
        };
        let sh = self.shape_impl(&ParaSpec {
            text,
            base: s,
            spans: vec![],
            width: Some(max_w),
            line_pitch: (size * 1.4).round(),
            rtl: false,
            tab_spaces: None,
        });
        (sh.width, sh.height)
    }

    fn shape_impl(&self, spec: &ParaSpec<'_>) -> DwShaped {
        let text: Vec<u16> = spec.text.encode_utf16().collect();
        let len16 = u32::try_from(text.len()).unwrap_or(u32::MAX);
        let mut colors: Vec<(Range<u32>, Style)> = Vec::new();
        let r = (|| -> windows::core::Result<DwShaped> {
            let f = self.format(&spec.base)?;
            let maxw = spec.width.unwrap_or(100_000.0).max(1.0);
            // SAFETY: valid factory/format; text buffer outlives the call.
            let l: IDWriteTextLayout =
                unsafe { self.factory.CreateTextLayout(&text, &f, maxw, 100_000.0)? };
            // SAFETY: all calls below configure the layout we just created with in-range values.
            unsafe {
                l.SetLineSpacing(
                    DWRITE_LINE_SPACING_METHOD_UNIFORM,
                    spec.line_pitch,
                    spec.line_pitch * 0.8,
                )?;
                if spec.width.is_none() {
                    l.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
                } else {
                    // R-REN-07 / §4.3: a too-long token breaks anywhere instead of overflowing.
                    l.SetWordWrapping(DWRITE_WORD_WRAPPING_EMERGENCY_BREAK)?;
                }
                if spec.rtl {
                    // R-REN-19: RTL paragraph; with RTL reading direction the leading edge is the
                    // right edge.
                    l.SetReadingDirection(DWRITE_READING_DIRECTION_RIGHT_TO_LEFT)?;
                }
                if let Some(n) = spec.tab_spaces {
                    l.SetIncrementalTabStop(f32::from(n) * self.space_advance(&spec.base))?;
                }
                for sp in &spec.spans {
                    let range = DWRITE_TEXT_RANGE {
                        startPosition: sp.range.start,
                        length: sp.range.end.saturating_sub(sp.range.start),
                    };
                    let s = &sp.style;
                    if s.family != spec.base.family {
                        let fam = if s.family == Family::Code {
                            &self.code
                        } else {
                            &self.body
                        };
                        l.SetFontFamilyName(fam, range)?;
                    }
                    if (s.size - spec.base.size).abs() > 0.01 {
                        l.SetFontSize(s.size, range)?;
                    }
                    if s.weight != spec.base.weight {
                        l.SetFontWeight(DWRITE_FONT_WEIGHT(i32::from(s.weight)), range)?;
                    }
                    if s.italic != spec.base.italic {
                        l.SetFontStyle(
                            if s.italic {
                                DWRITE_FONT_STYLE_ITALIC
                            } else {
                                DWRITE_FONT_STYLE_NORMAL
                            },
                            range,
                        )?;
                    }
                    if s.strike {
                        l.SetStrikethrough(true, range)?;
                    }
                    if s.underline {
                        l.SetUnderline(true, range)?;
                    }
                    if s.family == Family::Code
                        && spec.base.family == Family::Body
                        && range.length > 0
                    {
                        // F-13 / P-21: inline-code padding as character spacing (leading on the
                        // first character, trailing on the last), so the padded background never
                        // overlaps neighbouring text; hit-testing stays exact.
                        inline_code_spacing(&l, &text, range, s.size / 0.875 * 0.4);
                    }
                    l.SetDrawingEffect(&self.brk, range)?;
                    colors.push((sp.range.clone(), *s));
                }
            }
            if spec.base.strike || spec.base.underline {
                let range = DWRITE_TEXT_RANGE {
                    startPosition: 0,
                    length: len16,
                };
                // SAFETY: as above.
                unsafe {
                    l.SetStrikethrough(spec.base.strike, range)?;
                    l.SetUnderline(spec.base.underline, range)?;
                }
            }
            let mut m = DWRITE_TEXT_METRICS::default();
            // SAFETY: valid layout and out-pointer.
            unsafe { l.GetMetrics(&mut m)? };
            let lines = m.lineCount.max(1) as usize;
            Ok(DwShaped {
                layout: Some(l),
                width: m.widthIncludingTrailingWhitespace,
                height: lines as f32 * spec.line_pitch,
                lines,
                base: spec.base,
                runs: std::mem::take(&mut colors),
                pitch: spec.line_pitch,
            })
        })();
        r.unwrap_or(DwShaped {
            layout: None,
            width: 0.0,
            height: spec.line_pitch,
            lines: 1,
            base: spec.base,
            runs: Vec::new(),
            pitch: spec.line_pitch,
        })
    }
}

/// A shaped paragraph backed by an `IDWriteTextLayout`.
#[derive(Debug)]
pub struct DwShaped {
    /// The layout (`None` if DirectWrite failed; then nothing is drawn).
    pub layout: Option<IDWriteTextLayout>,
    width: f32,
    height: f32,
    lines: usize,
    /// Base style.
    pub base: Style,
    /// Styled ranges (UTF-16) — colors and decorations for the renderer.
    pub runs: Vec<(Range<u32>, Style)>,
    pitch: f32,
}

impl DwShaped {
    /// Style at a UTF-16 position (last matching span wins).
    pub fn style_at(&self, pos: u32) -> Style {
        self.runs
            .iter()
            .rev()
            .find(|r| r.0.contains(&pos))
            .map_or(self.base, |r| r.1)
    }
}

impl Shaped for DwShaped {
    fn width(&self) -> f32 {
        self.width
    }
    fn height(&self) -> f32 {
        self.height
    }
    fn lines(&self) -> usize {
        self.lines
    }
    fn hit(&self, x: f32, y: f32) -> u32 {
        let Some(l) = &self.layout else { return 0 };
        let mut trailing = BOOL(0);
        let mut inside = BOOL(0);
        let mut m = DWRITE_HIT_TEST_METRICS::default();
        // SAFETY: valid layout; out-pointers on the stack.
        if unsafe {
            l.HitTestPoint(
                x,
                y.clamp(0.0, (self.height - 0.5).max(0.0)),
                &mut trailing,
                &mut inside,
                &mut m,
            )
        }
        .is_err()
        {
            return 0;
        }
        m.textPosition + if trailing.as_bool() { m.length } else { 0 }
    }
    fn range_rects(&self, range: Range<u32>) -> Vec<Rect> {
        let Some(l) = &self.layout else {
            return Vec::new();
        };
        if range.end <= range.start {
            return Vec::new();
        }
        let mut count = 0u32;
        let len = range.end - range.start;
        // SAFETY: first call queries the count (expected to fail with E_NOT_SUFFICIENT_BUFFER).
        let _ = unsafe { l.HitTestTextRange(range.start, len, 0.0, 0.0, None, &mut count) };
        let mut buf = vec![DWRITE_HIT_TEST_METRICS::default(); count as usize];
        if count == 0 {
            return Vec::new();
        }
        // SAFETY: buffer sized from the first call.
        let r =
            unsafe { l.HitTestTextRange(range.start, len, 0.0, 0.0, Some(&mut buf), &mut count) };
        if r.is_err() {
            return Vec::new();
        }
        // Line boxes: snap to the uniform pitch so selection has no gaps between lines (B.6.13).
        buf.iter()
            .take(count as usize)
            .map(|m| {
                let line = (m.top / self.pitch).round();
                Rect::new(m.left, line * self.pitch, m.width, self.pitch)
            })
            .collect()
    }
    fn caret(&self, idx: u32) -> Rect {
        let Some(l) = &self.layout else {
            return Rect::new(0.0, 0.0, 0.0, self.pitch);
        };
        let (mut x, mut y) = (0.0f32, 0.0f32);
        let mut m = DWRITE_HIT_TEST_METRICS::default();
        // SAFETY: valid layout; out-pointers on the stack. Positions past the end are clamped by
        // DirectWrite (the caret after the last character).
        if unsafe { l.HitTestTextPosition(idx, false, &mut x, &mut y, &mut m) }.is_err() {
            return Rect::new(0.0, 0.0, 0.0, self.pitch);
        }
        let line = (m.top / self.pitch).round();
        Rect::new(x, line * self.pitch, 0.0, self.pitch)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Applies `pad` DIPs of leading spacing to the first character of `range` and trailing spacing
/// to its last character (surrogate pairs count as one character).
fn inline_code_spacing(l: &IDWriteTextLayout, text: &[u16], range: DWRITE_TEXT_RANGE, pad: f32) {
    let Ok(l1) = l.cast::<IDWriteTextLayout1>() else {
        return;
    };
    let start = range.startPosition as usize;
    let end = start + range.length as usize;
    let is_hi = |i: usize| text.get(i).is_some_and(|c| (0xD800..0xDC00).contains(c));
    let first_len = if is_hi(start) && end - start >= 2 {
        2
    } else {
        1
    };
    let last_len = if end - start >= 2 && is_hi(end - 2) {
        2
    } else {
        1
    };
    let r = |p: usize, n: usize| DWRITE_TEXT_RANGE {
        startPosition: p as u32,
        length: n as u32,
    };
    // SAFETY: valid layout; ranges lie inside the text.
    unsafe {
        if end - start <= first_len {
            let _ = l1.SetCharacterSpacing(pad, pad, 0.0, r(start, end - start));
        } else {
            let _ = l1.SetCharacterSpacing(pad, 0.0, 0.0, r(start, first_len));
            let _ = l1.SetCharacterSpacing(0.0, pad, 0.0, r(end - last_len, last_len));
        }
    }
}

/// [`TextShaper`] over the shared [`DWrite`] state.
#[derive(Debug)]
pub struct DwShaper<'a>(pub &'a DWrite);

impl TextShaper for DwShaper<'_> {
    fn shape(&mut self, spec: &ParaSpec<'_>) -> Box<dyn Shaped> {
        Box::new(self.0.shape_impl(spec))
    }
}
