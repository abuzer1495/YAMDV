//! Welcome page (simplified B.12, prompt 005 §3.12): icon, title, drop-zone text,
//! [Open file…], the last 8 recent files and the default-app status with [Set as default].

use super::*;
use std::cell::Cell;

thread_local! {
    /// Cached association status (R-SYS-02); refreshed when the app is re-activated.
    static DEFAULT_APP: Cell<Option<bool>> = const { Cell::new(None) };
    /// Not running from the install folder and not portable (R-SYS-05), queried once.
    static OFFER_INSTALL: Cell<Option<bool>> = const { Cell::new(None) };
}

/// The welcome page shows [Install YAMDV for this user].
pub(super) fn offer_install() -> bool {
    OFFER_INSTALL.with(|c| {
        c.get().unwrap_or_else(|| {
            let v = crate::install::offer_install();
            c.set(Some(v));
            v
        })
    })
}

/// Association status, queried once and cached.
pub(super) fn is_default() -> bool {
    DEFAULT_APP.with(|c| {
        c.get().unwrap_or_else(|| {
            let v = crate::shell::is_default_app();
            c.set(Some(v));
            v
        })
    })
}

/// Forgets the cached status (WM_ACTIVATEAPP).
pub(super) fn refresh_default() {
    DEFAULT_APP.with(|c| c.set(None));
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Hit {
    Open,
    Recent(String),
    SetDefault,
    Install,
}

impl Win {
    /// Layout: (rect, hit, label) for the clickable parts plus the static text positions.
    pub(super) fn welcome_items(&self, sh: &Shared) -> Vec<(Rect, Option<Hit>, String)> {
        let w = 480.0f32.min(self.cw - 48.0).max(240.0);
        let x = ((self.cw - w) / 2.0).round();
        let recent: Vec<String> = app::with(|a| {
            a.state
                .history
                .iter()
                .take(8)
                .map(|h| h.1.clone())
                .collect()
        })
        .unwrap_or_default();
        let h = 64.0
            + 16.0
            + 32.0
            + 28.0
            + 24.0
            + 36.0
            + 32.0
            + if recent.is_empty() {
                0.0
            } else {
                28.0 + 26.0 * recent.len() as f32
            }
            + 24.0
            + 32.0
            + if offer_install() { 44.0 } else { 0.0 };
        let mut y = ((self.ch - h) / 2.0).max(24.0).round();
        let mut out = Vec::new();
        out.push((
            Rect::new(x + (w - 64.0) / 2.0, y, 64.0, 64.0),
            None,
            "icon".to_string(),
        ));
        y += 80.0;
        out.push((
            Rect::new(x, y, w, 32.0),
            None,
            tr(sh.lang, Msg::WelcomeTitle).to_string(),
        ));
        y += 36.0;
        out.push((
            Rect::new(x, y, w, 24.0),
            None,
            tr(sh.lang, Msg::EmptyState).to_string(),
        ));
        y += 40.0;
        out.push((
            Rect::new(x + (w - 160.0) / 2.0, y, 160.0, 36.0),
            Some(Hit::Open),
            tr(sh.lang, Msg::WelcomeOpen).to_string(),
        ));
        y += 60.0;
        if !recent.is_empty() {
            out.push((
                Rect::new(x, y, w, 24.0),
                None,
                tr(sh.lang, Msg::WelcomeRecent).to_string(),
            ));
            y += 28.0;
            for p in recent {
                let name = ypath::file_name(&ypath::display_form(&p)).to_string();
                out.push((Rect::new(x, y, w, 24.0), Some(Hit::Recent(p)), name));
                y += 26.0;
            }
            y += 16.0;
        }
        let def = is_default();
        let m = if def {
            Msg::WelcomeIsDefault
        } else {
            Msg::WelcomeNotDefault
        };
        out.push((Rect::new(x, y, w, 24.0), None, tr(sh.lang, m).to_string()));
        if !def {
            y += 30.0;
            out.push((
                Rect::new(x + (w - 160.0) / 2.0, y, 160.0, 32.0),
                Some(Hit::SetDefault),
                tr(sh.lang, Msg::WelcomeSetDefault).to_string(),
            ));
        }
        if offer_install() {
            y += 44.0;
            out.push((
                Rect::new(x + (w - 260.0) / 2.0, y, 260.0, 32.0),
                Some(Hit::Install),
                tr(sh.lang, Msg::WelcomeInstall).to_string(),
            ));
        }
        out
    }

    pub(super) fn draw_welcome(&self, sh: &Shared, b: &Band<'_>) {
        let pal = &self.pal;
        let hover = self.side.hover;
        for (i, (r, hit, label)) in self.welcome_items(sh).into_iter().enumerate() {
            let hov = hover.is_some_and(|(x, y)| r.contains(x, y));
            let center = |t: &dyn yamdv_layout::shaper::Shaped| {
                (
                    (r.x + (r.w - t.width()) / 2.0).round(),
                    (r.y + (r.h - t.height()) / 2.0).round(),
                )
            };
            match (i, &hit) {
                (0, _) => {
                    b.fill(r, pal.accent, 1.0, 14.0);
                    let t = self.shape_ui(sh, "M\u{2193}", 26.0, 700, 0xffffff, None);
                    let (tx, ty) = center(t.as_ref());
                    b.text(t.as_ref(), tx, ty);
                }
                (1, _) => {
                    let t = self.shape_ui(sh, &label, 22.0, 600, pal.heading, None);
                    let (tx, ty) = center(t.as_ref());
                    b.text(t.as_ref(), tx, ty);
                }
                (_, Some(Hit::Open | Hit::SetDefault | Hit::Install)) => {
                    let primary = hit == Some(Hit::Open);
                    if primary {
                        b.fill(r, pal.accent, 1.0, 6.0);
                    } else {
                        b.fill(r, pal.bg, 1.0, 6.0);
                        b.stroke(r, pal.code_border, 6.0, 1.0);
                    }
                    if hov {
                        b.fill(r, rgb_text(self.dark), 0.08, 6.0);
                    }
                    let t = self.shape_ui(
                        sh,
                        &label,
                        14.0,
                        500,
                        if primary { 0xffffff } else { pal.text },
                        None,
                    );
                    let (tx, ty) = center(t.as_ref());
                    b.text(t.as_ref(), tx, ty);
                }
                (_, Some(Hit::Recent(_))) => {
                    if hov {
                        b.fill(r, pal.text, 0.06, 4.0);
                    }
                    let t = self.shape_ui(sh, &label, 13.0, 400, pal.link, None);
                    b.text(
                        t.as_ref(),
                        r.x + 8.0,
                        (r.y + (r.h - t.height()) / 2.0).round(),
                    );
                }
                _ => {
                    let t = self.shape_ui(sh, &label, 13.0, 400, pal.text_muted, Some(r.w));
                    let (tx, ty) = center(t.as_ref());
                    b.text(t.as_ref(), tx, ty);
                }
            }
        }
    }

    /// Click on the welcome page.
    pub(super) fn welcome_click(&mut self, sh: &Shared, x: f32, y: f32) -> Option<Action> {
        let hit = self
            .welcome_items(sh)
            .into_iter()
            .find(|(r, h, _)| h.is_some() && r.contains(x, y))?
            .1?;
        match hit {
            Hit::Open => Some(Action::OpenDialog),
            Hit::Install => {
                OFFER_INSTALL.with(|c| c.set(Some(false)));
                Some(Action::Install)
            }
            Hit::SetDefault => {
                refresh_default();
                Some(Action::Command(app::SET_DEFAULT))
            }
            Hit::Recent(p) => {
                if std::path::Path::new(&ypath::long_form(&p)).is_file() {
                    self.open(sh, &p, None, false);
                } else {
                    let n = ypath::file_name(&ypath::display_form(&p)).to_string();
                    self.toast_with(Msg::ToastMissing, &n);
                }
                None
            }
        }
    }
}
