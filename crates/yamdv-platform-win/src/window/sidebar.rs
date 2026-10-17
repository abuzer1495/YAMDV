//! Sidebar (B.5 minus FUT-02/03/33; prompt 005 §3.3): tabs Contents · History · Favorites,
//! TOC with title flattening, scroll spy, per-node collapse; grouped history; favorites;
//! edge chevron, `Ctrl+\`, drag-resize 180–480 px, left or right.

use super::*;
use yamdv_core::state::Group;

const TAB_H: f32 = 40.0;
const ROW_H: f32 = 28.0;
const CHEV: (f32, f32) = (14.0, 36.0);

/// Per-window sidebar state.
#[derive(Debug, Default)]
pub(super) struct Side {
    pub scroll: f32,
    pub hover: Option<(f32, f32)>,
    /// Width while dragging the inner edge.
    pub drag_w: Option<f32>,
    /// Collapsed TOC nodes (heading indexes) of the current document.
    pub collapsed: Vec<u32>,
}

/// What a sidebar row does.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum RowKind {
    Tab(u8),
    Toc {
        heading: usize,
        block: usize,
        depth: u8,
        kids: bool,
        open: bool,
        active: bool,
    },
    Group(String, Vec<String>),
    Hist {
        path: String,
        current: bool,
    },
    Fav(String),
    Hint(Msg),
}

/// A laid-out row (window DIPs, already scrolled).
#[derive(Debug, Clone)]
pub(super) struct Row {
    pub rect: Rect,
    pub kind: RowKind,
    pub text: String,
}

/// Minutes east of UTC (local time zone), via `FileTimeToLocalFileTime`.
fn tz_offset_min() -> i32 {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::Storage::FileSystem::FileTimeToLocalFileTime;
    let t = 133_000_000_000_000_000u64;
    let ft = FILETIME {
        dwLowDateTime: t as u32,
        dwHighDateTime: (t >> 32) as u32,
    };
    let mut lt = FILETIME::default();
    // SAFETY: valid in/out structures.
    if unsafe { FileTimeToLocalFileTime(&ft, &mut lt) }.is_err() {
        return 0;
    }
    let l = (u64::from(lt.dwHighDateTime) << 32) | u64::from(lt.dwLowDateTime);
    ((l as i64 - t as i64) / 600_000_000) as i32
}

fn file_name(p: &str) -> String {
    ypath::file_name(&ypath::display_form(p)).to_string()
}

impl Win {
    /// Sidebar width (DIPs) when visible.
    pub(super) fn side_w(&self, sh: &Shared) -> f32 {
        self.side
            .drag_w
            .unwrap_or(sh.side_w)
            .clamp(180.0, 480.0)
            .min((self.cw - 200.0).max(0.0))
    }

    /// Sidebar rectangle, `None` when hidden.
    pub(super) fn side_rect(&self, sh: &Shared) -> Option<Rect> {
        if !sh.side_visible {
            return None;
        }
        let w = self.side_w(sh);
        let x = if sh.side_right { self.cw - w } else { 0.0 };
        Some(Rect::new(x, 0.0, w, self.ch))
    }

    /// Document area `(x, width)` beside the sidebar.
    pub(super) fn doc_area(&self, sh: &Shared) -> (f32, f32) {
        match self.side_rect(sh) {
            Some(r) if sh.side_right => (0.0, r.x),
            Some(r) => (r.w, self.cw - r.w),
            None => (0.0, self.cw),
        }
    }

    /// Edge chevron (toggle) rectangle.
    pub(super) fn chevron(&self, sh: &Shared) -> Rect {
        let y = ((self.ch - CHEV.1) / 2.0).round();
        let x = match (self.side_rect(sh), sh.side_right) {
            (Some(r), false) => r.x + r.w,
            (Some(r), true) => r.x - CHEV.0,
            (None, false) => 0.0,
            (None, true) => self.cw - CHEV.0,
        };
        Rect::new(x, y, CHEV.0, CHEV.1)
    }

    fn tab(&self) -> u8 {
        app::with(|a| a.state.sidebar_tab).unwrap_or(0)
    }

    /// Lays out the rows of the current tab.
    pub(super) fn side_rows(&self, sh: &Shared) -> Vec<Row> {
        let Some(sr) = self.side_rect(sh) else {
            return Vec::new();
        };
        let mut rows = Vec::new();
        let tw = sr.w / 3.0;
        for (i, m) in [Msg::TabContents, Msg::TabHistory, Msg::TabFavorites]
            .into_iter()
            .enumerate()
        {
            rows.push(Row {
                rect: Rect::new(sr.x + tw * i as f32, 0.0, tw, TAB_H),
                kind: RowKind::Tab(i as u8),
                text: tr(sh.lang, m).to_string(),
            });
        }
        let mut y = TAB_H + 8.0 - self.side.scroll;
        let mut push = |rows: &mut Vec<Row>, kind: RowKind, text: String, indent: f32| {
            rows.push(Row {
                rect: Rect::new(sr.x + 8.0 + indent, y, sr.w - 16.0 - indent, ROW_H),
                kind,
                text,
            });
            y += ROW_H;
        };
        match self.tab() {
            0 => {
                let View::Doc(d) = &self.view else {
                    push(&mut rows, RowKind::Hint(Msg::TocEmpty), String::new(), 0.0);
                    return rows;
                };
                let Content::Markdown(doc) = &d.content else {
                    push(&mut rows, RowKind::Hint(Msg::TocEmpty), String::new(), 0.0);
                    return rows;
                };
                let hs = &doc.index.headings;
                let max_depth = sh.toc_depth;
                // Title flattening (R-NAV-05): a single leading H1 is dropped from the tree.
                let h1s = hs.iter().filter(|h| h.level == 1).count();
                let flatten = h1s == 1 && hs.first().is_some_and(|h| h.level == 1);
                let reading = d.viewer.scroll + 80.0;
                let active = hs
                    .iter()
                    .rposition(|h| d.viewer.block_top(&d.content, h.block as usize) <= reading);
                let mut hide_below: Option<u8> = None;
                let mut any = false;
                for (i, h) in hs.iter().enumerate() {
                    if flatten && i == 0 {
                        continue;
                    }
                    let lvl = if flatten {
                        h.level.saturating_sub(1).max(1)
                    } else {
                        h.level
                    };
                    if lvl > max_depth {
                        continue;
                    }
                    if let Some(c) = hide_below {
                        if lvl > c {
                            continue;
                        }
                        hide_below = None;
                    }
                    let kids = hs.get(i + 1).is_some_and(|n| n.level > h.level);
                    let open = !self.side.collapsed.contains(&(i as u32));
                    if kids && !open {
                        hide_below = Some(lvl);
                    }
                    any = true;
                    push(
                        &mut rows,
                        RowKind::Toc {
                            heading: i,
                            block: h.block as usize,
                            depth: lvl,
                            kids,
                            open,
                            active: active == Some(i),
                        },
                        h.text.clone(),
                        f32::from(lvl - 1) * 12.0,
                    );
                }
                if !any {
                    push(&mut rows, RowKind::Hint(Msg::TocEmpty), String::new(), 0.0);
                }
            }
            1 => {
                let Some((hist, start)) = app::with(|a| (a.state.history.clone(), a.session_start))
                else {
                    return rows;
                };
                let cur = match &self.view {
                    View::Doc(d) => Some(d.path.clone()),
                    _ => None,
                };
                for (g, idx) in
                    yamdv_core::state::group_history(&hist, start, app::now_ms(), tz_offset_min())
                {
                    let label = match g {
                        Group::Session => tr(sh.lang, Msg::GroupSession).to_string(),
                        Group::Today => tr(sh.lang, Msg::GroupToday).to_string(),
                        Group::Yesterday => tr(sh.lang, Msg::GroupYesterday).to_string(),
                        Group::Date(y, m, d) if sh.lang == Lang::Tr => format!("{d:02}.{m:02}.{y}"),
                        Group::Date(y, m, d) => format!("{y}-{m:02}-{d:02}"),
                    };
                    let paths: Vec<String> = idx
                        .iter()
                        .filter_map(|i| hist.get(*i).map(|h| h.1.clone()))
                        .collect();
                    push(
                        &mut rows,
                        RowKind::Group(label.clone(), paths.clone()),
                        label,
                        0.0,
                    );
                    for p in paths {
                        let current = cur.as_deref().is_some_and(|c| c.eq_ignore_ascii_case(&p));
                        let name = file_name(&p);
                        push(&mut rows, RowKind::Hist { path: p, current }, name, 8.0);
                    }
                }
            }
            _ => {
                let favs = app::with(|a| a.state.favorites.clone()).unwrap_or_default();
                if favs.is_empty() {
                    push(&mut rows, RowKind::Hint(Msg::FavEmpty), String::new(), 0.0);
                }
                for p in favs {
                    let name = file_name(&p);
                    push(&mut rows, RowKind::Fav(p), name, 0.0);
                }
            }
        }
        rows
    }

    /// Draws the sidebar and the edge chevron.
    pub(super) fn draw_side(&self, sh: &Shared, b: &Band<'_>) {
        let pal = &self.pal;
        let ch = self.chevron(sh);
        if let Some(sr) = self.side_rect(sh) {
            b.fill(sr, pal.code_bg, 1.0, 0.0);
            let edge = if sh.side_right {
                sr.x
            } else {
                sr.x + sr.w - 1.0
            };
            b.fill(Rect::new(edge, 0.0, 1.0, sr.h), pal.border_subtle, 1.0, 0.0);
            let tab = self.tab();
            let hover = self.side.hover;
            for r in self.side_rows(sh) {
                let hov = hover.is_some_and(|(x, y)| r.rect.contains(x, y));
                if let RowKind::Tab(_) = r.kind {
                } else if r.rect.y < TAB_H || r.rect.y > self.ch {
                    continue;
                }
                b.push_clip(r.rect);
                let ty = |t: &dyn yamdv_layout::shaper::Shaped| {
                    (r.rect.y + (r.rect.h - t.height()) / 2.0).round()
                };
                match &r.kind {
                    RowKind::Tab(i) => {
                        let on = *i == tab;
                        let t = self.shape_ui(
                            sh,
                            &r.text,
                            13.0,
                            if on { 600 } else { 400 },
                            if on { pal.text } else { pal.text_muted },
                            None,
                        );
                        b.text(
                            t.as_ref(),
                            (r.rect.x + (r.rect.w - t.width()) / 2.0).round(),
                            ty(t.as_ref()),
                        );
                        if on {
                            b.fill(
                                Rect::new(
                                    r.rect.x + 12.0,
                                    r.rect.y + r.rect.h - 3.0,
                                    r.rect.w - 24.0,
                                    2.0,
                                ),
                                pal.accent,
                                1.0,
                                1.0,
                            );
                        }
                    }
                    RowKind::Toc {
                        kids, open, active, ..
                    } => {
                        if *active {
                            b.fill(r.rect, pal.accent, 0.14, 4.0);
                        } else if hov {
                            b.fill(r.rect, pal.text, 0.06, 4.0);
                        }
                        let color = if *active { pal.accent } else { pal.text };
                        if *kids {
                            let a = self.shape_ui(
                                sh,
                                if *open { "\u{25BE}" } else { "\u{25B8}" },
                                11.0,
                                400,
                                pal.text_muted,
                                None,
                            );
                            b.text(a.as_ref(), r.rect.x + 2.0, ty(a.as_ref()));
                        }
                        let t = self.shape_ui(sh, &r.text, sh.toc_font, 400, color, None);
                        b.text(t.as_ref(), r.rect.x + 16.0, ty(t.as_ref()));
                    }
                    RowKind::Group(label, _) => {
                        let t = self.shape_ui(sh, label, 11.0, 600, pal.text_muted, None);
                        b.text(t.as_ref(), r.rect.x + 4.0, ty(t.as_ref()));
                        if hov {
                            let c = self.shape_ui(
                                sh,
                                tr(sh.lang, Msg::Clear),
                                11.0,
                                400,
                                pal.link,
                                None,
                            );
                            b.text(
                                c.as_ref(),
                                r.rect.x + r.rect.w - c.width() - 6.0,
                                ty(c.as_ref()),
                            );
                        }
                    }
                    RowKind::Hist { path, current } => {
                        if hov {
                            b.fill(r.rect, pal.text, 0.06, 4.0);
                        }
                        let missing = !std::path::Path::new(&ypath::long_form(path)).exists();
                        let color = if missing { pal.text_muted } else { pal.text };
                        let t = self.shape_ui(
                            sh,
                            &r.text,
                            13.0,
                            if *current { 600 } else { 400 },
                            color,
                            None,
                        );
                        b.text(t.as_ref(), r.rect.x + 6.0, ty(t.as_ref()));
                        if *current {
                            let c = self.shape_ui(
                                sh,
                                tr(sh.lang, Msg::CurrentTag),
                                11.0,
                                400,
                                pal.accent,
                                None,
                            );
                            b.text(
                                c.as_ref(),
                                r.rect.x + r.rect.w - c.width() - 28.0,
                                ty(c.as_ref()),
                            );
                        }
                        if hov {
                            let x = self.shape_ui(sh, "\u{00D7}", 14.0, 400, pal.text_muted, None);
                            b.text(x.as_ref(), r.rect.x + r.rect.w - 18.0, ty(x.as_ref()));
                        }
                    }
                    RowKind::Fav(_) => {
                        if hov {
                            b.fill(r.rect, pal.text, 0.06, 4.0);
                        }
                        let t = self.shape_ui(
                            sh,
                            &format!("\u{2605} {}", r.text),
                            13.0,
                            400,
                            pal.text,
                            None,
                        );
                        b.text(t.as_ref(), r.rect.x + 6.0, ty(t.as_ref()));
                        if hov {
                            let x = self.shape_ui(sh, "\u{00D7}", 14.0, 400, pal.text_muted, None);
                            b.text(x.as_ref(), r.rect.x + r.rect.w - 18.0, ty(x.as_ref()));
                        }
                    }
                    RowKind::Hint(m) => {
                        let t = self.shape_ui(sh, tr(sh.lang, *m), 12.0, 400, pal.text_muted, None);
                        b.text(t.as_ref(), r.rect.x + 6.0, ty(t.as_ref()));
                    }
                }
                b.pop_clip();
            }
        }
        let hov = self.side.hover.is_some_and(|(x, y)| ch.contains(x, y));
        b.fill(ch, pal.code_bg, 1.0, 4.0);
        b.stroke(ch, pal.border_subtle, 4.0, 1.0);
        let open = sh.side_visible;
        let glyph = match (open, sh.side_right) {
            (true, false) | (false, true) => "\u{2039}",
            _ => "\u{203A}",
        };
        let t = self.shape_ui(
            sh,
            glyph,
            14.0,
            600,
            if hov { pal.accent } else { pal.text_muted },
            None,
        );
        b.text(
            t.as_ref(),
            (ch.x + (ch.w - t.width()) / 2.0).round(),
            (ch.y + (ch.h - t.height()) / 2.0).round(),
        );
    }

    /// `true` when `(x, y)` is on the sidebar, its resize edge or the chevron.
    pub(super) fn over_side(&self, sh: &Shared, x: f32, y: f32) -> bool {
        self.chevron(sh).contains(x, y) || self.side_rect(sh).is_some_and(|r| r.contains(x, y))
    }

    fn on_edge(&self, sh: &Shared, x: f32) -> bool {
        self.side_rect(sh).is_some_and(|r| {
            let e = if sh.side_right { r.x } else { r.x + r.w };
            (x - e).abs() <= 4.0
        })
    }

    /// Mouse down on the sidebar; returns actions (commands run outside the borrow).
    pub(super) fn side_down(&mut self, sh: &Shared, x: f32, y: f32) -> Option<Action> {
        if self.chevron(sh).contains(x, y) {
            return Some(Action::Command(app::OUTLINE));
        }
        if self.on_edge(sh, x) {
            self.side.drag_w = Some(self.side_w(sh));
            // SAFETY: valid hwnd.
            unsafe { SetCapture(self.hwnd) };
            return None;
        }
        let rows = self.side_rows(sh);
        let r = rows.into_iter().find(|r| r.rect.contains(x, y))?;
        let near_right = x > r.rect.x + r.rect.w - 24.0;
        match r.kind {
            RowKind::Tab(i) => {
                app::with(|a| a.state.sidebar_tab = i);
                app::save_state();
                self.side.scroll = 0.0;
            }
            RowKind::Toc {
                heading,
                block,
                kids,
                open,
                ..
            } => {
                if kids && x < r.rect.x + 16.0 {
                    let h = heading as u32;
                    if open {
                        self.side.collapsed.push(h);
                    } else {
                        self.side.collapsed.retain(|c| *c != h);
                    }
                    if let View::Doc(d) = &self.view {
                        let (p, c) = (d.path.clone(), self.side.collapsed.clone());
                        app::with(|a| a.state.set_toc_collapsed(&p, c));
                        app::save_state();
                    }
                } else {
                    self.scroll_to_block(sh, block);
                }
            }
            RowKind::Group(_, paths) => {
                if near_right || x > r.rect.x + r.rect.w - 60.0 {
                    app::with(|a| {
                        for p in &paths {
                            a.state.remove_history(p);
                        }
                    });
                    app::save_state();
                }
            }
            RowKind::Hist { path, .. } => {
                if near_right {
                    app::with(|a| a.state.remove_history(&path));
                    app::save_state();
                } else {
                    self.open_from_side(sh, &path);
                }
            }
            RowKind::Fav(path) => {
                if near_right {
                    app::with(|a| {
                        a.state.toggle_favorite(&path);
                    });
                    app::save_state();
                } else {
                    self.open_from_side(sh, &path);
                }
            }
            RowKind::Hint(_) => {}
        }
        self.invalidate();
        None
    }

    fn open_from_side(&mut self, sh: &Shared, path: &str) {
        if std::path::Path::new(&ypath::long_form(path)).is_file() {
            self.open(sh, path, None, false);
        } else {
            self.toast_with(Msg::ToastMissing, &file_name(path));
        }
    }

    /// Mouse move: hover and edge drag. Returns `true` when handled.
    pub(super) fn side_move(&mut self, sh: &Shared, x: f32, y: f32) -> bool {
        if self.side.drag_w.is_some() {
            let w = if sh.side_right { self.cw - x } else { x };
            self.side.drag_w = Some(w.clamp(180.0, 480.0));
            update_size(self, sh);
            return true;
        }
        let over = self.over_side(sh, x, y);
        let had = self.side.hover.is_some();
        self.side.hover = over.then_some((x, y));
        if over || had {
            self.invalidate();
        }
        over
    }

    /// Mouse up after an edge drag: persists the width (R-NAV-04).
    pub(super) fn side_up(&mut self) -> Option<Action> {
        let w = self.side.drag_w.take()?;
        // SAFETY: no preconditions.
        let _ = unsafe { ReleaseCapture() };
        Some(Action::SetSetting(
            "sidebar.width",
            format!("{}", w.round() as i32),
        ))
    }

    /// Wheel over the sidebar list.
    pub(super) fn side_wheel(&mut self, delta: i16) {
        self.side.scroll = (self.side.scroll - f32::from(delta) / 120.0 * 3.0 * ROW_H).max(0.0);
        self.invalidate();
    }

    /// Smooth scroll so block `b` is 16 px below the top.
    pub(super) fn scroll_to_block(&mut self, sh: &Shared, b: usize) {
        let vh = self.view_h();
        let Some(d) = self.doc() else { return };
        let old = d.viewer.scroll;
        let top = d.viewer.block_top(&d.content, b);
        d.viewer.scroll_to(top, vh);
        self.prepare_now(sh);
        let Some(d) = self.doc() else { return };
        let target = (d.viewer.block_top(&d.content, b) - 16.0).max(0.0);
        d.viewer.scroll_to(old, vh);
        self.scroll_by(sh, target - old, true);
    }
}
