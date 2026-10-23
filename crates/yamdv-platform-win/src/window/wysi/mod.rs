//! V2-F WYSIWYG F2 (prompt 016, ADR-0020): inline reveal of the active block, block sources
//! with live previews and the language picker, typing transforms, the link editor, table
//! editing. Hooks are called from `edit.rs` / `window.rs`.

use super::*;
use yamdv_core::keymap::Chord;

#[cfg(feature = "dev-tools")]
mod drive;
mod insert;
mod lang;
mod linkpop;
mod reveal;
mod typing;
mod xsel;

pub(super) use reveal::RevealCache;

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
}

impl Win {
    /// Key hook (before the V2-E hooks): `Some` when consumed.
    pub(super) fn v2f_key(&mut self, sh: &Shared, c: Chord) -> Option<Option<Action>> {
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
        let edit = self.edit_mode().is_some();
        if app::keymap().action_of(c, edit) == Some("link") && self.linkpop_open() {
            return Some(None);
        }
        self.lang_key(sh, c)
    }

    /// Mouse-down hook: true when consumed.
    pub(super) fn v2f_down(&mut self, sh: &Shared, x: f32, y: f32) -> bool {
        self.dialog.is_none() && (self.linkpop_down(sh, x, y) || self.lang_down(sh, x, y))
    }

    /// Overlay paint hook (content origin).
    pub(super) fn v2f_draw(&self, sh: &Shared, b: &Band<'_>) {
        self.lang_draw(sh, b);
        self.linkpop_draw(sh, b);
    }
}
