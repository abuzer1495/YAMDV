# Math coverage (R-MATH-02, R-MATH-07) — V2-G

Every R-MATH-02 item renders without error; checked by `tests_cov::r_math_02_list_and_symbol_vocabulary_render`
(one sample per item, in SPEC order) and the symbol-vocabulary check (≥ 500 distinct names, no duplicates,
each `\name` renders).

**Coverage: 123/123 MUST items done, 0 missing.**

| Group | Commands | Items | Done | Status |
|---|---|---|---|---|
| Scripts | ^ _ (limits rules) | 2 | 2 | done |
| Fractions | \frac \dfrac \tfrac \cfrac \binom \dbinom \tbinom | 7 | 7 | done |
| Roots | \sqrt[n]{} | 1 | 1 | done |
| Big operators | \sum \prod \coprod \int \iint \iiint \oint \bigcup \bigcap \bigoplus \bigotimes \lim \limsup \liminf \max \min \sup \inf \det \gcd | 20 | 20 | done |
| Limits | \limits \nolimits | 2 | 2 | done |
| Delimiters | \left \right \middle \big \Big \bigg \Bigg | 7 | 7 | done |
| Environments | matrix pmatrix bmatrix Bmatrix vmatrix Vmatrix smallmatrix cases dcases aligned align align* gathered gather split array{…} equation | 17 | 17 | done |
| Rows/columns | \\ & | 2 | 2 | done |
| Accents | \hat \widehat \bar \overline \underline \vec \overrightarrow \overleftarrow \dot \ddot \tilde \widetilde \acute \grave \breve \check \mathring | 17 | 17 | done |
| Braces | \overbrace{}^{} \underbrace{}_{} | 2 | 2 | done |
| Stacking | \overset \underset \stackrel | 3 | 3 | done |
| Fonts | \mathrm \mathbf \mathit \mathsf \mathtt \mathcal \mathbb \mathfrak \mathscr \boldsymbol \bm \text \textbf \textit \textrm \operatorname | 16 | 16 | done |
| Spacing | \, \: \; \! \quad \qquad \hspace{} | 7 | 7 | done |
| Decorations | \not \cancel \boxed | 3 | 3 | done |
| Color | \color{name\|#hex}{…} \textcolor | 2 | 2 | done |
| Numbering | \tag{} | 1 | 1 | done |
| Styles | \displaystyle \textstyle \scriptstyle | 3 | 3 | done |
| Phantom | \phantom | 1 | 1 | done |
| Modulo | \mod \bmod \pmod | 3 | 3 | done |
| Dots | \ldots \cdots \vdots \ddots | 4 | 4 | done |
| Primes | ' | 1 | 1 | done |
| Greek | all letters and variants (`\var…`, `\digamma`, upright `\Alpha`…) | 1 | 1 | done |
| Symbols | ≈ 500 symbols/relations/arrows/operators (KaTeX core list) | 1 | 1 | done (≥ 500 names, unit-checked) |

## Macros (R-MATH-07, SHOULD — done)

- `\newcommand{\x}[n][default]{body}`, `\renewcommand`, `\def\x#1#2{body}` with `#1`…`#9`, `##` = `#`.
- Scope: document order. A definition applies to the rest of its formula and every later top-level block;
  *deviation:* inside the defining block it applies to the whole block (blocks are laid out as a unit).
  HTML export expands strictly in document order with one running environment (same engine, `render_with`).
- Limits (S-PARSE-05): ≤ 10 000 expansion steps and ≤ 100 000 produced tokens per formula, ≤ 1 000 macros;
  over a limit the formula shows in the error color with the message (R-MATH-03).
- Errors: `\newcommand` of a built-in or a different existing macro, `\renewcommand` of an unknown name,
  > 9 parameters, delimited `\def` parameters, missing arguments.

## Notes

- `align`/`gather`/`equation` render as their `aligned`/`gathered` forms inside the math block (no automatic numbering;
  `\tag{}` gives the number, right-aligned).
- `\middle` is only valid inside `\left … \right`.
