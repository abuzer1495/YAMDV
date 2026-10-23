//! Insert ▸ blocks (prompt 016 §3 item 6): each insertion is one undo step and lands on its own
//! lines, separated from the surrounding text by blank lines, in the buffer's line ending.

use crate::Editor;

/// GitHub alert kinds of Insert ▸ Callout ▸.
pub const CALLOUTS: [&str; 5] = ["NOTE", "TIP", "IMPORTANT", "WARNING", "CAUTION"];

/// A block to insert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Block {
    /// Empty `rows × cols` table (header + delimiter + body rows).
    Table(usize, usize),
    /// Fenced code block.
    Code,
    /// `> [!KIND]` callout (index into [`CALLOUTS`]).
    Callout(usize),
    /// `$$` math block.
    Math,
    /// Mermaid fence with a starter diagram.
    Mermaid,
    /// Thematic break.
    Rule,
}

/// Markdown table text for `rows` (header included) × `cols`, and the caret offset (first
/// header cell).
pub fn table_text(rows: usize, cols: usize, eol: &str) -> (String, usize) {
    let (rows, cols) = (rows.max(1), cols.max(1));
    let row = |cell: &str| {
        let mut s = String::from("|");
        for _ in 0..cols {
            s.push_str(cell);
            s.push('|');
        }
        s
    };
    let mut lines = vec![row("   "), row(" --- ")];
    for _ in 1..rows {
        lines.push(row("   "));
    }
    (lines.join(eol), 2)
}

/// Text of `b` and the caret offset inside it.
fn text(b: Block, eol: &str) -> (String, usize) {
    match b {
        Block::Table(r, c) => table_text(r, c, eol),
        Block::Code => (format!("```{eol}{eol}```"), 3 + eol.len()),
        Block::Callout(k) => {
            let k = CALLOUTS.get(k).copied().unwrap_or("NOTE");
            let s = format!("> [!{k}]{eol}> ");
            let n = s.len();
            (s, n)
        }
        Block::Math => (format!("$${eol}{eol}$$"), 2 + eol.len()),
        Block::Mermaid => {
            let head = format!("```mermaid{eol}");
            (
                format!("{head}graph TD{eol}  A --> B{eol}```"),
                head.len() + "graph TD".len(),
            )
        }
        Block::Rule => ("---".to_string(), 3),
    }
}

/// Inserts `b` after the caret's line (or on it when that line is blank). One undo step.
pub fn block(ed: &mut Editor, b: Block) {
    let eol = ed.eol;
    let (body, caret) = text(b, eol);
    let pos = ed.selection().end;
    let mut l = ed.line_of(pos);
    // After the caret's whole block (a table or paragraph is never split; R-TBL-08).
    if !ed.line_text(l).trim().is_empty() {
        while l + 1 < ed.line_count() && !ed.line_text(l + 1).trim().is_empty() {
            l += 1;
        }
    }
    let ls = ed.line_start(l);
    let line = ed.line_text(l);
    let content = line.trim_end_matches(['\n', '\r']);
    let le = ls + content.len();
    let has_next = line.len() > content.len();
    let blank_prev = l == 0 || ed.line_text(l - 1).trim().is_empty();
    let (at, mut s, mut off) = if content.trim().is_empty() {
        // On a blank line: replace it, keeping a blank line before when needed.
        let pre = if blank_prev { "" } else { eol };
        (ls..le, pre.to_string(), pre.len())
    } else {
        (le..le, format!("{eol}{eol}"), 2 * eol.len())
    };
    s.push_str(&body);
    off += caret;
    let next_blank = ed.line_count() > l + 1 && ed.line_text(l + 1).trim().is_empty();
    if !has_next || !next_blank {
        s.push_str(eol);
    }
    let start = at.start;
    ed.seal();
    ed.replace(at, &s, start + off);
    ed.seal();
}

/// Inserts footnote `[^n]` at the caret and its definition `[^n]: ` at the end of the
/// document; the caret goes to the definition. One undo step.
pub fn footnote(ed: &mut Editor) {
    let t = ed.text();
    let n = (1..).find(|n| !t.contains(&format!("[^{n}]"))).unwrap_or(1);
    let eol = ed.eol;
    let r = format!("[^{n}]");
    let def = format!("[^{n}]: ");
    let pos = ed.selection().end;
    let tail = if t.ends_with(['\n']) {
        eol.to_string()
    } else {
        format!("{eol}{eol}")
    };
    let end = t.len();
    let def_text = format!("{tail}{def}{eol}");
    let caret = end + r.len() + tail.len() + def.len();
    ed.seal();
    ed.replace_many(&[(pos..pos, r), (end..end, def_text)], (caret, caret));
    ed.seal();
}
