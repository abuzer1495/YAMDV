//! AC-6/AC-7 (pure part): `large-1mb.md` loads, indexes and copies (Ctrl+A + Ctrl+C) quickly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Instant;
use yamdv_core::doc::{Document, ReadOpts, text};

fn large() -> String {
    // Same construction as `cargo xtask gen-large`: repeat the typical sample to ≥ 1 MiB.
    let base = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/corpus/002/typical-basic.md"
    ))
    .unwrap();
    let mut s = String::new();
    let mut part = 0;
    while s.len() < 1024 * 1024 {
        part += 1;
        for line in base.lines() {
            if let Some(rest) = line.strip_prefix("# ") {
                s.push_str(&format!("# Part {part}: {rest}\n"));
            } else {
                s.push_str(line);
                s.push('\n');
            }
        }
        s.push('\n');
    }
    s
}

#[test]
fn select_all_copy_large() {
    let src = large();
    let t = Instant::now();
    let doc = Document::new(src, ReadOpts::default());
    let load = t.elapsed();
    let t = Instant::now();
    let all = text::all_text(&doc);
    let copy = t.elapsed();
    eprintln!(
        "large-1mb: {} blocks, index {load:?}, Ctrl+A+Ctrl+C text {copy:?} ({} chars)",
        doc.index.blocks.len(),
        all.len()
    );
    assert!(all.len() > 500_000);
    assert!(all.contains("\r\n\r\n"));
    // 300 ms budget (AC-7) holds for optimized builds; debug builds get a generous bound.
    let limit = if cfg!(debug_assertions) { 5_000 } else { 300 };
    assert!(copy.as_millis() < limit, "copy took {copy:?}");
}
