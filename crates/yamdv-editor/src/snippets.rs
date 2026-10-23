//! Tab snippets (prompt 015 §3.3, `editing.snippets`): a trigger that is the whole word before
//! the caret expands on Tab into Markdown with placeholders; Tab jumps to the next one, Esc
//! leaves. One undo step per expansion. Works in F2 and F3. Pure.

use std::ops::Range;

use crate::buffer::Editor;

/// Code-fence language triggers (`python` + Tab → a python fence).
pub const LANGS: &[&str] = &[
    "bash",
    "sh",
    "shell",
    "powershell",
    "ps1",
    "bat",
    "cpp",
    "csharp",
    "cs",
    "java",
    "kotlin",
    "go",
    "rust",
    "rs",
    "python",
    "py",
    "javascript",
    "js",
    "typescript",
    "ts",
    "jsx",
    "tsx",
    "json",
    "yaml",
    "yml",
    "toml",
    "ini",
    "xml",
    "html",
    "css",
    "scss",
    "sql",
    "markdown",
    "md",
    "diff",
    "dockerfile",
    "makefile",
    "php",
    "ruby",
    "swift",
    "lua",
    "dart",
    "scala",
    "perl",
    "haskell",
    "elixir",
    "graphql",
    "proto",
];

/// Callout triggers → GitHub alert type (`red` = caution).
const CALLOUTS: &[(&str, &str)] = &[
    ("note", "NOTE"),
    ("tip", "TIP"),
    ("important", "IMPORTANT"),
    ("warning", "WARNING"),
    ("caution", "CAUTION"),
    ("red", "CAUTION"),
];

/// Largest `tableRxC`.
const MAX_TABLE: usize = 20;

/// An expansion: text plus placeholder ranges (relative to the text start, in order). The last
/// stop is where the caret ends.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Expansion {
    /// Text to insert (lines joined with the document's line ending).
    pub text: String,
    /// Placeholder ranges.
    pub stops: Vec<Range<usize>>,
}

struct B<'a> {
    x: Expansion,
    eol: &'a str,
    indent: &'a str,
}

impl B<'_> {
    fn t(mut self, s: &str) -> Self {
        self.x.text.push_str(s);
        self
    }
    fn nl(mut self) -> Self {
        self.x.text.push_str(self.eol);
        self.x.text.push_str(self.indent);
        self
    }
    fn stop(mut self, s: &str) -> Self {
        let a = self.x.text.len();
        self.x.text.push_str(s);
        self.x.stops.push(a..self.x.text.len());
        self
    }
    fn end(mut self) -> Expansion {
        let n = self.x.text.len();
        self.x.stops.push(n..n);
        self.x
    }
}

fn table(rows: usize, cols: usize, b: B<'_>) -> Expansion {
    let mut b = b.t("|");
    for c in 1..=cols {
        b = b.t(" ").stop(&format!("Column {c}")).t(" |");
    }
    b = b.nl().t("|");
    for _ in 0..cols {
        b = b.t("---|");
    }
    for _ in 0..rows {
        b = b.nl().t("|");
        for _ in 0..cols {
            b = b.t(" ").stop("").t(" |");
        }
    }
    b.end()
}

fn table_dims(w: &str) -> Option<(usize, usize)> {
    let (r, c) = w.strip_prefix("table")?.split_once('x')?;
    let (r, c) = (r.parse::<usize>().ok()?, c.parse::<usize>().ok()?);
    ((1..=MAX_TABLE).contains(&r) && (1..=MAX_TABLE).contains(&c)).then_some((r, c))
}

fn fence(lang: &str, body: &str, b: B<'_>) -> Expansion {
    // The body is one placeholder; its lines take the document's line ending and indent.
    let sep = format!("{}{}", b.eol, b.indent);
    let body = body.replace('\n', &sep);
    b.t("```").t(lang).nl().stop(&body).nl().t("```").end()
}

/// Expansion of `word`; `now` = (`YYYY-MM-DD`, `HH:MM`); `indent` = the line's leading
/// whitespace (continuation lines keep it).
pub fn expand(word: &str, eol: &str, indent: &str, now: (&str, &str)) -> Option<Expansion> {
    let b = B {
        x: Expansion::default(),
        eol,
        indent,
    };
    if let Some(n) = word
        .strip_prefix('h')
        .and_then(|d| d.parse::<usize>().ok())
        .filter(|n| (1..=6).contains(n) && word.len() == 2)
    {
        return Some(b.t(&"#".repeat(n)).t(" ").stop("Heading").end());
    }
    if let Some((r, c)) = table_dims(word) {
        return Some(table(r, c, b));
    }
    if let Some(&(_, kind)) = CALLOUTS.iter().find(|c| c.0 == word) {
        return Some(b.t("> [!").t(kind).t("]").nl().t("> ").stop("Text").end());
    }
    if LANGS.contains(&word) {
        return Some(fence(word, "", b));
    }
    Some(match word {
        "table" => table(2, 2, b),
        "code" => b.t("```").stop("lang").nl().stop("").nl().t("```").end(),
        "link" => b.t("[").stop("text").t("](").stop("https://").t(")").end(),
        "img" => b.t("![").stop("alt").t("](").stop("image.png").t(")").end(),
        "line" | "hr" => b.t("---").nl().end(),
        "quote" => b.t("> ").stop("Quote").end(),
        "task" => b.t("- [ ] ").stop("Task").end(),
        "math" => b.t("$$").nl().stop("x").nl().t("$$").end(),
        "mermaid" | "flow" => fence("mermaid", "flowchart TD\n  A --> B", b),
        "seq" => fence("mermaid", "sequenceDiagram\n  A->>B: Hello", b),
        "gantt" => fence(
            "mermaid",
            "gantt\n  title Plan\n  section A\n  Task :a1, 2026-01-01, 3d",
            b,
        ),
        "date" => b.t(now.0).end(),
        "time" => b.t(now.1).end(),
        "details" => b
            .t("<details>")
            .nl()
            .t("<summary>")
            .stop("Summary")
            .t("</summary>")
            .nl()
            .nl()
            .stop("")
            .nl()
            .nl()
            .t("</details>")
            .end(),
        "footnote" => b.t("[^").stop("1").t("]").end(),
        _ => return None,
    })
}

/// Placeholder navigation after an expansion (absolute ranges; edits are assumed to happen in
/// the current placeholder, the caret leaving it ends the session).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    stops: Vec<Range<usize>>,
    cur: usize,
    len_at: usize,
}

/// The whole word before the caret (letters/digits, preceded by line start or whitespace).
fn trigger(ed: &Editor) -> Option<(Range<usize>, String)> {
    let sel = ed.selection();
    if !sel.is_empty() {
        return None;
    }
    let ls = ed.line_start(ed.line_of(sel.start));
    let before = ed.slice(ls..sel.start);
    let start = before
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_alphanumeric())
        .last()
        .map(|(i, _)| i)?;
    if before[..start]
        .chars()
        .next_back()
        .is_some_and(|c| !c.is_whitespace())
    {
        return None;
    }
    let after = ed.slice(sel.start..ed.line_end(ed.line_of(sel.start)));
    if after.chars().next().is_some_and(char::is_alphanumeric) {
        return None;
    }
    Some((ls + start..sel.start, before[start..].to_string()))
}

/// Tab: expands the trigger before the caret (one undo step) and selects the first placeholder.
pub fn try_expand(ed: &mut Editor, now: (&str, &str)) -> Option<Session> {
    let (r, word) = trigger(ed)?;
    let ls = ed.line_start(ed.line_of(r.start));
    let line = ed.slice(ls..r.start);
    let indent: String = line
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    let x = expand(&word, ed.eol, &indent, now)?;
    let stops: Vec<Range<usize>> = x
        .stops
        .iter()
        .map(|s| r.start + s.start..r.start + s.end)
        .collect();
    let first = stops.first()?.clone();
    ed.replace_select(r, &x.text, (first.start, first.end));
    ed.seal();
    let s = Session {
        stops,
        cur: 0,
        len_at: ed.len(),
    };
    (s.stops.len() > 1).then_some(s)
}

impl Session {
    /// Tab inside the session: selects the next placeholder. `false` = the session ended (the
    /// caret left the current placeholder, or the last one was reached).
    pub fn next(&mut self, ed: &mut Editor) -> bool {
        let delta = ed.len().cast_signed() - self.len_at.cast_signed();
        let Some(cur) = self.stops.get(self.cur).cloned() else {
            return false;
        };
        let cur_end = (cur.end.cast_signed() + delta)
            .max(cur.start.cast_signed())
            .cast_unsigned();
        let sel = ed.selection();
        if sel.start < cur.start || sel.end > cur_end {
            return false;
        }
        for s in self.stops.iter_mut().skip(self.cur + 1) {
            s.start = (s.start.cast_signed() + delta).cast_unsigned();
            s.end = (s.end.cast_signed() + delta).cast_unsigned();
        }
        self.len_at = ed.len();
        self.cur += 1;
        let Some(n) = self.stops.get(self.cur).cloned() else {
            return false;
        };
        ed.set_selection(n.start, n.end);
        self.cur + 1 < self.stops.len()
    }
}
