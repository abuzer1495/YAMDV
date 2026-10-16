//! Data/config formats and special lexers: JSON, YAML, TOML, INI, diff, regex.

use crate::{Cur, Out, State, Tok, is_ident_start};

fn quoted(c: &mut Cur<'_>, q: u8, escapes: bool) -> bool {
    c.i += 1;
    while !c.eof() {
        let ch = c.peek();
        if escapes && ch == b'\\' {
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
    false
}

fn followed_by_colon(c: &Cur<'_>, eqs: bool) -> bool {
    let mut k = c.i;
    while matches!(c.b.get(k), Some(b' ' | b'\t')) {
        k += 1;
    }
    matches!(c.b.get(k), Some(b':')) || (eqs && matches!(c.b.get(k), Some(b'=')))
}

const J_COMMENT: u8 = 1;

/// JSON / JSONC / JSON5.
pub(crate) fn json(state: State, line: &str, out: &mut Out) -> State {
    let mut c = Cur::new(line);
    if state.mode == J_COMMENT {
        let rest = line.find("*/");
        let end = rest.map_or(line.len(), |k| k + 2);
        out.push(0..end, Tok::Comment);
        if rest.is_none() {
            return state;
        }
        c.i = end;
    }
    while !c.eof() {
        let st = c.i;
        let ch = c.peek();
        if c.starts("//") {
            out.push(st..c.b.len(), Tok::Comment);
            break;
        }
        if c.starts("/*") {
            let rest = c.s.get(c.i + 2..).unwrap_or("");
            match rest.find("*/") {
                Some(k) => {
                    c.i += k + 4;
                    out.push(st..c.i, Tok::Comment);
                    continue;
                }
                None => {
                    out.push(st..c.b.len(), Tok::Comment);
                    return State {
                        mode: J_COMMENT,
                        arg: 0,
                    };
                }
            }
        }
        match ch {
            b'"' | b'\'' => {
                quoted(&mut c, ch, true);
                let t = if followed_by_colon(&c, false) {
                    Tok::Attribute
                } else {
                    Tok::Str
                };
                out.push(st..c.i, t);
            }
            b'-' | b'0'..=b'9' if ch != b'-' || c.at(1).is_ascii_digit() => {
                c.i += usize::from(ch == b'-');
                let r = c.number();
                out.push(st..r.end, Tok::Number);
            }
            b'{' | b'}' | b'[' | b']' | b',' | b':' => {
                c.i += 1;
                out.push(st..c.i, Tok::Punct);
            }
            _ if is_ident_start(ch) => {
                let r = c.ident(&[]);
                let w = c.text(&r);
                let t = if matches!(w, "true" | "false" | "null" | "Infinity" | "NaN") {
                    Tok::Constant
                } else if followed_by_colon(&c, false) {
                    Tok::Attribute
                } else {
                    Tok::Text
                };
                out.push(r, t);
            }
            _ => c.bump(),
        }
    }
    State::NORMAL
}

fn scalar(w: &str) -> Tok {
    let lw = w.to_ascii_lowercase();
    if matches!(
        lw.as_str(),
        "true" | "false" | "yes" | "no" | "on" | "off" | "null" | "~"
    ) {
        Tok::Constant
    } else if !w.is_empty()
        && w.trim_start_matches(['-', '+'])
            .starts_with(|c: char| c.is_ascii_digit())
        && w.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-+:".contains(c))
    {
        Tok::Number
    } else {
        Tok::Text
    }
}

const Y_BLOCK: u8 = 1;

/// YAML. `State` mode 1 = inside a block scalar (`|`/`>`), `arg` = its parent indent.
pub(crate) fn yaml(state: State, line: &str, out: &mut Out) -> State {
    let t = line.trim_start();
    let ind = line.len() - t.len();
    if state.mode == Y_BLOCK && (t.is_empty() || ind > usize::from(state.arg)) {
        out.push(0..line.len(), Tok::Str);
        return state;
    }
    let mut c = Cur::new(line);
    c.i = ind;
    if t.starts_with("---") || t.starts_with("...") {
        out.push(ind..line.len(), Tok::Meta);
        return State::NORMAL;
    }
    let mut next = State::NORMAL;
    let mut after_key = false;
    while !c.eof() {
        let st = c.i;
        let ch = c.peek();
        match ch {
            b' ' | b'\t' => c.skip_ws(),
            b'#' if st == 0 || matches!(c.b.get(st - 1), Some(b' ' | b'\t')) => {
                out.push(st..c.b.len(), Tok::Comment);
                break;
            }
            b'-' if c.at(1) == b' ' || st + 1 == c.b.len() => {
                c.i += 1;
                out.push(st..c.i, Tok::Punct);
            }
            b'"' | b'\'' => {
                quoted(&mut c, ch, ch == b'"');
                let t = if followed_by_colon(&c, false) {
                    Tok::Attribute
                } else {
                    Tok::Str
                };
                out.push(st..c.i, t);
            }
            b'&' | b'*' if is_ident_start(c.at(1)) => {
                c.i += 1;
                let _ = c.ident(b"-");
                out.push(st..c.i, Tok::Variable);
            }
            b'!' => {
                while !c.eof() && c.peek() != b' ' {
                    c.bump();
                }
                out.push(st..c.i, Tok::Type);
            }
            b'|' | b'>' if after_key || st == ind => {
                c.i += 1;
                while matches!(c.peek(), b'-' | b'+' | b'0'..=b'9') {
                    c.i += 1;
                }
                out.push(st..c.i, Tok::Operator);
                next = State {
                    mode: Y_BLOCK,
                    arg: u8::try_from(ind).unwrap_or(u8::MAX),
                };
            }
            b'{' | b'}' | b'[' | b']' | b',' | b':' => {
                c.i += 1;
                out.push(st..c.i, Tok::Punct);
            }
            _ => {
                // Plain scalar or key up to `: ` / ` #` / end.
                while !c.eof() {
                    if c.peek() == b':' && matches!(c.at(1), b' ' | b'\t' | 0) {
                        break;
                    }
                    if c.peek() == b' ' && c.at(1) == b'#' {
                        break;
                    }
                    c.bump();
                }
                let mut end = c.i;
                while end > st && matches!(c.b.get(end - 1), Some(b' ')) {
                    end -= 1;
                }
                let w = c.s.get(st..end).unwrap_or("");
                if c.peek() == b':' {
                    out.push(st..end, Tok::Attribute);
                    after_key = true;
                } else {
                    out.push(st..end, scalar(w));
                }
            }
        }
    }
    next
}

const T_MSTR: u8 = 1;

/// TOML. Mode 1 = inside a multi-line string (`arg` 1 = `"""`, 2 = `'''`).
pub(crate) fn toml(state: State, line: &str, out: &mut Out) -> State {
    let mut c = Cur::new(line);
    if state.mode == T_MSTR {
        let q = if state.arg == 1 { "\"\"\"" } else { "'''" };
        match line.find(q) {
            Some(k) => {
                c.i = k + 3;
                out.push(0..c.i, Tok::Str);
            }
            None => {
                out.push(0..line.len(), Tok::Str);
                return state;
            }
        }
    }
    let t = line.trim_start();
    if c.i == 0 && t.starts_with('[') {
        let ind = line.len() - t.len();
        let end = line.find(" #").unwrap_or(line.len());
        out.push(ind..end, Tok::Type);
        out.push(end..line.len(), Tok::Comment);
        return State::NORMAL;
    }
    let mut key_side = !line[..c.i].contains('=') && c.i == 0;
    while !c.eof() {
        let st = c.i;
        let ch = c.peek();
        match ch {
            b' ' | b'\t' => c.skip_ws(),
            b'#' => {
                out.push(st..c.b.len(), Tok::Comment);
                break;
            }
            b'=' => {
                key_side = false;
                c.i += 1;
                out.push(st..c.i, Tok::Operator);
            }
            b'"' | b'\'' => {
                let triple = if ch == b'"' {
                    c.starts("\"\"\"")
                } else {
                    c.starts("'''")
                };
                if triple {
                    c.i += 3;
                    let q = if ch == b'"' { "\"\"\"" } else { "'''" };
                    let rest = c.s.get(c.i..).unwrap_or("");
                    match rest.find(q) {
                        Some(k) => c.i += k + 3,
                        None => {
                            out.push(st..c.b.len(), Tok::Str);
                            return State {
                                mode: T_MSTR,
                                arg: if ch == b'"' { 1 } else { 2 },
                            };
                        }
                    }
                    out.push(st..c.i, Tok::Str);
                } else {
                    quoted(&mut c, ch, ch == b'"');
                    out.push(st..c.i, if key_side { Tok::Attribute } else { Tok::Str });
                }
            }
            b'[' | b']' | b'{' | b'}' | b',' | b'.' => {
                c.i += 1;
                out.push(st..c.i, Tok::Punct);
            }
            _ if ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'-' || ch == b'+' => {
                let r = c.ident(b"-+:.");
                let w = c.text(&r);
                let t = if key_side {
                    Tok::Attribute
                } else if matches!(w, "true" | "false" | "inf" | "nan") {
                    Tok::Constant
                } else {
                    scalar(w)
                };
                out.push(r, t);
            }
            _ => c.bump(),
        }
    }
    State::NORMAL
}

/// INI / conf / properties.
pub(crate) fn ini(_state: State, line: &str, out: &mut Out) -> State {
    let t = line.trim_start();
    let ind = line.len() - t.len();
    if t.starts_with(';') || t.starts_with('#') || t.starts_with('!') {
        out.push(ind..line.len(), Tok::Comment);
    } else if t.starts_with('[') {
        out.push(ind..line.len(), Tok::Type);
    } else if let Some(k) = t.find(['=', ':']) {
        let k = ind + k;
        out.push(ind..k, Tok::Attribute);
        out.push(k..k + 1, Tok::Operator);
        let v = line[k + 1..].trim();
        let vs = line.len() - line[k + 1..].trim_start().len();
        let t = scalar(v);
        let t = if t == Tok::Text && (v.starts_with('"') || v.starts_with('\'')) {
            Tok::Str
        } else {
            t
        };
        out.push(vs..vs + v.len(), t);
    }
    State::NORMAL
}

/// Unified/context diffs (R-CODE-05).
pub(crate) fn diff(_state: State, line: &str, out: &mut Out) -> State {
    let t = if line.starts_with("+++")
        || line.starts_with("---")
        || line.starts_with("diff ")
        || line.starts_with("index ")
        || line.starts_with("@@")
    {
        Tok::DiffMeta
    } else if line.starts_with('+') || line.starts_with('>') {
        Tok::DiffAdd
    } else if line.starts_with('-') || line.starts_with('<') {
        Tok::DiffDel
    } else {
        Tok::Text
    };
    out.push(0..line.len(), t);
    State::NORMAL
}

/// Regular expressions.
pub(crate) fn regex(_state: State, line: &str, out: &mut Out) -> State {
    let mut c = Cur::new(line);
    while !c.eof() {
        let st = c.i;
        let ch = c.peek();
        match ch {
            b'\\' => {
                c.i += 1;
                c.bump();
                out.push(st..c.i, Tok::Constant);
            }
            b'[' => {
                c.i += 1;
                while !c.eof() && c.peek() != b']' {
                    if c.peek() == b'\\' {
                        c.i += 1;
                    }
                    c.bump();
                }
                c.i = (c.i + 1).min(c.b.len());
                out.push(st..c.i, Tok::Str);
            }
            b'(' if c.at(1) == b'?' => {
                c.i += 2;
                while matches!(
                    c.peek(),
                    b':' | b'=' | b'!' | b'<' | b'>' | b'P' | b'a'..=b'z'
                ) {
                    c.i += 1;
                    if c.b.get(c.i - 1) == Some(&b'>') || c.b.get(c.i - 1) == Some(&b':') {
                        break;
                    }
                }
                out.push(st..c.i, Tok::Meta);
            }
            b'(' | b')' => {
                c.i += 1;
                out.push(st..c.i, Tok::Punct);
            }
            b'*' | b'+' | b'?' | b'|' => {
                c.i += 1;
                out.push(st..c.i, Tok::Operator);
            }
            b'{' => {
                let rest = c.s.get(c.i..).unwrap_or("");
                let end = rest.find('}').map_or(1, |k| k + 1);
                c.i += end;
                out.push(st..c.i, Tok::Operator);
            }
            b'^' | b'$' | b'.' => {
                c.i += 1;
                out.push(st..c.i, Tok::Keyword);
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
    fn json() {
        expect(
            "jsonc",
            "{ // c\n  \"a\": [1, -2.5e3, true, null],\n  /* x\n  */ \"b\": \"s\" }",
            &[
                ("// c", Comment),
                ("\"a\"", Attribute),
                ("1", Number),
                ("-2.5e3", Number),
                ("true", Constant),
                ("null", Constant),
                ("/* x", Comment),
                ("*/", Comment),
                ("\"b\"", Attribute),
                ("\"s\"", Str),
            ],
        );
        expect("json5", "{ key: 'v' }", &[("key", Attribute), ("'v'", Str)]);
    }

    #[test]
    fn yaml() {
        expect(
            "yaml",
            "---\nname: app # c\nlist:\n  - &a 1.5\n  - *a\nflag: true\ntext: |\n  line one\n  line two\nnext: \"q\"",
            &[
                ("---", Meta),
                ("name", Attribute),
                ("# c", Comment),
                ("list", Attribute),
                ("-", Punct),
                ("&a", Variable),
                ("1.5", Number),
                ("*a", Variable),
                ("flag", Attribute),
                ("true", Constant),
                ("text", Attribute),
                ("|", Operator),
                ("line one", Str),
                ("line two", Str),
                ("next", Attribute),
                ("\"q\"", Str),
            ],
        );
    }

    #[test]
    fn toml_ini() {
        expect(
            "toml",
            "[package]\nname = \"x\" # c\nn = 1_000\nok = true\ns = \"\"\"\nmulti\n\"\"\"\nd = 2026-10-16",
            &[
                ("[package]", Type),
                ("name", Attribute),
                ("\"x\"", Str),
                ("# c", Comment),
                ("n", Attribute),
                ("1_000", Number),
                ("true", Constant),
                ("\"\"\"", Str),
                ("multi", Str),
                ("\"\"\"", Str),
                ("2026-10-16", Number),
            ],
        );
        expect(
            "ini",
            "; c\n[core]\nname = value\nport: 8080",
            &[
                ("; c", Comment),
                ("[core]", Type),
                ("name", Attribute),
                ("port", Attribute),
                ("8080", Number),
            ],
        );
        expect(
            "properties",
            "# c\na.b=c",
            &[("# c", Comment), ("a.b", Attribute)],
        );
    }

    #[test]
    fn diff_regex() {
        expect(
            "diff",
            "--- a/x\n+++ b/x\n@@ -1 +1 @@\n-old\n+new\n same",
            &[
                ("--- a/x", DiffMeta),
                ("+++ b/x", DiffMeta),
                ("@@ -1 +1 @@", DiffMeta),
                ("-old", DiffDel),
                ("+new", DiffAdd),
            ],
        );
        expect(
            "regex",
            "^(?:a|b)+\\d{2,3}[a-z]$",
            &[
                ("^", Keyword),
                ("(?:", Meta),
                ("|", Operator),
                ("+", Operator),
                ("\\d", Constant),
                ("{2,3}", Operator),
                ("[a-z]", Str),
                ("$", Keyword),
            ],
        );
    }
}
