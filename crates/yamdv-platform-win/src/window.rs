//! The Phase 001 window skeleton: one top-level window of class `YAMDV.Window`, per-monitor DPI,
//! background = theme token `bg` following the Windows app mode live, Esc / Ctrl+W close,
//! no flicker, `GetMessageW` loop without timers or polling (idle CPU 0).
//! PHASE-004: custom title bar replaces the temporary system caption.
#![allow(unsafe_code)]

use crate::theme::{self, Mode};
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, DC_BRUSH, EndPaint, FillRect, GetMonitorInfoW, GetStockObject, HBRUSH,
    InvalidateRect, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, PAINTSTRUCT,
    SetDCBrushColor,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    AdjustWindowRectExForDpi, GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_ESCAPE};
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetCursorPos, GetMessageW, HICON, IDC_ARROW, IMAGE_ICON, LR_DEFAULTSIZE, LR_SHARED,
    LoadCursorW, LoadImageW, MINMAXINFO, MSG, PostQuitMessage, RegisterClassExW, SW_SHOW,
    SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos, ShowWindow, TranslateMessage, WINDOW_EX_STYLE,
    WM_DESTROY, WM_DPICHANGED, WM_ERASEBKGND, WM_GETMINMAXINFO, WM_KEYDOWN, WM_PAINT,
    WM_SETTINGCHANGE, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
};
use windows::core::{PCWSTR, w};

/// Window class name (prompt §4.6).
pub const CLASS_NAME: PCWSTR = w!("YAMDV.Window");
/// Initial client size in logical px.
pub const INITIAL_CLIENT: (i32, i32) = (1100, 760);
/// Minimum client size in logical px.
pub const MIN_CLIENT: (i32, i32) = (480, 360);

static DARK: AtomicBool = AtomicBool::new(false);

fn mode() -> Mode {
    if DARK.load(Ordering::Relaxed) {
        Mode::Dark
    } else {
        Mode::Light
    }
}

/// Scale a logical length to device pixels for `dpi`.
pub fn scale(v: i32, dpi: u32) -> i32 {
    let dpi = i32::try_from(dpi).unwrap_or(96);
    (v * dpi + 48) / 96
}

fn apply_mode(hwnd: HWND, m: Mode) {
    DARK.store(m == Mode::Dark, Ordering::Relaxed);
    let dark = windows::core::BOOL::from(m == Mode::Dark);
    // SAFETY: valid hwnd; pointer/size describe a BOOL on the stack.
    let r = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&raw const dark).cast(),
            u32::try_from(size_of::<windows::core::BOOL>()).unwrap_or(4),
        )
    };
    if let Err(e) = r {
        crate::log::warn("window", &format!("DWMWA_USE_IMMERSIVE_DARK_MODE: {e}"));
    }
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

/// `true` if `lparam` of `WM_SETTINGCHANGE` names the `ImmersiveColorSet` area.
fn is_immersive_color_set(lparam: LPARAM) -> bool {
    if lparam.0 == 0 {
        return false;
    }
    // SAFETY: for WM_SETTINGCHANGE, a non-zero lParam points to a NUL-terminated wide string.
    let s = unsafe { PCWSTR(lparam.0 as *const u16).to_string() }.unwrap_or_default();
    s == "ImmersiveColorSet"
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_ERASEBKGND => LRESULT(1), // everything is painted in WM_PAINT (no flicker)
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            // SAFETY: standard BeginPaint/EndPaint pair on our own window; DC_BRUSH is a stock
            // object that needs no cleanup.
            unsafe {
                let hdc = BeginPaint(hwnd, &mut ps);
                SetDCBrushColor(
                    hdc,
                    windows::Win32::Foundation::COLORREF(mode().bg_colorref()),
                );
                FillRect(hdc, &ps.rcPaint, HBRUSH(GetStockObject(DC_BRUSH).0));
                let _ = EndPaint(hwnd, &ps);
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            let vk = u16::try_from(wparam.0).unwrap_or(0);
            // SAFETY: GetKeyState has no preconditions.
            let ctrl = unsafe { GetKeyState(i32::from(VK_CONTROL.0)) } < 0;
            if vk == VK_ESCAPE.0 || (ctrl && vk == u16::from(b'W')) {
                // SAFETY: destroying our own window from its window procedure.
                let _ = unsafe { DestroyWindow(hwnd) };
                return LRESULT(0);
            }
            // SAFETY: default processing.
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_SETTINGCHANGE => {
            if is_immersive_color_set(lparam) {
                let m = theme::app_mode();
                if m != mode() {
                    apply_mode(hwnd, m);
                    // SAFETY: valid hwnd; invalidates the whole client area.
                    let _ = unsafe { InvalidateRect(Some(hwnd), None, false) };
                }
            }
            // SAFETY: default processing.
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_DPICHANGED => {
            // SAFETY: for WM_DPICHANGED lParam points to the suggested window RECT.
            let rc = unsafe { *(lparam.0 as *const RECT) };
            // SAFETY: valid hwnd.
            let _ = unsafe {
                SetWindowPos(
                    hwnd,
                    None,
                    rc.left,
                    rc.top,
                    rc.right - rc.left,
                    rc.bottom - rc.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                )
            };
            LRESULT(0)
        }
        WM_GETMINMAXINFO => {
            // SAFETY: valid hwnd (may be 96 before creation completes).
            let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
            let (w, h) =
                window_rect_for_client(scale(MIN_CLIENT.0, dpi), scale(MIN_CLIENT.1, dpi), dpi);
            // SAFETY: for WM_GETMINMAXINFO lParam points to a writable MINMAXINFO.
            let mmi = unsafe { &mut *(lparam.0 as *mut MINMAXINFO) };
            mmi.ptMinTrackSize = POINT { x: w, y: h };
            LRESULT(0)
        }
        WM_DESTROY => {
            // SAFETY: no preconditions.
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        // SAFETY: default processing for every other message.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// Create the window and run the message loop until it closes. Returns the process exit code.
pub fn run(title: &str) -> i32 {
    // SAFETY: module handle of the running exe.
    let Ok(hinstance) = (unsafe { GetModuleHandleW(None) }) else {
        return 1;
    };
    // SAFETY: icon resource 1 is embedded by build.rs; LR_SHARED handles need no cleanup.
    let icon = unsafe {
        LoadImageW(
            Some(hinstance.into()),
            PCWSTR(std::ptr::without_provenance(1)), /* MAKEINTRESOURCE(1) */
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
    if unsafe { RegisterClassExW(&wc) } == 0 {
        return 1;
    }

    // Center on the monitor that contains the mouse cursor.
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
    let x = work.left + (work.right - work.left - w) / 2;
    let y = work.top + (work.bottom - work.top - h) / 2;

    let title_w: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
    // SAFETY: class registered above; title buffer outlives the call.
    let Ok(hwnd) = (unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            CLASS_NAME,
            PCWSTR(title_w.as_ptr()),
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
        return 1;
    };
    apply_mode(hwnd, theme::app_mode());
    // SAFETY: valid hwnd.
    let _ = unsafe { ShowWindow(hwnd, SW_SHOW) };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpi_scaling() {
        assert_eq!(scale(1100, 96), 1100);
        assert_eq!(scale(1100, 144), 1650);
        assert_eq!(scale(480, 120), 600);
    }

    #[test]
    fn theme_bg_tokens() {
        assert_eq!(Mode::Light.bg_colorref(), 0x00ff_ffff);
        assert_eq!(Mode::Dark.bg_colorref(), 0x001e_1e1e);
    }
}
