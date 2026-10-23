//! Path data (`d`) to a `tiny_skia` path, counting commands against the S-RES-07 cap.

use tiny_skia::{Path, PathBuilder};

/// Number scanner shared by path data, number lists and `points`.
pub struct Lexer<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Lexer<'a> {
    /// Lexer over `s`.
    pub fn new(s: &'a str) -> Self {
        Self {
            b: s.as_bytes(),
            i: 0,
        }
    }
    fn skip(&mut self) {
        while self
            .b
            .get(self.i)
            .is_some_and(|c| c.is_ascii_whitespace() || *c == b',')
        {
            self.i += 1;
        }
    }
    /// Next command letter, if the next token is one.
    fn command(&mut self) -> Option<u8> {
        self.skip();
        let c = *self.b.get(self.i)?;
        if c.is_ascii_alphabetic() && c != b'e' && c != b'E' {
            self.i += 1;
            Some(c)
        } else {
            None
        }
    }
    /// Whether a number follows.
    fn at_number(&mut self) -> bool {
        self.skip();
        self.b
            .get(self.i)
            .is_some_and(|c| c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.'))
    }
    /// Arc flag (`0`/`1`, may be written without separators).
    fn flag(&mut self) -> Option<bool> {
        self.skip();
        let c = *self.b.get(self.i)?;
        self.i += 1;
        match c {
            b'0' => Some(false),
            b'1' => Some(true),
            _ => None,
        }
    }
    /// Next number.
    pub fn number(&mut self) -> Option<f32> {
        self.skip();
        let start = self.i;
        if matches!(self.b.get(self.i), Some(b'-' | b'+')) {
            self.i += 1;
        }
        let mut dot = false;
        while let Some(&c) = self.b.get(self.i) {
            if c.is_ascii_digit() {
                self.i += 1;
            } else if c == b'.' && !dot {
                dot = true;
                self.i += 1;
            } else {
                break;
            }
        }
        if matches!(self.b.get(self.i), Some(b'e' | b'E'))
            && self
                .b
                .get(self.i + 1)
                .is_some_and(|c| c.is_ascii_digit() || matches!(c, b'-' | b'+'))
        {
            self.i += 2;
            while self.b.get(self.i).is_some_and(u8::is_ascii_digit) {
                self.i += 1;
            }
        }
        let t = std::str::from_utf8(self.b.get(start..self.i)?).ok()?;
        let v: f32 = t.parse().ok()?;
        v.is_finite().then_some(v)
    }
}

/// Parses `d`; stops (keeping what was built) at the first error, as SVG requires.
/// `budget` is decremented per command; at zero parsing stops.
pub fn parse(d: &str, budget: &mut usize) -> Option<Path> {
    let mut lx = Lexer::new(d);
    let mut pb = PathBuilder::new();
    let (mut cx, mut cy, mut sx, mut sy) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    // Last control point for S/T reflection.
    let mut last_c: Option<(f32, f32)> = None;
    let mut last_q: Option<(f32, f32)> = None;
    let mut cmd = 0u8;
    let mut started = false;
    'outer: loop {
        if *budget == 0 {
            break;
        }
        let next = lx.command();
        match next {
            Some(c) => cmd = c,
            None if lx.at_number() && cmd != 0 && !matches!(cmd, b'z' | b'Z') => {
                // Implicit repetition; after M the repeat is L.
                if cmd == b'M' {
                    cmd = b'L';
                } else if cmd == b'm' {
                    cmd = b'l';
                }
            }
            None => break,
        }
        *budget -= 1;
        let rel = cmd.is_ascii_lowercase();
        let (ox, oy) = if rel { (cx, cy) } else { (0.0, 0.0) };
        macro_rules! n {
            () => {
                match lx.number() {
                    Some(v) => v,
                    None => break 'outer,
                }
            };
        }
        let up = cmd.to_ascii_uppercase();
        if up != b'M' && !started {
            // Path data must begin with a moveto.
            break;
        }
        match up {
            b'M' => {
                let (x, y) = (ox + n!(), oy + n!());
                pb.move_to(x, y);
                started = true;
                (cx, cy, sx, sy) = (x, y, x, y);
                last_c = None;
                last_q = None;
            }
            b'L' | b'H' | b'V' => {
                let (x, y) = match up {
                    b'L' => (ox + n!(), oy + n!()),
                    b'H' => (ox + n!(), cy),
                    _ => (cx, oy + n!()),
                };
                pb.line_to(x, y);
                (cx, cy) = (x, y);
                last_c = None;
                last_q = None;
            }
            b'C' | b'S' => {
                let (x1, y1) = if up == b'C' {
                    (ox + n!(), oy + n!())
                } else {
                    last_c.map_or((cx, cy), |(px, py)| (2.0 * cx - px, 2.0 * cy - py))
                };
                let (x2, y2, x, y) = (ox + n!(), oy + n!(), ox + n!(), oy + n!());
                pb.cubic_to(x1, y1, x2, y2, x, y);
                last_c = Some((x2, y2));
                last_q = None;
                (cx, cy) = (x, y);
            }
            b'Q' | b'T' => {
                let (x1, y1) = if up == b'Q' {
                    (ox + n!(), oy + n!())
                } else {
                    last_q.map_or((cx, cy), |(px, py)| (2.0 * cx - px, 2.0 * cy - py))
                };
                let (x, y) = (ox + n!(), oy + n!());
                pb.quad_to(x1, y1, x, y);
                last_q = Some((x1, y1));
                last_c = None;
                (cx, cy) = (x, y);
            }
            b'A' => {
                let (rx, ry, rot) = (n!(), n!(), n!());
                let Some(large) = lx.flag() else { break };
                let Some(sweep) = lx.flag() else { break };
                let (x, y) = (ox + n!(), oy + n!());
                arc(&mut pb, (cx, cy), rx, ry, rot, large, sweep, (x, y));
                (cx, cy) = (x, y);
                last_c = None;
                last_q = None;
            }
            b'Z' => {
                pb.close();
                (cx, cy) = (sx, sy);
                last_c = None;
                last_q = None;
            }
            _ => break,
        }
    }
    pb.finish()
}

/// Appends an SVG elliptical arc as cubic Béziers (F.6.5 endpoint → center conversion).
#[allow(clippy::too_many_arguments, clippy::many_single_char_names)]
pub fn arc(
    pb: &mut PathBuilder,
    p0: (f32, f32),
    rx: f32,
    ry: f32,
    rot_deg: f32,
    large: bool,
    sweep: bool,
    p1: (f32, f32),
) {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx < 1e-6 || ry < 1e-6 || (p0.0 - p1.0).abs() + (p0.1 - p1.1).abs() < 1e-6 {
        pb.line_to(p1.0, p1.1);
        return;
    }
    let phi = rot_deg.to_radians();
    let (sin, cos) = phi.sin_cos();
    let (dx, dy) = ((p0.0 - p1.0) / 2.0, (p0.1 - p1.1) / 2.0);
    let x1p = cos * dx + sin * dy;
    let y1p = -sin * dx + cos * dy;
    let lam = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
    if lam > 1.0 {
        let k = lam.sqrt();
        rx *= k;
        ry *= k;
    }
    let num = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p;
    let den = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
    let mut co = (num / den).max(0.0).sqrt();
    if large == sweep {
        co = -co;
    }
    let cxp = co * rx * y1p / ry;
    let cyp = -co * ry * x1p / rx;
    let cx = cos * cxp - sin * cyp + (p0.0 + p1.0) / 2.0;
    let cy = sin * cxp + cos * cyp + (p0.1 + p1.1) / 2.0;
    let ang = |ux: f32, uy: f32, vx: f32, vy: f32| {
        let a = (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
        if a.is_finite() { a } else { 0.0 }
    };
    let t1 = ang(1.0, 0.0, (x1p - cxp) / rx, (y1p - cyp) / ry);
    let mut dt = ang(
        (x1p - cxp) / rx,
        (y1p - cyp) / ry,
        (-x1p - cxp) / rx,
        (-y1p - cyp) / ry,
    );
    let tau = std::f32::consts::TAU;
    if !sweep && dt > 0.0 {
        dt -= tau;
    } else if sweep && dt < 0.0 {
        dt += tau;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let segs = ((dt.abs() / std::f32::consts::FRAC_PI_2).ceil() as usize).clamp(1, 4);
    let step = dt / segs as f32;
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let pt = |t: f32| {
        let (s, c) = t.sin_cos();
        (
            cx + rx * c * cos - ry * s * sin,
            cy + rx * c * sin + ry * s * cos,
        )
    };
    let der = |t: f32| {
        let (s, c) = t.sin_cos();
        (-rx * s * cos - ry * c * sin, -rx * s * sin + ry * c * cos)
    };
    let mut t = t1;
    for i in 0..segs {
        let t2 = t + step;
        let (a, b) = (pt(t), pt(t2));
        let (da, db) = (der(t), der(t2));
        let end = if i + 1 == segs { p1 } else { b };
        pb.cubic_to(
            a.0 + k * da.0,
            a.1 + k * da.1,
            b.0 - k * db.0,
            b.1 - k * db.1,
            end.0,
            end.1,
        );
        t = t2;
    }
}
