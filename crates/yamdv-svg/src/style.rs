//! Presentation values: colors, numbers, lengths, paints and transforms.

use tiny_skia::Transform;

/// Straight RGBA color.
pub type Rgba = [u8; 4];

/// A fill or stroke paint.
#[derive(Debug, Clone, PartialEq)]
pub enum Paint {
    /// `none`.
    None,
    /// Solid color.
    Color(Rgba),
    /// `currentColor`.
    Current,
    /// `url(#id)` with an optional fallback.
    Url(String, Option<Box<Paint>>),
}

const NAMED: &[(&str, u32)] = &[
    ("black", 0x000000),
    ("white", 0xffffff),
    ("red", 0xff0000),
    ("green", 0x008000),
    ("blue", 0x0000ff),
    ("yellow", 0xffff00),
    ("cyan", 0x00ffff),
    ("aqua", 0x00ffff),
    ("magenta", 0xff00ff),
    ("fuchsia", 0xff00ff),
    ("gray", 0x808080),
    ("grey", 0x808080),
    ("silver", 0xc0c0c0),
    ("maroon", 0x800000),
    ("olive", 0x808000),
    ("lime", 0x00ff00),
    ("teal", 0x008080),
    ("navy", 0x000080),
    ("purple", 0x800080),
    ("orange", 0xffa500),
    ("pink", 0xffc0cb),
    ("brown", 0xa52a2a),
    ("gold", 0xffd700),
    ("indigo", 0x4b0082),
    ("violet", 0xee82ee),
    ("crimson", 0xdc143c),
    ("coral", 0xff7f50),
    ("tomato", 0xff6347),
    ("salmon", 0xfa8072),
    ("khaki", 0xf0e68c),
    ("orchid", 0xda70d6),
    ("plum", 0xdda0dd),
    ("tan", 0xd2b48c),
    ("beige", 0xf5f5dc),
    ("ivory", 0xfffff0),
    ("lavender", 0xe6e6fa),
    ("turquoise", 0x40e0d0),
    ("skyblue", 0x87ceeb),
    ("steelblue", 0x4682b4),
    ("royalblue", 0x4169e1),
    ("dodgerblue", 0x1e90ff),
    ("darkblue", 0x00008b),
    ("darkred", 0x8b0000),
    ("darkgreen", 0x006400),
    ("darkgray", 0xa9a9a9),
    ("darkgrey", 0xa9a9a9),
    ("lightgray", 0xd3d3d3),
    ("lightgrey", 0xd3d3d3),
    ("lightblue", 0xadd8e6),
    ("lightgreen", 0x90ee90),
    ("whitesmoke", 0xf5f5f5),
    ("gainsboro", 0xdcdcdc),
    ("dimgray", 0x696969),
    ("dimgrey", 0x696969),
    ("slategray", 0x708090),
    ("forestgreen", 0x228b22),
    ("seagreen", 0x2e8b57),
    ("limegreen", 0x32cd32),
    ("chocolate", 0xd2691e),
    ("firebrick", 0xb22222),
    ("goldenrod", 0xdaa520),
    ("hotpink", 0xff69b4),
    ("deeppink", 0xff1493),
    ("darkorange", 0xff8c00),
    ("orangered", 0xff4500),
    ("midnightblue", 0x191970),
    ("cornflowerblue", 0x6495ed),
    ("mediumpurple", 0x9370db),
    ("rebeccapurple", 0x663399),
    ("sienna", 0xa0522d),
    ("peru", 0xcd853f),
    ("wheat", 0xf5deb3),
    ("snow", 0xfffafa),
    ("linen", 0xfaf0e6),
    ("mintcream", 0xf5fffa),
    ("aliceblue", 0xf0f8ff),
    ("darkslategray", 0x2f4f4f),
    ("darkcyan", 0x008b8b),
    ("darkviolet", 0x9400d3),
    ("darkmagenta", 0x8b008b),
    ("olivedrab", 0x6b8e23),
    ("yellowgreen", 0x9acd32),
];

fn hex_nibble(c: u8) -> Option<u8> {
    char::from(c)
        .to_digit(16)
        .and_then(|d| u8::try_from(d).ok())
}

/// Parses a CSS color (`#rgb`, `#rrggbb`, `#rgba`, `#rrggbbaa`, `rgb()`/`rgba()`, names,
/// `transparent`). `currentColor` is a [`Paint`], not a color.
pub fn color(s: &str) -> Option<Rgba> {
    let s = s.trim();
    if let Some(h) = s.strip_prefix('#') {
        let d: Vec<u8> = h.bytes().map(hex_nibble).collect::<Option<_>>()?;
        return match d.as_slice() {
            [r, g, b] => Some([r * 17, g * 17, b * 17, 255]),
            [r, g, b, a] => Some([r * 17, g * 17, b * 17, a * 17]),
            [r1, r2, g1, g2, b1, b2] => Some([r1 * 16 + r2, g1 * 16 + g2, b1 * 16 + b2, 255]),
            [r1, r2, g1, g2, b1, b2, a1, a2] => {
                Some([r1 * 16 + r2, g1 * 16 + g2, b1 * 16 + b2, a1 * 16 + a2])
            }
            _ => None,
        };
    }
    let lower = s.to_ascii_lowercase();
    if let Some(args) = lower
        .strip_prefix("rgba(")
        .or_else(|| lower.strip_prefix("rgb("))
        .and_then(|r| r.strip_suffix(')'))
    {
        let parts: Vec<&str> = args
            .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
            .filter(|p| !p.is_empty())
            .collect();
        let chan = |p: &str| -> Option<u8> {
            let v = match p.strip_suffix('%') {
                Some(q) => q.parse::<f32>().ok()? * 2.55,
                None => p.parse::<f32>().ok()?,
            };
            Some(clamp_u8(v))
        };
        let a = match parts.get(3) {
            Some(p) => match p.strip_suffix('%') {
                Some(q) => clamp_u8(q.parse::<f32>().ok()? * 2.55),
                None => clamp_u8(p.parse::<f32>().ok()? * 255.0),
            },
            None => 255,
        };
        return Some([
            chan(parts.first()?)?,
            chan(parts.get(1)?)?,
            chan(parts.get(2)?)?,
            a,
        ]);
    }
    if lower == "transparent" {
        return Some([0, 0, 0, 0]);
    }
    NAMED.iter().find(|(n, _)| *n == lower).map(|&(_, v)| {
        let [_, r, g, b] = v.to_be_bytes();
        [r, g, b, 255]
    })
}

/// Rounds and clamps to `0..=255`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn clamp_u8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

/// Parses a paint value.
pub fn paint(s: &str) -> Option<Paint> {
    let s = s.trim();
    if s == "none" {
        return Some(Paint::None);
    }
    if s.eq_ignore_ascii_case("currentcolor") {
        return Some(Paint::Current);
    }
    if let Some(r) = s.strip_prefix("url(") {
        let (inner, rest) = r.split_once(')')?;
        let inner = inner.trim().trim_matches(|c| c == '"' || c == '\'');
        let fb = paint(rest).map(Box::new);
        // Only same-document references (S-RES-07); anything else falls back.
        return match inner.strip_prefix('#') {
            Some(id) => Some(Paint::Url(id.to_string(), fb)),
            None => Some(fb.map_or(Paint::None, |b| *b)),
        };
    }
    color(s).map(Paint::Color)
}

/// Number list (`,` or whitespace separated; `1-2` splits at the sign).
pub fn numbers(s: &str) -> Vec<f32> {
    let mut out = Vec::new();
    let mut p = crate::path::Lexer::new(s);
    while let Some(v) = p.number() {
        out.push(v);
    }
    out
}

/// Length in user units; `%` resolves against `pct` (the reference length).
pub fn length(s: &str, pct: f32, font: f32) -> Option<f32> {
    let s = s.trim();
    let split = s
        .find(|c: char| c.is_ascii_alphabetic() || c == '%')
        .unwrap_or(s.len());
    let (n, unit) = s.split_at(split);
    let v: f32 = n.trim().parse().ok()?;
    let k = match unit.trim() {
        "" | "px" => 1.0,
        "pt" => 4.0 / 3.0,
        "pc" => 16.0,
        "mm" => 96.0 / 25.4,
        "cm" => 96.0 / 2.54,
        "in" => 96.0,
        "em" => font,
        "ex" => font / 2.0,
        "%" => pct / 100.0,
        _ => return None,
    };
    let r = v * k;
    r.is_finite().then_some(r)
}

/// Parses a `transform` list.
pub fn transform(s: &str) -> Transform {
    let mut t = Transform::identity();
    let mut rest = s;
    while let Some(open) = rest.find('(') {
        let name = rest
            .get(..open)
            .unwrap_or("")
            .trim_matches(|c: char| c.is_whitespace() || c == ',');
        let Some(close) = rest.find(')') else { break };
        let a = numbers(rest.get(open + 1..close).unwrap_or(""));
        let g = |i: usize| a.get(i).copied();
        let m = match (name, a.len()) {
            ("matrix", 6) => Transform::from_row(
                g(0).unwrap_or(1.0),
                g(1).unwrap_or(0.0),
                g(2).unwrap_or(0.0),
                g(3).unwrap_or(1.0),
                g(4).unwrap_or(0.0),
                g(5).unwrap_or(0.0),
            ),
            ("translate", 1 | 2) => {
                Transform::from_translate(g(0).unwrap_or(0.0), g(1).unwrap_or(0.0))
            }
            ("scale", 1 | 2) => {
                let sx = g(0).unwrap_or(1.0);
                Transform::from_scale(sx, g(1).unwrap_or(sx))
            }
            ("rotate", 1) => Transform::from_rotate(g(0).unwrap_or(0.0)),
            ("rotate", 3) => Transform::from_rotate_at(
                g(0).unwrap_or(0.0),
                g(1).unwrap_or(0.0),
                g(2).unwrap_or(0.0),
            ),
            ("skewX", 1) => Transform::from_skew(g(0).unwrap_or(0.0).to_radians().tan(), 0.0),
            ("skewY", 1) => Transform::from_skew(0.0, g(0).unwrap_or(0.0).to_radians().tan()),
            _ => Transform::identity(),
        };
        t = t.pre_concat(m);
        rest = rest.get(close + 1..).unwrap_or("");
    }
    t
}
