//! Animated GIF timing and frame disposal (R-IMG-01, FUT-20): pure, platform-free.
//!
//! The platform decodes each frame (WIC) into frame-sized premultiplied BGRA and hands it to
//! [`Canvas::draw`]; [`Player`] decides when the next frame is due.

/// S-RES-04: frames beyond this are ignored (the animation loops over the first 1 000).
pub const MAX_FRAMES: usize = 1000;

/// GIF disposal method of a frame (graphic control extension).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Disposal {
    /// 0: unspecified (treated as keep).
    #[default]
    None,
    /// 1: leave the frame in place.
    Keep,
    /// 2: clear the frame's rectangle to transparent.
    Background,
    /// 3: restore what was under the frame.
    Previous,
}

impl Disposal {
    /// From the GIF field value.
    #[must_use]
    pub fn from_gif(v: u8) -> Self {
        match v {
            1 => Self::Keep,
            2 => Self::Background,
            3 => Self::Previous,
            _ => Self::None,
        }
    }
}

/// Frame delay in ms from GIF centiseconds; 0 and 1 cs play at 100 ms like browsers do.
#[must_use]
pub fn delay_ms(cs: u16) -> u32 {
    if cs <= 1 { 100 } else { u32::from(cs) * 10 }
}

/// A frame rectangle on the logical screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    /// Left.
    pub x: u32,
    /// Top.
    pub y: u32,
    /// Width.
    pub w: u32,
    /// Height.
    pub h: u32,
}

/// The composited logical screen (premultiplied BGRA, `w * h * 4`).
#[derive(Debug, Clone, Default)]
pub struct Canvas {
    /// Width.
    pub w: u32,
    /// Height.
    pub h: u32,
    /// Pixels.
    pub px: Vec<u8>,
    /// Previous frame: its rectangle (clipped), disposal and, for `Previous`, what was under it.
    last: Option<(Rect, Disposal, Vec<u8>)>,
}

impl Canvas {
    /// A transparent canvas.
    #[must_use]
    pub fn new(w: u32, h: u32) -> Self {
        Self {
            w,
            h,
            px: vec![0; (w as usize) * (h as usize) * 4],
            last: None,
        }
    }

    /// Clears to transparent (the animation restarts).
    pub fn reset(&mut self) {
        self.px.fill(0);
        self.last = None;
    }

    /// Bytes held besides the canvas (the saved `Previous` region).
    #[must_use]
    pub fn extra_bytes(&self) -> usize {
        self.last.as_ref().map_or(0, |l| l.2.len())
    }

    fn clip(&self, r: Rect) -> Rect {
        let x = r.x.min(self.w);
        let y = r.y.min(self.h);
        Rect {
            x,
            y,
            w: r.w.min(self.w - x),
            h: r.h.min(self.h - y),
        }
    }

    fn row(&self, r: Rect, j: u32) -> std::ops::Range<usize> {
        let a = (((r.y + j) * self.w + r.x) * 4) as usize;
        a..a + (r.w * 4) as usize
    }

    /// Disposes the previous frame, then draws `src` (`r.w * r.h * 4` BGRA; alpha 0 is
    /// transparent) at `r` with `disposal` applied before the next frame.
    pub fn draw(&mut self, r: Rect, disposal: Disposal, src: &[u8]) {
        if let Some((pr, d, saved)) = self.last.take() {
            match d {
                Disposal::Background => {
                    for j in 0..pr.h {
                        let rg = self.row(pr, j);
                        if let Some(s) = self.px.get_mut(rg) {
                            s.fill(0);
                        }
                    }
                }
                Disposal::Previous => {
                    let n = (pr.w * 4) as usize;
                    for (j, line) in (0..pr.h).zip(saved.chunks(n.max(1))) {
                        let rg = self.row(pr, j);
                        if let Some(s) = self.px.get_mut(rg) {
                            s.copy_from_slice(line);
                        }
                    }
                }
                Disposal::None | Disposal::Keep => {}
            }
        }
        let c = self.clip(r);
        let mut saved = Vec::new();
        if disposal == Disposal::Previous {
            for j in 0..c.h {
                if let Some(s) = self.px.get(self.row(c, j)) {
                    saved.extend_from_slice(s);
                }
            }
        }
        let sw = (r.w * 4) as usize;
        for j in 0..c.h {
            let rg = self.row(c, j);
            let Some(line) = src.get(j as usize * sw..j as usize * sw + (c.w * 4) as usize) else {
                break;
            };
            let Some(dst) = self.px.get_mut(rg) else {
                break;
            };
            for (d, s) in dst
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(line.as_chunks::<4>().0)
            {
                if s[3] != 0 {
                    *d = *s;
                }
            }
        }
        self.last = Some((c, disposal, saved));
    }
}

/// Frame clock of one animation (ms timestamps from any monotonic source).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    /// Current frame.
    pub idx: usize,
    /// Frame count (≥ 1, ≤ [`MAX_FRAMES`]).
    pub frames: usize,
    /// When the next frame is due.
    pub due: u64,
}

impl Player {
    /// Starts at frame 0, shown at `now` for `first_delay` ms.
    #[must_use]
    pub fn new(frames: usize, first_delay: u32, now: u64) -> Self {
        Self {
            idx: 0,
            frames: frames.clamp(1, MAX_FRAMES),
            due: now + u64::from(first_delay),
        }
    }

    /// Advances one frame when due (no catch-up after a pause: the next delay counts from
    /// `now`). Returns the new index, `0` meaning the loop restarts.
    pub fn tick(&mut self, now: u64, delay_of: impl Fn(usize) -> u32) -> Option<usize> {
        if self.frames < 2 || now < self.due {
            return None;
        }
        self.idx = (self.idx + 1) % self.frames;
        self.due = now + u64::from(delay_of(self.idx));
        Some(self.idx)
    }

    /// Restarts the clock after a pause (hidden, inactive): the current frame stays.
    pub fn resume(&mut self, now: u64, delay: u32) {
        self.due = self.due.max(now).min(now + u64::from(delay));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(c: u8, n: usize) -> Vec<u8> {
        [c, c, c, 255].repeat(n)
    }

    #[test]
    fn timing_and_disposal() {
        // Timing: browser clamp, wrap, no catch-up, single frame never ticks.
        assert_eq!(
            (delay_ms(0), delay_ms(1), delay_ms(2), delay_ms(7)),
            (100, 100, 20, 70)
        );
        assert_eq!(Disposal::from_gif(3), Disposal::Previous);
        assert_eq!(Disposal::from_gif(9), Disposal::None);
        let d = |i: usize| [50, 30, 20][i];
        let mut p = Player::new(3, 50, 1000);
        assert_eq!(p.tick(1049, d), None);
        assert_eq!(p.tick(1050, d), Some(1));
        assert_eq!(p.due, 1080);
        assert_eq!(p.tick(5000, d), Some(2));
        assert_eq!(p.due, 5020);
        assert_eq!(p.tick(5020, d), Some(0));
        p.resume(9000, 50);
        assert_eq!(p.due, 9000);
        assert_eq!(Player::new(1, 10, 0).tick(99, d), None);
        assert_eq!(Player::new(5000, 10, 0).frames, MAX_FRAMES);

        // Disposal on a 4×2 canvas.
        let mut c = Canvas::new(4, 2);
        let full = Rect {
            x: 0,
            y: 0,
            w: 4,
            h: 2,
        };
        c.draw(full, Disposal::Keep, &px(10, 8));
        // Frame 2 at (1,0) 2×1 with "Previous": the next frame restores the 10s.
        c.draw(
            Rect {
                x: 1,
                y: 0,
                w: 2,
                h: 1,
            },
            Disposal::Previous,
            &px(20, 2),
        );
        assert_eq!(c.px[4], 20);
        assert_eq!(c.extra_bytes(), 8);
        // Transparent pixel keeps what is below; out-of-canvas part is clipped.
        let mut f = px(30, 2);
        f[3] = 0;
        c.draw(
            Rect {
                x: 3,
                y: 1,
                w: 2,
                h: 1,
            },
            Disposal::Background,
            &f,
        );
        assert_eq!(c.px[4], 10, "Previous restored");
        assert_eq!(c.px[(4 + 3) * 4], 10, "transparent pixel shows the canvas");
        c.draw(
            Rect {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            },
            Disposal::None,
            &px(40, 1),
        );
        assert_eq!(
            c.px[(4 + 3) * 4 + 3],
            0,
            "Background cleared the previous rect"
        );
        assert_eq!(c.px[0], 40);
        c.reset();
        assert!(c.px.iter().all(|&b| b == 0));
    }
}
