//! Display primitives of a laid-out diagram (logical px, origin top-left).
// Text sizes and point counts are small, positive and bounded.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

/// A theme color role (B.3.6) resolved by the renderer for light/dark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// Node fill.
    NodeFill,
    /// Node border.
    NodeStroke,
    /// Text.
    Text,
    /// Edge / message line and arrowheads.
    Edge,
    /// Background under edge labels.
    LabelBg,
    /// Subgraph fill.
    ClusterFill,
    /// Subgraph border.
    ClusterStroke,
    /// Note fill.
    NoteFill,
    /// Note border.
    NoteStroke,
    /// Lifeline.
    Lifeline,
    /// Activation bar fill.
    Activation,
    /// Frame (loop/alt/opt/par) border.
    Frame,
}

/// A color: a theme role or an explicit `style`/`classDef` color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ink {
    /// Theme role.
    Role(Role),
    /// Explicit `0xRRGGBB`.
    Rgb(u32),
}

/// Path segment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Seg {
    /// Move to.
    M(f32, f32),
    /// Line to.
    L(f32, f32),
    /// Cubic Bézier to (c1, c2, end).
    C(f32, f32, f32, f32, f32, f32),
    /// Close.
    Z,
}

/// A filled and/or stroked vector path.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    /// Segments.
    pub segs: Vec<Seg>,
    /// Fill.
    pub fill: Option<Ink>,
    /// Stroke.
    pub stroke: Option<Ink>,
    /// Stroke width.
    pub width: f32,
    /// Dashed stroke.
    pub dash: bool,
}

/// A measured text (see [`Measure::text`]) with its top-left at (x, y).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextPrim {
    /// Id returned by [`Measure::text`].
    pub id: u32,
    /// Left.
    pub x: f32,
    /// Top.
    pub y: f32,
    /// Color.
    pub color: Ink,
}

/// One primitive.
#[derive(Debug, Clone, PartialEq)]
pub enum Prim {
    /// Vector path.
    Path(Path),
    /// Text.
    Text(TextPrim),
}

/// A laid-out diagram.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Drawing {
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
    /// Primitives in paint order.
    pub prims: Vec<Prim>,
    /// Node boxes (flowchart nodes / sequence participant heads), for tests and hit checks.
    pub boxes: Vec<(f32, f32, f32, f32)>,
    /// Edge polylines before smoothing (flowchart), for tests.
    pub edges: Vec<Vec<(f32, f32)>>,
    /// The layout gave up (too many virtual nodes): show "Content too complex" (S-PARSE-03).
    pub complex: bool,
}

/// Text measurement (the renderer shapes with its `TextShaper`).
pub trait Measure {
    /// Lays out `s` (body family, `size` px, wrapped at `max_w`) and returns an id for drawing
    /// plus its width and height.
    fn text(&mut self, s: &str, size: f32, max_w: f32, bold: bool) -> (u32, f32, f32);
}

/// A measurer for tests and fuzzing: 0.6 em per char, 1.4 em lines, wraps at `max_w`.
#[derive(Debug, Default)]
pub struct MockMeasure {
    /// Texts measured so far.
    pub texts: Vec<String>,
}

impl Measure for MockMeasure {
    fn text(&mut self, s: &str, size: f32, max_w: f32, _bold: bool) -> (u32, f32, f32) {
        let cw = size * 0.6;
        let per = ((max_w / cw).floor() as usize).max(1);
        let (mut w, mut lines) = (0.0f32, 0usize);
        for l in s.split('\n') {
            let n = l.chars().count();
            let rows = n.div_ceil(per).max(1);
            lines += rows;
            w = w.max(n.min(per) as f32 * cw);
        }
        self.texts.push(s.to_string());
        let id = u32::try_from(self.texts.len() - 1).unwrap_or(u32::MAX);
        (id, w, lines as f32 * size * 1.4)
    }
}

const K: f32 = 0.552_284_8;

/// Rounded rectangle path (radius clamped to half the shorter side).
pub(crate) fn rrect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Vec<Seg> {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    if r <= 0.0 {
        return vec![
            Seg::M(x, y),
            Seg::L(x + w, y),
            Seg::L(x + w, y + h),
            Seg::L(x, y + h),
            Seg::Z,
        ];
    }
    let k = r * K;
    let (x1, y1) = (x + w, y + h);
    vec![
        Seg::M(x + r, y),
        Seg::L(x1 - r, y),
        Seg::C(x1 - r + k, y, x1, y + r - k, x1, y + r),
        Seg::L(x1, y1 - r),
        Seg::C(x1, y1 - r + k, x1 - r + k, y1, x1 - r, y1),
        Seg::L(x + r, y1),
        Seg::C(x + r - k, y1, x, y1 - r + k, x, y1 - r),
        Seg::L(x, y + r),
        Seg::C(x, y + r - k, x + r - k, y, x + r, y),
        Seg::Z,
    ]
}

/// Ellipse path.
pub(crate) fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Vec<Seg> {
    let (kx, ky) = (rx * K, ry * K);
    vec![
        Seg::M(cx + rx, cy),
        Seg::C(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry),
        Seg::C(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy),
        Seg::C(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry),
        Seg::C(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy),
        Seg::Z,
    ]
}

/// Polyline path.
pub(crate) fn poly(pts: &[(f32, f32)], close: bool) -> Vec<Seg> {
    let mut v: Vec<Seg> = pts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            if i == 0 {
                Seg::M(p.0, p.1)
            } else {
                Seg::L(p.0, p.1)
            }
        })
        .collect();
    if close {
        v.push(Seg::Z);
    }
    v
}

/// Smooth curve through `pts` (Catmull-Rom converted to cubic Béziers).
pub(crate) fn smooth(pts: &[(f32, f32)]) -> Vec<Seg> {
    let n = pts.len();
    let mut v = Vec::with_capacity(n + 1);
    let Some(&(x0, y0)) = pts.first() else {
        return v;
    };
    v.push(Seg::M(x0, y0));
    if n == 2 {
        if let Some(&(x, y)) = pts.get(1) {
            v.push(Seg::L(x, y));
        }
        return v;
    }
    let at = |i: isize| -> (f32, f32) {
        let i = i.clamp(0, n as isize - 1) as usize;
        pts.get(i).copied().unwrap_or((x0, y0))
    };
    for i in 0..n.saturating_sub(1) {
        let i = i as isize;
        let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        let c1 = (p1.0 + (p2.0 - p0.0) / 6.0, p1.1 + (p2.1 - p0.1) / 6.0);
        let c2 = (p2.0 - (p3.0 - p1.0) / 6.0, p2.1 - (p3.1 - p1.1) / 6.0);
        v.push(Seg::C(c1.0, c1.1, c2.0, c2.1, p2.0, p2.1));
    }
    v
}

/// Arrowhead variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Head {
    /// Filled triangle.
    Arrow,
    /// Two open strokes (sequence "async" `)`).
    Open,
    /// A cross.
    Cross,
}

/// Arrowhead at `tip` pointing along `from → tip`.
pub(crate) fn head(out: &mut Vec<Prim>, from: (f32, f32), tip: (f32, f32), kind: Head, s: f32) {
    let (dx, dy) = (tip.0 - from.0, tip.1 - from.1);
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-3 {
        return;
    }
    let (ux, uy) = (dx / len, dy / len);
    let (px, py) = (-uy, ux);
    let (l, w) = (9.0 * s, 4.5 * s);
    let base = (tip.0 - ux * l, tip.1 - uy * l);
    let a = (base.0 + px * w, base.1 + py * w);
    let b = (base.0 - px * w, base.1 - py * w);
    let ink = Some(Ink::Role(Role::Edge));
    match kind {
        Head::Arrow => out.push(Prim::Path(Path {
            segs: poly(&[tip, a, b], true),
            fill: ink,
            stroke: None,
            width: 0.0,
            dash: false,
        })),
        Head::Open => out.push(Prim::Path(Path {
            segs: poly(&[a, tip, b], false),
            fill: None,
            stroke: ink,
            width: 1.5 * s,
            dash: false,
        })),
        Head::Cross => {
            let c = (tip.0 - ux * 5.0 * s, tip.1 - uy * 5.0 * s);
            let d = 4.0 * s;
            let mut segs = poly(
                &[
                    (c.0 - d * (ux + px), c.1 - d * (uy + py)),
                    (c.0 + d * (ux + px), c.1 + d * (uy + py)),
                ],
                false,
            );
            segs.extend(poly(
                &[
                    (c.0 - d * (ux - px), c.1 - d * (uy - py)),
                    (c.0 + d * (ux - px), c.1 + d * (uy - py)),
                ],
                false,
            ));
            out.push(Prim::Path(Path {
                segs,
                fill: None,
                stroke: ink,
                width: 1.5 * s,
                dash: false,
            }));
        }
    }
}
