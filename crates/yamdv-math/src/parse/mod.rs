//! Bounded TeX-math tokenizer and parser (§2.2, S-PARSE-05): the R-MATH-02 subset → [`Node`]s.
//! Unknown commands are errors (R-MATH-03). Linear in the input; recursion depth ≤ 64.
//! Commands live in `cmd`, environments in `env`, font mapping in `styled`.

mod cmd;
mod env;
mod styled;

pub use cmd::known;
pub use styled::styled;

use crate::symbols::{self, Class};

/// Maximum formula length in characters (S-PARSE-05).
pub const MAX_CHARS: usize = 10_000;
/// Maximum nesting depth (S-PARSE-05).
pub const MAX_DEPTH: usize = 64;
/// Maximum number of AST nodes per formula.
pub const MAX_NODES: usize = 100_000;

/// Character font.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Font {
    /// Default math font: Latin and lowercase Greek letters italic, the rest upright.
    Default,
    /// `\mathrm`, `\text`, operator names.
    Roman,
    /// `\mathit`.
    Italic,
    /// `\mathbf`.
    Bold,
    /// `\mathbb`.
    Blackboard,
    /// `\mathcal`, `\mathscr`.
    Calligraphic,
    /// `\mathsf`, `\textsf`.
    SansSerif,
    /// `\mathtt`, `\texttt`.
    Monospace,
    /// `\mathfrak`.
    Fraktur,
    /// `\boldsymbol`, `\bm`.
    BoldItalic,
}

/// Fraction style override.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FracStyle {
    /// `\frac`, `\binom`: from the context.
    Auto,
    /// `\dfrac`.
    Display,
    /// `\tfrac`.
    Text,
}

/// Matrix-like environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Env {
    /// `matrix`.
    Matrix,
    /// `pmatrix`.
    PMatrix,
    /// `bmatrix`.
    BMatrix,
    /// `vmatrix`.
    VMatrix,
    /// `cases`.
    Cases,
    /// `aligned`, `align`, `align*`, `split`.
    Aligned,
    /// `Bmatrix`.
    BraceMatrix,
    /// `Vmatrix`.
    DoubleVMatrix,
    /// `smallmatrix`.
    Small,
    /// `dcases`.
    DCases,
    /// `gathered`, `gather`, `gather*`, `equation`, `equation*`.
    Gathered,
    /// `array{…}` (column spec in [`Node::Matrix::cols`]).
    Array,
}

/// Parsed math node.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// A character.
    Char {
        /// Character as written (before the font mapping).
        c: char,
        /// Atom class.
        class: Class,
        /// Font.
        font: Font,
    },
    /// `\text{…}` (upright, spaces kept).
    Text(String),
    /// Operator name (`\sin`, `\lim`, `\operatorname{…}`).
    Func {
        /// Name.
        name: String,
        /// Limits above/below in display style.
        limits: bool,
    },
    /// Big operator (`\sum` …).
    BigOp {
        /// Glyph.
        c: char,
        /// Limits above/below in display style.
        limits: bool,
    },
    /// `{…}`.
    Group(Vec<Node>),
    /// Sub-/superscripts.
    Scripts {
        /// Nucleus.
        base: Box<Node>,
        /// Superscript.
        sup: Option<Box<Node>>,
        /// Subscript.
        sub: Option<Box<Node>>,
    },
    /// `\frac`, `\dfrac`, `\tfrac`, `\binom`.
    Frac {
        /// Numerator.
        num: Box<Node>,
        /// Denominator.
        den: Box<Node>,
        /// Style override.
        style: FracStyle,
        /// Fraction bar (false for `\binom`).
        rule: bool,
        /// Delimiters (`\binom`: parentheses).
        delims: Option<(char, char)>,
    },
    /// `\sqrt[n]{…}`.
    Sqrt {
        /// Degree.
        index: Option<Box<Node>>,
        /// Radicand.
        body: Box<Node>,
    },
    /// `\left … \right`.
    LeftRight {
        /// Left delimiter (`None` = `.`).
        left: Option<char>,
        /// Right delimiter.
        right: Option<char>,
        /// Content.
        body: Vec<Node>,
    },
    /// `\big` family.
    Big {
        /// Delimiter.
        c: Option<char>,
        /// 1 = `\big`, 2 = `\Big`, 3 = `\bigg`, 4 = `\Bigg`.
        size: u8,
        /// Class (`\bigl` = open, `\bigr` = close, otherwise ord).
        class: Class,
    },
    /// Environment with rows of cells.
    Matrix {
        /// Kind.
        env: Env,
        /// Rows of cells.
        rows: Vec<Vec<Node>>,
        /// `array` column spec: `l`/`c`/`r`, `|` for a vertical rule (empty for other kinds).
        cols: Vec<char>,
    },
    /// Accent over the body.
    Accent {
        /// Combining accent glyph.
        c: char,
        /// Base.
        body: Box<Node>,
    },
    /// `\overline`.
    Overline(Box<Node>),
    /// `\underline`.
    Underline(Box<Node>),
    /// Horizontal space in em.
    Space(f32),
    /// `\color{c}`: the rest of the group in `color`.
    Color {
        /// `0xRRGGBB`.
        color: u32,
        /// Colored content.
        body: Vec<Node>,
    },
    /// `\displaystyle` … `\scriptscriptstyle`: the rest of the group in style `level` (0–3).
    Style {
        /// 0 display, 1 text, 2 script, 3 scriptscript.
        level: u8,
        /// Content.
        body: Vec<Node>,
    },
    /// `\limits` / `\nolimits` after an operator.
    Limits {
        /// The operator.
        base: Box<Node>,
        /// Limits above/below in every style (`false` = always at the side).
        on: bool,
    },
    /// `\middle` delimiter inside `\left … \right`.
    Middle(Option<char>),
    /// A horizontally stretched glyph over or under the body (`\widehat`, `\overrightarrow`,
    /// `\overbrace`, …).
    Stretch {
        /// Base glyph (stretched with the MATH table's horizontal variants and assemblies).
        c: char,
        /// Above (`true`) or below.
        over: bool,
        /// Scripts become limits (`\overbrace{}^{}`, `\underbrace{}_{}`).
        limits: bool,
        /// Content.
        body: Box<Node>,
    },
    /// `\overset`, `\underset`, `\stackrel`.
    Stack {
        /// Base.
        base: Box<Node>,
        /// Above (script style).
        over: Option<Box<Node>>,
        /// Below (script style).
        under: Option<Box<Node>>,
        /// Atom class of the result.
        class: Class,
    },
    /// `\phantom` (both), `\hphantom` (width), `\vphantom` (height and depth).
    Phantom {
        /// Content (measured, not drawn).
        body: Box<Node>,
        /// Keep the width.
        h: bool,
        /// Keep height and depth.
        v: bool,
    },
    /// `\cancel` (0), `\bcancel` (1), `\xcancel` (2), `\not` without a precomposed form (3).
    Cancel {
        /// Kind.
        kind: u8,
        /// Content.
        body: Box<Node>,
    },
    /// `\boxed`.
    Boxed(Box<Node>),
    /// `\tag{…}`: the equation number (right-aligned by the caller, R-MATH-02).
    Tag(String),
    /// An atom with an explicit class (`\bmod`).
    Class {
        /// Class.
        class: Class,
        /// Content.
        body: Box<Node>,
    },
}

/// Parse error (R-MATH-03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MathError {
    /// Message.
    pub msg: String,
    /// Character offset in the formula.
    pub pos: usize,
}

impl std::fmt::Display for MathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (position {})", self.msg, self.pos + 1)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Cmd(String),
    Char(char),
    Open,
    Close,
    Sup,
    Sub,
    Amp,
    Prime,
    Tilde,
}

struct Parser {
    toks: Vec<(Tok, usize)>,
    /// Raw characters (for `\text` and color names).
    chars: Vec<char>,
    i: usize,
    depth: usize,
    nodes: usize,
}

/// Where a list ends.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stop {
    Eof,
    Brace,
    Bracket,
    Right,
    Cell,
}

fn tokenize(chars: &[char]) -> Vec<(Tok, usize)> {
    let mut out = Vec::new();
    let mut k = 0;
    while let Some(&c) = chars.get(k) {
        let start = k;
        k += 1;
        let t = match c {
            '\\' => {
                let mut name = String::new();
                while let Some(&n) = chars.get(k) {
                    if n.is_ascii_alphabetic() {
                        name.push(n);
                        k += 1;
                    } else {
                        break;
                    }
                }
                if name.is_empty()
                    && let Some(&n) = chars.get(k)
                {
                    name.push(n);
                    k += 1;
                }
                Tok::Cmd(name)
            }
            '{' => Tok::Open,
            '}' => Tok::Close,
            '^' => Tok::Sup,
            '_' => Tok::Sub,
            '&' => Tok::Amp,
            '\'' => Tok::Prime,
            '~' => Tok::Tilde,
            c if c.is_whitespace() => continue,
            c => Tok::Char(c),
        };
        out.push((t, start));
    }
    out
}

fn err<T>(msg: impl Into<String>, pos: usize) -> Result<T, MathError> {
    Err(MathError {
        msg: msg.into(),
        pos,
    })
}

/// Parses a formula (without delimiters).
pub fn parse(src: &str) -> Result<Vec<Node>, MathError> {
    parse_limited(src, MAX_CHARS)
}

/// Maximum length of a formula after macro expansion (R-MATH-07; the expansion itself is
/// bounded by `macros::MAX_TOKENS`).
pub const MAX_EXPANDED: usize = 4 * crate::macros::MAX_TOKENS;

/// Parses a formula of at most `max` characters.
pub fn parse_limited(src: &str, max: usize) -> Result<Vec<Node>, MathError> {
    let chars: Vec<char> = src.chars().collect();
    if chars.len() > max {
        return err(format!("formula longer than {max} characters"), 0);
    }
    let mut p = Parser {
        toks: tokenize(&chars),
        chars,
        i: 0,
        depth: 0,
        nodes: 0,
    };
    let list = p.list(Stop::Eof)?;
    if let Some((_, pos)) = p.toks.get(p.i) {
        return err("unexpected token", *pos);
    }
    Ok(list)
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.i).map(|t| &t.0)
    }
    fn pos(&self) -> usize {
        self.toks.get(self.i).map_or(self.chars.len(), |t| t.1)
    }
    fn bump(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.i).map(|t| t.0.clone());
        self.i += 1;
        t
    }
    fn count(&mut self) -> Result<(), MathError> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return err("formula too large", self.pos());
        }
        Ok(())
    }

    fn enter(&mut self) -> Result<(), MathError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return err("nesting deeper than 64 levels", self.pos());
        }
        Ok(())
    }

    fn list(&mut self, stop: Stop) -> Result<Vec<Node>, MathError> {
        self.enter()?;
        let mut out: Vec<Node> = Vec::new();
        loop {
            let pos = self.pos();
            match self.peek() {
                None => {
                    if stop == Stop::Eof {
                        break;
                    }
                    let what = match stop {
                        Stop::Brace => "missing }",
                        Stop::Bracket => "missing ]",
                        Stop::Right => "missing \\right",
                        _ => "missing \\end",
                    };
                    return err(what, pos);
                }
                Some(Tok::Close) => {
                    if stop == Stop::Brace {
                        break;
                    }
                    return err("unexpected }", pos);
                }
                Some(Tok::Char(']')) if stop == Stop::Bracket => break,
                Some(Tok::Cmd(c)) if c == "right" => {
                    if stop == Stop::Right {
                        break;
                    }
                    return err("\\right without \\left", pos);
                }
                Some(Tok::Amp) => {
                    if stop == Stop::Cell {
                        break;
                    }
                    return err("& outside an environment", pos);
                }
                Some(Tok::Cmd(c)) if c == "\\" || c == "end" => {
                    if stop == Stop::Cell {
                        break;
                    }
                    return err(format!("\\{c} outside an environment"), pos);
                }
                Some(Tok::Cmd(c)) if c == "color" => {
                    self.bump();
                    let name = self.raw_arg()?;
                    let Some(color) = symbols::color(&name) else {
                        return err(format!("unknown color '{name}'"), pos);
                    };
                    let body = self.list(stop)?;
                    self.count()?;
                    out.push(Node::Color { color, body });
                    break;
                }
                Some(Tok::Cmd(c))
                    if matches!(
                        c.as_str(),
                        "displaystyle" | "textstyle" | "scriptstyle" | "scriptscriptstyle"
                    ) =>
                {
                    let level = match c.as_str() {
                        "displaystyle" => 0,
                        "textstyle" => 1,
                        "scriptstyle" => 2,
                        _ => 3,
                    };
                    self.bump();
                    let body = self.list(stop)?;
                    self.count()?;
                    out.push(Node::Style { level, body });
                    break;
                }
                Some(Tok::Cmd(c)) if c == "middle" => {
                    if stop != Stop::Right {
                        return err("\\middle outside \\left … \\right", pos);
                    }
                    self.bump();
                    let d = self.delim_tok()?;
                    self.count()?;
                    out.push(Node::Middle(d));
                }
                Some(Tok::Sup | Tok::Sub | Tok::Prime) => {
                    // A script without a nucleus attaches to an empty group.
                    let base = Node::Group(Vec::new());
                    let n = self.scripts(base)?;
                    out.push(n);
                }
                Some(_) => {
                    let atom = self.atom()?;
                    let n = self.scripts(atom)?;
                    out.push(n);
                }
            }
        }
        self.depth -= 1;
        Ok(out)
    }

    /// Attaches `^`, `_` and primes following a nucleus.
    fn scripts(&mut self, mut base: Node) -> Result<Node, MathError> {
        while let Some(Tok::Cmd(c)) = self.peek() {
            let on = match c.as_str() {
                "limits" => true,
                "nolimits" => false,
                _ => break,
            };
            let pos = self.pos();
            if !matches!(
                base,
                Node::BigOp { .. } | Node::Func { .. } | Node::Limits { .. }
            ) {
                return err("\\limits or \\nolimits not after an operator", pos);
            }
            self.bump();
            if let Node::Limits { base: b, .. } = base {
                base = *b;
            }
            self.count()?;
            base = Node::Limits {
                base: Box::new(base),
                on,
            };
        }
        let mut sup: Option<Node> = None;
        let mut sub: Option<Node> = None;
        let mut primes = 0usize;
        loop {
            let pos = self.pos();
            match self.peek() {
                Some(Tok::Prime) => {
                    self.bump();
                    primes += 1;
                }
                Some(Tok::Sup) => {
                    self.bump();
                    if sup.is_some() {
                        return err("double superscript", pos);
                    }
                    sup = Some(self.arg()?);
                }
                Some(Tok::Sub) => {
                    self.bump();
                    if sub.is_some() {
                        return err("double subscript", pos);
                    }
                    sub = Some(self.arg()?);
                }
                _ => break,
            }
        }
        if primes > 0 {
            let p = match primes {
                1 => '′',
                2 => '″',
                _ => '‴',
            };
            let prime = Node::Char {
                c: p,
                class: Class::Ord,
                font: Font::Roman,
            };
            // Unicode math fonts draw primes pre-raised (like text-font primes): they follow the
            // nucleus at full size, and any scripts attach to nucleus + prime.
            self.count()?;
            let base = Node::Group(vec![base, prime]);
            if sup.is_none() && sub.is_none() {
                return Ok(base);
            }
            self.count()?;
            return Ok(Node::Scripts {
                base: Box::new(base),
                sup: sup.map(Box::new),
                sub: sub.map(Box::new),
            });
        }
        if sup.is_none() && sub.is_none() {
            return Ok(base);
        }
        self.count()?;
        Ok(Node::Scripts {
            base: Box::new(base),
            sup: sup.map(Box::new),
            sub: sub.map(Box::new),
        })
    }

    /// A mandatory argument: a group or a single atom (no scripts).
    fn arg(&mut self) -> Result<Node, MathError> {
        let pos = self.pos();
        match self.peek() {
            None => err("missing argument", pos),
            Some(Tok::Close | Tok::Amp | Tok::Sup | Tok::Sub) => err("missing argument", pos),
            _ => self.atom(),
        }
    }

    /// Raw text of a `{…}` argument (balanced braces), for `\text` and color names.
    fn raw_arg(&mut self) -> Result<String, MathError> {
        let pos = self.pos();
        if self.peek() != Some(&Tok::Open) {
            return err("expected {", pos);
        }
        let start = pos + 1;
        let mut depth = 0usize;
        let mut end = None;
        while let Some((t, p)) = self.toks.get(self.i).cloned() {
            self.i += 1;
            match t {
                Tok::Open => depth += 1,
                Tok::Close => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(p);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(end) = end else {
            return err("missing }", pos);
        };
        Ok(self
            .chars
            .get(start..end)
            .map(|s| s.iter().collect())
            .unwrap_or_default())
    }

    fn group(&mut self) -> Result<Node, MathError> {
        self.bump();
        let list = self.list(Stop::Brace)?;
        self.bump();
        self.count()?;
        Ok(Node::Group(list))
    }

    fn atom(&mut self) -> Result<Node, MathError> {
        self.enter()?;
        let pos = self.pos();
        self.count()?;
        let n = match self.bump() {
            None => return err("unexpected end", pos),
            Some(Tok::Open) => {
                self.i -= 1;
                self.group()?
            }
            Some(Tok::Char(c)) => Node::Char {
                c: if c == '-' { '−' } else { c },
                class: symbols::char_class(c),
                font: Font::Default,
            },
            Some(Tok::Tilde) => Node::Space(1.0 / 3.0),
            Some(Tok::Cmd(name)) => self.command(&name, pos)?,
            Some(_) => return err("unexpected token", pos),
        };
        self.depth -= 1;
        Ok(n)
    }

    fn delim_tok(&mut self) -> Result<Option<char>, MathError> {
        let pos = self.pos();
        let s = match self.bump() {
            Some(Tok::Char(c)) => c.to_string(),
            Some(Tok::Cmd(n)) => format!("\\{n}"),
            _ => return err("missing delimiter", pos),
        };
        if let Some(d) = symbols::delimiter(&s) {
            return Ok(d);
        }
        crate::extra::delimiter(&s)
            .map_or_else(|| err(format!("bad delimiter {s}"), pos), |c| Ok(Some(c)))
    }
}
