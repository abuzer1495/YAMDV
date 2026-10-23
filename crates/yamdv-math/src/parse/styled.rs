//! Font mapping: `\mathbf` & co. onto the Mathematical Alphanumeric Symbols (R-MATH-02).

use super::{Font, Node};

/// Applies `font` to every character in `n` that has the default font.
pub(super) fn set_font(n: &mut Node, font: Font) {
    match n {
        Node::Char { font: f, .. } => {
            if *f == Font::Default || font != Font::Roman {
                *f = font;
            }
        }
        Node::Group(v)
        | Node::Color { body: v, .. }
        | Node::Style { body: v, .. }
        | Node::LeftRight { body: v, .. } => {
            for k in v {
                set_font(k, font);
            }
        }
        Node::Scripts { base, sup, sub } => {
            set_font(base, font);
            if let Some(s) = sup {
                set_font(s, font);
            }
            if let Some(s) = sub {
                set_font(s, font);
            }
        }
        Node::Frac { num, den, .. } => {
            set_font(num, font);
            set_font(den, font);
        }
        Node::Matrix { rows, .. } => {
            for k in rows.iter_mut().flatten() {
                set_font(k, font);
            }
        }
        Node::Stack {
            base, over, under, ..
        } => {
            set_font(base, font);
            for s in [over, under].into_iter().flatten() {
                set_font(s, font);
            }
        }
        Node::Sqrt { body, .. }
        | Node::Stretch { body, .. }
        | Node::Phantom { body, .. }
        | Node::Cancel { body, .. }
        | Node::Boxed(body)
        | Node::Limits { base: body, .. }
        | Node::Class { body, .. }
        | Node::Accent { body, .. }
        | Node::Overline(body)
        | Node::Underline(body) => set_font(body, font),
        _ => {}
    }
}

/// Maps a character to its Mathematical Alphanumeric Symbol for `font` (§2.2).
pub fn styled(c: char, font: Font) -> char {
    let off = |base: u32, k: u32| char::from_u32(base + k).unwrap_or(c);
    let up = c.is_ascii_uppercase();
    let lo = c.is_ascii_lowercase();
    let k = (c as u32).wrapping_sub(if up { 'A' as u32 } else { 'a' as u32 });
    match font {
        Font::Default | Font::Italic => {
            if c == 'h' {
                return 'ℎ';
            }
            if up {
                return off(0x1D434, k);
            }
            if lo {
                return off(0x1D44E, k);
            }
            if font == Font::Italic && !('α'..='ω').contains(&c) {
                return c;
            }
            greek_italic(c)
        }
        Font::Roman => c,
        Font::Bold => {
            if up {
                off(0x1D400, k)
            } else if lo {
                off(0x1D41A, k)
            } else if c.is_ascii_digit() {
                off(0x1D7CE, c as u32 - '0' as u32)
            } else {
                greek(c, 0x1D6A8, 0x1D6C2)
            }
        }
        Font::BoldItalic => {
            if up {
                off(0x1D468, k)
            } else if lo {
                off(0x1D482, k)
            } else if c.is_ascii_digit() {
                off(0x1D7CE, c as u32 - '0' as u32)
            } else {
                greek(c, 0x1D71C, 0x1D736)
            }
        }
        Font::SansSerif => {
            if up {
                off(0x1D5A0, k)
            } else if lo {
                off(0x1D5BA, k)
            } else if c.is_ascii_digit() {
                off(0x1D7E2, c as u32 - '0' as u32)
            } else {
                c
            }
        }
        Font::Monospace => {
            if up {
                off(0x1D670, k)
            } else if lo {
                off(0x1D68A, k)
            } else if c.is_ascii_digit() {
                off(0x1D7F6, c as u32 - '0' as u32)
            } else {
                c
            }
        }
        Font::Fraktur => {
            let special = match c {
                'C' => Some('ℭ'),
                'H' => Some('ℌ'),
                'I' => Some('ℑ'),
                'R' => Some('ℜ'),
                'Z' => Some('ℨ'),
                _ => None,
            };
            if let Some(s) = special {
                s
            } else if up {
                off(0x1D504, k)
            } else if lo {
                off(0x1D51E, k)
            } else {
                c
            }
        }
        Font::Blackboard => {
            let special = match c {
                'C' => Some('ℂ'),
                'H' => Some('ℍ'),
                'N' => Some('ℕ'),
                'P' => Some('ℙ'),
                'Q' => Some('ℚ'),
                'R' => Some('ℝ'),
                'Z' => Some('ℤ'),
                _ => None,
            };
            if let Some(s) = special {
                s
            } else if up {
                off(0x1D538, k)
            } else if lo {
                off(0x1D552, k)
            } else if c.is_ascii_digit() {
                off(0x1D7D8, c as u32 - '0' as u32)
            } else {
                c
            }
        }
        Font::Calligraphic => {
            let special = match c {
                'B' => Some('ℬ'),
                'E' => Some('ℰ'),
                'F' => Some('ℱ'),
                'H' => Some('ℋ'),
                'I' => Some('ℐ'),
                'L' => Some('ℒ'),
                'M' => Some('ℳ'),
                'R' => Some('ℛ'),
                'e' => Some('ℯ'),
                'g' => Some('ℊ'),
                'o' => Some('ℴ'),
                _ => None,
            };
            if let Some(s) = special {
                s
            } else if up {
                off(0x1D49C, k)
            } else if lo {
                off(0x1D4B6, k)
            } else {
                c
            }
        }
    }
}

/// Lowercase Greek (and the variant forms) → mathematical italic.
fn greek_italic(c: char) -> char {
    let v = c as u32;
    let m = match c {
        'α'..='ω' => 0x1D6FC + (v - 'α' as u32),
        '∂' => 0x1D715,
        'ϵ' => 0x1D716,
        'ϑ' => 0x1D717,
        'ϰ' => 0x1D718,
        'ϕ' => 0x1D719,
        'ϱ' => 0x1D71A,
        'ϖ' => 0x1D71B,
        _ => return c,
    };
    char::from_u32(m).unwrap_or(c)
}

/// Greek capital/small letter → the math alphabet starting at `cap` / `small`.
fn greek(c: char, cap: u32, small: u32) -> char {
    let v = c as u32;
    let m = match c {
        'Α'..='Ω' => cap + (v - 'Α' as u32),
        'α'..='ω' => small + (v - 'α' as u32),
        _ => return c,
    };
    char::from_u32(m).unwrap_or(c)
}
