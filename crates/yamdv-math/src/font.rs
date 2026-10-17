//! Font access the math layout needs (§2.1): glyph ids from `cmap`, design metrics, and the raw
//! OpenType `MATH` table. The platform implements [`MathFont`] over DirectWrite (Cambria Math);
//! tests use [`MockFont`].

/// Design metrics of one glyph in font units (y up).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GlyphMetrics {
    /// Advance width.
    pub advance: f32,
    /// Ink bounding box left.
    pub x_min: f32,
    /// Ink bounding box right.
    pub x_max: f32,
    /// Ink bottom (negative below the baseline).
    pub y_min: f32,
    /// Ink top.
    pub y_max: f32,
}

/// A math font.
pub trait MathFont {
    /// Units per em.
    fn units_per_em(&self) -> f32;
    /// Glyph id of `c` (0 = missing).
    fn glyph(&self, c: char) -> u16;
    /// Design metrics of glyph `g`.
    fn metrics(&self, g: u16) -> GlyphMetrics;
    /// Raw bytes of the OpenType `MATH` table (empty when the font has none).
    fn math_table(&self) -> &[u8];
}

/// Deterministic test font: upem 1000, glyph id = code point (BMP) or a hash for astral
/// characters; advance 500 (1000 for operators), ascent 700, descent 200 (`(`/`)`/`[`… 750/250),
/// no `MATH` table unless one is supplied.
#[derive(Debug, Default)]
pub struct MockFont {
    /// Raw `MATH` table.
    pub math: Vec<u8>,
}

impl MathFont for MockFont {
    fn units_per_em(&self) -> f32 {
        1000.0
    }
    fn glyph(&self, c: char) -> u16 {
        let v = c as u32;
        if v == 0xFFFF {
            return 0;
        }
        u16::try_from(v).unwrap_or_else(|_| u16::try_from(0xE000 + (v & 0x0FFF)).unwrap_or(1))
    }
    fn metrics(&self, g: u16) -> GlyphMetrics {
        let c = char::from_u32(u32::from(g)).unwrap_or('x');
        let wide = matches!(c, '∑' | '∏' | '∫' | '∬' | '∮' | '√');
        let delim = matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | '|' | '⟨' | '⟩');
        let advance = if wide { 1000.0 } else { 500.0 };
        let (y_min, y_max) = if delim || wide {
            (-250.0, 750.0)
        } else if matches!(
            c,
            '\u{302}' | '\u{303}' | '\u{304}' | '\u{307}' | '\u{308}' | '\u{20D7}'
        ) {
            (500.0, 700.0)
        } else {
            (-200.0, 700.0)
        };
        GlyphMetrics {
            advance,
            x_min: 0.0,
            x_max: advance,
            y_min,
            y_max,
        }
    }
    fn math_table(&self) -> &[u8] {
        &self.math
    }
}
