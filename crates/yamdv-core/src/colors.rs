//! Custom-color layer (R-THEME-06/07, FUT-04): the 11 editor tokens, hex/HSV conversion and the
//! `custom-theme.ini` round trip (`[light]` / `[dark]`; `border` alias per P-03).

use crate::ini::{Ini, Line};
use crate::theme::Palette;

/// The editor-facing tokens in UI order (prompt 012 §4.4).
pub const TOKENS: [&str; 11] = [
    "bg",
    "text",
    "heading",
    "link",
    "accent",
    "quote_bar",
    "code_bg",
    "border",
    "table_header_bg",
    "zebra_bg",
    "selection",
];

/// Keys accepted in `custom-theme.ini` besides [`TOKENS`] (explicit halves of `border`).
const EXTRA: [&str; 2] = ["code_border", "table_border"];

/// `#rrggbb` (or `rrggbb`, whitespace trimmed) → `0xRRGGBB`.
pub fn parse_hex(v: &str) -> Option<u32> {
    let v = v.trim();
    let h = v.strip_prefix('#').unwrap_or(v);
    (h.len() == 6 && h.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| u32::from_str_radix(h, 16).ok())
        .flatten()
}

/// `0xRRGGBB` → `#rrggbb`.
pub fn to_hex(c: u32) -> String {
    format!("#{:06x}", c & 0xFF_FFFF)
}

/// `0xRRGGBB` → (hue 0..360, saturation 0..1, value 0..1).
pub fn rgb_to_hsv(c: u32) -> (f32, f32, f32) {
    let r = ((c >> 16) & 0xFF) as f32 / 255.0;
    let g = ((c >> 8) & 0xFF) as f32 / 255.0;
    let b = (c & 0xFF) as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d <= f32::EPSILON {
        0.0
    } else if (max - r).abs() <= f32::EPSILON {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if (max - g).abs() <= f32::EPSILON {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let s = if max <= f32::EPSILON { 0.0 } else { d / max };
    (h, s, max)
}

/// (hue 0..360, saturation 0..1, value 0..1) → `0xRRGGBB` (inputs clamped / wrapped).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> u32 {
    let h = h.rem_euclid(360.0);
    let (s, v) = (s.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let ch = |f: f32| (((f + m) * 255.0).round().clamp(0.0, 255.0)) as u32;
    (ch(r) << 16) | (ch(g) << 8) | ch(b)
}

/// The effective value of `token` in `p`.
pub fn token(p: &Palette, token: &str) -> Option<u32> {
    Some(match token {
        "bg" => p.bg,
        "text" => p.text,
        "heading" => p.heading,
        "link" => p.link,
        "accent" => p.accent,
        "quote_bar" => p.quote_bar,
        "code_bg" => p.code_bg,
        "border" | "table_border" => p.table_border,
        "code_border" => p.code_border,
        "table_header_bg" => p.table_header_bg,
        "zebra_bg" => p.zebra(),
        "selection" => p.selection_color(),
        _ => return None,
    })
}

fn slot<'a>(p: &'a mut Palette, token: &str) -> Option<&'a mut u32> {
    Some(match token {
        "bg" => &mut p.bg,
        "text" => &mut p.text,
        "heading" => &mut p.heading,
        "link" => &mut p.link,
        "accent" => &mut p.accent,
        "quote_bar" => &mut p.quote_bar,
        "code_bg" => &mut p.code_bg,
        "code_border" => &mut p.code_border,
        "table_border" => &mut p.table_border,
        "table_header_bg" => &mut p.table_header_bg,
        "zebra_bg" => &mut p.zebra_bg,
        "selection" => &mut p.selection,
        _ => return None,
    })
}

/// Section name of a mode.
pub fn section(dark: bool) -> &'static str {
    if dark { "dark" } else { "light" }
}

/// Applies the `[light]` / `[dark]` overrides of `ini` to `p`; `false` when a value is invalid
/// (that token keeps the scheme value). `border` sets both borders; an explicit `code_border` or
/// `table_border` overrides the alias (P-03).
pub fn apply(p: &mut Palette, ini: &Ini, dark: bool) -> bool {
    let sec = section(dark);
    let mut valid = true;
    let mut set = |p: &mut Palette, k: &str| {
        if let Some(v) = ini.get(sec, k) {
            match parse_hex(v) {
                Some(c) if k == "border" => {
                    p.code_border = c;
                    p.table_border = c;
                }
                Some(c) => {
                    if let Some(s) = slot(p, k) {
                        *s = c;
                    }
                }
                None => valid = false,
            }
        }
    };
    set(p, "border");
    for k in TOKENS
        .iter()
        .chain(EXTRA.iter())
        .filter(|k| **k != "border")
    {
        set(p, k);
    }
    valid
}

/// Sets (`Some`) or removes (`None`, *Reset token*) `token` of a mode in `ini`. Setting
/// `border` also removes explicit `code_border` / `table_border` keys so the edit is visible.
pub fn set_token(ini: &mut Ini, dark: bool, token: &str, value: Option<u32>) {
    let sec = section(dark);
    if token == "border" {
        remove_keys(ini, sec, &["border", "code_border", "table_border"]);
    } else {
        remove_keys(ini, sec, &[token]);
    }
    if let Some(c) = value {
        ini.set(sec, token, &to_hex(c));
    }
}

/// *Reset all colors*: removes every token entry of both modes (comments and unknown keys stay).
pub fn reset_all(ini: &mut Ini) {
    let all: Vec<&str> = TOKENS.iter().chain(EXTRA.iter()).copied().collect();
    remove_keys(ini, "light", &all);
    remove_keys(ini, "dark", &all);
}

fn remove_keys(ini: &mut Ini, sec: &str, keys: &[&str]) {
    let mut cur = String::new();
    ini.lines.retain(|l| match l {
        Line::Section(s) => {
            cur.clone_from(s);
            true
        }
        Line::Entry { key, .. } => {
            !(cur.eq_ignore_ascii_case(sec) && keys.iter().any(|k| k.eq_ignore_ascii_case(key)))
        }
        Line::Other(_) => true,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::palette;

    #[test]
    fn hex_round_trip() {
        assert_eq!(parse_hex("#FF0000"), Some(0xff0000));
        assert_eq!(parse_hex(" 00ff7f "), Some(0x00ff7f));
        assert_eq!(parse_hex("#ff00"), None);
        assert_eq!(parse_hex("#gg0000"), None);
        assert_eq!(parse_hex("#+12345"), None);
        assert_eq!(to_hex(0x0a0b0c), "#0a0b0c");
    }

    #[test]
    fn hsv_round_trip() {
        assert_eq!(rgb_to_hsv(0xff0000), (0.0, 1.0, 1.0));
        let (h, s, v) = rgb_to_hsv(0x00ff00);
        assert!((h - 120.0).abs() < 1e-3 && (s - 1.0).abs() < 1e-6 && (v - 1.0).abs() < 1e-6);
        assert_eq!(hsv_to_rgb(240.0, 1.0, 1.0), 0x0000ff);
        assert_eq!(hsv_to_rgb(0.0, 0.0, 0.5), 0x808080);
        assert_eq!(hsv_to_rgb(-120.0, 1.0, 1.0), 0x0000ff);
        for c in [
            0u32, 0xffffff, 0x2563eb, 0x123456, 0xfedcba, 0x7f7f80, 0xff8800,
        ] {
            let (h, s, v) = rgb_to_hsv(c);
            assert_eq!(hsv_to_rgb(h, s, v), c, "{c:06x}");
        }
    }

    #[test]
    fn r_theme_06_ini_round_trip() {
        let mut ini = Ini::parse(b"# mine\n[light]\nbg = #fefefe\nunknown = 1\n");
        set_token(&mut ini, false, "link", Some(0xff0000));
        set_token(&mut ini, true, "border", Some(0x112233));
        let text = ini.write();
        let back = Ini::parse(text.as_bytes());
        assert_eq!(back.get("light", "link"), Some("#ff0000"));
        assert_eq!(back.get("dark", "border"), Some("#112233"));
        assert!(text.starts_with("# mine\n"));
        let mut p = palette("default", false);
        assert!(apply(&mut p, &back, false));
        assert_eq!((p.link, p.bg), (0xff0000, 0xfefefe));
        let mut d = palette("default", true);
        assert!(apply(&mut d, &back, true));
        assert_eq!((d.code_border, d.table_border), (0x112233, 0x112233));
        // Reset token, then reset all (unknown keys and comments stay).
        let mut ini = back;
        set_token(&mut ini, false, "link", None);
        assert_eq!(ini.get("light", "link"), None);
        reset_all(&mut ini);
        assert_eq!(ini.get("light", "bg"), None);
        assert_eq!(ini.get("dark", "border"), None);
        assert_eq!(ini.get("light", "unknown"), Some("1"));
    }

    #[test]
    fn p_03_explicit_border_overrides_alias_and_invalid_flagged() {
        let ini = Ini::parse(b"[light]\ncode_border = #010101\nborder = #020202\nzebra_bg = #030303\nselection = #040404\nlink = nope\n");
        let mut p = palette("default", false);
        assert!(!apply(&mut p, &ini, false));
        assert_eq!((p.code_border, p.table_border), (0x010101, 0x020202));
        assert_eq!(token(&p, "zebra_bg"), Some(0x030303));
        assert_eq!(token(&p, "selection"), Some(0x040404));
        assert_eq!(p.link, palette("default", false).link);
        for t in TOKENS {
            assert!(token(&p, t).is_some(), "{t}");
        }
    }
}
