//! Directory watcher (R-READ-01, ARCHITECTURE `watch`, F-26): ONE process-wide thread waits on
//! overlapped `ReadDirectoryChangesW` reads for every watch (document folders, `custom-theme.ini`,
//! expanded Files-tab folders). Dropping a [`Watcher`] cancels its read on that thread.
// Win32 sizes and handles: the casts are bounded (struct sizes, small buffers).
#![allow(unsafe_code)]
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, WAIT_OBJECT_0, WPARAM};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED, FILE_LIST_DIRECTORY,
    FILE_NOTIFY_CHANGE, FILE_NOTIFY_CHANGE_DIR_NAME, FILE_NOTIFY_CHANGE_FILE_NAME,
    FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING, ReadDirectoryChangesW,
};
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows::Win32::System::Threading::{
    CreateEventW, INFINITE, ResetEvent, SetEvent, WaitForMultipleObjects,
};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
use windows::core::HSTRING;

pub use crate::shell::WM_APP_FILE_CHANGED;

/// Most watches at once: `WaitForMultipleObjects` takes 64 handles, one is the control event.
/// Per window that is 32 Files-tab folders + the document folder + `custom-theme.ini`.
pub const MAX_WATCHES: usize = 63;

/// One registered watch; dropping it removes the watch (the read is cancelled on the thread).
pub struct Watcher {
    id: u64,
}

impl Drop for Watcher {
    fn drop(&mut self) {
        if let Some(s) = service() {
            s.live.fetch_sub(1, Ordering::Relaxed);
        }
        command(Cmd::Remove(self.id));
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
    add(format!("{dir}\\"), name.to_lowercase(), hwnd, tag)
}

/// Watches folder `dir` itself (Files tab, P-43): posts [`WM_APP_FILE_CHANGED`] with `tag` on
/// any file or folder name change inside it (not recursive).
pub fn watch_dir(dir: &str, hwnd: HWND, tag: usize) -> Option<Watcher> {
    add(
        format!("{}\\", dir.trim_end_matches('\\')),
        String::new(),
        hwnd,
        tag,
    )
}

/// Live watches (for tests and the F-26 thread-count probe).
pub fn active() -> usize {
    service().map_or(0, |s| s.live.load(Ordering::Relaxed) as usize)
}

struct Spec {
    id: u64,
    dir: String,
    name: String,
    hwnd: isize,
    tag: usize,
}

enum Cmd {
    Add(Spec),
    Remove(u64),
}

struct Service {
    ctrl: isize,
    queue: Mutex<Vec<Cmd>>,
    next: AtomicU64,
    live: AtomicU64,
}

static SERVICE: OnceLock<Option<Service>> = OnceLock::new();

fn service() -> Option<&'static Service> {
    SERVICE
        .get_or_init(|| {
            // SAFETY: unnamed auto-reset event, kept for the process lifetime.
            let ev = unsafe { CreateEventW(None, false, false, None) }.ok()?;
            let ctrl = ev.0 as isize;
            std::thread::Builder::new()
                .name("yamdv-watch".into())
                .stack_size(64 * 1024)
                .spawn(move || run(ctrl))
                .ok()?;
            Some(Service {
                ctrl,
                queue: Mutex::new(Vec::new()),
                next: AtomicU64::new(1),
                live: AtomicU64::new(0),
            })
        })
        .as_ref()
}

fn command(c: Cmd) {
    let Some(s) = service() else { return };
    if let Ok(mut q) = s.queue.lock() {
        q.push(c);
    }
    // SAFETY: the control event lives for the process lifetime.
    let _ = unsafe { SetEvent(handle(s.ctrl)) };
}

fn add(dir: String, name: String, hwnd: HWND, tag: usize) -> Option<Watcher> {
    let s = service()?;
    if s.live.load(Ordering::Relaxed) as usize >= MAX_WATCHES {
        return None;
    }
    let id = s.next.fetch_add(1, Ordering::Relaxed);
    s.live.fetch_add(1, Ordering::Relaxed);
    let hwnd = hwnd.0 as isize;
    command(Cmd::Add(Spec {
        id,
        dir,
        name,
        hwnd,
        tag,
    }));
    Some(Watcher { id })
}

fn handle(h: isize) -> HANDLE {
    HANDLE(std::ptr::without_provenance_mut(h.cast_unsigned()))
}

/// One open directory with its pending overlapped read. Boxed so the `OVERLAPPED` and the buffer
/// keep their addresses while the kernel owns them.
struct Entry {
    id: u64,
    dir: HANDLE,
    ov: Box<OVERLAPPED>,
    buf: Box<[u32]>,
    name: String,
    hwnd: isize,
    tag: usize,
}

impl Entry {
    fn open(s: Spec) -> Option<Entry> {
        let d = HSTRING::from(yamdv_core::path::long_form(&s.dir));
        // SAFETY: valid path; the handle is closed in `close`.
        let dir = unsafe {
            CreateFileW(
                &d,
                FILE_LIST_DIRECTORY.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
                None,
            )
        }
        .ok()?;
        // SAFETY: unnamed manual-reset event, closed in `close`.
        let Ok(ev) = (unsafe { CreateEventW(None, true, false, None) }) else {
            // SAFETY: opened above.
            let _ = unsafe { CloseHandle(dir) };
            return None;
        };
        let mut ov = Box::new(OVERLAPPED::default());
        ov.hEvent = ev;
        // Document watches see few names; folder watches list renames: 4 KiB is plenty, and an
        // overflow (0 bytes) is reported as "changed" anyway.
        let mut e = Entry {
            id: s.id,
            dir,
            ov,
            buf: vec![0u32; 1024].into_boxed_slice(),
            name: s.name,
            hwnd: s.hwnd,
            tag: s.tag,
        };
        if e.arm() {
            Some(e)
        } else {
            e.close();
            None
        }
    }

    fn filter(&self) -> FILE_NOTIFY_CHANGE {
        if self.name.is_empty() {
            FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_DIR_NAME
        } else {
            FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_LAST_WRITE | FILE_NOTIFY_CHANGE_SIZE
        }
    }

    /// Starts the next overlapped read; `false` when the folder is gone.
    fn arm(&mut self) -> bool {
        // SAFETY: our own event.
        let _ = unsafe { ResetEvent(self.ov.hEvent) };
        let filter = self.filter();
        // SAFETY: handle opened with FILE_FLAG_OVERLAPPED; the boxed buffer and OVERLAPPED stay
        // alive and unmoved until the read completes or `close` waited for its cancellation.
        unsafe {
            ReadDirectoryChangesW(
                self.dir,
                self.buf.as_mut_ptr().cast(),
                (self.buf.len() * 4) as u32,
                false,
                filter,
                None,
                Some(&raw mut *self.ov),
                None,
            )
        }
        .is_ok()
    }

    /// The read completed: posts the change when it matters, then re-arms. `false` = drop me.
    fn complete(&mut self) -> bool {
        let mut got = 0u32;
        // SAFETY: the event is signalled, so the read has completed; no wait needed.
        let ok = unsafe { GetOverlappedResult(self.dir, &raw const *self.ov, &mut got, false) };
        if ok.is_err() {
            return false;
        }
        // got == 0: buffer overflow, treat as "maybe changed".
        let hit = got == 0
            || self.name.is_empty()
            || notify_names(&self.buf, got as usize)
                .iter()
                .any(|n| n.to_lowercase() == self.name);
        if hit {
            // SAFETY: posting to a window handle is safe even if it was destroyed meanwhile.
            unsafe {
                let _ = PostMessageW(
                    Some(HWND(std::ptr::without_provenance_mut(
                        self.hwnd.cast_unsigned(),
                    ))),
                    WM_APP_FILE_CHANGED,
                    WPARAM(self.tag),
                    LPARAM(0),
                );
            }
        }
        self.arm()
    }

    /// Cancels the pending read, waits until the kernel let go of the buffer, closes handles.
    fn close(&mut self) {
        let mut n = 0u32;
        // SAFETY: our handle and OVERLAPPED; waiting on the result guarantees the read is done
        // (completed or cancelled) before the box is freed. Failing calls mean no read pending.
        unsafe {
            if CancelIoEx(self.dir, Some(&raw const *self.ov)).is_ok() {
                let _ = GetOverlappedResult(self.dir, &raw const *self.ov, &mut n, true);
            }
            let _ = CloseHandle(self.dir);
            let _ = CloseHandle(self.ov.hEvent);
        }
    }
}

/// The watcher thread: waits on the control event plus one event per watch.
fn run(ctrl: isize) {
    let mut entries: Vec<Entry> = Vec::new();
    loop {
        let mut hs = Vec::with_capacity(entries.len() + 1);
        hs.push(handle(ctrl));
        hs.extend(entries.iter().map(|e| e.ov.hEvent));
        // SAFETY: every handle is a live event owned by this thread (or the control event).
        let r = unsafe { WaitForMultipleObjects(&hs, false, INFINITE) };
        let i = r.0.wrapping_sub(WAIT_OBJECT_0.0) as usize;
        if i == 0 {
            let cmds = service()
                .and_then(|s| s.queue.lock().ok().map(|mut q| std::mem::take(&mut *q)))
                .unwrap_or_default();
            for c in cmds {
                match c {
                    Cmd::Add(spec) => {
                        if entries.len() < MAX_WATCHES
                            && let Some(e) = Entry::open(spec)
                        {
                            entries.push(e);
                        }
                    }
                    Cmd::Remove(id) => {
                        if let Some(k) = entries.iter().position(|e| e.id == id) {
                            entries.swap_remove(k).close();
                        }
                    }
                }
            }
        } else if let Some(e) = entries.get_mut(i - 1) {
            if !e.complete() {
                entries.swap_remove(i - 1).close();
            }
        } else {
            // WAIT_FAILED or an abandoned handle: should not happen; avoid a busy loop.
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
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
