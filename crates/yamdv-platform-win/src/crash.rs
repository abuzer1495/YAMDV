//! Panic hook: writes a short crash note (version, location, message — **no document content**,
//! S-PRIV-03) to `%LOCALAPPDATA%\YAMDV\logs\crash-<timestamp>.txt`, then aborts.
//! PHASE-006: recovery of unsaved text is added with the editor.
#![allow(unsafe_code)]

use std::path::PathBuf;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::Shell::{FOLDERID_LocalAppData, KF_FLAG_DEFAULT, SHGetKnownFolderPath};

/// `%LOCALAPPDATA%` resolved through the known-folder API (not the environment, S-PROC-08).
pub fn local_app_data() -> Option<PathBuf> {
    // SAFETY: FOLDERID_LocalAppData is a static GUID; the returned buffer is freed below.
    let p = unsafe { SHGetKnownFolderPath(&FOLDERID_LocalAppData, KF_FLAG_DEFAULT, None) }.ok()?;
    // SAFETY: p is a valid NUL-terminated string allocated by the shell.
    let s = unsafe { p.to_string() }.ok();
    // SAFETY: p was allocated with CoTaskMemAlloc by SHGetKnownFolderPath.
    unsafe { CoTaskMemFree(Some(p.0.cast_const().cast())) };
    s.map(PathBuf::from)
}

fn timestamp() -> String {
    // SAFETY: GetLocalTime has no preconditions.
    let t = unsafe { GetLocalTime() };
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}-{:03}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}

/// Format the crash note text (pure helper; no document content is ever passed in).
pub fn note(version: &str, location: &str, message: &str) -> String {
    format!("YAMDV crash note\nversion: {version}\nlocation: {location}\nmessage: {message}\n")
}

/// Install the panic hook.
pub fn install(version: &'static str) {
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".into());
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "non-string panic payload".into());
        // Bound the message so a huge payload can never be written out.
        let message: String = message.chars().take(512).collect();
        if let Some(dir) = local_app_data().map(|d| d.join("YAMDV").join("logs"))
            && std::fs::create_dir_all(&dir).is_ok()
        {
            let path = dir.join(format!("crash-{}.txt", timestamp()));
            let _ = std::fs::write(path, note(version, &location, &message));
        }
        std::process::abort();
    }));
}
