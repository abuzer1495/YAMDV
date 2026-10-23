//! Ctrl+K link editor popover in F2 (prompt 016 §3 item 4): Text, URL and Title fields for the
//! link at the caret (or the selection / word); Tab / Shift+Tab move between fields, Enter
//! applies as one undo step, Esc closes. Ctrl+V pastes into the field.

use super::*;
use crate::window::edit::Mode;
use yamdv_editor::linkedit::{self, Link};

const W: f32 = 340.0;
const ROW: f32 = 30.0;
const LABEL_W: f32 = 52.0;

/// Open popover state.
#[derive(Debug, Clone, Default)]
pub(in crate::window) struct LinkPop {
    /// Source range replaced on apply.
    pub range: Range<usize>,
    /// Text, URL, Title.
    pub fields: [String; 3],
    /// Focused field.
    pub focus: usize,
}

const LABELS: [(&str, &str); 3] = [("Text", "Metin"), ("URL", "URL"), ("Title", "Başlık")];

impl Win {
    /// Ctrl+K in F2: opens the editor for the link at the caret, or for the selection / word.
    pub(in crate::window) fn linkpop_open(&mut self) -> bool {
        let Some(e) = self.edit() else { return false };
        if e.mode != Some(Mode::Live) || e.active.is_none() {
            return false;
        }
        let ed = &e.ed;
        let sel = ed.selection();
        let l = ed.line_of(sel.start);
        let ls = ed.line_start(l);
        let line = ed.line_text(l);
        let found = linkedit::link_at(line.trim_end_matches(['\n', '\r']), ls, sel.start)
            .filter(|k| sel.end <= k.range.end);
        let link = found.unwrap_or_else(|| {
            let mut r = sel.clone();
            if r.is_empty() {
                let w = ed.word_at(sel.start);
                if ed.slice(w.clone()).chars().any(char::is_alphanumeric) {
                    r = w;
                }
            }
            Link {
                text: ed.slice(r.clone()),
                range: r,
                ..Link::default()
            }
        });
        let focus = usize::from(!link.text.is_empty());
        self.wysi.link = Some(LinkPop {
            range: link.range,
            fields: [link.text, link.url, link.title],
            focus,
        });
        self.invalidate();
        true
    }

    /// Writes the link back (one undo step) and closes.
    fn linkpop_apply(&mut self, sh: &Shared) {
        let Some(p) = self.wysi.link.take() else {
            return;
        };
        let [text, url, title] = p.fields;
        let text = if text.is_empty() { url.clone() } else { text };
        let md = linkedit::to_markdown(&Link {
            range: p.range.clone(),
            text,
            url,
            title,
        });
        let end = p.range.start + md.len();
        self.with_ed(sh, |ed| {
            ed.seal();
            ed.replace(p.range, &md, end);
            ed.seal();
        });
        self.invalidate();
    }

    /// Keys while open (all consumed).
    pub(in crate::window) fn linkpop_key(
        &mut self,
        sh: &Shared,
        c: Chord,
    ) -> Option<Option<Action>> {
        let p = self.wysi.link.as_mut()?;
        match c.vk {
            v if v == VK_ESCAPE.0 => self.wysi.link = None,
            v if v == VK_RETURN.0 => {
                self.linkpop_apply(sh);
                return Some(None);
            }
            v if v == VK_TAB.0 => {
                p.focus = if c.shift {
                    (p.focus + 2) % 3
                } else {
                    (p.focus + 1) % 3
                }
            }
            v if v == VK_DOWN.0 => p.focus = (p.focus + 1).min(2),
            v if v == VK_UP.0 => p.focus = p.focus.saturating_sub(1),
            0x08 => {
                if let Some(f) = p.fields.get_mut(p.focus) {
                    if c.ctrl {
                        f.clear();
                    } else {
                        f.pop();
                    }
                }
            }
            0x56 if c.ctrl => {
                let t = sys::paste_text(self.hwnd).unwrap_or_default();
                let t: String = t.chars().filter(|c| !c.is_control()).collect();
                if let Some(p) = self.wysi.link.as_mut()
                    && let Some(f) = p.fields.get_mut(p.focus)
                {
                    f.push_str(t.trim());
                }
            }
            _ => {}
        }
        self.invalidate();
        Some(None)
    }

    /// Typed character while open. True when consumed.
    pub(in crate::window) fn linkpop_char(&mut self, c: char) -> bool {
        let Some(p) = self.wysi.link.as_mut() else {
            return false;
        };
        if let Some(f) = p.fields.get_mut(p.focus) {
            f.push(c);
        }
        self.invalidate();
        true
    }

    /// Popover rectangle (content DIPs): under the caret line.
    fn linkpop_rect(&self, sh: &Shared) -> Option<Rect> {
        self.wysi.link.as_ref()?;
        let (ox, oy, p) = self.live_para(sh)?;
        let map = p.map.as_ref()?;
        let caret = self.edit()?.ed.caret();
        let c = p
            .shaped
            .caret(map.from_source(u32::try_from(caret).unwrap_or(0)));
        let h = ROW * 3.0 + 16.0 + 20.0;
        let x = (ox + c.x - 24.0)
            .clamp(8.0, (self.cw - W - 8.0).max(8.0))
            .round();
        let below = oy + c.y + c.h + 6.0;
        Some(Rect::new(x, below.round(), W, h))
    }

    /// Mouse down: focuses a field; inside = consumed; outside closes.
    pub(in crate::window) fn linkpop_down(&mut self, sh: &Shared, x: f32, y: f32) -> bool {
        let Some(r) = self.linkpop_rect(sh) else {
            return false;
        };
        if !r.contains(x, y) {
            self.wysi.link = None;
            self.invalidate();
            return false;
        }
        let k = ((y - r.y - 8.0) / ROW).floor();
        if let Some(p) = self.wysi.link.as_mut()
            && (0.0..3.0).contains(&k)
        {
            p.focus = k as usize;
        }
        self.invalidate();
        true
    }

    /// Paints the popover.
    pub(in crate::window) fn linkpop_draw(&self, sh: &Shared, b: &Band<'_>) {
        let (Some(r), Some(p)) = (self.linkpop_rect(sh), self.wysi.link.as_ref()) else {
            return;
        };
        let pal = &self.pal;
        let tok = yamdv_core::chrome::tokens(self.dark);
        let tr_ = matches!(sh.lang, Lang::Tr);
        b.fill(Rect::new(r.x, r.y + 2.0, r.w, r.h), 0x000000, 0.14, 9.0);
        b.fill(r, tok.menu_bg, 1.0, 8.0);
        b.stroke(r, tok.chrome_border, 8.0, 1.0);
        for (i, (en, trl)) in LABELS.iter().enumerate() {
            let y = r.y + 8.0 + ROW * i as f32;
            let lab = self.shape_ui(
                sh,
                if tr_ { trl } else { en },
                12.0,
                400,
                pal.text_muted,
                None,
            );
            b.text(
                lab.as_ref(),
                r.x + 10.0,
                (y + (ROW - lab.height()) / 2.0).round(),
            );
            let field = Rect::new(
                r.x + LABEL_W + 8.0,
                y + 3.0,
                r.w - LABEL_W - 18.0,
                ROW - 6.0,
            );
            b.fill(field, pal.bg, 1.0, 4.0);
            let border = if p.focus == i {
                pal.accent
            } else {
                tok.chrome_border
            };
            b.stroke(field, border, 4.0, 1.0);
            let s = p.fields.get(i).map_or("", String::as_str);
            let t = self.shape_ui(sh, s, 13.0, 400, pal.text, None);
            let tx = field.x + 6.0;
            let ty = (field.y + (field.h - t.height()) / 2.0).round();
            b.push_clip(field);
            b.text(t.as_ref(), tx, ty);
            if p.focus == i {
                let cx = (tx + t.width()).min(field.x + field.w - 4.0);
                b.fill(
                    Rect::new(cx.round(), field.y + 4.0, 1.0, field.h - 8.0),
                    pal.text,
                    1.0,
                    0.0,
                );
            }
            b.pop_clip();
        }
        let hint = if tr_ {
            "Enter uygula · Esc kapat · Tab alan"
        } else {
            "Enter apply · Esc close · Tab field"
        };
        let h = self.shape_ui(sh, hint, 11.0, 400, pal.text_muted, None);
        b.text(h.as_ref(), r.x + 10.0, (r.y + r.h - 22.0).round());
    }
}
