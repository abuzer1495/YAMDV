//! `mindmap` (FUT-18 SHOULD): indentation tree, node shapes, laid out with the V2-D tidy
//! mind-map layout (`yamdv_ui::mindmap`).
#![allow(clippy::cast_possible_truncation)]

use yamdv_core::doc::mind::MindItem;
use yamdv_ui::mindmap::{self as mm, Map};

use crate::draw::{self, Drawing, Ink, Measure, Path, Prim, Role, Seg, TextPrim};
use crate::{Error, err, lines};

/// Node outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MShape {
    /// No brackets.
    Default,
    /// `[x]`.
    Square,
    /// `(x)`.
    Round,
    /// `((x))`.
    Circle,
    /// `{{x}}`.
    Hexagon,
    /// `))x((` / `)x(`.
    Cloud,
}

/// A parsed mind map: root first; `items` are the descendants with nesting levels.
#[derive(Debug, Clone, PartialEq)]
pub struct Mind {
    /// Root label and shape.
    pub root: (String, MShape),
    /// Items (level ≥ 1) and shapes.
    pub items: Vec<(u8, String, MShape)>,
}

fn node_text(s: &str) -> (String, MShape) {
    let s = s.split(":::").next().unwrap_or(s).trim();
    const PAIRS: [(&str, &str, MShape); 6] = [
        ("((", "))", MShape::Circle),
        ("{{", "}}", MShape::Hexagon),
        ("))", "((", MShape::Cloud),
        ("(", ")", MShape::Round),
        (")", "(", MShape::Cloud),
        ("[", "]", MShape::Square),
    ];
    for (o, c, sh) in PAIRS {
        if let Some(i) = s.find(o)
            && let Some(inner) = s.get(i + o.len()..).and_then(|r| r.strip_suffix(c))
        {
            return (crate::label(inner), sh);
        }
    }
    (crate::label(s), MShape::Default)
}

/// Parses a `mindmap` block.
pub fn parse(src: &str) -> Result<Mind, Error> {
    let mut root: Option<(String, MShape)> = None;
    let mut items = Vec::new();
    let mut indents: Vec<usize> = Vec::new();
    for (ln, line) in lines(src).skip(1) {
        let t = line.trim();
        if t.starts_with("::icon(") {
            continue;
        }
        let ind = line.len() - line.trim_start().len();
        if root.is_none() {
            root = Some(node_text(t));
            indents.push(ind);
            continue;
        }
        while indents.last().is_some_and(|&i| i >= ind) && indents.len() > 1 {
            indents.pop();
        }
        if indents.last().is_some_and(|&i| i >= ind) {
            return Err(err("A mindmap has exactly one root", ln, 0));
        }
        indents.push(ind);
        if items.len() >= crate::MAX_NODES {
            return Err(err("Too many nodes", ln, 0));
        }
        let (text, sh) = node_text(t);
        let level = u8::try_from(indents.len() - 1).unwrap_or(u8::MAX).min(250);
        items.push((level, text, sh));
    }
    let root = root.ok_or_else(|| err("Empty mindmap", 0, 0))?;
    Ok(Mind { root, items })
}

fn outline(sh: MShape, x: f32, y: f32, w: f32, h: f32, s: f32) -> Vec<Seg> {
    match sh {
        MShape::Square => draw::rrect(x, y, w, h, 0.0),
        MShape::Round | MShape::Default => draw::rrect(x, y, w, h, 6.0 * s),
        MShape::Cloud => draw::rrect(x, y, w, h, h / 2.0),
        MShape::Circle => draw::ellipse(x + w / 2.0, y + h / 2.0, w / 2.0, h / 2.0),
        MShape::Hexagon => {
            let d = h / 2.0;
            draw::poly(
                &[
                    (x + d, y),
                    (x + w - d, y),
                    (x + w, y + d),
                    (x + w - d, y + h),
                    (x + d, y + h),
                    (x, y + d),
                ],
                true,
            )
        }
    }
}

/// Lays the map out at scale `s` (balanced, both sides of the root).
pub fn layout(d: &Mind, m: &mut dyn Measure, s: f32) -> Drawing {
    let mi: Vec<MindItem> = d
        .items
        .iter()
        .map(|(l, t, _)| MindItem {
            level: *l,
            text: t.clone(),
            offset: 0,
            leaf: false,
        })
        .collect();
    let mut map = Map::build(&d.root.0, None, &mi);
    for n in &mut map.nodes {
        n.collapsed = false;
    }
    let shapes: Vec<MShape> = std::iter::once(d.root.1)
        .chain(d.items.iter().map(|x| x.2))
        .collect();
    let texts: Vec<(u32, f32, f32)> = map
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            m.text(
                &n.text,
                if i == 0 { 16.0 } else { 13.0 } * s,
                220.0 * s,
                i == 0,
            )
        })
        .collect();
    let pad = 12.0 * s;
    let h = texts
        .iter()
        .skip(1)
        .map(|t| t.2)
        .fold(13.0 * 1.4 * s, f32::max)
        + pad;
    let widths: Vec<f32> = texts.iter().map(|t| t.1 + 2.0 * pad).collect();
    let r = texts.first().map_or((40.0, 30.0), |t| {
        if d.root.1 == MShape::Circle {
            let dd = t.1.max(t.2) + 3.0 * pad;
            (dd, dd)
        } else {
            (t.1 + 3.0 * pad, t.2 + 1.5 * pad)
        }
    });
    let placed = mm::layout(&map, &widths, h, r, true);
    let mut prims = Vec::new();
    for (i, n) in map.nodes.iter().enumerate() {
        let (Some(Some(pr)), Some(p)) = (placed.rects.get(i), n.parent) else {
            continue;
        };
        if let Some(Some(pp)) = placed.rects.get(p) {
            let c = mm::connector(*pp, *pr);
            let [a, b, cc, e] = c;
            prims.push(Prim::Path(Path {
                segs: vec![Seg::M(a.0, a.1), Seg::C(b.0, b.1, cc.0, cc.1, e.0, e.1)],
                fill: None,
                stroke: Some(Ink::Role(Role::Series(n.branch))),
                width: 2.0 * s,
                dash: false,
            }));
        }
    }
    let mut boxes = Vec::new();
    for (i, n) in map.nodes.iter().enumerate() {
        let Some(Some(r)) = placed.rects.get(i) else {
            continue;
        };
        let sh = shapes.get(i).copied().unwrap_or(MShape::Default);
        let fill = if i == 0 {
            Role::NodeFill
        } else {
            Role::Series(n.branch)
        };
        prims.push(Prim::Path(Path {
            segs: outline(sh, r.x, r.y, r.w, r.h, s),
            fill: Some(Ink::Role(fill)),
            stroke: (i == 0).then_some(Ink::Role(Role::NodeStroke)),
            width: s,
            dash: false,
        }));
        boxes.push((r.x, r.y, r.w, r.h));
        if let Some(t) = texts.get(i) {
            prims.push(Prim::Text(TextPrim {
                id: t.0,
                x: r.x + (r.w - t.1) / 2.0,
                y: r.y + (r.h - t.2) / 2.0,
                color: if i == 0 {
                    Ink::Role(Role::Text)
                } else {
                    Ink::Rgb(0x1F2937)
                },
            }));
        }
    }
    Drawing {
        w: placed.w,
        h: placed.h,
        prims,
        boxes,
        edges: Vec::new(),
        complex: false,
    }
}
