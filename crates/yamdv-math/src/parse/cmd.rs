//! Command dispatch (R-MATH-02): symbols, operators, fractions, radicals, accents, fonts, text,
//! spacing, boxes and the other structural commands.

use super::styled::set_font;
use super::{Font, FracStyle, MathError, Node, Parser, Stop, Tok, err};
use crate::extra;
use crate::symbols::{self, Class};

/// Structural commands handled by [`Parser::command`] (beyond the symbol tables); also used by
/// `\newcommand` to refuse redefining a built-in.
pub const STRUCTURAL: &[&str] = &[
    ",",
    ":",
    ">",
    ";",
    "!",
    " ",
    "quad",
    "qquad",
    "enspace",
    "thinspace",
    "medspace",
    "thickspace",
    "negthinspace",
    "negmedspace",
    "negthickspace",
    "nobreakspace",
    "space",
    "hspace",
    "kern",
    "mkern",
    "hskip",
    "mskip",
    "frac",
    "dfrac",
    "tfrac",
    "cfrac",
    "binom",
    "dbinom",
    "tbinom",
    "sqrt",
    "left",
    "middle",
    "right",
    "big",
    "Big",
    "bigg",
    "Bigg",
    "bigl",
    "Bigl",
    "biggl",
    "Biggl",
    "bigr",
    "Bigr",
    "biggr",
    "Biggr",
    "bigm",
    "Bigm",
    "biggm",
    "Biggm",
    "overline",
    "underline",
    "widehat",
    "widetilde",
    "overrightarrow",
    "overleftarrow",
    "overleftrightarrow",
    "underrightarrow",
    "underleftarrow",
    "overbrace",
    "underbrace",
    "overset",
    "underset",
    "stackrel",
    "text",
    "textrm",
    "textnormal",
    "textup",
    "mbox",
    "textbf",
    "textit",
    "emph",
    "textsf",
    "texttt",
    "operatorname",
    "operatornamewithlimits",
    "mathrm",
    "mathit",
    "mathbf",
    "mathbb",
    "mathcal",
    "mathscr",
    "mathsf",
    "mathtt",
    "mathfrak",
    "boldsymbol",
    "bm",
    "Bbb",
    "bold",
    "mathnormal",
    "not",
    "cancel",
    "bcancel",
    "xcancel",
    "boxed",
    "color",
    "textcolor",
    "tag",
    "displaystyle",
    "textstyle",
    "scriptstyle",
    "scriptscriptstyle",
    "phantom",
    "hphantom",
    "vphantom",
    "mod",
    "bmod",
    "pmod",
    "pod",
    "limits",
    "nolimits",
    "begin",
    "end",
    "newcommand",
    "renewcommand",
    "def",
    "\\",
];

/// Is `name` (without the backslash) a built-in command?
pub fn known(name: &str) -> bool {
    STRUCTURAL.contains(&name)
        || symbols::GREEK.iter().any(|g| g.0 == name)
        || extra::GREEK.iter().any(|g| g.0 == name)
        || symbols::SYMBOLS.iter().any(|g| g.0 == name)
        || extra::SYMBOLS.iter().any(|g| g.0 == name)
        || symbols::BIG_OPS.iter().any(|g| g.0 == name)
        || extra::BIG_OPS.iter().any(|g| g.0 == name)
        || symbols::FUNCS.iter().any(|g| g.0 == name)
        || extra::FUNCS.iter().any(|g| g.0 == name)
        || symbols::ACCENTS.iter().any(|g| g.0 == name)
        || extra::ACCENTS.iter().any(|g| g.0 == name)
}

/// Parses a TeX dimension (`1em`, `-2pt`, `3mu`, `0.5ex`) into em.
fn dimension(s: &str) -> Option<f32> {
    let s = s.trim();
    let split = s.find(|c: char| c.is_ascii_alphabetic()).unwrap_or(s.len());
    let (num, unit) = s.split_at(split);
    let v: f32 = num.trim().parse().ok()?;
    let k = match unit.trim() {
        "em" => 1.0,
        "ex" => 0.431,
        "mu" => 1.0 / 18.0,
        "pt" | "bp" | "px" => 0.1,
        "pc" => 1.2,
        "mm" => 0.2845,
        "cm" => 2.845,
        "in" => 7.227,
        _ => return None,
    };
    let em = v * k;
    (em.is_finite() && em.abs() <= 100.0).then_some(em)
}

/// Characters of `s` in `font` as math atoms (spaces kept as word spaces): `\textbf` & co.
fn text_in(s: &str, font: Font) -> Node {
    Node::Group(
        s.chars()
            .map(|c| {
                if c.is_whitespace() {
                    Node::Space(0.25)
                } else {
                    Node::Char {
                        c,
                        class: Class::Ord,
                        font,
                    }
                }
            })
            .collect(),
    )
}

fn txt(s: &str) -> Node {
    Node::Text(s.to_string())
}

impl Parser {
    /// A raw `{…}` argument or a single token's text (for `\hspace`, `\kern 1em`).
    fn dim_arg(&mut self) -> Result<f32, MathError> {
        let pos = self.pos();
        let s = if self.peek() == Some(&Tok::Open) {
            self.raw_arg()?
        } else {
            // `\kern1em`: digits, sign, dot and the unit letters as separate tokens.
            let mut s = String::new();
            while let Some(Tok::Char(c)) = self.peek() {
                let c = *c;
                if !(c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '+') || s.len() > 16
                {
                    break;
                }
                s.push(c);
                self.bump();
                if s.ends_with("em")
                    || s.ends_with("ex")
                    || s.ends_with("mu")
                    || s.ends_with("pt")
                    || s.ends_with("px")
                {
                    break;
                }
            }
            s
        };
        dimension(&s).map_or_else(|| err(format!("bad dimension '{s}'"), pos), Ok)
    }

    fn opt_bracket(&mut self) -> Result<Option<Node>, MathError> {
        if self.peek() != Some(&Tok::Char('[')) {
            return Ok(None);
        }
        self.bump();
        let l = self.list(Stop::Bracket)?;
        self.bump();
        Ok(Some(Node::Group(l)))
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn command(&mut self, name: &str, pos: usize) -> Result<Node, MathError> {
        if let Some((_, c)) = symbols::GREEK
            .iter()
            .chain(extra::GREEK.iter())
            .find(|g| g.0 == name)
        {
            return Ok(Node::Char {
                c: *c,
                class: Class::Ord,
                font: Font::Default,
            });
        }
        if let Some((_, c, class)) = symbols::SYMBOLS
            .iter()
            .chain(extra::SYMBOLS.iter())
            .find(|g| g.0 == name)
        {
            return Ok(Node::Char {
                c: *c,
                class: *class,
                font: Font::Roman,
            });
        }
        if let Some((_, c, limits)) = symbols::BIG_OPS
            .iter()
            .chain(extra::BIG_OPS.iter())
            .find(|g| g.0 == name)
        {
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
        if let Some((_, shown, limits)) = extra::FUNCS.iter().find(|g| g.0 == name) {
            return Ok(Node::Func {
                name: (*shown).to_string(),
                limits: *limits,
            });
        }
        if let Some(c) = symbols::ACCENTS
            .iter()
            .map(|a| (a.0, a.1))
            .chain(extra::ACCENTS.iter().copied())
            .find(|g| g.0 == name)
            .map(|g| g.1)
        {
            let body = self.arg()?;
            return Ok(Node::Accent {
                c,
                body: Box::new(body),
            });
        }
        let space = |em: f32| Ok(Node::Space(em));
        let stretch =
            |p: &mut Self, c: char, over: bool, limits: bool| -> Result<Node, MathError> {
                let body = p.arg()?;
                Ok(Node::Stretch {
                    c,
                    over,
                    limits,
                    body: Box::new(body),
                })
            };
        match name {
            "," | "thinspace" => space(3.0 / 18.0),
            ":" | ">" | "medspace" => space(4.0 / 18.0),
            ";" | "thickspace" => space(5.0 / 18.0),
            "!" | "negthinspace" => space(-3.0 / 18.0),
            "negmedspace" => space(-4.0 / 18.0),
            "negthickspace" => space(-5.0 / 18.0),
            " " | "nobreakspace" | "space" => space(1.0 / 3.0),
            "enspace" => space(0.5),
            "quad" => space(1.0),
            "qquad" => space(2.0),
            "hspace" | "kern" | "hskip" => {
                if self.peek() == Some(&Tok::Char('*')) {
                    self.bump();
                }
                Ok(Node::Space(self.dim_arg()?))
            }
            "mkern" | "mskip" => Ok(Node::Space(self.dim_arg()?)),
            "frac" | "dfrac" | "tfrac" | "cfrac" | "binom" | "dbinom" | "tbinom" => {
                let num = self.arg()?;
                let den = self.arg()?;
                let style = match name {
                    "dfrac" | "cfrac" | "dbinom" => FracStyle::Display,
                    "tfrac" | "tbinom" => FracStyle::Text,
                    _ => FracStyle::Auto,
                };
                let binom = name.ends_with("binom");
                Ok(Node::Frac {
                    num: Box::new(num),
                    den: Box::new(den),
                    style,
                    rule: !binom,
                    delims: binom.then_some(('(', ')')),
                })
            }
            "sqrt" => {
                let index = self.opt_bracket()?.map(Box::new);
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
            "widehat" => stretch(self, '\u{302}', true, false),
            "widetilde" => stretch(self, '\u{303}', true, false),
            "overrightarrow" => stretch(self, '→', true, false),
            "overleftarrow" => stretch(self, '←', true, false),
            "overleftrightarrow" => stretch(self, '↔', true, false),
            "underrightarrow" => stretch(self, '→', false, false),
            "underleftarrow" => stretch(self, '←', false, false),
            "overbrace" => stretch(self, '⏞', true, true),
            "underbrace" => stretch(self, '⏟', false, true),
            "overset" | "underset" | "stackrel" => {
                let script = self.arg()?;
                let base = self.arg()?;
                let class = match &base {
                    Node::Char { class, .. } => *class,
                    _ if name == "stackrel" => Class::Rel,
                    _ => Class::Ord,
                };
                let script = Some(Box::new(script));
                let (over, under) = if name == "underset" {
                    (None, script)
                } else {
                    (script, None)
                };
                Ok(Node::Stack {
                    base: Box::new(base),
                    over,
                    under,
                    class,
                })
            }
            "text" | "textrm" | "textnormal" | "textup" | "mbox" => Ok(Node::Text(self.raw_arg()?)),
            "textbf" => Ok(text_in(&self.raw_arg()?, Font::Bold)),
            "textit" | "emph" => Ok(text_in(&self.raw_arg()?, Font::Italic)),
            "textsf" => Ok(text_in(&self.raw_arg()?, Font::SansSerif)),
            "texttt" => Ok(text_in(&self.raw_arg()?, Font::Monospace)),
            "operatorname" | "operatornamewithlimits" => {
                let star = self.peek() == Some(&Tok::Char('*'));
                if star {
                    self.bump();
                }
                let s = self.raw_arg()?;
                let shown: String = s
                    .chars()
                    .filter(|c| !c.is_whitespace() && *c != '\\')
                    .collect();
                Ok(Node::Func {
                    name: shown,
                    limits: star || name == "operatornamewithlimits",
                })
            }
            "mathrm" | "mathit" | "mathbf" | "mathbb" | "Bbb" | "mathcal" | "mathscr"
            | "mathsf" | "mathtt" | "mathfrak" | "boldsymbol" | "bm" | "bold" | "mathnormal" => {
                let font = match name {
                    "mathrm" => Font::Roman,
                    "mathit" => Font::Italic,
                    "mathbf" | "bold" => Font::Bold,
                    "mathbb" | "Bbb" => Font::Blackboard,
                    "mathsf" => Font::SansSerif,
                    "mathtt" => Font::Monospace,
                    "mathfrak" => Font::Fraktur,
                    "boldsymbol" | "bm" => Font::BoldItalic,
                    "mathnormal" => Font::Default,
                    _ => Font::Calligraphic,
                };
                let mut body = self.arg()?;
                set_font(&mut body, font);
                Ok(body)
            }
            "not" => {
                let body = self.arg()?;
                if let Node::Char { c, class, font } = &body
                    && let Some(n) = extra::negate(*c)
                {
                    return Ok(Node::Char {
                        c: n,
                        class: *class,
                        font: *font,
                    });
                }
                Ok(Node::Cancel {
                    kind: 3,
                    body: Box::new(body),
                })
            }
            "cancel" | "bcancel" | "xcancel" => {
                let kind = match name {
                    "cancel" => 0,
                    "bcancel" => 1,
                    _ => 2,
                };
                Ok(Node::Cancel {
                    kind,
                    body: Box::new(self.arg()?),
                })
            }
            "boxed" => Ok(Node::Boxed(Box::new(self.arg()?))),
            "textcolor" => {
                let cname = self.raw_arg()?;
                let Some(color) = symbols::color(&cname) else {
                    return err(format!("unknown color '{cname}'"), pos);
                };
                let body = self.arg()?;
                Ok(Node::Color {
                    color,
                    body: vec![body],
                })
            }
            "tag" => {
                let star = self.peek() == Some(&Tok::Char('*'));
                if star {
                    self.bump();
                }
                let s = self.raw_arg()?;
                Ok(Node::Tag(if star { s } else { format!("({s})") }))
            }
            "phantom" | "hphantom" | "vphantom" => Ok(Node::Phantom {
                body: Box::new(self.arg()?),
                h: name != "vphantom",
                v: name != "hphantom",
            }),
            "bmod" => Ok(Node::Class {
                class: Class::Bin,
                body: Box::new(txt("mod")),
            }),
            "mod" | "pmod" | "pod" => {
                let x = self.arg()?;
                let mut v = vec![Node::Space(if name == "mod" { 1.0 } else { 0.444 })];
                if name != "mod" {
                    v.push(Node::Char {
                        c: '(',
                        class: Class::Open,
                        font: Font::Roman,
                    });
                }
                if name != "pod" {
                    v.push(txt("mod"));
                    v.push(Node::Space(6.0 / 18.0));
                }
                v.push(x);
                if name != "mod" {
                    v.push(Node::Char {
                        c: ')',
                        class: Class::Close,
                        font: Font::Roman,
                    });
                }
                Ok(Node::Group(v))
            }
            "begin" => self.env(pos),
            "newcommand" | "renewcommand" | "def" => err(
                format!("\\{name} must be expanded before parsing (internal)"),
                pos,
            ),
            _ => err(format!("unknown command \\{name}"), pos),
        }
    }
}
