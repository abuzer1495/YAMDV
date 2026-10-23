//! Heading palette (FUT-08, B.10.8): Ctrl+Shift+O / ⋯ ▸ Go to heading…; filter in
//! `yamdv_core::outline::filter`. Built on open, freed on close.

use super::*;

pub(in crate::window) struct Palette;

impl Win {
    pub(super) fn palette_open(&mut self, _sh: &Shared) {}
    pub(super) fn palette_key(&mut self, _sh: &Shared, _vk: u16, _ctrl: bool) {}
    pub(super) fn palette_char(&mut self, _sh: &Shared, _c: char) {}
    pub(super) fn palette_down(&mut self, _sh: &Shared, _x: f32, _y: f32) {}
    pub(super) fn palette_move(&mut self, _sh: &Shared, _x: f32, _y: f32) {}
    pub(super) fn palette_wheel(&mut self, _delta: i16) {}
    pub(super) fn palette_draw(&self, _sh: &Shared, _b: &Band<'_>) {}
}

#[cfg(feature = "dev-tools")]
impl Win {
    pub(super) fn palette_state(&self) -> Vec<(&'static str, String)> {
        Vec::new()
    }
}
