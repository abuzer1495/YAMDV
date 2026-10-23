//! Keyboard focus order of a laid-out block (HARD-04, prompt 014 §3.7). Pure.
//!
//! Tab walks the interactive parts of the document in reading order: links, code toolbar
//! buttons, task boxes, callout / details / front-matter toggles, footnote links, diagram ⛶
//! badges, table pills and placeholder buttons. A heading's fold toggle comes first in its block.

use crate::block::{Hot, HotKind};
use crate::geom::Rect;

/// A focus stop inside one block.
#[derive(Debug, Clone, PartialEq)]
pub enum Stop {
    /// The heading fold toggle in the gutter.
    Toggle,
    /// An interactive rectangle.
    Hot(HotKind),
}

/// A stop and its first rectangle (block coordinates).
#[derive(Debug, Clone, PartialEq)]
pub struct At {
    /// What.
    pub stop: Stop,
    /// Where (the first piece of a wrapped link).
    pub rect: Rect,
}

/// The hot region takes keyboard focus.
pub fn focusable(k: &HotKind) -> bool {
    matches!(
        k,
        HotKind::Link { .. }
            | HotKind::CodeCopy { .. }
            | HotKind::CodeLines { .. }
            | HotKind::CodeWrap { .. }
            | HotKind::Task { .. }
            | HotKind::Fold { .. }
            | HotKind::FootRef { .. }
            | HotKind::FootBack { .. }
            | HotKind::Diagram { .. }
            | HotKind::Table { .. }
            | HotKind::RenderAnyway { .. }
            | HotKind::CopyUrl { .. }
    )
}

/// Rows closer than this (DIP) count as one line.
const ROW_TOL: f32 = 4.0;

/// Focus stops of one block in reading order: top to bottom (rows within [`ROW_TOL`]), then
/// left to right; a link wrapped over several lines is one stop at its first piece. `toggle`
/// is the heading fold toggle rectangle, if the block has one.
pub fn order(hots: &[Hot], toggle: Option<Rect>) -> Vec<At> {
    let mut v: Vec<At> = Vec::new();
    for h in hots.iter().filter(|h| focusable(&h.kind)) {
        let dup = match &h.kind {
            HotKind::Link { link } => v
                .iter()
                .any(|a| matches!(&a.stop, Stop::Hot(HotKind::Link { link: l }) if l == link)),
            k => v.iter().any(|a| a.stop == Stop::Hot(k.clone())),
        };
        if !dup {
            v.push(At {
                stop: Stop::Hot(h.kind.clone()),
                rect: h.rect,
            });
        }
    }
    v.sort_by(|a, b| {
        let (ay, by) = (a.rect.y, b.rect.y);
        if (ay - by).abs() <= ROW_TOL {
            a.rect.x.total_cmp(&b.rect.x)
        } else {
            ay.total_cmp(&by)
        }
    });
    if let Some(r) = toggle {
        v.insert(
            0,
            At {
                stop: Stop::Toggle,
                rect: r,
            },
        );
    }
    v
}

/// Index of `stop` in `stops` (links match by index alone).
pub fn position(stops: &[At], stop: &Stop) -> Option<usize> {
    stops.iter().position(|a| match (&a.stop, stop) {
        (Stop::Hot(HotKind::Link { link: x }), Stop::Hot(HotKind::Link { link: y })) => x == y,
        (x, y) => x == y,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hot(kind: HotKind, x: f32, y: f32) -> Hot {
        Hot {
            rect: Rect::new(x, y, 10.0, 10.0),
            kind,
        }
    }

    #[test]
    fn reading_order_and_dedup() {
        let hots = vec![
            hot(HotKind::Link { link: 1 }, 50.0, 40.0),
            hot(HotKind::CodeWrap { node: 0 }, 300.0, 2.0),
            hot(HotKind::CodeCopy { node: 0 }, 200.0, 0.0),
            hot(HotKind::Link { link: 0 }, 400.0, 20.0),
            hot(HotKind::Link { link: 0 }, 0.0, 40.0),
            hot(HotKind::FootNote { n: 1 }, 0.0, 0.0),
            hot(HotKind::Task { item: 9 }, 0.0, 21.0),
        ];
        let o = order(&hots, None);
        let k: Vec<Stop> = o.iter().map(|a| a.stop.clone()).collect();
        assert_eq!(
            k,
            [
                Stop::Hot(HotKind::CodeCopy { node: 0 }),
                Stop::Hot(HotKind::CodeWrap { node: 0 }),
                Stop::Hot(HotKind::Task { item: 9 }),
                Stop::Hot(HotKind::Link { link: 0 }),
                Stop::Hot(HotKind::Link { link: 1 }),
            ]
        );
        assert_eq!(o[3].rect.x, 400.0);
        assert_eq!(position(&o, &Stop::Hot(HotKind::Link { link: 1 })), Some(4));
        assert_eq!(position(&o, &Stop::Toggle), None);
    }

    #[test]
    fn toggle_first() {
        let hots = vec![hot(HotKind::Link { link: 0 }, 5.0, 0.0)];
        let o = order(&hots, Some(Rect::new(-20.0, 0.0, 16.0, 16.0)));
        assert_eq!(o.len(), 2);
        assert_eq!(o[0].stop, Stop::Toggle);
        assert!(order(&[hot(HotKind::FootNote { n: 2 }, 0.0, 0.0)], None).is_empty());
    }
}
