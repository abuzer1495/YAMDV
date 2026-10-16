//! Reading windows (Phase 002): one top-level window per document in the same process, backend A
//! rendering, virtualized scrolling, selection/copy, open (CLI / `Ctrl+O` / drag & drop), guards
//! with the in-window confirm dialog, empty and error states. No timers run while idle (T7).
//! PHASE-004: custom title bar replaces the temporary system caption.
#![allow(unsafe_code)]
// Pixel/DIP geometry: float<->int conversions are bounded by window/bitmap sizes (ADR-0007).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::indexing_slicing
)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;
use std::time::Instant;

use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, GetMonitorInfoW, InvalidateRect, MONITOR_DEFAULTTONEAREST, MONITORINFO,
    MonitorFromPoint, PAINTSTRUCT, ScreenToClient, UpdateWindow,
};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    AdjustWindowRectExForDpi, GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
    VK_CONTROL, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_NEXT, VK_PRIOR, VK_RETURN, VK_SHIFT,
    VK_SPACE, VK_TAB, VK_UP,
};
use windows::Win32::UI::Shell::{DragAcceptFiles, DragFinish, DragQueryFileW, HDROP};
/// `WM_MOUSELEAVE` (lives in UI::Controls in windows-rs; avoided pulling that feature).
const WM_MOUSELEAVE: u32 = 0x02A3;
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetClientRect, GetCursorPos, GetMessageTime, GetMessageW, GetSystemMetrics, HICON, HTCLIENT,
    IDC_ARROW, IDC_IBEAM, IMAGE_ICON, KillTimer, LR_DEFAULTSIZE, LR_SHARED, LoadCursorW,
    LoadImageW, MINMAXINFO, MSG, PostMessageW, PostQuitMessage, RegisterClassExW, SM_CXDOUBLECLK,
    SM_CYDOUBLECLK, SW_INVALIDATE, SW_SHOW, SWP_NOACTIVATE, SWP_NOZORDER, ScrollWindowEx,
    SetCursor, SetPropW, SetTimer, SetWindowPos, SetWindowTextW, ShowWindow, TranslateMessage,
    WINDOW_EX_STYLE, WM_APP, WM_COMMAND, WM_DESTROY, WM_DPICHANGED, WM_DROPFILES, WM_ERASEBKGND,
    WM_GETMINMAXINFO, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_PAINT, WM_SETCURSOR, WM_SETTINGCHANGE, WM_SIZE, WM_TIMER, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
};
use windows::core::{HSTRING, PCWSTR, w};
use yamdv_core::decode::{self, Guard};
use yamdv_core::doc::detail::{Alert, Placeholder};
use yamdv_core::doc::{Document, ReadOpts};
use yamdv_core::i18n::{Lang, Msg, tr};
use yamdv_core::path as ypath;
use yamdv_core::settings::Settings;
use yamdv_core::theme::{self, Palette};
use yamdv_layout::Rect;
use yamdv_layout::block::{Bullet, Item, Metrics, Strings};
use yamdv_layout::shaper::{Family, ParaSpec, Style, TextShaper};
use yamdv_layout::viewer::{self, Content, Env, Hit, Viewer};
use yamdv_ui::dialog::{self as dlg, Button, ConfirmDialog, Geometry, Outcome};

use crate::dwrite::{DWrite, DwShaper};
use crate::render::{Band, Renderer};
use crate::sys::{self, FileInfo};
use crate::theme::{Mode, app_mode};

/// Window class name.
pub const CLASS_NAME: PCWSTR = w!("YAMDV.Window");
/// Initial client size in logical px.
pub const INITIAL_CLIENT: (i32, i32) = (1100, 760);
/// Minimum client size in logical px.
pub const MIN_CLIENT: (i32, i32) = (480, 360);
/// `WM_COMMAND` id of "Open…" (also used by `xtask memprobe` for M6).
pub const IDM_OPEN: usize = 0x101;
/// Window property holding the first-paint time in ms + 1 (read by `xtask memprobe`).
pub const PROP_FIRST_PAINT: PCWSTR = w!("YAMDV.FirstPaintMs");

const WM_APP_AUTOSCROLL: u32 = WM_APP + 1;
const TIMER_ANIM: usize = 1;
const TIMER_SB_HIDE: usize = 2;
const TIMER_SB_FADE: usize = 3;
const TIMER_DRAG: usize = 4;
const TIMER_MEMSTATS: usize = 6;
const LINE_STEP: f64 = 40.0;

/// Developer options (feature `dev-tools`; never in release builds).
#[derive(Debug, Clone, Default)]
pub struct DevOptions {
    /// Scroll top → bottom → top after the first paint, then set the title to `SCROLL_DONE`.
    pub autoscroll: bool,
    /// Write per-frame paint durations (ms) here.
    pub perf_log: Option<String>,
    /// Write the subsystem memory breakdown (JSON) here 2 s after the first paint (`mem-stats`).
    pub mem_stats: Option<String>,
}

/// Process-wide state shared by all windows (single UI thread).
struct Shared {
    dw: DWrite,
    lang: Lang,
    m: Metrics,
    read: ReadOpts,
    column_pct: f32,
    smooth: bool,
    autohide: bool,
    dev: DevOptions,
    #[cfg_attr(not(feature = "mem-stats"), allow(dead_code))]
    commit_at_start: u64,
}

thread_local! {
    static SHARED: RefCell<Option<Rc<Shared>>> = const { RefCell::new(None) };
    static WINDOWS: RefCell<HashMap<isize, Rc<RefCell<Win>>>> = RefCell::new(HashMap::new());
    static PENDING_OPEN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn shared() -> Option<Rc<Shared>> {
    SHARED.with(|s| s.borrow().clone())
}

/// Localized layout strings.
pub(crate) struct I18nStrings(Lang);

/// Layout strings for `lang`.
pub(crate) fn strings(lang: Lang) -> I18nStrings {
    I18nStrings(lang)
}

impl Strings for I18nStrings {
    fn alert(&self, a: Alert) -> &str {
        tr(
            self.0,
            match a {
                Alert::Note => Msg::CalloutNote,
                Alert::Tip => Msg::CalloutTip,
                Alert::Important => Msg::CalloutImportant,
                Alert::Warning => Msg::CalloutWarning,
                Alert::Caution => Msg::CalloutCaution,
            },
        )
    }
    fn placeholder(&self, p: Placeholder) -> &str {
        tr(
            self.0,
            match p {
                Placeholder::Table => Msg::LabelTable,
                Placeholder::Html => Msg::LabelHtml,
                Placeholder::Math => Msg::LabelMath,
                Placeholder::Footnotes => Msg::LabelFootnotes,
                Placeholder::FrontMatter => Msg::LabelFrontMatter,
            },
        )
    }
}

/// Scale a logical length to device pixels for `dpi`.
pub fn scale(v: i32, dpi: u32) -> i32 {
    let dpi = i32::try_from(dpi).unwrap_or(96);
    (v * dpi + 48) / 96
}

/// Column geometry (B.6.1): `clamp(pct × client, min(360, avail − 48), avail − 48)`, centered.
pub fn column(client_w: f32, pct: f32) -> (f32, f32) {
    let avail = client_w;
    let max = (avail - 48.0).max(1.0);
    let min = 360.0f32.min(max);
    let w = (pct * client_w).clamp(min, max).floor();
    (((avail - w) / 2.0).floor(), w)
}

/// Ease-out cubic.
pub fn ease_out(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

#[derive(Debug, Clone, Copy)]
struct Anim {
    from: f64,
    to: f64,
    start: Instant,
    ms: f64,
}

enum View {
    Empty { note: Option<Msg> },
    Error { reason: Msg, path: String },
    Doc(Box<DocView>),
}

struct DocView {
    content: Content,
    viewer: Viewer,
    #[allow(dead_code)]
    info: FileInfo,
    #[allow(dead_code)]
    encoding: &'static str,
}

#[derive(Debug, Clone)]
enum Pending {
    Large { path: String },
    Binary { path: String, plain: bool },
}

const BTN_CANCEL: u32 = 0;
const BTN_PLAIN: u32 = 1;
const BTN_RENDER: u32 = 2;
const BTN_OPEN: u32 = 3;

#[derive(Default)]
struct Scrollbar {
    alpha: f32,
    hover: bool,
    drag: Option<f64>,
    fade_start: Option<Instant>,
}

struct Win {
    hwnd: HWND,
    scale: f32,
    dark: bool,
    pal: Palette,
    cw: f32,
    ch: f32,
    view: View,
    sel: Option<(u32, u32)>,
    selecting: bool,
    clicks: (u32, i32, i32, i32),
    anim: Option<Anim>,
    sb: Scrollbar,
    tracking: bool,
    dialog: Option<(ConfirmDialog, Pending)>,
    renderer: Option<Renderer>,
    first_paint: bool,
    drag_dir: f64,
    perf: Vec<f64>,
    auto_phase: u8,
    auto_steps: u32,
}

enum Action {
    OpenDialog,
    NewWindow(String),
    Close,
}

fn rgb_text(dark: bool) -> u32 {
    if dark { 0xffffff } else { 0x1f1f1f }
}

impl Win {
    fn view_h(&self) -> f64 {
        f64::from(self.ch)
    }

    fn col(&self, sh: &Shared) -> (f32, f32) {
        column(self.cw, sh.column_pct)
    }

    fn invalidate(&self) {
        // SAFETY: valid hwnd.
        let _ = unsafe { InvalidateRect(Some(self.hwnd), None, false) };
    }

    fn set_title(&self, s: &str) {
        // SAFETY: valid hwnd; HSTRING outlives the call.
        let _ = unsafe { SetWindowTextW(self.hwnd, &HSTRING::from(s)) };
    }

    fn reduced_motion(&self) -> bool {
        !sys::animations_enabled()
    }

    // ---- opening -------------------------------------------------------------------------

    fn show_error(&mut self, reason: Msg, path: &str) {
        self.view = View::Error {
            reason,
            path: ypath::display_form(path),
        };
        self.sel = None;
        self.set_title("YAMDV");
        self.invalidate();
    }

    fn open(&mut self, sh: &Shared, path: &str, force: Option<bool>, allow_binary: bool) {
        let Some(full) = sys::full_path(path) else {
            self.show_error(Msg::InvalidPath, path);
            return;
        };
        if ypath::validate(&full).is_err() {
            self.show_error(Msg::InvalidPath, &full);
            return;
        }
        let long = ypath::long_form(&full);
        let (info, is_dir) = match sys::stat(&long) {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.show_error(Msg::NotFound, &full);
                return;
            }
            Err(_) => {
                self.show_error(Msg::ReadError, &full);
                return;
            }
        };
        if is_dir {
            self.show_error(Msg::DropFolders, &full);
            return;
        }
        match decode::size_guard(info.size) {
            Guard::Refuse => {
                self.show_error(Msg::TooLarge, &full);
                return;
            }
            Guard::AskLarge if force.is_none() => {
                self.ask(
                    sh,
                    Msg::LargeTitle,
                    Msg::LargeBody,
                    vec![
                        (Msg::Cancel, BTN_CANCEL),
                        (Msg::RenderAnyway, BTN_RENDER),
                        (Msg::OpenPlain, BTN_PLAIN),
                    ],
                    Pending::Large { path: full },
                );
                return;
            }
            _ => {}
        }
        let bytes = match std::fs::read(&long) {
            Ok(b) => b,
            Err(_) => {
                self.show_error(Msg::ReadError, &full);
                return;
            }
        };
        if !allow_binary && decode::binary_guard(&bytes) == Guard::AskBinary {
            drop(bytes);
            self.ask(
                sh,
                Msg::BinaryTitle,
                Msg::BinaryBody,
                vec![(Msg::Cancel, BTN_CANCEL), (Msg::OpenAnyway, BTN_OPEN)],
                Pending::Binary {
                    path: full,
                    plain: force == Some(true),
                },
            );
            return;
        }
        let d = decode::decode(&bytes);
        drop(bytes);
        let plain = force == Some(true) || ypath::extension(&full) == "txt";
        let (_, col_w) = self.col(sh);
        let content = if plain {
            Content::plain(d.text)
        } else {
            Content::Markdown(Document::new(d.text, sh.read))
        };
        let viewer = Viewer::new(&content, col_w, sh.m);
        self.view = View::Doc(Box::new(DocView {
            content,
            viewer,
            info,
            encoding: d.encoding,
        }));
        self.sel = None;
        self.anim = None;
        self.set_title(&format!(
            "{} \u{2014} YAMDV",
            ypath::file_name(&ypath::display_form(&full))
        ));
        self.invalidate();
    }

    fn ask(&mut self, sh: &Shared, title: Msg, body: Msg, buttons: Vec<(Msg, u32)>, p: Pending) {
        let b = buttons
            .into_iter()
            .map(|(m, id)| Button {
                label: tr(sh.lang, m).to_string(),
                id,
            })
            .collect();
        self.dialog = Some((
            ConfirmDialog::new(tr(sh.lang, title), tr(sh.lang, body), b, BTN_CANCEL),
            p,
        ));
        self.invalidate();
    }

    fn dialog_done(&mut self, sh: &Shared, id: u32) {
        let Some((_, p)) = self.dialog.take() else {
            return;
        };
        match (p, id) {
            (_, BTN_CANCEL) => {}
            (Pending::Large { path }, BTN_PLAIN) => self.open(sh, &path, Some(true), false),
            (Pending::Large { path }, BTN_RENDER) => self.open(sh, &path, Some(false), false),
            (Pending::Binary { path, plain }, BTN_OPEN) => self.open(sh, &path, Some(plain), true),
            _ => {}
        }
        self.invalidate();
    }

    fn geometry(&self, sh: &Shared) -> Option<Geometry> {
        let (d, _) = self.dialog.as_ref()?;
        Some(d.geometry(self.cw, self.ch, &mut |t, s, wgt, mw| {
            sh.dw.measure(t, s, wgt, mw)
        }))
    }

    // ---- scrolling -----------------------------------------------------------------------

    fn doc(&mut self) -> Option<&mut DocView> {
        match &mut self.view {
            View::Doc(d) => Some(d),
            _ => None,
        }
    }

    fn scroll_pos(&self) -> f64 {
        match &self.view {
            View::Doc(d) => d.viewer.scroll,
            _ => 0.0,
        }
    }

    /// Moves to `y` (DIPs) with `ScrollWindowEx` + exposed-strip painting.
    fn set_scroll(&mut self, y: f64) {
        let scale = f64::from(self.scale);
        let vh = self.view_h();
        let Some(d) = self.doc() else { return };
        let old = d.viewer.scroll;
        let y = (y.clamp(0.0, d.viewer.max_scroll(vh)) * scale).round() / scale;
        if (y - old).abs() < 1e-6 {
            return;
        }
        d.viewer.scroll_to(y, vh);
        let dy = ((old * scale).round() - (y * scale).round()) as i32;
        let ch_px = (self.ch * self.scale) as i32;
        // SAFETY: valid hwnd; scroll the client area and invalidate the exposed strip.
        unsafe {
            if dy.abs() < ch_px && self.dialog.is_none() {
                ScrollWindowEx(self.hwnd, 0, dy, None, None, None, None, SW_INVALIDATE);
                // The overlay scrollbar moved with the bits: repaint its strip.
                let mut rc = RECT::default();
                let _ = GetClientRect(self.hwnd, &mut rc);
                rc.left = rc.right - (16.0 * self.scale) as i32;
                let _ = InvalidateRect(Some(self.hwnd), Some(&rc), false);
            } else {
                let _ = InvalidateRect(Some(self.hwnd), None, false);
            }
        }
        self.show_scrollbar();
        // SAFETY: valid hwnd; paints the exposed strip now.
        let _ = unsafe { UpdateWindow(self.hwnd) };
    }

    fn scroll_by(&mut self, sh: &Shared, dy: f64, animate: bool) {
        let cur = self.scroll_pos();
        let base = self.anim.map_or(cur, |a| a.to);
        let target = base + dy;
        if animate && sh.smooth && !self.reduced_motion() {
            let vh = self.view_h();
            let max = match &self.view {
                View::Doc(d) => d.viewer.max_scroll(vh),
                _ => 0.0,
            };
            self.anim = Some(Anim {
                from: cur,
                to: target.clamp(0.0, max),
                start: Instant::now(),
                ms: 150.0,
            });
            // SAFETY: valid hwnd; the timer runs only while animating (T7).
            unsafe { SetTimer(Some(self.hwnd), TIMER_ANIM, 8, None) };
        } else {
            self.anim = None;
            self.set_scroll(target);
        }
    }

    fn tick_anim(&mut self) {
        let Some(a) = self.anim else {
            // SAFETY: valid hwnd.
            let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_ANIM) };
            return;
        };
        let t = a.start.elapsed().as_secs_f64() * 1000.0 / a.ms;
        let y = a.from + (a.to - a.from) * ease_out(t);
        self.set_scroll(y);
        if t >= 1.0 {
            self.anim = None;
            // SAFETY: valid hwnd.
            let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_ANIM) };
        }
    }

    fn show_scrollbar(&mut self) {
        if !matches!(self.view, View::Doc(_)) {
            return;
        }
        let was = self.sb.alpha;
        self.sb.alpha = 1.0;
        self.sb.fade_start = None;
        if let Some(sh) = shared()
            && sh.autohide
        {
            // SAFETY: valid hwnd; one-shot hide timer (killed when it fires).
            unsafe {
                let _ = KillTimer(Some(self.hwnd), TIMER_SB_FADE);
                SetTimer(Some(self.hwnd), TIMER_SB_HIDE, 1000, None);
            }
        }
        if was < 1.0 {
            self.invalidate_scrollbar();
        }
    }

    fn invalidate_scrollbar(&self) {
        let mut rc = RECT::default();
        // SAFETY: valid hwnd and out-pointer.
        unsafe {
            let _ = GetClientRect(self.hwnd, &mut rc);
            rc.left = rc.right - (16.0 * self.scale) as i32;
            let _ = InvalidateRect(Some(self.hwnd), Some(&rc), false);
        }
    }

    /// Thumb rectangle (client DIPs).
    fn thumb(&self) -> Option<Rect> {
        let View::Doc(d) = &self.view else {
            return None;
        };
        let vh = self.view_h();
        let total = d.viewer.total_height();
        if total <= vh + 0.5 {
            return None;
        }
        let w = if self.sb.hover || self.sb.drag.is_some() {
            12.0
        } else {
            8.0
        };
        let th = (vh * vh / total).max(32.0);
        let ty = d.viewer.scroll / d.viewer.max_scroll(vh).max(1.0) * (vh - th);
        Some(Rect::new(self.cw - 2.0 - w, ty as f32, w, th as f32))
    }

    // ---- selection -----------------------------------------------------------------------

    fn hit(&self, sh: &Shared, x: f32, y: f32) -> Option<Hit> {
        let View::Doc(d) = &self.view else {
            return None;
        };
        let (cx, _) = self.col(sh);
        d.viewer
            .hit(&d.content, x - cx, f64::from(y) + d.viewer.scroll)
    }

    fn over_text(&self, sh: &Shared, x: f32, y: f32) -> bool {
        let View::Doc(d) = &self.view else {
            return false;
        };
        let (cx, cw) = self.col(sh);
        if x < cx || x > cx + cw {
            return false;
        }
        let docy = f64::from(y) + d.viewer.scroll;
        let bi = d.viewer.block_at_y(docy);
        let Some(bl) = d.viewer.layout_of(bi) else {
            return false;
        };
        let by = d.viewer.block_top(&d.content, bi);
        let ly = (docy - by) as f32;
        bl.paras.iter().any(|p| {
            p.map.is_some()
                && ly >= p.y
                && ly < p.y + p.shaped.height()
                && x - cx >= p.x
                && x - cx <= p.x + p.shaped.width().max(4.0)
        })
    }

    fn copy(&self) {
        let (View::Doc(d), Some((a, b))) = (&self.view, self.sel) else {
            return;
        };
        let (a, b) = (a.min(b), a.max(b));
        if a == b {
            return;
        }
        let text = d.content.copy_text(a..b);
        sys::copy_text(self.hwnd, &text);
    }

    // ---- painting ------------------------------------------------------------------------

    fn paint(&mut self, sh: &Shared) {
        let t0 = Instant::now();
        let mut ps = PAINTSTRUCT::default();
        // SAFETY: standard BeginPaint/EndPaint pair.
        let hdc = unsafe { BeginPaint(self.hwnd, &mut ps) };
        let vh = self.view_h();
        let (cx, cwid) = self.col(sh);
        if let View::Doc(d) = &mut self.view {
            if (d.viewer.width - cwid).abs() > 0.5 {
                d.viewer.set_width(&d.content, cwid, vh);
            }
            let strings = I18nStrings(sh.lang);
            let mut shaper = DwShaper(&sh.dw);
            let mut env = Env {
                shaper: &mut shaper,
                m: sh.m,
                pal: &self.pal,
                strings: &strings,
                single_newline_break: sh.read.single_newline_break,
            };
            d.viewer.prepare(&mut d.content, &mut env, vh);
        }
        let geo = self.geometry(sh);
        if let Some(r) = &self.renderer {
            let me = &*self;
            r.paint(hdc, ps.rcPaint, me.pal.text, &mut |band| {
                me.draw(sh, band, cx, geo.as_ref())
            });
        }
        // SAFETY: matches BeginPaint.
        let _ = unsafe { EndPaint(self.hwnd, &ps) };
        if self.auto_phase > 0 {
            self.perf.push(t0.elapsed().as_secs_f64() * 1000.0);
        }
        if !self.first_paint {
            self.first_paint = true;
            let ms = crate::perf::ms_since_process_start();
            // SAFETY: valid hwnd; the property value is a plain integer (ms + 1), never a handle.
            unsafe {
                let _ = SetPropW(
                    self.hwnd,
                    PROP_FIRST_PAINT,
                    Some(HANDLE(std::ptr::without_provenance_mut(ms as usize + 1))),
                );
            }
            if sh.dev.autoscroll && matches!(self.view, View::Doc(_)) {
                self.auto_phase = 1;
                // SAFETY: valid hwnd.
                let _ = unsafe {
                    PostMessageW(Some(self.hwnd), WM_APP_AUTOSCROLL, WPARAM(0), LPARAM(0))
                };
            }
            if sh.dev.mem_stats.is_some() {
                // SAFETY: valid hwnd; one-shot (killed when it fires).
                unsafe { SetTimer(Some(self.hwnd), TIMER_MEMSTATS, 2000, None) };
            }
        }
    }

    fn shape_ui(
        &self,
        sh: &Shared,
        text: &str,
        size: f32,
        weight: u16,
        color: u32,
        max_w: Option<f32>,
    ) -> Box<dyn yamdv_layout::shaper::Shaped> {
        let s = Style {
            family: Family::Body,
            size,
            weight,
            italic: false,
            color,
            strike: false,
            underline: false,
        };
        DwShaper(&sh.dw).shape(&ParaSpec {
            text,
            base: s,
            spans: vec![],
            width: max_w,
            line_pitch: (size * 1.4).round(),
            rtl: false,
            tab_spaces: None,
        })
    }

    fn draw(&self, sh: &Shared, b: &Band<'_>, cx: f32, geo: Option<&Geometry>) {
        let pal = &self.pal;
        b.clear(pal.bg);
        match &self.view {
            View::Empty { note } => {
                let msg = tr(sh.lang, note.unwrap_or(Msg::EmptyState));
                let t = self.shape_ui(
                    sh,
                    msg,
                    15.0,
                    400,
                    pal.text_muted,
                    Some((self.cw - 48.0).max(100.0)),
                );
                let x = ((self.cw - t.width()) / 2.0).round();
                let y = ((self.ch - t.height()) / 2.0).round();
                b.text(t.as_ref(), x, y);
            }
            View::Error { reason, path } => {
                let w = 520.0f32.min(self.cw - 48.0).max(200.0);
                let inner = w - 48.0;
                let t = self.shape_ui(
                    sh,
                    tr(sh.lang, Msg::CannotOpen),
                    15.0,
                    600,
                    pal.text,
                    Some(inner),
                );
                let r = self.shape_ui(sh, tr(sh.lang, *reason), 13.0, 400, pal.text, Some(inner));
                let p = self.shape_ui(sh, path, 12.0, 400, pal.text_muted, Some(inner));
                let h = 24.0 + t.height() + 8.0 + r.height() + 8.0 + p.height() + 24.0;
                let x = ((self.cw - w) / 2.0).round();
                let y = ((self.ch - h) / 2.0).round();
                let card = Rect::new(x, y, w, h);
                b.fill(card, pal.code_bg, 1.0, 8.0);
                b.stroke(card, pal.code_border, 8.0, 1.0);
                let mut yy = y + 24.0;
                b.text(t.as_ref(), x + 24.0, yy);
                yy += t.height() + 8.0;
                b.text(r.as_ref(), x + 24.0, yy);
                yy += r.height() + 8.0;
                b.text(p.as_ref(), x + 24.0, yy);
            }
            View::Doc(d) => {
                let sel = self.sel.map(|(a, c)| a.min(c)..a.max(c));
                draw_doc(b, &d.content, &d.viewer, pal, cx, sel, self.view_h());
                if let Some(t) = self.thumb()
                    && self.sb.alpha > 0.0
                {
                    let a = if self.sb.hover || self.sb.drag.is_some() {
                        0.55
                    } else {
                        0.35
                    };
                    b.fill(t, pal.text, a * self.sb.alpha, t.w / 2.0);
                }
            }
        }
        if let (Some((d, _)), Some(g)) = (&self.dialog, geo) {
            b.fill(
                Rect::new(0.0, 0.0, self.cw, self.ch),
                0x000000,
                dlg::BACKDROP_ALPHA,
                0.0,
            );
            let c = Rect::new(g.card.x, g.card.y, g.card.w, g.card.h);
            b.fill(c, pal.bg, 1.0, 8.0);
            b.stroke(c, pal.code_border, 8.0, 1.0);
            let t = self.shape_ui(
                sh,
                &d.title,
                dlg::TITLE_SIZE,
                600,
                pal.text,
                Some(g.title.w),
            );
            b.text(t.as_ref(), g.title.x, g.title.y);
            let body = self.shape_ui(sh, &d.body, dlg::BODY_SIZE, 400, pal.text, Some(g.body.w));
            b.text(body.as_ref(), g.body.x, g.body.y);
            for (i, (btn, r)) in d.buttons.iter().zip(g.buttons.iter()).enumerate() {
                let rr = Rect::new(r.x, r.y, r.w, r.h);
                let primary = i == d.primary();
                if primary {
                    b.fill(rr, pal.accent, 1.0, 6.0);
                } else {
                    b.fill(rr, pal.bg, 1.0, 6.0);
                    b.stroke(rr, pal.code_border, 6.0, 1.0);
                }
                if d.hover == Some(i) {
                    b.fill(rr, rgb_text(self.dark), 0.06, 6.0);
                }
                let color = if primary { 0xffffff } else { pal.text };
                let l = self.shape_ui(sh, &btn.label, dlg::BODY_SIZE, 400, color, None);
                b.text(
                    l.as_ref(),
                    (r.x + (r.w - l.width()) / 2.0).round(),
                    (r.y + (r.h - l.height()) / 2.0).round(),
                );
                if d.focus_visible && d.focus == i {
                    // B.16: 2 px focus ring with 1 px offset.
                    b.stroke(
                        Rect::new(rr.x - 3.0, rr.y - 3.0, rr.w + 6.0, rr.h + 6.0),
                        pal.accent,
                        8.0,
                        2.0,
                    );
                }
            }
        }
    }

    fn set_dark(&mut self, dark: bool) {
        self.dark = dark;
        self.pal = theme::palette("default", dark);
        apply_mode(self.hwnd, if dark { Mode::Dark } else { Mode::Light });
        if let View::Doc(d) = &mut self.view {
            d.viewer.invalidate_layouts();
        }
        self.invalidate();
    }

    // ---- input ---------------------------------------------------------------------------

    fn key(&mut self, sh: &Shared, vk: u16) -> Option<Action> {
        // SAFETY: GetKeyState has no preconditions.
        let ctrl = unsafe { GetKeyState(i32::from(VK_CONTROL.0)) } < 0;
        // SAFETY: as above.
        let shift = unsafe { GetKeyState(i32::from(VK_SHIFT.0)) } < 0;
        if let Some((d, _)) = &mut self.dialog {
            let k = match vk {
                v if v == VK_RETURN.0 => Some(dlg::Key::Enter),
                v if v == VK_ESCAPE.0 => Some(dlg::Key::Esc),
                v if v == VK_TAB.0 => Some(if shift {
                    dlg::Key::ShiftTab
                } else {
                    dlg::Key::Tab
                }),
                v if v == VK_SPACE.0 => Some(dlg::Key::Space),
                _ => None,
            };
            match k.map(|k| d.key(k)) {
                Some(Outcome::Chosen(id)) => self.dialog_done(sh, id),
                Some(Outcome::Cancelled) => self.dialog_done(sh, BTN_CANCEL),
                Some(Outcome::Open { .. }) => self.invalidate(),
                None => {}
            }
            return None;
        }
        let page = (self.view_h() - LINE_STEP).max(LINE_STEP);
        match vk {
            v if v == VK_ESCAPE.0 => return Some(Action::Close),
            v if ctrl && v == u16::from(b'W') => return Some(Action::Close),
            v if ctrl && v == u16::from(b'O') => return Some(Action::OpenDialog),
            v if ctrl && v == u16::from(b'A') => {
                if let View::Doc(d) = &self.view {
                    self.sel = Some((
                        0,
                        u32::try_from(d.content.source().len()).unwrap_or(u32::MAX),
                    ));
                    self.invalidate();
                }
            }
            v if ctrl && v == u16::from(b'C') => self.copy(),
            v if v == VK_UP.0 => self.scroll_by(sh, -LINE_STEP, true),
            v if v == VK_DOWN.0 => self.scroll_by(sh, LINE_STEP, true),
            v if v == VK_PRIOR.0 => self.scroll_by(sh, -page, true),
            v if v == VK_NEXT.0 => self.scroll_by(sh, page, true),
            v if v == VK_SPACE.0 => self.scroll_by(sh, if shift { -page } else { page }, true),
            v if v == VK_HOME.0 => self.scroll_by(sh, -self.scroll_pos() - 1.0, true),
            v if v == VK_END.0 => self.scroll_by(sh, 1e12, true),
            _ => {}
        }
        None
    }

    fn mouse_down(&mut self, sh: &Shared, x: f32, y: f32) {
        if self.dialog.is_some() {
            if let Some(g) = self.geometry(sh)
                && let Some((d, _)) = &mut self.dialog
            {
                match d.click(&g, x, y) {
                    Outcome::Chosen(id) => self.dialog_done(sh, id),
                    Outcome::Cancelled => self.dialog_done(sh, BTN_CANCEL),
                    Outcome::Open { .. } => self.invalidate(),
                }
            }
            return;
        }
        if let Some(t) = self.thumb()
            && x >= self.cw - 16.0
        {
            if y >= t.y && y < t.y + t.h {
                self.sb.drag = Some(f64::from(y - t.y));
                // SAFETY: valid hwnd.
                unsafe { SetCapture(self.hwnd) };
            } else {
                let page = (self.view_h() - LINE_STEP).max(LINE_STEP);
                self.scroll_by(sh, if y < t.y { -page } else { page }, true);
            }
            self.invalidate_scrollbar();
            return;
        }
        let Some(h) = self.hit(sh, x, y) else {
            if self.sel.take().is_some() {
                self.invalidate();
            }
            return;
        };
        // Click counting (double = word, triple = block), within the double-click time/area.
        // SAFETY: no preconditions.
        let (t, dt, dx, dy) = unsafe {
            (
                GetMessageTime(),
                GetDoubleClickTime(),
                GetSystemMetrics(SM_CXDOUBLECLK),
                GetSystemMetrics(SM_CYDOUBLECLK),
            )
        };
        let (n, lt, lx, ly) = self.clicks;
        let (xi, yi) = (x as i32, y as i32);
        let same = u32::try_from(t.wrapping_sub(lt)).unwrap_or(u32::MAX) <= dt
            && (xi - lx).abs() <= dx
            && (yi - ly).abs() <= dy;
        let count = if same { n % 3 + 1 } else { 1 };
        self.clicks = (count, t, xi, yi);
        // SAFETY: GetKeyState has no preconditions.
        let shift = unsafe { GetKeyState(i32::from(VK_SHIFT.0)) } < 0;
        let View::Doc(d) = &self.view else { return };
        match count {
            2 => {
                if let Some(r) = d.viewer.word_at(&h) {
                    self.sel = Some((r.start, r.end));
                }
            }
            3 => {
                if let Some(r) = d.viewer.para_at(&h) {
                    self.sel = Some((r.start, r.end));
                }
            }
            _ => {
                self.sel = match (shift, self.sel) {
                    (true, Some((a, _))) => Some((a, h.source)),
                    _ => Some((h.source, h.source)),
                };
                self.selecting = true;
                // SAFETY: valid hwnd.
                unsafe { SetCapture(self.hwnd) };
            }
        }
        self.invalidate();
    }

    fn mouse_move(&mut self, sh: &Shared, x: f32, y: f32) {
        if !self.tracking {
            let mut tme = TRACKMOUSEEVENT {
                cbSize: u32::try_from(size_of::<TRACKMOUSEEVENT>()).unwrap_or(0),
                dwFlags: TME_LEAVE,
                hwndTrack: self.hwnd,
                dwHoverTime: 0,
            };
            // SAFETY: valid structure.
            if unsafe { TrackMouseEvent(&mut tme) }.is_ok() {
                self.tracking = true;
            }
        }
        if let Some((d, _)) = &mut self.dialog {
            let g = d.geometry(self.cw, self.ch, &mut |t, s, wgt, mw| {
                sh.dw.measure(t, s, wgt, mw)
            });
            if d.mouse_move(&g, x, y) {
                self.invalidate();
            }
            return;
        }
        if let Some(grab) = self.sb.drag {
            let vh = self.view_h();
            if let (View::Doc(dv), Some(t)) = (&self.view, self.thumb()) {
                let max = dv.viewer.max_scroll(vh);
                let track = (vh - f64::from(t.h)).max(1.0);
                let target = (f64::from(y) - grab) / track * max;
                self.anim = None;
                self.set_scroll(target);
            }
            return;
        }
        let hover = x >= self.cw - 16.0 && self.thumb().is_some();
        if hover != self.sb.hover {
            self.sb.hover = hover;
            if hover {
                self.show_scrollbar();
            }
            self.invalidate_scrollbar();
        }
        if self.selecting {
            self.drag_dir = if y < 0.0 {
                f64::from(y).max(-200.0)
            } else if y > self.ch {
                f64::from(y - self.ch).min(200.0)
            } else {
                0.0
            };
            if self.drag_dir != 0.0 {
                // SAFETY: valid hwnd; runs only while drag-selecting outside the window.
                unsafe { SetTimer(Some(self.hwnd), TIMER_DRAG, 30, None) };
            }
            let yy = y.clamp(0.0, self.ch - 1.0);
            if let Some(h) = self.hit(sh, x, yy)
                && let Some((a, _)) = self.sel
            {
                self.sel = Some((a, h.source));
                self.invalidate();
            }
        }
    }

    fn mouse_up(&mut self) {
        if self.selecting || self.sb.drag.is_some() {
            self.selecting = false;
            self.sb.drag = None;
            self.drag_dir = 0.0;
            // SAFETY: valid hwnd / capture release.
            unsafe {
                let _ = KillTimer(Some(self.hwnd), TIMER_DRAG);
                let _ = ReleaseCapture();
            }
            self.invalidate_scrollbar();
        }
    }

    fn wheel(&mut self, sh: &Shared, delta: i16) {
        if self.dialog.is_some() {
            return;
        }
        let lines = f64::from(sys::wheel_lines());
        let d = -f64::from(delta) / 120.0 * lines * LINE_STEP;
        // Precision touchpads send fractional notches: apply directly, no animation.
        let notch = delta % 120 == 0;
        self.scroll_by(sh, d, notch);
    }

    fn drop_files(&mut self, sh: &Shared, hdrop: HDROP) -> Vec<Action> {
        let mut files = Vec::new();
        let mut folder = false;
        // SAFETY: hdrop comes from WM_DROPFILES; lengths are queried before each copy and the
        // handle is released with DragFinish.
        unsafe {
            let n = DragQueryFileW(hdrop, 0xFFFF_FFFF, None);
            for i in 0..n.min(1000) {
                let len = DragQueryFileW(hdrop, i, None) as usize;
                if len == 0 || len > 32_767 {
                    continue;
                }
                let mut buf = vec![0u16; len + 1];
                let got = DragQueryFileW(hdrop, i, Some(&mut buf)) as usize;
                if got != len {
                    continue;
                }
                let p = String::from_utf16_lossy(buf.get(..len).unwrap_or(&[]));
                if std::fs::metadata(ypath::long_form(&p)).is_ok_and(|m| m.is_dir()) {
                    folder = true;
                } else {
                    files.push(p);
                }
            }
            DragFinish(hdrop);
        }
        let mut actions = Vec::new();
        let mut it = files.into_iter();
        if let Some(first) = it.next() {
            self.open(sh, &first, None, false);
            actions.extend(it.map(Action::NewWindow));
        } else if folder && !matches!(self.view, View::Doc(_)) {
            self.view = View::Empty {
                note: Some(Msg::DropFolders),
            };
            self.invalidate();
        }
        actions
    }

    fn autoscroll_step(&mut self) {
        let vh = self.view_h();
        let (pos, max) = match &self.view {
            View::Doc(d) => (d.viewer.scroll, d.viewer.max_scroll(vh)),
            _ => return,
        };
        self.auto_steps += 1;
        let step = 120.0;
        let done = self.auto_steps > 6000
            || match self.auto_phase {
                1 if pos >= max - 0.5 => {
                    self.auto_phase = 2;
                    false
                }
                2 if pos <= 0.5 => true,
                _ => false,
            };
        if done {
            self.auto_phase = 0;
            if let Some(sh) = shared()
                && let Some(p) = &sh.dev.perf_log
            {
                let _ = std::fs::write(p, crate::perf::report(&self.perf));
            }
            self.set_title("SCROLL_DONE");
            return;
        }
        self.set_scroll(if self.auto_phase == 1 {
            pos + step
        } else {
            pos - step
        });
        // SAFETY: valid hwnd.
        let _ = unsafe { PostMessageW(Some(self.hwnd), WM_APP_AUTOSCROLL, WPARAM(0), LPARAM(0)) };
    }

    fn timer(&mut self, sh: &Shared, id: usize) {
        match id {
            TIMER_ANIM => self.tick_anim(),
            TIMER_SB_HIDE => {
                // SAFETY: valid hwnd.
                let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_SB_HIDE) };
                if self.sb.hover || self.sb.drag.is_some() {
                    return;
                }
                if self.reduced_motion() {
                    self.sb.alpha = 0.0;
                    self.invalidate_scrollbar();
                } else {
                    self.sb.fade_start = Some(Instant::now());
                    // SAFETY: valid hwnd; runs only during the 300 ms fade.
                    unsafe { SetTimer(Some(self.hwnd), TIMER_SB_FADE, 16, None) };
                }
            }
            TIMER_SB_FADE => {
                let t = self
                    .sb
                    .fade_start
                    .map_or(1.0, |s| s.elapsed().as_secs_f32() * 1000.0 / 300.0);
                self.sb.alpha = (1.0 - t).max(0.0);
                if t >= 1.0 {
                    self.sb.fade_start = None;
                    // SAFETY: valid hwnd.
                    let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_SB_FADE) };
                }
                self.invalidate_scrollbar();
            }
            TIMER_DRAG => {
                if self.drag_dir == 0.0 || !self.selecting {
                    // SAFETY: valid hwnd.
                    let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_DRAG) };
                    return;
                }
                let d = self.drag_dir / 4.0;
                self.set_scroll(self.scroll_pos() + d);
                let mut pt = POINT::default();
                // SAFETY: valid out-pointer / hwnd.
                unsafe {
                    let _ = GetCursorPos(&mut pt);
                    let _ = ScreenToClient(self.hwnd, &mut pt);
                }
                let (x, y) = (pt.x as f32 / self.scale, pt.y as f32 / self.scale);
                self.mouse_move(sh, x, y);
            }
            TIMER_MEMSTATS => {
                // SAFETY: valid hwnd.
                let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_MEMSTATS) };
                #[cfg(feature = "mem-stats")]
                self.write_mem_stats(sh);
            }
            _ => {}
        }
    }

    #[cfg(feature = "mem-stats")]
    fn write_mem_stats(&self, sh: &Shared) {
        let Some(path) = &sh.dev.mem_stats else {
            return;
        };
        let (src, idx, cache, layouts, paras, items) = match &self.view {
            View::Doc(d) => {
                let (s, i, c) = match &d.content {
                    Content::Markdown(doc) => doc.heap_bytes(),
                    Content::Plain { text, chunks } => (text.capacity(), chunks.capacity() * 8, 0),
                };
                let (l, p, it) = d.viewer.layout_stats();
                (s, i, c, l, p, it)
            }
            _ => (0, 0, 0, 0, 0, 0),
        };
        let band = self
            .renderer
            .as_ref()
            .map_or(0, |r| crate::render::band_bytes(r.width()));
        let json = format!(
            "{{\"schema\":\"yamdv.memstats/1\",\"source_text_bytes\":{src},\"block_index_bytes\":{idx},\"detail_cache_bytes\":{cache},\"layout_cache\":{{\"layouts\":{layouts},\"paras\":{paras},\"items\":{items},\"model_bytes_estimate\":{}}},\"band_buffer_bytes\":{band},\"commit_bytes_now\":{},\"commit_bytes_at_start\":{},\"note\":\"DirectWrite and other native memory = commit growth minus the Rust-side numbers; see 002_REPORT.md\"}}\n",
            paras * 160 + items * 40,
            crate::perf::commit_bytes(),
            sh.commit_at_start
        );
        let _ = std::fs::write(path, json);
    }
}

/// Draws the visible part of a document (shared by windows and `--render-png`).
pub(crate) fn draw_doc(
    b: &Band<'_>,
    content: &Content,
    viewer: &Viewer,
    pal: &Palette,
    cx: f32,
    sel: Option<Range<u32>>,
    vh: f64,
) {
    let sel_alpha = f32::from(pal.selection_alpha_pct) / 100.0;
    for (_, y, bl) in viewer.visible(content, vh) {
        let y = y as f32;
        if !b.visible(y, bl.height.max(1.0)) {
            continue;
        }
        for it in &bl.items {
            match it {
                Item::Fill {
                    rect,
                    color,
                    radius,
                } => b.fill(rect.offset(cx, y), *color, 1.0, *radius),
                Item::Stroke {
                    rect,
                    color,
                    radius,
                } => b.stroke(rect.offset(cx, y), *color, *radius, 1.0),
                Item::Bullet {
                    cx: bx,
                    cy,
                    size,
                    shape,
                    color,
                } => match shape {
                    Bullet::Disc => b.circle(cx + bx, y + cy, size / 2.0, *color, None),
                    Bullet::Ring => b.circle(cx + bx, y + cy, size / 2.0, *color, Some(1.2)),
                    Bullet::Square => b.fill(
                        Rect::new(cx + bx - size / 2.0, y + cy - size / 2.0, *size, *size),
                        *color,
                        1.0,
                        0.0,
                    ),
                },
                Item::FadeRight { rect, color } => b.fade_right(rect.offset(cx, y), *color),
                Item::Para(i) => {
                    let Some(p) = bl.paras.get(*i) else { continue };
                    if let Some(s) = &sel {
                        for r in viewer::selection_rects(p, s) {
                            b.fill(r.offset(cx + p.x, y + p.y), pal.accent, sel_alpha, 0.0);
                        }
                    }
                    b.text(p.shaped.as_ref(), cx + p.x, y + p.y);
                }
            }
        }
    }
}

fn apply_mode(hwnd: HWND, m: Mode) {
    let dark = windows::core::BOOL::from(m == Mode::Dark);
    // SAFETY: valid hwnd; pointer/size describe a BOOL on the stack.
    let _ = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&raw const dark).cast(),
            u32::try_from(size_of::<windows::core::BOOL>()).unwrap_or(4),
        )
    };
}

fn window_rect_for_client(w: i32, h: i32, dpi: u32) -> (i32, i32) {
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: w,
        bottom: h,
    };
    // SAFETY: rc is a valid in/out rect.
    if unsafe {
        AdjustWindowRectExForDpi(&mut rc, WS_OVERLAPPEDWINDOW, false, WINDOW_EX_STYLE(0), dpi)
    }
    .is_err()
    {
        return (w, h);
    }
    (rc.right - rc.left, rc.bottom - rc.top)
}

fn is_immersive_color_set(lparam: LPARAM) -> bool {
    if lparam.0 == 0 {
        return false;
    }
    // SAFETY: for WM_SETTINGCHANGE, a non-zero lParam points to a NUL-terminated wide string.
    let s = unsafe { PCWSTR(lparam.0 as *const u16).to_string() }.unwrap_or_default();
    s == "ImmersiveColorSet"
}

fn lparam_xy(lp: LPARAM, scale: f32) -> (f32, f32) {
    let x = i32::from((lp.0 & 0xFFFF) as u16 as i16);
    let y = i32::from(((lp.0 >> 16) & 0xFFFF) as u16 as i16);
    (x as f32 / scale, y as f32 / scale)
}

fn update_size(w: &mut Win, sh: &Shared) {
    let mut rc = RECT::default();
    // SAFETY: valid hwnd and out-pointer.
    let _ = unsafe { GetClientRect(w.hwnd, &mut rc) };
    let (pw, ph) = (rc.right - rc.left, rc.bottom - rc.top);
    w.cw = pw as f32 / w.scale;
    w.ch = ph as f32 / w.scale;
    if let Some(r) = &mut w.renderer {
        r.resize(pw.max(1));
    }
    let (_, colw) = w.col(sh);
    let vh = w.view_h();
    if let View::Doc(d) = &mut w.view {
        d.viewer.set_width(&d.content, colw, vh);
    }
    w.invalidate();
}

fn handle(
    w: &mut Win,
    sh: &Shared,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> (Option<LRESULT>, Vec<Action>) {
    let mut actions = Vec::new();
    let r = match msg {
        WM_ERASEBKGND => Some(LRESULT(1)),
        WM_PAINT => {
            w.paint(sh);
            Some(LRESULT(0))
        }
        WM_SIZE => {
            update_size(w, sh);
            Some(LRESULT(0))
        }
        WM_KEYDOWN => {
            let vk = u16::try_from(wp.0).unwrap_or(0);
            actions.extend(w.key(sh, vk));
            Some(LRESULT(0))
        }
        WM_COMMAND if (wp.0 & 0xFFFF) == IDM_OPEN => {
            actions.push(Action::OpenDialog);
            Some(LRESULT(0))
        }
        WM_MOUSEWHEEL => {
            let delta = ((wp.0 >> 16) & 0xFFFF) as u16 as i16;
            w.wheel(sh, delta);
            Some(LRESULT(0))
        }
        WM_LBUTTONDOWN => {
            let (x, y) = lparam_xy(lp, w.scale);
            w.mouse_down(sh, x, y);
            Some(LRESULT(0))
        }
        WM_MOUSEMOVE => {
            let (x, y) = lparam_xy(lp, w.scale);
            w.mouse_move(sh, x, y);
            Some(LRESULT(0))
        }
        WM_LBUTTONUP => {
            w.mouse_up();
            Some(LRESULT(0))
        }
        WM_MOUSELEAVE => {
            w.tracking = false;
            if w.sb.hover {
                w.sb.hover = false;
                w.invalidate_scrollbar();
            }
            Some(LRESULT(0))
        }
        WM_SETCURSOR if (lp.0 & 0xFFFF) as u32 == HTCLIENT => {
            let mut pt = POINT::default();
            // SAFETY: valid out-pointer / hwnd.
            unsafe {
                let _ = GetCursorPos(&mut pt);
                let _ = ScreenToClient(w.hwnd, &mut pt);
            }
            let (x, y) = (pt.x as f32 / w.scale, pt.y as f32 / w.scale);
            let ibeam = w.dialog.is_none()
                && !(x >= w.cw - 16.0 && w.thumb().is_some())
                && w.over_text(sh, x, y);
            // SAFETY: system cursors; no cleanup needed.
            unsafe { SetCursor(LoadCursorW(None, if ibeam { IDC_IBEAM } else { IDC_ARROW }).ok()) };
            Some(LRESULT(1))
        }
        WM_TIMER => {
            w.timer(sh, wp.0);
            Some(LRESULT(0))
        }
        WM_APP_AUTOSCROLL => {
            w.autoscroll_step();
            Some(LRESULT(0))
        }
        WM_DROPFILES => {
            actions.extend(w.drop_files(sh, HDROP(wp.0 as *mut core::ffi::c_void)));
            Some(LRESULT(0))
        }
        WM_SETTINGCHANGE => {
            if is_immersive_color_set(lp) {
                let dark = app_mode() == Mode::Dark;
                if dark != w.dark {
                    w.set_dark(dark);
                }
            }
            None
        }
        WM_DPICHANGED => {
            let dpi = (wp.0 & 0xFFFF) as u32;
            w.scale = dpi.max(48) as f32 / 96.0;
            if let Some(r) = &mut w.renderer {
                r.set_scale(&sh.dw, w.hwnd, w.scale);
            }
            // SAFETY: for WM_DPICHANGED lParam points to the suggested window RECT.
            let rc = unsafe { *(lp.0 as *const RECT) };
            // SAFETY: valid hwnd.
            let _ = unsafe {
                SetWindowPos(
                    w.hwnd,
                    None,
                    rc.left,
                    rc.top,
                    rc.right - rc.left,
                    rc.bottom - rc.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                )
            };
            update_size(w, sh);
            if let View::Doc(d) = &mut w.view {
                d.viewer.invalidate_layouts();
            }
            Some(LRESULT(0))
        }
        WM_GETMINMAXINFO => {
            // SAFETY: valid hwnd.
            let dpi = unsafe { GetDpiForWindow(w.hwnd) }.max(96);
            let (mw, mh) =
                window_rect_for_client(scale(MIN_CLIENT.0, dpi), scale(MIN_CLIENT.1, dpi), dpi);
            // SAFETY: for WM_GETMINMAXINFO lParam points to a writable MINMAXINFO.
            let mmi = unsafe { &mut *(lp.0 as *mut MINMAXINFO) };
            mmi.ptMinTrackSize = POINT { x: mw, y: mh };
            Some(LRESULT(0))
        }
        _ => None,
    };
    (r, actions)
}

fn run_actions(hwnd: HWND, actions: Vec<Action>) {
    let Some(sh) = shared() else { return };
    for a in actions {
        match a {
            Action::Close => {
                // SAFETY: destroying our own window.
                let _ = unsafe { DestroyWindow(hwnd) };
                return;
            }
            Action::NewWindow(p) => create_window(&sh, Some(&p)),
            Action::OpenDialog => {
                let f = sys::Filters {
                    markdown: tr(sh.lang, Msg::FilterMarkdown),
                    text: tr(sh.lang, Msg::FilterText),
                    all: tr(sh.lang, Msg::FilterAll),
                };
                // Modal: no window state is borrowed while the dialog runs.
                let paths = sys::open_dialog(hwnd, &f);
                sys::trim_working_set();
                let mut it = paths.into_iter();
                if let Some(first) = it.next() {
                    let win = WINDOWS.with(|m| m.borrow().get(&(hwnd.0 as isize)).cloned());
                    let in_place = win.as_ref().is_some_and(|w| {
                        w.try_borrow()
                            .is_ok_and(|w| !matches!(w.view, View::Doc(_)))
                    });
                    if in_place {
                        if let Some(w) = win
                            && let Ok(mut w) = w.try_borrow_mut()
                        {
                            w.open(&sh, &first, None, false);
                        }
                    } else {
                        create_window(&sh, Some(&first));
                    }
                }
                for p in it {
                    create_window(&sh, Some(&p));
                }
            }
        }
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_DESTROY {
        WINDOWS.with(|m| m.borrow_mut().remove(&(hwnd.0 as isize)));
        if WINDOWS.with(|m| m.borrow().is_empty()) {
            // SAFETY: no preconditions; closing the last window exits.
            unsafe { PostQuitMessage(0) };
        }
        return LRESULT(0);
    }
    let win = WINDOWS.with(|m| m.borrow().get(&(hwnd.0 as isize)).cloned());
    if let (Some(win), Some(sh)) = (win, shared()) {
        let out = match win.try_borrow_mut() {
            Ok(mut w) => Some(handle(&mut w, &sh, msg, wp, lp)),
            Err(_) => None,
        };
        if let Some((r, actions)) = out {
            run_actions(hwnd, actions);
            if let Some(r) = r {
                return r;
            }
        }
    }
    // SAFETY: default processing.
    unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
}

fn register_class() -> bool {
    // SAFETY: module handle of the running exe.
    let Ok(hinstance) = (unsafe { GetModuleHandleW(None) }) else {
        return false;
    };
    // SAFETY: icon resource 1 is embedded by build.rs; LR_SHARED handles need no cleanup.
    let icon = unsafe {
        LoadImageW(
            Some(hinstance.into()),
            PCWSTR(std::ptr::without_provenance(1)),
            IMAGE_ICON,
            0,
            0,
            LR_DEFAULTSIZE | LR_SHARED,
        )
    }
    .map(|h| HICON(h.0))
    .unwrap_or_default();
    let wc = WNDCLASSEXW {
        cbSize: u32::try_from(size_of::<WNDCLASSEXW>()).unwrap_or(0),
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wndproc),
        hInstance: hinstance.into(),
        hIcon: icon,
        hIconSm: icon,
        // SAFETY: system cursor; no cleanup needed.
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
        lpszClassName: CLASS_NAME,
        ..Default::default()
    };
    // SAFETY: wc is fully initialized and its strings are static.
    unsafe { RegisterClassExW(&wc) != 0 }
}

fn create_window(sh: &Shared, path: Option<&str>) {
    // SAFETY: module handle of the running exe.
    let Ok(hinstance) = (unsafe { GetModuleHandleW(None) }) else {
        return;
    };
    let mut pt = POINT::default();
    // SAFETY: valid out-pointer.
    let _ = unsafe { GetCursorPos(&mut pt) };
    // SAFETY: no preconditions.
    let mon = unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST) };
    let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
    // SAFETY: valid monitor handle and out-pointers.
    let _ = unsafe { GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
    let mut mi = MONITORINFO {
        cbSize: u32::try_from(size_of::<MONITORINFO>()).unwrap_or(0),
        ..Default::default()
    };
    // SAFETY: valid monitor handle and correctly sized MONITORINFO.
    let _ = unsafe { GetMonitorInfoW(mon, &mut mi) };
    let work = mi.rcWork;
    let (mut w, mut h) = window_rect_for_client(
        scale(INITIAL_CLIENT.0, dpi_x),
        scale(INITIAL_CLIENT.1, dpi_x),
        dpi_x,
    );
    w = w.min(work.right - work.left);
    h = h.min(work.bottom - work.top);
    let n = WINDOWS.with(|m| m.borrow().len()) as i32;
    let off = (n % 8) * scale(24, dpi_x);
    let x = work.left + (work.right - work.left - w) / 2 + off;
    let y = work.top + (work.bottom - work.top - h) / 2 + off;
    // SAFETY: class registered; title is static.
    let Ok(hwnd) = (unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            CLASS_NAME,
            w!("YAMDV"),
            WS_OVERLAPPEDWINDOW,
            x,
            y,
            w,
            h,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
    }) else {
        return;
    };
    // SAFETY: valid hwnd.
    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
    let scale_f = dpi as f32 / 96.0;
    let dark = app_mode() == Mode::Dark;
    let mut rc = RECT::default();
    // SAFETY: valid hwnd and out-pointer.
    let _ = unsafe { GetClientRect(hwnd, &mut rc) };
    let renderer = Renderer::new(&sh.dw, Some(hwnd), (rc.right - rc.left).max(1), scale_f).ok();
    let win = Win {
        hwnd,
        scale: scale_f,
        dark,
        pal: theme::palette("default", dark),
        cw: (rc.right - rc.left) as f32 / scale_f,
        ch: (rc.bottom - rc.top) as f32 / scale_f,
        view: View::Empty { note: None },
        sel: None,
        selecting: false,
        clicks: (0, 0, 0, 0),
        anim: None,
        sb: Scrollbar::default(),
        tracking: false,
        dialog: None,
        renderer,
        first_paint: false,
        drag_dir: 0.0,
        perf: Vec::new(),
        auto_phase: 0,
        auto_steps: 0,
    };
    let rcw = Rc::new(RefCell::new(win));
    WINDOWS.with(|m| m.borrow_mut().insert(hwnd.0 as isize, rcw.clone()));
    apply_mode(hwnd, if dark { Mode::Dark } else { Mode::Light });
    // SAFETY: valid hwnd.
    unsafe { DragAcceptFiles(hwnd, true) };
    if let Some(p) = path
        && let Ok(mut w) = rcw.try_borrow_mut()
    {
        w.open(sh, p, None, false);
    }
    // SAFETY: valid hwnd.
    let _ = unsafe { ShowWindow(hwnd, SW_SHOW) };
}

/// Reads `settings.ini` (read-only in this phase, R-SET-05) and returns the settings.
fn load_settings() -> Settings {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_string_lossy().into_owned()))
        .unwrap_or_default();
    let portable = std::path::Path::new(&exe_dir)
        .join("yamdv.portable")
        .exists();
    let dirs = yamdv_core::settings::resolve_dirs(
        &exe_dir,
        portable,
        &std::env::var("APPDATA").unwrap_or_default(),
        &std::env::var("LOCALAPPDATA").unwrap_or_default(),
    );
    let dir = match dirs {
        yamdv_core::settings::DataDirs::Installed { config, .. } => config,
        yamdv_core::settings::DataDirs::Portable { dir } => dir,
    };
    let path = std::path::Path::new(&dir).join("settings.ini");
    let bytes = std::fs::metadata(&path)
        .ok()
        .filter(|m| m.len() <= 4 * 1024 * 1024)
        .and_then(|_| std::fs::read(&path).ok())
        .unwrap_or_default();
    Settings::from_ini(&yamdv_core::ini::Ini::parse(&bytes))
}

/// Opens one window per path (or one empty window) and runs the message loop.
pub fn run(paths: &[String], dev: DevOptions) -> i32 {
    let commit_at_start = crate::perf::commit_bytes();
    // SAFETY: COM for IFileOpenDialog on this (UI) thread.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let s = load_settings();
    let Ok(dw) = DWrite::new(s.str("typography.body_font"), s.str("typography.code_font")) else {
        return 1;
    };
    let m = Metrics {
        font_size: s.int("typography.font_size") as f32,
        line_height: s.float("typography.line_height") as f32,
        para_spacing: s.int("typography.paragraph_spacing") as f32,
    };
    let shared = Shared {
        dw,
        lang: yamdv_core::i18n::resolve(s.str("general.language"), sys::ui_language_tag()),
        m,
        read: ReadOpts {
            subscript_tilde: s.bool("reading.subscript_tilde"),
            smart_punctuation: s.bool("reading.smart_punctuation"),
            single_newline_break: s.bool("reading.single_newline_break"),
        },
        column_pct: s.int("typography.column_width_pct") as f32 / 100.0,
        smooth: s.bool("reading.smooth_scrolling"),
        autohide: s.bool("reading.scrollbar_autohide"),
        dev,
        commit_at_start,
    };
    drop(s);
    let sh = Rc::new(shared);
    SHARED.with(|c| *c.borrow_mut() = Some(sh.clone()));
    if !register_class() {
        return 1;
    }
    if paths.is_empty() {
        create_window(&sh, None);
    } else {
        for p in paths {
            create_window(&sh, Some(p));
        }
    }
    let _ = PENDING_OPEN.with(|p| p.borrow().len());
    let mut msg = MSG::default();
    loop {
        // SAFETY: standard message loop; msg is a valid out-pointer.
        let r = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if r.0 <= 0 {
            break;
        }
        // SAFETY: msg was filled by GetMessageW.
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    i32::try_from(msg.wParam.0).unwrap_or(0)
}

/// Selection range helper for tests and tooling.
pub fn normalize(sel: (u32, u32)) -> Range<u32> {
    sel.0.min(sel.1)..sel.0.max(sel.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpi_scaling() {
        assert_eq!(scale(1100, 96), 1100);
        assert_eq!(scale(1100, 144), 1650);
    }

    #[test]
    fn b61_column() {
        assert_eq!(column(1000.0, 0.65), (175.0, 650.0));
        assert_eq!(column(400.0, 0.65), (24.0, 352.0));
        assert_eq!(column(2000.0, 1.0).1, 1952.0);
    }

    #[test]
    fn easing() {
        assert_eq!(ease_out(0.0), 0.0);
        assert_eq!(ease_out(1.0), 1.0);
        assert!(ease_out(0.5) > 0.5);
    }
}
