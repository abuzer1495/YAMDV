//! V2-E editing plus (prompt 015): replace row, Tab snippets, line operations, auto-pair,
//! the F2 format toolbar, rich paste / file drop and the recovery journal. Hooks are called
//! from the key, char, mouse and paint routing of `window.rs` / `edit.rs`.

use super::*;
use yamdv_core::keymap::Chord;
use yamdv_editor::{clear, lines, md, pairs, snippets};

#[cfg(feature = "dev-tools")]
mod drive;
mod findrep;
mod paste;
mod recovery;
mod toolbar;

pub(super) use recovery::{BTN_JOURNAL_DISCARD, BTN_RESTORE, Journal, TIMER_JOURNAL};

/// Per-window V2-E state.
#[derive(Default)]
pub(super) struct Plus {
    /// Active Tab-snippet placeholder session.
    pub snip: Option<snippets::Session>,
    /// The format toolbar was dismissed (Esc) for this selection.
    pub tb_hidden: Option<Range<usize>>,
    /// Toolbar button under the mouse (tooltip).
    pub tb_hover: Option<usize>,
    /// Recovery journal of the open document.
    pub journal: Journal,
}

fn setting(k: &str) -> bool {
    app::get(k) == "true"
}

/// Local date and time for the `date` / `time` snippets.
fn local_now() -> (String, String) {
    use windows::Win32::System::SystemInformation::GetLocalTime;
    // SAFETY: GetLocalTime has no preconditions.
    let t = unsafe { GetLocalTime() };
    (
        format!("{:04}-{:02}-{:02}", t.wYear, t.wMonth, t.wDay),
        format!("{:02}:{:02}", t.wHour, t.wMinute),
    )
}

/// The caret is inside a fenced code block or an inline code span (auto-pair: brackets only).
fn in_code(ed: &yamdv_editor::Editor) -> bool {
    let caret = ed.caret();
    let line = ed.line_of(caret);
    let mut fence = false;
    for l in 0..line {
        let t = ed.slice(ed.line_start(l)..ed.line_end(l));
        let t = t.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = !fence;
        }
    }
    let before = ed.slice(ed.line_start(line)..caret);
    fence || before.matches('`').count() % 2 == 1
}

impl Win {
    /// V2-E keys, ahead of the keymap translation. `Some(_)` = consumed.
    pub(super) fn v2e_key(&mut self, sh: &Shared, c: Chord) -> Option<Option<Action>> {
        if self.dialog.is_some() || self.settings.is_some() || self.nav_modal() {
            return None;
        }
        let edit = self.edit_mode().is_some();
        let act = app::keymap().action_of(c, edit);
        if act == Some("replace") && matches!(self.view, View::Doc(_)) {
            self.replace_open(sh);
            return Some(None);
        }
        if self.find.is_some() {
            return self.findrep_key(sh, c);
        }
        if !self.typing() {
            self.plus.snip = None;
            return None;
        }
        let plain = !c.ctrl && !c.alt;
        if plain && c.vk == VK_ESCAPE.0 {
            if self.plus.snip.take().is_some() {
                return Some(None);
            }
            if self.toolbar_visible(sh) {
                self.plus.tb_hidden = self.edit().map(|e| e.ed.selection());
                self.invalidate();
                return Some(None);
            }
            return None;
        }
        if plain && !c.shift && c.vk == VK_TAB.0 && self.snippet_tab(sh) {
            return Some(None);
        }
        if plain && !c.shift && c.vk == 0x08 && setting("editing.auto_pair") {
            let mut done = false;
            self.with_ed(sh, |ed| done = pairs::backspace(ed));
            if done {
                return Some(None);
            }
        }
        match act? {
            "duplicate_line" => self.with_ed(sh, lines::duplicate),
            "select_line" => self.with_ed(sh, lines::select_line),
            "move_line_up" | "move_line_down" => {
                let up = act == Some("move_line_up");
                self.with_ed(sh, |ed| {
                    lines::move_lines(ed, up);
                });
            }
            "paste_plain" => self.paste_plain(sh),
            "highlight" => self.with_ed(sh, |ed| md::toggle_wrap(ed, md::Wrap::Highlight)),
            _ => return None,
        }
        Some(None)
    }

    /// Tab: next snippet placeholder, or a snippet expansion. True when consumed.
    fn snippet_tab(&mut self, sh: &Shared) -> bool {
        if let Some(mut s) = self.plus.snip.take() {
            let mut more = false;
            let v0 = self.edit().map_or(0, |e| e.ed.version);
            let sel0 = self.edit().map(|e| e.ed.selection());
            if let Some(e) = self.edit_mut() {
                more = s.next(&mut e.ed);
            }
            let moved = self.edit().map(|e| e.ed.selection()) != sel0
                || self.edit().map_or(0, |e| e.ed.version) != v0;
            if more {
                self.plus.snip = Some(s);
            }
            if moved {
                self.after_edit(sh, false);
                return true;
            }
        }
        if !setting("editing.snippets") {
            return false;
        }
        let (d, t) = local_now();
        let mut sess = None;
        let v0 = self.edit().map_or(0, |e| e.ed.version);
        self.with_ed(sh, |ed| sess = snippets::try_expand(ed, (&d, &t)));
        self.plus.snip = sess;
        self.edit().is_some_and(|e| e.ed.version != v0)
    }

    /// Typed character hook (auto-pair). True when the character was handled.
    pub(super) fn plus_char(&mut self, sh: &Shared, c: char) -> bool {
        if !setting("editing.auto_pair") {
            return false;
        }
        let Some(code) = self.edit().map(|e| in_code(&e.ed)) else {
            return false;
        };
        let mut done = false;
        if let Some(e) = self.edit_mut() {
            e.typed_at = Some(Instant::now());
        }
        self.with_ed(sh, |ed| done = pairs::type_char(ed, c, code));
        done
    }

    /// Toolbar command `k` (same order as the context menu's FMT0 block, plus highlight and
    /// clear formatting).
    pub(super) fn format_cmd(&mut self, sh: &Shared, k: toolbar::Cmd) {
        use toolbar::Cmd as T;
        self.with_ed(sh, |ed| match k {
            T::Bold => md::toggle_wrap(ed, md::Wrap::Bold),
            T::Italic => md::toggle_wrap(ed, md::Wrap::Italic),
            T::Strike => md::toggle_wrap(ed, md::Wrap::Strike),
            T::Highlight => md::toggle_wrap(ed, md::Wrap::Highlight),
            T::Code => md::toggle_wrap(ed, md::Wrap::Code),
            T::Link => md::link(ed),
            T::H(n) => md::line_format(ed, md::LineFormat::Heading(n)),
            T::Quote => md::line_format(ed, md::LineFormat::Quote),
            T::Bullet => md::line_format(ed, md::LineFormat::Bullet),
            T::Numbered => md::line_format(ed, md::LineFormat::Numbered),
            T::Task => md::line_format(ed, md::LineFormat::Task),
            T::Clear => clear::clear_format(ed),
        });
    }
}
