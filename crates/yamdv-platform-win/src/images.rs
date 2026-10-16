//! Images end to end (R-IMG-01…06, P-13, P-27): the Windows [`Fs`] layer behind
//! `image::resolve_policy`, the header probe, the decode worker (E.10: 256 KiB stack,
//! cancellable, exits after 5 s idle) with WIC decoding at display size in device pixels, and the
//! decoded-image store with the P-13 LRU.

#![allow(unsafe_code)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat32bppPBGRA, IWICBitmapDecoder, IWICImagingFactory,
    WICBitmapDitherTypeNone, WICBitmapInterpolationModeHighQualityCubic,
    WICBitmapPaletteTypeCustom, WICDecodeMetadataCacheOnDemand,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    GetDriveTypeW, GetFileAttributesExW, GetFileExInfoStandard, OPEN_EXISTING, ReadFile,
    WIN32_FILE_ATTRIBUTE_DATA,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};
use windows::core::HSTRING;
use yamdv_core::image::{self, DriveKind, Fs, Placeholder, Source};
use yamdv_layout::img::{ImageHost, ImgState};

/// Window message posted when a decode finished (`wParam` = generation).
pub const WM_IMAGE_DONE: u32 = WM_APP + 0x31;
/// Worker stack size (E.10).
pub const WORKER_STACK: usize = 256 * 1024;
/// Worker idle exit.
pub const IDLE_EXIT: Duration = Duration::from_secs(5);
const FSCTL_GET_REPARSE_POINT: u32 = 0x0009_00A8;
const TAG_MOUNT_POINT: u32 = 0xA000_0003;
const TAG_SYMLINK: u32 = 0xA000_000C;

/// The Windows file-system layer of the S-RES-02 policy.
#[derive(Debug, Default)]
pub struct WinFs;

fn attrs(path: &str) -> Option<WIN32_FILE_ATTRIBUTE_DATA> {
    let mut a = WIN32_FILE_ATTRIBUTE_DATA::default();
    // SAFETY: `a` is a valid out-buffer of the requested info level.
    unsafe {
        GetFileAttributesExW(
            &HSTRING::from(path),
            GetFileExInfoStandard,
            (&raw mut a).cast(),
        )
    }
    .ok()?;
    Some(a)
}

fn u16_at(b: &[u8], o: usize) -> usize {
    b.get(o..o + 2)
        .and_then(|s| <[u8; 2]>::try_from(s).ok())
        .map_or(0, |s| usize::from(u16::from_le_bytes(s)))
}

/// Substitute name of a symlink / mount-point reparse buffer (`""` for other tags).
pub fn parse_reparse(b: &[u8]) -> String {
    let tag = b
        .get(0..4)
        .and_then(|s| <[u8; 4]>::try_from(s).ok())
        .map_or(0, u32::from_le_bytes);
    let base = match tag {
        TAG_SYMLINK => 20,
        TAG_MOUNT_POINT => 16,
        _ => return String::new(),
    };
    let (off, len) = (u16_at(b, 8), u16_at(b, 10));
    let Some(raw) = b.get(base + off..base + off + len) else {
        return String::new();
    };
    let w: Vec<u16> = raw
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    String::from_utf16_lossy(&w)
}

impl Fs for WinFs {
    fn drive_kind(&mut self, root: &str) -> DriveKind {
        // SAFETY: valid NUL-terminated string.
        match unsafe { GetDriveTypeW(&HSTRING::from(root)) } {
            4 => DriveKind::Remote,
            2 | 3 | 5 | 6 => DriveKind::Local,
            _ => DriveKind::Unknown,
        }
    }
    fn reparse_target(&mut self, path: &str) -> Option<String> {
        let a = attrs(path)?;
        if a.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 == 0 {
            return None;
        }
        // SAFETY: opens the reparse point itself (never its target); handle closed below.
        let h = unsafe {
            CreateFileW(
                &HSTRING::from(path),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
                None,
            )
        }
        .ok()?;
        let mut buf = vec![0u8; 16 * 1024];
        let mut got = 0u32;
        // SAFETY: `buf` is a writable buffer of the given size.
        let ok = unsafe {
            DeviceIoControl(
                h,
                FSCTL_GET_REPARSE_POINT,
                None,
                0,
                Some(buf.as_mut_ptr().cast()),
                u32::try_from(buf.len()).unwrap_or(0),
                Some(&raw mut got),
                None,
            )
        }
        .is_ok();
        // SAFETY: handle from CreateFileW.
        let _ = unsafe { CloseHandle(h) };
        if !ok {
            return Some(String::new());
        }
        buf.truncate(got as usize);
        let t = parse_reparse(&buf);
        Some(t.strip_prefix("\\??\\").map_or(t.clone(), |r| {
            if r.to_ascii_lowercase().starts_with("unc\\") {
                t.clone()
            } else {
                r.to_string()
            }
        }))
    }
    fn size(&mut self, path: &str) -> Option<u64> {
        let a = attrs(path)?;
        if a.dwFileAttributes & 0x10 != 0 {
            return None;
        }
        Some((u64::from(a.nFileSizeHigh) << 32) | u64::from(a.nFileSizeLow))
    }
}

/// Reads at most `n` bytes from the start of a policy-checked local file.
fn read_head(path: &str, n: usize) -> Option<Vec<u8>> {
    // SAFETY: read-only open of an already policy-checked local path; closed below.
    let h = unsafe {
        CreateFileW(
            &HSTRING::from(path),
            GENERIC_READ.0,
            FILE_SHARE_READ,
            None,
            OPEN_EXISTING,
            Default::default(),
            None,
        )
    }
    .ok()?;
    let mut buf = vec![0u8; n];
    let mut got = 0u32;
    // SAFETY: valid handle and buffer.
    let r = unsafe { ReadFile(h, Some(&mut buf), Some(&raw mut got), None) };
    // SAFETY: handle from CreateFileW.
    let _ = unsafe { CloseHandle(h) };
    r.ok()?;
    buf.truncate(got as usize);
    Some(buf)
}

/// Decoded premultiplied BGRA pixels at device size.
#[derive(Debug, Clone)]
pub struct Bitmap {
    /// Width (device px).
    pub w: u32,
    /// Height.
    pub h: u32,
    /// `w * h * 4` bytes.
    pub px: Vec<u8>,
}

/// WIC decode of `src` scaled (high-quality cubic) to `w × h` device pixels.
pub fn decode(f: &IWICImagingFactory, src: &Source, w: u32, h: u32) -> Result<Bitmap, Placeholder> {
    let w = w.clamp(1, image::MAX_DIM);
    let h = h.clamp(1, image::MAX_DIM);
    // SAFETY: COM initialized on this thread; all objects released by Drop; the buffer size is
    // `w*h*4` and the stride `w*4`.
    unsafe {
        let dec: IWICBitmapDecoder = match src {
            Source::File(p) => f
                .CreateDecoderFromFilename(
                    &HSTRING::from(p.as_str()),
                    None,
                    GENERIC_READ,
                    WICDecodeMetadataCacheOnDemand,
                )
                .map_err(|_| Placeholder::Unsupported)?,
            Source::Data(d) => {
                let s = f.CreateStream().map_err(|_| Placeholder::Invalid)?;
                s.InitializeFromMemory(d)
                    .map_err(|_| Placeholder::Invalid)?;
                f.CreateDecoderFromStream(&s, std::ptr::null(), WICDecodeMetadataCacheOnDemand)
                    .map_err(|_| Placeholder::Unsupported)?
            }
        };
        let frame = dec.GetFrame(0).map_err(|_| Placeholder::Unsupported)?;
        let sc = f
            .CreateBitmapScaler()
            .map_err(|_| Placeholder::Unsupported)?;
        sc.Initialize(&frame, w, h, WICBitmapInterpolationModeHighQualityCubic)
            .map_err(|_| Placeholder::Unsupported)?;
        let cv = f
            .CreateFormatConverter()
            .map_err(|_| Placeholder::Unsupported)?;
        cv.Initialize(
            &sc,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeCustom,
        )
        .map_err(|_| Placeholder::Unsupported)?;
        let mut px = vec![0u8; (w as usize) * (h as usize) * 4];
        cv.CopyPixels(std::ptr::null(), w * 4, &mut px)
            .map_err(|_| Placeholder::Unsupported)?;
        Ok(Bitmap { w, h, px })
    }
}

/// Creates a WIC factory on the current (COM-initialized) thread.
pub fn factory() -> Option<IWICImagingFactory> {
    // SAFETY: plain COM activation.
    unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) }.ok()
}

enum Work {
    /// Policy + header probe of a source as written, relative to the document.
    Probe { doc: String },
    /// Decode at `w × h` device px.
    Decode { src: Source, w: u32, h: u32 },
}

struct Job {
    key: Box<str>,
    work: Work,
    generation: u64,
}

enum Res {
    Probed(ImgState, Option<Source>),
    Decoded(u32, u32, Result<Bitmap, Placeholder>),
}

type Done = (Box<str>, u64, Res);

/// Policy decision first, then (only for allowed local/data sources) the header probe.
fn probe_job(key: &str, doc: &str) -> Res {
    match image::resolve_policy(key, doc, &mut WinFs) {
        Err(p) => Res::Probed(ImgState::Fail(p), None),
        Ok(s) => Res::Probed(Store::probe_source(&s), Some(s)),
    }
}

fn run_job(f: &IWICImagingFactory, j: &Job) -> Res {
    match &j.work {
        Work::Probe { doc } => probe_job(&j.key, doc),
        Work::Decode { src, w, h } => Res::Decoded(*w, *h, decode(f, src, *w, *h)),
    }
}

/// Decode worker handle (E.10).
struct Worker {
    tx: Sender<Job>,
}

fn spawn(hwnd: isize, done: Arc<Mutex<Vec<Done>>>) -> Option<Worker> {
    let (tx, rx): (Sender<Job>, Receiver<Job>) = channel();
    std::thread::Builder::new()
        .name("yamdv-image".into())
        .stack_size(WORKER_STACK)
        .spawn(move || {
            // SAFETY: COM for this thread only, uninitialized on exit.
            let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            if let Some(f) = factory() {
                while let Ok(j) = rx.recv_timeout(IDLE_EXIT) {
                    {
                        {
                            let r = run_job(&f, &j);
                            if let Ok(mut d) = done.lock() {
                                d.push((j.key, j.generation, r));
                            }
                            // SAFETY: posting to a window handle owned by the UI thread.
                            let _ = unsafe {
                                PostMessageW(
                                    Some(HWND(hwnd as *mut _)),
                                    WM_IMAGE_DONE,
                                    WPARAM(usize::try_from(j.generation).unwrap_or(0)),
                                    LPARAM(0),
                                )
                            };
                        }
                    }
                }
            }
            // SAFETY: balances CoInitializeEx above.
            unsafe { CoUninitialize() };
        })
        .ok()?;
    Some(Worker { tx })
}

#[derive(Debug)]
struct Entry {
    state: ImgState,
    src: Option<Source>,
    bmp: Option<Bitmap>,
    pending: Option<(u32, u32)>,
}

/// Per-document image store: policy, probe, decoded pixels and the P-13 LRU.
pub struct Store {
    doc_path: String,
    map: RefCell<HashMap<Box<str>, Entry>>,
    lru: RefCell<image::Lru>,
    ids: RefCell<HashMap<Box<str>, u64>>,
    /// Images drawn in the current frame (visible ± 1 screen keep their pixels).
    pinned: RefCell<Vec<Box<str>>>,
    hwnd: isize,
    worker: RefCell<Option<Worker>>,
    done: Arc<Mutex<Vec<Done>>>,
    generation: u64,
    /// Synchronous decode (`--render-png`, tests): no worker.
    sync: Option<IWICImagingFactory>,
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Store")
            .field("doc", &self.doc_path)
            .finish()
    }
}

impl Store {
    /// Store for the document at `doc_path`; decodes post [`WM_IMAGE_DONE`] to `hwnd`.
    pub fn new(doc_path: &str, hwnd: isize) -> Self {
        static GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let generation = GEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            doc_path: doc_path.to_string(),
            map: RefCell::default(),
            lru: RefCell::default(),
            ids: RefCell::default(),
            pinned: RefCell::default(),
            hwnd,
            worker: RefCell::new(None),
            done: Arc::default(),
            generation,
            sync: None,
        }
    }
    /// Store that decodes on the calling thread (COM must be initialized).
    pub fn synchronous(doc_path: &str) -> Self {
        let mut s = Self::new(doc_path, 0);
        s.sync = factory();
        s
    }

    /// Bytes of decoded pixels held (visible + LRU).
    pub fn decoded_bytes(&self) -> usize {
        self.map
            .borrow()
            .values()
            .filter_map(|e| e.bmp.as_ref())
            .map(|b| b.px.len())
            .sum()
    }
    /// Bytes in the LRU part.
    pub fn lru_bytes(&self) -> usize {
        self.lru.borrow().bytes()
    }

    fn id(&self, key: &str) -> u64 {
        let mut ids = self.ids.borrow_mut();
        let n = ids.len() as u64;
        *ids.entry(key.into()).or_insert(n)
    }

    /// Starts a frame: forgets the pinned set.
    pub fn begin_frame(&self) {
        self.pinned.borrow_mut().clear();
    }

    /// Keeps `key` decoded this frame (visible ± 1 screen).
    pub fn pin(&self, key: &str) {
        self.pinned.borrow_mut().push(key.into());
    }

    /// Ends a frame: every decoded image not drawn in it moves into the LRU; over-budget
    /// entries are evicted (re-decoded on demand).
    pub fn end_frame(&self) {
        let pinned = self.pinned.borrow();
        let mut map = self.map.borrow_mut();
        let keys: Vec<Box<str>> = map.keys().cloned().collect();
        for k in keys {
            let id = self.id(&k);
            let Some(e) = map.get(&k) else { continue };
            let Some(b) = &e.bmp else { continue };
            if pinned.contains(&k) {
                self.lru.borrow_mut().remove(id);
                continue;
            }
            let size = b.px.len();
            let evict = self.lru.borrow_mut().touch(id, size, image::LRU_BYTES);
            for v in evict {
                if let Some((vk, _)) = self.ids.borrow().iter().find(|(_, x)| **x == v)
                    && let Some(e) = map.get_mut(vk)
                {
                    e.bmp = None;
                }
            }
        }
    }

    /// Pixels of `key` at `w × h` device px, if decoded; otherwise queues a decode.
    pub fn pixels(&self, key: &str, w: u32, h: u32) -> Option<std::cell::Ref<'_, Bitmap>> {
        self.pinned.borrow_mut().push(key.into());
        let need = {
            let mut map = self.map.borrow_mut();
            let e = map.get_mut(key)?;
            if !matches!(e.state, ImgState::Ready { .. }) {
                return None;
            }
            let ok = e.bmp.as_ref().is_some_and(|b| b.w == w && b.h == h);
            if !ok && e.pending != Some((w, h)) {
                e.pending = Some((w, h));
                e.src.clone()
            } else {
                None
            }
        };
        if let Some(src) = need {
            let j = Job {
                key: key.into(),
                work: Work::Decode { src, w, h },
                generation: self.generation,
            };
            if let Some(f) = &self.sync {
                let r = run_job(f, &j);
                self.apply(key, r);
            } else {
                self.queue(j);
            }
        }
        let map = self.map.borrow();
        std::cell::Ref::filter_map(map, |m| {
            m.get(key)
                .and_then(|e| e.bmp.as_ref())
                .filter(|b| b.w == w && b.h == h)
        })
        .ok()
    }

    fn queue(&self, j: Job) {
        let mut w = self.worker.borrow_mut();
        let j = match w.as_ref() {
            Some(x) => match x.tx.send(j) {
                Ok(()) => return,
                Err(e) => e.0,
            },
            None => j,
        };
        // Worker exited after idling (or never started): start a new one.
        *w = spawn(self.hwnd, Arc::clone(&self.done));
        if let Some(x) = w.as_ref() {
            let _ = x.tx.send(j);
        }
    }

    /// Applies a worker result; returns whether the layout changes.
    fn apply(&self, key: &str, r: Res) -> bool {
        let mut map = self.map.borrow_mut();
        let Some(e) = map.get_mut(key) else {
            return false;
        };
        let (w, h, r) = match r {
            Res::Probed(st, src) => {
                let changed = st != e.state;
                e.state = st;
                e.src = src;
                return changed;
            }
            Res::Decoded(w, h, r) => (w, h, r),
        };
        if e.pending == Some((w, h)) {
            e.pending = None;
        }
        match r {
            Ok(b) => {
                e.bmp = Some(b);
                false
            }
            Err(p) => {
                e.state = ImgState::Fail(p);
                e.bmp = None;
                true
            }
        }
    }

    /// Collects finished decodes; returns (any finished, any layout change).
    pub fn collect(&self) -> (bool, bool) {
        let done: Vec<Done> = self
            .done
            .lock()
            .map(|mut d| std::mem::take(&mut *d))
            .unwrap_or_default();
        let mut relayout = false;
        let any = !done.is_empty();
        for (k, generation, r) in done {
            if generation == self.generation {
                relayout |= self.apply(&k, r);
            }
        }
        (any, relayout)
    }

    pub(crate) fn probe_source(src: &Source) -> ImgState {
        let head = match src {
            Source::Data(d) => Some(d.get(..d.len().min(65_536)).unwrap_or(&[]).to_vec()),
            Source::File(p) => read_head(p, 65_536),
        };
        let Some(head) = head else {
            return ImgState::Fail(Placeholder::Missing);
        };
        match image::probe(&head) {
            None => ImgState::Fail(Placeholder::Unsupported),
            Some(p) if p.format == image::Format::Svg => ImgState::Fail(Placeholder::Unsupported),
            Some(p) if !image::within_limits(&p) => ImgState::Fail(Placeholder::TooLarge),
            Some(p) if p.w == 0 || p.h == 0 => ImgState::Pending,
            Some(p) => ImgState::Ready { w: p.w, h: p.h },
        }
    }
}

impl ImageHost for Store {
    fn state(&self, src: &str) -> ImgState {
        if let Some(e) = self.map.borrow().get(src) {
            return e.state.clone();
        }
        // Lexical refusals need no I/O: decided here (no layout jump for remote badges etc.).
        if let Err(p) = image::resolve(src, &self.doc_path) {
            let st = ImgState::Fail(p);
            self.map.borrow_mut().insert(
                src.into(),
                Entry {
                    state: st.clone(),
                    src: None,
                    bmp: None,
                    pending: None,
                },
            );
            return st;
        }
        self.map.borrow_mut().insert(
            src.into(),
            Entry {
                state: ImgState::Pending,
                src: None,
                bmp: None,
                pending: None,
            },
        );
        // File-system policy checks and the header probe run on the worker (E.10); the
        // estimate is replaced with scroll anchoring when the result arrives.
        let j = Job {
            key: src.into(),
            work: Work::Probe {
                doc: self.doc_path.clone(),
            },
            generation: self.generation,
        };
        if let Some(f) = &self.sync {
            let r = run_job(f, &j);
            self.apply(src, r);
            return self
                .map
                .borrow()
                .get(src)
                .map_or(ImgState::Pending, |e| e.state.clone());
        }
        self.queue(j);
        ImgState::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reparse_symlink_and_junction() {
        let name: Vec<u8> = "\\??\\UNC\\evil\\s"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let mut b = TAG_SYMLINK.to_le_bytes().to_vec();
        b.extend([0, 0, 0, 0]);
        b.extend(0u16.to_le_bytes());
        b.extend(u16::try_from(name.len()).unwrap_or(0).to_le_bytes());
        b.extend([0; 8]);
        b.extend(&name);
        assert_eq!(parse_reparse(&b), "\\??\\UNC\\evil\\s");
        let mut j = TAG_MOUNT_POINT.to_le_bytes().to_vec();
        j.extend([0, 0, 0, 0, 0, 0]);
        j.extend(u16::try_from(name.len()).unwrap_or(0).to_le_bytes());
        j.extend([0; 4]);
        j.extend(&name);
        assert_eq!(parse_reparse(&j), "\\??\\UNC\\evil\\s");
        assert_eq!(parse_reparse(&[1, 0, 0, 0]), "");
    }

    /// Windows twin of the recording-mock test: blocked forms are refused by the lexical
    /// policy before `WinFs` is ever consulted.
    #[test]
    fn s_res_02_blocked_before_fs_on_windows() {
        for s in [
            "\\\\server\\share\\a.png",
            "\\\\?\\C:\\a.png",
            "a.png:Zone.Identifier",
            "https://x/a.png",
            "CON",
        ] {
            assert!(image::resolve(s, "C:\\d\\x.md").is_err(), "{s}");
            assert!(
                image::resolve_policy(s, "C:\\d\\x.md", &mut WinFs).is_err(),
                "{s}"
            );
        }
    }

    #[test]
    fn decodes_png_data_uri_at_display_size() {
        // SAFETY: test thread COM init.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let png = image::base64("iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAFklEQVR4nGP8z8Dwn4GBgYGJAQoAAAwRAQQ+ohzBAAAAAElFTkSuQmCC").unwrap_or_default();
        let st = Store::synchronous("C:\\d\\x.md");
        let src = format!(
            "data:image/png;base64,{}",
            "iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAFklEQVR4nGP8z8Dwn4GBgYGJAQoAAAwRAQQ+ohzBAAAAAElFTkSuQmCC"
        );
        assert!(!png.is_empty());
        assert_eq!(st.state(&src), ImgState::Ready { w: 2, h: 2 });
        let b = st.pixels(&src, 8, 8).map(|b| b.px.len());
        assert_eq!(b, Some(256));
    }
}
