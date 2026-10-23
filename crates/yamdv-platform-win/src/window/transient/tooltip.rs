//! Tooltips (B.10.6, FUT-40) and the link status (B.10.4). The tooltip is drawn inside the
//! window after a one-shot 500 ms timer; it hides on move-out, scroll, key press and
//! deactivation.

use super::*;
use yamdv_core::chrome::Part;
use yamdv_ui::transient::{FOOT_PREVIEW, TIP_DELAY_MS, TIP_MAX_W, TIP_PAD, TIP_RADIUS, TIP_SIZE};

const STATUS_H: f32 = 22.0;

/// Middle ellipsis: `text` shortened to fit `max_w` at 12 px.
fn middle_ellipsis(sh: &Shared, text: &str, max_w: f32) -> String {
    let fits = |s: &str| sh.dw.measure(s, 12.0, 400, 100_000.0).0 <= max_w;
    if fits(text) {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut keep = chars.len();
    while keep > 4 {
        keep = keep * 9 / 10;
        let head: String = chars.iter().take(keep / 2).collect();
        let tail: String = chars.iter().skip(chars.len() - (keep - keep / 2)).collect();
        let s = format!("{head}…{tail}");
        if fits(&s) {
            return s;
        }
    }
    "…".into()
}

impl Win {
    pub(in crate::window) fn tz_tip_hide(&mut self) {
        // SAFETY: valid hwnd.
        let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_TIP) };
        self.tz.wait = None;
        if self.tz.tip.take().is_some() {
            // SAFETY: valid hwnd.
            let _ = unsafe { InvalidateRect(Some(self.hwnd), None, false) };
        }
    }

    /// Arms tooltip `t` (kept when it is already shown or waiting); `None` hides.
    pub(super) fn tip_want(&mut self, t: Option<Tip>) {
        let text = t.as_ref().map(|x| &x.text);
        if text.is_some()
            && (self.tz.tip.as_ref().map(|x| &x.text) == text
                || self.tz.wait.as_ref().map(|x| &x.text) == text)
        {
            return;
        }
        self.tz_tip_hide();
        if let Some(t) = t {
            self.tz.wait = Some(t);
            // SAFETY: valid hwnd; one-shot, killed when it fires (T7).
            unsafe { SetTimer(Some(self.hwnd), TIMER_TIP, TIP_DELAY_MS, None) };
        }
    }

    /// The delay passed: show the waiting tooltip.
    pub(super) fn tip_fire(&mut self) {
        // SAFETY: valid hwnd.
        let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_TIP) };
        if let Some(t) = self.tz.wait.take() {
            self.tz.tip = Some(t);
            // SAFETY: valid hwnd.
            let _ = unsafe { InvalidateRect(Some(self.hwnd), None, false) };
        }
    }

    /// Draws the tooltip (client DIPs, origin 0): 12 px, padding 4×8, radius 4, ≤ 360 wide.
    pub(super) fn tz_draw_tip(&self, sh: &Shared, b: &Band<'_>) {
        let Some(t) = &self.tz.tip else { return };
        let (fg, bg) = if self.dark {
            (0x1e1e1e, 0xf0f0f0)
        } else {
            (0xffffff, 0x333333)
        };
        let (py, px) = TIP_PAD;
        let s = shape(
            sh,
            &t.text,
            (TIP_SIZE, 400, fg),
            Some(TIP_MAX_W - 2.0 * px),
            Family::Body,
        );
        let w = (s.width() + 2.0 * px).ceil().min(TIP_MAX_W);
        let h = (s.height() + 2.0 * py).ceil();
        let screen = Bx::new(4.0, 4.0, self.cw - 8.0, self.ch + self.top - 8.0);
        let r = rect(yamdv_ui::transient::place(bx(t.at), w, h, 4.0, screen));
        b.fill(r, bg, 1.0, TIP_RADIUS);
        b.text(s.as_ref(), (r.x + px).round(), (r.y + py).round());
    }

    /// Link status at the bottom left (content DIPs): 8 px inset, 22 high, ≤ 60 % wide.
    fn status_rect(&self, sh: &Shared) -> Option<(Rect, String)> {
        let text = self.tz.status.as_ref()?;
        let (ax, aw) = self.doc_area(sh);
        let max = (aw * 0.6).max(80.0);
        let s = middle_ellipsis(sh, text, max - 16.0);
        let w = (sh.dw.measure(&s, 12.0, 400, 100_000.0).0 + 16.0)
            .ceil()
            .min(max);
        Some((
            Rect::new(ax + 8.0, self.ch - 8.0 - STATUS_H, w, STATUS_H),
            s,
        ))
    }

    pub(super) fn set_status(&mut self, sh: &Shared, s: Option<String>) {
        if self.tz.status == s {
            return;
        }
        if let Some((r, _)) = self.status_rect(sh) {
            self.inval_content(r);
        }
        self.tz.status = s;
        if let Some((r, _)) = self.status_rect(sh) {
            self.inval_content(r);
        }
    }

    pub(super) fn draw_status(&self, sh: &Shared, b: &Band<'_>) {
        let Some((r, s)) = self.status_rect(sh) else {
            return;
        };
        if !b.visible(r.y, r.h) {
            return;
        }
        let tok = ch::tokens(self.dark);
        b.fill(r, tok.menu_bg, 1.0, 4.0);
        b.stroke(r, tok.menu_border, 4.0, 1.0);
        let t = self.shape_ui(sh, &s, 12.0, 400, self.pal.text, None);
        b.push_clip(r);
        b.text(
            t.as_ref(),
            r.x + 8.0,
            (r.y + (r.h - t.height()) / 2.0).round(),
        );
        b.pop_clip();
    }

    /// Fixed overlays inside the scrolled rectangle (repainted after a scroll blit).
    pub(in crate::window) fn tz_overlay_rects(&self, sh: &Shared) -> Option<Rect> {
        self.status_rect(sh).map(|(r, _)| r)
    }

    /// Tooltip of a title-bar button.
    pub(super) fn chrome_tip(&self, sh: &Shared, p: Part) -> Option<String> {
        let m = match p {
            Part::Min => Msg::TipMinimize,
            Part::Max if self.maximized() => Msg::TipRestore,
            Part::Max => Msg::TipMaximize,
            Part::Close => Msg::TipClose,
            Part::More => Msg::TipMore,
            Part::Settings => Msg::MSettings,
            _ => return None,
        };
        Some(tr(sh.lang, m).to_string())
    }

    /// Word-count badge details (FUT-40): words, characters, reading minutes.
    fn words_tip(&self, sh: &Shared) -> Option<String> {
        let ((w, c), _) = self.word_counts()?;
        let mins = w.div_ceil(230).max(1);
        Some(
            tr(sh.lang, Msg::TipWords)
                .replace("{w}", &w.to_string())
                .replace("{c}", &c.to_string())
                .replace("{m}", &mins.to_string()),
        )
    }

    /// Footnote `n`, shortened to ≈ 300 characters.
    fn foot_preview(doc: &Document, n: u32) -> Option<String> {
        let u = doc.index.footnote_order.get((n as usize).checked_sub(1)?)?;
        let r = doc.index.footnotes.get(&u.label)?;
        let s = doc.source.get(r.start as usize..r.end as usize)?;
        let body = match s.find("]:") {
            Some(p) if s.starts_with("[^") => s.get(p + 2..).unwrap_or(""),
            _ => s,
        };
        Some(yamdv_ui::transient::preview(body, FOOT_PREVIEW))
    }

    /// Tooltip text of hot `kind` in block `bi` (link title, footnote, image title).
    fn hot_tip(
        &mut self,
        bi: usize,
        kind: &HotKind,
        at_source: Option<u32>,
    ) -> (Option<String>, Option<String>) {
        let View::Doc(d) = &mut self.view else {
            return (None, None);
        };
        let Content::Markdown(doc) = &mut d.content else {
            return (None, None);
        };
        let mut status = None;
        let tip = match kind {
            HotKind::Link { link } => doc.detail(bi).links.get(*link as usize).and_then(|l| {
                status = Some(l.dest.to_string());
                (!l.title.is_empty()).then(|| l.title.to_string())
            }),
            HotKind::FootRef { n, .. } => Self::foot_preview(doc, *n),
            HotKind::Image { src } => doc
                .detail(bi)
                .images
                .iter()
                .find(|i| i.src == *src && !i.title.is_empty())
                .map(|i| i.title.to_string()),
            _ => None,
        };
        let tip = tip.or_else(|| {
            let src = at_source?;
            let det = doc.detail(bi);
            let ri = det.runs.iter().position(|r| r.range.contains(&src))? as u32;
            det.abbrs
                .iter()
                .find(|(rr, _)| rr.contains(&ri))
                .map(|(_, t)| t.to_string())
        });
        (tip, status)
    }

    /// Hover over the content (content DIPs): tooltip, link status, media badge, fold gutter.
    pub(super) fn hover(&mut self, sh: &Shared, x: f32, y: f32) {
        let (cx, _) = self.col(sh);
        let at = self.tz_at(x, y);
        let mut tip = None;
        let mut status = None;
        let mut media = None;
        let mut gutter = None;
        if self.badge(sh).is_some_and(|(r, _)| r.contains(x, y)) {
            tip = self.words_tip(sh);
        } else if let View::Doc(d) = &self.view {
            let doc_y = f64::from(y) + d.viewer.scroll;
            gutter = Self::gutter_at(d, cx, x, doc_y);
            let bi = d.viewer.block_at_y(doc_y);
            let ly = (doc_y - d.viewer.block_top(&d.content, bi)) as f32;
            let lx = x - cx;
            let mut found = None;
            if let Some(bl) = d.viewer.layout_of(bi) {
                for h in bl.hot.iter().filter(|h| h.rect.contains(lx, ly)) {
                    if media.is_none()
                        && let Some(kind) = MediaKind::of(&h.kind)
                    {
                        media = Some(Media {
                            bi,
                            rect: h.rect,
                            kind,
                        });
                    }
                    found.get_or_insert_with(|| h.kind.clone());
                }
            }
            let src = d
                .viewer
                .hit(&d.content, lx, doc_y)
                .filter(|h| h.block == bi)
                .map(|h| h.source);
            if found.is_some() || src.is_some() {
                let k = found.unwrap_or(HotKind::CodeCopy { node: usize::MAX });
                (tip, status) = self.hot_tip(bi, &k, src);
            }
        }
        if tip.is_none() && gutter.is_some() {
            tip = Some(tr(sh.lang, Msg::TipFold).to_string());
        }
        if tip.is_none()
            && media
                .as_ref()
                .is_some_and(|m| self.media_badge_hit(sh, m, x, y).is_some())
        {
            tip = Some(tr(sh.lang, Msg::TipViewer).to_string());
        }
        if self.tz.media != media || self.tz.gutter_hover != gutter {
            self.tz.media = media;
            self.tz.gutter_hover = gutter;
            self.invalidate();
        }
        self.set_status(sh, status);
        self.tip_want(tip.map(|text| Tip { text, at }));
    }

    /// Keyboard focus moved to link `(b, l)`: its title shows after the delay.
    pub(in crate::window) fn tz_focus_tip(&mut self, sh: &Shared) {
        let Some((bi, l)) = self.focus else {
            self.tip_want(None);
            return;
        };
        let (cx, _) = self.col(sh);
        let at = match &self.view {
            View::Doc(d) => {
                let top = (d.viewer.block_top(&d.content, bi) - d.viewer.scroll) as f32;
                focus_rects(&d.viewer, bi, l)
                    .first()
                    .map(|r| Rect::new(cx + r.x, top + r.y + self.top, r.w, r.h))
            }
            _ => None,
        };
        let (tip, _) = self.hot_tip(bi, &HotKind::Link { link: l }, None);
        self.tip_want(tip.zip(at).map(|(text, at)| Tip { text, at }));
    }

    /// Driver: hover at (x, y); `fire` skips the 500 ms delay.
    #[cfg(feature = "dev-tools")]
    pub(in crate::window) fn tz_drive_hover(&mut self, sh: &Shared, x: f32, y: f32, fire: bool) {
        self.tz.mouse = (x, y);
        self.hover(sh, x, y);
        if fire {
            self.tip_fire();
        }
    }
}
