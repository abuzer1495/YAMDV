//! Context menu Insert ▸ (prompt 016 §3 item 6): each command is one undo step; in F2 the new
//! blocks render at once and the block holding the caret opens.

use super::*;
use crate::window::edit::Mode;
use yamdv_editor::insert::{self, Block};

impl Win {
    /// Insert ▸ item `k` (order of `app::INS0`).
    pub(in crate::window) fn insert_cmd(&mut self, sh: &Shared, k: u32) {
        if !self.typing() {
            return;
        }
        match k {
            0 => {
                self.pick_open();
                return;
            }
            1 => self.with_ed(sh, |ed| insert::block(ed, Block::Code)),
            2..=6 => self.with_ed(sh, |ed| insert::block(ed, Block::Callout(k as usize - 2))),
            7 => self.with_ed(sh, |ed| insert::block(ed, Block::Math)),
            8 => self.with_ed(sh, |ed| insert::block(ed, Block::Mermaid)),
            9 => self.with_ed(sh, |ed| insert::block(ed, Block::Rule)),
            10 => self.with_ed(sh, yamdv_editor::md::image),
            11 => {
                if !self.linkpop_open() {
                    self.with_ed(sh, yamdv_editor::md::link);
                }
                return;
            }
            12 => self.with_ed(sh, insert::footnote),
            _ => return,
        }
        self.f2_reopen(sh);
    }

    /// F2 after a structural edit: re-split the active block and keep the caret's block open.
    pub(in crate::window) fn f2_reopen(&mut self, sh: &Shared) {
        if self.edit_mode() != Some(Mode::Live) {
            return;
        }
        if self.edit().is_some_and(|e| e.active.is_some()) {
            self.f2_paste_split(sh);
        }
        if self.edit().is_some_and(|e| e.active.is_none())
            && let Some(c) = self.edit().map(|e| e.ed.caret())
            && let Some(bi) = self.block_of(c)
        {
            self.sync_preview(sh);
            self.enter_block(sh, bi, c);
        }
    }
}
