//! Window chrome of Phase V2-A (prompt 011 §3): the custom title bar (B.4, R-WIN-05, FUT-30) with
//! ⚙ / ⋯ and the caption buttons, F11 distraction-free fullscreen (R-WIN-06, FUT-46), the
//! word-count badge (B.10.3, R-READ-07, FUT-35) and the zoom indicator (B.10.2).
//!
//! The client area covers the caption (`WM_NCCALCSIZE`); everything else in the window draws
//! and hit-tests in *content* coordinates, which start `top` DIPs below the client top.

use super::*;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_STYLE, GetWindowLongPtrW, GetWindowPlacement, HTCAPTION, HTMAXBUTTON, HTTOP, HTTOPLEFT,
    HTTOPRIGHT, IsZoomed, SC_MAXIMIZE, SC_MINIMIZE, SC_RESTORE, SWP_FRAMECHANGED,
    SWP_NOOWNERZORDER, SetWindowLongPtrW, SetWindowPlacement, WINDOWPLACEMENT, WM_SYSCOMMAND,
    WS_CAPTION, WS_THICKFRAME,
};
use yamdv_core::chrome::{self as ch, Part};

/// Title-bar interaction state.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Caption {
    pub hover: Option<Part>,
    pub pressed: Option<Part>,
    /// The window is inactive (title at 55 %).
    pub inactive: bool,
    /// Fullscreen: the bar is shown temporarily (mouse in the top 4 px).
    pub reveal: bool,
    /// `TrackMouseEvent(TME_NONCLIENT)` is armed (Maximize hover).
    pub nc_tracking: bool,
}

thread_local! {
    /// Style and placement saved when entering fullscreen, per window.
    static SAVED: RefCell<HashMap<isize, (isize, WINDOWPLACEMENT)>> = RefCell::new(HashMap::new());
    /// The 16 px app icon (per pixel size).
    static ICON: RefCell<Option<(i32, HICON)>> = const { RefCell::new(None) };
}

/// Linear blend of `a` toward `b` by `t` (0…1).
fn blend(a: u32, b: u32, t: f32) -> u32 {
    let ch = |s: u32| {
        let (x, y) = (((a >> s) & 0xFF) as f32, ((b >> s) & 0xFF) as f32);
        ((x + (y - x) * t).round() as u32) & 0xFF
    };
    (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

fn app_icon(px: i32) -> HICON {
    if let Some(h) = ICON.with(|i| i.borrow().filter(|(s, _)| *s == px).map(|(_, h)| h)) {
        return h;
    }
    // SAFETY: module handle of the running exe; icon resource 1 is embedded by build.rs.
    let h = unsafe {
        GetModuleHandleW(None).ok().and_then(|m| {
            LoadImageW(
                Some(m.into()),
                PCWSTR(std::ptr::without_provenance(1)),
                IMAGE_ICON,
                px,
                px,
                LR_SHARED,
            )
            .ok()
        })
    }
    .map(|h| HICON(h.0))
    .unwrap_or_default();
    ICON.with(|i| *i.borrow_mut() = Some((px, h)));
    h
}

/// Applies or leaves fullscreen on `hwnd` (deferred: sends messages).
pub(super) fn apply_fullscreen(hwnd: HWND, on: bool) {
    let key = hwnd.0 as isize;
    // SAFETY: valid hwnd; structures live on the stack for the calls.
    unsafe {
        if on {
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
            let mut wp = WINDOWPLACEMENT {
                length: size_of::<WINDOWPLACEMENT>() as u32,
                ..Default::default()
            };
            let _ = GetWindowPlacement(hwnd, &mut wp);
            SAVED.with(|s| s.borrow_mut().insert(key, (style, wp)));
            let mon = windows::Win32::Graphics::Gdi::MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            let _ = GetMonitorInfoW(mon, &mut mi);
            let r = mi.rcMonitor;
            let caption = (WS_CAPTION.0 | WS_THICKFRAME.0) as isize;
            SetWindowLongPtrW(hwnd, GWL_STYLE, style & !caption);
            let _ = SetWindowPos(
                hwnd,
                None,
                r.left,
                r.top,
                r.right - r.left,
                r.bottom - r.top,
                SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
            );
        } else if let Some((style, wp)) = SAVED.with(|s| s.borrow_mut().remove(&key)) {
            SetWindowLongPtrW(hwnd, GWL_STYLE, style);
            let _ = SetWindowPlacement(hwnd, &wp);
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOZORDER
                    | SWP_NOOWNERZORDER
                    | SWP_FRAMECHANGED
                    | windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE
                    | windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE,
            );
        }
    }
}

/// `WM_NCCALCSIZE` (wParam TRUE): keep the system frame on the left, right and bottom (resize
/// borders, DWM shadow, Windows 11 rounded corners) and extend the client over the caption.
pub(super) fn nc_calc_size(hwnd: HWND, lp: LPARAM) -> Option<LRESULT> {
    // SAFETY: valid hwnd.
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32;
    if style & WS_CAPTION.0 != WS_CAPTION.0 {
        return None;
    }
    // SAFETY: for WM_NCCALCSIZE with wParam TRUE, lParam points to a NCCALCSIZE_PARAMS.
    unsafe {
        let p = lp.0 as *mut windows::Win32::UI::WindowsAndMessaging::NCCALCSIZE_PARAMS;
        let top = (*p).rgrc[0].top;
        let r = DefWindowProcW(hwnd, 0x0083, WPARAM(1), lp);
        if r.0 != 0 {
            return Some(r);
        }
        (*p).rgrc[0].top = top;
        if IsZoomed(hwnd).as_bool() {
            // Maximized windows hang over the monitor edge by the frame thickness.
            let dpi = GetDpiForWindow(hwnd).max(96);
            let frame = windows::Win32::UI::HiDpi::GetSystemMetricsForDpi(
                windows::Win32::UI::WindowsAndMessaging::SM_CYFRAME,
                dpi,
            ) + windows::Win32::UI::HiDpi::GetSystemMetricsForDpi(
                windows::Win32::UI::WindowsAndMessaging::SM_CXPADDEDBORDER,
                dpi,
            );
            (*p).rgrc[0].top += frame;
        }
    }
    Some(LRESULT(0))
}

impl Win {
    /// The bar is drawn (always, or revealed in fullscreen).
    pub(super) fn chrome_visible(&self) -> bool {
        !self.fs || self.cap.reveal
    }

    /// Content offset in device pixels.
    pub(super) fn top_px(&self) -> i32 {
        (self.top * self.scale).round() as i32
    }

    /// Mouse-message point in content DIPs.
    pub(super) fn lp_xy(&self, lp: LPARAM) -> (f32, f32) {
        let (x, y) = lparam_xy(lp, self.scale);
        (x, y - self.top)
    }

    /// Screen point (px) in content DIPs.
    pub(super) fn screen_xy(&self, sx: i32, sy: i32) -> (f32, f32) {
        let mut pt = POINT { x: sx, y: sy };
        // SAFETY: valid hwnd and point.
        let _ = unsafe { ScreenToClient(self.hwnd, &mut pt) };
        (pt.x as f32 / self.scale, pt.y as f32 / self.scale - self.top)
    }

    /// A content rectangle (device px relative to the content) in client px.
    pub(super) fn crc(&self, rc: RECT) -> RECT {
        let t = self.top_px();
        RECT {
            top: rc.top + t,
            bottom: rc.bottom + t,
            ..rc
        }
    }

    pub(super) fn maximized(&self) -> bool {
        // SAFETY: valid hwnd.
        unsafe { IsZoomed(self.hwnd).as_bool() }
    }

    pub(super) fn invalidate_bar(&self) {
        let rc = RECT {
            left: 0,
            top: 0,
            right: (self.cw * self.scale).ceil() as i32 + 1,
            bottom: (ch::BAR_H * self.scale).ceil() as i32,
        };
        // SAFETY: valid hwnd.
        let _ = unsafe { InvalidateRect(Some(self.hwnd), Some(&rc), false) };
    }

    /// `WM_NCHITTEST` over the client (DIPs relative to the client top-left).
    pub(super) fn nc_hit(&self, x: f32, y: f32) -> Option<u32> {
        if !self.chrome_visible() || self.dialog.is_some() && y >= ch::BAR_H {
            return None;
        }
        let max = self.maximized() || self.fs;
        let part = ch::hit(x, y, self.cw, max)?;
        Some(match part {
            Part::Top if x < ch::RESIZE * 2.0 => HTTOPLEFT,
            Part::Top if x > self.cw - ch::RESIZE * 2.0 => HTTOPRIGHT,
            Part::Top => HTTOP,
            Part::Max if !self.fs => HTMAXBUTTON,
            Part::Caption if !self.fs => HTCAPTION,
            _ => HTCLIENT,
        })
    }

    fn set_hover(&mut self, p: Option<Part>) {
        if self.cap.hover != p {
            self.cap.hover = p;
            self.invalidate_bar();
        }
    }

    /// Client mouse move at raw client DIPs; `true` when the bar takes it.
    pub(super) fn chrome_move(&mut self, x: f32, y: f32) -> bool {
        if self.fs {
            let reveal = if self.cap.reveal {
                y < ch::BAR_H
            } else {
                y < 4.0
            };
            if reveal != self.cap.reveal {
                self.cap.reveal = reveal;
                self.invalidate_bar();
                if !reveal {
                    // The bar overlaid the content: repaint what it covered.
                    self.invalidate();
                }
            }
        }
        if !self.chrome_visible() || y >= ch::BAR_H || self.selecting {
            self.set_hover(None);
            return false;
        }
        let p = ch::hit(x, y, self.cw, true).filter(|p| *p != Part::Caption);
        self.set_hover(p);
        if !self.tracking {
            self.tracking = true;
            let mut tme = TRACKMOUSEEVENT {
                cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE,
                hwndTrack: self.hwnd,
                dwHoverTime: 0,
            };
            // SAFETY: valid structure.
            let _ = unsafe { TrackMouseEvent(&mut tme) };
        }
        true
    }

    /// Button press at raw client DIPs; `true` when the bar takes it.
    pub(super) fn chrome_down(&mut self, x: f32, y: f32) -> bool {
        if !self.chrome_visible() || y >= ch::BAR_H {
            return false;
        }
        let p = ch::hit(x, y, self.cw, true).filter(|p| *p != Part::Caption);
        self.cap.pressed = p;
        self.invalidate_bar();
        true
    }

    /// Button release; runs the pressed button when still hovered.
    pub(super) fn chrome_up(&mut self) -> bool {
        let Some(p) = self.cap.pressed.take() else {
            return false;
        };
        self.invalidate_bar();
        if self.cap.hover == Some(p) {
            self.chrome_click(p);
        }
        true
    }

    /// Maximize hover via `WM_NCMOUSEMOVE` (the button is `HTMAXBUTTON` for Snap Layouts).
    pub(super) fn nc_move(&mut self, ht: u32) {
        let p = (ht == HTMAXBUTTON).then_some(Part::Max);
        self.set_hover(p);
        if !self.cap.nc_tracking {
            self.cap.nc_tracking = true;
            let mut tme = TRACKMOUSEEVENT {
                cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE | windows::Win32::UI::Input::KeyboardAndMouse::TME_NONCLIENT,
                hwndTrack: self.hwnd,
                dwHoverTime: 0,
            };
            // SAFETY: valid structure.
            let _ = unsafe { TrackMouseEvent(&mut tme) };
        }
    }

    pub(super) fn nc_leave(&mut self) {
        self.cap.nc_tracking = false;
        if self.cap.hover == Some(Part::Max) {
            self.set_hover(None);
        }
        if self.cap.pressed == Some(Part::Max) {
            self.cap.pressed = None;
        }
    }

    /// Runs a title-bar button (also used by the driver).
    pub(super) fn chrome_click(&mut self, p: Part) {
        let sys = |cmd: u32| {
            // SAFETY: valid hwnd; handled by DefWindowProc from the message loop.
            let _ = unsafe {
                PostMessageW(
                    Some(self.hwnd),
                    WM_SYSCOMMAND,
                    WPARAM(cmd as usize),
                    LPARAM(0),
                )
            };
        };
        match p {
            Part::Min => sys(SC_MINIMIZE),
            Part::Max if self.fs => self.toggle_fullscreen(),
            Part::Max => sys(if self.maximized() {
                SC_RESTORE
            } else {
                SC_MAXIMIZE
            }),
            Part::Close => self.actions.push(Action::Close),
            Part::More => self.actions.push(Action::ChromeMenu(true)),
            Part::Settings => self.actions.push(Action::ChromeMenu(false)),
            Part::Caption | Part::Top => {}
        }
    }

    /// Screen anchor under a title-bar button: (right edge, bottom) for ⋯, (left, bottom) for ⚙.
    pub(super) fn button_anchor(&self, part: Part) -> (i32, i32) {
        let (x, w) = ch::buttons(self.cw)
            .into_iter()
            .find(|b| b.0 == part)
            .map_or((self.cw - 200.0, 32.0), |b| (b.1, b.2));
        let edge = if part == Part::More { x + w } else { x };
        let mut pt = POINT {
            x: (edge * self.scale).round() as i32,
            y: (ch::BAR_H * self.scale).round() as i32,
        };
        // SAFETY: valid hwnd and point.
        let _ = unsafe { windows::Win32::Graphics::Gdi::ClientToScreen(self.hwnd, &mut pt) };
        (pt.x, pt.y)
    }

    /// `F11` / Appearance ▸ Full screen (R-WIN-06).
    pub(super) fn toggle_fullscreen(&mut self) {
        self.fs = !self.fs;
        self.top = if self.fs { 0.0 } else { ch::BAR_H };
        self.cap = Caption {
            inactive: self.cap.inactive,
            ..Caption::default()
        };
        self.defer(Deferred::Fullscreen(self.fs));
        self.invalidate();
    }

    /// B.10.2: shows the zoom indicator for 900 ms.
    pub(super) fn show_zoom(&mut self, text: String) {
        self.zoom_ind = Some(text);
        // SAFETY: valid hwnd; one-shot (killed when it fires, T7).
        unsafe { SetTimer(Some(self.hwnd), TIMER_ZOOM, 900, None) };
        self.invalidate();
    }

    /// Title text per `general.title_format` (file name or full path).
    fn title_text(&self, sh: &Shared) -> String {
        match &self.view {
            View::Doc(d) if d.virt => "YAMDV".to_string(),
            View::Doc(d) if sh.title_full => ypath::display_form(&d.path),
            View::Doc(d) => ypath::file_name(&d.path).to_string(),
            View::Error { path, .. } => ypath::file_name(path).to_string(),
            View::Empty { .. } => "YAMDV".to_string(),
        }
    }

    /// Mode pill text and whether it uses the warning colors.
    fn pill(&self, sh: &Shared) -> Option<(&'static str, bool)> {
        match self.edit_mode_id() {
            1 => return Some((tr(sh.lang, Msg::MLiveEdit), false)),
            2 => return Some((tr(sh.lang, Msg::MSplitEdit), false)),
            _ => {}
        }
        match &self.view {
            View::Doc(d) if d.virt || d.facts.read_only => {
                Some((tr(sh.lang, Msg::PillReadOnly), true))
            }
            _ => None,
        }
    }

    /// Draws the title bar (origin = client top).
    pub(super) fn draw_chrome(&self, sh: &Shared, b: &Band<'_>) {
        if !self.chrome_visible() || !b.visible(0.0, ch::BAR_H) {
            return;
        }
        let pal = &self.pal;
        let tok = ch::tokens(self.dark);
        let w = self.cw;
        b.fill(Rect::new(0.0, 0.0, w, ch::BAR_H), pal.bg, 1.0, 0.0);
        if self.scroll_pos() > 0.5 || self.fs {
            b.fill(Rect::new(0.0, ch::BAR_H - 1.0, w, 1.0), pal.border_subtle, 1.0, 0.0);
        }
        let fg = if self.cap.inactive {
            blend(pal.bg, pal.text, 0.55)
        } else {
            pal.text
        };
        let mut x = 12.0;
        let icon_px = (16.0 * self.scale).round() as i32;
        b.icon(app_icon(icon_px), x, 8.0, 16.0);
        x += 16.0 + 8.0;
        // Pill and dirty dot widths are reserved before ellipsizing the title.
        let pill = self.pill(sh).map(|(t, warn)| {
            let color = if warn {
                if self.dark { 0xd29922 } else { 0x9a6700 }
            } else {
                pal.accent
            };
            (self.shape_ui(sh, t, 11.0, 600, color, None), color)
        });
        let dirty = self.edit().is_some_and(|e| e.ed.is_dirty());
        let reserve = pill.as_ref().map_or(0.0, |(t, _)| t.width() + 16.0 + 8.0)
            + if dirty { 16.0 } else { 0.0 };
        let avail = (ch::title_right(w) - x - reserve).max(24.0);
        let full = self.title_text(sh);
        let title = ch::ellipsize_middle(&full, |s| sh.dw.measure(s, 12.0, 400, 10_000.0).0 <= avail);
        let t = self.shape_ui(sh, &title, 12.0, 400, fg, None);
        b.text(t.as_ref(), x, ((ch::BAR_H - t.height()) / 2.0).round());
        x += t.width() + 8.0;
        if let Some((pt, color)) = pill {
            let r = Rect::new(x, 7.0, pt.width() + 16.0, 18.0);
            b.fill(r, color, 0.12, if sh.m.rounded { 9.0 } else { 0.0 });
            b.text(pt.as_ref(), x + 8.0, (r.y + (r.h - pt.height()) / 2.0).round());
            x += r.w + 8.0;
        }
        if dirty {
            b.circle(x + 4.0, ch::BAR_H / 2.0, 3.5, pal.text_muted, None);
        }
        for (part, bx, bw) in ch::buttons(w) {
            let r = Rect::new(bx, 0.0, bw, ch::BAR_H);
            let hover = self.cap.hover == Some(part);
            let pressed = self.cap.pressed == Some(part) && hover;
            let mut glyph = fg;
            if part == Part::Close && hover {
                b.fill(r, ch::CLOSE_HOVER, if pressed { 0.85 } else { 1.0 }, 0.0);
                glyph = 0xffffff;
            } else if pressed {
                b.fill(r, tok.pressed.0, f32::from(tok.pressed.1) / 100.0, 0.0);
            } else if hover {
                b.fill(r, tok.hover.0, f32::from(tok.hover.1) / 100.0, 0.0);
            }
            self.draw_glyph(sh, b, part, r, glyph);
        }
    }

    fn draw_glyph(&self, sh: &Shared, b: &Band<'_>, part: Part, r: Rect, color: u32) {
        let max = self.maximized();
        let (code, size) = match part {
            Part::Settings => ("\u{E713}", 14.0),
            Part::More => ("\u{E712}", 14.0),
            Part::Min => ("\u{E921}", 10.0),
            Part::Max if max || self.fs => ("\u{E923}", 10.0),
            Part::Max => ("\u{E922}", 10.0),
            Part::Close => ("\u{E8BB}", 10.0),
            Part::Caption | Part::Top => return,
        };
        let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
        if let Some(g) = sh.dw.shape_icon(code, size, color) {
            use yamdv_layout::shaper::Shaped as _;
            b.text(
                &g,
                (cx - g.width() / 2.0).round(),
                (cy - g.height() / 2.0).round(),
            );
            return;
        }
        // Vector fallback.
        let s = size / 2.0;
        match part {
            Part::Min => b.fill(Rect::new(cx - s, cy, size, 1.0), color, 1.0, 0.0),
            Part::Max if max || self.fs => {
                b.stroke(Rect::new(cx - s, cy - s + 2.0, size - 2.0, size - 2.0), color, 0.0, 1.0);
                b.polyline(
                    &[(cx - s + 2.0, cy - s), (cx + s, cy - s), (cx + s, cy + s - 2.0)],
                    color,
                    1.0,
                    false,
                );
            }
            Part::Max => b.stroke(Rect::new(cx - s, cy - s, size, size), color, 0.0, 1.0),
            Part::Close => {
                b.polyline(&[(cx - s, cy - s), (cx + s, cy + s)], color, 1.0, false);
                b.polyline(&[(cx + s, cy - s), (cx - s, cy + s)], color, 1.0, false);
            }
            Part::More => {
                for dx in [-5.0, 0.0, 5.0] {
                    b.circle(cx + dx, cy, 1.2, color, None);
                }
            }
            Part::Settings => {
                b.circle(cx, cy, 6.0, color, Some(1.2));
                b.circle(cx, cy, 2.2, color, Some(1.2));
            }
            Part::Caption | Part::Top => {}
        }
    }

    /// Words and characters of the document and of the selection (cached by source length and
    /// selection; computed only while the badge is shown).
    fn word_counts(&self) -> Option<((usize, usize), Option<(usize, usize)>)> {
        let View::Doc(d) = &self.view else { return None };
        let src: std::borrow::Cow<'_, str> = match self.edit() {
            Some(e) => std::borrow::Cow::Owned(e.ed.text()),
            None => std::borrow::Cow::Borrowed(d.content.source()),
        };
        let key = (src.len(), self.sel);
        if let Some((k, v)) = self.wc_cache.get()
            && k == key
        {
            return Some(v);
        }
        let all = ch::count(&ch::reading_text(&src));
        let sel = self.sel.and_then(|(a, c)| {
            let (a, c) = (a.min(c) as usize, a.max(c) as usize);
            (c > a)
                .then(|| d.content.source().get(a..c))
                .flatten()
                .map(|t| ch::count(&ch::reading_text(t)))
        });
        self.wc_cache.set(Some((key, (all, sel))));
        Some((all, sel))
    }

    /// Badge rectangle (content DIPs) and its text, when shown.
    pub(super) fn badge(&self, sh: &Shared) -> Option<(Rect, String)> {
        if !sh.word_badge || self.dialog.is_some() {
            return None;
        }
        let ((w, c), sel) = self.word_counts()?;
        let (ws, cs) = match sel {
            Some((sw, sc)) => (format!("{sw} / {w}"), format!("{sc} / {c}")),
            None => (w.to_string(), c.to_string()),
        };
        let text = tr(sh.lang, Msg::WordBadge)
            .replace("{w}", &ws)
            .replace("{c}", &cs);
        let (tw, _) = sh.dw.measure(&text, 12.0, 400, 10_000.0);
        let (ax, aw) = self.doc_area(sh);
        let bw = tw.ceil() + 20.0;
        let r = Rect::new(
            (ax + aw - 16.0 - 12.0 - bw).round(),
            (self.ch - 12.0 - 24.0).round(),
            bw,
            24.0,
        );
        Some((r, text))
    }

    /// Word-count badge and zoom indicator (content coordinates).
    pub(super) fn draw_overlays(&self, sh: &Shared, b: &Band<'_>) {
        let tok = ch::tokens(self.dark);
        if let Some((r, text)) = self.badge(sh)
            && b.visible(r.y, r.h)
        {
            b.fill(r, self.pal.bg, 0.9, 12.0);
            b.stroke(r, tok.chrome_border, 12.0, 1.0);
            let t = self.shape_ui(sh, &text, 12.0, 400, self.pal.text_muted, None);
            b.text(
                t.as_ref(),
                (r.x + 10.0).round(),
                (r.y + (r.h - t.height()) / 2.0).round(),
            );
        }
        if let Some(z) = &self.zoom_ind {
            let r = Rect::new(
                ((self.cw - 140.0) / 2.0).round(),
                ((self.ch - 48.0) / 2.0).round(),
                140.0,
                48.0,
            );
            if b.visible(r.y, r.h) {
                b.fill(r, tok.toast_bg, 0.92, 10.0);
                let t = self.shape_ui(sh, z, 15.0, 400, tok.toast_text, None);
                b.text(
                    t.as_ref(),
                    (r.x + (r.w - t.width()) / 2.0).round(),
                    (r.y + (r.h - t.height()) / 2.0).round(),
                );
            }
        }
    }

    /// Invalidates the fixed overlays (they sit inside the scrolled rectangle).
    pub(super) fn invalidate_overlays(&self, sh: &Shared, ady: i32) {
        let px = |v: f32| (v * self.scale).round() as i32;
        let mut rects = Vec::new();
        if let Some((r, _)) = self.badge(sh) {
            rects.push(r);
        }
        if self.zoom_ind.is_some() {
            rects.push(Rect::new(
                (self.cw - 140.0) / 2.0,
                (self.ch - 48.0) / 2.0,
                140.0,
                48.0,
            ));
        }
        for r in rects {
            let rc = self.crc(RECT {
                left: px(r.x) - 2,
                top: px(r.y) - ady - 2,
                right: px(r.x + r.w) + 2,
                bottom: px(r.y + r.h) + ady + 2,
            });
            // SAFETY: valid hwnd.
            let _ = unsafe { InvalidateRect(Some(self.hwnd), Some(&rc), false) };
        }
    }
}

/// `COLORREF` helper kept local (the popup has its own).
#[allow(dead_code)]
fn colorref(rgb: u32) -> COLORREF {
    COLORREF(((rgb >> 16) & 0xff) | (rgb & 0xff00) | ((rgb & 0xff) << 16))
}
