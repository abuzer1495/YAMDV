//! Document-order scope of math macros (R-MATH-07): definitions made in a top-level block
//! apply to every later block. Definitions apply to their whole block (a block is laid out as
//! a unit; documented deviation in docs/MATH_COVERAGE.md).

use std::hash::{Hash, Hasher};
use std::rc::Rc;

use yamdv_core::doc::index::Index;
use yamdv_math::macros::{Macros, expand};

/// Macro environments per top-level block.
#[derive(Debug, Default)]
pub struct MacroScopes {
    key: u64,
    /// (first block, environment from that block on), ascending.
    snaps: Vec<(usize, Rc<Macros>)>,
}

impl MacroScopes {
    /// Identity of the defining blocks (indices and text) this scope set was built from.
    pub fn key(&self) -> u64 {
        self.key
    }

    /// Builds the scopes of `text` with index `ix`.
    pub fn build(text: &str, ix: &Index) -> Self {
        let key = scope_key(text, ix);
        let mut env = Macros::default();
        let mut snaps = Vec::new();
        for &b in &ix.macro_blocks {
            let Some(blk) = ix.blocks.get(b as usize) else {
                continue;
            };
            let s = text
                .get(blk.range.start as usize..blk.range.end as usize)
                .unwrap_or("");
            let before = env.clone();
            for f in math_spans(s) {
                // A failing formula keeps the definitions made before its error.
                let mut trial = env.clone();
                if expand(f, &mut trial).is_ok() {
                    env = trial;
                }
            }
            if env != before {
                snaps.push((b as usize, Rc::new(env.clone())));
            }
        }
        Self { key, snaps }
    }

    /// The environment of top-level block `b` and its id (0 = no macros).
    pub fn for_block(&self, b: usize) -> (u64, Option<Rc<Macros>>) {
        let i = self.snaps.partition_point(|s| s.0 <= b);
        match i.checked_sub(1).and_then(|i| self.snaps.get(i)) {
            Some((_, m)) if !m.is_empty() => (self.key ^ (i as u64 + 1), Some(m.clone())),
            _ => (0, None),
        }
    }
}

/// Hash of the defining blocks of `text` (cheap: only those blocks are read).
pub fn scope_key(text: &str, ix: &Index) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for &b in &ix.macro_blocks {
        b.hash(&mut h);
        if let Some(blk) = ix.blocks.get(b as usize) {
            text.get(blk.range.start as usize..blk.range.end as usize)
                .hash(&mut h);
        }
    }
    h.finish() | 1
}

/// The formulas of a block's source: `$$…$$`, `$…$` and a fenced `math` block's body.
pub fn math_spans(s: &str) -> Vec<&str> {
    let t = s.trim_start();
    for fence in ["```", "~~~"] {
        if let Some(rest) = t.strip_prefix(fence) {
            let info = rest.trim_start_matches(fence.chars().next().unwrap_or('`'));
            if info.trim_start().to_ascii_lowercase().starts_with("math") {
                let body = info.split_once('\n').map_or("", |x| x.1);
                let end = body.rfind(fence).unwrap_or(body.len());
                return vec![body.get(..end).unwrap_or(body)];
            }
            return Vec::new();
        }
    }
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while let Some(&c) = b.get(i) {
        match c {
            b'\\' => i += 2,
            b'$' => {
                let dd = b.get(i + 1) == Some(&b'$');
                let open = if dd { i + 2 } else { i + 1 };
                let close = if dd {
                    s.get(open..).and_then(|r| r.find("$$"))
                } else {
                    find_dollar(b, open)
                };
                match close {
                    Some(c) => {
                        let end = if dd { open + c } else { c };
                        if let Some(f) = s.get(open..end) {
                            out.push(f);
                        }
                        i = end + if dd { 2 } else { 1 };
                    }
                    None => break,
                }
            }
            _ => i += 1,
        }
    }
    out
}

fn find_dollar(b: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    while let Some(&c) = b.get(i) {
        match c {
            b'\\' => i += 2,
            b'$' => return Some(i),
            _ => i += 1,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use yamdv_core::doc::{Document, ReadOpts};

    #[test]
    fn scopes_follow_document_order() {
        let md = "$\\RR$\n\n$\\newcommand{\\RR}{\\mathbb R}\\RR$ and `x`\n\n$$\\def\\v#1{\\vec{#1}}$$\n\ntext $\\RR \\v x$\n\n```math\n\\renewcommand{\\RR}{R}\n```\n\nend\n";
        let d = Document::new(md.to_string(), ReadOpts::default());
        assert_eq!(d.index.macro_blocks, vec![1, 2, 4]);
        let s = MacroScopes::build(&d.source, &d.index);
        assert_eq!(s.for_block(0), (0, None));
        let (id1, m1) = s.for_block(1);
        assert!(id1 != 0 && m1.unwrap().get("RR").is_some());
        let (_, m3) = s.for_block(3);
        let m3 = m3.unwrap();
        assert!(m3.get("v").is_some_and(|m| m.params == 1));
        assert_eq!(s.for_block(5).1.unwrap().get("RR").unwrap().body, "R");
        assert_eq!(math_spans(r"a \$ $x$ $$y$$ $z"), vec!["x", "y"]);
    }
}
