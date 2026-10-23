//! Sidebar keyboard (↑/↓, Enter, ←/→, Home/End) and favorites reorder (FUT-03): drag with a
//! 2 px accent drop line, or Alt+↑/↓ on the focused favorite. The order persists atomically
//! with the rest of the state file.

use super::*;
use crate::window::sidebar::{ROW_H, Row, RowKind};

/// A pressed favorite row.
#[derive(Debug, Clone)]
pub(in crate::window) struct Drag {
    path: String,
    y0: f32,
    y: f32,
    moved: bool,
}

/// Moves `path` to index `to` (in the list without it). Pure.
fn reorder(list: &mut Vec<String>, path: &str, to: usize) -> bool {
    let Some(from) = list.iter().position(|p| p == path) else {
        return false;
    };
    let item = list.remove(from);
    let to = to.min(list.len());
    list.insert(to, item);
    from != to
}

fn focusable(r: &Row) -> bool {
    matches!(
        r.kind,
        RowKind::Toc { .. } | RowKind::Hist { .. } | RowKind::Fav(_) | RowKind::File { .. }
    )
}

impl Win {
    pub(in crate::window) fn fav_press(&mut self, path: String, y: f32) {
        self.nav.fav_drag = Some(Drag {
            path,
            y0: y,
            y,
            moved: false,
        });
        // SAFETY: valid hwnd.
        unsafe { SetCapture(self.hwnd) };
    }

    /// Mouse move during a favorite press; `true` when handled.
    pub(in crate::window) fn fav_move(&mut self, y: f32) -> bool {
        let Some(d) = &mut self.nav.fav_drag else {
            return false;
        };
        d.y = y;
        if (y - d.y0).abs() > 4.0 {
            d.moved = true;
        }
        self.invalidate();
        true
    }

    /// Target index and drop-line y of the current drag.
    fn fav_target(&self, sh: &Shared) -> Option<(usize, f32)> {
        let d = self.nav.fav_drag.as_ref().filter(|d| d.moved)?;
        let favs: Vec<Rect> = self
            .side_rows(sh)
            .into_iter()
            .filter(|r| matches!(r.kind, RowKind::Fav(_)))
            .map(|r| r.rect)
            .collect();
        let i = favs.iter().filter(|r| r.y + r.h / 2.0 < d.y).count();
        let y = favs
            .get(i)
            .map(|r| r.y)
            .or_else(|| favs.last().map(|r| r.y + r.h))?;
        Some((i, y))
    }

    /// Drop line y while dragging a favorite.
    pub(in crate::window) fn fav_drop_line(&self, sh: &Shared) -> Option<f32> {
        self.fav_target(sh).map(|t| t.1)
    }

    /// Mouse up: drop (reorder) or open; `true` when a favorite press ended.
    pub(in crate::window) fn fav_up(&mut self) -> bool {
        let Some(sh) = shared() else { return false };
        let target = self.fav_target(&sh);
        let Some(d) = self.nav.fav_drag.take() else {
            return false;
        };
        // SAFETY: no preconditions.
        let _ = unsafe { ReleaseCapture() };
        if !d.moved {
            self.open_from_side(&sh, &d.path);
        } else if let Some((i, _)) = target {
            let from = app::with(|a| a.state.favorites.iter().position(|p| *p == d.path)).flatten();
            // Index in the list without the dragged item.
            let to = match from {
                Some(f) if f < i => i - 1,
                _ => i,
            };
            self.fav_set(&d.path, to);
        }
        self.invalidate();
        true
    }

    /// Moves favorite `path` to index `to` and saves.
    pub(in crate::window) fn fav_set(&mut self, path: &str, to: usize) {
        let changed = app::with(|a| reorder(&mut a.state.favorites, path, to)).unwrap_or(false);
        if changed {
            app::save_state();
        }
    }

    /// Focuses the first useful row of the sidebar (F6).
    pub(in crate::window) fn side_focus_first(&mut self, sh: &Shared) {
        let rows = self.side_rows(sh);
        let pick = rows
            .iter()
            .position(|r| {
                matches!(
                    r.kind,
                    RowKind::Toc { active: true, .. } | RowKind::File { current: true, .. }
                )
            })
            .or_else(|| rows.iter().position(focusable));
        self.nav.side_focus = pick;
    }

    /// Scrolls the sidebar list so row `i` is visible.
    fn side_reveal(&mut self, sh: &Shared, i: usize) {
        let Some(sr) = self.side_rect(sh) else { return };
        let top = sr.y + self.tabs_h(sh);
        let Some(r) = self.side_rows(sh).get(i).map(|r| r.rect) else {
            return;
        };
        if r.y < top {
            self.side.scroll = (self.side.scroll - (top - r.y) - 8.0).max(0.0);
        } else if r.y + r.h > sr.y + sr.h {
            self.side.scroll += r.y + r.h - (sr.y + sr.h) + ROW_H / 2.0;
        }
    }

    /// Sidebar region keys; `true` when consumed.
    pub(in crate::window) fn side_key(
        &mut self,
        sh: &Shared,
        vk: u16,
        ctrl: bool,
        shift: bool,
        alt: bool,
    ) -> bool {
        let rows = self.side_rows(sh);
        let cur = self
            .nav
            .side_focus
            .filter(|i| rows.get(*i).is_some_and(focusable));
        let idx: Vec<usize> = (0..rows.len())
            .filter(|i| rows.get(*i).is_some_and(focusable))
            .collect();
        let at = cur.and_then(|c| idx.iter().position(|i| *i == c));
        let row = cur.and_then(|c| rows.get(c)).cloned();
        let go = |w: &mut Self, i: Option<&usize>| {
            if let Some(&i) = i {
                w.nav.side_focus = Some(i);
                w.side_reveal(sh, i);
            }
            w.invalidate();
        };
        match vk {
            // Alt+↑/↓ on a favorite wins over column wider / narrower.
            0x26 | 0x28 if alt => {
                let Some(Row {
                    kind: RowKind::Fav(p),
                    ..
                }) = &row
                else {
                    return false;
                };
                let favs = app::with(|a| a.state.favorites.clone()).unwrap_or_default();
                if let Some(f) = favs.iter().position(|x| x == p) {
                    let to = if vk == 0x26 {
                        f.saturating_sub(1)
                    } else {
                        f + 1
                    };
                    self.fav_set(p, to);
                    let rows = self.side_rows(sh);
                    let i = rows.iter().position(|r| r.kind == RowKind::Fav(p.clone()));
                    go(self, i.as_ref());
                }
                true
            }
            0x26 | 0x28 | 0x09 if !ctrl && !alt => {
                let down = vk == 0x28 || (vk == 0x09 && !shift);
                let next = match at {
                    Some(a) if down => idx.get(a + 1),
                    Some(a) => a.checked_sub(1).and_then(|a| idx.get(a)),
                    None => idx.first(),
                };
                go(self, next.or(cur.as_ref()));
                true
            }
            0x24 => {
                go(self, idx.first());
                true
            }
            0x23 => {
                go(self, idx.last());
                true
            }
            0x0D | 0x20 => {
                match row.map(|r| r.kind) {
                    Some(RowKind::Toc { block, .. }) => self.scroll_to_block(sh, block),
                    Some(RowKind::Hist { path, .. } | RowKind::Fav(path)) => {
                        self.open_from_side(sh, &path)
                    }
                    Some(RowKind::File { node, .. }) => self.files_click(sh, node),
                    _ => {}
                }
                self.invalidate();
                true
            }
            0x25 | 0x27 => {
                let right = vk == 0x27;
                match row.map(|r| r.kind) {
                    Some(RowKind::File {
                        node, dir, open, ..
                    }) => {
                        if dir && open != right {
                            self.files_click(sh, node);
                        } else if !right && let Some(p) = self.files_parent(node) {
                            let rows = self.side_rows(sh);
                            let i = rows.iter().position(
                                |r| matches!(r.kind, RowKind::File { node, .. } if node == p),
                            );
                            go(self, i.as_ref());
                        }
                    }
                    Some(RowKind::Toc {
                        heading,
                        kids: true,
                        open,
                        ..
                    }) if open != right => {
                        let h = heading as u32;
                        if open {
                            self.side.collapsed.push(h);
                        } else {
                            self.side.collapsed.retain(|c| *c != h);
                        }
                    }
                    _ => {}
                }
                self.invalidate();
                true
            }
            0x1B => {
                self.nav.region = Region::Document;
                self.nav.side_focus = None;
                self.invalidate();
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::reorder;

    #[test]
    fn reorder_moves_and_reports() {
        let mut v: Vec<String> = ["a", "b", "c"].iter().map(|s| (*s).to_string()).collect();
        assert!(reorder(&mut v, "c", 0));
        assert_eq!(v, ["c", "a", "b"]);
        assert!(!reorder(&mut v, "c", 0));
        assert!(reorder(&mut v, "c", 9));
        assert_eq!(v, ["a", "b", "c"]);
        assert!(!reorder(&mut v, "x", 0));
    }
}
