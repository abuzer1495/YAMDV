//! GFM table model for F2 table editing (R-TBL-01…08, prompt 016 §3 Part 2): parsing a table
//! block into cells, cell source ranges, structural edits, natural Turkish-aware sorting and
//! pretty-aligned re-serialization with East-Asian-width padding.

use std::cmp::Ordering;
use std::ops::Range;

/// Column alignment of the delimiter row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    /// `---`
    #[default]
    None,
    /// `:--`
    Left,
    /// `:-:`
    Center,
    /// `--:`
    Right,
}

impl Align {
    /// Layout code (0 none, 1 left, 2 center, 3 right).
    pub fn code(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Left => 1,
            Self::Center => 2,
            Self::Right => 3,
        }
    }
}

/// A parsed table: cell sources (escaped Markdown, trimmed), row 0 is the header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// Rows of cells; every row has `aligns.len()` cells.
    pub rows: Vec<Vec<String>>,
    /// Column alignments.
    pub aligns: Vec<Align>,
}

/// One cell of a table block: its segment between pipes and its trimmed content (byte ranges
/// in the block text). An empty cell's content is an empty range where typing goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellSpan {
    /// Between the pipes.
    pub seg: Range<usize>,
    /// Trimmed content.
    pub content: Range<usize>,
}

/// Cell layout of a table block (rows exclude the delimiter row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    /// Cells per row (row 0 = header).
    pub rows: Vec<Vec<CellSpan>>,
    /// Column alignments.
    pub aligns: Vec<Align>,
    /// Byte range of each row's line (without its line break); index 1 of `lines` is the
    /// delimiter row.
    pub lines: Vec<Range<usize>>,
}

impl Grid {
    /// Number of columns.
    pub fn cols(&self) -> usize {
        self.aligns.len()
    }

    /// Cell `(row, col)` holding byte offset `pos` (on a pipe: the cell to its right; past
    /// the last pipe: the last cell of the row).
    pub fn cell_at(&self, pos: usize) -> Option<(usize, usize)> {
        for (r, row) in self.rows.iter().enumerate() {
            let line = self.row_line(r)?;
            if pos < line.start || pos > line.end {
                continue;
            }
            let c = row
                .iter()
                .position(|s| pos <= s.seg.end)
                .unwrap_or(row.len().saturating_sub(1));
            return Some((r, c));
        }
        None
    }

    /// Line range of grid row `r`.
    pub fn row_line(&self, r: usize) -> Option<Range<usize>> {
        let k = if r == 0 { 0 } else { r + 1 };
        self.lines.get(k).cloned()
    }

    /// Content range of cell `(r, c)`.
    pub fn content(&self, r: usize, c: usize) -> Option<Range<usize>> {
        self.rows.get(r)?.get(c).map(|s| s.content.clone())
    }
}

/// Splits a row line at unescaped pipes: segment ranges relative to the line.
fn split_row(line: &str) -> Vec<Range<usize>> {
    let b = line.as_bytes();
    let mut cuts = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b.get(i) {
            Some(b'\\') => i += 2,
            Some(b'|') => {
                cuts.push(i);
                i += 1;
            }
            _ => i += 1,
        }
    }
    let lead = line.len() - line.trim_start().len();
    let trail = line.trim_end().len();
    let mut segs = Vec::new();
    let mut start = 0;
    let mut first = true;
    for &p in &cuts {
        if !(first && p == lead) {
            segs.push(start..p);
        }
        first = false;
        start = p + 1;
    }
    let ends_pipe = cuts.last().is_some_and(|&p| p + 1 == trail);
    if !ends_pipe && start <= line.len() {
        segs.push(start..line.len());
    }
    segs
}

fn delim_align(cell: &str) -> Option<Align> {
    let t = cell.trim();
    let l = t.starts_with(':');
    let r = t.ends_with(':') && t.len() > 1;
    let dashes = t.trim_start_matches(':').trim_end_matches(':');
    if dashes.is_empty() || !dashes.bytes().all(|c| c == b'-') {
        return None;
    }
    Some(match (l, r) {
        (true, true) => Align::Center,
        (true, false) => Align::Left,
        (false, true) => Align::Right,
        (false, false) => Align::None,
    })
}

/// Cell layout of `text` when it is exactly one GFM table (header, delimiter, body rows).
pub fn grid(text: &str) -> Option<Grid> {
    let mut lines = Vec::new();
    let mut s = 0;
    for l in text.split_inclusive('\n') {
        let body = l.trim_end_matches('\n').trim_end_matches('\r');
        lines.push(s..s + body.len());
        s += l.len();
    }
    while lines
        .last()
        .is_some_and(|r| text.get(r.clone()).is_some_and(|t| t.trim().is_empty()))
    {
        lines.pop();
    }
    if lines.len() < 2 {
        return None;
    }
    let line = |k: usize| lines.get(k).and_then(|r| text.get(r.clone())).unwrap_or("");
    if !line(0).contains('|') && !line(1).contains('|') {
        return None;
    }
    let aligns: Vec<Align> = split_row(line(1))
        .into_iter()
        .map(|r| line(1).get(r).and_then(delim_align))
        .collect::<Option<_>>()?;
    let head = split_row(line(0));
    if aligns.is_empty()
        || head.len() != aligns.len()
        || line(0).trim_start().starts_with(['>', '#'])
    {
        return None;
    }
    let mut rows = Vec::new();
    for (k, lr) in lines.iter().enumerate() {
        if k == 1 {
            continue;
        }
        let l = line(k);
        if k > 1 && l.trim().is_empty() {
            return None;
        }
        let row = split_row(l)
            .into_iter()
            .map(|sg| {
                let seg = lr.start + sg.start..lr.start + sg.end;
                let raw = l.get(sg.clone()).unwrap_or("");
                let lead = raw.len() - raw.trim_start().len();
                let t = raw.trim();
                let content = if t.is_empty() {
                    let at = seg.start + usize::from(!raw.is_empty());
                    at..at
                } else {
                    seg.start + lead..seg.start + lead + t.len()
                };
                CellSpan { seg, content }
            })
            .collect();
        rows.push(row);
    }
    Some(Grid {
        rows,
        aligns,
        lines,
    })
}

/// Parses `text` into a [`Table`] (rows padded / cut to the header width).
pub fn parse(text: &str) -> Option<Table> {
    let g = grid(text)?;
    let n = g.cols();
    let rows = g
        .rows
        .iter()
        .map(|row| {
            let mut v: Vec<String> = row
                .iter()
                .take(n)
                .map(|s| text.get(s.content.clone()).unwrap_or("").to_string())
                .collect();
            v.resize(n, String::new());
            v
        })
        .collect();
    Some(Table {
        rows,
        aligns: g.aligns,
    })
}

/// Display width of `s` in monospace columns (East Asian wide / fullwidth = 2, combining = 0).
pub fn display_width(s: &str) -> usize {
    s.chars().map(char_width).sum()
}

fn char_width(c: char) -> usize {
    let u = c as u32;
    if matches!(u, 0x300..=0x36F | 0x200B..=0x200F | 0xFE00..=0xFE0F | 0x20D0..=0x20FF) {
        return 0;
    }
    let wide = matches!(u,
        0x1100..=0x115F | 0x2E80..=0x303E | 0x3041..=0x33FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF
        | 0xA000..=0xA4CF | 0xAC00..=0xD7A3 | 0xF900..=0xFAFF | 0xFE30..=0xFE4F
        | 0xFF00..=0xFF60 | 0xFFE0..=0xFFE6 | 0x1F300..=0x1F64F | 0x1F900..=0x1F9FF
        | 0x20000..=0x3FFFD);
    if wide { 2 } else { 1 }
}

impl Table {
    /// Number of columns.
    pub fn cols(&self) -> usize {
        self.aligns.len()
    }

    /// Serializes with `eol`; `pretty` pads every column to its widest cell (East-Asian
    /// width aware) and aligns the padding with the column.
    pub fn to_markdown(&self, eol: &str, pretty: bool) -> String {
        let n = self.cols();
        let mut w = vec![3usize; n];
        if pretty {
            for row in &self.rows {
                for (c, cell) in row.iter().enumerate().take(n) {
                    if let Some(x) = w.get_mut(c) {
                        *x = (*x).max(display_width(cell));
                    }
                }
            }
        }
        let mut out = String::new();
        let cell_line = |row: &Vec<String>, out: &mut String| {
            out.push('|');
            for c in 0..n {
                let cell = row.get(c).map_or("", String::as_str);
                let width = if pretty {
                    w.get(c).copied().unwrap_or(3)
                } else {
                    0
                };
                let pad = width.saturating_sub(display_width(cell));
                let (l, r) = match self.aligns.get(c) {
                    Some(Align::Right) => (pad, 0),
                    Some(Align::Center) => (pad / 2, pad - pad / 2),
                    _ => (0, pad),
                };
                out.push(' ');
                out.push_str(&" ".repeat(l));
                out.push_str(cell);
                out.push_str(&" ".repeat(r));
                out.push_str(" |");
            }
        };
        for (k, row) in self.rows.iter().enumerate() {
            if k > 0 {
                out.push_str(eol);
            }
            cell_line(row, &mut out);
            if k == 0 {
                out.push_str(eol);
                out.push('|');
                for c in 0..n {
                    let width = w.get(c).copied().unwrap_or(3);
                    let a = self.aligns.get(c).copied().unwrap_or_default();
                    let (l, r) = match a {
                        Align::None => ("", ""),
                        Align::Left => (":", ""),
                        Align::Center => (":", ":"),
                        Align::Right => ("", ":"),
                    };
                    let dashes = width.saturating_sub(l.len() + r.len()).max(1);
                    out.push(' ');
                    out.push_str(l);
                    out.push_str(&"-".repeat(dashes));
                    out.push_str(r);
                    out.push_str(" |");
                }
            }
        }
        out
    }

    /// Inserts an empty body row at grid row `at` (1 ..= rows).
    pub fn insert_row(&mut self, at: usize) {
        let at = at.clamp(1, self.rows.len());
        self.rows.insert(at, vec![String::new(); self.cols()]);
    }

    /// Inserts an empty column at `at` (0 ..= cols).
    pub fn insert_col(&mut self, at: usize) {
        let at = at.min(self.cols());
        for row in &mut self.rows {
            row.insert(at.min(row.len()), String::new());
        }
        self.aligns.insert(at, Align::None);
    }

    /// Deletes body row `r` (the header stays). False when `r` is the header or out of range.
    pub fn delete_row(&mut self, r: usize) -> bool {
        if r == 0 || r >= self.rows.len() {
            return false;
        }
        self.rows.remove(r);
        true
    }

    /// Deletes column `c`; false when it is the only column.
    pub fn delete_col(&mut self, c: usize) -> bool {
        if self.cols() <= 1 || c >= self.cols() {
            return false;
        }
        for row in &mut self.rows {
            if c < row.len() {
                row.remove(c);
            }
        }
        self.aligns.remove(c);
        true
    }

    /// Sets the alignment of column `c`.
    pub fn set_align(&mut self, c: usize, a: Align) {
        if let Some(x) = self.aligns.get_mut(c) {
            *x = a;
        }
    }

    /// Sorts the body rows by column `c` (natural, Turkish collation, stable).
    pub fn sort(&mut self, c: usize, descending: bool) {
        if self.rows.len() < 2 {
            return;
        }
        let mut body = self.rows.split_off(1);
        body.sort_by(|a, b| {
            let o = natural_cmp(
                a.get(c).map_or("", String::as_str),
                b.get(c).map_or("", String::as_str),
            );
            if descending { o.reverse() } else { o }
        });
        self.rows.extend(body);
    }

    /// Moves body row `from` to `to` (grid rows, both ≥ 1).
    pub fn move_row(&mut self, from: usize, to: usize) -> bool {
        let n = self.rows.len();
        if from == 0 || to == 0 || from >= n || to >= n || from == to {
            return false;
        }
        let r = self.rows.remove(from);
        self.rows.insert(to, r);
        true
    }

    /// Moves column `from` to `to`.
    pub fn move_col(&mut self, from: usize, to: usize) -> bool {
        let n = self.cols();
        if from >= n || to >= n || from == to {
            return false;
        }
        for row in &mut self.rows {
            if from < row.len() && to < row.len() {
                let x = row.remove(from);
                row.insert(to, x);
            }
        }
        let a = self.aligns.remove(from);
        self.aligns.insert(to, a);
        true
    }
}

/// Turkish alphabet rank of a lowercase letter (others: by code point after the alphabet).
fn tr_rank(c: char) -> u32 {
    const ALPHA: &str = "abcçdefgğhıijklmnoöprsştuüvyz";
    match ALPHA.chars().position(|x| x == c) {
        Some(p) => u32::try_from(p).unwrap_or(0) + 1,
        None => 100 + c as u32,
    }
}

fn tr_lower(c: char) -> char {
    match c {
        'I' => 'ı',
        'İ' => 'i',
        _ => c.to_lowercase().next().unwrap_or(c),
    }
}

/// Natural, case-insensitive comparison with Turkish collation (`ç` after `c`, `ı` before
/// `i`, digit runs by value).
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut x = a.chars().peekable();
    let mut y = b.chars().peekable();
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return a.cmp(b),
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(p), Some(q)) if p.is_ascii_digit() && q.is_ascii_digit() => {
                let mut na = String::new();
                while let Some(d) = x.peek().copied().filter(char::is_ascii_digit) {
                    na.push(d);
                    x.next();
                }
                let mut nb = String::new();
                while let Some(d) = y.peek().copied().filter(char::is_ascii_digit) {
                    nb.push(d);
                    y.next();
                }
                let (ta, tb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
                let o = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(p), Some(q)) => {
                let o = tr_rank(tr_lower(p)).cmp(&tr_rank(tr_lower(q)));
                if o != Ordering::Equal {
                    return o;
                }
                x.next();
                y.next();
            }
        }
    }
}

/// Escapes unescaped pipes for a cell (`|` → `\|`) and turns line breaks into `<br>`.
pub fn escape_cell(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_bs = false;
    let t = s.replace("\r\n", "\n");
    for c in t.chars() {
        match c {
            '|' if !prev_bs => out.push_str("\\|"),
            '\n' => out.push_str("<br>"),
            _ => out.push(c),
        }
        prev_bs = c == '\\' && !prev_bs;
    }
    out
}

/// Cell source to plain text: `\|` → `|`, `<br>` → line break.
pub fn unescape_cell(s: &str) -> String {
    s.replace("\\|", "|").replace("<br>", "\n")
}

/// A new table from plain-text rows (the first row is the header).
pub fn from_rows(rows: &[Vec<String>]) -> Option<Table> {
    let n = rows.iter().map(Vec::len).max()?;
    if n == 0 {
        return None;
    }
    Some(Table {
        rows: rows
            .iter()
            .map(|r| {
                let mut v: Vec<String> = r.iter().map(|c| escape_cell(c.trim())).collect();
                v.resize(n, String::new());
                v
            })
            .collect(),
        aligns: vec![Align::None; n],
    })
}
