//! Graph-shaped diagrams mapped onto a [`Flow`] so the layered layout is reused (FUT-18
//! SHOULD): `stateDiagram(-v2)` here, `classDiagram` / `erDiagram` in [`crate::class`].

use std::collections::HashMap;

use crate::flow::{Colors, Dir, Edge, End, Flow, Line, Node, Shape, Subgraph, props};
use crate::{Error, MAX_EDGES, MAX_NODES, err};

/// Builds a [`Flow`] by id.
pub(crate) struct Builder {
    pub f: Flow,
    ids: HashMap<String, usize>,
    pub stack: Vec<usize>,
    pub line: usize,
}

impl Builder {
    pub fn new(dir: Dir) -> Self {
        Self {
            f: Flow {
                dir,
                ..Flow::default()
            },
            ids: HashMap::new(),
            stack: Vec::new(),
            line: 0,
        }
    }

    pub fn e(&self, m: impl Into<String>) -> Error {
        err(m, self.line, 0)
    }

    /// The node `id`, created with `shape` and label = id on first use.
    pub fn node(&mut self, id: &str, shape: Shape) -> Result<usize, Error> {
        if let Some(&i) = self.ids.get(id) {
            return Ok(i);
        }
        if self.f.nodes.len() >= MAX_NODES {
            return Err(self.e("Too many nodes"));
        }
        self.f.nodes.push(Node {
            id: id.to_string(),
            label: id.to_string(),
            shape,
            sub: self.stack.last().copied(),
            ..Node::default()
        });
        let i = self.f.nodes.len() - 1;
        self.ids.insert(id.to_string(), i);
        Ok(i)
    }

    pub fn get(&mut self, i: usize) -> Option<&mut Node> {
        self.f.nodes.get_mut(i)
    }

    pub fn edge(&mut self, e: Edge) -> Result<(), Error> {
        if self.f.edges.len() >= MAX_EDGES {
            return Err(self.e("Too many edges"));
        }
        self.f.edges.push(e);
        Ok(())
    }

    pub fn open(&mut self, id: &str, title: &str) {
        self.f.subs.push(Subgraph {
            id: id.to_string(),
            title: title.to_string(),
            parent: self.stack.last().copied(),
        });
        self.stack.push(self.f.subs.len() - 1);
    }

    /// Shared statements: `direction`, `classDef`, `class a,b name`, `style id …`.
    pub fn common(&mut self, l: &str) -> Result<bool, Error> {
        let (kw, after) = l.split_once(char::is_whitespace).unwrap_or((l, ""));
        let after = after.trim();
        match kw {
            "direction" => {
                self.f.dir = match after {
                    "LR" => Dir::LR,
                    "RL" => Dir::RL,
                    "BT" => Dir::BT,
                    _ => Dir::TB,
                };
            }
            "classDef" => {
                let (names, p) = after.split_once(char::is_whitespace).unwrap_or((after, ""));
                let p = props(p);
                for n in names.split(',') {
                    self.f.class_defs.insert(n.trim().to_string(), p);
                }
            }
            "class" | "cssClass" if after.contains(char::is_whitespace) => {
                let (ids, cls) = after
                    .rsplit_once(char::is_whitespace)
                    .unwrap_or((after, ""));
                let cls = cls.trim_matches('"').to_string();
                for id in ids.trim_matches('"').split(',') {
                    if let Some(&i) = self.ids.get(id.trim())
                        && let Some(n) = self.f.nodes.get_mut(i)
                    {
                        n.classes.push(cls.clone());
                    }
                }
            }
            "style" => {
                let (id, p) = after.split_once(char::is_whitespace).unwrap_or((after, ""));
                let p = props(p);
                if let Some(&i) = self.ids.get(id)
                    && let Some(n) = self.f.nodes.get_mut(i)
                {
                    n.style = p;
                }
            }
            "accTitle" | "accDescr" | "click" | "link" | "callback" | "title" => {}
            _ => return Ok(false),
        }
        Ok(true)
    }
}

/// Splits `id:::cls`.
pub(crate) fn class_suffix(s: &str) -> (&str, Option<&str>) {
    match s.split_once(":::") {
        Some((a, b)) => (a.trim(), Some(b.trim())),
        None => (s.trim(), None),
    }
}

/// Note colors (state notes are ordinary nodes linked by a dotted line).
const NOTE: Colors = Colors {
    fill: Some(0xFFF8C5),
    stroke: Some(0xE3C34D),
    color: Some(0x333333),
    width: None,
    dash: None,
};

/// Parses a `stateDiagram` / `stateDiagram-v2` block.
pub fn parse_state(src: &str) -> Result<Flow, Error> {
    let mut b = Builder::new(Dir::TB);
    let mut note: Option<(usize, String)> = None;
    let mut notes = 0usize;
    for (ln, line) in crate::lines(src).skip(1) {
        b.line = ln;
        let l = line.trim();
        if let Some((target, text)) = &mut note {
            if l == "end note" {
                let (t, x) = (*target, std::mem::take(text));
                add_note(&mut b, t, &x, &mut notes)?;
                note = None;
            } else {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(l);
            }
            continue;
        }
        if l == "}" {
            if b.stack.pop().is_none() {
                return Err(b.e("'}' without 'state … {'"));
            }
            continue;
        }
        if l == "--" || b.common(l)? {
            continue;
        }
        if let Some(r) = l.strip_prefix("note ") {
            // note left of|right of X : text   /   multi-line … end note
            let r = r.trim();
            let r = r
                .strip_prefix("left of")
                .or_else(|| r.strip_prefix("right of"))
                .ok_or_else(|| b.e("Expected 'note left of' or 'note right of'"))?
                .trim();
            let (id, text) = r.split_once(':').map_or((r, None), |(a, t)| (a, Some(t)));
            let t = b.node(id.trim(), Shape::Round)?;
            match text {
                Some(x) => add_note(&mut b, t, x.trim(), &mut notes)?,
                None => note = Some((t, String::new())),
            }
            continue;
        }
        if let Some(r) = l.strip_prefix("state ") {
            let r = r.trim();
            let (r, open) = match r.strip_suffix('{') {
                Some(x) => (x.trim(), true),
                None => (r, false),
            };
            // state "Label" as Id   /   state Id <<fork|join|choice>>   /   state Id {
            let (id, label, kind) = if let Some(q) = r.strip_prefix('"') {
                let (lab, rest) = q.split_once('"').ok_or_else(|| b.e("Unclosed quote"))?;
                let id = rest.trim().strip_prefix("as").map_or(lab, str::trim);
                (id, lab, None)
            } else if let Some((id, k)) = r.split_once("<<") {
                (id.trim(), id.trim(), Some(k.trim_end_matches(">>").trim()))
            } else {
                let (id, lab) = r
                    .split_once(':')
                    .map_or((r, r), |(a, b)| (a.trim(), b.trim()));
                (id, lab, None)
            };
            let shape = match kind {
                Some("fork" | "join") => Shape::Start,
                Some("choice") => Shape::Rhombus,
                _ => Shape::Round,
            };
            if open {
                b.open(id, label);
            } else {
                let i = b.node(id, shape)?;
                if let Some(n) = b.get(i) {
                    n.shape = shape;
                    n.label = if kind.is_some() {
                        String::new()
                    } else {
                        label.to_string()
                    };
                }
            }
            continue;
        }
        if let Some((a, rest)) = l.split_once("-->") {
            let (rhs, label) = rest
                .split_once(':')
                .map_or((rest, None), |(x, y)| (x, Some(y.trim().to_string())));
            let from = state_ref(&mut b, a.trim(), true)?;
            let to = state_ref(&mut b, rhs.trim(), false)?;
            b.edge(Edge {
                label: label.filter(|s| !s.is_empty()),
                ..Edge::new(from, to, Line::Solid, End::Arrow)
            })?;
            continue;
        }
        if let Some((id, desc)) = l.split_once(':') {
            let (id, cls) = class_suffix(id);
            let i = b.node(id, Shape::Round)?;
            if let Some(n) = b.get(i) {
                n.label = desc.trim().to_string();
                n.classes.extend(cls.map(str::to_string));
            }
            continue;
        }
        let (id, cls) = class_suffix(l);
        if id.is_empty() || id.contains(char::is_whitespace) {
            return Err(b.e(format!("Unexpected state line \"{l}\"")));
        }
        let i = b.node(id, Shape::Round)?;
        if let Some(n) = b.get(i) {
            n.classes.extend(cls.map(str::to_string));
        }
    }
    if note.is_some() {
        return Err(b.e("'note' without 'end note'"));
    }
    if !b.stack.is_empty() {
        return Err(b.e("Unclosed 'state … {'"));
    }
    Ok(b.f)
}

/// `[*]` is a start (source side) or end (target side) pseudo-state per composite scope.
fn state_ref(b: &mut Builder, s: &str, source: bool) -> Result<usize, Error> {
    let (s, cls) = class_suffix(s);
    if s == "[*]" {
        let scope = b.stack.last().map_or(String::new(), |&k| k.to_string());
        let (id, shape) = if source {
            (format!("\u{0}start{scope}"), Shape::Start)
        } else {
            (format!("\u{0}end{scope}"), Shape::End)
        };
        let i = b.node(&id, shape)?;
        if let Some(n) = b.get(i) {
            n.label.clear();
        }
        return Ok(i);
    }
    let i = b.node(s, Shape::Round)?;
    if let Some(n) = b.get(i) {
        n.classes.extend(cls.map(str::to_string));
    }
    Ok(i)
}

fn add_note(b: &mut Builder, target: usize, text: &str, k: &mut usize) -> Result<(), Error> {
    *k += 1;
    let i = b.node(&format!("\u{0}note{k}"), Shape::Rect)?;
    if let Some(n) = b.get(i) {
        n.label = text.to_string();
        n.style = NOTE;
    }
    b.edge(Edge::new(target, i, Line::Dotted, End::None))
}
