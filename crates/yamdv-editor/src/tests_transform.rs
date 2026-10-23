//! Typing-transform unit tests (prompt 016 §3 item 3).
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use crate::Editor;
use crate::transform::{enter, pipe_cells, space, task_toggle_at};

fn at_end(t: &str) -> Editor {
    let mut e = Editor::new(t, "\n");
    e.move_to(e.len(), false);
    e
}

#[test]
fn line_start_markers_are_one_step_and_undo_to_literal() {
    for (typed, esc) in [
        ("#", "\\# "),
        ("###", "\\### "),
        ("-", "\\- "),
        ("*", "\\* "),
        ("+", "\\+ "),
        ("1.", "1\\. "),
        (">", "\\> "),
        ("- [ ]", "- \\[ ] "),
        ("- [x]", "- \\[x] "),
    ] {
        let mut e = at_end(typed);
        assert!(space(&mut e), "{typed}");
        assert_eq!(e.text(), format!("{typed} "));
        assert_eq!(e.caret(), typed.len() + 1);
        assert!(e.undo());
        assert_eq!(e.text(), esc, "{typed}");
    }
    for no in ["#######", "a", "text #", "1x", "    #"] {
        let mut e = at_end(no);
        assert!(!space(&mut e), "{no}");
    }
}

fn ent(e: &mut Editor) -> bool {
    let n = e.len();
    enter(e, 0..n)
}

#[test]
fn enter_blocks_and_tables() {
    let mut e = at_end("```rust");
    assert!(ent(&mut e));
    assert_eq!(e.text(), "```rust\n\n```");
    assert_eq!(e.caret(), 8);
    assert!(e.undo());
    assert_eq!(e.text(), "```rust");
    let mut e = at_end("```");
    assert!(ent(&mut e));
    assert_eq!(e.text(), "```\n\n```");
    let mut e = at_end("$$");
    assert!(ent(&mut e));
    assert_eq!(e.text(), "$$\n\n$$");
    let mut e = at_end("---");
    assert!(ent(&mut e));
    assert_eq!((e.text().as_str(), e.caret()), ("---\n\n", 5));
    let mut e = at_end("| a | b |");
    assert!(ent(&mut e));
    assert_eq!(e.text(), "| a | b |\n| --- | --- |\n|  |  |");
    assert_eq!(e.caret(), "| a | b |\n| --- | --- |\n| ".len());
    assert!(e.undo());
    assert_eq!(e.text(), "| a | b |");
    // Already closed fence, plain text, mid-paragraph rule (setext) are left to Enter.
    let mut e = Editor::new("```\nx\n```", "\n");
    e.move_to(3, false);
    assert!(!enter(&mut e, 0..11));
    assert!(!ent(&mut at_end("plain")));
    let mut e = at_end("para\n---");
    assert!(!ent(&mut e));
    assert_eq!(pipe_cells("a \\| b | c").unwrap(), vec!["a \\| b", "c"]);
    assert!(pipe_cells("no pipes").is_none());
}

#[test]
fn task_checkbox_hit() {
    assert_eq!(task_toggle_at("- [ ] a", 3), Some((3, 'x')));
    assert_eq!(task_toggle_at("  1. [x] a", 6), Some((6, ' ')));
    assert_eq!(task_toggle_at("> * [X] a", 4), Some((5, ' ')));
    assert_eq!(task_toggle_at("- [ ] a", 7), None);
    assert_eq!(task_toggle_at("[ ] a", 1), None);
}
