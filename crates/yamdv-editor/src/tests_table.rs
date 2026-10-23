//! Table model and TSV/CSV unit tests (prompt 016 §3 Part 2, R-TBL-01…08).
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use crate::Editor;
use crate::insert::{Block, block};
use crate::table::{self, Align, display_width, escape_cell, natural_cmp, unescape_cell};
use crate::tsv;

const T: &str = "| Ad | Not |\n|:--|--:|\n| Çiğdem | 2 |\n| ali | 10 |\n| Ilgın | 1 |";

fn rows(t: &table::Table) -> Vec<String> {
    t.rows.iter().map(|r| r.join(",")).collect()
}

#[test]
fn round_trip_alignment_and_cells() {
    let t = table::parse(T).unwrap();
    assert_eq!(t.aligns, vec![Align::Left, Align::Right]);
    assert_eq!(rows(&t), vec!["Ad,Not", "Çiğdem,2", "ali,10", "Ilgın,1"]);
    // Pretty output parses back to the same table and is stable.
    let md = t.to_markdown("\n", true);
    assert_eq!(table::parse(&md).unwrap(), t);
    assert_eq!(table::parse(&md).unwrap().to_markdown("\n", true), md);
    assert!(
        md.starts_with("| Ad     | Not |\n| :----- | --: |\n| Çiğdem |   2 |"),
        "{md}"
    );
    // CRLF keeps CRLF; compact form.
    let c = t.to_markdown("\r\n", false);
    assert!(c.contains("| Ad | Not |\r\n| :-- | --: |\r\n"), "{c}");
    // Grid ranges: content of an empty cell is where typing goes.
    let g = table::grid("| a |  |\n|---|---|\n|| x |").unwrap();
    assert_eq!(g.content(0, 1), Some(6..6));
    assert_eq!(g.content(1, 0), Some(20..20));
    assert_eq!(g.cell_at(2), Some((0, 0)));
    assert_eq!(g.cell_at(22), Some((1, 1)));
    assert!(table::grid("just text\nmore").is_none());
    assert!(table::grid("| a |\n| b |").is_none(), "no delimiter row");
}

#[test]
fn escaping_and_east_asian_pretty_align() {
    let t = table::parse("| a \\| b | c |\n|---|---|\n| 漢字 | x |").unwrap();
    assert_eq!(t.rows[0][0], "a \\| b");
    assert_eq!(unescape_cell(&t.rows[0][0]), "a | b");
    assert_eq!(escape_cell("x|y\nz"), "x\\|y<br>z");
    assert_eq!(escape_cell("x\\|y"), "x\\|y", "already escaped");
    assert_eq!(display_width("漢字"), 4);
    assert_eq!(display_width("e\u{301}"), 1);
    let md = t.to_markdown("\n", true);
    let lines: Vec<&str> = md.lines().collect();
    let w: Vec<usize> = lines.iter().map(|l| display_width(l)).collect();
    assert!(w.iter().all(|&x| x == w[0]), "{md}");
}

#[test]
fn structure_edits_sort_and_move() {
    let mut t = table::parse(T).unwrap();
    t.sort(0, false);
    assert_eq!(
        rows(&t)[1..],
        ["ali,10", "Çiğdem,2", "Ilgın,1"],
        "Turkish: c < ç < ı"
    );
    t.sort(1, true);
    assert_eq!(
        rows(&t)[1..],
        ["ali,10", "Çiğdem,2", "Ilgın,1"],
        "natural numbers"
    );
    t.sort(1, false);
    assert_eq!(rows(&t)[1..], ["Ilgın,1", "Çiğdem,2", "ali,10"]);
    assert!(t.move_row(1, 3));
    assert_eq!(rows(&t)[3], "Ilgın,1");
    assert!(!t.move_row(0, 1), "the header stays");
    assert!(t.move_col(0, 1));
    assert_eq!(rows(&t)[0], "Not,Ad");
    assert_eq!(t.aligns, vec![Align::Right, Align::Left]);
    t.insert_col(2);
    t.insert_row(1);
    assert_eq!(rows(&t)[1], ",,");
    t.set_align(2, Align::Center);
    assert!(t.delete_row(1) && !t.delete_row(0));
    assert!(t.delete_col(2));
    assert_eq!(t.cols(), 2);
    assert_eq!(natural_cmp("a2", "a10"), std::cmp::Ordering::Less);
    assert_eq!(natural_cmp("ılık", "inek"), std::cmp::Ordering::Less);
    assert_eq!(natural_cmp("Şeker", "su"), std::cmp::Ordering::Greater);
}

#[test]
fn adjacent_table_insert_keeps_a_blank_line() {
    let src = "| a |\n|---|\n| 1 |\n\nnext";
    let mut e = Editor::new(src, "\n");
    e.move_to(2, false);
    block(&mut e, Block::Table(2, 2));
    let t = e.text();
    assert!(t.starts_with("| a |\n|---|\n| 1 |\n\n|"), "{t}");
    assert!(t.ends_with("|\n\nnext"), "{t}");
}

#[test]
fn tsv_and_csv_rfc4180() {
    let (r, d) = tsv::detect("a\tb\r\n1\t\"x\ty\"\r\n").unwrap();
    assert_eq!(d, '\t');
    assert_eq!(r, vec![vec!["a", "b"], vec!["1", "x\ty"]]);
    let (r, _) = tsv::detect("n,q\n\"Smith, J\",\"say \"\"hi\"\"\"\n\"two\nlines\",3\n").unwrap();
    assert_eq!(r[1], vec!["Smith, J", "say \"hi\""]);
    assert_eq!(r[2], vec!["two\nlines", "3"]);
    assert!(
        tsv::detect("hello, world").is_none(),
        "one CSV line is prose"
    );
    assert!(tsv::detect("plain text").is_none());
    let t = table::from_rows(&r).unwrap();
    assert_eq!(t.rows[2][0], "two<br>lines");
}
