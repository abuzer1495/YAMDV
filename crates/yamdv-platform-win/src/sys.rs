//! Small Win32 services for the reading window: path intake (`GetFullPathNameW`), file reading,
//! clipboard (`CF_UNICODETEXT`), `IFileOpenDialog`, working-set trim, system parameters.
#![allow(unsafe_code)]
// Pixel/DIP geometry: float<->int conversions are bounded by window/bitmap sizes (ADR-0007).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::indexing_slicing
)]

use windows::Win32::Foundation::GlobalFree;
use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND};
use windows::Win32::Storage::FileSystem::GetFullPathNameW;
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::CF_UNICODETEXT;
use windows::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    FOS_ALLOWMULTISELECT, FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FileOpenDialog, IFileOpenDialog,
    SIGDN_FILESYSPATH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    SPI_GETCLIENTAREAANIMATION, SPI_GETWHEELSCROLLLINES, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    SystemParametersInfoW,
};
use windows::core::{BOOL, HSTRING, PCWSTR};

/// `GetFullPathNameW` (normalizes `.`/`..`, relative paths, separators). `None` on failure.
pub fn full_path(path: &str) -> Option<String> {
    if path.is_empty() || path.contains('\0') {
        return None;
    }
    let p = HSTRING::from(path);
    // SAFETY: first call queries the required length (in u16 incl. NUL).
    let need = unsafe { GetFullPathNameW(&p, None, None) };
    if need == 0 {
        return None;
    }
    let mut buf = vec![0u16; need as usize + 1];
    // SAFETY: buffer sized from the first call.
    let n = unsafe { GetFullPathNameW(&p, Some(&mut buf), None) } as usize;
    if n == 0 || n >= buf.len() {
        return None;
    }
    Some(String::from_utf16_lossy(buf.get(..n).unwrap_or(&[])))
}

/// Facts about an opened file (R-OPEN-10).
#[derive(Debug, Clone, Default)]
pub struct FileInfo {
    /// Size in bytes.
    pub size: u64,
    /// Read-only attribute set.
    pub read_only: bool,
    /// Last modification time.
    pub mtime: Option<std::time::SystemTime>,
}

/// Metadata of a (long-form) path.
pub fn stat(long: &str) -> std::io::Result<(FileInfo, bool)> {
    let m = std::fs::metadata(long)?;
    Ok((
        FileInfo {
            size: m.len(),
            read_only: m.permissions().readonly(),
            mtime: m.modified().ok(),
        },
        m.is_dir(),
    ))
}

/// Puts `text` on the clipboard as `CF_UNICODETEXT`.
pub fn copy_text(hwnd: HWND, text: &str) -> bool {
    let w: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    let bytes = w.len() * 2;
    // SAFETY: standard clipboard sequence; the global memory is owned by the clipboard after a
    // successful SetClipboardData and freed by us otherwise.
    unsafe {
        if OpenClipboard(Some(hwnd)).is_err() {
            return false;
        }
        let _ = EmptyClipboard();
        let ok = (|| -> Option<()> {
            let h = GlobalAlloc(GMEM_MOVEABLE, bytes).ok()?;
            let p = GlobalLock(h);
            if p.is_null() {
                let _ = GlobalFree(Some(h));
                return None;
            }
            std::ptr::copy_nonoverlapping(w.as_ptr(), p.cast::<u16>(), w.len());
            let _ = GlobalUnlock(h);
            if SetClipboardData(u32::from(CF_UNICODETEXT.0), Some(HANDLE(h.0))).is_err() {
                let _ = GlobalFree(Some(HGLOBAL(h.0)));
                return None;
            }
            Some(())
        })()
        .is_some();
        let _ = CloseClipboard();
        ok
    }
}

/// Filter labels for the open dialog (R-OPEN-03).
pub struct Filters<'a> {
    /// "Markdown".
    pub markdown: &'a str,
    /// "Text".
    pub text: &'a str,
    /// "All files".
    pub all: &'a str,
}

/// Markdown extensions of R-OPEN-03.
pub const MD_PATTERN: &str = "*.md;*.markdown;*.mdown;*.mkd;*.mkdn;*.mdwn;*.mdtxt;*.mdtext";

/// `IFileOpenDialog` with multi-select. Returns the chosen paths (empty on cancel).
pub fn open_dialog(owner: HWND, f: &Filters<'_>) -> Vec<String> {
    // SAFETY: COM is initialized (apartment-threaded) by `app::run`; all objects are released by
    // their Drop impls; strings outlive the calls.
    unsafe {
        let Ok(dlg) =
            CoCreateInstance::<_, IFileOpenDialog>(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
        else {
            return Vec::new();
        };
        let names = [
            HSTRING::from(f.markdown),
            HSTRING::from(f.text),
            HSTRING::from(f.all),
        ];
        let specs = [
            HSTRING::from(MD_PATTERN),
            HSTRING::from("*.txt"),
            HSTRING::from("*.*"),
        ];
        let filters: Vec<COMDLG_FILTERSPEC> = names
            .iter()
            .zip(specs.iter())
            .map(|(n, s)| COMDLG_FILTERSPEC {
                pszName: PCWSTR(n.as_ptr()),
                pszSpec: PCWSTR(s.as_ptr()),
            })
            .collect();
        let _ = dlg.SetFileTypes(&filters);
        if let Ok(opts) = dlg.GetOptions() {
            let _ = dlg
                .SetOptions(opts | FOS_ALLOWMULTISELECT | FOS_FILEMUSTEXIST | FOS_FORCEFILESYSTEM);
        }
        if dlg.Show(Some(owner)).is_err() {
            return Vec::new();
        }
        let Ok(items) = dlg.GetResults() else {
            return Vec::new();
        };
        let n = items.GetCount().unwrap_or(0);
        let mut out = Vec::new();
        for i in 0..n {
            if let Ok(item) = items.GetItemAt(i)
                && let Ok(p) = item.GetDisplayName(SIGDN_FILESYSPATH)
            {
                if let Ok(s) = p.to_string() {
                    out.push(s);
                }
                CoTaskMemFree(Some(p.0 as *const core::ffi::c_void));
            }
        }
        out
    }
}

/// Trims the working set once (D.4 rule 10, after transient UI such as the open dialog).
pub fn trim_working_set() {
    // SAFETY: (-1, -1) asks the system to trim the current process's working set.
    unsafe {
        let _ = SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
    }
}

/// Client-area animations enabled? Off → reduced motion (R-A11Y-02).
pub fn animations_enabled() -> bool {
    let mut on = BOOL(1);
    // SAFETY: valid out-pointer for a BOOL.
    let r = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some((&raw mut on).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    r.is_err() || on.as_bool()
}

/// Lines per wheel notch (`SPI_GETWHEELSCROLLLINES`, default 3).
pub fn wheel_lines() -> u32 {
    let mut n = 3u32;
    // SAFETY: valid out-pointer for a UINT.
    let r = unsafe {
        SystemParametersInfoW(
            SPI_GETWHEELSCROLLLINES,
            0,
            Some((&raw mut n).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    if r.is_err() { 3 } else { n.min(100) }
}

/// UI language tag from `GetUserDefaultUILanguage` (`tr` for Turkish, else `en`).
pub fn ui_language_tag() -> &'static str {
    // SAFETY: no preconditions.
    let id = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() };
    if id & 0x3FF == 0x1F { "tr-TR" } else { "en-US" }
}
