//! Layout of the V2-G constructs (R-MATH-02): style switches, `\limits`, `\middle`, stretchy
//! over/under glyphs (`\widehat`, arrows, braces), `\overset`, phantoms, cancel lines, `\boxed`.

use super::{B, L, NULL_DELIM, Op, St};
use crate::parse::{MathError, Node};
use crate::symbols::Class;
use crate::table as t;

impl L<'_> {
    pub(super) fn node_ext(
        &mut self,
        n: &Node,
        st: St,
        color: Option<u32>,
    ) -> Result<(B, Option<Class>), MathError> {
        Ok(match n {
            Node::Style { level, body } => {
                let s = St {
                    level: *level,
                    cramped: st.cramped,
                };
                (self.hlist(body, s, color)?, Some(Class::Ord))
            }
            Node::Limits { base, .. } => self.node(base, st, color)?,
            Node::Class { class, body } => (self.node(body, st, color)?.0, Some(*class)),
            Node::Middle(c) => {
                let target = self.middle.unwrap_or(self.size(st));
                let b = match c {
                    Some(c) => self.delim(*c, target, st, color)?,
                    None => B {
                        w: NULL_DELIM * self.size(st),
                        ..B::default()
                    },
                };
                (b, Some(Class::Ord))
            }
            Node::Stretch { c, over, body, .. } => (
                self.stretch_node(*c, *over, body, st, color)?,
                Some(Class::Ord),
            ),
            Node::Stack {
                base,
                over,
                under,
                class,
            } => {
                let b = self.node(base, st, color)?.0;
                let mut out = self.limits(b, over.as_deref(), under.as_deref(), st, color)?;
                out.glyph = None;
                (out, Some(*class))
            }
            Node::Phantom { body, h, v } => {
                let b = self.node(body, st, color)?.0;
                (
                    B {
                        w: if *h { b.w } else { 0.0 },
                        h: if *v { b.h } else { 0.0 },
                        d: if *v { b.d } else { 0.0 },
                        ..B::default()
                    },
                    Some(Class::Ord),
                )
            }
            Node::Cancel { kind, body } => {
                let (b, cls) = self.node(body, st, color)?;
                (
                    self.cancel(*kind, b, st, color)?,
                    if *kind == 3 { cls } else { Some(Class::Ord) },
                )
            }
            Node::Boxed(body) => {
                let b = self
                    .node(
                        body,
                        St {
                            level: st.level.min(1).max(st.level),
                            ..st
                        },
                        color,
                    )?
                    .0;
                (self.boxed(b, st, color)?, Some(Class::Ord))
            }
            // A `\tag` below the top level (inside a group) is ignored.
            _ => (B::default(), None),
        })
    }

    /// A horizontally stretched glyph at least `target` wide (variants, then assembly).
    fn stretch_h(
        &mut self,
        c: char,
        target: f32,
        st: St,
        color: Option<u32>,
    ) -> Result<B, MathError> {
        let g = self.gid(c);
        let k = self.k(st);
        let need = target / k;
        let vars = self.t.variants(g, false);
        if let Some(v) = vars.iter().find(|v| v.advance >= need) {
            return self.glyph_box(v.glyph, st, color);
        }
        let parts = self.t.assembly(g, false);
        if parts.is_empty() {
            let last = vars.last().map_or(g, |v| v.glyph);
            let mut b = self.glyph_box(last, st, color)?;
            // No assembly: scale nothing, but center the widest variant over the target.
            if b.w < target {
                let dx = (target - b.w) / 2.0;
                for o in &mut b.ops {
                    if let Op::Glyph { x, .. } = o {
                        *x += dx;
                    }
                }
                b.w = target;
            }
            return Ok(b);
        }
        let (placed, len) = t::assemble(&parts, self.t.min_overlap(), need);
        self.count(placed.len())?;
        let mut out = B::default();
        let m0 = self.font.metrics(g);
        let left = m0.x_min.min(0.0);
        for (pg, off) in placed {
            let m = self.font.metrics(pg);
            out.ops.push(Op::Glyph {
                gid: pg,
                x: (left + off - m.x_min.min(0.0)) * k,
                y: 0.0,
                size: self.size(st),
                color,
            });
            out.h = out.h.max(m.y_max * k);
            out.d = out.d.max(-m.y_min * k);
        }
        out.w = len * k;
        Ok(out)
    }

    fn stretch_node(
        &mut self,
        c: char,
        over: bool,
        body: &Node,
        st: St,
        color: Option<u32>,
    ) -> Result<B, MathError> {
        let b = self
            .node(body, if over { st.cramp() } else { st }, color)?
            .0;
        let accent = matches!(c, '\u{302}' | '\u{303}');
        let g = self.stretch_h(c, b.w, st, color)?;
        let gap = 0.1 * self.size(st);
        let (bw, bh, bd) = (b.w, b.h, b.d);
        let (gw, gh, gd) = (g.w, g.h, g.d);
        // Center both on the wider of the two.
        let w = bw.max(gw);
        let mut out = B::default();
        out.put(b, (w - bw) / 2.0, 0.0);
        let y = if over {
            if accent {
                // Accent glyphs sit above the x-height by design: lift by the excess height.
                -(bh - self.c(st, t::ACCENT_BASE_HEIGHT)).max(0.0)
            } else {
                -(bh + gap + gd)
            }
        } else {
            bd + gap + gh
        };
        out.put(g, (w - gw) / 2.0, y);
        out.w = w;
        Ok(out)
    }

    fn line(
        &mut self,
        out: &mut B,
        p: (f32, f32),
        q: (f32, f32),
        w: f32,
        color: Option<u32>,
    ) -> Result<(), MathError> {
        self.count(1)?;
        out.ops.push(Op::Line {
            x1: p.0,
            y1: p.1,
            x2: q.0,
            y2: q.1,
            w,
            color,
        });
        Ok(())
    }

    fn cancel(&mut self, kind: u8, b: B, st: St, color: Option<u32>) -> Result<B, MathError> {
        let th = self.c(st, t::FRACTION_RULE_THICKNESS).max(0.5);
        let (w, h, d) = (b.w, b.h, b.d);
        let mut out = b;
        out.glyph = None;
        let pad = 0.05 * self.size(st);
        let (l, r, top, bot) = (-pad, w + pad, -(h + pad), d + pad);
        match kind {
            0 => self.line(&mut out, (l, bot), (r, top), th, color)?,
            1 => self.line(&mut out, (l, top), (r, bot), th, color)?,
            2 => {
                self.line(&mut out, (l, bot), (r, top), th, color)?;
                self.line(&mut out, (l, top), (r, bot), th, color)?;
            }
            _ => {
                // `\not`: a slash through the middle, about the height of a relation.
                let cx = w / 2.0;
                let half = 0.35 * self.size(st);
                let axis = self.c(st, t::AXIS_HEIGHT);
                self.line(
                    &mut out,
                    (cx - half * 0.4, -axis + half),
                    (cx + half * 0.4, -axis - half),
                    th,
                    color,
                )?;
            }
        }
        Ok(out)
    }

    fn boxed(&mut self, b: B, st: St, color: Option<u32>) -> Result<B, MathError> {
        let sep = 0.3 * self.size(st);
        let th = self.c(st, t::FRACTION_RULE_THICKNESS).max(0.5);
        let (w, h, d) = (b.w + 2.0 * sep, b.h + sep, b.d + sep);
        let mut out = B::default();
        out.put(b, sep, 0.0);
        self.count(4)?;
        for (x, y, rw, rh) in [
            (0.0, -h, w, th),
            (0.0, d - th, w, th),
            (0.0, -h, th, h + d),
            (w - th, -h, th, h + d),
        ] {
            out.ops.push(Op::Rule {
                x,
                y,
                w: rw,
                h: rh,
                color,
            });
        }
        out.w = w;
        out.h = out.h.max(h);
        out.d = out.d.max(d);
        Ok(out)
    }
}
