//! Geometry.

/// Axis-aligned rectangle in DIPs.
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
    /// New rectangle.
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// Right edge.
    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    /// Bottom edge.
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    /// Intersection (empty rectangles have zero width/height).
    pub fn intersect(&self, o: &Self) -> Self {
        let x = self.x.max(o.x);
        let y = self.y.max(o.y);
        let r = self.right().min(o.right());
        let b = self.bottom().min(o.bottom());
        Self::new(x, y, (r - x).max(0.0), (b - y).max(0.0))
    }
    /// Does `o` lie completely inside?
    pub fn contains_rect(&self, o: &Self) -> bool {
        o.x >= self.x && o.y >= self.y && o.right() <= self.right() && o.bottom() <= self.bottom()
    }
    /// Do the rectangles overlap?
    pub fn overlaps(&self, o: &Self) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }
    /// Translated copy.
    pub fn offset(&self, dx: f32, dy: f32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            ..*self
        }
    }

    /// Contains the point.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }
}

/// Band buffer cap in bytes (P-12).
pub const BAND_CAP_BYTES: usize = 524_288;

/// Band height in rows for a band `width_px` wide: `clamp(524288 / (4 × width_px), 32, 128)`.
pub fn band_rows(width_px: i32) -> i32 {
    let w = usize::try_from(width_px.max(1)).unwrap_or(1);
    i32::try_from((BAND_CAP_BYTES / (4 * w)).clamp(32, 128)).unwrap_or(32)
}

#[cfg(test)]
mod band_tests {
    use super::*;

    #[test]
    fn band_cap() {
        for (w, rows) in [(1000, 128), (1920, 68), (3840, 34), (7680, 32)] {
            assert_eq!(band_rows(w), rows, "width {w}");
            let bytes = usize::try_from(w * band_rows(w) * 4).unwrap_or(0);
            // ≤ 512 KiB except where the 32-row floor applies beyond 4096 px.
            if w <= 4096 {
                assert!(bytes <= BAND_CAP_BYTES, "{w}: {bytes}");
            }
        }
    }
}
