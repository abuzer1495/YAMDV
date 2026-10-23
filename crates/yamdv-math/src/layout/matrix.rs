//! Matrix-like environments (R-MATH-02): grids with per-kind style, gaps, alignment, row
//! stretch, `array` column specs and vertical rules.

use super::{B, L, Op, St};
use crate::parse::{Env, MathError, Node};
use crate::table as t;

impl L<'_> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn matrix(
        &mut self,
        env: Env,
        rows: &[Vec<Node>],
        cols: &[char],
        st: St,
        color: Option<u32>,
    ) -> Result<B, MathError> {
        let cst = match env {
            Env::Aligned | Env::Gathered | Env::DCases => St { level: 0, ..st },
            Env::Small => St {
                level: st.level.max(2),
                ..st
            },
            _ => St {
                level: st.level.max(1),
                ..st
            },
        };
        let em = self.size(st);
        let stretch = match env {
            Env::Cases | Env::DCases => 1.2,
            Env::Small => 0.7,
            _ => 1.0,
        };
        let strut_h = 0.84 * em * stretch;
        let strut_d = 0.36 * em * stretch;
        let jot = if matches!(env, Env::Aligned | Env::Gathered) {
            0.3 * em
        } else {
            0.0
        };
        let aligns: Vec<char> = cols.iter().copied().filter(|c| *c != '|').collect();
        let mut cells: Vec<Vec<B>> = Vec::with_capacity(rows.len());
        let mut ncols = 0usize;
        for row in rows {
            let mut r = Vec::with_capacity(row.len());
            for (ci, cell) in row.iter().enumerate() {
                let b = if env == Env::Aligned && ci % 2 == 1 {
                    // `{}` before the right-hand side so a leading relation gets its spacing.
                    let list = match cell {
                        Node::Group(v) => {
                            let mut l = vec![Node::Group(Vec::new())];
                            l.extend(v.iter().cloned());
                            l
                        }
                        n => vec![Node::Group(Vec::new()), n.clone()],
                    };
                    self.hlist(&list, cst, color)?
                } else {
                    self.node(cell, cst, color)?.0
                };
                r.push(b);
            }
            ncols = ncols.max(r.len());
            cells.push(r);
        }
        if env == Env::Array {
            ncols = ncols.max(aligns.len());
        }
        let mut widths = vec![0.0f32; ncols];
        for r in &cells {
            for (ci, b) in r.iter().enumerate() {
                if let Some(w) = widths.get_mut(ci) {
                    *w = w.max(b.w);
                }
            }
        }
        let gap_after = |ci: usize| -> f32 {
            match env {
                Env::Aligned => {
                    if ci % 2 == 1 {
                        2.0 * em
                    } else {
                        0.0
                    }
                }
                Env::Small => em / 3.0,
                _ => em,
            }
        };
        // `array` rules: boundary k (0 = before the first column, ncols = after the last).
        let mut rules_at: Vec<usize> = Vec::new();
        let mut k = 0usize;
        for c in cols {
            if *c == '|' {
                rules_at.push(k);
            } else {
                k += 1;
            }
        }
        let lead = if rules_at.first() == Some(&0) {
            em / 2.0
        } else {
            0.0
        };
        let mut xs = Vec::with_capacity(ncols);
        let mut x = lead;
        for (ci, w) in widths.iter().enumerate() {
            xs.push(x);
            x += w;
            if ci + 1 < ncols {
                x += gap_after(ci);
            }
        }
        let trail = if rules_at.last() == Some(&ncols) && ncols > 0 {
            em / 2.0
        } else {
            0.0
        };
        let total_w = x + trail;
        let mut out = B::default();
        let mut y = 0.0f32; // top of the current row (from the array top, downwards)
        for (ri, r) in cells.into_iter().enumerate() {
            let h = r.iter().map(|b| b.h).fold(strut_h, f32::max);
            let d = r.iter().map(|b| b.d).fold(strut_d, f32::max);
            if ri > 0 {
                y += jot;
            }
            let base = y + h;
            for (ci, b) in r.into_iter().enumerate() {
                let cw = widths.get(ci).copied().unwrap_or(0.0);
                let cx = xs.get(ci).copied().unwrap_or(0.0);
                let align = match env {
                    Env::Cases | Env::DCases => 'l',
                    Env::Aligned => {
                        if ci % 2 == 1 {
                            'l'
                        } else {
                            'r'
                        }
                    }
                    Env::Array => aligns.get(ci).copied().unwrap_or('c'),
                    _ => 'c',
                };
                let bx = match align {
                    'l' => cx,
                    'r' => cx + cw - b.w,
                    _ => cx + (cw - b.w) / 2.0,
                };
                out.put(b, bx, base);
            }
            y = base + d;
        }
        let th = self.c(st, t::FRACTION_RULE_THICKNESS);
        for k in rules_at {
            let rx = if k == 0 {
                0.0
            } else if k >= ncols {
                total_w - th
            } else {
                xs.get(k).copied().unwrap_or(0.0) - gap_after(k - 1) / 2.0
            };
            self.count(1)?;
            out.ops.push(Op::Rule {
                x: rx,
                y: 0.0,
                w: th,
                h: y,
                color,
            });
        }
        // Center the array on the axis.
        let axis = self.c(st, t::AXIS_HEIGHT);
        let shift = -(y / 2.0) - axis;
        let mut moved = B::default();
        moved.put(out, 0.0, shift);
        moved.h = moved.h.max(-shift);
        moved.d = moved.d.max(y + shift);
        moved.w = total_w;
        Ok(moved)
    }
}
