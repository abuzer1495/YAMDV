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
use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::ops::Range;

use windows::Win32::Graphics::DirectWrite::{
    DWRITE_BREAK_CONDITION, DWRITE_BREAK_CONDITION_NEUTRAL, DWRITE_FACTORY_TYPE_SHARED,
    DWRITE_FONT_METRICS, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_ITALIC,
    DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_GLYPH_METRICS,
    DWRITE_HIT_TEST_METRICS, DWRITE_INLINE_OBJECT_METRICS, DWRITE_LINE_SPACING_METHOD_UNIFORM,
    DWRITE_OVERHANG_METRICS, DWRITE_READING_DIRECTION_RIGHT_TO_LEFT, DWRITE_TEXT_METRICS,
    DWRITE_TEXT_RANGE, DWRITE_WORD_WRAPPING_EMERGENCY_BREAK, DWRITE_WORD_WRAPPING_NO_WRAP,
    DWriteCreateFactory, IDWriteFactory, IDWriteFont1, IDWriteFontCollection, IDWriteFontFace,
    IDWriteInlineObject, IDWriteInlineObject_Impl, IDWriteTextFormat, IDWriteTextLayout,
    IDWriteTextLayout1, IDWriteTextRenderer,
};
use windows::core::{BOOL, HSTRING, IUnknown, Interface, Ref, implement, w};
use yamdv_layout::Rect;
use yamdv_layout::rich::{MathFace, RichCache, RichHost};
use yamdv_layout::shaper::{Family, ParaSpec, Shaped, Style, TextShaper};
use yamdv_math::font::{GlyphMetrics, MathFont};

/// Marker drawing effect: makes DirectWrite split glyph runs at style boundaries so the renderer
/// can color each run (the color itself is looked up by text position).
#[implement()]
struct RunBreak;

/// Reserved space for an inline formula (R-MATH-04).
#[implement(IDWriteInlineObject)]
struct InlineBox(DWRITE_INLINE_OBJECT_METRICS);

impl IDWriteInlineObject_Impl for InlineBox_Impl {
    fn Draw(
        &self,
        _: *const core::ffi::c_void,
        _: Ref<IDWriteTextRenderer>,
        _: f32,
        _: f32,
        _: BOOL,
        _: BOOL,
        _: Ref<IUnknown>,
    ) -> windows::core::Result<()> {
        Ok(())
    }
    fn GetMetrics(&self) -> windows::core::Result<DWRITE_INLINE_OBJECT_METRICS> {
        Ok(self.0)
    }
    fn GetOverhangMetrics(&self) -> windows::core::Result<DWRITE_OVERHANG_METRICS> {
        Ok(DWRITE_OVERHANG_METRICS::default())
    }
    fn GetBreakConditions(
        &self,
        before: *mut DWRITE_BREAK_CONDITION,
        after: *mut DWRITE_BREAK_CONDITION,
    ) -> windows::core::Result<()> {
        // SAFETY: DirectWrite passes valid out-pointers.
        unsafe {
            if let Some(b) = before.as_mut() {
                *b = DWRITE_BREAK_CONDITION_NEUTRAL;
            }
            if let Some(a) = after.as_mut() {
                *a = DWRITE_BREAK_CONDITION_NEUTRAL;
            }
        }
        Ok(())
    }
}

/// The OpenType math font (Cambria Math, R-MATH-01) behind [`MathFont`].
pub struct DwMathFont {
    face: IDWriteFontFace,
    upem: f32,
    math: Vec<u8>,
    gids: RefCell<HashMap<char, u16>>,
    metrics: RefCell<HashMap<u16, GlyphMetrics>>,
}

impl DwMathFont {
    fn load(factory: &IDWriteFactory) -> Option<Self> {
        let mut coll: Option<IDWriteFontCollection> = None;
        // SAFETY: valid factory; out-pointers on the stack; the table pointer is copied before
        // `ReleaseFontTable`.
        unsafe {
            factory.GetSystemFontCollection(&mut coll, false).ok()?;
            let coll = coll?;
            let (mut idx, mut exists) = (0u32, BOOL(0));
            coll.FindFamilyName(w!("Cambria Math"), &mut idx, &mut exists)
                .ok()?;
            if !exists.as_bool() {
                return None;
            }
            let font = coll
                .GetFontFamily(idx)
                .ok()?
                .GetFirstMatchingFont(
                    DWRITE_FONT_WEIGHT_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    DWRITE_FONT_STYLE_NORMAL,
                )
                .ok()?;
            let face = font.CreateFontFace().ok()?;
            let mut fm = DWRITE_FONT_METRICS::default();
            face.GetMetrics(&mut fm);
            let tag = u32::from_le_bytes(*b"MATH");
            let mut data = std::ptr::null_mut();
            let mut size = 0u32;
            let mut ctx = std::ptr::null_mut();
            let mut has = BOOL(0);
            face.TryGetFontTable(tag, &mut data, &mut size, &mut ctx, &mut has)
                .ok()?;
            let math = if has.as_bool() && !data.is_null() {
                let v = std::slice::from_raw_parts(data.cast::<u8>(), size as usize).to_vec();
                face.ReleaseFontTable(ctx);
                v
            } else {
                return None;
            };
            Some(Self {
                face,
                upem: f32::from(fm.designUnitsPerEm.max(1)),
                math,
                gids: RefCell::new(HashMap::new()),
                metrics: RefCell::new(HashMap::new()),
            })
        }
    }
}

impl MathFont for DwMathFont {
    fn units_per_em(&self) -> f32 {
        self.upem
    }
    fn glyph(&self, c: char) -> u16 {
        if let Some(g) = self.gids.borrow().get(&c) {
            return *g;
        }
        let cp = u32::from(c);
        let mut g = 0u16;
        // SAFETY: one code point in, one glyph index out.
        let _ = unsafe { self.face.GetGlyphIndices(&cp, 1, &mut g) };
        self.gids.borrow_mut().insert(c, g);
        g
    }
    fn metrics(&self, g: u16) -> GlyphMetrics {
        if let Some(m) = self.metrics.borrow().get(&g) {
            return *m;
        }
        let mut d = DWRITE_GLYPH_METRICS::default();
        // SAFETY: one glyph index in, one metrics record out.
        let _ = unsafe { self.face.GetDesignGlyphMetrics(&g, 1, &mut d, false) };
        let adv = d.advanceWidth as f32;
        let vo = d.verticalOriginY as f32;
        let m = GlyphMetrics {
            advance: adv,
            x_min: d.leftSideBearing as f32,
            x_max: adv - d.rightSideBearing as f32,
            y_max: vo - d.topSideBearing as f32,
            y_min: vo - d.advanceHeight as f32 + d.bottomSideBearing as f32,
        };
        self.metrics.borrow_mut().insert(g, m);
        m
    }
    fn math_table(&self) -> &[u8] {
        &self.math
    }
}

thread_local! {
    /// The loaded math face for drawing (set when [`DWrite::math_face`] first loads it).
    static MATH_DRAW: RefCell<Option<IDWriteFontFace>> = const { RefCell::new(None) };
}

/// The math font face for glyph-run drawing (`None` until loaded or when unavailable).
pub fn math_draw_face() -> Option<IDWriteFontFace> {
    MATH_DRAW.with(|m| m.borrow().clone())
}

/// [`RichHost`] over the shared [`DWrite`] state and a document's cache.
#[derive(Debug)]
pub struct Rich<'a> {
    dw: &'a DWrite,
    cache: &'a RichCache,
    math: bool,
    mermaid: bool,
}

impl<'a> Rich<'a> {
    /// Host for one document with the `reading.render_math` / `reading.render_mermaid` flags.
    pub fn new(dw: &'a DWrite, cache: &'a RichCache, math: bool, mermaid: bool) -> Self {
        Self {
            dw,
            cache,
            math,
            mermaid,
        }
    }
}

impl RichHost for Rich<'_> {
    fn math(&self) -> Option<&MathFace> {
        self.dw.math_face()
    }
    fn render_math(&self) -> bool {
        self.math
    }
    fn render_mermaid(&self) -> bool {
        self.mermaid
    }
    fn cache(&self) -> Option<&RichCache> {
        Some(self.cache)
    }
    fn now_local(&self) -> Option<i64> {
        use windows::Win32::System::SystemInformation::GetLocalTime;
        // SAFETY: GetLocalTime has no preconditions.
        let t = unsafe { GetLocalTime() };
        let day = yamdv_mermaid::gantt::days_from_civil(
            i64::from(t.wYear),
            u32::from(t.wMonth),
            u32::from(t.wDay),
        );
        Some(day * 86_400 + i64::from(t.wHour) * 3600 + i64::from(t.wMinute) * 60)
    }
}

/// Shared DirectWrite state.
pub struct DWrite {
    /// Factory.
    pub factory: IDWriteFactory,
    body: HSTRING,
    code: HSTRING,
    formats: RefCell<HashMap<(bool, u32, u16, bool), IDWriteTextFormat>>,
    space: RefCell<HashMap<(bool, u32), f32>>,
    brk: IUnknown,
    math: OnceCell<Option<MathFace>>,
    /// Caption / chrome icon font (B.4): Segoe Fluent Icons → Segoe MDL2 Assets → none.
    icon: OnceCell<Option<HSTRING>>,
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
            math: OnceCell::new(),
            icon: OnceCell::new(),
        })
    }

    /// The math font, loaded on first use (R-MATH-01; §2.1 lazy).
    pub fn math_face(&self) -> Option<&MathFace> {
        self.math
            .get_or_init(|| {
                let f = DwMathFont::load(&self.factory)?;
                let face = f.face.clone();
                MATH_DRAW.with(|m| *m.borrow_mut() = Some(face));
                Some(MathFace::new(Box::new(f)))
            })
            .as_ref()
    }

    /// The chrome icon font family, if installed (B.4: Segoe Fluent Icons, then Segoe MDL2
    /// Assets; `None` → vector fallback).
    pub fn icon_family(&self) -> Option<&HSTRING> {
        self.icon
            .get_or_init(|| {
                let mut coll: Option<IDWriteFontCollection> = None;
                // SAFETY: valid factory and out-pointer.
                unsafe { self.factory.GetSystemFontCollection(&mut coll, false) }.ok()?;
                let coll = coll?;
                ["Segoe Fluent Icons", "Segoe MDL2 Assets"]
                    .into_iter()
                    .map(HSTRING::from)
                    .find(|h| family_exists(&coll, h))
            })
            .as_ref()
    }

    /// One icon-font glyph (`text`) at `size` DIPs in `color`; `None` without an icon font.
    pub fn shape_icon(&self, text: &str, size: f32, color: u32) -> Option<DwShaped> {
        let fam = self.icon_family()?.clone();
        self.shape_in(&fam, text, size, color)
    }

    /// `text` set in the font family `family` (font picker rows, FUT-49).
    pub fn shape_family(
        &self,
        family: &str,
        text: &str,
        size: f32,
        color: u32,
    ) -> Option<DwShaped> {
        self.shape_in(&HSTRING::from(family), text, size, color)
    }

    /// The installed font families (DirectWrite system collection), sorted, each with its
    /// monospace flag (`IDWriteFont1::IsMonospacedFont`). Called only when a picker opens.
    pub fn font_families(&self) -> Vec<(String, bool)> {
        let mut out = Vec::new();
        let mut coll: Option<IDWriteFontCollection> = None;
        // SAFETY: valid factory and out-pointer.
        if unsafe { self.factory.GetSystemFontCollection(&mut coll, false) }.is_err() {
            return out;
        }
        let Some(coll) = coll else { return out };
        // SAFETY: valid collection.
        let n = unsafe { coll.GetFontFamilyCount() };
        for i in 0..n {
            // SAFETY: index below the count.
            let Ok(fam) = (unsafe { coll.GetFontFamily(i) }) else {
                continue;
            };
            // SAFETY: valid family.
            let Ok(names) = (unsafe { fam.GetFamilyNames() }) else {
                continue;
            };
            let mut idx = 0u32;
            let mut found = BOOL(0);
            // SAFETY: valid strings; out-pointers on the stack.
            let _ = unsafe { names.FindLocaleName(w!("en-us"), &mut idx, &mut found) };
            if !found.as_bool() {
                idx = 0;
            }
            // SAFETY: index from FindLocaleName or 0; buffer sized from GetStringLength.
            let name = unsafe {
                let Ok(len) = names.GetStringLength(idx) else {
                    continue;
                };
                let mut buf = vec![0u16; len as usize + 1];
                if names.GetString(idx, &mut buf).is_err() {
                    continue;
                }
                String::from_utf16_lossy(&buf[..len as usize])
            };
            if name.is_empty() || name.starts_with('@') {
                continue;
            }
            // SAFETY: valid family; the cast only queries an interface.
            let mono = unsafe {
                fam.GetFirstMatchingFont(
                    DWRITE_FONT_WEIGHT_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    DWRITE_FONT_STYLE_NORMAL,
                )
            }
            .ok()
            .and_then(|f| f.cast::<IDWriteFont1>().ok())
            // SAFETY: valid font.
            .is_some_and(|f| unsafe { f.IsMonospacedFont() }.as_bool());
            out.push((name, mono));
        }
        out.sort_by_key(|a| a.0.to_lowercase());
        out.dedup_by(|a, b| a.0 == b.0);
        out
    }

    /// Whether `name` is an installed family (the picker shows the resolved name otherwise).
    pub fn has_family(&self, name: &str) -> bool {
        let mut coll: Option<IDWriteFontCollection> = None;
        // SAFETY: valid factory and out-pointer.
        unsafe { self.factory.GetSystemFontCollection(&mut coll, false) }.is_ok()
            && coll.is_some_and(|c| family_exists(&c, &HSTRING::from(name)))
    }

    /// Resolved body / code family names.
    pub fn resolved(&self) -> (String, String) {
        (self.body.to_string(), self.code.to_string())
    }

    fn shape_in(&self, fam: &HSTRING, text: &str, size: f32, color: u32) -> Option<DwShaped> {
        let base = Style {
            family: Family::Body,
            size,
            weight: 400,
            italic: false,
            color,
            strike: false,
            underline: false,
        };
        let mut sh = self.shape_impl(&ParaSpec {
            text,
            base,
            spans: vec![],
            width: None,
            line_pitch: size.ceil(),
            rtl: false,
            tab_spaces: None,
            objects: Vec::new(),
        });
        let l = sh.layout.as_ref()?;
        let len = u32::try_from(text.encode_utf16().count()).unwrap_or(0);
        let mut m = DWRITE_TEXT_METRICS::default();
        // SAFETY: valid layout; range within the text; out-pointer on the stack.
        unsafe {
            l.SetFontFamilyName(
                fam,
                DWRITE_TEXT_RANGE {
                    startPosition: 0,
                    length: len,
                },
            )
            .ok()?;
            l.GetMetrics(&mut m).ok()?;
        }
        sh.width = m.widthIncludingTrailingWhitespace;
        sh.height = m.height.max(size);
        Some(sh)
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
            objects: Vec::new(),
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
                // R-MATH-04: inline formulas are atomic, baseline-aligned inline objects; the
                // formula itself is drawn from the display list (`Item::Math`).
                for o in &spec.objects {
                    let obj: IDWriteInlineObject = InlineBox(DWRITE_INLINE_OBJECT_METRICS {
                        width: o.width,
                        height: o.ascent + o.descent,
                        baseline: o.ascent,
                        supportsSideways: BOOL(0),
                    })
                    .into();
                    l.SetInlineObject(
                        &obj,
                        DWRITE_TEXT_RANGE {
                            startPosition: o.range.0,
                            length: o.range.1.saturating_sub(o.range.0),
                        },
                    )?;
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

impl std::fmt::Debug for DwMathFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DwMathFont")
    }
}
