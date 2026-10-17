//! Bounded TeX-math tokenizer and parser (§2.2, S-PARSE-05): the FUT-17 core subset → [`Node`]s.
//! Unknown commands are errors (R-MATH-03). Linear in the input; recursion depth ≤ 64.

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
    /// `\mathcal`.
    Calligraphic,
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
    /// `aligned`.
    Aligned,
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
    let chars: Vec<char> = src.chars().collect();
    if chars.len() > MAX_CHARS {
        return err(format!("formula longer than {MAX_CHARS} characters"), 0);
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
    fn scripts(&mut self, base: Node) -> Result<Node, MathError> {
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
            // Unicode math fonts draw primes pre-raised: they sit at the superscript position
            // as ordinary glyphs, followed by an explicit superscript if any.
            sup = Some(match sup {
                None => Node::Group(vec![prime]),
                Some(s) => Node::Group(vec![prime, s]),
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
        symbols::delimiter(&s).map_or_else(|| err(format!("bad delimiter {s}"), pos), Ok)
    }

    #[allow(clippy::too_many_lines)]
    fn command(&mut self, name: &str, pos: usize) -> Result<Node, MathError> {
        if let Some((_, c)) = symbols::GREEK.iter().find(|g| g.0 == name) {
            return Ok(Node::Char {
                c: *c,
                class: Class::Ord,
                font: Font::Default,
            });
        }
        if let Some((_, c, class)) = symbols::SYMBOLS.iter().find(|g| g.0 == name) {
            return Ok(Node::Char {
                c: *c,
                class: *class,
                font: Font::Roman,
            });
        }
        if let Some((_, c, limits)) = symbols::BIG_OPS.iter().find(|g| g.0 == name) {
            return Ok(Node::BigOp {
                c: *c,
                limits: *limits,
            });
        }
        if let Some((n, limits)) = symbols::FUNCS.iter().find(|g| g.0 == name) {
            return Ok(Node::Func {
                name: (*n).to_string(),
                limits: *limits,
            });
        }
        if let Some((_, c, _)) = symbols::ACCENTS.iter().find(|g| g.0 == name) {
            let body = self.arg()?;
            return Ok(Node::Accent {
                c: *c,
                body: Box::new(body),
            });
        }
        let space = |em: f32| Ok(Node::Space(em));
        match name {
            "," => space(3.0 / 18.0),
            ":" | ">" => space(4.0 / 18.0),
            ";" => space(5.0 / 18.0),
            "!" => space(-3.0 / 18.0),
            " " => space(1.0 / 3.0),
            "quad" => space(1.0),
            "qquad" => space(2.0),
            "frac" | "dfrac" | "tfrac" | "binom" => {
                let num = self.arg()?;
                let den = self.arg()?;
                let style = match name {
                    "dfrac" => FracStyle::Display,
                    "tfrac" => FracStyle::Text,
                    _ => FracStyle::Auto,
                };
                let binom = name == "binom";
                Ok(Node::Frac {
                    num: Box::new(num),
                    den: Box::new(den),
                    style,
                    rule: !binom,
                    delims: binom.then_some(('(', ')')),
                })
            }
            "sqrt" => {
                let index = if self.peek() == Some(&Tok::Char('[')) {
                    self.bump();
                    let l = self.list(Stop::Bracket)?;
                    self.bump();
                    Some(Box::new(Node::Group(l)))
                } else {
                    None
                };
                let body = self.arg()?;
                Ok(Node::Sqrt {
                    index,
                    body: Box::new(body),
                })
            }
            "left" => {
                let left = self.delim_tok()?;
                let body = self.list(Stop::Right)?;
                self.bump();
                let right = self.delim_tok()?;
                Ok(Node::LeftRight { left, right, body })
            }
            "big" | "Big" | "bigg" | "Bigg" | "bigl" | "Bigl" | "biggl" | "Biggl" | "bigr"
            | "Bigr" | "biggr" | "Biggr" | "bigm" | "Bigm" | "biggm" | "Biggm" => {
                let size = match name.trim_end_matches(['l', 'r', 'm']) {
                    "big" => 1,
                    "Big" => 2,
                    "bigg" => 3,
                    _ => 4,
                };
                let class = match name.chars().last() {
                    Some('l') => Class::Open,
                    Some('r') => Class::Close,
                    Some('m') => Class::Rel,
                    _ => Class::Ord,
                };
                let c = self.delim_tok()?;
                Ok(Node::Big { c, size, class })
            }
            "overline" => Ok(Node::Overline(Box::new(self.arg()?))),
            "underline" => Ok(Node::Underline(Box::new(self.arg()?))),
            "text" => {
                let s = self.raw_arg()?;
                Ok(Node::Text(s))
            }
            "operatorname" => {
                let s = self.raw_arg()?;
                let name: String = s
                    .chars()
                    .filter(|c| !c.is_whitespace() && *c != '\\')
                    .collect();
                Ok(Node::Func {
                    name,
                    limits: false,
                })
            }
            "mathrm" | "mathit" | "mathbf" | "mathbb" | "mathcal" => {
                let font = match name {
                    "mathrm" => Font::Roman,
                    "mathit" => Font::Italic,
                    "mathbf" => Font::Bold,
                    "mathbb" => Font::Blackboard,
                    _ => Font::Calligraphic,
                };
                let mut body = self.arg()?;
                set_font(&mut body, font);
                Ok(body)
            }
            "begin" => self.env(pos),
            _ => err(format!("unknown command \\{name}"), pos),
        }
    }

    fn env(&mut self, pos: usize) -> Result<Node, MathError> {
        let name = self.raw_arg()?;
        let env = match name.as_str() {
            "matrix" => Env::Matrix,
            "pmatrix" => Env::PMatrix,
            "bmatrix" => Env::BMatrix,
            "vmatrix" => Env::VMatrix,
            "cases" => Env::Cases,
            "aligned" => Env::Aligned,
            _ => return err(format!("unknown environment {name}"), pos),
        };
        let mut rows: Vec<Vec<Node>> = vec![Vec::new()];
        loop {
            let cell = self.list(Stop::Cell)?;
            self.count()?;
            if let Some(r) = rows.last_mut() {
                r.push(Node::Group(cell));
            }
            let p = self.pos();
            match self.bump() {
                Some(Tok::Amp) => {}
                Some(Tok::Cmd(c)) if c == "\\" => rows.push(Vec::new()),
                Some(Tok::Cmd(c)) if c == "end" => {
                    let end = self.raw_arg()?;
                    if end != name {
                        return err(
                            format!("\\end{{{end}}} does not match \\begin{{{name}}}"),
                            p,
                        );
                    }
                    break;
                }
                _ => return err(format!("missing \\end{{{name}}}"), p),
            }
        }
        // A trailing `\\` leaves an empty last row.
        if rows.len() > 1
            && rows.last().is_some_and(|r| {
                r.len() == 1 && matches!(r.first(), Some(Node::Group(g)) if g.is_empty())
            })
        {
            rows.pop();
        }
        Ok(Node::Matrix { env, rows })
    }
}

/// Applies `font` to every character in `n` that has the default font.
fn set_font(n: &mut Node, font: Font) {
    match n {
        Node::Char { font: f, .. } => {
            if *f == Font::Default || font != Font::Roman {
                *f = font;
            }
        }
        Node::Group(v) | Node::Color { body: v, .. } | Node::LeftRight { body: v, .. } => {
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
        Node::Sqrt { body, .. }
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
