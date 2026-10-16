//! Spike harness: window, CLI, scrolling, frame timing, self-measurement and the log.

use std::fmt::Write as _;
use std::path::PathBuf;

use windows::Win32::Foundation::{FILETIME, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, InvalidateRect, PAINTSTRUCT, UpdateWindow,
};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
use windows::Win32::System::ProcessStatus::{
    GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX2,
};
use windows::Win32::System::SystemInformation::GetSystemTimePreciseAsFileTime;
use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};
use windows::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW,
    DispatchMessageW, GWLP_USERDATA, GetClientRect, GetMessageW, GetWindowLongPtrW, IDC_ARROW,
    LoadCursorW, MINMAXINFO, MSG, PostMessageW, PostQuitMessage, RegisterClassExW, SW_INVALIDATE,
    SW_SHOW, SWP_NOMOVE, SWP_NOZORDER, ScrollWindowEx, SetWindowLongPtrW, SetWindowPos,
    SetWindowTextW, ShowWindow, TranslateMessage, WINDOW_EX_STYLE, WM_APP, WM_CLOSE, WM_DESTROY,
    WM_ERASEBKGND, WM_GETMINMAXINFO, WM_KEYDOWN, WM_MOUSEWHEEL, WM_NCCREATE, WM_PAINT, WM_SIZE,
    WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
};
use windows::core::{HSTRING, PCWSTR, w};

use crate::scene::Scene;

/// How a backend reacts to scrolling.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScrollMode {
    /// `ScrollWindowEx` + repaint of the exposed strip only (GDI-presented backends).
    Blit,
    /// Full redraw + present (swap-chain backends).
    Redraw,
}

/// One rendering backend.
pub trait Backend: Sized {
    /// Short name used in the title and the log.
    const NAME: &'static str;
    /// Scroll strategy.
    const MODE: ScrollMode;
    /// Extra window ex-style (swap-chain backends need no redirection bitmap).
    const EX_STYLE: WINDOW_EX_STYLE;
    /// Creates device resources for `hwnd`; may mutate the scene's text layouts (drawing effects).
    fn create(
        hwnd: HWND,
        scene: &Scene,
        w: u32,
        h: u32,
        log: &mut Log,
    ) -> windows::core::Result<Self>;
    /// Client size changed.
    fn resize(&mut self, w: u32, h: u32);
    /// Paints `rc` (device px, client coordinates) of the view whose top is `scroll` device px.
    fn paint(
        &mut self,
        scene: &Scene,
        scroll: i32,
        rc: RECT,
        hdc: windows::Win32::Graphics::Gdi::HDC,
    );
    /// Redraws the whole client area and presents (Redraw mode only).
    fn present_full(&mut self, _scene: &Scene, _scroll: i32) {}
}

/// Parsed command line.
pub struct Args {
    pub autoscroll: bool,
    pub log: Option<PathBuf>,
    pub scale: Option<f32>,
    pub width: i32,
    pub height: i32,
    pub repeat: u32,
    pub step: i32,
}

fn parse_args() -> Args {
    let mut a = Args {
        autoscroll: false,
        log: None,
        scale: None,
        width: 1100,
        height: 760,
        repeat: 200,
        step: 40,
    };
    let mut it = std::env::args().skip(1);
    while let Some(s) = it.next() {
        match s.as_str() {
            "--autoscroll" => a.autoscroll = true,
            "--log" => a.log = it.next().map(PathBuf::from),
            "--scale" => a.scale = it.next().and_then(|v| v.parse().ok()),
            "--repeat" => a.repeat = it.next().and_then(|v| v.parse().ok()).unwrap_or(200),
            "--step" => a.step = it.next().and_then(|v| v.parse().ok()).unwrap_or(40),
            "--size" => {
                if let Some(v) = it.next()
                    && let Some((w, h)) = v.split_once('x')
                {
                    a.width = w.parse().unwrap_or(1100);
                    a.height = h.parse().unwrap_or(760);
                }
            }
            _ => {}
        }
    }
    a
}

/// Append-only `key=value` log, written on exit when `--log` is given.
#[derive(Default)]
pub struct Log {
    pub text: String,
}

impl Log {
    pub fn kv(&mut self, k: &str, v: impl std::fmt::Display) {
        let _ = writeln!(self.text, "{k}={v}");
    }
    pub fn pws(&mut self, label: &str) {
        let v = private_ws();
        self.kv(&format!("pws.{label}"), v);
    }
}

/// Private working set of this process (bytes), `PROCESS_MEMORY_COUNTERS_EX2`.
pub fn private_ws() -> u64 {
    let mut c = PROCESS_MEMORY_COUNTERS_EX2::default();
    unsafe {
        let _ = GetProcessMemoryInfo(
            GetCurrentProcess(),
            (&raw mut c).cast::<PROCESS_MEMORY_COUNTERS>(),
            size_of::<PROCESS_MEMORY_COUNTERS_EX2>() as u32,
        );
    }
    c.PrivateWorkingSetSize as u64
}

fn qpc() -> i64 {
    let mut v = 0;
    unsafe {
        let _ = QueryPerformanceCounter(&mut v);
    }
    v
}

fn qpf() -> f64 {
    let mut v = 0;
    unsafe {
        let _ = QueryPerformanceFrequency(&mut v);
    }
    v as f64
}

fn ft(f: FILETIME) -> u64 {
    (u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime)
}

fn ms_since_process_start() -> f64 {
    let (mut c, mut e, mut k, mut u) = Default::default();
    unsafe {
        let _ = GetProcessTimes(GetCurrentProcess(), &mut c, &mut e, &mut k, &mut u);
        let now = GetSystemTimePreciseAsFileTime();
        (ft(now).saturating_sub(ft(c))) as f64 / 10_000.0
    }
}

struct State<B: Backend> {
    args: Args,
    scene: Option<Scene>,
    backend: Option<B>,
    log: Log,
    scroll: i32,
    client_h: i32,
    first_paint_done: bool,
}

impl<B: Backend> State<B> {
    fn max_scroll(&self) -> i32 {
        (self.scene.as_ref().map_or(0, Scene::total_px) - self.client_h).max(0)
    }

    fn scroll_by(&mut self, hwnd: HWND, dy: i32) -> i32 {
        let new = (self.scroll + dy).clamp(0, self.max_scroll());
        let d = new - self.scroll;
        if d == 0 {
            return 0;
        }
        self.scroll = new;
        match B::MODE {
            ScrollMode::Blit => unsafe {
                let _ = ScrollWindowEx(hwnd, 0, -d, None, None, None, None, SW_INVALIDATE);
                let _ = UpdateWindow(hwnd);
            },
            ScrollMode::Redraw => {
                if let (Some(b), Some(sc)) = (self.backend.as_mut(), self.scene.as_ref()) {
                    b.present_full(sc, self.scroll);
                }
            }
        }
        d
    }

    fn autoscroll(&mut self, hwnd: HWND) {
        let freq = qpf();
        let scale = self.scene.as_ref().map_or(1.0, |s| s.scale);
        let step = (self.args.step as f32 * scale).round() as i32;
        let pws_before = private_ws();
        let mut times: Vec<f64> = Vec::new();
        let t_all = qpc();
        loop {
            let t0 = qpc();
            if self.scroll_by(hwnd, step) == 0 {
                break;
            }
            times.push((qpc() - t0) as f64 * 1000.0 / freq);
        }
        let total = (qpc() - t_all) as f64 * 1000.0 / freq;
        let mut sorted = times.clone();
        sorted.sort_by(f64::total_cmp);
        let pct = |p: f64| -> f64 {
            if sorted.is_empty() {
                return 0.0;
            }
            let i = ((sorted.len() as f64 - 1.0) * p).round() as usize;
            sorted.get(i).copied().unwrap_or(0.0)
        };
        let avg = if times.is_empty() {
            0.0
        } else {
            times.iter().sum::<f64>() / times.len() as f64
        };
        let l = &mut self.log;
        l.kv("scroll.frames", times.len());
        l.kv("scroll.step_px", step);
        l.kv("scroll.total_ms", format!("{total:.1}"));
        l.kv("scroll.avg_ms", format!("{avg:.3}"));
        l.kv("scroll.p50_ms", format!("{:.3}", pct(0.50)));
        l.kv("scroll.p99_ms", format!("{:.3}", pct(0.99)));
        l.kv("scroll.max_ms", format!("{:.3}", pct(1.0)));
        l.kv(
            "scroll.over_16ms",
            times.iter().filter(|t| **t > 16.0).count(),
        );
        l.kv("pws.before_scroll", pws_before);
        l.pws("after_scroll");
        // Return to the top so a screenshot after scrolling shows the reference area again.
        let top = -self.scroll;
        self.scroll_by(hwnd, top);
        unsafe {
            let _ = SetWindowTextW(
                hwnd,
                &HSTRING::from(format!("YAMDV spike {} SCROLL_DONE", B::NAME)),
            );
        }
    }
}

const WM_APP_AUTOSCROLL: u32 = WM_APP + 1;

unsafe extern "system" fn wndproc<B: Backend>(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> LRESULT {
    unsafe {
        if msg == WM_NCCREATE {
            let cs = &*(lp.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
            return DefWindowProcW(hwnd, msg, wp, lp);
        }
        let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State<B>;
        let Some(st) = p.as_mut() else {
            return DefWindowProcW(hwnd, msg, wp, lp);
        };
        match msg {
            WM_GETMINMAXINFO => {
                let mi = &mut *(lp.0 as *mut MINMAXINFO);
                mi.ptMaxTrackSize.x = 16_000;
                mi.ptMaxTrackSize.y = 16_000;
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            WM_SIZE => {
                let w = (lp.0 & 0xFFFF) as u32;
                let h = ((lp.0 >> 16) & 0xFFFF) as u32;
                st.client_h = h as i32;
                st.scroll = st.scroll.min(st.max_scroll());
                if let Some(b) = st.backend.as_mut() {
                    b.resize(w, h);
                }
                let _ = InvalidateRect(Some(hwnd), None, false);
                LRESULT(0)
            }
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                if let (Some(b), Some(sc)) = (st.backend.as_mut(), st.scene.as_ref()) {
                    b.paint(sc, st.scroll, ps.rcPaint, hdc);
                }
                let _ = EndPaint(hwnd, &ps);
                if !st.first_paint_done {
                    st.first_paint_done = true;
                    st.log
                        .kv("first_frame_ms", format!("{:.1}", ms_since_process_start()));
                    st.log.pws("first_frame");
                    if st.args.autoscroll {
                        let _ = PostMessageW(Some(hwnd), WM_APP_AUTOSCROLL, WPARAM(0), LPARAM(0));
                    }
                }
                LRESULT(0)
            }
            WM_APP_AUTOSCROLL => {
                st.autoscroll(hwnd);
                LRESULT(0)
            }
            WM_MOUSEWHEEL => {
                let delta = i32::from((wp.0 >> 16) as u16 as i16);
                let lines = (-delta * 3 * 20) / 120;
                st.scroll_by(
                    hwnd,
                    (lines as f32 * st.scene.as_ref().map_or(1.0, |s| s.scale)) as i32,
                );
                LRESULT(0)
            }
            WM_KEYDOWN if wp.0 == 0x1B => {
                let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

/// Runs the spike with backend `B` until the window closes.
pub fn run<B: Backend>() {
    let args = parse_args();
    let mut log = Log::default();
    log.kv("backend", B::NAME);
    log.pws("start");
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let hinst = GetModuleHandleW(None).unwrap_or_default();
        let class = w!("YAMDV.Spike");
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc::<B>),
            hInstance: hinst.into(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: class,
            ..Default::default()
        };
        RegisterClassExW(&wc);
        let st = Box::into_raw(Box::new(State::<B> {
            args,
            scene: None,
            backend: None,
            log,
            scroll: 0,
            client_h: 0,
            first_paint_done: false,
        }));
        let title = HSTRING::from(format!("YAMDV spike {}", B::NAME));
        let Ok(hwnd) = CreateWindowExW(
            B::EX_STYLE,
            class,
            PCWSTR(title.as_ptr()),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            800,
            600,
            None,
            None,
            Some(hinst.into()),
            Some(st.cast()),
        ) else {
            std::process::exit(3);
        };
        let s = &mut *st;
        s.log.pws("window_created");
        let dpi = GetDpiForWindow(hwnd);
        let scale = s.args.scale.unwrap_or(dpi as f32 / 96.0);
        s.log.kv("dpi", dpi);
        s.log.kv("scale", scale);
        let cw = (s.args.width as f32 * scale).round() as i32;
        let ch = (s.args.height as f32 * scale).round() as i32;
        let mut r = RECT {
            left: 0,
            top: 0,
            right: cw,
            bottom: ch,
        };
        let _ = AdjustWindowRectExForDpi(&mut r, WS_OVERLAPPEDWINDOW, false, B::EX_STYLE, dpi);
        let _ = SetWindowPos(
            hwnd,
            None,
            0,
            0,
            r.right - r.left,
            r.bottom - r.top,
            SWP_NOMOVE | SWP_NOZORDER,
        );
        let mut rc = RECT::default();
        let _ = GetClientRect(hwnd, &mut rc);
        s.log.kv("client", format!("{}x{}", rc.right, rc.bottom));
        s.client_h = rc.bottom;
        match Scene::new(scale, s.args.repeat, &mut s.log) {
            Ok(scene) => s.scene = Some(scene),
            Err(e) => {
                s.log.kv("error.scene", e);
                finish(s);
                std::process::exit(4);
            }
        }
        s.log.pws("scene_ready");
        let Some(scene) = s.scene.as_ref() else {
            return;
        };
        match B::create(hwnd, scene, rc.right as u32, rc.bottom as u32, &mut s.log) {
            Ok(b) => s.backend = Some(b),
            Err(e) => {
                s.log.kv("error.backend", e);
                finish(s);
                std::process::exit(5);
            }
        }
        s.log.pws("backend_ready");
        let _ = ShowWindow(hwnd, SW_SHOW);
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        s.log.pws("exit");
        finish(s);
    }
}

fn finish<B: Backend>(s: &mut State<B>) {
    if let Some(p) = &s.args.log {
        let _ = std::fs::write(p, &s.log.text);
    }
}
