//! Transient UI (prompt 013 §3.1–§3.4): tooltip and popover placement, the banner stack and the
//! fullscreen viewer's zoom math. Pure state + geometry; the platform layer draws.

/// Tooltip delay (hover or keyboard focus), ms (B.10.6).
pub const TIP_DELAY_MS: u32 = 500;
/// Tooltip text size (px).
pub const TIP_SIZE: f32 = 12.0;
/// Tooltip padding (vertical, horizontal).
pub const TIP_PAD: (f32, f32) = (4.0, 8.0);
/// Tooltip corner radius.
pub const TIP_RADIUS: f32 = 4.0;
/// Tooltip maximum width.
pub const TIP_MAX_W: f32 = 360.0;
/// Footnote preview length (characters).
pub const FOOT_PREVIEW: usize = 300;
/// Popover width and radius.
pub const POP_W: f32 = 360.0;
/// Popover corner radius.
pub const POP_RADIUS: f32 = 8.0;
/// Banner height.
pub const BANNER_H: f32 = 36.0;
/// Banners shown at once.
pub const MAX_BANNERS: usize = 2;

/// Axis-aligned rectangle (x, y, w, h) in screen or window units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bx {
    /// Left.
    pub x: f32,
    /// Top.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

impl Bx {
    /// New box.
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
}

/// Places a `w × h` box below `anchor` (gap `gap`), flipping above when it would leave
/// `screen`, then shifting horizontally to stay inside. Used by tooltips and popovers.
pub fn place(anchor: Bx, w: f32, h: f32, gap: f32, screen: Bx) -> Bx {
    let below = anchor.y + anchor.h + gap;
    let y = if below + h <= screen.y + screen.h || anchor.y - gap - h < screen.y {
        below.min((screen.y + screen.h - h).max(screen.y))
    } else {
        anchor.y - gap - h
    };
    let x = anchor.x.min(screen.x + screen.w - w).max(screen.x);
    Bx::new(x, y, w, h)
}

/// Shortens `s` to at most `n` characters, ending with `…` when cut (footnote preview).
pub fn preview(s: &str, n: usize) -> String {
    let t = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() <= n {
        return t;
    }
    let mut out: String = t.chars().take(n.saturating_sub(1)).collect();
    out = out.trim_end().to_string();
    out.push('…');
    out
}

/// A banner reason (B.10.5 + P-42), drawn with its own message and severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BannerKind {
    /// The file changed on disk while there are unsaved edits.
    Conflict,
    /// The file was deleted on disk.
    Deleted,
    /// The file is read-only.
    ReadOnly,
    /// Decoding fell back (not valid UTF-8).
    Decoding,
    /// Mark of the Web: links always ask.
    Motw,
    /// Running elevated.
    Elevated,
    /// An export failed.
    ExportFailed,
    /// A recovery journal of an earlier session exists (prompt 015 §3.6).
    Recovery,
}

impl BannerKind {
    /// Error (red) rather than warning (amber) styling.
    pub fn is_error(self) -> bool {
        matches!(self, Self::Conflict | Self::Deleted | Self::ExportFailed)
    }
}

/// The banner stack: newest first, at most [`MAX_BANNERS`] visible.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Banners {
    items: Vec<BannerKind>,
}

impl Banners {
    /// Shows `k` (moving it to the top when already present); `true` when the visible set
    /// changed.
    pub fn push(&mut self, k: BannerKind) -> bool {
        let before = self.visible();
        self.items.retain(|&x| x != k);
        self.items.insert(0, k);
        before != self.visible()
    }

    /// Removes `k`; `true` when it was there.
    pub fn dismiss(&mut self, k: BannerKind) -> bool {
        let n = self.items.len();
        self.items.retain(|&x| x != k);
        n != self.items.len()
    }

    /// Visible banners, newest on top.
    pub fn visible(&self) -> Vec<BannerKind> {
        self.items.iter().take(MAX_BANNERS).copied().collect()
    }

    /// `k` is in the stack (shown or queued).
    pub fn has(&self, k: BannerKind) -> bool {
        self.items.contains(&k)
    }

    /// Height the content moves down by.
    pub fn height(&self) -> f32 {
        self.items.len().min(MAX_BANNERS) as f32 * BANNER_H
    }

    /// Visible banner at y (0 = top of the banner area).
    pub fn at(&self, y: f32) -> Option<BannerKind> {
        if y < 0.0 {
            return None;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let i = (y / BANNER_H) as usize;
        self.visible().get(i).copied()
    }
}

/// Zoom limits of the fullscreen viewer (B.11).
pub const ZOOM_MIN: f32 = 0.10;
/// Maximum zoom.
pub const ZOOM_MAX: f32 = 8.0;
/// Zoom step per wheel notch / key press.
pub const ZOOM_STEP: f32 = 1.1;
/// Fit leaves this fraction of the viewport at most.
pub const FIT_FRACTION: f32 = 0.92;

/// Pan/zoom state of a fullscreen viewer: content point (u, v) shows at
/// (ox + u·scale, oy + v·scale) in the viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoomView {
    /// Content size at 100 %.
    pub content: (f32, f32),
    /// Viewport size.
    pub view: (f32, f32),
    /// Scale (1.0 = 100 %).
    pub scale: f32,
    /// Content origin in the viewport.
    pub ox: f32,
    /// Content origin in the viewport.
    pub oy: f32,
}

impl ZoomView {
    /// Opens at fit.
    pub fn new(content: (f32, f32), view: (f32, f32)) -> Self {
        let mut z = Self {
            content,
            view,
            scale: 1.0,
            ox: 0.0,
            oy: 0.0,
        };
        z.fit();
        z
    }

    /// Fit scale: the whole content in ≤ 92 % of the viewport, never above 100 %.
    pub fn fit_scale(&self) -> f32 {
        let (cw, ch) = (self.content.0.max(1.0), self.content.1.max(1.0));
        (self.view.0 * FIT_FRACTION / cw)
            .min(self.view.1 * FIT_FRACTION / ch)
            .min(1.0)
            .clamp(ZOOM_MIN, ZOOM_MAX)
    }

    /// Centers the content at `scale`.
    pub fn center(&mut self, scale: f32) {
        self.scale = scale.clamp(ZOOM_MIN, ZOOM_MAX);
        self.ox = (self.view.0 - self.content.0 * self.scale) / 2.0;
        self.oy = (self.view.1 - self.content.1 * self.scale) / 2.0;
    }

    /// Fit (key `0`, toolbar [fit]).
    pub fn fit(&mut self) {
        let s = self.fit_scale();
        self.center(s);
    }

    /// 100 % (toolbar [1:1]).
    pub fn actual(&mut self) {
        self.center(1.0);
    }

    /// At fit (within rounding).
    pub fn is_fit(&self) -> bool {
        (self.scale - self.fit_scale()).abs() < 1e-4
    }

    /// Double-click: toggles fit / 100 %.
    pub fn toggle(&mut self) {
        if self.is_fit() && (self.scale - 1.0).abs() > 1e-4 {
            self.actual();
        } else {
            self.fit();
        }
    }

    /// Zooms by `factor` keeping viewport point (px, py) fixed.
    pub fn zoom_at(&mut self, factor: f32, px: f32, py: f32) {
        let s = (self.scale * factor).clamp(ZOOM_MIN, ZOOM_MAX);
        let (u, v) = self.to_content(px, py);
        self.scale = s;
        self.ox = px - u * s;
        self.oy = py - v * s;
    }

    /// Wheel: `notches` > 0 zooms in (×1.1 each) at the cursor.
    pub fn wheel(&mut self, notches: f32, px: f32, py: f32) {
        self.zoom_at(ZOOM_STEP.powf(notches), px, py);
    }

    /// Keys `+` / `-` and toolbar buttons: zoom at the viewport center.
    pub fn step(&mut self, dir: i32) {
        let f = if dir > 0 { ZOOM_STEP } else { 1.0 / ZOOM_STEP };
        self.zoom_at(f, self.view.0 / 2.0, self.view.1 / 2.0);
    }

    /// Drag pan.
    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.ox += dx;
        self.oy += dy;
    }

    /// Viewport → content coordinates.
    pub fn to_content(&self, px: f32, py: f32) -> (f32, f32) {
        ((px - self.ox) / self.scale, (py - self.oy) / self.scale)
    }

    /// Viewport resize: keeps the content point at the old center centered.
    pub fn resize(&mut self, view: (f32, f32)) {
        let (u, v) = self.to_content(self.view.0 / 2.0, self.view.1 / 2.0);
        self.view = view;
        self.ox = view.0 / 2.0 - u * self.scale;
        self.oy = view.1 / 2.0 - v * self.scale;
    }

    /// Zoom in whole percent (toolbar label).
    pub fn percent(&self) -> u32 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let p = (self.scale * 100.0).round().max(0.0) as u32;
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banners_stack_newest_first_max_two() {
        let mut b = Banners::default();
        assert!(b.push(BannerKind::Motw));
        assert!(b.push(BannerKind::ReadOnly));
        assert_eq!(b.visible(), [BannerKind::ReadOnly, BannerKind::Motw]);
        assert!(b.push(BannerKind::Decoding));
        assert_eq!(b.visible(), [BannerKind::Decoding, BannerKind::ReadOnly]);
        assert_eq!(b.height(), 72.0);
        // Re-pushing the top one changes nothing visible.
        assert!(!b.push(BannerKind::Decoding));
        assert_eq!(b.at(40.0), Some(BannerKind::ReadOnly));
        assert_eq!(b.at(80.0), None);
        assert!(b.dismiss(BannerKind::Decoding));
        // The queued one comes back.
        assert_eq!(b.visible(), [BannerKind::ReadOnly, BannerKind::Motw]);
        assert!(!b.dismiss(BannerKind::Elevated));
        b.dismiss(BannerKind::ReadOnly);
        b.dismiss(BannerKind::Motw);
        assert_eq!(b.height(), 0.0);
        assert!(BannerKind::Conflict.is_error() && !BannerKind::Motw.is_error());
    }

    #[test]
    fn zoom_fit_and_cursor_anchor() {
        let mut z = ZoomView::new((2000.0, 1000.0), (1000.0, 800.0));
        assert!((z.scale - 0.46).abs() < 1e-4);
        assert!(z.is_fit());
        // Small images open at 100 %, centered.
        let s = ZoomView::new((100.0, 50.0), (1000.0, 800.0));
        assert_eq!((s.scale, s.ox, s.oy), (1.0, 450.0, 375.0));
        // The content point under the cursor stays put.
        let before = z.to_content(300.0, 200.0);
        z.wheel(3.0, 300.0, 200.0);
        let after = z.to_content(300.0, 200.0);
        assert!((before.0 - after.0).abs() < 1e-3 && (before.1 - after.1).abs() < 1e-3);
        assert!((z.scale - 0.46 * 1.331).abs() < 1e-3);
        // Clamps 10–800 %.
        z.wheel(100.0, 0.0, 0.0);
        assert_eq!(z.percent(), 800);
        z.wheel(-200.0, 0.0, 0.0);
        assert_eq!(z.percent(), 10);
        z.fit();
        z.toggle();
        assert_eq!(z.percent(), 100);
        z.toggle();
        assert!(z.is_fit());
        z.pan(10.0, -5.0);
        z.step(1);
        z.step(-1);
        assert!((z.scale - 0.46).abs() < 1e-4);
        z.resize((500.0, 400.0));
        assert_eq!(z.view, (500.0, 400.0));
    }

    #[test]
    fn placement_flips_and_clamps() {
        let screen = Bx::new(0.0, 0.0, 800.0, 600.0);
        let p = place(Bx::new(100.0, 100.0, 50.0, 20.0), 200.0, 40.0, 6.0, screen);
        assert_eq!((p.x, p.y), (100.0, 126.0));
        let p = place(Bx::new(700.0, 560.0, 50.0, 20.0), 200.0, 40.0, 6.0, screen);
        assert_eq!((p.x, p.y), (600.0, 514.0));
        assert_eq!(preview("a  b\nc", 300), "a b c");
        let long = "word ".repeat(100);
        let p = preview(&long, 300);
        assert!(p.chars().count() <= 300 && p.ends_with('…'));
    }

    #[test]
    fn dismiss_reveals_the_older_banner_and_hit_rows() {
        let mut b = Banners::default();
        b.push(BannerKind::Motw);
        b.push(BannerKind::ReadOnly);
        b.push(BannerKind::Deleted);
        assert_eq!(b.visible(), vec![BannerKind::Deleted, BannerKind::ReadOnly]);
        assert!(b.dismiss(BannerKind::Deleted));
        assert_eq!(b.visible(), vec![BannerKind::ReadOnly, BannerKind::Motw]);
        assert_eq!(b.at(1.0), Some(BannerKind::ReadOnly));
        assert_eq!(b.at(BANNER_H + 1.0), Some(BannerKind::Motw));
        assert_eq!(b.at(2.0 * BANNER_H + 1.0), None);
        assert!(!b.dismiss(BannerKind::Deleted));
        assert!(BannerKind::Conflict.is_error() && !BannerKind::Motw.is_error());
    }

    #[test]
    fn zoom_limits_and_steps() {
        let mut z = ZoomView::new((400.0, 300.0), (1000.0, 700.0));
        assert!(z.is_fit() && z.percent() == 100);
        for _ in 0..100 {
            z.step(1);
        }
        assert!((z.scale - ZOOM_MAX).abs() < 1e-4);
        for _ in 0..200 {
            z.step(-1);
        }
        assert!((z.scale - ZOOM_MIN).abs() < 1e-4);
        z.actual();
        assert_eq!(z.percent(), 100);
        z.toggle();
        assert!(z.is_fit());
    }
}
