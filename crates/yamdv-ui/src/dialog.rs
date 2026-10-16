//! In-window confirm dialog (B.10.6): width 420, title 15 px weight 600, body 13 px, buttons
//! right-aligned (primary last), `Enter` = primary, `Esc` = cancel, `Tab`/`Shift+Tab` cycle focus,
//! backdrop `rgba(0,0,0,.25)`; the focus ring is shown only after keyboard interaction (B.16).

/// Dialog width (DIPs).
pub const WIDTH: f32 = 420.0;
/// Padding.
pub const PAD: f32 = 24.0;
/// Button height.
pub const BUTTON_H: f32 = 32.0;
/// Gap between buttons.
pub const BUTTON_GAP: f32 = 8.0;
/// Horizontal padding inside a button.
pub const BUTTON_PAD_X: f32 = 14.0;
/// Title font size.
pub const TITLE_SIZE: f32 = 15.0;
/// Body font size.
pub const BODY_SIZE: f32 = 13.0;
/// Backdrop alpha (0.25).
pub const BACKDROP_ALPHA: f32 = 0.25;

/// A rectangle (DIPs).
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

/// Keys the dialog handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// Enter.
    Enter,
    /// Escape.
    Esc,
    /// Tab.
    Tab,
    /// Shift+Tab.
    ShiftTab,
    /// Space (activates the focused button).
    Space,
}

/// Outcome of input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Still open (repaint if `changed`).
    Open {
        /// Visual state changed.
        changed: bool,
    },
    /// Button `id` chosen.
    Chosen(u32),
    /// Cancelled (Esc).
    Cancelled,
}

/// A button.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Button {
    /// Label.
    pub label: String,
    /// Caller id.
    pub id: u32,
}

/// Geometry of the dialog for a given viewport.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Geometry {
    /// Card.
    pub card: Rect,
    /// Title origin.
    pub title: Rect,
    /// Body origin.
    pub body: Rect,
    /// Button rectangles (same order as `buttons`).
    pub buttons: Vec<Rect>,
}

/// Text measurement callback: `(text, size, weight, max_width) -> (width, height)`.
pub type Measure<'a> = dyn FnMut(&str, f32, u16, f32) -> (f32, f32) + 'a;

/// Dialog state.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfirmDialog {
    /// Title.
    pub title: String,
    /// Body.
    pub body: String,
    /// Buttons in visual order; the **last** one is primary.
    pub buttons: Vec<Button>,
    /// Id returned on Esc.
    pub cancel_id: u32,
    /// Focused button index.
    pub focus: usize,
    /// Show the focus ring (after keyboard interaction).
    pub focus_visible: bool,
    /// Hovered button.
    pub hover: Option<usize>,
}

impl ConfirmDialog {
    /// New dialog; focus starts on the primary (last) button.
    pub fn new(
        title: impl Into<String>,
        body: impl Into<String>,
        buttons: Vec<Button>,
        cancel_id: u32,
    ) -> Self {
        let focus = buttons.len().saturating_sub(1);
        Self {
            title: title.into(),
            body: body.into(),
            buttons,
            cancel_id,
            focus,
            focus_visible: false,
            hover: None,
        }
    }

    /// Index of the primary button.
    pub fn primary(&self) -> usize {
        self.buttons.len().saturating_sub(1)
    }

    /// Computes geometry. `measure(text, size, weight) -> (width, height)` (height for the given
    /// max width) comes from the platform shaper.
    pub fn geometry(&self, view_w: f32, view_h: f32, measure: &mut Measure<'_>) -> Geometry {
        let w = WIDTH.min((view_w - 48.0).max(200.0));
        let inner = w - 2.0 * PAD;
        let (_, th) = measure(&self.title, TITLE_SIZE, 600, inner);
        let (_, bh) = measure(&self.body, BODY_SIZE, 400, inner);
        let h = PAD + th + 8.0 + bh + 20.0 + BUTTON_H + PAD;
        let x = ((view_w - w) / 2.0).max(0.0).round();
        let y = ((view_h - h) / 2.0).max(0.0).round();
        let title = Rect {
            x: x + PAD,
            y: y + PAD,
            w: inner,
            h: th,
        };
        let body = Rect {
            x: x + PAD,
            y: title.y + th + 8.0,
            w: inner,
            h: bh,
        };
        let by = y + h - PAD - BUTTON_H;
        let mut right = x + w - PAD;
        let mut buttons = vec![Rect::default(); self.buttons.len()];
        for (i, b) in self.buttons.iter().enumerate().rev() {
            let (lw, _) = measure(&b.label, BODY_SIZE, 400, 10_000.0);
            let bw = (lw + 2.0 * BUTTON_PAD_X).max(72.0).round();
            right -= bw;
            if let Some(r) = buttons.get_mut(i) {
                *r = Rect {
                    x: right,
                    y: by,
                    w: bw,
                    h: BUTTON_H,
                };
            }
            right -= BUTTON_GAP;
        }
        Geometry {
            card: Rect { x, y, w, h },
            title,
            body,
            buttons,
        }
    }

    /// Keyboard input.
    pub fn key(&mut self, k: Key) -> Outcome {
        let n = self.buttons.len().max(1);
        match k {
            Key::Enter => self
                .buttons
                .get(self.primary())
                .map_or(Outcome::Cancelled, |b| Outcome::Chosen(b.id)),
            Key::Space => self
                .buttons
                .get(self.focus)
                .map_or(Outcome::Cancelled, |b| Outcome::Chosen(b.id)),
            Key::Esc => Outcome::Chosen(self.cancel_id),
            Key::Tab => {
                self.focus = (self.focus + 1) % n;
                self.focus_visible = true;
                Outcome::Open { changed: true }
            }
            Key::ShiftTab => {
                self.focus = (self.focus + n - 1) % n;
                self.focus_visible = true;
                Outcome::Open { changed: true }
            }
        }
    }

    /// Mouse move (hover state).
    pub fn mouse_move(&mut self, g: &Geometry, x: f32, y: f32) -> bool {
        let h = g.buttons.iter().position(|r| r.contains(x, y));
        let changed = h != self.hover;
        self.hover = h;
        changed
    }

    /// Mouse click.
    pub fn click(&mut self, g: &Geometry, x: f32, y: f32) -> Outcome {
        self.focus_visible = false;
        match g.buttons.iter().position(|r| r.contains(x, y)) {
            Some(i) => self
                .buttons
                .get(i)
                .map_or(Outcome::Cancelled, |b| Outcome::Chosen(b.id)),
            None => Outcome::Open { changed: true },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dlg() -> ConfirmDialog {
        ConfirmDialog::new(
            "Large file",
            "This file is larger than 20 MiB.",
            vec![
                Button {
                    label: "Cancel".into(),
                    id: 0,
                },
                Button {
                    label: "Render anyway".into(),
                    id: 2,
                },
                Button {
                    label: "Open as plain text (fast)".into(),
                    id: 1,
                },
            ],
            0,
        )
    }

    #[test]
    fn keyboard_contract() {
        let mut d = dlg();
        assert_eq!(
            d.key(Key::Enter),
            Outcome::Chosen(1),
            "Enter = primary (last)"
        );
        assert_eq!(d.key(Key::Esc), Outcome::Chosen(0));
        assert!(!d.focus_visible);
        assert_eq!(d.key(Key::Tab), Outcome::Open { changed: true });
        assert!(d.focus_visible);
        assert_eq!(d.focus, 0);
        assert_eq!(d.key(Key::Space), Outcome::Chosen(0));
        d.key(Key::ShiftTab);
        assert_eq!(d.focus, 2);
    }

    #[test]
    fn geometry_and_click() {
        let mut d = dlg();
        let mut m =
            |t: &str, s: f32, _w: u16, _mw: f32| (t.chars().count() as f32 * s * 0.5, s * 1.4);
        let g = d.geometry(1000.0, 700.0, &mut m);
        assert_eq!(g.card.w, WIDTH);
        assert!(
            g.buttons[2].x > g.buttons[1].x && g.buttons[1].x > g.buttons[0].x,
            "primary last, right-aligned"
        );
        assert!((g.buttons[2].x + g.buttons[2].w - (g.card.x + g.card.w - PAD)).abs() < 0.01);
        let c = g.buttons[1];
        assert_eq!(d.click(&g, c.x + 2.0, c.y + 2.0), Outcome::Chosen(2));
        assert!(d.mouse_move(&g, c.x + 2.0, c.y + 2.0));
        assert_eq!(d.hover, Some(1));
    }
}
