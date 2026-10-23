//! V2-D navigation (prompt 014): Files tab (FUT-33), floating sidebar card (FUT-02), favorites
//! reorder (FUT-03), heading palette and `[TOC]` (FUT-08), mind map (FUT-37) and keyboard reach
//! (HARD-04). Hooked into the window message handler by [`Win::nav_msg`]; everything heavy is
//! built on demand and freed on close.

use super::*;

mod favs;
mod files;
mod focus;
mod mindmap;
mod palette;
#[cfg(feature = "dev-tools")]
mod probe;

pub(super) use files::Files;
pub(super) use focus::ring;

/// Folder listings are ready (worker → window).
pub(super) const WM_APP_FILES: u32 = WM_APP + 0x40;
/// Debounce of folder change notifications.
const TIMER_FILES: usize = 22;
/// Watcher tags of the Files tab start here (0 = document, 1 = theme).
pub(super) const FILES_TAG: usize = 2;

/// F6 regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Region {
    #[default]
    Document,
    TitleBar,
    Banners,
    Sidebar,
}

/// Per-window V2-D state.
#[derive(Default)]
pub(super) struct Nav {
    /// Files tab tree (built when the tab is first shown).
    pub files: RefCell<Option<Files>>,
    pub palette: Option<palette::Palette>,
    pub mind: Option<Box<mindmap::Mind>>,
    /// Keyboard focus on a non-link stop of the document.
    pub kf: Option<(usize, yamdv_layout::focus::Stop)>,
    pub region: Region,
    /// Title-bar button with the keyboard (F6 region).
    pub chrome_focus: usize,
    /// Sidebar row with the keyboard (index into the non-tab rows).
    pub side_focus: Option<usize>,
    pub fav_drag: Option<favs::Drag>,
    /// The last input was the keyboard (focus rings are shown).
    pub kbd: bool,
}

impl Win {
    /// V2-D messages; `Some` when consumed.
    pub(super) fn nav_msg(
        &mut self,
        sh: &Shared,
        msg: u32,
        wp: WPARAM,
        lp: LPARAM,
    ) -> Option<LRESULT> {
        match msg {
            WM_APP_FILES => {
                self.files_collect(sh);
                Some(LRESULT(0))
            }
            crate::shell::WM_APP_FILE_CHANGED if wp.0 >= FILES_TAG => {
                self.files_dirty(wp.0 - FILES_TAG);
                // SAFETY: valid hwnd.
                unsafe { SetTimer(Some(self.hwnd), TIMER_FILES, 150, None) };
                Some(LRESULT(0))
            }
            WM_TIMER if wp.0 == TIMER_FILES => {
                // SAFETY: valid hwnd.
                let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_FILES) };
                self.files_relist();
                Some(LRESULT(0))
            }
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                let vk = u16::try_from(wp.0).unwrap_or(0);
                // SAFETY: GetKeyState has no preconditions.
                let (ctrl, shift, alt) = unsafe {
                    (
                        GetKeyState(i32::from(VK_CONTROL.0)) < 0,
                        GetKeyState(i32::from(VK_SHIFT.0)) < 0,
                        GetKeyState(0x12) < 0,
                    )
                };
                self.v2d_key(sh, vk, ctrl, shift, alt).then_some(LRESULT(0))
            }
            WM_CHAR if self.nav.palette.is_some() => {
                if let Some(c) = char::from_u32(wp.0 as u32) {
                    self.palette_char(sh, c);
                }
                Some(LRESULT(0))
            }
            WM_CHAR if self.nav.mind.is_some() => Some(LRESULT(0)),
            WM_LBUTTONDOWN | windows::Win32::UI::WindowsAndMessaging::WM_LBUTTONDBLCLK => {
                self.nav_kbd(false);
                let (x, y) = self.lp_xy(lp);
                if self.nav.palette.is_some() {
                    self.palette_down(sh, x, y);
                    return Some(LRESULT(0));
                }
                if self.nav.mind.is_some() {
                    self.mind_down(sh, x, y);
                    return Some(LRESULT(0));
                }
                None
            }
            WM_LBUTTONUP if self.nav.mind.is_some() => {
                self.mind_up();
                Some(LRESULT(0))
            }
            WM_MOUSEMOVE if self.nav.mind.is_some() || self.nav.palette.is_some() => {
                let (x, y) = self.lp_xy(lp);
                if self.nav.mind.is_some() {
                    self.mind_move(sh, x, y);
                } else {
                    self.palette_move(sh, x, y);
                }
                Some(LRESULT(0))
            }
            WM_MOUSEWHEEL if self.nav.mind.is_some() || self.nav.palette.is_some() => {
                let delta = ((wp.0 >> 16) & 0xFFFF) as u16 as i16;
                let sx = i32::from(lp.0 as u16 as i16);
                let sy = i32::from((lp.0 >> 16) as u16 as i16);
                let (x, y) = self.screen_xy(sx, sy);
                if self.nav.mind.is_some() {
                    self.mind_wheel(delta, x, y);
                } else {
                    self.palette_wheel(delta);
                }
                Some(LRESULT(0))
            }
            _ => None,
        }
    }

    /// Keyboard input mode (focus rings show only after keyboard use, B.16).
    pub(super) fn nav_kbd(&mut self, on: bool) {
        if self.nav.kbd != on {
            self.nav.kbd = on;
            if !on {
                self.nav.kf = None;
                self.nav.side_focus = None;
                self.nav.region = Region::Document;
            }
            self.invalidate();
        }
    }

    /// Keys of the V2-D features; `true` when consumed. Also called by the driver.
    pub(super) fn v2d_key(
        &mut self,
        sh: &Shared,
        vk: u16,
        ctrl: bool,
        shift: bool,
        alt: bool,
    ) -> bool {
        if self.dialog.is_some() || self.settings.is_some() || self.typing() {
            return false;
        }
        if self.nav.palette.is_some() {
            self.palette_key(sh, vk, ctrl);
            return true;
        }
        if self.nav.mind.is_some() {
            self.mind_key(sh, vk, ctrl);
            return true;
        }
        if self.tz_viewer_open() || self.find.is_some() {
            return false;
        }
        // F-27: these are keymap actions (rebindable, in the presets), looked up by chord.
        let c = yamdv_core::keymap::Chord {
            ctrl,
            shift,
            alt,
            vk,
        };
        let act = app::keymap().action_of(c, self.edit_mode().is_some());
        match act.unwrap_or("") {
            "sidebar_outline" => return self.side_select_tab_key(sh, 0),
            "sidebar_history" => return self.side_select_tab_key(sh, 1),
            "sidebar_favorites" => return self.side_select_tab_key(sh, 2),
            "sidebar_files" => return self.side_select_tab_key(sh, 3),
            "heading_palette" => {
                self.palette_open(sh);
                return true;
            }
            "mind_map" => {
                self.mind_open(sh);
                return true;
            }
            "next_region" | "prev_region" => {
                self.nav_kbd(true);
                self.region_step(sh, if act == Some("prev_region") { -1 } else { 1 });
                return true;
            }
            _ => {}
        }
        match self.nav.region {
            Region::Sidebar => self.side_key(sh, vk, ctrl, shift, alt),
            Region::TitleBar => self.chrome_key(sh, vk, shift),
            Region::Banners => self.banner_key(sh, vk, shift),
            Region::Document => self.doc_key(sh, vk, ctrl, shift, alt),
        }
    }

    fn side_select_tab_key(&mut self, sh: &Shared, t: u8) -> bool {
        self.side_select_tab(sh, t);
        true
    }

    /// Draws the V2-D overlays (content origin).
    pub(super) fn nav_draw(&self, sh: &Shared, b: &Band<'_>) {
        self.focus_draw(sh, b);
        self.mind_draw(sh, b);
        self.palette_draw(sh, b);
    }

    /// Something V2-D is modal (palette or mind map).
    pub(super) fn nav_modal(&self) -> bool {
        self.nav.palette.is_some() || self.nav.mind.is_some()
    }
}
