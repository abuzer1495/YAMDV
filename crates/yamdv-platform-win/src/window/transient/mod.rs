//! Transient UI (prompt 013, Phase V2-C): banners (B.10.5 + P-42), tooltips and the link
//! status (B.10.6 / B.10.4, FUT-40), the anchored Ask popover, fullscreen viewers (B.11,
//! FUT-36) and heading folding (R-READ-06, FUT-06). Everything is built on demand, freed on
//! close, and runs no timer while idle (T7).

mod banner;
mod fold;
mod popover;
mod tooltip;
mod viewer;

use super::*;
use windows::Win32::UI::WindowsAndMessaging::IDC_SIZEALL;
use yamdv_core::chrome as ch;
use yamdv_ui::transient::{BannerKind, Banners, Bx, ZoomView};

/// One-shot 500 ms tooltip delay.
const TIMER_TIP: usize = 20;
/// Fold animation frames (only while a fold animates).
const TIMER_FOLD: usize = 21;
/// One-shot: the viewer's next GIF frame (only while an animated GIF is open, T7).
const TIMER_FVGIF: usize = 24;

/// A tooltip, shown or waiting for its delay.
#[derive(Debug, Clone, PartialEq)]
struct Tip {
    text: String,
    /// Anchor (client DIPs): the tooltip goes below it, or above near the bottom.
    at: Rect,
}

/// Media under the mouse (⛶ badge / [⛶][CSV] pill).
#[derive(Debug, Clone, PartialEq)]
struct Media {
    bi: usize,
    /// Hot rectangle (block coordinates).
    rect: Rect,
    kind: MediaKind,
}

#[derive(Debug, Clone, PartialEq)]
enum MediaKind {
    Image(Box<str>),
    Diagram(usize),
    Table(usize, u32, u32),
}

impl MediaKind {
    fn of(k: &HotKind) -> Option<Self> {
        match k {
            HotKind::Image { src } => Some(Self::Image(src.clone())),
            HotKind::Diagram { node } => Some(Self::Diagram(*node)),
            HotKind::Table { node, rows, cols } => Some(Self::Table(*node, *rows, *cols)),
            _ => None,
        }
    }
}

/// The anchored Ask popover of an external link.
#[derive(Debug, Clone)]
struct Popover {
    url: String,
    /// IDN / credential warnings.
    notes: Vec<String>,
    can_open: bool,
    /// Link rectangle (content DIPs).
    anchor: Rect,
    focus: usize,
    hover: Option<usize>,
}

/// Fold animation in progress.
#[derive(Debug, Clone)]
struct FoldAnim {
    start: Instant,
    /// Hidden flags after the change.
    hidden: Vec<bool>,
    /// Blocks whose height animates.
    moving: Vec<bool>,
    folding: bool,
    /// Heading kept at its screen position: (block, screen y).
    keep: (usize, f64),
}

/// Transient UI state of one window.
#[derive(Default)]
pub(super) struct State {
    banners: Banners,
    export_msg: Option<Msg>,
    tip: Option<Tip>,
    wait: Option<Tip>,
    /// Link status text (B.10.4).
    status: Option<String>,
    pop: Option<Popover>,
    fv: Option<Box<viewer::FullView>>,
    media: Option<Media>,
    /// Folded headings (session only).
    folds: yamdv_layout::fold::Folds,
    anim: Option<FoldAnim>,
    gutter_hover: Option<usize>,
    /// Keyboard focus on a fold toggle (after the links in the Tab order).
    pub(in crate::window) fold_focus: Option<usize>,
    last_down: Option<(Instant, f32, f32)>,
    mouse: (f32, f32),
}

/// Shapes `text` in `family` (UI strings use the body face, URLs the code face).
fn shape(
    sh: &Shared,
    text: &str,
    (size, weight, color): (f32, u16, u32),
    max_w: Option<f32>,
    family: Family,
) -> Box<dyn yamdv_layout::shaper::Shaped> {
    DwShaper(&sh.dw).shape(&ParaSpec {
        text,
        base: Style {
            family,
            size,
            weight,
            italic: false,
            color,
            strike: false,
            underline: false,
        },
        spans: vec![],
        width: max_w,
        line_pitch: (size * 1.4).round(),
        rtl: false,
        tab_spaces: None,
        objects: Vec::new(),
    })
}

fn bx(r: Rect) -> Bx {
    Bx::new(r.x, r.y, r.w, r.h)
}

fn rect(b: Bx) -> Rect {
    Rect::new(b.x, b.y, b.w, b.h)
}

/// Soft shadow under a card (B.10 popover / viewer card).
fn shadow(b: &Band<'_>, r: Rect, radius: f32, dark: bool) {
    let a = if dark { 0.45 } else { 0.14 };
    for i in 1..=6 {
        let g = i as f32 * 4.0;
        b.fill(
            Rect::new(r.x - g / 2.0, r.y + 6.0 - g / 2.0, r.w + g, r.h + g),
            0x000000,
            a / 6.0,
            radius + g / 2.0,
        );
    }
}

/// Text centred in `r`.
fn center(b: &Band<'_>, t: &dyn yamdv_layout::shaper::Shaped, r: Rect) {
    b.text(
        t,
        (r.x + (r.w - t.width()) / 2.0).round(),
        (r.y + (r.h - t.height()) / 2.0).round(),
    );
}

impl Win {
    /// Content DIPs → invalidated client rectangle.
    fn inval_content(&self, r: Rect) {
        let px = |v: f32| (v * self.scale).round() as i32;
        let rc = self.crc(RECT {
            left: px(r.x) - 2,
            top: px(r.y) - 2,
            right: px(r.x + r.w) + 2,
            bottom: px(r.y + r.h) + 2,
        });
        // SAFETY: valid hwnd.
        let _ = unsafe { InvalidateRect(Some(self.hwnd), Some(&rc), false) };
    }

    fn tz_at(&self, x: f32, y: f32) -> Rect {
        Rect::new(x - 4.0, y + self.top, 8.0, 20.0)
    }

    /// The press at (x, y) is the second of a double-click.
    fn double_click(&mut self, x: f32, y: f32) -> bool {
        // SAFETY: no preconditions.
        let (ms, dx, dy) = unsafe {
            (
                GetDoubleClickTime(),
                GetSystemMetrics(SM_CXDOUBLECLK),
                GetSystemMetrics(SM_CYDOUBLECLK),
            )
        };
        let now = Instant::now();
        let dbl = self.tz.last_down.is_some_and(|(t, px, py)| {
            now.duration_since(t).as_millis() <= u128::from(ms)
                && (x - px).abs() * self.scale <= dx as f32 / 2.0 + 1.0
                && (y - py).abs() * self.scale <= dy as f32 / 2.0 + 1.0
        });
        self.tz.last_down = if dbl { None } else { Some((now, x, y)) };
        dbl
    }

    /// Per-document transient state goes away (another document is shown).
    pub(in crate::window) fn tz_doc_changed(&mut self) {
        self.tz.folds.clear();
        self.tz.anim = None;
        self.tz.fold_focus = None;
        self.tz.pop = None;
        self.tz.media = None;
        self.tz.status = None;
        if self.tz.fv.take().is_some() {
            sys::trim_once();
        }
        self.tz_tip_hide();
    }

    /// Keys first seen by the transient layer (`Some` = consumed).
    pub(in crate::window) fn tz_key(
        &mut self,
        sh: &Shared,
        vk: u16,
        ctrl: bool,
        shift: bool,
    ) -> Option<Option<Action>> {
        if self.tz.tip.is_some() || self.tz.wait.is_some() {
            self.tz_tip_hide();
        }
        if self.tz.fv.is_some() {
            self.fv_key(sh, vk, ctrl);
            return Some(None);
        }
        if self.tz.pop.is_some() {
            self.pop_key(vk, shift);
            return Some(None);
        }
        self.fold_key(sh, vk, ctrl, shift).then_some(None)
    }

    /// Mouse press (content DIPs; y < 0 is above the content). `true` when consumed.
    pub(in crate::window) fn tz_down(&mut self, sh: &Shared, x: f32, y: f32, alt: bool) -> bool {
        self.tz_tip_hide();
        if self.tz.fv.is_some() {
            self.fv_down(sh, x, y);
            return true;
        }
        if self.tz.pop.is_some() {
            self.pop_down(sh, x, y);
            return true;
        }
        if y < 0.0 {
            return self.banner_click(sh, x, y + self.top);
        }
        if self.fold_down(sh, x, y, alt) {
            self.tz.last_down = None;
            return true;
        }
        self.media_down(sh, x, y)
    }

    /// Window messages seen before the editor and the reading view (`Some` = consumed).
    pub(in crate::window) fn tz_msg(
        &mut self,
        sh: &Shared,
        msg: u32,
        wp: WPARAM,
        lp: LPARAM,
    ) -> Option<LRESULT> {
        match msg {
            WM_TIMER if wp.0 == TIMER_TIP => {
                self.tip_fire();
                Some(LRESULT(0))
            }
            WM_TIMER if wp.0 == TIMER_FVGIF => {
                // SAFETY: valid hwnd.
                let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_FVGIF) };
                self.fv_tick_gif();
                Some(LRESULT(0))
            }
            WM_TIMER if wp.0 == TIMER_FOLD => {
                self.fold_tick();
                Some(LRESULT(0))
            }
            WM_KEYDOWN | WM_SYSKEYDOWN if self.tz.fv.is_some() || self.tz.pop.is_some() => {
                // SAFETY: GetKeyState has no preconditions.
                let (ctrl, shift) = unsafe {
                    (
                        GetKeyState(i32::from(VK_CONTROL.0)) < 0,
                        GetKeyState(i32::from(VK_SHIFT.0)) < 0,
                    )
                };
                let vk = u16::try_from(wp.0).unwrap_or(0);
                self.tz_key(sh, vk, ctrl, shift);
                Some(LRESULT(0))
            }
            WM_CHAR if self.tz.fv.is_some() || self.tz.pop.is_some() => Some(LRESULT(0)),
            WM_LBUTTONDOWN => {
                if self.dialog.is_some() || self.settings.is_some() {
                    return None;
                }
                let (x, y) = self.lp_xy(lp);
                // SAFETY: GetKeyState has no preconditions.
                let alt = unsafe { GetKeyState(0x12) } < 0;
                self.tz_down(sh, x, y, alt).then_some(LRESULT(0))
            }
            WM_LBUTTONUP => {
                let fv = self.tz.fv.as_mut()?;
                fv.end_drag();
                Some(LRESULT(0))
            }
            WM_MOUSEMOVE => {
                let (x, y) = self.lp_xy(lp);
                self.tz.mouse = (x, y);
                if self.tz.fv.is_some() {
                    self.fv_move(sh, x, y);
                    return Some(LRESULT(0));
                }
                if self.tz.pop.is_some() {
                    self.pop_move(sh, x, y);
                    return Some(LRESULT(0));
                }
                if self.dialog.is_some() || self.settings.is_some() || self.selecting {
                    self.tip_want(None);
                    return None;
                }
                if y < 0.0 {
                    let at = self.tz_at(x, y);
                    let tip = self
                        .cap
                        .hover
                        .and_then(|p| self.chrome_tip(sh, p))
                        .map(|text| Tip { text, at });
                    self.tip_want(tip);
                    self.set_status(sh, None);
                } else {
                    self.hover(sh, x, y);
                }
                None
            }
            WM_MOUSEWHEEL => {
                self.tz_tip_hide();
                if self.tz.fv.is_some() {
                    let delta = ((wp.0 >> 16) & 0xFFFF) as u16 as i16;
                    let sx = i32::from(lp.0 as u16 as i16);
                    let sy = i32::from((lp.0 >> 16) as u16 as i16);
                    let (x, y) = self.screen_xy(sx, sy);
                    self.fv_wheel(delta, x, y, wp.0 & 0x0008 != 0);
                    return Some(LRESULT(0));
                }
                if self.tz.pop.take().is_some() {
                    self.invalidate();
                }
                self.set_status(sh, None);
                None
            }
            WM_MOUSELEAVE => {
                self.tz_tip_hide();
                self.set_status(sh, None);
                if self.tz.media.take().is_some() | self.tz.gutter_hover.take().is_some() {
                    self.invalidate();
                }
                None
            }
            0x0006 if wp.0 & 0xFFFF == 0 => {
                // WM_ACTIVATE / WA_INACTIVE.
                self.tz_tip_hide();
                None
            }
            WM_SETCURSOR if self.tz.fv.is_some() => {
                let cur = if self.tz.fv.as_ref().is_some_and(|f| f.dragging()) {
                    IDC_SIZEALL
                } else {
                    IDC_ARROW
                };
                // SAFETY: system cursors; no cleanup needed.
                unsafe { SetCursor(LoadCursorW(None, cur).ok()) };
                Some(LRESULT(1))
            }
            WM_SETCURSOR if self.tz.gutter_hover.is_some() || self.tz.media.is_some() => {
                let (x, y) = self.tz.mouse;
                let hand = self.tz.gutter_hover.is_some()
                    || self
                        .tz
                        .media
                        .as_ref()
                        .is_some_and(|m| self.media_badge_hit(sh, m, x, y).is_some());
                if !hand {
                    return None;
                }
                // SAFETY: system cursors; no cleanup needed.
                unsafe { SetCursor(LoadCursorW(None, IDC_HAND).ok()) };
                Some(LRESULT(1))
            }
            _ => None,
        }
    }

    /// Last mouse position (client DIPs).
    pub(in crate::window) fn tz_mouse(&self) -> (f32, f32) {
        self.tz.mouse
    }

    /// Transient overlays drawn over the document (content DIPs).
    pub(in crate::window) fn tz_draw(&self, sh: &Shared, b: &Band<'_>) {
        self.draw_gutter(sh, b);
        self.draw_media_badge(sh, b);
        if self.tz.fv.is_none() {
            self.draw_status(sh, b);
        }
        self.fv_draw(sh, b);
        self.pop_draw(sh, b);
        self.nav_draw(sh, b);
    }

    /// Client-level overlays (origin 0): banners and the tooltip.
    pub(in crate::window) fn tz_draw_client(&self, sh: &Shared, b: &Band<'_>) {
        self.tz_draw_banners(sh, b);
        self.nav_draw_client(sh, b);
        self.tz_draw_tip(sh, b);
    }

    /// Driver state: `(tooltip, banners, viewer, zoom %, folds, popover)`.
    #[cfg(feature = "dev-tools")]
    pub(in crate::window) fn tz_state(
        &self,
    ) -> (String, String, String, f64, usize, Option<&'static str>) {
        let banners: Vec<&str> = self
            .tz
            .banners
            .visible()
            .into_iter()
            .map(banner::name)
            .collect();
        let (viewer, zoom) = match &self.tz.fv {
            Some(f) => (f.describe(), f64::from(f.percent())),
            None => ("none".into(), 0.0),
        };
        let pop = if self.tz.pop.is_some() {
            Some("ask-url")
        } else if self.tz.banners.has(BannerKind::Conflict) {
            Some("conflict")
        } else {
            None
        };
        (
            self.tz
                .tip
                .as_ref()
                .map_or_else(|| "none".into(), |t| t.text.clone()),
            if banners.is_empty() {
                "none".into()
            } else {
                banners.join(",")
            },
            viewer,
            zoom,
            self.tz.folds.folded().len(),
            pop,
        )
    }
}

/// V2-D keyboard reach (HARD-04): entry points of the transient UI.
impl Win {
    /// The fullscreen viewer is open.
    pub(in crate::window) fn tz_viewer_open(&self) -> bool {
        self.tz.fv.is_some()
    }

    /// Fold toggle of heading block `bi` (content coordinates), when shown.
    pub(in crate::window) fn tz_toggle_rect(&self, sh: &Shared, bi: usize) -> Option<Rect> {
        let View::Doc(d) = &self.view else {
            return None;
        };
        let (cx, _) = self.col(sh);
        let lvl = d.content.heading_level(bi)?;
        (lvl <= yamdv_layout::fold::MAX_FOLD_LEVEL && cx >= 26.0 && !d.viewer.is_hidden(bi))
            .then(|| Self::gutter_rect(d, cx, bi))
            .flatten()
    }

    /// Opens the fullscreen viewer of a diagram / table hot region (⛶ by keyboard).
    pub(in crate::window) fn tz_open_hot(&mut self, sh: &Shared, bi: usize, k: &HotKind) -> bool {
        match MediaKind::of(k) {
            Some(m) => {
                self.fv_open(sh, bi, &m);
                self.tz.fv.is_some()
            }
            None => false,
        }
    }

    /// Banner buttons in order (client coordinates): (rect, banner, button id).
    pub(in crate::window) fn tz_banner_stops(&self, sh: &Shared) -> Vec<(Rect, BannerKind, u32)> {
        let mut out = Vec::new();
        for (k, r) in self.banner_rects() {
            let mut b = self.banner_buttons(sh, k, r);
            // The × closes last.
            b.sort_by_key(|x| x.1 == 0);
            out.extend(b.into_iter().map(|(br, id, _)| (br, k, id)));
        }
        out
    }
}

/// The overlay card shadow (floating sidebar, FUT-02).
pub(in crate::window) fn card_shadow(b: &Band<'_>, r: Rect, radius: f32, dark: bool) {
    shadow(b, r, radius, dark);
}

impl Win {
    /// Unfolds heading block `h` and every fold hiding it (palette jump, FUT-08).
    pub(in crate::window) fn tz_unfold(&mut self, h: usize) {
        let off = match &self.view {
            View::Doc(d) => d.content.range(h).start,
            _ => return,
        };
        self.tz_reveal_offset(off);
        if self.tz.folds.is_folded(h) {
            self.tz_drive_settle();
            self.tz_fold(h, false);
            self.tz_drive_settle();
        }
    }
}
