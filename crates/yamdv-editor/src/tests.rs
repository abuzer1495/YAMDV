//! Unit tests and proptest invariants (prompt 006 §2.1).
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use crate::md::{self, LineFormat, Wrap};
use crate::{Editor, Motion};
use proptest::prelude::*;

fn ed(text: &str, caret: usize) -> Editor {
    let mut e = Editor::new(text, "\n");
    e.move_to(caret, false);
    e
}

#[test]
fn grapheme_moves_skip_clusters_and_crlf() {
    let mut e = Editor::new("a\u{301}b\r\nğ👍🏽", "\r\n");
    e.motion(Motion::Right, false);
    assert_eq!(e.caret(), 3); // a + combining acute
    e.motion(Motion::Right, false);
    e.motion(Motion::Right, false);
    assert_eq!(e.caret(), 6); // over CRLF in one step
    e.motion(Motion::Right, false);
    e.motion(Motion::Right, false);
    assert_eq!(e.caret(), e.len()); // emoji + skin tone is one grapheme
    e.motion(Motion::Left, false);
    assert_eq!(e.caret(), 8);
    e.motion(Motion::Left, false);
    e.motion(Motion::Left, false);
    assert_eq!(e.caret(), 4); // before CRLF
}

#[test]
fn word_moves_and_word_at() {
    let mut e = ed("hello brave  world\nnext", 0);
    e.motion(Motion::WordRight, false);
    assert_eq!(e.caret(), 6);
    e.motion(Motion::WordRight, false);
    assert_eq!(e.caret(), 13);
    e.motion(Motion::WordRight, false);
    assert_eq!(e.caret(), 18);
    e.motion(Motion::WordRight, false);
    assert_eq!(e.caret(), 19);
    e.motion(Motion::WordLeft, false);
    assert_eq!(e.caret(), 18);
    e.motion(Motion::WordLeft, false);
    assert_eq!(e.caret(), 13);
    assert_eq!(e.word_at(8), 6..11);
    assert_eq!(e.line_range_at(3), 0..19);
}

#[test]
fn shift_extends_and_home_end() {
    let mut e = ed("  indented line\nx", 5);
    e.motion(Motion::LineEnd, true);
    assert_eq!(e.selected_text(), "ented line");
    e.motion(Motion::LineStart, false);
    assert_eq!(e.caret(), 2);
    e.motion(Motion::LineStart, false);
    assert_eq!(e.caret(), 0);
    e.motion(Motion::DocEnd, false);
    assert_eq!(e.caret(), e.len());
}

#[test]
fn typing_coalesces_per_word_and_pause() {
    let mut e = ed("", 0);
    for (i, c) in "hello world".chars().enumerate() {
        e.type_text(&c.to_string(), i as u64 * 50);
    }
    assert_eq!(e.history().undo_len(), 2);
    e.type_text("!", 5000);
    assert_eq!(e.history().undo_len(), 3);
    e.undo();
    assert_eq!(e.text(), "hello world");
    e.undo();
    assert_eq!(e.text(), "hello ");
    assert_eq!(e.caret(), 6);
    e.redo();
    assert_eq!(e.text(), "hello world");
}

#[test]
fn undo_restores_selection() {
    let mut e = ed("abc def", 0);
    e.set_selection(4, 7);
    e.type_text("X", 0);
    assert_eq!(e.text(), "abc X");
    e.undo();
    assert_eq!(e.text(), "abc def");
    assert_eq!(e.selection(), 4..7);
}

#[test]
fn backspace_delete_cut_paste() {
    let mut e = Editor::new("ab\r\ncd", "\r\n");
    e.move_to(4, false);
    e.backspace(0);
    assert_eq!(e.text(), "abcd");
    e.paste("x\ny");
    assert_eq!(e.text(), "abx\r\nycd");
    e.set_selection(0, 2);
    assert_eq!(e.cut(), "ab");
    e.move_to(0, false);
    e.delete(0);
    assert_eq!(e.text(), "\r\nycd");
}

#[test]
fn history_cap_4_mib() {
    let mut e = ed("", 0);
    let chunk = "x".repeat(512 * 1024);
    for _ in 0..12 {
        e.paste(&chunk);
    }
    assert!(e.history().bytes() <= crate::history::CAP_BYTES);
    assert!(e.history().undo_len() < 12);
}

#[test]
fn enter_continues_lists_tasks_quotes() {
    let mut e = ed("- item", 6);
    md::enter(&mut e);
    assert_eq!(e.text(), "- item\n- ");
    md::enter(&mut e);
    assert_eq!(e.text(), "- item\n");
    let mut e = ed("3. three", 8);
    md::enter(&mut e);
    assert_eq!(e.text(), "3. three\n4. ");
    let mut e = ed("  - [x] done", 12);
    md::enter(&mut e);
    assert_eq!(e.text(), "  - [x] done\n  - [ ] ");
    let mut e = ed("> quoted", 8);
    md::enter(&mut e);
    assert_eq!(e.text(), "> quoted\n> ");
    let mut e = Editor::new("plain", "\r\n");
    e.move_to(5, false);
    md::enter(&mut e);
    assert_eq!(e.text(), "plain\r\n");
}

#[test]
fn tab_indents_list_items_else_spaces() {
    let mut e = ed("- a\n- b", 7);
    md::tab(&mut e, "  ", false);
    assert_eq!(e.text(), "- a\n  - b");
    assert_eq!(e.caret(), 9);
    md::tab(&mut e, "  ", true);
    assert_eq!(e.text(), "- a\n- b");
    let mut e = ed("text", 2);
    md::tab(&mut e, "  ", false);
    assert_eq!(e.text(), "te    xt");
}

#[test]
fn wrap_toggles_on_word_and_selection() {
    let mut e = ed("make bold here", 7);
    md::toggle_wrap(&mut e, Wrap::Bold);
    assert_eq!(e.text(), "make **bold** here");
    assert_eq!(e.caret(), 9);
    md::toggle_wrap(&mut e, Wrap::Bold);
    assert_eq!(e.text(), "make bold here");
    // Italic on bold text adds a marker instead of removing the bold.
    let mut e = ed("**w**", 2);
    md::toggle_wrap(&mut e, Wrap::Italic);
    assert_eq!(e.text(), "***w***");
    md::toggle_wrap(&mut e, Wrap::Italic);
    assert_eq!(e.text(), "**w**");
    let mut e = ed("abc", 0);
    e.set_selection(0, 3);
    md::toggle_wrap(&mut e, Wrap::Code);
    assert_eq!(e.text(), "`abc`");
    assert_eq!(e.selected_text(), "abc");
    e.undo();
    assert_eq!(e.text(), "abc");
    let mut e = ed("a  b", 2);
    md::toggle_wrap(&mut e, Wrap::Strike);
    assert_eq!(e.text(), "a ~~~~ b");
    assert_eq!(e.caret(), 4);
}

#[test]
fn link_selects_url() {
    let mut e = ed("see docs now", 5);
    md::link(&mut e);
    assert_eq!(e.text(), "see [docs](url) now");
    assert_eq!(e.selected_text(), "url");
}

#[test]
fn line_formats_toggle() {
    let mut e = ed("Title\nbody", 2);
    md::line_format(&mut e, LineFormat::Heading(2));
    assert_eq!(e.text(), "## Title\nbody");
    assert_eq!(e.caret(), 5);
    md::line_format(&mut e, LineFormat::Heading(2));
    assert_eq!(e.text(), "Title\nbody");
    md::line_format(&mut e, LineFormat::Heading(1));
    md::line_format(&mut e, LineFormat::Paragraph);
    assert_eq!(e.text(), "Title\nbody");
    let mut e = ed("a\nb", 0);
    e.set_selection(0, 3);
    md::line_format(&mut e, LineFormat::Numbered);
    assert_eq!(e.text(), "1. a\n2. b");
    md::line_format(&mut e, LineFormat::Bullet);
    assert_eq!(e.text(), "- a\n- b");
    md::line_format(&mut e, LineFormat::Bullet);
    assert_eq!(e.text(), "a\nb");
    md::line_format(&mut e, LineFormat::Task);
    assert_eq!(e.text(), "- [ ] a\n- [ ] b");
    md::line_format(&mut e, LineFormat::Quote);
    assert_eq!(e.text(), "> - [ ] a\n> - [ ] b");
    md::line_format(&mut e, LineFormat::Quote);
    assert_eq!(e.text(), "- [ ] a\n- [ ] b");
    md::toggle_task(&mut e);
    assert_eq!(e.text(), "- [x] a\n- [x] b");
    e.undo();
    assert_eq!(e.text(), "- [ ] a\n- [ ] b");
}

#[test]
fn checkbox_edit_is_one_byte() {
    assert_eq!(md::checkbox_edit("- [ ] a", 2), Some((3..4, "x")));
    assert_eq!(md::checkbox_edit("- [X] a", 2), Some((3..4, " ")));
    assert_eq!(md::checkbox_edit("- a", 2), None);
}

#[derive(Debug, Clone)]
enum Op {
    Type(usize, String),
    Delete(usize, usize),
    Paste(usize, String),
    Backspace(usize),
    Format(usize, u8),
    Enter(usize),
    Undo,
    Redo,
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (any::<usize>(), "[a-zğüşıöçİ \\-\\[\\]*>#\n]{1,3}").prop_map(|(p, s)| Op::Type(p, s)),
        (any::<usize>(), 0usize..6).prop_map(|(p, n)| Op::Delete(p, n)),
        (any::<usize>(), "[a-z\r\n🎉]{0,6}").prop_map(|(p, s)| Op::Paste(p, s)),
        any::<usize>().prop_map(Op::Backspace),
        (any::<usize>(), 0u8..12).prop_map(|(p, f)| Op::Format(p, f)),
        any::<usize>().prop_map(Op::Enter),
        Just(Op::Undo),
        Just(Op::Redo),
    ]
}

fn place(e: &mut Editor, p: usize) {
    let n = e.len().max(1);
    e.move_to(p % (n + 1), false);
}

fn run(e: &mut Editor, o: &Op, t: u64) {
    match o {
        Op::Type(p, s) => {
            place(e, *p);
            e.type_text(s, t);
        }
        Op::Delete(p, n) => {
            place(e, *p);
            let c = e.caret();
            e.move_to(c + n, true);
            e.delete(t);
        }
        Op::Paste(p, s) => {
            place(e, *p);
            e.paste(s);
        }
        Op::Backspace(p) => {
            place(e, *p);
            e.backspace(t);
        }
        Op::Format(p, f) => {
            place(e, *p);
            match f {
                0 => md::toggle_wrap(e, Wrap::Bold),
                1 => md::toggle_wrap(e, Wrap::Italic),
                2 => md::toggle_wrap(e, Wrap::Code),
                3 => md::link(e),
                4 => md::line_format(e, LineFormat::Heading(2)),
                5 => md::line_format(e, LineFormat::Quote),
                6 => md::line_format(e, LineFormat::Bullet),
                7 => md::line_format(e, LineFormat::Numbered),
                8 => md::line_format(e, LineFormat::Task),
                9 => md::tab(e, "  ", false),
                10 => md::tab(e, "  ", true),
                _ => md::toggle_task(e),
            }
        }
        Op::Enter(p) => {
            place(e, *p);
            md::enter(e);
        }
        Op::Undo => {
            e.undo();
        }
        Op::Redo => {
            e.redo();
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 400, ..ProptestConfig::default() })]

    /// Random edit / undo / redo sequences: undoing everything restores the exact original
    /// bytes, and redoing everything restores the exact bytes of the newest state.
    #[test]
    fn undo_redo_restore_exact_bytes(init in "[a-z \n\\-#>]{0,40}", ops in prop::collection::vec(op(), 1..40), fast in any::<bool>()) {
        let mut e = Editor::new(&init, "\n");
        let mut t = 0u64;
        for o in &ops {
            // `fast` = 10 ms apart (coalescing active), else 2 s apart (no coalescing).
            t += if fast { 10 } else { 2000 };
            run(&mut e, o, t);
            prop_assert!(e.text().is_char_boundary(e.caret()));
        }
        while e.redo() {}
        let fin = e.text();
        while e.undo() {}
        prop_assert_eq!(e.text(), init);
        while e.redo() {}
        prop_assert_eq!(e.text(), fin);
    }

    /// Each single undo after an edit restores the bytes from before that edit (no coalescing).
    #[test]
    fn single_undo_restores_previous_state(init in "[a-z \n]{0,30}", ops in prop::collection::vec(op(), 1..25)) {
        let mut e = Editor::new(&init, "\n");
        let mut t = 0u64;
        for o in ops.iter().filter(|o| !matches!(o, Op::Undo | Op::Redo)) {
            t += 2000;
            let before = e.text();
            let v = e.version;
            run(&mut e, o, t);
            if e.version != v {
                let after = e.text();
                e.undo();
                prop_assert_eq!(e.text(), before);
                e.redo();
                prop_assert_eq!(e.text(), after);
            }
        }
    }
}

/// AC-3 (prompt 006 §2.2): decode → edit one word through the editor → encode. Every byte
/// outside the edited word is identical (CRLF, BOM, windows-1254, final-newline state kept).
#[test]
fn byte_identity_outside_the_edit() {
    use yamdv_core::{decode, encode};
    let body = "# Başlık\r\n\r\nÇağrı ışığı göründü; şu öğle güneşi İzmir'de.\r\nİkinci satır: ğüşıöçİ.\r\n";
    let (w1254, _, _) = encoding_rs::WINDOWS_1254.encode(body);
    let mut utf8_bom = b"\xEF\xBB\xBF".to_vec();
    utf8_bom.extend_from_slice(body.as_bytes());
    let mut utf16 = b"\xFF\xFE".to_vec();
    for u in body.encode_utf16() {
        utf16.extend_from_slice(&u.to_le_bytes());
    }
    for (name, bytes) in [
        ("windows-1254", w1254.into_owned()),
        ("UTF-8", utf8_bom),
        ("UTF-16LE", utf16),
    ] {
        let d = decode::decode(&bytes);
        assert_eq!(d.encoding, name);
        assert!(!d.lossy);
        let mut e = Editor::new(&d.text, encode::dominant_eol(d.line_endings));
        assert_eq!(e.eol, "\r\n");
        let at = d.text.find("güneşi").unwrap_or(0);
        e.move_to(at + 2, false);
        let w = e.word_at(e.caret());
        e.set_selection(w.start, w.end);
        e.type_text("ayışığı", 0);
        md::enter(&mut e); // a new line uses the dominant ending
        let out = encode::encode(&e.text(), d.encoding, d.had_bom).unwrap();
        let prefix = bytes.iter().zip(&out).take_while(|(a, b)| a == b).count();
        let suffix = bytes
            .iter()
            .rev()
            .zip(out.iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        let orig_word = encode::encode("güneşi", d.encoding, false).unwrap();
        // Everything before and after the edited word is byte-identical.
        assert!(prefix + suffix + orig_word.len() >= bytes.len(), "{name}");
        assert!(
            out.windows(4)
                .filter(|w| *w == b"\r\n\r\n" || *w == b"\r\0\n\0")
                .count()
                >= 1
        );
        let again = decode::decode(&out);
        assert!(again.text.contains("ayışığı\r\n"), "{name}");
    }
}
