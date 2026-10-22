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
    /// Code line numbers (`line_number`).
    pub line_number: u32,
    /// `==highlight==` background.
    pub highlight_bg: u32,
    /// `==highlight==` text (0 = `text`).
    pub highlight_text: u32,
    /// Table zebra rows (`mix(bg, table_header_bg, 50 %)`; 0 = compute).
    pub zebra_bg: u32,
    /// Selection color (custom token `selection`; 0 = `accent`), drawn at `selection_alpha_pct`.
    pub selection: u32,
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
    line_number: 0xb8b8b8,
    highlight_bg: 0xfff3a3,
    highlight_text: 0,
    zebra_bg: 0,
    selection: 0,
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
    line_number: 0x5a5a5a,
    highlight_bg: 0x665c1e,
    highlight_text: 0xf5f0d0,
    zebra_bg: 0,
    selection: 0,
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

/// 50 % mix of two colors.
pub fn mix(a: u32, b: u32) -> u32 {
    let ch = |s: u32| (((a >> s) & 0xFF) + ((b >> s) & 0xFF)) / 2;
    (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

impl Palette {
    /// Selection base color (`selection`, or `accent` when unset).
    pub fn selection_color(&self) -> u32 {
        if self.selection != 0 {
            self.selection
        } else {
            self.accent
        }
    }

    /// Zebra row color (B.3.1: `mix(bg, table_header_bg, 50 %)`).
    pub fn zebra(&self) -> u32 {
        if self.zebra_bg != 0 {
            self.zebra_bg
        } else {
            mix(self.bg, self.table_header_bg)
        }
    }
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

/// Palette choice of an export job (`export.pdf_colors`, `export.print_colors`,
/// `export.html_colors`; BUG-10, P-34/P-35/P-37/P-38).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportColors {
    /// The scheme's light tokens on a white page, whatever the UI mode (default).
    Light,
    /// The current UI mode, coherent across the whole page.
    Theme,
}

impl ExportColors {
    /// Parses a setting value (anything but `theme` is the default `light`).
    pub fn parse(v: &str) -> Self {
        if v.eq_ignore_ascii_case("theme") {
            Self::Theme
        } else {
            Self::Light
        }
    }

    /// Whether the job renders dark: only `theme` with a dark UI.
    pub fn dark(self, ui_dark: bool) -> bool {
        self == Self::Theme && ui_dark
    }
}

/// The palette an export job uses (R-EXP-01/02/07): `base` is the scheme palette (custom colours
/// already applied) of the mode returned by [`ExportColors::dark`]. A light job always gets a
/// white page. Selection, find and hover colours never reach a job (they are not in the palette
/// a job draws with: `selection` is cleared).
pub fn export_palette(base: Palette, dark: bool) -> Palette {
    let mut p = base;
    if !dark {
        p.bg = 0xffffff;
    }
    p.selection = 0;
    p
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
    fn bug_10_export_palette_per_setting_and_mode() {
        for scheme in SCHEMES {
            for ui_dark in [false, true] {
                // light: the scheme's light tokens on a white page, whatever the UI mode.
                let c = ExportColors::parse("light");
                assert!(!c.dark(ui_dark));
                let p = export_palette(palette(scheme, c.dark(ui_dark)), c.dark(ui_dark));
                assert_eq!(p.bg, 0xffffff);
                assert_eq!(p.text, palette(scheme, false).text);
                assert_eq!(p.code_bg, palette(scheme, false).code_bg);
                assert!(luminance(p.code_bg) > 0.5, "{scheme}: light code block");
                assert!(contrast(p.text, p.bg) >= 4.5);
                // theme: follows the UI mode everywhere.
                let c = ExportColors::parse("theme");
                assert_eq!(c.dark(ui_dark), ui_dark);
                let p = export_palette(palette(scheme, ui_dark), ui_dark);
                let want = if ui_dark {
                    palette(scheme, true).bg
                } else {
                    0xffffff
                };
                assert_eq!(p.bg, want);
                assert_eq!(
                    luminance(p.bg) < 0.2,
                    ui_dark,
                    "{scheme}: page matches mode"
                );
                assert_eq!(
                    luminance(p.code_bg) < 0.2,
                    ui_dark,
                    "{scheme}: blocks match page"
                );
            }
        }
        assert_eq!(ExportColors::parse("bogus"), ExportColors::Light);
        assert_eq!(ExportColors::parse("THEME"), ExportColors::Theme);
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
