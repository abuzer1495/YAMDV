//! `classDiagram` and `erDiagram` (FUT-18 SHOULD) as [`Flow`]s of compartment nodes.

use crate::Error;
use crate::flow::{Dir, Edge, End, Flow, Line, Shape};
use crate::graph::{Builder, class_suffix};

/// Class members split into (attributes, methods).
#[derive(Default)]
struct Members {
    attrs: Vec<String>,
    methods: Vec<String>,
    annotation: Option<String>,
}

fn ident(s: &str) -> &str {
    // `Name~T~` generics display as `Name<T>` (label only); the id keeps the plain name.
    s.split('~').next().unwrap_or(s).trim()
}

fn generic_label(s: &str) -> String {
    let s = s.trim();
    let mut parts = s.split('~');
    let name = parts.next().unwrap_or("");
    match parts.next() {
        Some(g) if !g.is_empty() => format!("{name}<{g}>"),
        _ => name.to_string(),
    }
}

/// A class relation operator: (tail, line, head) and its byte length.
fn class_op(s: &str) -> Option<(End, Line, End, usize)> {
    const LEFT: [(&str, End); 4] = [
        ("<|", End::Triangle),
        ("*", End::DiamondFilled),
        ("o", End::Diamond),
        ("<", End::Arrow),
    ];
    const RIGHT: [(&str, End); 4] = [
        ("|>", End::Triangle),
        ("*", End::DiamondFilled),
        ("o", End::Diamond),
        (">", End::Arrow),
    ];
    let (tail, rest, n0) = LEFT
        .iter()
        .find_map(|(p, e)| {
            s.strip_prefix(p)
                .filter(|r| r.starts_with("--") || r.starts_with(".."))
                .map(|r| (*e, r, p.len()))
        })
        .unwrap_or((End::None, s, 0));
    let line = if rest.starts_with("--") {
        Line::Solid
    } else if rest.starts_with("..") {
        Line::Dotted
    } else {
        return None;
    };
    let rest = rest.get(2..)?;
    let (head, n1) = RIGHT
        .iter()
        .find_map(|(p, e)| rest.starts_with(p).then_some((*e, p.len())))
        .unwrap_or((End::None, 0));
    Some((tail, line, head, n0 + 2 + n1))
}

/// Finds a relation: (left part, (tail, line, head), right part).
fn split_rel(l: &str) -> Option<(&str, (End, Line, End), &str)> {
    let mut quote = false;
    for (i, c) in l.char_indices() {
        if c == '"' {
            quote = !quote;
        }
        if quote || !matches!(c, '<' | '*' | 'o' | '-' | '.') {
            continue;
        }
        // An `o` only starts an operator right after whitespace or a quote.
        if c == 'o' && !l.get(..i).is_some_and(|b| b.ends_with([' ', '"'])) {
            continue;
        }
        if let Some((t, ln, h, n)) = l.get(i..).and_then(class_op) {
            return Some((l.get(..i)?, (t, ln, h), l.get(i + n..)?));
        }
    }
    None
}

/// `"card"` at the end (left side) or start (right side) of a relation part.
fn card(s: &str, left: bool) -> (String, Option<String>) {
    let s = s.trim();
    if left {
        if let Some(body) = s.strip_suffix('"')
            && let Some((id, c)) = body.rsplit_once('"')
        {
            return (id.trim().to_string(), Some(c.to_string()));
        }
    } else if let Some(body) = s.strip_prefix('"')
        && let Some((c, id)) = body.split_once('"')
    {
        return (id.trim().to_string(), Some(c.to_string()));
    }
    (s.to_string(), None)
}

fn class_node(b: &mut Builder, raw: &str, members: &mut Vec<Members>) -> Result<usize, Error> {
    let (raw, cls) = class_suffix(raw);
    let id = ident(raw);
    let i = b.node(id, Shape::Class)?;
    if members.len() <= i {
        members.resize_with(i + 1, Members::default);
    }
    if let Some(n) = b.get(i) {
        if raw.contains('~') {
            n.label = generic_label(raw);
        }
        n.classes.extend(cls.map(str::to_string));
    }
    Ok(i)
}

fn member(m: &mut Members, s: &str) {
    let s = s.trim();
    if s.is_empty() {
        return;
    }
    if let Some(a) = s.strip_prefix("<<").and_then(|x| x.strip_suffix(">>")) {
        m.annotation = Some(a.trim().to_string());
    } else if s.contains('(') {
        m.methods.push(s.to_string());
    } else {
        m.attrs.push(s.to_string());
    }
}

/// Parses a `classDiagram` block.
pub fn parse_class(src: &str) -> Result<Flow, Error> {
    let mut b = Builder::new(Dir::TB);
    let mut members: Vec<Members> = Vec::new();
    let mut body: Option<usize> = None;
    let mut namespaces = 0usize;
    for (ln, line) in crate::lines(src).skip(1) {
        b.line = ln;
        let l = line.trim();
        if let Some(i) = body {
            if l == "}" {
                body = None;
            } else if let Some(m) = members.get_mut(i) {
                member(m, l);
            }
            continue;
        }
        if l == "}" {
            if b.stack.pop().is_none() {
                return Err(b.e("'}' without '{'"));
            }
            continue;
        }
        if l.starts_with("note") {
            continue;
        }
        if let Some(r) = l.strip_prefix("namespace ") {
            namespaces += 1;
            let name = r.trim().trim_end_matches('{').trim();
            b.open(&format!("ns{namespaces}"), name);
            continue;
        }
        if let Some(r) = l.strip_prefix("class ") {
            let r = r.trim();
            let (r, open) = match r.strip_suffix('{') {
                Some(x) => (x.trim(), true),
                None => (r, false),
            };
            let (r, label) = match r.split_once('[') {
                Some((a, lab)) => (a.trim(), Some(lab.trim_end_matches(']').trim_matches('"'))),
                None => (r, None),
            };
            let i = class_node(&mut b, r, &mut members)?;
            if let (Some(lab), Some(n)) = (label, b.get(i)) {
                n.label = lab.to_string();
            }
            if open {
                body = Some(i);
            }
            continue;
        }
        if let Some(a) = l.strip_prefix("<<")
            && let Some((ann, id)) = a.split_once(">>")
        {
            let i = class_node(&mut b, id, &mut members)?;
            if let Some(m) = members.get_mut(i) {
                m.annotation = Some(ann.trim().to_string());
            }
            continue;
        }
        if b.common(l)? {
            continue;
        }
        if let Some((left, (tail, line, head), right)) = split_rel(l) {
            let (right, label) = right
                .split_once(':')
                .map_or((right, None), |(r, t)| (r, Some(t.trim().to_string())));
            let (lid, lc) = card(left, true);
            let (rid, rc) = card(right, false);
            let a = class_node(&mut b, &lid, &mut members)?;
            let z = class_node(&mut b, &rid, &mut members)?;
            let parts: Vec<String> = [lc, label, rc]
                .into_iter()
                .flatten()
                .filter(|s| !s.is_empty())
                .collect();
            let mut e = Edge::new(a, z, line, head);
            e.tail = tail;
            e.label = (!parts.is_empty()).then(|| parts.join("  "));
            b.edge(e)?;
            continue;
        }
        if let Some((id, m)) = l.split_once(':') {
            let i = class_node(&mut b, id, &mut members)?;
            if let Some(mm) = members.get_mut(i) {
                member(mm, m);
            }
            continue;
        }
        class_node(&mut b, l, &mut members)?;
    }
    if body.is_some() || !b.stack.is_empty() {
        return Err(b.e("Unclosed '{'"));
    }
    for (i, m) in members.into_iter().enumerate() {
        if let Some(n) = b.f.nodes.get_mut(i) {
            if let Some(a) = m.annotation {
                n.label = format!("«{a}»\n{}", n.label);
            }
            n.sections = vec![m.attrs.join("\n"), m.methods.join("\n")];
        }
    }
    Ok(b.f)
}

fn er_end(s: &str, left: bool) -> Option<End> {
    Some(match (s, left) {
        ("|o", true) | ("o|", false) => End::ZeroOne,
        ("||", _) => End::One,
        ("}o", true) | ("o{", false) => End::ZeroMany,
        ("}|", true) | ("|{", false) => End::OneMany,
        _ => return None,
    })
}

/// Parses an `erDiagram` block.
pub fn parse_er(src: &str) -> Result<Flow, Error> {
    let mut b = Builder::new(Dir::TB);
    let mut body: Option<(usize, Vec<String>)> = None;
    for (ln, line) in crate::lines(src).skip(1) {
        b.line = ln;
        let l = line.trim();
        if let Some((i, attrs)) = &mut body {
            if l == "}" {
                let (i, a) = (*i, std::mem::take(attrs));
                if let Some(n) = b.get(i) {
                    n.sections = vec![a.join("\n")];
                }
                body = None;
            } else {
                attrs.push(l.split_whitespace().collect::<Vec<_>>().join(" "));
            }
            continue;
        }
        if b.common(l)? {
            continue;
        }
        if let Some(name) = l.strip_suffix('{') {
            let i = entity(&mut b, name)?;
            body = Some((i, Vec::new()));
            continue;
        }
        let mut words = l.split_whitespace();
        let (Some(a), Some(op), Some(z)) = (words.next(), words.next(), words.next()) else {
            entity(&mut b, l)?;
            continue;
        };
        let line_kind = if op.contains("--") {
            Line::Solid
        } else if op.contains("..") {
            Line::Dotted
        } else {
            return Err(b.e(format!("Unknown relationship \"{op}\"")));
        };
        let (lc, rc) = (
            op.get(..2).unwrap_or(""),
            op.get(op.len().saturating_sub(2)..).unwrap_or(""),
        );
        let (Some(tail), Some(head)) = (er_end(lc, true), er_end(rc, false)) else {
            return Err(b.e(format!("Unknown cardinality \"{op}\"")));
        };
        let rest: Vec<&str> = words.collect();
        let label = rest.join(" ");
        let label = label
            .trim()
            .strip_prefix(':')
            .unwrap_or("")
            .trim()
            .trim_matches('"');
        let from = entity(&mut b, a)?;
        let to = entity(&mut b, z)?;
        let mut e = Edge::new(from, to, line_kind, head);
        e.tail = tail;
        e.label = (!label.is_empty()).then(|| label.to_string());
        b.edge(e)?;
    }
    if body.is_some() {
        return Err(b.e("Unclosed '{'"));
    }
    Ok(b.f)
}

fn entity(b: &mut Builder, raw: &str) -> Result<usize, Error> {
    let raw = raw.trim();
    let (id, alias) = match raw.split_once('[') {
        Some((a, l)) => (a.trim(), Some(l.trim_end_matches(']').trim_matches('"'))),
        None => (raw, None),
    };
    let id = id.trim_matches('"');
    let i = b.node(id, Shape::Class)?;
    if let (Some(a), Some(n)) = (alias, b.get(i)) {
        n.label = a.to_string();
    }
    Ok(i)
}
