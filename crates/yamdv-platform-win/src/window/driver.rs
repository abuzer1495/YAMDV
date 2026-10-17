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
}

thread_local! {
    static SCRIPT: RefCell<Option<Script>> = const { RefCell::new(None) };
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
            if let Some(s) = s.borrow_mut().as_mut() {
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
            for c in t.chars() {
                w.find_char(sh, c);
            }
            format!("ok type \"{t}\"")
        }
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
        Cmd::Resize(cw, ch) => resize(w, *cw, *ch),
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
    let ph = (ch * w.scale).round() as i32 + fh;
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
    let mut px = r.render_offscreen(cr.bottom - cr.top, me.pal.text, &mut |band| {
        me.draw(sh, band, cx, geo.as_ref());
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
    let (Ok(pw), Ok(ph)) = (u32::try_from(r.width()), u32::try_from(cr.bottom - cr.top)) else {
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
        _ => return None,
    })
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
        Some(_) => "document-guard",
    };
    let windows = WINDOWS.with(|m| m.borrow().len());
    let (find, find_count, find_index) = match &w.find {
        Some(f) => ("open", f.matches.len(), f.cur.map_or(0, |c| c + 1)),
        None => ("closed", 0, 0),
    };
    let source_len = match &w.view {
        View::Doc(d) => d.content.source().len(),
        _ => 0,
    };
    let favorites = app::with(|a| a.state.favorites.len()).unwrap_or(0);
    let (menu_items, menu_checked) = app::menu_probe(&w.menu_ctx(None));
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
        ("menu_items".into(), V::N(menu_items as f64)),
        ("menu_checked".into(), V::S(menu_checked)),
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
