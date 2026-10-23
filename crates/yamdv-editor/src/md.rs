//! Markdown-aware keys and formatting commands (R-EDIT-07 minus FUT-12, prompt 006 §2.1).
//! Every command is one undo step.

// Slices are taken at offsets of ASCII markers or at offsets computed from the same string, so
// they sit on char boundaries; positions are clamped to the buffer length; isize arithmetic is
// on byte offsets of buffers far below isize::MAX (documents ≤ 256 MiB, S-PARSE guard).
#![allow(
    clippy::indexing_slicing,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use std::ops::Range;

use crate::buffer::Editor;

/// Inline wrappers (`Ctrl+B`, `Ctrl+I`, ``Ctrl+` ``, `Ctrl+Shift+X`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wrap {
    /// `**…**`.
    Bold,
    /// `*…*`.
    Italic,
    /// `` `…` ``.
    Code,
    /// `~~…~~`.
    Strike,
    /// `==…==` (highlight, Ctrl+Shift+H).
    Highlight,
}

impl Wrap {
    fn marker(self) -> &'static str {
        match self {
            Wrap::Bold => "**",
            Wrap::Italic => "*",
            Wrap::Code => "`",
            Wrap::Strike => "~~",
            Wrap::Highlight => "==",
        }
    }
}

/// Line-level formats (`Ctrl+1…6`, `Ctrl+Shift+0`, `Ctrl+Shift+Q`, `Ctrl+Shift+8/7/9`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineFormat {
    /// `#`×n heading (toggles).
    Heading(u8),
    /// Plain paragraph (removes heading and list markers).
    Paragraph,
    /// `> ` quote (toggles).
    Quote,
    /// `- ` bullet list (toggles).
    Bullet,
    /// `1. ` numbered list (toggles).
    Numbered,
    /// `- [ ] ` task list (toggles).
    Task,
}

/// Parsed line prefix: `indent` `quote` `marker` `task`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Prefix {
    /// Leading spaces/tabs.
    pub indent: String,
    /// `>` quote markers with their spaces (possibly nested).
    pub quote: String,
    /// List marker with its trailing space(s): `- `, `* `, `1. `, `2) ` (empty if none).
    pub marker: String,
    /// Task box with its trailing space: `[ ] ` / `[x] ` (empty if none).
    pub task: String,
}

impl Prefix {
    /// Byte length of the whole prefix.
    pub fn len(&self) -> usize {
        self.indent.len() + self.quote.len() + self.marker.len() + self.task.len()
    }

    /// No prefix at all.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// A list item (bullet or ordered).
    pub fn is_list(&self) -> bool {
        !self.marker.is_empty()
    }

    /// Prefix for a continuation line (ordered numbers incremented, task box unchecked).
    pub fn next(&self) -> String {
        let mut m = self.marker.clone();
        let digits: String = m.chars().take_while(char::is_ascii_digit).collect();
        if !digits.is_empty()
            && let Ok(n) = digits.parse::<u64>()
        {
            m = format!("{}{}", n + 1, &m[digits.len()..]);
        }
        let task = if self.task.is_empty() { "" } else { "[ ] " };
        format!("{}{}{}{}", self.indent, self.quote, m, task)
    }
}

/// Parses the Markdown prefix of `line`.
pub fn prefix(line: &str) -> Prefix {
    let mut p = Prefix::default();
    let b = line.as_bytes();
    let mut i = 0;
    while i < b.len() && (b[i] == b' ' || b[i] == b'\t') {
        i += 1;
    }
    p.indent = line[..i].to_string();
    let qs = i;
    while i < b.len() && b[i] == b'>' {
        i += 1;
        while i < b.len() && b[i] == b' ' {
            i += 1;
        }
    }
    p.quote = line[qs..i].to_string();
    let ms = i;
    let mut j = i;
    if j < b.len() && matches!(b[j], b'-' | b'*' | b'+') {
        j += 1;
    } else {
        let d0 = j;
        while j < b.len() && b[j].is_ascii_digit() && j - d0 < 9 {
            j += 1;
        }
        if j > d0 && j < b.len() && matches!(b[j], b'.' | b')') {
            j += 1;
        } else {
            j = ms;
        }
    }
    if j > ms && j < b.len() && (b[j] == b' ' || b[j] == b'\t') {
        while j < b.len() && (b[j] == b' ' || b[j] == b'\t') {
            j += 1;
        }
        p.marker = line[ms..j].to_string();
        i = j;
        let rest = &line[i..];
        for t in ["[ ] ", "[x] ", "[X] "] {
            if rest.starts_with(t) {
                p.task = t.to_string();
            }
        }
    } else if j > ms && (j == b.len() || b[j] == b'\n' || b[j] == b'\r') {
        // A bare marker at the end of the line ("-" / "1.") is an empty item.
        p.marker = line[ms..j].to_string();
    }
    p
}

fn line_body(t: &str) -> &str {
    t.trim_end_matches(['\n', '\r'])
}

/// Enter: continue list / task / quote prefixes; an empty item ends the list.
pub fn enter(ed: &mut Editor) {
    let sel = ed.selection();
    let l = ed.line_of(sel.start);
    let ls = ed.line_start(l);
    let text = ed.line_text(l);
    let body = line_body(&text);
    let p = prefix(body);
    let eol = ed.eol;
    if p.is_list() || !p.quote.is_empty() {
        let content_empty =
            body.len() <= p.len() && body[p.len().min(body.len())..].trim().is_empty();
        if content_empty && sel.is_empty() && sel.start == ls + body.len() {
            // End the list: remove the marker (keep outer quote for nested lists).
            let keep = if p.is_list() {
                p.quote.clone()
            } else {
                String::new()
            };
            let caret = ls + keep.len();
            ed.replace(ls..ls + body.len(), &keep, caret);
            return;
        }
        let ins = format!("{eol}{}", p.next());
        let caret = sel.start + ins.len();
        ed.replace(sel, &ins, caret);
        return;
    }
    let caret = sel.start + eol.len();
    ed.replace(sel, eol, caret);
}

fn lines_of_selection(ed: &Editor) -> Range<usize> {
    let s = ed.selection();
    let a = ed.line_of(s.start);
    let mut b = ed.line_of(s.end);
    if b > a && s.end == ed.line_start(b) {
        b -= 1;
    }
    a..b + 1
}

/// Applies per-line prefix replacements `(line, old prefix len, new prefix)` as one undo step,
/// keeping the selection on the same text.
fn apply_lines(ed: &mut Editor, edits: Vec<(usize, usize, String)>) {
    let mut out: Vec<(Range<usize>, String)> = Vec::new();
    for (l, old, new) in edits {
        let s = ed.line_start(l);
        out.push((s..s + old, new));
    }
    let map = |pos: usize| -> usize {
        let mut d: isize = 0;
        for (r, t) in &out {
            if r.end <= pos {
                d += t.len() as isize - r.len() as isize;
            } else if r.start < pos {
                // Inside a replaced prefix: move to the end of the new prefix.
                return (r.start as isize + d + t.len() as isize) as usize;
            }
        }
        (pos as isize + d).max(0) as usize
    };
    let sel = (map(ed.anchor()), map(ed.caret()));
    ed.replace_many(&out, sel);
}

/// Tab / Shift+Tab: indent or outdent list items by `unit` (`"  "`, `"    "` or `"\t"`);
/// elsewhere Tab inserts 4 spaces and Shift+Tab does nothing.
pub fn tab(ed: &mut Editor, unit: &str, outdent: bool) {
    let lines = lines_of_selection(ed);
    let any_list = lines
        .clone()
        .any(|l| prefix(line_body(&ed.line_text(l))).is_list());
    if !any_list {
        if !outdent {
            let s = ed.selection();
            let c = s.start + 4;
            ed.replace(s, "    ", c);
        }
        return;
    }
    let mut edits = Vec::new();
    for l in lines {
        let t = ed.line_text(l);
        let p = prefix(line_body(&t));
        if !p.is_list() {
            continue;
        }
        if outdent {
            let ind = &p.indent;
            let cut = if ind.starts_with('\t') {
                1
            } else {
                ind.len() - ind.trim_start_matches(' ').len()
            }
            .min(if unit == "\t" { 4 } else { unit.len() });
            if cut > 0 {
                edits.push((l, cut, String::new()));
            }
        } else {
            edits.push((l, 0, unit.to_string()));
        }
    }
    if !edits.is_empty() {
        apply_lines(ed, edits);
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn run_len(s: &str, ch: char, from_end: bool) -> usize {
    if from_end {
        s.chars().rev().take_while(|&c| c == ch).count()
    } else {
        s.chars().take_while(|&c| c == ch).count()
    }
}

/// Toggles an inline wrapper on the selection, or on the word at the caret.
pub fn toggle_wrap(ed: &mut Editor, w: Wrap) {
    let m = w.marker();
    let ml = m.len();
    let sel = ed.selection();
    let caret = ed.caret();
    let mut r = sel.clone();
    let mut from_word = false;
    if r.is_empty() {
        let wr = ed.word_at(caret);
        if ed.slice(wr.clone()).chars().any(is_word_char) {
            r = wr;
            from_word = true;
        }
    }
    if r.is_empty() {
        let c = r.start + ml;
        ed.replace(r, &format!("{m}{m}"), c);
        return;
    }
    let text = ed.slice(r.clone());
    // Selection includes the markers: unwrap it.
    if text.len() >= 2 * ml && text.starts_with(m) && text.ends_with(m) {
        let inner = text[ml..text.len() - ml].to_string();
        let end = r.start + inner.len();
        ed.replace_select(r.clone(), &inner, (r.start, end));
        return;
    }
    // Markers around the range: remove them.
    let before = ed.slice(r.start.saturating_sub(3)..r.start);
    let after = ed.slice(r.end..r.end + 3);
    let around = before.ends_with(m) && after.starts_with(m) && {
        if w == Wrap::Italic {
            // `*x*` or `***x***`, not `**x**` (that is bold).
            let (a, b) = (run_len(&before, '*', true), run_len(&after, '*', false));
            a % 2 == 1 && b % 2 == 1
        } else {
            true
        }
    };
    if around {
        let edits = vec![
            (r.start - ml..r.start, String::new()),
            (r.end..r.end + ml, String::new()),
        ];
        let (a, c) = if from_word {
            (caret - ml, caret - ml)
        } else {
            (sel.start - ml, sel.end - ml)
        };
        ed.replace_many(&edits, (a, c));
        return;
    }
    let edits = vec![
        (r.start..r.start, m.to_string()),
        (r.end..r.end, m.to_string()),
    ];
    let (a, c) = if from_word {
        (caret + ml, caret + ml)
    } else {
        (sel.start + ml, sel.end + ml)
    };
    ed.replace_many(&edits, (a, c));
}

/// Ctrl+K: wraps the selection (or word) as `[sel](url)` and selects `url`.
pub fn link(ed: &mut Editor) {
    let mut r = ed.selection();
    if r.is_empty() {
        let wr = ed.word_at(ed.caret());
        if ed.slice(wr.clone()).chars().any(is_word_char) {
            r = wr;
        }
    }
    let text = ed.slice(r.clone());
    let s = format!("[{text}](url)");
    let url = r.start + text.len() + 3;
    ed.replace_select(r, &s, (url, url + 3));
}

fn heading_level(body: &str) -> (usize, usize) {
    let n = body.chars().take_while(|&c| c == '#').count();
    if (1..=6).contains(&n) && (body.len() == n || body[n..].starts_with(' ')) {
        let sp = body[n..].chars().take_while(|&c| c == ' ').count();
        (n, n + sp)
    } else {
        (0, 0)
    }
}

/// Applies a line format to every line of the selection.
pub fn line_format(ed: &mut Editor, f: LineFormat) {
    let lines: Vec<usize> = lines_of_selection(ed).collect();
    let bodies: Vec<String> = lines
        .iter()
        .map(|&l| line_body(&ed.line_text(l)).to_string())
        .collect();
    let prefixes: Vec<Prefix> = bodies.iter().map(|b| prefix(b)).collect();
    let all = |pred: &dyn Fn(&Prefix, &str) -> bool| {
        prefixes.iter().zip(&bodies).all(|(p, b)| pred(p, b))
    };
    let is_bullet = |p: &Prefix| p.marker.starts_with(['-', '*', '+']);
    let mut edits = Vec::new();
    match f {
        LineFormat::Heading(n) => {
            let n = usize::from(n.clamp(1, 6));
            let toggle_off = all(&|_, b| heading_level(b).0 == n);
            for (&l, b) in lines.iter().zip(&bodies) {
                let (_, len) = heading_level(b);
                let new = if toggle_off {
                    String::new()
                } else {
                    format!("{} ", "#".repeat(n))
                };
                edits.push((l, len, new));
            }
        }
        LineFormat::Paragraph => {
            for ((&l, b), p) in lines.iter().zip(&bodies).zip(&prefixes) {
                let (_, hl) = heading_level(b);
                if hl > 0 {
                    edits.push((l, hl, String::new()));
                } else if p.is_list() {
                    edits.push((l, p.len(), p.indent.clone() + &p.quote));
                }
            }
        }
        LineFormat::Quote => {
            let off = all(&|p, _| !p.quote.is_empty());
            for (&l, p) in lines.iter().zip(&prefixes) {
                if off {
                    edits.push((l, p.indent.len() + p.quote.len(), p.indent.clone()));
                } else {
                    edits.push((l, 0, "> ".to_string()));
                }
            }
        }
        LineFormat::Bullet | LineFormat::Numbered | LineFormat::Task => {
            let off = match f {
                LineFormat::Bullet => all(&|p, _| is_bullet(p) && p.task.is_empty()),
                LineFormat::Numbered => all(&|p, _| p.is_list() && !is_bullet(p)),
                _ => all(&|p, _| !p.task.is_empty()),
            };
            for (k, (&l, p)) in lines.iter().zip(&prefixes).enumerate() {
                let keep = p.indent.clone() + &p.quote;
                let old = p.len();
                let new = if off {
                    keep
                } else {
                    match f {
                        LineFormat::Bullet => keep + "- ",
                        LineFormat::Numbered => format!("{keep}{}. ", k + 1),
                        _ => keep + "- [ ] ",
                    }
                };
                edits.push((l, old, new));
            }
        }
    }
    apply_lines(ed, edits);
}

/// Ctrl+Enter: toggles `[ ]` ↔ `[x]` on the task lines of the selection.
pub fn toggle_task(ed: &mut Editor) {
    let mut out: Vec<(Range<usize>, String)> = Vec::new();
    for l in lines_of_selection(ed) {
        let t = ed.line_text(l);
        let p = prefix(line_body(&t));
        if p.task.is_empty() {
            continue;
        }
        let at = ed.line_start(l) + p.indent.len() + p.quote.len() + p.marker.len() + 1;
        let new = if p.task == "[ ] " { "x" } else { " " };
        out.push((at..at + 1, new.to_string()));
    }
    if !out.is_empty() {
        let sel = (ed.anchor(), ed.caret());
        ed.replace_many(&out, sel);
    }
}

/// Source edit for a task checkbox click at byte `pos` of `text` (the `[` of `[ ]`/`[x]`):
/// the byte range of the mark character and its replacement (R-F2-07, P-16/P-17).
pub fn checkbox_edit(text: &str, bracket: usize) -> Option<(Range<usize>, &'static str)> {
    let s = text.get(bracket..bracket + 3)?;
    match s {
        "[ ]" => Some((bracket + 1..bracket + 2, "x")),
        "[x]" | "[X]" => Some((bracket + 1..bracket + 2, " ")),
        _ => None,
    }
}
