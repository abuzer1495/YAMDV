//! R-MATH-02 coverage (docs/MATH_COVERAGE.md) and R-MATH-07 macro tests.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
use crate::font::MockFont;
use crate::layout::{Op, render, render_with};
use crate::macros::{MAX_STEPS, Macros, expand};
use crate::table::MathTable;

/// One sample per R-MATH-02 item, in SPEC order (mirrored by docs/MATH_COVERAGE.md).
pub(crate) const R_MATH_02: &[&str] = &[
    r"x^2_i",
    r"\sum\limits_{i=0}^n",
    r"\frac12",
    r"\dfrac ab",
    r"\tfrac ab",
    r"\cfrac{1}{1+\cfrac{1}{x}}",
    r"\binom nk",
    r"\dbinom nk",
    r"\tbinom nk",
    r"\sqrt[3]{x}",
    r"\sum",
    r"\prod",
    r"\coprod",
    r"\int",
    r"\iint",
    r"\iiint",
    r"\oint",
    r"\bigcup",
    r"\bigcap",
    r"\bigoplus",
    r"\bigotimes",
    r"\lim_{x\to0}",
    r"\limsup",
    r"\liminf",
    r"\max",
    r"\min",
    r"\sup",
    r"\inf",
    r"\det",
    r"\gcd",
    r"\int\nolimits_0^1",
    r"\int\limits_0^1",
    r"\left( \frac ab \middle| c \right)",
    r"\big( \Big[ \bigg\{ \Bigg\langle",
    r"\begin{matrix}a&b\\c&d\end{matrix}",
    r"\begin{pmatrix}a\end{pmatrix}",
    r"\begin{bmatrix}a\end{bmatrix}",
    r"\begin{Bmatrix}a\end{Bmatrix}",
    r"\begin{vmatrix}a\end{vmatrix}",
    r"\begin{Vmatrix}a\end{Vmatrix}",
    r"\begin{smallmatrix}a&b\end{smallmatrix}",
    r"\begin{cases}a&x>0\\b&\text{else}\end{cases}",
    r"\begin{dcases}a&x\end{dcases}",
    r"\begin{aligned}a&=b\\&=c\end{aligned}",
    r"\begin{align}a&=b\end{align}",
    r"\begin{align*}a&=b\end{align*}",
    r"\begin{gathered}a\\b\end{gathered}",
    r"\begin{gather}a\\b\end{gather}",
    r"\begin{split}a&=b\end{split}",
    r"\begin{array}{c|lr}a&b&c\\d&e&f\end{array}",
    r"\begin{equation}a\end{equation}",
    r"\hat a",
    r"\widehat{ab}",
    r"\bar a",
    r"\overline{ab}",
    r"\underline{ab}",
    r"\vec a",
    r"\overrightarrow{AB}",
    r"\overleftarrow{AB}",
    r"\dot a",
    r"\ddot a",
    r"\tilde a",
    r"\widetilde{ab}",
    r"\acute a",
    r"\grave a",
    r"\breve a",
    r"\check a",
    r"\mathring a",
    r"\overbrace{a+b}^{n}",
    r"\underbrace{a+b}_{n}",
    r"\overset{!}{=}",
    r"\underset{x}{=}",
    r"\stackrel{def}{=}",
    r"\mathrm{d}",
    r"\mathbf{v}",
    r"\mathit{x}",
    r"\mathsf{x}",
    r"\mathtt{x}",
    r"\mathcal{L}",
    r"\mathbb{R}",
    r"\mathfrak{g}",
    r"\mathscr{F}",
    r"\boldsymbol\alpha",
    r"\bm x",
    r"\text{if }",
    r"\textbf{b}",
    r"\textit{i}",
    r"\textrm{r}",
    r"\operatorname{tr}",
    r"\alpha\beta\gamma\Gamma\varepsilon\vartheta\varpi\varrho\varsigma\varphi\digamma",
    r"a\,b\:c\;d\!e\quad f\qquad g\hspace{1em}h",
    r"a \not= b",
    r"\cancel{x}",
    r"\boxed{x}",
    r"\color{red}{x}",
    r"\color{#00ff00}{x}",
    r"\textcolor{blue}{x}",
    r"E=mc^2 \tag{1}",
    r"\displaystyle\sum",
    r"\textstyle\sum",
    r"\scriptstyle x",
    r"\phantom{x}",
    r"a\mod b",
    r"a\bmod b",
    r"a\pmod b",
    r"\ldots\cdots\vdots\ddots",
    r"f'(x) f''",
];

fn ok(src: &str) -> bool {
    let f = MockFont::default();
    let tb = MathTable::new(&[], 1000.0);
    render(src, &f, &tb, 10.0, true).is_ok()
}

#[test]
fn r_math_02_list_and_symbol_vocabulary_render() {
    let missing: Vec<&&str> = R_MATH_02.iter().filter(|s| !ok(s)).collect();
    assert!(missing.is_empty(), "missing: {missing:?}");
    let mut names: Vec<&str> = crate::symbols::SYMBOLS.iter().map(|s| s.0).collect();
    names.extend(crate::extra::SYMBOLS.iter().map(|s| s.0));
    names.extend(crate::symbols::GREEK.iter().map(|s| s.0));
    names.extend(crate::extra::GREEK.iter().map(|s| s.0));
    names.extend(crate::symbols::BIG_OPS.iter().map(|s| s.0));
    names.extend(crate::extra::BIG_OPS.iter().map(|s| s.0));
    names.sort_unstable();
    let n = names.len();
    names.dedup();
    assert_eq!(n, names.len(), "duplicate symbol names");
    assert!(n >= 500, "{n} symbols");
    let bad: Vec<String> = names
        .iter()
        .map(|s| format!("\\{s}"))
        .filter(|s| !ok(s))
        .collect();
    assert!(bad.is_empty(), "{bad:?}");
    // Tag is split off and boxed; cancel draws a line; \boxed four rules.
    let f = MockFont::default();
    let tb = MathTable::new(&[], 1000.0);
    assert!(
        render(r"x \tag{2}", &f, &tb, 10.0, true)
            .unwrap()
            .tag
            .is_some()
    );
    let c = render(r"\cancel{x}", &f, &tb, 10.0, false).unwrap();
    assert_eq!(
        c.ops
            .iter()
            .filter(|o| matches!(o, Op::Line { .. }))
            .count(),
        1
    );
    let b = render(r"\boxed{x}", &f, &tb, 10.0, false).unwrap();
    assert_eq!(
        b.ops
            .iter()
            .filter(|o| matches!(o, Op::Rule { .. }))
            .count(),
        4
    );
    assert!(!ok(r"\left( a \middle| b") && !ok(r"a \middle| b") && !ok(r"\nosuchcommand"));
}

#[test]
fn macros_expand_in_document_order_and_stay_bounded() {
    let mut env = Macros::default();
    // Definitions are removed and recorded; later formulas see them.
    assert_eq!(
        expand(r"\newcommand{\RR}{\mathbb{R}} x", &mut env).unwrap(),
        " x"
    );
    assert_eq!(expand(r"\RR^n", &mut env).unwrap(), r"\mathbb{R}^n");
    assert_eq!(expand(r"\def\sq#1{#1^2}\sq{ab}", &mut env).unwrap(), "ab^2");
    assert_eq!(
        expand(r"\newcommand\p[2][x]{#1+#2} \p y \p[z]{w}", &mut env).unwrap(),
        " x+y z+w"
    );
    assert_eq!(expand(r"\renewcommand{\RR}{R}\RR", &mut env).unwrap(), "R");
    // Control words keep their separation after splicing; ## is a literal #.
    expand(r"\def\a{\alpha}", &mut env).unwrap();
    assert_eq!(expand(r"\a x\a", &mut env).unwrap(), r"\alpha x\alpha");
    assert_eq!(expand(r"\def\h{##}\h", &mut env).unwrap(), "#");
    // Without definitions the source is returned as is (borrowed).
    assert!(matches!(
        expand("a+b", &mut Macros::default()).unwrap(),
        std::borrow::Cow::Borrowed(_)
    ));
    // Errors: redefinition, renew of unknown, bad params, missing argument.
    for bad in [
        r"\newcommand{\RR}{x}",
        r"\newcommand{\frac}{x}",
        r"\renewcommand{\nothere}{x}",
        r"\newcommand{\q}[10]{x}",
        r"\def\d#2{x}",
        r"\sq",
    ] {
        assert!(expand(bad, &mut env).is_err(), "{bad}");
    }
    // Bombs: infinite recursion hits the step limit, doubling the token limit.
    let e = expand(r"\def\loop{\loop}\loop", &mut Macros::default()).unwrap_err();
    assert!(e.msg.contains(&format!("{MAX_STEPS} steps")), "{}", e.msg);
    let bomb = r"\def\a{xxxxxxxxxx}\def\b{\a\a\a\a\a\a\a\a\a\a}\def\c{\b\b\b\b\b\b\b\b\b\b}\def\d{\c\c\c\c\c\c\c\c\c\c}\def\e{\d\d\d\d\d\d\d\d\d\d}\e";
    let e = expand(bomb, &mut Macros::default()).unwrap_err();
    assert!(e.msg.contains("tokens"), "{}", e.msg);
    // The same environment drives layout (export and print use render_with too).
    let f = MockFont::default();
    let tb = MathTable::new(&[], 1000.0);
    let mut env = Macros::default();
    render_with(
        r"\newcommand{\half}{\frac12}",
        &mut env,
        &f,
        &tb,
        10.0,
        false,
    )
    .unwrap();
    let b = render_with(r"\half", &mut env, &f, &tb, 10.0, false).unwrap();
    assert_eq!(
        b.ops
            .iter()
            .filter(|o| matches!(o, Op::Rule { .. }))
            .count(),
        1
    );
    assert!(render_with(bomb, &mut Macros::default(), &f, &tb, 10.0, false).is_err());
}
