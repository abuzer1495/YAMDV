//! CLI output for a GUI-subsystem exe (APPENDIX F §F.4): write to an inherited stdout/stderr
//! handle (pipe or file) if present, else to the parent console via
//! `AttachConsole(ATTACH_PARENT_PROCESS)`; if there is neither, write nothing.
#![allow(unsafe_code)]

use windows::Win32::Foundation::{GENERIC_WRITE, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_WRITE, FILE_TYPE_UNKNOWN, GetFileType,
    OPEN_EXISTING, WriteFile,
};
use windows::Win32::System::Console::{
    ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE, STD_HANDLE,
    STD_OUTPUT_HANDLE, WriteConsoleW,
};
use windows::core::w;

/// Output stream.
#[derive(Debug, Clone, Copy)]
pub enum Stream {
    /// Standard output.
    Out,
    /// Standard error.
    Err,
}

fn inherited(which: STD_HANDLE) -> Option<HANDLE> {
    // SAFETY: GetStdHandle has no preconditions.
    let h = unsafe { GetStdHandle(which) }.ok()?;
    if h.is_invalid() || h.0.is_null() {
        return None;
    }
    // SAFETY: h is a handle value returned by GetStdHandle.
    if unsafe { GetFileType(h) } == FILE_TYPE_UNKNOWN {
        return None;
    }
    Some(h)
}

/// Write `text`; returns `true` if it went somewhere.
pub fn write(stream: Stream, text: &str) -> bool {
    let which = match stream {
        Stream::Out => STD_OUTPUT_HANDLE,
        Stream::Err => STD_ERROR_HANDLE,
    };
    if let Some(h) = inherited(which) {
        let mut written = 0u32;
        // SAFETY: h is a valid inherited handle; the buffer is a live byte slice.
        return unsafe { WriteFile(h, Some(text.as_bytes()), Some(&raw mut written), None) }
            .is_ok();
    }
    // SAFETY: no preconditions; fails harmlessly if there is no parent console.
    if unsafe { AttachConsole(ATTACH_PARENT_PROCESS) }.is_err() {
        return false;
    }
    // SAFETY: CONOUT$ is the attached console's output buffer; the handle is closed below.
    let Ok(con) = (unsafe {
        CreateFileW(
            w!("CONOUT$"),
            GENERIC_WRITE.0,
            FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }) else {
        return false;
    };
    // Start on a fresh line: the parent shell has already printed its prompt.
    let wide: Vec<u16> = format!("\r\n{text}").encode_utf16().collect();
    let mut written = 0u32;
    // SAFETY: con is a valid console handle and `wide` a live UTF-16 buffer.
    let ok = unsafe { WriteConsoleW(con, &wide, Some(&raw mut written), None) }.is_ok();
    // SAFETY: con was opened above and is closed exactly once.
    let _ = unsafe { windows::Win32::Foundation::CloseHandle(con) };
    ok
}
