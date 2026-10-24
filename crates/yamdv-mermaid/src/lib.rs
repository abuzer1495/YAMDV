//! Native Mermaid (R-MMD-01…07, FUT-18): bounded parsers for `flowchart`/`graph`,
//! `sequenceDiagram`, `gantt`, `pie`, `stateDiagram`, `classDiagram`, `erDiagram` and `mindmap`,
//! a layered (Sugiyama) layout shared by the graph types, the `init` theme directive, and display
//! primitives the renderer draws (vector paths and measured text).
//!
//! Platform-independent (pure) crate; see `docs/ARCHITECTURE.md` §E.2. Text is measured through
//! [`Measure`] (the renderer's `TextShaper`).
#![forbid(unsafe_code)]

pub mod class;
pub mod draw;
pub mod flow;
pub mod gantt;
mod graph;
mod layered;
pub mod mindmap;
pub mod pie;
pub mod seq;
pub mod theme;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_types;

pub use draw::{Drawing, Ink, Measure, Path, Prim, Role, Seg, TextPrim};

/// Largest diagram source accepted (bytes, S-PARSE-06).
pub const MAX_SRC: usize = 100 * 1024;
/// Parser cap on nodes / participants (above the renderer's hard limit; bounds memory).
pub const MAX_NODES: usize = 10_000;
/// Parser cap on edges / messages / notes.
pub const MAX_EDGES: usize = 30_000;

/// A syntax error with its 1-based position in the diagram source (R-MMD-04).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// Message (English; shown under the source).
    pub msg: String,
    /// Line (1-based).
    pub line: u32,
    /// Column (1-based, in characters).
    pub col: u32,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}, column {}: {}", self.line, self.col, self.msg)
    }
}

pub(crate) fn err(msg: impl Into<String>, line: usize, col: usize) -> Error {
    Error {
        msg: msg.into(),
        line: u32::try_from(line + 1).unwrap_or(u32::MAX),
        col: u32::try_from(col + 1).unwrap_or(u32::MAX),
    }
}

/// A parsed diagram.
#[derive(Debug, Clone, PartialEq)]
pub enum Diagram {
    /// `flowchart` / `graph`.
    Flow(flow::Flow),
    /// `sequenceDiagram`.
    Seq(seq::Seq),
    /// `stateDiagram(-v2)`, `classDiagram`, `erDiagram` (layered like a flowchart).
    Graph(flow::Flow),
    /// `gantt`.
    Gantt(gantt::Gantt),
    /// `pie`.
    Pie(pie::Pie),
    /// `mindmap`.
    Mind(mindmap::Mind),
}

/// Why a diagram did not parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// Another diagram type (gantt, pie, class, …): shown as a code block (FUT-18).
    Unsupported(String),
    /// Syntax error.
    Syntax(Error),
}

/// Source lines without `%%` comment lines (index = 0-based line number).
pub(crate) fn lines(src: &str) -> impl Iterator<Item = (usize, &str)> {
    src.lines().enumerate().filter(|(_, l)| {
        let t = l.trim_start();
        !t.is_empty() && !t.starts_with("%%")
    })
}

/// Parses a ```` ```mermaid ```` block.
pub fn parse(src: &str) -> Result<Diagram, Failure> {
    if src.len() > MAX_SRC {
        return Err(Failure::Syntax(err("Diagram source too large", 0, 0)));
    }
    let Some((_, first)) = lines(src).next() else {
        return Err(Failure::Syntax(err("Empty diagram", 0, 0)));
    };
    let kw = first.split_whitespace().next().unwrap_or("");
    match kw {
        "flowchart" | "graph" | "flowchart-elk" => {
            flow::parse(src).map(Diagram::Flow).map_err(Failure::Syntax)
        }
        "sequenceDiagram" => seq::parse(src).map(Diagram::Seq).map_err(Failure::Syntax),
        "stateDiagram" | "stateDiagram-v2" => graph::parse_state(src)
            .map(Diagram::Graph)
            .map_err(Failure::Syntax),
        "classDiagram" | "classDiagram-v2" => class::parse_class(src)
            .map(Diagram::Graph)
            .map_err(Failure::Syntax),
        "erDiagram" => class::parse_er(src)
            .map(Diagram::Graph)
            .map_err(Failure::Syntax),
        "gantt" => gantt::parse(src)
            .map(Diagram::Gantt)
            .map_err(Failure::Syntax),
        "pie" => pie::parse(src).map(Diagram::Pie).map_err(Failure::Syntax),
        "mindmap" => mindmap::parse(src)
            .map(Diagram::Mind)
            .map_err(Failure::Syntax),
        k => Err(Failure::Unsupported(k.trim_end_matches(':').to_string())),
    }
}

impl Diagram {
    /// Node and edge counts for the limits (S-PARSE-06): flowchart nodes/edges, sequence
    /// participants/(messages + notes).
    pub fn counts(&self) -> (usize, usize) {
        match self {
            Self::Flow(f) | Self::Graph(f) => (f.nodes.len(), f.edges.len()),
            Self::Seq(s) => (s.parts.len(), s.events.len()),
            Self::Gantt(g) => (g.tasks.len(), 0),
            Self::Pie(p) => (p.slices.len(), 0),
            Self::Mind(m) => (m.items.len() + 1, m.items.len()),
        }
    }

    /// Lays the diagram out at `scale` (1.0 = 14-px text, 40/50-px separations).
    pub fn layout(&self, m: &mut dyn Measure, scale: f32) -> Drawing {
        self.layout_at(m, scale, None)
    }

    /// [`Self::layout`] with the local wall clock (s since 1970, no zone) for the gantt today
    /// marker.
    pub fn layout_at(&self, m: &mut dyn Measure, scale: f32, now: Option<i64>) -> Drawing {
        match self {
            Self::Flow(f) | Self::Graph(f) => layered::layout(f, m, scale),
            Self::Seq(s) => seq::layout(s, m, scale),
            Self::Gantt(g) => gantt::layout(g, m, scale, now),
            Self::Pie(p) => pie::layout(p, m, scale),
            Self::Mind(d) => mindmap::layout(d, m, scale),
        }
    }
}

/// Parses a color value of `style`/`classDef` (`#rgb`, `#rrggbb`, a few CSS names).
pub(crate) fn color(v: &str) -> Option<u32> {
    let v = v.trim();
    if let Some(h) = v.strip_prefix('#') {
        let h: Vec<u32> = h.chars().map(|c| c.to_digit(16)).collect::<Option<_>>()?;
        return match h.as_slice() {
            [r, g, b] => Some((r * 17) << 16 | (g * 17) << 8 | (b * 17)),
            [r1, r2, g1, g2, b1, b2] | [r1, r2, g1, g2, b1, b2, _, _] => {
                Some((r1 << 4 | r2) << 16 | (g1 << 4 | g2) << 8 | (b1 << 4 | b2))
            }
            _ => None,
        };
    }
    Some(match v.to_ascii_lowercase().as_str() {
        "black" => 0x000000,
        "white" => 0xFFFFFF,
        "red" => 0xFF0000,
        "green" => 0x008000,
        "blue" => 0x0000FF,
        "yellow" => 0xFFFF00,
        "orange" => 0xFFA500,
        "purple" => 0x800080,
        "pink" => 0xFFC0CB,
        "gray" | "grey" => 0x808080,
        "lightgray" | "lightgrey" => 0xD3D3D3,
        "lightblue" => 0xADD8E6,
        "lightgreen" => 0x90EE90,
        _ => return None,
    })
}

/// Label text: `<br>` variants become line breaks, quotes are stripped.
pub(crate) fn label(s: &str) -> String {
    let s = s.trim();
    let s = s
        .strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .unwrap_or(s);
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('<') {
        out.push_str(rest.get(..i).unwrap_or(""));
        let tail = rest.get(i..).unwrap_or("");
        let low = tail.get(..6).unwrap_or(tail).to_ascii_lowercase();
        if low.starts_with("<br>") {
            out.push('\n');
            rest = tail.get(4..).unwrap_or("");
        } else if low.starts_with("<br/>") {
            out.push('\n');
            rest = tail.get(5..).unwrap_or("");
        } else if low.starts_with("<br />") {
            out.push('\n');
            rest = tail.get(6..).unwrap_or("");
        } else {
            out.push('<');
            rest = tail.get(1..).unwrap_or("");
        }
    }
    out.push_str(rest);
    out
}
