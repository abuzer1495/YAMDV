//! The anchored Ask popover of an external link (B.10, R-LINK-04): 360 wide, radius 8, URL in
//! the code face, IDN / credential warnings, [Open] [Copy link] [Cancel]. Focus stays inside;
//! Esc or a click outside closes it.

use super::*;
use yamdv_ui::transient::{POP_RADIUS, POP_W};

const POP_OPEN: u32 = 1;
const POP_COPY: u32 = 2;
const POP_CANCEL: u32 = 3;
const PAD: f32 = 16.0;
const BTN_H: f32 = 32.0;

/// Popover geometry (content DIPs).
struct Geo {
    card: Rect,
    buttons: Vec<(Rect, u32, String)>,
    title_h: f32,
    url_h: f32,
}

fn buttons(p: &Popover) -> Vec<(Msg, u32)> {
    let mut v = Vec::new();
    if p.can_open {
        v.push((Msg::OpenInBrowser, POP_OPEN));
    }
    v.push((Msg::CopyLink, POP_COPY));
    v.push((Msg::Cancel, POP_CANCEL));
    v
}

impl Win {
    /// Opens the popover for `url`, anchored to link `(block, link)` when it is visible.
    pub(in crate::window) fn tz_ask_url(
        &mut self,
        sh: &Shared,
        anchor: Option<(usize, u32)>,
        url: String,
        notes: Vec<String>,
        can_open: bool,
    ) {
        let (cx, _) = self.col(sh);
        let link = anchor.and_then(|(bi, l)| {
            let View::Doc(d) = &self.view else {
                return None;
            };
            let top = (d.viewer.block_top(&d.content, bi) - d.viewer.scroll) as f32;
            focus_rects(&d.viewer, bi, l)
                .first()
                .map(|r| Rect::new(cx + r.x, top + r.y, r.w, r.h))
        });
        let r = link
            .filter(|r| r.bottom() > 0.0 && r.y < self.ch)
            .unwrap_or_else(|| Rect::new(self.cw / 2.0, self.ch / 3.0, 1.0, 1.0));
        self.tz_tip_hide();
        let p = Popover {
            url,
            notes,
            can_open,
            anchor: r,
            focus: 0,
            hover: None,
        };
        // Default focus: Cancel (the safe choice).
        let focus = buttons(&p).len() - 1;
        self.tz.pop = Some(Popover { focus, ..p });
        self.invalidate();
    }

    fn pop_geo(&self, sh: &Shared) -> Option<Geo> {
        let p = self.tz.pop.as_ref()?;
        let inner = Some(POP_W - 2.0 * PAD);
        let title = tr(sh.lang, Msg::LinkAskTitle);
        let title_h = shape(sh, title, (14.0, 600, 0), inner, Family::Body).height();
        let url_h = shape(sh, &p.url, (12.0, 400, 0), inner, Family::Code).height();
        let notes_h: f32 = p
            .notes
            .iter()
            .map(|n| 6.0 + shape(sh, n, (12.0, 400, 0), inner, Family::Body).height())
            .sum();
        let h = (PAD + title_h + 8.0 + url_h + notes_h + PAD + BTN_H + PAD).ceil();
        let screen = Bx::new(8.0, 8.0, self.cw - 16.0, self.ch - 16.0);
        let card = rect(yamdv_ui::transient::place(
            bx(p.anchor),
            POP_W,
            h,
            6.0,
            screen,
        ));
        let mut x = card.right() - PAD;
        let mut out = Vec::new();
        for (m, id) in buttons(p).into_iter().rev() {
            let label = tr(sh.lang, m).to_string();
            let w = (sh.dw.measure(&label, 13.0, 400, 10_000.0).0 + 24.0)
                .ceil()
                .max(72.0);
            x -= w;
            out.push((
                Rect::new(x, card.bottom() - PAD - BTN_H, w, BTN_H),
                id,
                label,
            ));
            x -= 8.0;
        }
        out.reverse();
        Some(Geo {
            card,
            buttons: out,
            title_h,
            url_h,
        })
    }

    pub(super) fn pop_draw(&self, sh: &Shared, b: &Band<'_>) {
        let (Some(p), Some(g)) = (&self.tz.pop, self.pop_geo(sh)) else {
            return;
        };
        let tok = ch::tokens(self.dark);
        let card = g.card;
        shadow(b, card, POP_RADIUS, self.dark);
        b.fill(card, tok.menu_bg, 1.0, POP_RADIUS);
        b.stroke(card, tok.menu_border, POP_RADIUS, 1.0);
        let inner = Some(POP_W - 2.0 * PAD);
        let x = card.x + PAD;
        let mut y = card.y + PAD;
        let title = tr(sh.lang, Msg::LinkAskTitle);
        let t = shape(sh, title, (14.0, 600, self.pal.text), inner, Family::Body);
        b.text(t.as_ref(), x, y);
        y += g.title_h + 8.0;
        let u = shape(sh, &p.url, (12.0, 400, self.pal.text), inner, Family::Code);
        b.text(u.as_ref(), x, y);
        y += g.url_h;
        for n in &p.notes {
            y += 6.0;
            let s = shape(sh, n, (12.0, 400, self.pal.error), inner, Family::Body);
            b.text(s.as_ref(), x, y);
            y += s.height();
        }
        for (i, (r, id, label)) in g.buttons.iter().enumerate() {
            let primary = *id == POP_OPEN;
            if primary {
                b.fill(*r, self.pal.accent, 1.0, 6.0);
            } else {
                if p.hover == Some(i) {
                    b.fill(*r, tok.hover.0, f32::from(tok.hover.1) / 255.0, 6.0);
                }
                b.stroke(*r, tok.menu_border, 6.0, 1.0);
            }
            let c = if primary { 0xffffff } else { self.pal.text };
            let l = self.shape_ui(sh, label, 13.0, 400, c, None);
            center(b, l.as_ref(), *r);
            if p.focus == i {
                b.stroke(
                    Rect::new(r.x - 3.0, r.y - 3.0, r.w + 6.0, r.h + 6.0),
                    self.pal.accent,
                    8.0,
                    2.0,
                );
            }
        }
    }

    fn pop_activate(&mut self, id: u32) {
        let Some(p) = self.tz.pop.take() else { return };
        match id {
            POP_OPEN => sys::shell_open(&p.url),
            POP_COPY if sys::copy_text(self.hwnd, &p.url) => self.toast(Msg::Copied),
            _ => {}
        }
        self.invalidate();
    }

    /// Keys while the popover is open: Tab / arrows move, Enter / Space activate, Esc closes.
    pub(super) fn pop_key(&mut self, vk: u16, shift: bool) {
        let Some(p) = &mut self.tz.pop else { return };
        let list = buttons(p);
        let n = list.len();
        match vk {
            0x1B => self.tz.pop = None,
            0x09 | 0x25 | 0x27 => {
                let back = (vk == 0x09 && shift) || vk == 0x25;
                p.focus = if back {
                    (p.focus + n - 1) % n
                } else {
                    (p.focus + 1) % n
                };
            }
            0x0D | 0x20 => {
                let id = list.get(p.focus).map_or(POP_CANCEL, |b| b.1);
                self.pop_activate(id);
            }
            _ => {}
        }
        self.invalidate();
    }

    /// Press while open: a button activates, outside closes (consumed either way).
    pub(super) fn pop_down(&mut self, sh: &Shared, x: f32, y: f32) {
        let Some(g) = self.pop_geo(sh) else { return };
        if let Some((_, id, _)) = g.buttons.iter().find(|(r, _, _)| r.contains(x, y)) {
            self.pop_activate(*id);
        } else if !g.card.contains(x, y) {
            self.tz.pop = None;
            self.invalidate();
        }
    }

    pub(super) fn pop_move(&mut self, sh: &Shared, x: f32, y: f32) {
        let hover = self
            .pop_geo(sh)
            .and_then(|g| g.buttons.iter().position(|(r, _, _)| r.contains(x, y)));
        if let Some(p) = &mut self.tz.pop
            && p.hover != hover
        {
            p.hover = hover;
            self.invalidate();
        }
    }
}
