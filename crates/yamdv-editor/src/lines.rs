//! Line operations (prompt 015 §3.4, R-EDIT-07): duplicate (`Ctrl+Shift+D`), select line
//! (`Ctrl+L`, repeat extends), move lines (`Alt+↑/↓`). Each is one undo step. Pure.

use std::ops::Range;

use crate::buffer::Editor;

/// Lines touched by the selection (a selection ending at a line start excludes that line).
fn span(ed: &Editor) -> (usize, usize) {
    let s = ed.selection();
    let a = ed.line_of(s.start);
    let mut b = ed.line_of(s.end);
    if b > a && s.end == ed.line_start(b) {
        b -= 1;
    }
    (a, b)
}

/// Byte range of lines `a..=b` including the final line break when there is one.
fn block(ed: &Editor, a: usize, b: usize) -> Range<usize> {
    let end = if b + 1 < ed.line_count() {
        ed.line_start(b + 1)
    } else {
        ed.len()
    };
    ed.line_start(a)..end
}

/// Duplicates the selected lines below; the selection moves to the copy.
pub fn duplicate(ed: &mut Editor) {
    let (a, b) = span(ed);
    let r = block(ed, a, b);
    let text = ed.slice(r.clone());
    let (ins_at, ins) = if text.ends_with('\n') {
        (r.end, text.clone())
    } else {
        (r.end, format!("{}{}", ed.eol, text))
    };
    let shift = ins.len();
    ed.replace_select(
        ins_at..ins_at,
        &ins,
        (ed.anchor() + shift, ed.caret() + shift),
    );
    ed.seal();
}

/// Selects the current line (with its break); repeated, extends by the next line.
pub fn select_line(ed: &mut Editor) {
    let s = ed.selection();
    let (a, b) = span(ed);
    let whole = block(ed, a, b);
    if !s.is_empty() && s == whole {
        let next = (b + 1).min(ed.line_count().saturating_sub(1));
        let r = block(ed, a, next);
        ed.set_selection(r.start, r.end);
    } else {
        ed.set_selection(whole.start, whole.end);
    }
}

/// Moves the selected lines one line up (`up`) or down; `false` at the buffer edge.
pub fn move_lines(ed: &mut Editor, up: bool) -> bool {
    let (a, b) = span(ed);
    // The empty "line" after a final line break does not count.
    let mut n = ed.line_count();
    if n > 1 && ed.line_start(n - 1) == ed.len() {
        n -= 1;
    }
    if (up && a == 0) || (!up && b + 1 >= n) {
        return false;
    }
    let strip = |s: String| -> String {
        s.strip_suffix("\r\n")
            .or_else(|| s.strip_suffix('\n'))
            .map_or(s.clone(), str::to_string)
    };
    let (other, region) = if up {
        (a - 1, a - 1..=b)
    } else {
        (b + 1, a..=b + 1)
    };
    let r = block(ed, *region.start(), *region.end());
    let tail = ed.slice(r.clone());
    let ends_nl = tail.ends_with('\n');
    let moving = strip(ed.slice(block(ed, a, b)));
    let neighbour = strip(ed.slice(block(ed, other, other)));
    let eol = ed.eol;
    let body = if up {
        format!("{moving}{eol}{neighbour}")
    } else {
        format!("{neighbour}{eol}{moving}")
    };
    let text = if ends_nl {
        format!("{body}{eol}")
    } else {
        body
    };
    let shift: isize = if up {
        -((neighbour.len() + eol.len()).cast_signed())
    } else {
        (neighbour.len() + eol.len()).cast_signed()
    };
    let mv = |p: usize| (p.cast_signed() + shift).max(0).cast_unsigned();
    let sel = (mv(ed.anchor()), mv(ed.caret()));
    ed.replace_select(r, &text, sel);
    ed.seal();
    true
}
