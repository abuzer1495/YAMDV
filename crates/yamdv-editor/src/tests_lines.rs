//! Line-op and auto-pair unit tests (prompt 015 §3.4).
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use crate::{Editor, lines, pairs};

fn ed(t: &str, caret: usize) -> Editor {
    let mut e = Editor::new(t, "\n");
    e.move_to(caret, false);
    e
}

#[test]
fn duplicate_line_and_undo() {
    let mut e = ed("one\ntwo\n", 1);
    lines::duplicate(&mut e);
    assert_eq!(e.text(), "one\none\ntwo\n");
    assert_eq!(e.caret(), 5);
    let mut last = ed("a\nb", 3);
    lines::duplicate(&mut last);
    assert_eq!(last.text(), "a\nb\nb");
    assert!(last.undo());
    assert_eq!(last.text(), "a\nb");
}

#[test]
fn select_line_repeats_extend() {
    let mut e = ed("one\ntwo\nthree", 1);
    lines::select_line(&mut e);
    assert_eq!(e.selected_text(), "one\n");
    lines::select_line(&mut e);
    assert_eq!(e.selected_text(), "one\ntwo\n");
    lines::select_line(&mut e);
    assert_eq!(e.selected_text(), "one\ntwo\nthree");
}

#[test]
fn move_lines_up_down_one_step_each() {
    let mut e = ed("a\nb\nc\n", 2);
    assert!(lines::move_lines(&mut e, true));
    assert_eq!(e.text(), "b\na\nc\n");
    assert_eq!(e.caret(), 0);
    assert!(!lines::move_lines(&mut e, true));
    assert!(lines::move_lines(&mut e, false));
    assert!(lines::move_lines(&mut e, false));
    assert_eq!(e.text(), "a\nc\nb\n");
    assert_eq!(e.line_of(e.caret()), 2);
    assert!(!lines::move_lines(&mut e, false));
    e.undo();
    assert_eq!(e.text(), "a\nb\nc\n");
    let mut crlf = Editor::new("x\r\ny", "\r\n");
    crlf.move_to(4, false);
    assert!(lines::move_lines(&mut crlf, true));
    assert_eq!(crlf.text(), "y\r\nx");
}

#[test]
fn auto_pair_open_overtype_backspace() {
    let mut e = ed("", 0);
    assert!(pairs::type_char(&mut e, '(', false));
    assert_eq!((e.text().as_str(), e.caret()), ("()", 1));
    e.type_text("x", 0);
    assert!(pairs::type_char(&mut e, ')', false));
    assert_eq!((e.text().as_str(), e.caret()), ("(x)", 3));
    let mut b = ed("", 0);
    pairs::type_char(&mut b, '[', false);
    assert!(pairs::backspace(&mut b));
    assert_eq!(b.text(), "");
}

#[test]
fn auto_pair_doubles_quotes_wrap_and_code() {
    let mut e = ed("*", 1);
    assert!(pairs::type_char(&mut e, '*', false));
    assert_eq!((e.text().as_str(), e.caret()), ("****", 2));
    assert!(pairs::backspace(&mut e));
    assert_eq!(e.text(), "");
    let mut q = ed("don", 3);
    assert!(!pairs::type_char(&mut q, '\'', false));
    let mut w = ed("word", 0);
    w.set_selection(0, 4);
    assert!(pairs::type_char(&mut w, '"', false));
    assert_eq!(w.text(), "\"word\"");
    assert_eq!(w.selected_text(), "word");
    let mut c = ed("", 0);
    assert!(!pairs::type_char(&mut c, '"', true));
    assert!(pairs::type_char(&mut c, '{', true));
    assert_eq!(c.text(), "{}");
}
