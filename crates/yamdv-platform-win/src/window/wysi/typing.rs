//! F2 typing transforms (prompt 016 §3 item 3): Space after a line-start marker, Enter after a
//! fence / `$$` / rule / pipe header row. Not applied inside a fenced block's body.

use super::*;
use crate::window::edit::Mode;
use yamdv_editor::{md, transform};

impl Win {
    /// Transforms apply here: F2, an open block, the caret not in a fence body.
    fn transforms_on(&self) -> bool {
        let Some(e) = self.edit() else { return false };
        if e.mode != Some(Mode::Live) || e.active.is_none() {
            return false;
        }
        let caret = e.ed.caret();
        !self
            .active_fence()
            .is_some_and(|(start, f)| caret > start + f.info.end)
    }

    /// Typed text in F2: a space may complete a line-start transform. True when handled.
    pub(in crate::window) fn f2_type(&mut self, sh: &Shared, s: &str) -> bool {
        if s != " " || !self.transforms_on() {
            return false;
        }
        let mut done = false;
        self.with_ed(sh, |ed| done = transform::space(ed));
        done
    }

    /// Enter (no modifiers): a block transform in F2, else Markdown-aware Enter.
    pub(in crate::window) fn f2_enter(&mut self, sh: &Shared) {
        let scope = if self.transforms_on() {
            self.active_range()
        } else {
            None
        };
        self.with_ed(sh, |ed| {
            if !scope.is_some_and(|r| transform::enter(ed, r)) {
                md::enter(ed);
            }
        });
    }
}
