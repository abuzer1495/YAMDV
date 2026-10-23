//! Find bar (B.9, prompt 005 §3.8; replace row prompt 015 §3.2): incremental search over the
//! document source with Turkish i/İ/ı/I equivalence (plain search), whole word, optional regex
//! (100 ms guard), highlight all, "n/m", wrap-around. The replace row lives in `plus::findrep`.

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
    /// Replace row shown (Ctrl+H, [▸]).
    pub replace: bool,
    /// Replacement text.
    pub with: String,
    /// Keyboard field: 0 = query, 1 = replacement.
    pub field: u8,
    /// Whole-word and regex toggles.
    pub whole: bool,
    pub regex: bool,
    /// Bad pattern / guard hit.
    pub err: Option<Msg>,
}

/// Bar geometry (DIPs).
#[derive(Debug, Clone, Copy)]
pub(super) struct Rects {
    pub bar: Rect,
    pub toggle: Rect,
    pub regex: Rect,
    pub whole: Rect,
    pub case: Rect,
    /// Replace row (meaningful when the row is shown).
    pub with: Rect,
    pub one: Rect,
    pub all: Rect,
}

/// Full geometry; `tall` = replace row shown.
pub(super) fn rects(cw: f32, tall: bool) -> Rects {
    let w = 440.0f32.min(cw - 32.0).max(260.0);
    let bar = Rect::new(
        (cw - w - 16.0).round(),
        12.0,
        w,
        if tall { 76.0 } else { 40.0 },
    );
    let btn = |i: f32| {
        Rect::new(
            bar.x + bar.w - 8.0 - 32.0 * i - 2.0 * (i - 1.0),
            bar.y + 6.0,
            32.0,
            28.0,
        )
    };
    let row = bar.y + 42.0;
    let all = Rect::new(bar.x + bar.w - 8.0 - 104.0, row, 104.0, 28.0);
    let one = Rect::new(all.x - 6.0 - 80.0, row, 80.0, 28.0);
    Rects {
        bar,
        toggle: Rect::new(bar.x + 4.0, bar.y + 8.0, 20.0, 24.0),
        regex: btn(3.0),
        whole: btn(2.0),
        case: btn(1.0),
        with: Rect::new(bar.x + 28.0, row, (one.x - bar.x - 34.0).max(40.0), 28.0),
        one,
        all,
    }
}

impl Find {
    /// The search options as a replace-engine query.
    pub(super) fn query(&self) -> yamdv_editor::replace::Query {
        yamdv_editor::replace::Query {
            needle: self.query.clone(),
            case: self.case,
            whole_word: self.whole,
            regex: self.regex,
        }
    }
}

/// Bar geometry (DIPs): (bar, case toggle).
pub(super) fn bar_rects(cw: f32, tall: bool) -> (Rect, Rect) {
    let r = rects(cw, tall);
    (r.bar, r.case)
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

    pub(super) fn find_recompute(&mut self) {
        // F3 searches the editor buffer; F2 and read mode the document.
        let buf = self.find_source();
        let View::Doc(d) = &self.view else { return };
        let Some(f) = &mut self.find else { return };
        let src = buf.as_ref().map_or(d.content.source(), |b| b.0.as_str());
        f.err = None;
        f.matches = if f.regex || f.whole {
            match yamdv_editor::replace::find_all(
                src,
                &f.query(),
                yamdv_editor::replace::REGEX_GUARD,
            ) {
                Ok(v) => v,
                Err(e) => {
                    f.err = Some(match e {
                        yamdv_editor::replace::FindError::TooSlow => Msg::FindTooSlow,
                        yamdv_editor::replace::FindError::BadRegex(_) => Msg::FindBadRegex,
                    });
                    Vec::new()
                }
            }
            .into_iter()
            .map(|r| u32::try_from(r.start).unwrap_or(0)..u32::try_from(r.end).unwrap_or(0))
            .collect()
        } else {
            find::find_all(src, &f.query, f.case)
                .into_iter()
                .map(|(a, b)| u32::try_from(a).unwrap_or(0)..u32::try_from(b).unwrap_or(0))
                .collect()
        };
        f.dirty = false;
        // Incremental: the first match at or after the reading position.
        let top = buf
            .as_ref()
            .map_or_else(|| d.content.range(d.viewer.anchor().0).start, |b| b.1);
        f.cur = (!f.matches.is_empty()).then(|| {
            f.matches
                .partition_point(|m| m.start < top)
                .min(f.matches.len() - 1)
        });
    }

    /// Typed text (WM_CHAR) while the bar is open.
    pub(super) fn find_char(&mut self, sh: &Shared, c: char) {
        let Some(f) = &mut self.find else { return };
        let field = if f.field == 1 && f.replace {
            &mut f.with
        } else {
            &mut f.query
        };
        match c {
            '\u{8}' => {
                field.pop();
            }
            c if c.is_control() => return,
            c => field.push(c),
        }
        if f.field == 1 && f.replace {
            self.invalidate();
            return;
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

    /// Toggles option 0 = case, 1 = whole word, 2 = regex.
    pub(super) fn find_toggle(&mut self, sh: &Shared, which: u8) {
        if let Some(f) = &mut self.find {
            match which {
                0 => f.case = !f.case,
                1 => f.whole = !f.whole,
                _ => f.regex = !f.regex,
            }
        }
        self.find_recompute();
        self.find_reveal(sh);
    }

    /// Scrolls the current match into view (one third from the top).
    pub(super) fn find_reveal(&mut self, sh: &Shared) {
        let cur = self
            .find
            .as_ref()
            .and_then(|f| f.cur.and_then(|i| f.matches.get(i)).cloned());
        if let Some(m) = cur.clone()
            && self.find_reveal_edit(sh, m)
        {
            return;
        }
        // R-READ-06: a hit inside a folded section unfolds it first.
        if let Some(m) = cur {
            self.tz_reveal_offset(m.start);
        }
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

    /// The replace row is shown.
    pub(super) fn find_tall(&self) -> bool {
        self.find.as_ref().is_some_and(|f| f.replace)
    }

    /// A text field with a caret when focused.
    fn find_field(&self, sh: &Shared, b: &Band<'_>, r: Rect, text: &str, ph: Msg, focus: bool) {
        let pal = &self.pal;
        let (t, c) = if text.is_empty() {
            (tr(sh.lang, ph).to_string(), pal.text_muted)
        } else {
            (text.to_string(), pal.text)
        };
        let st = self.shape_ui(sh, &t, 14.0, 400, c, None);
        b.push_clip(Rect::new(r.x, r.y - 6.0, r.w.max(1.0), r.h + 12.0));
        // Long queries scroll so the end (and the caret) stays visible.
        let off = if text.is_empty() {
            0.0
        } else {
            (st.width() + 2.0 - r.w).max(0.0)
        };
        b.text(
            st.as_ref(),
            r.x - off,
            (r.y + (r.h - st.height()) / 2.0).round(),
        );
        if focus {
            let cx = r.x - off + if text.is_empty() { 0.0 } else { st.width() } + 1.0;
            b.fill(
                Rect::new(cx.min(r.x + r.w - 1.0), r.y + 4.0, 1.0, r.h - 8.0),
                pal.text,
                1.0,
                0.0,
            );
        }
        b.pop_clip();
    }

    /// A toggle / push button.
    fn find_btn(&self, sh: &Shared, b: &Band<'_>, r: Rect, label: &str, on: bool, framed: bool) {
        let pal = &self.pal;
        if on {
            b.fill(r, pal.accent, 0.18, 4.0);
            b.stroke(r, pal.accent, 4.0, 1.0);
        } else if framed {
            b.stroke(r, pal.code_border, 4.0, 1.0);
        }
        let t = self.shape_ui(sh, label, 12.0, 600, pal.text, Some(r.w - 4.0));
        b.text(
            t.as_ref(),
            (r.x + (r.w - t.width()) / 2.0).round(),
            (r.y + (r.h - t.height()) / 2.0).round(),
        );
    }

    /// Draws the bar (B.9) and the replace row.
    pub(super) fn draw_find(&self, sh: &Shared, b: &Band<'_>) {
        let Some(f) = &self.find else { return };
        let pal = &self.pal;
        let r = rects(self.cw, f.replace);
        b.fill(r.bar, pal.bg, 1.0, 8.0);
        b.stroke(r.bar, pal.code_border, 8.0, 1.0);
        let count = if f.query.is_empty() {
            String::new()
        } else if let Some(e) = f.err {
            tr(sh.lang, e).to_string()
        } else if f.matches.is_empty() {
            tr(sh.lang, Msg::FindNone).to_string()
        } else {
            format!("{}/{}", f.cur.map_or(0, |c| c + 1), f.matches.len())
        };
        let cc = if f.err.is_some() {
            pal.error
        } else {
            pal.text_muted
        };
        let ct = self.shape_ui(sh, &count, 12.0, 400, cc, Some(140.0));
        let cx = r.regex.x - 8.0 - ct.width();
        let qx = r.bar.x + 28.0;
        let q = Rect::new(qx, r.bar.y + 8.0, (cx - qx - 8.0).max(40.0), 24.0);
        self.find_field(
            sh,
            b,
            q,
            &f.query,
            Msg::FindPlaceholder,
            f.field == 0 || !f.replace,
        );
        b.text(
            ct.as_ref(),
            cx,
            (r.bar.y + (40.0 - ct.height()) / 2.0).round(),
        );
        self.find_btn(
            sh,
            b,
            r.toggle,
            if f.replace { "\u{25BE}" } else { "\u{25B8}" },
            false,
            false,
        );
        self.find_btn(sh, b, r.regex, tr(sh.lang, Msg::FindRegex), f.regex, false);
        self.find_btn(sh, b, r.whole, tr(sh.lang, Msg::FindWhole), f.whole, false);
        self.find_btn(sh, b, r.case, tr(sh.lang, Msg::FindCase), f.case, false);
        if f.replace {
            let wr = Rect::new(r.with.x, r.with.y + 2.0, r.with.w, 24.0);
            self.find_field(sh, b, wr, &f.with, Msg::ReplacePlaceholder, f.field == 1);
            self.find_btn(sh, b, r.one, tr(sh.lang, Msg::ReplaceOne), false, true);
            self.find_btn(sh, b, r.all, tr(sh.lang, Msg::ReplaceAll), false, true);
        }
    }

    /// Match ranges and current index for `draw_doc`.
    pub(super) fn find_marks(&self) -> (&[Range<u32>], Option<usize>) {
        match &self.find {
            Some(f) if !f.query.is_empty() => (&f.matches, f.cur),
            _ => (&[], None),
        }
    }
}
