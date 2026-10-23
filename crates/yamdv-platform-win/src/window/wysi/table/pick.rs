//! The 10×10 table picker (R-TBL-06): Ctrl+T and Insert ▸ Table… open it at the caret; the
//! live "R × C" label follows the mouse or the arrow keys; Enter / click inserts.

use super::*;
use yamdv_editor::insert::{self, Block};

/// Grid side.
const N: usize = 10;
/// Cell pitch.
const CELL: f32 = 18.0;
const PAD: f32 = 8.0;
const LABEL: f32 = 22.0;

/// The open picker: highlighted rows × columns (header included).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::window) struct Picker {
    pub rows: usize,
    pub cols: usize,
}

impl Win {
    /// Opens the picker (3 × 3 highlighted).
    pub(in crate::window) fn pick_open(&mut self) {
        if self.typing() {
            self.wysi.pick = Some(Picker { rows: 3, cols: 3 });
            self.invalidate();
        }
    }

    fn pick_rect(&self, sh: &Shared) -> Option<Rect> {
        self.wysi.pick?;
        let c = self
            .caret_rect(sh)
            .unwrap_or(Rect::new(40.0, 40.0, 1.0, 16.0));
        let side = PAD * 2.0 + CELL * N as f32;
        let h = side + LABEL;
        let x = c.x.clamp(8.0, (self.cw - side - 8.0).max(8.0));
        let below = c.y + c.h + 6.0;
        let y = if below + h > self.ch - 8.0 {
            (c.y - h - 6.0).max(8.0)
        } else {
            below
        };
        Some(Rect::new(x.round(), y.round(), side, h))
    }

    /// Cell (rows, cols) under (x, y), 1-based.
    fn pick_cell(&self, sh: &Shared, x: f32, y: f32) -> Option<(usize, usize)> {
        let r = self.pick_rect(sh)?;
        let (gx, gy) = (x - r.x - PAD, y - r.y - PAD);
        if gx < 0.0 || gy < 0.0 {
            return None;
        }
        let (c, rr) = ((gx / CELL) as usize, (gy / CELL) as usize);
        (c < N && rr < N).then_some((rr + 1, c + 1))
    }

    fn pick_accept(&mut self, sh: &Shared) {
        let Some(p) = self.wysi.pick.take() else {
            return;
        };
        self.with_ed(sh, |ed| insert::block(ed, Block::Table(p.rows, p.cols)));
        self.f2_reopen(sh);
        self.invalidate();
    }

    /// Picker keys: arrows resize, Enter inserts, Esc closes; other keys close it.
    pub(in crate::window) fn pick_key(&mut self, sh: &Shared, c: Chord) -> Option<Option<Action>> {
        let mut p = self.wysi.pick?;
        match c.vk {
            v if v == VK_LEFT.0 => p.cols = p.cols.saturating_sub(1).max(1),
            v if v == VK_RIGHT.0 => p.cols = (p.cols + 1).min(N),
            v if v == VK_UP.0 => p.rows = p.rows.saturating_sub(1).max(1),
            v if v == VK_DOWN.0 => p.rows = (p.rows + 1).min(N),
            v if v == VK_RETURN.0 => {
                self.pick_accept(sh);
                return Some(None);
            }
            v if v == VK_ESCAPE.0 => {
                self.wysi.pick = None;
                self.invalidate();
                return Some(None);
            }
            _ => {
                self.wysi.pick = None;
                self.invalidate();
                return None;
            }
        }
        self.wysi.pick = Some(p);
        self.invalidate();
        Some(None)
    }

    /// Mouse down: a cell inserts; inside = consumed; outside closes.
    pub(in crate::window) fn pick_down(&mut self, sh: &Shared, x: f32, y: f32) -> bool {
        let Some(r) = self.pick_rect(sh) else {
            return false;
        };
        if let Some((rows, cols)) = self.pick_cell(sh, x, y) {
            self.wysi.pick = Some(Picker { rows, cols });
            self.pick_accept(sh);
            return true;
        }
        if r.contains(x, y) {
            return true;
        }
        self.wysi.pick = None;
        self.invalidate();
        false
    }

    /// Mouse move: the highlight follows the pointer.
    pub(in crate::window) fn pick_hover(&mut self, sh: &Shared, x: f32, y: f32) {
        if let Some((rows, cols)) = self.pick_cell(sh, x, y)
            && self.wysi.pick != Some(Picker { rows, cols })
        {
            self.wysi.pick = Some(Picker { rows, cols });
            self.invalidate();
        }
    }

    /// Paints the picker.
    pub(in crate::window) fn pick_draw(&self, sh: &Shared, b: &Band<'_>) {
        let (Some(r), Some(p)) = (self.pick_rect(sh), self.wysi.pick) else {
            return;
        };
        let pal = &self.pal;
        let tok = yamdv_core::chrome::tokens(self.dark);
        b.fill(Rect::new(r.x, r.y + 2.0, r.w, r.h), 0x000000, 0.14, 9.0);
        b.fill(r, tok.menu_bg, 1.0, 8.0);
        b.stroke(r, tok.chrome_border, 8.0, 1.0);
        for rr in 0..N {
            for c in 0..N {
                let cell = Rect::new(
                    r.x + PAD + CELL * c as f32 + 1.0,
                    r.y + PAD + CELL * rr as f32 + 1.0,
                    CELL - 2.0,
                    CELL - 2.0,
                );
                if rr < p.rows && c < p.cols {
                    b.fill(cell, pal.accent, 0.35, 2.0);
                    b.stroke(cell, pal.accent, 2.0, 1.0);
                } else {
                    b.stroke(cell, tok.chrome_border, 2.0, 1.0);
                }
            }
        }
        let t = self.shape_ui(sh, &pick_label(p), 12.0, 600, pal.text, None);
        b.text(
            t.as_ref(),
            (r.x + (r.w - t.width()) / 2.0).round(),
            (r.bottom() - LABEL + (LABEL - t.height()) / 2.0 - 4.0).round(),
        );
    }
}

/// The live "R × C" label.
pub(in crate::window) fn pick_label(p: Picker) -> String {
    format!("{} × {}", p.rows, p.cols)
}
