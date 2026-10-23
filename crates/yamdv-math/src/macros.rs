//! Per-document macros (R-MATH-07): `\newcommand`, `\renewcommand` and `\def` with `#1`…`#9`.
//! Expansion is bounded (S-PARSE-05): at most [`MAX_STEPS`] expansions and [`MAX_TOKENS`]
//! produced tokens per formula; over a limit the formula is an error (R-MATH-03 error color).
//! A [`Macros`] set is threaded through the formulas of a document in document order, so a
//! definition applies to the rest of its formula and to every later formula.

use std::borrow::Cow;
use std::collections::HashMap;

use crate::parse::MathError;

/// Maximum macro expansions per formula.
pub const MAX_STEPS: usize = 10_000;
/// Maximum tokens produced by expansion per formula.
pub const MAX_TOKENS: usize = 100_000;
/// Maximum macros per document.
pub const MAX_MACROS: usize = 1_000;

/// One definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Macro {
    /// Number of parameters (0–9).
    pub params: u8,
    /// Default for an optional first parameter (`\newcommand{\x}[2][d]{…}`).
    pub default: Option<String>,
    /// Replacement text with `#1`…`#9`.
    pub body: String,
}

/// The macros defined so far in a document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Macros {
    map: HashMap<String, Macro>,
}

impl Macros {
    /// Number of definitions.
    pub fn len(&self) -> usize {
        self.map.len()
    }
    /// No definitions.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
    /// The definition of `\name`.
    pub fn get(&self, name: &str) -> Option<&Macro> {
        self.map.get(name)
    }
}

/// Does `src` define macros (cheap pre-scan for the document pass)?
pub fn defines(src: &str) -> bool {
    src.contains("\\newcommand") || src.contains("\\renewcommand") || src.contains("\\def")
}

fn fail<T>(msg: impl Into<String>) -> Result<T, MathError> {
    Err(MathError {
        msg: msg.into(),
        pos: 0,
    })
}

struct X<'a> {
    /// Remaining input, reversed (the next character is at the end).
    rest: Vec<char>,
    out: String,
    after_word: bool,
    steps: usize,
    tokens: usize,
    env: &'a mut Macros,
}

/// Expands the macros of `env` in `src` and records the definitions `src` makes in `env`
/// (removed from the result). Returns `src` unchanged when there is nothing to do.
pub fn expand<'s>(src: &'s str, env: &mut Macros) -> Result<Cow<'s, str>, MathError> {
    if !defines(src) && (env.is_empty() || !src.contains('\\')) {
        return Ok(Cow::Borrowed(src));
    }
    let mut x = X {
        rest: src.chars().rev().collect(),
        out: String::with_capacity(src.len()),
        after_word: false,
        steps: 0,
        tokens: 0,
        env,
    };
    x.run()?;
    Ok(Cow::Owned(x.out))
}

impl X<'_> {
    fn emit(&mut self, c: char) {
        if self.after_word && c.is_ascii_alphabetic() {
            self.out.push(' ');
        }
        self.after_word = false;
        self.out.push(c);
    }

    fn skip_ws(&mut self) {
        while self.rest.last().is_some_and(|c| c.is_whitespace()) {
            self.rest.pop();
        }
    }

    /// A control sequence name after `\` (letters, or one other character).
    fn cs_name(&mut self) -> String {
        let mut name = String::new();
        while let Some(&c) = self.rest.last() {
            if c.is_ascii_alphabetic() {
                name.push(c);
                self.rest.pop();
            } else {
                break;
            }
        }
        if name.is_empty()
            && let Some(c) = self.rest.pop()
        {
            name.push(c);
        }
        name
    }

    /// `{…}` content (balanced, `\{` `\}` escaped) or one token as text.
    fn arg(&mut self, what: &str) -> Result<String, MathError> {
        self.skip_ws();
        match self.rest.pop() {
            None => fail(format!("missing argument for {what}")),
            Some('{') => {
                let mut depth = 1usize;
                let mut s = String::new();
                loop {
                    let Some(c) = self.rest.pop() else {
                        return fail(format!("missing }} in argument of {what}"));
                    };
                    match c {
                        '\\' => {
                            s.push(c);
                            if let Some(n) = self.rest.pop() {
                                s.push(n);
                            }
                            continue;
                        }
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                return Ok(s);
                            }
                        }
                        _ => {}
                    }
                    s.push(c);
                }
            }
            Some('\\') => Ok(format!("\\{}", self.cs_name())),
            Some(c) => Ok(c.to_string()),
        }
    }

    /// `[…]` content when the next character is `[` (braces kept as written).
    fn opt(&mut self) -> Result<Option<String>, MathError> {
        self.skip_ws();
        if self.rest.last() != Some(&'[') {
            return Ok(None);
        }
        self.rest.pop();
        let mut s = String::new();
        let mut depth = 0usize;
        loop {
            let Some(c) = self.rest.pop() else {
                return fail("missing ]");
            };
            match c {
                '{' => depth += 1,
                '}' => depth = depth.saturating_sub(1),
                ']' if depth == 0 => return Ok(Some(s)),
                _ => {}
            }
            s.push(c);
        }
    }

    /// The macro name of a definition: `\name` or `{\name}`.
    fn def_name(&mut self, what: &str) -> Result<String, MathError> {
        let a = self.arg(what)?;
        let n = a.trim();
        match n.strip_prefix('\\') {
            Some(name) if !name.is_empty() => Ok(name.to_string()),
            _ => fail(format!("{what} needs a control sequence name")),
        }
    }

    fn define(&mut self, name: String, m: Macro) -> Result<(), MathError> {
        if !self.env.map.contains_key(&name) && self.env.map.len() >= MAX_MACROS {
            return fail(format!("more than {MAX_MACROS} macros"));
        }
        self.env.map.insert(name, m);
        Ok(())
    }

    fn newcommand(&mut self, renew: bool) -> Result<(), MathError> {
        let what = if renew {
            "\\renewcommand"
        } else {
            "\\newcommand"
        };
        let name = self.def_name(what)?;
        let exists = self.env.map.contains_key(&name) || crate::parse::known(&name);
        let builtin = crate::parse::known(&name) && !self.env.map.contains_key(&name);
        if renew && !exists {
            return fail(format!("\\renewcommand: \\{name} is not defined"));
        }
        let params = match self.opt()? {
            None => 0,
            Some(n) => match n.trim().parse::<u8>() {
                Ok(v) if v <= 9 => v,
                _ => return fail(format!("{what}: bad parameter count [{n}]")),
            },
        };
        let default = self.opt()?;
        if default.is_some() && params == 0 {
            return fail(format!("{what}: optional argument without parameters"));
        }
        let body = self.arg(what)?;
        if !renew && (builtin || self.env.map.get(&name).is_some_and(|old| old.body != body)) {
            return fail(format!("\\newcommand: \\{name} is already defined"));
        }
        let m = Macro {
            params,
            default,
            body,
        };
        // Re-reading an identical definition is accepted (document scopes re-expand the
        // defining formula against an environment that already holds it).
        if !renew && self.env.map.get(&name) == Some(&m) {
            return Ok(());
        }
        self.define(name, m)
    }

    fn def(&mut self) -> Result<(), MathError> {
        self.skip_ws();
        if self.rest.pop() != Some('\\') {
            return fail("\\def needs a control sequence name");
        }
        let name = self.cs_name();
        let mut params = 0u8;
        loop {
            self.skip_ws();
            match self.rest.last() {
                Some('#') => {
                    self.rest.pop();
                    let want = char::from(b'1' + params);
                    if params >= 9 || self.rest.pop() != Some(want) {
                        return fail("\\def: parameters must be #1, #2, … in order");
                    }
                    params += 1;
                }
                Some('{') => break,
                _ => return fail("\\def: delimited parameters are not supported"),
            }
        }
        let body = self.arg("\\def")?;
        self.define(
            name,
            Macro {
                params,
                default: None,
                body,
            },
        )
    }

    fn call(&mut self, name: &str, m: Macro) -> Result<(), MathError> {
        self.steps += 1;
        if self.steps > MAX_STEPS {
            return fail(format!("macro expansion exceeds {MAX_STEPS} steps"));
        }
        let mut args: Vec<String> = Vec::with_capacity(usize::from(m.params));
        let what = format!("\\{name}");
        for k in 0..m.params {
            if k == 0
                && let Some(d) = &m.default
            {
                let v = self.opt()?.unwrap_or_else(|| d.clone());
                args.push(v);
                continue;
            }
            args.push(self.arg(&what)?);
        }
        let mut exp = String::with_capacity(m.body.len());
        let mut it = m.body.chars().peekable();
        while let Some(c) = it.next() {
            if c != '#' {
                exp.push(c);
                continue;
            }
            match it.peek().copied() {
                Some('#') => {
                    it.next();
                    exp.push('#');
                }
                Some(d @ '1'..='9') => {
                    it.next();
                    let i = usize::from(d as u8 - b'1');
                    match args.get(i) {
                        Some(a) => exp.push_str(a),
                        None => return fail(format!("{what}: #{d} without a parameter")),
                    }
                }
                _ => exp.push('#'),
            }
        }
        self.tokens += exp.chars().filter(|c| !c.is_whitespace()).count();
        if self.tokens > MAX_TOKENS {
            return fail(format!("macro expansion exceeds {MAX_TOKENS} tokens"));
        }
        // A control word followed by a letter must stay separated after splicing.
        if exp.ends_with(|c: char| c.is_ascii_alphabetic())
            && self.rest.last().is_some_and(char::is_ascii_alphabetic)
            && control_word_tail(&exp)
        {
            self.rest.push(' ');
        }
        self.rest.extend(exp.chars().rev());
        Ok(())
    }

    fn run(&mut self) -> Result<(), MathError> {
        while let Some(c) = self.rest.pop() {
            if c != '\\' {
                if !c.is_whitespace() {
                    self.tokens += 1;
                    if self.tokens > MAX_TOKENS {
                        return fail(format!("macro expansion exceeds {MAX_TOKENS} tokens"));
                    }
                }
                self.emit(c);
                continue;
            }
            let name = self.cs_name();
            match name.as_str() {
                "newcommand" => self.newcommand(false)?,
                "renewcommand" => self.newcommand(true)?,
                "def" => self.def()?,
                _ => {
                    if let Some(m) = self.env.map.get(&name).cloned() {
                        self.call(&name, m)?;
                        continue;
                    }
                    self.tokens += 1;
                    let word = name.chars().all(|c| c.is_ascii_alphabetic()) && !name.is_empty();
                    self.emit('\\');
                    self.out.push_str(&name);
                    self.after_word = word;
                }
            }
        }
        Ok(())
    }
}

/// Does `s` end with a control word (`\name`)?
fn control_word_tail(s: &str) -> bool {
    let t = s.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    t.ends_with('\\') && !t.ends_with("\\\\")
}
