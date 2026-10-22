//! Table → CSV (R-EXP-04, S-EXP-04): UTF-8 with BOM, CRLF line endings, RFC 4180 quoting,
//! formatting stripped to plain text, formula-injection guard.

// Node indices come from the detail tree's own arena (in bounds by construction).
#![allow(clippy::indexing_slicing)]

use yamdv_core::doc::Document;
use yamdv_core::doc::detail::{Detail, NodeKind, RunKind};

/// One CSV field (RFC 4180): quoted when it contains `"`, `,`, CR or LF; with `guard`, a cell
/// starting with `=`, `+`, `-`, `@`, tab or CR is prefixed with `'` (S-EXP-04).
pub fn field(cell: &str, guard: bool, out: &mut String) {
    let mut v = String::with_capacity(cell.len() + 1);
    if guard && cell.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        v.push('\'');
    }
    v.push_str(cell);
    if v.contains(['"', ',', '\r', '\n']) {
        out.push('"');
        out.push_str(&v.replace('"', "\"\""));
        out.push('"');
    } else {
        out.push_str(&v);
    }
}

/// Rows of cells → CSV text (BOM + CRLF after every record).
pub fn to_csv(rows: &[Vec<String>], guard: bool) -> String {
    let mut out = String::from("\u{feff}");
    for r in rows {
        for (k, c) in r.iter().enumerate() {
            if k > 0 {
                out.push(',');
            }
            field(c, guard, &mut out);
        }
        out.push_str("\r\n");
    }
    out
}

fn cell_text(doc: &Document, d: &Detail, i: usize) -> String {
    let mut s = String::new();
    for r in d.runs_of(i) {
        match r.kind {
            RunKind::Text => s.push_str(r.text(&doc.source)),
            RunKind::SoftBreak => s.push(' '),
            RunKind::HardBreak => s.push('\n'),
        }
    }
    // Image chips "▣\u{2009}alt" → alt text.
    s.replace("\u{25A3}\u{2009}", "").trim().to_string()
}

/// Plain-text cells of table node `t` in detail tree `d`.
pub fn table_rows(doc: &Document, d: &Detail, t: usize) -> Vec<Vec<String>> {
    d.children(t)
        .filter(|&r| matches!(d.nodes[r].kind, NodeKind::TableRow { .. }))
        .map(|r| d.children(r).map(|c| cell_text(doc, d, c)).collect())
        .collect()
}

/// The table of top-level block `bi` containing source offset `at` (or its first table).
pub fn table_at(doc: &Document, bi: usize, at: u32) -> Option<Vec<Vec<String>>> {
    let d = doc.detail_uncached(bi);
    let tables: Vec<usize> = (0..d.nodes.len())
        .filter(|&i| matches!(d.nodes[i].kind, NodeKind::Table { .. }))
        .collect();
    let t = tables
        .iter()
        .copied()
        .find(|&i| d.nodes[i].range.contains(&at))
        .or_else(|| tables.first().copied())?;
    Some(table_rows(doc, &d, t))
}

#[cfg(test)]
mod tests {
    use super::*;
    use yamdv_core::doc::ReadOpts;

    #[test]
    fn r_exp_04_rfc4180_quoting_bom_crlf() {
        let rows = vec![
            vec!["a".to_string(), "b,c".to_string(), "say \"hi\"".to_string()],
            vec!["line\nbreak".to_string(), String::new(), "ü".to_string()],
        ];
        assert_eq!(
            to_csv(&rows, true),
            "\u{feff}a,\"b,c\",\"say \"\"hi\"\"\"\r\n\"line\nbreak\",,ü\r\n"
        );
        assert!(
            to_csv(&rows, true)
                .as_bytes()
                .starts_with(&[0xEF, 0xBB, 0xBF])
        );
    }

    #[test]
    fn s_exp_04_formula_guard() {
        let rows = vec![vec![
            "=SUM(A1)".to_string(),
            "+1".to_string(),
            "-2".to_string(),
            "@x".to_string(),
            "\tt".to_string(),
            "ok=1".to_string(),
        ]];
        assert_eq!(
            to_csv(&rows, true),
            "\u{feff}'=SUM(A1),'+1,'-2,'@x,'\tt,ok=1\r\n"
        );
        assert_eq!(
            to_csv(&rows, false),
            "\u{feff}=SUM(A1),+1,-2,@x,\tt,ok=1\r\n"
        );
        let mut s = String::new();
        field("=a,b", true, &mut s);
        assert_eq!(s, "\"'=a,b\"");
    }

    #[test]
    fn r_exp_04_table_cells_plain_text() {
        let doc = Document::new(
            "Intro\n\n| A | **B** |\n|---|---|\n| `x` | [l](https://e.com) |\n| =1 | a, b |\n"
                .into(),
            ReadOpts::default(),
        );
        let bi = (0..doc.index.blocks.len())
            .find(|&b| table_at(&doc, b, 0).is_some())
            .unwrap();
        let rows = table_at(&doc, bi, 0).unwrap();
        assert_eq!(
            rows,
            vec![vec!["A", "B"], vec!["x", "l"], vec!["=1", "a, b"]]
        );
        assert_eq!(
            to_csv(&rows, true),
            "\u{feff}A,B\r\nx,l\r\n'=1,\"a, b\"\r\n"
        );
    }
}
