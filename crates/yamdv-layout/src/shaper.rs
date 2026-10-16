//! Text shaping abstraction (§4.4.1) and a deterministic mock for tests.

// Indices come from the layout's own arenas (in bounds by construction); f32/f64/u32 casts are
// pixel and UTF-16 quantities far below the type limits (documents ≤ 256 MiB).
#![allow(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::explicit_counter_loop
)]

use std::any::Any;
use std::ops::Range;

use crate::geom::Rect;

/// Font role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// Body font (`typography.body_font`).
    Body,
    /// Code font (`typography.code_font`).
    Code,
}

/// Character style.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// Font role.
    pub family: Family,
    /// Size in DIPs.
    pub size: f32,
    /// Weight (400, 600).
    pub weight: u16,
    /// Italic.
    pub italic: bool,
    /// Color `0xRRGGBB`.
    pub color: u32,
    /// Strikethrough.
    pub strike: bool,
    /// Underline.
    pub underline: bool,
}

/// A style over a UTF-16 range.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    /// UTF-16 range.
    pub range: Range<u32>,
    /// Style.
    pub style: Style,
}

/// Paragraph shaping request.
#[derive(Debug, Clone)]
pub struct ParaSpec<'a> {
    /// Text (`\n` = forced break).
    pub text: &'a str,
    /// Base style.
    pub base: Style,
    /// Style overrides.
    pub spans: Vec<Span>,
    /// Wrap width (`None` = no wrapping, code).
    pub width: Option<f32>,
    /// Uniform line pitch (`font_size × line_height`).
    pub line_pitch: f32,
    /// Right-to-left paragraph (R-REN-19).
    pub rtl: bool,
    /// Tab stops every N space advances (code: 4, R-MD-17); `None` = shaper default.
    pub tab_spaces: Option<u8>,
}

/// One shaped paragraph.
pub trait Shaped: std::fmt::Debug {
    /// Laid-out width (widest line).
    fn width(&self) -> f32;
    /// Height (lines × pitch).
    fn height(&self) -> f32;
    /// Number of lines.
    fn lines(&self) -> usize;
    /// UTF-16 index nearest to the point (relative to the paragraph origin).
    fn hit(&self, x: f32, y: f32) -> u32;
    /// Line boxes covering the UTF-16 range (selection, inline-code backgrounds).
    fn range_rects(&self, range: Range<u32>) -> Vec<Rect>;
    /// Downcast for the platform renderer.
    fn as_any(&self) -> &dyn Any;
}

/// Shaper.
pub trait TextShaper {
    /// Shapes a paragraph.
    fn shape(&mut self, spec: &ParaSpec<'_>) -> Box<dyn Shaped>;
}

/// First-strong-character direction (R-REN-19): true for RTL scripts.
pub fn is_rtl(text: &str) -> bool {
    for c in text.chars() {
        let v = c as u32;
        if matches!(v, 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF | 0x10800..=0x10FFF | 0x1E800..=0x1EFFF)
        {
            return true;
        }
        if c.is_alphabetic() {
            return false;
        }
    }
    false
}

/// Deterministic mock: ½ em per narrow char, 1 em per wide char, greedy wrapping at spaces.
#[derive(Debug, Default)]
pub struct MockShaper;

#[derive(Debug)]
struct MockLine {
    range: Range<u32>,
    /// x of each UTF-16 position in `range` plus the end.
    xs: Vec<f32>,
}

/// Mock shaped paragraph.
#[derive(Debug)]
pub struct MockShaped {
    lines: Vec<MockLine>,
    pitch: f32,
}

fn advance(c: char, size: f32) -> f32 {
    if yamdv_core::doc::index::is_wide(c) {
        size
    } else {
        size * 0.5
    }
}

impl TextShaper for MockShaper {
    fn shape(&mut self, spec: &ParaSpec<'_>) -> Box<dyn Shaped> {
        let mut units: Vec<(u32, char, f32)> = Vec::new();
        let mut p16 = 0u32;
        for c in spec.text.chars() {
            let size = spec
                .spans
                .iter()
                .rev()
                .find(|s| s.range.contains(&p16))
                .map_or(spec.base.size, |s| s.style.size);
            units.push((p16, c, advance(c, size)));
            p16 += c.len_utf16() as u32;
        }
        let total16 = p16;
        let mut lines = Vec::new();
        let mut start = 0usize;
        let mut i = 0usize;
        let mut x = 0.0f32;
        let mut last_space: Option<usize> = None;
        let tab = f32::from(spec.tab_spaces.unwrap_or(4)) * spec.base.size * 0.5;
        let push = |lines: &mut Vec<MockLine>,
                    units: &[(u32, char, f32)],
                    a: usize,
                    b: usize,
                    end16: u32| {
            let mut xs = Vec::new();
            let mut x = 0.0f32;
            let s16 = units.get(a).map_or(end16, |u| u.0);
            for u in units.get(a..b).unwrap_or(&[]) {
                for _ in 0..u.1.len_utf16() {
                    xs.push(x);
                }
                x += if u.1 == '\t' { tab - x % tab } else { u.2 };
            }
            xs.push(x);
            let e16 = units.get(b).map_or(end16, |u| u.0);
            lines.push(MockLine {
                range: s16..e16,
                xs,
            });
        };
        while i < units.len() {
            let (_, c, w) = units[i];
            if c == '\n' {
                push(&mut lines, &units, start, i, total16);
                i += 1;
                start = i;
                x = 0.0;
                last_space = None;
                continue;
            }
            let w = if c == '\t' { tab - x % tab } else { w };
            if let Some(width) = spec.width
                && x + w > width
                && i > start
                && c != ' '
            {
                let brk = last_space.map_or(i, |s| s + 1);
                push(&mut lines, &units, start, brk, total16);
                start = brk;
                i = brk;
                x = 0.0;
                last_space = None;
                continue;
            }
            if c == ' ' {
                last_space = Some(i);
            }
            x += w;
            i += 1;
        }
        push(&mut lines, &units, start, units.len(), total16);
        Box::new(MockShaped {
            lines,
            pitch: spec.line_pitch,
        })
    }
}

impl Shaped for MockShaped {
    fn width(&self) -> f32 {
        self.lines
            .iter()
            .filter_map(|l| l.xs.last().copied())
            .fold(0.0, f32::max)
    }
    fn height(&self) -> f32 {
        self.lines.len() as f32 * self.pitch
    }
    fn lines(&self) -> usize {
        self.lines.len()
    }
    fn hit(&self, x: f32, y: f32) -> u32 {
        let li =
            ((y / self.pitch).floor().max(0.0) as usize).min(self.lines.len().saturating_sub(1));
        let Some(l) = self.lines.get(li) else {
            return 0;
        };
        let mut best = l.range.start;
        for (k, w) in l.xs.windows(2).enumerate() {
            if x >= (w[0] + w[1]) / 2.0 {
                best = l.range.start + k as u32 + 1;
            }
        }
        best.min(l.range.end)
    }
    fn range_rects(&self, range: Range<u32>) -> Vec<Rect> {
        let mut out = Vec::new();
        for (li, l) in self.lines.iter().enumerate() {
            let a = range.start.max(l.range.start);
            let b = range.end.min(l.range.end);
            if a >= b {
                continue;
            }
            let xa =
                l.xs.get((a - l.range.start) as usize)
                    .copied()
                    .unwrap_or(0.0);
            let xb =
                l.xs.get((b - l.range.start) as usize)
                    .copied()
                    .unwrap_or(xa);
            out.push(Rect::new(xa, li as f32 * self.pitch, xb - xa, self.pitch));
        }
        out
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style() -> Style {
        Style {
            family: Family::Body,
            size: 16.0,
            weight: 400,
            italic: false,
            color: 0,
            strike: false,
            underline: false,
        }
    }

    #[test]
    fn mock_wraps_and_hits() {
        let spec = ParaSpec {
            text: "aaaa bbbb cccc",
            base: style(),
            spans: vec![],
            width: Some(80.0),
            line_pitch: 20.0,
            rtl: false,
            tab_spaces: None,
        };
        let s = MockShaper.shape(&spec);
        assert_eq!(s.lines(), 2);
        assert_eq!(s.height(), 40.0);
        assert_eq!(s.hit(0.0, 25.0), 10);
        assert_eq!(s.hit(1000.0, 0.0), 10);
        assert_eq!(s.range_rects(3..12).len(), 2);
    }

    #[test]
    fn r_ren_19_direction() {
        assert!(is_rtl("  123 שלום abc"));
        assert!(is_rtl("«مرحبا»"));
        assert!(!is_rtl("abc שלום"));
        assert!(!is_rtl("123"));
    }
}
