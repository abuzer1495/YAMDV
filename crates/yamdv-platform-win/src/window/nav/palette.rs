//! Heading palette (FUT-08, B.10.8): Ctrl+Shift+O / ⋯ ▸ Go to heading…; the filter is pure
//! (`yamdv_core::outline::filter`, Turkish-aware). Built on open, freed on close.

use super::*;
use yamdv_core::outline::{Hit, filter};

const W: f32 = 560.0;
const INPUT_H: f32 = 44.0;
const ROW: f32 = 32.0;
const SHOWN: usize = 10;

/// Open palette.
pub(in crate::window) struct Palette {
    query: String,
    /// (level, text, block) of every heading.
    heads: Vec<(u8, String, usize)>,
    hits: Vec<Hit>,
    sel: usize,
    top: usize,
    hover: Option<usize>,
}

impl Palette {
    fn refilter(&mut self) {
        let titles: Vec<&str> = self.heads.iter().map(|h| h.1.as_str()).collect();
        self.hits = filter(&titles, &self.query);
        self.sel = 0;
        self.top = 0;
    }

    fn step(&mut self, d: isize) {
        let n = self.hits.len();
        if n == 0 {
            return;
        }
        self.sel = self.sel.saturating_add_signed(d).min(n - 1);
        if self.sel < self.top {
            self.top = self.sel;
        } else if self.sel >= self.top + SHOWN {
            self.top = self.sel + 1 - SHOWN;
        }
    }
}

impl Win {
    /// Card rectangle (content coordinates).
    fn palette_rect(&self) -> Rect {
        let w = W.min((self.cw - 48.0).max(200.0));
        let n = self
            .nav
            .palette
            .as_ref()
            .map_or(1, |p| p.hits.len().clamp(1, SHOWN));
        Rect::new(
            ((self.cw - w) / 2.0).round(),
            24.0,
            w,
            INPUT_H + n as f32 * ROW + 8.0,
        )
    }

    pub(in crate::window) fn palette_open(&mut self, _sh: &Shared) {
        let View::Doc(d) = &self.view else { return };
        let Content::Markdown(doc) = &d.content else {
            return;
        };
        let heads = doc
            .index
            .headings
            .iter()
            .map(|h| (h.level, h.text.clone(), h.block as usize))
            .collect();
        let mut p = Palette {
            query: String::new(),
            heads,
            hits: Vec::new(),
            sel: 0,
            top: 0,
            hover: None,
        };
        p.refilter();
        self.nav.mind = None;
        self.nav.palette = Some(p);
        self.invalidate();
    }

    fn palette_close(&mut self) {
        self.nav.palette = None;
        self.invalidate();
    }

    /// Jumps to hit `i` (unfolding it) and closes.
    fn palette_go(&mut self, sh: &Shared, i: usize) {
        let block = self
            .nav
            .palette
            .as_ref()
            .and_then(|p| p.hits.get(i).and_then(|h| p.heads.get(h.index)))
            .map(|h| h.2);
        self.palette_close();
        if let Some(b) = block {
            self.tz_unfold(b);
            self.scroll_to_block(sh, b);
        }
    }

    pub(super) fn palette_key(&mut self, sh: &Shared, vk: u16, ctrl: bool) {
        let Some(p) = &mut self.nav.palette else {
            return;
        };
        match vk {
            0x1B => self.palette_close(),
            0x0D => {
                let i = p.sel;
                self.palette_go(sh, i);
            }
            0x26 => p.step(-1),
            0x28 => p.step(1),
            0x21 => p.step(-(SHOWN as isize)),
            0x22 => p.step(SHOWN as isize),
            0x08 => {
                if ctrl {
                    p.query.clear();
                } else {
                    p.query.pop();
                }
                p.refilter();
            }
            _ => {}
        }
        self.invalidate();
    }

    pub(in crate::window) fn palette_char(&mut self, _sh: &Shared, c: char) {
        let Some(p) = &mut self.nav.palette else {
            return;
        };
        if c.is_control() {
            return;
        }
        p.query.push(c);
        p.refilter();
        self.invalidate();
    }

    /// Hit row under (x, y), content coordinates.
    fn palette_hit(&self, x: f32, y: f32) -> Option<usize> {
        let p = self.nav.palette.as_ref()?;
        let r = self.palette_rect();
        if !r.contains(x, y) || y < r.y + INPUT_H {
            return None;
        }
        let i = p.top + ((y - r.y - INPUT_H) / ROW) as usize;
        (i < p.hits.len()).then_some(i)
    }

    pub(super) fn palette_down(&mut self, sh: &Shared, x: f32, y: f32) {
        let r = self.palette_rect();
        if !r.contains(x, y) {
            self.palette_close();
            return;
        }
        if let Some(i) = self.palette_hit(x, y) {
            self.palette_go(sh, i);
        }
    }

    pub(super) fn palette_move(&mut self, _sh: &Shared, x: f32, y: f32) {
        let h = self.palette_hit(x, y);
        if let Some(p) = &mut self.nav.palette
            && p.hover != h
        {
            p.hover = h;
            self.invalidate();
        }
    }

    pub(super) fn palette_wheel(&mut self, delta: i16) {
        if let Some(p) = &mut self.nav.palette {
            let n = p.hits.len().saturating_sub(SHOWN);
            p.top = if delta > 0 {
                p.top.saturating_sub(3)
            } else {
                (p.top + 3).min(n)
            };
            self.invalidate();
        }
    }
}

/// Splits `text` into (piece, matched) by char ranges.
fn pieces(text: &str, ranges: &[(usize, usize)]) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    for (i, c) in text.chars().enumerate() {
        let m = ranges.iter().any(|&(a, b)| i >= a && i < b);
        match out.last_mut() {
            Some((s, lm)) if *lm == m => s.push(c),
            _ => out.push((c.to_string(), m)),
        }
    }
    out
}

impl Win {
    pub(super) fn palette_draw(&self, sh: &Shared, b: &Band<'_>) {
        let Some(p) = &self.nav.palette else { return };
        let pal = &self.pal;
        let r = self.palette_rect();
        crate::window::transient::card_shadow(b, r, 10.0, self.dark);
        b.fill(r, pal.bg, 1.0, 10.0);
        b.stroke(
            r,
            yamdv_core::chrome::tokens(self.dark).chrome_border,
            10.0,
            1.0,
        );
        let (q, qc) = if p.query.is_empty() {
            (tr(sh.lang, Msg::PaletteHint).to_string(), pal.text_muted)
        } else {
            (p.query.clone(), pal.text)
        };
        let t = self.shape_ui(sh, &q, 15.0, 400, qc, Some(r.w - 32.0));
        let ty = (r.y + (INPUT_H - t.height()) / 2.0).round();
        b.text(t.as_ref(), r.x + 16.0, ty);
        let caret_x = if p.query.is_empty() {
            r.x + 16.0
        } else {
            r.x + 16.0 + t.width() + 1.0
        };
        b.fill(
            Rect::new(caret_x, ty, 1.5, t.height()),
            pal.accent,
            1.0,
            0.0,
        );
        b.fill(
            Rect::new(r.x, r.y + INPUT_H - 1.0, r.w, 1.0),
            pal.border_subtle,
            1.0,
            0.0,
        );
        if p.hits.is_empty() {
            let n = self.shape_ui(
                sh,
                tr(sh.lang, Msg::PaletteNone),
                13.0,
                400,
                pal.text_muted,
                None,
            );
            b.text(
                n.as_ref(),
                r.x + 16.0,
                (r.y + INPUT_H + (ROW - n.height()) / 2.0).round(),
            );
            return;
        }
        b.push_clip(r);
        for (row, i) in (p.top..p.hits.len().min(p.top + SHOWN)).enumerate() {
            let Some(h) = p.hits.get(i) else { continue };
            let Some((lvl, text, _)) = p.heads.get(h.index) else {
                continue;
            };
            let y = r.y + INPUT_H + 4.0 + row as f32 * ROW;
            let rr = Rect::new(r.x + 6.0, y, r.w - 12.0, ROW);
            if i == p.sel {
                b.fill(rr, pal.accent, 0.14, 6.0);
            } else if p.hover == Some(i) {
                b.fill(rr, pal.text, 0.06, 6.0);
            }
            let mut x = rr.x + 10.0 + f32::from(lvl.saturating_sub(1)) * 12.0;
            let tag = self.shape_ui(sh, &format!("H{lvl}"), 10.0, 600, pal.text_muted, None);
            b.text(tag.as_ref(), x, (y + (ROW - tag.height()) / 2.0).round());
            x += 26.0;
            for (piece, m) in pieces(text, &h.ranges) {
                let (w, c) = if m {
                    (700, pal.accent)
                } else {
                    (400, pal.text)
                };
                let s = self.shape_ui(sh, &piece, 13.0, w, c, None);
                b.text(s.as_ref(), x, (y + (ROW - s.height()) / 2.0).round());
                x += s.width();
                if x > rr.x + rr.w {
                    break;
                }
            }
        }
        b.pop_clip();
    }
}

#[cfg(feature = "dev-tools")]
impl Win {
    pub(super) fn palette_state(&self) -> Vec<(&'static str, String)> {
        let (open, hits, first) = match &self.nav.palette {
            Some(p) => (
                "open",
                p.hits.len(),
                p.hits
                    .first()
                    .and_then(|h| p.heads.get(h.index))
                    .map(|h| h.1.clone())
                    .unwrap_or_default(),
            ),
            None => ("none", 0, String::new()),
        };
        vec![
            ("palette", open.into()),
            ("palette_hits", hits.to_string()),
            ("palette_first", first),
        ]
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn pieces_split_matches() {
        let p = super::pieces("Kurulum", &[(0, 3)]);
        assert_eq!(
            p,
            vec![("Kur".to_string(), true), ("ulum".to_string(), false)]
        );
    }
}
