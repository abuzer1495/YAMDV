//! OS services of the app shell (prompt 005 §3.5, §3.6, §3.10, §3.11): single instance
//! (S-IPC-01…03), directory watcher (R-READ-01), external editor (R-EXT, S-EXEC-03),
//! per-user registration (R-SYS-01…03, S-REG-01…03), atomic file writes (S-FILE-03).
// Win32 sizes and handles: the casts are bounded (struct sizes, small buffers).
#![allow(unsafe_code)]
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]

use std::os::windows::io::AsRawHandle;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, HWND, LPARAM, WPARAM};
use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, ERROR_SUCCESS};
use windows::Win32::Security::{
    GetLengthSid, GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_LIST_DIRECTORY, FILE_NOTIFY_CHANGE_FILE_NAME,
    FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING, ReadDirectoryChangesW,
};
use windows::Win32::System::IO::CancelSynchronousIo;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
    RegCloseKey, RegCreateKeyExW, RegDeleteKeyValueW, RegDeleteTreeW, RegOpenKeyExW,
    RegSetValueExW,
};
use windows::Win32::System::Threading::{
    CREATE_UNICODE_ENVIRONMENT, CreateMutexW, CreateProcessW, GetCurrentProcess, OpenProcessToken,
    PROCESS_INFORMATION, STARTUPINFOW,
};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW};
use windows::core::{HSTRING, PCWSTR, PWSTR};
use yamdv_core::cmdline;
use yamdv_core::reg;

/// Posted to the window when its document's directory changed (`wParam` = generation).
pub const WM_APP_FILE_CHANGED: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 20;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// FNV-1a over the current user's SID bytes (mutex name, S-IPC-01).
fn user_sid_hash() -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut tok = HANDLE::default();
    // SAFETY: pseudo handle of the current process; out-pointer on the stack.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut tok) }.is_err() {
        return h;
    }
    let mut buf = vec![0u8; 256];
    let mut len = 0u32;
    // SAFETY: buffer and length valid; token closed below.
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
    // SAFETY: opened above.
    let _ = unsafe { CloseHandle(tok) };
    if ok {
        // SAFETY: buf holds a TOKEN_USER filled by GetTokenInformation; the SID lives inside buf.
        unsafe {
            let tu = &*buf.as_ptr().cast::<TOKEN_USER>();
            let n = GetLengthSid(tu.User.Sid) as usize;
            let bytes = std::slice::from_raw_parts(tu.User.Sid.0.cast::<u8>(), n.min(68));
            for b in bytes {
                h ^= u64::from(*b);
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
        }
    }
    h
}

/// Holds the single-instance mutex for the life of the process.
pub struct Instance(#[allow(dead_code)] HANDLE);

/// Claims the single-instance mutex; `None` when another instance of this user already runs.
pub fn claim_instance() -> Option<Instance> {
    let name = HSTRING::from(format!("Local\\YAMDV-{:016x}", user_sid_hash()));
    // SAFETY: valid name; the handle is kept for the process lifetime.
    let h = unsafe { CreateMutexW(None, false, &name) }.ok()?;
    // SAFETY: no preconditions.
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        // SAFETY: handle created above.
        let _ = unsafe { CloseHandle(h) };
        return None;
    }
    Some(Instance(h))
}

/// Forwards `paths` (already made absolute) to the primary instance (R-WIN-01). `true` when a
/// primary window accepted them.
pub fn forward(paths: &[String], line: u64) -> bool {
    // SAFETY: class name is static.
    let Ok(target) = (unsafe { FindWindowW(windows::core::w!("YAMDV.Window"), PCWSTR::null()) })
    else {
        return false;
    };
    let mut pid = 0u32;
    // SAFETY: valid hwnd.
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(target, Some(&mut pid));
        // Lets the primary bring its window to the front (R-WIN-02).
        let _ = windows::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(pid);
    }
    let data = yamdv_core::ipc::encode_forward(paths, line);
    crate::dialog_host::send_copydata(target, &data, 5000)
}

/// Directory watcher thread for one window (R-READ-01). Dropping it stops the thread.
pub struct Watcher {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            // SAFETY: the thread handle is valid until joined; cancels its blocking read.
            unsafe {
                let _ = CancelSynchronousIo(HANDLE(t.as_raw_handle()));
            }
            let _ = t.join();
        }
    }
}

/// Watches the directory of `file`; posts [`WM_APP_FILE_CHANGED`] to `hwnd` on any change that
/// names the file (atomic-rename saves included).
pub fn watch(file: &str, hwnd: HWND) -> Option<Watcher> {
    watch_tagged(file, hwnd, 0)
}

/// [`watch`] with a tag passed as `wParam` (0 = document, 1 = `custom-theme.ini`).
pub fn watch_tagged(file: &str, hwnd: HWND, tag: usize) -> Option<Watcher> {
    let (dir, name) = file.rsplit_once('\\')?;
    let dir = format!("{dir}\\");
    let name = name.to_lowercase();
    let stop = Arc::new(AtomicBool::new(false));
    let st = stop.clone();
    let target = hwnd.0 as isize;
    let thread = std::thread::Builder::new()
        .name("yamdv-watch".into())
        .stack_size(64 * 1024)
        .spawn(move || watch_loop(&dir, &name, target, tag, &st))
        .ok()?;
    Some(Watcher {
        stop,
        thread: Some(thread),
    })
}

fn watch_loop(dir: &str, name: &str, hwnd: isize, tag: usize, stop: &AtomicBool) {
    let d = HSTRING::from(yamdv_core::path::long_form(dir));
    // SAFETY: valid path; directory handle closed at the end.
    let Ok(h) = (unsafe {
        CreateFileW(
            &d,
            FILE_LIST_DIRECTORY.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            None,
        )
    }) else {
        return;
    };
    let mut buf = vec![0u32; 4096];
    while !stop.load(Ordering::Relaxed) {
        let mut got = 0u32;
        // SAFETY: valid handle and DWORD-aligned buffer; synchronous call (cancelled on drop).
        let ok = unsafe {
            ReadDirectoryChangesW(
                h,
                buf.as_mut_ptr().cast(),
                (buf.len() * 4) as u32,
                false,
                FILE_NOTIFY_CHANGE_FILE_NAME
                    | FILE_NOTIFY_CHANGE_LAST_WRITE
                    | FILE_NOTIFY_CHANGE_SIZE,
                Some(&mut got),
                None,
                None,
            )
        }
        .is_ok();
        if !ok || stop.load(Ordering::Relaxed) {
            break;
        }
        // got == 0: buffer overflow, treat as "maybe changed".
        let hit = got == 0
            || notify_names(&buf, got as usize)
                .iter()
                .any(|n| n.to_lowercase() == name);
        if hit {
            // SAFETY: posting to a window handle is safe even if it was destroyed meanwhile.
            unsafe {
                let _ = PostMessageW(
                    Some(HWND(std::ptr::without_provenance_mut(hwnd.cast_unsigned()))),
                    WM_APP_FILE_CHANGED,
                    WPARAM(tag),
                    LPARAM(0),
                );
            }
        }
    }
    // SAFETY: opened above.
    let _ = unsafe { CloseHandle(h) };
}

/// File names in a `FILE_NOTIFY_INFORMATION` chain (bounds-checked).
fn notify_names(buf: &[u32], len: usize) -> Vec<String> {
    let bytes: Vec<u8> = buf.iter().flat_map(|w| w.to_le_bytes()).take(len).collect();
    let mut out = Vec::new();
    let mut at = 0usize;
    loop {
        let rd = |o: usize| {
            bytes
                .get(o..o + 4)
                .and_then(|s| s.try_into().ok())
                .map(u32::from_le_bytes)
        };
        let (Some(next), Some(nlen)) = (rd(at), rd(at + 8)) else {
            break;
        };
        let start = at + 12;
        if let Some(raw) = bytes.get(start..start + nlen as usize) {
            let u: Vec<u16> = raw
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes(*c))
                .collect();
            out.push(String::from_utf16_lossy(&u));
        }
        if next == 0 || out.len() > 1000 {
            break;
        }
        at += next as usize;
    }
    out
}

fn env(k: &str) -> String {
    std::env::var(k).unwrap_or_default()
}

/// Detects the external editor (R-EXT-02, FUT-10 scope): VS Code, Notepad++, Notepad.
fn detect_editor() -> Option<(String, cmdline::Editor)> {
    let cands = [
        (
            format!(
                "{}\\Programs\\Microsoft VS Code\\Code.exe",
                env("LOCALAPPDATA")
            ),
            cmdline::Editor::VsCode,
        ),
        (
            format!("{}\\Microsoft VS Code\\Code.exe", env("ProgramFiles")),
            cmdline::Editor::VsCode,
        ),
        (
            format!("{}\\Notepad++\\notepad++.exe", env("ProgramFiles")),
            cmdline::Editor::NotepadPp,
        ),
        (
            format!("{}\\Notepad++\\notepad++.exe", env("ProgramFiles(x86)")),
            cmdline::Editor::NotepadPp,
        ),
        (
            format!("{}\\System32\\notepad.exe", env("SystemRoot")),
            cmdline::Editor::Notepad,
        ),
    ];
    cands
        .into_iter()
        .find(|(p, _)| !p.starts_with('\\') && std::path::Path::new(p).is_file())
}

/// Starts `exe` with `args` via `CreateProcessW` only (S-EXEC-03). `false` when refused/failed.
pub fn spawn(exe: &str, args: &[String]) -> bool {
    if cmdline::forbidden_target(exe) || !std::path::Path::new(exe).is_file() {
        return false;
    }
    if crate::sys::dry_launch_pub() {
        return true;
    }
    create_process(exe, args)
}

/// Starts this executable again (driver scripts: second-instance forwarding, R-SYS-03).
pub fn spawn_self(args: &[String]) -> bool {
    create_process(&exe_path(), args)
}

fn create_process(exe: &str, args: &[String]) -> bool {
    let app = HSTRING::from(exe);
    let mut cl = wide(&cmdline::command_line(exe, args));
    let si = STARTUPINFOW {
        cb: std::mem::size_of::<STARTUPINFOW>() as u32,
        ..Default::default()
    };
    let mut pi = PROCESS_INFORMATION::default();
    // SAFETY: valid application name, writable command line and structures; handles closed.
    let ok = unsafe {
        CreateProcessW(
            &app,
            Some(PWSTR(cl.as_mut_ptr())),
            None,
            None,
            false,
            CREATE_UNICODE_ENVIRONMENT,
            None,
            PCWSTR::null(),
            &si,
            &mut pi,
        )
    }
    .is_ok();
    if ok {
        // SAFETY: handles returned by CreateProcessW.
        unsafe {
            let _ = CloseHandle(pi.hThread);
            let _ = CloseHandle(pi.hProcess);
        }
    }
    ok
}

/// Opens `file` at `line` in the configured or detected editor (R-EXT-01…03).
pub fn open_in_editor(
    file: &str,
    line: u32,
    custom_exe: &str,
    custom_args: &str,
    custom: bool,
) -> bool {
    if custom && !custom_exe.is_empty() {
        let args = cmdline::expand_template(custom_args, file, line, 1);
        return spawn(custom_exe, &args);
    }
    match detect_editor() {
        Some((exe, e)) => spawn(&exe, &cmdline::editor_args(e, file, line, 1)),
        None => false,
    }
}

fn set_values(vals: &[reg::Value]) -> bool {
    let mut ok = true;
    for v in vals {
        if !reg::allowed_key(&v.key) {
            ok = false;
            continue;
        }
        let k = HSTRING::from(v.key.as_str());
        let mut hk = HKEY::default();
        // SAFETY: HKCU subkey creation with an out-pointer on the stack; closed below.
        let r = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                &k,
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                None,
                &mut hk,
                None,
            )
        };
        if r != ERROR_SUCCESS {
            ok = false;
            continue;
        }
        let data: Vec<u8> = wide(&v.data).iter().flat_map(|u| u.to_le_bytes()).collect();
        let name = HSTRING::from(v.name.as_str());
        // SAFETY: valid key handle and data buffer.
        let r = unsafe {
            RegSetValueExW(
                hk,
                if v.name.is_empty() {
                    PCWSTR::null()
                } else {
                    PCWSTR(name.as_ptr())
                },
                None,
                REG_SZ,
                Some(&data),
            )
        };
        ok &= r == ERROR_SUCCESS;
        // SAFETY: opened above.
        let _ = unsafe { RegCloseKey(hk) };
    }
    ok
}

fn remove(items: &[reg::Removal]) {
    for r in items {
        match r {
            reg::Removal::Tree(k) => {
                let k = HSTRING::from(k.as_str());
                // SAFETY: HKCU subtree owned by YAMDV (S-REG-02).
                let _ = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, &k) };
                // SAFETY: as above; removes the now empty key itself.
                let _ = unsafe {
                    windows::Win32::System::Registry::RegDeleteKeyW(HKEY_CURRENT_USER, &k)
                };
            }
            reg::Removal::Value(k, n) => {
                let (k, n) = (HSTRING::from(k.as_str()), HSTRING::from(n.as_str()));
                // SAFETY: deletes one of our values from a shared key.
                let _ = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, &k, &n) };
            }
        }
    }
}

/// Absolute path of the running exe.
pub fn exe_path() -> String {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// R-SYS-01 per-user registration (explicit action only, S-REG-03).
pub fn register() -> bool {
    let ok = set_values(&reg::register_plan(&exe_path()));
    notify_assoc_changed();
    ok
}

/// Removes exactly what [`register`] and [`set_context_menu`] wrote (S-REG-02).
pub fn unregister() {
    remove(&reg::unregister_plan());
    notify_assoc_changed();
}

/// R-SYS-03 Explorer verb on/off.
pub fn set_context_menu(on: bool, label: &str) -> bool {
    let ok = if on {
        set_values(&reg::context_menu_plan(&exe_path(), label))
    } else {
        remove(&reg::context_menu_removal());
        true
    };
    notify_assoc_changed();
    ok
}

/// `true` when the Explorer verb key exists.
pub fn context_menu_present() -> bool {
    let k = HSTRING::from("Software\\Classes\\SystemFileAssociations\\.md\\shell\\YAMDV");
    let mut hk = HKEY::default();
    // SAFETY: read-only open of our own key (S-REG-04); closed below.
    let r = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, &k, None, KEY_SET_VALUE, &mut hk) };
    if r == ERROR_SUCCESS {
        // SAFETY: opened above.
        let _ = unsafe { RegCloseKey(hk) };
        true
    } else {
        false
    }
}

fn notify_assoc_changed() {
    use windows::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};
    // SAFETY: documented notification with no items.
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None) };
}

/// R-SYS-02 association status: is our exe the handler for `.md`?
pub fn is_default_app() -> bool {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW};
    let mut buf = [0u16; 256];
    let mut n = (buf.len() * 2) as u32;
    // SAFETY: valid key/value names and a writable buffer of `n` bytes; read-only (S-REG-04).
    // The `UserChoice` ProgId is what Windows Settings writes; reading it avoids loading the
    // shell association engine into the viewer process (M0).
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            windows::core::w!(
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.md\UserChoice"
            ),
            windows::core::w!("ProgId"),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut n),
        )
    };
    if r.is_err() {
        return false;
    }
    let len = (n as usize / 2).saturating_sub(1);
    String::from_utf16_lossy(buf.get(..len).unwrap_or(&[])) == yamdv_core::reg::PROGID
}

/// R-SYS-02: register, then open Windows Settings for the app (never writes `UserChoice`).
pub fn set_default_flow() -> bool {
    let ok = register();
    crate::sys::shell_open("ms-settings:defaultapps?registeredAppUser=YAMDV");
    ok
}

/// Atomic write (S-FILE-03): temp file in the same folder, then replace.
pub fn write_atomic(path: &str, text: &str) -> bool {
    let p = std::path::Path::new(path);
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let tmp = format!("{path}.tmp");
    std::fs::write(&tmp, text).is_ok() && std::fs::rename(&tmp, p).is_ok()
}

/// Document save (V1-B, S-FILE-03): a hidden `.~<name>.yamdv-<random>.tmp` created new next to
/// the target, flushed, then `ReplaceFileW` (keeps the target's attributes, ACL and streams),
/// falling back to `MoveFileExW(REPLACE_EXISTING | WRITE_THROUGH)`. A symbolic link is saved
/// through to its final target. Read-only targets are never cleared (callers refuse them).
pub fn save_document(path: &str, bytes: &[u8]) -> std::io::Result<()> {
    use std::hash::{BuildHasher, Hasher};
    use std::io::Write;
    use std::os::windows::fs::OpenOptionsExt;
    use windows::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_NORMAL, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        REPLACE_FILE_FLAGS, ReplaceFileW, SetFileAttributesW,
    };
    use windows::core::HSTRING;
    let exists = std::fs::symlink_metadata(path).is_ok();
    // Symlink → final target (`canonicalize` = GetFinalPathNameByHandleW).
    let target = if exists {
        std::fs::canonicalize(path)?
    } else {
        std::path::PathBuf::from(path)
    };
    let dir = target
        .parent()
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos()),
    );
    let tmp = dir.join(format!(".~{name}.yamdv-{:016x}.tmp", h.finish()));
    let write = || -> std::io::Result<()> {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .attributes(0x2) // FILE_ATTRIBUTE_HIDDEN
            .open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all() // FlushFileBuffers
    };
    if let Err(e) = write() {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    let (t, s) = (
        HSTRING::from(target.to_string_lossy().as_ref()),
        HSTRING::from(tmp.to_string_lossy().as_ref()),
    );
    // SAFETY: valid NUL-terminated paths for the duration of the calls.
    let ok = unsafe {
        (exists && ReplaceFileW(&t, &s, None, REPLACE_FILE_FLAGS(0), None, None).is_ok())
            || MoveFileExW(&s, &t, MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH).is_ok()
    };
    if !ok {
        let e = std::io::Error::last_os_error();
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    if !exists {
        // A new file must not stay hidden (ReplaceFileW keeps the old attributes otherwise).
        // SAFETY: valid path.
        let _ = unsafe { SetFileAttributesW(&t, FILE_ATTRIBUTE_NORMAL) };
    }
    Ok(())
}
