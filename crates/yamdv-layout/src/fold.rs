//! Heading folding (R-READ-06, prompt 013 §3.5): pure range math over top-level blocks.
//!
//! A fold on heading block `h` (level 1–3) hides every following block up to the next heading of
//! the same or a higher level (smaller number). State is session-only and lives in [`Folds`].

use std::ops::Range;

/// Highest heading level that gets a fold toggle (H1–H3).
pub const MAX_FOLD_LEVEL: u8 = 3;
/// Fold / unfold animation length in ms (instant with reduced motion).
pub const FOLD_ANIM_MS: u32 = 120;

/// Blocks hidden by folding heading `h`; `levels[i]` is block `i`'s heading level (if any).
/// Empty when `h` is not a foldable heading.
pub fn fold_range(levels: &[Option<u8>], h: usize) -> Range<usize> {
    let Some(&Some(l)) = levels.get(h) else {
        return h..h;
    };
    if l == 0 || l > MAX_FOLD_LEVEL {
        return h..h;
    }
    let end = levels
        .iter()
        .enumerate()
        .skip(h + 1)
        .find(|(_, v)| v.is_some_and(|v| v <= l))
        .map_or(levels.len(), |(i, _)| i);
    h + 1..end
}

/// Folded headings of one open document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Folds {
    folded: Vec<usize>,
}

impl Folds {
    /// Nothing folded.
    pub fn is_empty(&self) -> bool {
        self.folded.is_empty()
    }

    /// Heading `h` is folded.
    pub fn is_folded(&self, h: usize) -> bool {
        self.folded.contains(&h)
    }

    /// Folded heading blocks (sorted).
    pub fn folded(&self) -> &[usize] {
        &self.folded
    }

    /// Toggles heading `h`; returns the new folded state.
    pub fn toggle(&mut self, levels: &[Option<u8>], h: usize) -> bool {
        if let Some(p) = self.folded.iter().position(|&x| x == h) {
            self.folded.remove(p);
            return false;
        }
        if fold_range(levels, h).is_empty() {
            return false;
        }
        self.folded.push(h);
        self.folded.sort_unstable();
        true
    }

    /// Alt+click: folds (or, when all are folded, unfolds) every heading of `level`.
    pub fn toggle_level(&mut self, levels: &[Option<u8>], level: u8) {
        let all: Vec<usize> = (0..levels.len())
            .filter(|&i| levels[i] == Some(level) && !fold_range(levels, i).is_empty())
            .collect();
        if all.iter().all(|i| self.folded.contains(i)) {
            self.folded.retain(|i| !all.contains(i));
        } else {
            for i in all {
                if !self.folded.contains(&i) {
                    self.folded.push(i);
                }
            }
            self.folded.sort_unstable();
        }
    }

    /// Unfolds every fold whose hidden range contains block `b` (find auto-expand); `true` when
    /// something changed.
    pub fn reveal(&mut self, levels: &[Option<u8>], b: usize) -> bool {
        let n = self.folded.len();
        self.folded.retain(|&h| !fold_range(levels, h).contains(&b));
        n != self.folded.len()
    }

    /// Clears all folds (reopen / reload / structural edit).
    pub fn clear(&mut self) {
        self.folded.clear();
    }

    /// Per-block hidden flags for `levels.len()` blocks.
    pub fn hidden(&self, levels: &[Option<u8>]) -> Vec<bool> {
        let mut out = vec![false; levels.len()];
        for &h in &self.folded {
            for i in fold_range(levels, h) {
                if let Some(v) = out.get_mut(i) {
                    *v = true;
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> Vec<Option<u8>> {
        // 0 H1, 1 p, 2 H2, 3 p, 4 H3, 5 p, 6 H2, 7 p, 8 H4, 9 p, 10 H1, 11 p
        vec![
            Some(1),
            None,
            Some(2),
            None,
            Some(3),
            None,
            Some(2),
            None,
            Some(4),
            None,
            Some(1),
            None,
        ]
    }

    #[test]
    fn ranges_stop_at_same_or_higher_level() {
        let l = doc();
        assert_eq!(fold_range(&l, 0), 1..10);
        assert_eq!(fold_range(&l, 2), 3..6);
        assert_eq!(fold_range(&l, 4), 5..6);
        assert_eq!(fold_range(&l, 6), 7..10);
        assert_eq!(fold_range(&l, 10), 11..12);
        // H4 and paragraphs do not fold.
        assert!(fold_range(&l, 8).is_empty());
        assert!(fold_range(&l, 1).is_empty());
        assert!(fold_range(&l, 99).is_empty());
    }

    #[test]
    fn toggle_level_reveal_and_hidden() {
        let l = doc();
        let mut f = Folds::default();
        assert!(f.toggle(&l, 2));
        assert_eq!(
            f.hidden(&l),
            [
                false, false, false, true, true, true, false, false, false, false, false, false
            ]
        );
        assert!(!f.toggle(&l, 2));
        assert!(f.is_empty());
        f.toggle_level(&l, 2);
        assert_eq!(f.folded(), &[2, 6]);
        assert!(f.reveal(&l, 7));
        assert_eq!(f.folded(), &[2]);
        assert!(!f.reveal(&l, 11));
        f.toggle_level(&l, 2);
        assert_eq!(f.folded(), &[2, 6]);
        f.toggle_level(&l, 2);
        assert!(f.is_empty());
        // Nested folds: H1 over a folded H2.
        f.toggle(&l, 2);
        f.toggle(&l, 0);
        assert!(f.hidden(&l)[2] && f.hidden(&l)[9] && !f.hidden(&l)[10]);
        f.clear();
        assert!(f.is_empty());
    }
}
