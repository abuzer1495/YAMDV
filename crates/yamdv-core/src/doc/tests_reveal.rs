//! Reveal model unit tests (prompt 016: source-map round trips, reveal set).
#![allow(clippy::unwrap_used)]

use super::*;

fn para(t: &str, caret: u32) -> RevealPara {
    let a = analyze(t, ReadOpts::default()).unwrap();
    let s = reveal_set(&a, caret..caret);
    build(&a, t, &s)
}

/// Every verbatim layout position maps to a source offset and back.
fn round_trip(p: &RevealPara, t: &str) {
    for s in &p.text.segs {
        if !s.verbatim {
            continue;
        }
        let seg_text = p.text.slice16(s.u16_start, s.u16_start + s.u16_len);
        let mut k = 0u32;
        for c in seg_text.chars() {
            let src = p.text.to_source(s.u16_start + k);
            assert!(t.is_char_boundary(src as usize), "{src} in {t:?}");
            assert_eq!(p.text.from_source(src), s.u16_start + k, "{t:?} at {src}");
            k += u32::try_from(c.len_utf16()).unwrap();
        }
    }
}

#[test]
fn bold_reveals_only_near_caret() {
    let t = "a **bold** and *it* `code` [link](u)";
    let p = para(t, 6); // inside "bold"
    assert_eq!(p.shown, vec!["**", "**"]);
    assert!(p.text.text.starts_with("a **bold** and it code link"));
    let p = para(t, 0);
    assert!(p.shown.is_empty());
    assert_eq!(p.text.text, "a bold and it code link");
    // Adjacent counts: caret right before the opening marker.
    assert_eq!(para(t, 2).shown, vec!["**", "**"]);
    // Link: both pieces, code: both backticks.
    assert_eq!(para(t, 30).shown, vec!["[", "](u)"]);
    assert_eq!(para(t, 22).shown, vec!["`", "`"]);
    round_trip(&para(t, 6), t);
    round_trip(&para(t, 0), t);
}

#[test]
fn hidden_marker_positions_never_resolve_inside() {
    let t = "a **bold** b";
    let a = analyze(t, ReadOpts::default()).unwrap();
    let s = reveal_set(&a, 0..0);
    assert_eq!(snap(&a, &s, 3, true), 4);
    assert_eq!(snap(&a, &s, 3, false), 2);
    let p = build(&a, t, &s);
    assert_eq!(p.text.from_source(3), p.text.from_source(4));
    // Revealed: the caret may enter the marker.
    let s = reveal_set(&a, 3..3);
    assert_eq!(snap(&a, &s, 3, true), 3);
}

#[test]
fn line_markers_follow_the_caret_line() {
    let t = "## Head *x*";
    let p = para(t, 5);
    assert_eq!(p.shown, vec!["## "]);
    assert_eq!(p.text.text, "## Head x");
    let a = analyze(t, ReadOpts::default()).unwrap();
    assert_eq!(a.heading, 2);
    let t = "- one\n- [ ] task\n  more";
    let a = analyze("- one\n- [ ] task", ReadOpts::default()).unwrap();
    let p = build(&a, "- one\n- [ ] task", &reveal_set(&a, 2..2));
    assert_eq!(p.text.text, "- one\n☐ task");
    assert!(analyze(t, ReadOpts::default()).is_some());
    let t = "> [!NOTE]\n> body";
    let a = analyze(t, ReadOpts::default()).unwrap();
    assert!(a.quote);
    let p = build(t_or(t), t, &reveal_set(&a, 13..13));
    assert_eq!(p.text.text, "ⓘ Note\n> body");
    fn t_or(t: &str) -> &Analysis {
        Box::leak(Box::new(analyze(t, ReadOpts::default()).unwrap()))
    }
}

#[test]
fn escapes_entities_and_clusters() {
    let t = "x &amp; \\* 日本 👍🏽 :smile:";
    let p = para(t, 0);
    assert_eq!(p.text.text, "x & * 日本 👍🏽 😄");
    round_trip(&p, t);
    // Caret touching the entity shows its source.
    assert_eq!(para(t, 2).shown, vec!["&amp;"]);
    // The escape backslash is a marker of its own.
    assert_eq!(para(t, 8).shown, vec!["\\"]);
    // CJK / emoji map character-exactly (clusters are whole verbatim text).
    let p = para(t, 0);
    let pos = u32::try_from(t.find('本').unwrap()).unwrap();
    let l = p.text.from_source(pos);
    assert_eq!(p.text.to_source(l), pos);
}

#[test]
fn nested_and_selection_ends() {
    let t = "a ***bi*** and **b *i* b** z";
    let p = para(t, 6);
    assert_eq!(p.shown, vec!["***", "***"]);
    let a = analyze(t, ReadOpts::default()).unwrap();
    // A selection from the first construct into the second reveals both.
    let p = build(&a, t, &reveal_set(&a, 5..20));
    assert_eq!(p.shown.len(), 6);
    round_trip(&p, t);
}

#[test]
fn block_level_kinds_are_not_inline() {
    for t in ["```\ncode\n```", "$$\nx\n$$", "<div>x</div>", "---\na: 1\n---", "a\n\nb", "| a |\n|---|\n| 1 |"] {
        assert!(analyze(t, ReadOpts::default()).is_none(), "{t:?}");
    }
    assert!(analyze("", ReadOpts::default()).is_some());
}
