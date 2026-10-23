//! `%%{init: {"theme": …, "themeVariables": {…}}}%%` (R-MMD-03 SHOULD): the five Mermaid themes
//! and the variable subset, applied to a laid-out [`Drawing`] by replacing theme roles.

use crate::draw::{Drawing, Ink, Prim, Role};

/// A parsed `init` directive.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Init {
    /// Role colors to use instead of the app palette.
    pub roles: Vec<(Role, u32)>,
    /// `fontSize` relative to the 14 px default.
    pub font_scale: Option<f32>,
}

const VARS: [(&str, &[Role]); 6] = [
    ("primaryColor", &[Role::NodeFill]),
    ("primaryTextColor", &[Role::Text]),
    ("primaryBorderColor", &[Role::NodeStroke]),
    ("lineColor", &[Role::Edge, Role::Lifeline]),
    ("secondaryColor", &[Role::NoteFill, Role::Activation]),
    ("tertiaryColor", &[Role::ClusterFill]),
];

fn theme(name: &str) -> Option<Vec<(Role, u32)>> {
    use Role::{
        Activation, ClusterFill, ClusterStroke, Edge, Frame, Lifeline, NodeFill, NodeStroke,
        NoteFill, NoteStroke, Text,
    };
    // (node fill, node stroke, text, line, cluster fill, cluster stroke, note fill, note stroke)
    let t: [u32; 8] = match name {
        "default" => return Some(Vec::new()),
        "neutral" => [
            0xEEEEEE, 0x999999, 0x333333, 0x666666, 0xF5F5F5, 0xAAAAAA, 0xEAEAEA, 0x999999,
        ],
        "dark" => [
            0x1F2020, 0xCCCCCC, 0xE0E0E0, 0xD3D3D3, 0x333333, 0x888888, 0x444444, 0x999999,
        ],
        "forest" => [
            0xCDE498, 0x13540C, 0x000000, 0x008000, 0xCDFFB2, 0x6EAA49, 0xFFF5AD, 0xAAAA33,
        ],
        "base" => [
            0xFFF4DD, 0xD9A45A, 0x333333, 0x333333, 0xFFFFDE, 0xAAAA33, 0xFFF5AD, 0xAAAA33,
        ],
        _ => return None,
    };
    Some(vec![
        (NodeFill, t[0]),
        (NodeStroke, t[1]),
        (Text, t[2]),
        (Edge, t[3]),
        (Lifeline, t[3]),
        (Frame, t[3]),
        (ClusterFill, t[4]),
        (ClusterStroke, t[5]),
        (NoteFill, t[6]),
        (NoteStroke, t[7]),
        (Activation, t[4]),
    ])
}

/// The value of `key` in a loose JSON object (`"k": "v"`, `'k':'v'`, `k: 12`).
fn value_of(text: &str, key: &str) -> Option<String> {
    for (at, _) in text.match_indices(key) {
        let before = text.get(..at).and_then(|b| b.chars().next_back());
        if before.is_some_and(|c| c.is_alphanumeric()) {
            continue;
        }
        let rest = text.get(at + key.len()..)?;
        let rest = rest.trim_start_matches(['"', '\'']).trim_start();
        let Some(rest) = rest.strip_prefix(':') else {
            continue;
        };
        let rest = rest.trim_start();
        let q = rest.chars().next()?;
        if q == '"' || q == '\'' {
            let body = rest.get(1..)?;
            return body.find(q).and_then(|e| body.get(..e)).map(str::to_string);
        }
        let end = rest
            .find(|c: char| c == ',' || c == '}' || c.is_whitespace())
            .unwrap_or(rest.len());
        return rest.get(..end).map(str::to_string);
    }
    None
}

/// Parses the first `%%{init: …}%%` (or `%%{initialize: …}%%`) directive of `src`.
pub fn init(src: &str) -> Option<Init> {
    let line = src
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("%%{") && l.contains("init"))?;
    let body = line.get(3..)?.trim_end_matches("%%").trim_end_matches('}');
    let mut out = Init::default();
    if let Some(t) = value_of(body, "theme") {
        out.roles = theme(t.trim()).unwrap_or_default();
    }
    for (var, roles) in VARS {
        if let Some(c) = value_of(body, var).and_then(|v| crate::color(&v)) {
            for r in roles {
                out.roles.retain(|(x, _)| x != r);
                out.roles.push((*r, c));
            }
        }
    }
    if let Some(f) = value_of(body, "fontSize") {
        out.font_scale = f
            .trim_end_matches("px")
            .trim()
            .parse::<f32>()
            .ok()
            .filter(|v| v.is_finite())
            .map(|v| (v / 14.0).clamp(0.5, 3.0));
    }
    Some(out)
}

impl Init {
    fn ink(&self, i: Ink) -> Ink {
        match i {
            Ink::Role(r) => self
                .roles
                .iter()
                .find(|(x, _)| *x == r)
                .map_or(i, |(_, c)| Ink::Rgb(*c)),
            Ink::Rgb(_) => i,
        }
    }

    /// Replaces the overridden roles of `d`.
    pub fn apply(&self, d: &mut Drawing) {
        if self.roles.is_empty() {
            return;
        }
        for p in &mut d.prims {
            match p {
                Prim::Path(p) => {
                    p.fill = p.fill.map(|i| self.ink(i));
                    p.stroke = p.stroke.map(|i| self.ink(i));
                }
                Prim::Text(t) => t.color = self.ink(t.color),
            }
        }
    }
}
