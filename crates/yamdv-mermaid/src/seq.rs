//! Sequence diagram model, parser and layout (`sequenceDiagram`, FUT-18 subset).
// Participant indices come from this module's own parser (every index < `parts.len()`).
#![allow(clippy::indexing_slicing)]
#![allow(
    clippy::cast_precision_loss,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

use std::collections::HashMap;

use crate::draw::{self, Drawing, Head, Ink, Measure, Path, Prim, Role, Seg, TextPrim};
use crate::{Error, MAX_EDGES, MAX_NODES, err};

/// A participant or actor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Participant {
    /// Id.
    pub id: String,
    /// Label (`as …`, default the id).
    pub label: String,
    /// Drawn as a stick figure.
    pub actor: bool,
}

/// Message arrowhead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tip {
    /// `->>` filled arrow.
    Arrow,
    /// `->` no head.
    None,
    /// `-x` cross.
    Cross,
    /// `-)` open (async) arrow.
    Async,
}

/// A message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Sender.
    pub from: usize,
    /// Receiver.
    pub to: usize,
    /// Dotted line (`--`).
    pub dotted: bool,
    /// Head.
    pub tip: Tip,
    /// Text.
    pub text: String,
    /// `+` activates the receiver (`Some(true)`); `-` deactivates the sender (`Some(false)`).
    pub act: Option<bool>,
}

/// Note placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotePos {
    /// `left of`.
    Left,
    /// `right of`.
    Right,
    /// `over`.
    Over,
}

/// Frame kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frame {
    /// `loop`.
    Loop,
    /// `alt` (with `else`).
    Alt,
    /// `opt`.
    Opt,
    /// `par` (with `and`).
    Par,
}

impl Frame {
    fn word(self) -> &'static str {
        match self {
            Self::Loop => "loop",
            Self::Alt => "alt",
            Self::Opt => "opt",
            Self::Par => "par",
        }
    }
}

/// One statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Message.
    Msg(Message),
    /// Note.
    Note {
        /// Placement.
        pos: NotePos,
        /// First participant.
        a: usize,
        /// Second participant (`over A,B`).
        b: Option<usize>,
        /// Text.
        text: String,
    },
    /// `activate`.
    Activate(usize),
    /// `deactivate`.
    Deactivate(usize),
    /// Frame start.
    Start(Frame, String),
    /// `else` / `and`.
    Divider(String),
    /// `end`.
    End,
}

/// A parsed sequence diagram.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Seq {
    /// Participants in order.
    pub parts: Vec<Participant>,
    /// Statements.
    pub events: Vec<Event>,
    /// `autonumber`.
    pub autonumber: bool,
    /// `title`.
    pub title: Option<String>,
}

struct P {
    s: Seq,
    ids: HashMap<String, usize>,
}

impl P {
    fn part(&mut self, id: &str, ln: usize, col: usize) -> Result<usize, Error> {
        if id.is_empty() {
            return Err(err("Expected a participant", ln, col));
        }
        if let Some(i) = self.ids.get(id) {
            return Ok(*i);
        }
        if self.s.parts.len() >= MAX_NODES {
            return Err(err("Too many participants", ln, col));
        }
        self.s.parts.push(Participant {
            id: id.to_string(),
            label: id.to_string(),
            actor: false,
        });
        self.ids.insert(id.to_string(), self.s.parts.len() - 1);
        Ok(self.s.parts.len() - 1)
    }
}

const ARROWS: [(&str, bool, Tip); 8] = [
    ("-->>", true, Tip::Arrow),
    ("--x", true, Tip::Cross),
    ("--)", true, Tip::Async),
    ("-->", true, Tip::None),
    ("->>", false, Tip::Arrow),
    ("-x", false, Tip::Cross),
    ("-)", false, Tip::Async),
    ("->", false, Tip::None),
];

fn col_of(line: &str, byte: usize) -> usize {
    line.get(..byte).map_or(0, |s| s.chars().count())
}

/// Parses a sequence diagram.
pub fn parse(src: &str) -> Result<Seq, Error> {
    let mut p = P {
        s: Seq::default(),
        ids: HashMap::new(),
    };
    let mut frames: Vec<(Frame, usize)> = Vec::new();
    for (k, (ln, l)) in crate::lines(src).enumerate() {
        if k == 0 {
            continue; // `sequenceDiagram`
        }
        let lead = l.len() - l.trim_start().len();
        let t = l.trim();
        let word = t.split_whitespace().next().unwrap_or("");
        let rest = t.get(word.len()..).unwrap_or("").trim();
        if p.s.events.len() >= MAX_EDGES {
            return Err(err("Too many statements", ln, lead));
        }
        match word.to_ascii_lowercase().as_str() {
            "participant" | "actor" => {
                let (id, label) = match rest.split_once(" as ") {
                    Some((a, b)) => (a.trim(), crate::label(b)),
                    None => (rest, crate::label(rest)),
                };
                let i = p.part(id, ln, lead + word.len())?;
                p.s.parts[i].label = label;
                p.s.parts[i].actor = word == "actor";
                continue;
            }
            "autonumber" => {
                p.s.autonumber = true;
                continue;
            }
            "title" | "title:" => {
                p.s.title = Some(crate::label(rest.trim_start_matches(':')));
                continue;
            }
            "activate" | "deactivate" => {
                let i = p.part(rest, ln, lead + word.len())?;
                p.s.events.push(if word == "activate" {
                    Event::Activate(i)
                } else {
                    Event::Deactivate(i)
                });
                continue;
            }
            "note" => {
                let low = rest.to_ascii_lowercase();
                let (pos, after) = if low.starts_with("left of") {
                    (NotePos::Left, rest.get(7..).unwrap_or(""))
                } else if low.starts_with("right of") {
                    (NotePos::Right, rest.get(8..).unwrap_or(""))
                } else if low.starts_with("over") {
                    (NotePos::Over, rest.get(4..).unwrap_or(""))
                } else {
                    return Err(err(
                        "Expected 'left of', 'right of' or 'over'",
                        ln,
                        lead + word.len() + 1,
                    ));
                };
                let Some((who, text)) = after.split_once(':') else {
                    return Err(err("Expected ':' after the note position", ln, l.len()));
                };
                let mut it = who.split(',');
                let a = p.part(it.next().unwrap_or("").trim(), ln, lead)?;
                let b = match it.next() {
                    Some(b) => Some(p.part(b.trim(), ln, lead)?),
                    None => None,
                };
                p.s.events.push(Event::Note {
                    pos,
                    a,
                    b,
                    text: crate::label(text),
                });
                continue;
            }
            "loop" | "alt" | "opt" | "par" => {
                let f = match word {
                    "loop" => Frame::Loop,
                    "alt" => Frame::Alt,
                    "opt" => Frame::Opt,
                    _ => Frame::Par,
                };
                frames.push((f, ln));
                p.s.events.push(Event::Start(f, crate::label(rest)));
                continue;
            }
            "else" | "and" => {
                let want = if word == "else" {
                    Frame::Alt
                } else {
                    Frame::Par
                };
                if frames.last().map(|f| f.0) != Some(want) {
                    return Err(err(format!("'{word}' outside '{}'", want.word()), ln, lead));
                }
                p.s.events.push(Event::Divider(crate::label(rest)));
                continue;
            }
            "end" => {
                if frames.pop().is_none() {
                    return Err(err("'end' without a frame", ln, lead));
                }
                p.s.events.push(Event::End);
                continue;
            }
            "link" | "links" | "click" | "properties" | "details" => continue, // ignored (security)
            _ => {}
        }
        // Message: `A->>+B: text`.
        let (head, text) = match t.split_once(':') {
            Some((h, x)) => (h, crate::label(x)),
            None => (t, String::new()),
        };
        let Some(dash) = head.find('-') else {
            return Err(err(format!("Unexpected '{t}'"), ln, lead));
        };
        let from_id = head.get(..dash).unwrap_or("").trim();
        let tail = head.get(dash..).unwrap_or("");
        let Some(&(tok, dotted, tip)) = ARROWS.iter().find(|a| tail.starts_with(a.0)) else {
            return Err(err("Unknown arrow", ln, lead + col_of(t, dash)));
        };
        let mut after = tail.get(tok.len()..).unwrap_or("").trim_start();
        let mut act = None;
        if let Some(a) = after.strip_prefix('+') {
            act = Some(true);
            after = a;
        } else if let Some(a) = after.strip_prefix('-') {
            act = Some(false);
            after = a;
        }
        let to_id = after.trim();
        if to_id.contains(char::is_whitespace) || to_id.contains(['-', '>']) {
            return Err(err(
                "Expected ':' after the receiver",
                ln,
                lead + col_of(t, head.len()),
            ));
        }
        let from = p.part(from_id, ln, lead)?;
        let to = p.part(to_id, ln, lead + col_of(t, head.len()))?;
        p.s.events.push(Event::Msg(Message {
            from,
            to,
            dotted,
            tip,
            text,
            act,
        }));
    }
    if let Some((_, ln)) = frames.last() {
        return Err(err("Unclosed frame (missing 'end')", *ln, 0));
    }
    Ok(p.s)
}

struct OpenFrame {
    kind: Frame,
    label: String,
    y: f32,
    lo: usize,
    hi: usize,
    inner: usize,
    dividers: Vec<(f32, String)>,
}

/// Lays a sequence diagram out.
#[allow(clippy::too_many_lines)]
pub(crate) fn layout(q: &Seq, m: &mut dyn Measure, s: f32) -> Drawing {
    let n = q.parts.len();
    let fs = 14.0 * s;
    let mut prims: Vec<Prim> = Vec::new();
    // Participant heads.
    let heads: Vec<(u32, f32, f32)> = q
        .parts
        .iter()
        .map(|p| m.text(&p.label, fs, 150.0 * s, false))
        .collect();
    let pw: Vec<f32> = heads
        .iter()
        .map(|t| (t.1 + 24.0 * s).max(70.0 * s))
        .collect();
    let ph = q
        .parts
        .iter()
        .zip(&heads)
        .map(|(p, t)| {
            if p.actor {
                t.2 + 44.0 * s
            } else {
                t.2 + 16.0 * s
            }
        })
        .fold(0.0f32, f32::max);
    // Column constraints: (i, j, min center distance).
    let mut cons: Vec<(usize, usize, f32)> = Vec::new();
    for i in 1..n {
        cons.push((i - 1, i, (pw[i - 1] + pw[i]) / 2.0 + 30.0 * s));
    }
    let mut texts: Vec<Option<(u32, f32, f32)>> = Vec::with_capacity(q.events.len());
    let (mut left_need, mut right_need) = (0.0f32, 0.0f32);
    for e in &q.events {
        let t = match e {
            Event::Msg(msg) => {
                let t = m.text(&msg.text, fs, 260.0 * s, false);
                let (a, b) = (msg.from.min(msg.to), msg.from.max(msg.to));
                if a == b {
                    if a + 1 < n {
                        cons.push((a, a + 1, t.1 + 50.0 * s));
                    } else {
                        right_need = right_need.max(t.1 + 50.0 * s);
                    }
                } else {
                    cons.push((a, b, t.1 + 30.0 * s));
                }
                Some(t)
            }
            Event::Note { pos, a, b, text } => {
                let t = m.text(text, fs, 200.0 * s, false);
                let w = t.1 + 16.0 * s;
                match (pos, b) {
                    (NotePos::Left, _) => {
                        if *a > 0 {
                            cons.push((a - 1, *a, w + 20.0 * s + pw[a - 1] / 2.0));
                        } else {
                            left_need = left_need.max(w + 10.0 * s);
                        }
                    }
                    (NotePos::Right, _) => {
                        if a + 1 < n {
                            cons.push((*a, a + 1, w + 20.0 * s + pw[a + 1] / 2.0));
                        } else {
                            right_need = right_need.max(w + 10.0 * s);
                        }
                    }
                    (NotePos::Over, Some(b)) if a != b => {
                        let (x, y) = ((*a).min(*b), (*a).max(*b));
                        cons.push((x, y, w - 40.0 * s));
                    }
                    (NotePos::Over, _) => {}
                }
                Some(t)
            }
            Event::Start(_, l) | Event::Divider(l) => {
                Some(m.text(&format!("[{l}]"), 13.0 * s, 300.0 * s, false))
            }
            _ => None,
        };
        texts.push(t);
    }
    // Gaps between neighbouring columns.
    let mut gaps = vec![0.0f32; n.saturating_sub(1)];
    cons.sort_by_key(|c| c.1 - c.0);
    for (a, b, need) in cons {
        if b <= a || b > gaps.len() {
            continue;
        }
        let cur: f32 = gaps[a..b].iter().sum();
        if cur < need {
            let d = (need - cur) / (b - a) as f32;
            for g in &mut gaps[a..b] {
                *g += d;
            }
        }
    }
    let x0 = left_need.max(pw.first().copied().unwrap_or(0.0) / 2.0) + 10.0 * s;
    let mut cx = Vec::with_capacity(n);
    let mut acc = x0;
    for i in 0..n {
        if i > 0 {
            acc += gaps[i - 1];
        }
        cx.push(acc);
    }
    let width = acc + right_need.max(pw.last().copied().unwrap_or(0.0) / 2.0) + 10.0 * s;

    let mut y = 4.0 * s;
    if let Some(t) = &q.title {
        let tt = m.text(t, 16.0 * s, width.max(200.0 * s), true);
        prims.push(Prim::Text(TextPrim {
            id: tt.0,
            x: (width - tt.1).max(0.0) / 2.0,
            y,
            color: Ink::Role(Role::Text),
        }));
        y += tt.2 + 12.0 * s;
    }
    let top = y;
    y += ph + 20.0 * s;
    let mut body: Vec<Prim> = Vec::new();
    let mut late: Vec<Prim> = Vec::new();
    let mut active: Vec<Vec<f32>> = vec![Vec::new(); n];
    let mut bars: Vec<(usize, usize, f32, f32)> = Vec::new();
    let mut frames: Vec<OpenFrame> = Vec::new();
    let mut frame_out: Vec<(OpenFrame, f32)> = Vec::new();
    let mut last_y = y;
    let mut num = 0u32;
    let touch = |frames: &mut Vec<OpenFrame>, a: usize, b: usize| {
        for f in frames.iter_mut() {
            f.lo = f.lo.min(a);
            f.hi = f.hi.max(b);
        }
    };
    let edge =
        |v: &[f32], toward: f32| -> f32 { if v.is_empty() { 0.0 } else { 5.0 * s * toward } };
    for (e, t) in q.events.iter().zip(&texts) {
        match e {
            Event::Msg(msg) => {
                let t = t.unwrap_or((0, 0.0, 0.0));
                let (a, b) = (msg.from, msg.to);
                touch(&mut frames, a.min(b), a.max(b));
                y += t.2 + 4.0 * s;
                let ly = y;
                let ink = Some(Ink::Role(Role::Edge));
                let head = match msg.tip {
                    Tip::Arrow => Some(Head::Arrow),
                    Tip::Cross => Some(Head::Cross),
                    Tip::Async => Some(Head::Open),
                    Tip::None => None,
                };
                if a == b {
                    let xa = cx[a] + edge(&active[a], 1.0);
                    let pts = [
                        (xa, ly),
                        (xa + 34.0 * s, ly),
                        (xa + 34.0 * s, ly + 22.0 * s),
                        (xa, ly + 22.0 * s),
                    ];
                    body.push(Prim::Path(Path {
                        segs: draw::poly(&pts, false),
                        fill: None,
                        stroke: ink,
                        width: 1.5 * s,
                        dash: msg.dotted,
                    }));
                    if let Some(h) = head {
                        draw::head(&mut body, pts[2], pts[3], h, s);
                    }
                    body.push(Prim::Text(TextPrim {
                        id: t.0,
                        x: xa + 8.0 * s,
                        y: ly - t.2 - 2.0 * s,
                        color: Ink::Role(Role::Text),
                    }));
                    y += 22.0 * s;
                } else {
                    let dir = if b > a { 1.0 } else { -1.0 };
                    let xa = cx[a] + edge(&active[a], dir);
                    let xb = cx[b]
                        - edge(&active[b], dir)
                        - if msg.act == Some(true) {
                            5.0 * s * dir
                        } else {
                            0.0
                        };
                    body.push(Prim::Path(Path {
                        segs: draw::poly(&[(xa, ly), (xb, ly)], false),
                        fill: None,
                        stroke: ink,
                        width: 1.5 * s,
                        dash: msg.dotted,
                    }));
                    if let Some(h) = head {
                        draw::head(&mut body, (xa, ly), (xb, ly), h, s);
                    }
                    body.push(Prim::Text(TextPrim {
                        id: t.0,
                        x: f32::midpoint(xa, xb) - t.1 / 2.0,
                        y: ly - t.2 - 2.0 * s,
                        color: Ink::Role(Role::Text),
                    }));
                }
                if q.autonumber {
                    num += 1;
                    let nt = m.text(&num.to_string(), 10.0 * s, 40.0 * s, true);
                    let r = (nt.1.max(nt.2) / 2.0 + 2.0 * s).max(7.0 * s);
                    late.push(Prim::Path(Path {
                        segs: draw::ellipse(cx[a], ly, r, r),
                        fill: Some(Ink::Role(Role::Edge)),
                        stroke: None,
                        width: 0.0,
                        dash: false,
                    }));
                    late.push(Prim::Text(TextPrim {
                        id: nt.0,
                        x: cx[a] - nt.1 / 2.0,
                        y: ly - nt.2 / 2.0,
                        color: Ink::Role(Role::LabelBg),
                    }));
                }
                match msg.act {
                    Some(true) => active[b].push(ly),
                    Some(false) => {
                        if let Some(y0) = active[a].pop() {
                            bars.push((a, active[a].len(), y0, ly));
                        }
                    }
                    None => {}
                }
                last_y = ly;
                y += 14.0 * s;
            }
            Event::Note { pos, a, b, .. } => {
                let t = t.unwrap_or((0, 0.0, 0.0));
                let (w, h) = (t.1 + 16.0 * s, t.2 + 12.0 * s);
                let (x, lo, hi) = match (pos, b) {
                    (NotePos::Left, _) => (cx[*a] - 10.0 * s - w, *a, *a),
                    (NotePos::Right, _) => (cx[*a] + 10.0 * s, *a, *a),
                    (NotePos::Over, Some(b)) => {
                        let (l, r) = (cx[*a].min(cx[*b]), cx[*a].max(cx[*b]));
                        let w2 = w.max(r - l + 50.0 * s);
                        (f32::midpoint(l, r) - w2 / 2.0, (*a).min(*b), (*a).max(*b))
                    }
                    (NotePos::Over, None) => (cx[*a] - w / 2.0, *a, *a),
                };
                let w = match (pos, b) {
                    (NotePos::Over, Some(b)) => w.max((cx[*a] - cx[*b]).abs() + 50.0 * s),
                    _ => w,
                };
                touch(&mut frames, lo, hi);
                y += 4.0 * s;
                late.push(Prim::Path(Path {
                    segs: draw::rrect(x, y, w, h, 2.0 * s),
                    fill: Some(Ink::Role(Role::NoteFill)),
                    stroke: Some(Ink::Role(Role::NoteStroke)),
                    width: s,
                    dash: false,
                }));
                late.push(Prim::Text(TextPrim {
                    id: t.0,
                    x: x + (w - t.1) / 2.0,
                    y: y + 6.0 * s,
                    color: Ink::Role(Role::Text),
                }));
                y += h + 14.0 * s;
            }
            Event::Activate(i) => {
                active[*i].push(last_y);
            }
            Event::Deactivate(i) => {
                if let Some(y0) = active[*i].pop() {
                    bars.push((*i, active[*i].len(), y0, last_y.max(y0 + 10.0 * s)));
                }
            }
            Event::Start(kind, label) => {
                let _ = t;
                frames.push(OpenFrame {
                    kind: *kind,
                    label: label.clone(),
                    y,
                    lo: usize::MAX,
                    hi: 0,
                    inner: 0,
                    dividers: Vec::new(),
                });
                y += t.map_or(0.0, |t| t.2) + 22.0 * s;
            }
            Event::Divider(label) => {
                y += 4.0 * s;
                if let Some(f) = frames.last_mut() {
                    f.dividers.push((y, label.clone()));
                }
                y += t.map_or(0.0, |t| t.2) + 16.0 * s;
            }
            Event::End => {
                if let Some(f) = frames.pop() {
                    if let Some(p) = frames.last_mut() {
                        p.inner = p.inner.max(f.inner + 1);
                        if f.lo <= f.hi {
                            p.lo = p.lo.min(f.lo);
                            p.hi = p.hi.max(f.hi);
                        }
                    }
                    frame_out.push((f, y));
                }
                y += 14.0 * s;
            }
        }
    }
    y += 10.0 * s;
    let bottom = y;
    // Frames.
    let mut fr: Vec<Prim> = Vec::new();
    for (f, y1) in frame_out {
        let (lo, hi) = if f.lo <= f.hi {
            (f.lo, f.hi)
        } else {
            (0, n.saturating_sub(1))
        };
        if n == 0 {
            break;
        }
        let margin = 14.0 * s + 8.0 * s * f.inner as f32;
        let xa = cx[lo] - pw[lo] / 2.0 - margin + 10.0 * s;
        let xb = cx[hi] + pw[hi] / 2.0 + margin - 10.0 * s;
        fr.push(Prim::Path(Path {
            segs: draw::rrect(xa, f.y, xb - xa, y1 - f.y, 0.0),
            fill: None,
            stroke: Some(Ink::Role(Role::Frame)),
            width: s,
            dash: false,
        }));
        let kw = m.text(f.kind.word(), 12.0 * s, 100.0 * s, true);
        let (tw, th) = (kw.1 + 12.0 * s, kw.2 + 6.0 * s);
        fr.push(Prim::Path(Path {
            segs: vec![
                Seg::M(xa, f.y),
                Seg::L(xa + tw, f.y),
                Seg::L(xa + tw, f.y + th - 5.0 * s),
                Seg::L(xa + tw - 5.0 * s, f.y + th),
                Seg::L(xa, f.y + th),
                Seg::Z,
            ],
            fill: Some(Ink::Role(Role::LabelBg)),
            stroke: Some(Ink::Role(Role::Frame)),
            width: s,
            dash: false,
        }));
        fr.push(Prim::Text(TextPrim {
            id: kw.0,
            x: xa + 6.0 * s,
            y: f.y + 3.0 * s,
            color: Ink::Role(Role::Text),
        }));
        if !f.label.is_empty() {
            let lt = m.text(&format!("[{}]", f.label), 13.0 * s, 300.0 * s, false);
            fr.push(Prim::Text(TextPrim {
                id: lt.0,
                x: xa + tw + 8.0 * s,
                y: f.y + 3.0 * s,
                color: Ink::Role(Role::Text),
            }));
        }
        for (dy, l) in f.dividers {
            fr.push(Prim::Path(Path {
                segs: draw::poly(&[(xa, dy), (xb, dy)], false),
                fill: None,
                stroke: Some(Ink::Role(Role::Frame)),
                width: s,
                dash: true,
            }));
            if !l.is_empty() {
                let lt = m.text(&format!("[{l}]"), 13.0 * s, 300.0 * s, false);
                fr.push(Prim::Text(TextPrim {
                    id: lt.0,
                    x: f32::midpoint(xa, xb) - lt.1 / 2.0,
                    y: dy + 3.0 * s,
                    color: Ink::Role(Role::Text),
                }));
            }
        }
    }
    // Unclosed activations end at the bottom.
    for (i, st) in active.iter_mut().enumerate() {
        while let Some(y0) = st.pop() {
            bars.push((i, st.len(), y0, bottom - 6.0 * s));
        }
    }
    // Paint order: lifelines, frames, bars, messages, notes, heads.
    for i in 0..n {
        prims.push(Prim::Path(Path {
            segs: draw::poly(&[(cx[i], top + ph), (cx[i], bottom)], false),
            fill: None,
            stroke: Some(Ink::Role(Role::Lifeline)),
            width: s,
            dash: true,
        }));
    }
    prims.extend(fr);
    for (i, depth, y0, y1) in bars {
        let x = cx[i] - 5.0 * s + 4.0 * s * depth as f32;
        prims.push(Prim::Path(Path {
            segs: draw::rrect(x, y0, 10.0 * s, (y1 - y0).max(4.0 * s), 0.0),
            fill: Some(Ink::Role(Role::Activation)),
            stroke: Some(Ink::Role(Role::NodeStroke)),
            width: s,
            dash: false,
        }));
    }
    prims.extend(body);
    prims.extend(late);
    let mut boxes = Vec::new();
    for (row, yy) in [(0, top), (1, bottom)] {
        for (i, p) in q.parts.iter().enumerate() {
            let t = heads[i];
            let (x, w) = (cx[i] - pw[i] / 2.0, pw[i]);
            if row == 0 {
                boxes.push((x, yy, w, ph));
            }
            if p.actor {
                let c = cx[i];
                let hy = yy + 7.0 * s;
                prims.push(Prim::Path(Path {
                    segs: draw::ellipse(c, hy, 6.0 * s, 6.0 * s),
                    fill: Some(Ink::Role(Role::NodeFill)),
                    stroke: Some(Ink::Role(Role::NodeStroke)),
                    width: 1.5 * s,
                    dash: false,
                }));
                let mut segs = draw::poly(&[(c, hy + 6.0 * s), (c, hy + 22.0 * s)], false);
                segs.extend(draw::poly(
                    &[(c - 10.0 * s, hy + 11.0 * s), (c + 10.0 * s, hy + 11.0 * s)],
                    false,
                ));
                segs.extend(draw::poly(
                    &[
                        (c - 9.0 * s, hy + 32.0 * s),
                        (c, hy + 22.0 * s),
                        (c + 9.0 * s, hy + 32.0 * s),
                    ],
                    false,
                ));
                prims.push(Prim::Path(Path {
                    segs,
                    fill: None,
                    stroke: Some(Ink::Role(Role::NodeStroke)),
                    width: 1.5 * s,
                    dash: false,
                }));
                prims.push(Prim::Text(TextPrim {
                    id: t.0,
                    x: c - t.1 / 2.0,
                    y: yy + ph - t.2 - 2.0 * s,
                    color: Ink::Role(Role::Text),
                }));
            } else {
                prims.push(Prim::Path(Path {
                    segs: draw::rrect(x, yy, w, ph, 3.0 * s),
                    fill: Some(Ink::Role(Role::NodeFill)),
                    stroke: Some(Ink::Role(Role::NodeStroke)),
                    width: s,
                    dash: false,
                }));
                prims.push(Prim::Text(TextPrim {
                    id: t.0,
                    x: cx[i] - t.1 / 2.0,
                    y: yy + (ph - t.2) / 2.0,
                    color: Ink::Role(Role::Text),
                }));
            }
        }
    }
    // Notes left of the first participant can reach x < 0: shift into view.
    let (mut minx, mut maxx) = (0.0f32, width);
    for p in &prims {
        match p {
            Prim::Path(p) => {
                for sg in &p.segs {
                    if let Seg::M(x, _) | Seg::L(x, _) | Seg::C(_, _, _, _, x, _) = sg {
                        minx = minx.min(*x);
                        maxx = maxx.max(*x);
                    }
                }
            }
            Prim::Text(t) => minx = minx.min(t.x),
        }
    }
    let dx = if minx < 0.0 { -minx + 2.0 } else { 0.0 };
    if dx > 0.0 {
        shift(&mut prims, dx);
        for b in &mut boxes {
            b.0 += dx;
        }
    }
    Drawing {
        w: maxx + dx + 2.0,
        h: bottom + ph + 4.0 * s,
        prims,
        boxes,
        edges: Vec::new(),
        complex: false,
    }
}

fn shift(prims: &mut [Prim], dx: f32) {
    for p in prims {
        match p {
            Prim::Path(p) => {
                for s in &mut p.segs {
                    match s {
                        Seg::M(x, _) | Seg::L(x, _) => *x += dx,
                        Seg::C(a, _, b, _, c, _) => {
                            *a += dx;
                            *b += dx;
                            *c += dx;
                        }
                        Seg::Z => {}
                    }
                }
            }
            Prim::Text(t) => t.x += dx,
        }
    }
}
