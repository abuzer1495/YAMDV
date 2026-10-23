//! TSV / CSV paste (R-TBL-07, RFC 4180): inside a table the data fills cells from the caret
//! cell (rows / columns grow as needed); outside, the text is pasted and a "Convert to table"
//! offer appears. Plain text pasted in a cell keeps one line with `|` escaped.

use super::*;
use yamdv_editor::tsv;

/// The open "Convert to table" offer for the just pasted text.
#[derive(Debug, Clone)]
pub(in crate::window) struct TsvOffer {
    /// Buffer range of the pasted text.
    pub range: std::ops::Range<usize>,
    /// Buffer version right after the paste (any later edit drops the offer).
    pub version: u64,
    pub rows: Vec<Vec<String>>,
}

impl Win {
    /// Paste hook: true when handled here.
    pub(in crate::window) fn table_paste(&mut self, sh: &Shared, text: &str) -> bool {
        if self.edit_mode() != Some(Mode::Live) {
            return false;
        }
        if let Some((_, (r, c))) = self.table_cell() {
            match tsv::detect(text) {
                Some((rows, _)) => {
                    self.table_rewrite(sh, |tb| {
                        let need_r = r + rows.len();
                        let need_c = c + rows.iter().map(Vec::len).max().unwrap_or(0);
                        while tb.rows.len() < need_r {
                            let n = tb.rows.len();
                            tb.insert_row(n);
                        }
                        while tb.cols() < need_c {
                            let n = tb.cols();
                            tb.insert_col(n);
                        }
                        for (i, row) in rows.iter().enumerate() {
                            for (j, v) in row.iter().enumerate() {
                                if let Some(cell) =
                                    tb.rows.get_mut(r + i).and_then(|x| x.get_mut(c + j))
                                {
                                    *cell = table::escape_cell(v.trim());
                                }
                            }
                        }
                        Some((need_r - 1, need_c - 1))
                    });
                }
                None => {
                    let one = table::escape_cell(text.trim_end_matches(['\r', '\n']));
                    self.with_ed(sh, |ed| ed.paste(&one));
                }
            }
            return true;
        }
        let Some((rows, _)) = tsv::detect(text) else {
            return false;
        };
        let start = self.edit().map_or(0, |e| e.ed.selection().start);
        self.with_ed(sh, |ed| ed.paste(text));
        let (end, version) = self.edit().map_or((0, 0), |e| (e.ed.caret(), e.ed.version));
        self.f2_paste_split(sh);
        self.wysi.tsv_offer = Some(TsvOffer {
            range: start..end,
            version,
            rows,
        });
        self.invalidate();
        true
    }

    /// The offer, while the buffer is unchanged since the paste.
    fn tsv_live(&self) -> Option<&TsvOffer> {
        let o = self.wysi.tsv_offer.as_ref()?;
        (self.edit()?.ed.version == o.version).then_some(o)
    }

    fn tsv_rect(&self, sh: &Shared) -> Option<Rect> {
        self.tsv_live()?;
        let c = self.caret_rect(sh)?;
        let t = self.shape_ui(sh, tr(sh.lang, Msg::TblConvert), 12.0, 600, 0xffffff, None);
        Some(Rect::new(
            c.x.max(8.0).round(),
            (c.y + c.h + 6.0).round(),
            t.width() + 20.0,
            24.0,
        ))
    }

    /// Converts the pasted text into a table (one undo step).
    pub(in crate::window) fn tsv_convert(&mut self, sh: &Shared) -> bool {
        let Some(o) = self.tsv_live().cloned() else {
            return false;
        };
        self.wysi.tsv_offer = None;
        let Some(tb) = table::from_rows(&o.rows) else {
            return false;
        };
        let Some(e) = self.edit() else { return false };
        let eol = e.ed.eol;
        let before = e.ed.slice(o.range.start.saturating_sub(2)..o.range.start);
        let after = e.ed.slice(o.range.end..(o.range.end + 2).min(e.ed.len()));
        let mut md = String::new();
        if !before.is_empty() && !before.ends_with("\n\n") && !before.ends_with('\n') {
            md.push_str(eol);
        }
        if !before.is_empty() && !before.ends_with("\n\n") {
            md.push_str(eol);
        }
        let body = tb.to_markdown(eol, pretty());
        let caret = o.range.start + md.len() + body.len();
        md.push_str(&body);
        md.push_str(eol);
        if !after.is_empty() && !after.starts_with('\n') && !after.starts_with("\r\n") {
            md.push_str(eol);
        }
        let range = o.range.clone();
        self.with_ed(sh, |ed| {
            ed.seal();
            ed.replace(range, &md, caret);
            ed.seal();
        });
        self.f2_reopen(sh);
        self.invalidate();
        true
    }

    /// Mouse down on the offer.
    pub(in crate::window) fn tsv_down(&mut self, sh: &Shared, x: f32, y: f32) -> bool {
        match self.tsv_rect(sh) {
            Some(r) if r.contains(x, y) => self.tsv_convert(sh),
            Some(_) => {
                self.wysi.tsv_offer = None;
                self.invalidate();
                false
            }
            None => false,
        }
    }

    /// Paints the offer pill.
    pub(in crate::window) fn tsv_draw(&self, sh: &Shared, b: &Band<'_>) {
        let Some(r) = self.tsv_rect(sh) else { return };
        b.fill(r, 0x202020, 0.85, 6.0);
        let t = self.shape_ui(sh, tr(sh.lang, Msg::TblConvert), 12.0, 600, 0xffffff, None);
        b.text(
            t.as_ref(),
            (r.x + 10.0).round(),
            (r.y + (r.h - t.height()) / 2.0).round(),
        );
    }

    /// Driver probe: the offer is shown.
    pub(in crate::window) fn tsv_offered(&self) -> bool {
        self.tsv_live().is_some()
    }
}
