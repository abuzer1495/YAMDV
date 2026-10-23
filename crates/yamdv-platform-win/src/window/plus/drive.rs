//! Driver hooks of V2-E (dev-tools only): `ui v2e-<verb> <arg>` and state probes.
#![allow(unsafe_code)]

use super::*;
use windows::Win32::Foundation::{GlobalFree, HANDLE};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};

/// Puts raw `bytes` on the clipboard as format `fmt` (replacing its contents).
fn set_clip(hwnd: HWND, fmt: u32, bytes: &[u8]) -> bool {
    // SAFETY: standard clipboard sequence; the global memory belongs to the clipboard after a
    // successful SetClipboardData and is freed by us otherwise.
    unsafe {
        if OpenClipboard(Some(hwnd)).is_err() {
            return false;
        }
        let _ = EmptyClipboard();
        let ok = (|| -> Option<()> {
            let h = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1)).ok()?;
            let p = GlobalLock(h);
            if p.is_null() {
                let _ = GlobalFree(Some(h));
                return None;
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), p.cast::<u8>(), bytes.len());
            let _ = GlobalUnlock(h);
            if SetClipboardData(fmt, Some(HANDLE(h.0))).is_err() {
                let _ = GlobalFree(Some(h));
                return None;
            }
            Some(())
        })()
        .is_some();
        let _ = CloseClipboard();
        ok
    }
}

/// A `w × h` 24-bit bottom-up DIB with a diagonal pattern.
fn test_dib(w: u32, h: u32) -> Vec<u8> {
    let stride = (w as usize * 3).div_ceil(4) * 4;
    let mut v = Vec::with_capacity(40 + stride * h as usize);
    v.extend(40u32.to_le_bytes());
    v.extend(w.to_le_bytes());
    v.extend(h.to_le_bytes());
    v.extend(1u16.to_le_bytes());
    v.extend(24u16.to_le_bytes());
    v.extend([0u8; 24]);
    for y in 0..h {
        let mut row = vec![0u8; stride];
        for x in 0..w as usize {
            let c = if x as u32 == y { 0 } else { 255 };
            if let Some(px) = row.get_mut(x * 3..x * 3 + 3) {
                px.copy_from_slice(&[c, 128, 255 - c]);
            }
        }
        v.extend(row);
    }
    v
}

impl Win {
    /// `ui v2e-*` verbs.
    pub(in crate::window) fn v2e_ui(&mut self, sh: &Shared, verb: &str, arg: &str) -> String {
        let ok = match verb {
            "v2e-tb" => self.toolbar_press(sh, arg),
            // F2: opens the block holding `arg` and selects it (no DirectWrite hit test).
            "v2e-live-select" => {
                let at = match &self.view {
                    View::Doc(d) => d
                        .content
                        .source()
                        .find(arg)
                        .map(|p| (p, crate::window::edit::block_of(&d.content, p))),
                    _ => None,
                };
                at.is_some_and(|(p, bi)| {
                    self.enter_block(sh, bi, p);
                    if let Some(e) = self.edit_mut() {
                        e.ed.set_selection(p, p + arg.len());
                    }
                    self.invalidate();
                    true
                })
            }
            "v2e-set" => arg
                .split_once('=')
                .is_some_and(|(k, v)| app::set(k.trim(), v.trim())),
            "v2e-clip-html" => {
                let cf = yamdv_export::html::cf_html(arg);
                sys::copy_text_html(self.hwnd, "plain fallback", Some(&cf))
            }
            "v2e-clip-text" => sys::copy_text_html(self.hwnd, arg, None),
            "v2e-clip-bitmap" => {
                let (w, h) = arg.split_once('x').unwrap_or(("4", "3"));
                let (w, h) = (w.parse().unwrap_or(4), h.parse().unwrap_or(3));
                set_clip(self.hwnd, 8, &test_dib(w, h))
            }
            // Simulated TerminateProcess: the buffer and journal state vanish, the journal file
            // stays on disk; the next `open` is the restart.
            "v2e-kill" => {
                self.plus = Plus::default();
                if let View::Doc(d) = &mut self.view {
                    d.edit = None;
                    d.viewer.source_block = None;
                }
                self.view = View::Empty { note: None };
                true
            }
            _ => return format!("FAIL ui {verb}: unknown verb"),
        };
        format!("{} ui {verb} {arg}", if ok { "ok" } else { "FAIL" })
    }

    /// Numeric probes: `journal` (file on disk), `assets` (files in the asset folder),
    /// `toolbar` (visible).
    pub(in crate::window) fn v2e_state(&self, sh: &Shared) -> Vec<(&'static str, f64)> {
        let path = match &self.view {
            View::Doc(d) if !d.virt => Some(d.path.clone()),
            _ => None,
        };
        let journal = path.as_deref().is_some_and(|p| {
            std::path::Path::new(&app::recovery_dir())
                .join(yamdv_core::journal::file_name(p, 0))
                .exists()
        });
        let assets = path.as_deref().map_or(0, |p| {
            let long = ypath::long_form(p);
            let p = std::path::Path::new(&long);
            let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned());
            let rel = stem.and_then(|s| {
                yamdv_core::paste::assets::asset_dir(&app::get("editing.assets_dir"), &s)
            });
            match (p.parent(), rel) {
                (Some(dir), Some(rel)) => {
                    std::fs::read_dir(dir.join(rel)).map_or(0, |r| r.filter_map(Result::ok).count())
                }
                _ => 0,
            }
        });
        vec![
            ("journal", f64::from(u8::from(journal))),
            ("assets", assets as f64),
            ("toolbar", f64::from(u8::from(self.toolbar_visible(sh)))),
        ]
    }
}
