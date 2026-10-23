//! `pie` (FUT-18 MUST): title, showData, `"label" : value`, legend, B.3.6 series palette.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use crate::draw::{self, Drawing, Ink, Measure, Path, Prim, Role, Seg, TextPrim};
use crate::{Error, err, lines};

/// A parsed pie chart.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pie {
    /// Title.
    pub title: Option<String>,
    /// `showData`: values in the legend.
    pub show_data: bool,
    /// Slices (label, value > 0) in source order.
    pub slices: Vec<(String, f64)>,
}

/// Parses a `pie` block.
pub fn parse(src: &str) -> Result<Pie, Error> {
    let mut p = Pie::default();
    let mut it = lines(src);
    if let Some((_, first)) = it.next() {
        let rest = first.trim().strip_prefix("pie").unwrap_or("").trim();
        let rest = if let Some(r) = rest.strip_prefix("showData") {
            p.show_data = true;
            r.trim()
        } else {
            rest
        };
        if let Some(t) = rest.strip_prefix("title") {
            p.title = Some(t.trim().to_string());
        }
    }
    for (ln, line) in it {
        let l = line.trim();
        if l == "showData" {
            p.show_data = true;
        } else if let Some(t) = l.strip_prefix("title ") {
            p.title = Some(t.trim().to_string());
        } else if l.starts_with("accTitle") || l.starts_with("accDescr") {
        } else if let Some((k, v)) = l.rsplit_once(':') {
            let k = k.trim();
            let label = k
                .strip_prefix('"')
                .and_then(|x| x.strip_suffix('"'))
                .ok_or_else(|| err("Pie labels must be quoted", ln, 0))?;
            let v: f64 = v
                .trim()
                .parse()
                .ok()
                .filter(|v: &f64| v.is_finite() && *v >= 0.0)
                .ok_or_else(|| err(format!("Invalid pie value \"{}\"", v.trim()), ln, 0))?;
            if p.slices.len() >= crate::MAX_NODES {
                return Err(err("Too many slices", ln, 0));
            }
            p.slices.push((label.to_string(), v));
        } else {
            return Err(err(format!("Unexpected pie line \"{l}\""), ln, 0));
        }
    }
    Ok(p)
}

/// Whole-number percentages of each slice (Mermaid's `toFixed(0)`); 0 for an empty total.
pub fn percentages(p: &Pie) -> Vec<u32> {
    let total: f64 = p.slices.iter().map(|s| s.1).sum();
    p.slices
        .iter()
        .map(|s| {
            if total > 0.0 {
                (s.1 / total * 100.0).round() as u32
            } else {
                0
            }
        })
        .collect()
}

/// A sector path from angle `a0` to `a1` (radians, clockwise from 12 o'clock).
fn sector(cx: f32, cy: f32, r: f32, a0: f32, a1: f32) -> Vec<Seg> {
    let pt = |a: f32| (cx + r * a.sin(), cy - r * a.cos());
    let mut segs = vec![Seg::M(cx, cy)];
    let (x, y) = pt(a0);
    segs.push(Seg::L(x, y));
    let n = ((a1 - a0) / std::f32::consts::FRAC_PI_2).ceil().max(1.0) as usize;
    let step = (a1 - a0) / n as f32;
    let k = 4.0 / 3.0 * (step / 4.0).tan() * r;
    for i in 0..n {
        let (b0, b1) = (a0 + step * i as f32, a0 + step * (i + 1) as f32);
        let (p0, p1) = (pt(b0), pt(b1));
        // Tangent directions (clockwise).
        let (t0, t1) = ((b0.cos(), b0.sin()), (b1.cos(), b1.sin()));
        segs.push(Seg::C(
            p0.0 + k * t0.0,
            p0.1 + k * t0.1,
            p1.0 - k * t1.0,
            p1.1 - k * t1.1,
            p1.0,
            p1.1,
        ));
    }
    segs.push(Seg::Z);
    segs
}

/// Lays the chart out at scale `s`.
pub fn layout(p: &Pie, m: &mut dyn Measure, s: f32) -> Drawing {
    let mut prims = Vec::new();
    let text = Ink::Role(Role::Text);
    let mut top = 8.0 * s;
    let r = 120.0 * s;
    let legend_x = 2.0 * r + 48.0 * s;
    let legend: Vec<(u32, f32, f32)> = p
        .slices
        .iter()
        .map(|(l, v)| {
            let t = if p.show_data {
                format!("{l} [{v}]")
            } else {
                l.clone()
            };
            m.text(&t, 13.0 * s, 260.0 * s, false)
        })
        .collect();
    let lw = legend.iter().map(|t| t.1).fold(0.0, f32::max);
    let w = legend_x + 22.0 * s + lw + 8.0 * s;
    if let Some(t) = &p.title {
        let tt = m.text(t, 16.0 * s, w, true);
        prims.push(Prim::Text(TextPrim {
            id: tt.0,
            x: (w - tt.1) / 2.0,
            y: top,
            color: text,
        }));
        top += tt.2 + 12.0 * s;
    }
    let (cx, cy) = (r + 16.0 * s, top + r);
    let total: f64 = p.slices.iter().map(|x| x.1).sum();
    let pct = percentages(p);
    let tau = std::f32::consts::TAU;
    let mut a = 0.0f32;
    let mut boxes = Vec::new();
    for (i, (_, v)) in p.slices.iter().enumerate() {
        if total <= 0.0 || *v <= 0.0 {
            continue;
        }
        let da = (*v / total) as f32 * tau;
        let ink = Ink::Role(Role::Series((i % 10) as u8));
        let segs = if da >= tau - 1e-4 {
            draw::ellipse(cx, cy, r, r)
        } else {
            sector(cx, cy, r, a, a + da)
        };
        prims.push(Prim::Path(Path {
            segs,
            fill: Some(ink),
            stroke: Some(Ink::Role(Role::LabelBg)),
            width: 1.5 * s,
            dash: false,
        }));
        let pc = pct.get(i).copied().unwrap_or(0);
        if pc >= 3 {
            let mid = a + da / 2.0;
            let t = m.text(&format!("{pc}%"), 12.0 * s, 60.0 * s, false);
            let (lx, ly) = (cx + 0.7 * r * mid.sin(), cy - 0.7 * r * mid.cos());
            prims.push(Prim::Text(TextPrim {
                id: t.0,
                x: lx - t.1 / 2.0,
                y: ly - t.2 / 2.0,
                color: Ink::Rgb(0x1F2937),
            }));
        }
        a += da;
    }
    boxes.push((cx - r, cy - r, 2.0 * r, 2.0 * r));
    let n = legend.len() as f32;
    let pitch = 22.0 * s;
    let mut ly = (cy - n * pitch / 2.0).max(top);
    for (i, t) in legend.iter().enumerate() {
        prims.push(Prim::Path(Path {
            segs: draw::rrect(legend_x, ly + 3.0 * s, 14.0 * s, 14.0 * s, 2.0 * s),
            fill: Some(Ink::Role(Role::Series((i % 10) as u8))),
            stroke: None,
            width: 0.0,
            dash: false,
        }));
        prims.push(Prim::Text(TextPrim {
            id: t.0,
            x: legend_x + 22.0 * s,
            y: ly + (20.0 * s - t.2) / 2.0,
            color: text,
        }));
        ly += pitch;
    }
    Drawing {
        w,
        h: (cy + r).max(ly) + 12.0 * s,
        prims,
        boxes,
        edges: Vec::new(),
        complex: false,
    }
}
