//! Find bar (B.9 without the replace row, prompt 005 §3.8): incremental search over the
//! document source with Turkish i/İ/ı/I equivalence, highlight all, "n/m", wrap-around.

use super::*;
use yamdv_core::find;

/// Find state of one window.
#[derive(Debug, Default)]
pub(super) struct Find {
    pub query: String,
    pub case: bool,
    /// Source byte ranges of all matches, ascending.
    pub matches: Vec<Range<u32>>,
    pub cur: Option<usize>,
    /// The document changed: recompute on the next use.
    pub dirty: bool,
}

/// Bar geometry (DIPs): (bar, case toggle).
pub(super) fn bar_rects(cw: f32) -> (Rect, Rect) {
    let w = 380.0f32.min(cw - 32.0).max(200.0);
    let bar = Rect::new((cw - w - 16.0).round(), 12.0, w, 40.0);
    let case = Rect::new(bar.x + bar.w - 40.0, bar.y + 6.0, 32.0, 28.0);
    (bar, case)
}

impl Win {
    pub(super) fn find_open(&mut self) {
        if self.find.is_none() {
            self.find = Some(Find::default());
        }
        self.invalidate();
    }

    pub(super) fn find_close(&mut self) {
        self.find = None;
        self.invalidate();
    }

    fn find_recompute(&mut self) {
        let View::Doc(d) = &self.view else { return };
        let Some(f) = &mut self.find else { return };
        let src = d.content.source();
        f.matches = find::find_all(src, &f.query, f.case)
            .into_iter()
            .map(|(a, b)| u32::try_from(a).unwrap_or(0)..u32::try_from(b).unwrap_or(0))
            .collect();
        f.dirty = false;
        // Incremental: the first match at or after the reading position.
        let top = d.content.range(d.viewer.anchor().0).start;
        f.cur = (!f.matches.is_empty()).then(|| {
            f.matches
                .partition_point(|m| m.start < top)
                .min(f.matches.len() - 1)
        });
    }

    /// Typed text (WM_CHAR) while the bar is open.
    pub(super) fn find_char(&mut self, sh: &Shared, c: char) {
        let Some(f) = &mut self.find else { return };
        match c {
            '\u{8}' => {
                f.query.pop();
            }
            c if c.is_control() => return,
            c => f.query.push(c),
        }
        self.find_recompute();
        self.find_reveal(sh);
    }

    /// Enter / Shift+Enter / Ctrl+G / Ctrl+Shift+G.
    pub(super) fn find_step(&mut self, sh: &Shared, dir: i32) {
        if self.find.as_ref().is_some_and(|f| f.dirty) {
            self.find_recompute();
        }
        let Some(f) = &mut self.find else { return };
        f.cur = find::step(f.cur, f.matches.len(), dir);
        self.find_reveal(sh);
    }

    pub(super) fn find_toggle_case(&mut self, sh: &Shared) {
        if let Some(f) = &mut self.find {
            f.case = !f.case;
        }
        self.find_recompute();
        self.find_reveal(sh);
    }

    /// Scrolls the current match into view (one third from the top).
    fn find_reveal(&mut self, _sh: &Shared) {
        let target = {
            let (View::Doc(d), Some(f)) = (&self.view, &self.find) else {
                return;
            };
            let Some(m) = f.cur.and_then(|i| f.matches.get(i)) else {
                self.invalidate();
                return;
            };
            let n = d.content.len();
            let mut lo = 0usize;
            let mut hi = n;
            while lo + 1 < hi {
                let mid = (lo + hi) / 2;
                if d.content.range(mid).start <= m.start {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            let top = d.viewer.block_top(&d.content, lo);
            let bottom = d.viewer.block_top(&d.content, lo + 1);
            let s = d.viewer.scroll;
            let vh = self.view_h();
            (top < s + 60.0 || bottom > s + vh).then(|| (top - vh / 3.0).max(0.0))
        };
        if let Some(y) = target {
            self.set_scroll(y);
        }
        self.invalidate();
    }

    /// Draws the bar (B.9).
    pub(super) fn draw_find(&self, sh: &Shared, b: &Band<'_>) {
        let Some(f) = &self.find else { return };
        let pal = &self.pal;
        let (bar, case) = bar_rects(self.cw);
        b.fill(bar, pal.bg, 1.0, 8.0);
        b.stroke(bar, pal.code_border, 8.0, 1.0);
        let (q, qc) = if f.query.is_empty() {
            (
                tr(sh.lang, Msg::FindPlaceholder).to_string(),
                pal.text_muted,
            )
        } else {
            (f.query.clone(), pal.text)
        };
        let count = if f.query.is_empty() {
            String::new()
        } else if f.matches.is_empty() {
            tr(sh.lang, Msg::FindNone).to_string()
        } else {
            format!("{}/{}", f.cur.map_or(0, |c| c + 1), f.matches.len())
        };
        let ct = self.shape_ui(sh, &count, 12.0, 400, pal.text_muted, None);
        let cx = case.x - 8.0 - ct.width();
        let qt = self.shape_ui(sh, &q, 14.0, 400, qc, Some((cx - bar.x - 20.0).max(40.0)));
        b.push_clip(Rect::new(
            bar.x + 12.0,
            bar.y,
            (cx - bar.x - 16.0).max(1.0),
            bar.h,
        ));
        b.text(
            qt.as_ref(),
            bar.x + 12.0,
            (bar.y + (bar.h - qt.height()) / 2.0).round(),
        );
        b.pop_clip();
        // Caret after the query.
        let caret_x =
            (bar.x + 12.0 + if f.query.is_empty() { 0.0 } else { qt.width() } + 1.0).min(cx - 6.0);
        b.fill(
            Rect::new(caret_x, bar.y + 10.0, 1.0, 20.0),
            pal.text,
            1.0,
            0.0,
        );
        b.text(
            ct.as_ref(),
            cx,
            (bar.y + (bar.h - ct.height()) / 2.0).round(),
        );
        if f.case {
            b.fill(case, pal.accent, 0.18, 4.0);
            b.stroke(case, pal.accent, 4.0, 1.0);
        }
        let at = self.shape_ui(sh, tr(sh.lang, Msg::FindCase), 12.0, 600, pal.text, None);
        b.text(
            at.as_ref(),
            (case.x + (case.w - at.width()) / 2.0).round(),
            (case.y + (case.h - at.height()) / 2.0).round(),
        );
    }

    /// Match ranges and current index for `draw_doc`.
    pub(super) fn find_marks(&self) -> (&[Range<u32>], Option<usize>) {
        match &self.find {
            Some(f) if !f.query.is_empty() => (&f.matches, f.cur),
            _ => (&[], None),
        }
    }
}
