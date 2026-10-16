//! Built-in syntax highlighter (R-CODE-01): small hand-written lexers, no grammar files.
//!
//! Platform-independent (pure) crate; see `docs/ARCHITECTURE.md` §E.2. Every lexer works line by
//! line from a compact **start state** ([`State`]), so highlighting is lazy and virtualizable:
//! a caller can keep one `State` per line (or per N lines) and re-lex only what is visible.
#![forbid(unsafe_code)]
// Byte-index scanning over ASCII delimiters: every index is checked against the line length
// before use (see `Cur`); UTF-8 boundaries are kept by only splitting at ASCII bytes.
#![allow(clippy::indexing_slicing)]

mod clike;
mod langs;
mod markup;
mod misc;

use std::ops::Range;

pub use langs::{ALL_NAMES, Lang, lang_for};

/// Token class (R-CODE-01, colors per B.3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tok {
    /// Default text / whitespace.
    Text,
    /// Keyword.
    Keyword,
    /// String literal.
    Str,
    /// Regular expression literal.
    Regex,
    /// Number.
    Number,
    /// Comment.
    Comment,
    /// Function name (call or definition).
    Function,
    /// Type name.
    Type,
    /// Constant / builtin.
    Constant,
    /// Operator.
    Operator,
    /// Punctuation.
    Punct,
    /// Markup tag name.
    Tag,
    /// Attribute / property / key.
    Attribute,
    /// Variable (sigils, special variables).
    Variable,
    /// Meta / preprocessor / annotation.
    Meta,
    /// Diff added line.
    DiffAdd,
    /// Diff deleted line.
    DiffDel,
    /// Diff hunk header / file header (muted).
    DiffMeta,
    /// Markdown heading.
    Heading,
}

/// Start-of-line lexer state. Small and `Copy` so callers can store one per line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct State {
    /// Lexer-specific mode (0 = normal).
    pub mode: u8,
    /// Mode argument (delimiter index, nesting depth, …).
    pub arg: u8,
}

impl State {
    /// Normal state.
    pub const NORMAL: Self = Self { mode: 0, arg: 0 };
}

/// One token: byte range in the line and its class.
pub type Token = (Range<usize>, Tok);

/// Maximum line length lexed; the rest of a longer line is plain text (bounded work).
pub const MAX_LINE: usize = 10_000;

/// Lexes one line (without its line terminator) starting in `state`; returns the tokens
/// (covering the whole line, adjacent equal classes merged) and the state for the next line.
pub fn highlight_line(lang: Lang, state: State, line: &str) -> (Vec<Token>, State) {
    let (head, tail) = if line.len() > MAX_LINE {
        let mut cut = MAX_LINE;
        while !line.is_char_boundary(cut) {
            cut -= 1;
        }
        (&line[..cut], Some(cut))
    } else {
        (line, None)
    };
    let mut out = Out::default();
    let next = match lang.kind() {
        langs::Kind::CLike(spec) => clike::lex(spec, state, head, &mut out),
        langs::Kind::Markup => markup::xml(state, head, &mut out),
        langs::Kind::Css => markup::css(state, head, &mut out),
        langs::Kind::Markdown => markup::markdown(state, head, &mut out),
        langs::Kind::Json => misc::json(state, head, &mut out),
        langs::Kind::Yaml => misc::yaml(state, head, &mut out),
        langs::Kind::Toml => misc::toml(state, head, &mut out),
        langs::Kind::Ini => misc::ini(state, head, &mut out),
        langs::Kind::Diff => misc::diff(state, head, &mut out),
        langs::Kind::Regex => misc::regex(state, head, &mut out),
        langs::Kind::Plain => {
            out.push(0..head.len(), Tok::Text);
            state
        }
    };
    let mut toks = out.finish(head.len());
    if let Some(cut) = tail {
        toks.push((cut..line.len(), Tok::Text));
    }
    (toks, next)
}

/// Highlights a whole block; one token list per line (split at `\n`, `\r` stripped).
pub fn highlight_block(lang: Lang, text: &str) -> Vec<Vec<Token>> {
    let mut st = State::NORMAL;
    text.split('\n')
        .map(|l| {
            let l = l.strip_suffix('\r').unwrap_or(l);
            let (t, n) = highlight_line(lang, st, l);
            st = n;
            t
        })
        .collect()
}

/// Token collector: fills gaps with [`Tok::Text`] and merges neighbours of the same class.
#[derive(Debug, Default)]
pub(crate) struct Out {
    toks: Vec<Token>,
}

impl Out {
    pub(crate) fn push(&mut self, r: Range<usize>, t: Tok) {
        if r.start >= r.end {
            return;
        }
        let end = self.toks.last().map_or(0, |x| x.0.end);
        if r.start < end {
            return; // overlapping (should not happen); keep the earlier token
        }
        if r.start > end {
            self.push_merge(end..r.start, Tok::Text);
        }
        self.push_merge(r, t);
    }

    fn push_merge(&mut self, r: Range<usize>, t: Tok) {
        if let Some(last) = self.toks.last_mut()
            && last.1 == t
            && last.0.end == r.start
        {
            last.0.end = r.end;
            return;
        }
        self.toks.push((r, t));
    }

    fn finish(mut self, len: usize) -> Vec<Token> {
        let end = self.toks.last().map_or(0, |x| x.0.end);
        if end < len {
            self.push_merge(end..len, Tok::Text);
        }
        self.toks
    }
}

/// Byte cursor over one line.
pub(crate) struct Cur<'a> {
    pub(crate) s: &'a str,
    pub(crate) b: &'a [u8],
    pub(crate) i: usize,
}

impl<'a> Cur<'a> {
    pub(crate) fn new(s: &'a str) -> Self {
        Self {
            s,
            b: s.as_bytes(),
            i: 0,
        }
    }
    pub(crate) fn eof(&self) -> bool {
        self.i >= self.b.len()
    }
    pub(crate) fn peek(&self) -> u8 {
        self.at(0)
    }
    pub(crate) fn at(&self, k: usize) -> u8 {
        self.b.get(self.i + k).copied().unwrap_or(0)
    }
    pub(crate) fn starts(&self, p: &str) -> bool {
        self.b
            .get(self.i..)
            .is_some_and(|r| r.starts_with(p.as_bytes()))
    }
    pub(crate) fn starts_ci(&self, p: &str) -> bool {
        self.b
            .get(self.i..self.i + p.len())
            .is_some_and(|r| r.eq_ignore_ascii_case(p.as_bytes()))
    }
    /// Advances by one character (UTF-8 aware).
    pub(crate) fn bump(&mut self) {
        let c = self.s.get(self.i..).and_then(|r| r.chars().next());
        self.i += c.map_or(1, char::len_utf8);
    }
    pub(crate) fn skip_ws(&mut self) {
        while matches!(self.peek(), b' ' | b'\t') {
            self.i += 1;
        }
    }
    /// Reads an identifier (ASCII letters, digits, `_`, plus `extra` bytes and non-ASCII).
    pub(crate) fn ident(&mut self, extra: &[u8]) -> Range<usize> {
        let st = self.i;
        while !self.eof() {
            let c = self.peek();
            if c.is_ascii_alphanumeric() || c == b'_' || c >= 0x80 || extra.contains(&c) {
                self.bump();
            } else {
                break;
            }
        }
        st..self.i
    }
    pub(crate) fn text(&self, r: &Range<usize>) -> &'a str {
        self.s.get(r.clone()).unwrap_or("")
    }
    /// Reads a number literal (decimal, hex/octal/binary, floats, exponents, `_` separators,
    /// alphanumeric suffixes).
    pub(crate) fn number(&mut self) -> Range<usize> {
        let st = self.i;
        if self.peek() == b'0' && matches!(self.at(1) | 0x20, b'x' | b'o' | b'b') {
            self.i += 2;
        }
        while !self.eof() {
            let c = self.peek();
            if c.is_ascii_alphanumeric() || c == b'_' {
                let exp = matches!(c, b'e' | b'E') && matches!(self.at(1), b'+' | b'-');
                self.i += if exp { 2 } else { 1 };
            } else if c == b'.' && self.at(1).is_ascii_digit() {
                self.i += 1;
            } else {
                break;
            }
        }
        st..self.i
    }
}

pub(crate) fn is_ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_' || c >= 0x80
}

#[cfg(test)]
pub(crate) mod test_util {
    use super::*;

    /// Non-text tokens as (text, class).
    pub(crate) fn toks(lang: &str, src: &str) -> Vec<(String, Tok)> {
        let l = lang_for(lang).expect("language");
        let mut v = Vec::new();
        for (line, ts) in src.split('\n').zip(highlight_block(l, src)) {
            for (r, t) in ts {
                if t != Tok::Text {
                    v.push((line[r].to_string(), t));
                }
            }
        }
        v
    }

    /// Asserts that every expected (text, class) pair occurs, in order.
    pub(crate) fn expect(lang: &str, src: &str, want: &[(&str, Tok)]) {
        let got = toks(lang, src);
        let mut it = got.iter();
        for (w, t) in want {
            assert!(
                it.any(|(g, gt)| g.trim() == *w && gt == t),
                "{lang}: missing {w:?} as {t:?} (in order) in {got:?}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_cover_every_line_exactly() {
        for name in langs::ALL_NAMES {
            let l = lang_for(name).expect(name);
            let src = "fn main() { let s = \"x\\\"y\"; /* c */ } // t\n# h\n<a b=\"c\">d</a>\n- [x]: 1.5e3\n'''doc\nmore'''\n";
            for line in src.split('\n') {
                let (t, _) = highlight_line(l, State::NORMAL, line);
                let mut end = 0;
                for (r, _) in &t {
                    assert_eq!(r.start, end, "{name}: gap/overlap in {line:?}: {t:?}");
                    assert!(line.is_char_boundary(r.start) && line.is_char_boundary(r.end));
                    end = r.end;
                }
                assert_eq!(end, line.len(), "{name}: {line:?}");
            }
        }
    }

    #[test]
    fn unknown_language_is_none() {
        assert!(lang_for("klingon").is_none());
        assert!(lang_for("").is_none());
    }

    #[test]
    fn very_long_lines_are_bounded() {
        let l = lang_for("rust").expect("rust");
        let line = "a".repeat(MAX_LINE + 50);
        let (t, _) = highlight_line(l, State::NORMAL, &line);
        assert_eq!(t.last().map(|x| x.0.end), Some(line.len()));
    }
}
