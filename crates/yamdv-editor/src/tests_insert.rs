//! Insert ▸ unit tests (prompt 016 §3 item 6).
#![allow(clippy::unwrap_used)]

use crate::Editor;
use crate::insert::{Block, block, footnote, table_text};

fn at(t: &str, pos: usize, eol: &'static str) -> Editor {
    let mut e = Editor::new(t, eol);
    e.move_to(pos, false);
    e
}

#[test]
fn blocks_land_on_own_lines_as_one_step() {
    let mut e = at("para one\nnext", 3, "\n");
    block(&mut e, Block::Code);
    assert_eq!(e.text(), "para one\n\n```\n\n```\n\nnext");
    assert_eq!(e.caret(), "para one\n\n```\n".len());
    assert!(e.undo());
    assert_eq!(e.text(), "para one\nnext");
    let mut e = at("a\r\n\r\n", 3, "\r\n");
    block(&mut e, Block::Callout(3));
    assert_eq!(e.text(), "a\r\n\r\n> [!WARNING]\r\n> \r\n");
    let mut e = at("x", 1, "\n");
    block(&mut e, Block::Rule);
    assert_eq!(e.text(), "x\n\n---\n");
    let mut e = at("", 0, "\n");
    block(&mut e, Block::Math);
    assert_eq!(e.text(), "$$\n\n$$\n");
    assert_eq!(e.caret(), 3);
    let (t, c) = table_text(3, 2, "\n");
    assert_eq!(t, "|   |   |\n| --- | --- |\n|   |   |\n|   |   |");
    assert_eq!(c, 2);
}

#[test]
fn footnote_adds_reference_and_definition() {
    let mut e = at("See this.[^1]\n\n[^1]: one\n", 8, "\n");
    footnote(&mut e);
    assert_eq!(e.text(), "See this[^2].[^1]\n\n[^1]: one\n\n[^2]: \n");
    assert_eq!(e.caret(), e.len() - 1);
    assert!(e.undo());
    assert_eq!(e.text(), "See this.[^1]\n\n[^1]: one\n");
}
