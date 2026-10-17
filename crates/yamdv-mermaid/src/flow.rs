//! Flowchart model and parser (`flowchart`/`graph`, FUT-18 subset).

use std::collections::HashMap;

use crate::{Error, MAX_EDGES, MAX_NODES, err};

/// Layout direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dir {
    /// Top to bottom (`TB`/`TD`).
    #[default]
    TB,
    /// Bottom to top.
    BT,
    /// Left to right.
    LR,
    /// Right to left.
    RL,
}

/// Node shape (other Mermaid shapes fall back to [`Shape::Rect`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shape {
    /// `[text]`.
    #[default]
    Rect,
    /// `(text)`.
    Round,
    /// `([text])`.
    Stadium,
    /// `[[text]]`.
    Subroutine,
    /// `[(text)]`.
    Cylinder,
    /// `((text))`.
    Circle,
    /// `{text}`.
    Rhombus,
}

/// Explicit node colors (`style`, `classDef`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Colors {
    /// `fill`.
    pub fill: Option<u32>,
    /// `stroke`.
    pub stroke: Option<u32>,
    /// `color` (text).
    pub color: Option<u32>,
}

impl Colors {
    fn apply(&mut self, o: &Self) {
        self.fill = o.fill.or(self.fill);
        self.stroke = o.stroke.or(self.stroke);
        self.color = o.color.or(self.color);
    }
}

/// A node.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Node {
    /// Id.
    pub id: String,
    /// Label (defaults to the id).
    pub label: String,
    /// Shape.
    pub shape: Shape,
    /// Classes (`:::`, `class`).
    pub classes: Vec<String>,
    /// `style` colors.
    pub style: Colors,
    /// Innermost subgraph.
    pub sub: Option<usize>,
}

/// Edge line kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Line {
    /// `--`.
    Solid,
    /// `-.-`.
    Dotted,
    /// `==`.
    Thick,
}

/// An edge.
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    /// Source node index.
    pub from: usize,
    /// Target node index.
    pub to: usize,
    /// Line kind.
    pub line: Line,
    /// Arrowhead at the target.
    pub arrow: bool,
    /// Label.
    pub label: Option<String>,
}

/// A subgraph.
#[derive(Debug, Clone, PartialEq)]
pub struct Subgraph {
    /// Id.
    pub id: String,
    /// Title.
    pub title: String,
    /// Parent subgraph.
    pub parent: Option<usize>,
}

/// A parsed flowchart.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Flow {
    /// Direction.
    pub dir: Dir,
    /// Nodes.
    pub nodes: Vec<Node>,
    /// Edges.
    pub edges: Vec<Edge>,
    /// Subgraphs.
    pub subs: Vec<Subgraph>,
    /// `classDef`s.
    pub class_defs: HashMap<String, Colors>,
}

impl Flow {
    /// Effective colors of node `i` (classes in order, then `style`).
    pub fn colors(&self, i: usize) -> Colors {
        let mut c = Colors::default();
        if let Some(n) = self.nodes.get(i) {
            if let Some(d) = self.class_defs.get("default") {
                c.apply(d);
            }
            for k in &n.classes {
                if let Some(d) = self.class_defs.get(k) {
                    c.apply(d);
                }
            }
            c.apply(&n.style);
        }
        c
    }
}

fn props(s: &str) -> Colors {
    let mut c = Colors::default();
    for p in s.split([',', ';']) {
        let Some((k, v)) = p.split_once(':') else {
            continue;
        };
        let v = crate::color(v);
        match k.trim() {
            "fill" => c.fill = v,
            "stroke" => c.stroke = v,
            "color" => c.color = v,
            _ => {}
        }
    }
    c
}

struct P<'a> {
    f: Flow,
    ids: HashMap<String, usize>,
    stack: Vec<usize>,
    line: usize,
    chars: Vec<char>,
    pos: usize,
    _src: &'a str,
}

fn is_id(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

impl P<'_> {
    fn e(&self, msg: impl Into<String>) -> Error {
        err(msg, self.line, self.pos)
    }
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }
    fn at(&self, s: &str) -> bool {
        s.chars()
            .enumerate()
            .all(|(i, c)| self.chars.get(self.pos + i) == Some(&c))
    }
    fn ws(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.pos += 1;
        }
    }
    fn rest(&self) -> String {
        self.chars.get(self.pos..).unwrap_or(&[]).iter().collect()
    }

    fn node(&mut self, id: &str) -> Result<usize, Error> {
        if let Some(i) = self.ids.get(id) {
            let i = *i;
            if let (Some(&s), Some(n)) = (self.stack.last(), self.f.nodes.get_mut(i))
                && n.sub.is_none()
            {
                n.sub = Some(s);
            }
            return Ok(i);
        }
        if self.f.nodes.len() >= MAX_NODES {
            return Err(self.e("Too many nodes"));
        }
        self.f.nodes.push(Node {
            id: id.to_string(),
            label: id.to_string(),
            sub: self.stack.last().copied(),
            ..Node::default()
        });
        let i = self.f.nodes.len() - 1;
        self.ids.insert(id.to_string(), i);
        Ok(i)
    }

    /// Reads text up to `close` (quoted text may contain it).
    fn text_until(&mut self, close: &str) -> Result<String, Error> {
        let start = self.pos;
        self.ws();
        let mut s = String::new();
        if self.peek() == Some('"') {
            self.pos += 1;
            loop {
                match self.peek() {
                    None => return Err(self.e("Unterminated string")),
                    Some('"') => {
                        self.pos += 1;
                        break;
                    }
                    Some(c) => {
                        s.push(c);
                        self.pos += 1;
                    }
                }
            }
            self.ws();
            if !self.at(close) {
                return Err(self.e(format!("Expected '{close}'")));
            }
        } else {
            self.pos = start;
            while !self.at(close) {
                let Some(c) = self.peek() else {
                    self.pos = start;
                    return Err(self.e(format!("Expected '{close}'")));
                };
                s.push(c);
                self.pos += 1;
            }
        }
        self.pos += close.chars().count();
        Ok(crate::label(&s))
    }

    /// `id[shape text]:::class`
    fn node_ref(&mut self) -> Result<usize, Error> {
        self.ws();
        let start = self.pos;
        while self.peek().is_some_and(is_id)
            || (self.peek() == Some('-') && self.chars.get(self.pos + 1).is_some_and(|c| is_id(*c)))
        {
            self.pos += 1;
        }
        if self.pos == start {
            return Err(self.e("Expected a node id"));
        }
        let id: String = self
            .chars
            .get(start..self.pos)
            .unwrap_or(&[])
            .iter()
            .collect();
        let i = self.node(&id)?;
        const SHAPES: [(&str, &str, Shape); 13] = [
            ("([", "])", Shape::Stadium),
            ("[[", "]]", Shape::Subroutine),
            ("[(", ")]", Shape::Cylinder),
            ("(((", ")))", Shape::Circle),
            ("((", "))", Shape::Circle),
            ("{{", "}}", Shape::Rect),
            ("[/", "/]", Shape::Rect),
            ("[\\", "\\]", Shape::Rect),
            ("[/", "\\]", Shape::Rect),
            (">", "]", Shape::Rect),
            ("[", "]", Shape::Rect),
            ("(", ")", Shape::Round),
            ("{", "}", Shape::Rhombus),
        ];
        for (open, close, shape) in SHAPES {
            if self.at(open) {
                self.pos += open.chars().count();
                let save = self.pos;
                let t = match self.text_until(close) {
                    Ok(t) => t,
                    Err(e) if open == "[/" => {
                        self.pos = save;
                        self.text_until("\\]").map_err(|_| e)?
                    }
                    Err(e) => return Err(e),
                };
                if let Some(n) = self.f.nodes.get_mut(i) {
                    n.label = t;
                    n.shape = shape;
                }
                break;
            }
        }
        if self.at(":::") {
            self.pos += 3;
            let s = self.pos;
            while self.peek().is_some_and(|c| is_id(c) || c == '-') {
                self.pos += 1;
            }
            let c: String = self.chars.get(s..self.pos).unwrap_or(&[]).iter().collect();
            if let Some(n) = self.f.nodes.get_mut(i) {
                n.classes.push(c);
            }
        }
        Ok(i)
    }

    fn group(&mut self) -> Result<Vec<usize>, Error> {
        let mut v = vec![self.node_ref()?];
        loop {
            self.ws();
            if self.peek() == Some('&') {
                self.pos += 1;
                v.push(self.node_ref()?);
            } else {
                return Ok(v);
            }
        }
    }

    fn count(&mut self, c: char) -> usize {
        let mut n = 0;
        while self.peek() == Some(c) {
            self.pos += 1;
            n += 1;
        }
        n
    }

    /// An edge operator, or `None` at end of statement.
    fn edge_op(&mut self) -> Result<Option<(Line, bool, Option<String>)>, Error> {
        self.ws();
        if self.peek().is_none() {
            return Ok(None);
        }
        if self.peek() == Some('<') {
            self.pos += 1;
        }
        let (line, arrow, mut label) = if self.at("-.") {
            self.pos += 1;
            self.count('.');
            if self.at("->") {
                self.pos += 2;
                (Line::Dotted, true, None)
            } else if self.peek() == Some('-') {
                self.pos += 1;
                (Line::Dotted, false, None)
            } else {
                // `-. text .->`
                let t = self.labeled(&[(".->", true), (".-", false)])?;
                (Line::Dotted, t.1, Some(t.0))
            }
        } else if self.at("==") {
            let n = self.count('=');
            if self.peek() == Some('>') {
                self.pos += 1;
                (Line::Thick, true, None)
            } else if n >= 3 {
                (Line::Thick, false, None)
            } else {
                let t = self.labeled(&[("==>", true), ("===", false)])?;
                (Line::Thick, t.1, Some(t.0))
            }
        } else if self.at("--") {
            let n = self.count('-');
            if self.peek() == Some('>') {
                self.pos += 1;
                (Line::Solid, true, None)
            } else if n >= 3 {
                (Line::Solid, false, None)
            } else {
                let t = self.labeled(&[("-->", true), ("---", false)])?;
                (Line::Solid, t.1, Some(t.0))
            }
        } else {
            return Err(self.e(format!("Unexpected '{}'", self.peek().unwrap_or(' '))));
        };
        self.ws();
        if self.peek() == Some('|') {
            self.pos += 1;
            label = Some(self.text_until("|")?);
        }
        Ok(Some((line, arrow, label)))
    }

    fn labeled(&mut self, ends: &[(&str, bool)]) -> Result<(String, bool), Error> {
        let start = self.pos;
        let mut s = String::new();
        loop {
            for (e, arrow) in ends {
                if self.at(e) {
                    self.pos += e.chars().count();
                    if *arrow || matches!(e.chars().last(), Some('-' | '=')) {
                        // Extra dashes of a longer operator.
                        while matches!(self.peek(), Some('-' | '=')) {
                            self.pos += 1;
                        }
                        if self.peek() == Some('>') {
                            self.pos += 1;
                            return Ok((crate::label(&s), true));
                        }
                    }
                    return Ok((crate::label(&s), *arrow));
                }
            }
            let Some(c) = self.peek() else {
                self.pos = start;
                return Err(self.e("Unterminated edge label"));
            };
            s.push(c);
            self.pos += 1;
        }
    }

    fn statement(&mut self, st: &str) -> Result<(), Error> {
        let t = st.trim();
        let word = t.split_whitespace().next().unwrap_or("");
        let after = t.get(word.len()..).unwrap_or("").trim();
        match word {
            "subgraph" => {
                let (id, title) = if let Some(b) = after.find('[') {
                    let id = after.get(..b).unwrap_or("").trim();
                    let title = after
                        .get(b + 1..)
                        .unwrap_or("")
                        .trim_end()
                        .trim_end_matches(']');
                    (id.to_string(), crate::label(title))
                } else {
                    let l = crate::label(after);
                    (l.clone(), l)
                };
                if id.is_empty() && title.is_empty() {
                    return Err(self.e("Subgraph needs a title"));
                }
                self.f.subs.push(Subgraph {
                    id,
                    title,
                    parent: self.stack.last().copied(),
                });
                self.stack.push(self.f.subs.len() - 1);
                return Ok(());
            }
            "end" if after.is_empty() => {
                if self.stack.pop().is_none() {
                    return Err(self.e("'end' without 'subgraph'"));
                }
                return Ok(());
            }
            "direction" | "click" | "linkStyle" | "callback" => return Ok(()),
            "classDef" => {
                let mut it = after.splitn(2, char::is_whitespace);
                let names = it.next().unwrap_or("");
                let p = props(it.next().unwrap_or(""));
                for n in names.split(',') {
                    self.f.class_defs.insert(n.trim().to_string(), p);
                }
                return Ok(());
            }
            "class" => {
                let mut it = after.rsplitn(2, char::is_whitespace);
                let cls = it.next().unwrap_or("").to_string();
                for id in it.next().unwrap_or("").split(',') {
                    let i = self.node(id.trim())?;
                    if let Some(n) = self.f.nodes.get_mut(i) {
                        n.classes.push(cls.clone());
                    }
                }
                return Ok(());
            }
            "style" => {
                let mut it = after.splitn(2, char::is_whitespace);
                let id = it.next().unwrap_or("");
                let p = props(it.next().unwrap_or(""));
                let i = self.node(id)?;
                if let Some(n) = self.f.nodes.get_mut(i) {
                    n.style.apply(&p);
                }
                return Ok(());
            }
            _ => {}
        }
        let mut left = self.group()?;
        while let Some((line, arrow, label)) = self.edge_op()? {
            let right = self.group()?;
            for &a in &left {
                for &b in &right {
                    if self.f.edges.len() >= MAX_EDGES {
                        return Err(self.e("Too many edges"));
                    }
                    self.f.edges.push(Edge {
                        from: a,
                        to: b,
                        line,
                        arrow,
                        label: label.clone(),
                    });
                }
            }
            left = right;
        }
        if self.pos < self.chars.len() {
            return Err(self.e(format!("Unexpected '{}'", self.rest().trim())));
        }
        Ok(())
    }
}

/// Splits a line at `;` outside brackets and quotes; returns (char offset, statement).
fn split_statements(l: &str) -> Vec<(usize, String)> {
    let mut v = Vec::new();
    let (mut depth, mut quote, mut cur, mut start) = (0i32, false, String::new(), 0usize);
    for (i, c) in l.chars().enumerate() {
        match c {
            '"' => quote = !quote,
            '[' | '(' | '{' if !quote => depth += 1,
            ']' | ')' | '}' if !quote => depth -= 1,
            ';' if !quote && depth <= 0 => {
                v.push((start, std::mem::take(&mut cur)));
                start = i + 1;
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    v.push((start, cur));
    v
}

/// Parses a flowchart.
pub fn parse(src: &str) -> Result<Flow, Error> {
    let mut p = P {
        f: Flow::default(),
        ids: HashMap::new(),
        stack: Vec::new(),
        line: 0,
        chars: Vec::new(),
        pos: 0,
        _src: src,
    };
    let mut header = true;
    let mut open_line = Vec::new();
    for (ln, l) in crate::lines(src) {
        p.line = ln;
        if header {
            header = false;
            let mut w = l.split_whitespace().skip(1);
            p.f.dir = match w.next().map(|d| d.trim_end_matches(';')) {
                None | Some("TB" | "TD") => Dir::TB,
                Some("BT") => Dir::BT,
                Some("LR") => Dir::LR,
                Some("RL") => Dir::RL,
                Some(_) => {
                    let col = l.find(char::is_whitespace).unwrap_or(0) + 1;
                    return Err(err("Unknown direction", ln, col));
                }
            };
            continue;
        }
        for (off, st) in split_statements(l) {
            if st.trim().is_empty() || st.trim_start().starts_with("%%") {
                continue;
            }
            let depth = p.stack.len();
            p.chars = st.chars().collect();
            p.pos = 0;
            p.ws();
            p.statement(&st).map_err(|mut e| {
                e.col = e.col.saturating_add(u32::try_from(off).unwrap_or(0));
                e
            })?;
            if p.stack.len() > depth {
                open_line.push(ln);
            } else if p.stack.len() < depth {
                open_line.pop();
            }
        }
    }
    if let Some(ln) = open_line.last() {
        return Err(err("Unclosed subgraph", *ln, 0));
    }
    Ok(p.f)
}
