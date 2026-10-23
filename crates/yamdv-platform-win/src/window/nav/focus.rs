//! Keyboard reach (HARD-04): Tab walks every interactive part of the document in reading order
//! (pure order: `yamdv_layout::focus`), Enter / Space activate, F6 / Shift+F6 cycle title bar →
//! banners → sidebar → document. Rings show after keyboard use only.

use super::*;
use yamdv_core::chrome::{self as ch, Part};
use yamdv_core::doc::index::BlockKind;
use yamdv_layout::focus::{At, Stop, order, position};

/// Title-bar buttons in Tab order.
const PARTS: [Part; 5] = [
    Part::Settings,
    Part::More,
    Part::Min,
    Part::Max,
    Part::Close,
];

impl Win {
    /// The block can hold a focus stop (cheap source test; no layout needed).
    fn block_may_focus(&mut self, b: usize) -> bool {
        let Some(d) = self.doc() else { return false };
        if d.viewer.is_hidden(b) {
            return false;
        }
        let Content::Markdown(doc) = &mut d.content else {
            return false;
        };
        let Some(blk) = doc.index.blocks.get(b) else {
            return false;
        };
        let kind = blk.kind;
        let r = blk.range.start as usize..blk.range.end as usize;
        let src = doc.source.get(r).unwrap_or("");
        let foot = src.contains("[^");
        let tasks = src.contains("[ ]") || src.contains("[x]") || src.contains("[X]");
        let details = src.contains("<details");
        let links = !doc.detail(b).links.is_empty();
        links
            || foot
            || match kind {
                BlockKind::Heading(_)
                | BlockKind::Code
                | BlockKind::Table
                | BlockKind::FrontMatter
                | BlockKind::FootnoteDef => true,
                BlockKind::List => tasks,
                BlockKind::Html => details,
                BlockKind::BlockQuote { alert } => alert || details,
                _ => false,
            }
    }

    /// Focus stops of block `b`, laying it out first when needed (the view scrolls to it).
    fn stops_of(&mut self, sh: &Shared, b: usize) -> Vec<At> {
        let laid = matches!(&self.view, View::Doc(d) if d.viewer.layout_of(b).is_some());
        if !laid {
            let vh = self.view_h();
            let top = match &self.view {
                View::Doc(d) => d.viewer.block_top(&d.content, b),
                _ => return Vec::new(),
            };
            self.anim = None;
            self.set_scroll((top - vh / 3.0).max(0.0));
            self.prepare_now(sh);
        }
        let toggle = self.tz_toggle_rect(sh, b);
        let (cx, _) = self.col(sh);
        let View::Doc(d) = &self.view else {
            return Vec::new();
        };
        let Some(bl) = d.viewer.layout_of(b) else {
            return Vec::new();
        };
        let top = (d.viewer.block_top(&d.content, b) - d.viewer.scroll) as f32;
        let t = toggle.map(|r| Rect::new(r.x - cx, r.y - top, r.w, r.h));
        order(&bl.hot, t)
    }

    /// The current stop: (block, stop).
    fn cur_stop(&self) -> Option<(usize, Stop)> {
        if let Some((b, l)) = self.focus {
            return Some((b, Stop::Hot(HotKind::Link { link: l })));
        }
        if let Some(h) = self.tz.fold_focus {
            return Some((h, Stop::Toggle));
        }
        self.nav.kf.clone()
    }

    fn set_stop(&mut self, s: Option<(usize, Stop)>) {
        self.focus = None;
        self.tz.fold_focus = None;
        self.nav.kf = None;
        match s {
            Some((b, Stop::Hot(HotKind::Link { link }))) => self.focus = Some((b, link)),
            Some((b, Stop::Toggle)) => self.tz.fold_focus = Some(b),
            other => self.nav.kf = other,
        }
    }

    /// Tab / Shift+Tab in the document.
    pub(in crate::window) fn doc_focus_step(&mut self, sh: &Shared, dir: i32) {
        let n = match &self.view {
            View::Doc(d) => d.content.len(),
            _ => return,
        };
        let cur = self.cur_stop();
        let mut next: Option<(usize, Stop, Rect)> = None;
        if let Some((b, s)) = &cur {
            let st = self.stops_of(sh, *b);
            if let Some(i) = position(&st, s) {
                let j = if dir > 0 {
                    i.checked_add(1)
                } else {
                    i.checked_sub(1)
                };
                if let Some(a) = j.and_then(|j| st.get(j)) {
                    next = Some((*b, a.stop.clone(), a.rect));
                }
            }
        }
        if next.is_none() {
            let mut b = match (&cur, dir > 0) {
                (Some((b, _)), true) => b + 1,
                (Some((b, _)), false) => *b,
                (None, true) => 0,
                (None, false) => n,
            };
            for _ in 0..n {
                if dir > 0 && b >= n {
                    break;
                }
                if dir < 0 {
                    if b == 0 {
                        break;
                    }
                    b -= 1;
                }
                if self.block_may_focus(b) {
                    let st = self.stops_of(sh, b);
                    let pick = if dir > 0 { st.first() } else { st.last() };
                    if let Some(a) = pick {
                        next = Some((b, a.stop.clone(), a.rect));
                        break;
                    }
                }
                if dir > 0 {
                    b += 1;
                }
            }
        }
        match next {
            Some((b, s, r)) => {
                self.set_stop(Some((b, s)));
                self.reveal_rect(sh, b, r);
            }
            None => self.set_stop(None),
        }
        self.tz_focus_tip(sh);
        self.invalidate();
    }

    /// Scrolls so rect `r` of block `b` is in view (off-screen focus).
    fn reveal_rect(&mut self, sh: &Shared, b: usize, r: Rect) {
        let vh = self.view_h();
        let Some(d) = self.doc() else { return };
        let top = d.viewer.block_top(&d.content, b);
        let (y0, y1) = (top + f64::from(r.y), top + f64::from(r.y + r.h));
        let s = d.viewer.scroll;
        if y0 >= s + 8.0 && y1 <= s + vh - 8.0 {
            return;
        }
        self.anim = None;
        self.set_scroll((y0 - vh / 3.0).max(0.0));
        self.prepare_now(sh);
    }

    /// Content rectangle of the current non-link stop.
    fn kf_rect(&mut self, sh: &Shared) -> Option<(usize, HotKind, Rect)> {
        let (b, s) = self.nav.kf.clone()?;
        let st = self.stops_of(sh, b);
        let a = st.get(position(&st, &s)?)?;
        let Stop::Hot(k) = &a.stop else { return None };
        let (cx, _) = self.col(sh);
        let View::Doc(d) = &self.view else {
            return None;
        };
        let top = (d.viewer.block_top(&d.content, b) - d.viewer.scroll) as f32;
        Some((
            b,
            k.clone(),
            Rect::new(cx + a.rect.x, top + a.rect.y, a.rect.w, a.rect.h),
        ))
    }

    /// Enter / Space on the current non-link stop: the same as a click on it.
    fn kf_activate(&mut self, sh: &Shared) -> bool {
        let Some((b, k, r)) = self.kf_rect(sh) else {
            return false;
        };
        if matches!(k, HotKind::Diagram { .. } | HotKind::Table { .. }) {
            return self.tz_open_hot(sh, b, &k);
        }
        let keep = self.nav.kf.clone();
        let (x, y) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
        self.mouse_down(sh, x, y, false);
        self.mouse_up();
        if self.nav.kf.is_none() && !self.tz_viewer_open() {
            self.nav.kf = keep;
        }
        self.invalidate();
        true
    }

    /// Document keys: Tab, Enter / Space on a stop, Esc.
    pub(super) fn doc_key(
        &mut self,
        sh: &Shared,
        vk: u16,
        ctrl: bool,
        shift: bool,
        alt: bool,
    ) -> bool {
        if ctrl || alt || !matches!(self.view, View::Doc(_)) {
            return false;
        }
        match vk {
            0x09 => {
                self.nav_kbd(true);
                self.doc_focus_step(sh, if shift { -1 } else { 1 });
                true
            }
            0x0D | 0x20 if self.nav.kf.is_some() => self.kf_activate(sh),
            0x20 if self.focus.is_some() => {
                if let Some(a) = self.key(sh, 0x0D, false, false) {
                    self.actions.push(a);
                }
                true
            }
            0x1B if self.nav.kf.is_some() => {
                self.nav.kf = None;
                self.invalidate();
                true
            }
            _ => false,
        }
    }
}

impl Win {
    /// F6 / Shift+F6: title bar → banners → sidebar → document (regions that are shown).
    pub(super) fn region_step(&mut self, sh: &Shared, dir: i32) {
        let order = [
            Region::TitleBar,
            Region::Banners,
            Region::Sidebar,
            Region::Document,
        ];
        let has = |w: &Self, r: Region| match r {
            Region::TitleBar => w.chrome_visible(),
            Region::Banners => !w.tz_banner_stops(sh).is_empty(),
            Region::Sidebar => w.side_rect(sh).is_some(),
            Region::Document => true,
        };
        let mut i = order
            .iter()
            .position(|r| *r == self.nav.region)
            .unwrap_or(3);
        for _ in 0..4 {
            i = if dir > 0 { (i + 1) % 4 } else { (i + 3) % 4 };
            if let Some(&r) = order.get(i)
                && has(self, r)
            {
                self.nav.region = r;
                break;
            }
        }
        self.nav.chrome_focus = 0;
        self.nav.side_focus = None;
        if self.nav.region == Region::Sidebar {
            self.side_focus_first(sh);
        }
        self.invalidate();
    }

    /// Title-bar region keys.
    pub(super) fn chrome_key(&mut self, _sh: &Shared, vk: u16, shift: bool) -> bool {
        let n = PARTS.len();
        match vk {
            0x09 | 0x27 | 0x25 => {
                let back = vk == 0x25 || (vk == 0x09 && shift);
                self.nav.chrome_focus = (self.nav.chrome_focus + if back { n - 1 } else { 1 }) % n;
                self.invalidate();
                true
            }
            0x0D | 0x20 => {
                if let Some(&p) = PARTS.get(self.nav.chrome_focus) {
                    self.chrome_click(p);
                }
                true
            }
            0x1B => {
                self.nav.region = Region::Document;
                self.invalidate();
                true
            }
            _ => false,
        }
    }

    /// Banner region keys (buttons of all visible banners in order).
    pub(super) fn banner_key(&mut self, sh: &Shared, vk: u16, shift: bool) -> bool {
        let stops = self.tz_banner_stops(sh);
        let n = stops.len().max(1);
        match vk {
            0x09 | 0x27 | 0x25 => {
                let back = vk == 0x25 || (vk == 0x09 && shift);
                self.nav.chrome_focus = (self.nav.chrome_focus + if back { n - 1 } else { 1 }) % n;
                self.invalidate();
                true
            }
            0x0D | 0x20 => {
                if let Some(&(_, k, id)) = stops.get(self.nav.chrome_focus) {
                    self.tz_banner_button(sh, k, id);
                    self.nav.chrome_focus = 0;
                    if self.tz_banner_stops(sh).is_empty() {
                        self.nav.region = Region::Document;
                    }
                }
                true
            }
            0x1B => {
                self.nav.region = Region::Document;
                self.invalidate();
                true
            }
            _ => false,
        }
    }

    /// Focus ring of a non-link document stop (content origin).
    pub(super) fn focus_draw(&self, sh: &Shared, b: &Band<'_>) {
        if !self.nav.kbd || self.nav.region != Region::Document {
            return;
        }
        let Some((bi, s)) = &self.nav.kf else { return };
        let View::Doc(d) = &self.view else { return };
        let Some(bl) = d.viewer.layout_of(*bi) else {
            return;
        };
        let (cx, _) = self.col(sh);
        let top = (d.viewer.block_top(&d.content, *bi) - d.viewer.scroll) as f32;
        let st = order(&bl.hot, None);
        if let Some(a) = position(&st, s).and_then(|i| st.get(i)) {
            ring(
                b,
                Rect::new(cx + a.rect.x, top + a.rect.y, a.rect.w, a.rect.h),
                self.pal.accent,
            );
        }
    }

    /// Rings of the title-bar and banner regions (client origin).
    pub(in crate::window) fn nav_draw_client(&self, sh: &Shared, b: &Band<'_>) {
        if !self.nav.kbd {
            return;
        }
        match self.nav.region {
            Region::TitleBar => {
                let part = PARTS.get(self.nav.chrome_focus).copied();
                if let Some((_, x, w)) =
                    ch::buttons(self.cw).into_iter().find(|p| Some(p.0) == part)
                {
                    ring(
                        b,
                        Rect::new(x + 2.0, 2.0, w - 4.0, ch::BAR_H - 4.0),
                        self.pal.accent,
                    );
                }
            }
            Region::Banners => {
                if let Some((r, _, _)) = self.tz_banner_stops(sh).get(self.nav.chrome_focus) {
                    ring(b, *r, self.pal.accent);
                }
            }
            _ => {}
        }
    }
}

/// The 2 px focus ring with 1 px offset (B.16).
pub(in crate::window) fn ring(b: &Band<'_>, r: Rect, accent: u32) {
    b.stroke(
        Rect::new(r.x - 3.0, r.y - 3.0, r.w + 6.0, r.h + 6.0),
        accent,
        4.0,
        2.0,
    );
}

#[cfg(feature = "dev-tools")]
impl Win {
    /// Driver: the current stop's kind name and its document y.
    pub(super) fn focus_probe(&mut self, sh: &Shared) -> Option<(String, f64, usize)> {
        let (b, s) = self.cur_stop()?;
        let name = match &s {
            Stop::Toggle => "Toggle".to_string(),
            Stop::Hot(k) => format!("{k:?}")
                .split([' ', '{', '('])
                .next()
                .unwrap_or("")
                .to_string(),
        };
        let st = self.stops_of(sh, b);
        let r = st.get(position(&st, &s)?)?.rect;
        let View::Doc(d) = &self.view else {
            return None;
        };
        Some((name, d.viewer.block_top(&d.content, b) + f64::from(r.y), b))
    }
}
