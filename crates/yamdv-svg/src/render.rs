//! Tree walk and drawing onto a `tiny_skia::Pixmap` at display size.

use std::collections::HashMap;

use tiny_skia::{
    FillRule, GradientStop, LineCap, LineJoin, LinearGradient, Paint as SkPaint, PathBuilder,
    Pixmap, Point, RadialGradient, Rect, Shader, SpreadMode, Stroke, StrokeDash, Transform,
};

use crate::Error;
use crate::style::{self, Paint, Rgba};
use crate::xml::{self, Doc, El, Kid};

/// `use` nesting cap (S-RES-07).
pub const MAX_USE_DEPTH: usize = 8;
/// Resolved `use` instances cap (S-RES-07).
pub const MAX_INSTANCES: usize = 10_000;
/// Path command cap for the whole document (S-RES-07).
pub const MAX_PATH_COMMANDS: usize = 1_000_000;
/// Drawn shapes cap (bounds the work `use` fan-out can cause).
pub const MAX_DRAWS: usize = 200_000;
/// Output pixel cap (S-RES-04 `MAX_PIXELS`).
pub const MAX_PIXELS: u64 = 100_000_000;

/// Horizontal text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Anchor {
    /// `start`.
    #[default]
    Start,
    /// `middle`.
    Middle,
    /// `end`.
    End,
}

/// A single-line text run in output pixels; the platform draws it (system fonts, S-RES-06).
#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    /// Pen x (device px); ignored when `continues`.
    pub x: f32,
    /// Baseline y (device px).
    pub y: f32,
    /// Font size (device px).
    pub size: f32,
    /// Straight RGBA.
    pub color: Rgba,
    /// Alignment of the chunk this run starts.
    pub anchor: Anchor,
    /// Bold.
    pub bold: bool,
    /// Italic.
    pub italic: bool,
    /// First family of `font-family` (generic names mapped by the platform).
    pub family: String,
    /// Text (whitespace collapsed).
    pub text: String,
    /// Continues the previous run's pen position (a `tspan` without `x`).
    pub continues: bool,
}

/// Rendered SVG: premultiplied RGBA pixels plus the text runs drawn on top.
#[derive(Debug, Clone)]
pub struct Image {
    /// Width (px).
    pub w: u32,
    /// Height (px).
    pub h: u32,
    /// `w * h * 4` premultiplied RGBA bytes.
    pub rgba: Vec<u8>,
    /// Text runs.
    pub texts: Vec<TextRun>,
}

#[derive(Debug, Clone)]
struct State {
    fill: Paint,
    fill_op: f32,
    fill_rule: FillRule,
    stroke: Paint,
    stroke_op: f32,
    sw: f32,
    cap: LineCap,
    join: LineJoin,
    miter: f32,
    dash: Vec<f32>,
    dash_off: f32,
    color: Rgba,
    opacity: f32,
    font: f32,
    family: String,
    bold: bool,
    italic: bool,
    anchor: Anchor,
    visible: bool,
}

impl State {
    fn new(color: Rgba) -> Self {
        Self {
            fill: Paint::Color([0, 0, 0, 255]),
            fill_op: 1.0,
            fill_rule: FillRule::Winding,
            stroke: Paint::None,
            stroke_op: 1.0,
            sw: 1.0,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter: 4.0,
            dash: Vec::new(),
            dash_off: 0.0,
            color,
            opacity: 1.0,
            font: 16.0,
            family: String::new(),
            bold: false,
            italic: false,
            anchor: Anchor::Start,
            visible: true,
        }
    }
}

/// Presentation properties of `el`: attributes, then the `style` attribute (wins). Event
/// attributes (`on*`) are never read.
fn props(el: &El) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = el
        .attrs
        .iter()
        .filter(|(k, _)| !k.starts_with("on") && k != "style")
        .cloned()
        .collect();
    if let Some(st) = el.attr("style") {
        for decl in st.split(';') {
            if let Some((k, v)) = decl.split_once(':') {
                out.push((k.trim().to_string(), v.trim().to_string()));
            }
        }
    }
    out
}

fn unit(v: &str) -> Option<f32> {
    let v = v.trim();
    match v.strip_suffix('%') {
        Some(p) => p.trim().parse::<f32>().ok().map(|x| x / 100.0),
        None => v.parse::<f32>().ok(),
    }
    .filter(|x| x.is_finite())
}

struct Ctx<'a> {
    doc: &'a Doc,
    ids: HashMap<&'a str, usize>,
    pix: Pixmap,
    texts: Vec<TextRun>,
    path_budget: usize,
    instances: usize,
    draws: usize,
    /// Viewport (user units) for `%` lengths.
    vw: f32,
    vh: f32,
}

impl Ctx<'_> {
    fn diag(&self) -> f32 {
        (self.vw.hypot(self.vh)) / std::f32::consts::SQRT_2
    }
    fn len(&self, el: &El, k: &str, pct: f32, font: f32) -> f32 {
        el.attr(k)
            .and_then(|v| style::length(v, pct, font))
            .unwrap_or(0.0)
    }

    /// Applies `el`'s properties to `st`; false when `display: none`.
    fn apply(&self, el: &El, st: &mut State) -> bool {
        let mut shown = true;
        let parent_font = st.font;
        for (k, v) in props(el) {
            let v = v.as_str();
            if v == "inherit" {
                continue;
            }
            match k.as_str() {
                "fill" => {
                    if let Some(p) = style::paint(v) {
                        st.fill = p;
                    }
                }
                "stroke" => {
                    if let Some(p) = style::paint(v) {
                        st.stroke = p;
                    }
                }
                "fill-opacity" => st.fill_op = unit(v).unwrap_or(1.0).clamp(0.0, 1.0),
                "stroke-opacity" => st.stroke_op = unit(v).unwrap_or(1.0).clamp(0.0, 1.0),
                "opacity" => st.opacity *= unit(v).unwrap_or(1.0).clamp(0.0, 1.0),
                "stroke-width" => {
                    if let Some(w) = style::length(v, self.diag(), st.font) {
                        st.sw = w.max(0.0);
                    }
                }
                "stroke-linecap" => {
                    st.cap = match v {
                        "round" => LineCap::Round,
                        "square" => LineCap::Square,
                        _ => LineCap::Butt,
                    }
                }
                "stroke-linejoin" => {
                    st.join = match v {
                        "round" => LineJoin::Round,
                        "bevel" => LineJoin::Bevel,
                        _ => LineJoin::Miter,
                    }
                }
                "stroke-miterlimit" => st.miter = v.parse().unwrap_or(4.0f32).max(1.0),
                "stroke-dasharray" => {
                    st.dash = if v == "none" {
                        Vec::new()
                    } else {
                        style::numbers(v).into_iter().map(f32::abs).collect()
                    }
                }
                "stroke-dashoffset" => st.dash_off = v.parse().unwrap_or(0.0),
                "fill-rule" => {
                    st.fill_rule = if v == "evenodd" {
                        FillRule::EvenOdd
                    } else {
                        FillRule::Winding
                    }
                }
                "color" => {
                    if let Some(c) = style::color(v) {
                        st.color = c;
                    }
                }
                "font-size" => {
                    if let Some(f) = style::length(v, parent_font, parent_font) {
                        st.font = f.clamp(0.0, 10_000.0);
                    }
                }
                "font-family" => {
                    st.family = v
                        .split(',')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .trim_matches(|c| c == '"' || c == '\'')
                        .to_string();
                }
                "font-weight" => {
                    st.bold =
                        matches!(v, "bold" | "bolder") || v.parse::<u32>().is_ok_and(|w| w >= 600);
                }
                "font-style" => st.italic = matches!(v, "italic" | "oblique"),
                "text-anchor" => {
                    st.anchor = match v {
                        "middle" => Anchor::Middle,
                        "end" => Anchor::End,
                        _ => Anchor::Start,
                    }
                }
                "display" => shown = v != "none",
                "visibility" => st.visible = v == "visible",
                _ => {}
            }
        }
        shown
    }

    fn walk(&mut self, i: usize, st: &State, ts: Transform, use_depth: usize) {
        let doc = self.doc;
        let Some(el) = doc.el(i) else { return };
        if self.draws >= MAX_DRAWS {
            return;
        }
        let mut st = st.clone();
        if !self.apply(el, &mut st) {
            return;
        }
        let ts = match el.attr("transform") {
            Some(t) => ts.pre_concat(style::transform(t)),
            None => ts,
        };
        match el.name.as_str() {
            "svg" | "g" | "a" | "switch" => {
                let ts = if el.name == "svg" && i != 0 {
                    let x = self.len(el, "x", self.vw, st.font);
                    let y = self.len(el, "y", self.vh, st.font);
                    let w = el
                        .attr("width")
                        .and_then(|v| style::length(v, self.vw, st.font))
                        .unwrap_or(self.vw);
                    let h = el
                        .attr("height")
                        .and_then(|v| style::length(v, self.vh, st.font))
                        .unwrap_or(self.vh);
                    ts.pre_translate(x, y).pre_concat(view_box(el, w, h))
                } else {
                    ts
                };
                self.kids(el, &st, ts, use_depth);
            }
            "use" => self.use_el(el, &st, ts, use_depth),
            "text" => self.text(el, &st, ts),
            "path" | "rect" | "circle" | "ellipse" | "line" | "polyline" | "polygon" => {
                if let Some(p) = self.shape(el, &st) {
                    self.draw(&p, &st, ts);
                }
            }
            // defs, symbol, gradients, clipPath, mask, pattern, marker, filter, style, script,
            // foreignObject, image, title, desc, metadata…: not drawn here (S-RES-07, ADR-0021).
            _ => {}
        }
    }

    fn kids(&mut self, el: &El, st: &State, ts: Transform, use_depth: usize) {
        for k in &el.kids {
            if let Kid::El(c) = k {
                self.walk(*c, st, ts, use_depth);
            }
        }
    }

    fn use_el(&mut self, el: &El, st: &State, ts: Transform, use_depth: usize) {
        // Same-document references only (S-RES-07): `#id`; anything else is ignored.
        let Some(id) = el.attr("href").and_then(|h| h.trim().strip_prefix('#')) else {
            return;
        };
        let Some(&target) = self.ids.get(id) else {
            return;
        };
        if use_depth >= MAX_USE_DEPTH || self.instances >= MAX_INSTANCES {
            return;
        }
        self.instances += 1;
        let x = self.len(el, "x", self.vw, st.font);
        let y = self.len(el, "y", self.vh, st.font);
        let ts = ts.pre_translate(x, y);
        let Some(t) = self.doc.el(target) else { return };
        if t.name == "symbol" {
            let w = el
                .attr("width")
                .and_then(|v| style::length(v, self.vw, st.font))
                .unwrap_or(self.vw);
            let h = el
                .attr("height")
                .and_then(|v| style::length(v, self.vh, st.font))
                .unwrap_or(self.vh);
            let mut s2 = st.clone();
            if self.apply(t, &mut s2) {
                let ts = ts.pre_concat(view_box(t, w, h));
                self.kids(t, &s2, ts, use_depth + 1);
            }
        } else {
            self.walk(target, st, ts, use_depth + 1);
        }
    }

    fn shape(&mut self, el: &El, st: &State) -> Option<tiny_skia::Path> {
        let (vw, vh, f) = (self.vw, self.vh, st.font);
        let num = |k: &str, pct: f32| {
            el.attr(k)
                .and_then(|v| style::length(v, pct, f))
                .unwrap_or(0.0)
        };
        match el.name.as_str() {
            "path" => {
                let d = el.attr("d")?;
                crate::path::parse(d, &mut self.path_budget)
            }
            "rect" => {
                let (x, y, w, h) = (
                    num("x", vw),
                    num("y", vh),
                    num("width", vw),
                    num("height", vh),
                );
                if w <= 0.0 || h <= 0.0 {
                    return None;
                }
                let rxa = el.attr("rx").and_then(|v| style::length(v, vw, f));
                let rya = el.attr("ry").and_then(|v| style::length(v, vh, f));
                let (rx, ry) = match (rxa, rya) {
                    (Some(a), Some(b)) => (a, b),
                    (Some(a), None) => (a, a),
                    (None, Some(b)) => (b, b),
                    (None, None) => (0.0, 0.0),
                };
                let (rx, ry) = (rx.clamp(0.0, w / 2.0), ry.clamp(0.0, h / 2.0));
                if rx <= 0.0 || ry <= 0.0 {
                    return Some(PathBuilder::from_rect(Rect::from_xywh(x, y, w, h)?));
                }
                let k = 0.552_284_8;
                let mut pb = PathBuilder::new();
                pb.move_to(x + rx, y);
                pb.line_to(x + w - rx, y);
                pb.cubic_to(
                    x + w - rx + k * rx,
                    y,
                    x + w,
                    y + ry - k * ry,
                    x + w,
                    y + ry,
                );
                pb.line_to(x + w, y + h - ry);
                pb.cubic_to(
                    x + w,
                    y + h - ry + k * ry,
                    x + w - rx + k * rx,
                    y + h,
                    x + w - rx,
                    y + h,
                );
                pb.line_to(x + rx, y + h);
                pb.cubic_to(
                    x + rx - k * rx,
                    y + h,
                    x,
                    y + h - ry + k * ry,
                    x,
                    y + h - ry,
                );
                pb.line_to(x, y + ry);
                pb.cubic_to(x, y + ry - k * ry, x + rx - k * rx, y, x + rx, y);
                pb.close();
                pb.finish()
            }
            "circle" | "ellipse" => {
                let (cx, cy) = (num("cx", vw), num("cy", vh));
                let (rx, ry) = if el.name == "circle" {
                    let r = num("r", self.diag());
                    (r, r)
                } else {
                    (num("rx", vw), num("ry", vh))
                };
                if rx <= 0.0 || ry <= 0.0 {
                    return None;
                }
                let mut pb = PathBuilder::new();
                pb.push_oval(Rect::from_xywh(cx - rx, cy - ry, 2.0 * rx, 2.0 * ry)?);
                pb.finish()
            }
            "line" => {
                let mut pb = PathBuilder::new();
                pb.move_to(num("x1", vw), num("y1", vh));
                pb.line_to(num("x2", vw), num("y2", vh));
                pb.finish()
            }
            _ => {
                let pts = style::numbers(el.attr("points")?);
                let n = (pts.len() / 2).min(self.path_budget);
                self.path_budget -= n;
                let mut pb = PathBuilder::new();
                for (j, &[x, y]) in pts.as_chunks::<2>().0.iter().take(n).enumerate() {
                    if j == 0 {
                        pb.move_to(x, y);
                    } else {
                        pb.line_to(x, y);
                    }
                }
                if el.name == "polygon" {
                    pb.close();
                }
                pb.finish()
            }
        }
    }

    fn shader(&self, p: &Paint, st: &State, op: f32, bbox: Rect) -> Option<Shader<'static>> {
        let solid = |c: Rgba| {
            let a = f32::from(c[3]) / 255.0 * op * st.opacity;
            Some(Shader::SolidColor(tiny_skia::Color::from_rgba8(
                c[0],
                c[1],
                c[2],
                style::clamp_u8(a * 255.0),
            )))
        };
        match p {
            Paint::None => None,
            Paint::Color(c) => solid(*c),
            Paint::Current => solid(st.color),
            Paint::Url(id, fb) => match self.gradient(id, bbox, op * st.opacity, st.color) {
                Some(s) => Some(s),
                None => fb.as_deref().and_then(|f| self.shader(f, st, op, bbox)),
            },
        }
    }

    /// Gradient element chain (`href` inheritance, ≤ 8 links).
    fn chain(&self, id: &str) -> Vec<usize> {
        let mut out = Vec::new();
        let mut cur = self.ids.get(id).copied();
        while let Some(i) = cur {
            if out.len() >= MAX_USE_DEPTH || out.contains(&i) {
                break;
            }
            out.push(i);
            cur = self
                .doc
                .el(i)
                .and_then(|e| e.attr("href"))
                .and_then(|h| h.trim().strip_prefix('#'))
                .and_then(|h| self.ids.get(h).copied());
        }
        out
    }

    /// First stop color of a gradient (text fill fallback).
    fn first_stop(&self, id: &str, color: Rgba) -> Option<Rgba> {
        self.stops(&self.chain(id), 1.0, color).first().map(|s| s.1)
    }

    fn stops(&self, chain: &[usize], op: f32, current: Rgba) -> Vec<(f32, Rgba)> {
        let doc = self.doc;
        for &g in chain {
            let Some(el) = doc.el(g) else { continue };
            let mut out: Vec<(f32, Rgba)> = Vec::new();
            for k in &el.kids {
                let Kid::El(s) = k else { continue };
                let Some(se) = doc.el(*s).filter(|e| e.name == "stop") else {
                    continue;
                };
                let (mut off, mut col, mut sop) = (0.0f32, [0, 0, 0, 255], 1.0f32);
                for (k, v) in props(se) {
                    match k.as_str() {
                        "offset" => off = unit(&v).unwrap_or(0.0),
                        "stop-color" => {
                            col = if v.eq_ignore_ascii_case("currentcolor") {
                                current
                            } else {
                                style::color(&v).unwrap_or(col)
                            };
                        }
                        "stop-opacity" => sop = unit(&v).unwrap_or(1.0).clamp(0.0, 1.0),
                        _ => {}
                    }
                }
                let prev = out.last().map_or(0.0, |p| p.0);
                let a = f32::from(col[3]) * sop * op;
                out.push((
                    off.clamp(0.0, 1.0).max(prev),
                    [col[0], col[1], col[2], style::clamp_u8(a)],
                ));
            }
            if !out.is_empty() {
                return out;
            }
        }
        Vec::new()
    }

    fn gradient(&self, id: &str, bbox: Rect, op: f32, current: Rgba) -> Option<Shader<'static>> {
        let chain = self.chain(id);
        let doc = self.doc;
        let first = doc.el(*chain.first()?)?;
        let kind = first.name.as_str();
        if kind != "linearGradient" && kind != "radialGradient" {
            return None;
        }
        let get = |k: &str| {
            chain
                .iter()
                .find_map(|&e| doc.el(e).and_then(|x| x.attr(k)))
        };
        let stops: Vec<GradientStop> = self
            .stops(&chain, op, current)
            .into_iter()
            .map(|(o, c)| {
                GradientStop::new(o, tiny_skia::Color::from_rgba8(c[0], c[1], c[2], c[3]))
            })
            .collect();
        let user = get("gradientUnits") == Some("userSpaceOnUse");
        let coord = |k: &str, def: f32, pct: f32| -> f32 {
            match get(k) {
                Some(v) if user => style::length(v, pct, 16.0).unwrap_or(def * pct),
                Some(v) => unit(v).unwrap_or(def),
                None if user => def * pct,
                None => def,
            }
        };
        let mut t = if user {
            Transform::identity()
        } else {
            if bbox.width() <= 0.0 || bbox.height() <= 0.0 {
                return None;
            }
            Transform::from_row(bbox.width(), 0.0, 0.0, bbox.height(), bbox.x(), bbox.y())
        };
        if let Some(gt) = get("gradientTransform") {
            t = t.pre_concat(style::transform(gt));
        }
        let mode = match get("spreadMethod") {
            Some("reflect") => SpreadMode::Reflect,
            Some("repeat") => SpreadMode::Repeat,
            _ => SpreadMode::Pad,
        };
        let (vw, vh, d) = (self.vw, self.vh, self.diag());
        if kind == "linearGradient" {
            let p1 = Point::from_xy(coord("x1", 0.0, vw), coord("y1", 0.0, vh));
            let p2 = Point::from_xy(coord("x2", 1.0, vw), coord("y2", 0.0, vh));
            LinearGradient::new(p1, p2, stops, mode, t)
        } else {
            let (cx, cy, r) = (
                coord("cx", 0.5, vw),
                coord("cy", 0.5, vh),
                coord("r", 0.5, d),
            );
            let fx = if get("fx").is_some() {
                coord("fx", 0.5, vw)
            } else {
                cx
            };
            let fy = if get("fy").is_some() {
                coord("fy", 0.5, vh)
            } else {
                cy
            };
            RadialGradient::new(
                Point::from_xy(fx, fy),
                0.0,
                Point::from_xy(cx, cy),
                r,
                stops,
                mode,
                t,
            )
        }
    }

    fn draw(&mut self, p: &tiny_skia::Path, st: &State, ts: Transform) {
        if !st.visible {
            return;
        }
        self.draws += 1;
        let bbox = p.compute_tight_bounds().unwrap_or_else(|| p.bounds());
        if let Some(sh) = self.shader(&st.fill, st, st.fill_op, bbox) {
            let paint = SkPaint {
                shader: sh,
                anti_alias: true,
                ..SkPaint::default()
            };
            self.pix.fill_path(p, &paint, st.fill_rule, ts, None);
        }
        if st.sw > 0.0
            && let Some(sh) = self.shader(&st.stroke, st, st.stroke_op, bbox)
        {
            let mut dash = st.dash.clone();
            if dash.len() % 2 == 1 {
                dash.extend_from_within(..);
            }
            let stroke = Stroke {
                width: st.sw,
                miter_limit: st.miter,
                line_cap: st.cap,
                line_join: st.join,
                dash: if dash.iter().any(|d| *d > 0.0) {
                    StrokeDash::new(dash, st.dash_off)
                } else {
                    None
                },
            };
            let paint = SkPaint {
                shader: sh,
                anti_alias: true,
                ..SkPaint::default()
            };
            self.pix.stroke_path(p, &paint, &stroke, ts, None);
        }
    }

    fn text_color(&self, st: &State) -> Option<Rgba> {
        let c = match &st.fill {
            Paint::None => return None,
            Paint::Color(c) => *c,
            Paint::Current => st.color,
            Paint::Url(id, fb) => self
                .first_stop(id, st.color)
                .or(match fb.as_deref() {
                    Some(Paint::Color(c)) => Some(*c),
                    _ => None,
                })
                .unwrap_or([0, 0, 0, 255]),
        };
        let a = f32::from(c[3]) * st.fill_op * st.opacity;
        Some([c[0], c[1], c[2], style::clamp_u8(a)])
    }

    fn text(&mut self, el: &El, st: &State, ts: Transform) {
        let first = |k: &str, pct: f32, f: f32| {
            el.attr(k).and_then(|v| {
                v.split(|c: char| c == ',' || c.is_whitespace())
                    .find(|s| !s.is_empty())
                    .and_then(|s| style::length(s, pct, f))
            })
        };
        let x = first("x", self.vw, st.font).unwrap_or(0.0)
            + first("dx", self.vw, st.font).unwrap_or(0.0);
        let y = first("y", self.vh, st.font).unwrap_or(0.0)
            + first("dy", self.vh, st.font).unwrap_or(0.0);
        let start = self.texts.len();
        let mut pen = Some((x, y));
        self.text_kids(el, st, ts, &mut pen, x, y);
        // Trim the text element's leading and trailing white space.
        if let Some(r) = self.texts.get_mut(start) {
            r.text = r.text.trim_start().to_string();
        }
        if self.texts.len() > start
            && let Some(r) = self.texts.last_mut()
        {
            r.text = r.text.trim_end().to_string();
        }
        let mut i = start;
        while i < self.texts.len() {
            if self.texts.get(i).is_some_and(|r| r.text.is_empty()) {
                // Keep a dropped run's absolute position for the run after it.
                let (cont, rx, ry) = self
                    .texts
                    .get(i)
                    .map_or((true, 0.0, 0.0), |r| (r.continues, r.x, r.y));
                self.texts.remove(i);
                if let Some(nx) = self.texts.get_mut(i).filter(|n| n.continues && !cont) {
                    nx.continues = false;
                    nx.x = rx;
                    nx.y = ry;
                }
            } else {
                i += 1;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn text_kids(
        &mut self,
        el: &El,
        st: &State,
        ts: Transform,
        pen: &mut Option<(f32, f32)>,
        x: f32,
        y: f32,
    ) {
        for k in &el.kids {
            match k {
                Kid::Text(t) => {
                    let mut s = String::with_capacity(t.len());
                    let mut ws = false;
                    for c in t.chars() {
                        if c.is_whitespace() {
                            if !ws {
                                s.push(' ');
                            }
                            ws = true;
                        } else {
                            s.push(c);
                            ws = false;
                        }
                    }
                    let Some(color) = self.text_color(st).filter(|_| st.visible) else {
                        // Invisible text still advances the pen; approximate by continuing.
                        continue;
                    };
                    let (cont, px, py) = match pen.take() {
                        Some((a, b)) => (false, a, b),
                        None => (true, x, y),
                    };
                    let p = ts.map_point_xy(px, py);
                    let scale = (ts.sx * ts.sy - ts.kx * ts.ky).abs().sqrt();
                    self.texts.push(TextRun {
                        x: p.0,
                        y: p.1,
                        size: st.font * scale,
                        color,
                        anchor: st.anchor,
                        bold: st.bold,
                        italic: st.italic,
                        family: st.family.clone(),
                        text: s,
                        continues: cont,
                    });
                }
                Kid::El(c) => {
                    let Some(sub) = self.doc.el(*c).filter(|e| e.name == "tspan") else {
                        continue;
                    };
                    let mut s2 = st.clone();
                    if !self.apply(sub, &mut s2) {
                        continue;
                    }
                    let f = s2.font;
                    let num = |k: &str, pct: f32| {
                        sub.attr(k).and_then(|v| {
                            v.split(|c: char| c == ',' || c.is_whitespace())
                                .find(|s| !s.is_empty())
                                .and_then(|s| style::length(s, pct, f))
                        })
                    };
                    let (nx, ny) = (num("x", self.vw), num("y", self.vh));
                    let (dx, dy) = (
                        num("dx", self.vw).unwrap_or(0.0),
                        num("dy", self.vh).unwrap_or(0.0),
                    );
                    let last = self.texts.last().map(|r| (r.x, r.y));
                    let base_y = last.map_or(y, |(_, ly)| {
                        ts.invert().map_or(y, |inv| inv.map_point_xy(0.0, ly).1)
                    });
                    if nx.is_some() || ny.is_some() || dy != 0.0 {
                        let ax = nx.map_or_else(
                            || {
                                // dy-only shift keeps x flowing; approximated as a new chunk at x.
                                x
                            },
                            |v| v + dx,
                        );
                        *pen = Some((ax, ny.unwrap_or(base_y) + dy));
                    }
                    self.text_kids(sub, &s2, ts, pen, x, y);
                }
            }
        }
    }
}

trait MapXy {
    fn map_point_xy(&self, x: f32, y: f32) -> (f32, f32);
}

impl MapXy for Transform {
    fn map_point_xy(&self, x: f32, y: f32) -> (f32, f32) {
        (
            self.sx * x + self.kx * y + self.tx,
            self.ky * x + self.sy * y + self.ty,
        )
    }
}

/// `viewBox` + `preserveAspectRatio` of `el` mapped onto a `w × h` viewport.
fn view_box(el: &El, w: f32, h: f32) -> Transform {
    let vb = el.attr("viewBox").map(style::numbers).unwrap_or_default();
    let [vx, vy, vw, vh] = match vb.as_slice() {
        [a, b, c, d] if *c > 0.0 && *d > 0.0 => [*a, *b, *c, *d],
        _ => return Transform::identity(),
    };
    let par = el.attr("preserveAspectRatio").unwrap_or("xMidYMid meet");
    let (sx, sy) = (w / vw, h / vh);
    if par.trim().starts_with("none") {
        return Transform::from_row(sx, 0.0, 0.0, sy, -vx * sx, -vy * sy);
    }
    let s = if par.contains("slice") {
        sx.max(sy)
    } else {
        sx.min(sy)
    };
    let (ex, ey) = (w - vw * s, h - vh * s);
    let ax = if par.contains("xMin") {
        0.0
    } else if par.contains("xMax") {
        ex
    } else {
        ex / 2.0
    };
    let ay = if par.contains("YMin") {
        0.0
    } else if par.contains("YMax") {
        ey
    } else {
        ey / 2.0
    };
    Transform::from_row(s, 0.0, 0.0, s, ax - vx * s, ay - vy * s)
}

fn intrinsic(doc: &Doc) -> Result<(f32, f32), Error> {
    let root = doc.el(0).ok_or(Error::NoSvg)?;
    let vb = root.attr("viewBox").map(style::numbers).unwrap_or_default();
    let vb = match vb.as_slice() {
        [_, _, c, d] if *c > 0.0 && *d > 0.0 => Some((*c, *d)),
        _ => None,
    };
    let dim = |k: &str| {
        root.attr(k)
            .filter(|v| !v.trim().ends_with('%'))
            .and_then(|v| style::length(v, 0.0, 16.0))
            .filter(|v| *v > 0.0)
    };
    let (w, h) = match (dim("width"), dim("height"), vb) {
        (Some(w), Some(h), _) => (w, h),
        (Some(w), None, Some((a, b))) => (w, w * b / a),
        (None, Some(h), Some((a, b))) => (h * a / b, h),
        (None, None, Some(v)) => v,
        (w, h, None) => (w.unwrap_or(300.0), h.unwrap_or(150.0)),
    };
    if !(w.is_finite() && h.is_finite()) || w > 1e6 || h > 1e6 {
        return Err(Error::TooLarge);
    }
    Ok((w.max(1.0), h.max(1.0)))
}

/// Intrinsic size (user units = CSS px): `width`/`height`, else the `viewBox`, else 300 × 150.
pub fn size(src: &str) -> Result<(f32, f32), Error> {
    intrinsic(&xml::parse(src)?)
}

/// Renders `src` into a `w × h` pixmap; `current` is `currentColor` (the text color).
pub fn render(src: &str, w: u32, h: u32, current: Rgba) -> Result<Image, Error> {
    let doc = xml::parse(src)?;
    let (iw, ih) = intrinsic(&doc)?;
    let (w, h) = (w.max(1), h.max(1));
    if u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err(Error::TooLarge);
    }
    let pix = Pixmap::new(w, h).ok_or(Error::TooLarge)?;
    let root = doc.el(0).ok_or(Error::NoSvg)?;
    let has_vb = root
        .attr("viewBox")
        .is_some_and(|v| style::numbers(v).len() == 4);
    #[allow(clippy::cast_precision_loss)]
    let (fw, fh) = (w as f32, h as f32);
    let ts = if has_vb {
        view_box(root, fw, fh)
    } else {
        Transform::from_scale(fw / iw, fh / ih)
    };
    let (vw, vh) = match root.attr("viewBox").map(style::numbers).as_deref() {
        Some([_, _, c, d]) if *c > 0.0 && *d > 0.0 => (*c, *d),
        _ => (iw, ih),
    };
    let ids = doc
        .els
        .iter()
        .enumerate()
        .filter_map(|(i, e)| e.attr("id").map(|id| (id, i)))
        .fold(HashMap::new(), |mut m, (id, i)| {
            m.entry(id).or_insert(i);
            m
        });
    let mut ctx = Ctx {
        doc: &doc,
        ids,
        pix,
        texts: Vec::new(),
        path_budget: MAX_PATH_COMMANDS,
        instances: 0,
        draws: 0,
        vw,
        vh,
    };
    let st = State::new(current);
    ctx.walk(0, &st, ts, 0);
    let texts = std::mem::take(&mut ctx.texts);
    Ok(Image {
        w,
        h,
        rgba: ctx.pix.take(),
        texts,
    })
}
