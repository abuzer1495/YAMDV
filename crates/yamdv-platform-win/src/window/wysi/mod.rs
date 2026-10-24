//! V2-F WYSIWYG F2 (prompt 016, ADR-0020): inline reveal of the active block, block sources
//! with live previews and the language picker, typing transforms, the link editor, table
//! editing. Hooks are called from `edit.rs` / `window.rs`.

use super::*;
use yamdv_core::keymap::Chord;

#[cfg(feature = "dev-tools")]
mod drive;
mod imgsize;
mod insert;
mod lang;
mod linkpop;
mod reveal;
mod table;
mod typing;
mod xsel;

pub(super) use reveal::RevealCache;
pub(super) use table::TblOp;
#[cfg(feature = "dev-tools")]
pub(super) use table::pick_label;

/// Debounced Mermaid preview timer.
pub(super) const TIMER_F2_PREVIEW: usize = 32;

/// Per-window V2-F state.
#[derive(Default)]
pub(super) struct Wysi {
    /// Selected language-picker row.
    pub lang_sel: usize,
    /// Picker dismissed (Esc / accepted) at this buffer version.
    pub lang_dismissed: Option<u64>,
    /// Debounced Mermaid preview waiting for its timer.
    pub pending: Option<yamdv_layout::reveal::Preview>,
    /// Open Ctrl+K link editor.
    pub link: Option<linkpop::LinkPop>,
    /// Open 10×10 table picker.
    pub pick: Option<table::Picker>,
    /// Table grip drag in progress.
    pub grip: Option<table::GripDrag>,
    /// Grips shown for the mouse position.
    pub grip_hover: Vec<(bool, usize)>,
    /// "Convert to table" offer after a TSV / CSV paste.
    pub tsv_offer: Option<table::TsvOffer>,
    /// Context-menu point (source offset) for Table ▸ on a table that is not open.
    pub tbl_at: Option<usize>,
    /// Selected image with resize handles (R-F2-12).
    pub img: Option<imgsize::ImgSel>,
}

impl Win {
    /// Key hook (before the V2-E hooks): `Some` when consumed.
    pub(super) fn v2f_key(&mut self, sh: &Shared, c: Chord) -> Option<Option<Action>> {
        if self.wysi.img.is_some() {
            if c.vk == VK_ESCAPE.0 && self.img_esc() {
                return Some(None);
            }
            // Any other key ends the image selection.
            self.wysi.img = None;
            self.invalidate();
        }
        if self.dialog.is_some() || self.settings.is_some() {
            return None;
        }
        if let Some(r) = self.xsel_key(sh, c) {
            return Some(r);
        }
        if !self.typing() {
            return None;
        }
        if self.wysi.link.is_some() {
            return self.linkpop_key(sh, c);
        }
        if let Some(r) = self.table_key(sh, c) {
            return Some(r);
        }
        let edit = self.edit_mode().is_some();
        if app::keymap().action_of(c, edit) == Some("link") && self.linkpop_open() {
            return Some(None);
        }
        self.lang_key(sh, c)
    }

    /// Mouse-down hook: true when consumed.
    pub(super) fn v2f_down(&mut self, sh: &Shared, x: f32, y: f32) -> bool {
        self.dialog.is_none()
            && (self.linkpop_down(sh, x, y)
                || self.pick_down(sh, x, y)
                || self.tsv_down(sh, x, y)
                || self.table_tools_down(sh, x, y)
                || self.grip_down(sh, x, y)
                || self.lang_down(sh, x, y))
    }

    /// Mouse-move hook: true when consumed (grip drag).
    pub(super) fn v2f_move(&mut self, sh: &Shared, x: f32, y: f32) -> bool {
        if self.img_move(sh, x) {
            return true;
        }
        self.pick_hover(sh, x, y);
        self.grip_move(sh, x, y)
    }

    /// Mouse-up hook: true when consumed (grip drop).
    pub(super) fn v2f_up(&mut self, sh: &Shared) -> bool {
        self.img_up(sh) || self.grip_up(sh)
    }

    /// Overlay paint hook (content origin).
    pub(super) fn v2f_draw(&self, sh: &Shared, b: &Band<'_>) {
        self.lang_draw(sh, b);
        self.table_tools_draw(sh, b);
        self.grips_draw(sh, b);
        self.img_draw(sh, b);
        self.tsv_draw(sh, b);
        self.linkpop_draw(sh, b);
        self.pick_draw(sh, b);
    }
}
