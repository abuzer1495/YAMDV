//! OpenType `MATH` table reader (§2.1): `MathConstants`, `MathGlyphInfo` (italics correction,
//! top-accent attachment) and `MathVariants` (size variants and glyph assemblies).
//!
//! Every read is bounds-checked; a missing or malformed table falls back to built-in defaults
//! (fractions of an em), so layout never fails because of font data.

/// Index of a `MathConstants` value (see [`Constants::get`]).
pub type C = usize;

/// `scriptPercentScaleDown`.
pub const SCRIPT_PERCENT: C = 0;
/// `scriptScriptPercentScaleDown`.
pub const SCRIPT_SCRIPT_PERCENT: C = 1;
/// `delimitedSubFormulaMinHeight`.
pub const DELIMITED_SUB_FORMULA_MIN_HEIGHT: C = 2;
/// `displayOperatorMinHeight`.
pub const DISPLAY_OPERATOR_MIN_HEIGHT: C = 3;
/// `mathLeading`.
pub const MATH_LEADING: C = 4;
/// `axisHeight`.
pub const AXIS_HEIGHT: C = 5;
/// `accentBaseHeight`.
pub const ACCENT_BASE_HEIGHT: C = 6;
/// `flattenedAccentBaseHeight`.
pub const FLATTENED_ACCENT_BASE_HEIGHT: C = 7;
/// `subscriptShiftDown`.
pub const SUBSCRIPT_SHIFT_DOWN: C = 8;
/// `subscriptTopMax`.
pub const SUBSCRIPT_TOP_MAX: C = 9;
/// `subscriptBaselineDropMin`.
pub const SUBSCRIPT_BASELINE_DROP_MIN: C = 10;
/// `superscriptShiftUp`.
pub const SUPERSCRIPT_SHIFT_UP: C = 11;
/// `superscriptShiftUpCramped`.
pub const SUPERSCRIPT_SHIFT_UP_CRAMPED: C = 12;
/// `superscriptBottomMin`.
pub const SUPERSCRIPT_BOTTOM_MIN: C = 13;
/// `superscriptBaselineDropMax`.
pub const SUPERSCRIPT_BASELINE_DROP_MAX: C = 14;
/// `subSuperscriptGapMin`.
pub const SUB_SUPERSCRIPT_GAP_MIN: C = 15;
/// `superscriptBottomMaxWithSubscript`.
pub const SUPERSCRIPT_BOTTOM_MAX_WITH_SUBSCRIPT: C = 16;
/// `spaceAfterScript`.
pub const SPACE_AFTER_SCRIPT: C = 17;
/// `upperLimitGapMin`.
pub const UPPER_LIMIT_GAP_MIN: C = 18;
/// `upperLimitBaselineRiseMin`.
pub const UPPER_LIMIT_BASELINE_RISE_MIN: C = 19;
/// `lowerLimitGapMin`.
pub const LOWER_LIMIT_GAP_MIN: C = 20;
/// `lowerLimitBaselineDropMin`.
pub const LOWER_LIMIT_BASELINE_DROP_MIN: C = 21;
/// `stackTopShiftUp`.
pub const STACK_TOP_SHIFT_UP: C = 22;
/// `stackTopDisplayStyleShiftUp`.
pub const STACK_TOP_DISPLAY_STYLE_SHIFT_UP: C = 23;
/// `stackBottomShiftDown`.
pub const STACK_BOTTOM_SHIFT_DOWN: C = 24;
/// `stackBottomDisplayStyleShiftDown`.
pub const STACK_BOTTOM_DISPLAY_STYLE_SHIFT_DOWN: C = 25;
/// `stackGapMin`.
pub const STACK_GAP_MIN: C = 26;
/// `stackDisplayStyleGapMin`.
pub const STACK_DISPLAY_STYLE_GAP_MIN: C = 27;
/// `fractionNumeratorShiftUp`.
pub const FRACTION_NUMERATOR_SHIFT_UP: C = 32;
/// `fractionNumeratorDisplayStyleShiftUp`.
pub const FRACTION_NUMERATOR_DISPLAY_STYLE_SHIFT_UP: C = 33;
/// `fractionDenominatorShiftDown`.
pub const FRACTION_DENOMINATOR_SHIFT_DOWN: C = 34;
/// `fractionDenominatorDisplayStyleShiftDown`.
pub const FRACTION_DENOMINATOR_DISPLAY_STYLE_SHIFT_DOWN: C = 35;
/// `fractionNumeratorGapMin`.
pub const FRACTION_NUMERATOR_GAP_MIN: C = 36;
/// `fractionNumDisplayStyleGapMin`.
pub const FRACTION_NUM_DISPLAY_STYLE_GAP_MIN: C = 37;
/// `fractionRuleThickness`.
pub const FRACTION_RULE_THICKNESS: C = 38;
/// `fractionDenominatorGapMin`.
pub const FRACTION_DENOMINATOR_GAP_MIN: C = 39;
/// `fractionDenomDisplayStyleGapMin`.
pub const FRACTION_DENOM_DISPLAY_STYLE_GAP_MIN: C = 40;
/// `overbarVerticalGap`.
pub const OVERBAR_VERTICAL_GAP: C = 43;
/// `overbarRuleThickness`.
pub const OVERBAR_RULE_THICKNESS: C = 44;
/// `overbarExtraAscender`.
pub const OVERBAR_EXTRA_ASCENDER: C = 45;
/// `underbarVerticalGap`.
pub const UNDERBAR_VERTICAL_GAP: C = 46;
/// `underbarRuleThickness`.
pub const UNDERBAR_RULE_THICKNESS: C = 47;
/// `underbarExtraDescender`.
pub const UNDERBAR_EXTRA_DESCENDER: C = 48;
/// `radicalVerticalGap`.
pub const RADICAL_VERTICAL_GAP: C = 49;
/// `radicalDisplayStyleVerticalGap`.
pub const RADICAL_DISPLAY_STYLE_VERTICAL_GAP: C = 50;
/// `radicalRuleThickness`.
pub const RADICAL_RULE_THICKNESS: C = 51;
/// `radicalExtraAscender`.
pub const RADICAL_EXTRA_ASCENDER: C = 52;
/// `radicalKernBeforeDegree`.
pub const RADICAL_KERN_BEFORE_DEGREE: C = 53;
/// `radicalKernAfterDegree`.
pub const RADICAL_KERN_AFTER_DEGREE: C = 54;
/// `radicalDegreeBottomRaisePercent`.
pub const RADICAL_DEGREE_BOTTOM_RAISE_PERCENT: C = 55;

/// Number of `MathConstants` fields.
pub const N_CONSTANTS: usize = 56;

/// Defaults in thousandths of an em (percentages for indices 0, 1 and 55).
const DEFAULTS: [i16; N_CONSTANTS] = [
    70, 50, 1300, 1300, // percents, delimited min, display op min
    150, 250, 450, 650, // leading, axis, accent base, flattened
    150, 350, 50, 360, 290, 110, 250, 160, 360, 40, // scripts
    100, 300, 100, 600, // limits
    440, 680, 350, 690, 120, 280, // stacks
    300, 600, 200, 200, // stretch stacks
    390, 680, 340, 690, 40, 120, 40, 40, 120, // fractions
    350, 100, // skewed
    120, 40, 40, 120, 40, 40, // over/under bar
    50, 150, 40, 40, 280, -550, // radical
    60,
];

fn be16(b: &[u8], at: usize) -> Option<u16> {
    let s = b.get(at..at.checked_add(2)?)?;
    Some(u16::from_be_bytes([*s.first()?, *s.get(1)?]))
}

fn bei16(b: &[u8], at: usize) -> Option<i16> {
    be16(b, at).map(|v| i16::from_be_bytes(v.to_be_bytes()))
}

/// `MathConstants` in font units (percentages for the two script scale-downs and the radical
/// degree raise).
#[derive(Debug, Clone, PartialEq)]
pub struct Constants {
    v: [f32; N_CONSTANTS],
}

impl Constants {
    /// Value of constant `c` (0 for an unknown index).
    pub fn get(&self, c: C) -> f32 {
        self.v.get(c).copied().unwrap_or(0.0)
    }
}

/// One size variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Variant {
    /// Glyph.
    pub glyph: u16,
    /// Advance along the stretch axis (font units).
    pub advance: f32,
}

/// One assembly part.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Part {
    /// Glyph.
    pub glyph: u16,
    /// Connector length at the start.
    pub start: f32,
    /// Connector length at the end.
    pub end: f32,
    /// Full advance along the axis.
    pub full: f32,
    /// Extender (may repeat).
    pub extender: bool,
}

/// Parsed `MATH` table (the raw bytes are kept; lookups read them on demand).
#[derive(Debug, Clone)]
pub struct MathTable {
    raw: Vec<u8>,
    upem: f32,
    /// Constants (defaults where the table is missing).
    pub constants: Constants,
    glyph_info: usize,
    variants: usize,
}

/// Coverage index of glyph `g` in the coverage table at `at`.
fn coverage(b: &[u8], at: usize, g: u16) -> Option<usize> {
    match be16(b, at)? {
        1 => {
            let n = usize::from(be16(b, at + 2)?);
            let (mut lo, mut hi) = (0usize, n);
            while lo < hi {
                let mid = lo + (hi - lo) / 2;
                let v = be16(b, at + 4 + mid * 2)?;
                match v.cmp(&g) {
                    std::cmp::Ordering::Equal => return Some(mid),
                    std::cmp::Ordering::Less => lo = mid + 1,
                    std::cmp::Ordering::Greater => hi = mid,
                }
            }
            None
        }
        2 => {
            let n = usize::from(be16(b, at + 2)?);
            let (mut lo, mut hi) = (0usize, n);
            while lo < hi {
                let mid = lo + (hi - lo) / 2;
                let r = at + 4 + mid * 6;
                let (s, e, idx) = (be16(b, r)?, be16(b, r + 2)?, be16(b, r + 4)?);
                if g < s {
                    hi = mid;
                } else if g > e {
                    lo = mid + 1;
                } else {
                    return Some(usize::from(idx) + usize::from(g - s));
                }
            }
            None
        }
        _ => None,
    }
}

impl MathTable {
    /// Parses `raw` for a font with `upem` units per em.
    pub fn new(raw: &[u8], upem: f32) -> Self {
        let upem = if upem.is_finite() && upem > 0.0 {
            upem
        } else {
            1000.0
        };
        let mut v = [0.0f32; N_CONSTANTS];
        for ((i, d), slot) in DEFAULTS.iter().enumerate().zip(v.iter_mut()) {
            *slot = if matches!(i, 0 | 1 | 55) {
                f32::from(*d)
            } else {
                f32::from(*d) * upem / 1000.0
            };
        }
        let mut t = Self {
            raw: Vec::new(),
            upem,
            constants: Constants { v },
            glyph_info: 0,
            variants: 0,
        };
        if be16(raw, 0) != Some(1) {
            return t;
        }
        let consts = usize::from(be16(raw, 4).unwrap_or(0));
        if consts != 0 && raw.len() >= consts + 214 {
            for (i, slot) in t.constants.v.iter_mut().enumerate() {
                let at = match i {
                    0..=3 => consts + 2 * i,
                    55 => consts + 212,
                    _ => consts + 8 + 4 * (i - 4),
                };
                if let Some(x) = bei16(raw, at) {
                    *slot = f32::from(x);
                }
            }
        }
        t.glyph_info = usize::from(be16(raw, 6).unwrap_or(0));
        t.variants = usize::from(be16(raw, 8).unwrap_or(0));
        t.raw = raw.to_vec();
        t
    }

    /// Units per em.
    pub fn upem(&self) -> f32 {
        self.upem
    }

    fn value_record(&self, sub: usize, g: u16) -> Option<f32> {
        if self.glyph_info == 0 {
            return None;
        }
        let gi = self.glyph_info;
        let off = usize::from(be16(&self.raw, gi + sub)?);
        if off == 0 {
            return None;
        }
        let at = gi + off;
        let cov = usize::from(be16(&self.raw, at)?);
        let n = usize::from(be16(&self.raw, at + 2)?);
        let idx = coverage(&self.raw, at + cov, g)?;
        if idx >= n {
            return None;
        }
        bei16(&self.raw, at + 4 + idx * 4).map(f32::from)
    }

    /// Italics correction of `g` (font units; 0 when absent).
    pub fn italics(&self, g: u16) -> f32 {
        self.value_record(0, g).unwrap_or(0.0)
    }

    /// Top-accent attachment x of `g` (font units).
    pub fn top_accent(&self, g: u16) -> Option<f32> {
        self.value_record(2, g)
    }

    /// `minConnectorOverlap`.
    pub fn min_overlap(&self) -> f32 {
        if self.variants == 0 {
            return 0.0;
        }
        be16(&self.raw, self.variants).map_or(0.0, f32::from)
    }

    fn construction(&self, g: u16, vertical: bool) -> Option<usize> {
        if self.variants == 0 {
            return None;
        }
        let mv = self.variants;
        let cov = usize::from(be16(&self.raw, mv + if vertical { 2 } else { 4 })?);
        if cov == 0 {
            return None;
        }
        let vcount = usize::from(be16(&self.raw, mv + 6)?);
        let hcount = usize::from(be16(&self.raw, mv + 8)?);
        let idx = coverage(&self.raw, mv + cov, g)?;
        let (base, n) = if vertical {
            (mv + 10, vcount)
        } else {
            (mv + 10 + 2 * vcount, hcount)
        };
        if idx >= n {
            return None;
        }
        let off = usize::from(be16(&self.raw, base + 2 * idx)?);
        (off != 0).then_some(mv + off)
    }

    /// Size variants of `g` along the vertical (or horizontal) axis, smallest first (at most 64).
    pub fn variants(&self, g: u16, vertical: bool) -> Vec<Variant> {
        let Some(c) = self.construction(g, vertical) else {
            return Vec::new();
        };
        let n = usize::from(be16(&self.raw, c + 2).unwrap_or(0)).min(64);
        (0..n)
            .filter_map(|k| {
                Some(Variant {
                    glyph: be16(&self.raw, c + 4 + 4 * k)?,
                    advance: f32::from(be16(&self.raw, c + 6 + 4 * k)?),
                })
            })
            .collect()
    }

    /// Glyph assembly parts of `g` (bottom-to-top / left-to-right; at most 32).
    pub fn assembly(&self, g: u16, vertical: bool) -> Vec<Part> {
        let Some(c) = self.construction(g, vertical) else {
            return Vec::new();
        };
        let off = usize::from(be16(&self.raw, c).unwrap_or(0));
        if off == 0 {
            return Vec::new();
        }
        let a = c + off;
        let n = usize::from(be16(&self.raw, a + 4).unwrap_or(0)).min(32);
        (0..n)
            .filter_map(|k| {
                let p = a + 6 + 10 * k;
                Some(Part {
                    glyph: be16(&self.raw, p)?,
                    start: f32::from(be16(&self.raw, p + 2)?),
                    end: f32::from(be16(&self.raw, p + 4)?),
                    full: f32::from(be16(&self.raw, p + 6)?),
                    extender: be16(&self.raw, p + 8)? & 1 != 0,
                })
            })
            .collect()
    }
}

/// Places assembly `parts` to cover `target` (font units) along the axis: returns
/// `(glyph, offset)` pairs (offset from the start edge) and the total length. Extenders repeat
/// as often as needed (at most 500 glyphs); connector overlaps are spread evenly within
/// `[min_overlap, connector lengths]`.
pub fn assemble(parts: &[Part], min_overlap: f32, target: f32) -> (Vec<(u16, f32)>, f32) {
    if parts.is_empty() {
        return (Vec::new(), 0.0);
    }
    let ext = parts.iter().filter(|p| p.extender).count();
    let build = |reps: usize| -> Vec<Part> {
        let mut v = Vec::new();
        for p in parts {
            if p.extender {
                for _ in 0..reps {
                    v.push(*p);
                }
            } else {
                v.push(*p);
            }
        }
        v
    };
    // Lengths with the maximum overlap (shortest) and minimum overlap (longest).
    let span = |v: &[Part]| -> (f32, f32) {
        let full: f32 = v.iter().map(|p| p.full).sum();
        let mut max_ov = 0.0;
        for w in v.windows(2) {
            if let [a, b] = w {
                max_ov += a.end.min(b.start).max(min_overlap);
            }
        }
        let min_ov = min_overlap * v.len().saturating_sub(1) as f32;
        (full - max_ov, full - min_ov)
    };
    let mut reps = if ext == 0 { 0 } else { 1 };
    let mut seq = build(reps);
    while ext > 0 && span(&seq).1 < target && seq.len() < 500 {
        reps += 1;
        seq = build(reps);
    }
    let (short, long) = span(&seq);
    // Fraction of the adjustable overlap to remove.
    let t = if long > short {
        ((target - short) / (long - short)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut out = Vec::with_capacity(seq.len());
    let mut pos = 0.0f32;
    for (k, p) in seq.iter().enumerate() {
        if let Some(prev) = k.checked_sub(1).and_then(|j| seq.get(j)) {
            let maxo = prev.end.min(p.start).max(min_overlap);
            let ov = maxo - t * (maxo - min_overlap);
            pos -= ov;
        }
        out.push((p.glyph, pos));
        pos += p.full;
    }
    (out, pos)
}
