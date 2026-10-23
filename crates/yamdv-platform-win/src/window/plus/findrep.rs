//! Replace row of the find bar (prompt 015 §3.2): Ctrl+H opens it (read mode switches to F3
//! first, P-46), [Replace] replaces the current match and selects the next, [Replace all] is
//! one undo step; plain search keeps Turkish folding, the optional regex (`$1`, `${name}`) runs
//! under the 100 ms guard. F3 matches are mirrored in the preview by source offset.

use super::*;
use crate::window::edit::Mode;
use crate::window::findbar::rects;
use yamdv_editor::replace::{self, FindError};

impl Win {
    /// Ctrl+H.
    pub(super) fn replace_open(&mut self, sh: &Shared) {
        if self.edit_mode().is_none() {
            self.edit_toggle(sh, Mode::Split);
            if self.edit_mode().is_none() {
                return;
            }
        }
        let sel = self
            .edit()
            .map(|e| e.ed.selected_text())
            .filter(|t| !t.is_empty() && !t.contains('\n') && t.len() <= 200);
        self.find_open();
        if let Some(f) = &mut self.find {
            if let Some(t) = sel {
                f.query = t;
            }
            f.replace = true;
            f.field = u8::from(!f.query.is_empty());
        }
        self.find_recompute();
        self.invalidate();
    }

    /// Keys while the bar is open (before the bar's own Esc / Enter handling).
    pub(super) fn findrep_key(&mut self, sh: &Shared, c: Chord) -> Option<Option<Action>> {
        let f = self.find.as_mut()?;
        // Alt+C / Alt+W / Alt+R: case, whole word, regex (keyboard reach of the option chips).
        if c.alt && !c.ctrl {
            let which = match c.vk {
                0x43 => 0,
                0x57 => 1,
                0x52 => 2,
                _ => return None,
            };
            self.find_toggle(sh, which);
            self.invalidate();
            return Some(None);
        }
        // Backspace / Delete edit the bar's field, never the document behind it.
        if !c.ctrl && (c.vk == 0x08 || c.vk == 0x2E) {
            if c.vk == 0x08 {
                self.find_char(sh, '\u{8}');
            }
            return Some(None);
        }
        if !f.replace {
            return None;
        }
        if c.vk == VK_TAB.0 && !c.ctrl {
            f.field = 1 - f.field.min(1);
            self.invalidate();
            return Some(None);
        }
        if c.vk == VK_RETURN.0 && f.field == 1 {
            if c.ctrl {
                self.replace_all_now(sh);
            } else {
                self.replace_one(sh);
            }
            return Some(None);
        }
        None
    }

    fn find_query(&self) -> Option<(replace::Query, String)> {
        let f = self.find.as_ref()?;
        (!f.query.is_empty()).then(|| (f.query(), f.with.clone()))
    }

    fn find_err(&mut self, e: FindError) {
        if let Some(f) = &mut self.find {
            f.err = Some(match e {
                FindError::TooSlow => Msg::FindTooSlow,
                FindError::BadRegex(_) => Msg::FindBadRegex,
            });
        }
        self.invalidate();
    }

    /// The edited buffer is what replace works on (F2 first leaves the open block).
    fn replace_prep(&mut self, sh: &Shared) -> bool {
        if self.edit_mode().is_none() {
            return false;
        }
        if self.edit().is_some_and(|e| e.active.is_some()) {
            self.edit_leave_block(sh);
        }
        true
    }

    /// [Replace].
    pub(super) fn replace_one(&mut self, sh: &Shared) {
        let Some((q, with)) = self.find_query() else {
            return;
        };
        if !self.replace_prep(sh) {
            return;
        }
        let mut res = Ok(false);
        self.with_ed(sh, |ed| res = replace::replace_next(ed, &q, &with));
        if let Err(e) = res {
            self.find_err(e);
            return;
        }
        self.find_recompute();
        let sel = self.edit().map(|e| e.ed.selection());
        if let (Some(f), Some(sel)) = (&mut self.find, sel) {
            let s = u32::try_from(sel.start).unwrap_or(0);
            if let Some(i) = f.matches.iter().position(|m| m.start == s) {
                f.cur = Some(i);
            }
        }
        self.find_reveal(sh);
    }

    /// [Replace all]: one undo step.
    pub(super) fn replace_all_now(&mut self, sh: &Shared) {
        let Some((q, with)) = self.find_query() else {
            return;
        };
        if !self.replace_prep(sh) {
            return;
        }
        let mut res = Ok(0);
        self.with_ed(sh, |ed| res = replace::replace_all(ed, &q, &with));
        match res {
            Err(e) => self.find_err(e),
            Ok(n) => {
                self.find_recompute();
                self.toast_with(Msg::Replaced, &format!(" {n}"));
                self.invalidate();
            }
        }
    }

    /// Clicks on the bar; true when inside it.
    pub(in crate::window) fn find_click(&mut self, sh: &Shared, x: f32, y: f32) -> bool {
        let Some(f) = &mut self.find else {
            return false;
        };
        let r = rects(self.cw, f.replace);
        if !r.bar.contains(x, y) {
            return false;
        }
        if r.toggle.contains(x, y) {
            f.replace = !f.replace;
            f.field = 0;
            if f.replace && self.edit_mode().is_none() {
                self.replace_open(sh);
            }
        } else if r.case.contains(x, y) {
            self.find_toggle(sh, 0);
        } else if r.whole.contains(x, y) {
            self.find_toggle(sh, 1);
        } else if r.regex.contains(x, y) {
            self.find_toggle(sh, 2);
        } else if f.replace && r.one.contains(x, y) {
            self.replace_one(sh);
        } else if f.replace && r.all.contains(x, y) {
            self.replace_all_now(sh);
        } else if f.replace {
            f.field = u8::from(y >= r.with.y - 2.0);
        }
        self.invalidate();
        true
    }
}
