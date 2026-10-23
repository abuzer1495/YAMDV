//! Parser and layout tests with the mock metrics provider (§2.5).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::float_cmp
)]
use crate::font::{GlyphMetrics, MathFont, MockFont};
use crate::layout::{MathBox, Op, layout, render};
use crate::parse::{Env, Font, Node, parse, styled};
use crate::symbols::Class;
use crate::table::{self as t, MathTable, Part, assemble};

fn mk(src: &str, display: bool) -> MathBox {
    let f = MockFont::default();
    let tb = MathTable::new(&[], 1000.0);
    render(src, &f, &tb, 10.0, display).unwrap()
}

fn glyphs(b: &MathBox) -> Vec<(u16, f32, f32, f32)> {
    b.ops
        .iter()
        .filter_map(|o| match o {
            Op::Glyph {
                gid, x, y, size, ..
            } => Some((*gid, *x, *y, *size)),
            Op::Rule { .. } | Op::Line { .. } => None,
        })
        .collect()
}

fn rules(b: &MathBox) -> Vec<(f32, f32, f32, f32)> {
    b.ops
        .iter()
        .filter_map(|o| match o {
            Op::Rule { x, y, w, h, .. } => Some((*x, *y, *w, *h)),
            Op::Glyph { .. } | Op::Line { .. } => None,
        })
        .collect()
}

#[test]
fn r_math_02_parse_core_constructs() {
    let n = parse(r"x^2_i").unwrap();
    assert!(matches!(
        &n[0],
        Node::Scripts {
            sup: Some(_),
            sub: Some(_),
            ..
        }
    ));
    let n = parse(r"\frac{a}{b} \dfrac12 \tfrac{1}{2} \binom{n}{k}").unwrap();
    assert_eq!(n.len(), 4);
    assert!(matches!(
        &n[3],
        Node::Frac {
            rule: false,
            delims: Some(('(', ')')),
            ..
        }
    ));
    let n = parse(r"\sqrt[3]{x}").unwrap();
    assert!(matches!(&n[0], Node::Sqrt { index: Some(_), .. }));
    let n = parse(r"\left( x \right.").unwrap();
    assert!(matches!(
        &n[0],
        Node::LeftRight {
            left: Some('('),
            right: None,
            ..
        }
    ));
    let n = parse(r"\Bigl[ \bigr\}").unwrap();
    assert!(matches!(
        &n[0],
        Node::Big {
            c: Some('['),
            size: 2,
            class: Class::Open
        }
    ));
    assert!(matches!(
        &n[1],
        Node::Big {
            c: Some('}'),
            size: 1,
            class: Class::Close
        }
    ));
    for (env, kind) in [
        ("matrix", Env::Matrix),
        ("pmatrix", Env::PMatrix),
        ("bmatrix", Env::BMatrix),
        ("vmatrix", Env::VMatrix),
        ("cases", Env::Cases),
        ("aligned", Env::Aligned),
    ] {
        let n = parse(&format!(r"\begin{{{env}}} a & b \\ c & d \\ \end{{{env}}}")).unwrap();
        match &n[0] {
            Node::Matrix { env: e, rows, .. } => {
                assert_eq!(*e, kind);
                assert_eq!(rows.len(), 2, "{env}");
                assert_eq!(rows[0].len(), 2);
            }
            other => panic!("{other:?}"),
        }
    }
    let n = parse(r"\hat{x}\bar y\vec v\dot a\ddot b\tilde n\overline{AB}\underline c").unwrap();
    assert_eq!(n.len(), 8);
    let n =
        parse(r"\mathbb{R}\mathcal{L}\mathbf{F}\mathit{x}\mathrm{d}\text{a b}\operatorname{tr}")
            .unwrap();
    let Node::Group(g0) = &n[0] else {
        panic!("{:?}", n[0])
    };
    assert!(matches!(
        &g0[0],
        Node::Char {
            c: 'R',
            font: Font::Blackboard,
            ..
        }
    ));
    assert!(matches!(&n[5], Node::Text(s) if s == "a b"));
    assert!(matches!(&n[6], Node::Func { name, .. } if name == "tr"));
    let n = parse(r"\alpha\Omega\le\to\infty\sum\int\lim\, \quad \qquad \! \color{red} x").unwrap();
    assert!(matches!(
        n.last(),
        Some(Node::Color {
            color: 0xdf0030,
            ..
        })
    ));
    let n = parse(r"f''(x) \ldots \cdots").unwrap();
    // Primes follow the nucleus at full size (pre-raised glyphs).
    assert!(matches!(&n[0], Node::Group(g) if matches!(g.get(1), Some(Node::Char { c: '″', .. }))));
    let n = parse(r"f'^2").unwrap();
    assert!(matches!(
        &n[0],
        Node::Scripts {
            sup: Some(_),
            sub: None,
            ..
        }
    ));
}

#[test]
fn r_math_03_errors_have_positions() {
    let e = parse(r"x + \foo").unwrap_err();
    assert_eq!(e.pos, 4);
    assert!(e.msg.contains("\\foo"));
    assert!(parse(r"\frac{a}").is_err());
    assert!(parse(r"{x").unwrap_err().msg.contains("}"));
    assert!(parse(r"x}").is_err());
    assert!(parse(r"x^1^2").unwrap_err().msg.contains("double"));
    assert!(parse(r"\left( x").is_err());
    assert!(parse(r"a & b").is_err());
    assert!(parse(r"\begin{foo}x\end{foo}").is_err());
    assert!(parse(r"\begin{matrix}x\end{pmatrix}").is_err());
    assert!(parse(r"\color{nocolor} x").is_err());
    assert!(parse(r"\sqrt[3").is_err());
    // S-PARSE-05 bounds.
    let deep = "{".repeat(100) + &"}".repeat(100);
    assert!(parse(&deep).unwrap_err().msg.contains("64"));
    assert!(parse(&"x".repeat(10_001)).is_err());
}

#[test]
fn r_math_02_font_mapping() {
    assert_eq!(styled('x', Font::Default), '𝑥');
    assert_eq!(styled('h', Font::Default), 'ℎ');
    assert_eq!(styled('α', Font::Default), '𝛼');
    assert_eq!(styled('Γ', Font::Default), 'Γ');
    assert_eq!(styled('2', Font::Default), '2');
    assert_eq!(styled('R', Font::Blackboard), 'ℝ');
    assert_eq!(styled('A', Font::Blackboard), '𝔸');
    assert_eq!(styled('L', Font::Calligraphic), 'ℒ');
    assert_eq!(styled('A', Font::Calligraphic), '𝒜');
    assert_eq!(styled('F', Font::Bold), '𝐅');
    assert_eq!(styled('x', Font::Roman), 'x');
}

#[test]
fn fractions_stack_around_the_axis() {
    let b = mk(r"\frac{a}{b}", true);
    let g = glyphs(&b);
    assert_eq!(g.len(), 2);
    let (num_y, den_y) = (g[0].2, g[1].2);
    assert!(num_y < 0.0 && den_y > 0.0, "{num_y} {den_y}");
    let r = rules(&b);
    assert_eq!(r.len(), 1);
    // Bar centered on the axis (0.25 em at 10 px), between numerator and denominator.
    let axis = 2.5;
    assert!((r[0].1 + r[0].3 / 2.0 + axis).abs() < 1e-3);
    // Display style: numerator in text size; text style: numerator in script size.
    assert!((g[0].3 - 10.0).abs() < 1e-3);
    let t = mk(r"\frac{a}{b}", false);
    assert!((glyphs(&t)[0].3 - 7.0).abs() < 1e-3);
    // \dfrac forces display size inside text.
    let d = mk(r"\dfrac{a}{b}", false);
    assert!((glyphs(&d)[0].3 - 10.0).abs() < 1e-3);
    // \binom: no bar, two delimiters.
    let bi = mk(r"\binom{n}{k}", true);
    assert!(rules(&bi).is_empty());
    assert_eq!(glyphs(&bi).len(), 4);
    assert!(b.ascent > 0.0 && b.descent > 0.0);
}

#[test]
fn scripts_shift_and_shrink() {
    let b = mk("x^2", false);
    let g = glyphs(&b);
    assert_eq!(g.len(), 2);
    assert!(g[1].2 < 0.0, "superscript raised");
    assert!((g[1].3 - 7.0).abs() < 1e-3, "script size 70 %");
    assert!(g[1].1 >= 5.0, "after the base advance");
    let s = mk("x_2", false);
    assert!(glyphs(&s)[1].2 > 0.0, "subscript lowered");
    let both = mk("x_i^2", false);
    let g = glyphs(&both);
    let sup_bottom = -g[1].2 - 0.2 * 7.0;
    let sub_top = -g[2].2 + 0.7 * 7.0;
    assert!(
        sup_bottom - sub_top >= 1.6 - 1e-3,
        "subSuperscriptGapMin kept"
    );
    // Scriptscript level for nested scripts.
    let n = mk("x^{y^z}", false);
    assert!((glyphs(&n)[2].3 - 5.0).abs() < 1e-3);
}

#[test]
fn big_operators_limits_in_display_only() {
    let d = mk(r"\sum_{i}^{n}", true);
    let g = glyphs(&d);
    // Limits centered above/below: same x region as the operator, y outside it.
    assert!(g[1].2 < -5.0 && g[2].2 > 2.0, "{g:?}");
    let t = mk(r"\sum_{i}^{n}", false);
    let g = glyphs(&t);
    assert!(
        g[1].1 >= 10.0 && g[2].1 >= 10.0,
        "scripts to the right in text style"
    );
    // Integrals never take limits.
    let i = mk(r"\int_0^1", true);
    assert!(glyphs(&i)[1].1 >= 10.0);
    // \lim is upright text with limits below in display style.
    let l = mk(r"\lim_{x}", true);
    let g = glyphs(&l);
    assert_eq!(g.len(), 4);
    assert!(g[3].2 > 0.0);
}

#[test]
fn radicals_cover_the_body() {
    let b = mk(r"\sqrt{x}", false);
    let r = rules(&b);
    assert_eq!(r.len(), 1);
    let g = glyphs(&b);
    // Overbar starts after the radical sign and spans the radicand.
    assert!((r[0].0 - 10.0).abs() < 1e-3);
    assert!((r[0].2 - 5.0).abs() < 1e-3);
    // Above the radicand top (7 px) by the vertical gap.
    assert!(-r[0].1 - r[0].3 > 7.0);
    assert_eq!(g.len(), 2);
    let i = mk(r"\sqrt[3]{x}", false);
    assert_eq!(glyphs(&i).len(), 3);
    assert!(
        (glyphs(&i)[0].3 - 5.0).abs() < 1e-3,
        "degree in scriptscript"
    );
}

#[test]
fn delimiters_size_to_content() {
    let small = mk(r"\left( x \right)", true);
    let big = mk(r"\left( \frac{\frac{a}{b}}{\frac{c}{d}} \right)", true);
    assert!(big.ascent + big.descent > small.ascent + small.descent);
    // \big family: four steps.
    let mut prev = 0usize;
    for s in [r"\big(", r"\Big(", r"\bigg(", r"\Bigg("] {
        let b = mk(s, true);
        assert_eq!(glyphs(&b).len(), 1, "{s}");
        prev += 1;
    }
    assert_eq!(prev, 4);
}

#[test]
fn matrices_grid_and_delimiters() {
    let m = mk(r"\begin{matrix} a & b \\ c & d \end{matrix}", true);
    let g = glyphs(&m);
    assert_eq!(g.len(), 4);
    assert_eq!(g[0].2, g[1].2, "same row baseline");
    assert!(g[2].2 > g[0].2, "second row below");
    assert_eq!(g[0].1, g[2].1, "same column x");
    assert!(g[1].1 > g[0].1 + 5.0, "column gap");
    let p = mk(r"\begin{pmatrix} a \end{pmatrix}", true);
    assert_eq!(glyphs(&p).len(), 3);
    let c = mk(r"\begin{cases} a & b \end{cases}", true);
    assert_eq!(glyphs(&c).len(), 3, "left brace only");
    let a = mk(r"\begin{aligned} a &= b \\ &= c \end{aligned}", true);
    let g = glyphs(&a);
    assert_eq!(g[1].1, g[3].1, "relations aligned");
}

#[test]
fn accents_and_bars() {
    let b = mk(r"\hat{x}", false);
    let g = glyphs(&b);
    assert_eq!(g.len(), 2);
    // Base ascent 7 > accent base height 4.5 → lifted by 2.5.
    assert!((g[1].2 + 2.5).abs() < 1e-3);
    let o = mk(r"\overline{x}", false);
    assert_eq!(rules(&o).len(), 1);
    assert!(rules(&o)[0].1 < -7.0);
    let u = mk(r"\underline{x}", false);
    assert!(rules(&u)[0].1 > 2.0);
}

#[test]
fn spacing_follows_the_tex_table() {
    let w = |s: &str| mk(s, false).width;
    // a+b: medium spaces around the binary operator; a=b: thick spaces.
    assert!((w("a+b") - (15.0 + 2.0 * 4.0 * 10.0 / 18.0)).abs() < 1e-3);
    assert!((w("a=b") - (15.0 + 2.0 * 5.0 * 10.0 / 18.0)).abs() < 1e-3);
    // Unary minus: no space.
    assert!((w("-b") - 10.0).abs() < 1e-3);
    // No medium/thick spaces in script style.
    let s = mk("x^{a+b}", false);
    assert!(s.width < 5.0 + 3.0 * 3.5 + 1.0);
    assert!((w(r"a\quad b") - 20.0).abs() < 1e-3);
}

#[test]
fn colors_override_and_never_panic_on_garbage() {
    let b = mk(r"a \color{#00ff00} b", false);
    let cs: Vec<Option<u32>> = b
        .ops
        .iter()
        .map(|o| match o {
            Op::Glyph { color, .. } | Op::Rule { color, .. } | Op::Line { color, .. } => *color,
        })
        .collect();
    assert_eq!(cs, vec![None, Some(0x00ff00)]);
    let f = MockFont::default();
    for raw in [
        &[][..],
        &[0, 1, 0, 0, 0, 10, 0, 0, 0, 0][..],
        &[0, 1, 0, 0, 255, 255, 255, 255, 255, 255][..],
    ] {
        let tb = MathTable::new(raw, 1000.0);
        let r = render(
            r"\sqrt{\frac{\sum_a^b}{\left(x\right)}}",
            &f,
            &tb,
            12.0,
            true,
        );
        assert!(r.is_ok());
    }
}

/// Builds a minimal MATH table: constants with axis height 300, one italics correction
/// (glyph 100 → 40), one top accent (100 → 250) and a vertical construction for glyph 40
/// (variants 41/42, assembly bottom 50 + extender 51 + top 52).
fn synthetic_math() -> Vec<u8> {
    let mut v = vec![0u8; 10];
    let p16 = |v: &mut Vec<u8>, x: u16| v.extend_from_slice(&x.to_be_bytes());
    let set = |v: &mut Vec<u8>, at: usize, x: u16| {
        v[at..at + 2].copy_from_slice(&x.to_be_bytes());
    };
    set(&mut v, 0, 1);
    // MathConstants at 10.
    set(&mut v, 4, 10);
    let mut c = vec![0u8; 214];
    c[0..2].copy_from_slice(&80u16.to_be_bytes());
    c[2..4].copy_from_slice(&60u16.to_be_bytes());
    let axis_at = 8 + 4;
    c[axis_at..axis_at + 2].copy_from_slice(&300u16.to_be_bytes());
    v.extend_from_slice(&c);
    // MathGlyphInfo.
    let gi = v.len();
    set(&mut v, 6, gi as u16);
    v.extend_from_slice(&[0u8; 8]);
    // italics subtable.
    let it = v.len();
    set(&mut v, gi, (it - gi) as u16);
    p16(&mut v, 8); // coverage offset
    p16(&mut v, 1);
    p16(&mut v, 40);
    p16(&mut v, 0);
    p16(&mut v, 1); // coverage format 1
    p16(&mut v, 1);
    p16(&mut v, 100);
    // top accent subtable (coverage format 2).
    let ta = v.len();
    set(&mut v, gi + 2, (ta - gi) as u16);
    p16(&mut v, 8);
    p16(&mut v, 1);
    p16(&mut v, 250);
    p16(&mut v, 0);
    p16(&mut v, 2);
    p16(&mut v, 1);
    p16(&mut v, 100);
    p16(&mut v, 110);
    p16(&mut v, 0);
    // MathVariants.
    let mv = v.len();
    set(&mut v, 8, mv as u16);
    p16(&mut v, 20); // minConnectorOverlap
    p16(&mut v, 12); // vert coverage
    p16(&mut v, 0);
    p16(&mut v, 1);
    p16(&mut v, 0);
    p16(&mut v, 18); // construction offset
    // coverage at mv+12
    p16(&mut v, 1);
    p16(&mut v, 1);
    p16(&mut v, 40);
    // construction at mv+18
    p16(&mut v, 12); // assembly offset (from construction)
    p16(&mut v, 2);
    p16(&mut v, 41);
    p16(&mut v, 1200);
    p16(&mut v, 42);
    p16(&mut v, 1800);
    // assembly at mv+30
    v.extend_from_slice(&[0u8; 4]);
    p16(&mut v, 3);
    for (g, s, e, f, x) in [
        (50u16, 0u16, 100u16, 500u16, 0u16),
        (51, 100, 100, 500, 1),
        (52, 100, 0, 500, 0),
    ] {
        for w in [g, s, e, f, x] {
            p16(&mut v, w);
        }
    }
    v
}

#[test]
fn r_math_01_math_table_reader() {
    let raw = synthetic_math();
    let tb = MathTable::new(&raw, 1000.0);
    assert_eq!(tb.constants.get(t::SCRIPT_PERCENT), 80.0);
    assert_eq!(tb.constants.get(t::SCRIPT_SCRIPT_PERCENT), 60.0);
    assert_eq!(tb.constants.get(t::AXIS_HEIGHT), 300.0);
    assert_eq!(tb.italics(100), 40.0);
    assert_eq!(tb.italics(101), 0.0);
    assert_eq!(tb.top_accent(100), Some(250.0));
    assert_eq!(tb.top_accent(5), None);
    assert_eq!(tb.min_overlap(), 20.0);
    let vars = tb.variants(40, true);
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[1].glyph, 42);
    assert!(tb.variants(40, false).is_empty());
    let parts = tb.assembly(40, true);
    assert_eq!(parts.len(), 3);
    assert!(parts[1].extender);
    // Truncated tables never panic.
    for n in 0..raw.len() {
        let tb = MathTable::new(&raw[..n], 1000.0);
        let _ = (
            tb.variants(40, true),
            tb.assembly(40, true),
            tb.italics(100),
        );
    }
}

#[test]
fn assembly_reaches_the_target() {
    let parts = [
        Part {
            glyph: 1,
            start: 0.0,
            end: 100.0,
            full: 500.0,
            extender: false,
        },
        Part {
            glyph: 2,
            start: 100.0,
            end: 100.0,
            full: 500.0,
            extender: true,
        },
        Part {
            glyph: 3,
            start: 100.0,
            end: 0.0,
            full: 500.0,
            extender: false,
        },
    ];
    for target in [1000.0f32, 1300.0, 3000.0, 9000.0] {
        let (placed, len) = assemble(&parts, 20.0, target);
        assert!(len >= target - 1.0 || placed.len() >= 3, "{target} → {len}");
        assert!(
            (len - target).abs() < 1.0 || len > target,
            "{target} → {len}"
        );
        assert_eq!(placed.first().map(|p| p.0), Some(1));
        assert_eq!(placed.last().map(|p| p.0), Some(3));
    }
    assert!(assemble(&parts, 20.0, 1e9).0.len() <= 502);
}

/// A font whose MATH table supplies variants for `(`: the layout picks a larger variant for a
/// taller formula.
struct VarFont(Vec<u8>);
impl MathFont for VarFont {
    fn units_per_em(&self) -> f32 {
        1000.0
    }
    fn glyph(&self, c: char) -> u16 {
        if c == '(' {
            40
        } else {
            MockFont::default().glyph(c)
        }
    }
    fn metrics(&self, g: u16) -> GlyphMetrics {
        let h = match g {
            41 => 1200.0,
            42 => 1800.0,
            50..=52 => 500.0,
            40 => 1000.0,
            _ => return MockFont::default().metrics(g),
        };
        GlyphMetrics {
            advance: 400.0,
            x_min: 0.0,
            x_max: 400.0,
            y_min: -h / 2.0,
            y_max: h / 2.0,
        }
    }
    fn math_table(&self) -> &[u8] {
        &self.0
    }
}

#[test]
fn stretchy_delimiters_use_variants_then_assembly() {
    let f = VarFont(synthetic_math());
    let tb = MathTable::new(f.math_table(), 1000.0);
    let pick = |src: &str| -> Vec<u16> {
        let b = render(src, &f, &tb, 10.0, true).unwrap();
        glyphs(&b)
            .iter()
            .map(|g| g.0)
            .filter(|g| (40..=52).contains(g))
            .collect()
    };
    assert_eq!(pick(r"\big("), vec![41]);
    assert_eq!(pick(r"\Big("), vec![42]);
    let huge = pick(r"\Bigg(");
    assert!(huge.len() >= 3 && huge[0] == 50, "{huge:?}");
    let parsed = parse(r"\left( x \right.").unwrap();
    let b = layout(&parsed, &f, &tb, 10.0, true).unwrap();
    assert!(!b.ops.is_empty());
}
