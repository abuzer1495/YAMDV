//! App shell state and commands (prompt 005 §3.1–§3.11): settings + `state.ini` persistence,
//! live re-styling of every window, the owner-drawn context menu (B.7.1), and the commands it
//! runs. Runs on the UI thread only; never while a window's state is borrowed.

use super::*;
use std::cell::Cell;
use windows::Win32::Graphics::Gdi::{
    CreateFontW, CreateSolidBrush, DT_LEFT, DT_RIGHT, DT_SINGLELINE, DT_VCENTER, DeleteObject,
    DrawTextW, ExcludeClipRect, FillRect, HGDIOBJ, SelectObject, SetBkMode, SetTextColor,
    TRANSPARENT,
};
use windows::Win32::UI::Controls::{DRAWITEMSTRUCT, MEASUREITEMSTRUCT, ODS_SELECTED, ODT_MENU};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetMenuItemCount, GetMenuItemInfoW, HMENU, MENUINFO,
    MENUITEMINFOW, MF_OWNERDRAW, MF_POPUP, MF_SEPARATOR, MIIM_DATA, MIIM_SUBMENU,
    MIM_APPLYTOSUBMENUS, MIM_BACKGROUND, MNC_EXECUTE, MNC_SELECT, SetMenuInfo, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, TrackPopupMenuEx,
};
use yamdv_core::ini::Ini;
use yamdv_core::state::State;

/// The 11 schemes (R-THEME-02) in menu order.
pub(super) const SCHEMES: [(&str, &str); 11] = [
    ("default", "Default"),
    ("github", "GitHub"),
    ("glacier", "Glacier"),
    ("forest", "Forest"),
    ("sunset", "Sunset"),
    ("typewriter", "Typewriter"),
    ("editorial", "Editorial"),
    ("violet", "Violet"),
    ("lychee", "Lychee"),
    ("mint", "Mint"),
    ("neon", "Neon"),
];

/// Process-wide mutable shell state.
pub(super) struct App {
    pub settings: Settings,
    ini: Ini,
    pub state: State,
    /// Config directory (`%APPDATA%\YAMDV` or `<exe>\YAMDV-data`).
    pub dir: String,
    /// Process start (History "Current session").
    pub session_start: i64,
    /// `custom-theme.ini` watcher (only while custom colours are on) and its target window.
    theme_watch: Option<(crate::shell::Watcher, isize)>,
}

/// Stops the `custom-theme.ini` watcher (end of the message loop).
pub(super) fn shutdown() {
    with(|a| a.theme_watch = None);
}

/// Keeps the `custom-theme.ini` watcher alive on some open window while
/// `appearance.use_custom_colors` is on (B.4 hot reload); stops it otherwise.
pub(super) fn ensure_theme_watch(hint: Option<HWND>) {
    let on = get("appearance.use_custom_colors") == "true";
    let alive = |h: isize| WINDOWS.with(|m| m.borrow().contains_key(&h));
    let need = with(|a| match &a.theme_watch {
        Some((_, h)) if on && alive(*h) => false,
        _ => {
            a.theme_watch = None;
            on
        }
    })
    .unwrap_or(false);
    if !need {
        return;
    }
    let target = hint
        .map(|h| h.0 as isize)
        .filter(|h| alive(*h))
        .or_else(|| WINDOWS.with(|m| m.borrow().keys().next().copied()));
    let Some(t) = target else { return };
    let file = format!("{}\\custom-theme.ini", config_dir());
    let w = crate::shell::watch_tagged(
        &file,
        HWND(std::ptr::without_provenance_mut(t.cast_unsigned())),
        1,
    );
    if let Some(w) = w {
        with(|a| a.theme_watch = Some((w, t)));
    }
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    /// Owner-drawn menu item texts of the open menu: (label, shortcut, mark 0/1 check/2 radio).
    static ITEMS: RefCell<Vec<(String, String, u8)>> = const { RefCell::new(Vec::new()) };
    static MENU_DARK: Cell<(u32, u32, u32)> = const { Cell::new((0xffffff, 0x1f2328, 0x0969da)) };
}

/// Milliseconds since the Unix epoch.
pub(super) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(0))
}

/// Runs `f` on the app state.
pub(super) fn with<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|a| a.try_borrow_mut().ok().and_then(|mut a| a.as_mut().map(f)))
}

fn read_small(path: &str) -> Vec<u8> {
    std::fs::metadata(path)
        .ok()
        .filter(|m| m.len() <= 4 * 1024 * 1024)
        .and_then(|_| std::fs::read(path).ok())
        .unwrap_or_default()
}

/// Config directory per R-SET-05 (portable when `yamdv.portable` sits beside the exe).
pub(super) fn config_dir() -> String {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_string_lossy().into_owned()))
        .unwrap_or_default();
    let portable = std::path::Path::new(&exe_dir)
        .join("yamdv.portable")
        .exists();
    match yamdv_core::settings::resolve_dirs(
        &exe_dir,
        portable,
        &std::env::var("APPDATA").unwrap_or_default(),
        &std::env::var("LOCALAPPDATA").unwrap_or_default(),
    ) {
        yamdv_core::settings::DataDirs::Installed { config, .. } => config,
        yamdv_core::settings::DataDirs::Portable { dir } => dir,
    }
}

/// Loads `settings.ini` and `state.ini`; returns a copy of the settings.
pub(super) fn init() -> Settings {
    let dir = config_dir();
    let ini = Ini::parse(&read_small(&format!("{dir}\\settings.ini")));
    let settings = Settings::from_ini(&ini);
    let state = State::parse(&Ini::parse(&read_small(&format!("{dir}\\state.ini"))));
    let out = settings.clone();
    APP.with(|a| {
        *a.borrow_mut() = Some(App {
            settings,
            ini,
            state,
            dir,
            session_start: now_ms(),
            theme_watch: None,
        });
    });
    out
}

/// Path of `settings.ini`.
pub(super) fn settings_path() -> String {
    with(|a| format!("{}\\settings.ini", a.dir)).unwrap_or_default()
}

/// Writes `state.ini` atomically.
pub(super) fn save_state() {
    if let Some((p, t)) = with(|a| (format!("{}\\state.ini", a.dir), a.state.write())) {
        let _ = crate::shell::write_atomic(&p, &t);
    }
}

/// Sets one setting, persists `settings.ini` (round-trip safe) and re-styles every window.
/// Returns `false` when the file could not be written.
pub(super) fn set(key: &str, value: &str) -> bool {
    let Some((path, text)) = with(|a| {
        if !a.settings.set(key, value) {
            return None;
        }
        let (sec, k) = key.split_once('.')?;
        a.ini.set(sec, k, value);
        Some((format!("{}\\settings.ini", a.dir), a.ini.write()))
    })
    .flatten() else {
        return false;
    };
    let ok = crate::shell::write_atomic(&path, &text);
    apply_all();
    ok
}

/// Current value of a setting as text.
pub(super) fn get(key: &str) -> String {
    with(|a| match a.settings.get(key) {
        Some(yamdv_core::settings::Value::Bool(b)) => b.to_string(),
        Some(yamdv_core::settings::Value::Int(i)) => i.to_string(),
        Some(yamdv_core::settings::Value::Float(f)) => f.to_string(),
        Some(yamdv_core::settings::Value::Str(s)) => s.clone(),
        None => String::new(),
    })
    .unwrap_or_default()
}

/// Effective dark flag for `mode`.
pub(super) fn is_dark(mode: &str) -> bool {
    match mode {
        "dark" => true,
        "light" => false,
        _ => app_mode() == Mode::Dark,
    }
}

/// `custom-theme.ini` overrides (R-THEME-07, FUT-04 scope): 11 tokens per mode; returns `false`
/// when the file has invalid values.
pub(super) fn palette_for(scheme: &str, dark: bool) -> (Palette, bool) {
    let mut p = theme::palette(scheme, dark);
    let Some(dir) = with(|a| a.dir.clone()) else {
        return (p, true);
    };
    if get("appearance.use_custom_colors") != "true" {
        return (p, true);
    }
    let ini = Ini::parse(&read_small(&format!("{dir}\\custom-theme.ini")));
    let sec = if dark { "dark" } else { "light" };
    let mut valid = true;
    let tokens: [(&str, &mut u32); 10] = [
        ("bg", &mut p.bg),
        ("text", &mut p.text),
        ("heading", &mut p.heading),
        ("link", &mut p.link),
        ("accent", &mut p.accent),
        ("quote_bar", &mut p.quote_bar),
        ("code_bg", &mut p.code_bg),
        ("table_header_bg", &mut p.table_header_bg),
        ("code_border", &mut p.code_border),
        ("table_border", &mut p.table_border),
    ];
    for (k, slot) in tokens {
        if let Some(v) = ini.get(sec, k) {
            match parse_hex(v) {
                Some(c) => *slot = c,
                None => valid = false,
            }
        }
    }
    if let Some(v) = ini.get(sec, "border") {
        match parse_hex(v) {
            Some(c) => {
                p.code_border = c;
                p.table_border = c;
            }
            None => valid = false,
        }
    }
    (p, valid)
}

fn parse_hex(v: &str) -> Option<u32> {
    let h = v.trim().strip_prefix('#')?;
    (h.len() == 6)
        .then(|| u32::from_str_radix(h, 16).ok())
        .flatten()
}

/// Rebuilds the shared state from the current settings and re-styles every window.
pub(super) fn apply_all() {
    let Some(old) = shared() else { return };
    let Some(s) = with(|a| a.settings.clone()) else {
        return;
    };
    let Some(new) = build_shared(
        &s,
        Some((old.dw.clone(), old.fonts.clone())),
        old.dev.clone(),
        old.commit_at_start,
    ) else {
        return;
    };
    let fonts_changed = new.fonts != old.fonts;
    drop(old);
    let sh = Rc::new(new);
    SHARED.with(|c| *c.borrow_mut() = Some(sh.clone()));
    let wins: Vec<_> = WINDOWS.with(|m| m.borrow().values().cloned().collect());
    for w in wins {
        let hwnd = match w.try_borrow_mut() {
            Ok(mut w) => {
                if fonts_changed {
                    let mut rc = RECT::default();
                    // SAFETY: valid hwnd and out-pointer.
                    let _ = unsafe { GetClientRect(w.hwnd, &mut rc) };
                    w.renderer =
                        Renderer::new(&sh.dw, Some(w.hwnd), (rc.right - rc.left).max(1), w.scale)
                            .ok();
                }
                w.restyle(&sh);
                w.hwnd
            }
            Err(_) => continue,
        };
        flush_deferred(hwnd, &w);
    }
    ensure_theme_watch(None);
}

// ---- context menu (B.7.1) -------------------------------------------------------------------

/// Menu command ids.
mod cmd {
    pub const COPY: u32 = 1;
    pub const COPY_MD: u32 = 2;
    pub const COPY_LINK: u32 = 3;
    pub const SELECT_ALL: u32 = 4;
    pub const MODE0: u32 = 10; // + 0..3
    pub const SCHEME0: u32 = 20; // + 0..11
    pub const BODY_FONT: u32 = 40;
    pub const FONT_BIG: u32 = 41;
    pub const FONT_SMALL: u32 = 42;
    pub const FONT_RESET: u32 = 43;
    pub const COL_WIDE: u32 = 44;
    pub const COL_NARROW: u32 = 45;
    pub const COL_RESET: u32 = 46;
    pub const OUTLINE: u32 = 47;
    pub const OUTLINE_LEFT: u32 = 48;
    pub const OUTLINE_RIGHT: u32 = 49;
    pub const SINGLE_NL: u32 = 50;
    pub const CODE_NUMS: u32 = 51;
    pub const CODE_WRAP: u32 = 52;
    pub const EDITOR: u32 = 60;
    pub const FAVORITE: u32 = 61;
    pub const OPEN: u32 = 70;
    pub const RELOAD: u32 = 71;
    pub const EXPLORER: u32 = 72;
    pub const COPY_PATH: u32 = 73;
    pub const LANG0: u32 = 80; // + 0..3
    pub const LINKS0: u32 = 84; // + 0..3
    pub const RESTORE_POS: u32 = 88;
    pub const SET_DEFAULT: u32 = 89;
    pub const CTX_MENU: u32 = 90;
    pub const EDIT_SETTINGS: u32 = 91;
    pub const CONFIG_DIR: u32 = 92;
    pub const SHORTCUTS: u32 = 93;
    pub const ABOUT: u32 = 94;
    // V1-B editing (prompt 006 §2.6).
    pub const LIVE_EDIT: u32 = 100;
    pub const SPLIT_EDIT: u32 = 101;
    pub const SWAP_PANES: u32 = 102;
    pub const UNDO: u32 = 103;
    pub const REDO: u32 = 104;
    pub const CUT: u32 = 105;
    pub const PASTE: u32 = 106;
    /// Format ▸ items: bold, italic, strike, code, link, h1, h2, h3, paragraph, quote, bullet,
    /// numbered, task.
    pub const FMT0: u32 = 110; // + 0..13
    pub const AUTOSAVE: u32 = 130;
    pub const EFONT_BODY: u32 = 131;
    pub const EFONT_CODE: u32 = 132;
    pub const ELINES: u32 = 133;
    pub const EWRAP: u32 = 134;
    // V1-D export, print, packaging (prompt 008 §2–§4).
    pub const EXPORT_HTML: u32 = 140;
    pub const EXPORT_PDF: u32 = 141;
    pub const PRINT: u32 = 142;
    pub const CSV_COPY: u32 = 143;
    pub const CSV_EXPORT: u32 = 144;
    pub const INSTALL: u32 = 145;
}
pub(super) use cmd::*;

struct Builder {
    lang: Lang,
}

impl Builder {
    fn item(&self, m: HMENU, id: u32, label: &str, key: &str, mark: u8) {
        let idx = ITEMS.with(|i| {
            let mut i = i.borrow_mut();
            i.push((label.to_string(), key.to_string(), mark));
            i.len() - 1
        });
        // SAFETY: valid menu; owner-drawn item data is our index (no pointer).
        let _ = unsafe {
            AppendMenuW(
                m,
                MF_OWNERDRAW,
                id as usize,
                PCWSTR(std::ptr::without_provenance(idx + 1)),
            )
        };
    }
    fn msg(&self, m: HMENU, id: u32, msg: Msg, key: &str, mark: u8) {
        self.item(m, id, tr(self.lang, msg), key, mark);
    }
    fn sep(&self, m: HMENU) {
        // SAFETY: valid menu.
        let _ = unsafe { AppendMenuW(m, MF_SEPARATOR, 0, PCWSTR::null()) };
    }
    fn sub(&self, m: HMENU, msg: Msg) -> HMENU {
        // SAFETY: plain menu creation; owned by the parent after AppendMenuW.
        let s = unsafe { CreatePopupMenu() }.unwrap_or_default();
        let idx = ITEMS.with(|i| {
            let mut i = i.borrow_mut();
            i.push((tr(self.lang, msg).to_string(), "\u{203a}".to_string(), 3));
            i.len() - 1
        });
        // SAFETY: valid menus; item data is our index.
        let _ = unsafe {
            AppendMenuW(
                m,
                MF_POPUP | MF_OWNERDRAW,
                s.0 as usize,
                PCWSTR(std::ptr::without_provenance(idx + 1)),
            )
        };
        s
    }
}

fn radio(on: bool) -> u8 {
    if on { 2 } else { 0 }
}
fn check(on: bool) -> u8 {
    u8::from(on)
}

/// Context of the menu: the link under the cursor, if any.
#[derive(Default)]
pub(super) struct MenuCtx {
    pub link: Option<String>,
    pub has_doc: bool,
    pub path: Option<String>,
    /// Edit mode: 0 = read, 1 = Live Edit (F2), 2 = Split Edit (F3).
    pub edit: u8,
    /// A table is under the cursor (Copy / Export table as CSV, R-EXP-04).
    pub table: bool,
}

fn build(lang: Lang, c: &MenuCtx) -> HMENU {
    ITEMS.with(|i| i.borrow_mut().clear());
    let b = Builder { lang };
    // SAFETY: plain menu creation; destroyed after TrackPopupMenuEx.
    let m = unsafe { CreatePopupMenu() }.unwrap_or_default();
    if c.edit != 0 {
        b.msg(m, UNDO, Msg::MUndo, "Ctrl+Z", 0);
        b.msg(m, REDO, Msg::MRedo, "Ctrl+Y", 0);
        b.msg(m, CUT, Msg::MCut, "Ctrl+X", 0);
        b.msg(m, COPY, Msg::MCopy, "Ctrl+C", 0);
        b.msg(m, PASTE, Msg::MPaste, "Ctrl+V", 0);
        b.sep(m);
        let f = b.sub(m, Msg::MFormat);
        for (i, (msg, key)) in [
            (Msg::MBold, "Ctrl+B"),
            (Msg::MItalic, "Ctrl+I"),
            (Msg::MStrike, "Ctrl+Shift+X"),
            (Msg::MInlineCode, "Ctrl+`"),
            (Msg::MLink, "Ctrl+K"),
            (Msg::MHeading1, "Ctrl+1"),
            (Msg::MHeading2, "Ctrl+2"),
            (Msg::MHeading3, "Ctrl+3"),
            (Msg::MParagraph, "Ctrl+Shift+0"),
            (Msg::MQuote, "Ctrl+Shift+Q"),
            (Msg::MBulletList, "Ctrl+Shift+8"),
            (Msg::MNumberedList, "Ctrl+Shift+7"),
            (Msg::MTaskList, "Ctrl+Shift+9"),
        ]
        .into_iter()
        .enumerate()
        {
            b.msg(f, FMT0 + i as u32, msg, key, 0);
        }
        b.sep(m);
    } else {
        b.msg(m, COPY, Msg::MCopy, "Ctrl+C", 0);
    }
    b.msg(m, COPY_MD, Msg::MCopyMd, "Ctrl+Shift+C", 0);
    if c.link.is_some() {
        b.msg(m, COPY_LINK, Msg::MCopyLink, "", 0);
    }
    if c.table {
        b.msg(m, CSV_COPY, Msg::MCopyTableCsv, "", 0);
        b.msg(m, CSV_EXPORT, Msg::MExportTableCsv, "", 0);
    }
    b.msg(m, SELECT_ALL, Msg::MSelectAll, "Ctrl+A", 0);
    b.sep(m);
    let ap = b.sub(m, Msg::MAppearance);
    let mode = get("appearance.mode");
    let modes = b.sub(ap, Msg::MThemeMode);
    for (i, (v, msg)) in [
        ("system", Msg::MSystem),
        ("light", Msg::MLight),
        ("dark", Msg::MDark),
    ]
    .into_iter()
    .enumerate()
    {
        b.msg(modes, MODE0 + i as u32, msg, "", radio(mode == v));
    }
    let scheme = get("appearance.scheme");
    let sc = b.sub(ap, Msg::MScheme);
    for (i, (v, name)) in SCHEMES.iter().enumerate() {
        b.item(sc, SCHEME0 + i as u32, name, "", radio(scheme == *v));
    }
    b.msg(ap, BODY_FONT, Msg::MBodyFont, "", 0);
    let fs = b.sub(ap, Msg::MFontSize);
    b.msg(fs, FONT_BIG, Msg::MBigger, "Ctrl+=", 0);
    b.msg(fs, FONT_SMALL, Msg::MSmaller, "Ctrl+-", 0);
    b.msg(fs, FONT_RESET, Msg::MReset, "Ctrl+0", 0);
    let cw = b.sub(ap, Msg::MColumn);
    b.msg(cw, COL_WIDE, Msg::MWider, "Alt+\u{2191}", 0);
    b.msg(cw, COL_NARROW, Msg::MNarrower, "Alt+\u{2193}", 0);
    b.msg(cw, COL_RESET, Msg::MReset, "Alt+0", 0);
    b.msg(
        ap,
        OUTLINE,
        Msg::MShowOutline,
        "Ctrl+\\",
        check(get("sidebar.visible") == "true"),
    );
    let op = b.sub(ap, Msg::MOutlinePos);
    let right = get("sidebar.position").starts_with("right");
    b.msg(op, OUTLINE_LEFT, Msg::MLeft, "", radio(!right));
    b.msg(op, OUTLINE_RIGHT, Msg::MRight, "", radio(right));
    b.msg(
        ap,
        SINGLE_NL,
        Msg::MSingleNl,
        "",
        check(get("reading.single_newline_break") == "true"),
    );
    b.msg(
        ap,
        CODE_NUMS,
        Msg::MCodeNums,
        "",
        check(get("reading.code_line_numbers") == "true"),
    );
    b.msg(
        ap,
        CODE_WRAP,
        Msg::MCodeWrap,
        "",
        check(get("reading.code_wrap") == "true"),
    );
    if c.has_doc {
        b.msg(m, EDITOR, Msg::MExtEditor, "F4", 0);
        let fav = c
            .path
            .as_deref()
            .is_some_and(|p| with(|a| a.state.is_favorite(p)).unwrap_or(false));
        if fav {
            b.item(
                m,
                FAVORITE,
                &format!("\u{2605} {}", tr(lang, Msg::MFavRemove)),
                "Ctrl+D",
                0,
            );
        } else {
            b.item(
                m,
                FAVORITE,
                &format!("\u{2606} {}", tr(lang, Msg::MFavAdd)),
                "Ctrl+D",
                0,
            );
        }
    }
    if c.has_doc {
        let e = b.sub(m, Msg::MEdit);
        b.msg(e, LIVE_EDIT, Msg::MLiveEdit, "F2", check(c.edit == 1));
        b.msg(e, SPLIT_EDIT, Msg::MSplitEdit, "F3", check(c.edit == 2));
        b.msg(
            e,
            SWAP_PANES,
            Msg::MSwapPanes,
            "",
            check(get("editing.split_swapped") == "true"),
        );
    }
    let f = b.sub(m, Msg::MFile);
    b.msg(f, OPEN, Msg::MOpen, "Ctrl+O", 0);
    if c.has_doc {
        b.msg(f, RELOAD, Msg::MReload, "F5", 0);
        b.msg(f, EXPLORER, Msg::MShowExplorer, "", 0);
        b.msg(f, COPY_PATH, Msg::MCopyPath, "", 0);
    }
    if c.has_doc {
        let x = b.sub(m, Msg::MExport);
        b.msg(x, EXPORT_HTML, Msg::MExportHtml, "", 0);
        b.msg(x, EXPORT_PDF, Msg::MExportPdf, "Ctrl+Shift+E", 0);
        b.msg(x, PRINT, Msg::MPrint, "Ctrl+P", 0);
    }
    let st = b.sub(m, Msg::MSettings);
    let lang_v = get("general.language");
    let lg = b.sub(st, Msg::MLanguage);
    b.msg(lg, LANG0, Msg::MSystem, "", radio(lang_v == "system"));
    b.item(lg, LANG0 + 1, "English", "", radio(lang_v == "en"));
    b.item(lg, LANG0 + 2, "Türkçe", "", radio(lang_v == "tr"));
    let ext = get("general.open_external_links");
    let el = b.sub(st, Msg::MExtLinks);
    for (i, (v, msg)) in [
        ("ask", Msg::MAsk),
        ("direct", Msg::MDirect),
        ("never", Msg::MNever),
    ]
    .into_iter()
    .enumerate()
    {
        b.msg(el, LINKS0 + i as u32, msg, "", radio(ext == v));
    }
    b.msg(
        st,
        RESTORE_POS,
        Msg::MRestorePos,
        "",
        check(get("general.restore_reading_position") == "true"),
    );
    b.msg(st, SET_DEFAULT, Msg::MSetDefault, "", 0);
    b.msg(
        st,
        CTX_MENU,
        Msg::MCtxMenu,
        "",
        check(get("integration.context_menu") == "true"),
    );
    b.msg(
        st,
        AUTOSAVE,
        Msg::MAutosave,
        "",
        check(get("editing.autosave") == "true"),
    );
    let efont = get("editing.editor_font");
    let ef = b.sub(st, Msg::MEditorFont);
    b.msg(ef, EFONT_BODY, Msg::MFontBody, "", radio(efont != "code"));
    b.msg(ef, EFONT_CODE, Msg::MFontCode, "", radio(efont == "code"));
    b.msg(
        st,
        ELINES,
        Msg::MEditorLineNumbers,
        "",
        check(get("editing.editor_line_numbers") == "true"),
    );
    b.msg(
        st,
        EWRAP,
        Msg::MEditorSoftWrap,
        "",
        check(get("editing.editor_soft_wrap") == "true"),
    );
    b.msg(st, EDIT_SETTINGS, Msg::MEditSettings, "", 0);
    b.msg(st, CONFIG_DIR, Msg::MConfigFolder, "", 0);
    b.sep(m);
    b.msg(m, SHORTCUTS, Msg::MShortcuts, "F1", 0);
    b.msg(m, ABOUT, Msg::MAbout, "", 0);
    m
}

/// Driver probe: builds the menu without showing it; (item count, checked labels joined by `|`).
#[cfg(feature = "dev-tools")]
pub(super) fn menu_probe(c: &MenuCtx) -> (usize, String) {
    let Some(sh) = shared() else {
        return (0, String::new());
    };
    let m = build(sh.lang, c);
    // SAFETY: menu created by build(); destroyed here.
    let _ = unsafe { DestroyMenu(m) };
    ITEMS.with(|i| {
        let i = i.borrow();
        let checked: Vec<&str> = i
            .iter()
            .filter(|x| x.2 == 1 || x.2 == 2)
            .map(|x| x.0.as_str())
            .collect();
        (i.len(), checked.join("|"))
    })
}

/// Shows the context menu at screen point `(x, y)` and returns the chosen command (0 = none).
pub(super) fn track(hwnd: HWND, x: i32, y: i32, c: &MenuCtx, pal: &Palette) -> u32 {
    let Some(sh) = shared() else { return 0 };
    MENU_DARK.with(|d| d.set((pal.bg, pal.text, pal.accent)));
    let m = build(sh.lang, c);
    // SAFETY: brush lives until the menu is destroyed below.
    let brush = unsafe { CreateSolidBrush(colorref(pal.bg)) };
    let mi = MENUINFO {
        cbSize: size_of::<MENUINFO>() as u32,
        fMask: MIM_BACKGROUND | MIM_APPLYTOSUBMENUS,
        hbrBack: brush,
        ..Default::default()
    };
    // SAFETY: valid menu and MENUINFO; TrackPopupMenuEx runs a modal loop with no borrow held.
    unsafe {
        let _ = SetMenuInfo(m, &mi);
        let r = TrackPopupMenuEx(m, (TPM_RETURNCMD | TPM_RIGHTBUTTON).0, x, y, hwnd, None);
        let _ = DestroyMenu(m);
        let _ = DeleteObject(HGDIOBJ(brush.0));
        r.0 as u32
    }
}

fn colorref(rgb: u32) -> windows::Win32::Foundation::COLORREF {
    windows::Win32::Foundation::COLORREF(
        ((rgb >> 16) & 0xff) | (rgb & 0xff00) | ((rgb & 0xff) << 16),
    )
}

fn item_text(data: usize) -> Option<(String, String, u8)> {
    ITEMS.with(|i| i.borrow().get(data.checked_sub(1)?).cloned())
}

/// `WM_MEASUREITEM` for menu items.
pub(super) fn measure(hwnd: HWND, lp: LPARAM) -> bool {
    // SAFETY: for WM_MEASUREITEM lParam points to a writable MEASUREITEMSTRUCT.
    let mis = unsafe { &mut *(lp.0 as *mut MEASUREITEMSTRUCT) };
    if mis.CtlType != ODT_MENU {
        return false;
    }
    // SAFETY: valid hwnd.
    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
    let (label, key, _) = item_text(mis.itemData).unwrap_or_default();
    let chars = label.chars().count() + key.chars().count();
    mis.itemWidth = scale(
        40 + 8 * chars as i32 + if key.is_empty() { 0 } else { 32 },
        dpi,
    ) as u32;
    mis.itemHeight = scale(28, dpi) as u32;
    true
}

/// `WM_DRAWITEM` for menu items (theme colors, check/radio marks, shortcut column).
pub(super) fn draw_item(hwnd: HWND, lp: LPARAM) -> bool {
    // SAFETY: for WM_DRAWITEM lParam points to a DRAWITEMSTRUCT valid during the call.
    let dis = unsafe { &*(lp.0 as *const DRAWITEMSTRUCT) };
    if dis.CtlType != ODT_MENU {
        return false;
    }
    let (bg, fg, accent) = MENU_DARK.with(Cell::get);
    let (label, key, mark) = item_text(dis.itemData).unwrap_or_default();
    // SAFETY: valid hwnd.
    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
    let sel = dis.itemState.0 & ODS_SELECTED.0 != 0;
    let hdc = dis.hDC;
    let mut rc = dis.rcItem;
    let hover = theme::mix(bg, accent);
    // SAFETY: valid DC from the system; created GDI objects are deleted below.
    unsafe {
        let br = CreateSolidBrush(colorref(if sel { hover } else { bg }));
        FillRect(hdc, &rc, br);
        let _ = DeleteObject(HGDIOBJ(br.0));
        let font = CreateFontW(
            -scale(13, dpi),
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            windows::Win32::Graphics::Gdi::DEFAULT_CHARSET,
            windows::Win32::Graphics::Gdi::OUT_DEFAULT_PRECIS,
            windows::Win32::Graphics::Gdi::CLIP_DEFAULT_PRECIS,
            windows::Win32::Graphics::Gdi::CLEARTYPE_QUALITY,
            0,
            w!("Segoe UI"),
        );
        let old = SelectObject(hdc, HGDIOBJ(font.0));
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, colorref(fg));
        let pad = scale(12, dpi);
        if mark != 0 && mark != 3 {
            let mut mr = rc;
            mr.left += pad;
            let mut t: Vec<u16> = if mark == 1 { "\u{2713}" } else { "\u{25CF}" }
                .encode_utf16()
                .collect();
            DrawTextW(hdc, &mut t, &mut mr, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
        }
        rc.left += scale(36, dpi);
        rc.right -= pad;
        let mut t: Vec<u16> = label.encode_utf16().collect();
        DrawTextW(hdc, &mut t, &mut rc, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
        if !key.is_empty() {
            SetTextColor(hdc, colorref(theme::mix(fg, bg)));
            let mut k: Vec<u16> = key.encode_utf16().collect();
            DrawTextW(hdc, &mut k, &mut rc, DT_RIGHT | DT_SINGLELINE | DT_VCENTER);
        }
        SelectObject(hdc, old);
        let _ = DeleteObject(HGDIOBJ(font.0));
        if mark == 3 {
            // Our own "›" replaces the system submenu arrow (drawn in the system color).
            let r = dis.rcItem;
            let _ = ExcludeClipRect(hdc, r.left, r.top, r.right, r.bottom);
        }
    }
    true
}

/// `WM_MENUCHAR`: typing a letter jumps to (and opens/executes) the first matching item.
pub(super) fn menu_char(wp: WPARAM, lp: LPARAM) -> Option<LRESULT> {
    let ch = char::from_u32((wp.0 & 0xFFFF) as u32)?
        .to_lowercase()
        .next()?;
    let m = HMENU(lp.0 as *mut core::ffi::c_void);
    // SAFETY: the system passes the active menu handle.
    let n = unsafe { GetMenuItemCount(Some(m)) };
    for i in 0..n.max(0) {
        let mut mii = MENUITEMINFOW {
            cbSize: size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_DATA | MIIM_SUBMENU,
            ..Default::default()
        };
        // SAFETY: valid menu and structure.
        if unsafe { GetMenuItemInfoW(m, i as u32, true, &mut mii) }.is_err() {
            continue;
        }
        let Some((label, _, _)) = item_text(mii.dwItemData) else {
            continue;
        };
        let first = label
            .chars()
            .find(|c| c.is_alphanumeric())
            .and_then(|c| c.to_lowercase().next());
        if first == Some(ch) {
            let action = if mii.hSubMenu.is_invalid() {
                MNC_EXECUTE
            } else {
                MNC_SELECT
            };
            return Some(LRESULT(((action as isize) << 16) | i as isize));
        }
    }
    None
}

// ---- commands -------------------------------------------------------------------------------

/// Built-in help document (F1), compiled into the exe.
pub(super) fn help_text(lang: Lang) -> &'static str {
    match lang {
        Lang::Tr => include_str!("../../assets/help_tr.md"),
        Lang::En => include_str!("../../assets/help_en.md"),
    }
}

fn toggle(key: &str) {
    let v = get(key) == "true";
    set(key, if v { "false" } else { "true" });
}

/// Zoom step (prompt 005 §3.9): font size ±1 px, column width ±5 %; `0` resets.
pub(super) fn zoom(hwnd: HWND, font: bool, delta: i64) {
    let (key, def, step, msg, unit) = if font {
        ("typography.font_size", 16, 1, Msg::ToastFontSize, " px")
    } else {
        ("typography.column_width_pct", 65, 5, Msg::ToastColumn, " %")
    };
    let cur: i64 = get(key).parse().unwrap_or(def);
    let next = if delta == 0 { def } else { cur + delta * step };
    set(key, &next.to_string());
    let shown = get(key);
    with_win(hwnd, |w, _| w.toast_with(msg, &format!("{shown}{unit}")));
}

/// Runs `f` on the window state of `hwnd` (then flushes its deferred calls).
pub(super) fn with_win(hwnd: HWND, f: impl FnOnce(&mut Win, &Shared)) {
    let (Some(win), Some(sh)) = (
        WINDOWS.with(|m| m.borrow().get(&(hwnd.0 as isize)).cloned()),
        shared(),
    ) else {
        return;
    };
    if let Ok(mut w) = win.try_borrow_mut() {
        f(&mut w, &sh);
    }
    flush_deferred(hwnd, &win);
}

/// Executes a menu command (also used by keyboard shortcuts and the test driver).
pub(super) fn run_command(hwnd: HWND, id: u32, link: Option<String>) -> Vec<Action> {
    let mut out = Vec::new();
    let scheme = |i: u32| SCHEMES.get(i as usize).map(|s| s.0);
    match id {
        COPY => with_win(hwnd, |w, _| w.copy()),
        COPY_MD => with_win(hwnd, |w, _| w.copy_markdown()),
        COPY_LINK => {
            if let Some(l) = link {
                with_win(hwnd, |w, _| {
                    if sys::copy_text(w.hwnd, &l) {
                        w.toast(Msg::ToastLinkCopied);
                    }
                });
            }
        }
        SELECT_ALL => with_win(hwnd, |w, _| w.select_all()),
        i if (MODE0..MODE0 + 3).contains(&i) => {
            let v = ["system", "light", "dark"][(i - MODE0) as usize];
            set("appearance.mode", v);
        }
        i if (SCHEME0..SCHEME0 + 11).contains(&i) => {
            if let Some(v) = scheme(i - SCHEME0) {
                set("appearance.scheme", v);
            }
        }
        BODY_FONT => out.push(Action::FontDialog),
        FONT_BIG => zoom(hwnd, true, 1),
        FONT_SMALL => zoom(hwnd, true, -1),
        FONT_RESET => zoom(hwnd, true, 0),
        COL_WIDE => zoom(hwnd, false, 1),
        COL_NARROW => zoom(hwnd, false, -1),
        COL_RESET => zoom(hwnd, false, 0),
        OUTLINE => toggle("sidebar.visible"),
        OUTLINE_LEFT => {
            set("sidebar.position", "left");
        }
        OUTLINE_RIGHT => {
            set("sidebar.position", "right");
        }
        SINGLE_NL => toggle("reading.single_newline_break"),
        CODE_NUMS => toggle("reading.code_line_numbers"),
        CODE_WRAP => toggle("reading.code_wrap"),
        EDITOR => with_win(hwnd, |w, _| w.open_editor()),
        FAVORITE => with_win(hwnd, |w, _| w.toggle_favorite()),
        OPEN => out.push(Action::OpenDialog),
        EXPORT_HTML => out.push(Action::Export(super::ExportKind::Html)),
        EXPORT_PDF => out.push(Action::Export(super::ExportKind::Pdf)),
        PRINT => out.push(Action::Export(super::ExportKind::Print)),
        CSV_COPY => with_win(hwnd, |w, _| {
            if let Some(t) = w.ctx_table.clone()
                && sys::copy_text(w.hwnd, &t)
            {
                w.toast(Msg::Copied);
            }
        }),
        CSV_EXPORT => {
            let mut t = None;
            with_win(hwnd, |w, _| t = w.ctx_table.clone());
            if let Some(t) = t {
                out.push(Action::Export(super::ExportKind::Csv(t)));
            }
        }
        INSTALL => out.push(Action::Install),
        RELOAD => with_win(hwnd, |w, sh| w.reload(sh)),
        EXPLORER => with_win(hwnd, |w, _| {
            if let View::Doc(d) = &w.view
                && !d.virt
            {
                sys::reveal_in_explorer(&d.path);
            }
        }),
        COPY_PATH => with_win(hwnd, |w, _| {
            if let View::Doc(d) = &w.view
                && sys::copy_text(w.hwnd, &ypath::display_form(&d.path))
            {
                w.toast(Msg::ToastPathCopied);
            }
        }),
        i if (LANG0..LANG0 + 3).contains(&i) => {
            set(
                "general.language",
                ["system", "en", "tr"][(i - LANG0) as usize],
            );
        }
        i if (LINKS0..LINKS0 + 3).contains(&i) => {
            set(
                "general.open_external_links",
                ["ask", "direct", "never"][(i - LINKS0) as usize],
            );
        }
        RESTORE_POS => toggle("general.restore_reading_position"),
        SET_DEFAULT => {
            let ok = crate::shell::set_default_flow();
            with_win(hwnd, |w, _| {
                w.toast(if ok {
                    Msg::ToastDefaultApps
                } else {
                    Msg::ToastSaveFailed
                });
            });
        }
        CTX_MENU => {
            let on = get("integration.context_menu") != "true";
            let label = shared().map_or("Open with YAMDV", |s| {
                if s.lang == Lang::Tr {
                    "YAMDV ile aç"
                } else {
                    "Open with YAMDV"
                }
            });
            let ok = crate::shell::set_context_menu(on, label);
            set(
                "integration.context_menu",
                if on { "true" } else { "false" },
            );
            with_win(hwnd, |w, _| {
                w.toast(match (ok, on) {
                    (false, _) => Msg::ToastSaveFailed,
                    (true, true) => Msg::ToastCtxOn,
                    (true, false) => Msg::ToastCtxOff,
                });
            });
        }
        EDIT_SETTINGS => {
            let p = settings_path();
            if !std::path::Path::new(&p).exists() {
                let text = with(|a| {
                    let mut ini = a.ini.clone();
                    a.settings.write_into(&mut ini);
                    ini.write()
                })
                .unwrap_or_default();
                let _ = crate::shell::write_atomic(&p, &text);
            }
            let s = with(|a| a.settings.clone()).unwrap_or_default();
            let ok = crate::shell::open_in_editor(
                &p,
                1,
                s.str("external_editor.custom_exe"),
                s.str("external_editor.custom_args"),
                s.str("external_editor.mode") == "custom",
            );
            if !ok {
                with_win(hwnd, |w, _| w.toast(Msg::ToastNoEditor));
            }
        }
        CONFIG_DIR => {
            let p = settings_path();
            if std::path::Path::new(&p).exists() {
                sys::reveal_in_explorer(&p);
            } else if let Some(d) = with(|a| a.dir.clone()) {
                let _ = std::fs::create_dir_all(&d);
                sys::reveal_in_explorer(&d);
            }
        }
        SHORTCUTS => out.push(Action::Help),
        ABOUT => with_win(hwnd, |w, sh| w.about(sh)),
        SWAP_PANES => toggle("editing.split_swapped"),
        AUTOSAVE => toggle("editing.autosave"),
        EFONT_BODY => {
            set("editing.editor_font", "body");
        }
        EFONT_CODE => {
            set("editing.editor_font", "code");
        }
        ELINES => toggle("editing.editor_line_numbers"),
        EWRAP => toggle("editing.editor_soft_wrap"),
        LIVE_EDIT | SPLIT_EDIT | UNDO | REDO | CUT | PASTE => {
            with_win(hwnd, |w, sh| w.edit_command(sh, id));
        }
        i if (FMT0..FMT0 + 13).contains(&i) => with_win(hwnd, |w, sh| w.edit_command(sh, id)),
        _ => {}
    }
    out
}

impl Win {
    /// Applies the current shared state (theme, fonts, reading options) to this window.
    pub(super) fn restyle(&mut self, sh: &Shared) {
        self.dark = is_dark(&sh.mode);
        let (pal, valid) = palette_for(&sh.scheme, self.dark);
        self.pal = pal;
        if !valid {
            self.toast(Msg::ToastThemeInvalid);
        }
        apply_mode(self.hwnd, if self.dark { Mode::Dark } else { Mode::Light });
        update_size(self, sh);
        if matches!(self.view, View::Doc(_)) {
            self.reload(sh);
        }
        self.invalidate();
    }

    /// Re-reads and re-lays out the document, keeping the top block (R-READ-01).
    pub(super) fn reload(&mut self, sh: &Shared) {
        // Edit mode: rebuild from the buffer, never from disk (V1-B).
        if self.edit_rebuild(sh) {
            return;
        }
        let View::Doc(d) = &self.view else { return };
        let (block, off) = d.viewer.anchor();
        let path = d.path.clone();
        let plain = matches!(d.content, Content::Plain { .. });
        let virt = d.virt;
        let sel = self.sel;
        if virt {
            self.show_help(sh);
        } else {
            self.open_inner(sh, &path, Some(plain), true, false);
        }
        self.sel = sel;
        if let View::Doc(d) = &self.view {
            let y = d.viewer.block_top(&d.content, block) + off;
            self.set_scroll(y);
        }
        self.invalidate();
    }

    /// Select all (`Ctrl+A`).
    pub(super) fn select_all(&mut self) {
        if let View::Doc(d) = &self.view {
            self.sel = Some((
                0,
                u32::try_from(d.content.source().len()).unwrap_or(u32::MAX),
            ));
            self.invalidate();
        }
    }

    /// Source line (1-based) of the selection start or the top visible block (R-EXT-01).
    fn current_line(&self) -> u32 {
        let View::Doc(d) = &self.view else { return 1 };
        let off = match self.sel {
            Some((a, b)) => a.min(b) as usize,
            None => d.content.range(d.viewer.anchor().0).start as usize,
        };
        let src = d.content.source();
        let n = src
            .get(..off.min(src.len()))
            .map_or(0, |s| s.matches('\n').count());
        u32::try_from(n + 1).unwrap_or(1)
    }

    /// `F4` / `Ctrl+E` (R-EXT-01…03).
    pub(super) fn open_editor(&mut self) {
        let View::Doc(d) = &self.view else { return };
        if d.virt {
            return;
        }
        let path = d.path.clone();
        let line = self.current_line();
        let s = with(|a| a.settings.clone()).unwrap_or_default();
        let ok = crate::shell::open_in_editor(
            &path,
            line,
            s.str("external_editor.custom_exe"),
            s.str("external_editor.custom_args"),
            s.str("external_editor.mode") == "custom",
        );
        if !ok {
            self.toast(Msg::ToastNoEditor);
        }
    }

    /// `Ctrl+D` (R-NAV-09).
    pub(super) fn toggle_favorite(&mut self) {
        let View::Doc(d) = &self.view else { return };
        if d.virt {
            return;
        }
        let p = d.path.clone();
        let on = with(|a| a.state.toggle_favorite(&p)).unwrap_or(false);
        save_state();
        self.toast(if on {
            Msg::ToastFavAdded
        } else {
            Msg::ToastFavRemoved
        });
    }

    /// About dialog (in-window, version + paths).
    pub(super) fn about(&mut self, sh: &Shared) {
        let body = format!(
            "{}{}\n{}{}\n{}{}",
            tr(sh.lang, Msg::AboutVersion),
            env!("CARGO_PKG_VERSION"),
            tr(sh.lang, Msg::AboutExe),
            crate::shell::exe_path(),
            tr(sh.lang, Msg::AboutConfig),
            settings_path()
        );
        self.ask_text(
            sh,
            Msg::AboutTitle,
            body,
            vec![(Msg::Ok, BTN_CANCEL)],
            Pending::Info,
        );
    }

    /// Shows the built-in help document (F1) in this window.
    pub(super) fn show_help(&mut self, sh: &Shared) {
        let text = help_text(sh.lang).to_string();
        self.show_text(sh, "YAMDV help".to_string(), text, false, true);
    }

    /// Saves this window's reading position (R-READ-02).
    pub(super) fn save_position(&self) {
        let View::Doc(d) = &self.view else { return };
        if d.virt {
            return;
        }
        let (b, off) = d.viewer.anchor();
        let h = d.viewer.layout_of(b).map_or(0.0, |l| f64::from(l.height));
        let ppm = if h > 0.0 {
            (off / h * 1e6).clamp(0.0, 1e6) as u32
        } else {
            0
        };
        let p = d.path.clone();
        with(|a| {
            a.state.set_position(
                &p,
                yamdv_core::state::Position {
                    block: u32::try_from(b).unwrap_or(0),
                    ppm,
                },
                now_ms(),
            );
        });
    }

    /// Restores the saved reading position of the current document.
    pub(super) fn restore_position(&mut self, sh: &Shared) {
        if !sh.restore_pos {
            return;
        }
        let View::Doc(d) = &self.view else { return };
        let Some(pos) = with(|a| a.state.position(&d.path)).flatten() else {
            return;
        };
        let b = (pos.block as usize).min(d.content.len().saturating_sub(1));
        let top = d.viewer.block_top(&d.content, b);
        let h = d.viewer.block_top(&d.content, b + 1) - top;
        self.set_scroll(top + h.max(0.0) * f64::from(pos.ppm) / 1e6);
    }
}

// ---- instances, windows, geometry -----------------------------------------------------------

/// S-IPC-01/02: a valid forward message from a process of the same user → (paths, line).
pub(super) fn accept_forward(sender: HWND, cds: &COPYDATASTRUCT) -> Option<(Vec<String>, u64)> {
    if cds.dwData != yamdv_core::ipc::MAGIC as usize
        || cds.lpData.is_null()
        || cds.cbData as usize > yamdv_core::ipc::FORWARD_MAX_BYTES
    {
        return None;
    }
    // SAFETY: the system copied `cbData` bytes at `lpData` for the duration of this message.
    let bytes = unsafe { std::slice::from_raw_parts(cds.lpData.cast::<u8>(), cds.cbData as usize) };
    let r = yamdv_core::ipc::decode(bytes).ok()?;
    if r.kind != yamdv_core::ipc::Kind::Forward {
        return None;
    }
    let mut pid = 0u32;
    // SAFETY: tolerates stale handles (returns 0).
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(sender, Some(&mut pid))
    };
    if !crate::dialog_host::same_user_pid(pid) {
        return None;
    }
    match r.outcome {
        yamdv_core::ipc::Outcome::Paths(p) => Some((p, r.nonce)),
        _ => None,
    }
}

/// The window showing `full`, if any (R-WIN-02).
fn window_of(full: &str) -> Option<HWND> {
    WINDOWS.with(|m| {
        m.borrow().values().find_map(|w| {
            let w = w.try_borrow().ok()?;
            match &w.view {
                View::Doc(d) if !d.virt && d.path.eq_ignore_ascii_case(full) => Some(w.hwnd),
                _ => None,
            }
        })
    })
}

fn activate(hwnd: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{IsIconic, SW_RESTORE};
    // SAFETY: valid hwnd.
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        if !SetForegroundWindow(hwnd).as_bool() {
            use windows::Win32::UI::WindowsAndMessaging::{FLASHW_TRAY, FLASHWINFO, FlashWindowEx};
            let fi = FLASHWINFO {
                cbSize: size_of::<FLASHWINFO>() as u32,
                hwnd,
                dwFlags: FLASHW_TRAY,
                uCount: 3,
                dwTimeout: 0,
            };
            let _ = FlashWindowEx(&fi);
        }
    }
}

/// One window per file (R-WIN-02): activates the window of `path` or opens a new one; `line`
/// scrolls to a source line (`--line`). Returns the window.
pub(super) fn open_or_activate(path: &str, line: Option<u32>) -> Option<HWND> {
    let sh = shared()?;
    let full = sys::full_path(path).unwrap_or_else(|| path.to_string());
    let h = match window_of(&full) {
        Some(h) => {
            activate(h);
            h
        }
        None => create_window(&sh, Some(path))?,
    };
    if let Some(l) = line {
        with_win(h, |w, _| w.goto_line(l));
    }
    Some(h)
}

/// Paths forwarded by a second launch (S-IPC-03: validated like user-opened paths; open only).
pub(super) fn open_forwarded(hwnd: HWND, paths: Vec<String>, line: u64) {
    let line = u32::try_from(line).ok().filter(|l| *l > 0);
    if paths.is_empty() {
        activate(hwnd);
        return;
    }
    for p in paths {
        let full = sys::full_path(&p).unwrap_or_else(|| p.clone());
        // The empty window of the primary instance is reused.
        let empty = window_of(&full).is_none()
            && WINDOWS.with(|m| {
                m.borrow().get(&(hwnd.0 as isize)).is_some_and(|w| {
                    w.try_borrow()
                        .is_ok_and(|w| !matches!(w.view, View::Doc(_)))
                })
            });
        if empty {
            with_win(hwnd, |w, sh| w.open(sh, &p, None, false));
            if let Some(l) = line {
                with_win(hwnd, |w, _| w.goto_line(l));
            }
            activate(hwnd);
        } else {
            open_or_activate(&p, line);
        }
    }
}

fn monitors() -> (
    Vec<yamdv_core::state::WorkArea>,
    yamdv_core::state::WorkArea,
) {
    use windows::Win32::Foundation::{LPARAM, RECT};
    use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR};
    const MONITORINFOF_PRIMARY: u32 = 1;
    extern "system" fn cb(m: HMONITOR, _: HDC, _: *mut RECT, lp: LPARAM) -> windows::core::BOOL {
        // SAFETY: lp is the Vec passed below, alive for the synchronous enumeration.
        let v = unsafe { &mut *(lp.0 as *mut Vec<(yamdv_core::state::WorkArea, bool)>) };
        let mut mi = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        // SAFETY: valid monitor handle and sized structure.
        if unsafe { GetMonitorInfoW(m, &mut mi) }.as_bool() {
            let r = mi.rcWork;
            v.push((
                (r.left, r.top, r.right, r.bottom),
                mi.dwFlags & MONITORINFOF_PRIMARY != 0,
            ));
        }
        true.into()
    }
    let mut v: Vec<(yamdv_core::state::WorkArea, bool)> = Vec::new();
    // SAFETY: callback only touches `v` during the call.
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(cb), LPARAM(&raw mut v as isize));
    }
    let primary = v
        .iter()
        .find(|x| x.1)
        .or(v.first())
        .map_or((0, 0, 1280, 720), |x| x.0);
    (v.into_iter().map(|x| x.0).collect(), primary)
}

/// R-WIN-03: the remembered window rectangle, validated against the current monitors.
pub(super) fn restored_geometry() -> Option<yamdv_core::state::WinGeom> {
    if get("general.remember_window_geometry") == "false" {
        return None;
    }
    let g = with(|a| a.state.window).flatten()?;
    let (mons, primary) = monitors();
    Some(yamdv_core::state::validate_geom(g, &mons, primary))
}

/// Stores the (restored) window rectangle of `hwnd`.
pub(super) fn save_geometry(hwnd: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowPlacement, SW_SHOWMAXIMIZED, WINDOWPLACEMENT,
    };
    let mut wp = WINDOWPLACEMENT {
        length: size_of::<WINDOWPLACEMENT>() as u32,
        ..Default::default()
    };
    // SAFETY: valid hwnd and sized structure.
    if unsafe { GetWindowPlacement(hwnd, &mut wp) }.is_err() {
        return;
    }
    let r = wp.rcNormalPosition;
    let g = yamdv_core::state::WinGeom {
        x: r.left,
        y: r.top,
        w: r.right - r.left,
        h: r.bottom - r.top,
        max: wp.showCmd == SW_SHOWMAXIMIZED.0 as u32,
    };
    with(|a| a.state.window = Some(g));
}

/// Body font… (`ChooseFontW` in the dialog host, family name only).
pub(super) fn font_dialog(hwnd: HWND) {
    if HOST.with(|h| h.borrow().is_some()) {
        return;
    }
    match crate::dialog_host::launch_kind(hwnd, yamdv_core::ipc::Kind::Font) {
        Some(h) => {
            // SAFETY: valid hwnd; WM_ENABLE is not handled by us.
            let _ = unsafe { EnableWindow(hwnd, false) };
            HOST.with(|x| *x.borrow_mut() = Some(h));
        }
        None => show_toast(hwnd, Msg::DialogHostFailed),
    }
}

impl Win {
    /// Context menu at the keyboard position (Shift+F10, Menu key): top-left of the column.
    pub(super) fn kbd_menu(&self) -> Action {
        let mut pt = POINT {
            x: (48.0 * self.scale) as i32,
            y: (48.0 * self.scale) as i32,
        };
        // SAFETY: valid hwnd and point.
        let _ = unsafe { windows::Win32::Graphics::Gdi::ClientToScreen(self.hwnd, &mut pt) };
        Action::ContextMenu(pt.x, pt.y, None)
    }

    /// Destination of the link under client point `(x, y)` (Copy link address).
    pub(super) fn link_at(&mut self, sh: &Shared, x: f32, y: f32) -> Option<String> {
        let (cx, _) = self.col(sh);
        let View::Doc(d) = &mut self.view else {
            return None;
        };
        let doc_y = f64::from(y) + d.viewer.scroll;
        let (bi, k) = d.viewer.hot_at(&d.content, x - cx, doc_y)?;
        let HotKind::Link { link } = k else {
            return None;
        };
        let Content::Markdown(doc) = &mut d.content else {
            return None;
        };
        let det = doc.detail(bi);
        det.links.get(link as usize).map(|l| l.dest.to_string())
    }

    pub(super) fn menu_ctx(&self, link: Option<String>) -> MenuCtx {
        match &self.view {
            View::Doc(d) => MenuCtx {
                link,
                has_doc: !d.virt,
                path: Some(d.path.clone()),
                edit: self.edit_mode_id(),
                table: self.ctx_table.is_some(),
            },
            _ => MenuCtx::default(),
        }
    }

    /// Scrolls to source line `line` (1-based, `--line`).
    pub(super) fn goto_line(&mut self, line: u32) {
        let View::Doc(d) = &self.view else { return };
        let src = d.content.source();
        let off = src
            .match_indices('\n')
            .nth((line as usize).saturating_sub(2))
            .map_or(0, |(i, _)| if line <= 1 { 0 } else { i + 1 });
        let off = u32::try_from(off).unwrap_or(0);
        let n = d.content.len();
        let b = (0..n)
            .rfind(|&i| d.content.range(i).start <= off)
            .unwrap_or(0);
        let y = d.viewer.block_top(&d.content, b);
        self.set_scroll(y);
        self.invalidate();
    }

    /// Debounced watcher notification (R-READ-01, R-OPEN-11).
    pub(super) fn file_changed(&mut self, sh: &Shared) {
        if self.edit_file_changed(sh) {
            return;
        }
        let View::Doc(d) = &self.view else { return };
        if d.virt {
            return;
        }
        let long = ypath::long_form(&d.path);
        if std::path::Path::new(&long).is_file() {
            self.dimmed = false;
            self.reload(sh);
        } else if !self.dimmed {
            self.dimmed = true;
            self.toast(Msg::ToastDeleted);
            self.invalidate();
        }
    }
}
