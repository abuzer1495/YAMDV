//! Code-fence language picker (prompt 016 §3 item 2): while the caret is in the opening fence's
//! info string of the F2 block, a list of matching languages (prefix first, then substring)
//! appears under it; ↑/↓ choose, Enter/Tab accept (one undo step), Esc dismisses.

use super::*;
use yamdv_core::doc::fence::FenceKind;

const ROW: f32 = 24.0;
const W: f32 = 180.0;
const MAX: usize = 8;

impl Win {
    /// Picker state: info range (absolute) and suggestions, when shown.
    pub(in crate::window) fn lang_pick(&self) -> Option<(Range<usize>, Vec<&'static str>)> {
        let e = self.edit()?;
        if e.mode != Some(crate::window::edit::Mode::Live)
            || self.wysi.lang_dismissed == Some(e.ed.version)
        {
            return None;
        }
        let (start, f) = self.active_fence()?;
        if f.kind == FenceKind::Math && f.info.is_empty() {
            return None;
        }
        let info = start + f.info.start..start + f.info.end;
        let sel = e.ed.selection();
        if !sel.is_empty() || sel.start < info.start || sel.start > info.end {
            return None;
        }
        let typed = e.ed.slice(info.start..sel.start);
        let items = yamdv_core::doc::fence::suggest(yamdv_highlight_names(), &typed, MAX);
        if items.is_empty() || (items.len() == 1 && items.first() == Some(&typed.as_str())) {
            return None;
        }
        Some((info, items))
    }

    /// List rectangle and row rectangles (content DIPs).
    fn lang_rects(&self, sh: &Shared, n: usize) -> Option<(Rect, Vec<Rect>)> {
        let (ox, oy, p) = self.live_para(sh)?;
        let map = p.map.as_ref()?;
        let caret = self.edit()?.ed.caret();
        let c = p
            .shaped
            .caret(map.from_source(u32::try_from(caret).unwrap_or(0)));
        let x = (ox + c.x).clamp(8.0, (self.cw - W - 8.0).max(8.0)).round();
        let y = (oy + c.y + c.h + 4.0).round();
        let r = Rect::new(x, y, W, ROW * n as f32 + 8.0);
        let rows = (0..n)
            .map(|i| Rect::new(x + 4.0, y + 4.0 + ROW * i as f32, W - 8.0, ROW))
            .collect();
        Some((r, rows))
    }

    /// Keys while the picker is shown.
    pub(in crate::window) fn lang_key(&mut self, sh: &Shared, c: Chord) -> Option<Option<Action>> {
        if c.ctrl || c.alt {
            return None;
        }
        let (info, items) = self.lang_pick()?;
        let n = items.len();
        let sel = self.wysi.lang_sel.min(n - 1);
        match c.vk {
            v if v == VK_DOWN.0 => self.wysi.lang_sel = (sel + 1) % n,
            v if v == VK_UP.0 => self.wysi.lang_sel = (sel + n - 1) % n,
            v if v == VK_ESCAPE.0 => {
                self.wysi.lang_dismissed = self.edit().map(|e| e.ed.version);
            }
            v if v == VK_RETURN.0 || v == VK_TAB.0 => {
                let name = items.get(sel).copied().unwrap_or("");
                self.lang_accept(sh, info, name);
            }
            _ => return None,
        }
        self.invalidate();
        Some(None)
    }

    fn lang_accept(&mut self, sh: &Shared, info: Range<usize>, name: &str) {
        self.wysi.lang_sel = 0;
        let end = info.start + name.len();
        self.with_ed(sh, |ed| {
            ed.replace(info, name, end);
            ed.seal();
        });
        self.wysi.lang_dismissed = self.edit().map(|e| e.ed.version);
    }

    /// Mouse down on a suggestion. True when the press was on the list.
    pub(in crate::window) fn lang_down(&mut self, sh: &Shared, x: f32, y: f32) -> bool {
        let Some((info, items)) = self.lang_pick() else {
            return false;
        };
        let Some((r, rows)) = self.lang_rects(sh, items.len()) else {
            return false;
        };
        if !r.contains(x, y) {
            return false;
        }
        if let Some(i) = rows.iter().position(|q| q.contains(x, y))
            && let Some(name) = items.get(i)
        {
            self.lang_accept(sh, info, name);
        }
        true
    }

    /// Paints the picker (content origin).
    pub(in crate::window) fn lang_draw(&self, sh: &Shared, b: &Band<'_>) {
        let Some((_, items)) = self.lang_pick() else {
            return;
        };
        let Some((r, rows)) = self.lang_rects(sh, items.len()) else {
            return;
        };
        let pal = &self.pal;
        let tok = yamdv_core::chrome::tokens(self.dark);
        b.fill(Rect::new(r.x, r.y + 2.0, r.w, r.h), 0x000000, 0.14, 9.0);
        b.fill(r, tok.menu_bg, 1.0, 8.0);
        b.stroke(r, tok.chrome_border, 8.0, 1.0);
        let sel = self.wysi.lang_sel.min(items.len() - 1);
        for (i, (name, row)) in items.iter().zip(rows).enumerate() {
            if i == sel {
                b.fill(row, pal.accent, 0.16, 4.0);
            }
            let t = self.shape_ui(sh, name, 13.0, 400, pal.text, None);
            b.text(
                t.as_ref(),
                row.x + 8.0,
                (row.y + (row.h - t.height()) / 2.0).round(),
            );
        }
    }

    /// Driver probe: the suggestions shown (`|`-joined) or empty.
    #[cfg(feature = "dev-tools")]
    pub(in crate::window) fn lang_probe(&self) -> String {
        self.lang_pick()
            .map(|(_, v)| v.join("|"))
            .unwrap_or_default()
    }
}

/// Every highlighter language name and alias.
fn yamdv_highlight_names() -> &'static [&'static str] {
    yamdv_layout::highlight_names()
}
