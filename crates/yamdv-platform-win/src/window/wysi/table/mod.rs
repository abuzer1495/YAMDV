//! F2 table editing (prompt 016 §3 Part 2, R-TBL-01…08): the active table stays a rendered
//! grid whose cells use the inline reveal; cell navigation keys, `\|` escaping, structural
//! commands (hover pill tools, Table ▸, shortcuts), drag grips, the insert picker and
//! TSV / CSV paste.

mod grips;
mod ops;
mod paste;
mod pick;

use super::*;
use crate::window::edit::Mode;
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_BACK, VK_DELETE};
use yamdv_core::doc::ReadOpts;
use yamdv_core::doc::reveal as rv;
use yamdv_core::doc::text::{ParaText, Seg};
use yamdv_editor::table::{self, Grid};
use yamdv_layout::table_edit::TableReveal;

pub(in crate::window) use grips::GripDrag;
pub(in crate::window) use ops::TblOp;
pub(in crate::window) use paste::TsvOffer;
pub(in crate::window) use pick::{Picker, pick_label};

/// The F2 active block when it is a table.
pub(in crate::window) struct ActiveTable {
    /// Buffer offset of the block.
    pub start: usize,
    /// Block text.
    pub text: String,
    /// Cell layout.
    pub grid: Grid,
}

/// Cells that start like a block construct keep their source verbatim (no inline reveal).
fn verbatim_cell(t: &str) -> bool {
    let s = t.trim_start();
    s.starts_with(['#', '>', '-', '+', '*', '='])
        || s.split_once(['.', ')'])
            .is_some_and(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// One cell's reveal paragraph with block-relative source offsets.
fn cell_para(
    text: &str,
    content: &std::ops::Range<usize>,
    opts: ReadOpts,
    sel: &std::ops::Range<u32>,
) -> rv::RevealPara {
    let cell = text.get(content.clone()).unwrap_or("");
    let off = u32::try_from(content.start).unwrap_or(0);
    let an = (!cell.is_empty() && !verbatim_cell(cell))
        .then(|| rv::analyze(cell, opts))
        .flatten()
        .filter(|a| a.heading == 0 && !a.quote && a.lines.len() <= 1);
    let Some(an) = an else {
        let len16 = u32::try_from(cell.encode_utf16().count()).unwrap_or(0);
        let end = off + u32::try_from(cell.len()).unwrap_or(0);
        return rv::RevealPara {
            text: ParaText {
                text: cell.to_string(),
                segs: vec![Seg {
                    u16_start: 0,
                    u16_len: len16,
                    byte_start: 0,
                    src: off..end,
                    verbatim: true,
                    style: 0,
                    link: yamdv_core::doc::detail::NO_LINK,
                }],
            },
            marks: Vec::new(),
            shown: Vec::new(),
        };
    };
    let end = off + u32::try_from(cell.len()).unwrap_or(0);
    let inside = |p: u32| p >= off && p <= end;
    let set = if inside(sel.start) || inside(sel.end) {
        rv::reveal_set(
            &an,
            sel.start.saturating_sub(off)..sel.end.saturating_sub(off).min(end - off),
        )
    } else {
        vec![false; an.pieces.len()]
    };
    let mut p = rv::build(&an, cell, &set);
    for s in &mut p.text.segs {
        s.src = s.src.start + off..s.src.end + off;
    }
    p
}

/// The cell grid of an active table for the selection `sel` (block-relative).
pub(super) fn table_reveal(
    g: &Grid,
    text: &str,
    opts: ReadOpts,
    sel: std::ops::Range<u32>,
) -> TableReveal {
    let n = g.cols();
    let cells = g
        .rows
        .iter()
        .enumerate()
        .map(|(r, row)| {
            (0..n)
                .map(|c| {
                    let content = row.get(c).map(|s| s.content.clone()).unwrap_or_else(|| {
                        let e = g.row_line(r).map_or(0, |l| l.end);
                        e..e
                    });
                    cell_para(text, &content, opts, &sel)
                })
                .collect()
        })
        .collect();
    TableReveal {
        cells,
        aligns: g.aligns.iter().map(|a| a.code()).collect(),
    }
}

/// `editing.auto_format_tables` (default on).
pub(super) fn pretty() -> bool {
    app::get("editing.auto_format_tables") != "false"
}

impl Win {
    /// The F2 active block as a table.
    pub(in crate::window) fn active_table(&self) -> Option<ActiveTable> {
        if self.edit_mode() != Some(Mode::Live) {
            return None;
        }
        let r = self.active_range()?;
        let text = self.edit()?.ed.slice(r.clone());
        let grid = table::grid(&text)?;
        Some(ActiveTable {
            start: r.start,
            text,
            grid,
        })
    }

    /// The active table and the caret's cell.
    pub(in crate::window) fn table_cell(&self) -> Option<(ActiveTable, (usize, usize))> {
        let t = self.active_table()?;
        let caret = self.edit()?.ed.caret();
        let rc = t.grid.cell_at(caret.checked_sub(t.start)?)?;
        Some((t, rc))
    }

    /// Caret to the end of cell `(r, c)` of the active table.
    pub(in crate::window) fn table_goto(&mut self, sh: &Shared, r: usize, c: usize) -> bool {
        let Some(t) = self.active_table() else {
            return false;
        };
        let Some(cr) = t.grid.content(r, c) else {
            return false;
        };
        if let Some(e) = self.edit_mut() {
            e.ed.move_to(t.start + cr.end, false);
        }
        self.after_edit(sh, false);
        true
    }

    /// Driver probe: "row,col" of the caret cell (empty outside a table).
    pub(in crate::window) fn table_probe(&self) -> String {
        self.table_cell()
            .map_or(String::new(), |(_, (r, c))| format!("{r},{c}"))
    }

    /// Typed text in a table cell: pipes are escaped (R-TBL-04).
    pub(in crate::window) fn table_type_fix(&self, s: &str) -> Option<String> {
        (s.contains('|') && self.table_cell().is_some()).then(|| table::escape_cell(s))
    }

    /// Table context keys (SHORTCUTS "Table context"); `Some` when consumed.
    pub(in crate::window) fn table_key(&mut self, sh: &Shared, c: Chord) -> Option<Option<Action>> {
        if let Some(r) = self.pick_key(sh, c) {
            return Some(r);
        }
        let (t, (r, col)) = self.table_cell()?;
        let rows = t.grid.rows.len();
        let cols = t.grid.cols();
        let caret = self.edit()?.ed.caret();
        let cr = t.grid.content(r, col)?;
        let (cs, ce) = (t.start + cr.start, t.start + cr.end);
        let vk = c.vk;
        if c.ctrl && c.alt && !c.shift {
            let op = match vk {
                v if v == VK_UP.0 => TblOp::RowUp,
                v if v == VK_DOWN.0 => TblOp::RowDown,
                v if v == VK_LEFT.0 => TblOp::ColMoveLeft,
                v if v == VK_RIGHT.0 => TblOp::ColMoveRight,
                _ => return None,
            };
            self.table_op(sh, op);
            return Some(None);
        }
        if c.ctrl && c.shift && !c.alt && vk == VK_BACK.0 {
            self.table_op(sh, TblOp::DelRow);
            return Some(None);
        }
        if c.ctrl || c.alt {
            return None;
        }
        match vk {
            v if v == VK_TAB.0 && !c.shift => {
                if col + 1 < cols {
                    self.table_goto(sh, r, col + 1);
                } else if r + 1 < rows {
                    self.table_goto(sh, r + 1, 0);
                } else {
                    self.table_rewrite(sh, |tb| {
                        let n = tb.rows.len();
                        tb.insert_row(n);
                        Some((n, 0))
                    });
                }
            }
            v if v == VK_TAB.0 => {
                if col > 0 {
                    self.table_goto(sh, r, col - 1);
                } else if r > 0 {
                    self.table_goto(sh, r - 1, cols - 1);
                }
            }
            v if v == VK_RETURN.0 && c.shift => self.with_ed(sh, |ed| ed.type_text("<br>", 0)),
            v if v == VK_RETURN.0 => {
                if r + 1 < rows {
                    self.table_goto(sh, r + 1, col);
                } else {
                    self.table_rewrite(sh, |tb| {
                        let n = tb.rows.len();
                        tb.insert_row(n);
                        Some((n, col))
                    });
                }
            }
            v if (v == VK_UP.0 || v == VK_DOWN.0) && !c.shift => {
                let up = v == VK_UP.0;
                if up && r > 0 {
                    self.table_goto(sh, r - 1, col);
                } else if !up && r + 1 < rows {
                    self.table_goto(sh, r + 1, col);
                } else {
                    self.live_adjacent(sh, up, None);
                }
            }
            v if v == VK_LEFT.0 && !c.shift && caret <= cs => {
                if col > 0 {
                    self.table_goto(sh, r, col - 1);
                } else if r > 0 {
                    self.table_goto(sh, r - 1, cols - 1);
                } else {
                    self.live_adjacent(sh, true, None);
                }
            }
            v if v == VK_RIGHT.0 && !c.shift && caret >= ce => {
                if col + 1 < cols {
                    let start = t.grid.content(r, col + 1).map_or(0, |x| t.start + x.start);
                    if let Some(e) = self.edit_mut() {
                        e.ed.move_to(start, false);
                    }
                    self.after_edit(sh, false);
                } else if r + 1 < rows {
                    let start = t.grid.content(r + 1, 0).map_or(0, |x| t.start + x.start);
                    if let Some(e) = self.edit_mut() {
                        e.ed.move_to(start, false);
                    }
                    self.after_edit(sh, false);
                } else {
                    self.live_adjacent(sh, false, None);
                }
            }
            v if v == VK_HOME.0 => {
                if let Some(e) = self.edit_mut() {
                    e.ed.move_to(cs, c.shift);
                }
                self.after_edit(sh, false);
            }
            v if v == VK_END.0 => {
                if let Some(e) = self.edit_mut() {
                    e.ed.move_to(ce, c.shift);
                }
                self.after_edit(sh, false);
            }
            // The pipes stay: nothing to delete past the cell edges.
            v if v == VK_BACK.0 && caret <= cs && self.edit()?.ed.selection().is_empty() => {}
            v if v == VK_DELETE.0 && caret >= ce && self.edit()?.ed.selection().is_empty() => {}
            _ => return None,
        }
        self.invalidate();
        Some(None)
    }

    /// Replaces the active table with `f`'s result (pretty-aligned per the setting) as one undo
    /// step; the caret goes to the returned cell. `f` returning `None` changes nothing.
    pub(in crate::window) fn table_rewrite(
        &mut self,
        sh: &Shared,
        f: impl FnOnce(&mut table::Table) -> Option<(usize, usize)>,
    ) -> bool {
        let Some(at) = self.active_table() else {
            return false;
        };
        let Some(mut tb) = table::parse(&at.text) else {
            return false;
        };
        let Some((r, c)) = f(&mut tb) else {
            return false;
        };
        let eol = self.edit().map_or("\n", |e| e.ed.eol);
        let tail = &at.text[at.text.trim_end().len()..];
        let new = tb.to_markdown(eol, pretty()) + tail;
        let caret = table::grid(&new)
            .and_then(|g| {
                g.content(
                    r.min(g.rows.len().saturating_sub(1)),
                    c.min(g.cols().saturating_sub(1)),
                )
            })
            .map_or(at.start, |x| at.start + x.end);
        let range = at.start..at.start + at.text.len();
        self.with_ed(sh, |ed| {
            ed.seal();
            ed.replace(range, &new, caret);
            ed.seal();
        });
        self.invalidate();
        true
    }

    /// Leaving an edited table: re-serialize it pretty-aligned (R-TBL-05) when it changed;
    /// merged into the last edit step. Called before the document syncs.
    pub(in crate::window) fn table_format_on_leave(&mut self) {
        if !pretty() {
            return;
        }
        let Some(at) = self.active_table() else {
            return;
        };
        let (old, eol) = match &self.view {
            View::Doc(d) => {
                let Some(a) = d.edit.as_deref().and_then(|e| e.active) else {
                    return;
                };
                (
                    d.content
                        .source()
                        .get(a.start..a.start + a.old_len)
                        .unwrap_or("")
                        .to_string(),
                    d.edit.as_deref().map_or("\n", |e| e.ed.eol),
                )
            }
            _ => return,
        };
        if old == at.text {
            return;
        }
        let Some(tb) = table::parse(&at.text) else {
            return;
        };
        let tail = &at.text[at.text.trim_end().len()..];
        let new = tb.to_markdown(eol, true) + tail;
        if new == at.text {
            return;
        }
        if let Some(e) = self.edit_mut() {
            let caret = e.ed.caret();
            let r = at.start..at.start + at.text.len();
            let c = if caret >= r.end {
                caret - r.len() + new.len()
            } else {
                caret.min(at.start + new.len())
            };
            e.ed.replace(r, &new, c);
        }
    }

    /// Client point → buffer offset inside a cell of the active table.
    pub(in crate::window) fn table_hit(&self, sh: &Shared, x: f32, y: f32) -> Option<usize> {
        let View::Doc(d) = &self.view else {
            return None;
        };
        let e = d.edit.as_deref()?;
        let a = e.active?;
        let sb = d.viewer.source_block.as_ref()?;
        sb.table.as_ref()?;
        let bl = d.viewer.layout_of(a.block)?;
        let (cx, _) = self.col(sh);
        let top = (d.viewer.block_top(&d.content, a.block) - d.viewer.scroll) as f32;
        let (bx, by) = (x - cx, y - top);
        let cell = bl.hot.iter().find(|h| {
            matches!(h.kind, yamdv_layout::block::HotKind::Cell { .. }) && h.rect.contains(bx, by)
        })?;
        let p = bl
            .paras
            .iter()
            .find(|p| cell.rect.contains(p.x.max(cell.rect.x + 1.0), p.y + 1.0))?;
        let idx = p.shaped.hit(bx - p.x, by - p.y);
        p.map.as_ref().map(|m| m.to_source(idx) as usize)
    }
}
