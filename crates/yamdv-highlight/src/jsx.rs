//! JSX elements inside the JS/TS lexer (`jsx`, `tsx`): tags, attributes, text children and
//! `{…}` expressions. The element stack lives in [`State::ext`](crate::State::ext).

use crate::{Cur, Out, Tok, is_ident_start};

/// Maximum element depth kept across lines (4 bits per level in `ext`).
const MAX_DEPTH: u8 = 6;

/// Pending tag head across a line break.
pub(crate) const OPEN: u8 = 1;
pub(crate) const CLOSE: u8 = 2;

/// Open-element stack: per level the brace depth of the `{…}` child expression (0 = text).
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Jsx {
    lv: [u8; MAX_DEPTH as usize],
    len: u8,
    /// Inside a tag head that started on an earlier line (`OPEN` / `CLOSE`).
    pub(crate) pending: u8,
}

/// How a tag head ended.
pub(crate) enum End {
    Eol,
    Gt,
    SlashGt,
}

impl Jsx {
    pub(crate) fn unpack(ext: u32) -> Self {
        let mut j = Self {
            len: (ext & 7) as u8,
            pending: ((ext >> 3) & 3) as u8,
            ..Self::default()
        };
        j.len = j.len.min(MAX_DEPTH);
        for (k, l) in j.lv.iter_mut().enumerate() {
            *l = ((ext >> (8 + 4 * k)) & 15) as u8;
        }
        j
    }
    pub(crate) fn pack(&self) -> u32 {
        let mut e = u32::from(self.len) | u32::from(self.pending) << 3;
        let mut sh = 8;
        for l in &self.lv {
            e |= u32::from(*l & 15) << sh;
            sh += 4;
        }
        e
    }
    fn top(&mut self) -> Option<&mut u8> {
        self.len
            .checked_sub(1)
            .and_then(|k| self.lv.get_mut(usize::from(k)))
    }
    /// Element text children are being read (inside an element, outside `{…}`).
    pub(crate) fn in_children(&self) -> bool {
        self.len
            .checked_sub(1)
            .and_then(|k| self.lv.get(usize::from(k)))
            .is_some_and(|&b| b == 0)
    }
    /// Inside a `{…}` child expression.
    pub(crate) fn in_expr(&self) -> bool {
        self.len > 0 && !self.in_children()
    }
    /// `{` (`+1`) or `}` (`-1`) inside an expression; returns true when it closed the expression.
    pub(crate) fn brace(&mut self, open: bool) -> bool {
        let Some(t) = self.top() else { return false };
        if open {
            *t = (*t + 1).min(15);
            false
        } else {
            *t = t.saturating_sub(1);
            *t == 0
        }
    }
    /// Applies the end of a tag head; false when the line ended inside it.
    pub(crate) fn finish(&mut self, e: &End) -> bool {
        match e {
            End::Eol => return false,
            End::Gt if self.pending == OPEN => {
                if let Some(l) = self.lv.get_mut(usize::from(self.len)) {
                    *l = 0;
                    self.len += 1;
                }
            }
            End::Gt => self.len = self.len.saturating_sub(1),
            End::SlashGt => {}
        }
        self.pending = 0;
        true
    }
}

/// `<name`, `<>` (fragment) or, inside children, `</`.
pub(crate) fn tag_start(c: &Cur<'_>, children: bool) -> bool {
    if c.peek() != b'<' {
        return false;
    }
    let n = c.at(1);
    if n == b'/' {
        return children;
    }
    if n == b'>' {
        return true;
    }
    if !is_ident_start(n) {
        return false;
    }
    // TSX generic arrows `<T,>` / `<T extends X>` are not elements.
    let rest = c.s.get(c.i + 1..).unwrap_or("");
    let name = rest
        .find(|ch: char| !(ch.is_alphanumeric() || "_.-:$".contains(ch)))
        .unwrap_or(rest.len());
    let after = rest.get(name..).unwrap_or("").trim_start();
    !(after.starts_with(',') || after.starts_with("extends "))
}

/// Lexes `<` / `</` and the tag name at the cursor.
pub(crate) fn tag_open(c: &mut Cur<'_>, out: &mut Out) -> u8 {
    let st = c.i;
    let close = c.at(1) == b'/';
    c.i += if close { 2 } else { 1 };
    out.push(st..c.i, Tok::Punct);
    let r = c.ident(b"-:.$");
    out.push(r, Tok::Tag);
    // Generic element type arguments `<Box<string> …>`.
    if !close && c.peek() == b'<' {
        let st = c.i;
        let mut depth = 0u32;
        while !c.eof() {
            match c.peek() {
                b'<' => depth += 1,
                b'>' => depth -= 1,
                _ => {}
            }
            c.bump();
            if depth == 0 {
                break;
            }
        }
        out.push(st..c.i, Tok::Type);
    }
    if close { CLOSE } else { OPEN }
}

/// Attributes up to `>` / `/>` or the end of the line.
pub(crate) fn tag_body(c: &mut Cur<'_>, out: &mut Out) -> End {
    loop {
        c.skip_ws();
        if c.eof() {
            return End::Eol;
        }
        let st = c.i;
        let ch = c.peek();
        if c.starts("/>") {
            c.i += 2;
            out.push(st..c.i, Tok::Punct);
            return End::SlashGt;
        }
        match ch {
            b'>' => {
                c.i += 1;
                out.push(st..c.i, Tok::Punct);
                return End::Gt;
            }
            b'"' | b'\'' => {
                c.i += 1;
                while !c.eof() && c.peek() != ch {
                    c.bump();
                }
                c.i = (c.i + 1).min(c.b.len());
                out.push(st..c.i, Tok::Str);
            }
            b'=' => {
                c.i += 1;
                out.push(st..c.i, Tok::Operator);
            }
            b'{' => {
                // Attribute expression: balanced braces on this line, shown as text.
                c.i += 1;
                out.push(st..c.i, Tok::Punct);
                let mut depth = 1u32;
                let inner = c.i;
                while !c.eof() {
                    match c.peek() {
                        b'{' => depth += 1,
                        b'}' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    c.bump();
                }
                out.push(inner..c.i, Tok::Text);
                if !c.eof() {
                    out.push(c.i..c.i + 1, Tok::Punct);
                    c.i += 1;
                }
            }
            _ if is_ident_start(ch) => {
                let r = c.ident(b"-:.");
                out.push(r, Tok::Attribute);
            }
            _ => c.bump(),
        }
    }
}
