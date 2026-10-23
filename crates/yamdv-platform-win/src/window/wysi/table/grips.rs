//! Drag grips (R-TBL-03): a column grip on the header and a row grip at the left edge of the
//! active table; dragging shows a 2 px accent drop line and moves the row / column as one undo
//! step.

use super::*;

/// A grip drag in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::window) struct GripDrag {
    /// Column (true) or row grip.
    pub col: bool,
    pub from: usize,
    pub to: usize,
}

/// Grip long / short side.
const LONG: f32 = 20.0;
const SHORT: f32 = 12.0;

impl Win {
    /// Cell rectangles of the active table (client DIPs): (row, col, rect).
    pub(in crate::window) fn table_cells(&self, sh: &Shared) -> Vec<(usize, usize, Rect)> {
        let View::Doc(d) = &self.view else {
            return Vec::new();
        };
        let Some(a) = d.edit.as_deref().and_then(|e| e.active) else {
            return Vec::new();
        };
        if d.viewer
            .source_block
            .as_ref()
            .and_then(|s| s.table.as_ref())
            .is_none()
        {
            return Vec::new();
        }
        let Some(bl) = d.viewer.layout_of(a.block) else {
            return Vec::new();
        };
        let (cx, _) = self.col(sh);
        let top = (d.viewer.block_top(&d.content, a.block) - d.viewer.scroll) as f32;
        bl.hot
            .iter()
            .filter_map(|h| match h.kind {
                yamdv_layout::block::HotKind::Cell { row, col } => Some((
                    row as usize,
                    col as usize,
                    Rect::new(cx + h.rect.x, top + h.rect.y, h.rect.w, h.rect.h),
                )),
                _ => None,
            })
            .collect()
    }

    /// Every grip: (column grip?, index, rect).
    fn grips(&self, sh: &Shared) -> Vec<(bool, usize, Rect)> {
        let cells = self.table_cells(sh);
        let mut out = Vec::new();
        for &(r, c, rect) in &cells {
            if r == 0 {
                out.push((
                    true,
                    c,
                    Rect::new(
                        rect.x + (rect.w - LONG) / 2.0,
                        rect.y - SHORT / 2.0,
                        LONG,
                        SHORT,
                    ),
                ));
            }
            if c == 0 && r > 0 {
                out.push((
                    false,
                    r,
                    Rect::new(
                        rect.x - SHORT - 2.0,
                        rect.y + (rect.h - LONG) / 2.0,
                        SHORT,
                        LONG,
                    ),
                ));
            }
        }
        out
    }

    /// The grip under (x, y).
    pub(in crate::window) fn grip_at(&self, sh: &Shared, x: f32, y: f32) -> Option<(bool, usize)> {
        self.grips(sh)
            .into_iter()
            .find(|(_, _, r)| Rect::new(r.x - 2.0, r.y - 2.0, r.w + 4.0, r.h + 4.0).contains(x, y))
            .map(|(c, i, _)| (c, i))
    }

    /// Grips shown for the mouse at (x, y): the hovered column's (pointer over the header) and
    /// the hovered row's.
    fn grips_shown(&self, sh: &Shared, x: f32, y: f32) -> Vec<(bool, usize, Rect)> {
        let cells = self.table_cells(sh);
        let near = |r: &Rect| {
            Rect::new(
                r.x - SHORT - 4.0,
                r.y - SHORT,
                r.w + SHORT + 4.0,
                r.h + SHORT,
            )
            .contains(x, y)
        };
        let hover_col = cells
            .iter()
            .find(|(r, _, rc)| *r == 0 && near(rc))
            .map(|(_, c, _)| *c);
        let hover_row = cells
            .iter()
            .find(|(r, _, rc)| {
                *r > 0 && y >= rc.y && y < rc.bottom() && x >= rc.x - SHORT - 4.0 && x < rc.right()
            })
            .map(|(r, _, _)| *r);
        let drag = self.wysi.grip;
        self.grips(sh)
            .into_iter()
            .filter(|(col, i, _)| match drag {
                Some(g) => g.col == *col && g.from == *i,
                None if *col => hover_col == Some(*i),
                None => hover_row == Some(*i),
            })
            .collect()
    }

    /// Mouse down on a grip: starts the drag.
    pub(in crate::window) fn grip_down(&mut self, sh: &Shared, x: f32, y: f32) -> bool {
        let Some((col, i)) = self.grip_at(sh, x, y) else {
            return false;
        };
        self.wysi.grip = Some(GripDrag {
            col,
            from: i,
            to: i,
        });
        // SAFETY: valid hwnd.
        unsafe { SetCapture(self.hwnd) };
        self.invalidate();
        true
    }

    /// Mouse move: drag target / grip hover. True while dragging.
    pub(in crate::window) fn grip_move(&mut self, sh: &Shared, x: f32, y: f32) -> bool {
        let Some(mut g) = self.wysi.grip else {
            let shown: Vec<_> = self
                .grips_shown(sh, x, y)
                .into_iter()
                .map(|(c, i, _)| (c, i))
                .collect();
            if shown != self.wysi.grip_hover {
                self.wysi.grip_hover = shown;
                self.invalidate();
            }
            return false;
        };
        let cells = self.table_cells(sh);
        let to = if g.col {
            cells
                .iter()
                .filter(|(r, _, _)| *r == 0)
                .min_by(|a, b| {
                    let da = (a.2.x + a.2.w / 2.0 - x).abs();
                    let db = (b.2.x + b.2.w / 2.0 - x).abs();
                    da.total_cmp(&db)
                })
                .map(|(_, c, _)| *c)
        } else {
            cells
                .iter()
                .filter(|(r, c, _)| *r > 0 && *c == 0)
                .min_by(|a, b| {
                    let da = (a.2.y + a.2.h / 2.0 - y).abs();
                    let db = (b.2.y + b.2.h / 2.0 - y).abs();
                    da.total_cmp(&db)
                })
                .map(|(r, _, _)| *r)
        };
        if let Some(to) = to
            && to != g.to
        {
            g.to = to;
            self.wysi.grip = Some(g);
            self.invalidate();
        }
        true
    }

    /// Mouse up: drops the row / column (one undo step). True when a drag ended.
    pub(in crate::window) fn grip_up(&mut self, sh: &Shared) -> bool {
        let Some(g) = self.wysi.grip.take() else {
            return false;
        };
        // SAFETY: no preconditions.
        let _ = unsafe { ReleaseCapture() };
        if g.to != g.from {
            let rc = self.table_cell().map_or((0, 0), |(_, rc)| rc);
            self.table_rewrite(sh, |tb| {
                if g.col {
                    tb.move_col(g.from, g.to).then_some((rc.0, g.to))
                } else {
                    tb.move_row(g.from, g.to).then_some((g.to, rc.1))
                }
            });
        }
        self.invalidate();
        true
    }

    /// Paints the shown grips and the drop indicator.
    pub(in crate::window) fn grips_draw(&self, sh: &Shared, b: &Band<'_>) {
        let (mx, my) = self.tz_mouse();
        let pal = &self.pal;
        for (col, _, r) in self.grips_shown(sh, mx, my) {
            b.fill(r, pal.bg, 1.0, 3.0);
            b.stroke(r, pal.text_muted, 3.0, 1.0);
            // ⋮⋮ as two columns of three dots (rotated for the column grip).
            for i in 0..2 {
                for j in 0..3 {
                    let (dx, dy) = if col {
                        (
                            r.x + r.w / 2.0 - 4.0 + 4.0 * j as f32,
                            r.y + r.h / 2.0 - 2.0 + 3.0 * i as f32,
                        )
                    } else {
                        (
                            r.x + r.w / 2.0 - 2.0 + 3.0 * i as f32,
                            r.y + r.h / 2.0 - 4.0 + 4.0 * j as f32,
                        )
                    };
                    b.fill(
                        Rect::new(dx - 0.75, dy - 0.75, 1.5, 1.5),
                        pal.text_muted,
                        1.0,
                        0.75,
                    );
                }
            }
        }
        let Some(g) = self.wysi.grip else { return };
        if g.to == g.from {
            return;
        }
        let cells = self.table_cells(sh);
        let all = cells.iter().fold(None::<Rect>, |acc, (_, _, r)| {
            Some(acc.map_or(*r, |a| {
                let x0 = a.x.min(r.x);
                let y0 = a.y.min(r.y);
                Rect::new(
                    x0,
                    y0,
                    a.right().max(r.right()) - x0,
                    a.bottom().max(r.bottom()) - y0,
                )
            }))
        });
        let Some(all) = all else { return };
        let line = if g.col {
            cells
                .iter()
                .find(|(r, c, _)| *r == 0 && *c == g.to)
                .map(|(_, _, rc)| {
                    let x = if g.to > g.from { rc.right() } else { rc.x };
                    Rect::new(x - 1.0, all.y, 2.0, all.h)
                })
        } else {
            cells
                .iter()
                .find(|(r, c, _)| *r == g.to && *c == 0)
                .map(|(_, _, rc)| {
                    let y = if g.to > g.from { rc.bottom() } else { rc.y };
                    Rect::new(all.x, y - 1.0, all.w, 2.0)
                })
        };
        if let Some(l) = line {
            b.fill(l, pal.accent, 1.0, 0.0);
        }
    }

    /// Driver: drags grip (`col` / `row`, index) onto `to` through the mouse handlers.
    #[cfg(feature = "dev-tools")]
    pub(in crate::window) fn grip_drive(
        &mut self,
        sh: &Shared,
        col: bool,
        from: usize,
        to: usize,
    ) -> bool {
        let Some(r) = self
            .grips(sh)
            .into_iter()
            .find(|(c, i, _)| *c == col && *i == from)
            .map(|g| g.2)
        else {
            return false;
        };
        if !self.grip_down(sh, r.x + r.w / 2.0, r.y + r.h / 2.0) {
            return false;
        }
        let Some(t) = self
            .table_cells(sh)
            .into_iter()
            .find(|(rr, cc, _)| {
                if col {
                    *rr == 0 && *cc == to
                } else {
                    *rr == to && *cc == 0
                }
            })
            .map(|x| x.2)
        else {
            return false;
        };
        self.grip_move(sh, t.x + t.w / 2.0, t.y + t.h / 2.0);
        self.grip_up(sh)
    }
}
