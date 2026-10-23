//! Heading folding (R-READ-06, FUT-06): ▸/▾ toggles left of H1–H3. Click folds one heading,
//! Alt+click every heading of its level; Tab reaches the toggles after the links. The heading
//! keeps its screen position; 120 ms animation (instant with reduced motion). Session only.

use super::*;
use yamdv_layout::fold::{FOLD_ANIM_MS, MAX_FOLD_LEVEL};

const GUTTER: f32 = 18.0;

fn levels(content: &Content) -> Vec<Option<u8>> {
    (0..content.len())
        .map(|i| content.heading_level(i).filter(|l| *l <= MAX_FOLD_LEVEL))
        .collect()
}

impl Win {
    /// Toggle box of heading `bi` (content DIPs).
    fn gutter_rect(d: &DocView, cx: f32, bi: usize) -> Option<Rect> {
        let bl = d.viewer.layout_of(bi)?;
        let top = (d.viewer.block_top(&d.content, bi) - d.viewer.scroll) as f32;
        let h = bl.height.min(48.0);
        Some(Rect::new(
            cx - GUTTER - 6.0,
            (top + (h - GUTTER) / 2.0).round(),
            GUTTER,
            GUTTER,
        ))
    }

    /// Heading whose toggle is at (x, document y).
    pub(super) fn gutter_at(d: &DocView, cx: f32, x: f32, doc_y: f64) -> Option<usize> {
        if x < cx - GUTTER - 14.0 || x > cx + 2.0 || cx < GUTTER + 8.0 {
            return None;
        }
        let bi = d.viewer.block_at_y(doc_y);
        let lvl = d.content.heading_level(bi)?;
        (lvl <= MAX_FOLD_LEVEL && !d.viewer.is_hidden(bi)).then_some(bi)
    }

    pub(super) fn draw_gutter(&self, sh: &Shared, b: &Band<'_>) {
        let View::Doc(d) = &self.view else { return };
        let (cx, _) = self.col(sh);
        if cx < GUTTER + 8.0 || self.tz.fv.is_some() {
            return;
        }
        let mut show: Vec<usize> = self.tz.folds.folded().to_vec();
        show.extend(self.tz.gutter_hover);
        show.extend(self.tz.fold_focus);
        for h in show {
            let Some(r) = Self::gutter_rect(d, cx, h) else {
                continue;
            };
            if !b.visible(r.y, r.h) {
                continue;
            }
            if self.tz.gutter_hover == Some(h) {
                let tok = ch::tokens(self.dark);
                b.fill(r, tok.hover.0, f32::from(tok.hover.1) / 255.0, 4.0);
            }
            let glyph = if self.tz.folds.is_folded(h) {
                "▸"
            } else {
                "▾"
            };
            let t = self.shape_ui(sh, glyph, 13.0, 400, self.pal.text_muted, None);
            center(b, t.as_ref(), r);
            if self.tz.fold_focus == Some(h) {
                b.stroke(
                    Rect::new(r.x - 2.0, r.y - 2.0, r.w + 4.0, r.h + 4.0),
                    self.pal.accent,
                    5.0,
                    2.0,
                );
            }
        }
    }

    /// Toggles heading `h` (`alt`: every heading of its level).
    pub(in crate::window) fn tz_fold(&mut self, h: usize, alt: bool) {
        self.fold_sync();
        let vh = self.view_h();
        let reduced = self.reduced_motion();
        let View::Doc(d) = &mut self.view else { return };
        let lv = levels(&d.content);
        let Some(Some(lvl)) = lv.get(h).copied() else {
            return;
        };
        let before = self.tz.folds.hidden(&lv);
        if alt {
            self.tz.folds.toggle_level(&lv, lvl);
        } else {
            self.tz.folds.toggle(&lv, h);
        }
        let after = self.tz.folds.hidden(&lv);
        if before == after {
            self.invalidate();
            return;
        }
        let keep = (h, d.viewer.block_top(&d.content, h) - d.viewer.scroll);
        let moving: Vec<bool> = before.iter().zip(&after).map(|(a, b)| a != b).collect();
        let count = |v: &[bool]| v.iter().filter(|x| **x).count();
        let folding = count(&after) > count(&before);
        if reduced {
            d.viewer.set_hidden(&d.content, after, Vec::new(), 0.0);
            self.tz.anim = None;
        } else {
            let union: Vec<bool> = before.iter().zip(&after).map(|(a, b)| *a || *b).collect();
            let start = if folding { 1.0 } else { 0.0 };
            d.viewer
                .set_hidden(&d.content, union, moving.clone(), start);
            self.tz.anim = Some(FoldAnim {
                start: Instant::now(),
                hidden: after,
                moving,
                folding,
                keep,
            });
            // SAFETY: valid hwnd; runs only while animating (T7).
            unsafe { SetTimer(Some(self.hwnd), TIMER_FOLD, 8, None) };
        }
        self.keep_heading(keep, vh);
        self.invalidate();
    }

    fn keep_heading(&mut self, (h, y): (usize, f64), vh: f64) {
        let View::Doc(d) = &mut self.view else { return };
        let top = d.viewer.block_top(&d.content, h);
        d.viewer.scroll_to(top - y, vh);
    }

    pub(super) fn fold_tick(&mut self) {
        let vh = self.view_h();
        let Some(a) = self.tz.anim.clone() else {
            // SAFETY: valid hwnd.
            let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_FOLD) };
            return;
        };
        let t = (a.start.elapsed().as_secs_f64() * 1000.0 / f64::from(FOLD_ANIM_MS)).min(1.0);
        let View::Doc(d) = &mut self.view else { return };
        if t >= 1.0 {
            d.viewer
                .set_hidden(&d.content, a.hidden.clone(), Vec::new(), 0.0);
            self.tz.anim = None;
            // SAFETY: valid hwnd.
            let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_FOLD) };
        } else {
            let e = ease_out(t);
            let squash = if a.folding { 1.0 - e } else { e };
            let union: Vec<bool> = a
                .hidden
                .iter()
                .zip(&a.moving)
                .map(|(h, m)| *h || *m)
                .collect();
            d.viewer
                .set_hidden(&d.content, union, a.moving.clone(), squash);
        }
        self.keep_heading(a.keep, vh);
        self.invalidate();
    }

    /// Folds are dropped when the viewer lost them (reparse after an edit).
    fn fold_sync(&mut self) {
        if self.tz.folds.is_empty() || self.tz.anim.is_some() {
            return;
        }
        let View::Doc(d) = &mut self.view else { return };
        let want = self.tz.folds.hidden(&levels(&d.content));
        let ok = want.len() == d.content.len()
            && want
                .iter()
                .enumerate()
                .all(|(i, h)| *h == d.viewer.is_hidden(i));
        if !ok {
            self.tz.folds.clear();
            self.tz.fold_focus = None;
            d.viewer.set_hidden(&d.content, Vec::new(), Vec::new(), 0.0);
        }
    }

    /// A find hit at source offset `off`: unfold what hides it (instant).
    pub(in crate::window) fn tz_reveal_offset(&mut self, off: u32) {
        self.fold_sync();
        if self.tz.folds.is_empty() {
            return;
        }
        self.tz_drive_settle();
        let View::Doc(d) = &mut self.view else { return };
        let b = (0..d.content.len())
            .rfind(|&i| d.content.range(i).start <= off)
            .unwrap_or(0);
        let lv = levels(&d.content);
        if self.tz.folds.reveal(&lv, b) {
            let hidden = self.tz.folds.hidden(&lv);
            d.viewer.set_hidden(&d.content, hidden, Vec::new(), 0.0);
            self.invalidate();
        }
    }

    /// Tab past the last link reaches the toggles; `true` when one took the focus.
    pub(in crate::window) fn tz_fold_focus_step(&mut self, forward: bool) -> bool {
        let View::Doc(d) = &self.view else {
            return false;
        };
        let lv = levels(&d.content);
        let heads: Vec<usize> = (0..lv.len())
            .filter(|&i| lv[i].is_some() && !d.viewer.is_hidden(i))
            .collect();
        let next = match (self.tz.fold_focus, forward) {
            (None, true) => heads.first().copied(),
            (None, false) => heads.last().copied(),
            (Some(c), true) => heads.iter().copied().find(|&h| h > c),
            (Some(c), false) => heads.iter().copied().rfind(|&h| h < c),
        };
        self.tz.fold_focus = next;
        if let Some(h) = next {
            self.focus = None;
            let vh = self.view_h();
            let (top, s) = match &self.view {
                View::Doc(d) => (d.viewer.block_top(&d.content, h), d.viewer.scroll),
                _ => (0.0, 0.0),
            };
            if top < s + 8.0 || top > s + vh - 48.0 {
                self.set_scroll((top - vh / 3.0).max(0.0));
            }
        }
        self.invalidate();
        next.is_some()
    }

    /// Keys while a toggle has the focus; `true` when consumed.
    pub(super) fn fold_key(&mut self, _sh: &Shared, vk: u16, ctrl: bool, shift: bool) -> bool {
        let Some(h) = self.tz.fold_focus else {
            return false;
        };
        match vk {
            0x09 if !ctrl => {
                if !self.tz_fold_focus_step(!shift) && !shift {
                    // Past the last toggle: the Tab cycle starts again at the first link.
                    self.tz.fold_focus = None;
                    return false;
                }
                true
            }
            0x0D | 0x20 => {
                // SAFETY: GetKeyState has no preconditions.
                let alt = edit::KEY_ALT
                    .with(std::cell::Cell::get)
                    .unwrap_or_else(|| unsafe { GetKeyState(0x12) } < 0);
                self.tz_fold(h, alt);
                true
            }
            0x1B => {
                self.tz.fold_focus = None;
                self.invalidate();
                true
            }
            _ => {
                self.tz.fold_focus = None;
                self.invalidate();
                false
            }
        }
    }

    /// Press on a toggle (content DIPs); `true` when consumed.
    pub(super) fn fold_down(&mut self, sh: &Shared, x: f32, y: f32, alt: bool) -> bool {
        let (cx, _) = self.col(sh);
        let View::Doc(d) = &self.view else {
            return false;
        };
        let doc_y = f64::from(y) + d.viewer.scroll;
        let Some(h) = Self::gutter_at(d, cx, x, doc_y) else {
            return false;
        };
        self.tz.fold_focus = None;
        self.tz_fold(h, alt);
        true
    }

    /// Driver: the first heading whose source contains `text`.
    #[cfg(feature = "dev-tools")]
    pub(in crate::window) fn tz_heading(&self, text: &str) -> Option<usize> {
        let View::Doc(d) = &self.view else {
            return None;
        };
        (0..d.content.len()).find(|&i| {
            let r = d.content.range(i);
            d.content.heading_level(i).is_some()
                && d.content
                    .source()
                    .get(r.start as usize..r.end as usize)
                    .is_some_and(|s| s.contains(text))
        })
    }

    /// Finishes a running fold animation now (driver, find).
    pub(in crate::window) fn tz_drive_settle(&mut self) {
        if let Some(a) = &mut self.tz.anim {
            let ms = u64::from(FOLD_ANIM_MS) + 1;
            a.start = Instant::now()
                .checked_sub(std::time::Duration::from_millis(ms))
                .unwrap_or(a.start);
            self.fold_tick();
        }
    }
}
