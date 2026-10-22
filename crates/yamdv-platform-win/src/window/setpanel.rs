//! Settings panel (FUT-31, B.8, prompt 012 §4): the in-window modal card with section
//! navigation, search (FUT-26), toggles / segmented controls / dropdowns / sliders / text
//! fields, the font pickers (FUT-49), the colour editor (FUT-04) and the shortcut editor
//! (FUT-05). Every change applies at once through `app::set` (atomic `settings.ini` write +
//! live restyle); operations that need the app state run as [`Op`]s outside the window borrow.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use super::*;
use std::rc::Rc;
use yamdv_core::colors::{self, TOKENS};
use yamdv_core::keymap::{self as km, ACTIONS, Chord, Keymap};
use yamdv_editor::{Editor, Motion};
use yamdv_ui::settings_panel::{self as sp, Control, ROWS, SECTIONS, Section};

/// Dropdown popup ids: `SET_OPT0 + option index` (routed through `app::run_command`).
pub(super) const SET_OPT0: u32 = 9000;
/// Confirm button of the panel's confirmations.
pub(super) const BTN_PANEL_OK: u32 = 31;
const SMALL_H: f32 = 40.0;
const FONT_ROW_H: f32 = 28.0;
const VK_BACK: u16 = 0x08;
const VK_TAB_: u16 = 0x09;
const VK_ENTER: u16 = 0x0D;
const VK_ESC: u16 = 0x1B;
const VK_SPACE_: u16 = 0x20;
const VK_PGUP: u16 = 0x21;
const VK_PGDN: u16 = 0x22;
const VK_END_: u16 = 0x23;
const VK_HOME_: u16 = 0x24;
const VK_LEFT_: u16 = 0x25;
const VK_UP_: u16 = 0x26;
const VK_RIGHT_: u16 = 0x27;
const VK_DOWN_: u16 = 0x28;
const VK_DEL: u16 = 0x2E;

/// Installed families `(name, monospace)`, shared by the font list and the cache.
type Fonts = Rc<Vec<(String, bool)>>;

thread_local! {
    /// Installed font families, enumerated once when a picker first opens (FUT-49).
    static FONTS: RefCell<Option<Fonts>> = const { RefCell::new(None) };
}

fn rr(r: yamdv_ui::dialog::Rect) -> Rect {
    Rect::new(r.x, r.y, r.w, r.h)
}

fn pick(lang: Lang, en: &'static str, tr: &'static str) -> &'static str {
    match lang {
        Lang::En => en,
        Lang::Tr => tr,
    }
}

/// A one-line text field on the editor core.
pub(super) struct Field {
    ed: Editor,
}

impl Field {
    pub(super) fn new(t: &str) -> Self {
        let mut ed = Editor::new(t, "\n");
        ed.motion(Motion::DocEnd, false);
        Self { ed }
    }

    pub(super) fn text(&self) -> String {
        self.ed.text()
    }

    pub(super) fn set(&mut self, t: &str) {
        *self = Self::new(t);
    }

    fn char(&mut self, c: char) -> bool {
        if c.is_control() {
            return false;
        }
        let mut b = [0u8; 4];
        self.ed.type_text(c.encode_utf8(&mut b), 0);
        true
    }

    /// Editing keys; `true` when handled.
    fn key(&mut self, hwnd: HWND, vk: u16, ctrl: bool, shift: bool) -> bool {
        match vk {
            VK_LEFT_ => self
                .ed
                .motion(if ctrl { Motion::WordLeft } else { Motion::Left }, shift),
            VK_RIGHT_ => self.ed.motion(
                if ctrl {
                    Motion::WordRight
                } else {
                    Motion::Right
                },
                shift,
            ),
            VK_HOME_ => self.ed.motion(Motion::LineStart, shift),
            VK_END_ => self.ed.motion(Motion::LineEnd, shift),
            VK_BACK => self.ed.backspace(0),
            VK_DEL => self.ed.delete(0),
            0x41 if ctrl => self.ed.select_all(),
            0x43 if ctrl => {
                let t = self.ed.selected_text();
                if !t.is_empty() {
                    sys::copy_text(hwnd, &t);
                }
            }
            0x58 if ctrl => {
                let t = self.ed.cut();
                if !t.is_empty() {
                    sys::copy_text(hwnd, &t);
                }
            }
            0x56 if ctrl => {
                if let Some(t) = sys::paste_text(hwnd) {
                    let one: String = t.chars().filter(|c| !c.is_control()).collect();
                    self.ed.paste(&one);
                }
            }
            0x5A if ctrl => {
                self.ed.undo();
            }
            0x59 if ctrl => {
                self.ed.redo();
            }
            _ => return false,
        }
        true
    }
}

/// One entry of the content list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum It {
    /// Section heading.
    Head(Section),
    /// `ROWS[i]`.
    Row(usize),
    /// Colour token `TOKENS[i]` (light + dark swatches).
    Color(usize),
    /// Keymap action `ACTIONS[i]`.
    Key(usize),
}

impl It {
    fn height(self) -> f32 {
        match self {
            It::Head(_) => sp::HEAD_H,
            It::Row(_) => sp::ROW_H,
            It::Color(_) | It::Key(_) => SMALL_H,
        }
    }

    fn section(self) -> Section {
        match self {
            It::Head(s) => s,
            It::Row(i) => ROWS.get(i).map_or(Section::General, |r| r.section),
            It::Color(_) => Section::Appearance,
            It::Key(_) => Section::Shortcuts,
        }
    }
}

/// Keyboard focus inside the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Focus {
    Search,
    Nav,
    Item(usize),
    ResetSection,
    ResetAll,
    Close,
}

/// Colour editor popover (FUT-04): HSV square, hue slider, hex input.
pub(super) struct ColorPop {
    pub(super) token: usize,
    pub(super) dark: bool,
    h: f32,
    s: f32,
    v: f32,
    pub(super) hex: Field,
    /// 1 = SV square, 2 = hue slider.
    drag: u8,
    /// Anchor (swatch) rect.
    at: Rect,
}

impl ColorPop {
    fn rgb(&self) -> u32 {
        colors::hsv_to_rgb(self.h, self.s, self.v)
    }

    fn set_rgb(&mut self, c: u32) {
        let (h, s, v) = colors::rgb_to_hsv(c);
        // Keep the hue of greys while editing.
        if s > 0.0 {
            self.h = h;
        }
        self.s = s;
        self.v = v;
        self.hex.set(&colors::to_hex(c));
    }

    fn rects(&self, panel: Rect) -> [Rect; 7] {
        let w = 248.0;
        let h = 286.0;
        let x = (self.at.x + self.at.w - w).clamp(panel.x + 8.0, panel.right() - w - 8.0);
        let mut y = self.at.bottom() + 6.0;
        if y + h > panel.bottom() - 8.0 {
            y = (self.at.y - h - 6.0).max(panel.y + 8.0);
        }
        let pop = Rect::new(x, y, w, h);
        let sv = Rect::new(x + 16.0, y + 16.0, 216.0, 140.0);
        let hue = Rect::new(x + 16.0, sv.bottom() + 12.0, 216.0, 14.0);
        let hex = Rect::new(x + 16.0, hue.bottom() + 14.0, 120.0, 28.0);
        let prev = Rect::new(hex.right() + 8.0, hex.y, 28.0, 28.0);
        let reset = Rect::new(x + 16.0, hex.bottom() + 14.0, 100.0, 28.0);
        let done = Rect::new(pop.right() - 16.0 - 80.0, reset.y, 80.0, 28.0);
        [pop, sv, hue, hex, prev, reset, done]
    }
}

/// Font picker list (FUT-49).
pub(super) struct FontList {
    pub(super) row: usize,
    pub(super) fonts: Rc<Vec<(String, bool)>>,
    pub(super) query: Field,
    scroll: f32,
    pub(super) sel: usize,
    mono: bool,
    at: Rect,
}

impl FontList {
    pub(super) fn shown(&self) -> Vec<usize> {
        sp::font_filter(&self.fonts, &self.query.text(), self.mono)
    }

    fn rects(&self, panel: Rect) -> (Rect, Rect, Rect) {
        let w = 300.0;
        let h = 320.0_f32.min(panel.h - 32.0);
        let x = (self.at.right() - w).clamp(panel.x + 8.0, panel.right() - w - 8.0);
        let mut y = self.at.bottom() + 4.0;
        if y + h > panel.bottom() - 8.0 {
            y = (self.at.y - h - 4.0).max(panel.y + 8.0);
        }
        let list = Rect::new(x, y, w, h);
        let q = Rect::new(x + 8.0, y + 8.0, w - 16.0, 28.0);
        let rows = Rect::new(x, q.bottom() + 6.0, w, h - 50.0);
        (list, q, rows)
    }
}

/// The open panel.
pub(super) struct Panel {
    /// Selected section (index into `SECTIONS`).
    pub(super) sec: usize,
    scroll: f32,
    drag: (f32, f32),
    grab: Option<(f32, f32, (f32, f32))>,
    pub(super) search: Field,
    pub(super) focus: Focus,
    ring: bool,
    /// Text row being edited.
    pub(super) text: Option<(usize, Field)>,
    /// Shortcut being captured (`ACTIONS` index).
    pub(super) capture: Option<usize>,
    /// Conflict of the last captured chord: (action, other action, chord display).
    pub(super) conflict: Option<(usize, usize, String)>,
    pub(super) color: Option<ColorPop>,
    pub(super) font: Option<FontList>,
    /// Slider being dragged: (row, value).
    slider: Option<(usize, f64)>,
    /// Row whose dropdown popup is open.
    pub(super) dropdown: Option<usize>,
    km: Keymap,
    cols: Vec<(u32, u32, bool, bool)>,
}

/// Panel operations run outside the window borrow (`Action::Panel`).
#[derive(Debug, Clone)]
pub(super) enum Op {
    Set(&'static str, String),
    Reset(Vec<&'static str>),
    Shortcut(&'static str, Option<String>),
    ResetShortcuts,
    Color(usize, bool, Option<u32>),
    ResetColors,
    Clear(u8),
    Command(u32),
    EditTheme,
    /// Opens the dropdown of `ROWS[i]` below the screen point.
    Dropdown(usize, (i32, i32)),
}

/// Every searchable entry in display order (static rows with colours and shortcuts spliced in).
fn all_items() -> Vec<It> {
    let mut out = Vec::new();
    for (i, r) in ROWS.iter().enumerate() {
        if r.key == "btn.reset_colors" {
            out.extend((0..TOKENS.len()).map(It::Color));
        }
        out.push(It::Row(i));
        if r.key == "btn.reset_shortcuts" {
            out.extend((0..ACTIONS.len()).map(It::Key));
        }
    }
    out
}

fn search_item(it: It, km: &Keymap, lang: Lang) -> sp::Item {
    match it {
        It::Head(s) => sp::Item {
            section: s,
            label: String::new(),
            extra: String::new(),
        },
        It::Row(i) => {
            let Some(r) = ROWS.get(i) else {
                return search_item(It::Head(Section::General), km, lang);
            };
            let (l, d) = sp::row_text(r, lang);
            let opts: Vec<String> = dropdown_opts(r.key, lang)
                .into_iter()
                .map(|o| o.1)
                .collect();
            sp::Item {
                section: r.section,
                label: l.to_string(),
                extra: format!("{d} {} {}", r.key, opts.join(" ")),
            }
        }
        It::Color(i) => {
            let t = TOKENS.get(i).copied().unwrap_or("bg");
            sp::Item {
                section: Section::Appearance,
                label: sp::token_label(t, lang).to_string(),
                extra: format!("{t} color colour renk custom-theme"),
            }
        }
        It::Key(i) => {
            let Some(a) = ACTIONS.get(i) else {
                return search_item(It::Head(Section::Shortcuts), km, lang);
            };
            sp::Item {
                section: Section::Shortcuts,
                label: km::label(a, lang).to_string(),
                extra: format!("{} {} shortcut kısayol", a.id, km.shown(a.id)),
            }
        }
    }
}

/// Settings keys of a section (Reset section) or of every section (Reset all).
fn setting_keys(sec: Option<Section>) -> Vec<&'static str> {
    ROWS.iter()
        .filter(|r| sec.is_none_or(|s| r.section == s))
        .map(|r| r.key)
        .filter(|k| yamdv_core::settings::def(k).is_some())
        .collect()
}

/// Dropdown options (value, label) of a row.
pub(super) fn dropdown_opts(key: &str, lang: Lang) -> Vec<(String, String)> {
    if key == "appearance.scheme" {
        return app::SCHEMES
            .iter()
            .map(|(v, l)| ((*v).to_string(), (*l).to_string()))
            .collect();
    }
    sp::options(key)
        .iter()
        .map(|v| ((*v).to_string(), sp::option_label(key, v, lang)))
        .collect()
}

fn info_value(key: &str, lang: Lang) -> String {
    let portable = app::is_portable();
    match key {
        "info.portable" => if portable {
            pick(lang, "On", "Açık")
        } else {
            pick(lang, "Off", "Kapalı")
        }
        .to_string(),
        "info.offline" => pick(lang, "No network access", "Ağ erişimi yok").to_string(),
        "info.version" => env!("CARGO_PKG_VERSION").to_string(),
        "info.commit" => option_env!("YAMDV_COMMIT").unwrap_or("dev").to_string(),
        "info.license" => pick(lang, "See LICENSE", "LICENSE dosyasına bakın").to_string(),
        "info.config_path" => app::data_dir(),
        "info.exe_path" => crate::shell::exe_path(),
        _ => String::new(),
    }
}

fn button_w(label: &str) -> f32 {
    (label.chars().count() as f32 * 7.4 + 28.0)
        .max(88.0)
        .round()
}

impl Panel {
    fn new(sec: usize) -> Self {
        Self {
            sec,
            scroll: 0.0,
            drag: (0.0, 0.0),
            grab: None,
            search: Field::new(""),
            focus: Focus::Search,
            ring: false,
            text: None,
            capture: None,
            conflict: None,
            color: None,
            font: None,
            slider: None,
            dropdown: None,
            km: app::keymap(),
            cols: app::token_colors(),
        }
    }

    /// Re-reads the keymap and colours after a change.
    pub(super) fn refresh(&mut self) {
        self.km = app::keymap();
        self.cols = app::token_colors();
    }

    pub(super) fn section(&self) -> Section {
        SECTIONS.get(self.sec).copied().unwrap_or(Section::General)
    }

    /// The content list: the current section, or every match grouped by section.
    pub(super) fn items(&self, lang: Lang) -> Vec<It> {
        let q = self.search.text();
        let all = all_items();
        if q.trim().is_empty() {
            let s = self.section();
            let mut v = vec![It::Head(s)];
            v.extend(all.into_iter().filter(|it| it.section() == s));
            return v;
        }
        let mut out = Vec::new();
        for s in SECTIONS {
            let hits: Vec<It> = all
                .iter()
                .copied()
                .filter(|it| it.section() == s)
                .filter(|&it| sp::matches(&search_item(it, &self.km, lang), &q, lang))
                .collect();
            if !hits.is_empty() {
                out.push(It::Head(s));
                out.extend(hits);
            }
        }
        out
    }

    fn content_h(items: &[It]) -> f32 {
        items.iter().map(|i| i.height()).sum::<f32>() + sp::PAD
    }

    /// Item rectangles (full content width) at the current scroll.
    fn layout(&self, g: &sp::Geo, items: &[It]) -> Vec<Rect> {
        let c = rr(g.content);
        let mut y = c.y + 4.0 - self.scroll;
        items
            .iter()
            .map(|it| {
                let r = Rect::new(c.x + sp::PAD, y, c.w - 2.0 * sp::PAD, it.height());
                y += it.height();
                r
            })
            .collect()
    }

    fn clamp_scroll(&mut self, g: &sp::Geo, items: &[It]) {
        let max = (Self::content_h(items) - g.content.h).max(0.0);
        self.scroll = self.scroll.clamp(0.0, max);
    }

    fn ensure_visible(&mut self, g: &sp::Geo, items: &[It], i: usize) {
        let top: f32 = items.iter().take(i).map(|x| x.height()).sum::<f32>() + 4.0;
        let h = items.get(i).map_or(0.0, |x| x.height());
        if top - self.scroll < 0.0 {
            self.scroll = top - 4.0;
        } else if top + h - self.scroll > g.content.h - 8.0 {
            self.scroll = top + h - g.content.h + 8.0;
        }
        self.clamp_scroll(g, items);
    }

    /// Header buttons: (reset section, reset all).
    fn header_buttons(g: &sp::Geo, lang: Lang) -> (Rect, Rect) {
        let close = rr(g.close);
        let wa = button_w(tr(lang, Msg::PanelResetAll));
        let ws = button_w(tr(lang, Msg::PanelResetSection));
        let all = Rect::new(close.x - 8.0 - wa, g.header.y + 10.0, wa, 28.0);
        let sec = Rect::new(all.x - 8.0 - ws, all.y, ws, 28.0);
        (sec, all)
    }
}

/// Control rectangle of `it` inside its row rect `r`.
fn control_rect(it: It, r: Rect, lang: Lang) -> Rect {
    let cy = r.y + r.h / 2.0;
    let xr = r.right();
    match it {
        It::Head(_) => Rect::new(xr, r.y, 0.0, 0.0),
        It::Color(_) => Rect::new(xr - 116.0, cy - 12.0, 116.0, 24.0),
        It::Key(_) => Rect::new(xr - 200.0, cy - 14.0, 200.0, 28.0),
        It::Row(i) => {
            let Some(row) = ROWS.get(i) else {
                return Rect::new(xr, r.y, 0.0, 0.0);
            };
            match sp::control(row) {
                Control::Toggle => Rect::new(xr - 40.0, cy - 10.0, 40.0, 20.0),
                Control::Segmented => {
                    let n = sp::options(row.key).len().max(1) as f32;
                    Rect::new(xr - n * 88.0, cy - 14.0, n * 88.0, 28.0)
                }
                Control::Dropdown | Control::Font(_) => {
                    Rect::new(xr - 220.0, cy - 14.0, 220.0, 28.0)
                }
                Control::Text => Rect::new(xr - 260.0, cy - 14.0, 260.0, 28.0),
                Control::Slider { .. } => Rect::new(xr - 180.0, cy - 12.0, 180.0, 24.0),
                Control::Button => {
                    let w = button_w(sp::row_text(row, lang).0);
                    Rect::new(xr - w, cy - 14.0, w, 28.0)
                }
                Control::Info => Rect::new(r.x + r.w * 0.4, r.y, r.w * 0.6, r.h),
            }
        }
    }
}

fn slider_value(row: &sp::Row) -> f64 {
    app::get(row.key).parse::<f64>().unwrap_or(0.0)
}

/// Left/Right adjustment of a row's value.
fn adjusted(row: &sp::Row, up: bool, lang: Lang) -> Option<String> {
    let cur = app::get(row.key);
    match sp::control(row) {
        Control::Toggle => Some(up.to_string()),
        Control::Segmented | Control::Dropdown => {
            let opts = dropdown_opts(row.key, lang);
            let i = opts.iter().position(|o| o.0 == cur).unwrap_or(0);
            let j = if up {
                (i + 1).min(opts.len().saturating_sub(1))
            } else {
                i.saturating_sub(1)
            };
            opts.get(j).map(|o| o.0.clone()).filter(|v| *v != cur)
        }
        Control::Slider { min, max, step } => {
            let v = slider_value(row);
            let n = sp::snap(if up { v + step } else { v - step }, min, max, step);
            Some(sp::slider_text(n, step))
        }
        _ => None,
    }
}

fn luminance(c: u32) -> f32 {
    let r = ((c >> 16) & 0xff) as f32 / 255.0;
    let g = ((c >> 8) & 0xff) as f32 / 255.0;
    let bl = (c & 0xff) as f32 / 255.0;
    0.299 * r + 0.587 * g + 0.114 * bl
}

impl Win {
    fn panel_geo(&self) -> Option<sp::Geo> {
        let p = self.settings.as_ref()?;
        Some(sp::geo(self.cw, self.ch, p.drag))
    }

    /// Opens the panel at `sec` (or the last section). With `toggle` and no section, an open
    /// panel closes instead (⚙ / `Ctrl+,`).
    pub(super) fn panel_open(&mut self, sec: Option<Section>, toggle: bool) {
        if self.settings.is_some() && toggle && sec.is_none() {
            self.panel_close();
            return;
        }
        let i = sec
            .and_then(|s| SECTIONS.iter().position(|&x| x == s))
            .or_else(|| self.settings.as_ref().map(|p| p.sec))
            .unwrap_or(0);
        let mut p = Box::new(Panel::new(i));
        if let Some(old) = self.settings.take() {
            p.drag = old.drag;
        }
        self.settings = Some(p);
        self.invalidate();
    }

    pub(super) fn panel_close(&mut self) {
        if let Some(mut p) = self.settings.take() {
            let t = p.text.take();
            self.commit_text(t);
        }
        // Built on demand, fully freed on close: the family list goes too (open lists keep
        // their own `Rc`).
        FONTS.with(|f| f.borrow_mut().take());
        self.invalidate();
    }

    /// Commits an edited text row.
    fn commit_text(&mut self, t: Option<(usize, Field)>) {
        if let Some((i, f)) = t
            && let Some(row) = ROWS.get(i)
        {
            let v = f.text();
            if v != app::get(row.key) {
                self.actions.push(Action::Panel(Op::Set(row.key, v)));
            }
        }
    }

    /// Screen point below `r` (client DIPs) for a popup.
    fn screen_below(&self, r: Rect) -> (i32, i32) {
        let mut pt = POINT {
            x: (r.x * self.scale).round() as i32,
            y: ((r.bottom() + self.top) * self.scale).round() as i32,
        };
        // SAFETY: valid hwnd and point.
        let _ = unsafe { windows::Win32::Graphics::Gdi::ClientToScreen(self.hwnd, &mut pt) };
        (pt.x, pt.y)
    }

    fn panel_confirm(&mut self, sh: &Shared, what: &'static str) {
        let (t, b, btn) = match what {
            "reset_all" => (
                Msg::PanelResetAllTitle,
                Msg::PanelResetAllBody,
                Msg::PanelReset,
            ),
            "btn.reset_colors" => (
                Msg::PanelResetColorsTitle,
                Msg::PanelResetColorsBody,
                Msg::PanelReset,
            ),
            _ => (Msg::PanelClearTitle, Msg::PanelClearBody, Msg::PanelClear),
        };
        self.ask(
            sh,
            t,
            b,
            vec![(btn, BTN_PANEL_OK), (Msg::Cancel, BTN_CANCEL)],
            Pending::Panel(what),
        );
    }

    /// A confirmation of the panel was accepted.
    pub(super) fn panel_confirmed(&mut self, what: &'static str) {
        let op = match what {
            "reset_all" => Op::Reset(setting_keys(None)),
            "btn.reset_colors" => Op::ResetColors,
            "btn.clear_history" => Op::Clear(0),
            "btn.clear_favorites" => Op::Clear(1),
            _ => Op::Clear(2),
        };
        self.actions.push(Action::Panel(op));
    }

    /// Activates `it` (row rect `r`): at the click point `at`, or from the keyboard (`None`).
    fn activate(&mut self, sh: &Shared, it: It, r: Rect, at: Option<(f32, f32)>) {
        let lang = sh.lang;
        let c = control_rect(it, r, lang);
        match it {
            It::Head(_) => {}
            It::Color(i) => {
                let dark = match at {
                    Some((x, _)) => x >= c.x + 58.0,
                    None => self.dark,
                };
                self.open_color(i, dark, c);
            }
            It::Key(i) => {
                if let Some(p) = &mut self.settings {
                    p.capture = Some(i);
                    p.conflict = None;
                }
            }
            It::Row(i) => {
                let Some(row) = ROWS.get(i) else { return };
                self.activate_row(sh, i, row, c, at);
            }
        }
        self.invalidate();
    }

    fn activate_row(
        &mut self,
        sh: &Shared,
        i: usize,
        row: &'static sp::Row,
        c: Rect,
        at: Option<(f32, f32)>,
    ) {
        match sp::control(row) {
            Control::Toggle => {
                let on = app::get(row.key) == "true";
                self.actions
                    .push(Action::Panel(Op::Set(row.key, (!on).to_string())));
            }
            Control::Segmented => {
                let opts = sp::options(row.key);
                let cur = app::get(row.key);
                let j = match at {
                    Some((x, _)) => ((x - c.x) / 88.0).floor().max(0.0) as usize,
                    None => {
                        (opts.iter().position(|o| *o == cur).unwrap_or(0) + 1) % opts.len().max(1)
                    }
                };
                if let Some(v) = opts.get(j) {
                    self.actions
                        .push(Action::Panel(Op::Set(row.key, (*v).to_string())));
                }
            }
            Control::Dropdown => {
                if let Some(p) = &mut self.settings {
                    p.dropdown = Some(i);
                }
                let pt = self.screen_below(c);
                self.actions.push(Action::Panel(Op::Dropdown(i, pt)));
            }
            Control::Font(mono) => self.open_fonts(sh, i, mono, c),
            Control::Text => {
                if let Some(p) = &mut self.settings {
                    p.text = Some((i, Field::new(&app::get(row.key))));
                }
            }
            Control::Slider { min, max, step } => {
                if let Some((x, _)) = at {
                    let f = f64::from(((x - c.x) / c.w).clamp(0.0, 1.0));
                    let v = sp::snap(min + f * (max - min), min, max, step);
                    if let Some(p) = &mut self.settings {
                        p.slider = Some((i, v));
                    }
                }
            }
            Control::Button => self.panel_button(sh, row.key),
            Control::Info => {
                if matches!(row.key, "info.config_path" | "info.exe_path") {
                    sys::copy_text(self.hwnd, &info_value(row.key, sh.lang));
                    self.toast(Msg::Copied);
                }
            }
        }
    }

    fn panel_button(&mut self, sh: &Shared, key: &'static str) {
        let op = match key {
            "btn.reset_colors"
            | "btn.clear_history"
            | "btn.clear_favorites"
            | "btn.clear_positions" => {
                self.panel_confirm(sh, key);
                return;
            }
            "btn.reset_shortcuts" => Op::ResetShortcuts,
            "btn.default_app" => Op::Command(app::SET_DEFAULT),
            "btn.config_folder" => Op::Command(app::CONFIG_DIR),
            "btn.edit_settings" => Op::Command(app::EDIT_SETTINGS),
            "btn.install" => Op::Command(app::INSTALL),
            "btn.edit_custom_theme" => Op::EditTheme,
            _ => return,
        };
        self.actions.push(Action::Panel(op));
    }

    pub(super) fn open_color(&mut self, token: usize, dark: bool, at: Rect) {
        let Some(p) = &mut self.settings else { return };
        let c = p
            .cols
            .get(token)
            .map_or(0, |c| if dark { c.1 } else { c.0 });
        let mut pop = ColorPop {
            token,
            dark,
            h: 0.0,
            s: 0.0,
            v: 0.0,
            hex: Field::new(""),
            drag: 0,
            at,
        };
        pop.set_rgb(c);
        p.color = Some(pop);
    }

    fn open_fonts(&mut self, sh: &Shared, row: usize, mono: bool, at: Rect) {
        let fonts = FONTS.with(|f| {
            f.borrow_mut()
                .get_or_insert_with(|| Rc::new(sh.dw.font_families()))
                .clone()
        });
        let Some(p) = &mut self.settings else { return };
        let cur = ROWS.get(row).map(|r| app::get(r.key)).unwrap_or_default();
        let mut l = FontList {
            row,
            fonts,
            query: Field::new(""),
            scroll: 0.0,
            sel: 0,
            mono,
            at,
        };
        let shown = l.shown();
        if let Some(i) = shown.iter().position(|&k| {
            l.fonts
                .get(k)
                .is_some_and(|f| f.0.eq_ignore_ascii_case(&cur))
        }) {
            l.sel = i;
            l.scroll = (i as f32 * FONT_ROW_H - 100.0).max(0.0);
        }
        p.font = Some(l);
    }

    /// Applies the selected font of the open list.
    pub(super) fn choose_font(&mut self) {
        let Some(p) = &mut self.settings else { return };
        let Some(l) = p.font.take() else { return };
        let shown = l.shown();
        if let (Some(row), Some(name)) = (
            ROWS.get(l.row),
            shown
                .get(l.sel)
                .and_then(|&k| l.fonts.get(k))
                .map(|f| f.0.clone()),
        ) {
            self.actions.push(Action::Panel(Op::Set(row.key, name)));
        }
        self.invalidate();
    }

    pub(super) fn apply_color(&mut self, close: bool) {
        let Some(p) = &mut self.settings else { return };
        let Some(c) = &p.color else { return };
        let op = Op::Color(c.token, c.dark, Some(c.rgb()));
        if close {
            p.color = None;
        }
        self.actions.push(Action::Panel(op));
    }

    fn panel_reset_section(&mut self) {
        let Some(p) = &self.settings else { return };
        let s = p.section();
        self.actions
            .push(Action::Panel(Op::Reset(setting_keys(Some(s)))));
        if s == Section::Shortcuts {
            self.actions.push(Action::Panel(Op::ResetShortcuts));
        }
    }
}

// ---- mouse ----------------------------------------------------------------------------------

impl Win {
    /// Mouse press while the panel is open (always consumed).
    pub(super) fn panel_down(&mut self, sh: &Shared, x: f32, y: f32) {
        let Some(g) = self.panel_geo() else { return };
        let panel = rr(g.panel);
        if self.fonts_down(panel, x, y) || self.color_down(panel, x, y) {
            self.invalidate();
            return;
        }
        // A click elsewhere commits an edited text row and ends a capture.
        if let Some(p) = &mut self.settings {
            let t = p.text.take();
            p.capture = None;
            p.conflict = None;
            p.ring = false;
            self.commit_text(t);
        }
        if !panel.contains(x, y) || rr(g.close).contains(x, y) {
            self.panel_close();
            return;
        }
        let lang = sh.lang;
        let (rs, ra) = Panel::header_buttons(&g, lang);
        if rs.contains(x, y) {
            self.panel_reset_section();
            return;
        }
        if ra.contains(x, y) {
            self.panel_confirm(sh, "reset_all");
            return;
        }
        let Some(p) = &mut self.settings else { return };
        if rr(g.header).contains(x, y) {
            p.grab = Some((x, y, p.drag));
            return;
        }
        if rr(g.search).contains(x, y) {
            p.focus = Focus::Search;
            self.invalidate();
            return;
        }
        let nav = rr(g.nav);
        if nav.contains(x, y) {
            let i = ((y - nav.y - 4.0) / sp::NAV_ITEM_H).floor();
            if i >= 0.0 && (i as usize) < SECTIONS.len() {
                p.sec = i as usize;
                p.scroll = 0.0;
                p.search.set("");
                p.focus = Focus::Nav;
            }
            self.invalidate();
            return;
        }
        if rr(g.content).contains(x, y) {
            let items = p.items(lang);
            let rects = p.layout(&g, &items);
            if let Some(k) = rects.iter().position(|r| r.contains(x, y))
                && let (Some(&it), Some(&r)) = (items.get(k), rects.get(k))
            {
                p.focus = Focus::Item(k);
                if control_rect(it, r, lang).contains(x, y) {
                    self.activate(sh, it, r, Some((x, y)));
                }
            }
        }
        self.invalidate();
    }

    /// Press with the font list open: `true` when consumed.
    fn fonts_down(&mut self, panel: Rect, x: f32, y: f32) -> bool {
        let Some(l) = self.settings.as_mut().and_then(|p| p.font.as_mut()) else {
            return false;
        };
        let (list, _, rows) = l.rects(panel);
        if !list.contains(x, y) {
            if let Some(p) = &mut self.settings {
                p.font = None;
            }
            return true;
        }
        if rows.contains(x, y) {
            let i = ((y - rows.y + l.scroll) / FONT_ROW_H).floor() as usize;
            if i < l.shown().len() {
                l.sel = i;
                self.choose_font();
            }
        }
        true
    }

    /// Press with the colour popover open: `true` when consumed.
    fn color_down(&mut self, panel: Rect, x: f32, y: f32) -> bool {
        let Some(c) = self.settings.as_mut().and_then(|p| p.color.as_mut()) else {
            return false;
        };
        let [pop, sv, hue, _hex, _prev, reset, done] = c.rects(panel);
        let (token, dark) = (c.token, c.dark);
        if !pop.contains(x, y) {
            self.apply_color(true);
        } else if reset.contains(x, y) {
            if let Some(p) = &mut self.settings {
                p.color = None;
            }
            self.actions
                .push(Action::Panel(Op::Color(token, dark, None)));
        } else if done.contains(x, y) {
            self.apply_color(true);
        } else if sv.contains(x, y) || hue.contains(x, y) {
            c.drag = if sv.contains(x, y) { 1 } else { 2 };
            self.panel_move(x, y);
        }
        true
    }

    pub(super) fn panel_move(&mut self, x: f32, y: f32) {
        let Some(g) = self.panel_geo() else { return };
        let panel = rr(g.panel);
        let Some(p) = &mut self.settings else { return };
        if let Some((mx, my, d)) = p.grab {
            p.drag = (d.0 + x - mx, d.1 + y - my);
            self.invalidate();
            return;
        }
        if let Some(c) = &mut p.color
            && c.drag != 0
        {
            let [_, sv, hue, ..] = c.rects(panel);
            if c.drag == 1 {
                c.s = ((x - sv.x) / sv.w).clamp(0.0, 1.0);
                c.v = 1.0 - ((y - sv.y) / sv.h).clamp(0.0, 1.0);
            } else {
                c.h = ((x - hue.x) / hue.w).clamp(0.0, 1.0) * 360.0;
            }
            let rgb = c.rgb();
            c.hex.set(&colors::to_hex(rgb));
            self.invalidate();
            return;
        }
        if let Some((i, _)) = p.slider
            && let Some(row) = ROWS.get(i)
            && let Control::Slider { min, max, step } = sp::control(row)
        {
            let items = p.items(Lang::En);
            let rects = p.layout(&g, &items);
            if let Some(r) = items
                .iter()
                .position(|&it| it == It::Row(i))
                .and_then(|k| rects.get(k))
            {
                let c = control_rect(It::Row(i), *r, Lang::En);
                let f = f64::from(((x - c.x) / c.w).clamp(0.0, 1.0));
                p.slider = Some((i, sp::snap(min + f * (max - min), min, max, step)));
                self.invalidate();
            }
        }
    }

    pub(super) fn panel_up(&mut self) {
        let Some(p) = &mut self.settings else { return };
        p.grab = None;
        if let Some(c) = &mut p.color
            && c.drag != 0
        {
            c.drag = 0;
            self.apply_color(false);
        }
        let Some(p) = &mut self.settings else { return };
        if let Some((i, v)) = p.slider.take()
            && let Some(row) = ROWS.get(i)
            && let Control::Slider { step, .. } = sp::control(row)
        {
            self.actions
                .push(Action::Panel(Op::Set(row.key, sp::slider_text(v, step))));
        }
        self.invalidate();
    }

    pub(super) fn panel_wheel(&mut self, sh: &Shared, delta: i16) {
        let Some(g) = self.panel_geo() else { return };
        let Some(p) = &mut self.settings else { return };
        let d = f32::from(delta) / 120.0 * 56.0;
        if let Some(l) = &mut p.font {
            let n = l.shown().len() as f32;
            let (_, _, rows) = l.rects(rr(g.panel));
            l.scroll = (l.scroll - d).clamp(0.0, (n * FONT_ROW_H - rows.h).max(0.0));
        } else {
            p.scroll -= d;
            let items = p.items(sh.lang);
            p.clamp_scroll(&g, &items);
        }
        self.invalidate();
    }

    /// Text input while the panel is open.
    pub(super) fn panel_char(&mut self, c: char) {
        let Some(p) = &mut self.settings else { return };
        if p.capture.is_some() {
            return;
        }
        let changed = if let Some(l) = &mut p.font {
            let ch = l.query.char(c);
            if ch {
                l.sel = 0;
                l.scroll = 0.0;
            }
            ch
        } else if let Some(cp) = &mut p.color {
            cp.hex.char(c)
        } else if let Some((_, f)) = &mut p.text {
            f.char(c)
        } else if p.focus == Focus::Search {
            let ch = p.search.char(c);
            if ch {
                p.scroll = 0.0;
            }
            ch
        } else {
            false
        };
        if changed {
            self.invalidate();
        }
    }
}

// ---- keyboard -------------------------------------------------------------------------------

impl Win {
    /// Key press while the panel is open; `false` lets it through (`Alt+F4`).
    pub(super) fn panel_key(
        &mut self,
        sh: &Shared,
        vk: u16,
        ctrl: bool,
        shift: bool,
        alt: bool,
    ) -> bool {
        if alt && vk == 0x73 {
            return false;
        }
        let Some(g) = self.panel_geo() else {
            return false;
        };
        let Some(p) = &mut self.settings else {
            return false;
        };
        let done = if p.capture.is_some() {
            self.capture_key(vk, ctrl, shift, alt);
            true
        } else if p.font.is_some() {
            self.fonts_key(&g, vk, ctrl, shift);
            true
        } else if p.color.is_some() {
            self.color_key(vk, ctrl, shift);
            true
        } else if p.text.is_some() {
            self.text_key(vk, ctrl, shift);
            true
        } else {
            false
        };
        if !done {
            self.nav_key(sh, &g, vk, ctrl, shift);
        }
        self.invalidate();
        true
    }

    /// Shortcut capture (FUT-05): Esc cancels, Backspace clears, a conflict is shown inline
    /// and nothing is saved until it is resolved.
    fn capture_key(&mut self, vk: u16, ctrl: bool, shift: bool, alt: bool) {
        let Some(p) = &mut self.settings else { return };
        let Some(i) = p.capture else { return };
        if matches!(vk, 0x10 | 0x11 | 0x12 | 0x5B | 0x5C | 0xA0..=0xA5) {
            return;
        }
        let plain = !ctrl && !shift && !alt;
        if vk == VK_ESC && plain {
            p.capture = None;
            p.conflict = None;
            return;
        }
        if vk == VK_BACK && plain {
            p.capture = None;
            p.conflict = None;
            if let Some(a) = ACTIONS.get(i) {
                self.actions
                    .push(Action::Panel(Op::Shortcut(a.id, Some(String::new()))));
            }
            return;
        }
        let c = Chord {
            ctrl,
            shift,
            alt,
            vk,
        };
        if !c.bindable() {
            return;
        }
        let mut k = p.km.clone();
        match k.set(i, vec![c]) {
            Ok(text) => {
                p.capture = None;
                p.conflict = None;
                p.km = k;
                if let Some(a) = ACTIONS.get(i) {
                    self.actions
                        .push(Action::Panel(Op::Shortcut(a.id, Some(text))));
                }
            }
            Err(o) => p.conflict = Some((i, o, c.display())),
        }
    }

    fn fonts_key(&mut self, g: &sp::Geo, vk: u16, ctrl: bool, shift: bool) {
        let hwnd = self.hwnd;
        let Some(p) = &mut self.settings else { return };
        let Some(l) = &mut p.font else { return };
        let n = l.shown().len();
        let (_, _, rows) = l.rects(rr(g.panel));
        let page = (rows.h / FONT_ROW_H).floor().max(1.0) as usize;
        match vk {
            VK_ESC => {
                p.font = None;
                return;
            }
            VK_ENTER => {
                self.choose_font();
                return;
            }
            VK_UP_ => l.sel = l.sel.saturating_sub(1),
            VK_DOWN_ => l.sel = (l.sel + 1).min(n.saturating_sub(1)),
            VK_PGUP => l.sel = l.sel.saturating_sub(page),
            VK_PGDN => l.sel = (l.sel + page).min(n.saturating_sub(1)),
            _ => {
                if l.query.key(hwnd, vk, ctrl, shift) {
                    l.sel = 0;
                    l.scroll = 0.0;
                }
            }
        }
        let top = l.sel as f32 * FONT_ROW_H;
        if top < l.scroll {
            l.scroll = top;
        } else if top + FONT_ROW_H > l.scroll + rows.h {
            l.scroll = top + FONT_ROW_H - rows.h;
        }
    }

    fn color_key(&mut self, vk: u16, ctrl: bool, shift: bool) {
        let hwnd = self.hwnd;
        let Some(p) = &mut self.settings else { return };
        let Some(c) = &mut p.color else { return };
        match vk {
            VK_ESC => p.color = None,
            VK_ENTER => {
                if let Some(v) = colors::parse_hex(c.hex.text().trim()) {
                    c.set_rgb(v);
                    self.apply_color(true);
                }
            }
            _ => {
                c.hex.key(hwnd, vk, ctrl, shift);
                if let Some(v) = colors::parse_hex(c.hex.text().trim()) {
                    let (h, s, vv) = colors::rgb_to_hsv(v);
                    if s > 0.0 {
                        c.h = h;
                    }
                    c.s = s;
                    c.v = vv;
                }
            }
        }
    }

    fn text_key(&mut self, vk: u16, ctrl: bool, shift: bool) {
        let hwnd = self.hwnd;
        let Some(p) = &mut self.settings else { return };
        match vk {
            VK_ESC => p.text = None,
            VK_ENTER | VK_TAB_ => {
                let t = p.text.take();
                self.commit_text(t);
            }
            _ => {
                if let Some((_, f)) = &mut p.text {
                    f.key(hwnd, vk, ctrl, shift);
                }
            }
        }
    }

    /// Focus movement and activation (Tab ring, arrows, Space / Enter, Esc).
    #[allow(clippy::too_many_lines)]
    fn nav_key(&mut self, sh: &Shared, g: &sp::Geo, vk: u16, ctrl: bool, shift: bool) {
        let hwnd = self.hwnd;
        let lang = sh.lang;
        let Some(p) = &mut self.settings else { return };
        let items = p.items(lang);
        let focusable: Vec<usize> = (0..items.len())
            .filter(|&k| !matches!(items.get(k), Some(It::Head(_))))
            .collect();
        p.ring = true;
        if ctrl && vk == 0x46 {
            p.focus = Focus::Search;
            p.search.ed.select_all();
            return;
        }
        if ctrl && vk == 0xBC {
            self.panel_close();
            return;
        }
        if vk == VK_TAB_ {
            let order: Vec<Focus> = [Focus::Search, Focus::Nav]
                .into_iter()
                .chain(focusable.iter().map(|&k| Focus::Item(k)))
                .chain([Focus::ResetSection, Focus::ResetAll, Focus::Close])
                .collect();
            let cur = order.iter().position(|f| *f == p.focus).unwrap_or(0);
            let n = order.len();
            let next = if shift {
                (cur + n - 1) % n
            } else {
                (cur + 1) % n
            };
            p.focus = order.get(next).copied().unwrap_or(Focus::Search);
            if let Focus::Item(k) = p.focus {
                p.ensure_visible(g, &items, k);
            }
            return;
        }
        if vk == VK_ESC && !(p.focus == Focus::Search && !p.search.text().is_empty()) {
            self.panel_close();
            return;
        }
        match p.focus {
            Focus::Search => match vk {
                VK_ESC => {
                    p.search.set("");
                    p.scroll = 0.0;
                }
                VK_DOWN_ | VK_ENTER => {
                    if let Some(&k) = focusable.first() {
                        p.focus = Focus::Item(k);
                        p.ensure_visible(g, &items, k);
                    }
                }
                _ => {
                    if p.search.key(hwnd, vk, ctrl, shift) {
                        p.scroll = 0.0;
                    }
                }
            },
            Focus::Nav => match vk {
                VK_UP_ | VK_DOWN_ => {
                    p.sec = if vk == VK_UP_ {
                        p.sec.saturating_sub(1)
                    } else {
                        (p.sec + 1).min(SECTIONS.len() - 1)
                    };
                    p.scroll = 0.0;
                    p.search.set("");
                }
                VK_RIGHT_ | VK_ENTER => {
                    if let Some(&k) = focusable.first() {
                        p.focus = Focus::Item(k);
                    }
                }
                _ => {}
            },
            Focus::Item(k) => match vk {
                VK_UP_ | VK_DOWN_ => {
                    let pos = focusable.iter().position(|&f| f == k).unwrap_or(0);
                    let np = if vk == VK_UP_ {
                        pos.checked_sub(1)
                    } else {
                        Some(pos + 1).filter(|&x| x < focusable.len())
                    };
                    match np.and_then(|x| focusable.get(x)) {
                        Some(&nk) => {
                            p.focus = Focus::Item(nk);
                            p.ensure_visible(g, &items, nk);
                        }
                        None if vk == VK_UP_ => p.focus = Focus::Search,
                        None => {}
                    }
                }
                VK_SPACE_ | VK_ENTER => {
                    let rects = p.layout(g, &items);
                    if let (Some(&it), Some(&r)) = (items.get(k), rects.get(k)) {
                        self.activate(sh, it, r, None);
                    }
                }
                VK_LEFT_ | VK_RIGHT_ => {
                    if let Some(It::Row(i)) = items.get(k).copied()
                        && let Some(row) = ROWS.get(i)
                        && let Some(v) = adjusted(row, vk == VK_RIGHT_, lang)
                    {
                        self.actions.push(Action::Panel(Op::Set(row.key, v)));
                    }
                }
                VK_HOME_ | VK_END_ => {
                    let f = if vk == VK_HOME_ {
                        focusable.first()
                    } else {
                        focusable.last()
                    };
                    if let Some(&f) = f {
                        p.focus = Focus::Item(f);
                        p.ensure_visible(g, &items, f);
                    }
                }
                VK_PGUP | VK_PGDN => {
                    let d = if vk == VK_PGUP {
                        -g.content.h
                    } else {
                        g.content.h
                    };
                    p.scroll += d * 0.8;
                    p.clamp_scroll(g, &items);
                }
                _ => {}
            },
            Focus::ResetSection | Focus::ResetAll | Focus::Close => {
                if matches!(vk, VK_SPACE_ | VK_ENTER) {
                    match p.focus {
                        Focus::ResetSection => self.panel_reset_section(),
                        Focus::ResetAll => self.panel_confirm(sh, "reset_all"),
                        _ => self.panel_close(),
                    }
                }
            }
        }
    }
}

// ---- drawing --------------------------------------------------------------------------------

impl Win {
    fn draw_field(
        &self,
        sh: &Shared,
        b: &Band<'_>,
        f: &Field,
        r: Rect,
        focused: bool,
        placeholder: &str,
    ) {
        let pal = &self.pal;
        let tok = yamdv_core::chrome::tokens(self.dark);
        b.fill(r, pal.bg, 1.0, 4.0);
        b.stroke(
            r,
            if focused {
                pal.accent
            } else {
                tok.chrome_border
            },
            4.0,
            if focused { 2.0 } else { 1.0 },
        );
        let text = f.text();
        b.push_clip(Rect::new(r.x + 4.0, r.y + 1.0, r.w - 8.0, r.h - 2.0));
        let tx = r.x + 8.0;
        let w_at = |n: usize| {
            text.get(..n).map_or(0.0, |t| {
                self.shape_ui(sh, t, 13.0, 400, pal.text, None).width()
            })
        };
        if text.is_empty() {
            if !placeholder.is_empty() {
                let s = self.shape_ui(sh, placeholder, 13.0, 400, pal.text_muted, None);
                b.text(s.as_ref(), tx, (r.y + (r.h - s.height()) / 2.0).round());
            }
        } else {
            let sel = f.ed.selection();
            if focused && !sel.is_empty() {
                let (a, z) = (w_at(sel.start), w_at(sel.end));
                b.fill(
                    Rect::new(tx + a, r.y + 5.0, z - a, r.h - 10.0),
                    pal.selection,
                    0.45,
                    0.0,
                );
            }
            let s = self.shape_ui(sh, &text, 13.0, 400, pal.text, None);
            b.text(s.as_ref(), tx, (r.y + (r.h - s.height()) / 2.0).round());
        }
        if focused {
            let cx = w_at(f.ed.caret());
            b.fill(
                Rect::new(tx + cx, r.y + 6.0, 1.0, r.h - 12.0),
                pal.text,
                1.0,
                0.0,
            );
        }
        b.pop_clip();
    }

    fn draw_button(&self, sh: &Shared, b: &Band<'_>, r: Rect, label: &str, focused: bool) {
        let pal = &self.pal;
        let tok = yamdv_core::chrome::tokens(self.dark);
        b.fill(r, tok.menu_bg, 1.0, 6.0);
        b.stroke(r, tok.chrome_border, 6.0, 1.0);
        if focused {
            self.ring(b, r);
        }
        let l = self.shape_ui(sh, label, 13.0, 400, pal.text, None);
        b.text(
            l.as_ref(),
            (r.x + (r.w - l.width()) / 2.0).round(),
            (r.y + (r.h - l.height()) / 2.0).round(),
        );
    }

    fn ring(&self, b: &Band<'_>, r: Rect) {
        b.stroke(
            Rect::new(r.x - 2.0, r.y - 2.0, r.w + 4.0, r.h + 4.0),
            self.pal.accent,
            7.0,
            2.0,
        );
    }

    /// Draws the panel (before the confirmation dialog).
    pub(super) fn draw_settings(&self, sh: &Shared, b: &Band<'_>) {
        let Some(p) = &self.settings else { return };
        let g = sp::geo(self.cw, self.ch, p.drag);
        let lang = sh.lang;
        let pal = &self.pal;
        let tok = yamdv_core::chrome::tokens(self.dark);
        let panel = rr(g.panel);
        // Scrim and card.
        b.fill(Rect::new(0.0, 0.0, self.cw, self.ch), 0x000000, 0.32, 0.0);
        b.fill(
            Rect::new(panel.x, panel.y + 4.0, panel.w, panel.h),
            0x000000,
            0.18,
            10.0,
        );
        b.fill(panel, tok.menu_bg, 1.0, 8.0);
        b.stroke(panel, tok.menu_border, 8.0, 1.0);
        self.draw_panel_header(sh, b, p, &g);
        // Search + navigation.
        let search = Rect::new(g.search.x, g.search.y + 6.0, g.search.w, g.search.h - 4.0);
        let searching = !p.search.text().trim().is_empty();
        let typing = p.text.is_none() && p.font.is_none() && p.color.is_none();
        self.draw_field(
            sh,
            b,
            &p.search,
            search,
            p.focus == Focus::Search && typing,
            tr(lang, Msg::PanelSearch),
        );
        let nav = rr(g.nav);
        b.fill(
            Rect::new(
                nav.right(),
                g.header.y + g.header.h,
                1.0,
                panel.bottom() - (g.header.y + g.header.h),
            ),
            tok.separator,
            1.0,
            0.0,
        );
        for (i, s) in SECTIONS.iter().enumerate() {
            let r = Rect::new(
                nav.x + 8.0,
                nav.y + 4.0 + i as f32 * sp::NAV_ITEM_H,
                nav.w - 16.0,
                sp::NAV_ITEM_H - 2.0,
            );
            if r.bottom() > panel.bottom() {
                break;
            }
            let sel = i == p.sec && !searching;
            if sel {
                b.fill(r, tok.menu_hover, 1.0, 6.0);
                b.fill(
                    Rect::new(r.x, r.y + 8.0, 3.0, r.h - 16.0),
                    pal.accent,
                    1.0,
                    1.5,
                );
            }
            if p.ring && p.focus == Focus::Nav && i == p.sec {
                self.ring(b, r);
            }
            let w = if sel { 600 } else { 400 };
            let l = self.shape_ui(sh, sp::section_title(*s, lang), 13.0, w, pal.text, None);
            b.text(
                l.as_ref(),
                r.x + 12.0,
                (r.y + (r.h - l.height()) / 2.0).round(),
            );
        }
        // Content.
        let content = rr(g.content);
        let items = p.items(lang);
        let rects = p.layout(&g, &items);
        b.push_clip(Rect::new(
            content.x,
            content.y + 1.0,
            content.w,
            content.h - 2.0,
        ));
        if items.is_empty() {
            let l = self.shape_ui(
                sh,
                tr(lang, Msg::PanelNoMatch),
                13.0,
                400,
                pal.text_muted,
                None,
            );
            b.text(l.as_ref(), content.x + sp::PAD, content.y + 24.0);
        }
        for (k, (&it, &r)) in items.iter().zip(rects.iter()).enumerate() {
            if r.bottom() < content.y || r.y > content.bottom() {
                continue;
            }
            let focused = p.ring && p.focus == Focus::Item(k);
            self.draw_item(sh, b, p, it, r, focused);
        }
        b.pop_clip();
        let total = Panel::content_h(&items);
        if total > content.h {
            let th = (content.h * content.h / total).max(24.0);
            let ty = content.y + (content.h - th) * (p.scroll / (total - content.h));
            b.fill(
                Rect::new(content.right() - 6.0, ty, 3.0, th),
                pal.text_muted,
                0.5,
                1.5,
            );
        }
        if let Some(c) = &p.color {
            self.draw_color(sh, b, c, panel);
        }
        if let Some(l) = &p.font {
            self.draw_fonts(sh, b, l, panel);
        }
    }

    fn draw_panel_header(&self, sh: &Shared, b: &Band<'_>, p: &Panel, g: &sp::Geo) {
        let lang = sh.lang;
        let pal = &self.pal;
        let tok = yamdv_core::chrome::tokens(self.dark);
        let t = self.shape_ui(sh, tr(lang, Msg::MSettings), 16.0, 600, pal.text, None);
        b.text(
            t.as_ref(),
            g.panel.x + 20.0,
            (g.header.y + (g.header.h - t.height()) / 2.0).round(),
        );
        let (rs, ra) = Panel::header_buttons(g, lang);
        let ring = p.ring;
        self.draw_button(
            sh,
            b,
            rs,
            tr(lang, Msg::PanelResetSection),
            ring && p.focus == Focus::ResetSection,
        );
        self.draw_button(
            sh,
            b,
            ra,
            tr(lang, Msg::PanelResetAll),
            ring && p.focus == Focus::ResetAll,
        );
        let close = rr(g.close);
        let x = self.shape_ui(sh, "\u{2715}", 14.0, 400, pal.text, None);
        b.text(
            x.as_ref(),
            (close.x + (close.w - x.width()) / 2.0).round(),
            (close.y + (close.h - x.height()) / 2.0).round(),
        );
        if ring && p.focus == Focus::Close {
            self.ring(b, close);
        }
        b.fill(
            Rect::new(g.panel.x, (g.header.y + g.header.h) - 1.0, g.panel.w, 1.0),
            tok.separator,
            1.0,
            0.0,
        );
    }

    fn draw_label(&self, sh: &Shared, b: &Band<'_>, r: Rect, w: f32, text: &str, desc: &str) {
        let pal = &self.pal;
        let l = self.shape_ui(sh, text, 13.5, 500, pal.text, None);
        b.push_clip(Rect::new(r.x, r.y, w, r.h));
        if desc.is_empty() {
            b.text(l.as_ref(), r.x, (r.y + (r.h - l.height()) / 2.0).round());
        } else {
            b.text(l.as_ref(), r.x, r.y + 9.0);
            let d = self.shape_ui(sh, desc, 12.0, 400, pal.text_muted, None);
            b.text(d.as_ref(), r.x, r.y + 30.0);
        }
        b.pop_clip();
    }
}

impl Win {
    fn draw_item(&self, sh: &Shared, b: &Band<'_>, p: &Panel, it: It, r: Rect, focused: bool) {
        let lang = sh.lang;
        let pal = &self.pal;
        let c = control_rect(it, r, lang);
        let label_w = (c.x - r.x - 16.0).max(80.0);
        if focused {
            self.ring(b, Rect::new(r.x - 8.0, r.y + 2.0, r.w + 16.0, r.h - 4.0));
        }
        match it {
            It::Head(s) => {
                let l = self.shape_ui(sh, sp::section_title(s, lang), 14.0, 700, pal.heading, None);
                b.text(l.as_ref(), r.x, r.y + 8.0);
            }
            It::Color(i) => {
                let t = TOKENS.get(i).copied().unwrap_or("bg");
                self.draw_label(sh, b, r, label_w, sp::token_label(t, lang), "");
                self.draw_swatches(sh, b, p, i, c);
            }
            It::Key(i) => {
                let Some(a) = ACTIONS.get(i) else { return };
                self.draw_label(sh, b, r, label_w, km::label(a, lang), "");
                self.draw_key(sh, b, p, i, c, r);
            }
            It::Row(i) => {
                let Some(row) = ROWS.get(i) else { return };
                let (l, d) = sp::row_text(row, lang);
                if matches!(sp::control(row), Control::Font(_)) {
                    // The resolved face under the control (R-THEME-04).
                    let (bo, co) = sh.dw.resolved();
                    let res = if row.key.ends_with("code_font") {
                        co
                    } else {
                        bo
                    };
                    let d = format!("{} {res}", pick(lang, "Resolved:", "Kullanılan:"));
                    self.draw_label(sh, b, r, label_w, l, &d);
                } else {
                    self.draw_label(sh, b, r, label_w, l, d);
                }
                self.draw_control(sh, b, p, i, row, c);
            }
        }
    }

    fn draw_swatches(&self, sh: &Shared, b: &Band<'_>, p: &Panel, i: usize, c: Rect) {
        let pal = &self.pal;
        let tok = yamdv_core::chrome::tokens(self.dark);
        let (l, d, lo, dov) = p.cols.get(i).copied().unwrap_or((0, 0, false, false));
        for (k, (col, over, name)) in [(l, lo, Msg::PanelLight), (d, dov, Msg::PanelDark)]
            .into_iter()
            .enumerate()
        {
            let sw = Rect::new(c.x + k as f32 * 60.0, c.y, 56.0, c.h);
            b.fill(sw, col, 1.0, 4.0);
            b.stroke(
                sw,
                if over { pal.accent } else { tok.chrome_border },
                4.0,
                if over { 2.0 } else { 1.0 },
            );
            let fg = if luminance(col) > 0.5 {
                0x20_2020
            } else {
                0xf4_f4f4
            };
            let n = self.shape_ui(sh, tr(sh.lang, name), 10.5, 500, fg, None);
            b.text(
                n.as_ref(),
                (sw.x + (sw.w - n.width()) / 2.0).round(),
                (sw.y + (sw.h - n.height()) / 2.0).round(),
            );
        }
    }

    fn draw_key(&self, sh: &Shared, b: &Band<'_>, p: &Panel, i: usize, c: Rect, r: Rect) {
        let lang = sh.lang;
        let pal = &self.pal;
        let tok = yamdv_core::chrome::tokens(self.dark);
        let Some(a) = ACTIONS.get(i) else { return };
        let conflict = p.conflict.as_ref().filter(|c| c.0 == i);
        let capturing = p.capture == Some(i);
        b.fill(c, pal.bg, 1.0, 4.0);
        b.stroke(
            c,
            if capturing {
                pal.accent
            } else {
                tok.chrome_border
            },
            4.0,
            if capturing { 2.0 } else { 1.0 },
        );
        let text = if capturing {
            conflict.map_or_else(
                || tr(lang, Msg::PanelPressKeys).to_string(),
                |c| c.2.clone(),
            )
        } else {
            let s = p.km.shown(a.id);
            if s.is_empty() {
                tr(lang, Msg::PanelUnbound).to_string()
            } else {
                s
            }
        };
        let col = if capturing && conflict.is_none() {
            pal.text_muted
        } else {
            pal.text
        };
        let wgt = if p.km.overridden(i) { 600 } else { 400 };
        let s = self.shape_ui(sh, &text, 12.5, wgt, col, None);
        b.push_clip(c);
        b.text(
            s.as_ref(),
            c.x + 10.0,
            (c.y + (c.h - s.height()) / 2.0).round(),
        );
        b.pop_clip();
        if let Some((_, o, _)) = conflict {
            let other = ACTIONS.get(*o).map_or("", |a| km::label(a, lang));
            let m = format!("{}{other}", tr(lang, Msg::PanelConflict));
            let e = self.shape_ui(sh, &m, 11.5, 600, pal.error, None);
            b.text(
                e.as_ref(),
                (c.x - e.width() - 10.0).max(r.x),
                (c.y + (c.h - e.height()) / 2.0).round(),
            );
        } else if capturing {
            let h = self.shape_ui(
                sh,
                tr(lang, Msg::PanelCaptureHint),
                11.0,
                400,
                pal.text_muted,
                None,
            );
            b.text(
                h.as_ref(),
                (c.x - h.width() - 10.0).max(r.x),
                (c.y + (c.h - h.height()) / 2.0).round(),
            );
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_control(&self, sh: &Shared, b: &Band<'_>, p: &Panel, i: usize, row: &sp::Row, c: Rect) {
        let lang = sh.lang;
        let pal = &self.pal;
        let tok = yamdv_core::chrome::tokens(self.dark);
        let ctl = sp::control(row);
        let cur = app::get(row.key);
        let mid = |h: f32| (c.y + (c.h - h) / 2.0).round();
        match ctl {
            Control::Button => self.draw_button(sh, b, c, sp::row_text(row, lang).0, false),
            Control::Toggle => {
                let on = cur == "true";
                b.fill(
                    c,
                    if on { pal.accent } else { tok.chrome_border },
                    1.0,
                    c.h / 2.0,
                );
                let kx = if on {
                    c.right() - c.h / 2.0
                } else {
                    c.x + c.h / 2.0
                };
                b.circle(kx, c.y + c.h / 2.0, c.h / 2.0 - 3.0, 0xff_ffff, None);
            }
            Control::Segmented => {
                b.stroke(c, tok.chrome_border, 6.0, 1.0);
                for (k, v) in sp::options(row.key).iter().enumerate() {
                    let s = Rect::new(c.x + k as f32 * 88.0, c.y, 88.0, c.h);
                    let sel = *v == cur;
                    if sel {
                        b.fill(
                            Rect::new(s.x + 2.0, s.y + 2.0, s.w - 4.0, s.h - 4.0),
                            pal.accent,
                            1.0,
                            5.0,
                        );
                    }
                    let fg = if sel { 0xff_ffff } else { pal.text };
                    let w = if sel { 600 } else { 400 };
                    let t =
                        self.shape_ui(sh, &sp::option_label(row.key, v, lang), 12.0, w, fg, None);
                    b.push_clip(s);
                    b.text(
                        t.as_ref(),
                        (s.x + (s.w - t.width()) / 2.0).round().max(s.x + 4.0),
                        mid(t.height()),
                    );
                    b.pop_clip();
                }
            }
            Control::Dropdown | Control::Font(_) => {
                b.fill(c, pal.bg, 1.0, 4.0);
                b.stroke(c, tok.chrome_border, 4.0, 1.0);
                let font = matches!(ctl, Control::Font(_));
                let installed = font && sh.dw.has_family(&cur);
                let shown = if font {
                    if installed || cur.is_empty() {
                        cur.clone()
                    } else {
                        let (bo, co) = sh.dw.resolved();
                        let res = if row.key.ends_with("code_font") {
                            co
                        } else {
                            bo
                        };
                        format!("{cur} — {}{res}", tr(lang, Msg::PanelFontMissing))
                    }
                } else {
                    dropdown_opts(row.key, lang)
                        .into_iter()
                        .find(|o| o.0 == cur)
                        .map_or(cur.clone(), |o| o.1)
                };
                b.push_clip(Rect::new(c.x, c.y, c.w - 24.0, c.h));
                let own = installed
                    .then(|| sh.dw.shape_family(&cur, &shown, 13.0, pal.text))
                    .flatten();
                match own {
                    Some(s) => b.text(
                        &s,
                        c.x + 10.0,
                        mid(yamdv_layout::shaper::Shaped::height(&s)),
                    ),
                    None => {
                        let t = self.shape_ui(sh, &shown, 13.0, 400, pal.text, None);
                        b.text(t.as_ref(), c.x + 10.0, mid(t.height()));
                    }
                }
                b.pop_clip();
                let cx = c.right() - 14.0;
                let cy = c.y + c.h / 2.0;
                b.polyline(
                    &[(cx - 4.0, cy - 2.0), (cx, cy + 2.0), (cx + 4.0, cy - 2.0)],
                    pal.text_muted,
                    1.5,
                    false,
                );
            }
            Control::Text => match p.text.as_ref().filter(|t| t.0 == i) {
                Some((_, f)) => self.draw_field(sh, b, f, c, true, ""),
                None => self.draw_field(sh, b, &Field::new(&cur), c, false, ""),
            },
            Control::Slider { min, max, step } => {
                let v = p
                    .slider
                    .filter(|s| s.0 == i)
                    .map_or_else(|| slider_value(row), |s| s.1);
                let f = if max > min {
                    ((v - min) / (max - min)).clamp(0.0, 1.0) as f32
                } else {
                    0.0
                };
                let track = Rect::new(c.x, c.y + c.h / 2.0 - 2.0, c.w, 4.0);
                b.fill(track, tok.chrome_border, 1.0, 2.0);
                b.fill(
                    Rect::new(track.x, track.y, track.w * f, 4.0),
                    pal.accent,
                    1.0,
                    2.0,
                );
                b.circle(c.x + c.w * f, c.y + c.h / 2.0, 7.0, pal.accent, None);
                b.circle(c.x + c.w * f, c.y + c.h / 2.0, 4.0, 0xff_ffff, None);
                let t = self.shape_ui(sh, &sp::slider_text(v, step), 12.5, 500, pal.text, None);
                b.text(
                    t.as_ref(),
                    (c.x - t.width() - 14.0).round(),
                    mid(t.height()),
                );
            }
            Control::Info => {
                let v = info_value(row.key, lang);
                let t = self.shape_ui(sh, &v, 12.5, 400, pal.text, None);
                b.push_clip(c);
                b.text(
                    t.as_ref(),
                    (c.right() - t.width()).max(c.x),
                    mid(t.height()),
                );
                b.pop_clip();
            }
        }
    }

    fn draw_color(&self, sh: &Shared, b: &Band<'_>, c: &ColorPop, panel: Rect) {
        let tok = yamdv_core::chrome::tokens(self.dark);
        let [pop, sv, hue, hex, prev, reset, done] = c.rects(panel);
        b.fill(
            Rect::new(pop.x, pop.y + 3.0, pop.w, pop.h),
            0x000000,
            0.2,
            9.0,
        );
        b.fill(pop, tok.menu_bg, 1.0, 8.0);
        b.stroke(pop, tok.menu_border, 8.0, 1.0);
        // Saturation × value square as a 24×14 grid of cells.
        let (nx, ny) = (24_u16, 14_u16);
        let (cw, chh) = (sv.w / f32::from(nx), sv.h / f32::from(ny));
        for j in 0..ny {
            for i in 0..nx {
                let s = (f32::from(i) + 0.5) / f32::from(nx);
                let v = 1.0 - (f32::from(j) + 0.5) / f32::from(ny);
                b.fill(
                    Rect::new(
                        sv.x + f32::from(i) * cw,
                        sv.y + f32::from(j) * chh,
                        cw + 0.5,
                        chh + 0.5,
                    ),
                    colors::hsv_to_rgb(c.h, s, v),
                    1.0,
                    0.0,
                );
            }
        }
        b.stroke(sv, tok.chrome_border, 0.0, 1.0);
        b.circle(
            sv.x + c.s * sv.w,
            sv.y + (1.0 - c.v) * sv.h,
            6.0,
            0xff_ffff,
            Some(2.0),
        );
        let n = 36_u16;
        let hw = hue.w / f32::from(n);
        for i in 0..n {
            b.fill(
                Rect::new(hue.x + f32::from(i) * hw, hue.y, hw + 0.5, hue.h),
                colors::hsv_to_rgb(f32::from(i) * 360.0 / f32::from(n), 1.0, 1.0),
                1.0,
                0.0,
            );
        }
        let hx = hue.x + c.h / 360.0 * hue.w;
        b.stroke(
            Rect::new(hx - 3.0, hue.y - 2.0, 6.0, hue.h + 4.0),
            0xff_ffff,
            2.0,
            2.0,
        );
        self.draw_field(sh, b, &c.hex, hex, true, "#rrggbb");
        b.fill(prev, c.rgb(), 1.0, 4.0);
        b.stroke(prev, tok.chrome_border, 4.0, 1.0);
        self.draw_button(sh, b, reset, tr(sh.lang, Msg::PanelReset), false);
        self.draw_button(sh, b, done, tr(sh.lang, Msg::PanelDone), false);
    }

    fn draw_fonts(&self, sh: &Shared, b: &Band<'_>, l: &FontList, panel: Rect) {
        let pal = &self.pal;
        let tok = yamdv_core::chrome::tokens(self.dark);
        let (list, q, rows) = l.rects(panel);
        b.fill(
            Rect::new(list.x, list.y + 3.0, list.w, list.h),
            0x000000,
            0.2,
            9.0,
        );
        b.fill(list, tok.menu_bg, 1.0, 8.0);
        b.stroke(list, tok.menu_border, 8.0, 1.0);
        self.draw_field(sh, b, &l.query, q, true, tr(sh.lang, Msg::PanelSearchFonts));
        let shown = l.shown();
        b.push_clip(rows);
        // Virtualized: only the visible rows are shaped (each in its own face).
        for k in sp::visible_rows(l.scroll, rows.h, FONT_ROW_H, shown.len()) {
            let Some((name, mono)) = shown.get(k).and_then(|&i| l.fonts.get(i)) else {
                continue;
            };
            let r = Rect::new(
                rows.x + 4.0,
                rows.y + k as f32 * FONT_ROW_H - l.scroll,
                rows.w - 8.0,
                FONT_ROW_H,
            );
            if k == l.sel {
                b.fill(r, tok.menu_hover, 1.0, 4.0);
            }
            let my = |h: f32| (r.y + (r.h - h) / 2.0).round();
            match sh.dw.shape_family(name, name, 14.0, pal.text) {
                Some(s) => b.text(&s, r.x + 8.0, my(yamdv_layout::shaper::Shaped::height(&s))),
                None => {
                    let t = self.shape_ui(sh, name, 13.0, 400, pal.text, None);
                    b.text(t.as_ref(), r.x + 8.0, my(t.height()));
                }
            }
            if *mono {
                let m = self.shape_ui(sh, "mono", 10.5, 500, pal.text_muted, None);
                b.text(m.as_ref(), r.right() - m.width() - 8.0, my(m.height()));
            }
        }
        b.pop_clip();
    }
}

// ---- outside the borrow -----------------------------------------------------------------------

/// Runs a panel operation outside the window borrow.
pub(super) fn run_op(hwnd: HWND, op: Op) -> Vec<Action> {
    let mut out = Vec::new();
    match op {
        Op::Set(k, v) => {
            app::set(k, &v);
        }
        Op::Reset(keys) => {
            app::reset_keys(&keys);
        }
        Op::Shortcut(id, v) => {
            app::set_shortcut(id, v.as_deref());
        }
        Op::ResetShortcuts => {
            app::reset_shortcuts();
        }
        Op::Color(token, dark, v) => {
            let t = TOKENS.get(token).copied().unwrap_or("bg");
            // Editing a colour turns custom colours on (otherwise nothing visible happens).
            if v.is_some() && app::get("appearance.use_custom_colors") != "true" {
                app::set("appearance.use_custom_colors", "true");
            }
            app::edit_custom_theme(|ini| colors::set_token(ini, dark, t, v));
        }
        Op::ResetColors => {
            app::edit_custom_theme(colors::reset_all);
        }
        Op::Clear(n) => {
            app::clear_state(n);
            app::with_win(hwnd, |w, _| w.toast(Msg::PanelCleared));
        }
        Op::Command(id) => out.extend(app::run_command(hwnd, id, None)),
        Op::EditTheme => {
            let p = app::custom_theme_path();
            if !std::path::Path::new(&p).exists() {
                app::edit_custom_theme(|_| {});
            }
            let s = app::settings_snapshot();
            let ok = crate::shell::open_in_editor(
                &p,
                1,
                s.str("external_editor.custom_exe"),
                s.str("external_editor.custom_args"),
                s.str("external_editor.mode") == "custom",
            );
            if !ok {
                app::with_win(hwnd, |w, _| w.toast(Msg::ToastNoEditor));
            }
        }
        Op::Dropdown(i, pt) => {
            let Some(row) = ROWS.get(i) else { return out };
            let Some(sh) = shared() else { return out };
            let cur = app::get(row.key);
            let opts = dropdown_opts(row.key, sh.lang);
            let model = app::options_menu(sh.lang, &opts, &cur);
            popup::open_menu(hwnd, pt, false, model, None);
        }
    }
    app::with_win(hwnd, |w, _| {
        if let Some(p) = &mut w.settings {
            p.refresh();
        }
        w.invalidate();
    });
    out
}

/// A dropdown option was chosen (`SET_OPT0 + j`).
pub(super) fn option_chosen(hwnd: HWND, j: u32) {
    let mut row = None;
    app::with_win(hwnd, |w, _| {
        row = w.settings.as_mut().and_then(|p| p.dropdown.take());
    });
    let (Some(i), Some(sh)) = (row, shared()) else {
        return;
    };
    let Some(row) = ROWS.get(i) else { return };
    if let Some((v, _)) = dropdown_opts(row.key, sh.lang).get(j as usize) {
        run_op(hwnd, Op::Set(row.key, v.clone()));
    }
}

// ---- driver (dev-tools scripts) ---------------------------------------------------------------

#[cfg(feature = "dev-tools")]
/// Script slug of a section (`settings-open typography`).
pub(super) fn section_slug(s: Section) -> String {
    format!("{s:?}").to_ascii_lowercase()
}

#[cfg(feature = "dev-tools")]
fn slug_section(a: &str) -> Option<Section> {
    let a = a.replace(['-', '_'], "");
    SECTIONS.into_iter().find(|&s| section_slug(s) == a)
}

#[cfg(feature = "dev-tools")]
/// The item a script names: a row key (`appearance.mode`, `btn.reset_all`), `key:<action>`,
/// or `color:<token>`.
fn item_of(name: &str) -> Option<It> {
    if let Some(id) = name.strip_prefix("key:") {
        return km::index(id).map(It::Key);
    }
    if let Some(t) = name.strip_prefix("color:") {
        return TOKENS.iter().position(|x| *x == t).map(It::Color);
    }
    ROWS.iter().position(|r| r.key == name).map(It::Row)
}

#[cfg(feature = "dev-tools")]
impl Win {
    /// `ui settings-*` verbs; `None` = not a settings verb.
    pub(super) fn panel_drive(&mut self, sh: &Shared, verb: &str, arg: &str) -> Option<String> {
        let r = match verb {
            "settings-open" => {
                let sec = slug_section(arg);
                if !arg.is_empty() && sec.is_none() {
                    return Some(format!("FAIL ui settings-open {arg}: unknown section"));
                }
                self.panel_open(sec, false);
                format!("ok ui settings-open {arg}")
            }
            "settings-close" => {
                self.panel_close();
                "ok ui settings-close".into()
            }
            "settings-search" => {
                let Some(p) = &mut self.settings else {
                    return Some("FAIL ui settings-search: panel closed".into());
                };
                p.search.set(arg);
                p.scroll = 0.0;
                p.focus = Focus::Search;
                self.invalidate();
                format!("ok ui settings-search {arg}")
            }
            "settings-section" => match (slug_section(arg), &mut self.settings) {
                (Some(s), Some(p)) => {
                    p.sec = SECTIONS.iter().position(|&x| x == s).unwrap_or(0);
                    p.search.set("");
                    p.scroll = 0.0;
                    self.invalidate();
                    format!("ok ui settings-section {arg}")
                }
                _ => format!("FAIL ui settings-section {arg}"),
            },
            "settings-click" => {
                let (name, dark) = match arg.strip_suffix(":dark") {
                    Some(n) => (n, Some(true)),
                    None => (arg.strip_suffix(":light").unwrap_or(arg), None),
                };
                let (Some(it), Some(g)) = (item_of(name), self.panel_geo()) else {
                    return Some(format!("FAIL ui settings-click {arg}: unknown item"));
                };
                let Some(p) = &mut self.settings else {
                    return Some("FAIL ui settings-click: panel closed".into());
                };
                if !p.items(sh.lang).contains(&it) {
                    p.search.set("");
                    p.sec = SECTIONS
                        .iter()
                        .position(|&x| x == it.section())
                        .unwrap_or(0);
                }
                let items = p.items(sh.lang);
                let k = items.iter().position(|x| *x == it).unwrap_or(0);
                p.ensure_visible(&g, &items, k);
                let rects = p.layout(&g, &items);
                let r = rects.get(k).copied().unwrap_or_else(|| rr(g.content));
                let at = match (it, dark) {
                    (It::Color(_), Some(true)) => {
                        let c = control_rect(it, r, sh.lang);
                        Some((c.x + 70.0, c.y + 4.0))
                    }
                    (It::Color(_), None) => {
                        let c = control_rect(it, r, sh.lang);
                        Some((c.x + 4.0, c.y + 4.0))
                    }
                    _ => None,
                };
                self.activate(sh, it, r, at);
                format!("ok ui settings-click {arg}")
            }
            "settings-set" => {
                let (k, v) = arg.split_once('=').unwrap_or((arg, ""));
                match ROWS.iter().find(|r| r.key == k) {
                    Some(r) => {
                        self.actions
                            .push(Action::Panel(Op::Set(r.key, v.to_string())));
                        format!("ok ui settings-set {arg}")
                    }
                    None => format!("FAIL ui settings-set {arg}: unknown key"),
                }
            }
            "settings-capture" => match Chord::parse(arg) {
                Some(c) if self.settings.as_ref().is_some_and(|p| p.capture.is_some()) => {
                    self.capture_key(c.vk, c.ctrl, c.shift, c.alt);
                    format!("ok ui settings-capture {arg}")
                }
                _ => format!("FAIL ui settings-capture {arg}"),
            },
            "settings-color" => {
                let hex = colors::parse_hex(arg);
                match (self.settings.as_mut().and_then(|p| p.color.as_mut()), hex) {
                    (Some(c), Some(v)) => {
                        c.set_rgb(v);
                        self.apply_color(true);
                        format!("ok ui settings-color {arg}")
                    }
                    _ => format!("FAIL ui settings-color {arg}"),
                }
            }
            "settings-font" => match self.settings.as_mut().and_then(|p| p.font.as_mut()) {
                Some(f) => {
                    f.query.set(arg);
                    f.sel = 0;
                    if f.shown().is_empty() {
                        format!("FAIL ui settings-font {arg}: not installed")
                    } else {
                        self.choose_font();
                        format!("ok ui settings-font {arg}")
                    }
                }
                None => "FAIL ui settings-font: picker closed".into(),
            },
            _ => return None,
        };
        Some(r)
    }

    /// State fields for scripts.
    pub(super) fn panel_state(&self, lang: Lang) -> Vec<(&'static str, String)> {
        let Some(p) = &self.settings else {
            return vec![
                ("settings", "closed".into()),
                ("settings_section", "none".into()),
                ("settings_rows", "0".into()),
                ("capture", "none".into()),
                ("conflict", "none".into()),
                ("color_popover", "closed".into()),
                ("font_list", "closed".into()),
            ];
        };
        let items = p.items(lang);
        let rows = items.iter().filter(|i| !matches!(i, It::Head(_))).count();
        let capture = p
            .capture
            .and_then(|i| ACTIONS.get(i))
            .map_or_else(|| "none".to_string(), |a| a.id.to_string());
        let conflict = p
            .conflict
            .as_ref()
            .and_then(|c| ACTIONS.get(c.1))
            .map_or_else(|| "none".to_string(), |a| a.id.to_string());
        let color = p.color.as_ref().map_or_else(
            || "closed".to_string(),
            |c| {
                let t = TOKENS.get(c.token).copied().unwrap_or("?");
                format!("{t}:{}", if c.dark { "dark" } else { "light" })
            },
        );
        let font = p
            .font
            .as_ref()
            .map_or_else(|| "closed".to_string(), |f| f.shown().len().to_string());
        vec![
            ("settings", "open".into()),
            ("settings_section", section_slug(p.section())),
            ("settings_rows", rows.to_string()),
            ("capture", capture),
            ("conflict", conflict),
            ("color_popover", color),
            ("font_list", font),
        ]
    }
}
