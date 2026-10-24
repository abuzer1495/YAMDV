//! F2 image resize handles (R-F2-12, prompt 017 §3 item 7): clicking an image in F2 selects it
//! and shows 4 corner handles (8 × 8, accent); dragging one keeps the aspect ratio and shows a
//! live "W px" label; the drop writes the width (`|N`, `<img width>`) as one undo step; `Esc`
//! cancels a drag (or clears the selection).

use super::*;
use crate::window::edit::Mode;
use yamdv_layout::block::HotKind;

/// Handle side (DIPs).
const HANDLE: f32 = 8.0;
/// Smallest width a drag produces.
const MIN_W: f32 = 16.0;

/// Selected image (by block and source as written).
#[derive(Debug, Clone)]
pub(in crate::window) struct ImgSel {
    pub block: usize,
    pub src: Box<str>,
    pub drag: Option<ImgDrag>,
}

/// A handle drag in progress.
#[derive(Debug, Clone, Copy)]
pub(in crate::window) struct ImgDrag {
    /// Corner 0 = top-left, 1 = top-right, 2 = bottom-left, 3 = bottom-right.
    pub corner: u8,
    pub x0: f32,
    pub w0: f32,
    pub w: f32,
}

impl Win {
    /// Image rectangles of block `bi` (client DIPs) with their sources.
    fn img_rects(&self, sh: &Shared, bi: usize) -> Vec<(Rect, Box<str>)> {
        let View::Doc(d) = &self.view else {
            return Vec::new();
        };
        let Some(bl) = d.viewer.layout_of(bi) else {
            return Vec::new();
        };
        let (cx, _) = self.col(sh);
        let top = (d.viewer.block_top(&d.content, bi) - d.viewer.scroll) as f32;
        bl.hot
            .iter()
            .filter_map(|h| match &h.kind {
                HotKind::Image { src } => Some((
                    Rect::new(cx + h.rect.x, top + h.rect.y, h.rect.w, h.rect.h),
                    src.clone(),
                )),
                _ => None,
            })
            .collect()
    }

    /// Rectangle of the selected image.
    fn img_sel_rect(&self, sh: &Shared) -> Option<Rect> {
        let s = self.wysi.img.as_ref()?;
        self.img_rects(sh, s.block)
            .into_iter()
            .find(|(_, src)| *src == s.src)
            .map(|r| r.0)
    }

    /// The four handles of `r`.
    fn img_handles(r: Rect) -> [Rect; 4] {
        let h = HANDLE / 2.0;
        [
            Rect::new(r.x - h, r.y - h, HANDLE, HANDLE),
            Rect::new(r.right() - h, r.y - h, HANDLE, HANDLE),
            Rect::new(r.x - h, r.bottom() - h, HANDLE, HANDLE),
            Rect::new(r.right() - h, r.bottom() - h, HANDLE, HANDLE),
        ]
    }

    /// Mouse down in F2 (before caret placement): starts a drag on a handle, selects a clicked
    /// image, or clears the selection. True when consumed.
    pub(in crate::window) fn img_down(&mut self, sh: &Shared, x: f32, y: f32) -> bool {
        if self.edit_mode() != Some(Mode::Live) || self.dialog.is_some() {
            return false;
        }
        if let Some(r) = self.img_sel_rect(sh) {
            let hit = Self::img_handles(r)
                .iter()
                .position(|h| Rect::new(h.x - 3.0, h.y - 3.0, h.w + 6.0, h.h + 6.0).contains(x, y));
            if let (Some(k), Some(s)) = (hit, self.wysi.img.as_mut()) {
                s.drag = Some(ImgDrag {
                    corner: u8::try_from(k).unwrap_or(3),
                    x0: x,
                    w0: r.w,
                    w: r.w,
                });
                // SAFETY: valid hwnd.
                unsafe { SetCapture(self.hwnd) };
                self.invalidate();
                return true;
            }
        }
        let bi = match &self.view {
            View::Doc(d) => d.viewer.block_at_y(f64::from(y) + d.viewer.scroll),
            _ => return false,
        };
        let hit = self
            .img_rects(sh, bi)
            .into_iter()
            .find(|(r, _)| r.contains(x, y));
        let had = self.wysi.img.take().is_some();
        if let Some((_, src)) = hit {
            self.wysi.img = Some(ImgSel {
                block: bi,
                src,
                drag: None,
            });
            self.invalidate();
            return true;
        }
        if had {
            self.invalidate();
        }
        false
    }

    /// Mouse move: resizes while dragging. True while dragging.
    pub(in crate::window) fn img_move(&mut self, sh: &Shared, x: f32) -> bool {
        let (_, cw) = self.col(sh);
        let Some(d) = self.wysi.img.as_mut().and_then(|s| s.drag.as_mut()) else {
            return false;
        };
        let dx = x - d.x0;
        let dx = if d.corner % 2 == 0 { -dx } else { dx };
        let w = (d.w0 + dx).clamp(MIN_W, cw.max(MIN_W)).round();
        if (w - d.w).abs() >= 0.5 {
            d.w = w;
            self.invalidate();
        }
        true
    }

    /// Mouse up: writes the new width (one undo step). True when a drag ended.
    pub(in crate::window) fn img_up(&mut self, sh: &Shared) -> bool {
        let Some(s) = self.wysi.img.as_mut() else {
            return false;
        };
        let Some(d) = s.drag.take() else {
            return false;
        };
        let block = s.block;
        // SAFETY: no preconditions.
        let _ = unsafe { ReleaseCapture() };
        if (d.w - d.w0).abs() >= 0.5 {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            self.img_write(sh, block, d.w as u32);
        }
        self.invalidate();
        true
    }

    /// Writes `width` into the source of the image in block `bi`.
    fn img_write(&mut self, sh: &Shared, bi: usize, width: u32) -> bool {
        let range = match &self.view {
            View::Doc(d) => match &d.content {
                yamdv_layout::viewer::Content::Markdown(doc) => doc
                    .index
                    .blocks
                    .get(bi)
                    .map(|b| b.range.start as usize..b.range.end as usize),
                _ => None,
            },
            _ => None,
        };
        let Some(range) = range else { return false };
        let Some(e) = self.edit() else { return false };
        let text = e.ed.slice(range.clone());
        let Some((r, new)) = yamdv_editor::imgsize::set_width(&text, width) else {
            return false;
        };
        let at = range.start + r.start..range.start + r.end;
        let caret = e.ed.caret();
        let caret = if caret >= at.end {
            caret + new.len() - at.len()
        } else if caret > at.start {
            at.start + new.len()
        } else {
            caret
        };
        self.with_ed(sh, |ed| {
            ed.seal();
            ed.replace(at, &new, caret);
            ed.seal();
        });
        true
    }

    /// `Esc`: cancels a drag, else clears the selection. Consumed only then.
    pub(in crate::window) fn img_esc(&mut self) -> bool {
        let Some(s) = self.wysi.img.as_mut() else {
            return false;
        };
        if s.drag.take().is_some() {
            // SAFETY: no preconditions.
            let _ = unsafe { ReleaseCapture() };
        } else {
            self.wysi.img = None;
        }
        self.invalidate();
        true
    }

    /// Paints the selection frame, the handles and the live width label.
    pub(in crate::window) fn img_draw(&self, sh: &Shared, b: &Band<'_>) {
        let Some(r) = self.img_sel_rect(sh) else {
            return;
        };
        let pal = &self.pal;
        let drag = self.wysi.img.as_ref().and_then(|s| s.drag);
        let r = match drag {
            Some(d) if r.w > 0.0 => {
                let h = r.h * d.w / r.w;
                // The corner opposite the dragged one stays put.
                let x = if d.corner % 2 == 0 {
                    r.right() - d.w
                } else {
                    r.x
                };
                let y = if d.corner < 2 { r.bottom() - h } else { r.y };
                Rect::new(x, y, d.w, h)
            }
            _ => r,
        };
        b.stroke(r, pal.accent, 0.0, 1.0);
        for h in Self::img_handles(r) {
            b.fill(h, pal.accent, 1.0, 1.0);
        }
        if let Some(d) = drag {
            let t = self.shape_ui(sh, &format!("{} px", d.w), 12.0, 600, 0xffffff, None);
            let lw = t.width() + 12.0;
            let lr = Rect::new(
                r.x + (r.w - lw) / 2.0,
                r.bottom() + 6.0,
                lw,
                t.height() + 6.0,
            );
            b.fill(lr, 0x202020, 0.85, 4.0);
            b.text(t.as_ref(), (lr.x + 6.0).round(), (lr.y + 3.0).round());
        }
    }

    #[cfg(feature = "dev-tools")]
    /// Driver: selects the image whose source contains `needle`, drags its bottom-right handle by
    /// `dx` and drops it (or presses Esc first when `esc`).
    pub(in crate::window) fn img_drive(
        &mut self,
        sh: &Shared,
        needle: &str,
        dx: f32,
        esc: bool,
    ) -> bool {
        let bi_max = match &self.view {
            View::Doc(d) => match &d.content {
                yamdv_layout::viewer::Content::Markdown(doc) => doc.index.blocks.len(),
                _ => 0,
            },
            _ => 0,
        };
        // `needle@N` picks the N-th match (1-based).
        let (needle, nth) = needle
            .rsplit_once('@')
            .and_then(|(n, k)| Some((n, k.parse::<usize>().ok()?)))
            .unwrap_or((needle, 1));
        let found = (0..bi_max)
            .flat_map(|bi| self.img_rects(sh, bi))
            .filter(|(_, s)| s.contains(needle))
            .nth(nth.saturating_sub(1))
            .map(|(r, _)| r);
        let Some(r) = found else { return false };
        if !self.img_down(sh, r.x + r.w / 2.0, r.y + r.h / 2.0) {
            return false;
        }
        let Some(r) = self.img_sel_rect(sh) else {
            return false;
        };
        let (hx, hy) = (r.right(), r.bottom());
        if !self.img_down(sh, hx, hy) {
            return false;
        }
        self.img_move(sh, hx + dx);
        if esc {
            self.img_esc();
        }
        self.img_up(sh);
        true
    }

    #[cfg(feature = "dev-tools")]
    /// Driver probe: (selected, dragging, live width).
    pub(in crate::window) fn img_probe(&self) -> (bool, f32) {
        let s = self.wysi.img.as_ref();
        (s.is_some(), s.and_then(|s| s.drag).map_or(0.0, |d| d.w))
    }
}
