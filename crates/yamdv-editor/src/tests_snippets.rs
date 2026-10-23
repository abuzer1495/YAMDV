//! Snippet unit tests (prompt 015 §3.3).
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use crate::Editor;
use crate::snippets::{self, expand};

const NOW: (&str, &str) = ("2026-10-23", "20:30");

fn at_end(t: &str) -> Editor {
    let mut e = Editor::new(t, "\n");
    e.move_to(e.len(), false);
    e
}

#[test]
fn table_rxc_expands_and_tab_walks_cells() {
    let mut e = at_end("table2x3");
    let mut s = snippets::try_expand(&mut e, NOW).unwrap();
    assert_eq!(
        e.text(),
        "| Column 1 | Column 2 | Column 3 |\n|---|---|---|\n|  |  |  |\n|  |  |  |"
    );
    assert_eq!(e.selected_text(), "Column 1");
    e.type_text("Name", 0);
    assert!(s.next(&mut e));
    assert_eq!(e.selected_text(), "Column 2");
    assert!(e.text().starts_with("| Name | Column 2 |"));
    // One undo step per expansion (typing is its own step).
    e.undo();
    e.undo();
    assert_eq!(e.text(), "table2x3");
}

#[test]
fn triggers_need_the_whole_word() {
    assert!(snippets::try_expand(&mut at_end("xtable"), NOW).is_none());
    assert!(snippets::try_expand(&mut at_end("a.h2"), NOW).is_none());
    let mut e = Editor::new("h2x", "\n");
    e.move_to(2, false);
    assert!(snippets::try_expand(&mut e, NOW).is_none());
    assert!(expand("table0x2", "\n", "", NOW).is_none());
    assert!(expand("table21x2", "\n", "", NOW).is_none());
    assert!(expand("h7", "\n", "", NOW).is_none());
    assert!(expand("hello", "\n", "", NOW).is_none());
}

#[test]
fn languages_callouts_and_blocks() {
    let mut e = at_end("  python");
    snippets::try_expand(&mut e, NOW);
    assert_eq!(e.text(), "  ```python\n  \n  ```");
    assert_eq!(
        expand("red", "\n", "", NOW).unwrap().text,
        "> [!CAUTION]\n> Text"
    );
    assert_eq!(
        expand("note", "\n", "", NOW).unwrap().text,
        "> [!NOTE]\n> Text"
    );
    assert_eq!(expand("h3", "\n", "", NOW).unwrap().text, "### Heading");
    assert_eq!(expand("date", "\n", "", NOW).unwrap().text, "2026-10-23");
    assert_eq!(expand("time", "\n", "", NOW).unwrap().text, "20:30");
    assert_eq!(expand("hr", "\r\n", "", NOW).unwrap().text, "---\r\n");
    assert!(
        expand("seq", "\r\n", "", NOW)
            .unwrap()
            .text
            .contains("sequenceDiagram\r\n  A->>B")
    );
    for w in [
        "table",
        "code",
        "link",
        "img",
        "line",
        "quote",
        "task",
        "tip",
        "important",
        "warning",
        "caution",
        "math",
        "mermaid",
        "flow",
        "gantt",
        "details",
        "footnote",
        "h1",
        "h6",
        "rust",
    ] {
        assert!(expand(w, "\n", "", NOW).is_some(), "{w}");
    }
}

#[test]
fn link_placeholders_and_leaving() {
    let mut e = at_end("see link");
    let mut s = snippets::try_expand(&mut e, NOW).unwrap();
    assert_eq!(e.text(), "see [text](https://)");
    assert_eq!(e.selected_text(), "text");
    e.type_text("YAMDV", 0);
    let mut away = s.clone();
    assert!(s.next(&mut e));
    assert_eq!(e.selected_text(), "https://");
    assert!(!s.next(&mut e)); // last stop: caret after ")"
    assert_eq!(e.caret(), e.len());
    // The caret left the placeholder (click elsewhere): Tab ends the session.
    e.move_to(0, false);
    assert!(!away.next(&mut e));
}

#[test]
fn insert_table_and_code_block() {
    let mut ed = Editor::new("text", "\n");
    ed.set_selection(4, 4);
    let s = snippets::insert(&mut ed, "table", ("2026-01-01", "10:00"));
    assert!(ed.text().starts_with("text\n| "));
    assert!(s.is_some());
    let mut ed = Editor::new("a = 1", "\n");
    ed.set_selection(0, 5);
    assert!(snippets::insert(&mut ed, "code", ("", "")).is_none());
    assert_eq!(ed.text(), "```\na = 1\n```");
    assert_eq!(ed.selection(), 3..3);
    ed.undo();
    assert_eq!(ed.text(), "a = 1");
}
