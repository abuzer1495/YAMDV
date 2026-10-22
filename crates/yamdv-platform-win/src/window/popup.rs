//! Layered popup windows (B.7.1, BUG-09): the context / ⋯ / ⚙ menus and the color-scheme card
//! flyout (B.7.3, FUT-32), replacing `TrackPopupMenuEx` owner-draw. One popup tree is open at a
//! time; it is not modal: the owner keeps the keyboard focus and forwards keys here, the root
//! popup holds the mouse capture (outside clicks close it), and a choice is posted back to the
//! owner as `WM_APP_MENU` so commands run outside any borrow.

use std::time::Instant;

use super::app::{self, Entry, Model};
use super::*;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Dwm::{
    DWMWA_BORDER_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DWMWCP_ROUND,
};
use windows::Win32::Graphics::Gdi::{ClientToScreen, MonitorFromPoint};
use windows::Win32::UI::Input::KeyboardAndMouse::GetCapture;
use windows::Win32::UI::WindowsAndMessaging::{
    CS_DROPSHADOW, DestroyWindow, HWND_TOPMOST, LWA_ALPHA, SW_SHOWNOACTIVATE,
    SWP_NOACTIVATE as SWP_NA, SetLayeredWindowAttributes, WM_CAPTURECHANGED, WM_LBUTTONDOWN as LBD,
    WM_LBUTTONUP as LBU, WM_MOUSEMOVE as MM, WM_NCHITTEST, WM_PAINT as PAINT, WM_RBUTTONDOWN,
    WM_RBUTTONUP, WM_TIMER as TIMER, WNDCLASSEXW as WC, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use yamdv_core::chrome as ch;

/// Posted to the owner when an item was chosen: `wParam` = command id.
pub(super) const WM_APP_MENU: u32 = WM_APP + 7;
/// Posted to the owner when the popup must close outside any borrow (capture lost).
pub(super) const WM_APP_POPUP_CLOSE: u32 = WM_APP + 8;

const CLASS: PCWSTR = w!("YAMDV.Popup");
const TIMER_SUB: usize = 1;
const TIMER_FADE: usize = 2;
const ITEM_H: f32 = 32.0;
const SEP_H: f32 = 9.0;
const PAD: f32 = 4.0;
const FADE_MS: f64 = 80.0;

/// What the popup shows.
enum Kind {
    Menu(Model),
    /// Scheme cards: the persisted scheme (revert target), its index and the previewed card.
    Cards {
        orig: String,
        sel: usize,
        preview: Option<usize>,
    },
}

struct Level {
    /// Menu index in the model (0 for the cards).
    menu: usize,
    hwnd: HWND,
    /// Screen rectangle in pixels.
    rect: ch::Px,
    sel: Option<usize>,
    renderer: Option<Renderer>,
    opened: Instant,
    alpha: u8,
    /// Width in DIPs.
    w: f32,
}

struct Popup {
    owner: HWND,
    kind: Kind,
    levels: Vec<Level>,
    link: Option<String>,
    scale: f32,
    pal: Palette,
    dark: bool,
    rounded: bool,
    instant: bool,
    /// Hovered entry waiting for the 150 ms submenu delay: (level, entry).
    pending: Option<(usize, usize)>,
    /// Keyboard navigation happened (selection shows without hover).
    kbd: bool,
}

thread_local! {
    static POPUP: RefCell<Option<Popup>> = const { RefCell::new(None) };
    /// The link of the menu whose item was chosen (Copy link address).
    static LINK: RefCell<Option<String>> = const { RefCell::new(None) };
    static REGISTERED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// A popup is open.
pub(super) fn is_open() -> bool {
    POPUP.with(|p| p.try_borrow().map(|p| p.is_some()).unwrap_or(true))
}

/// The open popup belongs to `owner`.
pub(super) fn owned_by(owner: HWND) -> bool {
    POPUP.with(|p| {
        p.try_borrow()
            .is_ok_and(|p| p.as_ref().is_some_and(|p| p.owner == owner))
    })
}

/// The link of the last chosen menu (taken once).
pub(super) fn take_link() -> Option<String> {
    LINK.with(|l| l.borrow_mut().take())
}

fn register() -> bool {
    if REGISTERED.with(std::cell::Cell::get) {
        return true;
    }
    // SAFETY: module handle of the running exe.
    let Ok(hinstance) = (unsafe { GetModuleHandleW(None) }) else {
        return false;
    };
    let wc = WC {
        cbSize: u32::try_from(size_of::<WC>()).unwrap_or(0),
        style: CS_DROPSHADOW,
        lpfnWndProc: Some(wndproc),
        hInstance: hinstance.into(),
        // SAFETY: system cursor; no cleanup needed.
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
        lpszClassName: CLASS,
        ..Default::default()
    };
    // SAFETY: wc is fully initialized and its strings are static.
    let ok = unsafe { RegisterClassExW(&wc) != 0 };
    REGISTERED.with(|r| r.set(ok));
    ok
}

fn shape(
    sh: &Shared,
    text: &str,
    size: f32,
    weight: u16,
    color: u32,
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
        width: None,
        line_pitch: (size * 1.4).round(),
        rtl: false,
        tab_spaces: None,
        objects: Vec::new(),
    })
}

/// Entry heights of `entries` (DIPs).
fn heights(entries: &[Entry]) -> impl Iterator<Item = f32> + '_ {
    entries.iter().map(|e| match e {
        Entry::Sep => SEP_H,
        Entry::Item { .. } => ITEM_H,
    })
}

/// Size of a menu (DIPs): width 260…360 (B.7.1).
fn menu_size(sh: &Shared, entries: &[Entry]) -> (f32, f32) {
    let mut w: f32 = 260.0;
    for e in entries {
        if let Entry::Item { label, key, .. } = e {
            let (lw, _) = sh.dw.measure(label, 13.0, 400, 10_000.0);
            let kw = if key.is_empty() {
                0.0
            } else {
                24.0 + sh.dw.measure(key, 12.0, 400, 10_000.0).0
            };
            w = w.max(44.0 + lw + kw + 32.0);
        }
    }
    (w.min(360.0).ceil(), 2.0 * PAD + heights(entries).sum::<f32>())
}

/// Entry index at popup-local DIP y.
fn entry_at(entries: &[Entry], y: f32) -> Option<usize> {
    let mut top = PAD;
    for (i, h) in heights(entries).enumerate() {
        if y >= top && y < top + h {
            return Some(i);
        }
        top += h;
    }
    None
}

/// Top of entry `i` (popup-local DIPs).
fn entry_top(entries: &[Entry], i: usize) -> f32 {
    PAD + heights(entries).take(i).sum::<f32>()
}

fn work_area(pt: POINT) -> ch::Px {
    // SAFETY: no preconditions.
    let mon = unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST) };
    let mut mi = MONITORINFO {
        cbSize: u32::try_from(size_of::<MONITORINFO>()).unwrap_or(0),
        ..Default::default()
    };
    // SAFETY: valid monitor handle and sized structure.
    let _ = unsafe { GetMonitorInfoW(mon, &mut mi) };
    let r = mi.rcWork;
    (r.left, r.top, r.right, r.bottom)
}

fn colorref(rgb: u32) -> COLORREF {
    COLORREF(((rgb >> 16) & 0xff) | (rgb & 0xff00) | ((rgb & 0xff) << 16))
}

/// Creates one popup window at `rect` (screen px); not shown yet.
fn create(owner: HWND, rect: ch::Px, rounded: bool, border: u32, instant: bool) -> Option<HWND> {
    if !register() {
        return None;
    }
    // SAFETY: module handle of the running exe.
    let hinstance = unsafe { GetModuleHandleW(None) }.ok()?;
    // SAFETY: class registered; owner is a live window of this thread.
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            CLASS,
            w!(""),
            WS_POPUP,
            rect.0,
            rect.1,
            rect.2 - rect.0,
            rect.3 - rect.1,
            Some(owner),
            None,
            Some(hinstance.into()),
            None,
        )
    }
    .ok()?;
    let corner = if rounded {
        DWMWCP_ROUND
    } else {
        DWMWCP_DONOTROUND
    };
    let color = colorref(border);
    // SAFETY: valid hwnd; attribute values live on the stack for the calls.
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&raw const corner).cast(),
            size_of_val(&corner) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR,
            (&raw const color).cast(),
            size_of_val(&color) as u32,
        );
        let _ = SetLayeredWindowAttributes(
            hwnd,
            COLORREF(0),
            if instant { 255 } else { 0 },
            LWA_ALPHA,
        );
    }
    Some(hwnd)
}

fn show(hwnd: HWND) {
    // SAFETY: valid hwnd; shown without activation (the owner keeps the focus).
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NA | windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE
                | windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE,
        );
    }
}

/// Base fields shared by menus and cards.
struct Base {
    owner: HWND,
    scale: f32,
    pal: Palette,
    dark: bool,
}

fn base(owner: HWND) -> Option<Base> {
    let win = WINDOWS.with(|m| m.borrow().get(&(owner.0 as isize)).cloned())?;
    let w = win.try_borrow().ok()?;
    Some(Base {
        owner,
        scale: w.scale,
        pal: w.pal,
        dark: w.dark,
    })
}

fn rounded() -> bool {
    app::get("appearance.corner_style") != "sharp"
}

fn instant() -> bool {
    !sys::animations_enabled()
}

/// Opens `model` at screen point `anchor`; `right` aligns the menu's right edge with the
/// anchor (⋯ button). Closes any open popup first.
pub(super) fn open_menu(
    owner: HWND,
    anchor: (i32, i32),
    right: bool,
    model: Model,
    link: Option<String>,
) {
    close();
    let (Some(sh), Some(b)) = (shared(), base(owner)) else {
        return;
    };
    let Some(entries) = model.menus.first() else {
        return;
    };
    let (wd, hd) = menu_size(&sh, entries);
    let (wp, hp) = ((wd * b.scale).round() as i32, (hd * b.scale).round() as i32);
    let a = if right { (anchor.0 - wp, anchor.1) } else { anchor };
    let (x, y) = ch::place(
        a,
        (wp, hp),
        work_area(POINT {
            x: anchor.0,
            y: anchor.1,
        }),
        None,
    );
    let rect = (x, y, x + wp, y + hp);
    let tok = ch::tokens(b.dark);
    let (rd, inst) = (rounded(), instant());
    let Some(hwnd) = create(owner, rect, rd, tok.menu_border, inst) else {
        return;
    };
    let renderer = Renderer::new(&sh.dw, Some(hwnd), wp.max(1), b.scale).ok();
    POPUP.with(|p| {
        *p.borrow_mut() = Some(Popup {
            owner: b.owner,
            kind: Kind::Menu(model),
            levels: vec![Level {
                menu: 0,
                hwnd,
                rect,
                sel: None,
                renderer,
                opened: Instant::now(),
                alpha: if inst { 255 } else { 0 },
                w: wd,
            }],
            link,
            scale: b.scale,
            pal: b.pal,
            dark: b.dark,
            rounded: rd,
            instant: inst,
            pending: None,
            kbd: false,
        });
    });
    show(hwnd);
    start(hwnd, inst);
}

/// Opens the color-scheme card flyout (B.7.3) at `anchor`.
pub(super) fn open_cards(owner: HWND, anchor: (i32, i32)) {
    close();
    let (Some(sh), Some(b)) = (shared(), base(owner)) else {
        return;
    };
    let orig = app::get("appearance.scheme");
    let sel = app::SCHEMES
        .iter()
        .position(|s| s.0 == orig)
        .unwrap_or(0);
    let (wd, hd) = ch::cards_size(app::SCHEMES.len());
    let (wp, hp) = ((wd * b.scale).round() as i32, (hd * b.scale).round() as i32);
    let (x, y) = ch::place(
        anchor,
        (wp, hp),
        work_area(POINT {
            x: anchor.0,
            y: anchor.1,
        }),
        None,
    );
    let rect = (x, y, x + wp, y + hp);
    let tok = ch::tokens(b.dark);
    let (rd, inst) = (rounded(), instant());
    let Some(hwnd) = create(owner, rect, rd, tok.menu_border, inst) else {
        return;
    };
    let renderer = Renderer::new(&sh.dw, Some(hwnd), wp.max(1), b.scale).ok();
    POPUP.with(|p| {
        *p.borrow_mut() = Some(Popup {
            owner: b.owner,
            kind: Kind::Cards {
                orig,
                sel,
                preview: None,
            },
            levels: vec![Level {
                menu: 0,
                hwnd,
                rect,
                sel: Some(sel),
                renderer,
                opened: Instant::now(),
                alpha: if inst { 255 } else { 0 },
                w: wd,
            }],
            link: None,
            scale: b.scale,
            pal: b.pal,
            dark: b.dark,
            rounded: rd,
            instant: inst,
            pending: None,
            kbd: false,
        });
    });
    show(hwnd);
    start(hwnd, inst);
}

fn start(root: HWND, inst: bool) {
    // SAFETY: valid hwnd of this thread; the fade timer runs only while fading (T7).
    unsafe {
        SetCapture(root);
        if !inst {
            SetTimer(Some(root), TIMER_FADE, 15, None);
        }
    }
}

/// Closes the popup tree; a previewed scheme is reverted.
pub(super) fn close() {
    let Some(p) = POPUP.with(|p| p.try_borrow_mut().ok().and_then(|mut p| p.take())) else {
        return;
    };
    if let Kind::Cards {
        orig,
        preview: Some(_),
        ..
    } = &p.kind
    {
        app::set_transient("appearance.scheme", orig);
    }
    destroy(p);
}

fn destroy(p: Popup) {
    // SAFETY: windows of this thread; the state was taken out first (no re-entrant borrow).
    unsafe {
        if p.levels.first().is_some_and(|l| GetCapture() == l.hwnd) {
            let _ = ReleaseCapture();
        }
        for l in p.levels.iter().rev() {
            let _ = DestroyWindow(l.hwnd);
        }
    }
}

/// Closes the popup if it belongs to `owner` (deactivation, destruction).
pub(super) fn close_for(owner: HWND) {
    if owned_by(owner) {
        close();
    }
}

/// What a handler decided; run after the state borrow is released.
enum Do {
    Nothing,
    Close,
    Choose(u32),
    /// Open the submenu of (level, entry); `kbd` selects its first item.
    OpenSub(usize, usize, bool),
    /// Preview card `Some(i)` or revert (`None`).
    Preview(Option<usize>),
    /// Apply card `i` and close.
    Apply(usize),
    Repaint,
}

fn run(d: Do) {
    match d {
        Do::Nothing => {}
        Do::Close => close(),
        Do::Choose(id) => {
            let Some(p) = POPUP.with(|p| p.try_borrow_mut().ok().and_then(|mut p| p.take())) else {
                return;
            };
            let owner = p.owner;
            LINK.with(|l| *l.borrow_mut() = p.link.clone());
            destroy(p);
            // SAFETY: valid owner hwnd; the command runs from its message loop.
            let _ = unsafe { PostMessageW(Some(owner), WM_APP_MENU, WPARAM(id as usize), LPARAM(0)) };
        }
        Do::OpenSub(lvl, i, kbd) => open_sub(lvl, i, kbd),
        Do::Preview(i) => {
            let name = POPUP.with(|p| {
                let mut p = p.try_borrow_mut().ok()?;
                let p = p.as_mut()?;
                let Kind::Cards { orig, preview, .. } = &mut p.kind else {
                    return None;
                };
                if *preview == i {
                    return None;
                }
                *preview = i;
                Some(match i {
                    Some(k) => app::SCHEMES.get(k).map_or(orig.clone(), |s| s.0.to_string()),
                    None => orig.clone(),
                })
            });
            if let Some(n) = name {
                app::set_transient("appearance.scheme", &n);
                repaint_all();
            }
        }
        Do::Apply(i) => {
            let Some(p) = POPUP.with(|p| p.try_borrow_mut().ok().and_then(|mut p| p.take())) else {
                return;
            };
            destroy(p);
            if let Some(s) = app::SCHEMES.get(i) {
                app::set("appearance.scheme", s.0);
            }
        }
        Do::Repaint => repaint_all(),
    }
}

fn repaint_all() {
    let hs: Vec<HWND> = POPUP.with(|p| {
        p.try_borrow()
            .ok()
            .and_then(|p| p.as_ref().map(|p| p.levels.iter().map(|l| l.hwnd).collect()))
            .unwrap_or_default()
    });
    for h in hs {
        // SAFETY: valid hwnd.
        let _ = unsafe { InvalidateRect(Some(h), None, false) };
    }
}

fn open_sub(lvl: usize, i: usize, kbd: bool) {
    let Some(sh) = shared() else { return };
    // Close deeper levels first (outside the borrow: DestroyWindow sends messages).
    let extra: Vec<HWND> = POPUP.with(|p| {
        let Ok(mut p) = p.try_borrow_mut() else {
            return Vec::new();
        };
        let Some(p) = p.as_mut() else {
            return Vec::new();
        };
        p.levels.drain((lvl + 1).min(p.levels.len())..).map(|l| l.hwnd).collect()
    });
    for h in extra {
        // SAFETY: popup window of this thread, already removed from the state.
        let _ = unsafe { DestroyWindow(h) };
    }
    let plan = POPUP.with(|p| {
        let p = p.try_borrow().ok()?;
        let p = p.as_ref()?;
        let Kind::Menu(m) = &p.kind else { return None };
        let l = p.levels.get(lvl)?;
        let entries = m.menus.get(l.menu)?;
        let Some(Entry::Item { sub: Some(s), .. }) = entries.get(i) else {
            return None;
        };
        let sub = m.menus.get(*s)?;
        let (wd, hd) = menu_size(&sh, sub);
        let (wp, hp) = ((wd * p.scale).round() as i32, (hd * p.scale).round() as i32);
        let top = l.rect.1 + ((entry_top(entries, i) - PAD) * p.scale).round() as i32;
        let row = (l.rect.0, top, l.rect.2, top + (ITEM_H * p.scale).round() as i32);
        let (x, y) = ch::place((row.2, row.1), (wp, hp), work_area(POINT { x: row.2, y: row.1 }), Some(row));
        Some((
            *s,
            (x, y, x + wp, y + hp),
            wd,
            p.owner,
            p.rounded,
            p.instant,
            ch::tokens(p.dark).menu_border,
            p.scale,
            sub.iter().map(|e| matches!(e, Entry::Item { .. })).collect::<Vec<bool>>(),
        ))
    });
    let Some((menu, rect, wd, owner, rd, inst, border, scale, selectable)) = plan else {
        return;
    };
    let Some(hwnd) = create(owner, rect, rd, border, inst) else {
        return;
    };
    let renderer = Renderer::new(&sh.dw, Some(hwnd), (rect.2 - rect.0).max(1), scale).ok();
    let root = POPUP.with(|p| {
        let mut p = p.try_borrow_mut().ok()?;
        let p = p.as_mut()?;
        p.levels.push(Level {
            menu,
            hwnd,
            rect,
            sel: if kbd { ch::step(&selectable, None, 1) } else { None },
            renderer,
            opened: Instant::now(),
            alpha: if inst { 255 } else { 0 },
            w: wd,
        });
        p.levels.first().map(|l| l.hwnd)
    });
    show(hwnd);
    if let Some(r) = root
        && !inst
    {
        // SAFETY: valid hwnd; runs only while fading.
        unsafe { SetTimer(Some(r), TIMER_FADE, 15, None) };
    }
}

// ---- input ----------------------------------------------------------------------------------

/// Selectable flags and labels of level `lvl`.
fn level_info(p: &Popup, lvl: usize) -> Option<(Vec<bool>, Vec<Option<String>>)> {
    let Kind::Menu(m) = &p.kind else { return None };
    let entries = m.menus.get(p.levels.get(lvl)?.menu)?;
    Some((
        entries
            .iter()
            .map(|e| matches!(e, Entry::Item { .. }))
            .collect(),
        entries
            .iter()
            .map(|e| match e {
                Entry::Item { label, .. } => Some(label.clone()),
                Entry::Sep => None,
            })
            .collect(),
    ))
}

/// Activates entry `i` of level `lvl` (Enter / click).
fn activate(p: &Popup, lvl: usize, i: usize, kbd: bool) -> Do {
    let Kind::Menu(m) = &p.kind else {
        return Do::Nothing;
    };
    let Some(l) = p.levels.get(lvl) else {
        return Do::Nothing;
    };
    match m.menus.get(l.menu).and_then(|e| e.get(i)) {
        Some(Entry::Item { sub: Some(_), .. }) => Do::OpenSub(lvl, i, kbd),
        Some(Entry::Item { id, .. }) => Do::Choose(*id),
        _ => Do::Nothing,
    }
}

/// Keyboard input forwarded by the owner (`WM_KEYDOWN` / `WM_SYSKEYDOWN` / `WM_CHAR`).
/// Returns `true` when the key was consumed.
pub(super) fn key(msg: u32, wp: WPARAM) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        WM_KEYUP, WM_SYSCHAR, WM_SYSKEYDOWN as SKD, WM_SYSKEYUP,
    };
    if !is_open() {
        return false;
    }
    let d = POPUP.with(|p| {
        let Ok(mut pb) = p.try_borrow_mut() else {
            return Do::Nothing;
        };
        let Some(p) = pb.as_mut() else {
            return Do::Nothing;
        };
        p.kbd = true;
        let lvl = p.levels.len().saturating_sub(1);
        if msg == WM_KEYUP || msg == WM_SYSKEYUP {
            return Do::Nothing;
        }
        if let Kind::Cards { sel, .. } = &p.kind {
            let n = app::SCHEMES.len();
            let cur = p.levels.first().and_then(|l| l.sel).unwrap_or(*sel);
            if msg == WM_CHAR || msg == WM_SYSCHAR {
                return Do::Nothing;
            }
            let (dx, dy) = match wp.0 as u16 {
                v if v == VK_LEFT.0 => (-1, 0),
                v if v == VK_RIGHT.0 => (1, 0),
                v if v == VK_UP.0 => (0, -1),
                v if v == VK_DOWN.0 => (0, 1),
                v if v == VK_RETURN.0 || v == VK_SPACE.0 => return Do::Apply(cur),
                v if v == VK_ESCAPE.0 || v == 0x12 => return Do::Close,
                _ => return Do::Nothing,
            };
            let next = ch::card_move(cur, dx, dy, n);
            if let Some(l) = p.levels.first_mut() {
                l.sel = Some(next);
            }
            return Do::Preview(Some(next));
        }
        let Some((selectable, labels)) = level_info(p, lvl) else {
            return Do::Nothing;
        };
        let cur = p.levels.get(lvl).and_then(|l| l.sel);
        if msg == WM_CHAR || msg == WM_SYSCHAR {
            let Some(c) = char::from_u32(wp.0 as u32).filter(|c| c.is_alphanumeric()) else {
                return Do::Nothing;
            };
            let refs: Vec<Option<&str>> = labels.iter().map(|l| l.as_deref()).collect();
            if let Some(i) = ch::jump(&refs, cur, c) {
                if let Some(l) = p.levels.get_mut(lvl) {
                    l.sel = Some(i);
                }
                return Do::Repaint;
            }
            return Do::Nothing;
        }
        let _ = SKD;
        let set = |p: &mut Popup, i: Option<usize>| {
            if let Some(l) = p.levels.get_mut(lvl) {
                l.sel = i;
            }
            Do::Repaint
        };
        match wp.0 as u16 {
            v if v == VK_DOWN.0 => set(p, ch::step(&selectable, cur, 1)),
            v if v == VK_UP.0 => set(p, ch::step(&selectable, cur, -1)),
            v if v == VK_HOME.0 => set(p, ch::edge(&selectable, false)),
            v if v == VK_END.0 => set(p, ch::edge(&selectable, true)),
            v if v == VK_RIGHT.0 => match cur {
                Some(i) if matches!(activate(p, lvl, i, true), Do::OpenSub(..)) => {
                    Do::OpenSub(lvl, i, true)
                }
                _ => Do::Nothing,
            },
            v if v == VK_RETURN.0 || v == VK_SPACE.0 => match cur {
                Some(i) => activate(p, lvl, i, true),
                None => Do::Nothing,
            },
            v if v == VK_LEFT.0 || v == VK_ESCAPE.0 => {
                if lvl == 0 {
                    Do::Close
                } else {
                    Do::OpenSub(lvl - 1, usize::MAX, true)
                }
            }
            // Alt / F10 close the menu (Windows menu-bar convention).
            0x12 | 0x79 => Do::Close,
            _ => Do::Nothing,
        }
    });
    match d {
        // `OpenSub(l, MAX)` = close the levels above `l` (← / Esc one level).
        Do::OpenSub(l, usize::MAX, _) => close_above(l),
        d => run(d),
    }
    true
}

fn close_above(lvl: usize) {
    let extra: Vec<HWND> = POPUP.with(|p| {
        let Ok(mut p) = p.try_borrow_mut() else {
            return Vec::new();
        };
        let Some(p) = p.as_mut() else {
            return Vec::new();
        };
        p.levels.drain((lvl + 1).min(p.levels.len())..).map(|l| l.hwnd).collect()
    });
    for h in extra {
        // SAFETY: popup window of this thread, already removed from the state.
        let _ = unsafe { DestroyWindow(h) };
    }
    repaint_all();
}

/// Mouse at screen point `pt` (`down`/`up`: a button event).
fn mouse(pt: POINT, ev: u32) -> Do {
    POPUP.with(|p| {
        let Ok(mut pb) = p.try_borrow_mut() else {
            return Do::Nothing;
        };
        let Some(p) = pb.as_mut() else {
            return Do::Nothing;
        };
        let hit = p.levels.iter().rposition(|l| {
            pt.x >= l.rect.0 && pt.x < l.rect.2 && pt.y >= l.rect.1 && pt.y < l.rect.3
        });
        let down = ev == LBD || ev == WM_RBUTTONDOWN;
        let up = ev == LBU || ev == WM_RBUTTONUP;
        let Some(lvl) = hit else {
            if down {
                return Do::Close;
            }
            // Leaving the flyout reverts the preview (B.7.3).
            if let Kind::Cards { preview: Some(_), .. } = &p.kind {
                return Do::Preview(None);
            }
            return Do::Nothing;
        };
        let Some(l) = p.levels.get(lvl) else {
            return Do::Nothing;
        };
        let (lx, ly) = (
            (pt.x - l.rect.0) as f32 / p.scale,
            (pt.y - l.rect.1) as f32 / p.scale,
        );
        if let Kind::Cards { .. } = &p.kind {
            let i = ch::card_at(lx, ly, app::SCHEMES.len());
            if up && let Some(i) = i {
                return Do::Apply(i);
            }
            if let Some(i) = i
                && let Some(l) = p.levels.first_mut()
            {
                l.sel = Some(i);
            }
            return Do::Preview(i);
        }
        let Kind::Menu(m) = &p.kind else {
            return Do::Nothing;
        };
        let Some(entries) = m.menus.get(l.menu) else {
            return Do::Nothing;
        };
        let i = entry_at(entries, ly).filter(|&i| matches!(entries.get(i), Some(Entry::Item { .. })));
        if up {
            return match i {
                Some(i) => activate(p, lvl, i, false),
                None => Do::Nothing,
            };
        }
        if down {
            return Do::Nothing;
        }
        let changed = l.sel != i;
        p.kbd = false;
        if let Some(l) = p.levels.get_mut(lvl) {
            l.sel = i;
        }
        if changed {
            p.pending = i.map(|i| (lvl, i));
            if let Some(r) = p.levels.first() {
                // SAFETY: valid hwnd; one-shot 150 ms submenu delay (B.7.1).
                unsafe { SetTimer(Some(r.hwnd), TIMER_SUB, 150, None) };
            }
            return Do::Repaint;
        }
        Do::Nothing
    })
}

fn timer(id: usize) {
    if id == TIMER_SUB {
        let d = POPUP.with(|p| {
            let Ok(mut pb) = p.try_borrow_mut() else {
                return Do::Nothing;
            };
            let Some(p) = pb.as_mut() else {
                return Do::Nothing;
            };
            if let Some(r) = p.levels.first() {
                // SAFETY: valid hwnd.
                let _ = unsafe { KillTimer(Some(r.hwnd), TIMER_SUB) };
            }
            let Some((lvl, i)) = p.pending.take() else {
                return Do::Nothing;
            };
            if p.levels.get(lvl).and_then(|l| l.sel) != Some(i) {
                return Do::Nothing;
            }
            match activate(p, lvl, i, false) {
                Do::OpenSub(..) => Do::OpenSub(lvl, i, false),
                _ => Do::OpenSub(lvl, usize::MAX, false),
            }
        });
        match d {
            Do::OpenSub(l, usize::MAX, _) => {
                if POPUP.with(|p| p.borrow().as_ref().is_some_and(|p| p.levels.len() > l + 1)) {
                    close_above(l);
                }
            }
            d => run(d),
        }
        return;
    }
    // Fade (80 ms, linear; B.15).
    let todo: Vec<(HWND, u8)> = POPUP.with(|p| {
        let Ok(mut pb) = p.try_borrow_mut() else {
            return Vec::new();
        };
        let Some(p) = pb.as_mut() else {
            return Vec::new();
        };
        let mut v = Vec::new();
        for l in &mut p.levels {
            if l.alpha < 255 {
                let t = l.opened.elapsed().as_secs_f64() * 1000.0 / FADE_MS;
                l.alpha = (t.clamp(0.0, 1.0) * 255.0) as u8;
                v.push((l.hwnd, l.alpha));
            }
        }
        if v.is_empty()
            && let Some(r) = p.levels.first()
        {
            // SAFETY: valid hwnd; nothing fades any more (T7).
            let _ = unsafe { KillTimer(Some(r.hwnd), TIMER_FADE) };
        }
        v
    });
    for (h, a) in todo {
        // SAFETY: valid layered popup.
        let _ = unsafe { SetLayeredWindowAttributes(h, COLORREF(0), a, LWA_ALPHA) };
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        PAINT => {
            paint(hwnd);
            return LRESULT(0);
        }
        WM_ERASEBKGND => return LRESULT(1),
        // Never activates (WS_EX_NOACTIVATE); the whole window is client.
        WM_NCHITTEST => return LRESULT(HTCLIENT as isize),
        MM | LBD | LBU | WM_RBUTTONDOWN | WM_RBUTTONUP => {
            let mut pt = POINT {
                x: i32::from(lp.0 as u16 as i16),
                y: i32::from((lp.0 >> 16) as u16 as i16),
            };
            // SAFETY: valid hwnd and point.
            let _ = unsafe { ClientToScreen(hwnd, &mut pt) };
            let d = mouse(pt, msg);
            run(d);
            return LRESULT(0);
        }
        TIMER => {
            timer(wp.0);
            return LRESULT(0);
        }
        WM_CAPTURECHANGED => {
            // Lost the capture to another window (Alt+Tab, another app): close via the owner.
            let ours = POPUP.with(|p| {
                p.try_borrow().ok().and_then(|p| {
                    p.as_ref().map(|p| {
                        (
                            p.owner,
                            p.levels.first().is_some_and(|l| l.hwnd == hwnd),
                            p.levels.iter().any(|l| l.hwnd.0 as isize == lp.0),
                        )
                    })
                })
            });
            if let Some((owner, true, false)) = ours {
                // SAFETY: valid owner hwnd.
                let _ = unsafe {
                    PostMessageW(Some(owner), WM_APP_POPUP_CLOSE, WPARAM(0), LPARAM(0))
                };
            }
            return LRESULT(0);
        }
        _ => {}
    }
    // SAFETY: default processing.
    unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
}

// ---- drawing --------------------------------------------------------------------------------

fn paint(hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    // SAFETY: standard BeginPaint/EndPaint pair.
    let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
    if let Some(sh) = shared() {
        POPUP.with(|p| {
            let Ok(p) = p.try_borrow() else { return };
            let Some(p) = p.as_ref() else { return };
            let Some((i, l)) = p.levels.iter().enumerate().find(|(_, l)| l.hwnd == hwnd) else {
                return;
            };
            if let Some(r) = &l.renderer {
                r.paint(hdc, ps.rcPaint, p.pal.text, &mut |b| draw_level(&sh, p, i, b));
            }
        });
    }
    // SAFETY: matches BeginPaint.
    let _ = unsafe { EndPaint(hwnd, &ps) };
}

fn draw_level(sh: &Shared, p: &Popup, lvl: usize, b: &Band<'_>) {
    let Some(l) = p.levels.get(lvl) else { return };
    let tok = ch::tokens(p.dark);
    let r = if p.rounded { 8.0 } else { 0.0 };
    let h = (l.rect.3 - l.rect.1) as f32 / p.scale;
    b.clear(tok.menu_bg);
    b.stroke(Rect::new(0.0, 0.0, l.w, h), tok.menu_border, r, 1.0);
    match &p.kind {
        Kind::Cards { orig, .. } => draw_cards(sh, p, l, orig, b),
        Kind::Menu(m) => {
            let Some(entries) = m.menus.get(l.menu) else {
                return;
            };
            let mut y = PAD;
            let (text, muted) = (p.pal.text, p.pal.text_muted);
            for (i, e) in entries.iter().enumerate() {
                match e {
                    Entry::Sep => {
                        b.fill(Rect::new(12.0, y + 4.0, l.w - 24.0, 1.0), tok.separator, 1.0, 0.0);
                        y += SEP_H;
                    }
                    Entry::Item {
                        label,
                        key,
                        mark,
                        sub,
                        ..
                    } => {
                        if l.sel == Some(i) {
                            b.fill(
                                Rect::new(PAD, y, l.w - 2.0 * PAD, ITEM_H),
                                tok.menu_hover,
                                1.0,
                                if p.rounded { 4.0 } else { 0.0 },
                            );
                        }
                        let cy = y + ITEM_H / 2.0;
                        match mark {
                            1 => b.polyline(
                                &[(18.0, cy), (21.5, cy + 3.5), (28.0, cy - 4.0)],
                                p.pal.accent,
                                1.6,
                                false,
                            ),
                            2 => b.circle(23.0, cy, 3.5, p.pal.accent, None),
                            _ => {}
                        }
                        let t = shape(sh, label, 13.0, 400, text);
                        b.text(t.as_ref(), 44.0, (cy - t.height() / 2.0).round());
                        if !key.is_empty() {
                            let k = shape(sh, key, 12.0, 400, muted);
                            b.text(
                                k.as_ref(),
                                (l.w - 16.0 - k.width()).round(),
                                (cy - k.height() / 2.0).round(),
                            );
                        }
                        if sub.is_some() {
                            let x = l.w - 18.0;
                            b.polyline(
                                &[(x, cy - 4.0), (x + 4.0, cy), (x, cy + 4.0)],
                                muted,
                                1.3,
                                false,
                            );
                        }
                        y += ITEM_H;
                    }
                }
            }
        }
    }
}

fn draw_cards(sh: &Shared, p: &Popup, l: &Level, orig: &str, b: &Band<'_>) {
    let r = if p.rounded { 8.0 } else { 0.0 };
    let tok = ch::tokens(p.dark);
    for (i, (key, name)) in app::SCHEMES.iter().enumerate() {
        let (x, y, w, _) = ch::card_rect(i);
        let pal = theme::palette(key, p.dark);
        let prev = Rect::new(x, y, w, 56.0);
        b.fill(prev, pal.bg, 1.0, r);
        b.stroke(prev, tok.chrome_border, r, 1.0);
        b.fill(Rect::new(x + 10.0, y + 12.0, (w * 0.4).round(), 6.0), pal.heading, 1.0, 1.0);
        b.fill(Rect::new(x + 10.0, y + 24.0, w - 34.0, 4.0), pal.text, 0.35, 1.0);
        b.fill(Rect::new(x + 10.0, y + 32.0, w - 50.0, 4.0), pal.text, 0.35, 1.0);
        b.fill(Rect::new(x + 10.0, y + 42.0, 24.0, 3.0), pal.accent, 1.0, 1.0);
        b.fill(Rect::new(x + w - 40.0, y + 36.0, 30.0, 12.0), pal.code_bg, 1.0, 2.0);
        let t = shape(sh, name, 12.0, 400, p.pal.text);
        b.text(t.as_ref(), (x + (w - t.width()) / 2.0).round(), y + 62.0);
        let selected = *key == orig;
        if selected {
            b.stroke(
                Rect::new(x - 2.0, y - 2.0, w + 4.0, 60.0),
                p.pal.accent,
                r + 2.0,
                2.0,
            );
            // Check badge (top right).
            let (bx, by) = (x + w - 10.0, y + 10.0);
            b.circle(bx, by, 7.0, p.pal.accent, None);
            b.polyline(
                &[(bx - 3.5, by), (bx - 1.0, by + 2.5), (bx + 3.5, by - 2.5)],
                0xffffff,
                1.5,
                false,
            );
        } else if l.sel == Some(i) {
            b.stroke(
                Rect::new(x - 1.0, y - 1.0, w + 2.0, 58.0),
                p.pal.accent,
                r + 1.0,
                1.0,
            );
        }
    }
}

// ---- driver (dev-tools) ---------------------------------------------------------------------

/// `(open, levels, root items, total items, kind)` for the state dump.
#[cfg(feature = "dev-tools")]
pub(super) fn probe() -> (bool, usize, usize, usize, &'static str, String) {
    POPUP.with(|p| {
        let p = p.borrow();
        let Some(p) = p.as_ref() else {
            return (false, 0, 0, 0, "none", String::new());
        };
        match &p.kind {
            Kind::Menu(m) => {
                let root = m
                    .menus
                    .first()
                    .map_or(0, |e| e.iter().filter(|e| matches!(e, Entry::Item { .. })).count());
                let first = m
                    .menus
                    .first()
                    .and_then(|e| {
                        e.iter().find_map(|e| match e {
                            Entry::Item { label, .. } => Some(label.clone()),
                            Entry::Sep => None,
                        })
                    })
                    .unwrap_or_default();
                (true, p.levels.len(), root, m.item_count(), "menu", first)
            }
            Kind::Cards { preview, .. } => (
                true,
                p.levels.len(),
                app::SCHEMES.len(),
                app::SCHEMES.len(),
                if preview.is_some() { "cards-preview" } else { "cards" },
                String::new(),
            ),
        }
    })
}

/// Driver: hover entry / card `i` of the deepest level (`None` = leave the popup).
#[cfg(feature = "dev-tools")]
pub(super) fn drive_hover(i: Option<usize>) {
    let pt = POPUP.with(|p| {
        let p = p.borrow();
        let p = p.as_ref()?;
        let l = p.levels.last()?;
        let Some(i) = i else {
            return Some(POINT {
                x: l.rect.0 - 50,
                y: l.rect.1 - 50,
            });
        };
        let (x, y) = match &p.kind {
            Kind::Cards { .. } => {
                let (x, y, w, h) = ch::card_rect(i);
                (x + w / 2.0, y + h / 3.0)
            }
            Kind::Menu(m) => (
                l.w / 2.0,
                entry_top(m.menus.get(l.menu)?, i) + ITEM_H / 2.0,
            ),
        };
        Some(POINT {
            x: l.rect.0 + (x * p.scale) as i32,
            y: l.rect.1 + (y * p.scale) as i32,
        })
    });
    if let Some(pt) = pt {
        run(mouse(pt, MM));
    }
}

/// Driver: click entry / card `i` of the deepest level.
#[cfg(feature = "dev-tools")]
pub(super) fn drive_click(i: usize) {
    drive_hover(Some(i));
    let pt = POPUP.with(|p| {
        let p = p.borrow();
        let p = p.as_ref()?;
        let l = p.levels.last()?;
        let (x, y) = match &p.kind {
            Kind::Cards { .. } => {
                let (x, y, w, h) = ch::card_rect(i);
                (x + w / 2.0, y + h / 3.0)
            }
            Kind::Menu(m) => (
                l.w / 2.0,
                entry_top(m.menus.get(l.menu)?, i) + ITEM_H / 2.0,
            ),
        };
        Some(POINT {
            x: l.rect.0 + (x * p.scale) as i32,
            y: l.rect.1 + (y * p.scale) as i32,
        })
    });
    if let Some(pt) = pt {
        run(mouse(pt, LBU));
    }
}

/// Driver: renders the root level offscreen to `out` (PNG), like the window snapshots.
#[cfg(feature = "dev-tools")]
pub(super) fn snapshot(out: &str) -> Result<(u32, u32), String> {
    let sh = shared().ok_or("no shared state")?;
    POPUP.with(|p| {
        let p = p.borrow();
        let p = p.as_ref().ok_or("no popup open")?;
        let l = p.levels.first().ok_or("no level")?;
        let r = l.renderer.as_ref().ok_or("no renderer")?;
        let h = l.rect.3 - l.rect.1;
        let mut px = r.render_offscreen(h, p.pal.text, &mut |b| draw_level(&sh, p, 0, b));
        for q in px.as_chunks_mut::<4>().0 {
            q[3] = 255;
        }
        let (pw, ph) = (
            u32::try_from(r.width()).map_err(|e| e.to_string())?,
            u32::try_from(h).map_err(|e| e.to_string())?,
        );
        crate::devtools::write_png(out, pw, ph, &px).map_err(|e| e.to_string())?;
        Ok((pw, ph))
    })
}
