//! Out-of-process dialog host (F-09, S-PROC-07, S-IPC-05, ADR-0013).
//!
//! The main process never loads the shell's file-dialog components. `Ctrl+O` starts a second
//! instance of our own executable by absolute path with
//! `--dialog-host open <owner-hwnd> <nonce>`. The host shows one `IFileOpenDialog` owned by the
//! main window, sends the result back with `WM_COPYDATA` (magic, versioned payload, nonce;
//! see `yamdv_core::ipc`) from a message-only window, and exits.
#![allow(unsafe_code)]
// Win32 sizes and handles: the casts are bounded (struct sizes, buffers ≤ 4 MiB, class names).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use std::time::Instant;

use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Security::Cryptography::{BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom};
use windows::Win32::Security::{EqualSid, GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::LibraryLoader::{GetModuleFileNameW, GetModuleHandleW};
use windows::Win32::System::Threading::{
    CreateProcessW, GetCurrentProcess, GetExitCodeProcess, OpenProcess, OpenProcessToken,
    PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, STARTUPINFOW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClassNameW, GetWindowThreadProcessId,
    HWND_MESSAGE, IsWindow, RegisterClassW, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_COPYDATA, WNDCLASSW,
};
use windows::core::{HSTRING, PCWSTR, PWSTR, w};
use yamdv_core::i18n::{Msg, tr};
use yamdv_core::ipc::{self, Kind, Outcome, Reply};

/// Exit codes of the host.
pub const EXIT_OK: i32 = 0;
/// The dialog could not be shown.
pub const EXIT_DIALOG_FAILED: i32 = 1;
/// Refused: bad owner (not our window class or not the same user).
pub const EXIT_REFUSED: i32 = 2;

fn hwnd_from(v: isize) -> HWND {
    HWND(std::ptr::without_provenance_mut(v as usize))
}

fn token_user(process: HANDLE) -> Option<Vec<u8>> {
    let mut tok = HANDLE::default();
    // SAFETY: valid process handle; out-pointer on the stack.
    unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut tok) }.ok()?;
    let mut buf = vec![0u8; 256];
    let mut len = 0u32;
    // SAFETY: buffer and its length are valid; the token handle is closed below.
    let ok = unsafe {
        GetTokenInformation(
            tok,
            TokenUser,
            Some(buf.as_mut_ptr().cast()),
            buf.len() as u32,
            &mut len,
        )
    }
    .is_ok();
    // SAFETY: handle opened above.
    let _ = unsafe { CloseHandle(tok) };
    ok.then_some(buf)
}

/// S-IPC-05: the owner window must be a YAMDV main window of a process of the same user.
fn owner_ok(owner: HWND) -> bool {
    // SAFETY: IsWindow only tests the handle.
    if !unsafe { IsWindow(Some(owner)) }.as_bool() {
        return false;
    }
    let mut cls = [0u16; 64];
    // SAFETY: valid buffer.
    let n = unsafe { GetClassNameW(owner, &mut cls) };
    if String::from_utf16_lossy(cls.get(..n.max(0) as usize).unwrap_or(&[])) != "YAMDV.Window" {
        return false;
    }
    let mut pid = 0u32;
    // SAFETY: valid hwnd; out-pointer on the stack.
    unsafe { GetWindowThreadProcessId(owner, Some(&mut pid)) };
    // SAFETY: plain open for query; closed below.
    let Ok(h) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }) else {
        return false;
    };
    let theirs = token_user(h);
    // SAFETY: opened above.
    let _ = unsafe { CloseHandle(h) };
    // SAFETY: pseudo handle of the current process.
    let mine = token_user(unsafe { GetCurrentProcess() });
    match (theirs, mine) {
        (Some(a), Some(b)) => {
            // SAFETY: both buffers hold TOKEN_USER structures filled by GetTokenInformation.
            unsafe {
                let a = &*a.as_ptr().cast::<TOKEN_USER>();
                let b = &*b.as_ptr().cast::<TOKEN_USER>();
                EqualSid(a.User.Sid, b.User.Sid).is_ok()
            }
        }
        _ => false,
    }
}

extern "system" fn host_wndproc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    // SAFETY: default handling for the message-only window.
    unsafe { DefWindowProcW(h, m, wp, lp) }
}

fn send_reply(owner: HWND, reply: &Reply) {
    let data = ipc::encode(reply);
    // SAFETY: registering and creating a message-only window of our own class.
    let me = unsafe {
        let inst = GetModuleHandleW(None).unwrap_or_default();
        let wc = WNDCLASSW {
            lpfnWndProc: Some(host_wndproc),
            hInstance: inst.into(),
            lpszClassName: w!("YAMDV.DialogHost"),
            ..Default::default()
        };
        RegisterClassW(&wc);
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("YAMDV.DialogHost"),
            PCWSTR::null(),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(inst.into()),
            None,
        )
    };
    let Ok(me) = me else { return };
    let cds = COPYDATASTRUCT {
        dwData: ipc::MAGIC as usize,
        cbData: data.len() as u32,
        lpData: data.as_ptr() as *mut _,
    };
    // SAFETY: cds and data outlive the synchronous call; the receiver copies the data.
    unsafe {
        let _ = SendMessageTimeoutW(
            owner,
            WM_COPYDATA,
            WPARAM(me.0 as usize),
            LPARAM(&raw const cds as isize),
            SMTO_ABORTIFHUNG,
            10_000,
            None,
        );
        let _ = DestroyWindow(me);
    }
}

/// Host entry point (`--dialog-host open <owner> <nonce>`); hardening was applied by `main`.
pub fn run_host(owner: isize, nonce: u64) -> i32 {
    let owner = hwnd_from(owner);
    if !owner_ok(owner) {
        return EXIT_REFUSED;
    }
    // SAFETY: COM for IFileOpenDialog on this thread.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let s = crate::window::load_settings();
    let lang = yamdv_core::i18n::resolve(s.str("general.language"), crate::sys::ui_language_tag());
    let f = crate::sys::Filters {
        markdown: tr(lang, Msg::FilterMarkdown),
        text: tr(lang, Msg::FilterText),
        all: tr(lang, Msg::FilterAll),
    };
    let (outcome, code) = match crate::sys::open_dialog(owner, &f) {
        Some(p) if p.is_empty() => (Outcome::Cancel, EXIT_OK),
        Some(p) => (Outcome::Paths(p), EXIT_OK),
        None => (Outcome::Error, EXIT_DIALOG_FAILED),
    };
    send_reply(
        owner,
        &Reply {
            kind: Kind::Open,
            nonce,
            outcome,
        },
    );
    code
}

/// A running host the main process waits for.
#[derive(Debug)]
pub struct HostWait {
    /// Owner (disabled) main window.
    pub owner: HWND,
    /// Host process id.
    pub pid: u32,
    /// Host process handle (waited on by the message loop).
    pub process: HANDLE,
    /// Per-launch nonce.
    pub nonce: u64,
    /// Launch time (T-DLG logging).
    pub started: Instant,
    /// A valid result arrived.
    pub answered: bool,
}

impl Drop for HostWait {
    fn drop(&mut self) {
        // SAFETY: handle returned by CreateProcessW, closed once.
        let _ = unsafe { CloseHandle(self.process) };
    }
}

fn nonce() -> u64 {
    let mut b = [0u8; 8];
    // SAFETY: valid buffer; system-preferred RNG needs no algorithm handle.
    let _ = unsafe { BCryptGenRandom(None, &mut b, BCRYPT_USE_SYSTEM_PREFERRED_RNG) };
    u64::from_le_bytes(b)
}

/// Starts the host for an open dialog owned by `owner`.
pub fn launch(owner: HWND) -> Option<HostWait> {
    let mut buf = vec![0u16; 32_768];
    // SAFETY: valid buffer; NULL module = our own executable.
    let n = unsafe { GetModuleFileNameW(None, &mut buf) } as usize;
    if n == 0 || n >= buf.len() {
        return None;
    }
    let exe = String::from_utf16_lossy(buf.get(..n).unwrap_or(&[]));
    let n = nonce();
    let mut cmd: Vec<u16> = format!("\"{exe}\" --dialog-host open {} {n}", owner.0 as isize)
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let app = HSTRING::from(exe.as_str());
    let si = STARTUPINFOW {
        cb: size_of::<STARTUPINFOW>() as u32,
        ..Default::default()
    };
    let mut pi = PROCESS_INFORMATION::default();
    // SAFETY: application name is the absolute path of our own exe (S-PROC-07); the mutable
    // command line buffer outlives the call; no handle inheritance.
    unsafe {
        CreateProcessW(
            &app,
            Some(PWSTR(cmd.as_mut_ptr())),
            None,
            None,
            false,
            PROCESS_CREATION_FLAGS(0),
            None,
            PCWSTR::null(),
            &si,
            &mut pi,
        )
    }
    .ok()?;
    // SAFETY: thread handle not needed.
    let _ = unsafe { CloseHandle(pi.hThread) };
    Some(HostWait {
        owner,
        pid: pi.dwProcessId,
        process: pi.hProcess,
        nonce: n,
        started: Instant::now(),
        answered: false,
    })
}

/// Exit code of a finished host (`None` if unavailable).
pub fn exit_code(h: &HostWait) -> Option<u32> {
    let mut c = 0u32;
    // SAFETY: valid process handle and out-pointer.
    unsafe { GetExitCodeProcess(h.process, &mut c) }.ok()?;
    Some(c)
}

/// S-IPC-05 checks on a received `WM_COPYDATA`: magic, sender process id, decode, nonce.
/// Returns the outcome when the message is a valid answer from `h`.
pub fn accept(h: &HostWait, receiver: HWND, sender: HWND, cds: &COPYDATASTRUCT) -> Option<Outcome> {
    if receiver != h.owner || h.answered || cds.dwData != ipc::MAGIC as usize {
        return None;
    }
    let len = cds.cbData as usize;
    if len > ipc::MAX_BYTES || cds.lpData.is_null() {
        return None;
    }
    let mut pid = 0u32;
    // SAFETY: GetWindowThreadProcessId tolerates stale handles (returns 0).
    unsafe { GetWindowThreadProcessId(sender, Some(&mut pid)) };
    if pid != h.pid {
        return None;
    }
    // SAFETY: the system copied `cbData` bytes at `lpData` into our address space for the
    // duration of this message.
    let bytes = unsafe { std::slice::from_raw_parts(cds.lpData.cast::<u8>(), len) };
    let r = ipc::decode(bytes).ok()?;
    (r.nonce == h.nonce && r.kind == Kind::Open).then_some(r.outcome)
}
