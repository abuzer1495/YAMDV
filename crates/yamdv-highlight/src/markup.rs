//! Markup-like lexers: XML/HTML, CSS/SCSS/LESS, Markdown.

use crate::{Cur, Out, State, Tok, is_ident_start};

const X_COMMENT: u8 = 1;
const X_TAG: u8 = 2;
const X_ATTRSTR: u8 = 3;
const X_CDATA: u8 = 4;

fn scan_until(c: &mut Cur<'_>, end: &str) -> bool {
    while !c.eof() {
        if c.starts(end) {
            c.i += end.len();
            return true;
        }
        c.bump();
    }
    false
}

/// Inside a tag: attributes until `>` / `/>`. Returns `Some(state)` when the line ended first.
fn tag_body(c: &mut Cur<'_>, out: &mut Out, mut pending_str: Option<u8>) -> Option<State> {
    loop {
        if let Some(q) = pending_str.take() {
            let st = c.i;
            let qc = if q == 1 { b'"' } else { b'\'' };
            while !c.eof() && c.peek() != qc {
                c.bump();
            }
            if c.eof() {
                out.push(st..c.i, Tok::Str);
                return Some(State {
                    mode: X_ATTRSTR,
                    arg: q,
                });
            }
            c.i += 1;
            out.push(st..c.i, Tok::Str);
        }
        c.skip_ws();
        if c.eof() {
            return Some(State {
                mode: X_TAG,
                arg: 0,
            });
        }
        let st = c.i;
        let ch = c.peek();
        if c.starts("/>") {
            c.i += 2;
            out.push(st..c.i, Tok::Punct);
            return None;
        }
        if ch == b'>' || (ch == b'?' && c.at(1) == b'>') {
            c.i += if ch == b'?' { 2 } else { 1 };
            out.push(st..c.i, Tok::Punct);
            return None;
        }
        if ch == b'"' || ch == b'\'' {
            c.i += 1;
            out.push(st..c.i, Tok::Str);
            pending_str = Some(if ch == b'"' { 1 } else { 2 });
            continue;
        }
        if ch == b'=' {
            c.i += 1;
            out.push(st..c.i, Tok::Operator);
            continue;
        }
        if is_ident_start(ch) || ch == b':' || ch == b'@' || ch == b'#' || ch == b'v' {
            let r = c.ident(b"-:.@#");
            out.push(r, Tok::Attribute);
            continue;
        }
        c.bump();
    }
}

/// XML / HTML / SVG / XAML / Vue templates.
pub(crate) fn xml(state: State, line: &str, out: &mut Out) -> State {
    let mut c = Cur::new(line);
    match state.mode {
        X_COMMENT => {
            let closed = scan_until(&mut c, "-->");
            out.push(0..c.i, Tok::Comment);
            if !closed {
                return state;
            }
        }
        X_CDATA => {
            let closed = scan_until(&mut c, "]]>");
            out.push(0..c.i, Tok::Str);
            if !closed {
                return state;
            }
        }
        X_TAG | X_ATTRSTR => {
            let pend = (state.mode == X_ATTRSTR).then_some(state.arg);
            if let Some(s) = tag_body(&mut c, out, pend) {
                return s;
            }
        }
        _ => {}
    }
    while !c.eof() {
        let st = c.i;
        if c.starts("<!--") {
            c.i += 4;
            let closed = scan_until(&mut c, "-->");
            out.push(st..c.i, Tok::Comment);
            if !closed {
                return State {
                    mode: X_COMMENT,
                    arg: 0,
                };
            }
            continue;
        }
        if c.starts("<![CDATA[") {
            c.i += 9;
            let closed = scan_until(&mut c, "]]>");
            out.push(st..c.i, Tok::Str);
            if !closed {
                return State {
                    mode: X_CDATA,
                    arg: 0,
                };
            }
            continue;
        }
        if c.peek() == b'<' && (is_ident_start(c.at(1)) || matches!(c.at(1), b'/' | b'?' | b'!')) {
            c.i += 1;
            if matches!(c.peek(), b'/' | b'?' | b'!') {
                c.i += 1;
            }
            out.push(st..c.i, Tok::Punct);
            let r = c.ident(b"-:.");
            out.push(r, Tok::Tag);
            if let Some(s) = tag_body(&mut c, out, None) {
                return s;
            }
            continue;
        }
        if c.peek() == b'&' {
            let rest = c.s.get(c.i..).unwrap_or("");
            if let Some(k) = rest.find(';').filter(|&k| k <= 10 && k > 1) {
                c.i += k + 1;
                out.push(st..c.i, Tok::Constant);
                continue;
            }
        }
        if c.starts("{{") {
            let closed = scan_until(&mut c, "}}");
            let _ = closed;
            out.push(st..c.i, Tok::Variable);
            continue;
        }
        c.bump();
    }
    State::NORMAL
}

const C_COMMENT: u8 = 1;

/// CSS / SCSS / LESS. `State::arg` = brace depth (also kept inside comments).
pub(crate) fn css(state: State, line: &str, out: &mut Out) -> State {
    let mut c = Cur::new(line);
    let mut depth = state.arg;
    if state.mode == C_COMMENT {
        let closed = scan_until(&mut c, "*/");
        out.push(0..c.i, Tok::Comment);
        if !closed {
            return state;
        }
    }
    let mut value = false; // after `prop:` inside a block
    while !c.eof() {
        let st = c.i;
        let ch = c.peek();
        if c.starts("/*") {
            c.i += 2;
            let closed = scan_until(&mut c, "*/");
            out.push(st..c.i, Tok::Comment);
            if !closed {
                return State {
                    mode: C_COMMENT,
                    arg: depth,
                };
            }
            continue;
        }
        if c.starts("//") {
            out.push(st..c.b.len(), Tok::Comment);
            break;
        }
        match ch {
            b' ' | b'\t' => c.skip_ws(),
            b'{' => {
                depth = depth.saturating_add(1);
                value = false;
                c.i += 1;
                out.push(st..c.i, Tok::Punct);
            }
            b'}' => {
                depth = depth.saturating_sub(1);
                value = false;
                c.i += 1;
                out.push(st..c.i, Tok::Punct);
            }
            b';' => {
                value = false;
                c.i += 1;
                out.push(st..c.i, Tok::Punct);
            }
            b'"' | b'\'' => {
                c.i += 1;
                while !c.eof() && c.peek() != ch {
                    if c.peek() == b'\\' {
                        c.i += 1;
                    }
                    c.bump();
                }
                c.i = (c.i + 1).min(c.b.len());
                out.push(st..c.i, Tok::Str);
            }
            b'@' => {
                c.i += 1;
                let _ = c.ident(b"-");
                out.push(st..c.i, Tok::Keyword);
            }
            b'$' => {
                c.i += 1;
                let _ = c.ident(b"-");
                out.push(st..c.i, Tok::Variable);
            }
            b'!' if c.starts("!important") => {
                c.i += 10;
                out.push(st..c.i, Tok::Keyword);
            }
            b'#' if value => {
                c.i += 1;
                let _ = c.ident(&[]);
                out.push(st..c.i, Tok::Constant);
            }
            b'#' | b'.' if !value && (is_ident_start(c.at(1)) || c.at(1) == b'-') => {
                c.i += 1;
                let _ = c.ident(b"-");
                out.push(st..c.i, if ch == b'#' { Tok::Constant } else { Tok::Type });
            }
            b':' => {
                c.i += 1;
                let rest = c.s.get(c.i..).unwrap_or("");
                let selector_line = rest
                    .find('{')
                    .is_some_and(|b| rest.find(';').is_none_or(|sc| b < sc));
                if depth > 0 && !value && !selector_line {
                    value = true;
                    out.push(st..c.i, Tok::Punct);
                } else {
                    let _ = c.ident(b"-:");
                    out.push(st..c.i, Tok::Function);
                }
            }
            b'0'..=b'9' => {
                let r = c.number();
                while c.peek() == b'%' {
                    c.i += 1;
                }
                out.push(r.start..c.i, Tok::Number);
            }
            b'.' if c.at(1).is_ascii_digit() => {
                let r = c.number();
                out.push(r, Tok::Number);
            }
            _ if is_ident_start(ch) || ch == b'-' => {
                let r = c.ident(b"-");
                let w = c.text(&r);
                let t = if value {
                    if c.peek() == b'(' {
                        Tok::Function
                    } else {
                        Tok::Constant
                    }
                } else if depth > 0 {
                    // property (followed by ':') or nested selector
                    let mut k = c.i;
                    while matches!(c.b.get(k), Some(b' ')) {
                        k += 1;
                    }
                    if c.b.get(k) == Some(&b':')
                        && !matches!(c.b.get(k + 1), Some(b':'))
                        && !w.is_empty()
                    {
                        Tok::Attribute
                    } else {
                        Tok::Tag
                    }
                } else {
                    Tok::Tag
                };
                out.push(r, t);
            }
            _ => {
                c.bump();
                if b",>+~*()[]=".contains(&ch) {
                    out.push(st..c.i, Tok::Operator);
                }
            }
        }
    }
    State {
        mode: 0,
        arg: depth,
    }
}

const MD_FENCE: u8 = 1;

/// Markdown: headings, fences, quotes, list markers, inline code, emphasis, links.
pub(crate) fn markdown(state: State, line: &str, out: &mut Out) -> State {
    let t = line.trim_start();
    let ind = line.len() - t.len();
    let fence = t.starts_with("```") || t.starts_with("~~~");
    if state.mode == MD_FENCE {
        if fence {
            out.push(0..line.len(), Tok::Meta);
            return State::NORMAL;
        }
        out.push(0..line.len(), Tok::Str);
        return state;
    }
    if fence {
        out.push(0..line.len(), Tok::Meta);
        return State {
            mode: MD_FENCE,
            arg: 0,
        };
    }
    if t.starts_with('#') && t.trim_start_matches('#').starts_with([' ', '\t'])
        || (t.starts_with('#') && t.trim_start_matches('#').is_empty())
    {
        out.push(0..line.len(), Tok::Heading);
        return State::NORMAL;
    }
    if t.starts_with('>') {
        out.push(0..line.len(), Tok::Comment);
        return State::NORMAL;
    }
    if t.starts_with("<!--") {
        out.push(0..line.len(), Tok::Comment);
        return State::NORMAL;
    }
    let mut c = Cur::new(line);
    c.i = ind;
    // List markers and thematic breaks.
    let tb = t
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    if tb.len() >= 3
        && (tb.chars().all(|ch| ch == '-')
            || tb.chars().all(|ch| ch == '*')
            || tb.chars().all(|ch| ch == '_'))
    {
        out.push(ind..line.len(), Tok::Punct);
        return State::NORMAL;
    }
    if matches!(c.peek(), b'-' | b'*' | b'+') && c.at(1) == b' ' {
        out.push(c.i..c.i + 1, Tok::Keyword);
        c.i += 2;
    } else {
        let st = c.i;
        while c.peek().is_ascii_digit() {
            c.i += 1;
        }
        if c.i > st && matches!(c.peek(), b'.' | b')') {
            c.i += 1;
            out.push(st..c.i, Tok::Keyword);
        } else {
            c.i = st;
        }
    }
    while !c.eof() {
        let st = c.i;
        let ch = c.peek();
        match ch {
            b'`' => {
                c.i += 1;
                while !c.eof() && c.peek() != b'`' {
                    c.bump();
                }
                c.i = (c.i + 1).min(c.b.len());
                out.push(st..c.i, Tok::Str);
            }
            b'*' | b'_' if c.at(1) == ch => {
                let pat = if ch == b'*' { "**" } else { "__" };
                c.i += 2;
                let rest = c.s.get(c.i..).unwrap_or("");
                if let Some(k) = rest.find(pat) {
                    c.i += k + 2;
                    out.push(st..c.i, Tok::Keyword);
                }
            }
            b'*' | b'_' if c.at(1) != b' ' => {
                c.i += 1;
                let rest = c.s.get(c.i..).unwrap_or("");
                if let Some(k) = rest.find(ch as char).filter(|&k| k > 0) {
                    c.i += k + 1;
                    out.push(st..c.i, Tok::Type);
                }
            }
            b'[' | b'!' if ch == b'[' || c.at(1) == b'[' => {
                let rest = c.s.get(c.i..).unwrap_or("");
                if let Some(k) = rest.find("](") {
                    let after = &rest[k + 2..];
                    if let Some(e) = after.find(')') {
                        out.push(st..st + k + 1, Tok::Tag);
                        out.push(st + k + 1..st + k + 2 + e + 1, Tok::Attribute);
                        c.i = st + k + 2 + e + 1;
                        continue;
                    }
                }
                c.bump();
            }
            b'<' if is_ident_start(c.at(1)) || c.at(1) == b'/' => {
                let rest = c.s.get(c.i..).unwrap_or("");
                if let Some(k) = rest.find('>') {
                    c.i += k + 1;
                    out.push(st..c.i, Tok::Tag);
                } else {
                    c.bump();
                }
            }
            _ => c.bump(),
        }
    }
    State::NORMAL
}

#[cfg(test)]
mod tests {
    use crate::Tok::*;
    use crate::test_util::expect;

    #[test]
    fn xml_html() {
        expect(
            "html",
            "<!-- c\n-->\n<a href=\"x\" disabled>t &amp; u</a>\n<img\n  src='y'\n/>",
            &[
                ("<!-- c", Comment),
                ("-->", Comment),
                ("a", Tag),
                ("href", Attribute),
                ("\"x\"", Str),
                ("disabled", Attribute),
                ("&amp;", Constant),
                ("a", Tag),
                ("img", Tag),
                ("src", Attribute),
                ("'y'", Str),
            ],
        );
        expect(
            "svg",
            "<svg viewBox=\"0 0 1 1\"><![CDATA[x]]></svg>",
            &[("svg", Tag), ("viewBox", Attribute), ("<![CDATA[x]]>", Str)],
        );
        expect(
            "xaml",
            "<Grid x:Name=\"g\" />",
            &[("Grid", Tag), ("x:Name", Attribute)],
        );
        expect(
            "vue",
            "<div v-if=\"ok\">{{ msg }}</div>",
            &[("div", Tag), ("v-if", Attribute), ("{{ msg }}", Variable)],
        );
    }

    #[test]
    fn css_family() {
        expect(
            "css",
            "/* c */\n@media screen {\n  .a, #b:hover { color: #fff; margin: 0 1.5em !important; }\n}",
            &[
                ("/* c */", Comment),
                ("@media", Keyword),
                (".a", Type),
                ("#b", Constant),
                (":hover", Function),
                ("color", Attribute),
                ("#fff", Constant),
                ("margin", Attribute),
                ("0", Number),
                ("1.5em", Number),
                ("!important", Keyword),
            ],
        );
        expect(
            "scss",
            "$w: 10px; // c\n.x { width: calc($w * 2); }",
            &[
                ("$w", Variable),
                ("10px", Number),
                ("// c", Comment),
                (".x", Type),
                ("width", Attribute),
                ("calc", Function),
                ("$w", Variable),
            ],
        );
        expect(
            "less",
            "@c: red; a { color: @c; }",
            &[("@c", Keyword), ("a", Tag), ("color", Attribute)],
        );
    }

    #[test]
    fn markdown() {
        expect(
            "md",
            "# Title\n> quote\n- **bold** and `code` [l](u)\n```rust\nlet x;\n```",
            &[
                ("# Title", Heading),
                ("> quote", Comment),
                ("-", Keyword),
                ("**bold**", Keyword),
                ("`code`", Str),
                ("[l]", Tag),
                ("(u)", Attribute),
                ("```rust", Meta),
                ("let x;", Str),
                ("```", Meta),
            ],
        );
    }
}
