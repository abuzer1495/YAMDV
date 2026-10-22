//! In-window test driver (prompt 004 §4.5, dev-tools only): runs a `--script` file by calling
//! the window's own handlers (no `SendInput`), asserts over a JSON state dump, writes
//! `<script>.log` and exits with 0 (all assertions passed) or 3.

use std::cell::RefCell;

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{KillTimer, PostQuitMessage, SetTimer};
use yamdv_core::devscript::{self, Cmd};
use yamdv_core::doc::detail::NO_LINK;

use super::app;
use super::{Content, Pending, Shared, TIMER_SCRIPT, View, WINDOWS, Win, focus_rects};
use crate::sys;
use serde_like::Get as _;

struct Script {
    path: String,
    cmds: Vec<Cmd>,
    pc: usize,
    log: Vec<String>,
    fails: u32,
    /// `sleep`: resume after this instant.
    resume: Option<std::time::Instant>,
    /// `type` in an edit mode: one character per step, so each edit is painted (T5).
    chars: std::collections::VecDeque<char>,
}

thread_local! {
    static SCRIPT: RefCell<Option<Script>> = const { RefCell::new(None) };
    /// The file written by the last `export-html` / `export-csv` (V1-D).
    static LAST_OUT: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Loads the script (once) and starts the step timer on `hwnd`.
pub(super) fn start(hwnd: HWND, path: &str) {
    if SCRIPT.with(|s| s.borrow().is_some()) {
        return;
    }
    sys::set_launch_dry_run();
    let src = std::fs::read_to_string(path).unwrap_or_default();
    let (cmds, log, fails) = match devscript::parse(&src) {
        Ok(c) => (c, Vec::new(), 0),
        Err(e) => (Vec::new(), vec![format!("FAIL parse: {e}")], 1),
    };
    SCRIPT.with(|s| {
        *s.borrow_mut() = Some(Script {
            path: path.to_string(),
            cmds,
            pc: 0,
            log,
            fails,
            resume: None,
            chars: std::collections::VecDeque::new(),
        });
    });
    // SAFETY: valid hwnd; killed when the script ends.
    unsafe { SetTimer(Some(hwnd), TIMER_SCRIPT, 60, None) };
}

/// One step: runs commands until one that needs the message loop (input events).
pub(super) fn tick(w: &mut Win, sh: &Shared) {
    let now = std::time::Instant::now();
    if SCRIPT.with(|s| {
        s.borrow()
            .as_ref()
            .and_then(|s| s.resume)
            .is_some_and(|r| now < r)
    }) {
        return;
    }
    let next = SCRIPT.with(|s| s.borrow_mut().as_mut().and_then(|s| s.chars.pop_front()));
    if let Some(c) = next {
        w.edit_type(sh, &c.to_string());
        return;
    }
    loop {
        let cmd = SCRIPT.with(|s| {
            let mut s = s.borrow_mut();
            let s = s.as_mut()?;
            let c = s.cmds.get(s.pc).cloned();
            s.pc += 1;
            c
        });
        let Some(cmd) = cmd else {
            finish(w);
            return;
        };
        let yield_after = !matches!(cmd, Cmd::Assert(..) | Cmd::Dump | Cmd::WaitIdle);
        let line = run(w, sh, &cmd);
        SCRIPT.with(|s| {
            if let Some(s) = s.borrow_mut().as_mut()
                && !line.is_empty()
            {
                if line.starts_with("FAIL") {
                    s.fails += 1;
                }
                s.log.push(line);
            }
        });
        if yield_after {
            return;
        }
    }
}

fn finish(w: &mut Win) {
    // SAFETY: valid hwnd.
    let _ = unsafe { KillTimer(Some(w.hwnd), TIMER_SCRIPT) };
    let code = SCRIPT.with(|s| {
        let s = s.borrow();
        let Some(s) = s.as_ref() else { return 3 };
        let mut out = s.log.join("\n");
        out.push_str(&format!(
            "\n{} ({} failed)\n",
            if s.fails == 0 { "PASS" } else { "FAIL" },
            s.fails
        ));
        let _ = std::fs::write(format!("{}.log", s.path), out);
        if s.fails == 0 { 0 } else { 3 }
    });
    // SAFETY: no preconditions; ends the message loop with the script result.
    unsafe { PostQuitMessage(code) };
}

fn run(w: &mut Win, sh: &Shared, cmd: &Cmd) -> String {
    match cmd {
        Cmd::Click(x, y) => {
            w.mouse_down(sh, *x, *y, false);
            w.mouse_up();
            format!("ok click {x} {y}")
        }
        Cmd::ClickLink(text, new) => match find_link(w, text) {
            None => format!("FAIL click-link \"{text}\": no such link"),
            Some((b, l)) => {
                w.scroll_into_view(sh, b, l);
                let (cx, _) = w.col(sh);
                let View::Doc(d) = &w.view else {
                    return "FAIL click-link: no document".into();
                };
                let top = (d.viewer.block_top(&d.content, b) - d.viewer.scroll) as f32;
                let Some(r) = focus_rects(&d.viewer, b, l).first().copied() else {
                    return format!("FAIL click-link \"{text}\": not laid out");
                };
                let (x, y) = (cx + r.x + r.w.min(4.0), top + r.y + r.h / 2.0);
                if *new {
                    w.middle_click(sh, x, y);
                } else {
                    w.mouse_down(sh, x, y, false);
                    w.mouse_up();
                }
                format!("ok click-link \"{text}\" at {x:.0},{y:.0}")
            }
        },
        Cmd::Key(c) => {
            if c.alt && (c.vk == 0x25 || c.vk == 0x27) {
                w.history_go(sh, if c.vk == 0x25 { -1 } else { 1 });
            } else if let Some(a) = w.key(sh, c.vk, c.ctrl, c.shift) {
                w.actions.push(a);
            }
            format!("ok key {c:?}")
        }
        Cmd::Wheel(d) => {
            w.wheel(sh, *d);
            format!("ok wheel {d}")
        }
        Cmd::WaitIdle => {
            if let Some(a) = w.anim.take() {
                w.set_scroll(a.to);
            }
            w.prepare_now(sh);
            "ok wait-idle".into()
        }
        Cmd::Assert(f, op, v) => {
            let st = state(w, sh);
            let actual = st
                .get(f)
                .map(|x| match x {
                    serde_like::V::S(s) => s.clone(),
                    serde_like::V::N(n) => format!("{n}"),
                })
                .unwrap_or_else(|| "<missing>".into());
            let pass = devscript::check(&actual, *op, v);
            format!(
                "{} assert {f} {op:?} {v} (actual {actual})",
                if pass { "ok" } else { "FAIL" }
            )
        }
        Cmd::Dump => format!("dump {}", serde_like::to_json(&state(w, sh))),
        Cmd::Command(name) => match command_id(name) {
            Some(id) => {
                w.actions.push(super::Action::Command(id));
                format!("ok command {name}")
            }
            None => format!("FAIL command {name}: unknown"),
        },
        Cmd::Type(t) => {
            if w.find.is_none() && w.typing() {
                SCRIPT.with(|s| {
                    if let Some(s) = s.borrow_mut().as_mut() {
                        s.chars.extend(t.chars());
                    }
                });
            } else {
                for c in t.chars() {
                    w.find_char(sh, c);
                }
            }
            format!("ok type \"{t}\"")
        }
        Cmd::ClickText(t) => click_text(w, sh, t, Click::Text),
        Cmd::ClickTask(t) => click_text(w, sh, t, Click::Task),
        Cmd::ClickRender(t) => click_text(w, sh, t, Click::Render),
        Cmd::SideClick(text) => {
            let row = w
                .side_rows(sh)
                .into_iter()
                .find(|r| r.text.contains(text.as_str()));
            match row {
                Some(r) => {
                    let (x, y) = (r.rect.x + r.rect.w / 2.0, r.rect.y + r.rect.h / 2.0);
                    w.mouse_down(sh, x, y, false);
                    w.mouse_up();
                    format!("ok side-click \"{text}\" at {x:.0},{y:.0}")
                }
                None => format!("FAIL side-click \"{text}\": no such row"),
            }
        }
        Cmd::WriteFile(f, t) => match std::fs::write(rel(f), t) {
            Ok(()) => format!("ok write-file {f}"),
            Err(e) => format!("FAIL write-file {f}: {e}"),
        },
        Cmd::Open(f) => {
            w.open(sh, &rel(f), None, false);
            format!("ok open {f}")
        }
        Cmd::Launch(f) => {
            if crate::shell::spawn_self(&[rel(f)]) {
                format!("ok launch {f}")
            } else {
                format!("FAIL launch {f}")
            }
        }
        Cmd::Snapshot(f) => snapshot(w, sh, f),
        Cmd::ExportHtml(f) => {
            let out = rel(f);
            let _ = std::fs::remove_file(&out);
            let ok = w
                .export_job(sh)
                .is_some_and(|j| crate::export::export_html(&sh.dw, &j, &out));
            LAST_OUT.with(|l| *l.borrow_mut() = Some(out));
            format!("{} export-html {f}", if ok { "ok" } else { "FAIL" })
        }
        Cmd::ExportCsv(f) => {
            let out = rel(f);
            let _ = std::fs::remove_file(&out);
            let csv = match &w.view {
                View::Doc(d) => match &d.content {
                    Content::Markdown(doc) => {
                        doc.index.blocks.iter().enumerate().find_map(|(bi, b)| {
                            matches!(b.kind, yamdv_core::doc::index::BlockKind::Table)
                                .then(|| crate::export::table_csv(doc, bi, b.range.start))
                                .flatten()
                        })
                    }
                    _ => None,
                },
                _ => None,
            };
            let ok = csv.is_some_and(|t| crate::shell::write_atomic(&out, &t));
            LAST_OUT.with(|l| *l.borrow_mut() = Some(out));
            format!("{} export-csv {f}", if ok { "ok" } else { "FAIL" })
        }
        Cmd::Resize(cw, ch) => resize(w, *cw, *ch),
        Cmd::Ui(verb, arg) => ui(w, sh, verb, arg),
        Cmd::Sleep(ms) => {
            let until =
                std::time::Instant::now() + std::time::Duration::from_millis(u64::from(*ms));
            SCRIPT.with(|s| {
                if let Some(s) = s.borrow_mut().as_mut() {
                    s.resume = Some(until);
                }
            });
            format!("ok sleep {ms}")
        }
    }
}

fn resize(w: &mut Win, cw: f32, ch: f32) -> String {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, GetWindowRect};
    let (mut wr, mut cr) = (RECT::default(), RECT::default());
    // SAFETY: valid hwnd and out-structures.
    let ok =
        unsafe { GetWindowRect(w.hwnd, &mut wr).is_ok() && GetClientRect(w.hwnd, &mut cr).is_ok() };
    if !ok {
        return "FAIL resize".into();
    }
    let fw = (wr.right - wr.left) - (cr.right - cr.left);
    let fh = (wr.bottom - wr.top) - (cr.bottom - cr.top);
    let pw = (cw * w.scale).round() as i32 + fw;
    // `ch` is the content height; the title bar sits above it (B.4).
    let ph = ((ch + w.top) * w.scale).round() as i32 + fh;
    w.defer(super::Deferred::SetPos(wr.left, wr.top, pw, ph));
    format!("ok resize {cw} {ch}")
}

/// Renders the whole client area offscreen (same draw path as WM_PAINT) to a PNG.
fn snapshot(w: &mut Win, sh: &Shared, f: &str) -> String {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::GetClientRect;
    w.prepare_now(sh);
    let mut cr = RECT::default();
    // SAFETY: valid hwnd and out-structure.
    let _ = unsafe { GetClientRect(w.hwnd, &mut cr) };
    let (cx, _) = w.col(sh);
    let geo = w.geometry(sh);
    let Some(r) = &w.renderer else {
        return "FAIL snapshot: no renderer".into();
    };
    let me = &*w;
    // Content only (the goldens of earlier phases); `ui shot-window` includes the title bar.
    let full = f.starts_with("top-");
    let h = if full {
        cr.bottom - cr.top
    } else {
        cr.bottom - cr.top - me.top_px()
    };
    let mut px = r.render_offscreen(h, me.pal.text, &mut |band| {
        if full {
            band.set_origin(me.top);
        }
        me.draw(sh, band, cx, geo.as_ref());
        me.draw_overlays(sh, band);
        if full {
            band.set_origin(0.0);
            me.draw_chrome(sh, band);
        }
    });
    for p in px.as_chunks_mut::<4>().0 {
        p[3] = 255;
    }
    let dir = std::env::var("YAMDV_SNAPSHOT_DIR").unwrap_or_default();
    let out = if dir.is_empty() {
        rel(f)
    } else {
        std::path::Path::new(&dir)
            .join(f)
            .to_string_lossy()
            .into_owned()
    };
    let (Ok(pw), Ok(ph)) = (u32::try_from(r.width()), u32::try_from(h)) else {
        return "FAIL snapshot: size".into();
    };
    match crate::devtools::write_png(&out, pw, ph, &px) {
        Ok(()) => format!("ok snapshot {f} {pw}x{ph}"),
        Err(e) => format!("FAIL snapshot {f}: {e}"),
    }
}

/// Path relative to the script's folder.
fn rel(f: &str) -> String {
    let dir = SCRIPT.with(|s| {
        s.borrow().as_ref().and_then(|s| {
            std::path::Path::new(&s.path)
                .parent()
                .map(std::path::Path::to_path_buf)
        })
    });
    dir.unwrap_or_default()
        .join(f)
        .to_string_lossy()
        .into_owned()
}

/// Popup operations run outside the window borrow (`Action::PopupOp`).
#[derive(Debug, Clone)]
pub(super) enum PopupOp {
    Hover(Option<usize>),
    Click(usize),
    Key(u16),
    Shot(String),
}

/// Runs a popup operation (no window borrow is held).
pub(super) fn popup_op(_hwnd: HWND, op: PopupOp) {
    let line = match op {
        PopupOp::Hover(i) => {
            super::popup::drive_hover(i);
            format!("ok ui popup-hover {i:?}")
        }
        PopupOp::Click(i) => {
            super::popup::drive_click(i);
            format!("ok ui popup-click {i}")
        }
        PopupOp::Key(vk) => {
            let used = super::popup::key(
                windows::Win32::UI::WindowsAndMessaging::WM_KEYDOWN,
                windows::Win32::Foundation::WPARAM(usize::from(vk)),
            );
            format!("{} ui popup-key {vk}", if used { "ok" } else { "FAIL" })
        }
        PopupOp::Shot(f) => {
            let dir = std::env::var("YAMDV_SNAPSHOT_DIR").unwrap_or_default();
            let out = if dir.is_empty() {
                rel(&f)
            } else {
                std::path::Path::new(&dir)
                    .join(&f)
                    .to_string_lossy()
                    .into_owned()
            };
            match super::popup::snapshot(&out) {
                Ok((pw, ph)) => format!("ok ui popup-shot {f} {pw}x{ph}"),
                Err(e) => format!("FAIL ui popup-shot {f}: {e}"),
            }
        }
    };
    SCRIPT.with(|s| {
        if let Some(s) = s.borrow_mut().as_mut() {
            if line.starts_with("FAIL") {
                s.fails += 1;
            }
            s.log.push(line);
        }
    });
}

fn vk_of(name: &str) -> Option<u16> {
    Some(match name {
        "esc" => 0x1B,
        "enter" => 0x0D,
        "up" => 0x26,
        "down" => 0x28,
        "left" => 0x25,
        "right" => 0x27,
        "home" => 0x24,
        "end" => 0x23,
        "space" => 0x20,
        s if s.len() == 1 => u16::from(s.as_bytes().first()?.to_ascii_uppercase()),
        _ => return None,
    })
}

/// `ui <verb> [arg]`: title bar, popup and chrome operations (V2-A).
fn ui(w: &mut Win, sh: &Shared, verb: &str, arg: &str) -> String {
    use yamdv_core::chrome::Part;
    let part = |a: &str| match a {
        "settings" => Some(Part::Settings),
        "more" => Some(Part::More),
        "min" => Some(Part::Min),
        "max" => Some(Part::Max),
        "close" => Some(Part::Close),
        _ => None,
    };
    let op = |w: &mut Win, op: PopupOp| {
        w.actions.push(super::Action::PopupOp(op));
        String::new()
    };
    match verb {
        "click" => match part(arg) {
            Some(p) => {
                w.chrome_click(p);
                format!("ok ui click {arg}")
            }
            None => format!("FAIL ui click {arg}: unknown button"),
        },
        "dblclick-caption" => {
            use windows::Win32::Foundation::{LPARAM, WPARAM};
            let (x, y) = w.button_anchor(Part::Settings);
            let (x, y) = (x - 200, y - 10);
            let lp = (((y & 0xFFFF) as isize) << 16) | ((x & 0xFFFF) as isize);
            // SAFETY: valid hwnd; DefWindowProc handles the caption double-click.
            let _ = unsafe {
                windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                    Some(w.hwnd),
                    0x00A3,
                    WPARAM(2),
                    LPARAM(lp),
                )
            };
            "ok ui dblclick-caption".into()
        }
        "hover" => {
            let y: f32 = arg.parse().unwrap_or(0.0);
            let x = w.cw / 2.0;
            w.chrome_move(x, y);
            format!("ok ui hover {y}")
        }
        "popup-hover" => op(w, PopupOp::Hover(arg.parse().ok())),
        "popup-click" => match arg.parse() {
            Ok(i) => op(w, PopupOp::Click(i)),
            Err(_) => format!("FAIL ui popup-click {arg}"),
        },
        "popup-key" => match vk_of(arg) {
            Some(vk) => op(w, PopupOp::Key(vk)),
            None => format!("FAIL ui popup-key {arg}: unknown key"),
        },
        "popup-shot" => op(w, PopupOp::Shot(arg.to_string())),
        "shot-window" => snapshot(w, sh, &format!("top-{arg}")).replacen("top-", "", 1),
        _ => format!("FAIL ui {verb}: unknown verb"),
    }
}

/// Context-menu command by script name.
fn command_id(name: &str) -> Option<u32> {
    let (verb, arg) = name.split_once(' ').unwrap_or((name, ""));
    let idx = |list: &[&str]| {
        list.iter()
            .position(|v| *v == arg)
            .and_then(|i| u32::try_from(i).ok())
    };
    Some(match verb {
        "mode" => app::MODE0 + idx(&["system", "light", "dark"])?,
        "scheme" => {
            let names: Vec<&str> = app::SCHEMES.iter().map(|s| s.0).collect();
            app::SCHEME0 + idx(&names)?
        }
        "favorite" => app::FAVORITE,
        "outline" => app::OUTLINE,
        "reload" => app::RELOAD,
        "select-all" => app::SELECT_ALL,
        "restore-position" => app::RESTORE_POS,
        "live" => app::LIVE_EDIT,
        "split" => app::SPLIT_EDIT,
        "swap" => app::SWAP_PANES,
        "autosave" => app::AUTOSAVE,
        "undo" => app::UNDO,
        "redo" => app::REDO,
        "line-numbers" => app::ELINES,
        "soft-wrap" => app::EWRAP,
        "fullscreen" => app::FULLSCREEN,
        "word-count" => app::WORD_COUNT,
        "scheme-cards" => app::SCHEME_CARDS,
        "settings-menu" => app::SETTINGS_MENU,
        "more-menu" => app::MORE_MENU,
        _ => return None,
    })
}

/// Clicks the first rendered occurrence of `text` (or the checkbox of its task item).
/// What `click_text` aims at.
#[derive(Clone, Copy, PartialEq)]
enum Click {
    Text,
    Task,
    Render,
}

fn click_text(w: &mut Win, sh: &Shared, text: &str, what: Click) -> String {
    use yamdv_layout::block::HotKind;
    let found = match &w.view {
        View::Doc(d) => d
            .content
            .source()
            .find(text)
            .map(|p| (p, super::edit::block_of(&d.content, p))),
        _ => None,
    };
    let Some((pos, bi)) = found else {
        return format!("FAIL click-text \"{text}\": not in the document");
    };
    let top = match &w.view {
        View::Doc(d) => d.viewer.block_top(&d.content, bi),
        _ => 0.0,
    };
    let s = w.scroll_pos();
    if top < s + 40.0 || top > s + w.view_h() - 120.0 {
        w.set_scroll(top - 80.0);
    }
    if let Some(a) = w.anim.take() {
        w.set_scroll(a.to);
    }
    w.prepare_now(sh);
    let (cx, _) = w.col(sh);
    let pt = (|| {
        let View::Doc(d) = &w.view else { return None };
        let bl = d.viewer.layout_of(bi)?;
        let top = (d.viewer.block_top(&d.content, bi) - d.viewer.scroll) as f32;
        if what == Click::Render {
            let h = bl
                .hot
                .iter()
                .find(|h| matches!(h.kind, HotKind::RenderAnyway { .. }))?;
            return Some((
                cx + h.rect.x + h.rect.w / 2.0,
                top + h.rect.y + h.rect.h / 2.0,
            ));
        }
        if what == Click::Task {
            let h = bl
                .hot
                .iter()
                .rfind(|h| matches!(h.kind, HotKind::Task { item } if item as usize <= pos))?;
            return Some((
                cx + h.rect.x + h.rect.w / 2.0,
                top + h.rect.y + h.rect.h / 2.0,
            ));
        }
        let (a, b) = (
            u32::try_from(pos).ok()?,
            u32::try_from(pos + text.len()).ok()?,
        );
        bl.paras.iter().find_map(|p| {
            let r = *yamdv_layout::viewer::selection_rects(p, &(a..b)).first()?;
            Some((cx + p.x + r.x + r.w / 2.0, top + p.y + r.y + r.h / 2.0))
        })
    })();
    let Some((x, y)) = pt else {
        return format!("FAIL click-text \"{text}\": not laid out");
    };
    if !w.edit_mouse_down(sh, x, y, false) {
        w.mouse_down(sh, x, y, false);
    }
    if !w.edit_mouse_up() {
        w.mouse_up();
    }
    format!(
        "ok click-{} \"{text}\" at {x:.0},{y:.0}",
        match what {
            Click::Text => "text",
            Click::Task => "task",
            Click::Render => "render",
        }
    )
}

/// Escapes CR/LF for one-line state values (scripts compare against `\n` / `\r`).
fn esc_text(s: &str) -> String {
    s.chars()
        .take(4000)
        .collect::<String>()
        .replace('\r', "\\r")
        .replace('\n', "\\n")
}

/// First link whose text contains `text` (document order).
fn find_link(w: &mut Win, text: &str) -> Option<(usize, u32)> {
    let View::Doc(d) = &mut w.view else {
        return None;
    };
    let Content::Markdown(doc) = &mut d.content else {
        return None;
    };
    let n = doc.index.blocks.len();
    for b in 0..n {
        let det = doc.detail(b);
        let src = doc.source.as_str();
        let mut acc: Vec<(u32, String)> = Vec::new();
        for r in det.runs.iter().filter(|r| r.link != NO_LINK) {
            match acc.iter_mut().find(|(l, _)| *l == r.link) {
                Some((_, s)) => s.push_str(r.text(src)),
                None => acc.push((r.link, r.text(src).to_string())),
            }
        }
        if let Some((l, _)) = acc.into_iter().find(|(_, s)| s.contains(text)) {
            return Some((b, l));
        }
    }
    None
}

fn link_text(w: &mut Win, b: usize, l: u32) -> String {
    let View::Doc(d) = &mut w.view else {
        return String::new();
    };
    let Content::Markdown(doc) = &mut d.content else {
        return String::new();
    };
    let det = doc.detail(b);
    let src = doc.source.as_str();
    det.runs
        .iter()
        .filter(|r| r.link == l)
        .map(|r| r.text(src))
        .collect()
}

fn state(w: &mut Win, sh: &Shared) -> Vec<(String, serde_like::V)> {
    use serde_like::V;
    let doc = match &w.view {
        View::Doc(d) => yamdv_core::path::file_name(&d.path).to_string(),
        View::Empty { .. } => "none".into(),
        View::Error { .. } => "error".into(),
    };
    let focus = match w.focus {
        Some((b, l)) => link_text(w, b, l),
        None => "none".into(),
    };
    let blocked = yamdv_core::i18n::tr(sh.lang, yamdv_core::i18n::Msg::BlockedSafety);
    let popover = match &w.dialog {
        None => "none",
        Some((_, Pending::Url { .. })) => "ask-url",
        Some((_, Pending::Local { .. })) => "confirm-local",
        Some((d, Pending::Reveal { .. })) if d.title == blocked => "blocked",
        Some((_, Pending::Reveal { .. })) => "folder",
        Some((_, Pending::Edit(super::edit::Ask::Conflict))) => "conflict",
        Some((_, Pending::Edit(super::edit::Ask::Blocked(_)))) => "edit-blocked",
        Some((_, Pending::Edit(super::edit::Ask::Unmappable))) => "unmappable",
        Some((_, Pending::Edit(super::edit::Ask::Unsaved))) => "unsaved",
        Some(_) => "document-guard",
    };
    let windows = WINDOWS.with(|m| m.borrow().len());
    let (find, find_count, find_index) = match &w.find {
        Some(f) => ("open", f.matches.len(), f.cur.map_or(0, |c| c + 1)),
        None => ("closed", 0, 0),
    };
    let (source_len, (math_cached, diagrams_cached)) = match &w.view {
        View::Doc(d) => (d.content.source().len(), d.rich.len()),
        _ => (0, (0, 0)),
    };
    let favorites = app::with(|a| a.state.favorites.len()).unwrap_or(0);
    let edit_mode = ["read", "live", "split"][usize::from(w.edit_mode_id())];
    let (dirty, active, caret, sel_len, buffer, undo, t5) = match w.edit() {
        Some(e) => {
            let mut l = e.lat.clone();
            l.sort_by(f64::total_cmp);
            let p99 = l
                .get(((l.len() as f64 * 0.99).ceil() as usize).saturating_sub(1))
                .copied()
                .unwrap_or(0.0);
            (
                u8::from(e.ed.is_dirty()),
                e.active.map_or(-1.0, |a| a.block as f64),
                e.ed.caret(),
                e.ed.selection().len(),
                esc_text(&e.ed.text()),
                e.ed.history().undo_len(),
                (p99, l.len()),
            )
        }
        None => (0, -1.0, 0, 0, String::new(), 0, (0.0, 0)),
    };
    let (preview, file) = match &w.view {
        View::Doc(d) if !d.virt => (
            esc_text(d.content.source()),
            std::fs::read(yamdv_core::path::long_form(&d.path))
                .map(|b| esc_text(&String::from_utf8_lossy(&b)))
                .unwrap_or_default(),
        ),
        _ => (String::new(), String::new()),
    };
    let title = {
        use windows::Win32::UI::WindowsAndMessaging::GetWindowTextW;
        let mut buf = [0u16; 512];
        // SAFETY: valid hwnd and buffer.
        let n = unsafe { GetWindowTextW(w.hwnd, &mut buf) };
        String::from_utf16_lossy(buf.get(..usize::try_from(n).unwrap_or(0)).unwrap_or(&[]))
    };
    let (menu_items, menu_checked) = app::menu_probe(&w.menu_ctx(None));
    let (p_open, p_levels, p_root, p_items, p_kind, p_first) = super::popup::probe();
    let disk = std::fs::read(app::settings_path()).unwrap_or_default();
    let disk_scheme = yamdv_core::settings::Settings::from_ini(&yamdv_core::ini::Ini::parse(&disk))
        .str("appearance.scheme")
        .to_string();
    let (badge, words, chars) = match w.badge(sh) {
        Some((_, t)) => {
            let n: Vec<String> = t
                .split(|c: char| !c.is_ascii_digit() && c != '/')
                .filter(|x| !x.is_empty() && x.chars().any(|c| c.is_ascii_digit()))
                .map(str::to_string)
                .collect();
            (
                esc_text(&t),
                n.first().cloned().unwrap_or_default(),
                n.get(1).cloned().unwrap_or_default(),
            )
        }
        None => ("none".to_string(), String::new(), String::new()),
    };
    let out = LAST_OUT
        .with(|l| l.borrow().clone())
        .and_then(|p| std::fs::read(p).ok())
        .unwrap_or_default();
    let out_text = String::from_utf8_lossy(&out).into_owned();
    let lower = out_text.to_ascii_lowercase();
    let srcs: Vec<&str> = lower.split(" src=\"").skip(1).collect();
    vec![
        ("document".into(), V::S(doc)),
        ("scroll_y".into(), V::N(w.scroll_pos())),
        ("focus".into(), V::S(focus)),
        ("popover".into(), V::S(popover.into())),
        ("banner".into(), V::S("none".into())),
        ("nav_depth".into(), V::N(w.history.len() as f64)),
        ("nav_pos".into(), V::N(w.hist_pos as f64)),
        ("windows".into(), V::N(windows as f64)),
        ("launched".into(), V::N(f64::from(sys::launches()))),
        ("find".into(), V::S(find.into())),
        ("find_count".into(), V::N(find_count as f64)),
        ("find_index".into(), V::N(find_index as f64)),
        ("scheme".into(), V::S(sh.scheme.clone())),
        ("mode".into(), V::S(sh.mode.clone())),
        ("dark".into(), V::N(f64::from(u8::from(w.dark)))),
        (
            "sidebar".into(),
            V::S(if sh.side_visible { "visible" } else { "hidden" }.into()),
        ),
        ("favorites".into(), V::N(favorites as f64)),
        ("source_len".into(), V::N(source_len as f64)),
        ("math_cached".into(), V::N(math_cached as f64)),
        ("diagrams_cached".into(), V::N(diagrams_cached as f64)),
        ("menu_items".into(), V::N(menu_items as f64)),
        ("menu_checked".into(), V::S(menu_checked)),
        ("edit_mode".into(), V::S(edit_mode.into())),
        ("dirty".into(), V::N(f64::from(dirty))),
        ("active_block".into(), V::N(active)),
        ("caret".into(), V::N(caret as f64)),
        ("sel_len".into(), V::N(sel_len as f64)),
        ("undo_depth".into(), V::N(undo as f64)),
        ("buffer".into(), V::S(buffer)),
        ("preview".into(), V::S(preview)),
        ("file".into(), V::S(file)),
        ("title".into(), V::S(title)),
        ("out_len".into(), V::N(out.len() as f64)),
        (
            "out_hex".into(),
            V::S(sys::hex(out.get(..out.len().min(8)).unwrap_or(&[]))),
        ),
        (
            "out_text".into(),
            V::S(esc_text(&out_text.chars().take(600).collect::<String>())),
        ),
        (
            "out_scripts".into(),
            V::N(lower.matches("<script").count() as f64),
        ),
        (
            "out_csp".into(),
            V::N(f64::from(u8::from(
                out_text.contains(yamdv_export::html::EXPORT_CSP),
            ))),
        ),
        ("out_imgs".into(), V::N(srcs.len() as f64)),
        (
            "out_ext_src".into(),
            V::N(
                srcs.iter()
                    .filter(|s| !s.starts_with("data:image/"))
                    .count() as f64,
            ),
        ),
        ("t5_p99_ms".into(), V::N(t5.0)),
        ("t5_n".into(), V::N(t5.1 as f64)),
        ("popup_open".into(), V::N(f64::from(u8::from(p_open)))),
        ("popup_levels".into(), V::N(p_levels as f64)),
        ("popup_root_items".into(), V::N(p_root as f64)),
        ("popup_items".into(), V::N(p_items as f64)),
        ("popup_kind".into(), V::S(p_kind.into())),
        ("popup_first".into(), V::S(p_first)),
        ("maximized".into(), V::N(f64::from(u8::from(w.maximized())))),
        ("fullscreen".into(), V::N(f64::from(u8::from(w.fs)))),
        ("chrome_top".into(), V::N(f64::from(w.top))),
        (
            "chrome_visible".into(),
            V::N(f64::from(u8::from(w.chrome_visible()))),
        ),
        ("badge".into(), V::S(badge)),
        ("words".into(), V::S(words)),
        ("chars".into(), V::S(chars)),
        (
            "zoom_visible".into(),
            V::N(f64::from(u8::from(w.zoom_ind.is_some()))),
        ),
        (
            "zoom_text".into(),
            V::S(w.zoom_ind.clone().unwrap_or_default()),
        ),
        ("disk_scheme".into(), V::S(disk_scheme)),
    ]
}

/// Minimal JSON for the state dump (no serde in the product dependency set).
mod serde_like {
    /// Value.
    pub(super) enum V {
        S(String),
        N(f64),
    }
    pub(super) trait Get {
        fn get(&self, k: &str) -> Option<&V>;
    }
    impl Get for Vec<(String, V)> {
        fn get(&self, k: &str) -> Option<&V> {
            self.iter().find(|(n, _)| n == k).map(|(_, v)| v)
        }
    }
    pub(super) fn to_json(v: &[(String, V)]) -> String {
        let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
        let body: Vec<String> = v
            .iter()
            .map(|(k, x)| match x {
                V::S(s) => format!("\"{}\":\"{}\"", esc(k), esc(s)),
                V::N(n) => format!("\"{}\":{n:.1}", esc(k)),
            })
            .collect();
        format!("{{{}}}", body.join(","))
    }
}
