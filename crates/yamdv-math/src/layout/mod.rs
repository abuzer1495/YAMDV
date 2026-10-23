//! TeX-style math layout (§2.3): boxes, the four styles and their cramped variants, the atom
//! spacing table, OpenType `MATH` constants for scripts, limits, fractions, radicals, accents and
//! stretchy delimiters. Output: [`MathBox`] display ops (glyphs in the math font + rules).

use crate::font::MathFont;
use crate::macros::Macros;
use crate::parse::{Env, Font, FracStyle, MathError, Node, styled};
use crate::symbols::Class;
use crate::table::{self as t, MathTable};

mod ext;
mod matrix;

/// Maximum display ops per formula (bounds memory; S-PARSE-05).
pub const MAX_OPS: usize = 50_000;

/// One display op. Coordinates are DIPs relative to the formula origin (left edge, baseline),
/// y grows downwards.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    /// A glyph of the math font with its baseline origin at (x, y).
    Glyph {
        /// Glyph id.
        gid: u16,
        /// Origin x.
        x: f32,
        /// Baseline y.
        y: f32,
        /// Font size (DIPs).
        size: f32,
        /// `\color` override (`None` = theme text color).
        color: Option<u32>,
    },
    /// A filled rectangle (fraction bar, radical overbar, over/underline).
    Rule {
        /// Left.
        x: f32,
        /// Top.
        y: f32,
        /// Width.
        w: f32,
        /// Height.
        h: f32,
        /// Color override.
        color: Option<u32>,
    },
    /// A straight line (`\cancel`, `\not` overlays).
    Line {
        /// Start x.
        x1: f32,
        /// Start y.
        y1: f32,
        /// End x.
        x2: f32,
        /// End y.
        y2: f32,
        /// Stroke width.
        w: f32,
        /// Color override.
        color: Option<u32>,
    },
}

/// A laid-out formula.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MathBox {
    /// Advance width.
    pub width: f32,
    /// Height above the baseline.
    pub ascent: f32,
    /// Depth below the baseline.
    pub descent: f32,
    /// Display ops.
    pub ops: Vec<Op>,
    /// `\tag{…}`: the equation number, laid out on its own; display math draws it
    /// right-aligned on the formula's baseline (R-MATH-02).
    pub tag: Option<Box<MathBox>>,
}

/// Style: 0 display, 1 text, 2 script, 3 scriptscript.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct St {
    level: u8,
    cramped: bool,
}

impl St {
    fn display(self) -> bool {
        self.level == 0
    }
    fn sup(self) -> St {
        St {
            level: if self.level < 2 { 2 } else { 3 },
            cramped: self.cramped,
        }
    }
    fn sub(self) -> St {
        St {
            cramped: true,
            ..self.sup()
        }
    }
    fn num(self) -> St {
        St {
            level: match self.level {
                0 => 1,
                1 => 2,
                _ => 3,
            },
            cramped: self.cramped,
        }
    }
    fn den(self) -> St {
        St {
            cramped: true,
            ..self.num()
        }
    }
    fn cramp(self) -> St {
        St {
            cramped: true,
            ..self
        }
    }
    fn script(self) -> bool {
        self.level >= 2
    }
}

#[derive(Debug, Clone, Default)]
struct B {
    w: f32,
    h: f32,
    d: f32,
    ops: Vec<Op>,
    /// Italic correction of the last glyph.
    ic: f32,
    /// Single glyph (for accents): (gid, scale).
    glyph: Option<(u16, f32)>,
}

impl Op {
    /// Moves the op by (dx, dy).
    pub fn shift(&mut self, dx: f32, dy: f32) {
        match self {
            Op::Glyph { x, y, .. } | Op::Rule { x, y, .. } => {
                *x += dx;
                *y += dy;
            }
            Op::Line { x1, y1, x2, y2, .. } => {
                *x1 += dx;
                *x2 += dx;
                *y1 += dy;
                *y2 += dy;
            }
        }
    }
}

impl MathBox {
    /// Moves every op by (dx, dy) (the box metrics are unchanged).
    pub fn shift(&mut self, dx: f32, dy: f32) {
        for o in &mut self.ops {
            o.shift(dx, dy);
        }
    }
}

impl B {
    fn put(&mut self, c: B, dx: f32, dy: f32) {
        self.h = self.h.max(c.h - dy);
        self.d = self.d.max(c.d + dy);
        for mut o in c.ops {
            o.shift(dx, dy);
            self.ops.push(o);
        }
    }
}

struct L<'a> {
    font: &'a dyn MathFont,
    t: &'a MathTable,
    base: f32,
    ops: usize,
    /// Size of `\middle` delimiters in the `\left … \right` being laid out.
    middle: Option<f32>,
}

const NULL_DELIM: f32 = 0.12;

/// Lays out parsed `nodes` at font size `size` (DIPs) in display or text style.
pub fn layout(
    nodes: &[Node],
    font: &dyn MathFont,
    table: &MathTable,
    size: f32,
    display: bool,
) -> Result<MathBox, MathError> {
    let mut l = L {
        font,
        t: table,
        base: size,
        ops: 0,
        middle: None,
    };
    let st = St {
        level: if display { 0 } else { 1 },
        cramped: false,
    };
    let mut tag = None;
    let mut body: Vec<Node> = Vec::with_capacity(nodes.len());
    for n in nodes {
        match n {
            Node::Tag(s) => tag = Some(s.clone()),
            _ => body.push(n.clone()),
        }
    }
    let b = l.hlist(&body, st, None)?;
    let tag = match tag {
        Some(s) => {
            let t = l.string_box(&s, St { level: 1, ..st }, None)?;
            Some(Box::new(MathBox {
                width: t.w,
                ascent: t.h,
                descent: t.d,
                ops: t.ops,
                tag: None,
            }))
        }
        None => None,
    };
    Ok(MathBox {
        width: b.w,
        ascent: b.h,
        descent: b.d,
        ops: b.ops,
        tag,
    })
}

/// Parses and lays out `src` (convenience for callers and the fuzzer).
pub fn render(
    src: &str,
    font: &dyn MathFont,
    table: &MathTable,
    size: f32,
    display: bool,
) -> Result<MathBox, MathError> {
    render_with(src, &mut Macros::default(), font, table, size, display)
}

/// Expands the document macros `env` (R-MATH-07; definitions in `src` are added to `env`),
/// then parses and lays out `src`. Export and print use the same entry point.
pub fn render_with(
    src: &str,
    env: &mut Macros,
    font: &dyn MathFont,
    table: &MathTable,
    size: f32,
    display: bool,
) -> Result<MathBox, MathError> {
    let nodes = nodes_of(src, env)?;
    layout(&nodes, font, table, size, display)
}

/// Expands and parses `src` without a font: the same errors [`render_with`] reports before
/// layout (driver probe for the error color on machines without a math font).
pub fn check(src: &str, env: &mut Macros) -> Result<(), MathError> {
    nodes_of(src, env).map(|_| ())
}

fn nodes_of(src: &str, env: &mut Macros) -> Result<Vec<crate::parse::Node>, MathError> {
    let expanded = crate::macros::expand(src, env)?;
    if matches!(expanded, std::borrow::Cow::Borrowed(_)) {
        crate::parse::parse(&expanded)
    } else {
        crate::parse::parse_limited(&expanded, crate::parse::MAX_EXPANDED)
    }
}

fn spacing(left: Class, right: Class, script: bool) -> f32 {
    use Class::{Bin, Close, Inner, Op as O, Open, Ord, Punct, Rel};
    let idx = |c: Class| match c {
        Ord => 0,
        O => 1,
        Bin => 2,
        Rel => 3,
        Open => 4,
        Close => 5,
        Punct => 6,
        Inner => 7,
    };
    // 1 = thin always, 2 = thin (not script), 3 = medium (not script), 4 = thick (not script).
    const T: [[u8; 8]; 8] = [
        [0, 1, 3, 4, 0, 0, 0, 2],
        [1, 1, 0, 4, 0, 0, 0, 2],
        [3, 3, 0, 0, 3, 0, 0, 3],
        [4, 4, 0, 0, 4, 0, 0, 4],
        [0, 0, 0, 0, 0, 0, 0, 0],
        [0, 1, 3, 4, 0, 0, 0, 2],
        [2, 2, 0, 2, 2, 2, 2, 2],
        [2, 1, 3, 4, 2, 0, 2, 2],
    ];
    let v = T
        .get(idx(left))
        .and_then(|r| r.get(idx(right)))
        .copied()
        .unwrap_or(0);
    match (v, script) {
        (1, _) => 3.0,
        (2, false) => 3.0,
        (3, false) => 4.0,
        (4, false) => 5.0,
        _ => 0.0,
    }
}

impl L<'_> {
    fn size(&self, st: St) -> f32 {
        match st.level {
            0 | 1 => self.base,
            2 => self.base * self.t.constants.get(t::SCRIPT_PERCENT) / 100.0,
            _ => self.base * self.t.constants.get(t::SCRIPT_SCRIPT_PERCENT) / 100.0,
        }
    }
    /// Font units → DIPs at the style's size.
    fn k(&self, st: St) -> f32 {
        self.size(st) / self.t.upem()
    }
    fn c(&self, st: St, c: t::C) -> f32 {
        self.t.constants.get(c) * self.k(st)
    }

    fn count(&mut self, n: usize) -> Result<(), MathError> {
        self.ops += n;
        if self.ops > MAX_OPS {
            return Err(MathError {
                msg: "formula too large".into(),
                pos: 0,
            });
        }
        Ok(())
    }

    fn glyph_box(&mut self, gid: u16, st: St, color: Option<u32>) -> Result<B, MathError> {
        self.count(1)?;
        let k = self.k(st);
        let m = self.font.metrics(gid);
        Ok(B {
            w: m.advance * k,
            h: m.y_max * k,
            d: -m.y_min * k,
            ops: vec![Op::Glyph {
                gid,
                x: 0.0,
                y: 0.0,
                size: self.size(st),
                color,
            }],
            ic: self.t.italics(gid) * k,
            glyph: Some((gid, k)),
        })
    }

    fn gid(&self, c: char) -> u16 {
        self.font.glyph(c)
    }

    fn char_box(
        &mut self,
        c: char,
        font: Font,
        st: St,
        color: Option<u32>,
    ) -> Result<B, MathError> {
        let m = styled(c, font);
        let mut g = self.gid(m);
        if g == 0 {
            g = self.gid(c);
        }
        let mut b = self.glyph_box(g, st, color)?;
        if c == ' ' {
            b.w = b.w.max(0.25 * self.size(st));
        }
        Ok(b)
    }

    fn string_box(&mut self, s: &str, st: St, color: Option<u32>) -> Result<B, MathError> {
        let mut out = B::default();
        for c in s.chars() {
            let b = self.char_box(c, Font::Roman, st, color)?;
            let x = out.w;
            let w = b.w;
            let ic = b.ic;
            out.put(b, x, 0.0);
            out.w = x + w;
            out.ic = ic;
        }
        Ok(out)
    }

    fn hlist(&mut self, nodes: &[Node], st: St, color: Option<u32>) -> Result<B, MathError> {
        // Flatten `\color` switches into the list.
        let mut flat: Vec<(&Node, Option<u32>)> = Vec::new();
        fn push<'n>(v: &mut Vec<(&'n Node, Option<u32>)>, nodes: &'n [Node], c: Option<u32>) {
            for n in nodes {
                match n {
                    Node::Color { color, body } => push(v, body, Some(*color)),
                    _ => v.push((n, c)),
                }
            }
        }
        push(&mut flat, nodes, color);
        let mut items: Vec<(B, Option<Class>)> = Vec::with_capacity(flat.len());
        for (n, c) in flat {
            let (b, class) = self.node(n, st, c)?;
            items.push((b, class));
        }
        // Binary operator reclassification (TeX rules 5 and 6).
        let mut prev: Option<usize> = None;
        for k in 0..items.len() {
            let Some(cls) = items.get(k).and_then(|i| i.1) else {
                continue;
            };
            if cls == Class::Bin {
                let p = prev.and_then(|p| items.get(p)).and_then(|i| i.1);
                if matches!(
                    p,
                    None | Some(Class::Bin | Class::Op | Class::Rel | Class::Open | Class::Punct)
                ) && let Some(i) = items.get_mut(k)
                {
                    i.1 = Some(Class::Ord);
                }
            }
            if matches!(cls, Class::Rel | Class::Close | Class::Punct)
                && let Some(p) = prev
                && let Some(pi) = items.get_mut(p)
                && pi.1 == Some(Class::Bin)
            {
                pi.1 = Some(Class::Ord);
            }
            prev = Some(k);
        }
        let mu = self.size(st) / 18.0;
        let mut out = B::default();
        let mut last: Option<Class> = None;
        let n_items = items.len();
        for (b, cls) in items {
            if let (Some(l), Some(r)) = (last, cls) {
                out.w += spacing(l, r, st.script()) * mu;
            }
            if cls.is_some() {
                last = cls;
            }
            let x = out.w;
            let w = b.w;
            let ic = b.ic;
            let single = n_items == 1;
            let g = b.glyph;
            out.put(b, x, 0.0);
            out.w = x + w;
            out.ic = ic;
            if single {
                out.glyph = g;
            }
        }
        Ok(out)
    }

    #[allow(clippy::too_many_lines)]
    fn node(
        &mut self,
        n: &Node,
        st: St,
        color: Option<u32>,
    ) -> Result<(B, Option<Class>), MathError> {
        Ok(match n {
            Node::Char { c, class, font } => (self.char_box(*c, *font, st, color)?, Some(*class)),
            Node::Text(s) => (self.string_box(s, st, color)?, Some(Class::Ord)),
            Node::Func { name, .. } => {
                let mut b = self.string_box(name, st, color)?;
                b.ic = 0.0;
                b.glyph = None;
                (b, Some(Class::Op))
            }
            Node::BigOp { c, .. } => (self.big_op(*c, st, color)?, Some(Class::Op)),
            Node::Group(v) => {
                let mut b = self.hlist(v, st, color)?;
                if v.len() != 1 {
                    b.glyph = None;
                }
                (b, Some(Class::Ord))
            }
            Node::Scripts { base, sup, sub } => {
                let limits = match &**base {
                    Node::Limits { on, .. } => *on,
                    Node::Stretch { limits: true, .. } => true,
                    Node::BigOp { limits: true, .. } | Node::Func { limits: true, .. } => {
                        st.display()
                    }
                    _ => false,
                };
                let (bb, cls) = self.node(base, st, color)?;
                let is_char = matches!(**base, Node::Char { .. });
                let b = if limits {
                    self.limits(bb, sup.as_deref(), sub.as_deref(), st, color)?
                } else {
                    self.scripts(bb, is_char, sup.as_deref(), sub.as_deref(), st, color)?
                };
                (b, cls)
            }
            Node::Frac {
                num,
                den,
                style,
                rule,
                delims,
            } => {
                let fst = match style {
                    FracStyle::Auto => st,
                    FracStyle::Display => St { level: 0, ..st },
                    FracStyle::Text => St { level: 1, ..st },
                };
                let b = self.frac(num, den, fst, *rule, color)?;
                match delims {
                    Some((l, r)) => (
                        self.delimited(Some(*l), b, Some(*r), fst, color)?,
                        Some(Class::Inner),
                    ),
                    None => (b, Some(Class::Ord)),
                }
            }
            Node::Sqrt { index, body } => (
                self.sqrt(index.as_deref(), body, st, color)?,
                Some(Class::Ord),
            ),
            Node::LeftRight { left, right, body } => {
                let mut inner = self.hlist(body, st, color)?;
                if body.iter().any(|n| matches!(n, Node::Middle(_))) {
                    // Second pass with `\middle` sized like the outer delimiters.
                    let outer = self.middle;
                    self.middle = Some(self.delim_target(&inner, st));
                    inner = self.hlist(body, st, color)?;
                    self.middle = outer;
                }
                (
                    self.delimited(*left, inner, *right, st, color)?,
                    Some(Class::Inner),
                )
            }
            Node::Big { c, size, class } => {
                let em = [1.2f32, 1.8, 2.4, 3.0]
                    .get(usize::from(size.saturating_sub(1)))
                    .copied()
                    .unwrap_or(1.2);
                let target = em * self.size(st);
                let b = match c {
                    Some(c) => self.delim(*c, target, st, color)?,
                    None => B {
                        w: NULL_DELIM * self.size(st),
                        ..B::default()
                    },
                };
                (b, Some(*class))
            }
            Node::Matrix { env, rows, cols } => {
                let b = self.matrix(*env, rows, cols, st, color)?;
                match env {
                    Env::BraceMatrix => (
                        self.delimited(Some('{'), b, Some('}'), st, color)?,
                        Some(Class::Inner),
                    ),
                    Env::DoubleVMatrix => (
                        self.delimited(Some('‖'), b, Some('‖'), st, color)?,
                        Some(Class::Inner),
                    ),
                    Env::DCases => (
                        self.delimited(Some('{'), b, None, st, color)?,
                        Some(Class::Inner),
                    ),
                    Env::PMatrix => (
                        self.delimited(Some('('), b, Some(')'), st, color)?,
                        Some(Class::Inner),
                    ),
                    Env::BMatrix => (
                        self.delimited(Some('['), b, Some(']'), st, color)?,
                        Some(Class::Inner),
                    ),
                    Env::VMatrix => (
                        self.delimited(Some('|'), b, Some('|'), st, color)?,
                        Some(Class::Inner),
                    ),
                    Env::Cases => (
                        self.delimited(Some('{'), b, None, st, color)?,
                        Some(Class::Inner),
                    ),
                    Env::Matrix | Env::Aligned | Env::Small | Env::Gathered | Env::Array => {
                        (b, Some(Class::Ord))
                    }
                }
            }
            Node::Accent { c, body } => (self.accent(*c, body, st, color)?, Some(Class::Ord)),
            Node::Overline(body) => {
                let b = self.node(body, st.cramp(), color)?.0;
                let (gap, th, extra) = (
                    self.c(st, t::OVERBAR_VERTICAL_GAP),
                    self.c(st, t::OVERBAR_RULE_THICKNESS),
                    self.c(st, t::OVERBAR_EXTRA_ASCENDER),
                );
                let mut out = B::default();
                let top = b.h + gap + th;
                let w = b.w;
                out.put(b, 0.0, 0.0);
                self.count(1)?;
                out.ops.push(Op::Rule {
                    x: 0.0,
                    y: -top,
                    w,
                    h: th,
                    color,
                });
                out.w = w;
                out.h = out.h.max(top + extra);
                (out, Some(Class::Ord))
            }
            Node::Underline(body) => {
                let b = self.node(body, st, color)?.0;
                let (gap, th, extra) = (
                    self.c(st, t::UNDERBAR_VERTICAL_GAP),
                    self.c(st, t::UNDERBAR_RULE_THICKNESS),
                    self.c(st, t::UNDERBAR_EXTRA_DESCENDER),
                );
                let mut out = B::default();
                let top = b.d + gap;
                let w = b.w;
                out.put(b, 0.0, 0.0);
                self.count(1)?;
                out.ops.push(Op::Rule {
                    x: 0.0,
                    y: top,
                    w,
                    h: th,
                    color,
                });
                out.w = w;
                out.d = out.d.max(top + th + extra);
                (out, Some(Class::Ord))
            }
            Node::Space(em) => (
                B {
                    w: em * self.size(st),
                    ..B::default()
                },
                None,
            ),
            Node::Color { color: c, body } => (self.hlist(body, st, Some(*c))?, Some(Class::Ord)),
            other => self.node_ext(other, st, color)?,
        })
    }

    fn big_op(&mut self, c: char, st: St, color: Option<u32>) -> Result<B, MathError> {
        let mut g = self.gid(c);
        if st.display() {
            let min = self.t.constants.get(t::DISPLAY_OPERATOR_MIN_HEIGHT);
            let vars = self.t.variants(g, true);
            if let Some(v) = vars.iter().find(|v| v.advance >= min).or(vars.last()) {
                g = v.glyph;
            }
        }
        let mut b = self.glyph_box(g, st, color)?;
        // Center on the math axis.
        let axis = self.c(st, t::AXIS_HEIGHT);
        let shift = (b.h - b.d) / 2.0 - axis;
        for o in &mut b.ops {
            if let Op::Glyph { y, .. } = o {
                *y += shift;
            }
        }
        b.h -= shift;
        b.d += shift;
        b.glyph = None;
        Ok(b)
    }

    #[allow(clippy::too_many_arguments)]
    fn scripts(
        &mut self,
        base: B,
        is_char: bool,
        sup: Option<&Node>,
        sub: Option<&Node>,
        st: St,
        color: Option<u32>,
    ) -> Result<B, MathError> {
        let c = |l: &Self, k| l.c(st, k);
        let sup_b = match sup {
            Some(n) => Some(self.node(n, st.sup(), color)?.0),
            None => None,
        };
        let sub_b = match sub {
            Some(n) => Some(self.node(n, st.sub(), color)?.0),
            None => None,
        };
        let (bh, bd) = if is_char {
            (0.0, 0.0)
        } else {
            (base.h, base.d)
        };
        let mut sup_shift = 0.0f32;
        let mut sub_shift = 0.0f32;
        if let Some(s) = &sup_b {
            let up = if st.cramped {
                c(self, t::SUPERSCRIPT_SHIFT_UP_CRAMPED)
            } else {
                c(self, t::SUPERSCRIPT_SHIFT_UP)
            };
            sup_shift = up
                .max(bh - c(self, t::SUPERSCRIPT_BASELINE_DROP_MAX))
                .max(s.d + c(self, t::SUPERSCRIPT_BOTTOM_MIN));
        }
        if let Some(s) = &sub_b {
            sub_shift =
                c(self, t::SUBSCRIPT_SHIFT_DOWN).max(bd + c(self, t::SUBSCRIPT_BASELINE_DROP_MIN));
            if sup_b.is_none() {
                sub_shift = sub_shift.max(s.h - c(self, t::SUBSCRIPT_TOP_MAX));
            }
        }
        if let (Some(p), Some(b)) = (&sup_b, &sub_b) {
            let gap = (sup_shift - p.d) - (b.h - sub_shift);
            let min = c(self, t::SUB_SUPERSCRIPT_GAP_MIN);
            if gap < min {
                let mut delta = min - gap;
                let room = c(self, t::SUPERSCRIPT_BOTTOM_MAX_WITH_SUBSCRIPT) - (sup_shift - p.d);
                if room > 0.0 {
                    let up = room.min(delta);
                    sup_shift += up;
                    delta -= up;
                }
                sub_shift += delta;
            }
        }
        let mut out = B::default();
        let bw = base.w;
        let ic = base.ic;
        out.put(base, 0.0, 0.0);
        out.w = bw;
        let mut right = bw;
        if let Some(s) = sup_b {
            let x = bw + ic;
            right = right.max(x + s.w);
            out.put(s, x, -sup_shift);
        }
        if let Some(s) = sub_b {
            right = right.max(bw + s.w);
            out.put(s, bw, sub_shift);
        }
        out.w = right + c(self, t::SPACE_AFTER_SCRIPT);
        Ok(out)
    }

    fn limits(
        &mut self,
        base: B,
        sup: Option<&Node>,
        sub: Option<&Node>,
        st: St,
        color: Option<u32>,
    ) -> Result<B, MathError> {
        let up = match sup {
            Some(n) => Some(self.node(n, st.sup(), color)?.0),
            None => None,
        };
        let lo = match sub {
            Some(n) => Some(self.node(n, st.sub(), color)?.0),
            None => None,
        };
        let w = base
            .w
            .max(up.as_ref().map_or(0.0, |b| b.w))
            .max(lo.as_ref().map_or(0.0, |b| b.w));
        let ic = base.ic;
        let (bh, bd, bw) = (base.h, base.d, base.w);
        let mut out = B::default();
        out.put(base, (w - bw) / 2.0, 0.0);
        if let Some(u) = up {
            let gap = self
                .c(st, t::UPPER_LIMIT_GAP_MIN)
                .max(self.c(st, t::UPPER_LIMIT_BASELINE_RISE_MIN) - u.d);
            let y = -(bh + gap + u.d);
            let x = (w - u.w) / 2.0 + ic / 2.0;
            out.put(u, x, y);
        }
        if let Some(l) = lo {
            let gap = self
                .c(st, t::LOWER_LIMIT_GAP_MIN)
                .max(self.c(st, t::LOWER_LIMIT_BASELINE_DROP_MIN) - l.h);
            let y = bd + gap + l.h;
            let x = (w - l.w) / 2.0 - ic / 2.0;
            out.put(l, x, y);
        }
        out.w = w;
        Ok(out)
    }

    fn frac(
        &mut self,
        num: &Node,
        den: &Node,
        st: St,
        rule: bool,
        color: Option<u32>,
    ) -> Result<B, MathError> {
        let n = self.node(num, st.num(), color)?.0;
        let d = self.node(den, st.den(), color)?.0;
        let c = |l: &Self, k| l.c(st, k);
        let axis = c(self, t::AXIS_HEIGHT);
        let disp = st.display();
        let (mut up, mut down);
        if rule {
            let th = c(self, t::FRACTION_RULE_THICKNESS);
            let (su, sd, gn, gd) = if disp {
                (
                    c(self, t::FRACTION_NUMERATOR_DISPLAY_STYLE_SHIFT_UP),
                    c(self, t::FRACTION_DENOMINATOR_DISPLAY_STYLE_SHIFT_DOWN),
                    c(self, t::FRACTION_NUM_DISPLAY_STYLE_GAP_MIN),
                    c(self, t::FRACTION_DENOM_DISPLAY_STYLE_GAP_MIN),
                )
            } else {
                (
                    c(self, t::FRACTION_NUMERATOR_SHIFT_UP),
                    c(self, t::FRACTION_DENOMINATOR_SHIFT_DOWN),
                    c(self, t::FRACTION_NUMERATOR_GAP_MIN),
                    c(self, t::FRACTION_DENOMINATOR_GAP_MIN),
                )
            };
            up = su.max(axis + th / 2.0 + gn + n.d);
            down = sd.max(d.h + gd - axis + th / 2.0);
        } else {
            let (su, sd, g) = if disp {
                (
                    c(self, t::STACK_TOP_DISPLAY_STYLE_SHIFT_UP),
                    c(self, t::STACK_BOTTOM_DISPLAY_STYLE_SHIFT_DOWN),
                    c(self, t::STACK_DISPLAY_STYLE_GAP_MIN),
                )
            } else {
                (
                    c(self, t::STACK_TOP_SHIFT_UP),
                    c(self, t::STACK_BOTTOM_SHIFT_DOWN),
                    c(self, t::STACK_GAP_MIN),
                )
            };
            up = su;
            down = sd;
            let gap = (up - n.d) - (d.h - down);
            if gap < g {
                up += (g - gap) / 2.0;
                down += (g - gap) / 2.0;
            }
        }
        let pad = NULL_DELIM * self.size(st);
        let inner = n.w.max(d.w);
        let mut out = B::default();
        let (nw, dw) = (n.w, d.w);
        out.put(n, pad + (inner - nw) / 2.0, -up);
        out.put(d, pad + (inner - dw) / 2.0, down);
        if rule {
            let th = c(self, t::FRACTION_RULE_THICKNESS);
            self.count(1)?;
            out.ops.push(Op::Rule {
                x: pad,
                y: -(axis + th / 2.0),
                w: inner,
                h: th,
                color,
            });
            out.h = out.h.max(axis + th / 2.0);
        }
        out.w = inner + 2.0 * pad;
        Ok(out)
    }

    fn sqrt(
        &mut self,
        index: Option<&Node>,
        body: &Node,
        st: St,
        color: Option<u32>,
    ) -> Result<B, MathError> {
        let b = self.node(body, st.cramp(), color)?.0;
        let th = self.c(st, t::RADICAL_RULE_THICKNESS);
        let mut gap = if st.display() {
            self.c(st, t::RADICAL_DISPLAY_STYLE_VERTICAL_GAP)
        } else {
            self.c(st, t::RADICAL_VERTICAL_GAP)
        };
        let extra = self.c(st, t::RADICAL_EXTRA_ASCENDER);
        let target = b.h + b.d + gap + th;
        let mut rad = self.stretch_v('√', target, st, color)?;
        let total = rad.h + rad.d;
        if total > target {
            gap += (total - target) / 2.0;
        }
        // Radical top = body top + gap + rule thickness.
        let top = b.h + gap + th;
        let dy = rad.h - top;
        for o in &mut rad.ops {
            if let Op::Glyph { y, .. } = o {
                *y += dy;
            }
        }
        rad.h = top;
        rad.d = total - top;
        let mut out = B::default();
        let mut x0 = 0.0f32;
        if let Some(ix) = index {
            let sst = St {
                level: 3,
                cramped: true,
            };
            let deg = self.node(ix, sst, color)?.0;
            let before = self.c(st, t::RADICAL_KERN_BEFORE_DEGREE);
            let after = self.c(st, t::RADICAL_KERN_AFTER_DEGREE);
            let raise =
                self.t.constants.get(t::RADICAL_DEGREE_BOTTOM_RAISE_PERCENT) / 100.0 * total;
            let base_y = rad.d - raise;
            let dw = deg.w;
            let dd = deg.d;
            out.put(deg, before, base_y - dd);
            x0 = (before + dw + after).max(0.0);
        }
        let rw = rad.w;
        let bw = b.w;
        out.put(rad, x0, 0.0);
        out.put(b, x0 + rw, 0.0);
        self.count(1)?;
        out.ops.push(Op::Rule {
            x: x0 + rw,
            y: -top,
            w: bw,
            h: th,
            color,
        });
        out.h = out.h.max(top + extra);
        out.w = x0 + rw + bw;
        Ok(out)
    }

    /// A vertically stretched glyph at least `target` tall (variants, then assembly), baseline
    /// unchanged (glyph metrics as designed).
    fn stretch_v(
        &mut self,
        c: char,
        target: f32,
        st: St,
        color: Option<u32>,
    ) -> Result<B, MathError> {
        let g = self.gid(c);
        let k = self.k(st);
        let need = target / k;
        let vars = self.t.variants(g, true);
        if let Some(v) = vars.iter().find(|v| v.advance >= need) {
            return self.glyph_box(v.glyph, st, color);
        }
        let parts = self.t.assembly(g, true);
        if parts.is_empty() {
            let last = vars.last().map_or(g, |v| v.glyph);
            return self.glyph_box(last, st, color);
        }
        let (placed, len) = t::assemble(&parts, self.t.min_overlap(), need);
        self.count(placed.len())?;
        let mut out = B::default();
        let mut w = 0.0f32;
        // Bottom of the assembly sits at the original glyph's bottom.
        let m0 = self.font.metrics(g);
        let bottom = m0.y_min;
        for (pg, off) in placed {
            let m = self.font.metrics(pg);
            // Part ink bottom at `bottom + off` (font units, y up).
            let y_up = bottom + off - m.y_min;
            out.ops.push(Op::Glyph {
                gid: pg,
                x: 0.0,
                y: -y_up * k,
                size: self.size(st),
                color,
            });
            w = w.max(m.advance * k);
        }
        out.w = w;
        out.d = -bottom * k;
        out.h = (bottom + len) * k;
        Ok(out)
    }

    /// Delimiter of total size ≥ `target`, centered on the axis.
    fn delim(&mut self, c: char, target: f32, st: St, color: Option<u32>) -> Result<B, MathError> {
        let mut b = self.stretch_v(c, target, st, color)?;
        let axis = self.c(st, t::AXIS_HEIGHT);
        let shift = (b.h - b.d) / 2.0 - axis;
        for o in &mut b.ops {
            if let Op::Glyph { y, .. } = o {
                *y += shift;
            }
        }
        b.h -= shift;
        b.d += shift;
        b.glyph = None;
        b.ic = 0.0;
        Ok(b)
    }

    fn delimited(
        &mut self,
        l: Option<char>,
        inner: B,
        r: Option<char>,
        st: St,
        color: Option<u32>,
    ) -> Result<B, MathError> {
        let target = self.delim_target(&inner, st);
        let em = self.size(st);
        let mut out = B::default();
        let mut x = 0.0f32;
        let side =
            |l: &mut Self, out: &mut B, x: &mut f32, c: Option<char>| -> Result<(), MathError> {
                match c {
                    Some(c) => {
                        let b = l.delim(c, target, st, color)?;
                        let w = b.w;
                        out.put(b, *x, 0.0);
                        *x += w;
                    }
                    None => *x += NULL_DELIM * em,
                }
                Ok(())
            };
        side(self, &mut out, &mut x, l)?;
        let iw = inner.w;
        out.put(inner, x, 0.0);
        x += iw;
        side(self, &mut out, &mut x, r)?;
        out.w = x;
        Ok(out)
    }

    /// TeX's delimiter size for content `inner` (`\left`, `\middle`, `\right`).
    fn delim_target(&self, inner: &B, st: St) -> f32 {
        let axis = self.c(st, t::AXIS_HEIGHT);
        let delta = (inner.h - axis).max(inner.d + axis);
        let em = self.size(st);
        (2.0 * delta * 0.901).max(2.0 * delta - 0.5 * em)
    }

    fn accent(&mut self, c: char, body: &Node, st: St, color: Option<u32>) -> Result<B, MathError> {
        let b = self.node(body, st.cramp(), color)?.0;
        let base_attach = match b.glyph {
            Some((g, k)) => self.t.top_accent(g).map_or(b.w / 2.0, |a| a * k),
            None => b.w / 2.0,
        };
        let ag = self.gid(c);
        let acc = self.glyph_box(ag, st, color)?;
        let k = self.k(st);
        let m = self.font.metrics(ag);
        let acc_attach = self
            .t
            .top_accent(ag)
            .map_or((m.x_min + m.x_max) / 2.0 * k, |a| a * k);
        let lift = (b.h - self.c(st, t::ACCENT_BASE_HEIGHT)).max(0.0);
        let mut out = B::default();
        let (w, ic, g) = (b.w, b.ic, b.glyph);
        out.put(b, 0.0, 0.0);
        out.put(acc, base_attach - acc_attach, -lift);
        out.w = w;
        out.ic = ic;
        out.glyph = g;
        Ok(out)
    }
}
