//! "Clear formatting" of the F2 format toolbar (prompt 015 §3.1): removes paired inline
//! markers (`**` `__` `~~` `==` `*` `_` `` ` ``) inside the selection and the line prefix
//! (heading, quote, list, task; indentation stays) of the touched lines. One undo step. Pure.

use crate::buffer::Editor;
use crate::md;

/// Inline markers, longest first.
const MARKS: &[&str] = &["**", "__", "~~", "==", "*", "_", "`"];

fn word(c: Option<char>) -> bool {
    c.is_some_and(char::is_alphanumeric)
}

/// Removes markers `m` in pairs (an odd last one stays); `_` inside a word is not a marker.
fn strip_pairs(s: &str, m: &str) -> String {
    let mut idx: Vec<usize> = s
        .match_indices(m)
        .map(|(i, _)| i)
        .filter(|&i| {
            !(m.starts_with('_')
                && word(s[..i].chars().next_back())
                && word(s[i + m.len()..].chars().next()))
        })
        .collect();
    if idx.len() % 2 == 1 {
        idx.pop();
    }
    let mut out = String::with_capacity(s.len());
    let mut last = 0;
    for i in idx {
        out.push_str(&s[last..i]);
        last = i + m.len();
    }
    out.push_str(&s[last..]);
    out
}

/// Inline markers removed (pairs only).
pub fn strip_inline(s: &str) -> String {
    MARKS
        .iter()
        .fold(s.to_string(), |acc, m| strip_pairs(&acc, m))
}

/// One line without heading / quote / list / task prefix (indentation kept).
pub fn strip_line(l: &str) -> String {
    let p = md::prefix(l);
    let body = &l[p.len().min(l.len())..];
    let hashes = body.chars().take_while(|c| *c == '#').count();
    let body = if (1..=6).contains(&hashes) && body[hashes..].starts_with([' ', '\t']) {
        body[hashes..].trim_start_matches([' ', '\t'])
    } else {
        body
    };
    format!("{}{body}", p.indent)
}

/// Clears inline and line formatting of the selection (or the caret's line).
pub fn clear_format(ed: &mut Editor) {
    let sel = ed.selection();
    let (l0, l1) = (ed.line_of(sel.start), ed.line_of(sel.end));
    let (ls, le) = (ed.line_start(l0), ed.line_end(l1));
    let mut r = if sel.is_empty() { ls..le } else { sel };
    // A selected word inside markers (`**|Bold|**`) takes the markers along.
    while r.start > ls && r.end < le {
        let (a, b) = (ed.slice(r.start - 1..r.start), ed.slice(r.end..r.end + 1));
        if a != b || !matches!(a.as_str(), "*" | "_" | "~" | "=" | "`") {
            break;
        }
        r = r.start - 1..r.end + 1;
    }
    let mid = strip_inline(&ed.slice(r.clone()));
    let joined = format!("{}{}{}", ed.slice(ls..r.start), mid, ed.slice(r.end..le));
    let text = joined
        .split('\n')
        .map(strip_line)
        .collect::<Vec<_>>()
        .join("\n");
    if text == ed.slice(ls..le) {
        return;
    }
    let end = ls + text.len();
    ed.replace_select(ls..le, &text, (ls, end));
    ed.seal();
}
