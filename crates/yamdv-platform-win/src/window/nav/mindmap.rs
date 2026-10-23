//! Mind map viewer (FUT-37): Ctrl+Shift+M / Export ▸ View as mind map. An opaque frame over the
//! document with [Right | Balanced]; pure tree + O(n) layout in `yamdv_ui::mindmap`. Wheel
//! zooms, drag pans, click collapses / expands, double-click jumps, Esc closes, Ctrl+C copies a
//! 1× PNG (≤ 8192 px per side). Only nodes inside the viewport are drawn.

use super::*;
use std::time::Instant;
use yamdv_ui::mindmap::{Map, Placed, Rect as MRect, connector, hues, layout};
use yamdv_ui::transient::ZoomView;

/// Leaf / branch node height and the root size padding.
const NODE_H: f32 = 28.0;
const ROOT_H: f32 = 40.0;
const MAX_W: f32 = 320.0;
const TOOL_H: f32 = 36.0;
const PAD: f32 = 24.0;

/// Open mind map.
pub(in crate::window) struct Mind {
    map: Map,
    widths: Vec<f32>,
    root: (f32, f32),
    placed: Placed,
    zoom: ZoomView,
    balanced: bool,
    /// Press: (x, y, node under it).
    press: Option<(f32, f32, Option<usize>)>,
    moved: bool,
    /// Last click (time, node) for double-click.
    last: Option<(Instant, usize)>,
    /// Build time (report-only gate).
    pub build_ms: f64,
}

impl Mind {
    fn relayout(&mut self) {
        self.placed = layout(&self.map, &self.widths, NODE_H, self.root, self.balanced);
        self.zoom.content = (self.placed.w + 2.0 * PAD, self.placed.h + 2.0 * PAD);
    }

    /// Node at content point (u, v).
    fn node_at(&self, u: f32, v: f32) -> Option<usize> {
        let (u, v) = (u - PAD, v - PAD);
        self.placed
            .rects
            .iter()
            .position(|r| r.is_some_and(|r| r.contains(u, v)))
    }
}

impl Win {
    /// Viewport of the map (content coordinates, below the toolbar).
    fn mind_view(&self) -> Rect {
        Rect::new(0.0, TOOL_H, self.cw, (self.ch - TOOL_H).max(1.0))
    }

    pub(super) fn mind_open(&mut self, sh: &Shared) {
        let t0 = Instant::now();
        let (src, path) = match &self.view {
            View::Doc(d) => (d.content.source().to_string(), d.path.clone()),
            _ => return,
        };
        let (h1, items) = yamdv_core::doc::mind::mind_items(&src);
        drop(src);
        let name = crate::window::sidebar::file_name(&path);
        let root_text = h1.unwrap_or(name);
        let map = Map::build(&root_text, Some(0), &items);
        let widths: Vec<f32> = map
            .nodes
            .iter()
            .map(|n| {
                (sh.dw.measure(&n.text, 13.0, 400, MAX_W).0 + 20.0)
                    .ceil()
                    .min(MAX_W + 20.0)
            })
            .collect();
        let rw = sh.dw.measure(&root_text, 15.0, 600, MAX_W).0 + 32.0;
        let v = self.mind_view();
        let mut m = Mind {
            map,
            widths,
            root: (rw.ceil(), ROOT_H),
            placed: Placed::default(),
            zoom: ZoomView::new((1.0, 1.0), (v.w, v.h)),
            balanced: false,
            press: None,
            moved: false,
            last: None,
            build_ms: 0.0,
        };
        m.relayout();
        m.zoom = ZoomView::new(m.zoom.content, (v.w, v.h));
        if m.zoom.scale > 1.0 {
            m.zoom.scale = 1.0;
            m.zoom.ox = (v.w - m.zoom.content.0) / 2.0;
            m.zoom.oy = (v.h - m.zoom.content.1) / 2.0;
        }
        m.build_ms = t0.elapsed().as_secs_f64() * 1000.0;
        self.nav.palette = None;
        self.nav.mind = Some(Box::new(m));
        self.invalidate();
    }

    fn mind_close(&mut self) {
        self.nav.mind = None;
        sys::trim_once();
        self.invalidate();
    }

    pub(super) fn mind_mode(&mut self, balanced: bool) {
        let v = self.mind_view();
        if let Some(m) = &mut self.nav.mind
            && m.balanced != balanced
        {
            m.balanced = balanced;
            m.relayout();
            let s = m.zoom.scale;
            m.zoom = ZoomView::new(m.zoom.content, (v.w, v.h));
            m.zoom.scale = s.min(m.zoom.scale.max(s));
            self.invalidate();
        }
    }

    /// Double-click: jump to the node's place in the document.
    fn mind_jump(&mut self, sh: &Shared, node: usize) {
        let off = self
            .nav
            .mind
            .as_ref()
            .and_then(|m| m.map.nodes.get(node))
            .and_then(|n| n.offset);
        self.mind_close();
        let Some(off) = off else { return };
        let Some(d) = self.doc() else { return };
        let off = u32::try_from(off).unwrap_or(u32::MAX);
        let b = (0..d.content.len())
            .rfind(|&i| d.content.range(i).start <= off)
            .unwrap_or(0);
        self.tz_reveal_offset(off);
        self.scroll_to_block(sh, b);
    }

    pub(super) fn mind_key(&mut self, sh: &Shared, vk: u16, ctrl: bool) {
        let v = self.mind_view();
        let Some(m) = &mut self.nav.mind else { return };
        match vk {
            0x1B => self.mind_close(),
            0x43 if ctrl => self.mind_copy(sh),
            0xBB | 0x6B => m.zoom.step(1),
            0xBD | 0x6D => m.zoom.step(-1),
            0x30 | 0x60 => {
                m.zoom = ZoomView::new(m.zoom.content, (v.w, v.h));
            }
            0x25 => m.zoom.pan(40.0, 0.0),
            0x27 => m.zoom.pan(-40.0, 0.0),
            0x26 => m.zoom.pan(0.0, 40.0),
            0x28 => m.zoom.pan(0.0, -40.0),
            _ => {}
        }
        self.invalidate();
    }

    /// Toolbar buttons: (rect, id) — 0 Right, 1 Balanced, 2 Copy, 3 Close.
    fn mind_tools(&self, sh: &Shared) -> Vec<(Rect, u8, String)> {
        let labels = [
            tr(sh.lang, Msg::MindRight).to_string(),
            tr(sh.lang, Msg::MindBalanced).to_string(),
            "\u{29C9}".to_string(),
            "\u{00D7}".to_string(),
        ];
        let widths: Vec<f32> = labels
            .iter()
            .map(|l| sh.dw.measure(l, 13.0, 600, 200.0).0 + 24.0)
            .collect();
        let total: f32 = widths.iter().sum::<f32>() + 12.0;
        let mut x = ((self.cw - total) / 2.0).round();
        let mut out = Vec::new();
        for (i, (l, w)) in labels.into_iter().zip(widths).enumerate() {
            if i == 2 {
                x += 12.0;
            }
            out.push((Rect::new(x, 4.0, w, TOOL_H - 8.0), i as u8, l));
            x += w;
        }
        out
    }

    pub(super) fn mind_down(&mut self, sh: &Shared, x: f32, y: f32) {
        if let Some((_, id, _)) = self.mind_tools(sh).into_iter().find(|t| t.0.contains(x, y)) {
            match id {
                0 => self.mind_mode(false),
                1 => self.mind_mode(true),
                2 => self.mind_copy(sh),
                _ => self.mind_close(),
            }
            return;
        }
        let v = self.mind_view();
        let Some(m) = &mut self.nav.mind else { return };
        let (u, w) = m.zoom.to_content(x - v.x, y - v.y);
        let hit = m.node_at(u, w);
        m.press = Some((x, y, hit));
        m.moved = false;
        // SAFETY: valid hwnd.
        unsafe { SetCapture(self.hwnd) };
    }

    pub(super) fn mind_move(&mut self, _sh: &Shared, x: f32, y: f32) {
        let Some(m) = &mut self.nav.mind else { return };
        let Some((px, py, n)) = m.press else { return };
        if !m.moved && (x - px).abs() + (y - py).abs() < 4.0 {
            return;
        }
        m.moved = true;
        m.zoom.pan(x - px, y - py);
        m.press = Some((x, y, n));
        self.invalidate();
    }

    pub(super) fn mind_up(&mut self) {
        // SAFETY: no preconditions.
        let _ = unsafe { ReleaseCapture() };
        let Some(sh) = shared() else { return };
        let Some(m) = &mut self.nav.mind else { return };
        let (Some((_, _, Some(n))), false) = (m.press.take(), m.moved) else {
            return;
        };
        // SAFETY: no preconditions.
        let dbl = u128::from(unsafe { GetDoubleClickTime() });
        let double = m
            .last
            .is_some_and(|(t, k)| k == n && t.elapsed().as_millis() <= dbl);
        if double {
            // The first click toggled; undo it, then jump.
            m.map.toggle(n);
            m.last = None;
            self.mind_jump(&sh, n);
            return;
        }
        m.last = Some((Instant::now(), n));
        if m.map.toggle(n) {
            m.relayout();
        }
        self.invalidate();
    }

    pub(super) fn mind_wheel(&mut self, delta: i16, x: f32, y: f32) {
        let v = self.mind_view();
        if let Some(m) = &mut self.nav.mind {
            m.zoom.wheel(f32::from(delta) / 120.0, x - v.x, y - v.y);
            self.invalidate();
        }
    }
}

impl Win {
    /// Draws the map with content origin `(ox, oy)` and `scale`; culls to `clip`.
    fn mind_paint(
        &self,
        sh: &Shared,
        b: &Band<'_>,
        m: &Mind,
        (ox, oy): (f32, f32),
        scale: f32,
        clip: Rect,
    ) {
        let pal = &self.pal;
        let hue = hues(pal.accent);
        let tf = |r: MRect| {
            Rect::new(
                ox + (r.x + PAD) * scale,
                oy + (r.y + PAD) * scale,
                r.w * scale,
                r.h * scale,
            )
        };
        let vis = |r: &Rect| {
            r.x < clip.x + clip.w
                && r.x + r.w > clip.x
                && r.y < clip.y + clip.h
                && r.y + r.h > clip.y
        };
        let rect = |i: usize| m.placed.rects.get(i).copied().flatten();
        // Connectors first.
        for (i, n) in m.map.nodes.iter().enumerate() {
            let (Some(c), Some(p)) = (rect(i), n.parent.and_then(rect)) else {
                continue;
            };
            let [s, c1, c2, e] = connector(p, c);
            let bx = Rect::new(
                s.0.min(e.0),
                s.1.min(e.1),
                (s.0 - e.0).abs(),
                (s.1 - e.1).abs().max(1.0),
            );
            if !vis(&tf(MRect {
                x: bx.x,
                y: bx.y,
                w: bx.w,
                h: bx.h,
            })) {
                continue;
            }
            let pt = |q: (f32, f32)| (ox + (q.0 + PAD) * scale, oy + (q.1 + PAD) * scale);
            let (s, c1, c2, e) = (pt(s), pt(c1), pt(c2), pt(e));
            let col = hue
                .get(usize::from(n.branch))
                .copied()
                .unwrap_or(pal.accent);
            b.path(
                &[
                    yamdv_mermaid::Seg::M(s.0, s.1),
                    yamdv_mermaid::Seg::C(c1.0, c1.1, c2.0, c2.1, e.0, e.1),
                ],
                (0.0, 0.0),
                None,
                Some((col, (1.5 * scale).max(0.75), false)),
            );
        }
        for (i, n) in m.map.nodes.iter().enumerate() {
            let Some(r) = rect(i).map(tf) else { continue };
            if !vis(&r) {
                continue;
            }
            let col = hue
                .get(usize::from(n.branch))
                .copied()
                .unwrap_or(pal.accent);
            let (size, weight, fg) = if i == 0 {
                b.fill(r, pal.accent, 1.0, 10.0 * scale);
                (15.0, 600, pal.bg)
            } else if n.leaf {
                b.fill(r, pal.code_bg, 1.0, 6.0 * scale);
                (13.0, 400, pal.text)
            } else {
                b.fill(r, col, 0.16, 6.0 * scale);
                b.stroke(r, col, 6.0 * scale, 1.0);
                (13.0, 600, pal.text)
            };
            if scale >= 0.3 {
                let t = self.shape_ui(sh, &n.text, size * scale, weight, fg, None);
                b.push_clip(r);
                b.text(
                    t.as_ref(),
                    (r.x + (if i == 0 { 16.0 } else { 10.0 }) * scale).round(),
                    (r.y + (r.h - t.height()) / 2.0).round(),
                );
                b.pop_clip();
            }
            if n.collapsed && !n.kids.is_empty() {
                let c = m.map.hidden_count(i);
                let t = self.shape_ui(sh, &c.to_string(), 10.0, 600, pal.bg, None);
                let w = t.width() + 10.0;
                let br = Rect::new(r.x + r.w - w / 2.0, r.y - 7.0, w, 14.0);
                b.fill(br, col, 1.0, 7.0);
                b.text(
                    t.as_ref(),
                    (br.x + 5.0).round(),
                    (br.y + (14.0 - t.height()) / 2.0).round(),
                );
            }
        }
    }

    pub(super) fn mind_draw(&self, sh: &Shared, b: &Band<'_>) {
        let Some(m) = &self.nav.mind else { return };
        let pal = &self.pal;
        // Opaque frame over the document.
        b.fill(Rect::new(0.0, 0.0, self.cw, self.ch), pal.bg, 1.0, 0.0);
        let v = self.mind_view();
        b.push_clip(v);
        self.mind_paint(
            sh,
            b,
            m,
            (v.x + m.zoom.ox, v.y + m.zoom.oy),
            m.zoom.scale,
            v,
        );
        b.pop_clip();
        b.fill(
            Rect::new(0.0, TOOL_H - 1.0, self.cw, 1.0),
            pal.border_subtle,
            1.0,
            0.0,
        );
        for (r, id, label) in self.mind_tools(sh) {
            let on = (id == 1) == m.balanced && id < 2;
            if on {
                b.fill(r, pal.accent, 0.16, 6.0);
            }
            let t = self.shape_ui(
                sh,
                &label,
                13.0,
                600,
                if on { pal.accent } else { pal.text },
                None,
            );
            b.text(
                t.as_ref(),
                (r.x + (r.w - t.width()) / 2.0).round(),
                (r.y + (r.h - t.height()) / 2.0).round(),
            );
        }
    }

    /// Ctrl+C: the whole map as a 1× PNG (≤ 8192 px per side).
    fn mind_copy(&mut self, sh: &Shared) {
        let Some(m) = &self.nav.mind else { return };
        #[allow(clippy::cast_possible_truncation)]
        let (w, h) = (
            m.zoom.content.0.ceil().clamp(1.0, 8192.0) as i32,
            m.zoom.content.1.ceil().clamp(1.0, 8192.0) as i32,
        );
        let Ok(r) = Renderer::new(&sh.dw, None, w, 1.0) else {
            return;
        };
        let clip = Rect::new(0.0, 0.0, w as f32, h as f32);
        let mut px = r.render_offscreen(h, self.pal.text, &mut |band| {
            band.clear(self.pal.bg);
            self.mind_paint(sh, band, m, (0.0, 0.0), 1.0, clip);
        });
        for p in px.as_chunks_mut::<4>().0 {
            p[3] = 255;
        }
        #[allow(clippy::cast_sign_loss)]
        let (w, h) = (w as u32, h as u32);
        let png = crate::snapshot::encode_png(w, h, &px).ok();
        if sys::copy_image(self.hwnd, w, h, &px, png.as_deref()) {
            self.toast(Msg::MindCopied);
        }
    }
}

#[cfg(feature = "dev-tools")]
impl Win {
    pub(super) fn mind_state(&self) -> Vec<(&'static str, String)> {
        match &self.nav.mind {
            Some(m) => vec![
                ("mind", if m.balanced { "balanced" } else { "right" }.into()),
                ("mind_nodes", m.map.nodes.len().to_string()),
                ("mind_visible", m.map.visible_count().to_string()),
                ("mind_build_ms", format!("{:.1}", m.build_ms)),
            ],
            None => vec![("mind", "none".into()), ("mind_nodes", "0".into())],
        }
    }

    /// Driver: double-click node `text` (jump), or click it (toggle) when `jump` is false.
    pub(in crate::window) fn mind_click_text(
        &mut self,
        sh: &Shared,
        text: &str,
        jump: bool,
    ) -> bool {
        let v = self.mind_view();
        let Some(m) = &self.nav.mind else {
            return false;
        };
        let Some(i) = m.map.nodes.iter().position(|n| n.text == text) else {
            return false;
        };
        let Some(r) = m.placed.rects.get(i).copied().flatten() else {
            return false;
        };
        let x = v.x + m.zoom.ox + (r.x + PAD + r.w / 2.0) * m.zoom.scale;
        let y = v.y + m.zoom.oy + (r.y + PAD + r.h / 2.0) * m.zoom.scale;
        let clicks = if jump { 2 } else { 1 };
        for _ in 0..clicks {
            self.mind_down(sh, x, y);
            self.mind_up();
        }
        true
    }
}
