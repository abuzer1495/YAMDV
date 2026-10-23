//! Mind map viewer (FUT-37): Ctrl+Shift+M / Export ▸ View as mind map.

use super::*;

pub(in crate::window) struct Mind;

impl Win {
    pub(super) fn mind_open(&mut self, _sh: &Shared) {}
    pub(super) fn mind_key(&mut self, _sh: &Shared, _vk: u16, _ctrl: bool) {}
    pub(super) fn mind_down(&mut self, _sh: &Shared, _x: f32, _y: f32) {}
    pub(super) fn mind_up(&mut self) {}
    pub(super) fn mind_move(&mut self, _sh: &Shared, _x: f32, _y: f32) {}
    pub(super) fn mind_wheel(&mut self, _delta: i16, _x: f32, _y: f32) {}
    pub(super) fn mind_draw(&self, _sh: &Shared, _b: &Band<'_>) {}
}

#[cfg(feature = "dev-tools")]
impl Win {
    pub(super) fn mind_state(&self) -> Vec<(&'static str, String)> {
        Vec::new()
    }
}
