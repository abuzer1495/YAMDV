//! Background export UI (BUG-11, B.10.7): the progress card, taskbar progress, Cancel, the
//! close confirmation and the completion toast with [Open] / [Show in folder].
#![allow(unsafe_code)]

use super::*;
use crate::job::{EXIT_CANCELLED, EXIT_OK, Worker};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::UI::Shell::{
    ITaskbarList3, TBPF_INDETERMINATE, TBPF_NOPROGRESS, TBPF_NORMAL, TaskbarList,
};
use yamdv_core::progress::{self, JobKind};

/// One-shot timer that reveals the card 300 ms after the job started.
pub(super) const TIMER_JOB: usize = 18;
/// The card appears only for jobs longer than this.
const CARD_DELAY_MS: u128 = 300;
/// Button id of "Cancel export and close" in the close confirmation.
pub(super) const BTN_JOB_CLOSE: u32 = 30;
const CARD_W: f32 = 360.0;
const CARD_H: f32 = 112.0;

thread_local! {
    /// The running export worker (one per process; export commands are refused meanwhile).
    pub(super) static JOB: RefCell<Option<Worker>> = const { RefCell::new(None) };
    static TASKBAR: RefCell<Option<ITaskbarList3>> = const { RefCell::new(None) };
    /// Outcome of the last finished job (driver state `job_last`).
    static LAST: std::cell::Cell<&'static str> = const { std::cell::Cell::new("none") };
}

/// `ok`, `cancelled`, `failed` or `none` (no job finished yet).
#[cfg_attr(not(feature = "dev-tools"), allow(dead_code))]
pub(super) fn last_outcome() -> &'static str {
    LAST.with(std::cell::Cell::get)
}

/// Whether an export worker is running.
pub(super) fn busy() -> bool {
    JOB.with(|j| j.borrow().is_some())
}

/// The worker's process handle (waited on by the message loop).
pub(super) fn process() -> Option<HANDLE> {
    JOB.with(|j| j.borrow().as_ref().map(|w| w.process))
}

fn taskbar(f: impl FnOnce(&ITaskbarList3)) {
    TASKBAR.with(|t| {
        let mut t = t.borrow_mut();
        if t.is_none() {
            // SAFETY: COM is initialized on the UI thread (window::run).
            let tb: Option<ITaskbarList3> =
                unsafe { CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER) }.ok();
            // SAFETY: HrInit is required once before use.
            *t = tb.filter(|tb| unsafe { tb.HrInit() }.is_ok());
        }
        if let Some(tb) = t.as_ref() {
            f(tb);
        }
    });
}

/// Starts `text` (a worker job) for `owner`; toasts on failure.
pub(super) fn start(owner: HWND, kind: JobKind, text: &str, out: Option<String>) -> bool {
    let Some(w) = crate::job::spawn(owner, kind, text, out) else {
        return false;
    };
    JOB.with(|j| *j.borrow_mut() = Some(w));
    taskbar(|tb| {
        // SAFETY: valid hwnd.
        let _ = unsafe { tb.SetProgressState(owner, TBPF_INDETERMINATE) };
    });
    // SAFETY: valid hwnd; one-shot timer (killed when it fires).
    unsafe { SetTimer(Some(owner), TIMER_JOB, 310, None) };
    true
}

/// A `WM_COPYDATA` progress record for `receiver`; `true` when it was one (and valid).
pub(super) fn on_copydata(receiver: HWND, sender: HWND, cds: &COPYDATASTRUCT) -> bool {
    let r = JOB.with(|j| {
        let mut j = j.borrow_mut();
        let w = j.as_mut()?;
        let r = w.accept(receiver, sender, cds)?;
        w.last = Some(r);
        Some(r)
    });
    let Some(r) = r else { return false };
    if r.total > 0 {
        taskbar(|tb| {
            // SAFETY: valid hwnd.
            unsafe {
                let _ = tb.SetProgressState(receiver, TBPF_NORMAL);
                let _ = tb.SetProgressValue(receiver, u64::from(r.n), u64::from(r.total));
            }
        });
    }
    // SAFETY: valid hwnd.
    let _ = unsafe { InvalidateRect(Some(receiver), None, false) };
    true
}

/// The worker ended: outcome toast, partial-file cleanup, taskbar reset, deferred close.
pub(super) fn exited() {
    let Some(w) = JOB.with(|j| j.borrow_mut().take()) else {
        return;
    };
    let code = w.exit_code();
    let owner = w.owner;
    taskbar(|tb| {
        // SAFETY: valid (or stale, then harmless) hwnd.
        let _ = unsafe { tb.SetProgressState(owner, TBPF_NOPROGRESS) };
    });
    if code != Some(EXIT_OK) {
        for f in w.partial_files() {
            let _ = std::fs::remove_file(f);
        }
    }
    crate::log::warn(
        "export-job",
        &format!(
            "{:?} ended with {code:?} after {} ms",
            w.kind,
            w.started.elapsed().as_millis()
        ),
    );
    LAST.with(|l| {
        l.set(match code {
            Some(EXIT_OK) => "ok",
            Some(EXIT_CANCELLED) => "cancelled",
            _ => "failed",
        });
    });
    let close = w.close_after;
    let kind = w.kind;
    let out = w.out.clone();
    drop(w);
    if close {
        // SAFETY: valid hwnd; the job is gone, so WM_CLOSE now passes.
        let _ = unsafe { PostMessageW(Some(owner), 0x0010, WPARAM(0), LPARAM(0)) };
        return;
    }
    app::with_win(owner, |win, _| {
        let print = kind == JobKind::Print;
        match code {
            Some(EXIT_OK) => {
                if let Some(o) = out.filter(|_| !print) {
                    win.done_file = Some(o.clone());
                    win.toast_with(Msg::Exported, &format!(": {}", ypath::file_name(&o)));
                    // Longer for the actions (B.10.7).
                    // SAFETY: valid hwnd.
                    unsafe { SetTimer(Some(win.hwnd), TIMER_TOAST, 8000, None) };
                } else {
                    win.toast(Msg::PrintSent);
                }
            }
            Some(EXIT_CANCELLED) => win.toast(if print {
                Msg::PrintCancelled
            } else {
                Msg::ExportCancelled
            }),
            _ => win.toast(if print {
                Msg::PrintFailed
            } else {
                Msg::ExportFailed
            }),
        }
        win.invalidate();
    });
}

/// Card and button rectangles in client DIPs.
fn card_rects(cw: f32, ch: f32) -> (Rect, Rect) {
    let w = CARD_W.min((cw - 24.0).max(200.0));
    let card = Rect::new(
        ((cw - w) / 2.0).round(),
        (ch - CARD_H - 24.0).round(),
        w,
        CARD_H,
    );
    let cancel = Rect::new(card.x + card.w - 96.0, card.y + card.h - 40.0, 80.0, 28.0);
    (card, cancel)
}

/// Toast action buttons ([Open], [Show in folder]) right-aligned in the toast rect `t`.
pub(super) fn toast_buttons(t: Rect) -> (Rect, Rect) {
    let reveal = Rect::new(t.x + t.w - 128.0, t.y + 6.0, 116.0, t.h - 12.0);
    let open = Rect::new(reveal.x - 72.0, t.y + 6.0, 64.0, t.h - 12.0);
    (open, reveal)
}

impl Win {
    /// The card is visible (this window owns the job and it runs ≥ 300 ms).
    fn card_visible(&self) -> bool {
        JOB.with(|j| {
            j.borrow().as_ref().is_some_and(|w| {
                w.owner == self.hwnd && w.started.elapsed().as_millis() >= CARD_DELAY_MS
            })
        })
    }

    /// Draws the progress card (bottom center, above the toast position).
    pub(super) fn draw_job(&self, sh: &Shared, b: &Band<'_>) {
        if !self.card_visible() {
            return;
        }
        let Some((kind, out, last, elapsed, cancelling)) = JOB.with(|j| {
            j.borrow().as_ref().map(|w| {
                (
                    w.kind,
                    w.out.clone(),
                    w.last,
                    u64::try_from(w.started.elapsed().as_millis()).unwrap_or(u64::MAX),
                    w.cancelling,
                )
            })
        }) else {
            return;
        };
        let pal = &self.pal;
        let (card, cancel) = card_rects(self.cw, self.ch);
        b.fill(card, pal.bg, 1.0, 8.0);
        b.stroke(card, pal.code_border, 8.0, 1.0);
        let inner = card.w - 32.0;
        let title = self.shape_ui(
            sh,
            progress::title(sh.lang, kind),
            14.0,
            600,
            pal.text,
            None,
        );
        b.text(title.as_ref(), card.x + 16.0, card.y + 12.0);
        let name = out
            .as_deref()
            .map(|o| progress::middle_ellipsis(ypath::file_name(o), 44))
            .unwrap_or_default();
        if !name.is_empty() {
            let t = self.shape_ui(sh, &name, 12.0, 400, pal.text_muted, Some(inner));
            b.text(t.as_ref(), card.x + 16.0, card.y + 34.0);
        }
        let rec = last.unwrap_or(progress::Record {
            nonce: 0,
            phase: progress::Phase::Preparing,
            n: 0,
            total: 0,
        });
        let line = if cancelling {
            tr(sh.lang, Msg::Cancel).to_string() + "…"
        } else {
            progress::status_line(sh.lang, kind, &rec, elapsed)
        };
        let st = self.shape_ui(sh, &line, 12.0, 400, pal.text, Some(inner - 96.0));
        b.text(st.as_ref(), card.x + 16.0, card.y + 78.0);
        // Bar: determinate when the worker reported a total; otherwise a static third
        // (no animation timer, T7 / reduced motion).
        let bar = Rect::new(card.x + 16.0, card.y + 58.0, inner, 4.0);
        b.fill(bar, pal.code_border, 1.0, 2.0);
        let frac = if rec.total > 0 {
            f64::from(rec.n) / f64::from(rec.total)
        } else {
            0.33
        };
        #[allow(clippy::cast_possible_truncation)]
        let fw = (f64::from(bar.w) * frac.clamp(0.0, 1.0)) as f32;
        if fw > 0.0 {
            b.fill(Rect::new(bar.x, bar.y, fw, bar.h), pal.accent, 1.0, 2.0);
        }
        b.fill(cancel, pal.bg, 1.0, 6.0);
        b.stroke(cancel, pal.code_border, 6.0, 1.0);
        let l = self.shape_ui(sh, tr(sh.lang, Msg::Cancel), 13.0, 400, pal.text, None);
        b.text(
            l.as_ref(),
            (cancel.x + (cancel.w - l.width()) / 2.0).round(),
            (cancel.y + (cancel.h - l.height()) / 2.0).round(),
        );
    }

    /// Click on the card: Cancel; `true` when the click was consumed.
    pub(super) fn job_click(&mut self, x: f32, y: f32) -> bool {
        if !self.card_visible() {
            return false;
        }
        let (card, cancel) = card_rects(self.cw, self.ch);
        if cancel.contains(x, y) {
            cancel_job();
            self.invalidate();
        }
        card.contains(x, y)
    }

    /// Click on the completion toast's [Open] / [Show in folder]; `true` when consumed.
    pub(super) fn toast_click(&mut self, x: f32, y: f32) -> bool {
        let (Some(path), Some(t)) = (self.done_file.clone(), self.toast_rect.get()) else {
            return false;
        };
        let (open, reveal) = toast_buttons(t);
        if open.contains(x, y) {
            sys::shell_open(&path);
        } else if reveal.contains(x, y) {
            sys::reveal_in_explorer(&path);
        } else {
            return t.contains(x, y);
        }
        self.toast = None;
        self.done_file = None;
        self.invalidate();
        true
    }

    /// WM_CLOSE while this window's export runs: ask (B.10.7). `Some` = handled (kept open).
    pub(super) fn job_close(&mut self, sh: &Shared) -> Option<LRESULT> {
        let mine = JOB.with(|j| {
            j.borrow()
                .as_ref()
                .is_some_and(|w| w.owner == self.hwnd && !w.close_after)
        });
        let closing = JOB.with(|j| {
            j.borrow()
                .as_ref()
                .is_some_and(|w| w.owner == self.hwnd && w.close_after)
        });
        if closing {
            return Some(LRESULT(0));
        }
        if !mine {
            return None;
        }
        if self.dialog.is_none() {
            self.ask(
                sh,
                Msg::JobCloseTitle,
                Msg::JobCloseBody,
                vec![
                    (Msg::JobCloseCancel, BTN_JOB_CLOSE),
                    (Msg::JobCloseKeep, BTN_CANCEL),
                ],
                Pending::JobClose,
            );
        }
        Some(LRESULT(0))
    }
}

/// Signals Cancel to the running worker.
pub(super) fn cancel_job() {
    JOB.with(|j| {
        if let Some(w) = j.borrow_mut().as_mut() {
            w.cancel();
        }
    });
}

/// "Cancel export and close": cancel now, close when the worker has ended.
pub(super) fn cancel_and_close() {
    JOB.with(|j| {
        if let Some(w) = j.borrow_mut().as_mut() {
            w.close_after = true;
            w.cancel();
        }
    });
}

/// State for the dev-tools driver: `(kind, n, total, cancelling)` of the running job.
#[cfg_attr(not(feature = "dev-tools"), allow(dead_code))]
pub(super) fn state() -> Option<(JobKind, u32, u32, bool)> {
    JOB.with(|j| {
        j.borrow().as_ref().map(|w| {
            let (n, t) = w.last.map_or((0, 0), |r| (r.n, r.total));
            (w.kind, n, t, w.cancelling)
        })
    })
}
