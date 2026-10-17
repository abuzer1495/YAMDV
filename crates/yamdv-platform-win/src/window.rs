//! Reading windows (Phase 002): one top-level window per document in the same process, backend A
//! rendering, virtualized scrolling, selection/copy, open (CLI / `Ctrl+O` / drag & drop), guards
//! with the in-window confirm dialog, empty and error states. No timers run while idle (T7).
//! PHASE-004: custom title bar replaces the temporary system caption.
#![allow(unsafe_code)]
// Pixel/DIP geometry: float<->int conversions are bounded by window/bitmap sizes (ADR-0007).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::indexing_slicing
)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;
use std::time::Instant;

use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, GetMonitorInfoW, InvalidateRect, MONITOR_DEFAULTTONEAREST, MONITORINFO,
    MonitorFromPoint, PAINTSTRUCT, ScreenToClient, UpdateWindow,
};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    AdjustWindowRectExForDpi, GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
    VK_CONTROL, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_NEXT, VK_PRIOR, VK_RETURN, VK_SHIFT,
    VK_SPACE, VK_TAB, VK_UP,
};
use windows::Win32::UI::Shell::{DragAcceptFiles, DragFinish, DragQueryFileW, HDROP};
/// `WM_MOUSELEAVE` (lives in UI::Controls in windows-rs; avoided pulling that feature).
const WM_MOUSELEAVE: u32 = 0x02A3;
use windows::Win32::Foundation::WAIT_OBJECT_0;
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::Threading::INFINITE;
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_LEFT, VK_RIGHT};
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetClientRect, GetCursorPos, GetMessageTime, GetSystemMetrics, HICON, HTCLIENT, IDC_ARROW,
    IDC_IBEAM, IMAGE_ICON, KillTimer, LR_DEFAULTSIZE, LR_SHARED, LoadCursorW, LoadImageW,
    MINMAXINFO, MSG, PostMessageW, PostQuitMessage, RegisterClassExW, SM_CXDOUBLECLK,
    SM_CYDOUBLECLK, SW_INVALIDATE, SW_SHOW, SWP_NOACTIVATE, SWP_NOZORDER, ScrollWindowEx,
    SetCursor, SetPropW, SetTimer, SetWindowPos, SetWindowTextW, ShowWindow, TranslateMessage,
    WINDOW_EX_STYLE, WM_APP, WM_COMMAND, WM_DESTROY, WM_DPICHANGED, WM_DROPFILES, WM_ERASEBKGND,
    WM_GETMINMAXINFO, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_PAINT, WM_SETCURSOR, WM_SETTINGCHANGE, WM_SIZE, WM_TIMER, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    IDC_HAND, SW_SHOWMAXIMIZED, WM_CHAR, WM_CONTEXTMENU, WM_DRAWITEM, WM_MBUTTONUP, WM_MEASUREITEM,
    WM_MENUCHAR, WM_SYSKEYDOWN, WM_XBUTTONUP,
};

mod app;
#[cfg(feature = "dev-tools")]
mod driver;
mod findbar;
use windows::Win32::UI::WindowsAndMessaging::{
    MWMO_INPUTAVAILABLE, MsgWaitForMultipleObjectsEx, PM_REMOVE, PeekMessageW, QS_ALLINPUT,
    SetForegroundWindow, WM_COPYDATA, WM_MOUSEHWHEEL, WM_QUIT, WaitMessage,
};
use windows::core::{HSTRING, PCWSTR, w};
use yamdv_core::decode::{self, Guard};
use yamdv_core::doc::detail::{Alert, Placeholder};
use yamdv_core::doc::{Document, ReadOpts};
use yamdv_core::i18n::{Lang, Msg, tr};
use yamdv_core::link as ylink;
use yamdv_core::path as ypath;
use yamdv_core::settings::Settings;
use yamdv_core::theme::{self, Palette};
use yamdv_layout::Rect;
use yamdv_layout::block::{Bullet, HotKind, Item, Metrics, Strings};
use yamdv_layout::shaper::{Family, ParaSpec, Style, TextShaper};
use yamdv_layout::viewer::{self, Content, Env, FootTarget, Hit, Viewer};
use yamdv_ui::dialog::{self as dlg, Button, ConfirmDialog, Geometry, Outcome};

use crate::dwrite::{DWrite, DwShaper};
use crate::render::{Band, Renderer};
use crate::sys::{self, FileInfo};
use crate::theme::{Mode, app_mode};

/// Window class name.
pub const CLASS_NAME: PCWSTR = w!("YAMDV.Window");
/// Initial client size in logical px.
pub const INITIAL_CLIENT: (i32, i32) = (1100, 760);
/// Minimum client size in logical px.
pub const MIN_CLIENT: (i32, i32) = (480, 360);
/// `WM_COMMAND` id of "Open…" (also used by `xtask memprobe` for M6).
pub const IDM_OPEN: usize = 0x101;
/// Window property holding the first-paint time in ms + 1 (read by `xtask memprobe`).
pub const PROP_FIRST_PAINT: PCWSTR = w!("YAMDV.FirstPaintMs");
/// Window property: current band buffer bytes + 1 (read by memprobe, F-12).
pub const PROP_BAND_BYTES: PCWSTR = w!("YAMDV.BandBytes");

const WM_APP_AUTOSCROLL: u32 = WM_APP + 1;
/// One font warm-up task per message (F-10).
const WM_APP_WARM: u32 = WM_APP + 2;
const TIMER_ANIM: usize = 1;
const TIMER_SB_HIDE: usize = 2;
const TIMER_SB_FADE: usize = 3;
const TIMER_DRAG: usize = 4;
const TIMER_MEMSTATS: usize = 6;
const TIMER_TOAST: usize = 7;
const TIMER_RELOAD: usize = 10;
const TIMER_COPIED: usize = 8;
#[cfg(feature = "dev-tools")]
const TIMER_SCRIPT: usize = 9;
const LINE_STEP: f64 = 40.0;
/// `MK_CONTROL` in mouse-message `wParam`.
const MK_CONTROL_BIT: usize = 0x0008;

/// Developer options (feature `dev-tools`; never in release builds).
#[derive(Debug, Clone, Default)]
pub struct DevOptions {
    /// Scroll top → bottom → top after the first paint, then set the title to `SCROLL_DONE`.
    pub autoscroll: bool,
    /// Write per-frame paint durations (ms) here.
    pub perf_log: Option<String>,
    /// Write the subsystem memory breakdown (JSON) here 2 s after the first paint (`mem-stats`).
    pub mem_stats: Option<String>,
    /// In-window test driver script (§4.5); a log is written next to it, exit code 0 = pass.
    pub script: Option<String>,
}

/// Process-wide state shared by all windows (single UI thread).
struct Shared {
    dw: Rc<DWrite>,
    /// `appearance.scheme`.
    scheme: String,
    /// `appearance.mode` (system / light / dark).
    mode: String,
    /// Font families the `DWrite` instance was built with.
    fonts: (String, String),
    /// `general.restore_reading_position`.
    restore_pos: bool,
    lang: Lang,
    m: Metrics,
    read: ReadOpts,
    column_pct: f32,
    smooth: bool,
    autohide: bool,
    dev: DevOptions,
    /// `general.open_external_links`.
    external: String,
    #[cfg_attr(not(feature = "mem-stats"), allow(dead_code))]
    commit_at_start: u64,
}

thread_local! {
    static SHARED: RefCell<Option<Rc<Shared>>> = const { RefCell::new(None) };
    static WINDOWS: RefCell<HashMap<isize, Rc<RefCell<Win>>>> = RefCell::new(HashMap::new());
    static PENDING_OPEN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// The dialog host this UI thread waits for (F-09); at most one at a time.
    static HOST: RefCell<Option<crate::dialog_host::HostWait>> = const { RefCell::new(None) };
}

fn shared() -> Option<Rc<Shared>> {
    SHARED.with(|s| s.borrow().clone())
}

/// Localized layout strings.
pub(crate) struct I18nStrings(Lang);

/// Layout strings for `lang`.
#[cfg_attr(not(feature = "dev-tools"), allow(dead_code))]
pub(crate) fn strings(lang: Lang) -> I18nStrings {
    I18nStrings(lang)
}

impl Strings for I18nStrings {
    fn alert(&self, a: Alert) -> &str {
        tr(
            self.0,
            match a {
                Alert::Note => Msg::CalloutNote,
                Alert::Tip => Msg::CalloutTip,
                Alert::Important => Msg::CalloutImportant,
                Alert::Warning => Msg::CalloutWarning,
                Alert::Caution => Msg::CalloutCaution,
            },
        )
    }
    fn placeholder(&self, p: Placeholder) -> &str {
        tr(
            self.0,
            match p {
                Placeholder::Table => Msg::LabelTable,
                Placeholder::Html => Msg::LabelHtml,
                Placeholder::Math => Msg::LabelMath,
                Placeholder::Footnotes => Msg::LabelFootnotes,
                Placeholder::FrontMatter => Msg::LabelFrontMatter,
                Placeholder::Details => Msg::LabelDetails,
            },
        )
    }
    fn front_matter_summary(&self, n: usize) -> String {
        if n == 1 {
            tr(self.0, Msg::FrontMatterOne).to_string()
        } else {
            tr(self.0, Msg::FrontMatterN).replace("{}", &n.to_string())
        }
    }
}

/// Scale a logical length to device pixels for `dpi`.
pub fn scale(v: i32, dpi: u32) -> i32 {
    let dpi = i32::try_from(dpi).unwrap_or(96);
    (v * dpi + 48) / 96
}

/// Column geometry (B.6.1): `clamp(pct × client, min(360, avail − 48), avail − 48)`, centered.
pub fn column(client_w: f32, pct: f32) -> (f32, f32) {
    let avail = client_w;
    let max = (avail - 48.0).max(1.0);
    let min = 360.0f32.min(max);
    let w = (pct * client_w).clamp(min, max).floor();
    (((avail - w) / 2.0).floor(), w)
}

/// Ease-out cubic.
pub fn ease_out(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

#[derive(Debug, Clone, Copy)]
struct Anim {
    from: f64,
    to: f64,
    start: Instant,
    ms: f64,
}

enum View {
    Empty { note: Option<Msg> },
    Error { reason: Msg, path: String },
    Doc(Box<DocView>),
}

struct DocView {
    content: Content,
    viewer: Viewer,
    /// Full path of the document (link resolution, history).
    path: String,
    /// Mark-of-the-Web `ZoneId` ≥ 3 (S-LINK-04).
    internet: bool,
    #[allow(dead_code)]
    info: FileInfo,
    #[allow(dead_code)]
    encoding: &'static str,
    /// Images of this document (R-IMG-*).
    images: crate::images::Store,
    /// Built-in document (help); no file, history, watcher or position.
    virt: bool,
}

#[derive(Debug, Clone)]
enum Pending {
    Large {
        path: String,
    },
    Binary {
        path: String,
        plain: bool,
    },
    /// External URL awaiting the Ask decision (R-LINK-04).
    Url {
        url: String,
    },
    /// Allowlisted local file awaiting confirmation (S-LINK-03).
    Local {
        path: String,
    },
    /// Folder / blocked file: show in Explorer (selects, never runs).
    Reveal {
        path: String,
    },
    /// Information only (About).
    Info,
}

const BTN_CANCEL: u32 = 0;
const BTN_PLAIN: u32 = 1;
const BTN_RENDER: u32 = 2;
const BTN_OPEN: u32 = 3;
const BTN_COPY: u32 = 4;
const BTN_REVEAL: u32 = 5;

#[derive(Default)]
struct Scrollbar {
    alpha: f32,
    hover: bool,
    drag: Option<f64>,
    fade_start: Option<Instant>,
}

struct Win {
    hwnd: HWND,
    scale: f32,
    dark: bool,
    pal: Palette,
    cw: f32,
    ch: f32,
    view: View,
    sel: Option<(u32, u32)>,
    selecting: bool,
    clicks: (u32, i32, i32, i32),
    anim: Option<Anim>,
    sb: Scrollbar,
    tracking: bool,
    dialog: Option<(ConfirmDialog, Pending)>,
    renderer: Option<Renderer>,
    first_paint: bool,
    warm: Vec<yamdv_layout::warm::WarmTask>,
    warm_pending: bool,
    toast: Option<Msg>,
    /// Dynamic suffix of the toast text (link target names).
    toast_arg: String,
    /// Navigation history (path, scroll) and position (R-LINK-02).
    history: Vec<(String, f64)>,
    hist_pos: usize,
    /// Hovered code block (block, node) — its toolbar is shown (B.6.7).
    code_hover: Option<(usize, usize)>,
    /// Code block whose copy button shows the check mark (1.2 s).
    code_copied: Option<(usize, usize)>,
    /// Code scrollbar drag: (block, region, grab x, offset at grab).
    hdrag: Option<(usize, u16, f32, f32)>,
    drag_dir: f64,
    perf: Vec<f64>,
    auto_phase: u8,
    /// Win32 calls that dispatch messages synchronously, requested by a handler and run by
    /// `wndproc` once the borrow is released (P-18).
    deferred: Vec<Deferred>,
    auto_steps: u32,
    /// Keyboard focus on link `(block, link)` (B.16, Tab order).
    focus: Option<(usize, u32)>,
    /// Actions requested by methods (new windows), drained by `handle`.
    actions: Vec<Action>,
    /// The file was deleted: the document is shown dimmed (R-OPEN-11).
    dimmed: bool,
    /// Directory watcher of the open file (R-READ-01).
    watcher: Option<crate::shell::Watcher>,
    /// Find bar state (B.9).
    find: Option<findbar::Find>,
}

enum Action {
    OpenDialog,
    /// Context menu at a screen point, with the link under the cursor (B.7.1).
    ContextMenu(i32, i32, Option<String>),
    /// A menu command run outside the window borrow.
    Command(u32),
    /// F1 help window.
    Help,
    /// Body font dialog (dialog host).
    FontDialog,
    /// Paths forwarded by a second instance (S-IPC-03: open only), with `--line`.
    Forward(Vec<String>, u64),
    /// Validated dialog-host answer (S-IPC-05).
    HostResult(yamdv_core::ipc::Kind, yamdv_core::ipc::Outcome),
    NewWindow(String),
    Close,
}

fn rgb_text(dark: bool) -> u32 {
    if dark { 0xffffff } else { 0x1f1f1f }
}

impl Win {
    fn view_h(&self) -> f64 {
        f64::from(self.ch)
    }

    fn col(&self, sh: &Shared) -> (f32, f32) {
        column(self.cw, sh.column_pct)
    }

    fn invalidate(&self) {
        // SAFETY: valid hwnd.
        let _ = unsafe { InvalidateRect(Some(self.hwnd), None, false) };
    }

    fn set_title(&mut self, s: &str) {
        self.defer(Deferred::Title(s.to_string()));
    }

    fn defer(&mut self, d: Deferred) {
        if !self.deferred.contains(&d) {
            self.deferred.push(d);
        }
    }

    fn reduced_motion(&self) -> bool {
        !sys::animations_enabled()
    }

    // ---- opening -------------------------------------------------------------------------

    fn show_error(&mut self, reason: Msg, path: &str) {
        self.view = View::Error {
            reason,
            path: ypath::display_form(path),
        };
        self.sel = None;
        self.set_title("YAMDV");
        self.invalidate();
    }

    fn open(&mut self, sh: &Shared, path: &str, force: Option<bool>, allow_binary: bool) {
        self.open_inner(sh, path, force, allow_binary, true);
    }

    /// Opens `path`; `record` = a user open (history, position restore, watcher).
    fn open_inner(
        &mut self,
        sh: &Shared,
        path: &str,
        force: Option<bool>,
        allow_binary: bool,
        record: bool,
    ) {
        if record {
            self.save_position();
        }
        let Some(full) = sys::full_path(path) else {
            self.show_error(Msg::InvalidPath, path);
            return;
        };
        if ypath::validate(&full).is_err() {
            self.show_error(Msg::InvalidPath, &full);
            return;
        }
        let long = ypath::long_form(&full);
        let (info, is_dir) = match sys::stat(&long) {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.show_error(Msg::NotFound, &full);
                return;
            }
            Err(_) => {
                self.show_error(Msg::ReadError, &full);
                return;
            }
        };
        if is_dir {
            self.show_error(Msg::DropFolders, &full);
            return;
        }
        match decode::size_guard(info.size) {
            Guard::Refuse => {
                self.show_error(Msg::TooLarge, &full);
                return;
            }
            Guard::AskLarge if force.is_none() => {
                self.ask(
                    sh,
                    Msg::LargeTitle,
                    Msg::LargeBody,
                    vec![
                        (Msg::Cancel, BTN_CANCEL),
                        (Msg::RenderAnyway, BTN_RENDER),
                        (Msg::OpenPlain, BTN_PLAIN),
                    ],
                    Pending::Large { path: full },
                );
                return;
            }
            _ => {}
        }
        let bytes = match std::fs::read(&long) {
            Ok(b) => b,
            Err(_) => {
                self.show_error(Msg::ReadError, &full);
                return;
            }
        };
        if !allow_binary && decode::binary_guard(&bytes) == Guard::AskBinary {
            drop(bytes);
            self.ask(
                sh,
                Msg::BinaryTitle,
                Msg::BinaryBody,
                vec![(Msg::Cancel, BTN_CANCEL), (Msg::OpenAnyway, BTN_OPEN)],
                Pending::Binary {
                    path: full,
                    plain: force == Some(true),
                },
            );
            return;
        }
        let d = decode::decode(&bytes);
        drop(bytes);
        let plain = force == Some(true) || ypath::extension(&full) == "txt";
        // S-LINK-04: the document's own Zone.Identifier stream (a path we build ourselves).
        let internet = std::fs::read_to_string(format!("{long}:Zone.Identifier"))
            .ok()
            .and_then(|z| ylink::zone_id(&z))
            .is_some_and(|z| z >= 3);
        self.show_text(sh, full.clone(), d.text, plain, false);
        if let View::Doc(dv) = &mut self.view {
            dv.internet = internet;
            dv.info = info;
            dv.encoding = d.encoding;
        }
        if record {
            app::with(|a| a.state.add_history(&full, app::now_ms()));
            app::save_state();
            self.restore_position(sh);
            self.watcher = crate::shell::watch(&long, self.hwnd);
        }
        self.dimmed = false;
    }

    /// Shows `text` as the document `full` (a file, or a built-in document when `virt`).
    fn show_text(&mut self, sh: &Shared, full: String, text: String, plain: bool, virt: bool) {
        let (_, col_w) = self.col(sh);
        let content = if plain {
            Content::plain(text)
        } else {
            Content::Markdown(Document::new(text, sh.read))
        };
        let mut viewer = Viewer::new(&content, col_w, sh.m);
        viewer.code_default = yamdv_layout::code::CodeOpts {
            line_numbers: sh.read.code_line_numbers,
            wrap: sh.read.code_wrap,
            fold_toggled: false,
            front_matter: sh.read.front_matter,
        };
        self.warm_pending = true;
        if virt {
            self.watcher = None;
        }
        if self.history.is_empty() {
            self.history.push((full.clone(), 0.0));
            self.hist_pos = 0;
        }
        self.view = View::Doc(Box::new(DocView {
            content,
            viewer,
            path: full.clone(),
            internet: false,
            info: FileInfo::default(),
            encoding: "UTF-8",
            images: crate::images::Store::new(&full, self.hwnd.0 as isize),
            virt,
        }));
        if let Some(f) = &mut self.find {
            f.dirty = true;
        }
        self.sel = None;
        self.anim = None;
        self.set_title(&format!(
            "{} \u{2014} YAMDV",
            ypath::file_name(&ypath::display_form(&full))
        ));
        self.invalidate();
    }

    fn ask(&mut self, sh: &Shared, title: Msg, body: Msg, buttons: Vec<(Msg, u32)>, p: Pending) {
        let b = buttons
            .into_iter()
            .map(|(m, id)| Button {
                label: tr(sh.lang, m).to_string(),
                id,
            })
            .collect();
        self.dialog = Some((
            ConfirmDialog::new(tr(sh.lang, title), tr(sh.lang, body), b, BTN_CANCEL),
            p,
        ));
        self.invalidate();
    }

    fn dialog_done(&mut self, sh: &Shared, id: u32) {
        let Some((_, p)) = self.dialog.take() else {
            return;
        };
        match (p, id) {
            (_, BTN_CANCEL) => {}
            (Pending::Large { path }, BTN_PLAIN) => self.open(sh, &path, Some(true), false),
            (Pending::Large { path }, BTN_RENDER) => self.open(sh, &path, Some(false), false),
            (Pending::Binary { path, plain }, BTN_OPEN) => self.open(sh, &path, Some(plain), true),
            (Pending::Url { url }, BTN_OPEN) => sys::shell_open(&url),
            (Pending::Url { url }, BTN_COPY) => {
                if sys::copy_text(self.hwnd, &url) {
                    self.toast(Msg::Copied);
                }
            }
            (Pending::Local { path }, BTN_OPEN) => sys::shell_open(&path),
            (Pending::Reveal { path } | Pending::Local { path }, BTN_REVEAL) => {
                sys::reveal_in_explorer(&path);
            }
            (Pending::Reveal { path }, BTN_OPEN) => sys::reveal_in_explorer(&path),
            (Pending::Info, _) => {}
            _ => {}
        }
        self.invalidate();
    }

    fn ask_text(
        &mut self,
        sh: &Shared,
        title: Msg,
        body: String,
        buttons: Vec<(Msg, u32)>,
        p: Pending,
    ) {
        let b = buttons
            .into_iter()
            .map(|(m, id)| Button {
                label: tr(sh.lang, m).to_string(),
                id,
            })
            .collect();
        self.dialog = Some((
            ConfirmDialog::new(tr(sh.lang, title), body, b, BTN_CANCEL),
            p,
        ));
        self.invalidate();
    }

    /// R-LINK-01…06: activates link `link` of block `bi`.
    fn link_click(&mut self, sh: &Shared, bi: usize, link: u32, new_window: bool) {
        let Some(d) = self.doc() else { return };
        let internet = d.internet;
        let doc_path = d.path.clone();
        let Content::Markdown(doc) = &mut d.content else {
            return;
        };
        let det = doc.detail(bi);
        let Some(l) = det.links.get(link as usize) else {
            return;
        };
        let (dest, wiki) = (l.dest.to_string(), l.wiki);
        match ylink::classify(&dest, wiki) {
            ylink::Target::Anchor(a) => self.jump_anchor(sh, &a),
            ylink::Target::External(u) => {
                let url = u.serialize();
                let mode = match sh.external.as_str() {
                    "direct" => ylink::ExternalMode::Open,
                    "never" => ylink::ExternalMode::Never,
                    _ => ylink::ExternalMode::Ask,
                };
                if ylink::must_ask(mode, internet) {
                    let mut body = url.clone();
                    if u.is_idn() {
                        body.push('\n');
                        body.push_str(tr(sh.lang, Msg::LinkPunycode));
                        body.push_str(&sys::idn_to_ascii(&u.host).unwrap_or_default());
                    }
                    if u.userinfo.is_some() {
                        body.push_str("\n\n");
                        body.push_str(tr(sh.lang, Msg::LinkCredentials));
                    }
                    let mut buttons = vec![(Msg::Cancel, BTN_CANCEL), (Msg::CopyLink, BTN_COPY)];
                    if mode != ylink::ExternalMode::Never {
                        buttons.push((Msg::OpenInBrowser, BTN_OPEN));
                    }
                    self.ask_text(sh, Msg::LinkAskTitle, body, buttons, Pending::Url { url });
                } else if mode == ylink::ExternalMode::Never {
                    if sys::copy_text(self.hwnd, &url) {
                        self.toast(Msg::Copied);
                    }
                } else {
                    sys::shell_open(&url);
                }
            }
            ylink::Target::Local { path, fragment } => {
                let dir = doc_path
                    .rsplit_once(['\\', '/'])
                    .map_or(String::new(), |(d, _)| d.to_string());
                let found = if wiki {
                    sys::find_wiki(&dir, &path)
                } else {
                    let full = format!("{dir}\\{}", path.replace('/', "\\"));
                    sys::full_path(&full).filter(|f| sys::stat(&ypath::long_form(f)).is_ok())
                };
                let Some(full) = found else {
                    self.toast_with(Msg::LinkNotFound, ypath::file_name(&path));
                    return;
                };
                let is_dir = sys::stat(&ypath::long_form(&full)).is_ok_and(|(_, dir)| dir);
                if is_dir {
                    let body = ypath::display_form(&full);
                    self.ask_text(
                        sh,
                        Msg::ShowFolderTitle,
                        body,
                        vec![(Msg::Cancel, BTN_CANCEL), (Msg::ShowInExplorer, BTN_OPEN)],
                        Pending::Reveal { path: full },
                    );
                    return;
                }
                match ylink::local_kind(&full) {
                    ylink::LocalKind::Markdown if new_window => {
                        self.actions.push(Action::NewWindow(full));
                    }
                    ylink::LocalKind::Markdown => {
                        self.focus = None;
                        let scroll = self.scroll_pos();
                        self.history.truncate(self.hist_pos + 1);
                        if let Some(cur) = self.history.get_mut(self.hist_pos) {
                            cur.1 = scroll;
                        }
                        self.open(sh, &full, None, false);
                        if self.doc().is_some_and(|d| d.path == full) {
                            self.history.push((full, 0.0));
                            self.hist_pos = self.history.len() - 1;
                            if let Some(f) = fragment {
                                self.prepare_now(sh);
                                self.jump_anchor(sh, &f);
                            }
                        }
                    }
                    ylink::LocalKind::Allowlisted => {
                        let body = ypath::display_form(&full);
                        self.ask_text(
                            sh,
                            Msg::OpenLocalTitle,
                            body,
                            vec![(Msg::Cancel, BTN_CANCEL), (Msg::OpenButton, BTN_OPEN)],
                            Pending::Local { path: full },
                        );
                    }
                    ylink::LocalKind::Blocked => {
                        let body = ypath::display_form(&full);
                        self.ask_text(
                            sh,
                            Msg::BlockedSafety,
                            body,
                            vec![(Msg::Cancel, BTN_CANCEL), (Msg::ShowInExplorer, BTN_REVEAL)],
                            Pending::Reveal { path: full },
                        );
                    }
                }
            }
            ylink::Target::Blocked => self.toast(Msg::BlockedLinkType),
        }
    }

    /// R-LINK-01: `#slug` / `{#id}` / HTML `id`/`name` / heading text (wikilinks) → smooth scroll.
    fn jump_anchor(&mut self, sh: &Shared, a: &str) {
        let vh = self.view_h();
        let Some(d) = self.doc() else { return };
        let Content::Markdown(doc) = &d.content else {
            return;
        };
        let ix = &doc.index;
        let lower = a.to_lowercase();
        let block = ix
            .headings
            .iter()
            .find(|h| h.explicit_id.as_deref() == Some(a) || h.slug == a)
            .or_else(|| {
                ix.headings
                    .iter()
                    .find(|h| h.slug == lower || h.text.to_lowercase() == lower)
            })
            .map(|h| h.block as usize)
            .or_else(|| ix.html_anchors.get(a).map(|&b| b as usize));
        let Some(b) = block else {
            self.toast_with(Msg::LinkNotFound, &format!("#{a}"));
            return;
        };
        let old = d.viewer.scroll;
        let top = d.viewer.block_top(&d.content, b);
        d.viewer.scroll_to(top, vh);
        self.prepare_now(sh);
        let Some(d) = self.doc() else { return };
        let target = (d.viewer.block_top(&d.content, b) - 16.0).max(0.0);
        d.viewer.scroll_to(old, vh);
        self.scroll_by(sh, target - old, true);
    }

    /// R-LINK-02: history back (`-1`) / forward (`+1`) with per-entry scroll restore.
    fn history_go(&mut self, sh: &Shared, dir: i32) {
        let Some(next) = self.hist_pos.checked_add_signed(dir as isize) else {
            return;
        };
        if next >= self.history.len() {
            return;
        }
        let scroll = self.scroll_pos();
        if let Some(cur) = self.history.get_mut(self.hist_pos) {
            cur.1 = scroll;
        }
        self.focus = None;
        let (path, y) = self.history[next].clone();
        self.open(sh, &path, None, false);
        self.hist_pos = next;
        self.prepare_now(sh);
        self.set_scroll(y);
    }

    fn geometry(&self, sh: &Shared) -> Option<Geometry> {
        let (d, _) = self.dialog.as_ref()?;
        Some(d.geometry(self.cw, self.ch, &mut |t, s, wgt, mw| {
            sh.dw.measure(t, s, wgt, mw)
        }))
    }

    // ---- scrolling -----------------------------------------------------------------------

    fn doc(&mut self) -> Option<&mut DocView> {
        match &mut self.view {
            View::Doc(d) => Some(d),
            _ => None,
        }
    }

    fn scroll_pos(&self) -> f64 {
        match &self.view {
            View::Doc(d) => d.viewer.scroll,
            _ => 0.0,
        }
    }

    /// Moves to `y` (DIPs) with `ScrollWindowEx` + exposed-strip painting.
    fn set_scroll(&mut self, y: f64) {
        let scale = f64::from(self.scale);
        let vh = self.view_h();
        let Some(d) = self.doc() else { return };
        let old = d.viewer.scroll;
        let y = (y.clamp(0.0, d.viewer.max_scroll(vh)) * scale).round() / scale;
        if (y - old).abs() < 1e-6 {
            return;
        }
        d.viewer.scroll_to(y, vh);
        let dy = ((old * scale).round() - (y * scale).round()) as i32;
        let ch_px = (self.ch * self.scale) as i32;
        // SAFETY: valid hwnd; scroll the client area and invalidate the exposed strip.
        unsafe {
            if dy.abs() < ch_px && self.dialog.is_none() {
                ScrollWindowEx(self.hwnd, 0, dy, None, None, None, None, SW_INVALIDATE);
                // The overlay scrollbar moved with the bits: repaint its strip.
                let mut rc = RECT::default();
                let _ = GetClientRect(self.hwnd, &mut rc);
                rc.left = rc.right - (16.0 * self.scale) as i32;
                let _ = InvalidateRect(Some(self.hwnd), Some(&rc), false);
            } else {
                let _ = InvalidateRect(Some(self.hwnd), None, false);
            }
        }
        self.show_scrollbar();
        // Paint the exposed strip right after the handler returns (no RefCell borrow is held
        // then, so the synchronous WM_PAINT reaches `paint`).
        self.defer(Deferred::Update);
    }

    fn scroll_by(&mut self, sh: &Shared, dy: f64, animate: bool) {
        let cur = self.scroll_pos();
        let base = self.anim.map_or(cur, |a| a.to);
        let target = base + dy;
        if animate && sh.smooth && !self.reduced_motion() {
            let vh = self.view_h();
            let max = match &self.view {
                View::Doc(d) => d.viewer.max_scroll(vh),
                _ => 0.0,
            };
            self.anim = Some(Anim {
                from: cur,
                to: target.clamp(0.0, max),
                start: Instant::now(),
                ms: 150.0,
            });
            // SAFETY: valid hwnd; the timer runs only while animating (T7).
            unsafe { SetTimer(Some(self.hwnd), TIMER_ANIM, 8, None) };
        } else {
            self.anim = None;
            self.set_scroll(target);
        }
    }

    fn tick_anim(&mut self) {
        let Some(a) = self.anim else {
            // SAFETY: valid hwnd.
            let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_ANIM) };
            return;
        };
        let t = a.start.elapsed().as_secs_f64() * 1000.0 / a.ms;
        let y = a.from + (a.to - a.from) * ease_out(t);
        self.set_scroll(y);
        if t >= 1.0 {
            self.anim = None;
            // SAFETY: valid hwnd.
            let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_ANIM) };
        }
    }

    fn show_scrollbar(&mut self) {
        if !matches!(self.view, View::Doc(_)) {
            return;
        }
        let was = self.sb.alpha;
        self.sb.alpha = 1.0;
        self.sb.fade_start = None;
        if let Some(sh) = shared()
            && sh.autohide
        {
            // SAFETY: valid hwnd; one-shot hide timer (killed when it fires).
            unsafe {
                let _ = KillTimer(Some(self.hwnd), TIMER_SB_FADE);
                SetTimer(Some(self.hwnd), TIMER_SB_HIDE, 1000, None);
            }
        }
        if was < 1.0 {
            self.invalidate_scrollbar();
        }
    }

    fn invalidate_scrollbar(&self) {
        let mut rc = RECT::default();
        // SAFETY: valid hwnd and out-pointer.
        unsafe {
            let _ = GetClientRect(self.hwnd, &mut rc);
            rc.left = rc.right - (16.0 * self.scale) as i32;
            let _ = InvalidateRect(Some(self.hwnd), Some(&rc), false);
        }
    }

    /// Thumb rectangle (client DIPs).
    fn thumb(&self) -> Option<Rect> {
        let View::Doc(d) = &self.view else {
            return None;
        };
        let vh = self.view_h();
        let total = d.viewer.total_height();
        if total <= vh + 0.5 {
            return None;
        }
        let w = if self.sb.hover || self.sb.drag.is_some() {
            12.0
        } else {
            8.0
        };
        let th = (vh * vh / total).max(32.0);
        let ty = d.viewer.scroll / d.viewer.max_scroll(vh).max(1.0) * (vh - th);
        Some(Rect::new(self.cw - 2.0 - w, ty as f32, w, th as f32))
    }

    // ---- selection -----------------------------------------------------------------------

    fn hit(&self, sh: &Shared, x: f32, y: f32) -> Option<Hit> {
        let View::Doc(d) = &self.view else {
            return None;
        };
        let (cx, _) = self.col(sh);
        d.viewer
            .hit(&d.content, x - cx, f64::from(y) + d.viewer.scroll)
    }

    /// Over a link, footnote reference/back-link or fold toggle (hand cursor, B.6.10).
    fn over_hot(&self, sh: &Shared, x: f32, y: f32) -> bool {
        let View::Doc(d) = &self.view else {
            return false;
        };
        let (cx, _) = self.col(sh);
        let doc_y = f64::from(y) + d.viewer.scroll;
        d.viewer
            .hot_at(&d.content, x - cx, doc_y)
            .is_some_and(|(_, k)| {
                matches!(
                    k,
                    HotKind::Link { .. }
                        | HotKind::FootRef { .. }
                        | HotKind::FootBack { .. }
                        | HotKind::Fold { .. }
                )
            })
    }

    fn over_text(&self, sh: &Shared, x: f32, y: f32) -> bool {
        let View::Doc(d) = &self.view else {
            return false;
        };
        let (cx, cw) = self.col(sh);
        if x < cx || x > cx + cw {
            return false;
        }
        let docy = f64::from(y) + d.viewer.scroll;
        let bi = d.viewer.block_at_y(docy);
        let Some(bl) = d.viewer.layout_of(bi) else {
            return false;
        };
        let by = d.viewer.block_top(&d.content, bi);
        let ly = (docy - by) as f32;
        bl.paras.iter().any(|p| {
            p.map.is_some()
                && ly >= p.y
                && ly < p.y + p.shaped.height()
                && x - cx >= p.x
                && x - cx <= p.x + p.shaped.width().max(4.0)
        })
    }

    fn copy(&self) {
        let (View::Doc(d), Some((a, b))) = (&self.view, self.sel) else {
            return;
        };
        let (a, b) = (a.min(b), a.max(b));
        if a == b {
            return;
        }
        let text = d.content.copy_text(a..b);
        let html = match &d.content {
            Content::Markdown(doc) => Some(yamdv_export::html::cf_html(
                &yamdv_export::html::fragment(doc, a..b),
            )),
            Content::Plain { .. } => None,
        };
        sys::copy_text_html(self.hwnd, &text, html.as_deref());
    }

    /// `Ctrl+Shift+C` → Copy as Markdown: the source slice expanded to whole constructs.
    fn copy_markdown(&self) {
        let (View::Doc(d), Some((a, b))) = (&self.view, self.sel) else {
            return;
        };
        let (a, b) = (a.min(b), a.max(b));
        if a == b {
            return;
        }
        let text = match &d.content {
            Content::Markdown(doc) => yamdv_export::html::markdown_slice(doc, a..b),
            Content::Plain { text, .. } => {
                text.get(a as usize..b as usize).unwrap_or("").to_string()
            }
        };
        sys::copy_text(self.hwnd, &text);
    }

    // ---- painting ------------------------------------------------------------------------

    fn paint(&mut self, sh: &Shared) {
        let t0 = Instant::now();
        let mut ps = PAINTSTRUCT::default();
        // SAFETY: standard BeginPaint/EndPaint pair.
        let hdc = unsafe { BeginPaint(self.hwnd, &mut ps) };
        let vh = self.view_h();
        let (cx, cwid) = self.col(sh);
        if let View::Doc(d) = &mut self.view {
            if (d.viewer.width - cwid).abs() > 0.5 {
                d.viewer.set_width(&d.content, cwid, vh);
            }
            let strings = I18nStrings(sh.lang);
            let mut shaper = DwShaper(&sh.dw);
            let mut env = Env {
                shaper: &mut shaper,
                m: sh.m,
                pal: &self.pal,
                strings: &strings,
                single_newline_break: sh.read.single_newline_break,
                images: &d.images,
            };
            d.viewer.prepare(&mut d.content, &mut env, vh);
        }
        let geo = self.geometry(sh);
        if let Some(r) = &self.renderer {
            let me = &*self;
            if let View::Doc(d) = &me.view {
                d.images.begin_frame();
            }
            r.paint(hdc, ps.rcPaint, me.pal.text, &mut |band| {
                me.draw(sh, band, cx, geo.as_ref())
            });
            if let View::Doc(d) = &me.view {
                // P-13: everything within the viewport + one screen keeps its pixels.
                for (_, _, bl) in d.viewer.visible(&d.content, me.view_h() * 2.0) {
                    for it in &bl.items {
                        if let Item::Image { src, .. } = it {
                            d.images.pin(src);
                        }
                    }
                }
                d.images.end_frame();
            }
        }
        // SAFETY: matches BeginPaint.
        let _ = unsafe { EndPaint(self.hwnd, &ps) };
        if let Some(r) = &self.renderer {
            let bytes = crate::render::band_bytes(r.width());
            // SAFETY: valid hwnd; the property value is a plain integer, never a handle.
            unsafe {
                let _ = SetPropW(
                    self.hwnd,
                    PROP_BAND_BYTES,
                    Some(HANDLE(std::ptr::without_provenance_mut(bytes + 1))),
                );
            }
        }
        if self.auto_phase > 0 {
            self.perf.push(t0.elapsed().as_secs_f64() * 1000.0);
        }
        if self.warm_pending {
            self.warm_pending = false;
            self.start_warmup();
        }
        if !self.first_paint {
            self.first_paint = true;
            let ms = crate::perf::ms_since_process_start();
            // SAFETY: valid hwnd; the property value is a plain integer (ms + 1), never a handle.
            unsafe {
                let _ = SetPropW(
                    self.hwnd,
                    PROP_FIRST_PAINT,
                    Some(HANDLE(std::ptr::without_provenance_mut(ms as usize + 1))),
                );
            }
            if sh.dev.autoscroll && matches!(self.view, View::Doc(_)) {
                self.auto_phase = 1;
                // SAFETY: valid hwnd.
                let _ = unsafe {
                    PostMessageW(Some(self.hwnd), WM_APP_AUTOSCROLL, WPARAM(0), LPARAM(0))
                };
            }
            #[cfg(feature = "dev-tools")]
            if let Some(p) = &sh.dev.script {
                driver::start(self.hwnd, p);
            }
            if sh.dev.mem_stats.is_some() {
                // SAFETY: valid hwnd; one-shot (killed when it fires).
                unsafe { SetTimer(Some(self.hwnd), TIMER_MEMSTATS, 2000, None) };
            }
        }
    }

    /// Queues the font warm-up for the current document (F-10). Runs one task per posted
    /// message after the first paint, so input messages interleave between tasks.
    fn start_warmup(&mut self) {
        let View::Doc(d) = &self.view else { return };
        let text: &str = match &d.content {
            Content::Markdown(doc) => &doc.source,
            Content::Plain { text, .. } => text,
        };
        let mut plan = yamdv_layout::warm::warm_plan(&d.viewer.metrics(), text);
        plan.reverse();
        let was_empty = self.warm.is_empty();
        self.warm = plan;
        if was_empty && !self.warm.is_empty() {
            // SAFETY: valid hwnd.
            let _ = unsafe { PostMessageW(Some(self.hwnd), WM_APP_WARM, WPARAM(0), LPARAM(0)) };
        }
    }

    fn shape_ui(
        &self,
        sh: &Shared,
        text: &str,
        size: f32,
        weight: u16,
        color: u32,
        max_w: Option<f32>,
    ) -> Box<dyn yamdv_layout::shaper::Shaped> {
        let s = Style {
            family: Family::Body,
            size,
            weight,
            italic: false,
            color,
            strike: false,
            underline: false,
        };
        DwShaper(&sh.dw).shape(&ParaSpec {
            text,
            base: s,
            spans: vec![],
            width: max_w,
            line_pitch: (size * 1.4).round(),
            rtl: false,
            tab_spaces: None,
        })
    }

    fn draw(&self, sh: &Shared, b: &Band<'_>, cx: f32, geo: Option<&Geometry>) {
        let pal = &self.pal;
        b.clear(pal.bg);
        match &self.view {
            View::Empty { note } => {
                let msg = tr(sh.lang, note.unwrap_or(Msg::EmptyState));
                let t = self.shape_ui(
                    sh,
                    msg,
                    15.0,
                    400,
                    pal.text_muted,
                    Some((self.cw - 48.0).max(100.0)),
                );
                let x = ((self.cw - t.width()) / 2.0).round();
                let y = ((self.ch - t.height()) / 2.0).round();
                b.text(t.as_ref(), x, y);
            }
            View::Error { reason, path } => {
                let w = 520.0f32.min(self.cw - 48.0).max(200.0);
                let inner = w - 48.0;
                let t = self.shape_ui(
                    sh,
                    tr(sh.lang, Msg::CannotOpen),
                    15.0,
                    600,
                    pal.text,
                    Some(inner),
                );
                let r = self.shape_ui(sh, tr(sh.lang, *reason), 13.0, 400, pal.text, Some(inner));
                let p = self.shape_ui(sh, path, 12.0, 400, pal.text_muted, Some(inner));
                let h = 24.0 + t.height() + 8.0 + r.height() + 8.0 + p.height() + 24.0;
                let x = ((self.cw - w) / 2.0).round();
                let y = ((self.ch - h) / 2.0).round();
                let card = Rect::new(x, y, w, h);
                b.fill(card, pal.code_bg, 1.0, 8.0);
                b.stroke(card, pal.code_border, 8.0, 1.0);
                let mut yy = y + 24.0;
                b.text(t.as_ref(), x + 24.0, yy);
                yy += t.height() + 8.0;
                b.text(r.as_ref(), x + 24.0, yy);
                yy += r.height() + 8.0;
                b.text(p.as_ref(), x + 24.0, yy);
            }
            View::Doc(d) => {
                let sel = self.sel.map(|(a, c)| a.min(c)..a.max(c));
                draw_doc(
                    b,
                    &d.content,
                    &d.viewer,
                    pal,
                    cx,
                    sel,
                    self.view_h(),
                    Some(&d.images),
                    self.find_marks(),
                );
                if self.dimmed {
                    b.fill(Rect::new(0.0, 0.0, self.cw, self.ch), pal.bg, 0.5, 0.0);
                }
                self.draw_code_toolbar(sh, b, d, pal, cx);
                if let Some((fb, fl)) = self.focus {
                    let top = (d.viewer.block_top(&d.content, fb) - d.viewer.scroll) as f32;
                    for r in focus_rects(&d.viewer, fb, fl) {
                        // B.16: 2 px focus ring with 1 px offset.
                        b.stroke(
                            Rect::new(cx + r.x - 3.0, top + r.y - 3.0, r.w + 6.0, r.h + 6.0),
                            pal.accent,
                            4.0,
                            2.0,
                        );
                    }
                }
                if let Some(t) = self.thumb()
                    && self.sb.alpha > 0.0
                {
                    let a = if self.sb.hover || self.sb.drag.is_some() {
                        0.55
                    } else {
                        0.35
                    };
                    b.fill(t, pal.text, a * self.sb.alpha, t.w / 2.0);
                }
            }
        }
        self.draw_find(sh, b);
        if let Some(m) = self.toast {
            let text = format!("{}{}", tr(sh.lang, m), self.toast_arg);
            let t = self.shape_ui(
                sh,
                &text,
                13.0,
                400,
                pal.bg,
                Some((self.cw - 64.0).max(100.0)),
            );
            let (tw, th) = (t.width() + 32.0, t.height() + 20.0);
            let r = Rect::new(
                ((self.cw - tw) / 2.0).round(),
                (self.ch - th - 24.0).round(),
                tw,
                th,
            );
            b.fill(r, pal.text, 0.92, 8.0);
            b.text(t.as_ref(), r.x + 16.0, r.y + 10.0);
        }
        if let (Some((d, _)), Some(g)) = (&self.dialog, geo) {
            b.fill(
                Rect::new(0.0, 0.0, self.cw, self.ch),
                0x000000,
                dlg::BACKDROP_ALPHA,
                0.0,
            );
            let c = Rect::new(g.card.x, g.card.y, g.card.w, g.card.h);
            b.fill(c, pal.bg, 1.0, 8.0);
            b.stroke(c, pal.code_border, 8.0, 1.0);
            let t = self.shape_ui(
                sh,
                &d.title,
                dlg::TITLE_SIZE,
                600,
                pal.text,
                Some(g.title.w),
            );
            b.text(t.as_ref(), g.title.x, g.title.y);
            let body = self.shape_ui(sh, &d.body, dlg::BODY_SIZE, 400, pal.text, Some(g.body.w));
            b.text(body.as_ref(), g.body.x, g.body.y);
            for (i, (btn, r)) in d.buttons.iter().zip(g.buttons.iter()).enumerate() {
                let rr = Rect::new(r.x, r.y, r.w, r.h);
                let primary = i == d.primary();
                if primary {
                    b.fill(rr, pal.accent, 1.0, 6.0);
                } else {
                    b.fill(rr, pal.bg, 1.0, 6.0);
                    b.stroke(rr, pal.code_border, 6.0, 1.0);
                }
                if d.hover == Some(i) {
                    b.fill(rr, rgb_text(self.dark), 0.06, 6.0);
                }
                let color = if primary { 0xffffff } else { pal.text };
                let l = self.shape_ui(sh, &btn.label, dlg::BODY_SIZE, 400, color, None);
                b.text(
                    l.as_ref(),
                    (r.x + (r.w - l.width()) / 2.0).round(),
                    (r.y + (r.h - l.height()) / 2.0).round(),
                );
                if d.focus_visible && d.focus == i {
                    // B.16: 2 px focus ring with 1 px offset.
                    b.stroke(
                        Rect::new(rr.x - 3.0, rr.y - 3.0, rr.w + 6.0, rr.h + 6.0),
                        pal.accent,
                        8.0,
                        2.0,
                    );
                }
            }
        }
    }

    fn set_dark(&mut self, dark: bool) {
        self.dark = dark;
        self.pal = shared().map_or_else(
            || theme::palette("default", dark),
            |sh| app::palette_for(&sh.scheme, dark).0,
        );
        apply_mode(self.hwnd, if dark { Mode::Dark } else { Mode::Light });
        if let View::Doc(d) = &mut self.view {
            d.viewer.invalidate_layouts();
        }
        self.invalidate();
    }

    // ---- input ---------------------------------------------------------------------------

    fn key(&mut self, sh: &Shared, vk: u16, ctrl: bool, shift: bool) -> Option<Action> {
        if let Some((d, _)) = &mut self.dialog {
            let k = match vk {
                v if v == VK_RETURN.0 => Some(dlg::Key::Enter),
                v if v == VK_ESCAPE.0 => Some(dlg::Key::Esc),
                v if v == VK_TAB.0 => Some(if shift {
                    dlg::Key::ShiftTab
                } else {
                    dlg::Key::Tab
                }),
                v if v == VK_SPACE.0 => Some(dlg::Key::Space),
                _ => None,
            };
            match k.map(|k| d.key(k)) {
                Some(Outcome::Chosen(id)) => self.dialog_done(sh, id),
                Some(Outcome::Cancelled) => self.dialog_done(sh, BTN_CANCEL),
                Some(Outcome::Open { .. }) => self.invalidate(),
                None => {}
            }
            return None;
        }
        let page = (self.view_h() - LINE_STEP).max(LINE_STEP);
        if self.find.is_some() {
            match vk {
                v if v == VK_ESCAPE.0 => {
                    self.find_close();
                    return None;
                }
                v if v == VK_RETURN.0 => {
                    self.find_step(sh, if shift { -1 } else { 1 });
                    return None;
                }
                _ => {}
            }
        }
        match vk {
            0x70 => return Some(Action::Help),
            0x73 => self.open_editor(),
            v if ctrl && v == u16::from(b'E') => self.open_editor(),
            0x74 => return Some(Action::Command(app::RELOAD)),
            0x5D => return Some(self.kbd_menu()),
            v if ctrl && v == u16::from(b'D') => self.toggle_favorite(),
            v if ctrl && v == u16::from(b'F') => self.find_open(),
            v if ctrl && v == u16::from(b'G') => {
                self.find_open();
                self.find_step(sh, if shift { -1 } else { 1 });
            }
            0xBB | 0x6B if ctrl => return Some(Action::Command(app::FONT_BIG)),
            0xBD | 0x6D if ctrl => return Some(Action::Command(app::FONT_SMALL)),
            0x30 | 0x60 if ctrl => return Some(Action::Command(app::FONT_RESET)),
            v if ctrl && v == VK_UP.0 => return Some(Action::Command(app::FONT_BIG)),
            v if ctrl && v == VK_DOWN.0 => return Some(Action::Command(app::FONT_SMALL)),
            0xDC if ctrl => return Some(Action::Command(app::OUTLINE)),
            v if v == VK_ESCAPE.0 && self.focus.is_some() => {
                self.focus = None;
                self.invalidate();
            }
            v if v == VK_ESCAPE.0 => return Some(Action::Close),
            v if v == VK_TAB.0 => self.focus_step(sh, if shift { -1 } else { 1 }),
            v if v == VK_RETURN.0 => {
                if let Some((bi, l)) = self.focus {
                    self.link_click(sh, bi, l, ctrl);
                }
            }
            v if ctrl && v == u16::from(b'W') => return Some(Action::Close),
            v if ctrl && v == u16::from(b'O') => return Some(Action::OpenDialog),
            v if ctrl && v == u16::from(b'A') => self.select_all(),
            v if ctrl && shift && v == u16::from(b'C') => self.copy_markdown(),
            v if ctrl && v == u16::from(b'C') => self.copy(),
            v if v == VK_UP.0 => self.scroll_by(sh, -LINE_STEP, true),
            v if v == VK_DOWN.0 => self.scroll_by(sh, LINE_STEP, true),
            v if v == VK_PRIOR.0 => self.scroll_by(sh, -page, true),
            v if v == VK_NEXT.0 => self.scroll_by(sh, page, true),
            v if v == VK_SPACE.0 => self.scroll_by(sh, if shift { -page } else { page }, true),
            v if v == VK_HOME.0 => self.scroll_by(sh, -self.scroll_pos() - 1.0, true),
            v if v == VK_END.0 => self.scroll_by(sh, 1e12, true),
            _ => {}
        }
        None
    }

    fn mouse_down(&mut self, sh: &Shared, x: f32, y: f32, ctrl: bool) {
        if self.find.is_some() && self.dialog.is_none() {
            let (bar, case) = findbar::bar_rects(self.cw);
            if case.contains(x, y) {
                self.find_toggle_case(sh);
                return;
            }
            if bar.contains(x, y) {
                return;
            }
        }
        if self.dialog.is_some() {
            if let Some(g) = self.geometry(sh)
                && let Some((d, _)) = &mut self.dialog
            {
                match d.click(&g, x, y) {
                    Outcome::Chosen(id) => self.dialog_done(sh, id),
                    Outcome::Cancelled => self.dialog_done(sh, BTN_CANCEL),
                    Outcome::Open { .. } => self.invalidate(),
                }
            }
            return;
        }
        if let Some(t) = self.thumb()
            && x >= self.cw - 16.0
        {
            if y >= t.y && y < t.y + t.h {
                self.sb.drag = Some(f64::from(y - t.y));
                // SAFETY: valid hwnd.
                unsafe { SetCapture(self.hwnd) };
            } else {
                let page = (self.view_h() - LINE_STEP).max(LINE_STEP);
                self.scroll_by(sh, if y < t.y { -page } else { page }, true);
            }
            self.invalidate_scrollbar();
            return;
        }
        if let View::Doc(d) = &self.view {
            let (cx, _) = self.col(sh);
            let doc_y = f64::from(y) + d.viewer.scroll;
            if let Some((bi, r, top)) = d.viewer.region_at(&d.content, x - cx, doc_y)
                && let Some(g) = d
                    .viewer
                    .layout_of(bi)
                    .and_then(|l| l.regions.get(usize::from(r)))
            {
                let ly = (doc_y - top) as f32;
                if ly >= g.track.y - 4.0 && ly < g.track.bottom() + 4.0 {
                    self.hdrag = Some((bi, r, x, d.viewer.hscroll(bi, r)));
                    // SAFETY: valid hwnd.
                    unsafe { SetCapture(self.hwnd) };
                    return;
                }
            }
        }
        if self.focus.take().is_some() {
            self.invalidate();
        }
        if self.code_click(sh, x, y, ctrl) {
            return;
        }
        let Some(h) = self.hit(sh, x, y) else {
            if self.sel.take().is_some() {
                self.invalidate();
            }
            return;
        };
        // Click counting (double = word, triple = block), within the double-click time/area.
        // SAFETY: no preconditions.
        let (t, dt, dx, dy) = unsafe {
            (
                GetMessageTime(),
                GetDoubleClickTime(),
                GetSystemMetrics(SM_CXDOUBLECLK),
                GetSystemMetrics(SM_CYDOUBLECLK),
            )
        };
        let (n, lt, lx, ly) = self.clicks;
        let (xi, yi) = (x as i32, y as i32);
        let same = u32::try_from(t.wrapping_sub(lt)).unwrap_or(u32::MAX) <= dt
            && (xi - lx).abs() <= dx
            && (yi - ly).abs() <= dy;
        let count = if same { n % 3 + 1 } else { 1 };
        self.clicks = (count, t, xi, yi);
        // SAFETY: GetKeyState has no preconditions.
        let shift = unsafe { GetKeyState(i32::from(VK_SHIFT.0)) } < 0;
        let View::Doc(d) = &self.view else { return };
        match count {
            2 => {
                if let Some(r) = d.viewer.word_at(&h) {
                    self.sel = Some((r.start, r.end));
                }
            }
            3 => {
                if let Some(r) = d.viewer.para_at(&h) {
                    self.sel = Some((r.start, r.end));
                }
            }
            _ => {
                self.sel = match (shift, self.sel) {
                    (true, Some((a, _))) => Some((a, h.source)),
                    _ => Some((h.source, h.source)),
                };
                self.selecting = true;
                // SAFETY: valid hwnd.
                unsafe { SetCapture(self.hwnd) };
            }
        }
        self.invalidate();
    }

    fn mouse_move(&mut self, sh: &Shared, x: f32, y: f32) {
        if !self.tracking {
            let mut tme = TRACKMOUSEEVENT {
                cbSize: u32::try_from(size_of::<TRACKMOUSEEVENT>()).unwrap_or(0),
                dwFlags: TME_LEAVE,
                hwndTrack: self.hwnd,
                dwHoverTime: 0,
            };
            // SAFETY: valid structure.
            if unsafe { TrackMouseEvent(&mut tme) }.is_ok() {
                self.tracking = true;
            }
        }
        if let Some((d, _)) = &mut self.dialog {
            let g = d.geometry(self.cw, self.ch, &mut |t, s, wgt, mw| {
                sh.dw.measure(t, s, wgt, mw)
            });
            if d.mouse_move(&g, x, y) {
                self.invalidate();
            }
            return;
        }
        if let Some((bi, r, gx, off)) = self.hdrag {
            if let View::Doc(d) = &mut self.view
                && let Some(g) = d
                    .viewer
                    .layout_of(bi)
                    .and_then(|l| l.regions.get(usize::from(r)))
            {
                let k = g.content_w / g.track.w.max(1.0);
                if d.viewer.set_hscroll(bi, r, off + (x - gx) * k) {
                    self.invalidate();
                }
            }
            return;
        }
        if let Some(grab) = self.sb.drag {
            let vh = self.view_h();
            if let (View::Doc(dv), Some(t)) = (&self.view, self.thumb()) {
                let max = dv.viewer.max_scroll(vh);
                let track = (vh - f64::from(t.h)).max(1.0);
                let target = (f64::from(y) - grab) / track * max;
                self.anim = None;
                self.set_scroll(target);
            }
            return;
        }
        let ch = if let View::Doc(d) = &self.view {
            let (cx, _) = self.col(sh);
            d.viewer
                .toolbar_at(&d.content, x - cx, f64::from(y) + d.viewer.scroll)
                .and_then(|(bi, _, it)| match it {
                    Item::CodeToolbar { node, .. } => Some((bi, *node)),
                    _ => None,
                })
        } else {
            None
        };
        if ch != self.code_hover {
            self.code_hover = ch;
            self.invalidate();
        }
        let hover = x >= self.cw - 16.0 && self.thumb().is_some();
        if hover != self.sb.hover {
            self.sb.hover = hover;
            if hover {
                self.show_scrollbar();
            }
            self.invalidate_scrollbar();
        }
        if self.selecting {
            self.drag_dir = if y < 0.0 {
                f64::from(y).max(-200.0)
            } else if y > self.ch {
                f64::from(y - self.ch).min(200.0)
            } else {
                0.0
            };
            if self.drag_dir != 0.0 {
                // SAFETY: valid hwnd; runs only while drag-selecting outside the window.
                unsafe { SetTimer(Some(self.hwnd), TIMER_DRAG, 30, None) };
            }
            let yy = y.clamp(0.0, self.ch - 1.0);
            if let Some(h) = self.hit(sh, x, yy)
                && let Some((a, _)) = self.sel
            {
                self.sel = Some((a, h.source));
                self.invalidate();
            }
        }
    }

    fn mouse_up(&mut self) {
        if self.hdrag.take().is_some() {
            // SAFETY: capture release.
            let _ = unsafe { ReleaseCapture() };
        }
        if self.selecting || self.sb.drag.is_some() {
            self.selecting = false;
            self.sb.drag = None;
            self.drag_dir = 0.0;
            // SAFETY: valid hwnd / capture release.
            unsafe {
                let _ = KillTimer(Some(self.hwnd), TIMER_DRAG);
                let _ = ReleaseCapture();
            }
            self.invalidate_scrollbar();
        }
    }

    fn wheel(&mut self, sh: &Shared, delta: i16) {
        if self.dialog.is_some() {
            return;
        }
        let lines = f64::from(sys::wheel_lines());
        let d = -f64::from(delta) / 120.0 * lines * LINE_STEP;
        // Precision touchpads send fractional notches: apply directly, no animation.
        let notch = delta % 120 == 0;
        self.scroll_by(sh, d, notch);
    }

    /// Horizontal scroll of the code region under the cursor (R-CODE-03): `Shift+Wheel`,
    /// `WM_MOUSEHWHEEL` and touchpad deltas. Positive `delta` scrolls right. Only the region
    /// repaints. Returns false when no region is under the cursor.
    fn hwheel(&mut self, sh: &Shared, delta: i16, sx: i32, sy: i32) -> bool {
        if self.dialog.is_some() {
            return false;
        }
        let mut pt = POINT { x: sx, y: sy };
        // SAFETY: valid hwnd and point.
        let _ = unsafe { ScreenToClient(self.hwnd, &mut pt) };
        let (x, y) = (pt.x as f32 / self.scale, pt.y as f32 / self.scale);
        let (cx, _) = self.col(sh);
        let View::Doc(d) = &mut self.view else {
            return false;
        };
        let doc_y = f64::from(y) + d.viewer.scroll;
        let Some((bi, r, top)) = d.viewer.region_at(&d.content, x - cx, doc_y) else {
            return false;
        };
        let step = f32::from(delta) / 120.0 * 3.0 * LINE_STEP as f32;
        let cur = d.viewer.hscroll(bi, r);
        if d.viewer.set_hscroll(bi, r, cur + step)
            && let Some(g) = d
                .viewer
                .layout_of(bi)
                .and_then(|l| l.regions.get(usize::from(r)))
        {
            let ty = (top - d.viewer.scroll) as f32;
            let s = self.scale;
            let rc = RECT {
                left: ((cx + g.view.x) * s).floor() as i32,
                top: ((ty + g.view.y) * s).floor() as i32,
                right: ((cx + g.view.right()) * s).ceil() as i32,
                bottom: ((ty + g.track.bottom() + 2.0) * s).ceil() as i32,
            };
            // SAFETY: valid hwnd and rect.
            let _ = unsafe { InvalidateRect(Some(self.hwnd), Some(&rc), false) };
        }
        true
    }

    fn drop_files(&mut self, sh: &Shared, hdrop: HDROP) -> Vec<Action> {
        let mut files = Vec::new();
        let mut folder = false;
        // SAFETY: hdrop comes from WM_DROPFILES; lengths are queried before each copy and the
        // handle is released with DragFinish.
        unsafe {
            let n = DragQueryFileW(hdrop, 0xFFFF_FFFF, None);
            for i in 0..n.min(1000) {
                let len = DragQueryFileW(hdrop, i, None) as usize;
                if len == 0 || len > 32_767 {
                    continue;
                }
                let mut buf = vec![0u16; len + 1];
                let got = DragQueryFileW(hdrop, i, Some(&mut buf)) as usize;
                if got != len {
                    continue;
                }
                let p = String::from_utf16_lossy(buf.get(..len).unwrap_or(&[]));
                if std::fs::metadata(ypath::long_form(&p)).is_ok_and(|m| m.is_dir()) {
                    folder = true;
                } else {
                    files.push(p);
                }
            }
            DragFinish(hdrop);
        }
        let mut actions = Vec::new();
        let mut it = files.into_iter();
        if let Some(first) = it.next() {
            self.open(sh, &first, None, false);
            actions.extend(it.map(Action::NewWindow));
        } else if folder && !matches!(self.view, View::Doc(_)) {
            self.view = View::Empty {
                note: Some(Msg::DropFolders),
            };
            self.invalidate();
        }
        actions
    }

    fn autoscroll_step(&mut self) {
        let vh = self.view_h();
        let (pos, max) = match &self.view {
            View::Doc(d) => (d.viewer.scroll, d.viewer.max_scroll(vh)),
            _ => return,
        };
        self.auto_steps += 1;
        let step = 120.0;
        let done = self.auto_steps > 100_000
            || match self.auto_phase {
                1 if pos >= max - 0.5 => {
                    self.auto_phase = 2;
                    false
                }
                2 if pos <= 0.5 => true,
                _ => false,
            };
        if done {
            self.auto_phase = 0;
            if let Some(sh) = shared()
                && let Some(p) = &sh.dev.perf_log
            {
                let _ = std::fs::write(p, crate::perf::report(&self.perf));
            }
            self.set_title("SCROLL_DONE");
            return;
        }
        self.set_scroll(if self.auto_phase == 1 {
            pos + step
        } else {
            pos - step
        });
        // SAFETY: valid hwnd.
        let _ = unsafe { PostMessageW(Some(self.hwnd), WM_APP_AUTOSCROLL, WPARAM(0), LPARAM(0)) };
    }

    /// Code toolbar buttons (R-CODE-02/05): copy, line numbers, wrap. True when handled.
    fn code_click(&mut self, sh: &Shared, x: f32, y: f32, new_window: bool) -> bool {
        let (cx, _) = self.col(sh);
        let View::Doc(d) = &mut self.view else {
            return false;
        };
        let doc_y = f64::from(y) + d.viewer.scroll;
        let Some((bi, kind)) = d.viewer.hot_at(&d.content, x - cx, doc_y) else {
            return false;
        };
        match kind {
            HotKind::CodeCopy { node } => {
                let raw = d
                    .viewer
                    .toolbar_at(&d.content, x - cx, doc_y)
                    .and_then(|(_, _, it)| match it {
                        Item::CodeToolbar { raw, .. } => Some(raw.clone()),
                        _ => None,
                    })
                    .unwrap_or(0..0);
                let text = d
                    .content
                    .source()
                    .get(raw.start as usize..raw.end as usize)
                    .unwrap_or("")
                    .to_string();
                if sys::copy_text(self.hwnd, &text) {
                    self.code_copied = Some((bi, node));
                    // SAFETY: valid hwnd; one-shot timer.
                    unsafe { SetTimer(Some(self.hwnd), TIMER_COPIED, 1200, None) };
                    self.toast(Msg::Copied);
                }
            }
            HotKind::CodeLines { node } => {
                let mut o = d.viewer.code_opts(bi, node);
                o.line_numbers = !o.line_numbers;
                d.viewer.set_code_opts(bi, node, o);
            }
            HotKind::CodeWrap { node } => {
                let mut o = d.viewer.code_opts(bi, node);
                o.wrap = !o.wrap;
                d.viewer.set_code_opts(bi, node, o);
            }
            HotKind::Fold { node } => {
                let mut o = d.viewer.code_opts(bi, node);
                o.fold_toggled = !o.fold_toggled;
                d.viewer.set_code_opts(bi, node, o);
                if node == yamdv_layout::block::CROSS_DETAILS {
                    // The container's sibling blocks change height too.
                    d.viewer.invalidate_layouts();
                }
            }
            HotKind::FootRef { n, .. } => {
                self.footnote_jump(sh, FootTarget::Note(n));
                return true;
            }
            HotKind::FootBack { n, k } => {
                self.footnote_jump(sh, FootTarget::Ref(n, k));
                return true;
            }
            HotKind::FootNote { .. } | HotKind::Image { .. } | HotKind::ImagePlaceholder { .. } => {
                return false;
            }
            HotKind::CopyUrl { url } => {
                let _ = sys::copy_text(self.hwnd, &url);
            }
            HotKind::Link { link } => {
                self.link_click(sh, bi, link, new_window);
                return true;
            }
        }
        self.invalidate();
        true
    }

    /// Lays out the blocks around the current scroll position now (outside WM_PAINT).
    fn prepare_now(&mut self, sh: &Shared) {
        let vh = self.view_h();
        let (_, cwid) = self.col(sh);
        if let View::Doc(d) = &mut self.view {
            if (d.viewer.width - cwid).abs() > 0.5 {
                d.viewer.set_width(&d.content, cwid, vh);
            }
            let strings = I18nStrings(sh.lang);
            let mut shaper = DwShaper(&sh.dw);
            let mut env = Env {
                shaper: &mut shaper,
                m: sh.m,
                pal: &self.pal,
                strings: &strings,
                single_newline_break: sh.read.single_newline_break,
                images: &d.images,
            };
            d.viewer.prepare(&mut d.content, &mut env, vh);
        }
    }

    /// R-REN-10: smooth-scrolls to a footnote or back to one of its references (B.15). The
    /// target block is laid out first at an estimated position, then the exact y is used.
    fn footnote_jump(&mut self, sh: &Shared, t: FootTarget) {
        let vh = self.view_h();
        let Some(d) = self.doc() else { return };
        let Some((b, _)) = Viewer::footnote_block(&d.content, t) else {
            return;
        };
        let old = d.viewer.scroll;
        let approx = d.viewer.block_top(&d.content, b);
        d.viewer.scroll_to(approx, vh);
        self.prepare_now(sh);
        let Some(d) = self.doc() else { return };
        let target = d.viewer.footnote_target(&d.content, t).unwrap_or(approx);
        // Leave a little context above the target (B.15 anchor offset).
        let target = (target - 24.0).max(0.0);
        d.viewer.scroll_to(old, vh);
        self.scroll_by(sh, target - old, true);
    }

    /// Draws the hovered code block's toolbar (label + buttons).
    fn draw_code_toolbar(&self, sh: &Shared, b: &Band<'_>, d: &DocView, pal: &Palette, cx: f32) {
        let Some((hb, hn)) = self.code_hover else {
            return;
        };
        for (bi, y, bl) in d.viewer.visible(&d.content, self.view_h()) {
            if bi != hb {
                continue;
            }
            let y = y as f32;
            for it in &bl.items {
                let Item::CodeToolbar {
                    node,
                    label,
                    label_rect,
                    buttons,
                    opts,
                    ..
                } = it
                else {
                    continue;
                };
                if *node != hn {
                    continue;
                }
                if !label.is_empty() {
                    let t = self.shape_ui(sh, label, 11.0, 400, pal.text, None);
                    b.text(t.as_ref(), cx + label_rect.x, y + label_rect.y);
                }
                let copied = self.code_copied == Some((bi, *node));
                let glyphs = [
                    if copied { "\u{2713}" } else { "\u{2398}" },
                    "#",
                    "\u{21B5}",
                ];
                let on = [copied, opts.line_numbers, opts.wrap];
                for ((r, g), on) in buttons.iter().zip(glyphs).zip(on) {
                    let r = r.offset(cx, y);
                    b.fill(r, pal.bg, 0.9, 4.0);
                    if on {
                        b.fill(r, pal.accent, 0.18, 4.0);
                    }
                    b.stroke(r, pal.code_border, 4.0, 1.0);
                    let t = self.shape_ui(sh, g, 12.0, 400, pal.text, None);
                    b.text(
                        t.as_ref(),
                        r.x + ((r.w - t.width()) / 2.0).round(),
                        r.y + ((r.h - t.height()) / 2.0).round(),
                    );
                }
            }
        }
    }

    /// Shows a toast for 4 s (B.10).
    /// Tab / Shift+Tab: next / previous link in document order (B.16); scrolls it into view.
    fn focus_step(&mut self, sh: &Shared, dir: i32) {
        let cur = self.focus;
        let Some(d) = self.doc() else { return };
        let Content::Markdown(doc) = &mut d.content else {
            return;
        };
        let n = doc.index.blocks.len();
        let count = |doc: &mut Document, b: usize| -> u32 {
            u32::try_from(doc.detail(b).links.len()).unwrap_or(0)
        };
        let next = if dir > 0 {
            let (mut b, mut l) = cur.map_or((0, 0), |(b, l)| (b, l + 1));
            loop {
                if b >= n {
                    break None;
                }
                if l < count(doc, b) {
                    break Some((b, l));
                }
                b += 1;
                l = 0;
            }
        } else {
            match cur {
                Some((b, l)) if l > 0 => Some((b, l - 1)),
                _ => {
                    let mut b = cur.map_or(n, |(b, _)| b);
                    loop {
                        if b == 0 {
                            break None;
                        }
                        b -= 1;
                        let c = count(doc, b);
                        if c > 0 {
                            break Some((b, c - 1));
                        }
                    }
                }
            }
        };
        self.focus = next;
        if let Some((b, l)) = next {
            self.scroll_into_view(sh, b, l);
        }
        self.invalidate();
    }

    /// Scrolls so link `l` of block `b` is visible (auto-scroll for focus and the driver).
    fn scroll_into_view(&mut self, sh: &Shared, b: usize, l: u32) {
        let vh = self.view_h();
        for _ in 0..2 {
            let Some(d) = self.doc() else { return };
            let top = d.viewer.block_top(&d.content, b);
            let ry = focus_rects(&d.viewer, b, l)
                .first()
                .map_or((0.0, 20.0), |r| (f64::from(r.y), f64::from(r.h)));
            let (y0, y1) = (top + ry.0, top + ry.0 + ry.1);
            let s = d.viewer.scroll;
            if y0 >= s + 8.0 && y1 <= s + vh - 8.0 {
                return;
            }
            self.anim = None;
            self.set_scroll((y0 - vh / 3.0).max(0.0));
            self.prepare_now(sh);
        }
    }

    /// Middle-click on a link: Markdown targets open in a new window (R-LINK).
    fn middle_click(&mut self, sh: &Shared, x: f32, y: f32) {
        let (cx, _) = self.col(sh);
        let View::Doc(d) = &self.view else { return };
        if let Some((bi, HotKind::Link { link })) =
            d.viewer
                .hot_at(&d.content, x - cx, f64::from(y) + d.viewer.scroll)
        {
            self.link_click(sh, bi, link, true);
        }
    }

    fn toast(&mut self, m: Msg) {
        self.toast_with(m, "");
    }

    /// Toast with a dynamic suffix (e.g. "Link target not found: <name>").
    fn toast_with(&mut self, m: Msg, arg: &str) {
        self.toast = Some(m);
        self.toast_arg = arg.to_string();
        // SAFETY: valid hwnd; one-shot timer (killed when it fires).
        unsafe {
            SetTimer(Some(self.hwnd), TIMER_TOAST, 4000, None);
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    fn timer(&mut self, sh: &Shared, id: usize) {
        if id == TIMER_RELOAD {
            // SAFETY: valid hwnd.
            let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_RELOAD) };
            self.file_changed(sh);
            return;
        }
        if id == TIMER_COPIED {
            self.code_copied = None;
            // SAFETY: valid hwnd.
            unsafe {
                let _ = KillTimer(Some(self.hwnd), TIMER_COPIED);
                let _ = InvalidateRect(Some(self.hwnd), None, false);
            }
            return;
        }
        if id == TIMER_TOAST {
            self.toast = None;
            // SAFETY: valid hwnd.
            unsafe {
                let _ = KillTimer(Some(self.hwnd), TIMER_TOAST);
                let _ = InvalidateRect(Some(self.hwnd), None, false);
            }
            return;
        }
        match id {
            TIMER_ANIM => self.tick_anim(),
            #[cfg(feature = "dev-tools")]
            TIMER_SCRIPT => driver::tick(self, sh),
            TIMER_SB_HIDE => {
                // SAFETY: valid hwnd.
                let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_SB_HIDE) };
                if self.sb.hover || self.sb.drag.is_some() {
                    return;
                }
                if self.reduced_motion() {
                    self.sb.alpha = 0.0;
                    self.invalidate_scrollbar();
                } else {
                    self.sb.fade_start = Some(Instant::now());
                    // SAFETY: valid hwnd; runs only during the 300 ms fade.
                    unsafe { SetTimer(Some(self.hwnd), TIMER_SB_FADE, 16, None) };
                }
            }
            TIMER_SB_FADE => {
                let t = self
                    .sb
                    .fade_start
                    .map_or(1.0, |s| s.elapsed().as_secs_f32() * 1000.0 / 300.0);
                self.sb.alpha = (1.0 - t).max(0.0);
                if t >= 1.0 {
                    self.sb.fade_start = None;
                    // SAFETY: valid hwnd.
                    let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_SB_FADE) };
                }
                self.invalidate_scrollbar();
            }
            TIMER_DRAG => {
                if self.drag_dir == 0.0 || !self.selecting {
                    // SAFETY: valid hwnd.
                    let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_DRAG) };
                    return;
                }
                let d = self.drag_dir / 4.0;
                self.set_scroll(self.scroll_pos() + d);
                let mut pt = POINT::default();
                // SAFETY: valid out-pointer / hwnd.
                unsafe {
                    let _ = GetCursorPos(&mut pt);
                    let _ = ScreenToClient(self.hwnd, &mut pt);
                }
                let (x, y) = (pt.x as f32 / self.scale, pt.y as f32 / self.scale);
                self.mouse_move(sh, x, y);
            }
            TIMER_MEMSTATS => {
                // SAFETY: valid hwnd.
                let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_MEMSTATS) };
                #[cfg(feature = "mem-stats")]
                self.write_mem_stats(sh);
            }
            _ => {}
        }
    }

    #[cfg(feature = "mem-stats")]
    fn write_mem_stats(&self, sh: &Shared) {
        let Some(path) = &sh.dev.mem_stats else {
            return;
        };
        let (src, idx, cache, layouts, paras, items) = match &self.view {
            View::Doc(d) => {
                let (s, i, c) = match &d.content {
                    Content::Markdown(doc) => doc.heap_bytes(),
                    Content::Plain { text, chunks } => (text.capacity(), chunks.capacity() * 8, 0),
                };
                let (l, p, it) = d.viewer.layout_stats();
                (s, i, c, l, p, it)
            }
            _ => (0, 0, 0, 0, 0, 0),
        };
        let band = self
            .renderer
            .as_ref()
            .map_or(0, |r| crate::render::band_bytes(r.width()));
        let (img, lru) = match &self.view {
            View::Doc(d) => (d.images.decoded_bytes(), d.images.lru_bytes()),
            _ => (0, 0),
        };
        let json = format!(
            "{{\"schema\":\"yamdv.memstats/1\",\"image_decoded_bytes\":{img},\"image_lru_bytes\":{lru},\"source_text_bytes\":{src},\"block_index_bytes\":{idx},\"detail_cache_bytes\":{cache},\"layout_cache\":{{\"layouts\":{layouts},\"paras\":{paras},\"items\":{items},\"model_bytes_estimate\":{}}},\"band_buffer_bytes\":{band},\"commit_bytes_now\":{},\"commit_bytes_at_start\":{},\"note\":\"DirectWrite and other native memory = commit growth minus the Rust-side numbers; see 002_REPORT.md\"}}\n",
            paras * 160 + items * 40,
            crate::perf::commit_bytes(),
            sh.commit_at_start
        );
        let _ = std::fs::write(path, json);
    }
}

/// Draws the visible part of a document (shared by windows and `--render-png`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_doc(
    b: &Band<'_>,
    content: &Content,
    viewer: &Viewer,
    pal: &Palette,
    cx: f32,
    sel: Option<Range<u32>>,
    vh: f64,
    images: Option<&crate::images::Store>,
    marks: (&[Range<u32>], Option<usize>),
) {
    let sel_alpha = f32::from(pal.selection_alpha_pct) / 100.0;
    for (bi, y, bl) in viewer.visible(content, vh) {
        let y = y as f32;
        if !b.visible(y, bl.height.max(1.0)) {
            continue;
        }
        let off = |r: u16| {
            let max = bl
                .regions
                .get(usize::from(r))
                .map_or(0.0, |g| g.max_scroll());
            viewer.hscroll(bi, r).min(max)
        };
        for it in &bl.items {
            match it {
                Item::Fill {
                    rect,
                    color,
                    radius,
                } => b.fill(rect.offset(cx, y), *color, 1.0, *radius),
                Item::Stroke {
                    rect,
                    color,
                    radius,
                } => b.stroke(rect.offset(cx, y), *color, *radius, 1.0),
                Item::StrokeW {
                    rect,
                    color,
                    radius,
                    width,
                } => b.stroke(rect.offset(cx, y), *color, *radius, *width),
                Item::Line {
                    pts,
                    color,
                    width,
                    closed,
                } => {
                    let moved: Vec<(f32, f32)> = pts.iter().map(|p| (p.0 + cx, p.1 + y)).collect();
                    b.polyline(&moved, *color, *width, *closed);
                }
                Item::Bullet {
                    cx: bx,
                    cy,
                    size,
                    shape,
                    color,
                } => match shape {
                    Bullet::Disc => b.circle(cx + bx, y + cy, size / 2.0, *color, None),
                    Bullet::Ring => b.circle(cx + bx, y + cy, size / 2.0, *color, Some(1.2)),
                    Bullet::Square => b.fill(
                        Rect::new(cx + bx - size / 2.0, y + cy - size / 2.0, *size, *size),
                        *color,
                        1.0,
                        0.0,
                    ),
                },
                Item::FadeRight { rect, color } => b.fade_right(rect.offset(cx, y), *color),
                Item::Para(i) => {
                    let Some(p) = bl.paras.get(*i) else { continue };
                    let px = cx + p.x - p.hscroll.map_or(0.0, off);
                    if !marks.0.is_empty()
                        && let Some(map) = &p.map
                        && let (Some(f), Some(l)) = (map.segs.first(), map.segs.last())
                    {
                        // Find matches (B.9): all in `highlight_bg`, the current one in accent.
                        let (start, end) = (f.src.start, l.src.end);
                        let from = marks.0.partition_point(|m| m.end <= start);
                        for (k, m) in marks.0.iter().enumerate().skip(from) {
                            if m.start >= end {
                                break;
                            }
                            let cur = marks.1 == Some(k);
                            for r in viewer::selection_rects(p, m) {
                                let (c, a) = if cur {
                                    (pal.accent, 0.45)
                                } else {
                                    (pal.highlight_bg, 0.9)
                                };
                                b.fill(r.offset(px, y + p.y), c, a, 2.0);
                            }
                        }
                    }
                    if let Some(s) = &sel {
                        for r in viewer::selection_rects(p, s) {
                            b.fill(r.offset(px, y + p.y), pal.accent, sel_alpha, 0.0);
                        }
                    }
                    b.text(p.shaped.as_ref(), px, y + p.y);
                }
                // Drawn by the window on hover (not part of static renders).
                Item::CodeToolbar { .. } => {}
                Item::Image { rect, src, .. } => {
                    let r = rect.offset(cx, y);
                    let (wp, hp) = (
                        (r.w * b.scale).round().max(1.0) as u32,
                        (r.h * b.scale).round().max(1.0) as u32,
                    );
                    match images.and_then(|s| s.pixels(src, wp, hp)) {
                        Some(bm) => b.image(r.x, r.y, bm.w, bm.h, &bm.px),
                        // Decoding: a quiet `code_bg` box of the final size (no layout shift).
                        None => b.fill(r, pal.code_bg, 1.0, 4.0),
                    }
                }
                Item::PushClip(r) => b.push_clip(r.offset(cx, y)),
                Item::PopClip => b.pop_clip(),
                Item::HScrollBar(ri) => {
                    let Some(g) = bl.regions.get(*ri) else {
                        continue;
                    };
                    let o = off(u16::try_from(*ri).unwrap_or(0));
                    b.fill(g.track.offset(cx, y), pal.text, 0.08, 3.0);
                    b.fill(g.thumb(o).offset(cx, y), pal.text, 0.35, 3.0);
                }
            }
        }
    }
}

/// Hot rectangles (block-local) of link `l` in block `b`, when laid out.
fn focus_rects(v: &Viewer, b: usize, l: u32) -> Vec<Rect> {
    v.layout_of(b).map_or_else(Vec::new, |bl| {
        bl.hot
            .iter()
            .filter(|h| matches!(h.kind, HotKind::Link { link } if link == l))
            .map(|h| h.rect)
            .collect()
    })
}

fn apply_mode(hwnd: HWND, m: Mode) {
    let dark = windows::core::BOOL::from(m == Mode::Dark);
    // SAFETY: valid hwnd; pointer/size describe a BOOL on the stack.
    let _ = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&raw const dark).cast(),
            u32::try_from(size_of::<windows::core::BOOL>()).unwrap_or(4),
        )
    };
}

fn window_rect_for_client(w: i32, h: i32, dpi: u32) -> (i32, i32) {
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: w,
        bottom: h,
    };
    // SAFETY: rc is a valid in/out rect.
    if unsafe {
        AdjustWindowRectExForDpi(&mut rc, WS_OVERLAPPEDWINDOW, false, WINDOW_EX_STYLE(0), dpi)
    }
    .is_err()
    {
        return (w, h);
    }
    (rc.right - rc.left, rc.bottom - rc.top)
}

fn is_immersive_color_set(lparam: LPARAM) -> bool {
    if lparam.0 == 0 {
        return false;
    }
    // SAFETY: for WM_SETTINGCHANGE, a non-zero lParam points to a NUL-terminated wide string.
    let s = unsafe { PCWSTR(lparam.0 as *const u16).to_string() }.unwrap_or_default();
    s == "ImmersiveColorSet"
}

/// Alt is held (Alt+wheel column zoom).
fn alt_down() -> bool {
    // SAFETY: GetKeyState has no preconditions.
    unsafe { GetKeyState(0x12) < 0 }
}

fn lparam_xy(lp: LPARAM, scale: f32) -> (f32, f32) {
    let x = i32::from((lp.0 & 0xFFFF) as u16 as i16);
    let y = i32::from(((lp.0 >> 16) & 0xFFFF) as u16 as i16);
    (x as f32 / scale, y as f32 / scale)
}

fn update_size(w: &mut Win, sh: &Shared) {
    let mut rc = RECT::default();
    // SAFETY: valid hwnd and out-pointer.
    let _ = unsafe { GetClientRect(w.hwnd, &mut rc) };
    let (pw, ph) = (rc.right - rc.left, rc.bottom - rc.top);
    w.cw = pw as f32 / w.scale;
    w.ch = ph as f32 / w.scale;
    if let Some(r) = &mut w.renderer {
        r.resize(pw.max(1));
    }
    let (_, colw) = w.col(sh);
    let vh = w.view_h();
    if let View::Doc(d) = &mut w.view {
        d.viewer.set_width(&d.content, colw, vh);
    }
    w.invalidate();
}

fn handle(
    w: &mut Win,
    sh: &Shared,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> (Option<LRESULT>, Vec<Action>) {
    let mut actions = Vec::new();
    let r = match msg {
        WM_ERASEBKGND => Some(LRESULT(1)),
        WM_COPYDATA => {
            // SAFETY: for WM_COPYDATA, lParam points to a COPYDATASTRUCT valid during the call.
            let cds = unsafe { &*(lp.0 as *const COPYDATASTRUCT) };
            let sender = HWND(wp.0 as *mut core::ffi::c_void);
            if let Some((paths, line)) = app::accept_forward(sender, cds) {
                actions.push(Action::Forward(paths, line));
                return (Some(LRESULT(1)), actions);
            }
            let out = HOST.with(|h| {
                let mut h = h.borrow_mut();
                let hw = h.as_mut()?;
                let o = crate::dialog_host::accept(hw, w.hwnd, sender, cds)?;
                hw.answered = true;
                crate::log::warn(
                    "dialog-host",
                    &format!("answer after {} ms", hw.started.elapsed().as_millis()),
                );
                Some((hw.kind, o))
            });
            match out {
                Some((k, o)) => {
                    actions.push(Action::HostResult(k, o));
                    Some(LRESULT(1))
                }
                None => Some(LRESULT(0)),
            }
        }
        WM_PAINT => {
            w.paint(sh);
            Some(LRESULT(0))
        }
        WM_SIZE => {
            update_size(w, sh);
            Some(LRESULT(0))
        }
        WM_KEYDOWN => {
            let vk = u16::try_from(wp.0).unwrap_or(0);
            // SAFETY: GetKeyState has no preconditions.
            let (ctrl, shift) = unsafe {
                (
                    GetKeyState(i32::from(VK_CONTROL.0)) < 0,
                    GetKeyState(i32::from(VK_SHIFT.0)) < 0,
                )
            };
            actions.extend(w.key(sh, vk, ctrl, shift));
            Some(LRESULT(0))
        }
        // R-LINK-02: Alt+← / Alt+→ and mouse X1/X2 move through the history.
        // §3.9: Alt+↑ / Alt+↓ / Alt+0 change the column width.
        WM_SYSKEYDOWN if wp.0 == usize::from(VK_UP.0) => {
            actions.push(Action::Command(app::COL_WIDE));
            Some(LRESULT(0))
        }
        WM_SYSKEYDOWN if wp.0 == usize::from(VK_DOWN.0) => {
            actions.push(Action::Command(app::COL_NARROW));
            Some(LRESULT(0))
        }
        WM_SYSKEYDOWN if wp.0 == 0x30 || wp.0 == 0x60 => {
            actions.push(Action::Command(app::COL_RESET));
            Some(LRESULT(0))
        }
        // Shift+F10 opens the context menu (F10 arrives as a system key).
        WM_SYSKEYDOWN if wp.0 == 0x79 => {
            // SAFETY: GetKeyState has no preconditions.
            if unsafe { GetKeyState(i32::from(VK_SHIFT.0)) } < 0 {
                actions.push(w.kbd_menu());
                Some(LRESULT(0))
            } else {
                None
            }
        }
        WM_CONTEXTMENU => {
            if lp.0 == -1 {
                actions.push(w.kbd_menu());
            } else {
                let (sx, sy) = (
                    i32::from(lp.0 as u16 as i16),
                    i32::from((lp.0 >> 16) as u16 as i16),
                );
                let mut pt = POINT { x: sx, y: sy };
                // SAFETY: valid hwnd and point.
                let _ = unsafe { ScreenToClient(w.hwnd, &mut pt) };
                let link = w.link_at(sh, pt.x as f32 / w.scale, pt.y as f32 / w.scale);
                actions.push(Action::ContextMenu(sx, sy, link));
            }
            Some(LRESULT(0))
        }
        WM_MEASUREITEM => app::measure(w.hwnd, lp).then_some(LRESULT(1)),
        WM_DRAWITEM => app::draw_item(w.hwnd, lp).then_some(LRESULT(1)),
        WM_MENUCHAR => app::menu_char(wp, lp),
        WM_CHAR if w.find.is_some() => {
            if let Some(c) = char::from_u32(wp.0 as u32) {
                w.find_char(sh, c);
            }
            Some(LRESULT(0))
        }
        crate::shell::WM_APP_FILE_CHANGED => {
            // Debounce bursts of change notifications (atomic saves); R-READ-01 ≤ 100 ms.
            // SAFETY: valid hwnd.
            unsafe { SetTimer(Some(w.hwnd), TIMER_RELOAD, 50, None) };
            Some(LRESULT(0))
        }
        WM_SYSKEYDOWN if wp.0 == usize::from(VK_LEFT.0) || wp.0 == usize::from(VK_RIGHT.0) => {
            let dir = if wp.0 == usize::from(VK_LEFT.0) {
                -1
            } else {
                1
            };
            w.history_go(sh, dir);
            Some(LRESULT(0))
        }
        WM_XBUTTONUP => {
            let dir = if (wp.0 >> 16) & 0xFFFF == 1 { -1 } else { 1 };
            w.history_go(sh, dir);
            Some(LRESULT(1))
        }
        WM_COMMAND if (wp.0 & 0xFFFF) == IDM_OPEN => {
            actions.push(Action::OpenDialog);
            Some(LRESULT(0))
        }
        WM_MOUSEWHEEL if wp.0 & MK_CONTROL_BIT != 0 || alt_down() => {
            let delta = ((wp.0 >> 16) & 0xFFFF) as u16 as i16;
            let font = wp.0 & MK_CONTROL_BIT != 0;
            let id = match (font, delta > 0) {
                (true, true) => app::FONT_BIG,
                (true, false) => app::FONT_SMALL,
                (false, true) => app::COL_WIDE,
                (false, false) => app::COL_NARROW,
            };
            actions.push(Action::Command(id));
            Some(LRESULT(0))
        }
        WM_MOUSEWHEEL => {
            let delta = ((wp.0 >> 16) & 0xFFFF) as u16 as i16;
            let shift = wp.0 & 0x0004 != 0; // MK_SHIFT
            let (sx, sy) = (
                i32::from(lp.0 as u16 as i16),
                i32::from((lp.0 >> 16) as u16 as i16),
            );
            if !(shift && w.hwheel(sh, -delta, sx, sy)) {
                w.wheel(sh, delta);
            }
            Some(LRESULT(0))
        }
        WM_MOUSEHWHEEL => {
            let delta = ((wp.0 >> 16) & 0xFFFF) as u16 as i16;
            let (sx, sy) = (
                i32::from(lp.0 as u16 as i16),
                i32::from((lp.0 >> 16) as u16 as i16),
            );
            w.hwheel(sh, delta, sx, sy);
            Some(LRESULT(0))
        }
        WM_LBUTTONDOWN => {
            let (x, y) = lparam_xy(lp, w.scale);
            w.mouse_down(sh, x, y, wp.0 & MK_CONTROL_BIT != 0);
            Some(LRESULT(0))
        }
        // Middle-click on a Markdown link opens it in a new window.
        WM_MBUTTONUP => {
            let (x, y) = lparam_xy(lp, w.scale);
            if w.dialog.is_none() {
                w.middle_click(sh, x, y);
            }
            Some(LRESULT(0))
        }
        WM_MOUSEMOVE => {
            let (x, y) = lparam_xy(lp, w.scale);
            w.mouse_move(sh, x, y);
            Some(LRESULT(0))
        }
        WM_LBUTTONUP => {
            w.mouse_up();
            Some(LRESULT(0))
        }
        WM_MOUSELEAVE => {
            w.tracking = false;
            if w.sb.hover {
                w.sb.hover = false;
                w.invalidate_scrollbar();
            }
            Some(LRESULT(0))
        }
        WM_SETCURSOR if (lp.0 & 0xFFFF) as u32 == HTCLIENT => {
            let mut pt = POINT::default();
            // SAFETY: valid out-pointer / hwnd.
            unsafe {
                let _ = GetCursorPos(&mut pt);
                let _ = ScreenToClient(w.hwnd, &mut pt);
            }
            let (x, y) = (pt.x as f32 / w.scale, pt.y as f32 / w.scale);
            let hand = w.dialog.is_none() && w.over_hot(sh, x, y);
            let ibeam = !hand
                && w.dialog.is_none()
                && !(x >= w.cw - 16.0 && w.thumb().is_some())
                && w.over_text(sh, x, y);
            let cur = if hand {
                IDC_HAND
            } else if ibeam {
                IDC_IBEAM
            } else {
                IDC_ARROW
            };
            // SAFETY: system cursors; no cleanup needed.
            unsafe { SetCursor(LoadCursorW(None, cur).ok()) };
            Some(LRESULT(1))
        }
        WM_TIMER => {
            w.timer(sh, wp.0);
            Some(LRESULT(0))
        }
        WM_APP_WARM => {
            if let Some((st, sample)) = w.warm.pop() {
                sh.dw.warm(&st, sample);
                if !w.warm.is_empty() {
                    // SAFETY: valid hwnd.
                    let _ =
                        unsafe { PostMessageW(Some(w.hwnd), WM_APP_WARM, WPARAM(0), LPARAM(0)) };
                }
            }
            Some(LRESULT(0))
        }
        crate::images::WM_IMAGE_DONE => {
            if let View::Doc(d) = &mut w.view {
                let (any, relayout) = d.images.collect();
                if relayout {
                    d.viewer.invalidate_layouts();
                }
                if any {
                    w.invalidate();
                }
            }
            Some(LRESULT(0))
        }
        WM_APP_AUTOSCROLL => {
            w.autoscroll_step();
            Some(LRESULT(0))
        }
        WM_DROPFILES => {
            actions.extend(w.drop_files(sh, HDROP(wp.0 as *mut core::ffi::c_void)));
            Some(LRESULT(0))
        }
        WM_SETTINGCHANGE => {
            if is_immersive_color_set(lp) && sh.mode == "system" {
                let dark = app_mode() == Mode::Dark;
                if dark != w.dark {
                    w.set_dark(dark);
                }
            }
            None
        }
        WM_DPICHANGED => {
            let dpi = (wp.0 & 0xFFFF) as u32;
            w.scale = dpi.max(48) as f32 / 96.0;
            if let Some(r) = &mut w.renderer {
                r.set_scale(&sh.dw, w.hwnd, w.scale);
            }
            // SAFETY: for WM_DPICHANGED lParam points to the suggested window RECT.
            let rc = unsafe { *(lp.0 as *const RECT) };
            w.defer(Deferred::SetPos(
                rc.left,
                rc.top,
                rc.right - rc.left,
                rc.bottom - rc.top,
            ));
            update_size(w, sh);
            if let View::Doc(d) = &mut w.view {
                d.viewer.invalidate_layouts();
            }
            Some(LRESULT(0))
        }
        WM_GETMINMAXINFO => {
            // SAFETY: valid hwnd.
            let dpi = unsafe { GetDpiForWindow(w.hwnd) }.max(96);
            let (mw, mh) =
                window_rect_for_client(scale(MIN_CLIENT.0, dpi), scale(MIN_CLIENT.1, dpi), dpi);
            // SAFETY: for WM_GETMINMAXINFO lParam points to a writable MINMAXINFO.
            let mmi = unsafe { &mut *(lp.0 as *mut MINMAXINFO) };
            mmi.ptMinTrackSize = POINT { x: mw, y: mh };
            Some(LRESULT(0))
        }
        _ => None,
    };
    actions.append(&mut w.actions);
    (r, actions)
}

/// Deferred Win32 calls that dispatch messages synchronously (P-18 audit, ADR-0007 amendment).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Deferred {
    /// `UpdateWindow` (sends WM_PAINT).
    Update,
    /// `SetWindowTextW` (sends WM_SETTEXT).
    Title(String),
    /// `SetWindowPos` (sends WM_WINDOWPOSCHANGING/CHANGED, WM_SIZE, may paint).
    SetPos(i32, i32, i32, i32),
}

fn flush_deferred(hwnd: HWND, win: &Rc<RefCell<Win>>) {
    let ds = win
        .try_borrow_mut()
        .map(|mut w| std::mem::take(&mut w.deferred))
        .unwrap_or_default();
    run_deferred(hwnd, ds);
}

fn run_deferred(hwnd: HWND, ds: Vec<Deferred>) {
    for d in ds {
        // SAFETY: valid hwnd; no window-state borrow is held here.
        unsafe {
            match d {
                Deferred::Update => {
                    let _ = UpdateWindow(hwnd);
                }
                Deferred::Title(t) => {
                    let _ = SetWindowTextW(hwnd, &HSTRING::from(t));
                }
                Deferred::SetPos(x, y, w, h) => {
                    let _ = SetWindowPos(hwnd, None, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
                }
            }
        }
    }
}

fn run_actions(hwnd: HWND, actions: Vec<Action>) {
    run_actions_inner(hwnd, actions);
    if let Some(w) = WINDOWS.with(|m| m.borrow().get(&(hwnd.0 as isize)).cloned()) {
        flush_deferred(hwnd, &w);
    }
}

fn run_actions_inner(hwnd: HWND, actions: Vec<Action>) {
    let Some(sh) = shared() else { return };
    for a in actions {
        match a {
            Action::Close => {
                // SAFETY: destroying our own window.
                let _ = unsafe { DestroyWindow(hwnd) };
                return;
            }
            Action::NewWindow(p) => {
                create_window(&sh, Some(&p));
            }
            Action::ContextMenu(x, y, link) => {
                let Some((ctx, pal)) = WINDOWS
                    .with(|m| m.borrow().get(&(hwnd.0 as isize)).cloned())
                    .and_then(|w| {
                        w.try_borrow()
                            .ok()
                            .map(|w| (w.menu_ctx(link.clone()), w.pal))
                    })
                else {
                    continue;
                };
                let id = app::track(hwnd, x, y, &ctx, &pal);
                if id != 0 {
                    let more = app::run_command(hwnd, id, link);
                    run_actions_inner(hwnd, more);
                }
            }
            Action::Command(id) => {
                let more = app::run_command(hwnd, id, None);
                run_actions_inner(hwnd, more);
            }
            Action::Help => {
                if let Some(h) = create_window(&sh, None) {
                    app::with_win(h, |w, sh| w.show_help(sh));
                }
            }
            Action::FontDialog => app::font_dialog(hwnd),
            Action::Forward(paths, line) => app::open_forwarded(hwnd, paths, line),
            Action::OpenDialog => {
                if HOST.with(|h| h.borrow().is_some()) {
                    continue;
                }
                match crate::dialog_host::launch(hwnd) {
                    Some(h) => {
                        // SAFETY: valid hwnd; WM_ENABLE is not handled by us (no re-entry).
                        let _ = unsafe { EnableWindow(hwnd, false) };
                        HOST.with(|x| *x.borrow_mut() = Some(h));
                    }
                    None => show_toast(hwnd, Msg::DialogHostFailed),
                }
            }
            Action::HostResult(yamdv_core::ipc::Kind::Font, o) => {
                // SAFETY: valid hwnd.
                unsafe {
                    let _ = EnableWindow(hwnd, true);
                    let _ = SetForegroundWindow(hwnd);
                }
                if let yamdv_core::ipc::Outcome::Paths(p) = o
                    && let Some(name) = p.first()
                    && !name.is_empty()
                    && name.len() <= 64
                {
                    app::set("typography.body_font", name);
                }
            }
            Action::HostResult(_, o) => {
                // SAFETY: valid hwnd.
                unsafe {
                    let _ = EnableWindow(hwnd, true);
                    let _ = SetForegroundWindow(hwnd);
                }
                let paths = match o {
                    yamdv_core::ipc::Outcome::Paths(p) => p,
                    yamdv_core::ipc::Outcome::Cancel => Vec::new(),
                    yamdv_core::ipc::Outcome::Error => {
                        show_toast(hwnd, Msg::DialogHostFailed);
                        Vec::new()
                    }
                };
                // Returned paths are treated exactly like user-opened paths (S-IPC-05).
                let mut it = paths.into_iter();
                if let Some(first) = it.next() {
                    let win = WINDOWS.with(|m| m.borrow().get(&(hwnd.0 as isize)).cloned());
                    let in_place = win.as_ref().is_some_and(|w| {
                        w.try_borrow()
                            .is_ok_and(|w| !matches!(w.view, View::Doc(_)))
                    });
                    if in_place {
                        if let Some(w) = win
                            && let Ok(mut w) = w.try_borrow_mut()
                        {
                            w.open(&sh, &first, None, false);
                        }
                    } else {
                        app::open_or_activate(&first, None);
                    }
                }
                for p in it {
                    app::open_or_activate(&p, None);
                }
            }
        }
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_DESTROY {
        if let Some(w) = WINDOWS.with(|m| m.borrow().get(&(hwnd.0 as isize)).cloned())
            && let Ok(mut w) = w.try_borrow_mut()
        {
            w.save_position();
            app::save_geometry(hwnd);
            w.watcher = None;
            app::save_state();
        }
        WINDOWS.with(|m| m.borrow_mut().remove(&(hwnd.0 as isize)));
        if WINDOWS.with(|m| m.borrow().is_empty()) {
            // SAFETY: no preconditions; closing the last window exits.
            unsafe { PostQuitMessage(0) };
        }
        return LRESULT(0);
    }
    let win = WINDOWS.with(|m| m.borrow().get(&(hwnd.0 as isize)).cloned());
    if let (Some(win), Some(sh)) = (win, shared()) {
        let out = crate::reentry::dispatch(&win, msg, |w| handle(w, &sh, msg, wp, lp));
        // Re-entrant WM_PAINT while a handler holds the borrow: leave the region invalid so it is
        // painted once the handler returns (DefWindowProc would validate it unpainted).
        if out.is_none() && msg == WM_PAINT {
            return LRESULT(0);
        }
        if out.is_some() {
            flush_deferred(hwnd, &win);
        }
        if let Some((r, actions)) = out {
            run_actions(hwnd, actions);
            if let Some(r) = r {
                return r;
            }
        }
    }
    // SAFETY: default processing.
    unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
}

fn register_class() -> bool {
    // SAFETY: module handle of the running exe.
    let Ok(hinstance) = (unsafe { GetModuleHandleW(None) }) else {
        return false;
    };
    // SAFETY: icon resource 1 is embedded by build.rs; LR_SHARED handles need no cleanup.
    let icon = unsafe {
        LoadImageW(
            Some(hinstance.into()),
            PCWSTR(std::ptr::without_provenance(1)),
            IMAGE_ICON,
            0,
            0,
            LR_DEFAULTSIZE | LR_SHARED,
        )
    }
    .map(|h| HICON(h.0))
    .unwrap_or_default();
    let wc = WNDCLASSEXW {
        cbSize: u32::try_from(size_of::<WNDCLASSEXW>()).unwrap_or(0),
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wndproc),
        hInstance: hinstance.into(),
        hIcon: icon,
        hIconSm: icon,
        // SAFETY: system cursor; no cleanup needed.
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
        lpszClassName: CLASS_NAME,
        ..Default::default()
    };
    // SAFETY: wc is fully initialized and its strings are static.
    unsafe { RegisterClassExW(&wc) != 0 }
}

fn create_window(sh: &Shared, path: Option<&str>) -> Option<HWND> {
    // SAFETY: module handle of the running exe.
    let Ok(hinstance) = (unsafe { GetModuleHandleW(None) }) else {
        return None;
    };
    let mut pt = POINT::default();
    // SAFETY: valid out-pointer.
    let _ = unsafe { GetCursorPos(&mut pt) };
    // SAFETY: no preconditions.
    let mon = unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST) };
    let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
    // SAFETY: valid monitor handle and out-pointers.
    let _ = unsafe { GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
    let mut mi = MONITORINFO {
        cbSize: u32::try_from(size_of::<MONITORINFO>()).unwrap_or(0),
        ..Default::default()
    };
    // SAFETY: valid monitor handle and correctly sized MONITORINFO.
    let _ = unsafe { GetMonitorInfoW(mon, &mut mi) };
    let work = mi.rcWork;
    let (mut w, mut h) = window_rect_for_client(
        scale(INITIAL_CLIENT.0, dpi_x),
        scale(INITIAL_CLIENT.1, dpi_x),
        dpi_x,
    );
    w = w.min(work.right - work.left);
    h = h.min(work.bottom - work.top);
    let n = WINDOWS.with(|m| m.borrow().len()) as i32;
    let off = (n % 8) * scale(28, dpi_x);
    let mut x = work.left + (work.right - work.left - w) / 2 + off;
    let mut y = work.top + (work.bottom - work.top - h) / 2 + off;
    let mut maximize = false;
    // R-WIN-03: the remembered geometry, validated against the current monitors.
    if let Some(g) = app::restored_geometry() {
        x = g.x + off;
        y = g.y + off;
        w = g.w;
        h = g.h;
        maximize = g.max && n == 0;
    }
    // SAFETY: class registered; title is static.
    let Ok(hwnd) = (unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            CLASS_NAME,
            w!("YAMDV"),
            WS_OVERLAPPEDWINDOW,
            x,
            y,
            w,
            h,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
    }) else {
        return None;
    };
    // SAFETY: valid hwnd.
    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
    let scale_f = dpi as f32 / 96.0;
    let dark = app::is_dark(&sh.mode);
    let mut rc = RECT::default();
    // SAFETY: valid hwnd and out-pointer.
    let _ = unsafe { GetClientRect(hwnd, &mut rc) };
    let renderer = Renderer::new(&sh.dw, Some(hwnd), (rc.right - rc.left).max(1), scale_f).ok();
    let win = Win {
        hwnd,
        scale: scale_f,
        dark,
        pal: app::palette_for(&sh.scheme, dark).0,
        cw: (rc.right - rc.left) as f32 / scale_f,
        ch: (rc.bottom - rc.top) as f32 / scale_f,
        view: View::Empty { note: None },
        sel: None,
        selecting: false,
        clicks: (0, 0, 0, 0),
        anim: None,
        sb: Scrollbar::default(),
        tracking: false,
        dialog: None,
        renderer,
        first_paint: false,
        warm: Vec::new(),
        warm_pending: false,
        toast: None,
        toast_arg: String::new(),
        history: Vec::new(),
        hist_pos: 0,
        code_hover: None,
        code_copied: None,
        hdrag: None,
        drag_dir: 0.0,
        perf: Vec::new(),
        auto_phase: 0,
        deferred: Vec::new(),
        auto_steps: 0,
        focus: None,
        actions: Vec::new(),
        dimmed: false,
        watcher: None,
        find: None,
    };
    let rcw = Rc::new(RefCell::new(win));
    WINDOWS.with(|m| m.borrow_mut().insert(hwnd.0 as isize, rcw.clone()));
    apply_mode(hwnd, if dark { Mode::Dark } else { Mode::Light });
    // SAFETY: valid hwnd.
    unsafe { DragAcceptFiles(hwnd, true) };
    if let Some(p) = path
        && let Ok(mut w) = rcw.try_borrow_mut()
    {
        w.open(sh, p, None, false);
    }
    flush_deferred(hwnd, &rcw);
    // SAFETY: valid hwnd.
    let _ = unsafe { ShowWindow(hwnd, if maximize { SW_SHOWMAXIMIZED } else { SW_SHOW }) };
    Some(hwnd)
}

/// Reads `settings.ini` (read-only in this phase, R-SET-05) and returns the settings.
pub(crate) fn load_settings() -> Settings {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_string_lossy().into_owned()))
        .unwrap_or_default();
    let portable = std::path::Path::new(&exe_dir)
        .join("yamdv.portable")
        .exists();
    let dirs = yamdv_core::settings::resolve_dirs(
        &exe_dir,
        portable,
        &std::env::var("APPDATA").unwrap_or_default(),
        &std::env::var("LOCALAPPDATA").unwrap_or_default(),
    );
    let dir = match dirs {
        yamdv_core::settings::DataDirs::Installed { config, .. } => config,
        yamdv_core::settings::DataDirs::Portable { dir } => dir,
    };
    let path = std::path::Path::new(&dir).join("settings.ini");
    let bytes = std::fs::metadata(&path)
        .ok()
        .filter(|m| m.len() <= 4 * 1024 * 1024)
        .and_then(|_| std::fs::read(&path).ok())
        .unwrap_or_default();
    Settings::from_ini(&yamdv_core::ini::Ini::parse(&bytes))
}

/// Builds the process-wide state from `s`; reuses `dw` when the fonts did not change.
fn build_shared(
    s: &Settings,
    dw: Option<(Rc<DWrite>, (String, String))>,
    dev: DevOptions,
    commit_at_start: u64,
) -> Option<Shared> {
    let fonts = (
        s.str("typography.body_font").to_string(),
        s.str("typography.code_font").to_string(),
    );
    let dw = match dw {
        Some((d, f)) if f == fonts => d,
        _ => Rc::new(DWrite::new(&fonts.0, &fonts.1).ok()?),
    };
    let m = Metrics {
        font_size: s.int("typography.font_size") as f32,
        line_height: s.float("typography.line_height") as f32,
        para_spacing: s.int("typography.paragraph_spacing") as f32,
        table_zebra: s.bool("appearance.table_zebra"),
        rounded: s.str("appearance.corner_style") != "sharp",
    };
    Some(Shared {
        dw,
        scheme: s.str("appearance.scheme").to_string(),
        mode: s.str("appearance.mode").to_string(),
        fonts,
        restore_pos: s.bool("general.restore_reading_position"),
        lang: yamdv_core::i18n::resolve(s.str("general.language"), sys::ui_language_tag()),
        m,
        read: ReadOpts {
            subscript_tilde: s.bool("reading.subscript_tilde"),
            smart_punctuation: s.bool("reading.smart_punctuation"),
            single_newline_break: s.bool("reading.single_newline_break"),
            code_line_numbers: s.bool("reading.code_line_numbers"),
            code_wrap: s.bool("reading.code_wrap"),
            front_matter: match s.str("reading.front_matter") {
                "expanded" => 1,
                "hidden" => 2,
                _ => 0,
            },
        },
        column_pct: s.int("typography.column_width_pct") as f32 / 100.0,
        smooth: s.bool("reading.smooth_scrolling"),
        external: s.str("general.open_external_links").to_string(),
        autohide: s.bool("reading.scrollbar_autohide"),
        dev,
        commit_at_start,
    })
}

/// Opens one window per path (or one empty window) and runs the message loop.
pub fn run(paths: &[String], dev: DevOptions, line: Option<u32>) -> i32 {
    let commit_at_start = crate::perf::commit_bytes();
    // S-IPC-01 / R-WIN-01: a second launch forwards its paths and exits.
    let instance = crate::shell::claim_instance();
    if instance.is_none() {
        let full: Vec<String> = paths
            .iter()
            .map(|p| sys::full_path(p).unwrap_or_else(|| p.clone()))
            .collect();
        if crate::shell::forward(&full, u64::from(line.unwrap_or(0))) {
            return 0;
        }
    }
    // SAFETY: COM for IFileOpenDialog on this (UI) thread.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let s = app::init();
    let Some(shared) = build_shared(&s, None, dev, commit_at_start) else {
        return 1;
    };
    drop(s);
    let sh = Rc::new(shared);
    SHARED.with(|c| *c.borrow_mut() = Some(sh.clone()));
    if !register_class() {
        return 1;
    }
    if paths.is_empty() {
        create_window(&sh, None);
    } else {
        for p in paths {
            app::open_or_activate(p, line);
        }
    }
    let _ = PENDING_OPEN.with(|p| p.borrow().len());
    let mut msg = MSG::default();
    // Message loop that also waits on the dialog host's process handle (F-09).
    loop {
        let h = HOST.with(|h| h.borrow().as_ref().map(|h| h.process));
        if let Some(ph) = h {
            // SAFETY: valid process handle; returns on input or when the host exits.
            let r = unsafe {
                MsgWaitForMultipleObjectsEx(Some(&[ph]), INFINITE, QS_ALLINPUT, MWMO_INPUTAVAILABLE)
            };
            if r == WAIT_OBJECT_0 {
                host_exited();
                continue;
            }
        } else {
            // SAFETY: blocks until a message is available.
            let _ = unsafe { WaitMessage() };
        }
        // SAFETY: standard message pump; msg is a valid out-pointer.
        while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
            if msg.message == WM_QUIT {
                return i32::try_from(msg.wParam.0).unwrap_or(0);
            }
            // SAFETY: msg was filled by PeekMessageW.
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}

/// The dialog host process ended: re-enable the owner if no answer arrived; toast on failure.
fn host_exited() {
    let Some(h) = HOST.with(|h| h.borrow_mut().take()) else {
        return;
    };
    if h.answered {
        return;
    }
    // SAFETY: valid (or stale, then harmless) hwnd.
    let _ = unsafe { EnableWindow(h.owner, true) };
    let code = crate::dialog_host::exit_code(&h);
    crate::log::warn(
        "dialog-host",
        &format!("exited without an answer, code {code:?}"),
    );
    show_toast(h.owner, Msg::DialogHostFailed);
}

/// Shows a transient toast in `hwnd` (B.10).
fn show_toast(hwnd: HWND, m: Msg) {
    let Some(win) = WINDOWS.with(|x| x.borrow().get(&(hwnd.0 as isize)).cloned()) else {
        return;
    };
    if let Ok(mut w) = win.try_borrow_mut() {
        w.toast(m);
    }
}

/// Selection range helper for tests and tooling.
pub fn normalize(sel: (u32, u32)) -> Range<u32> {
    sel.0.min(sel.1)..sel.0.max(sel.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpi_scaling() {
        assert_eq!(scale(1100, 96), 1100);
        assert_eq!(scale(1100, 144), 1650);
    }

    #[test]
    fn b61_column() {
        assert_eq!(column(1000.0, 0.65), (175.0, 650.0));
        assert_eq!(column(400.0, 0.65), (24.0, 352.0));
        assert_eq!(column(2000.0, 1.0).1, 1952.0);
    }

    #[test]
    fn easing() {
        assert_eq!(ease_out(0.0), 0.0);
        assert_eq!(ease_out(1.0), 1.0);
        assert!(ease_out(0.5) > 0.5);
    }
}
