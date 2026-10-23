//! F2 typing transforms (prompt 016 §3 item 3). Each transform is its own undo step; Ctrl+Z right
//! after it restores the literal text (line-start markers come back backslash-escaped, so they
//! show as typed instead of formatting).

use std::ops::Range;

use crate::Editor;

/// The line-start trigger completed by a space at the caret: `(line start, escaped form)`.
fn space_trigger(prefix: &str) -> Option<String> {
    let lead = prefix.len() - prefix.trim_start_matches(' ').len();
    let (ind, t) = prefix.split_at(lead);
    let hashes = t.chars().take_while(|&c| c == '#').count();
    if lead <= 3 && (1..=6).contains(&hashes) && hashes == t.len() {
        return Some(format!("{ind}\\{t}"));
    }
    if matches!(t, "-" | "*" | "+") || (lead <= 3 && t == ">") {
        return Some(format!("{ind}\\{t}"));
    }
    let digits = t.chars().take_while(char::is_ascii_digit).count();
    if (1..=9).contains(&digits) && t.len() == digits + 1 && (t.ends_with('.') || t.ends_with(')'))
    {
        return Some(format!("{ind}{}\\{}", &t[..digits], &t[digits..]));
    }
    // `[ ]` / `[x]` typed at the start of a list item's content.
    for task in ["[ ]", "[x]", "[X]"] {
        if let Some(item) = t.strip_suffix(task)
            && is_item_marker(item)
        {
            return Some(format!("{ind}{item}\\{task}"));
        }
    }
    None
}

/// `- `, `* `, `+ `, `1. `, `1) ` (with the trailing space).
fn is_item_marker(s: &str) -> bool {
    let Some(m) = s.strip_suffix(' ') else {
        return false;
    };
    let d = m.chars().take_while(char::is_ascii_digit).count();
    matches!(m, "-" | "*" | "+")
        || ((1..=9).contains(&d) && m.len() == d + 1 && (m.ends_with('.') || m.ends_with(')')))
}

/// A space typed in F2: when it completes a line-start marker (`#`…`######`, `-`/`*`/`+`,
/// `1.`, `>`, `[ ]`/`[x]` in a list item), applies it as its own undo step. True when handled.
pub fn space(ed: &mut Editor) -> bool {
    let sel = ed.selection();
    if !sel.is_empty() {
        return false;
    }
    let ls = ed.line_start(ed.line_of(sel.start));
    let prefix = ed.slice(ls..sel.start);
    let Some(esc) = space_trigger(&prefix) else {
        return false;
    };
    ed.seal();
    // Step 1 (what Ctrl+Z returns to): the literal, escaped marker.
    ed.replace(ls..sel.start, &format!("{esc} "), ls + esc.len() + 1);
    ed.seal();
    ed.replace(
        ls..ls + esc.len() + 1,
        &format!("{prefix} "),
        ls + prefix.len() + 1,
    );
    ed.seal();
    true
}

/// Fence opener line: `(fence chars, info)`.
fn fence_open(line: &str) -> Option<String> {
    let t = line.trim_start_matches(' ');
    if line.len() - t.len() > 3 {
        return None;
    }
    let ch = t.chars().next()?;
    let n = t.chars().take_while(|&c| c == ch).count();
    match ch {
        '`' if n >= 3 && !t[n..].contains('`') => Some(t[..n].to_string()),
        '~' if n >= 3 => Some(t[..n].to_string()),
        '$' if t == "$$" => Some("$$".into()),
        _ => None,
    }
}

fn is_rule(line: &str) -> bool {
    let t: String = line.chars().filter(|c| *c != ' ').collect();
    t.len() >= 3
        && (t.chars().all(|c| c == '-')
            || t.chars().all(|c| c == '*')
            || t.chars().all(|c| c == '_'))
}

/// Cells of a pipe row (edge pipes trimmed, `\|` kept inside a cell); `None` unless ≥ 1 pipe.
pub fn pipe_cells(line: &str) -> Option<Vec<String>> {
    let t = line.trim();
    if !t.contains('|') {
        return None;
    }
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut esc = false;
    for c in t.chars() {
        if c == '|' && !esc {
            cells.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
        esc = c == '\\' && !esc;
    }
    cells.push(cur);
    if t.starts_with('|') {
        cells.remove(0);
    }
    if t.ends_with('|') && !t.ends_with("\\|") {
        cells.pop();
    }
    let cells: Vec<String> = cells.into_iter().map(|c| c.trim().to_string()).collect();
    (cells.len() >= 2 || (t.starts_with('|') && t.ends_with('|') && !cells.is_empty()))
        .then_some(cells)
}

/// Enter at the end of a line in F2 (`scope` = the active block): ```` ``` ````/```` ```lang ````
/// opens a code block, `$$` a math block, `---` makes a rule, a pipe header row gets its
/// delimiter row and an empty body row. One undo step. True when handled.
pub fn enter(ed: &mut Editor, scope: Range<usize>) -> bool {
    let sel = ed.selection();
    if !sel.is_empty() {
        return false;
    }
    let l = ed.line_of(sel.start);
    let ls = ed.line_start(l);
    let line = ed.line_text(l);
    let body = line.trim_end_matches(['\n', '\r']);
    if sel.start != ls + body.len() {
        return false;
    }
    let eol = ed.eol;
    let first = ls <= scope.start;
    let prev_blank = first || ed.line_text(l.saturating_sub(1)).trim().is_empty();
    let after = ed.slice(
        (ls + line.len()).min(scope.end.max(ls + line.len()))..scope.end.max(ls + line.len()),
    );
    let (ins, caret_off) = if let Some(f) = fence_open(body) {
        let closed = after.lines().any(|x| {
            let x = x.trim();
            if f == "$$" {
                x == "$$"
            } else {
                x.len() >= f.len() && x.chars().all(|c| f.starts_with(c))
            }
        });
        if closed || !prev_blank {
            return false;
        }
        (format!("{eol}{eol}{f}"), eol.len())
    } else if is_rule(body) && prev_blank {
        (format!("{eol}{eol}"), 2 * eol.len())
    } else if let Some(cells) =
        pipe_cells(body).filter(|_| prev_blank && !after.trim_start().starts_with('|'))
    {
        let n = cells.len();
        let delim = format!("|{}", " --- |".repeat(n));
        let row = format!("|{}", "  |".repeat(n));
        (
            format!("{eol}{delim}{eol}{row}"),
            eol.len() + delim.len() + eol.len() + 2,
        )
    } else {
        return false;
    };
    ed.seal();
    let at = sel.start;
    ed.replace(at..at, &ins, at + caret_off);
    ed.seal();
    true
}
