//! Mind map model and tidy-tree layout (FUT-37, prompt 014 §3.6). Pure, O(n).
//!
//! Node 0 is the root (first H1 or the file name); headings become branches and list items
//! leaves. Above [`COLLAPSE_ABOVE`] nodes every node whose children are leaves starts collapsed
//! (a count badge shows what it hides), so only the visible part is laid out and drawn.

use yamdv_core::doc::mind::MindItem;

/// Node count above which leaves start collapsed.
pub const COLLAPSE_ABOVE: usize = 2000;
/// Horizontal gap between a node and its children (DIP).
pub const HGAP: f32 = 36.0;
/// Vertical gap between sibling subtrees (DIP).
pub const VGAP: f32 = 8.0;

/// One node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Label.
    pub text: String,
    /// Parent (None for the root).
    pub parent: Option<usize>,
    /// Children in document order.
    pub kids: Vec<usize>,
    /// Source offset to jump to (None for a file-name root).
    pub offset: Option<usize>,
    /// List item.
    pub leaf: bool,
    /// Children hidden.
    pub collapsed: bool,
    /// Top-level branch (colour index 0–5); the root uses 0.
    pub branch: u8,
}

/// The whole map.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Map {
    /// Nodes; 0 is the root.
    pub nodes: Vec<Node>,
}

/// A rectangle (DIP, map coordinates).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    /// Left.
    pub x: f32,
    /// Top.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

impl Rect {
    /// Contains the point.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

impl Map {
    /// Builds the tree from document items (levels nest by a stack: O(n)).
    pub fn build(root: &str, root_offset: Option<usize>, items: &[MindItem]) -> Self {
        let mut nodes = Vec::with_capacity(items.len() + 1);
        nodes.push(Node {
            text: root.to_string(),
            parent: None,
            kids: Vec::new(),
            offset: root_offset,
            leaf: false,
            collapsed: false,
            branch: 0,
        });
        let mut stack: Vec<(u8, usize)> = vec![(0, 0)];
        let mut branches = 0u8;
        for it in items {
            while stack.last().is_some_and(|&(l, _)| l >= it.level && l > 0) {
                stack.pop();
            }
            let p = stack.last().map_or(0, |s| s.1);
            let branch = if p == 0 {
                branches = branches.wrapping_add(1);
                (branches - 1) % 6
            } else {
                nodes.get(p).map_or(0, |n: &Node| n.branch)
            };
            let i = nodes.len();
            nodes.push(Node {
                text: it.text.clone(),
                parent: Some(p),
                kids: Vec::new(),
                offset: Some(it.offset),
                leaf: it.leaf,
                collapsed: false,
                branch,
            });
            if let Some(n) = nodes.get_mut(p) {
                n.kids.push(i);
            }
            stack.push((it.level, i));
        }
        let mut m = Self { nodes };
        if m.nodes.len() > COLLAPSE_ABOVE {
            let leafy: Vec<bool> = m
                .nodes
                .iter()
                .map(|x| {
                    x.kids
                        .iter()
                        .any(|&k| m.nodes.get(k).is_some_and(|c| c.leaf))
                })
                .collect();
            for (x, l) in m.nodes.iter_mut().zip(leafy) {
                x.collapsed = l;
            }
        }
        m
    }

    /// Nodes hidden under `i` (the count badge of a collapsed node).
    pub fn hidden_count(&self, i: usize) -> usize {
        let mut n = 0;
        let mut st: Vec<usize> = self.nodes.get(i).map_or_else(Vec::new, |x| x.kids.clone());
        while let Some(k) = st.pop() {
            n += 1;
            if let Some(x) = self.nodes.get(k) {
                st.extend(x.kids.iter().copied());
            }
        }
        n
    }

    /// Toggles collapse of a node with children; returns whether anything changed.
    pub fn toggle(&mut self, i: usize) -> bool {
        match self.nodes.get_mut(i) {
            Some(n) if !n.kids.is_empty() => {
                n.collapsed = !n.collapsed;
                true
            }
            _ => false,
        }
    }

    /// Visible nodes (no collapsed ancestor).
    pub fn visible_count(&self) -> usize {
        let mut n = 0;
        let mut st = vec![0usize];
        while let Some(i) = st.pop() {
            n += 1;
            if let Some(x) = self.nodes.get(i).filter(|x| !x.collapsed) {
                st.extend(x.kids.iter().copied());
            }
        }
        n
    }
}

/// Layout result: a rectangle per visible node (None = hidden) and the total size; every
/// rectangle is inside (0, 0, w, h).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Placed {
    /// Per node.
    pub rects: Vec<Option<Rect>>,
    /// Width of the whole map.
    pub w: f32,
    /// Height of the whole map.
    pub h: f32,
}

/// Tidy layout in O(n): children sit `HGAP` right (or left) of their parent, visible
/// childless nodes stack top to bottom, and a parent centres on its first and last child.
/// `widths[i]` is node i's width; every non-root node is `h` tall; `root` is the root size.
/// `balanced` puts later top-level branches on the left so both sides hold about half the rows.
pub fn layout(map: &Map, widths: &[f32], h: f32, root: (f32, f32), balanced: bool) -> Placed {
    let n = map.nodes.len();
    let Some(r0) = map.nodes.first() else {
        return Placed::default();
    };
    let node = |i: usize| map.nodes.get(i);
    let kids_of = |i: usize| node(i).map_or(&[][..], |x| x.kids.as_slice());
    let open = |i: usize| node(i).is_some_and(|x| !x.collapsed && !x.kids.is_empty());
    let wid = |i: usize| widths.get(i).copied().unwrap_or(0.0);
    // Rows per top-level branch (visible childless nodes) to split the sides.
    let rows_of = |b: usize| {
        let mut r = 0usize;
        let mut st = vec![b];
        while let Some(i) = st.pop() {
            if open(i) {
                st.extend(kids_of(i).iter().copied());
            } else {
                r += 1;
            }
        }
        r
    };
    let mut left = vec![false; n];
    if balanced && !r0.collapsed {
        let rows: Vec<usize> = r0.kids.iter().map(|&b| rows_of(b)).collect();
        let total: usize = rows.iter().sum();
        let mut acc = 0;
        for (k, (&b, &r)) in r0.kids.iter().zip(&rows).enumerate() {
            if acc * 2 >= total
                && k > 0
                && let Some(l) = left.get_mut(b)
            {
                *l = true;
            }
            acc += r;
        }
    }
    // Pre-order of visible nodes per side (explicit stack: deep lists cannot overflow).
    let mut sides: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
    let mut dx = vec![0f32; n];
    if !r0.collapsed {
        for &b in r0.kids.iter().rev() {
            let s = usize::from(left.get(b).copied().unwrap_or(false));
            let mut st = vec![(b, root.0 / 2.0 + HGAP)];
            let mut order = Vec::new();
            while let Some((i, x)) = st.pop() {
                if let Some(d) = dx.get_mut(i) {
                    *d = x;
                }
                order.push(i);
                if open(i) {
                    let nx = x + wid(i) + HGAP;
                    st.extend(kids_of(i).iter().rev().map(|&k| (k, nx)));
                }
            }
            // Branches are visited last to first; prepend to keep document order.
            if let Some(side) = sides.get_mut(s) {
                order.append(side);
                *side = order;
            }
        }
    }
    let mut cy = vec![0f32; n];
    let mut extent = [0f32; 2];
    for (order, ext) in sides.iter().zip(extent.iter_mut()) {
        let mut cursor = 0f32;
        for &i in order {
            if !open(i)
                && let Some(c) = cy.get_mut(i)
            {
                *c = cursor + h / 2.0;
                cursor += h + VGAP;
            }
        }
        for &i in order.iter().rev() {
            if open(i)
                && let (Some(&a), Some(&b)) = (kids_of(i).first(), kids_of(i).last())
            {
                let mid =
                    (cy.get(a).copied().unwrap_or(0.0) + cy.get(b).copied().unwrap_or(0.0)) / 2.0;
                if let Some(c) = cy.get_mut(i) {
                    *c = mid;
                }
            }
        }
        *ext = (cursor - VGAP).max(0.0);
    }
    let height = extent[0].max(extent[1]).max(root.1);
    let mut rects: Vec<Option<Rect>> = vec![None; n];
    // Centre each side on the root.
    for (s, (order, ext)) in sides.iter().zip(extent).enumerate() {
        let off = (height - ext) / 2.0;
        for &i in order {
            let (w, x0, y0) = (
                wid(i),
                dx.get(i).copied().unwrap_or(0.0),
                cy.get(i).copied().unwrap_or(0.0),
            );
            let x = if s == 1 { -x0 - w } else { x0 };
            if let Some(r) = rects.get_mut(i) {
                *r = Some(Rect {
                    x,
                    y: y0 + off - h / 2.0,
                    w,
                    h,
                });
            }
        }
    }
    if let Some(r) = rects.first_mut() {
        *r = Some(Rect {
            x: -root.0 / 2.0,
            y: (height - root.1) / 2.0,
            w: root.0,
            h: root.1,
        });
    }
    let (mut x0, mut x1) = (f32::MAX, f32::MIN);
    for r in rects.iter().flatten() {
        x0 = x0.min(r.x);
        x1 = x1.max(r.x + r.w);
    }
    for r in rects.iter_mut().flatten() {
        r.x -= x0;
    }
    Placed {
        rects,
        w: x1 - x0,
        h: height,
    }
}

/// Cubic connector from parent `p` to child `c`: start, two controls, end.
pub fn connector(p: Rect, c: Rect) -> [(f32, f32); 4] {
    let right = c.x >= p.x + p.w / 2.0;
    let (sx, ex) = if right {
        (p.x + p.w, c.x)
    } else {
        (p.x, c.x + c.w)
    };
    let (sy, ey) = (p.y + p.h / 2.0, c.y + c.h / 2.0);
    let mx = (sx + ex) / 2.0;
    [(sx, sy), (mx, sy), (mx, ey), (ex, ey)]
}

/// Six branch hues from the accent, rotated in 40° steps (same saturation and lightness).
pub fn hues(accent: u32) -> [u32; 6] {
    let r = ((accent >> 16) & 0xFF) as f32 / 255.0;
    let g = ((accent >> 8) & 0xFF) as f32 / 255.0;
    let b = (accent & 0xFF) as f32 / 255.0;
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    let l = (mx + mn) / 2.0;
    let d = mx - mn;
    let s = if d == 0.0 {
        0.0
    } else {
        d / (1.0 - (2.0 * l - 1.0).abs())
    };
    let h = if d == 0.0 {
        0.0
    } else if mx == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if mx == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let mut out = [0u32; 6];
    for (k, o) in out.iter_mut().enumerate() {
        let hh = (h + 40.0 * k as f32).rem_euclid(360.0);
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((hh / 60.0).rem_euclid(2.0) - 1.0).abs());
        let m = l - c / 2.0;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let sector = (hh / 60.0) as u32;
        let (r1, g1, b1) = match sector {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let q = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u32;
        *o = (q(r1) << 16) | (q(g1) << 8) | q(b1);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn it(level: u8, t: &str) -> MindItem {
        MindItem {
            level,
            text: t.into(),
            offset: 0,
            leaf: level > 6,
        }
    }

    fn sample() -> Map {
        Map::build(
            "Root",
            None,
            &[
                it(2, "A"),
                it(7, "a1"),
                it(8, "a1x"),
                it(7, "a2"),
                it(2, "B"),
                it(3, "B1"),
                it(2, "C"),
                it(2, "D"),
            ],
        )
    }

    fn overlaps(p: &Placed) -> bool {
        let v: Vec<Rect> = p.rects.iter().flatten().copied().collect();
        v.iter().enumerate().any(|(i, a)| {
            v.iter()
                .skip(i + 1)
                .any(|b| a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h)
        })
    }

    #[test]
    fn build_nests_and_colours_branches() {
        let m = sample();
        assert_eq!(m.nodes.len(), 9);
        assert_eq!(m.nodes[0].kids, [1, 5, 7, 8]);
        assert_eq!(m.nodes[2].kids, [3]);
        assert_eq!(m.nodes[3].parent, Some(2));
        assert_eq!(m.nodes[6].branch, 1);
        assert_eq!(m.nodes[8].branch, 3);
        assert_eq!(m.hidden_count(1), 3);
    }

    #[test]
    fn layout_right_and_balanced_no_overlap() {
        let mut m = sample();
        let w = vec![60.0; m.nodes.len()];
        for bal in [false, true] {
            let p = layout(&m, &w, 20.0, (80.0, 30.0), bal);
            assert!(!overlaps(&p), "balanced {bal}");
            assert!(
                p.rects
                    .iter()
                    .flatten()
                    .all(|r| r.x >= 0.0 && r.x + r.w <= p.w + 0.01)
            );
            assert!(
                p.rects
                    .iter()
                    .flatten()
                    .all(|r| r.y >= -0.01 && r.y + r.h <= p.h + 0.01)
            );
            let root = p.rects[0].unwrap_or_default();
            let lefts = p
                .rects
                .iter()
                .flatten()
                .filter(|r| r.x + r.w <= root.x)
                .count();
            assert_eq!(lefts > 0, bal);
            // A parent centres on its children.
            let (a, b) = (
                p.rects[2].unwrap_or_default(),
                p.rects[3].unwrap_or_default(),
            );
            assert!((a.y - b.y).abs() < 0.01);
        }
        assert!(m.toggle(1));
        let p = layout(&m, &w, 20.0, (80.0, 30.0), false);
        assert_eq!(p.rects.iter().flatten().count(), 6);
        assert_eq!(m.visible_count(), 6);
    }

    #[test]
    fn large_maps_start_collapsed_and_scale() {
        let mut items = Vec::new();
        for s in 0..60 {
            items.push(it(2, &format!("S{s}")));
            for k in 0..50 {
                items.push(it(7, &format!("i{k}")));
            }
        }
        let m = Map::build("Big", None, &items);
        assert!(m.nodes.len() > COLLAPSE_ABOVE);
        assert_eq!(m.visible_count(), 61);
        assert_eq!(m.hidden_count(1), 50);
        let w = vec![40.0; m.nodes.len()];
        let t = std::time::Instant::now();
        let p = layout(&m, &w, 18.0, (60.0, 28.0), true);
        assert!(t.elapsed().as_millis() < 200);
        assert_eq!(p.rects.iter().flatten().count(), 61);
        // Deep nesting is iterative.
        let deep: Vec<MindItem> = (0..5000)
            .map(|d| it(7u8.saturating_add(u8::try_from(d % 200).unwrap_or(0)), "d"))
            .collect();
        let m = Map::build("Deep", None, &deep);
        let w = vec![10.0; m.nodes.len()];
        let _ = layout(&m, &w, 10.0, (10.0, 10.0), false);
    }

    #[test]
    fn hues_rotate_from_accent() {
        let h = hues(0x0078D4);
        assert_eq!(h[0], 0x0078D4);
        let mut s = h.to_vec();
        s.dedup();
        assert_eq!(s.len(), 6);
        let c = connector(
            Rect {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            },
            Rect {
                x: 50.0,
                y: 40.0,
                w: 10.0,
                h: 10.0,
            },
        );
        assert_eq!(c[0], (10.0, 5.0));
        assert_eq!(c[3], (50.0, 45.0));
    }
}
