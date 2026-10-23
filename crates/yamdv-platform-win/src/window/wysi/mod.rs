//! V2-F WYSIWYG F2 (prompt 016, ADR-0020): inline reveal of the active block, block sources
//! with live previews and the language picker, typing transforms, the link editor, table
//! editing. Hooks are called from `edit.rs` / `window.rs`.

use super::*;
use yamdv_core::keymap::Chord;

#[cfg(feature = "dev-tools")]
mod drive;
mod lang;
mod reveal;
mod typing;

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
}

impl Win {
    /// Key hook (before the V2-E hooks): `Some` when consumed.
    pub(super) fn v2f_key(&mut self, sh: &Shared, c: Chord) -> Option<Option<Action>> {
        if self.dialog.is_some() || self.settings.is_some() || !self.typing() {
            return None;
        }
        self.lang_key(sh, c)
    }

    /// Mouse-down hook: true when consumed.
    pub(super) fn v2f_down(&mut self, sh: &Shared, x: f32, y: f32) -> bool {
        self.dialog.is_none() && self.lang_down(sh, x, y)
    }

    /// Overlay paint hook (content origin).
    pub(super) fn v2f_draw(&self, sh: &Shared, b: &Band<'_>) {
        self.lang_draw(sh, b);
    }
}
