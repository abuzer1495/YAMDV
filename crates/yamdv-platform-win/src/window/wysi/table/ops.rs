//! Table structure commands (R-TBL-02): the tool strip of the active table, Table ▸ in the
//! context menu and the table shortcuts. Every command is one undo step.

use super::*;
use yamdv_editor::table::Align;

/// A table command (order = `app::TBL0` + index = tool strip order).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::window) enum TblOp {
    RowAbove,
    RowBelow,
    ColLeft,
    ColRight,
    DelRow,
    DelCol,
    AlignLeft,
    AlignCenter,
    AlignRight,
    AlignNone,
    SortAsc,
    SortDesc,
    RowUp,
    RowDown,
    ColMoveLeft,
    ColMoveRight,
    DelTable,
}

impl TblOp {
    pub(in crate::window) const ALL: [Self; 17] = [
        Self::RowAbove,
        Self::RowBelow,
        Self::ColLeft,
        Self::ColRight,
        Self::DelRow,
        Self::DelCol,
        Self::AlignLeft,
        Self::AlignCenter,
        Self::AlignRight,
        Self::AlignNone,
        Self::SortAsc,
        Self::SortDesc,
        Self::RowUp,
        Self::RowDown,
        Self::ColMoveLeft,
        Self::ColMoveRight,
        Self::DelTable,
    ];

    #[cfg(feature = "dev-tools")]
    /// Script / driver name.
    pub(in crate::window) fn name(self) -> &'static str {
        [
            "row-above",
            "row-below",
            "col-left",
            "col-right",
            "del-row",
            "del-col",
            "align-left",
            "align-center",
            "align-right",
            "align-none",
            "sort-asc",
            "sort-desc",
            "row-up",
            "row-down",
            "col-move-left",
            "col-move-right",
            "del-table",
        ][self as usize]
    }

    #[cfg(feature = "dev-tools")]
    pub(in crate::window) fn from_name(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|o| o.name() == s)
    }

    /// Menu / tooltip label.
    pub(in crate::window) fn msg(self) -> Msg {
        [
            Msg::MTblRowAbove,
            Msg::MTblRowBelow,
            Msg::MTblColLeft,
            Msg::MTblColRight,
            Msg::MTblDelRow,
            Msg::MTblDelCol,
            Msg::MTblAlignLeft,
            Msg::MTblAlignCenter,
            Msg::MTblAlignRight,
            Msg::MTblAlignNone,
            Msg::MTblSortAsc,
            Msg::MTblSortDesc,
            Msg::MTblRowUp,
            Msg::MTblRowDown,
            Msg::MTblColMoveLeft,
            Msg::MTblColMoveRight,
            Msg::MTblDelete,
        ][self as usize]
    }

    /// Segoe MDL2 glyph and the text fallback.
    fn glyph(self) -> (&'static str, &'static str) {
        match self {
            Self::RowAbove => ("\u{E70E}", "+↑"),
            Self::RowBelow => ("\u{E70D}", "+↓"),
            Self::ColLeft => ("\u{E76B}", "+←"),
            Self::ColRight => ("\u{E76C}", "+→"),
            Self::DelRow => ("\u{E738}", "−R"),
            Self::DelCol => ("\u{E738}", "−C"),
            Self::AlignLeft => ("\u{E8E4}", "⯇"),
            Self::AlignCenter => ("\u{E8E3}", "≡"),
            Self::AlignRight => ("\u{E8E2}", "⯈"),
            Self::AlignNone => ("\u{E8E4}", "∅"),
            Self::SortAsc => ("\u{E8CB}", "A↓"),
            Self::SortDesc => ("\u{E8CB}", "Z↓"),
            Self::RowUp => ("\u{E70E}", "⇡"),
            Self::RowDown => ("\u{E70D}", "⇣"),
            Self::ColMoveLeft => ("\u{E76B}", "⇠"),
            Self::ColMoveRight => ("\u{E76C}", "⇢"),
            Self::DelTable => ("\u{E74D}", "✕"),
        }
    }

    /// Separator before this tool in the strip.
    fn group_start(self) -> bool {
        matches!(
            self,
            Self::DelRow | Self::AlignLeft | Self::SortAsc | Self::RowUp | Self::DelTable
        )
    }
}

/// Tool button size.
const TOOL: f32 = 24.0;
const GAP: f32 = 7.0;

impl Win {
    /// Runs `op` on the caret cell of the active table; with no active table, first enters the
    /// table at the context-menu point.
    pub(in crate::window) fn table_op(&mut self, sh: &Shared, op: TblOp) {
        if self.table_cell().is_none() {
            let Some(pos) = self.wysi.tbl_at.take() else {
                return;
            };
            let Some(bi) = self.block_of(pos) else { return };
            self.edit_leave_block(sh);
            self.enter_block(sh, bi, pos);
        }
        let Some((t, (r, c))) = self.table_cell() else {
            return;
        };
        if op == TblOp::DelTable {
            let range = t.start..t.start + t.text.len();
            self.with_ed(sh, |ed| {
                ed.seal();
                ed.replace(range.clone(), "", range.start);
                ed.seal();
            });
            self.f2_reopen(sh);
            self.invalidate();
            return;
        }
        self.table_rewrite(sh, |tb| {
            let rows = tb.rows.len();
            let cols = tb.cols();
            Some(match op {
                TblOp::RowAbove => {
                    tb.insert_row(r.max(1));
                    (r.max(1), c)
                }
                TblOp::RowBelow => {
                    tb.insert_row(r + 1);
                    (r + 1, c)
                }
                TblOp::ColLeft => {
                    tb.insert_col(c);
                    (r, c)
                }
                TblOp::ColRight => {
                    tb.insert_col(c + 1);
                    (r, c + 1)
                }
                TblOp::DelRow => {
                    if !tb.delete_row(r) {
                        return None;
                    }
                    (r.min(rows - 2), c)
                }
                TblOp::DelCol => {
                    if !tb.delete_col(c) {
                        return None;
                    }
                    (r, c.min(cols - 2))
                }
                TblOp::AlignLeft | TblOp::AlignCenter | TblOp::AlignRight | TblOp::AlignNone => {
                    let a = match op {
                        TblOp::AlignLeft => Align::Left,
                        TblOp::AlignCenter => Align::Center,
                        TblOp::AlignRight => Align::Right,
                        _ => Align::None,
                    };
                    tb.set_align(c, a);
                    (r, c)
                }
                TblOp::SortAsc | TblOp::SortDesc => {
                    tb.sort(c, op == TblOp::SortDesc);
                    (r, c)
                }
                TblOp::RowUp => {
                    if r < 2 || !tb.move_row(r, r - 1) {
                        return None;
                    }
                    (r - 1, c)
                }
                TblOp::RowDown => {
                    if r == 0 || !tb.move_row(r, r + 1) {
                        return None;
                    }
                    (r + 1, c)
                }
                TblOp::ColMoveLeft => {
                    if c == 0 || !tb.move_col(c, c - 1) {
                        return None;
                    }
                    (r, c - 1)
                }
                TblOp::ColMoveRight => {
                    if !tb.move_col(c, c + 1) {
                        return None;
                    }
                    (r, c + 1)
                }
                TblOp::DelTable => return None,
            })
        });
    }

    /// Client rectangle of the active table (block hot `Table`).
    pub(in crate::window) fn active_table_rect(&self, sh: &Shared) -> Option<Rect> {
        self.active_table()?;
        let View::Doc(d) = &self.view else {
            return None;
        };
        let a = d.edit.as_deref()?.active?;
        let bl = d.viewer.layout_of(a.block)?;
        let h = bl
            .hot
            .iter()
            .find(|h| matches!(h.kind, yamdv_layout::block::HotKind::Table { .. }))?;
        let (cx, _) = self.col(sh);
        let top = (d.viewer.block_top(&d.content, a.block) - d.viewer.scroll) as f32;
        Some(Rect::new(cx + h.rect.x, top + h.rect.y, h.rect.w, h.rect.h))
    }

    /// Tool strip buttons above (or below, near the top) the active table.
    pub(in crate::window) fn table_tools(&self, sh: &Shared) -> Vec<(Rect, TblOp)> {
        let Some(r) = self.active_table_rect(sh) else {
            return Vec::new();
        };
        let y = if r.y - TOOL - 10.0 < 4.0 {
            r.bottom() + 6.0
        } else {
            r.y - TOOL - 8.0
        };
        let mut x = r.x.max(4.0) + 4.0;
        let mut out = Vec::new();
        for op in TblOp::ALL {
            if op.group_start() && !out.is_empty() {
                x += GAP;
            }
            out.push((Rect::new(x, y, TOOL, TOOL), op));
            x += TOOL;
        }
        out
    }

    /// The ⛶ and CSV buttons that end the strip (R-TBL-02; the hover badge is hidden meanwhile).
    fn table_media_tools(&self, sh: &Shared) -> Option<(Rect, Rect)> {
        let last = self.table_tools(sh).last()?.0;
        let x = last.right() + GAP;
        Some((
            Rect::new(x, last.y, TOOL + 4.0, TOOL),
            Rect::new(x + TOOL + 4.0, last.y, 36.0, TOOL),
        ))
    }

    /// Paints the tool strip.
    pub(in crate::window) fn table_tools_draw(&self, sh: &Shared, b: &Band<'_>) {
        let tools = self.table_tools(sh);
        let (Some(first), Some(last)) = (tools.first(), tools.last()) else {
            return;
        };
        let media = self.table_media_tools(sh);
        let end = media.map_or(last.0.right(), |(_, c)| c.right());
        let pill = Rect::new(first.0.x - 4.0, first.0.y, end - first.0.x + 8.0, TOOL);
        // Hairline ring so the strip stays visible on dark schemes; opaque body hides the text below.
        b.fill(
            Rect::new(pill.x - 1.0, pill.y - 1.0, pill.w + 2.0, pill.h + 2.0),
            0xffffff,
            0.22,
            7.0,
        );
        b.fill(pill, 0x202020, 1.0, 6.0);
        let (mx, my) = self.tz_mouse();
        if let Some((open, csv)) = media {
            b.fill(
                Rect::new(open.x - GAP / 2.0, open.y + 5.0, 1.0, open.h - 10.0),
                0xffffff,
                0.4,
                0.0,
            );
            for (r, label, size, weight) in [(open, "⛶", 14.0, 400), (csv, "CSV", 11.0, 600)] {
                if r.contains(mx, my) {
                    b.fill(r, 0xffffff, 0.18, 4.0);
                }
                let g = self.shape_ui(sh, label, size, weight, 0xffffff, None);
                b.text(
                    g.as_ref(),
                    (r.x + (r.w - g.width()) / 2.0).round(),
                    (r.y + (r.h - g.height()) / 2.0).round(),
                );
            }
        }
        for (i, (r, op)) in tools.iter().enumerate() {
            if op.group_start() && i > 0 {
                b.fill(
                    Rect::new(r.x - GAP / 2.0, r.y + 5.0, 1.0, r.h - 10.0),
                    0xffffff,
                    0.4,
                    0.0,
                );
            }
            if r.contains(mx, my) {
                b.fill(*r, 0xffffff, 0.18, 4.0);
            }
            let (icon, fb) = op.glyph();
            let size = if matches!(
                op,
                TblOp::SortDesc
                    | TblOp::RowUp
                    | TblOp::RowDown
                    | TblOp::ColMoveLeft
                    | TblOp::ColMoveRight
                    | TblOp::DelRow
                    | TblOp::DelCol
                    | TblOp::AlignNone
            ) {
                None
            } else {
                Some(12.0)
            };
            let g: Option<Box<dyn yamdv_layout::shaper::Shaped>> = match size {
                Some(s) => sh
                    .dw
                    .shape_icon(icon, s, 0xffffff)
                    .map(|g| Box::new(g) as Box<dyn yamdv_layout::shaper::Shaped>),
                None => None,
            };
            let g = g.unwrap_or_else(|| self.shape_ui(sh, fb, 11.0, 600, 0xffffff, None));
            b.text(
                g.as_ref(),
                (r.x + (r.w - g.width()) / 2.0).round(),
                (r.y + (r.h - g.height()) / 2.0).round(),
            );
        }
    }

    /// Mouse down on the tool strip: consumed when on it.
    pub(in crate::window) fn table_tools_down(&mut self, sh: &Shared, x: f32, y: f32) -> bool {
        let Some(op) = self
            .table_tools(sh)
            .into_iter()
            .find(|(r, _)| r.contains(x, y))
            .map(|(_, o)| o)
        else {
            return match self.table_media_tools(sh) {
                Some((open, csv)) if open.contains(x, y) || csv.contains(x, y) => {
                    self.active_table_media(sh, csv.contains(x, y));
                    true
                }
                _ => false,
            };
        };
        self.table_op(sh, op);
        true
    }
}
