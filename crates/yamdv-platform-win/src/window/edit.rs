//! V1-B editing (prompt 006): Split Edit (F3, editor pane + live preview), block-level Live
//! Edit (F2, one top-level block shown as its source), saving (autosave, Ctrl+S, conflicts,
//! S-FILE-03), IME, task checkbox toggle. One [`Editor`] per document serves both modes.

use super::*;
use std::cell::RefCell as Cell2;
use windows::Win32::UI::Input::Ime::{
    CANDIDATEFORM, CFS_EXCLUDE, CFS_POINT, COMPOSITIONFORM, GCS_COMPSTR, GCS_RESULTSTR,
    ImmGetCompositionStringW, ImmGetContext, ImmReleaseContext, ImmSetCandidateWindow,
    ImmSetCompositionWindow,
};
use windows::Win32::UI::WindowsAndMessaging::{IDC_SIZEWE, WM_IME_COMPOSITION};
use yamdv_core::encode::{self, EncodeError, Stamp};
use yamdv_editor::{Editor, Motion, md};

pub(super) const TIMER_AUTOSAVE: usize = 13;
pub(super) const TIMER_PREVIEW: usize = 14;
pub(super) const TIMER_CARET: usize = 15;
pub(super) const TIMER_EDRAG: usize = 16;
const WM_CLOSE: u32 = 0x0010;
const WM_ACTIVATE: u32 = 0x0006;
const WM_IME_STARTCOMPOSITION: u32 = 0x010D;
const WM_IME_ENDCOMPOSITION: u32 = 0x010E;
const SPLITTER: f32 = 6.0;
const PANE_PAD: f32 = 12.0;
const BLINK_MS: u32 = 530;
/// Caret blink stops after 5 s without input (T7: no idle timers).
const BLINK_STOP: f64 = 5.0;
/// Preview re-parse debounce after a typing pause (§2.3: preview ≤ 50 ms after the pause).
const PREVIEW_MS: u32 = 30;

pub(super) const BTN_OVERWRITE: u32 = 10;
pub(super) const BTN_RELOAD_DISCARD: u32 = 11;
pub(super) const BTN_SAVE_AS: u32 = 12;
pub(super) const BTN_CONVERT: u32 = 13;
pub(super) const BTN_SAVE: u32 = 14;
pub(super) const BTN_DISCARD: u32 = 15;

/// What an editing dialog is about (stored in `Pending::Edit`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Ask {
    /// Editing refused (read-only / lossy / plain / built-in); the requested mode.
    Blocked(Mode),
    /// The file changed on disk while the buffer is dirty.
    Conflict,
    /// The buffer has characters the file's encoding cannot represent.
    Unmappable,
    /// Close with unsaved changes (autosave off).
    Unsaved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Live,
    Split,
}

/// File facts captured at open (the bytes are dropped after decoding).
#[derive(Debug, Clone)]
pub(super) struct Facts {
    pub encoding: &'static str,
    pub bom: bool,
    pub lossy: bool,
    pub eol: &'static str,
    pub read_only: bool,
    pub plain: bool,
    pub stamp: Option<Stamp>,
    /// Mode to enter after a "Save … as…" round trip through the dialog host.
    pub after_save_as: Option<Mode>,
}

impl Default for Facts {
    fn default() -> Self {
        Self {
            encoding: "UTF-8",
            bom: false,
            lossy: false,
            eol: "\n",
            read_only: false,
            plain: false,
            stamp: None,
            after_save_as: None,
        }
    }
}

/// `editing.*` settings snapshot.
#[derive(Debug, Clone)]
pub(super) struct Opts {
    pub autosave: bool,
    pub delay: u32,
    pub code_font: bool,
    pub line_numbers: bool,
    pub soft_wrap: bool,
    pub ratio: f32,
    pub swapped: bool,
    pub indent: &'static str,
}

impl Opts {
    fn load() -> Self {
        let b = |k: &str| app::get(k) == "true";
        Self {
            autosave: b("editing.autosave"),
            delay: app::get("editing.autosave_delay_ms")
                .parse()
                .unwrap_or(800u32)
                .clamp(300, 5000),
            code_font: app::get("editing.editor_font") == "code",
            line_numbers: b("editing.editor_line_numbers"),
            soft_wrap: b("editing.editor_soft_wrap"),
            ratio: app::get("editing.split_ratio")
                .parse()
                .unwrap_or(0.5f32)
                .clamp(0.2, 0.8),
            swapped: b("editing.split_swapped"),
            indent: match app::get("editing.list_indent").as_str() {
                "4" => "    ",
                "tab" => "\t",
                _ => "  ",
            },
        }
    }
}

/// F2: the block being edited. The document is frozen meanwhile; the block's current text is
/// `ed[start .. start + old_len + (ed.len() − doc.len())]`.
#[derive(Debug, Clone, Copy)]
pub(super) struct Active {
    pub block: usize,
    pub start: usize,
    /// Length of the block text in the frozen document (without its final line break).
    pub old_len: usize,
}

/// F3 editor pane: the buffer as plain content with its own viewer.
pub(super) struct Pane {
    pub content: Content,
    pub viewer: Viewer,
    /// Horizontal scroll (soft wrap off).
    pub hx: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Drag {
    Pane,
    Preview,
    Splitter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Follow {
    None,
    Preview,
}

pub(super) struct Edit {
    pub ed: Editor,
    pub mode: Option<Mode>,
    pub active: Option<Active>,
    pub pane: Option<Pane>,
    pub opts: Opts,
    goal_x: Option<f32>,
    drag: Option<Drag>,
    drag_y: f32,
    blink_on: bool,
    last_input: Instant,
    /// Key → caret painted latency (T5): the time of the last unpainted edit.
    pub typed_at: Option<Instant>,
    pub lat: Vec<f64>,
    pub ime: String,
    surrogate: Option<u16>,
    /// Buffer version the preview document mirrors.
    synced: u64,
    follow: Follow,
    last_preview_scroll: f64,
    /// Close was confirmed (dialog answered); the next WM_CLOSE passes.
    close_ok: bool,
    dirty_shown: bool,
    nums: Cell2<HashMap<usize, Box<dyn yamdv_layout::shaper::Shaped>>>,
    /// F2 inline reveal analysis of the active block (ADR-0020).
    pub reveal: Option<super::wysi::RevealCache>,
}

impl Edit {
    pub(super) fn new(text: &str, eol: &'static str) -> Self {
        Self {
            ed: Editor::new(text, eol),
            mode: None,
            active: None,
            pane: None,
            opts: Opts::load(),
            goal_x: None,
            drag: None,
            drag_y: 0.0,
            blink_on: true,
            last_input: Instant::now(),
            typed_at: None,
            lat: Vec::new(),
            ime: String::new(),
            surrogate: None,
            synced: 0,
            follow: Follow::None,
            last_preview_scroll: 0.0,
            close_ok: false,
            dirty_shown: false,
            nums: Cell2::new(HashMap::new()),
            reveal: None,
        }
    }
}

/// FILETIME ticks of a modification time (0 when unknown).
pub(super) fn ticks(t: Option<std::time::SystemTime>) -> u64 {
    t.and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| {
            u64::try_from(d.as_nanos() / 100)
                .unwrap_or(0)
                .saturating_add(116_444_736_000_000_000)
        })
}

fn now_ms() -> u64 {
    u64::try_from(app::now_ms()).unwrap_or(0)
}

/// Block containing source offset `pos` (last block starting at or before it).
pub(super) fn block_of(content: &Content, pos: usize) -> usize {
    let n = content.len();
    let (mut lo, mut hi) = (0usize, n);
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        if content.range(mid).start as usize <= pos {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

thread_local! {
    /// Alt state of a chord re-dispatched by the keymap (FUT-05); `None` = read the keyboard.
    pub(super) static KEY_ALT: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

fn key_down(vk: u16) -> bool {
    // SAFETY: GetKeyState has no preconditions.
    unsafe { GetKeyState(i32::from(vk)) < 0 }
}

fn prep(
    viewer: &mut Viewer,
    content: &mut Content,
    sh: &Shared,
    pal: &Palette,
    images: &crate::images::Store,
    rich: &yamdv_layout::rich::RichCache,
    vh: f64,
) {
    let strings = I18nStrings(sh.lang);
    let mut shaper = DwShaper(&sh.dw);
    let mut env = Env {
        shaper: &mut shaper,
        m: sh.m,
        pal,
        strings: &strings,
        single_newline_break: sh.read.single_newline_break,
        images,
        rich: &crate::dwrite::Rich::new(&sh.dw, rich, sh.read.render_math, sh.read.render_mermaid),
    };
    viewer.prepare(content, &mut env, vh);
}

/// Pane geometry (client DIPs).
pub(super) struct Geo {
    pub r: Rect,
    pub gutter: f32,
    pub text_x: f32,
    pub text_w: f32,
}

impl Win {
    // ---- access --------------------------------------------------------------------------

    pub(super) fn edit(&self) -> Option<&Edit> {
        match &self.view {
            View::Doc(d) => d.edit.as_deref(),
            _ => None,
        }
    }

    pub(super) fn edit_mut(&mut self) -> Option<&mut Edit> {
        match &mut self.view {
            View::Doc(d) => d.edit.as_deref_mut(),
            _ => None,
        }
    }

    pub(super) fn edit_mode(&self) -> Option<Mode> {
        self.edit().and_then(|e| e.mode)
    }

    pub(super) fn edit_mode_id(&self) -> u8 {
        match self.edit_mode() {
            None => 0,
            Some(Mode::Live) => 1,
            Some(Mode::Split) => 2,
        }
    }

    /// Keys/chars go to the editor: F3, or F2 with an active block.
    pub(super) fn typing(&self) -> bool {
        self.edit().is_some_and(|e| match e.mode {
            Some(Mode::Split) => true,
            Some(Mode::Live) => e.active.is_some(),
            None => false,
        })
    }

    /// (editor pane, preview, splitter) in Split Edit.
    pub(super) fn split_rects(&self) -> Option<(Rect, Rect, Rect)> {
        let e = self.edit()?;
        if e.mode != Some(Mode::Split) {
            return None;
        }
        let avail = (self.cw - SPLITTER).max(2.0);
        let ew = (avail * e.opts.ratio).round();
        let pw = avail - ew;
        let h = self.ch;
        Some(if e.opts.swapped {
            (
                Rect::new(pw + SPLITTER, 0.0, ew, h),
                Rect::new(0.0, 0.0, pw, h),
                Rect::new(pw, 0.0, SPLITTER, h),
            )
        } else {
            (
                Rect::new(0.0, 0.0, ew, h),
                Rect::new(ew + SPLITTER, 0.0, pw, h),
                Rect::new(ew, 0.0, SPLITTER, h),
            )
        })
    }

    pub(super) fn pane_geo(&self, sh: &Shared) -> Option<Geo> {
        let (r, _, _) = self.split_rects()?;
        let e = self.edit()?;
        let gutter = if e.opts.line_numbers {
            let digits = e.ed.line_count().to_string().len().max(3) as f32;
            (digits * sh.m.font_size * 0.62 + 2.0 * PANE_PAD).round()
        } else {
            0.0
        };
        let text_x = r.x + gutter + PANE_PAD;
        Some(Geo {
            r,
            gutter,
            text_x,
            text_w: (r.w - gutter - 2.0 * PANE_PAD).max(40.0),
        })
    }

    fn help_rect(g: &Geo) -> Rect {
        Rect::new(g.r.x + g.r.w - 16.0 - 28.0, g.r.h - 16.0 - 28.0, 28.0, 28.0)
    }

    fn invalidate_rect(&self, r: Rect) {
        let s = self.scale;
        let rc = RECT {
            left: (r.x * s).floor() as i32,
            top: (r.y * s).floor() as i32,
            right: ((r.x + r.w) * s).ceil() as i32,
            bottom: ((r.y + r.h) * s).ceil() as i32,
        };
        let rc = self.crc(rc);
        // SAFETY: valid hwnd and rect.
        let _ = unsafe { InvalidateRect(Some(self.hwnd), Some(&rc), false) };
    }

    pub(super) fn invalidate_pane(&self) {
        match self.split_rects() {
            Some((r, _, _)) => self.invalidate_rect(r),
            None => self.invalidate(),
        }
    }

    // ---- facts / blocking ----------------------------------------------------------------

    /// Why editing is refused, if it is.
    fn edit_blocked(&self) -> Option<Msg> {
        let View::Doc(d) = &self.view else {
            return Some(Msg::EditVirtual);
        };
        if d.virt {
            Some(Msg::EditVirtual)
        } else if d.facts.plain {
            Some(Msg::EditPlain)
        } else if d.facts.read_only {
            Some(Msg::EditReadOnly)
        } else if d.facts.lossy {
            Some(Msg::EditLossy)
        } else {
            None
        }
    }

    // ---- modes ---------------------------------------------------------------------------

    /// F2 / F3: enter `mode`, switch to it, or leave it when it is the current one.
    pub(super) fn edit_toggle(&mut self, sh: &Shared, mode: Mode) {
        if self.dialog.is_some() || !matches!(self.view, View::Doc(_)) {
            return;
        }
        if self.edit_mode() == Some(mode) {
            self.edit_exit(sh);
            return;
        }
        if let Some(why) = self.edit_blocked() {
            let mut buttons = vec![(Msg::Cancel, BTN_CANCEL)];
            match why {
                Msg::EditReadOnly => buttons.push((Msg::SaveCopyAs, BTN_SAVE_AS)),
                Msg::EditLossy => buttons.push((Msg::ConvertEdit, BTN_CONVERT)),
                _ => {}
            }
            self.ask(
                sh,
                Msg::EditNotAvailable,
                why,
                buttons,
                Pending::Edit(Ask::Blocked(mode)),
            );
            return;
        }
        self.edit_enter(sh, mode);
    }

    fn edit_enter(&mut self, sh: &Shared, mode: Mode) {
        self.edit_leave_block(sh);
        {
            let View::Doc(d) = &mut self.view else { return };
            if d.edit.is_none() {
                d.edit = Some(Box::new(Edit::new(d.content.source(), d.facts.eol)));
            }
            let top = d.content.range(d.viewer.anchor().0).start as usize;
            let Some(e) = d.edit.as_deref_mut() else {
                return;
            };
            e.opts = Opts::load();
            e.mode = Some(mode);
            e.pane = None;
            e.close_ok = false;
            if mode == Mode::Split {
                let content = Content::plain(e.ed.text());
                let mut viewer = Viewer::new(&content, 400.0, sh.m);
                viewer.plain = yamdv_layout::viewer::PlainOpts {
                    editor: true,
                    code_font: e.opts.code_font,
                    wrap: e.opts.soft_wrap,
                };
                e.pane = Some(Pane {
                    content,
                    viewer,
                    hx: 0.0,
                });
                // Reading position kept: the caret starts at the top visible block.
                if e.ed.caret() == 0 && e.ed.anchor() == 0 {
                    e.ed.move_to(top, false);
                }
            }
            e.synced = e.ed.version;
            e.last_input = Instant::now();
            e.blink_on = true;
        }
        self.sel = None;
        self.focus = None;
        if let Some(f) = &mut self.find {
            f.dirty = true;
        }
        update_size(self, sh);
        if mode == Mode::Split {
            self.prep_pane(sh);
            self.pane_scroll_to_caret(sh, true);
        }
        self.edit_title();
        self.blink_restart();
        self.invalidate();
    }

    /// Leaves the edit mode (autosave), keeping the buffer only while it has unsaved changes.
    pub(super) fn edit_exit(&mut self, sh: &Shared) {
        self.edit_leave_block(sh);
        self.sync_preview(sh);
        let autosave = self
            .edit()
            .is_some_and(|e| e.opts.autosave && e.ed.is_dirty());
        if autosave {
            self.edit_save(sh, false, false);
        }
        if let View::Doc(d) = &mut self.view {
            if let Some(e) = d.edit.as_deref_mut() {
                e.mode = None;
                e.pane = None;
                e.drag = None;
            }
            if d.edit.as_ref().is_some_and(|e| !e.ed.is_dirty()) {
                d.edit = None;
            }
        }
        self.kill(TIMER_CARET);
        if let Some(f) = &mut self.find {
            f.dirty = true;
        }
        update_size(self, sh);
        self.edit_title();
        self.invalidate();
    }

    fn kill(&self, id: usize) {
        // SAFETY: valid hwnd.
        let _ = unsafe { KillTimer(Some(self.hwnd), id) };
    }

    fn timer_once(&self, id: usize, ms: u32) {
        // SAFETY: valid hwnd.
        unsafe { SetTimer(Some(self.hwnd), id, ms, None) };
    }

    pub(super) fn edit_title(&mut self) {
        let Some(sh) = shared() else { return };
        let View::Doc(d) = &self.view else { return };
        let name = ypath::file_name(&ypath::display_form(&d.path)).to_string();
        let (dirty, mode) = d
            .edit
            .as_deref()
            .map_or((false, None), |e| (e.ed.is_dirty(), e.mode));
        let mut t = name;
        if dirty {
            t.push_str(" \u{25CF}");
        }
        match mode {
            Some(Mode::Live) => {
                t.push_str(" \u{2014} ");
                t.push_str(tr(sh.lang, Msg::MLiveEdit));
            }
            Some(Mode::Split) => {
                t.push_str(" \u{2014} ");
                t.push_str(tr(sh.lang, Msg::MSplitEdit));
            }
            None => {}
        }
        t.push_str(" \u{2014} YAMDV");
        if let Some(e) = self.edit_mut() {
            e.dirty_shown = dirty;
        }
        self.set_title(&t);
    }

    // ---- F2 block ------------------------------------------------------------------------

    /// Activates block `bi` with the caret at `caret` (clamped into the block).
    pub(super) fn enter_block(&mut self, sh: &Shared, bi: usize, caret: usize) {
        {
            let View::Doc(d) = &mut self.view else { return };
            let Some(e) = d.edit.as_deref_mut() else {
                return;
            };
            if bi >= d.content.len() {
                return;
            }
            let r = d.content.range(bi);
            let src = d.content.source();
            let (start, mut end) = (r.start as usize, (r.end as usize).min(src.len()));
            let b = src.as_bytes();
            if end > start && b.get(end - 1) == Some(&b'\n') {
                end -= 1;
                if end > start && b.get(end - 1) == Some(&b'\r') {
                    end -= 1;
                }
            }
            e.active = Some(Active {
                block: bi,
                start,
                old_len: end - start,
            });
            e.ed.move_to(caret.clamp(start, end), false);
            e.synced = e.ed.version;
            d.viewer.source_block = Some(yamdv_layout::viewer::SourceBlock {
                block: bi,
                start: u32::try_from(start).unwrap_or(0),
                text: src.get(start..end).unwrap_or("").to_string(),
                code_font: e.opts.code_font,
                reveal: None,
                preview: None,
            });
            d.viewer.invalidate_block(bi);
        }
        self.sel = None;
        self.live_reveal();
        self.prepare_now(sh);
        self.live_scroll_to_caret(sh);
        self.blink_restart();
        self.invalidate();
    }

    /// Current active range in buffer offsets.
    pub(super) fn active_range(&self) -> Option<Range<usize>> {
        let View::Doc(d) = &self.view else {
            return None;
        };
        let e = d.edit.as_deref()?;
        let a = e.active?;
        let delta = e.ed.len() as i64 - d.content.source().len() as i64;
        let len = (a.old_len as i64 + delta).max(0) as usize;
        Some(a.start..a.start + len)
    }

    /// Leaves the F2 block: the document catches up with the buffer (only the edited blocks
    /// re-parse; untouched blocks are never re-serialized). Returns the splice.
    pub(super) fn edit_leave_block(
        &mut self,
        _sh: &Shared,
    ) -> Option<yamdv_core::doc::index::Splice> {
        let vh = self.view_h();
        let View::Doc(d) = &mut self.view else {
            return None;
        };
        let e = d.edit.as_deref_mut()?;
        let a = e.active.take()?;
        d.viewer.source_block = None;
        d.viewer.invalidate_block(a.block);
        let text = e.ed.text();
        e.synced = e.ed.version;
        let sp = match &mut d.content {
            Content::Markdown(doc) => doc.sync_to(&text).map(|(_, sp)| sp),
            _ => None,
        };
        if let Some(sp) = &sp {
            d.viewer.splice(&d.content, sp, vh);
            d.viewer.refresh_toc(&d.content);
        }
        if let Some(f) = &mut self.find {
            f.dirty = true;
        }
        self.invalidate();
        sp
    }

    /// Source offset under client (x, y) in the rendered document.
    pub(super) fn doc_hit(&self, sh: &Shared, x: f32, y: f32) -> Option<usize> {
        let (cx, _) = self.col(sh);
        let View::Doc(d) = &self.view else {
            return None;
        };
        let pos = d
            .viewer
            .hit(&d.content, x - cx, f64::from(y) + d.viewer.scroll)?
            .source as usize;
        // Offsets after an edited active block shift by the pending edit.
        let e = d.edit.as_deref()?;
        Some(match e.active {
            Some(a) if pos >= a.start + a.old_len => {
                let delta = e.ed.len() as i64 - d.content.source().len() as i64;
                (pos as i64 + delta).max(0) as usize
            }
            _ => pos,
        })
    }

    /// F2 click at client (x, y) outside the active block: leave it and enter the clicked one
    /// with the caret at the clicked source position.
    fn live_click(&mut self, sh: &Shared, x: f32, y: f32) {
        let (cx, _) = self.col(sh);
        let (hit, old_end, delta) = {
            let View::Doc(d) = &self.view else { return };
            let doc_y = f64::from(y) + d.viewer.scroll;
            let Some(h) = d.viewer.hit(&d.content, x - cx, doc_y) else {
                return;
            };
            let old_end = d
                .edit
                .as_deref()
                .and_then(|e| e.active)
                .map(|a| d.content.range(a.block).end as usize);
            let delta = d
                .edit
                .as_deref()
                .map_or(0, |e| e.ed.len() as i64 - d.content.source().len() as i64);
            (h, old_end, delta)
        };
        self.edit_leave_block(sh);
        let mut pos = hit.source as usize;
        if let Some(end) = old_end
            && pos >= end
        {
            pos = (pos as i64 + delta).max(0) as usize;
        }
        let bi = match &self.view {
            View::Doc(d) => block_of(&d.content, pos),
            _ => return,
        };
        self.enter_block(sh, bi, pos);
    }

    /// The active block's laid paragraph and its client origin.
    pub(super) fn live_para(
        &self,
        sh: &Shared,
    ) -> Option<(f32, f32, &yamdv_layout::block::LaidPara)> {
        let View::Doc(d) = &self.view else {
            return None;
        };
        let a = d.edit.as_deref()?.active?;
        let p = d.viewer.layout_of(a.block)?.paras.first()?;
        let (cx, _) = self.col(sh);
        let top = (d.viewer.block_top(&d.content, a.block) - d.viewer.scroll) as f32;
        Some((cx + p.x, top + p.y, p))
    }

    fn live_scroll_to_caret(&mut self, sh: &Shared) {
        let Some(r) = self.caret_rect(sh) else { return };
        let vh = self.view_h() as f32;
        let s = self.scroll_pos();
        if r.y < 8.0 {
            self.set_scroll(s + f64::from(r.y) - 48.0);
        } else if r.y + r.h > vh - 8.0 {
            self.set_scroll(s + f64::from(r.y + r.h - vh) + 48.0);
        }
    }

    // ---- F3 pane -------------------------------------------------------------------------

    fn prep_pane(&mut self, sh: &Shared) {
        let Some(g) = self.pane_geo(sh) else { return };
        let vh = self.view_h();
        let pal = self.pal;
        let View::Doc(d) = &mut self.view else { return };
        let DocView {
            edit, images, rich, ..
        } = &mut **d;
        let Some(p) = edit.as_deref_mut().and_then(|e| e.pane.as_mut()) else {
            return;
        };
        p.viewer.set_width(&p.content, g.text_w, vh);
        prep(&mut p.viewer, &mut p.content, sh, &pal, images, rich, vh);
    }

    fn pane_chunk_of(p: &Pane, pos: usize) -> usize {
        match &p.content {
            Content::Plain { chunks, .. } => chunks
                .partition_point(|r| r.start as usize <= pos)
                .saturating_sub(1),
            _ => 0,
        }
    }

    /// Caret in pane document coordinates: (x from the text origin, y, height).
    fn pane_caret(e: &Edit) -> Option<(f32, f64, f32)> {
        let p = e.pane.as_ref()?;
        let pos = e.ed.caret();
        let i = Self::pane_chunk_of(p, pos);
        let para = p.viewer.layout_of(i)?.paras.first()?;
        let map = para.map.as_ref()?;
        let r = para
            .shaped
            .caret(map.from_source(u32::try_from(pos).unwrap_or(0)));
        Some((
            para.x + r.x,
            p.viewer.block_top(&p.content, i) + f64::from(para.y + r.y),
            r.h,
        ))
    }

    /// Keeps the caret visible in the pane; `center` puts it one third from the top.
    pub(super) fn pane_scroll_to_caret(&mut self, sh: &Shared, center: bool) {
        let Some(g) = self.pane_geo(sh) else { return };
        let vh = self.view_h();
        let pitch = f64::from(sh.m.pitch());
        let need_layout = self.edit().is_some_and(|e| Self::pane_caret(e).is_none());
        if need_layout {
            if let Some(e) = self.edit_mut()
                && let Some(p) = e.pane.as_mut()
            {
                let i = Self::pane_chunk_of(p, e.ed.caret());
                let y = p.viewer.block_top(&p.content, i);
                p.viewer.scroll_to(y - vh / 3.0, vh);
            }
            self.prep_pane(sh);
        }
        let mut changed = false;
        if let Some(e) = self.edit_mut()
            && let Some((x, y, h)) = Self::pane_caret(e)
            && let Some(p) = e.pane.as_mut()
        {
            let s = p.viewer.scroll;
            let h = f64::from(h);
            if center && (y < s || y + h > s + vh) {
                p.viewer.scroll_to(y - vh / 3.0, vh);
                changed = true;
            } else if y < s + 4.0 {
                p.viewer.scroll_to(y - pitch, vh);
                changed = true;
            } else if y + h > s + vh - 4.0 {
                p.viewer.scroll_to(y + h - vh + pitch, vh);
                changed = true;
            }
            if !e.opts.soft_wrap {
                if x - p.hx > g.text_w - 24.0 {
                    p.hx = x - g.text_w + 64.0;
                    changed = true;
                } else if x < p.hx + 8.0 && p.hx > 0.0 {
                    p.hx = (x - 64.0).max(0.0);
                    changed = true;
                }
            }
        }
        if changed || need_layout {
            self.prep_pane(sh);
            self.invalidate_pane();
            self.sync_preview_scroll();
        }
    }

    /// Scroll sync, editor → preview: the preview shows the block at the pane's top line.
    fn sync_preview_scroll(&mut self) {
        let target = {
            let View::Doc(d) = &self.view else { return };
            let Some(e) = d.edit.as_deref() else { return };
            let Some(p) = &e.pane else { return };
            let Some(h) = p.viewer.hit(&p.content, 0.0, p.viewer.scroll + 2.0) else {
                return;
            };
            if p.viewer.scroll <= 0.5 {
                0.0
            } else {
                let bi = block_of(&d.content, h.source as usize);
                (d.viewer.block_top(&d.content, bi) - 16.0).max(0.0)
            }
        };
        if let Some(e) = self.edit_mut() {
            e.follow = Follow::None;
        }
        if (self.scroll_pos() - target).abs() > 0.5 {
            self.set_scroll(target);
        }
        let s = self.scroll_pos();
        if let Some(e) = self.edit_mut() {
            e.last_preview_scroll = s;
        }
    }

    /// Scroll sync, preview → editor (wheel / scrollbar on the preview).
    fn sync_pane_scroll(&mut self, sh: &Shared) {
        let vh = self.view_h();
        let pitch = f64::from(sh.m.pitch());
        {
            let View::Doc(d) = &mut self.view else { return };
            let s = d.viewer.scroll;
            let top_src = d.content.range(d.viewer.anchor().0).start as usize;
            let Some(e) = d.edit.as_deref_mut() else {
                return;
            };
            if e.follow != Follow::Preview || (s - e.last_preview_scroll).abs() < 0.5 {
                return;
            }
            e.last_preview_scroll = s;
            let line = e.ed.line_of(top_src);
            let Some(p) = e.pane.as_mut() else { return };
            let i = Self::pane_chunk_of(p, top_src);
            let r = p.content.range(i);
            let first = e.ed.line_of(r.start as usize);
            let y = if s <= 0.5 {
                0.0
            } else {
                p.viewer.block_top(&p.content, i) + line.saturating_sub(first) as f64 * pitch
            };
            p.viewer.scroll_to(y, vh);
        }
        self.prep_pane(sh);
        self.invalidate_pane();
    }

    // ---- edits ---------------------------------------------------------------------------

    /// After every buffer change or caret move: mirror, re-render, autosave timer, title.
    pub(super) fn after_edit(&mut self, sh: &Shared, changed: bool) {
        let vh = self.view_h();
        let mode = self.edit_mode();
        if let Some(e) = self.edit_mut() {
            e.last_input = Instant::now();
            e.blink_on = true;
            if changed {
                e.goal_x = None;
            }
        }
        if changed {
            match mode {
                Some(Mode::Split) => {
                    if let Some(e) = self.edit_mut() {
                        let text = e.ed.text();
                        if let Some(p) = e.pane.as_mut()
                            && let Some(sp) = p.content.plain_sync(&text)
                        {
                            p.viewer.splice(&p.content, &sp, vh);
                        }
                    }
                    self.timer_once(TIMER_PREVIEW, PREVIEW_MS);
                    self.prep_pane(sh);
                }
                Some(Mode::Live) => self.live_after_edit(sh),
                None => self.sync_preview(sh),
            }
            if let Some(f) = &mut self.find {
                f.dirty = true;
            }
            self.journal_edited();
            let (autosave, delay) = self
                .edit()
                .map_or((false, 0), |e| (e.opts.autosave, e.opts.delay));
            if autosave && mode.is_some() {
                self.timer_once(TIMER_AUTOSAVE, delay);
            }
            let dirty = self.edit().is_some_and(|e| e.ed.is_dirty());
            if self.edit().is_some_and(|e| e.dirty_shown != dirty) {
                self.edit_title();
            }
        }
        match mode {
            Some(Mode::Split) => {
                self.pane_scroll_to_caret(sh, false);
                self.invalidate_pane();
            }
            Some(Mode::Live) => {
                if !changed && self.live_reveal() {
                    self.prepare_now(sh);
                }
                self.live_scroll_to_caret(sh);
                self.invalidate();
            }
            None => self.invalidate(),
        }
        self.ime_position(sh);
        self.blink_restart();
    }

    /// F2: an edit inside the active block re-lays out only that block; anything else (undo
    /// elsewhere, select-all) leaves the block and re-syncs the document.
    fn live_after_edit(&mut self, sh: &Shared) {
        let inside: Option<bool> = (|| {
            let View::Doc(d) = &self.view else {
                return None;
            };
            let e = d.edit.as_deref()?;
            let a = e.active?;
            let new = e.ed.text();
            let (o, n) = (d.content.source().as_bytes(), new.as_bytes());
            let p = o.iter().zip(n).take_while(|(x, y)| x == y).count();
            let max_s = o.len().min(n.len()) - p;
            let s = o
                .iter()
                .rev()
                .zip(n.iter().rev())
                .take(max_s)
                .take_while(|(x, y)| x == y)
                .count();
            let old_end = o.len() - s;
            let caret = e.ed.caret();
            let end_now = (a.start + a.old_len) as i64 + n.len() as i64 - o.len() as i64;
            Some(
                p >= a.start
                    && old_end <= a.start + a.old_len
                    && caret >= a.start
                    && (caret as i64) <= end_now,
            )
        })();
        match inside {
            None => {
                self.sync_preview(sh);
                return;
            }
            Some(false) => {
                let s = self.edit().map(|e| (e.ed.anchor(), e.ed.caret()));
                match s {
                    Some((a, c)) if a != c => self.xsel_set(sh, a, c),
                    _ => {
                        self.edit_leave_block(sh);
                    }
                }
                return;
            }
            Some(true) => {}
        }
        let Some(r) = self.active_range() else { return };
        if let View::Doc(d) = &mut self.view
            && let Some(e) = d.edit.as_deref()
            && let Some(sb) = &mut d.viewer.source_block
        {
            sb.text = e.ed.slice(r);
            let b = sb.block;
            d.viewer.invalidate_block(b);
        }
        self.live_reveal();
        self.prepare_now(sh);
    }

    /// F3 / read mode: brings the preview document up to the buffer (incremental splice).
    pub(super) fn sync_preview(&mut self, _sh: &Shared) {
        let vh = self.view_h();
        {
            let View::Doc(d) = &mut self.view else { return };
            let Some(e) = d.edit.as_deref_mut() else {
                return;
            };
            if e.active.is_some() || e.synced == e.ed.version {
                return;
            }
            e.synced = e.ed.version;
            let text = e.ed.text();
            if let Content::Markdown(doc) = &mut d.content
                && let Some((_, sp)) = doc.sync_to(&text)
            {
                d.viewer.splice(&d.content, &sp, vh);
                d.viewer.refresh_toc(&d.content);
            }
        }
        if let Some(f) = &mut self.find {
            f.dirty = true;
        }
        if let Some((_, p, _)) = self.split_rects() {
            self.invalidate_rect(p);
        } else {
            self.invalidate();
        }
    }

    pub(super) fn with_ed(&mut self, sh: &Shared, f: impl FnOnce(&mut Editor)) {
        let Some(e) = self.edit_mut() else { return };
        let v = e.ed.version;
        f(&mut e.ed);
        let changed = e.ed.version != v;
        self.after_edit(sh, changed);
    }

    /// Typed text (WM_CHAR, IME result, driver `type`).
    pub(super) fn edit_type(&mut self, sh: &Shared, s: &str) {
        if s.is_empty() {
            return;
        }
        if let Some(e) = self.edit_mut() {
            e.typed_at = Some(Instant::now());
        }
        if self.f2_type(sh, s) {
            return;
        }
        let t = now_ms();
        self.with_ed(sh, |ed| ed.type_text(s, t));
    }

    pub(super) fn edit_char(&mut self, sh: &Shared, unit: u16) {
        if unit >= 0x20 && unit != 0x7F {
            self.xsel_input(sh);
        }
        let Some(e) = self.edit_mut() else { return };
        let s = if (0xD800..0xDC00).contains(&unit) {
            e.surrogate = Some(unit);
            return;
        } else if (0xDC00..0xE000).contains(&unit) {
            let Some(hi) = e.surrogate.take() else { return };
            String::from_utf16(&[hi, unit]).unwrap_or_default()
        } else {
            e.surrogate = None;
            match char::from_u32(u32::from(unit)) {
                Some(c) if !c.is_control() => {
                    if self.linkpop_char(c) || self.plus_char(sh, c) {
                        return;
                    }
                    c.to_string()
                }
                _ => return,
            }
        };
        self.edit_type(sh, &s);
    }

    // ---- caret geometry --------------------------------------------------------------------

    /// Caret rectangle in client DIPs (F3 pane or F2 block).
    pub(super) fn caret_rect(&self, sh: &Shared) -> Option<Rect> {
        let e = self.edit()?;
        match e.mode? {
            Mode::Split => {
                let g = self.pane_geo(sh)?;
                let p = e.pane.as_ref()?;
                let (x, y, h) = Self::pane_caret(e)?;
                Some(Rect::new(
                    g.text_x - p.hx + x,
                    (y - p.viewer.scroll) as f32,
                    1.5,
                    h,
                ))
            }
            Mode::Live => {
                let (ox, oy, p) = self.live_para(sh)?;
                let map = p.map.as_ref()?;
                let r = p
                    .shaped
                    .caret(map.from_source(u32::try_from(e.ed.caret()).unwrap_or(0)));
                Some(Rect::new(ox + r.x, oy + r.y, 1.5, r.h))
            }
        }
    }

    /// Vertical caret move by `dy` DIPs (↑/↓ one line, PgUp/PgDn a page). F2: past the first
    /// or last line opens the previous / next block.
    fn edit_vertical(&mut self, sh: &Shared, dy: f32, extend: bool) {
        let Some(r) = self.caret_rect(sh) else { return };
        let goal = self.edit().and_then(|e| e.goal_x).unwrap_or(r.x);
        let target_y = r.y + r.h / 2.0 + dy;
        match self.edit_mode() {
            Some(Mode::Split) => {
                let Some(g) = self.pane_geo(sh) else { return };
                let page = dy.abs() > r.h * 1.5;
                let vh = self.view_h();
                if page {
                    if let Some(e) = self.edit_mut()
                        && let Some(p) = e.pane.as_mut()
                    {
                        let s = p.viewer.scroll + f64::from(dy);
                        p.viewer.scroll_to(s, vh);
                    }
                    self.prep_pane(sh);
                    self.invalidate_pane();
                }
                let page_y = if page { r.y + r.h / 2.0 } else { target_y };
                let Some(e) = self.edit_mut() else { return };
                let Some(p) = e.pane.as_ref() else { return };
                let doc_y = if page {
                    p.viewer.scroll + f64::from(page_y)
                } else {
                    p.viewer.scroll + f64::from(target_y)
                };
                let pos = if doc_y < 0.0 || (page && dy < 0.0 && p.viewer.scroll <= 0.0) {
                    0
                } else if doc_y > p.viewer.total_height() {
                    e.ed.len()
                } else {
                    match p.viewer.hit(&p.content, goal - g.text_x + p.hx, doc_y) {
                        Some(h) => h.source as usize,
                        None => e.ed.caret(),
                    }
                };
                e.ed.move_to(pos, extend);
            }
            Some(Mode::Live) => {
                let Some((ox, oy, p)) = self.live_para(sh) else {
                    return;
                };
                let rel = target_y - oy;
                if rel < 0.0 || rel > p.shaped.height() {
                    if extend {
                        if let Some(r) = self.active_range() {
                            self.xsel_extend(sh, if dy < 0.0 { r.start } else { r.end });
                        }
                    } else {
                        self.live_adjacent(sh, dy < 0.0, Some(goal));
                    }
                    return;
                }
                let idx = p.shaped.hit(goal - ox, rel);
                let pos = p.map.as_ref().map_or(0, |m| m.to_source(idx)) as usize;
                if let Some(e) = self.edit_mut() {
                    e.ed.move_to(pos, extend);
                }
            }
            None => return,
        }
        self.after_edit(sh, false);
        if let Some(e) = self.edit_mut() {
            e.goal_x = Some(goal);
        }
    }

    /// F2: opens the previous (caret on its last line) or next (first line) block.
    fn live_adjacent(&mut self, sh: &Shared, up: bool, goal: Option<f32>) {
        let Some(a) = self.edit().and_then(|e| e.active) else {
            return;
        };
        let sp = self.edit_leave_block(sh);
        let n = match &self.view {
            View::Doc(d) => d.content.len(),
            _ => return,
        };
        // The edited block's replacement range after the splice.
        let (first, count) = sp.map_or((a.block, 1), |s| (s.replaced.start, s.inserted.max(1)));
        let target = if up {
            if first == 0 {
                self.edit_title();
                return;
            }
            first - 1
        } else {
            let t = first + count;
            if t >= n {
                self.edit_title();
                return;
            }
            t
        };
        let start = match &self.view {
            View::Doc(d) => d.content.range(target).start as usize,
            _ => return,
        };
        self.enter_block(sh, target, start);
        let pos = match goal {
            None if up => self.active_range().map(|r| r.end),
            None => None,
            Some(goal) => self.live_para(sh).map(|(ox, _, p)| {
                let y = if up { p.shaped.height() - 4.0 } else { 4.0 };
                let idx = p.shaped.hit(goal - ox, y);
                p.map.as_ref().map_or(0, |m| m.to_source(idx)) as usize
            }),
        };
        if let Some(pos) = pos {
            if let Some(e) = self.edit_mut() {
                e.ed.move_to(pos, false);
            }
            self.after_edit(sh, false);
        }
        if let Some(e) = self.edit_mut() {
            e.goal_x = goal;
        }
    }

    /// Home / End on the visual (wrapped) line; at its edge, the logical line edge.
    fn edit_home_end(&mut self, sh: &Shared, end: bool, extend: bool) {
        let Some(r) = self.caret_rect(sh) else { return };
        let y = r.y + r.h / 2.0;
        let x = if end { 1.0e7 } else { -1.0e7 };
        let vis = match self.edit_mode() {
            Some(Mode::Split) => self.edit().and_then(|e| {
                let p = e.pane.as_ref()?;
                p.viewer
                    .hit(&p.content, x, p.viewer.scroll + f64::from(y))
                    .map(|h| h.source as usize)
            }),
            Some(Mode::Live) => self.live_para(sh).and_then(|(_, oy, p)| {
                let idx = p.shaped.hit(x, y - oy);
                p.map.as_ref().map(|m| m.to_source(idx) as usize)
            }),
            None => None,
        };
        let Some(e) = self.edit_mut() else { return };
        let caret = e.ed.caret();
        let line = e.ed.line_of(caret);
        let (ls, le) = (e.ed.line_start(line), e.ed.line_end(line));
        match (end, vis) {
            (false, Some(v)) if v > ls && v < caret => e.ed.move_to(v, extend),
            (true, Some(v)) if v < le && v > caret => e.ed.move_to(v, extend),
            (false, _) => e.ed.motion(Motion::LineStart, extend),
            (true, _) => e.ed.motion(Motion::LineEnd, extend),
        }
        self.after_edit(sh, false);
    }

    // ---- keys ------------------------------------------------------------------------------

    fn move_h(&mut self, sh: &Shared, m: Motion, extend: bool) {
        if let Some(e) = self.edit_mut() {
            e.ed.motion(m, extend);
        }
        self.after_edit(sh, false);
    }

    fn move_to_pos(&mut self, sh: &Shared, pos: usize, extend: bool) {
        if let Some(e) = self.edit_mut() {
            e.ed.move_to(pos, extend);
        }
        self.after_edit(sh, false);
    }

    /// F2: caret at the active block's start/end with no selection → the arrow leaves it.
    fn at_block_edge(&self, end: bool) -> bool {
        let (Some(r), Some(e)) = (self.active_range(), self.edit()) else {
            return false;
        };
        e.ed.selection().is_empty() && e.ed.caret() == if end { r.end } else { r.start }
    }

    /// Editing keys; `Some(_)` = handled. Called by `key` after the dialog and find bar.
    pub(super) fn edit_key(
        &mut self,
        sh: &Shared,
        vk: u16,
        ctrl: bool,
        shift: bool,
    ) -> Option<Option<Action>> {
        if !matches!(self.view, View::Doc(_)) {
            return None;
        }
        let alt = KEY_ALT
            .with(std::cell::Cell::get)
            .unwrap_or_else(|| key_down(0x12));
        match vk {
            0x71 => {
                self.edit_toggle(sh, Mode::Live);
                return Some(None);
            }
            0x72 => {
                self.edit_toggle(sh, Mode::Split);
                return Some(None);
            }
            0x53 if ctrl && !alt && self.edit().is_some() => {
                self.edit_leave_block(sh);
                self.edit_save(sh, false, false);
                return Some(None);
            }
            _ => {}
        }
        let mode = self.edit_mode()?;
        if vk == VK_ESCAPE.0 {
            if mode == Mode::Live && self.edit().is_some_and(|e| e.active.is_some()) {
                self.edit_leave_block(sh);
                self.edit_title();
            } else {
                self.edit_exit(sh);
            }
            return Some(None);
        }
        // Undo / redo also work in F2 between blocks.
        if ctrl && !alt {
            let redo = match vk {
                0x5A => Some(shift),
                0x59 => Some(true),
                _ => None,
            };
            if let Some(redo) = redo {
                self.with_ed(sh, |ed| {
                    if redo {
                        ed.redo();
                    } else {
                        ed.undo();
                    }
                });
                return Some(None);
            }
        }
        if !self.typing() {
            return None;
        }
        // AltGr (Ctrl+Alt) produces characters: never a shortcut.
        if ctrl && alt {
            return Some(None);
        }
        let pitch = sh.m.pitch();
        let page = (self.ch - pitch * 2.0).max(pitch);
        let live = mode == Mode::Live;
        match vk {
            v if v == VK_LEFT.0 => {
                if live && !shift && self.at_block_edge(false) {
                    self.live_adjacent(sh, true, None);
                } else {
                    let m = if ctrl { Motion::WordLeft } else { Motion::Left };
                    self.move_h(sh, m, shift);
                }
            }
            v if v == VK_RIGHT.0 => {
                if live && !shift && self.at_block_edge(true) {
                    self.live_adjacent(sh, false, None);
                } else {
                    let m = if ctrl {
                        Motion::WordRight
                    } else {
                        Motion::Right
                    };
                    self.move_h(sh, m, shift);
                }
            }
            v if v == VK_UP.0 && !ctrl => self.edit_vertical(sh, -pitch, shift),
            v if v == VK_DOWN.0 && !ctrl => self.edit_vertical(sh, pitch, shift),
            v if v == VK_PRIOR.0 => self.edit_vertical(sh, -page, shift),
            v if v == VK_NEXT.0 => self.edit_vertical(sh, page, shift),
            v if v == VK_HOME.0 && ctrl => match self.active_range() {
                Some(r) if live => self.move_to_pos(sh, r.start, shift),
                _ => self.move_h(sh, Motion::DocStart, shift),
            },
            v if v == VK_END.0 && ctrl => match self.active_range() {
                Some(r) if live => self.move_to_pos(sh, r.end, shift),
                _ => self.move_h(sh, Motion::DocEnd, shift),
            },
            v if v == VK_HOME.0 => self.edit_home_end(sh, false, shift),
            v if v == VK_END.0 => self.edit_home_end(sh, true, shift),
            0x08 => {
                let t = now_ms();
                self.with_ed(sh, |ed| {
                    if ctrl && ed.selection().is_empty() {
                        ed.motion(Motion::WordLeft, true);
                    }
                    ed.backspace(t);
                });
            }
            0x2E if shift && !ctrl => self.edit_cut(sh),
            0x2E => {
                let t = now_ms();
                self.with_ed(sh, |ed| {
                    if ctrl && ed.selection().is_empty() {
                        ed.motion(Motion::WordRight, true);
                    }
                    ed.delete(t);
                });
            }
            // Prompt 015 §3.5: rich paste (HTML → Markdown, bitmaps → assets).
            0x2D if shift && !ctrl => self.paste_rich(sh),
            0x2D if ctrl => self.edit_copy(),
            v if v == VK_RETURN.0 && ctrl => self.with_ed(sh, md::toggle_task),
            v if v == VK_RETURN.0 && !shift => self.f2_enter(sh),
            v if v == VK_RETURN.0 => self.with_ed(sh, md::enter),
            v if v == VK_TAB.0 && !ctrl => {
                let unit = self.edit().map_or("  ", |e| e.opts.indent);
                self.with_ed(sh, |ed| md::tab(ed, unit, shift));
            }
            0x41 if ctrl => {
                let r = self.active_range().filter(|_| live);
                if let Some(e) = self.edit_mut() {
                    match r {
                        Some(r) => e.ed.set_selection(r.start, r.end),
                        None => e.ed.select_all(),
                    }
                }
                self.after_edit(sh, false);
            }
            0x43 if ctrl && !shift => self.edit_copy(),
            0x58 if ctrl && shift => self.with_ed(sh, |ed| md::toggle_wrap(ed, md::Wrap::Strike)),
            0x58 if ctrl => self.edit_cut(sh),
            0x56 if ctrl && !shift => self.paste_rich(sh),
            0x56 if ctrl => self.edit_paste(sh),
            0x42 if ctrl => self.with_ed(sh, |ed| md::toggle_wrap(ed, md::Wrap::Bold)),
            0x49 if ctrl => self.with_ed(sh, |ed| md::toggle_wrap(ed, md::Wrap::Italic)),
            0xC0 if ctrl => self.with_ed(sh, |ed| md::toggle_wrap(ed, md::Wrap::Code)),
            0x4B if ctrl => self.with_ed(sh, md::link),
            0x31..=0x36 if ctrl && !shift => {
                let n = u8::try_from(vk - 0x30).unwrap_or(1);
                self.with_ed(sh, |ed| md::line_format(ed, md::LineFormat::Heading(n)));
            }
            0x30 if ctrl && shift => {
                self.with_ed(sh, |ed| md::line_format(ed, md::LineFormat::Paragraph));
            }
            0x51 if ctrl && shift => {
                self.with_ed(sh, |ed| md::line_format(ed, md::LineFormat::Quote));
            }
            0x38 if ctrl && shift => {
                self.with_ed(sh, |ed| md::line_format(ed, md::LineFormat::Bullet));
            }
            0x37 if ctrl && shift => {
                self.with_ed(sh, |ed| md::line_format(ed, md::LineFormat::Numbered));
            }
            0x39 if ctrl && shift => {
                self.with_ed(sh, |ed| md::line_format(ed, md::LineFormat::Task));
            }
            // Find, open, close, zoom … stay global.
            _ if ctrl => return None,
            // F1, F4, F5 …, menu key.
            0x70..=0x7B | 0x5D => return None,
            _ => {}
        }
        Some(None)
    }

    pub(super) fn edit_copy(&mut self) {
        let Some(e) = self.edit() else { return };
        let t = e.ed.selected_text();
        if !t.is_empty() {
            sys::copy_text(self.hwnd, &t);
        }
    }

    fn edit_cut(&mut self, sh: &Shared) {
        let hwnd = self.hwnd;
        self.with_ed(sh, |ed| {
            if !ed.selection().is_empty() {
                let t = ed.cut();
                sys::copy_text(hwnd, &t);
            }
        });
    }

    /// Plain-text paste (Ctrl+V and Ctrl+Shift+V alike: no HTML conversion in V1).
    pub(super) fn edit_paste(&mut self, sh: &Shared) {
        let Some(t) = sys::paste_text(self.hwnd) else {
            return;
        };
        self.with_ed(sh, |ed| ed.paste(&t));
        self.f2_paste_split(sh);
    }

    /// Context-menu commands of the edit modes.
    pub(super) fn edit_command(&mut self, sh: &Shared, id: u32) {
        match id {
            app::LIVE_EDIT => self.edit_toggle(sh, Mode::Live),
            app::SPLIT_EDIT => self.edit_toggle(sh, Mode::Split),
            app::UNDO => self.with_ed(sh, |ed| {
                ed.undo();
            }),
            app::REDO => self.with_ed(sh, |ed| {
                ed.redo();
            }),
            app::CUT => self.edit_cut(sh),
            app::PASTE => self.paste_rich(sh),
            i if (app::FMT0..app::FMT0 + 13).contains(&i) => {
                if !self.typing() {
                    return;
                }
                let k = i - app::FMT0;
                self.with_ed(sh, |ed| match k {
                    0 => md::toggle_wrap(ed, md::Wrap::Bold),
                    1 => md::toggle_wrap(ed, md::Wrap::Italic),
                    2 => md::toggle_wrap(ed, md::Wrap::Strike),
                    3 => md::toggle_wrap(ed, md::Wrap::Code),
                    4 => md::link(ed),
                    5..=7 => md::line_format(
                        ed,
                        md::LineFormat::Heading(u8::try_from(k - 4).unwrap_or(1)),
                    ),
                    8 => md::line_format(ed, md::LineFormat::Paragraph),
                    9 => md::line_format(ed, md::LineFormat::Quote),
                    10 => md::line_format(ed, md::LineFormat::Bullet),
                    11 => md::line_format(ed, md::LineFormat::Numbered),
                    _ => md::line_format(ed, md::LineFormat::Task),
                });
            }
            _ => {}
        }
    }

    // ---- task checkbox -----------------------------------------------------------------------

    /// Task checkbox click (F2, or read mode with `reading.task_toggle_in_read_mode`): one
    /// single-character edit of the source; saved like any other edit.
    pub(super) fn task_click(&mut self, sh: &Shared, item: u32) -> bool {
        let in_edit = self.edit_mode() == Some(Mode::Live);
        if !in_edit && app::get("reading.task_toggle_in_read_mode") != "true" {
            return false;
        }
        if self.edit_blocked().is_some() {
            return false;
        }
        self.edit_leave_block(sh);
        let edit = {
            let View::Doc(d) = &mut self.view else {
                return false;
            };
            let src = d.content.source();
            let start = item as usize;
            let line_end = src
                .get(start..)
                .and_then(|s| s.find('\n'))
                .map_or(src.len(), |n| start + n);
            let Some(b) = src.get(start..line_end).and_then(|l| l.find('[')) else {
                return false;
            };
            let Some((r, ch)) = md::checkbox_edit(src, start + b) else {
                return false;
            };
            if d.edit.is_none() {
                d.edit = Some(Box::new(Edit::new(src, d.facts.eol)));
            }
            (r, ch)
        };
        let (r, ch) = edit;
        if let Some(e) = self.edit_mut() {
            let sel = (e.ed.anchor(), e.ed.caret());
            e.ed.seal();
            e.ed.replace_select(r, ch, sel);
            e.ed.seal();
        }
        self.after_edit(sh, true);
        // Read mode (and F2 between blocks) saves at once when autosave is on.
        if self.edit().is_some_and(|e| e.opts.autosave) {
            self.edit_save(sh, false, false);
            if self.edit_mode().is_none()
                && let View::Doc(d) = &mut self.view
                && d.edit.as_ref().is_some_and(|e| !e.ed.is_dirty())
            {
                d.edit = None;
            }
        }
        self.edit_title();
        true
    }

    // ---- saving ----------------------------------------------------------------------------

    /// Saves the buffer (S-FILE-03 atomic replace) keeping encoding, BOM, line endings and the
    /// final newline. `force` skips the conflict check; `utf8` converts the encoding.
    pub(super) fn edit_save(&mut self, sh: &Shared, force: bool, utf8: bool) -> bool {
        let (path, text, facts) = {
            let View::Doc(d) = &self.view else {
                return false;
            };
            let Some(e) = d.edit.as_deref() else {
                return false;
            };
            if !e.ed.is_dirty() && !force && !utf8 {
                return true;
            }
            (d.path.clone(), e.ed.text(), d.facts.clone())
        };
        self.kill(TIMER_AUTOSAVE);
        let (enc, bom) = if utf8 {
            ("UTF-8", facts.bom && facts.encoding == "UTF-8")
        } else {
            (facts.encoding, facts.bom)
        };
        let bytes = match encode::encode(&text, enc, bom) {
            Ok(b) => b,
            Err(EncodeError::Unmappable | EncodeError::Unknown) => {
                if self.dialog.is_none() {
                    self.ask(
                        sh,
                        Msg::UnmappableTitle,
                        Msg::UnmappableBody,
                        vec![
                            (Msg::Cancel, BTN_CANCEL),
                            (Msg::SaveMineAs, BTN_SAVE_AS),
                            (Msg::ConvertSave, BTN_CONVERT),
                        ],
                        Pending::Edit(Ask::Unmappable),
                    );
                }
                return false;
            }
        };
        let long = ypath::long_form(&path);
        if !force
            && let Some(st) = facts.stamp
            && let Ok((info, _)) = sys::stat(&long)
            && st.changed(info.size, ticks(info.mtime), || std::fs::read(&long).ok())
        {
            self.ask_conflict(sh);
            return false;
        }
        if let Err(e) = crate::shell::save_document(&long, &bytes) {
            crate::log::warn("save", &format!("{e}"));
            self.toast(Msg::SaveFailed);
            self.journal_save_failed();
            return false;
        }
        self.journal_clear();
        let mtime = sys::stat(&long).map_or(0, |(i, _)| ticks(i.mtime));
        if let View::Doc(d) = &mut self.view {
            d.facts.stamp = Some(Stamp::of(&bytes, mtime));
            d.facts.encoding = enc;
            d.facts.bom = bom;
            d.encoding = enc;
            if let Some(e) = d.edit.as_deref_mut() {
                e.ed.mark_saved();
            }
        }
        self.edit_title();
        true
    }

    fn ask_conflict(&mut self, sh: &Shared) {
        // FUT-34: a banner above the content instead of a modal dialog.
        self.tz_banner(sh, yamdv_ui::transient::BannerKind::Conflict, true);
    }

    /// Answer of an editing dialog.
    pub(super) fn edit_dialog(&mut self, sh: &Shared, a: Ask, id: u32) {
        match (a, id) {
            (Ask::Blocked(m), BTN_CONVERT) => {
                if let View::Doc(d) = &mut self.view {
                    d.facts.lossy = false;
                    d.facts.encoding = "UTF-8";
                    d.facts.bom = false;
                }
                self.edit_enter(sh, m);
            }
            (Ask::Blocked(m), BTN_SAVE_AS) => {
                if let View::Doc(d) = &mut self.view {
                    d.facts.after_save_as = Some(m);
                }
                self.actions.push(Action::SaveAsDialog);
            }
            (Ask::Conflict, BTN_OVERWRITE) => {
                self.edit_save(sh, true, false);
            }
            (Ask::Conflict, BTN_RELOAD_DISCARD) => self.edit_discard_reload(sh),
            (Ask::Conflict | Ask::Unmappable, BTN_SAVE_AS) => {
                let m = self.edit_mode();
                if let View::Doc(d) = &mut self.view {
                    d.facts.after_save_as = m;
                }
                self.actions.push(Action::SaveAsDialog);
            }
            (Ask::Unmappable, BTN_CONVERT) => {
                self.edit_save(sh, false, true);
            }
            (Ask::Unsaved, BTN_SAVE) => {
                if self.edit_save(sh, false, false) {
                    self.close_now();
                }
            }
            (Ask::Unsaved, BTN_DISCARD) => {
                self.journal_clear();
                if let View::Doc(d) = &mut self.view {
                    d.edit = None;
                }
                self.close_now();
            }
            _ => {}
        }
    }

    fn close_now(&mut self) {
        if let Some(e) = self.edit_mut() {
            e.close_ok = true;
        }
        // SAFETY: valid hwnd.
        let _ = unsafe { PostMessageW(Some(self.hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
    }

    /// "Reload (discard mine)": the file's current text replaces the buffer.
    fn edit_discard_reload(&mut self, sh: &Shared) {
        self.journal_clear();
        let mode = self.edit_mode();
        if let View::Doc(d) = &mut self.view {
            d.edit = None;
            d.viewer.source_block = None;
        }
        self.reload(sh);
        if let Some(m) = mode {
            self.edit_enter(sh, m);
        }
    }

    /// Dialog-host answer of "Save … as…": writes the buffer (or the file text) to `path` in
    /// the original encoding (UTF-8 when it cannot be represented), then opens it in place.
    pub(super) fn edit_save_as(&mut self, sh: &Shared, path: &str) {
        let (text, facts) = {
            let View::Doc(d) = &self.view else { return };
            let t = d
                .edit
                .as_deref()
                .map_or_else(|| d.content.source().to_string(), |e| e.ed.text());
            (t, d.facts.clone())
        };
        let (enc, bom) = if facts.lossy {
            ("UTF-8", false)
        } else {
            (facts.encoding, facts.bom)
        };
        let bytes = encode::encode(&text, enc, bom)
            .or_else(|_| encode::encode(&text, "UTF-8", false))
            .unwrap_or_default();
        let Some(full) = sys::full_path(path) else {
            self.toast(Msg::SaveFailed);
            return;
        };
        if crate::shell::save_document(&ypath::long_form(&full), &bytes).is_err() {
            self.toast(Msg::SaveFailed);
            return;
        }
        if let View::Doc(d) = &mut self.view {
            d.edit = None;
            d.viewer.source_block = None;
        }
        self.open(sh, &full, None, false);
        if let Some(m) = facts.after_save_as {
            self.edit_toggle(sh, m);
        }
    }

    /// Watcher notification while an editor exists: own saves are ignored (stamp match), a
    /// clean buffer follows the file, a dirty one asks. True when handled.
    pub(super) fn edit_file_changed(&mut self, sh: &Shared) -> bool {
        let (long, stamp, dirty, mode) = {
            let View::Doc(d) = &self.view else {
                return false;
            };
            let Some(e) = d.edit.as_deref() else {
                return false;
            };
            (
                ypath::long_form(&d.path),
                d.facts.stamp,
                e.ed.is_dirty(),
                e.mode,
            )
        };
        let Ok((info, _)) = sys::stat(&long) else {
            // Deleted: keep the buffer (a save recreates the file).
            return true;
        };
        let changed = stamp.is_none_or(|st| {
            st.changed(info.size, ticks(info.mtime), || std::fs::read(&long).ok())
        });
        if !changed {
            return true;
        }
        if dirty {
            self.ask_conflict(sh);
        } else {
            if let View::Doc(d) = &mut self.view {
                d.edit = None;
                d.viewer.source_block = None;
            }
            self.reload(sh);
            if let Some(m) = mode {
                self.edit_enter(sh, m);
            }
        }
        true
    }

    /// `reload` / `restyle` with an editor: rebuild the views from the buffer (never from
    /// disk), keeping mode, caret and reading position. True when handled.
    pub(super) fn edit_rebuild(&mut self, sh: &Shared) -> bool {
        let View::Doc(d) = &mut self.view else {
            return false;
        };
        let Some(mut e) = d.edit.take() else {
            return false;
        };
        let (block, off) = d.viewer.anchor();
        let path = d.path.clone();
        let facts = d.facts.clone();
        let (info, internet, encoding) = (d.info.clone(), d.internet, d.encoding);
        e.active = None;
        let mode = e.mode.take();
        e.pane = None;
        e.synced = e.ed.version;
        let text = e.ed.text();
        self.show_text(sh, path, text, false, false);
        if let View::Doc(d) = &mut self.view {
            d.facts = facts;
            d.info = info;
            d.internet = internet;
            d.encoding = encoding;
            d.edit = Some(e);
            let y = d.viewer.block_top(&d.content, block) + off;
            d.viewer.scroll_to(y, f64::from(self.ch));
        }
        match mode {
            Some(m) => self.edit_enter(sh, m),
            None => self.edit_title(),
        }
        true
    }

    /// Before opening another file in this window: saves when autosave is on; refuses with
    /// autosave off and unsaved changes. True = may proceed.
    pub(super) fn edit_release(&mut self, sh: &Shared) -> bool {
        let Some(e) = self.edit() else { return true };
        if !e.ed.is_dirty() {
            return true;
        }
        if e.opts.autosave {
            self.edit_leave_block(sh);
            return self.edit_save(sh, false, false);
        }
        self.toast(Msg::UnsavedTitle);
        false
    }

    // ---- caret blink / IME ----------------------------------------------------------------

    fn blink_restart(&mut self) {
        if self.edit_mode().is_some() {
            self.timer_once(TIMER_CARET, BLINK_MS);
        }
    }

    fn ime_position(&self, sh: &Shared) {
        let Some(r) = self.caret_rect(sh) else { return };
        let s = self.scale;
        let pt = POINT {
            x: (r.x * s) as i32,
            y: ((r.y + self.top) * s) as i32,
        };
        // SAFETY: valid hwnd; the context is released below.
        unsafe {
            let himc = ImmGetContext(self.hwnd);
            if himc.is_invalid() {
                return;
            }
            let cf = COMPOSITIONFORM {
                dwStyle: CFS_POINT,
                ptCurrentPos: pt,
                rcArea: RECT::default(),
            };
            let _ = ImmSetCompositionWindow(himc, &cf);
            let cand = CANDIDATEFORM {
                dwIndex: 0,
                dwStyle: CFS_EXCLUDE,
                ptCurrentPos: pt,
                rcArea: RECT {
                    left: pt.x,
                    top: pt.y,
                    right: pt.x + 2,
                    bottom: pt.y + (r.h * s) as i32,
                },
            };
            let _ = ImmSetCandidateWindow(himc, &cand);
            let _ = ImmReleaseContext(self.hwnd, himc);
        }
    }

    fn ime_string(&self, kind: windows::Win32::UI::Input::Ime::IME_COMPOSITION_STRING) -> String {
        // SAFETY: valid hwnd; the buffer size comes from the first call; released below.
        unsafe {
            let himc = ImmGetContext(self.hwnd);
            if himc.is_invalid() {
                return String::new();
            }
            let n = ImmGetCompositionStringW(himc, kind, None, 0);
            let mut out = String::new();
            if n > 0 {
                let mut buf = vec![0u16; (n as usize).div_ceil(2)];
                let got = ImmGetCompositionStringW(
                    himc,
                    kind,
                    Some(buf.as_mut_ptr().cast()),
                    u32::try_from(buf.len() * 2).unwrap_or(0),
                );
                if got > 0 {
                    buf.truncate(got as usize / 2);
                    out = String::from_utf16_lossy(&buf);
                }
            }
            let _ = ImmReleaseContext(self.hwnd, himc);
            out
        }
    }

    // ---- mouse -----------------------------------------------------------------------------

    fn pane_hit(&self, sh: &Shared, x: f32, y: f32) -> Option<usize> {
        let g = self.pane_geo(sh)?;
        let e = self.edit()?;
        let p = e.pane.as_ref()?;
        let doc_y = p.viewer.scroll + f64::from(y);
        if doc_y > p.viewer.total_height() - 1.0 {
            return Some(e.ed.len());
        }
        p.viewer
            .hit(&p.content, x - g.text_x + p.hx, doc_y.max(0.0))
            .map(|h| h.source as usize)
    }

    fn click_select(&mut self, pos: usize, extend: bool, clicks: u32) {
        let Some(e) = self.edit_mut() else { return };
        match clicks {
            2 => {
                let r = e.ed.word_at(pos);
                e.ed.set_selection(r.start, r.end);
            }
            n if n >= 3 => {
                let r = e.ed.line_range_at(pos);
                e.ed.set_selection(r.start, r.end);
            }
            _ => e.ed.move_to(pos, extend),
        }
        e.goal_x = None;
    }

    fn count_clicks(&mut self, x: f32, y: f32) -> u32 {
        // SAFETY: no preconditions.
        let (t, dx, dy, dbl) = unsafe {
            (
                GetMessageTime(),
                GetSystemMetrics(SM_CXDOUBLECLK),
                GetSystemMetrics(SM_CYDOUBLECLK),
                GetDoubleClickTime() as i32,
            )
        };
        let (n, lt, lx, ly) = self.clicks;
        let (px, py) = ((x * self.scale) as i32, (y * self.scale) as i32);
        let n = if t.wrapping_sub(lt) <= dbl && (px - lx).abs() <= dx && (py - ly).abs() <= dy {
            n + 1
        } else {
            1
        };
        self.clicks = (n, t, px, py);
        n
    }

    fn capture(&self) {
        // SAFETY: valid hwnd.
        unsafe { SetCapture(self.hwnd) };
    }

    fn set_drag(&mut self, d: Drag, y: f32) {
        if let Some(e) = self.edit_mut() {
            e.drag = Some(d);
            e.drag_y = y;
        }
        self.capture();
    }

    /// Mouse down in an edit mode; `true` = handled.
    pub(super) fn edit_mouse_down(&mut self, sh: &Shared, x: f32, y: f32, ctrl: bool) -> bool {
        let Some(mode) = self.edit_mode() else {
            return false;
        };
        if self.dialog.is_some()
            || (mode == Mode::Live && self.over_side(sh, x, y))
            || (self.find.is_some()
                && findbar::bar_rects(self.cw, self.find_tall())
                    .0
                    .contains(x, y))
        {
            return false;
        }
        let shift = key_down(VK_SHIFT.0);
        if let Some((pr, prev, sp)) = self.split_rects() {
            if (x - (sp.x + sp.w / 2.0)).abs() <= 4.0 {
                self.set_drag(Drag::Splitter, y);
                return true;
            }
            if pr.contains(x, y) {
                if let Some(g) = self.pane_geo(sh)
                    && Self::help_rect(&g).contains(x, y)
                {
                    self.actions.push(Action::Help);
                    return true;
                }
                let n = self.count_clicks(x, y);
                if let Some(pos) = self.pane_hit(sh, x, y) {
                    self.click_select(pos, shift, n);
                }
                self.set_drag(Drag::Pane, y);
                self.after_edit(sh, false);
                return true;
            }
            if !prev.contains(x, y)
                || ctrl
                || (x >= prev.x + prev.w - 16.0 && self.thumb().is_some())
                || self.over_hot(sh, x, y)
            {
                return false;
            }
            // Click in the preview puts the caret at the corresponding source position.
            self.sync_preview(sh);
            let (cx, _) = self.col(sh);
            let pos = {
                let View::Doc(d) = &self.view else {
                    return false;
                };
                d.viewer
                    .hit(&d.content, x - cx, f64::from(y) + d.viewer.scroll)
                    .map(|h| h.source as usize)
            };
            if let Some(pos) = pos {
                let n = self.count_clicks(x, y);
                self.click_select(pos, shift, n);
                self.set_drag(Drag::Preview, y);
                self.after_edit(sh, false);
            }
            return true;
        }
        // F2.
        if x >= self.cw - 16.0 && self.thumb().is_some() {
            return false;
        }
        let (cx, _) = self.col(sh);
        let hot = {
            let View::Doc(d) = &self.view else {
                return false;
            };
            d.viewer
                .hot_at(&d.content, x - cx, f64::from(y) + d.viewer.scroll)
                .map(|(_, k)| k)
        };
        match hot {
            Some(HotKind::Task { item }) => return self.task_click(sh, item),
            Some(
                HotKind::CodeCopy { .. }
                | HotKind::CodeLines { .. }
                | HotKind::CodeWrap { .. }
                | HotKind::Fold { .. },
            ) => return false,
            _ if ctrl => return false,
            _ => {}
        }
        // Inside the active block: caret / selection.
        if let Some((ox, oy, p)) = self.live_para(sh)
            && y >= oy - 8.0
            && y <= oy + p.shaped.height() + 8.0
        {
            let idx = p.shaped.hit(x - ox, y - oy);
            let pos = p.map.as_ref().map_or(0, |m| m.to_source(idx)) as usize;
            let n = self.count_clicks(x, y);
            self.click_select(pos, shift, n);
            self.set_drag(Drag::Preview, y);
            self.after_edit(sh, false);
            return true;
        }
        self.count_clicks(x, y);
        if shift && let Some(pos) = self.doc_hit(sh, x, y) {
            self.xsel_extend(sh, pos);
            self.set_drag(Drag::Preview, y);
            return true;
        }
        self.live_click(sh, x, y);
        self.edit_title();
        true
    }

    fn edit_mouse_move(&mut self, sh: &Shared, x: f32, y: f32) -> bool {
        let Some(drag) = self.edit().and_then(|e| e.drag) else {
            return false;
        };
        match drag {
            Drag::Splitter => {
                let swapped = self.edit().is_some_and(|e| e.opts.swapped);
                let avail = (self.cw - SPLITTER).max(2.0);
                let mut r = x / avail;
                if swapped {
                    r = 1.0 - r;
                }
                if let Some(e) = self.edit_mut() {
                    e.opts.ratio = r.clamp(0.2, 0.8);
                }
                update_size(self, sh);
            }
            Drag::Pane => {
                if let Some(e) = self.edit_mut() {
                    e.drag_y = y;
                }
                if y < 0.0 || y > self.ch {
                    self.timer_once(TIMER_EDRAG, 30);
                }
                if let Some(pos) = self.pane_hit(sh, x, y.clamp(0.0, self.ch - 1.0)) {
                    self.move_to_pos(sh, pos, true);
                }
            }
            Drag::Preview => {
                let pos = match self.edit_mode() {
                    Some(Mode::Live) => {
                        let inside = self.live_para(sh).and_then(|(ox, oy, p)| {
                            (y >= oy - 8.0 && y <= oy + p.shaped.height() + 8.0).then(|| {
                                let idx = p.shaped.hit(x - ox, y - oy);
                                p.map.as_ref().map_or(0, |m| m.to_source(idx)) as usize
                            })
                        });
                        if inside.is_none()
                            && let Some(pos) = self.doc_hit(sh, x, y)
                        {
                            self.xsel_extend(sh, pos);
                            return true;
                        }
                        inside
                    }
                    _ => {
                        let (cx, _) = self.col(sh);
                        let View::Doc(d) = &self.view else {
                            return true;
                        };
                        d.viewer
                            .hit(&d.content, x - cx, f64::from(y) + d.viewer.scroll)
                            .map(|h| h.source as usize)
                    }
                };
                if let Some(mut pos) = pos {
                    if let Some(r) = self.active_range() {
                        pos = pos.clamp(r.start, r.end);
                    }
                    self.move_to_pos(sh, pos, true);
                }
            }
        }
        true
    }

    pub(super) fn edit_mouse_up(&mut self) -> bool {
        let Some(drag) = self.edit().and_then(|e| e.drag) else {
            return false;
        };
        if let Some(e) = self.edit_mut() {
            e.drag = None;
        }
        // SAFETY: no preconditions.
        let _ = unsafe { ReleaseCapture() };
        self.kill(TIMER_EDRAG);
        if drag == Drag::Splitter {
            let r = self.edit().map_or(0.5, |e| e.opts.ratio);
            self.actions
                .push(Action::SetSetting("editing.split_ratio", format!("{r:.2}")));
        }
        true
    }

    fn edit_wheel(&mut self, sh: &Shared, delta: i32, x: f32, y: f32) -> bool {
        let Some((pr, _, _)) = self.split_rects() else {
            return false;
        };
        if !pr.contains(x, y) {
            if let Some(e) = self.edit_mut() {
                e.follow = Follow::Preview;
            }
            return false;
        }
        let vh = self.view_h();
        let step = f64::from(delta) / 120.0 * f64::from(sh.m.pitch()) * 3.0;
        if let Some(e) = self.edit_mut()
            && let Some(p) = e.pane.as_mut()
        {
            let s = p.viewer.scroll - step;
            p.viewer.scroll_to(s, vh);
        }
        self.prep_pane(sh);
        self.invalidate_pane();
        self.sync_preview_scroll();
        true
    }

    // ---- message hook ------------------------------------------------------------------------

    fn timer_msg(&mut self, sh: &Shared, id: usize) -> Option<LRESULT> {
        match id {
            TIMER_AUTOSAVE => {
                self.kill(TIMER_AUTOSAVE);
                if self
                    .edit()
                    .is_some_and(|e| e.opts.autosave && e.mode.is_some())
                {
                    self.edit_save(sh, false, false);
                }
            }
            plus::TIMER_JOURNAL => self.journal_tick(),
            super::wysi::TIMER_F2_PREVIEW => self.f2_preview_tick(sh),
            TIMER_PREVIEW => {
                self.kill(TIMER_PREVIEW);
                self.sync_preview(sh);
            }
            TIMER_CARET => {
                let idle = self
                    .edit()
                    .map_or(f64::MAX, |e| e.last_input.elapsed().as_secs_f64());
                if idle >= BLINK_STOP || self.edit_mode().is_none() {
                    self.kill(TIMER_CARET);
                    if let Some(e) = self.edit_mut() {
                        e.blink_on = true;
                    }
                } else {
                    if let Some(e) = self.edit_mut() {
                        e.blink_on = !e.blink_on;
                    }
                    self.timer_once(TIMER_CARET, BLINK_MS);
                }
                if let Some(r) = self.caret_rect(sh) {
                    self.invalidate_rect(Rect::new(r.x - 2.0, r.y, r.w + 4.0, r.h));
                }
            }
            TIMER_EDRAG => {
                let y = self.edit().map_or(0.0, |e| e.drag_y);
                if self.edit().and_then(|e| e.drag) != Some(Drag::Pane)
                    || (0.0..=self.ch).contains(&y)
                {
                    self.kill(TIMER_EDRAG);
                    return Some(LRESULT(0));
                }
                let vh = self.view_h();
                let dy = if y < 0.0 { y } else { y - self.ch };
                if let Some(e) = self.edit_mut()
                    && let Some(p) = e.pane.as_mut()
                {
                    let s = p.viewer.scroll + f64::from(dy.clamp(-60.0, 60.0));
                    p.viewer.scroll_to(s, vh);
                }
                self.prep_pane(sh);
                let x = self.pane_geo(sh).map_or(0.0, |g| g.text_x);
                if let Some(pos) = self.pane_hit(sh, x, y.clamp(0.0, self.ch - 1.0))
                    && let Some(e) = self.edit_mut()
                {
                    e.ed.move_to(pos, true);
                }
                self.invalidate_pane();
                self.sync_preview_scroll();
                self.timer_once(TIMER_EDRAG, 30);
            }
            _ => return None,
        }
        Some(LRESULT(0))
    }

    fn cursor_msg(&self, sh: &Shared) -> Option<LRESULT> {
        let mut pt = POINT::default();
        // SAFETY: valid out-pointer.
        let _ = unsafe { GetCursorPos(&mut pt) };
        let (x, y) = self.screen_xy(pt.x, pt.y);
        if y < 0.0 {
            return None;
        }
        let cur = if let Some((pr, _, sp)) = self.split_rects() {
            if (x - (sp.x + sp.w / 2.0)).abs() <= 4.0 {
                Some(IDC_SIZEWE)
            } else if pr.contains(x, y) {
                let help = self
                    .pane_geo(sh)
                    .is_some_and(|g| Self::help_rect(&g).contains(x, y));
                Some(if help { IDC_HAND } else { IDC_IBEAM })
            } else {
                None
            }
        } else if self
            .live_para(sh)
            .is_some_and(|(_, oy, p)| y >= oy && y <= oy + p.shaped.height())
        {
            Some(IDC_IBEAM)
        } else {
            None
        }?;
        // SAFETY: system cursors; no cleanup needed.
        unsafe { SetCursor(LoadCursorW(None, cur).ok()) };
        Some(LRESULT(1))
    }

    fn close_msg(&mut self, sh: &Shared) -> Option<LRESULT> {
        let (dirty, autosave, ok) = self.edit().map_or((false, false, true), |e| {
            (e.ed.is_dirty(), e.opts.autosave, e.close_ok)
        });
        if !dirty {
            self.journal_clear();
            return None;
        }
        if ok {
            return None;
        }
        self.edit_leave_block(sh);
        if autosave {
            return if self.edit_save(sh, false, false) {
                None
            } else {
                Some(LRESULT(0))
            };
        }
        if self.dialog.is_none() {
            self.ask(
                sh,
                Msg::UnsavedTitle,
                Msg::UnsavedBody,
                vec![
                    (Msg::Cancel, BTN_CANCEL),
                    (Msg::Discard, BTN_DISCARD),
                    (Msg::Save, BTN_SAVE),
                ],
                Pending::Edit(Ask::Unsaved),
            );
        }
        Some(LRESULT(0))
    }

    fn ime_msg(&mut self, sh: &Shared, flags: u32) -> Option<LRESULT> {
        if flags & GCS_RESULTSTR.0 != 0 {
            let s = self.ime_string(GCS_RESULTSTR);
            if let Some(e) = self.edit_mut() {
                e.ime.clear();
            }
            self.edit_type(sh, &s);
        }
        if flags & GCS_COMPSTR.0 != 0 {
            let s = self.ime_string(GCS_COMPSTR);
            if let Some(e) = self.edit_mut() {
                e.ime = s;
            }
            self.ime_position(sh);
            self.invalidate();
        }
        Some(LRESULT(0))
    }

    /// Editing part of `handle`: `Some` = consumed.
    pub(super) fn edit_msg(
        &mut self,
        sh: &Shared,
        msg: u32,
        wp: WPARAM,
        lp: LPARAM,
    ) -> Option<LRESULT> {
        match msg {
            WM_CLOSE => self.close_msg(sh),
            WM_ACTIVATE if wp.0 & 0xFFFF == 0 => {
                if self
                    .edit()
                    .is_some_and(|e| e.mode.is_some() && e.opts.autosave && e.ed.is_dirty())
                {
                    self.edit_leave_block(sh);
                    self.edit_save(sh, false, false);
                }
                None
            }
            WM_CHAR
                if self.find.is_none()
                    && self.dialog.is_none()
                    && (self.typing() || self.xsel().is_some()) =>
            {
                self.edit_char(sh, wp.0 as u16);
                Some(LRESULT(0))
            }
            WM_IME_STARTCOMPOSITION if self.typing() => {
                self.ime_position(sh);
                Some(LRESULT(0))
            }
            WM_IME_COMPOSITION if self.typing() => self.ime_msg(sh, lp.0 as u32),
            WM_IME_ENDCOMPOSITION if self.typing() => {
                if let Some(e) = self.edit_mut() {
                    e.ime.clear();
                }
                self.invalidate();
                Some(LRESULT(0))
            }
            WM_LBUTTONDOWN => {
                let (x, y) = self.lp_xy(lp);
                if y < 0.0 || self.fs && self.cap.reveal && y < yamdv_core::chrome::BAR_H {
                    return None;
                }
                self.edit_mouse_down(sh, x, y, wp.0 & MK_CONTROL_BIT != 0)
                    .then_some(LRESULT(0))
            }
            WM_MOUSEMOVE => {
                let (x, y) = self.lp_xy(lp);
                if y < 0.0 && !self.selecting {
                    return None;
                }
                self.edit_mouse_move(sh, x, y).then_some(LRESULT(0))
            }
            WM_LBUTTONUP => self.edit_mouse_up().then_some(LRESULT(0)),
            WM_MOUSEWHEEL if self.edit_mode().is_some() && self.dialog.is_none() => {
                let delta = i32::from((wp.0 >> 16) as u16 as i16);
                let pt = POINT {
                    x: i32::from(lp.0 as u16 as i16),
                    y: i32::from((lp.0 >> 16) as u16 as i16),
                };
                let (x, y) = self.screen_xy(pt.x, pt.y);
                self.edit_wheel(sh, delta, x, y).then_some(LRESULT(0))
            }
            WM_SETCURSOR
                if (lp.0 & 0xFFFF) as u32 == HTCLIENT
                    && self.dialog.is_none()
                    && self.edit_mode().is_some() =>
            {
                self.cursor_msg(sh)
            }
            WM_TIMER => self.timer_msg(sh, wp.0),
            _ => None,
        }
    }

    /// Paint pre-step: preview → editor scroll sync and pane layout.
    pub(super) fn edit_prepare(&mut self, sh: &Shared) {
        if self.edit_mode() != Some(Mode::Split) {
            return;
        }
        self.sync_pane_scroll(sh);
        self.prep_pane(sh);
    }

    /// Paint post-step: T5 latency sample (edit → caret painted).
    pub(super) fn edit_painted(&mut self) {
        if let Some(e) = self.edit_mut()
            && let Some(t) = e.typed_at.take()
            && e.lat.len() < 100_000
        {
            e.lat.push(t.elapsed().as_secs_f64() * 1000.0);
        }
    }

    /// Resize: the pane width follows.
    pub(super) fn edit_resize(&mut self, sh: &Shared) {
        if self.edit_mode() == Some(Mode::Split) {
            self.prep_pane(sh);
        }
    }

    // ---- drawing -----------------------------------------------------------------------------

    /// Draws the editing overlays: the F3 pane (gutter, current line, selection, caret, "?") or
    /// the F2 caret/selection on the active block, plus the IME composition.
    pub(super) fn draw_edit(&self, sh: &Shared, b: &Band<'_>) {
        let Some(e) = self.edit() else { return };
        let pal = &self.pal;
        match e.mode {
            Some(Mode::Split) => {
                let (Some(g), Some(p), Some((_, _, sp))) =
                    (self.pane_geo(sh), e.pane.as_ref(), self.split_rects())
                else {
                    return;
                };
                b.fill(g.r, pal.bg, 1.0, 0.0);
                b.fill(sp, pal.code_bg, 1.0, 0.0);
                b.fill(
                    Rect::new((sp.x + sp.w / 2.0 - 0.5).round(), 0.0, 1.0, sp.h),
                    pal.code_border,
                    1.0,
                    0.0,
                );
                if g.gutter > 0.0 {
                    b.fill(
                        Rect::new(g.r.x, 0.0, g.gutter, g.r.h),
                        pal.code_bg,
                        1.0,
                        0.0,
                    );
                }
                let vh = self.view_h();
                let caret = Self::pane_caret(e);
                b.push_clip(Rect::new(g.r.x + g.gutter, 0.0, g.r.w - g.gutter, g.r.h));
                if let Some((_, y, h)) = caret {
                    let y = (y - p.viewer.scroll) as f32;
                    b.fill(
                        Rect::new(g.r.x + g.gutter, y, g.r.w - g.gutter, h),
                        pal.text,
                        0.05,
                        0.0,
                    );
                }
                let s = e.ed.selection();
                let sel = (!s.is_empty()).then(|| {
                    u32::try_from(s.start).unwrap_or(0)..u32::try_from(s.end).unwrap_or(0)
                });
                let marks = self.find_marks();
                draw_doc(
                    b,
                    &p.content,
                    &p.viewer,
                    pal,
                    g.text_x - p.hx,
                    sel,
                    vh,
                    None,
                    marks,
                );
                if let Some((x, y, h)) = caret {
                    let r = Rect::new(g.text_x - p.hx + x, (y - p.viewer.scroll) as f32, 1.5, h);
                    self.draw_caret(sh, b, e, r);
                }
                b.pop_clip();
                if g.gutter > 0.0 {
                    self.draw_numbers(sh, b, e, p, &g);
                }
                let q = Self::help_rect(&g);
                b.circle(q.x + 14.0, q.y + 14.0, 14.0, pal.code_bg, None);
                b.circle(q.x + 14.0, q.y + 14.0, 14.0, pal.code_border, Some(1.0));
                let t = self.shape_ui(sh, "?", 14.0, 600, pal.text_muted, None);
                b.text(
                    t.as_ref(),
                    (q.x + 14.0 - t.width() / 2.0).round(),
                    (q.y + 14.0 - t.height() / 2.0).round(),
                );
            }
            Some(Mode::Live) => {
                let Some((ox, oy, p)) = self.live_para(sh) else {
                    return;
                };
                let s = e.ed.selection();
                if !s.is_empty() {
                    let sel_alpha = f32::from(pal.selection_alpha_pct) / 100.0;
                    let r = u32::try_from(s.start).unwrap_or(0)..u32::try_from(s.end).unwrap_or(0);
                    for rr in yamdv_layout::viewer::selection_rects(p, &r) {
                        b.fill(rr.offset(ox, oy), pal.selection_color(), sel_alpha, 0.0);
                    }
                }
                if let Some(r) = self.caret_rect(sh) {
                    self.draw_caret(sh, b, e, r);
                }
            }
            None => {}
        }
    }

    fn draw_caret(&self, sh: &Shared, b: &Band<'_>, e: &Edit, r: Rect) {
        let pal = &self.pal;
        if !e.ime.is_empty() {
            let size = if e.opts.code_font {
                sh.m.code_size()
            } else {
                sh.m.font_size
            };
            let t = self.shape_ui(sh, &e.ime, size, 400, pal.text, None);
            let w = t.width() + 2.0;
            b.fill(Rect::new(r.x, r.y, w, r.h), pal.bg, 1.0, 0.0);
            b.text(t.as_ref(), r.x + 1.0, r.y + (r.h - t.height()) / 2.0);
            b.fill(Rect::new(r.x, r.y + r.h - 2.0, w, 1.0), pal.text, 1.0, 0.0);
            return;
        }
        if e.blink_on {
            b.fill(Rect::new(r.x.round(), r.y, r.w, r.h), pal.text, 1.0, 0.0);
        }
    }

    fn draw_number(
        &self,
        sh: &Shared,
        b: &Band<'_>,
        e: &Edit,
        g: &Geo,
        line: usize,
        yh: (f32, f32),
    ) {
        let (y, h) = yh;
        let size = (sh.m.font_size * 0.85).round();
        let mut cache = e.nums.borrow_mut();
        if cache.len() > 2000 {
            cache.clear();
        }
        let t = cache.entry(line).or_insert_with(|| {
            self.shape_ui(sh, &line.to_string(), size, 400, self.pal.text_muted, None)
        });
        b.text(
            t.as_ref(),
            (g.r.x + g.gutter - PANE_PAD / 2.0 - t.width()).round(),
            (y + (h - t.height()) / 2.0).round(),
        );
    }

    fn draw_numbers(&self, sh: &Shared, b: &Band<'_>, e: &Edit, p: &Pane, g: &Geo) {
        let vh = self.view_h();
        for (bi, y, bl) in p.viewer.visible(&p.content, vh) {
            let y = y as f32;
            let Some(para) = bl.paras.first() else {
                continue;
            };
            let Some(map) = &para.map else { continue };
            let start = p.content.range(bi).start as usize;
            let mut line = e.ed.line_of(start) + 1;
            let mut idx16 = 0u32;
            let mut pending = true;
            for ch in map.text.chars() {
                if pending {
                    let r = para.shaped.caret(idx16);
                    let ly = y + para.y + r.y;
                    if ly > self.ch {
                        break;
                    }
                    if ly + r.h >= 0.0 {
                        self.draw_number(sh, b, e, g, line, (ly, r.h));
                    }
                    pending = false;
                }
                idx16 += u32::try_from(ch.len_utf16()).unwrap_or(1);
                if ch == '\n' {
                    line += 1;
                    pending = true;
                }
            }
            if pending {
                let r = para.shaped.caret(idx16);
                let ly = y + para.y + r.y;
                if ly <= self.ch && ly + r.h >= 0.0 {
                    self.draw_number(sh, b, e, g, line, (ly, r.h));
                }
            }
        }
    }

    // ---- find (F3 searches the buffer) -------------------------------------------------------

    /// F3: the buffer text and the pane's top offset (first match at or after it).
    pub(super) fn find_source(&self) -> Option<(String, u32)> {
        let e = self.edit()?;
        if e.mode != Some(Mode::Split) {
            return None;
        }
        let p = e.pane.as_ref()?;
        let top = p
            .viewer
            .hit(&p.content, 0.0, p.viewer.scroll + 2.0)
            .map_or(0, |h| h.source);
        Some((p.content.source().to_string(), top))
    }

    /// F3: selects match `m` in the buffer and scrolls the pane to it. True when handled.
    pub(super) fn find_reveal_edit(&mut self, sh: &Shared, m: Range<u32>) -> bool {
        if self.edit_mode() != Some(Mode::Split) {
            return false;
        }
        if let Some(e) = self.edit_mut() {
            e.ed.set_selection(m.start as usize, m.end as usize);
        }
        self.pane_scroll_to_caret(sh, true);
        self.invalidate();
        true
    }
}
