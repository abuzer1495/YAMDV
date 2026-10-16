//! Re-entrancy guard (P-18, F-11). Window state lives in a `RefCell`; Win32 calls that dispatch
//! messages synchronously can re-enter the window procedure while a handler holds the borrow.
//! [`dispatch`] detects that case: it never panics, counts the event and (debug builds) logs the
//! message id. Callers skip re-entrant `WM_PAINT` (the region stays invalid) and fall back to
//! `DefWindowProcW` for everything else; state mutations that need such calls are deferred.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering};

static REENTRIES: AtomicU32 = AtomicU32::new(0);
static LAST_MSG: AtomicU32 = AtomicU32::new(0);

/// Number of re-entrant dispatches detected so far (process-wide).
pub fn count() -> u32 {
    REENTRIES.load(Ordering::Relaxed)
}

/// Message id of the last re-entrant dispatch.
pub fn last_msg() -> u32 {
    LAST_MSG.load(Ordering::Relaxed)
}

/// Runs `f` with the mutable borrow, or returns `None` (and records `msg`) when the state is
/// already borrowed by an outer handler on this thread.
pub fn dispatch<T, R>(cell: &RefCell<T>, msg: u32, f: impl FnOnce(&mut T) -> R) -> Option<R> {
    if let Ok(mut s) = cell.try_borrow_mut() {
        Some(f(&mut s))
    } else {
        REENTRIES.fetch_add(1, Ordering::Relaxed);
        LAST_MSG.store(msg, Ordering::Relaxed);
        if cfg!(debug_assertions) {
            crate::log::warn(
                "reentry",
                &format!("re-entrant dispatch of message 0x{msg:04X}"),
            );
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_dispatch_is_detected() {
        let c = RefCell::new(0u32);
        let before = count();
        let r = dispatch(&c, 0x0400, |v| {
            *v += 1;
            dispatch(&c, 0x000F, |v| *v += 1)
        });
        assert_eq!(r, Some(None));
        assert_eq!(*c.borrow(), 1);
        assert!(count() > before);
        assert_eq!(last_msg(), 0x000F);
    }

    /// A real synchronous message: inside the handler of WM_USER the window sends itself
    /// WM_USER+1 with `SendMessageW`, which re-enters the window procedure synchronously.
    #[test]
    #[allow(unsafe_code)]
    fn synchronous_send_message_is_detected() {
        use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
        use windows::Win32::System::LibraryLoader::GetModuleHandleW;
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, HWND_MESSAGE, RegisterClassW,
            SendMessageW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_USER, WNDCLASSW,
        };
        use windows::core::w;
        thread_local!(static STATE: RefCell<u32> = const { RefCell::new(0) });
        unsafe extern "system" fn proc_(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
            if m == WM_USER || m == WM_USER + 1 {
                let handled = STATE.with(|s| {
                    dispatch(s, m, |v| {
                        *v += 1;
                        if m == WM_USER {
                            // SAFETY: our own valid window; re-enters `proc_` synchronously.
                            unsafe { SendMessageW(h, WM_USER + 1, None, None) };
                        }
                    })
                });
                if handled.is_some() {
                    return LRESULT(0);
                }
            }
            // SAFETY: default processing.
            unsafe { DefWindowProcW(h, m, wp, lp) }
        }
        // SAFETY: standard class registration and message-only window; destroyed below.
        unsafe {
            let hi = GetModuleHandleW(None).unwrap_or_default();
            let wc = WNDCLASSW {
                lpfnWndProc: Some(proc_),
                hInstance: hi.into(),
                lpszClassName: w!("YamdvReentryTest"),
                ..Default::default()
            };
            RegisterClassW(&wc);
            let h = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("YamdvReentryTest"),
                w!(""),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(hi.into()),
                None,
            )
            .unwrap_or_default();
            let before = count();
            SendMessageW(h, WM_USER, None, None);
            assert_eq!(count(), before + 1);
            assert_eq!(last_msg(), WM_USER + 1);
            STATE.with(|s| assert_eq!(*s.borrow(), 1));
            let _ = DestroyWindow(h);
        }
    }
}
