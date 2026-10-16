//! Fenwick tree over block heights (§4.4.4).

/// Binary indexed tree of `f64` heights with point updates and prefix sums.
#[derive(Debug, Clone, Default)]
pub struct Fenwick {
    tree: Vec<f64>,
    values: Vec<f64>,
}

impl Fenwick {
    /// Builds from heights in O(n).
    pub fn from_heights(h: &[f64]) -> Self {
        let n = h.len();
        let mut tree = vec![0.0; n + 1];
        for (i, v) in h.iter().enumerate() {
            let k = i + 1;
            if let Some(t) = tree.get_mut(k) {
                *t += v;
            }
            let parent = k + k.isolate_lowest_one();
            if parent <= n {
                let tk = tree.get(k).copied().unwrap_or(0.0);
                if let Some(t) = tree.get_mut(parent) {
                    *t += tk;
                }
            }
        }
        Self {
            tree,
            values: h.to_vec(),
        }
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Height of entry `i`.
    pub fn get(&self, i: usize) -> f64 {
        self.values.get(i).copied().unwrap_or(0.0)
    }

    /// Sets entry `i` to `v`.
    pub fn set(&mut self, i: usize, v: f64) {
        let Some(old) = self.values.get_mut(i) else {
            return;
        };
        let d = v - *old;
        *old = v;
        let mut k = i + 1;
        while let Some(t) = self.tree.get_mut(k) {
            *t += d;
            k += k.isolate_lowest_one();
        }
    }

    /// Sum of entries `0..i`.
    pub fn prefix(&self, i: usize) -> f64 {
        let mut k = i.min(self.values.len());
        let mut s = 0.0;
        while k > 0 {
            s += self.tree.get(k).copied().unwrap_or(0.0);
            k &= k - 1;
        }
        s
    }

    /// Total.
    pub fn total(&self) -> f64 {
        self.prefix(self.values.len())
    }

    /// Index of the entry containing offset `y` (clamped to the last entry).
    pub fn find(&self, y: f64) -> usize {
        let n = self.values.len();
        if n == 0 {
            return 0;
        }
        let mut pos = 0usize;
        let mut rem = y;
        let mut step = n.next_power_of_two();
        while step > 0 {
            let next = pos + step;
            if next <= n
                && let Some(&t) = self.tree.get(next)
                && t <= rem
            {
                pos = next;
                rem -= t;
            }
            step >>= 1;
        }
        pos.min(n - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_find_update() {
        let h: Vec<f64> = (0..100).map(|i| f64::from(i % 7 + 1)).collect();
        let mut f = Fenwick::from_heights(&h);
        let mut acc = 0.0;
        for (i, v) in h.iter().enumerate() {
            assert!((f.prefix(i) - acc).abs() < 1e-9);
            assert_eq!(f.find(acc), i);
            assert_eq!(f.find(acc + v - 0.01), i);
            acc += v;
        }
        assert_eq!(f.find(1e9), 99);
        f.set(10, 100.0);
        assert!((f.total() - (acc - h[10] + 100.0)).abs() < 1e-9);
        assert_eq!(f.find(f.prefix(10) + 50.0), 10);
    }
}
