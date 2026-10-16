//! Theme token data (Appendix B §B.3.1–§B.3.4) for all 11 schemes, light and dark, plus WCAG
//! contrast helpers (R-A11Y-04). Phase 002 uses only `Default` with mode Follow System.

pub use crate::theme_table::{CALLOUTS, DARK, LIGHT, SYNTAX};

/// Scheme names in R-THEME-02 order (index into [`LIGHT`] / [`DARK`]).
pub const SCHEMES: [&str; 11] = [
    "default",
    "github",
    "glacier",
    "forest",
    "sunset",
    "typewriter",
    "editorial",
    "violet",
    "lychee",
    "mint",
    "neon",
];

/// Document tokens of one scheme in one mode (`0xRRGGBB`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// Page background.
    pub bg: u32,
    /// Body text.
    pub text: u32,
    /// Headings H1–H5.
    pub heading: u32,
    /// Links.
    pub link: u32,
    /// Accent (selection base, checkboxes).
    pub accent: u32,
    /// Blockquote bar.
    pub quote_bar: u32,
    /// Blockquote text.
    pub quote_text: u32,
    /// Code block background.
    pub code_bg: u32,
    /// Inline code background.
    pub inline_code_bg: u32,
    /// Table header background.
    pub table_header_bg: u32,
    /// Code block border.
    pub code_border: u32,
    /// Table grid.
    pub table_border: u32,
    /// Hairlines (HR, footnote separator).
    pub border_subtle: u32,
    /// Muted text (H6, labels).
    pub text_muted: u32,
    /// Selection alpha applied to `accent` (percent).
    pub selection_alpha_pct: u8,
    /// Error text.
    pub error: u32,
}

/// Shared light tokens (B.3.1); the per-scheme fields are overwritten.
pub const LIGHT_SHARED: Palette = Palette {
    bg: 0xffffff,
    text: 0x333333,
    heading: 0x333333,
    link: 0x2563eb,
    accent: 0x2563eb,
    quote_bar: 0xdddddd,
    quote_text: 0x666666,
    code_bg: 0xf5f5f5,
    inline_code_bg: 0xf0f0f0,
    table_header_bg: 0xf5f5f5,
    code_border: 0xe0e0e0,
    table_border: 0xdddddd,
    border_subtle: 0xeeeeee,
    text_muted: 0x7a7a7a,
    selection_alpha_pct: 20,
    error: 0xcc0000,
};

/// Dark base for every scheme except GitHub (B.3.2).
pub const DARK_BASE: Palette = Palette {
    bg: 0x1e1e1e,
    text: 0xd4d4d4,
    heading: 0xd4d4d4,
    link: 0x4da5ec,
    accent: 0x4da5ec,
    quote_bar: 0x555555,
    quote_text: 0xaaaaaa,
    code_bg: 0x2d2d2d,
    inline_code_bg: 0x3a3a3a,
    table_header_bg: 0x2d2d2d,
    code_border: 0x3a3a3a,
    table_border: 0x3a3a3a,
    border_subtle: 0x333333,
    text_muted: 0x9a9a9a,
    selection_alpha_pct: 30,
    error: 0xff6b6b,
};

/// GitHub dark (B.3.2).
pub const GITHUB_DARK: Palette = Palette {
    bg: 0x0d1117,
    text: 0xe6edf3,
    heading: 0xe6edf3,
    link: 0x4493f8,
    accent: 0x4493f8,
    quote_bar: 0x3d444d,
    quote_text: 0x9198a1,
    code_bg: 0x151b23,
    inline_code_bg: 0x262c36,
    table_header_bg: 0x151b23,
    code_border: 0x3d444d,
    table_border: 0x3d444d,
    ..DARK_BASE
};

/// Callout colors (B.3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Callout {
    /// Type (`note`, `tip`, …).
    pub name: &'static str,
    /// Light bar/icon/title color.
    pub light: u32,
    /// Light background.
    pub light_bg: u32,
    /// Dark color.
    pub dark: u32,
    /// Dark background.
    pub dark_bg: u32,
}

/// One syntax token class (B.3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Syntax {
    /// Class description from the spec.
    pub class: &'static str,
    /// Light color.
    pub light: u32,
    /// Dark color.
    pub dark: u32,
}

/// Palette for `scheme` (unknown → default) in light or dark mode.
pub fn palette(scheme: &str, dark: bool) -> Palette {
    let i = SCHEMES
        .iter()
        .position(|s| s.eq_ignore_ascii_case(scheme))
        .unwrap_or(0);
    let table = if dark { &DARK } else { &LIGHT };
    table.get(i).copied().unwrap_or(LIGHT_SHARED)
}

/// WCAG relative luminance of `0xRRGGBB`.
pub fn luminance(c: u32) -> f64 {
    let ch = |v: u32| {
        let s = f64::from(v & 0xFF) / 255.0;
        if s <= 0.040_45 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * ch(c >> 16) + 0.7152 * ch(c >> 8) + 0.0722 * ch(c)
}

/// WCAG contrast ratio between two colors.
pub fn contrast(a: u32, b: u32) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r_a11y_04_contrast_every_palette() {
        let mut failures = Vec::new();
        for (i, name) in SCHEMES.iter().enumerate() {
            for (mode, p) in [("light", LIGHT[i]), ("dark", DARK[i])] {
                for (tok, c, min) in [
                    ("text", p.text, 4.5),
                    ("link", p.link, 4.5),
                    ("heading", p.heading, 3.0),
                ] {
                    let r = contrast(c, p.bg);
                    if r < min {
                        failures.push(format!("{name}/{mode}/{tok} {r:.2} < {min}"));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn data_shape() {
        assert_eq!(palette("default", false).bg, 0xffffff);
        assert_eq!(palette("GitHub", true).bg, 0x0d1117);
        assert_eq!(palette("mint", false).heading, 0x0ea271);
        assert_eq!(palette("nope", true).bg, 0x1e1e1e);
        assert!((contrast(0xffffff, 0x000000) - 21.0).abs() < 1e-9);
        assert_eq!(CALLOUTS.len(), 5);
    }
}
