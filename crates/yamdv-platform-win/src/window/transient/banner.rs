//! Banners (B.10.5, P-42): 36 px rows under the title bar, at most two, newest on top. The
//! content moves down and keeps its scroll anchor.

use super::*;
use yamdv_core::theme::CALLOUTS;
use yamdv_ui::transient::BANNER_H;

/// Driver name of a banner.
#[cfg(feature = "dev-tools")]
pub(super) fn name(k: BannerKind) -> &'static str {
    match k {
        BannerKind::Conflict => "conflict",
        BannerKind::Deleted => "deleted",
        BannerKind::ReadOnly => "read-only",
        BannerKind::Decoding => "decoding",
        BannerKind::Motw => "motw",
        BannerKind::Elevated => "elevated",
        BannerKind::ExportFailed => "export-failed",
        BannerKind::Recovery => "recovery",
    }
}

impl Win {
    /// Shows or removes banner `k`.
    pub(in crate::window) fn tz_banner(&mut self, sh: &Shared, k: BannerKind, on: bool) {
        let changed = if on {
            self.tz.banners.push(k)
        } else {
            self.tz.banners.dismiss(k)
        };
        if changed {
            self.tz_relayout(sh);
        }
    }

    /// Export failure banner (BUG-11 → B.10.5): `None` clears it.
    pub(in crate::window) fn tz_export_banner(&mut self, sh: &Shared, m: Option<Msg>) {
        self.tz.export_msg = m;
        self.tz_banner(sh, BannerKind::ExportFailed, m.is_some());
        self.invalidate();
    }

    /// Content offset = title bar (unless fullscreen) + banners.
    pub(in crate::window) fn tz_relayout(&mut self, sh: &Shared) {
        let bar = if self.fs { 0.0 } else { ch::BAR_H };
        let top = bar + self.tz.banners.height();
        if (top - self.top).abs() > 0.01 {
            self.top = top;
            update_size(self, sh);
        }
        // SAFETY: valid hwnd.
        let _ = unsafe { InvalidateRect(Some(self.hwnd), None, false) };
    }

    /// Height taken by the banners (fullscreen toggling).
    pub(in crate::window) fn tz_banners_h(&self) -> f32 {
        self.tz.banners.height()
    }

    /// Banners of a newly shown document (P-42): MotW, read-only, decoding fallback,
    /// elevation. Banners of the previous document go away.
    pub(in crate::window) fn tz_on_open(&mut self, sh: &Shared) {
        let (internet, ro, lossy) = match &self.view {
            View::Doc(d) => (d.internet, d.facts.read_only, d.facts.lossy),
            _ => (false, false, false),
        };
        for k in [
            BannerKind::Conflict,
            BannerKind::Deleted,
            BannerKind::ReadOnly,
            BannerKind::Decoding,
            BannerKind::Motw,
            BannerKind::Recovery,
        ] {
            self.tz.banners.dismiss(k);
        }
        if lossy {
            self.tz.banners.push(BannerKind::Decoding);
        }
        if ro {
            self.tz.banners.push(BannerKind::ReadOnly);
        }
        if internet {
            self.tz.banners.push(BannerKind::Motw);
        }
        // Scripted runs keep the layout of the goldens (CI runners are elevated).
        if sh.dev.script.is_none()
            && !self.tz.banners.has(BannerKind::Elevated)
            && sys::is_elevated()
        {
            self.tz.banners.push(BannerKind::Elevated);
        }
        if self.journal_on_open() {
            self.tz.banners.push(BannerKind::Recovery);
        }
        self.tz_relayout(sh);
    }

    fn banner_text(&self, sh: &Shared, k: BannerKind) -> String {
        let m = match k {
            BannerKind::Conflict => Msg::ConflictTitle,
            BannerKind::Deleted => Msg::ToastDeleted,
            BannerKind::ReadOnly => Msg::BannerReadOnly,
            BannerKind::Decoding => {
                return tr(sh.lang, Msg::BannerDecoding).replace("{}", "UTF-8 (U+FFFD)");
            }
            BannerKind::Motw => Msg::BannerMotw,
            BannerKind::Elevated => Msg::BannerElevated,
            BannerKind::ExportFailed => self.tz.export_msg.unwrap_or(Msg::BannerExportFailed),
            BannerKind::Recovery => {
                return format!("{} {}", tr(sh.lang, Msg::RecoverBanner), self.journal_name());
            }
        };
        tr(sh.lang, m).to_string()
    }

    /// Buttons of banner `k`, right-aligned in `r` (client DIPs); id 0 = ×.
    pub(super) fn banner_buttons(
        &self,
        sh: &Shared,
        k: BannerKind,
        r: Rect,
    ) -> Vec<(Rect, u32, String)> {
        let list: Vec<(u32, String)> = if k == BannerKind::Conflict {
            [
                (Msg::Overwrite, edit::BTN_OVERWRITE),
                (Msg::ReloadDiscard, edit::BTN_RELOAD_DISCARD),
                (Msg::SaveMineAs, edit::BTN_SAVE_AS),
            ]
            .into_iter()
            .map(|(m, id)| (id, tr(sh.lang, m).to_string()))
            .collect()
        } else if k == BannerKind::Recovery {
            [
                (Msg::RecoverRestore, plus::BTN_RESTORE),
                (Msg::RecoverDiscard, plus::BTN_JOURNAL_DISCARD),
            ]
            .into_iter()
            .map(|(m, id)| (id, tr(sh.lang, m).to_string()))
            .collect()
        } else {
            vec![(0, "×".into())]
        };
        let mut x = r.x + r.w - 8.0;
        let mut out = Vec::new();
        for (id, label) in list.into_iter().rev() {
            let w = if id == 0 {
                28.0
            } else {
                sh.dw.measure(&label, 13.0, 600, 10_000.0).0.ceil() + 16.0
            };
            x -= w;
            out.push((Rect::new(x, r.y + 4.0, w, r.h - 8.0), id, label));
            x -= 4.0;
        }
        out.reverse();
        out
    }

    pub(super) fn banner_rects(&self) -> Vec<(BannerKind, Rect)> {
        let y0 = self.top - self.tz.banners.height();
        self.tz
            .banners
            .visible()
            .into_iter()
            .enumerate()
            .map(|(i, k)| {
                (
                    k,
                    Rect::new(0.0, y0 + i as f32 * BANNER_H, self.cw, BANNER_H),
                )
            })
            .collect()
    }

    /// Draws the banners (client DIPs, origin 0).
    pub(super) fn tz_draw_banners(&self, sh: &Shared, b: &Band<'_>) {
        for (k, r) in self.banner_rects() {
            if !b.visible(r.y, r.h) {
                continue;
            }
            let c = if k.is_error() {
                &CALLOUTS[3]
            } else {
                &CALLOUTS[0]
            };
            let (fg, bg) = if self.dark {
                (c.dark, c.dark_bg)
            } else {
                (c.light, c.light_bg)
            };
            b.fill(r, self.pal.bg, 1.0, 0.0);
            b.fill(r, bg, 1.0, 0.0);
            b.fill(Rect::new(0.0, r.y + r.h - 1.0, r.w, 1.0), fg, 0.5, 0.0);
            let icon = if k.is_error() { "⚠" } else { "ⓘ" };
            let ic = self.shape_ui(sh, icon, 14.0, 400, fg, None);
            b.text(ic.as_ref(), 16.0, (r.y + (r.h - ic.height()) / 2.0).round());
            let btns = self.banner_buttons(sh, k, r);
            let right = btns.first().map_or(r.w - 16.0, |(br, _, _)| br.x - 8.0);
            let text = self.banner_text(sh, k);
            let t = self.shape_ui(sh, &text, 13.0, 400, self.pal.text, None);
            b.push_clip(Rect::new(0.0, r.y, right, r.h));
            b.text(t.as_ref(), 40.0, (r.y + (r.h - t.height()) / 2.0).round());
            b.pop_clip();
            for (br, id, label) in btns {
                let (size, weight) = if id == 0 { (16.0, 400) } else { (13.0, 600) };
                let l = self.shape_ui(sh, &label, size, weight, fg, None);
                center(b, l.as_ref(), br);
            }
        }
    }

    /// Click at client (x, y); `true` when it hit a banner.
    pub(super) fn banner_click(&mut self, sh: &Shared, x: f32, y: f32) -> bool {
        let Some((k, r)) = self
            .banner_rects()
            .into_iter()
            .find(|(_, r)| r.contains(x, y))
        else {
            return false;
        };
        let hit = self
            .banner_buttons(sh, k, r)
            .into_iter()
            .find(|(br, _, _)| br.contains(x, y));
        if let Some((_, id, _)) = hit {
            self.tz_banner_button(sh, k, id);
        }
        true
    }

    /// Activates button `id` of banner `k` (0 = ×).
    pub(in crate::window) fn tz_banner_button(&mut self, sh: &Shared, k: BannerKind, id: u32) {
        if k == BannerKind::Conflict {
            self.tz_banner(sh, k, false);
            self.edit_dialog(sh, edit::Ask::Conflict, id);
        } else if k == BannerKind::Recovery {
            self.tz_banner(sh, k, false);
            self.journal_button(sh, id);
        } else if id == 0 {
            if k == BannerKind::ExportFailed {
                self.tz.export_msg = None;
                self.export_banner_dismissed();
            }
            self.tz_banner(sh, k, false);
        }
        self.invalidate();
    }
}
