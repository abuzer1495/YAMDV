//! `ui v2f-*` driver verbs (dev-tools): no DirectWrite hit test under the test runner.

use super::*;
use std::cell::RefCell;

thread_local! {
    /// Per-block hashes of the document file at `ui v2f-hash-mark`.
    static MARK: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
}

/// FNV-1a 64.
fn fnv(b: &[u8]) -> u64 {
    b.iter().fold(0xcbf2_9ce4_8422_2325, |h, &x| {
        (h ^ u64::from(x)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Hashes of the raw byte blocks of `bytes` (non-blank lines plus their trailing blank lines;
/// the BOM belongs to the first block).
/// A block's bytes without the blank lines that separate it from the next one.
fn trim_blank(b: &[u8]) -> &[u8] {
    let n = b
        .iter()
        .rposition(|c| !c.is_ascii_whitespace())
        .map_or(0, |i| i + 1);
    let end = b
        .get(n..)
        .and_then(|t| t.iter().position(|&c| c == b'\n'))
        .map_or(b.len(), |i| n + i + 1);
    b.get(..end).unwrap_or(b)
}

/// Hashes of the blank-line separated blocks of a file.
fn block_hashes(bytes: &[u8]) -> Vec<u64> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut seen_text = false;
    let mut pos = 0;
    for line in bytes.split_inclusive(|&c| c == b'\n') {
        let blank = line.iter().all(|c| c.is_ascii_whitespace());
        if !blank && seen_text && pos > start {
            // A text line after blank lines starts a new block.
            let prev_blank = bytes
                .get(..pos)
                .is_some_and(|p| p.ends_with(b"\n\n") || p.ends_with(b"\n\r\n"));
            if prev_blank {
                out.push(fnv(trim_blank(bytes.get(start..pos).unwrap_or(&[]))));
                start = pos;
            }
        }
        seen_text |= !blank;
        pos += line.len();
    }
    if pos > start {
        out.push(fnv(trim_blank(bytes.get(start..pos).unwrap_or(&[]))));
    }
    out
}

impl Win {
    fn doc_file_bytes(&self) -> Vec<u8> {
        match &self.view {
            View::Doc(d) if !d.virt => {
                std::fs::read(yamdv_core::path::long_form(&d.path)).unwrap_or_default()
            }
            _ => Vec::new(),
        }
    }

    /// Driver probes: blocks changed on disk since the mark, BOM present, all-CRLF endings.
    pub(in crate::window) fn v2f_file_probe(&self) -> (f64, f64, f64) {
        let b = self.doc_file_bytes();
        let now = block_hashes(&b);
        // Blocks of the file that are not byte-identical to a block of the marked file.
        let changed = MARK.with(|m| {
            let mut old = m.borrow().clone();
            now.iter()
                .filter(|h| {
                    old.iter()
                        .position(|o| o == *h)
                        .map(|i| old.swap_remove(i))
                        .is_none()
                })
                .count()
        });
        let bom = b.starts_with(&[0xEF, 0xBB, 0xBF]);
        let lf = b.iter().filter(|&&c| c == b'\n').count();
        let crlf = b.windows(2).filter(|w| w == b"\r\n").count();
        (
            changed as f64,
            f64::from(u8::from(bom)),
            f64::from(u8::from(lf > 0 && lf == crlf)),
        )
    }
}

impl Win {
    /// `ui v2f-*` verbs.
    pub(in crate::window) fn v2f_ui(&mut self, sh: &Shared, verb: &str, arg: &str) -> String {
        let ok = match verb {
            // F2: opens the block holding `needle` and puts the caret `k` bytes into it
            // (`arg` = "k|needle"; selection with "k-j|needle").
            "v2f-caret" => {
                let (spec, needle) = arg.split_once('|').unwrap_or(("0", arg));
                let (a, b) = spec.split_once('-').unwrap_or((spec, spec));
                let (a, b) = (a.parse().unwrap_or(0usize), b.parse().unwrap_or(0usize));
                let _ = self.edit_leave_block(sh);
                let at = match &self.view {
                    View::Doc(d) => d
                        .content
                        .source()
                        .find(needle)
                        .map(|p| (p, crate::window::edit::block_of(&d.content, p))),
                    _ => None,
                };
                at.is_some_and(|(p, bi)| {
                    self.enter_block(sh, bi, p + a);
                    if let Some(e) = self.edit_mut() {
                        e.ed.set_selection(p + a, p + b);
                    }
                    self.after_edit(sh, false);
                    true
                })
            }
            // F2: click on the task checkbox at `needle` + k (inside the active block).
            "v2f-task-click" => {
                let (k, needle) = arg.split_once('|').unwrap_or(("0", arg));
                let pos = self
                    .edit()
                    .and_then(|e| e.ed.text().find(needle))
                    .map(|p| p + k.parse().unwrap_or(0usize));
                pos.is_some_and(|p| self.f2_task_click(sh, p))
            }
            // Table command by name (`row-above`, `sort-desc`, …).
            "v2f-table" => match super::table::TblOp::from_name(arg) {
                Some(op) => {
                    self.table_op(sh, op);
                    true
                }
                None => false,
            },
            // Grip drag through the mouse handlers: "col|row FROM TO".
            "v2f-grip" => {
                let p: Vec<&str> = arg.split_whitespace().collect();
                match (
                    p.first(),
                    p.get(1).and_then(|s| s.parse().ok()),
                    p.get(2).and_then(|s| s.parse().ok()),
                ) {
                    (Some(k), Some(f), Some(t)) => self.grip_drive(sh, *k == "col", f, t),
                    _ => false,
                }
            }
            "v2f-tsv-convert" => self.tsv_convert(sh),
            // Click the tool strip button `arg` through the mouse handler.
            "v2f-tool" => {
                let r = super::table::TblOp::from_name(arg).and_then(|op| {
                    self.table_tools(sh)
                        .into_iter()
                        .find(|(_, o)| *o == op)
                        .map(|t| t.0)
                });
                r.is_some_and(|r| self.v2f_down(sh, r.x + r.w / 2.0, r.y + r.h / 2.0))
            }
            "v2f-hash-mark" => {
                let h = block_hashes(&self.doc_file_bytes());
                MARK.with(|m| *m.borrow_mut() = h);
                true
            }
            _ => false,
        };
        if ok {
            "ok".into()
        } else {
            format!("err {verb}")
        }
    }
}
