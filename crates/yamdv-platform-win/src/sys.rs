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
    copy_text_html(hwnd, text, None)
}

/// Copies plain text and, when given, a `CF_HTML` document (§4.9, R-REN-16).
pub fn copy_text_html(hwnd: HWND, text: &str, cf_html: Option<&str>) -> bool {
    use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
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
            if let Some(html) = cf_html {
                let fmt = RegisterClipboardFormatW(windows::core::w!("HTML Format"));
                let bytes = html.as_bytes();
                if fmt != 0
                    && let Ok(h) = GlobalAlloc(GMEM_MOVEABLE, bytes.len() + 1)
                {
                    let p = GlobalLock(h);
                    if p.is_null() {
                        let _ = GlobalFree(Some(h));
                    } else {
                        std::ptr::copy_nonoverlapping(bytes.as_ptr(), p.cast::<u8>(), bytes.len());
                        *p.cast::<u8>().add(bytes.len()) = 0;
                        let _ = GlobalUnlock(h);
                        if SetClipboardData(fmt, Some(HANDLE(h.0))).is_err() {
                            let _ = GlobalFree(Some(HGLOBAL(h.0)));
                        }
                    }
                }
            }
            Some(())
        })()
        .is_some();
        let _ = CloseClipboard();
        ok
    }
}

/// Plain text from the clipboard (`CF_UNICODETEXT`), `None` when there is none (V1-B paste).
pub fn paste_text(hwnd: HWND) -> Option<String> {
    use windows::Win32::System::DataExchange::{GetClipboardData, IsClipboardFormatAvailable};
    use windows::Win32::System::Memory::GlobalSize;
    // SAFETY: standard clipboard read; the handle belongs to the clipboard and is only locked
    // for the copy; reads stay within GlobalSize.
    unsafe {
        IsClipboardFormatAvailable(u32::from(CF_UNICODETEXT.0)).ok()?;
        OpenClipboard(Some(hwnd)).ok()?;
        let out = (|| -> Option<String> {
            let h = GetClipboardData(u32::from(CF_UNICODETEXT.0)).ok()?;
            let g = HGLOBAL(h.0);
            let p = GlobalLock(g);
            if p.is_null() {
                return None;
            }
            let max = GlobalSize(g) / 2;
            let s = std::slice::from_raw_parts(p.cast::<u16>(), max);
            let n = s.iter().position(|&c| c == 0).unwrap_or(max);
            let t = String::from_utf16_lossy(s.get(..n).unwrap_or(&[]));
            let _ = GlobalUnlock(g);
            Some(t)
        })();
        let _ = CloseClipboard();
        out
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
/// Runs only in the dialog-host process (S-PROC-07). `None` when the dialog could not be
/// shown; `Some(vec![])` when the user cancelled.
pub fn open_dialog(owner: HWND, f: &Filters<'_>) -> Option<Vec<String>> {
    // SAFETY: COM is initialized (apartment-threaded) by `app::run`; all objects are released by
    // their Drop impls; strings outlive the calls.
    unsafe {
        let Ok(dlg) =
            CoCreateInstance::<_, IFileOpenDialog>(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
        else {
            return None;
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
        if let Err(e) = dlg.Show(Some(owner)) {
            // HRESULT_FROM_WIN32(ERROR_CANCELLED)
            return if e.code().0 as u32 == 0x8007_04C7 {
                Some(Vec::new())
            } else {
                None
            };
        }
        let Ok(items) = dlg.GetResults() else {
            return None;
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
        Some(out)
    }
}

/// What a save dialog saves: a non-Markdown export (filter name, pattern, extension) and the
/// default file name.
#[derive(Debug, Clone, Default)]
pub struct SaveSpec {
    /// `(filter name, pattern, default extension)`; `None` = Markdown/Text/All.
    pub export: Option<(String, String, String)>,
    /// Default file name (`<doc stem>.html` …).
    pub default_name: Option<String>,
}

/// `IFileSaveDialog` (V1-B "Save … as…"). `Some(None)` = cancelled, `None` = failure.
pub fn save_dialog(owner: HWND, f: &Filters<'_>) -> Option<Option<String>> {
    save_dialog_spec(owner, f, &SaveSpec::default())
}

/// [`save_dialog`] with an export filter and a default file name.
pub fn save_dialog_spec(owner: HWND, f: &Filters<'_>, spec: &SaveSpec) -> Option<Option<String>> {
    use windows::Win32::UI::Shell::{FOS_OVERWRITEPROMPT, FileSaveDialog, IFileSaveDialog};
    // SAFETY: as in `open_dialog`.
    unsafe {
        let dlg: IFileSaveDialog =
            CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        let (names, specs, ext) = match &spec.export {
            Some((n, p, e)) => (
                vec![HSTRING::from(n.as_str()), HSTRING::from(f.all)],
                vec![HSTRING::from(p.as_str()), HSTRING::from("*.*")],
                e.as_str(),
            ),
            None => (
                vec![
                    HSTRING::from(f.markdown),
                    HSTRING::from(f.text),
                    HSTRING::from(f.all),
                ],
                vec![
                    HSTRING::from(MD_PATTERN),
                    HSTRING::from("*.txt"),
                    HSTRING::from("*.*"),
                ],
                "md",
            ),
        };
        let filters: Vec<COMDLG_FILTERSPEC> = names
            .iter()
            .zip(specs.iter())
            .map(|(n, s)| COMDLG_FILTERSPEC {
                pszName: PCWSTR(n.as_ptr()),
                pszSpec: PCWSTR(s.as_ptr()),
            })
            .collect();
        let _ = dlg.SetFileTypes(&filters);
        let _ = dlg.SetDefaultExtension(&HSTRING::from(ext));
        if let Some(n) = &spec.default_name {
            let _ = dlg.SetFileName(&HSTRING::from(n.as_str()));
        }
        if let Ok(opts) = dlg.GetOptions() {
            let _ = dlg.SetOptions(opts | FOS_OVERWRITEPROMPT | FOS_FORCEFILESYSTEM);
        }
        if let Err(e) = dlg.Show(Some(owner)) {
            return (e.code().0 as u32 == 0x8007_04C7).then_some(None);
        }
        let item = dlg.GetResult().ok()?;
        let p = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let s = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as *const core::ffi::c_void));
        s.map(Some)
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

/// Hands a validated URL or an allowlisted local file to the default handler (S-EXEC-04,
/// R-LINK-04): `ShellExecuteExW` with the "open" verb, never `SEE_MASK_NOZONECHECKS`.
pub fn shell_open(target: &str) {
    if dry_launch() {
        return;
    }
    use windows::Win32::UI::Shell::{SEE_MASK_NOASYNC, SHELLEXECUTEINFOW, ShellExecuteExW};
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let file = windows::core::HSTRING::from(target);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: u32::try_from(std::mem::size_of::<SHELLEXECUTEINFOW>()).unwrap_or(0),
        fMask: SEE_MASK_NOASYNC,
        lpVerb: windows::core::w!("open"),
        lpFile: windows::core::PCWSTR(file.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    // SAFETY: `info` is fully initialized and `file` outlives the call.
    let _ = unsafe { ShellExecuteExW(&mut info) };
}

/// Opens Explorer with `path` selected (R-LINK-05); never runs the file.
pub fn reveal_in_explorer(path: &str) {
    if dry_launch() {
        return;
    }
    use windows::Win32::UI::Shell::{ILCreateFromPathW, ILFree, SHOpenFolderAndSelectItems};
    let p = windows::core::HSTRING::from(path);
    // SAFETY: valid NUL-terminated path; the PIDL is freed below.
    unsafe {
        let pidl = ILCreateFromPathW(windows::core::PCWSTR(p.as_ptr()));
        if !pidl.is_null() {
            let _ = SHOpenFolderAndSelectItems(pidl, None, 0);
            ILFree(Some(pidl));
        }
    }
}

/// Punycode form of an IDN host via `IdnToAscii` (offline, S-LINK-02).
pub fn idn_to_ascii(host: &str) -> Option<String> {
    use windows::Win32::Globalization::IdnToAscii;
    let src: Vec<u16> = host.encode_utf16().collect();
    let mut buf = [0u16; 512];
    // SAFETY: valid input slice and output buffer.
    let n = unsafe { IdnToAscii(0, &src, Some(&mut buf)) };
    let n = usize::try_from(n)
        .ok()
        .filter(|&n| n > 0 && n <= buf.len())?;
    Some(String::from_utf16_lossy(buf.get(..n)?))
}

/// R-LINK-03: `Page.md` (then `Page.markdown`) in `dir`, then recursively in subfolders (max
/// depth 4, first match by shortest path). Bounded: at most 10 000 directory entries.
pub fn find_wiki(dir: &str, name: &str) -> Option<String> {
    let mut cands = vec![name.replace('/', "\\")];
    if let Some(stem) = name.strip_suffix(".md") {
        cands.push(format!("{stem}.markdown"));
    }
    let mut level = vec![dir.to_string()];
    let mut budget = 10_000usize;
    for _ in 0..=4 {
        let mut hits: Vec<String> = Vec::new();
        for d in &level {
            for c in &cands {
                let p = format!("{d}\\{c}");
                if std::path::Path::new(&yamdv_core::path::long_form(&p)).is_file() {
                    hits.push(p);
                }
            }
        }
        if let Some(h) = hits.into_iter().min_by_key(String::len) {
            return Some(h);
        }
        let mut next = Vec::new();
        for d in &level {
            let Ok(rd) = std::fs::read_dir(yamdv_core::path::long_form(d)) else {
                continue;
            };
            for e in rd.flatten() {
                if budget == 0 {
                    return None;
                }
                budget -= 1;
                if e.file_type().is_ok_and(|t| t.is_dir() && !t.is_symlink()) {
                    next.push(format!("{d}\\{}", e.file_name().to_string_lossy()));
                }
            }
        }
        next.sort();
        level = next;
    }
    None
}

static DRY_RUN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static LAUNCHES: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Test driver (§4.5): count `shell_open` / `reveal_in_explorer` instead of launching.
pub fn set_launch_dry_run() {
    DRY_RUN.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Launches requested since start (counted only under the test driver).
pub fn launches() -> u32 {
    LAUNCHES.load(std::sync::atomic::Ordering::Relaxed)
}

/// Counts a launch; `true` when launches are dry (test driver).
pub(crate) fn dry_launch_pub() -> bool {
    dry_launch()
}

fn dry_launch() -> bool {
    LAUNCHES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    DRY_RUN.load(std::sync::atomic::Ordering::Relaxed)
}

/// D.4 rule 10: one working-set trim right after a print / PDF / export operation (never
/// periodic; transient buffers are freed by the caller before this).
pub fn trim_once() {
    use windows::Win32::System::Memory::{
        SETPROCESSWORKINGSETSIZEEX_FLAGS, SetProcessWorkingSetSizeEx,
    };
    use windows::Win32::System::Threading::GetCurrentProcess;
    // SAFETY: pseudo handle of this process; (-1, -1) asks the system to trim the working set.
    let _ = unsafe {
        SetProcessWorkingSetSizeEx(
            GetCurrentProcess(),
            usize::MAX,
            usize::MAX,
            SETPROCESSWORKINGSETSIZEEX_FLAGS(0),
        )
    };
}

/// User locale name (`en-US`, `tr-TR`, …) for the *auto* paper size.
pub fn user_locale() -> String {
    let mut buf = [0u16; 85];
    // SAFETY: valid buffer.
    let n = unsafe { windows::Win32::Globalization::GetUserDefaultLocaleName(&mut buf) };
    let n = usize::try_from(n).unwrap_or(0).saturating_sub(1);
    String::from_utf16_lossy(buf.get(..n).unwrap_or(&[]))
}

/// The print dialog's answer (prompt 008 §3.1): serializable data only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintChoice {
    /// Printer name.
    pub printer: String,
    /// DEVMODE bytes (`dmSize + dmDriverExtra`).
    pub devmode: Vec<u8>,
    /// 1-based inclusive page range (`None` = all pages).
    pub range: Option<(u32, u32)>,
    /// Copies.
    pub copies: u32,
}

/// `PrintDlgExW` (dialog host only): `Some(Some(choice))` = print, `Some(None)` = cancelled,
/// `None` = the dialog failed.
pub fn print_dialog(owner: HWND) -> Option<Option<PrintChoice>> {
    use windows::Win32::Foundation::GlobalFree;
    use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
    use windows::Win32::UI::Controls::Dialogs::{
        DEVNAMES, PD_NOCURRENTPAGE, PD_NOSELECTION, PD_PAGENUMS, PD_RESULT_PRINT,
        PD_USEDEVMODECOPIESANDCOLLATE, PRINTDLGEXW, PRINTPAGERANGE, PrintDlgExW,
        START_PAGE_GENERAL,
    };
    let mut range = PRINTPAGERANGE {
        nFromPage: 1,
        nToPage: 1,
    };
    let mut pd = PRINTDLGEXW {
        lStructSize: size_of::<PRINTDLGEXW>() as u32,
        hwndOwner: owner,
        Flags: PD_NOSELECTION | PD_NOCURRENTPAGE | PD_USEDEVMODECOPIESANDCOLLATE,
        nMaxPageRanges: 1,
        lpPageRanges: &raw mut range,
        nMinPage: 1,
        nMaxPage: 0xFFFF,
        nCopies: 1,
        nStartPage: START_PAGE_GENERAL,
        ..Default::default()
    };
    // SAFETY: the structure and the page-range buffer outlive the modal call.
    unsafe { PrintDlgExW(&raw mut pd) }.ok()?;
    // SAFETY: the dialog allocated the movable global blocks; each is locked, copied, unlocked
    // and freed exactly once; DEVNAMES offsets are in characters from the block start.
    let (printer, devmode) = unsafe {
        let mut printer = String::new();
        if !pd.hDevNames.is_invalid() {
            let p = GlobalLock(pd.hDevNames).cast::<u16>();
            if !p.is_null() {
                let dn = &*p.cast::<DEVNAMES>();
                let start = p.add(usize::from(dn.wDeviceOffset));
                let mut n = 0usize;
                while n < 256 && *start.add(n) != 0 {
                    n += 1;
                }
                printer = String::from_utf16_lossy(std::slice::from_raw_parts(start, n));
                let _ = GlobalUnlock(pd.hDevNames);
            }
            let _ = GlobalFree(Some(pd.hDevNames));
        }
        let mut dm = Vec::new();
        if !pd.hDevMode.is_invalid() {
            let p = GlobalLock(pd.hDevMode).cast::<u8>();
            if !p.is_null() {
                let size = GlobalSize(pd.hDevMode).min(64 * 1024);
                dm = std::slice::from_raw_parts(p, size).to_vec();
                let _ = GlobalUnlock(pd.hDevMode);
            }
            let _ = GlobalFree(Some(pd.hDevMode));
        }
        (printer, dm)
    };
    if pd.dwResultAction != PD_RESULT_PRINT || printer.is_empty() {
        return Some(None);
    }
    let range = (pd.Flags.0 & PD_PAGENUMS.0 != 0 && pd.nPageRanges > 0)
        .then_some((range.nFromPage.max(1), range.nToPage.max(range.nFromPage)));
    Some(Some(PrintChoice {
        printer,
        devmode,
        range,
        copies: pd.nCopies.max(1),
    }))
}

/// Lower-case hex of `b` (DEVMODE bytes in the dialog-host reply).
pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Inverse of [`hex`] (`None` on odd length or a non-hex digit).
pub fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| s.get(i..i + 2).and_then(|h| u8::from_str_radix(h, 16).ok()))
        .collect()
}
