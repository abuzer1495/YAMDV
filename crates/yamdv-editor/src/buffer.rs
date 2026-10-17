//! The shared editor component's state (prompt 006 §2.1): a rope buffer, one caret with an
//! anchor (selection), grapheme-aware movement, Unicode word bounds and undo/redo.
//! Offsets are UTF-8 byte offsets into the buffer and always sit on grapheme boundaries.

// Slices are taken at offsets of ASCII markers or at offsets computed from the same string, so
// they sit on char boundaries; positions are clamped to the buffer length; isize arithmetic is
// on byte offsets of buffers far below isize::MAX (documents ≤ 256 MiB, S-PARSE guard).
#![allow(
    clippy::indexing_slicing,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use std::ops::Range;

use ropey::Rope;
use unicode_segmentation::UnicodeSegmentation;

use crate::history::{Change, History, Kind};

/// Logical caret motions (vertical and wrapped-line motions need layout and come from the
/// platform as absolute positions through [`Editor::move_to`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    /// One grapheme left.
    Left,
    /// One grapheme right.
    Right,
    /// Start of the previous word (Ctrl+←).
    WordLeft,
    /// Start of the next word (Ctrl+→).
    WordRight,
    /// Start of the logical line.
    LineStart,
    /// End of the logical line (before the line break).
    LineEnd,
    /// Start of the buffer (Ctrl+Home).
    DocStart,
    /// End of the buffer (Ctrl+End).
    DocEnd,
}

/// Editor state shared by F2 and F3 for one document.
#[derive(Debug, Clone)]
pub struct Editor {
    rope: Rope,
    caret: usize,
    anchor: usize,
    hist: History,
    /// Line break inserted by Enter (the document's dominant ending, S-FILE-10).
    pub eol: &'static str,
    /// Incremented on every buffer change.
    pub version: u64,
    saved_version: u64,
}

impl Editor {
    /// New editor over `text`; `eol` is `"\n"` or `"\r\n"`.
    pub fn new(text: &str, eol: &'static str) -> Self {
        Self {
            rope: Rope::from_str(text),
            caret: 0,
            anchor: 0,
            hist: History::default(),
            eol,
            version: 0,
            saved_version: 0,
        }
    }

    /// Whole buffer as a string.
    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    /// The rope (read-only).
    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    /// Buffer length in bytes.
    pub fn len(&self) -> usize {
        self.rope.len_bytes()
    }

    /// Empty buffer.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Caret byte offset.
    pub fn caret(&self) -> usize {
        self.caret
    }

    /// Anchor byte offset (equals the caret without a selection).
    pub fn anchor(&self) -> usize {
        self.anchor
    }

    /// Selected byte range (may be empty).
    pub fn selection(&self) -> Range<usize> {
        self.caret.min(self.anchor)..self.caret.max(self.anchor)
    }

    /// Text in `r` (clamped).
    pub fn slice(&self, r: Range<usize>) -> String {
        let n = self.len();
        let (a, b) = (self.snap(r.start.min(n)), self.snap(r.end.min(n)));
        if a >= b {
            return String::new();
        }
        self.rope.byte_slice(a..b).to_string()
    }

    /// Selected text.
    pub fn selected_text(&self) -> String {
        self.slice(self.selection())
    }

    /// Changes not yet saved.
    pub fn is_dirty(&self) -> bool {
        self.version != self.saved_version
    }

    /// Marks the current version as saved.
    pub fn mark_saved(&mut self) {
        self.saved_version = self.version;
    }

    /// History (for tests and the 4 MiB cap check).
    pub fn history(&self) -> &History {
        &self.hist
    }

    /// Closes the open undo group (mode switch, caret moved by the mouse, …).
    pub fn seal(&mut self) {
        self.hist.seal();
    }

    /// Sets anchor and caret (snapped to char boundaries, clamped).
    pub fn set_selection(&mut self, anchor: usize, caret: usize) {
        self.anchor = self.snap(anchor);
        self.caret = self.snap(caret);
    }

    /// Moves the caret to `pos`; `extend` keeps the anchor (Shift).
    pub fn move_to(&mut self, pos: usize, extend: bool) {
        self.caret = self.snap(pos);
        if !extend {
            self.anchor = self.caret;
        }
        self.hist.seal();
    }

    fn snap(&self, pos: usize) -> usize {
        let mut p = pos.min(self.len());
        // Back to a char boundary (byte_to_char floors).
        p = self.rope.char_to_byte(self.rope.byte_to_char(p));
        p
    }

    /// Line (0-based) of a byte offset.
    pub fn line_of(&self, pos: usize) -> usize {
        self.rope.byte_to_line(pos.min(self.len()))
    }

    /// Byte offset of the start of line `l` (clamped).
    pub fn line_start(&self, l: usize) -> usize {
        let l = l.min(self.rope.len_lines().saturating_sub(1));
        self.rope.line_to_byte(l)
    }

    /// Byte offset of the end of line `l`, before `\n` / `\r\n`.
    pub fn line_end(&self, l: usize) -> usize {
        let start = self.line_start(l);
        let text = self.line_text(l);
        start + text.trim_end_matches(['\n', '\r']).len()
    }

    /// Text of line `l` including its line break.
    pub fn line_text(&self, l: usize) -> String {
        if l >= self.rope.len_lines() {
            return String::new();
        }
        self.rope.line(l).to_string()
    }

    /// Number of lines.
    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    /// Grapheme boundary before `pos` (line breaks are graphemes; CRLF is one).
    pub fn prev_grapheme(&self, pos: usize) -> usize {
        if pos == 0 {
            return 0;
        }
        let l = self.line_of(pos);
        let ls = self.line_start(l);
        if pos == ls {
            // Before the previous line's break.
            let pl = l - 1;
            return self.line_end(pl);
        }
        let t = self.line_text(l);
        let off = pos - ls;
        let mut prev = 0;
        for (i, _) in t.grapheme_indices(true) {
            if i >= off {
                break;
            }
            prev = i;
        }
        ls + prev
    }

    /// Grapheme boundary after `pos`.
    pub fn next_grapheme(&self, pos: usize) -> usize {
        let n = self.len();
        if pos >= n {
            return n;
        }
        let l = self.line_of(pos);
        let ls = self.line_start(l);
        let t = self.line_text(l);
        let off = pos - ls;
        for (i, g) in t.grapheme_indices(true) {
            if i + g.len() > off {
                return ls + i + g.len();
            }
        }
        n
    }

    /// Unicode word range containing `pos` (double-click), within its line.
    pub fn word_at(&self, pos: usize) -> Range<usize> {
        let l = self.line_of(pos);
        let ls = self.line_start(l);
        let t = self.line_text(l);
        let body = t.trim_end_matches(['\n', '\r']);
        let off = (pos - ls).min(body.len());
        for (i, w) in body.split_word_bound_indices() {
            if off >= i && off < i + w.len() {
                return ls + i..ls + i + w.len();
            }
        }
        // At the end of the line: the word before it.
        if let Some((i, w)) = body.split_word_bound_indices().next_back()
            && i + w.len() == off
            && w.chars().any(char::is_alphanumeric)
        {
            return ls + i..ls + off;
        }
        pos..pos
    }

    /// Whole logical line of `pos` including its break (triple-click).
    pub fn line_range_at(&self, pos: usize) -> Range<usize> {
        let l = self.line_of(pos);
        let s = self.line_start(l);
        s..s + self.line_text(l).len()
    }

    fn word_right(&self, pos: usize) -> usize {
        let l = self.line_of(pos);
        let ls = self.line_start(l);
        let le = self.line_end(l);
        if pos >= le {
            // Next line start.
            return if l + 1 < self.line_count() {
                self.line_start(l + 1)
            } else {
                self.len()
            };
        }
        let t = self.line_text(l);
        let body = &t[..le - ls];
        let off = pos - ls;
        let mut seen_current = false;
        for (i, w) in body.split_word_bound_indices() {
            if i + w.len() <= off {
                continue;
            }
            if !seen_current {
                seen_current = true;
                continue;
            }
            if !w.chars().all(char::is_whitespace) {
                return ls + i;
            }
        }
        le
    }

    fn word_left(&self, pos: usize) -> usize {
        let l = self.line_of(pos);
        let ls = self.line_start(l);
        if pos <= ls {
            return if l > 0 { self.line_end(l - 1) } else { 0 };
        }
        let t = self.line_text(l);
        let off = pos - ls;
        let mut best = 0;
        for (i, w) in t.split_word_bound_indices() {
            if i >= off {
                break;
            }
            if !w.chars().all(char::is_whitespace) {
                best = i;
            }
        }
        ls + best
    }

    /// Applies a logical motion.
    pub fn motion(&mut self, m: Motion, extend: bool) {
        let sel = self.selection();
        let p = self.caret;
        let target = match m {
            // Without Shift, ←/→ collapse a selection to its edge.
            Motion::Left if !extend && !sel.is_empty() => sel.start,
            Motion::Right if !extend && !sel.is_empty() => sel.end,
            Motion::Left => self.prev_grapheme(p),
            Motion::Right => self.next_grapheme(p),
            Motion::WordLeft => self.word_left(p),
            Motion::WordRight => self.word_right(p),
            Motion::LineStart => {
                // Smart home: first non-blank, then column 0.
                let l = self.line_of(p);
                let ls = self.line_start(l);
                let t = self.line_text(l);
                let ind = t.len() - t.trim_start_matches([' ', '\t']).len();
                if p == ls + ind {
                    ls
                } else {
                    ls + ind.min(self.line_end(l) - ls)
                }
            }
            Motion::LineEnd => self.line_end(self.line_of(p)),
            Motion::DocStart => 0,
            Motion::DocEnd => self.len(),
        };
        self.move_to(target, extend);
    }

    // ---- editing ---------------------------------------------------------------------------

    fn apply(&mut self, c: &Change) {
        let a = self.rope.byte_to_char(c.pos);
        if !c.del.is_empty() {
            let b = self.rope.byte_to_char(c.pos + c.del.len());
            self.rope.remove(a..b);
        }
        if !c.ins.is_empty() {
            self.rope.insert(a, &c.ins);
        }
        self.version += 1;
    }

    fn unapply(&mut self, c: &Change) {
        let inv = Change {
            pos: c.pos,
            del: c.ins.clone(),
            ins: c.del.clone(),
        };
        self.apply(&inv);
    }

    /// Replaces `r` by `text` as one undo step (or part of the open one when `join`), then sets
    /// the selection.
    fn edit(
        &mut self,
        r: Range<usize>,
        text: &str,
        kind: Kind,
        now_ms: u64,
        sel_after: (usize, usize),
    ) {
        let r = self.snap(r.start)..self.snap(r.end.max(r.start));
        let c = Change {
            pos: r.start,
            del: self.slice(r.clone()),
            ins: text.to_string(),
        };
        if c.del.is_empty() && c.ins.is_empty() {
            return;
        }
        let before = (self.anchor, self.caret);
        self.apply(&c);
        self.anchor = sel_after.0.min(self.len());
        self.caret = sel_after.1.min(self.len());
        self.hist
            .record(c, kind, now_ms, before, (self.anchor, self.caret));
    }

    /// Replaces `r` by `text` as one non-coalescing undo step; the caret goes to `caret`
    /// (anchor = caret).
    pub fn replace(&mut self, r: Range<usize>, text: &str, caret: usize) {
        self.edit(r, text, Kind::Other, 0, (caret, caret));
    }

    /// Like [`Editor::replace`] with an explicit selection afterwards.
    pub fn replace_select(&mut self, r: Range<usize>, text: &str, sel: (usize, usize)) {
        self.edit(r, text, Kind::Other, 0, sel);
    }

    /// Several replacements as ONE undo step. `edits` are in buffer order, non-overlapping, in
    /// the coordinates before any of them; the selection is set afterwards.
    pub fn replace_many(&mut self, edits: &[(Range<usize>, String)], sel: (usize, usize)) {
        let before = (self.anchor, self.caret);
        let mut shift: isize = 0;
        let mut first = true;
        for (r, t) in edits {
            let s = (r.start as isize + shift) as usize;
            let e = (r.end as isize + shift) as usize;
            let c = Change {
                pos: s,
                del: self.slice(s..e),
                ins: t.clone(),
            };
            shift += t.len() as isize - (e - s) as isize;
            if c.del.is_empty() && c.ins.is_empty() {
                continue;
            }
            self.apply(&c);
            if first {
                self.hist.record(c, Kind::Other, 0, before, sel);
                first = false;
            } else {
                self.hist.extend_last(c, sel);
            }
        }
        if !first {
            self.hist.seal();
            self.anchor = sel.0.min(self.len());
            self.caret = sel.1.min(self.len());
        }
    }

    /// Types `s` (WM_CHAR text) over the selection; coalesced per word / 1 s pause.
    pub fn type_text(&mut self, s: &str, now_ms: u64) {
        let r = self.selection();
        let kind = if r.is_empty() {
            Kind::Typing
        } else {
            Kind::Other
        };
        let end = r.start + s.len();
        self.edit(r, s, kind, now_ms, (end, end));
    }

    /// Pastes plain text: line breaks normalised to the document's ending (S-FILE-10 keeps
    /// the file's dominant ending for new lines).
    pub fn paste(&mut self, s: &str) {
        let norm = normalize_eol(s, self.eol);
        let r = self.selection();
        let end = r.start + norm.len();
        self.edit(r, &norm, Kind::Other, 0, (end, end));
        self.hist.seal();
    }

    /// Backspace: deletes the selection or the grapheme before the caret.
    pub fn backspace(&mut self, now_ms: u64) {
        let r = self.selection();
        if !r.is_empty() {
            self.edit(r.clone(), "", Kind::Other, now_ms, (r.start, r.start));
            return;
        }
        let p = self.prev_grapheme(self.caret);
        if p < self.caret {
            self.edit(p..self.caret, "", Kind::Delete, now_ms, (p, p));
        }
    }

    /// Delete: deletes the selection or the grapheme after the caret.
    pub fn delete(&mut self, now_ms: u64) {
        let r = self.selection();
        if !r.is_empty() {
            self.edit(r.clone(), "", Kind::Other, now_ms, (r.start, r.start));
            return;
        }
        let n = self.next_grapheme(self.caret);
        if n > self.caret {
            let c = self.caret;
            self.edit(c..n, "", Kind::Delete, now_ms, (c, c));
        }
    }

    /// Cut: returns the selected text and deletes it (one undo step).
    pub fn cut(&mut self) -> String {
        let r = self.selection();
        let t = self.slice(r.clone());
        if !r.is_empty() {
            self.edit(r.clone(), "", Kind::Other, 0, (r.start, r.start));
            self.hist.seal();
        }
        t
    }

    /// Selects everything.
    pub fn select_all(&mut self) {
        self.anchor = 0;
        self.caret = self.len();
    }

    /// Undo; restores the selection from before the group. Returns false when empty.
    pub fn undo(&mut self) -> bool {
        let Some(g) = self.hist.pop_undo() else {
            return false;
        };
        for c in g.changes.iter().rev() {
            self.unapply(c);
        }
        self.anchor = g.sel_before.0.min(self.len());
        self.caret = g.sel_before.1.min(self.len());
        self.hist.push_redo(g);
        true
    }

    /// Redo; restores the selection from after the group.
    pub fn redo(&mut self) -> bool {
        let Some(g) = self.hist.pop_redo() else {
            return false;
        };
        for c in &g.changes {
            self.apply(c);
        }
        self.anchor = g.sel_after.0.min(self.len());
        self.caret = g.sel_after.1.min(self.len());
        self.hist.push_undo(g);
        true
    }

    /// Replaces the whole buffer (reload / convert) and clears the history.
    pub fn reset(&mut self, text: &str) {
        self.rope = Rope::from_str(text);
        self.hist = History::default();
        self.caret = self.caret.min(self.len());
        self.caret = self.snap(self.caret);
        self.anchor = self.caret;
        self.version += 1;
        self.saved_version = self.version;
    }
}

/// Converts every line break in `s` to `eol`.
pub fn normalize_eol(s: &str, eol: &str) -> String {
    let lf = s.replace("\r\n", "\n").replace('\r', "\n");
    if eol == "\n" {
        lf
    } else {
        lf.replace('\n', eol)
    }
}
