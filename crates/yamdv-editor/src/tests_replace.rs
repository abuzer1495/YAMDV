//! Find & replace unit tests (prompt 015 §3.2).
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::time::{Duration, Instant};

use crate::Editor;
use crate::replace::{self, FindError, Query, REGEX_GUARD};

fn q(n: &str) -> Query {
    Query {
        needle: n.into(),
        ..Query::default()
    }
}

#[test]
fn replace_all_is_one_undo_group() {
    let mut e = Editor::new("foo x foo y FOO\n", "\n");
    let n = replace::replace_all(&mut e, &q("foo"), "bar").unwrap();
    assert_eq!(n, 3);
    assert_eq!(e.text(), "bar x bar y bar\n");
    assert!(e.undo());
    assert_eq!(e.text(), "foo x foo y FOO\n");
    assert!(!e.undo());
    assert!(e.redo());
    assert_eq!(e.text(), "bar x bar y bar\n");
}

#[test]
fn case_whole_word_and_turkish_folding() {
    let hay = "İstanbul istanbul ISTANBUL ıstanbul foo foobar";
    assert_eq!(
        replace::find_all(hay, &q("istanbul"), REGEX_GUARD)
            .unwrap()
            .len(),
        4
    );
    let mut c = q("istanbul");
    c.case = true;
    assert_eq!(replace::find_all(hay, &c, REGEX_GUARD).unwrap().len(), 1);
    let mut w = q("foo");
    w.whole_word = true;
    assert_eq!(replace::find_all(hay, &w, REGEX_GUARD).unwrap().len(), 1);
}

#[test]
fn regex_groups_names_and_errors() {
    let mut e = Editor::new("a1 b22 c333", "\n");
    let mut r = q(r"\d+");
    r.regex = true;
    assert_eq!(replace::replace_all(&mut e, &r, "#").unwrap(), 3);
    assert_eq!(e.text(), "a# b# c#");
    let mut e = Editor::new("John Smith", "\n");
    let mut r = q(r"(?P<first>\w+) (\w+)");
    r.regex = true;
    replace::replace_all(&mut e, &r, "$2, ${first}").unwrap();
    assert_eq!(e.text(), "Smith, John");
    let mut bad = q("(");
    bad.regex = true;
    assert!(matches!(
        replace::find_all("x", &bad, REGEX_GUARD),
        Err(FindError::BadRegex(_))
    ));
    // Turkish folding is plain-search only (ADR-0019): regex folds ASCII.
    let mut i = q("istanbul");
    i.regex = true;
    assert_eq!(
        replace::find_all("ISTANBUL İstanbul", &i, REGEX_GUARD)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn replace_next_replaces_selection_then_selects_next() {
    let mut e = Editor::new("foo foo foo", "\n");
    assert!(!replace::replace_next(&mut e, &q("foo"), "x").unwrap());
    assert_eq!(e.selection(), 0..3);
    assert!(replace::replace_next(&mut e, &q("foo"), "x").unwrap());
    assert_eq!(e.text(), "x foo foo");
    assert_eq!(e.selection(), 2..5);
}

#[test]
fn regex_guard_catastrophic_pattern_is_linear() {
    let hay = format!("{}b", "a".repeat(100 * 1024));
    let mut r = q("(a+)+$");
    r.regex = true;
    let t0 = Instant::now();
    let out = replace::find_all(&hay, &r, REGEX_GUARD);
    let ms = t0.elapsed();
    assert!(matches!(out, Ok(ref v) if v.is_empty()) || out == Err(FindError::TooSlow));
    assert!(ms < Duration::from_millis(50), "took {ms:?}");
}
