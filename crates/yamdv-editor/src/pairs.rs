//! Auto-pairing (prompt 015 §3.4, `editing.auto_pair`, default off): `( [ { " ' `` ` `` ** __ ==`,
//! over-typing a closer skips it, Backspace between an empty pair deletes both; inside code only
//! brackets pair. Pure.

use crate::buffer::Editor;

const BRACKETS: &[(char, char)] = &[('(', ')'), ('[', ']'), ('{', '}')];
const QUOTES: &[char] = &['"', '\'', '`'];
const DOUBLES: &[char] = &['*', '_', '='];

fn char_before(ed: &Editor, p: usize) -> Option<char> {
    let a = ed.prev_grapheme(p);
    ed.slice(a..p).chars().next_back()
}

fn char_after(ed: &Editor, p: usize) -> Option<char> {
    let b = ed.next_grapheme(p);
    ed.slice(p..b).chars().next()
}

/// Typed `c` with auto-pairing on; `true` when handled (otherwise type it normally).
/// `in_code` = the caret is in inline code or a code block.
pub fn type_char(ed: &mut Editor, c: char, in_code: bool) -> bool {
    let sel = ed.selection();
    let p = sel.start;
    let next = char_after(ed, sel.end);
    // Over-typing the closer that follows.
    let is_closer = BRACKETS.iter().any(|b| b.1 == c) || QUOTES.contains(&c);
    if sel.is_empty() && is_closer && next == Some(c) {
        ed.move_to(ed.next_grapheme(p), false);
        return true;
    }
    let close = if let Some(b) = BRACKETS.iter().find(|b| b.0 == c) {
        b.1.to_string()
    } else if in_code {
        return false;
    } else if QUOTES.contains(&c) {
        // Apostrophes in words ("don't") never pair.
        if c == '\'' && char_before(ed, p).is_some_and(char::is_alphanumeric) {
            return false;
        }
        c.to_string()
    } else if DOUBLES.contains(&c) && sel.is_empty() && char_before(ed, p) == Some(c) {
        // Second `*` of `**`: close with `**`, unless a closer follows already.
        if next == Some(c) {
            return false;
        }
        format!("{c}{c}")
    } else {
        return false;
    };
    let mid = ed.slice(sel.clone());
    let open = c.to_string();
    let text = format!("{open}{mid}{close}");
    let a = p + open.len();
    ed.replace_select(sel, &text, (a, a + mid.len()));
    ed.seal();
    true
}

/// Backspace with auto-pairing on: between an empty pair deletes both; `true` when handled.
pub fn backspace(ed: &mut Editor) -> bool {
    let s = ed.selection();
    if !s.is_empty() || s.start == 0 {
        return false;
    }
    let p = s.start;
    let before = ed.slice(p.saturating_sub(2)..p);
    let after = ed.slice(p..(p + 2).min(ed.len()));
    for d in DOUBLES {
        let dd = format!("{d}{d}");
        if before == dd && after == dd {
            ed.replace(p - 2..p + 2, "", p - 2);
            return true;
        }
    }
    let (Some(b), Some(a)) = (char_before(ed, p), char_after(ed, p)) else {
        return false;
    };
    let pair = BRACKETS.contains(&(b, a)) || (QUOTES.contains(&b) && a == b);
    if pair {
        ed.replace(p - b.len_utf8()..p + a.len_utf8(), "", p - b.len_utf8());
    }
    pair
}
