//! Generic lexer for C-like and script languages, configured by a [`Spec`].

use crate::jsx::{self, Jsx};
use crate::langs::Spec;
use crate::{Cur, Out, State, Tok, is_ident_start};

const M_BLOCK: u8 = 1;
const M_STR: u8 = 2;

// String delimiter codes (State::arg in M_STR).
const D_DQ: u8 = 1;
const D_SQ: u8 = 2;
const D_BT: u8 = 3;
const D_TDQ: u8 = 4;
const D_TSQ: u8 = 5;
const D_RAW: u8 = 32; // + number of hashes

fn eq(spec: &Spec, a: &str, b: &str) -> bool {
    if spec.case_insensitive {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

fn in_set(spec: &Spec, set: &[&str], w: &str) -> bool {
    set.iter().any(|k| eq(spec, k, w))
}

/// Scans a string body until its closing delimiter; returns true when it closed.
fn scan_str(c: &mut Cur<'_>, d: u8) -> bool {
    match d {
        D_TDQ | D_TSQ => {
            let q = if d == D_TDQ { "\"\"\"" } else { "'''" };
            while !c.eof() {
                if c.peek() == b'\\' {
                    c.i += 1;
                    c.bump();
                } else if c.starts(q) {
                    c.i += 3;
                    return true;
                } else {
                    c.bump();
                }
            }
            false
        }
        d if d >= D_RAW => {
            let n = usize::from(d - D_RAW);
            while !c.eof() {
                if c.peek() == b'"' && (1..=n).all(|k| c.at(k) == b'#') {
                    c.i += 1 + n;
                    return true;
                }
                c.bump();
            }
            false
        }
        _ => {
            let q = match d {
                D_DQ => b'"',
                D_SQ => b'\'',
                _ => b'`',
            };
            while !c.eof() {
                let ch = c.peek();
                if ch == b'\\' {
                    c.i += 1;
                    if !c.eof() {
                        c.bump();
                    }
                } else if ch == q {
                    c.i += 1;
                    return true;
                } else {
                    c.bump();
                }
            }
            // Only triple, backtick and raw strings (and shell/python quotes) span lines.
            false
        }
    }
}

fn multiline(spec: &Spec, d: u8) -> bool {
    matches!(d, D_TDQ | D_TSQ | D_BT) || d >= D_RAW || (spec.squote && spec.sigils.contains(&b'$'))
}

/// Scans a block comment from inside it; returns the new state (NORMAL when closed).
fn scan_block(spec: &Spec, c: &mut Cur<'_>, idx: usize, mut depth: u8) -> State {
    let Some((open, close)) = spec.block_comments.get(idx) else {
        c.i = c.b.len();
        return State::NORMAL;
    };
    while !c.eof() {
        if c.starts(close) {
            c.i += close.len();
            if depth <= 1 {
                return State::NORMAL;
            }
            depth -= 1;
        } else if spec.nested_comments && c.starts(open) {
            c.i += open.len();
            depth = depth.saturating_add(1).min(31);
        } else {
            c.bump();
        }
    }
    State {
        mode: M_BLOCK,
        arg: u8::try_from(idx).unwrap_or(0) | (depth << 3),
        ext: 0,
    }
}

/// Lexes one line.
pub(crate) fn lex(spec: &Spec, state: State, line: &str, out: &mut Out) -> State {
    if !spec.jsx {
        return lex_line(spec, state, line, out, &mut Jsx::default());
    }
    let mut jx = Jsx::unpack(state.ext);
    let mut st = lex_line(spec, state, line, out, &mut jx);
    st.ext = jx.pack();
    st
}

#[allow(clippy::too_many_lines)]
fn lex_line(spec: &Spec, state: State, line: &str, out: &mut Out, jx: &mut Jsx) -> State {
    let mut c = Cur::new(line);
    // Resume a multi-line construct.
    match state.mode {
        M_BLOCK => {
            let st = scan_block(spec, &mut c, usize::from(state.arg & 7), state.arg >> 3);
            out.push(0..c.i, Tok::Comment);
            if st != State::NORMAL {
                return st;
            }
        }
        M_STR => {
            let closed = scan_str(&mut c, state.arg);
            out.push(0..c.i, Tok::Str);
            if !closed {
                return state;
            }
        }
        _ => {}
    }
    let mut line_start = c.b[..c.i].iter().all(|b| b.is_ascii_whitespace());
    // Regex literals are allowed at expression start / after operators.
    let mut regex_ok = true;
    // Previous keyword introduces a definition name (fn/def/func/function/class…).
    let mut def_next = false;
    while !c.eof() {
        let st = c.i;
        if spec.jsx {
            match jsx_step(&mut c, out, jx, regex_ok) {
                Step::Eol => return State::NORMAL,
                Step::Done => {
                    regex_ok = false;
                    line_start = false;
                    continue;
                }
                Step::Expr => {
                    regex_ok = true;
                    continue;
                }
                Step::Lex => {}
            }
            if c.eof() {
                break;
            }
        }
        let ch = c.peek();
        if ch == b' ' || ch == b'\t' {
            c.skip_ws();
            continue;
        }
        let first = line_start;
        line_start = false;
        // Preprocessor lines.
        if spec.preproc && first && ch == b'#' {
            // A trailing line comment stays a comment.
            let rest = c.s.get(c.i..).unwrap_or("");
            let end = rest.find("//").map_or(c.b.len(), |k| c.i + k);
            out.push(st..end, Tok::Meta);
            out.push(end..c.b.len(), Tok::Comment);
            return State::NORMAL;
        }
        // Block comments.
        if let Some(idx) = spec.block_comments.iter().position(|(o, _)| c.starts(o)) {
            let open = spec.block_comments[idx].0;
            c.i += open.len();
            let ns = scan_block(spec, &mut c, idx, 1);
            out.push(st..c.i, Tok::Comment);
            if ns != State::NORMAL {
                return ns;
            }
            continue;
        }
        // Line comments.
        if let Some(lc) = spec.line_comments.iter().find(|p| {
            if spec.case_insensitive {
                c.starts_ci(p)
            } else {
                c.starts(p)
            }
        }) {
            // `#` inside shell words (e.g. `a#b`) is not a comment; at a token start it is.
            let _ = lc;
            out.push(st..c.b.len(), Tok::Comment);
            return State::NORMAL;
        }
        // Rust attributes `#[…]` / `#![…]`.
        if spec.raw_rust && ch == b'#' && (c.at(1) == b'[' || (c.at(1) == b'!' && c.at(2) == b'['))
        {
            let rest = c.s.get(c.i..).unwrap_or("");
            let end = rest.find(']').map_or(c.b.len(), |k| c.i + k + 1);
            c.i = end;
            out.push(st..end, Tok::Meta);
            continue;
        }
        // Rust raw strings r"…", r#"…"#, br"…".
        if spec.raw_rust && (ch == b'r' || (ch == b'b' && c.at(1) == b'r')) {
            let k = if ch == b'b' { 2 } else { 1 };
            let mut n = 0;
            while c.at(k + n) == b'#' {
                n += 1;
            }
            if c.at(k + n) == b'"' && n < 200 {
                c.i += k + n + 1;
                let d = D_RAW + u8::try_from(n).unwrap_or(0);
                let closed = scan_str(&mut c, d);
                out.push(st..c.i, Tok::Str);
                if !closed {
                    return State {
                        mode: M_STR,
                        arg: d,
                        ext: 0,
                    };
                }
                regex_ok = false;
                continue;
            }
        }
        // Strings.
        let delim = if spec.triple && c.starts("\"\"\"") {
            Some(D_TDQ)
        } else if spec.triple && spec.squote && c.starts("'''") {
            Some(D_TSQ)
        } else if ch == b'"' {
            Some(D_DQ)
        } else if ch == b'\'' && spec.squote {
            Some(D_SQ)
        } else if ch == b'`' && spec.backtick {
            Some(D_BT)
        } else {
            None
        };
        if let Some(d) = delim {
            c.i += if matches!(d, D_TDQ | D_TSQ) { 3 } else { 1 };
            let closed = scan_str(&mut c, d);
            out.push(st..c.i, Tok::Str);
            if !closed && multiline(spec, d) {
                return State {
                    mode: M_STR,
                    arg: d,
                    ext: 0,
                };
            }
            regex_ok = false;
            continue;
        }
        // Char literals / lifetimes when `'` is not a string quote.
        if ch == b'\'' && !spec.squote {
            let rest = c.b.get(c.i + 1..).unwrap_or(&[]);
            let close = if rest.first() == Some(&b'\\') {
                rest.iter()
                    .skip(1)
                    .take(10)
                    .position(|&b| b == b'\'')
                    .map(|p| p + 2)
            } else {
                // One UTF-8 character then a quote.
                let s = c.s.get(c.i + 1..).unwrap_or("");
                s.chars().next().and_then(|ch| {
                    (s.as_bytes().get(ch.len_utf8()) == Some(&b'\'')).then_some(ch.len_utf8() + 1)
                })
            };
            if let Some(n) = close {
                c.i += 1 + n;
                out.push(st..c.i, Tok::Str);
            } else {
                c.i += 1;
                let r = c.ident(&[]);
                out.push(st..r.end, Tok::Variable); // lifetime / label
            }
            regex_ok = false;
            continue;
        }
        // Variables with sigils.
        if spec.sigils.contains(&ch)
            && !(ch == b'%' && spec.sigils.len() > 1 && !is_ident_start(c.at(1)) && c.at(1) != b'{')
        {
            if ch == b'%' && spec.case_insensitive {
                // Batch: %var%, %1, %%i, %~dp0
                c.i += 1;
                if c.peek() == b'%' {
                    c.i += 1;
                    let _ = c.ident(&[]);
                } else if c.peek() == b'~' || c.peek().is_ascii_digit() {
                    while c.peek().is_ascii_alphanumeric() || c.peek() == b'~' {
                        c.i += 1;
                    }
                } else {
                    let _ = c.ident(&[]);
                    if c.peek() == b'%' {
                        c.i += 1;
                    }
                }
                out.push(st..c.i, Tok::Variable);
                continue;
            }
            if c.at(1) == b'{' {
                let rest = c.s.get(c.i..).unwrap_or("");
                let end = rest.find('}').map_or(c.b.len(), |k| c.i + k + 1);
                c.i = end;
                out.push(st..end, Tok::Variable);
                continue;
            }
            if is_ident_start(c.at(1))
                || (ch == b'$'
                    && matches!(
                        c.at(1),
                        b'@' | b'?' | b'#' | b'*' | b'!' | b'0'..=b'9' | b'$' | b'<' | b'^'
                    ))
            {
                c.i += 1;
                if is_ident_start(c.peek()) {
                    let _ = c.ident(if ch == b'$' && spec.ident_extra.contains(&b'-') {
                        b":"
                    } else {
                        &[]
                    });
                } else {
                    c.i += 1;
                }
                let w = c.s.get(st..c.i).unwrap_or("");
                let t = if in_set(spec, spec.constants, w) {
                    Tok::Constant
                } else {
                    Tok::Variable
                };
                out.push(st..c.i, t);
                regex_ok = false;
                continue;
            }
        }
        // Annotations / decorators.
        if spec.annotations && ch == b'@' && is_ident_start(c.at(1)) {
            c.i += 1;
            let _ = c.ident(b".");
            out.push(st..c.i, Tok::Meta);
            continue;
        }
        // Numbers.
        if ch.is_ascii_digit() || (ch == b'.' && c.at(1).is_ascii_digit()) {
            let r = c.number();
            out.push(r, Tok::Number);
            regex_ok = false;
            continue;
        }
        // Identifiers.
        if is_ident_start(ch)
            || (spec.ident_extra.contains(&ch) && ch == b'$')
            || (ch == b'.' && spec.keywords.contains(&".PHONY") && c.starts(".PHONY"))
        {
            let r = c.ident(spec.ident_extra);
            let w = c.text(&r);
            // String prefixes: f"…", r'…', b"…", u8"…", L"…", @"…".
            let q = c.peek();
            if w.len() <= 2
                && (q == b'"' || (q == b'\'' && spec.squote))
                && w.chars().all(|ch| "rbfuRBFUL8".contains(ch))
            {
                c.i += 1;
                let d = if q == b'"' { D_DQ } else { D_SQ };
                let d = if spec.triple && c.starts(if d == D_DQ { "\"\"" } else { "''" }) {
                    c.i += 2;
                    if d == D_DQ { D_TDQ } else { D_TSQ }
                } else {
                    d
                };
                let closed = scan_str(&mut c, d);
                out.push(r.start..c.i, Tok::Str);
                if !closed && multiline(spec, d) {
                    return State {
                        mode: M_STR,
                        arg: d,
                        ext: 0,
                    };
                }
                continue;
            }
            // Makefile targets `name:`.
            if spec.targets
                && first
                && c.peek() == b':'
                && c.at(1) != b'='
                && !in_set(spec, spec.keywords, w)
            {
                out.push(r, Tok::Function);
                continue;
            }
            let kw_allowed = !spec.line_start_keywords || first || w.eq_ignore_ascii_case("as");
            let t = if kw_allowed && in_set(spec, spec.keywords, w) {
                def_next = matches!(
                    w,
                    "fn" | "def"
                        | "defp"
                        | "func"
                        | "function"
                        | "fun"
                        | "sub"
                        | "class"
                        | "struct"
                        | "enum"
                        | "trait"
                        | "interface"
                        | "type"
                        | "module"
                        | "defmodule"
                        | "message"
                        | "service"
                        | "rpc"
                        | "record"
                        | "object"
                );
                regex_ok = true;
                out.push(r, Tok::Keyword);
                continue;
            } else if in_set(spec, spec.types, w) {
                Tok::Type
            } else if in_set(spec, spec.constants, w) {
                Tok::Constant
            } else if def_next {
                if c.peek() == b'('
                    || !w.starts_with(|ch: char| ch.is_ascii_uppercase())
                    || !spec.cap_types
                {
                    Tok::Function
                } else {
                    Tok::Type
                }
            } else {
                let mut k = c.i;
                while matches!(c.b.get(k), Some(b' ')) {
                    k += 1;
                }
                if c.b.get(k) == Some(&b'(')
                    || (spec.raw_rust
                        && c.b.get(k) == Some(&b'!')
                        && matches!(c.b.get(k + 1), Some(b'(' | b'[' | b'{')))
                {
                    Tok::Function
                } else if spec.cap_types && w.starts_with(|ch: char| ch.is_ascii_uppercase()) {
                    if w.len() > 1
                        && w.bytes()
                            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
                    {
                        Tok::Constant
                    } else {
                        Tok::Type
                    }
                } else {
                    Tok::Text
                }
            };
            def_next = false;
            regex_ok = false;
            out.push(r, t);
            continue;
        }
        // Regex literals.
        if spec.regex
            && ch == b'/'
            && regex_ok
            && c.at(1) != b'/'
            && c.at(1) != b'*'
            && c.at(1) != b' '
        {
            let mut k = c.i + 1;
            let mut class = false;
            let mut ok = false;
            while let Some(&b) = c.b.get(k) {
                match b {
                    b'\\' => k += 1,
                    b'[' => class = true,
                    b']' => class = false,
                    b'/' if !class => {
                        ok = true;
                        break;
                    }
                    _ => {}
                }
                k += 1;
            }
            if ok {
                c.i = k + 1;
                while c.peek().is_ascii_alphabetic() {
                    c.i += 1;
                }
                out.push(st..c.i, Tok::Regex);
                regex_ok = false;
                continue;
            }
        }
        // Operators and punctuation.
        if b"+-*/%=<>!&|^~?:".contains(&ch) {
            c.i += 1;
            out.push(st..c.i, Tok::Operator);
            regex_ok = true;
            continue;
        }
        if b"()[]{},.;".contains(&ch) {
            if spec.jsx && matches!(ch, b'{' | b'}') && jx.in_expr() {
                jx.brace(ch == b'{');
            }
            c.i += 1;
            out.push(st..c.i, Tok::Punct);
            regex_ok = !matches!(ch, b')' | b']' | b'}');
            continue;
        }
        c.bump();
        regex_ok = false;
    }
    State::NORMAL
}

enum Step {
    /// Not JSX here; lex as JS/TS.
    Lex,
    /// A tag or text run was consumed.
    Done,
    /// A `{` child expression was opened.
    Expr,
    /// The line ended inside a tag head.
    Eol,
}

/// One JSX step at the cursor: a pending/new tag head, a text-children run or `{`.
fn jsx_step(c: &mut Cur<'_>, out: &mut Out, jx: &mut Jsx, regex_ok: bool) -> Step {
    if jx.pending == 0 {
        let children = jx.in_children();
        if children && !jsx::tag_start(c, true) {
            let st = c.i;
            if c.peek() == b'{' {
                c.i += 1;
                out.push(st..c.i, Tok::Punct);
                jx.brace(true);
                return Step::Expr;
            }
            if c.peek() == b'<' {
                c.i += 1; // a lone `<` in text
            }
            while !c.eof() && !matches!(c.peek(), b'<' | b'{') {
                c.bump();
            }
            out.push(st..c.i, Tok::Text);
            return Step::Done;
        }
        if !((children || regex_ok) && jsx::tag_start(c, children)) {
            return Step::Lex;
        }
        jx.pending = jsx::tag_open(c, out);
    }
    let end = jsx::tag_body(c, out);
    if jx.finish(&end) {
        Step::Done
    } else {
        Step::Eol
    }
}

#[cfg(test)]
mod tests {
    use crate::Tok::*;
    use crate::test_util::expect;

    #[test]
    fn rust() {
        expect(
            "rust",
            "/// doc\n#[derive(Debug)]\npub fn main() -> Result<(), String> {\n    let s = r#\"raw \"q\"\"#; // c\n    let n = 0x1F_u8 + 1.5e3;\n    println!(\"{}\", 'x');\n}\n/* a /* nested */ still */ fn f<'a>() {}",
            &[
                ("/// doc", Comment),
                ("pub", Keyword),
                ("fn", Keyword),
                ("main", Function),
                ("Result", Type),
                ("String", Type),
                ("let", Keyword),
                ("r#\"raw \"q\"\"#", Str),
                ("// c", Comment),
                ("0x1F_u8", Number),
                ("1.5e3", Number),
                ("println", Function),
                ("\"{}\"", Str),
                ("'x'", Str),
                ("/* a /* nested */ still */", Comment),
                ("'a", Variable),
            ],
        );
    }

    #[test]
    fn c_and_cpp() {
        expect(
            "c",
            "#include <stdio.h>\nint main(void) { return 0; } /* x */",
            &[
                ("#include <stdio.h>", Meta),
                ("int", Type),
                ("main", Function),
                ("void", Type),
                ("return", Keyword),
                ("0", Number),
                ("/* x */", Comment),
            ],
        );
        expect(
            "cpp",
            "template <typename T> class Box { std::vector<T> v; nullptr; };",
            &[
                ("template", Keyword),
                ("typename", Keyword),
                ("T", Type),
                ("class", Keyword),
                ("Box", Type),
                ("std", Type),
                ("vector", Type),
                ("nullptr", Constant),
            ],
        );
    }

    #[test]
    fn csharp_java_kotlin() {
        expect(
            "csharp",
            "public async Task<int> Run() { var s = $\"x\"; return null; }",
            &[
                ("public", Keyword),
                ("async", Keyword),
                ("Task", Type),
                ("int", Type),
                ("Run", Function),
                ("var", Keyword),
                ("null", Constant),
            ],
        );
        expect(
            "java",
            "@Override\npublic static void main(String[] args) {}",
            &[
                ("@Override", Meta),
                ("public", Keyword),
                ("static", Keyword),
                ("void", Type),
                ("main", Function),
                ("String", Type),
            ],
        );
        expect(
            "kotlin",
            "data class P(val x: Int)\nfun go() = \"\"\"multi\nline\"\"\"",
            &[
                ("data", Keyword),
                ("class", Keyword),
                ("P", Function),
                ("val", Keyword),
                ("Int", Type),
                ("fun", Keyword),
                ("go", Function),
                ("\"\"\"multi", Str),
                ("line\"\"\"", Str),
            ],
        );
    }

    #[test]
    fn go_python() {
        expect(
            "go",
            "package main\nfunc main() { s := `raw`; fmt.Println(len(s), nil) }",
            &[
                ("package", Keyword),
                ("func", Keyword),
                ("main", Function),
                ("`raw`", Str),
                ("Println", Function),
                ("len", Constant),
                ("nil", Constant),
            ],
        );
        expect(
            "python",
            "@dataclass\ndef f(x: int) -> None:\n    \"\"\"Doc\n    more\"\"\"\n    return f'{x}' # c",
            &[
                ("@dataclass", Meta),
                ("def", Keyword),
                ("f", Function),
                ("int", Type),
                ("None", Constant),
                ("\"\"\"Doc", Str),
                ("more\"\"\"", Str),
                ("return", Keyword),
                ("f'{x}'", Str),
                ("# c", Comment),
            ],
        );
    }

    #[test]
    fn js_ts() {
        expect(
            "js",
            "const re = /a[/]b/gi; let t = `x${y}`; console.log(re)",
            &[
                ("const", Keyword),
                ("/a[/]b/gi", Regex),
                ("let", Keyword),
                ("`x${y}`", Str),
                ("console", Constant),
                ("log", Function),
            ],
        );
        expect(
            "ts",
            "interface A { x: number }\nexport type B = A | undefined;",
            &[
                ("interface", Keyword),
                ("A", Type),
                ("number", Type),
                ("export", Keyword),
                ("type", Keyword),
                ("B", Type),
                ("undefined", Constant),
            ],
        );
        expect(
            "jsx",
            "function App() {\n  return <div className=\"a\" onClick={() => go(1)}>\n    Hi if {n / 2} <b>x</b>\n    {items.map(i => <Item key={i} />)}\n  </div>;\n}\nconst r = 1 / 2;",
            &[
                ("function", Keyword),
                ("App", Function),
                ("return", Keyword),
                ("div", Tag),
                ("className", Attribute),
                ("\"a\"", Str),
                ("onClick", Attribute),
                ("2", Number),
                ("b", Tag),
                ("b", Tag),
                ("map", Function),
                ("Item", Tag),
                ("key", Attribute),
                ("div", Tag),
                ("const", Keyword),
                ("1", Number),
                ("/", Operator),
            ],
        );
        // Text children are not lexed as code; TSX generics are not elements.
        assert!(
            !crate::test_util::toks("jsx", "<p>\n if x\n</p>")
                .iter()
                .any(|(_, t)| *t == Keyword)
        );
        expect(
            "tsx",
            "const f = <T,>(x: T) => x;\nlet e: JSX.Element = <>\n  <Box<string>\n    title='t'\n  />\n</>;",
            &[
                ("const", Keyword),
                ("T", Type),
                ("let", Keyword),
                ("JSX", Constant),
                ("Box", Tag),
                ("title", Attribute),
                ("'t'", Str),
            ],
        );
    }

    #[test]
    fn shells() {
        expect(
            "bash",
            "#!/bin/bash\nif [ -f \"$HOME/x\" ]; then echo ${A:-b} $1; fi",
            &[
                ("#!/bin/bash", Comment),
                ("if", Keyword),
                ("\"$HOME/x\"", Str),
                ("then", Keyword),
                ("echo", Constant),
                ("${A:-b}", Variable),
                ("$1", Variable),
                ("fi", Keyword),
            ],
        );
        expect(
            "console",
            "$ ls -la $HOME \\\n  | grep x # c\ntotal 8\nuser@host:~/src$ echo \"hi\"\nPS C:\\> Get-Item",
            &[
                ("$", Meta),
                ("$HOME", Variable),
                ("|", Operator),
                ("# c", Comment),
                ("user@host:~/src$", Meta),
                ("echo", Constant),
                ("\"hi\"", Str),
                ("PS C:\\>", Meta),
            ],
        );
        assert!(crate::test_util::toks("console", "total 8 if then").is_empty());
        expect(
            "powershell",
            "<# block #>\nfunction Get-X { param($Path) Write-Host $true }",
            &[
                ("<# block #>", Comment),
                ("function", Keyword),
                ("param", Keyword),
                ("$Path", Variable),
                ("Write-Host", Constant),
                ("$true", Constant),
            ],
        );
        expect(
            "bat",
            "@echo off\nREM comment\nset X=%~dp0\nif exist %X% goto :eof",
            &[
                ("echo", Keyword),
                ("off", Constant),
                ("REM comment", Comment),
                ("set", Keyword),
                ("%~dp0", Variable),
                ("if", Keyword),
                ("exist", Keyword),
                ("%X%", Variable),
                ("goto", Keyword),
            ],
        );
    }

    #[test]
    fn sql_php_ruby() {
        expect(
            "sql",
            "SELECT id, COUNT(*) FROM users WHERE name = 'x' -- c",
            &[
                ("SELECT", Keyword),
                ("COUNT", Constant),
                ("FROM", Keyword),
                ("WHERE", Keyword),
                ("'x'", Str),
                ("-- c", Comment),
            ],
        );
        expect(
            "php",
            "<?php function f($a) { return $a ?? null; } # c",
            &[
                ("function", Keyword),
                ("f", Function),
                ("$a", Variable),
                ("return", Keyword),
                ("null", Constant),
                ("# c", Comment),
            ],
        );
        expect(
            "ruby",
            "def ok?(x)\n  @n = x =~ /ab+/ ? true : nil\nend",
            &[
                ("def", Keyword),
                ("ok?", Function),
                ("@n", Variable),
                ("/ab+/", Regex),
                ("true", Constant),
                ("nil", Constant),
                ("end", Keyword),
            ],
        );
    }

    #[test]
    fn swift_lua_dart_r() {
        expect(
            "swift",
            "@MainActor\nstruct V { let x: Int = 1 }\nfunc f() async throws {}",
            &[
                ("@MainActor", Meta),
                ("struct", Keyword),
                ("V", Type),
                ("let", Keyword),
                ("Int", Type),
                ("func", Keyword),
                ("f", Function),
                ("async", Keyword),
                ("throws", Keyword),
            ],
        );
        expect(
            "lua",
            "--[[ block\n]] local function f() return nil end -- c",
            &[
                ("--[[ block", Comment),
                ("]]", Comment),
                ("local", Keyword),
                ("function", Keyword),
                ("f", Function),
                ("nil", Constant),
                ("-- c", Comment),
            ],
        );
        expect(
            "dart",
            "Future<void> main() async { final s = '''x'''; }",
            &[
                ("Future", Type),
                ("main", Function),
                ("async", Keyword),
                ("final", Keyword),
                ("'''x'''", Str),
            ],
        );
        expect(
            "r",
            "f <- function(x) { if (is.na(x)) TRUE else NULL } # c",
            &[
                ("function", Keyword),
                ("if", Keyword),
                ("is.na", Function),
                ("TRUE", Constant),
                ("NULL", Constant),
                ("# c", Comment),
            ],
        );
    }

    #[test]
    fn scala_perl_haskell_elixir() {
        expect(
            "scala",
            "case class P(x: Int)\nobject M { def f = None }",
            &[
                ("case", Keyword),
                ("class", Keyword),
                ("P", Function),
                ("Int", Type),
                ("object", Keyword),
                ("def", Keyword),
                ("f", Function),
                ("None", Constant),
            ],
        );
        expect(
            "perl",
            "my $x = shift; my @a = (1); if ($x =~ /^\\d+$/) { print \"ok\" }",
            &[
                ("my", Keyword),
                ("$x", Variable),
                ("shift", Constant),
                ("@a", Variable),
                ("/^\\d+$/", Regex),
                ("print", Constant),
                ("\"ok\"", Str),
            ],
        );
        expect(
            "haskell",
            "{- c {- n -} -}\nmodule Main where\nmain = putStrLn \"hi\" -- c",
            &[
                ("{- c {- n -} -}", Comment),
                ("module", Keyword),
                ("Main", Type),
                ("where", Keyword),
                ("putStrLn", Constant),
                ("\"hi\"", Str),
                ("-- c", Comment),
            ],
        );
        expect(
            "elixir",
            "defmodule M do\n  @doc \"x\"\n  def ok?(a), do: nil\nend",
            &[
                ("defmodule", Keyword),
                ("M", Type),
                ("do", Keyword),
                ("@doc", Variable),
                ("def", Keyword),
                ("ok?", Function),
                ("nil", Constant),
                ("end", Keyword),
            ],
        );
    }

    #[test]
    fn config_languages() {
        expect(
            "nginx",
            "server {\n  listen 80; # c\n  proxy_pass http://$host;\n}",
            &[
                ("server", Keyword),
                ("listen", Keyword),
                ("80", Number),
                ("# c", Comment),
                ("proxy_pass", Keyword),
                ("$host", Variable),
            ],
        );
        expect(
            "graphql",
            "query Q($id: ID!) { user(id: $id) { name } }",
            &[
                ("query", Keyword),
                ("Q", Function),
                ("$id", Variable),
                ("ID", Type),
                ("user", Function),
            ],
        );
        expect(
            "proto",
            "syntax = \"proto3\";\nmessage M { repeated int32 x = 1; }",
            &[
                ("syntax", Keyword),
                ("\"proto3\"", Str),
                ("message", Keyword),
                ("M", Type),
                ("repeated", Keyword),
                ("int32", Type),
                ("1", Number),
            ],
        );
        expect(
            "dockerfile",
            "FROM rust:1.99 AS build\nRUN cargo build # c\nENV A=$B",
            &[
                ("FROM", Keyword),
                ("AS", Keyword),
                ("RUN", Keyword),
                ("# c", Comment),
                ("ENV", Keyword),
                ("$B", Variable),
            ],
        );
        expect(
            "makefile",
            ".PHONY: all\nall: build\n\tcargo build $(FLAGS) # c\nifeq ($(A),1)",
            &[(".PHONY", Keyword), ("all", Function), ("ifeq", Keyword)],
        );
    }
}
