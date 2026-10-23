//! F2 cross-block selection (prompt 016 §3 item 5): a selection that leaves the active block
//! (Shift+click, drag, Shift+arrows at the block edge, Ctrl+A) closes the block and becomes a
//! document selection over the rendered blocks. Copy puts Markdown + HTML + plain text on the
//! clipboard; cut / delete / typing / paste replace it as one undo step.

use super::*;
use crate::window::edit::Mode;

impl Win {
    /// The F2 document selection (no active block), ordered.
    pub(in crate::window) fn xsel(&self) -> Option<Range<usize>> {
        let e = self.edit()?;
        if e.mode != Some(Mode::Live) || e.active.is_some() {
            return None;
        }
        let (a, b) = self.sel?;
        (a != b).then(|| a.min(b) as usize..a.max(b) as usize)
    }

    /// Block index containing source offset `pos`.
    pub(in crate::window) fn block_of(&self, pos: usize) -> Option<usize> {
        match &self.view {
            View::Doc(d) if !d.content.is_empty() => {
                Some(crate::window::edit::block_of(&d.content, pos))
            }
            _ => None,
        }
    }

    /// Extends the F2 selection to `pos`, leaving the active block (the document syncs first, so
    /// buffer and document offsets agree).
    pub(in crate::window) fn xsel_extend(&mut self, sh: &Shared, pos: usize) {
        let anchor = match (self.xsel(), self.sel) {
            (Some(_), Some((a, _))) => a as usize,
            _ => self.edit().map_or(pos, |e| e.ed.anchor()),
        };
        self.xsel_set(sh, anchor, pos);
    }

    /// Sets the F2 document selection `anchor → caret`.
    pub(in crate::window) fn xsel_set(&mut self, sh: &Shared, anchor: usize, caret: usize) {
        self.edit_leave_block(sh);
        let Some(e) = self.edit_mut() else { return };
        let len = e.ed.len();
        let (a, c) = (anchor.min(len), caret.min(len));
        e.ed.set_selection(a, c);
        self.sel = Some((u32::try_from(a).unwrap_or(0), u32::try_from(c).unwrap_or(0)));
        self.edit_title();
        self.invalidate();
    }

    /// Clears the document selection and opens the block at the caret.
    fn xsel_collapse(&mut self, sh: &Shared, at: usize) {
        self.sel = None;
        if let Some(bi) = self.block_of(at) {
            self.enter_block(sh, bi, at);
        }
        self.invalidate();
    }

    /// Replaces the document selection with `text` (one undo step) and opens the block at the
    /// caret. False when there is no document selection.
    pub(in crate::window) fn xsel_replace(&mut self, sh: &Shared, text: &str) -> bool {
        let Some(r) = self.xsel() else { return false };
        let end = r.start + text.len();
        self.sel = None;
        self.with_ed(sh, |ed| {
            ed.seal();
            ed.replace(r, text, end);
            ed.seal();
        });
        self.xsel_collapse(sh, end);
        true
    }

    /// Typed input while a document selection exists: deletes it first and opens the block.
    pub(in crate::window) fn xsel_input(&mut self, sh: &Shared) {
        self.xsel_replace(sh, "");
    }

    /// Copies the document selection as Markdown (source), HTML and plain text.
    fn xsel_copy(&self, r: Range<usize>) {
        let View::Doc(d) = &self.view else { return };
        let (a, b) = (
            u32::try_from(r.start).unwrap_or(0),
            u32::try_from(r.end).unwrap_or(0),
        );
        let plain = d.content.copy_text(a..b);
        let md = d.content.source().get(r).unwrap_or("").to_string();
        let html = match &d.content {
            Content::Markdown(doc) => Some(yamdv_export::html::cf_html(
                &yamdv_export::html::fragment(doc, a..b),
            )),
            Content::Plain { .. } => None,
        };
        sys::copy_clip(self.hwnd, &plain, html.as_deref(), Some(&md));
    }

    /// Keys while a document selection exists (`Some` = consumed).
    pub(in crate::window) fn xsel_key(
        &mut self,
        sh: &Shared,
        c: yamdv_core::keymap::Chord,
    ) -> Option<Option<Action>> {
        let live = self.edit_mode() == Some(Mode::Live);
        // Ctrl+A in F2 selects the whole document.
        if live && c.ctrl && !c.alt && !c.shift && c.vk == 0x41 {
            let n = self.edit()?.ed.len();
            self.xsel_set(sh, 0, n);
            return Some(None);
        }
        let r = self.xsel()?;
        let caret = self.sel.map_or(r.end, |(_, c)| c as usize);
        let (ctrl, shift) = (c.ctrl && !c.alt, c.shift);
        match c.vk {
            0x43 if ctrl => self.xsel_copy(r),
            0x58 if ctrl => {
                self.xsel_copy(r);
                self.xsel_replace(sh, "");
            }
            0x2E | 0x08 if !ctrl => {
                self.xsel_replace(sh, "");
            }
            0x56 if ctrl => {
                self.xsel_replace(sh, "");
                self.paste_rich(sh);
            }
            v if v == VK_ESCAPE.0 => {
                self.sel = None;
                self.invalidate();
            }
            v if (v == VK_DOWN.0 || v == VK_UP.0) && shift => {
                let down = v == VK_DOWN.0;
                let bi = self.block_of(caret.saturating_sub(usize::from(down && caret > 0)))?;
                let View::Doc(d) = &self.view else {
                    return None;
                };
                let pos = if down {
                    let t = (bi + 1).min(d.content.len().saturating_sub(1));
                    trim_eol(d.content.source(), d.content.range(t))
                } else {
                    let cur = d.content.range(bi).start as usize;
                    if caret > cur {
                        cur
                    } else {
                        d.content.range(bi.saturating_sub(1)).start as usize
                    }
                };
                self.xsel_extend(sh, pos);
            }
            v if (v == VK_LEFT.0 || v == VK_RIGHT.0) && shift => {
                let s = self.edit()?.ed.text();
                let mut p = caret;
                if v == VK_LEFT.0 {
                    p = p.saturating_sub(1);
                    while p > 0 && !s.is_char_boundary(p) {
                        p -= 1;
                    }
                } else {
                    p = (p + 1).min(s.len());
                    while p < s.len() && !s.is_char_boundary(p) {
                        p += 1;
                    }
                }
                self.xsel_extend(sh, p);
            }
            v if v == VK_LEFT.0 || v == VK_UP.0 => self.xsel_collapse(sh, r.start),
            v if v == VK_RIGHT.0 || v == VK_DOWN.0 => self.xsel_collapse(sh, r.end),
            // Ctrl+Z etc. drop the selection and fall through.
            _ if c.ctrl => {
                self.sel = None;
                return None;
            }
            _ => {}
        }
        Some(None)
    }

    /// After a paste in F2: text with blank lines becomes several blocks right away; the block
    /// holding the caret stays open.
    pub(in crate::window) fn f2_paste_split(&mut self, sh: &Shared) {
        let Some(r) = self.active_range() else { return };
        let Some(e) = self.edit() else { return };
        if e.mode != Some(Mode::Live) {
            return;
        }
        let t = e.ed.slice(r);
        if !(t.contains("\n\n") || t.contains("\n\r\n")) {
            return;
        }
        let caret = e.ed.caret();
        self.edit_leave_block(sh);
        if let Some(bi) = self.block_of(caret) {
            self.enter_block(sh, bi, caret);
        }
    }
}

/// End of block `r` without its trailing line break.
fn trim_eol(src: &str, r: Range<u32>) -> usize {
    let s = src.get(r.start as usize..r.end as usize).unwrap_or("");
    r.start as usize + s.trim_end_matches(['\n', '\r']).len()
}
