//! Layered (Sugiyama) flowchart layout (prompt 007 §3.2): DFS cycle breaking, longest-path
//! ranks with dummy nodes, 4 barycenter sweeps, median alignment + compaction (40/50 px), then
//! the direction transform.
// Indices below come from this module's own graph construction (every id < its vector's len).
#![allow(clippy::indexing_slicing)]
#![allow(
    clippy::cast_precision_loss,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

use crate::draw::{self, Drawing, Head, Ink, Measure, Path, Prim, Role, Seg, TextPrim};
use crate::flow::{Dir, Flow, Line, Shape};

/// Virtual-node cap (dummy chains of long edges); above it the layout reports `complex`.
const MAX_VIRTUAL: usize = 60_000;

struct V {
    rank: usize,
    /// Size across ranks / along ranks.
    cs: f32,
    rs: f32,
    real: Option<usize>,
    /// Cluster chain, outermost first.
    chain: Vec<usize>,
}

fn ancestors(f: &Flow, sub: Option<usize>) -> Vec<usize> {
    let mut v = Vec::new();
    let mut c = sub;
    while let Some(i) = c {
        if v.len() > f.subs.len() {
            break;
        }
        v.push(i);
        c = f.subs.get(i).and_then(|s| s.parent);
    }
    v.reverse();
    v
}

fn common(a: &[usize], b: &[usize]) -> usize {
    a.iter().zip(b).take_while(|(x, y)| x == y).count()
}

/// Inversions of `v` (merge sort).
fn inversions(v: &mut [usize]) -> u64 {
    let n = v.len();
    if n < 2 {
        return 0;
    }
    let mid = n / 2;
    let mut c = inversions(&mut v[..mid]) + inversions(&mut v[mid..]);
    let mut merged = Vec::with_capacity(n);
    let (mut i, mut j) = (0, mid);
    while i < mid && j < n {
        if v[i] <= v[j] {
            merged.push(v[i]);
            i += 1;
        } else {
            merged.push(v[j]);
            c += (mid - i) as u64;
            j += 1;
        }
    }
    merged.extend_from_slice(&v[i..mid]);
    merged.extend_from_slice(&v[j..n]);
    v.copy_from_slice(&merged);
    c
}

struct G {
    vs: Vec<V>,
    /// Virtual edges (upper → lower rank, adjacent ranks).
    up: Vec<Vec<usize>>,
    down: Vec<Vec<usize>>,
    layers: Vec<Vec<usize>>,
    pos: Vec<usize>,
}

impl G {
    fn crossings(&self) -> u64 {
        let mut total = 0;
        for l in &self.layers {
            let mut e: Vec<(usize, usize)> = Vec::new();
            for &u in l {
                for &w in &self.down[u] {
                    e.push((self.pos[u], self.pos[w]));
                }
            }
            e.sort_unstable();
            let mut t: Vec<usize> = e.iter().map(|x| x.1).collect();
            total += inversions(&mut t);
        }
        total
    }

    fn sort_layer(&mut self, r: usize, downward: bool) {
        let l = self.layers[r].clone();
        let bary: Vec<f32> = l
            .iter()
            .map(|&v| {
                let nb = if downward { &self.up[v] } else { &self.down[v] };
                if nb.is_empty() {
                    self.pos[v] as f32
                } else {
                    nb.iter().map(|&w| self.pos[w] as f32).sum::<f32>() / nb.len() as f32
                }
            })
            .collect();
        // Cluster barycenters (members stay contiguous, R-MMD subgraphs).
        let mut cb: std::collections::HashMap<usize, (f32, f32)> = Default::default();
        for (k, &v) in l.iter().enumerate() {
            for &c in &self.vs[v].chain {
                let e = cb.entry(c).or_insert((0.0, 0.0));
                e.0 += bary[k];
                e.1 += 1.0;
            }
        }
        let key = |k: usize, d: usize| -> (f32, usize) {
            let v = l[k];
            match self.vs[v].chain.get(d) {
                Some(&c) => {
                    let (s, n) = cb.get(&c).copied().unwrap_or((0.0, 1.0));
                    (s / n, c + 1)
                }
                None => (bary[k], 0),
            }
        };
        let maxd = l.iter().map(|&v| self.vs[v].chain.len()).max().unwrap_or(0);
        let mut idx: Vec<usize> = (0..l.len()).collect();
        idx.sort_by(|&a, &b| {
            for d in 0..=maxd {
                let (ka, kb) = (key(a, d), key(b, d));
                let o = ka.0.total_cmp(&kb.0).then(ka.1.cmp(&kb.1));
                if o != std::cmp::Ordering::Equal {
                    return o;
                }
                if ka.1 == 0 && kb.1 == 0 {
                    break;
                }
            }
            bary[a].total_cmp(&bary[b]).then(a.cmp(&b))
        });
        let nl: Vec<usize> = idx.iter().map(|&k| l[k]).collect();
        for (p, &v) in nl.iter().enumerate() {
            self.pos[v] = p;
        }
        self.layers[r] = nl;
    }
}

/// Pool-adjacent-violators: nondecreasing `z` closest (least squares) to `t`.
fn pav(t: &[f32]) -> Vec<f32> {
    let mut blocks: Vec<(f32, f32, usize)> = Vec::new(); // (sum, weight, count)
    for &x in t {
        blocks.push((x, 1.0, 1));
        while blocks.len() >= 2 {
            let n = blocks.len();
            let (a, b) = (blocks[n - 2], blocks[n - 1]);
            if a.0 / a.1 > b.0 / b.1 {
                blocks.pop();
                blocks[n - 2] = (a.0 + b.0, a.1 + b.1, a.2 + b.2);
            } else {
                break;
            }
        }
    }
    let mut z = Vec::with_capacity(t.len());
    for (s, w, c) in blocks {
        z.extend(std::iter::repeat_n(s / w, c));
    }
    z
}

fn median(mut v: Vec<f32>) -> Option<f32> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f32::total_cmp);
    let n = v.len();
    Some(if n % 2 == 1 {
        v[n / 2]
    } else {
        f32::midpoint(v[n / 2 - 1], v[n / 2])
    })
}

/// Point where the segment from the center of box (cx, cy, w, h) toward `to` leaves the shape.
fn clip(shape: Shape, c: (f32, f32), w: f32, h: f32, to: (f32, f32)) -> (f32, f32) {
    let (dx, dy) = (to.0 - c.0, to.1 - c.1);
    if dx.abs() < 1e-4 && dy.abs() < 1e-4 {
        return c;
    }
    let (hw, hh) = (w / 2.0, h / 2.0);
    let t = match shape {
        Shape::Rhombus => 1.0 / (dx.abs() / hw + dy.abs() / hh),
        Shape::Circle => hw / (dx * dx + dy * dy).sqrt(),
        _ => {
            let tx = if dx.abs() > 1e-6 {
                hw / dx.abs()
            } else {
                f32::MAX
            };
            let ty = if dy.abs() > 1e-6 {
                hh / dy.abs()
            } else {
                f32::MAX
            };
            tx.min(ty)
        }
    };
    let t = t.min(1.0);
    (c.0 + dx * t, c.1 + dy * t)
}

struct NodeGeo {
    w: f32,
    h: f32,
    text: (u32, f32, f32),
}

fn node_geo(f: &Flow, m: &mut dyn Measure, s: f32) -> Vec<NodeGeo> {
    f.nodes
        .iter()
        .map(|n| {
            let t = m.text(&n.label, 14.0 * s, 200.0 * s, false);
            let (w0, h0) = (t.1 + 24.0 * s, t.2 + 16.0 * s);
            let (w, h) = match n.shape {
                Shape::Rect | Shape::Round => (w0, h0),
                Shape::Subroutine => (w0 + 16.0 * s, h0),
                Shape::Stadium => (w0 + h0 / 2.0, h0),
                Shape::Cylinder => (w0, h0 + 12.0 * s),
                Shape::Circle => {
                    let d = (t.1 * t.1 + t.2 * t.2).sqrt() + 16.0 * s;
                    (d, d)
                }
                Shape::Rhombus => {
                    let d = t.1 + t.2 + 24.0 * s;
                    (d, d)
                }
            };
            NodeGeo { w, h, text: t }
        })
        .collect()
}

#[allow(clippy::too_many_lines)]
pub(crate) fn layout(f: &Flow, m: &mut dyn Measure, s: f32) -> Drawing {
    let n = f.nodes.len();
    let geo = node_geo(f, m, s);
    let horizontal = matches!(f.dir, Dir::LR | Dir::RL);
    let (pad, sep, rsep) = (12.0 * s, 40.0 * s, 50.0 * s);
    let title: Vec<(u32, f32, f32)> = f
        .subs
        .iter()
        .map(|g| m.text(&g.title, 14.0 * s, 300.0 * s, true))
        .collect();
    let title_h = title.iter().map(|t| t.2).fold(0.0f32, f32::max) + 8.0 * s;
    let labels: Vec<Option<(u32, f32, f32)>> = f
        .edges
        .iter()
        .map(|e| {
            e.label
                .as_deref()
                .filter(|l| !l.is_empty())
                .map(|l| m.text(l, 13.0 * s, 160.0 * s, false))
        })
        .collect();

    // 1. Cycle breaking: DFS; edges to a node on the stack are reversed.
    let mut adj: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
    for (ei, e) in f.edges.iter().enumerate() {
        if e.from != e.to && e.from < n && e.to < n {
            adj[e.from].push((e.to, ei));
        }
    }
    let mut rev = vec![false; f.edges.len()];
    let mut state = vec![0u8; n];
    for root in 0..n {
        if state[root] != 0 {
            continue;
        }
        let mut st: Vec<(usize, usize)> = vec![(root, 0)];
        state[root] = 1;
        while let Some(&mut (v, ref mut i)) = st.last_mut() {
            if let Some(&(w, ei)) = adj[v].get(*i) {
                *i += 1;
                match state[w] {
                    0 => {
                        state[w] = 1;
                        st.push((w, 0));
                    }
                    1 => rev[ei] = true,
                    _ => {}
                }
            } else {
                state[v] = 2;
                st.pop();
            }
        }
    }
    let dag: Vec<(usize, usize, usize)> = f
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.from != e.to && e.from < n && e.to < n)
        .map(|(i, e)| {
            if rev[i] {
                (e.to, e.from, i)
            } else {
                (e.from, e.to, i)
            }
        })
        .collect();

    // 2. Longest-path ranks (Kahn order).
    let mut indeg = vec![0usize; n];
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(a, b, _) in &dag {
        indeg[b] += 1;
        out[a].push(b);
    }
    let mut rank = vec![0usize; n];
    let mut q: std::collections::VecDeque<usize> = (0..n).filter(|&v| indeg[v] == 0).collect();
    while let Some(v) = q.pop_front() {
        for &w in &out[v] {
            rank[w] = rank[w].max(rank[v] + 1);
            indeg[w] -= 1;
            if indeg[w] == 0 {
                q.push_back(w);
            }
        }
    }
    let mut g = G {
        vs: (0..n)
            .map(|i| {
                let (c, r) = if horizontal {
                    (geo[i].h, geo[i].w)
                } else {
                    (geo[i].w, geo[i].h)
                };
                V {
                    rank: rank[i],
                    cs: c,
                    rs: r,
                    real: Some(i),
                    chain: ancestors(f, f.nodes[i].sub),
                }
            })
            .collect(),
        up: vec![Vec::new(); n],
        down: vec![Vec::new(); n],
        layers: Vec::new(),
        pos: Vec::new(),
    };
    // Dummy chains; `chains[e]` lists the virtual nodes of edge `e` in source → target order.
    let mut chains: Vec<Vec<usize>> = vec![Vec::new(); f.edges.len()];
    let mut complex = false;
    for &(a, b, ei) in &dag {
        let mut c = vec![a];
        let cl = {
            let (x, y) = (&g.vs[a].chain, &g.vs[b].chain);
            x[..common(x, y)].to_vec()
        };
        for r in rank[a] + 1..rank[b] {
            if g.vs.len() >= MAX_VIRTUAL {
                complex = true;
                break;
            }
            g.vs.push(V {
                rank: r,
                cs: 12.0 * s,
                rs: 0.0,
                real: None,
                chain: cl.clone(),
            });
            g.up.push(Vec::new());
            g.down.push(Vec::new());
            c.push(g.vs.len() - 1);
        }
        c.push(b);
        for w in c.windows(2) {
            g.down[w[0]].push(w[1]);
            g.up[w[1]].push(w[0]);
        }
        if rev[ei] {
            c.reverse();
        }
        chains[ei] = c;
    }
    if complex {
        return Drawing {
            complex: true,
            ..Drawing::default()
        };
    }
    let nr = g.vs.iter().map(|v| v.rank + 1).max().unwrap_or(0);
    g.layers = vec![Vec::new(); nr];
    g.pos = vec![0; g.vs.len()];
    for (i, v) in g.vs.iter().enumerate() {
        g.pos[i] = g.layers[v.rank].len();
        g.layers[v.rank].push(i);
    }
    for r in 0..nr {
        g.sort_layer(r, true);
    }

    // 3. Barycenter crossing reduction: 4 sweeps, keep the best.
    let mut best = (g.crossings(), g.layers.clone());
    for sweep in 0..4 {
        let down = sweep % 2 == 0;
        if down {
            for r in 1..nr {
                g.sort_layer(r, true);
            }
        } else {
            for r in (0..nr.saturating_sub(1)).rev() {
                g.sort_layer(r, false);
            }
        }
        let c = g.crossings();
        if c < best.0 {
            best = (c, g.layers.clone());
        }
    }
    g.layers = best.1;
    for l in &g.layers {
        for (p, &v) in l.iter().enumerate() {
            g.pos[v] = p;
        }
    }

    // 4. Cross coordinates: pack, then median alignment + compaction (PAV).
    let gap = |a: &V, b: &V| -> f32 {
        let base = match (a.real.is_some(), b.real.is_some()) {
            (true, true) => sep,
            (false, false) => 10.0 * s,
            _ => 20.0 * s,
        };
        let c = common(&a.chain, &b.chain);
        let leave = (a.chain.len() - c) as f32;
        let enter = (b.chain.len() - c) as f32;
        let t = if horizontal { title_h } else { 0.0 };
        (a.cs + b.cs) / 2.0 + base + leave * pad + enter * (pad + t)
    };
    let mut x = vec![0.0f32; g.vs.len()];
    for l in &g.layers {
        let mut cur = 0.0;
        for (k, &v) in l.iter().enumerate() {
            if k > 0 {
                cur += gap(&g.vs[l[k - 1]], &g.vs[v]);
            }
            x[v] = cur;
        }
    }
    for it in 0..8 {
        let down = it % 2 == 0;
        let order: Vec<usize> = if down {
            (0..nr).collect()
        } else {
            (0..nr).rev().collect()
        };
        for r in order {
            let l = &g.layers[r];
            if l.is_empty() {
                continue;
            }
            let desired: Vec<f32> = l
                .iter()
                .map(|&v| {
                    let nb = if down { &g.up[v] } else { &g.down[v] };
                    let both: Vec<f32> = if it >= 6 {
                        g.up[v].iter().chain(&g.down[v]).map(|&w| x[w]).collect()
                    } else {
                        nb.iter().map(|&w| x[w]).collect()
                    };
                    median(both).unwrap_or(x[v])
                })
                .collect();
            let mut off = vec![0.0f32; l.len()];
            for k in 1..l.len() {
                off[k] = off[k - 1] + gap(&g.vs[l[k - 1]], &g.vs[l[k]]);
            }
            let t: Vec<f32> = desired.iter().zip(&off).map(|(d, o)| d - o).collect();
            let z = pav(&t);
            for (k, &v) in l.iter().enumerate() {
                x[v] = z[k] + off[k];
            }
        }
    }

    // Rank coordinates with label and subgraph room in the gaps.
    let lsize: Vec<f32> = g
        .layers
        .iter()
        .map(|l| l.iter().map(|&v| g.vs[v].rs).fold(0.0, f32::max))
        .collect();
    let mut extra = vec![0.0f32; nr];
    for (ei, c) in chains.iter().enumerate() {
        if let (Some(Some(lb)), false) = (labels.get(ei), c.is_empty()) {
            let k = (c.len().saturating_sub(2)) / 2;
            let ra = g.vs[c[k]].rank.min(g.vs[c[k + 1]].rank);
            let sz = if horizontal { lb.1 } else { lb.2 } + 12.0 * s;
            extra[ra] = extra[ra].max(sz);
        }
    }
    if !f.subs.is_empty() {
        let mut minr = vec![usize::MAX; f.subs.len()];
        let mut maxr = vec![0usize; f.subs.len()];
        for v in g.vs.iter().filter(|v| v.real.is_some()) {
            for &c in &v.chain {
                minr[c] = minr[c].min(v.rank);
                maxr[c] = maxr[c].max(v.rank);
            }
        }
        for r in 0..nr.saturating_sub(1) {
            let enter = g.layers[r + 1]
                .iter()
                .map(|&v| g.vs[v].chain.iter().filter(|&&c| minr[c] == r + 1).count())
                .max()
                .unwrap_or(0) as f32;
            let leave = g.layers[r]
                .iter()
                .map(|&v| g.vs[v].chain.iter().filter(|&&c| maxr[c] == r).count())
                .max()
                .unwrap_or(0) as f32;
            let (te, tl) = match f.dir {
                Dir::TB => (title_h, 0.0),
                Dir::BT => (0.0, title_h),
                _ => (0.0, 0.0),
            };
            extra[r] += enter * (pad + te) + leave * (pad + tl);
        }
    }
    let mut ry = vec![0.0f32; nr];
    for r in 1..nr {
        ry[r] = ry[r - 1] + lsize[r - 1] + rsep + extra[r - 1];
    }
    // Abstract (cross, rank) centers.
    let center = |v: usize| -> (f32, f32) {
        let r = g.vs[v].rank;
        (x[v], ry[r] + lsize[r] / 2.0)
    };
    let total_r = ry.last().copied().unwrap_or(0.0) + lsize.last().copied().unwrap_or(0.0);
    let to_xy = |p: (f32, f32)| -> (f32, f32) {
        match f.dir {
            Dir::TB => (p.0, p.1),
            Dir::BT => (p.0, total_r - p.1),
            Dir::LR => (p.1, p.0),
            Dir::RL => (total_r - p.1, p.0),
        }
    };

    // 5. Geometry in screen space.
    let ncenter: Vec<(f32, f32)> = (0..n).map(|i| to_xy(center(i))).collect();
    let rects: Vec<(f32, f32, f32, f32)> = (0..n)
        .map(|i| {
            let (cx, cy) = ncenter[i];
            (cx - geo[i].w / 2.0, cy - geo[i].h / 2.0, geo[i].w, geo[i].h)
        })
        .collect();
    let mut edges: Vec<Vec<(f32, f32)>> = Vec::with_capacity(f.edges.len());
    for (ei, e) in f.edges.iter().enumerate() {
        let pts: Vec<(f32, f32)> = if e.from == e.to {
            let Some(&(x0, y0, w, h)) = rects.get(e.from) else {
                edges.push(Vec::new());
                continue;
            };
            let (x1, q) = (x0 + w, h / 4.0);
            let cy = y0 + h / 2.0;
            vec![
                (x1, cy - q),
                (x1 + 22.0 * s, cy - q - 6.0 * s),
                (x1 + 22.0 * s, cy + q + 6.0 * s),
                (x1, cy + q),
            ]
        } else {
            let c = &chains[ei];
            let mut pts: Vec<(f32, f32)> = c.iter().map(|&v| to_xy(center(v))).collect();
            let k = pts.len();
            if k >= 2 {
                let (a, b) = (e.from, e.to);
                pts[0] = clip(f.nodes[a].shape, pts[0], geo[a].w, geo[a].h, pts[1]);
                pts[k - 1] = clip(f.nodes[b].shape, pts[k - 1], geo[b].w, geo[b].h, pts[k - 2]);
            }
            pts
        };
        edges.push(pts);
    }
    // Edge label boxes (center of the middle segment).
    let lab_boxes: Vec<Option<(f32, f32, f32, f32, u32)>> = edges
        .iter()
        .enumerate()
        .map(|(ei, pts)| {
            let lb = labels.get(ei).copied().flatten()?;
            let k = pts.len();
            if k < 2 {
                return None;
            }
            let mid = if f.edges[ei].from == f.edges[ei].to {
                (pts[1].0, f32::midpoint(pts[1].1, pts[2].1))
            } else {
                let j = (k - 2) / 2;
                (
                    f32::midpoint(pts[j].0, pts[j + 1].0),
                    f32::midpoint(pts[j].1, pts[j + 1].1),
                )
            };
            let (w, h) = (lb.1 + 8.0 * s, lb.2 + 4.0 * s);
            Some((mid.0 - w / 2.0, mid.1 - h / 2.0, w, h, lb.0))
        })
        .collect();
    // Subgraph boxes, deepest first.
    let mut sub_box: Vec<Option<(f32, f32, f32, f32)>> = vec![None; f.subs.len()];
    let depth = |i: usize| ancestors(f, Some(i)).len();
    let mut order: Vec<usize> = (0..f.subs.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(depth(i)));
    for &si in &order {
        let mut bb: Option<(f32, f32, f32, f32)> = None;
        let mut add = |r: (f32, f32, f32, f32)| {
            bb = Some(match bb {
                None => (r.0, r.1, r.0 + r.2, r.1 + r.3),
                Some(b) => (
                    b.0.min(r.0),
                    b.1.min(r.1),
                    b.2.max(r.0 + r.2),
                    b.3.max(r.1 + r.3),
                ),
            });
        };
        for (i, nd) in f.nodes.iter().enumerate() {
            if nd.sub == Some(si) {
                add(rects[i]);
            }
        }
        for (ci, c) in f.subs.iter().enumerate() {
            if c.parent == Some(si)
                && let Some(r) = sub_box[ci]
            {
                add(r);
            }
        }
        if let Some(b) = bb {
            let tw = title[si].1 + 2.0 * pad;
            let (x0, y0) = (b.0 - pad, b.1 - pad - title_h);
            let w = (b.2 - b.0 + 2.0 * pad).max(tw);
            sub_box[si] = Some((x0, y0, w, b.3 + pad - y0));
        }
    }

    // Normalize to the bounding box.
    let mut bb = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    let mut grow = |x: f32, y: f32| {
        bb = (bb.0.min(x), bb.1.min(y), bb.2.max(x), bb.3.max(y));
    };
    for r in rects.iter().chain(sub_box.iter().flatten()) {
        grow(r.0, r.1);
        grow(r.0 + r.2, r.1 + r.3);
    }
    for p in edges.iter().flatten() {
        grow(p.0, p.1);
    }
    for b in lab_boxes.iter().flatten() {
        grow(b.0, b.1);
        grow(b.0 + b.2, b.1 + b.3);
    }
    if bb.0 > bb.2 {
        bb = (0.0, 0.0, 0.0, 0.0);
    }
    let (ox, oy) = (-bb.0 + 2.0, -bb.1 + 2.0);
    let mv = |p: (f32, f32)| (p.0 + ox, p.1 + oy);

    let mut prims = Vec::new();
    // Subgraphs (outer first).
    for &si in order.iter().rev() {
        if let Some((x0, y0, w, h)) = sub_box[si] {
            let (x0, y0) = mv((x0, y0));
            prims.push(Prim::Path(Path {
                segs: draw::rrect(x0, y0, w, h, 4.0 * s),
                fill: Some(Ink::Role(Role::ClusterFill)),
                stroke: Some(Ink::Role(Role::ClusterStroke)),
                width: s,
                dash: false,
            }));
            prims.push(Prim::Text(TextPrim {
                id: title[si].0,
                x: x0 + (w - title[si].1) / 2.0,
                y: y0 + 4.0 * s,
                color: Ink::Role(Role::Text),
            }));
        }
    }
    // Edges.
    let mut out_edges = Vec::with_capacity(edges.len());
    for (ei, pts) in edges.iter().enumerate() {
        let pts: Vec<(f32, f32)> = pts.iter().map(|&p| mv(p)).collect();
        let e = &f.edges[ei];
        if pts.len() >= 2 {
            let (width, dash) = match e.line {
                Line::Solid => (1.5 * s, false),
                Line::Dotted => (1.5 * s, true),
                Line::Thick => (3.0 * s, false),
            };
            prims.push(Prim::Path(Path {
                segs: draw::smooth(&pts),
                fill: None,
                stroke: Some(Ink::Role(Role::Edge)),
                width,
                dash,
            }));
            if e.arrow {
                let k = pts.len();
                draw::head(&mut prims, pts[k - 2], pts[k - 1], Head::Arrow, s);
            }
        }
        out_edges.push(pts);
    }
    for b in lab_boxes.iter().flatten() {
        let (x0, y0) = mv((b.0, b.1));
        prims.push(Prim::Path(Path {
            segs: draw::rrect(x0, y0, b.2, b.3, 2.0 * s),
            fill: Some(Ink::Role(Role::LabelBg)),
            stroke: None,
            width: 0.0,
            dash: false,
        }));
        prims.push(Prim::Text(TextPrim {
            id: b.4,
            x: x0 + 4.0 * s,
            y: y0 + 2.0 * s,
            color: Ink::Role(Role::Text),
        }));
    }
    // Nodes.
    let mut boxes = Vec::with_capacity(n);
    for (i, nd) in f.nodes.iter().enumerate() {
        let (x0, y0) = mv((rects[i].0, rects[i].1));
        let (w, h) = (rects[i].2, rects[i].3);
        boxes.push((x0, y0, w, h));
        let col = f.colors(i);
        let fill = Some(col.fill.map_or(Ink::Role(Role::NodeFill), Ink::Rgb));
        let stroke = Some(col.stroke.map_or(Ink::Role(Role::NodeStroke), Ink::Rgb));
        let (cx, cy) = (x0 + w / 2.0, y0 + h / 2.0);
        let shape = |segs: Vec<Seg>| {
            Prim::Path(Path {
                segs,
                fill,
                stroke,
                width: s,
                dash: false,
            })
        };
        match nd.shape {
            // B.3.6: rect nodes have radius 6; round nodes are rounder.
            Shape::Rect => prims.push(shape(draw::rrect(x0, y0, w, h, 6.0 * s))),
            Shape::Round => prims.push(shape(draw::rrect(x0, y0, w, h, 14.0 * s))),
            Shape::Stadium => prims.push(shape(draw::rrect(x0, y0, w, h, h / 2.0))),
            Shape::Circle => prims.push(shape(draw::ellipse(cx, cy, w / 2.0, h / 2.0))),
            Shape::Rhombus => prims.push(shape(draw::poly(
                &[(cx, y0), (x0 + w, cy), (cx, y0 + h), (x0, cy)],
                true,
            ))),
            Shape::Subroutine => {
                prims.push(shape(draw::rrect(x0, y0, w, h, 0.0)));
                let mut segs = draw::poly(&[(x0 + 8.0 * s, y0), (x0 + 8.0 * s, y0 + h)], false);
                segs.extend(draw::poly(
                    &[(x0 + w - 8.0 * s, y0), (x0 + w - 8.0 * s, y0 + h)],
                    false,
                ));
                prims.push(Prim::Path(Path {
                    segs,
                    fill: None,
                    stroke,
                    width: s,
                    dash: false,
                }));
            }
            Shape::Cylinder => {
                let ry = 6.0 * s;
                let (x1, y1) = (x0 + w, y0 + h);
                let (kx, ky) = (w / 2.0 * 0.552_284_8, ry * 0.552_284_8);
                prims.push(shape(vec![
                    Seg::M(x0, y0 + ry),
                    Seg::L(x0, y1 - ry),
                    Seg::C(x0, y1 - ry + ky, cx - kx, y1, cx, y1),
                    Seg::C(cx + kx, y1, x1, y1 - ry + ky, x1, y1 - ry),
                    Seg::L(x1, y0 + ry),
                    Seg::C(x1, y0 + ry - ky, cx + kx, y0, cx, y0),
                    Seg::C(cx - kx, y0, x0, y0 + ry - ky, x0, y0 + ry),
                    Seg::Z,
                ]));
                prims.push(Prim::Path(Path {
                    segs: vec![
                        Seg::M(x0, y0 + ry),
                        Seg::C(x0, y0 + ry + ky, cx - kx, y0 + 2.0 * ry, cx, y0 + 2.0 * ry),
                        Seg::C(cx + kx, y0 + 2.0 * ry, x1, y0 + ry + ky, x1, y0 + ry),
                    ],
                    fill: None,
                    stroke,
                    width: s,
                    dash: false,
                }));
            }
        }
        let t = geo[i].text;
        let ty = if nd.shape == Shape::Cylinder {
            cy - t.2 / 2.0 + 3.0 * s
        } else {
            cy - t.2 / 2.0
        };
        prims.push(Prim::Text(TextPrim {
            id: t.0,
            x: cx - t.1 / 2.0,
            y: ty,
            color: col.color.map_or(Ink::Role(Role::Text), Ink::Rgb),
        }));
    }
    Drawing {
        w: bb.2 - bb.0 + 4.0,
        h: bb.3 - bb.1 + 4.0,
        prims,
        boxes,
        edges: out_edges,
        complex: false,
    }
}
