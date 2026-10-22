//! Background export jobs (BUG-11, R-EXP-07, B.10.7).
//!
//! PDF, print and HTML exports run in the worker process (`yamdv --print-job <file> <owner>
//! <nonce> <cancel>`, ADR-0018) while the main window keeps pumping messages. The worker reports
//! [`yamdv_core::progress::Record`]s by `WM_COPYDATA` (S-IPC-05 checks: magic, sender process id,
//! nonce) and polls an inheritable manual-reset event between pages and bands for Cancel. The
//! event is the only handle the worker inherits (`PROC_THREAD_ATTRIBUTE_HANDLE_LIST`).
#![allow(unsafe_code)]
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]

use std::path::PathBuf;
use std::time::Instant;

use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, WAIT_OBJECT_0};
use windows::Win32::Security::SECURITY_ATTRIBUTES;
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Threading::{
    CreateEventW, CreateProcessW, DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT,
    GetExitCodeProcess, InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
    PROC_THREAD_ATTRIBUTE_HANDLE_LIST, PROCESS_INFORMATION, STARTUPINFOEXW, STARTUPINFOW, SetEvent,
    UpdateProcThreadAttribute, WaitForSingleObject,
};
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
use windows::core::{HSTRING, PCWSTR, PWSTR};
use yamdv_core::progress::{self, JobKind, Phase, Record};

/// Worker exit codes (`--print-job`).
pub const EXIT_OK: u32 = 0;
/// No such printer.
pub const EXIT_NO_PRINTER: u32 = 3;
/// Cancelled by the user (partial output removed).
pub const EXIT_CANCELLED: u32 = 4;

/// Whether the (event) handle is signalled, without waiting.
pub fn is_set(h: HANDLE) -> bool {
    // SAFETY: valid event handle (or a stale one: then WAIT_FAILED, read as "not set").
    unsafe { WaitForSingleObject(h, 0) == WAIT_OBJECT_0 }
}

/// Worker side: progress reports to the owner window and the Cancel event.
#[derive(Debug)]
pub struct Link {
    owner: HWND,
    nonce: u64,
    cancel: HANDLE,
    throttle: progress::Throttle,
    t0: Instant,
}

impl Link {
    /// From the worker's command-line arguments (owner window, nonce, inherited event handle).
    pub fn new(owner: isize, nonce: u64, cancel: isize) -> Self {
        Self {
            owner: HWND(owner as *mut core::ffi::c_void),
            nonce,
            cancel: HANDLE(cancel as *mut core::ffi::c_void),
            throttle: progress::Throttle::default(),
            t0: Instant::now(),
        }
    }

    /// The Cancel event (checked inside band loops).
    pub fn cancel_handle(&self) -> HANDLE {
        self.cancel
    }

    /// Reports `n` of `total` in `phase` (throttled to 10/s); `false` once Cancel was pressed.
    pub fn report(&mut self, phase: Phase, n: u32, total: u32) -> bool {
        let ms = u64::try_from(self.t0.elapsed().as_millis()).unwrap_or(u64::MAX);
        if self.throttle.due(ms, phase, n, total) {
            let r = Record {
                nonce: self.nonce,
                phase,
                n,
                total,
            };
            crate::dialog_host::send_copydata_as(
                self.owner,
                progress::MAGIC as usize,
                &progress::encode(&r),
                250,
            );
        }
        !is_set(self.cancel)
    }
}

/// Main side: a running worker (one per process; export commands are disabled meanwhile).
#[derive(Debug)]
pub struct Worker {
    /// Window that started the job (progress card, taskbar button).
    pub owner: HWND,
    /// Worker process id (checked on every record).
    pub pid: u32,
    /// Worker process handle (waited on by the message loop).
    pub process: HANDLE,
    nonce: u64,
    cancel: HANDLE,
    /// What is being exported.
    pub kind: JobKind,
    /// Output file (PDF / HTML); removed when the job fails or is cancelled.
    pub out: Option<String>,
    job_file: PathBuf,
    /// Start time (card delay, ETA).
    pub started: Instant,
    /// Latest record.
    pub last: Option<Record>,
    /// Cancel was requested.
    pub cancelling: bool,
    /// Close the owner window when the worker has ended ("Cancel export and close").
    pub close_after: bool,
}

impl Drop for Worker {
    fn drop(&mut self) {
        // SAFETY: handles returned by CreateProcessW / CreateEventW, closed once.
        unsafe {
            let _ = CloseHandle(self.process);
            let _ = CloseHandle(self.cancel);
        }
        let _ = std::fs::remove_file(&self.job_file);
    }
}

impl Worker {
    /// Signals Cancel; the worker stops at the next page or band and removes partial output.
    pub fn cancel(&mut self) {
        self.cancelling = true;
        // SAFETY: our event handle.
        let _ = unsafe { SetEvent(self.cancel) };
    }

    /// Exit code of the finished worker (`None` if unavailable).
    pub fn exit_code(&self) -> Option<u32> {
        let mut c = 0u32;
        // SAFETY: valid process handle and out-pointer.
        unsafe { GetExitCodeProcess(self.process, &mut c) }.ok()?;
        Some(c)
    }

    /// S-IPC-05 checks on a `WM_COPYDATA` progress record: receiver, magic, sender process id,
    /// size and nonce.
    pub fn accept(&self, receiver: HWND, sender: HWND, cds: &COPYDATASTRUCT) -> Option<Record> {
        if receiver != self.owner || cds.dwData != progress::MAGIC as usize {
            return None;
        }
        if cds.cbData as usize != progress::LEN || cds.lpData.is_null() {
            return None;
        }
        let mut pid = 0u32;
        // SAFETY: GetWindowThreadProcessId tolerates stale handles (returns 0).
        unsafe { GetWindowThreadProcessId(sender, Some(&mut pid)) };
        if pid != self.pid {
            return None;
        }
        // SAFETY: lpData points to cbData bytes for the duration of WM_COPYDATA.
        let bytes = unsafe { std::slice::from_raw_parts(cds.lpData as *const u8, progress::LEN) };
        let r = progress::decode(bytes)?;
        (r.nonce == self.nonce).then_some(r)
    }

    /// Partial output to remove after failure or Cancel (the file and its atomic-write temp).
    pub fn partial_files(&self) -> Vec<String> {
        self.out
            .iter()
            .flat_map(|o| [o.clone(), format!("{o}.tmp")])
            .collect()
    }
}

/// Starts the worker for the serialized job `text` (owned by `owner`); `None` on failure.
pub fn spawn(owner: HWND, kind: JobKind, text: &str, out: Option<String>) -> Option<Worker> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let mut buf = vec![0u16; 32_768];
    // SAFETY: valid buffer; NULL module = our own executable.
    let n = unsafe { GetModuleFileNameW(None, &mut buf) } as usize;
    if n == 0 || n >= buf.len() {
        return None;
    }
    let exe = String::from_utf16_lossy(buf.get(..n).unwrap_or(&[]));
    let job_file = std::env::temp_dir().join(format!(
        "yamdv-print-{}-{}.job",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&job_file, text).ok()?;
    let sa = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: core::ptr::null_mut(),
        bInheritHandle: true.into(),
    };
    // SAFETY: anonymous manual-reset event, inheritable (passed to the worker only).
    let Ok(cancel) = (unsafe { CreateEventW(Some(&raw const sa), true, false, PCWSTR::null()) })
    else {
        let _ = std::fs::remove_file(&job_file);
        return None;
    };
    let nonce = crate::dialog_host::nonce();
    let mut cmd: Vec<u16> = format!(
        "\"{exe}\" --print-job \"{}\" {} {nonce} {}",
        job_file.display(),
        owner.0 as isize,
        cancel.0 as isize
    )
    .encode_utf16()
    .chain(Some(0))
    .collect();
    let started = start_process(&exe, &mut cmd, cancel);
    let Some(pi) = started else {
        // SAFETY: our event handle.
        let _ = unsafe { CloseHandle(cancel) };
        let _ = std::fs::remove_file(&job_file);
        return None;
    };
    // SAFETY: thread handle not needed.
    let _ = unsafe { CloseHandle(pi.hThread) };
    Some(Worker {
        owner,
        pid: pi.dwProcessId,
        process: pi.hProcess,
        nonce,
        cancel,
        kind,
        out,
        job_file,
        started: Instant::now(),
        last: None,
        cancelling: false,
        close_after: false,
    })
}

/// `CreateProcessW` of our own exe inheriting exactly `inherit` (S-PROC-07).
fn start_process(exe: &str, cmd: &mut [u16], inherit: HANDLE) -> Option<PROCESS_INFORMATION> {
    let mut size = 0usize;
    // SAFETY: size query (fails with ERROR_INSUFFICIENT_BUFFER by design).
    let _ = unsafe { InitializeProcThreadAttributeList(None, 1, None, &mut size) };
    let mut attr = vec![0u8; size.max(64)];
    let list = LPPROC_THREAD_ATTRIBUTE_LIST(attr.as_mut_ptr().cast());
    // SAFETY: buffer of the queried size.
    unsafe { InitializeProcThreadAttributeList(Some(list), 1, None, &mut size) }.ok()?;
    let handles = [inherit];
    // SAFETY: `handles` outlives CreateProcessW below; the list was initialized above.
    let ok = unsafe {
        UpdateProcThreadAttribute(
            list,
            0,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
            Some(handles.as_ptr().cast()),
            size_of::<HANDLE>(),
            None,
            None,
        )
    }
    .is_ok();
    let app = HSTRING::from(exe);
    let si = STARTUPINFOEXW {
        StartupInfo: STARTUPINFOW {
            cb: size_of::<STARTUPINFOEXW>() as u32,
            ..Default::default()
        },
        lpAttributeList: list,
    };
    let mut pi = PROCESS_INFORMATION::default();
    // SAFETY: application name is the absolute path of our own exe (S-PROC-07); the mutable
    // command line outlives the call; only the handle list above is inherited.
    let r = ok
        && unsafe {
            CreateProcessW(
                &app,
                Some(PWSTR(cmd.as_mut_ptr())),
                None,
                None,
                true,
                EXTENDED_STARTUPINFO_PRESENT,
                None,
                PCWSTR::null(),
                (&raw const si).cast(),
                &mut pi,
            )
        }
        .is_ok();
    // SAFETY: initialized above; deleted once.
    unsafe { DeleteProcThreadAttributeList(list) };
    drop(attr);
    r.then_some(pi)
}
