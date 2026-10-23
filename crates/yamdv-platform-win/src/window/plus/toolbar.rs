//! F2 format toolbar (prompt 015 §3.1, R-F2-04, B.13.2): shown above a non-empty selection in
//! the open F2 block (8 px gap; below it when there is no room), h36, radius 8, `menu_bg` with
//! the `shadow` underlay, 28×28 buttons with group separators and tooltips. Esc dismisses it
//! for that selection; typing collapses the selection and so hides it. `editing.format_toolbar`.

use super::*;
use crate::window::edit::Mode;

/// Toolbar commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::window) enum Cmd {
    Bold,
    Italic,
    Strike,
    Highlight,
    Code,
    Link,
    H(u8),
    Quote,
    Bullet,
    Numbered,
    Task,
    Clear,
}

/// Buttons in order; `None` = group separator.
const ITEMS: &[Option<(Cmd, &str, u16, Msg)>] = &[
    Some((Cmd::Bold, "B", 800, Msg::TbBold)),
    Some((Cmd::Italic, "I", 400, Msg::TbItalic)),
    Some((Cmd::Strike, "S", 400, Msg::TbStrike)),
    Some((Cmd::Highlight, "H", 600, Msg::TbHighlight)),
    Some((Cmd::Code, "</>", 400, Msg::TbCode)),
    Some((Cmd::Link, "\u{2197}", 400, Msg::TbLink)),
    None,
    Some((Cmd::H(1), "H1", 600, Msg::TbH1)),
    Some((Cmd::H(2), "H2", 600, Msg::TbH2)),
    Some((Cmd::H(3), "H3", 600, Msg::TbH3)),
    None,
    Some((Cmd::Quote, "\u{201C}", 700, Msg::TbQuote)),
    Some((Cmd::Bullet, "\u{2022}", 700, Msg::TbBullet)),
    Some((Cmd::Numbered, "1.", 600, Msg::TbNumbered)),
    Some((Cmd::Task, "\u{2610}", 400, Msg::TbTask)),
    None,
    Some((Cmd::Clear, "T\u{2093}", 600, Msg::TbClear)),
];

const H: f32 = 36.0;
const BTN: f32 = 28.0;
const PAD: f32 = 4.0;
const SEP: f32 = 9.0;
const GAP: f32 = 8.0;

/// Bar width (DIPs).
fn width() -> f32 {
    ITEMS
        .iter()
        .map(|i| if i.is_some() { BTN } else { SEP })
        .sum::<f32>()
        + PAD * 2.0
}

/// Button rectangles inside `bar`, with their item index.
fn buttons(bar: Rect) -> Vec<(usize, Rect)> {
    let mut x = bar.x + PAD;
    let mut out = Vec::new();
    for (i, it) in ITEMS.iter().enumerate() {
        if it.is_some() {
            out.push((i, Rect::new(x, bar.y + (H - BTN) / 2.0, BTN, BTN)));
            x += BTN;
        } else {
            x += SEP;
        }
    }
    out
}

impl Win {
    /// The toolbar is shown (F2, open block, non-empty selection, not dismissed).
    pub(super) fn toolbar_visible(&self, _sh: &Shared) -> bool {
        let Some(e) = self.edit() else { return false };
        e.mode == Some(Mode::Live)
            && e.active.is_some()
            && !e.ed.selection().is_empty()
            && self.plus.tb_hidden.as_ref() != Some(&e.ed.selection())
            && self.find.is_none()
            && self.dialog.is_none()
            && setting("editing.format_toolbar")
    }

    /// Toolbar rectangle (content DIPs).
    pub(in crate::window) fn toolbar_rect(&self, sh: &Shared) -> Option<Rect> {
        if !self.toolbar_visible(sh) {
            return None;
        }
        let sel = self.edit()?.ed.selection();
        let (ox, oy, p) = self.live_para(sh)?;
        let map = p.map.as_ref()?;
        let at = |pos: usize| p.shaped.caret(map.from_source(u32::try_from(pos).unwrap_or(0)));
        let (a, b) = (at(sel.start), at(sel.end));
        let (top, bottom) = (oy + a.y.min(b.y), oy + (a.y + a.h).max(b.y + b.h));
        let w = width();
        let cx = if (a.y - b.y).abs() < 1.0 {
            ox + (a.x + b.x) / 2.0
        } else {
            ox + a.x
        };
        let x = (cx - w / 2.0).clamp(8.0, (self.cw - w - 8.0).max(8.0)).round();
        let above = top - GAP - H;
        let y = if above >= 4.0 { above } else { bottom + GAP };
        Some(Rect::new(x, y.round(), w, H))
    }

    /// Mouse down: runs a button. True when the press was on the toolbar.
    pub(in crate::window) fn toolbar_down(&mut self, sh: &Shared, x: f32, y: f32) -> bool {
        let Some(bar) = self.toolbar_rect(sh) else { return false };
        if !bar.contains(x, y) {
            return false;
        }
        let hit = buttons(bar).into_iter().find(|(_, r)| r.contains(x, y));
        if let Some((i, _)) = hit
            && let Some(Some((cmd, ..))) = ITEMS.get(i)
        {
            self.format_cmd(sh, *cmd);
        }
        true
    }

    /// Mouse move: hover (tooltip). True when over the toolbar.
    pub(in crate::window) fn toolbar_move(&mut self, sh: &Shared, x: f32, y: f32) -> bool {
        let bar = self.toolbar_rect(sh);
        let hover = bar.and_then(|b| {
            buttons(b)
                .into_iter()
                .find(|(_, r)| r.contains(x, y))
                .map(|(i, _)| i)
        });
        if hover != self.plus.tb_hover {
            self.plus.tb_hover = hover;
            self.invalidate();
        }
        bar.is_some_and(|b| b.contains(x, y))
    }

    /// Driver: clicks the button whose tooltip starts with `name` (e.g. "Bold").
    #[cfg(feature = "dev-tools")]
    pub(in crate::window) fn toolbar_press(&mut self, sh: &Shared, name: &str) -> bool {
        let Some(bar) = self.toolbar_rect(sh) else { return false };
        for (i, r) in buttons(bar) {
            if let Some(Some((_, _, _, m))) = ITEMS.get(i)
                && tr(Lang::En, *m).starts_with(name)
            {
                return self.toolbar_down(sh, r.x + 4.0, r.y + 4.0);
            }
        }
        false
    }

    /// Paints the toolbar (content origin).
    pub(in crate::window) fn toolbar_draw(&self, sh: &Shared, b: &Band<'_>) {
        let Some(bar) = self.toolbar_rect(sh) else { return };
        let pal = &self.pal;
        let tok = yamdv_core::chrome::tokens(self.dark);
        b.fill(Rect::new(bar.x, bar.y + 2.0, bar.w, bar.h), 0x000000, 0.14, 9.0);
        b.fill(bar, tok.menu_bg, 1.0, 8.0);
        b.stroke(bar, tok.chrome_border, 8.0, 1.0);
        let mut x = bar.x + PAD;
        for (i, it) in ITEMS.iter().enumerate() {
            let Some((_, label, weight, _)) = it else {
                b.fill(Rect::new(x + 4.0, bar.y + 8.0, 1.0, H - 16.0), tok.chrome_border, 1.0, 0.0);
                x += SEP;
                continue;
            };
            let r = Rect::new(x, bar.y + (H - BTN) / 2.0, BTN, BTN);
            if self.plus.tb_hover == Some(i) {
                b.fill(r, pal.text, 0.08, 4.0);
            }
            let t = self.shape_ui(sh, label, 13.0, *weight, pal.text, None);
            b.text(
                t.as_ref(),
                (r.x + (r.w - t.width()) / 2.0).round(),
                (r.y + (r.h - t.height()) / 2.0).round(),
            );
            x += BTN;
        }
        // Tooltip under the hovered button.
        if let Some(i) = self.plus.tb_hover
            && let Some(Some((.., m))) = ITEMS.get(i)
            && let Some((_, r)) = buttons(bar).into_iter().find(|(k, _)| *k == i)
        {
            let t = self.shape_ui(sh, tr(sh.lang, *m), 12.0, 400, pal.bg, Some(320.0));
            let w = t.width() + 16.0;
            let tx = (r.x + r.w / 2.0 - w / 2.0).clamp(4.0, (self.cw - w - 4.0).max(4.0));
            let tip = Rect::new(tx.round(), bar.y + bar.h + 4.0, w, t.height() + 8.0);
            b.fill(tip, pal.text, 0.92, 4.0);
            b.text(t.as_ref(), tip.x + 8.0, tip.y + 4.0);
        }
    }
}
