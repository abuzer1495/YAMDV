//! Fullscreen viewers (B.11, FUT-36): images, Mermaid diagrams and tables. Hover shows a ⛶
//! badge (tables: a [⛶][CSV] pill); a double-click opens. Scrim .82, fit ≤ 92 %, toolbar
//! pill top right; the wheel zooms at the cursor (10–800 %), dragging pans. Built on demand,
//! freed on close.

use super::*;

const TOOLBAR_H: f32 = 36.0;
const TABLE_HEAD: f32 = 28.0;
const GRIP: f32 = 14.0;
const SCRIM: f32 = 0.82;
/// Diagram card width of the standalone layout (the HTML export column).
const DIAGRAM_W: f32 = 812.0;

enum FvKind {
    Image {
        bmp: crate::images::Bitmap,
        src: yamdv_core::image::Source,
        nat: (u32, u32),
    },
    Block {
        viewer: Box<Viewer>,
        csv: Option<String>,
    },
}

#[derive(Debug, Clone, Copy)]
enum Drag {
    Pan(f32, f32),
    Move(f32, f32),
    Grip(f32, f32),
}

type Frame = (Rect, f32, f32, f32, u32, u32, Vec<u8>);

/// An open viewer.
pub(super) struct FullView {
    kind: FvKind,
    /// Pan / zoom; the viewport is the content area, or the table card body.
    z: ZoomView,
    #[cfg_attr(not(feature = "dev-tools"), allow(dead_code))]
    what: &'static str,
    /// Table card (content DIPs).
    card: Option<Rect>,
    /// "Table · R × C".
    title: String,
    drag: Option<Drag>,
    hover: Option<usize>,
    /// Last rendered frame of a block viewer (keyed by viewport and zoom).
    cache: RefCell<Option<Frame>>,
}

impl FullView {
    fn new(kind: FvKind, z: ZoomView, what: &'static str) -> Self {
        Self {
            kind,
            z,
            what,
            card: None,
            title: String::new(),
            drag: None,
            hover: None,
            cache: RefCell::new(None),
        }
    }

    pub(super) fn end_drag(&mut self) {
        self.drag = None;
    }

    pub(super) fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    #[cfg(feature = "dev-tools")]
    pub(super) fn percent(&self) -> u32 {
        self.z.percent()
    }

    /// Driver description: `image:fit`, `diagram`, `table:Table · 3 × 2` …
    #[cfg(feature = "dev-tools")]
    pub(super) fn describe(&self) -> String {
        if self.card.is_some() {
            format!("{}:{}", self.what, self.title)
        } else if self.z.is_fit() {
            format!("{}:fit", self.what)
        } else {
            self.what.to_string()
        }
    }
}

impl Win {
    /// Badge rectangles of the hovered media (content DIPs): (⛶, CSV).
    fn media_badges(&self, sh: &Shared, m: &Media) -> (Rect, Option<Rect>) {
        let r = self.media_win_rect(sh, m);
        if let MediaKind::Table(..) = m.kind {
            let x = r.right() - 8.0 - 76.0;
            (
                Rect::new(x, r.y + 4.0, 32.0, 24.0),
                Some(Rect::new(x + 32.0, r.y + 4.0, 44.0, 24.0)),
            )
        } else {
            (Rect::new(r.right() - 36.0, r.y + 8.0, 28.0, 28.0), None)
        }
    }

    /// The F2 table tool strip carries ⛶ / CSV while a table is being edited (R-TBL-03).
    fn table_badge_hidden(&self, m: &Media) -> bool {
        matches!(m.kind, MediaKind::Table(..)) && self.active_table().is_some()
    }

    /// ⛶ (`csv` false) or CSV export of the active F2 table, from its tool strip.
    pub(in crate::window) fn active_table_media(&mut self, sh: &Shared, csv: bool) {
        let Some(r) = self.active_table_rect(sh) else {
            return;
        };
        if csv {
            if let Some(t) = self.table_csv_at(sh, r.x + 4.0, r.y + r.h / 2.0) {
                self.actions.push(Action::Export(ExportKind::Csv(t)));
            }
            return;
        }
        let View::Doc(d) = &self.view else { return };
        let Some(bi) = d.edit.as_deref().and_then(|e| e.active).map(|a| a.block) else {
            return;
        };
        let kind = d
            .viewer
            .layout_of(bi)
            .and_then(|bl| bl.hot.iter().find_map(|h| MediaKind::of(&h.kind)));
        if let Some(k @ MediaKind::Table(..)) = kind {
            self.fv_open(sh, bi, &k);
        }
    }

    /// `Some(false)` = ⛶, `Some(true)` = CSV.
    pub(super) fn media_badge_hit(&self, sh: &Shared, m: &Media, x: f32, y: f32) -> Option<bool> {
        if self.table_badge_hidden(m) {
            return None;
        }
        let (open, csv) = self.media_badges(sh, m);
        if open.contains(x, y) {
            Some(false)
        } else {
            csv.filter(|c| c.contains(x, y)).map(|_| true)
        }
    }

    fn media_win_rect(&self, sh: &Shared, m: &Media) -> Rect {
        let (cx, _) = self.col(sh);
        let View::Doc(d) = &self.view else {
            return Rect::default();
        };
        let top = (d.viewer.block_top(&d.content, m.bi) - d.viewer.scroll) as f32;
        Rect::new(cx + m.rect.x, top + m.rect.y, m.rect.w, m.rect.h)
    }

    pub(super) fn draw_media_badge(&self, sh: &Shared, b: &Band<'_>) {
        let Some(m) = &self.tz.media else { return };
        if self.tz.fv.is_some() || self.tz.pop.is_some() || self.table_badge_hidden(m) {
            return;
        }
        let (open, csv) = self.media_badges(sh, m);
        let pill = csv.map_or(open, |c| Rect::new(open.x, open.y, open.w + c.w, open.h));
        b.fill(pill, 0x202020, 0.75, 6.0);
        let g = self.shape_ui(sh, "⛶", 14.0, 400, 0xffffff, None);
        center(b, g.as_ref(), open);
        if let Some(c) = csv {
            b.fill(
                Rect::new(c.x, c.y + 5.0, 1.0, c.h - 10.0),
                0xffffff,
                0.4,
                0.0,
            );
            let t = self.shape_ui(sh, "CSV", 11.0, 600, 0xffffff, None);
            center(b, t.as_ref(), c);
        }
    }

    /// Press on the content: ⛶ / CSV badges, or a double-click on media.
    pub(super) fn media_down(&mut self, sh: &Shared, x: f32, y: f32) -> bool {
        let Some(m) = self.tz.media.clone() else {
            self.tz.last_down = Some((Instant::now(), x, y));
            return false;
        };
        match self.media_badge_hit(sh, &m, x, y) {
            Some(false) => {
                self.fv_open(sh, m.bi, &m.kind);
                return true;
            }
            Some(true) => {
                let r = self.media_win_rect(sh, &m);
                if let Some(t) = self.table_csv_at(sh, r.x + 4.0, r.y + r.h / 2.0) {
                    self.actions.push(Action::Export(ExportKind::Csv(t)));
                }
                return true;
            }
            None => {}
        }
        if self.double_click(x, y) && self.media_win_rect(sh, &m).contains(x, y) {
            self.fv_open(sh, m.bi, &m.kind);
            return true;
        }
        false
    }

    /// Source of detail node `node` of block `bi`.
    fn node_source(&mut self, bi: usize, node: usize) -> Option<String> {
        let View::Doc(d) = &mut self.view else {
            return None;
        };
        let Content::Markdown(doc) = &mut d.content else {
            return None;
        };
        let r = doc.detail(bi).nodes.get(node)?.range.clone();
        doc.source
            .get(r.start as usize..r.end as usize)
            .map(str::to_string)
    }

    /// `md` laid out on its own at `width` (block 0 is the media).
    fn standalone(&self, sh: &Shared, md: &str, width: f32) -> (Content, Viewer) {
        let mut content = Content::Markdown(Document::new(md.to_string(), sh.read));
        let mut viewer = Viewer::new(&content, width, sh.m);
        let path = match &self.view {
            View::Doc(d) => d.path.clone(),
            _ => String::new(),
        };
        let images = crate::images::Store::synchronous(&path);
        let cache = yamdv_layout::rich::RichCache::default();
        let rich = crate::dwrite::Rich::new(&sh.dw, &cache, true, true);
        crate::export::layout_all(
            &sh.dw,
            &mut content,
            &mut viewer,
            &self.pal,
            &images,
            &rich,
            sh.lang,
            sh.m,
        );
        (content, viewer)
    }

    fn open_image(&self, src: &str) -> Option<FullView> {
        let View::Doc(d) = &self.view else {
            return None;
        };
        let (source, nw, nh) = d.images.source(src)?;
        let f = crate::images::factory()?;
        // Decoded at most at twice the viewport (pixels); zoom beyond scales it.
        let cap = (2.0 * self.cw.max(self.ch) * self.scale).max(256.0);
        let k = (cap / nw.max(nh).max(1) as f32).min(1.0);
        let w = ((nw as f32 * k).round() as u32).max(1);
        let h = ((nh as f32 * k).round() as u32).max(1);
        let bmp = crate::images::decode(&f, &source, w, h).ok()?;
        let z = ZoomView::new((nw as f32, nh as f32), (self.cw, self.ch));
        Some(FullView::new(
            FvKind::Image {
                bmp,
                src: source,
                nat: (nw, nh),
            },
            z,
            "image",
        ))
    }

    fn open_diagram(&mut self, sh: &Shared, bi: usize, node: usize) -> Option<FullView> {
        let md = self.node_source(bi, node)?;
        let (_, mut viewer) = self.standalone(sh, &md, DIAGRAM_W);
        // A diagram wider than its card is laid out again at its own width (scale 1).
        let wide = viewer
            .layout_of(0)
            .and_then(|bl| {
                bl.regions
                    .iter()
                    .map(|r| r.content_w - r.view.w)
                    .reduce(f32::max)
            })
            .unwrap_or(0.0);
        if wide > 1.0 {
            viewer = self.standalone(sh, &md, DIAGRAM_W + wide + 2.0).1;
        }
        let h = viewer.layout_of(0)?.height;
        let z = ZoomView::new((viewer.width, h), (self.cw, self.ch));
        Some(FullView::new(
            FvKind::Block {
                viewer: Box::new(viewer),
                csv: None,
            },
            z,
            "diagram",
        ))
    }

    fn open_table(
        &mut self,
        sh: &Shared,
        bi: usize,
        node: usize,
        rows: u32,
        cols: u32,
    ) -> Option<FullView> {
        let md = self.node_source(bi, node)?;
        // Natural column widths: laid out wide, then at the table's own width.
        let nat = self
            .standalone(sh, &md, 4000.0)
            .1
            .layout_of(0)
            .and_then(|bl| {
                bl.hot
                    .iter()
                    .find_map(|h| matches!(h.kind, HotKind::Table { .. }).then(|| h.rect.right()))
            })
            .unwrap_or(600.0);
        let (content, viewer) = self.standalone(sh, &md, nat.ceil() + 2.0);
        let csv = match &content {
            Content::Markdown(doc) => crate::export::table_csv(doc, 0, 0),
            Content::Plain { .. } => None,
        };
        let size = (viewer.width, viewer.layout_of(0)?.height);
        let w = (size.0 + 2.0).min(self.cw * 0.92).max(240.0);
        let h = (size.1 + TABLE_HEAD + 2.0).min(self.ch * 0.92).max(120.0);
        let card = Rect::new(
            ((self.cw - w) / 2.0).round(),
            ((self.ch - h) / 2.0).round(),
            w,
            h,
        );
        let mut z = ZoomView::new(size, (card.w, card.h - TABLE_HEAD));
        z.actual();
        z.ox = 0.0;
        z.oy = 0.0;
        let mut fv = FullView::new(
            FvKind::Block {
                viewer: Box::new(viewer),
                csv,
            },
            z,
            "table",
        );
        fv.card = Some(card);
        fv.title = tr(sh.lang, Msg::ViewTable)
            .replace("{r}", &rows.to_string())
            .replace("{c}", &cols.to_string());
        Some(fv)
    }

    /// Opens the viewer for media `kind` of block `bi`.
    pub(super) fn fv_open(&mut self, sh: &Shared, bi: usize, kind: &MediaKind) {
        self.tz_tip_hide();
        self.tz.pop = None;
        let fv = match kind {
            MediaKind::Image(src) => self.open_image(src),
            MediaKind::Diagram(node) => self.open_diagram(sh, bi, *node),
            MediaKind::Table(node, rows, cols) => self.open_table(sh, bi, *node, *rows, *cols),
        };
        if let Some(fv) = fv {
            self.tz.fv = Some(Box::new(fv));
            self.tz.media = None;
            self.set_status(sh, None);
        }
        self.invalidate();
    }

    /// Closes the viewer and gives its memory back.
    pub(in crate::window) fn fv_close(&mut self) {
        if self.tz.fv.take().is_some() {
            sys::trim_once();
            self.invalidate();
        }
    }

    /// Body viewport (content DIPs).
    fn fv_body(&self) -> Rect {
        match self.tz.fv.as_ref().and_then(|f| f.card) {
            Some(c) => Rect::new(c.x, c.y + TABLE_HEAD, c.w, c.h - TABLE_HEAD),
            None => Rect::new(0.0, 0.0, self.cw, self.ch),
        }
    }

    /// Toolbar pill (12 px inset, 36 high): [−][100 %][+][fit][1:1][copy][×].
    fn fv_toolbar(&self) -> (Rect, Vec<(Rect, usize)>) {
        const W: [f32; 7] = [36.0, 60.0, 36.0, 40.0, 40.0, 36.0, 36.0];
        let w: f32 = W.iter().sum::<f32>() + 8.0;
        let bar = Rect::new(self.cw - 12.0 - w, 12.0, w, TOOLBAR_H);
        let mut x = bar.x + 4.0;
        let mut out = Vec::new();
        for (i, bw) in W.into_iter().enumerate() {
            out.push((Rect::new(x, bar.y, bw, TOOLBAR_H), i));
            x += bw;
        }
        (bar, out)
    }

    fn fv_button(&mut self, sh: &Shared, i: usize) {
        let Some(fv) = &mut self.tz.fv else { return };
        match i {
            0 => fv.z.step(-1),
            1 => fv.z.toggle(),
            2 => fv.z.step(1),
            3 => fv.z.fit(),
            4 => fv.z.actual(),
            5 => return self.fv_copy(sh),
            _ => return self.fv_close(),
        }
        self.invalidate();
    }

    /// Copy: image → bitmap + PNG, diagram → PNG (2×), table → CSV.
    fn fv_copy(&mut self, sh: &Shared) {
        let Some(fv) = &self.tz.fv else { return };
        let ok = match &fv.kind {
            FvKind::Image { src, nat, .. } => crate::images::factory()
                .and_then(|f| crate::images::decode(&f, src, nat.0.min(8192), nat.1.min(8192)).ok())
                .is_some_and(|bmp| {
                    let png = crate::snapshot::encode_png(bmp.w, bmp.h, &bmp.px).ok();
                    sys::copy_image(self.hwnd, bmp.w, bmp.h, &bmp.px, png.as_deref())
                }),
            FvKind::Block { csv: Some(t), .. } => sys::copy_text(self.hwnd, t),
            FvKind::Block { viewer, csv: None } => {
                self.diagram_png(sh, viewer).is_some_and(|(w, h, px)| {
                    let png = crate::snapshot::encode_png(w, h, &px).ok();
                    sys::copy_image(self.hwnd, w, h, &px, png.as_deref())
                })
            }
        };
        let m = if fv.card.is_some() {
            Msg::CopiedCsv
        } else {
            Msg::CopiedImage
        };
        if ok {
            self.toast(m);
        }
    }

    /// The whole diagram at scale 2, opaque BGRA.
    fn diagram_png(&self, sh: &Shared, viewer: &Viewer) -> Option<(u32, u32, Vec<u8>)> {
        let bl = viewer.layout_of(0)?;
        let scale = 2.0f32;
        let w_px = (viewer.width * scale).round().clamp(1.0, 8192.0) as i32;
        let h_px = (bl.height.max(1.0) * scale).round().clamp(1.0, 8192.0) as i32;
        let r = Renderer::new(&sh.dw, None, w_px, scale).ok()?;
        let mut px = r.render_offscreen(h_px, self.pal.text, &mut |band| {
            band.clear(self.pal.bg);
            draw_block(
                band,
                bl,
                &|reg| viewer.hscroll(0, reg),
                &self.pal,
                (0.0, 0.0),
                None,
                None,
                (&[], None),
            );
        });
        for p in px.as_chunks_mut::<4>().0 {
            p[3] = 255;
        }
        Some((w_px as u32, h_px as u32, px))
    }

    /// Draws the viewer (content DIPs).
    pub(super) fn fv_draw(&self, sh: &Shared, b: &Band<'_>) {
        let Some(fv) = &self.tz.fv else { return };
        b.fill(Rect::new(0.0, 0.0, self.cw, self.ch), 0x000000, SCRIM, 0.0);
        let body = self.fv_body();
        if let Some(c) = fv.card {
            shadow(b, c, 8.0, self.dark);
            b.fill(c, self.pal.bg, 1.0, 8.0);
            b.stroke(c, self.pal.border_subtle, 8.0, 1.0);
            b.fill(
                Rect::new(c.x, c.y + TABLE_HEAD - 1.0, c.w, 1.0),
                self.pal.border_subtle,
                1.0,
                0.0,
            );
            let t = self.shape_ui(sh, &fv.title, 13.0, 600, self.pal.text, None);
            b.text(
                t.as_ref(),
                c.x + 12.0,
                (c.y + (TABLE_HEAD - t.height()) / 2.0).round(),
            );
            let e = self.shape_ui(
                sh,
                tr(sh.lang, Msg::ExportCsv),
                12.0,
                400,
                self.pal.link,
                None,
            );
            b.text(
                e.as_ref(),
                (c.right() - 12.0 - e.width()).round(),
                (c.y + (TABLE_HEAD - e.height()) / 2.0).round(),
            );
        }
        let dst = Rect::new(
            body.x + fv.z.ox,
            body.y + fv.z.oy,
            fv.z.content.0 * fv.z.scale,
            fv.z.content.1 * fv.z.scale,
        );
        b.push_clip(body);
        match &fv.kind {
            FvKind::Image { bmp, .. } => b.image_scaled(dst, bmp.w, bmp.h, &bmp.px),
            FvKind::Block { viewer, .. } => {
                if fv.card.is_none() {
                    b.fill(dst, self.pal.bg, 1.0, 8.0 * fv.z.scale);
                }
                self.fv_blit(sh, b, fv, viewer, body);
            }
        }
        b.pop_clip();
        if let Some(c) = fv.card {
            self.card_bars(b, fv, c, body);
        }
        let (bar, btns) = self.fv_toolbar();
        b.fill(bar, 0x202020, 0.9, TOOLBAR_H / 2.0);
        for (r, i) in btns {
            if fv.hover == Some(i) {
                b.fill(
                    Rect::new(r.x + 2.0, r.y + 4.0, r.w - 4.0, r.h - 8.0),
                    0xffffff,
                    0.15,
                    6.0,
                );
            }
            let label = match i {
                0 => "−".to_string(),
                1 => format!("{} %", fv.z.percent()),
                2 => "+".into(),
                3 => "⤢".into(),
                4 => "1:1".into(),
                5 => "⧉".into(),
                _ => "✕".into(),
            };
            let t = self.shape_ui(
                sh,
                &label,
                if i == 1 || i == 4 { 13.0 } else { 14.0 },
                400,
                0xffffff,
                None,
            );
            center(b, t.as_ref(), r);
        }
    }

    /// Table card scroll indicators and the 14 px resize grip.
    fn card_bars(&self, b: &Band<'_>, fv: &FullView, c: Rect, body: Rect) {
        let (vw, vh) = fv.z.view;
        let (cw, chh) = (fv.z.content.0 * fv.z.scale, fv.z.content.1 * fv.z.scale);
        let muted = self.pal.text_muted;
        if cw > vw + 0.5 {
            let tw = (vw * vw / cw).max(24.0);
            let tx = (-fv.z.ox / (cw - vw)).clamp(0.0, 1.0) * (vw - tw);
            b.fill(
                Rect::new(body.x + tx, c.bottom() - 6.0, tw, 4.0),
                muted,
                0.5,
                2.0,
            );
        }
        if chh > vh + 0.5 {
            let th = (vh * vh / chh).max(24.0);
            let ty = (-fv.z.oy / (chh - vh)).clamp(0.0, 1.0) * (vh - th);
            b.fill(
                Rect::new(c.right() - 6.0, body.y + ty, 4.0, th),
                muted,
                0.5,
                2.0,
            );
        }
        for i in 0..3 {
            let o = 4.0 + i as f32 * 4.0;
            b.fill(
                Rect::new(c.right() - o, c.bottom() - 3.0, 2.0, 2.0),
                muted,
                1.0,
                0.0,
            );
            b.fill(
                Rect::new(c.right() - 3.0, c.bottom() - o, 2.0, 2.0),
                muted,
                1.0,
                0.0,
            );
        }
    }

    /// Renders the visible part of a block viewer at the current zoom (cached) and blits it.
    fn fv_blit(&self, sh: &Shared, b: &Band<'_>, fv: &FullView, viewer: &Viewer, body: Rect) {
        let Some(bl) = viewer.layout_of(0) else {
            return;
        };
        let fresh =
            fv.cache.borrow().as_ref().is_some_and(|c| {
                c.0 == body && c.1 == fv.z.scale && c.2 == fv.z.ox && c.3 == fv.z.oy
            });
        if !fresh {
            let w_px = (body.w * self.scale).ceil().clamp(1.0, 8192.0) as i32;
            let h_px = (body.h * self.scale).ceil().clamp(1.0, 8192.0) as i32;
            let s = (self.scale * fv.z.scale).clamp(0.02, 32.0);
            let Ok(r) = Renderer::new(&sh.dw, None, w_px, s) else {
                return;
            };
            let at = (fv.z.ox / fv.z.scale, fv.z.oy / fv.z.scale);
            let px = r.render_offscreen(h_px, self.pal.text, &mut |band| {
                draw_block(
                    band,
                    bl,
                    &|reg| viewer.hscroll(0, reg),
                    &self.pal,
                    at,
                    None,
                    None,
                    (&[], None),
                );
            });
            *fv.cache.borrow_mut() = Some((
                body,
                fv.z.scale,
                fv.z.ox,
                fv.z.oy,
                w_px as u32,
                h_px as u32,
                px,
            ));
        }
        if let Some(c) = fv.cache.borrow().as_ref() {
            b.image(body.x, body.y, c.4, c.5, &c.6);
        }
    }

    pub(super) fn fv_down(&mut self, sh: &Shared, x: f32, y: f32) {
        let (_, btns) = self.fv_toolbar();
        if let Some((_, i)) = btns.into_iter().find(|(r, _)| r.contains(x, y)) {
            return self.fv_button(sh, i);
        }
        let body = self.fv_body();
        let dbl = self.double_click(x, y);
        let csv_w = sh
            .dw
            .measure(tr(sh.lang, Msg::ExportCsv), 12.0, 400, 10_000.0)
            .0;
        let Some(fv) = &mut self.tz.fv else { return };
        if let Some(c) = fv.card {
            if Rect::new(c.right() - GRIP, c.bottom() - GRIP, GRIP, GRIP).contains(x, y) {
                fv.drag = Some(Drag::Grip(x, y));
            } else if Rect::new(c.x, c.y, c.w, TABLE_HEAD).contains(x, y) {
                if x >= c.right() - 16.0 - csv_w {
                    if let FvKind::Block { csv: Some(t), .. } = &fv.kind {
                        let t = t.clone();
                        self.actions.push(Action::Export(ExportKind::Csv(t)));
                    }
                } else {
                    fv.drag = Some(Drag::Move(x, y));
                }
            } else if c.contains(x, y) {
                fv.drag = Some(Drag::Pan(x, y));
            } else {
                self.fv_close();
            }
            return;
        }
        if dbl {
            fv.z.toggle();
            self.invalidate();
        } else if body.contains(x, y) {
            fv.drag = Some(Drag::Pan(x, y));
        }
    }

    pub(super) fn fv_move(&mut self, sh: &Shared, x: f32, y: f32) {
        let (_, btns) = self.fv_toolbar();
        let hover = btns
            .into_iter()
            .find(|(r, _)| r.contains(x, y))
            .map(|(_, i)| i);
        let at = self.tz_at(x, y);
        let Some(fv) = &mut self.tz.fv else { return };
        let mut dirty = fv.hover != hover;
        fv.hover = hover;
        if let Some(d) = fv.drag {
            dirty = true;
            match d {
                Drag::Pan(px, py) => {
                    fv.z.pan(x - px, y - py);
                    fv.drag = Some(Drag::Pan(x, y));
                }
                Drag::Move(px, py) => {
                    if let Some(c) = &mut fv.card {
                        c.x += x - px;
                        c.y += y - py;
                    }
                    fv.drag = Some(Drag::Move(x, y));
                }
                Drag::Grip(px, py) => {
                    if let Some(c) = &mut fv.card {
                        c.w = (c.w + x - px).max(200.0);
                        c.h = (c.h + y - py).max(TABLE_HEAD + 60.0);
                        fv.z.resize((c.w, c.h - TABLE_HEAD));
                    }
                    fv.drag = Some(Drag::Grip(x, y));
                }
            }
        }
        const TIPS: [Msg; 7] = [
            Msg::ViewZoomOut,
            Msg::ViewZoom,
            Msg::ViewZoomIn,
            Msg::ViewFit,
            Msg::ViewActual,
            Msg::ViewCopy,
            Msg::ViewClose,
        ];
        let tip = hover.and_then(|i| TIPS.get(i)).map(|m| Tip {
            text: tr(sh.lang, *m).to_string(),
            at,
        });
        self.tip_want(tip);
        if dirty {
            self.invalidate();
        }
    }

    /// Wheel: zoom at the cursor (a table card scrolls; Ctrl zooms it).
    pub(super) fn fv_wheel(&mut self, delta: i16, x: f32, y: f32, ctrl: bool) {
        let body = self.fv_body();
        let Some(fv) = &mut self.tz.fv else { return };
        let notches = f32::from(delta) / 120.0;
        if fv.card.is_some() && !ctrl {
            fv.z.pan(0.0, notches * 48.0);
        } else {
            fv.z.wheel(notches, x - body.x, y - body.y);
        }
        self.invalidate();
    }

    /// Keys: + − zoom, 0 fit, 1 actual size, Ctrl+C copy, Esc closes.
    pub(super) fn fv_key(&mut self, sh: &Shared, vk: u16, ctrl: bool) {
        let Some(fv) = &mut self.tz.fv else { return };
        match vk {
            0x1B => return self.fv_close(),
            0xBB | 0x6B => fv.z.step(1),
            0xBD | 0x6D => fv.z.step(-1),
            0x30 | 0x60 => fv.z.fit(),
            0x31 | 0x61 => fv.z.actual(),
            0x43 if ctrl => return self.fv_copy(sh),
            _ => return,
        }
        self.invalidate();
    }

    /// The content area changed size.
    pub(in crate::window) fn tz_resized(&mut self) {
        let (w, h) = (self.cw, self.ch);
        if let Some(fv) = &mut self.tz.fv
            && fv.card.is_none()
        {
            fv.z.resize((w, h.max(1.0)));
        }
    }

    /// Driver: opens the viewer of the first `what` (`image`, `diagram`, `table`).
    #[cfg(feature = "dev-tools")]
    pub(in crate::window) fn tz_drive_open(&mut self, sh: &Shared, what: &str) -> bool {
        let Some((bi, h)) = self.tz_find_hot(sh, what) else {
            return false;
        };
        let Some(k) = MediaKind::of(&h.kind) else {
            return false;
        };
        self.fv_open(sh, bi, &k);
        self.tz.fv.is_some()
    }

    /// Driver: content point of the first hot rectangle of kind `what` (`link`,
    /// `footref`, `image`, `diagram`, `table`), scrolled into view.
    #[cfg(feature = "dev-tools")]
    pub(in crate::window) fn tz_drive_hot(
        &mut self,
        sh: &Shared,
        what: &str,
    ) -> Option<(f32, f32)> {
        let (bi, h) = self.tz_find_hot(sh, what)?;
        let (cx, _) = self.col(sh);
        let View::Doc(d) = &self.view else {
            return None;
        };
        let top = (d.viewer.block_top(&d.content, bi) - d.viewer.scroll) as f32;
        let (x, y) = (
            h.rect.x + h.rect.w.min(8.0) / 2.0,
            h.rect.y + h.rect.h / 2.0,
        );
        Some((cx + x, top + y))
    }

    #[cfg(feature = "dev-tools")]
    fn tz_find_hot(
        &mut self,
        sh: &Shared,
        what: &str,
    ) -> Option<(usize, yamdv_layout::block::Hot)> {
        let n = match &self.view {
            View::Doc(d) => d.content.len(),
            _ => 0,
        };
        for bi in 0..n {
            let top = match &self.view {
                View::Doc(d) => d.viewer.block_top(&d.content, bi),
                _ => 0.0,
            };
            let s = self.scroll_pos();
            if top < s || top > s + self.view_h() - 40.0 {
                self.set_scroll((top - 40.0).max(0.0));
                self.prepare_now(sh);
            }
            let found = match &self.view {
                View::Doc(d) => d.viewer.layout_of(bi).and_then(|bl| {
                    bl.hot
                        .iter()
                        .find(|h| {
                            matches!(
                                (&h.kind, what),
                                (HotKind::Image { .. }, "image")
                                    | (HotKind::Diagram { .. }, "diagram")
                                    | (HotKind::Table { .. }, "table")
                                    | (HotKind::Link { .. }, "link")
                                    | (HotKind::FootRef { .. }, "footref")
                            )
                        })
                        .cloned()
                }),
                _ => None,
            };
            if let Some(h) = found {
                return Some((bi, h));
            }
        }
        None
    }

    /// Driver: wheel `notches` at content point (x, y); the content point under it before
    /// and after.
    #[cfg(feature = "dev-tools")]
    pub(in crate::window) fn tz_drive_wheel(
        &mut self,
        notches: f32,
        x: f32,
        y: f32,
    ) -> Option<((f32, f32), (f32, f32))> {
        let body = self.fv_body();
        let fv = self.tz.fv.as_mut()?;
        let (px, py) = (x - body.x, y - body.y);
        let before = fv.z.to_content(px, py);
        fv.z.wheel(notches, px, py);
        let after = fv.z.to_content(px, py);
        self.invalidate();
        Some((before, after))
    }

    /// Driver: a viewer key (`+`, `-`, `0`, `1`, `esc`, `copy`).
    #[cfg(feature = "dev-tools")]
    pub(in crate::window) fn tz_drive_key(&mut self, sh: &Shared, key: &str) -> bool {
        let (vk, ctrl) = match key {
            "+" => (0xBB, false),
            "-" => (0xBD, false),
            "0" => (0x30, false),
            "1" => (0x31, false),
            "esc" => (0x1B, false),
            "copy" => (0x43, true),
            _ => return false,
        };
        self.fv_key(sh, vk, ctrl);
        true
    }
}
