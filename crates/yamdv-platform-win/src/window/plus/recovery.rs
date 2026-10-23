//! Recovery journal (prompt 015 §3.6): while autosave is off (or the last save failed) the
//! buffer is copied to `<recovery dir>\<hash>.yjr` 2 s after an edit, at most every 10 s, by a
//! single worker thread (temp file + rename). A save, Discard or a clean close deletes it; the
//! next open of the file offers [Restore] · [Discard].

use super::*;
use std::path::PathBuf;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Mutex, OnceLock};
use yamdv_core::journal as yj;
use yamdv_ui::transient::BannerKind;

/// Journal one-shot timer.
pub(in crate::window) const TIMER_JOURNAL: usize = 31;
/// Recovery banner buttons.
pub(in crate::window) const BTN_RESTORE: u32 = 60;
pub(in crate::window) const BTN_JOURNAL_DISCARD: u32 = 61;

/// Journal state of one window.
#[derive(Debug, Default)]
pub(in crate::window) struct Journal {
    sched: yj::Schedule,
    /// The last save of this document failed (journal even with autosave on).
    pub save_failed: bool,
    /// The 32 MiB toast was shown for this document.
    big_toasted: bool,
    /// Journal file written (or queued) for the open document.
    file: Option<PathBuf>,
    /// Earlier-session journal offered by the banner: file, header, text.
    offer: Option<(PathBuf, yj::Header, String)>,
}

enum Job {
    Write(PathBuf, Vec<u8>),
    Delete(PathBuf),
}

/// Serial worker: writes and deletes in order, off the UI thread.
fn worker() -> Option<std::sync::MutexGuard<'static, Sender<Job>>> {
    static TX: OnceLock<Mutex<Sender<Job>>> = OnceLock::new();
    TX.get_or_init(|| {
        let (tx, rx) = channel::<Job>();
        let _ = std::thread::Builder::new()
            .name("yamdv-journal".into())
            .spawn(move || {
                for job in rx {
                    match job {
                        Job::Write(p, bytes) => {
                            if let Some(d) = p.parent() {
                                let _ = std::fs::create_dir_all(d);
                            }
                            let tmp = p.with_extension("yjr.tmp");
                            if std::fs::write(&tmp, &bytes).is_err()
                                || std::fs::rename(&tmp, &p).is_err()
                            {
                                let _ = std::fs::remove_file(&tmp);
                            }
                        }
                        Job::Delete(p) => {
                            let _ = std::fs::remove_file(&p);
                        }
                    }
                }
            });
        Mutex::new(tx)
    })
    .lock()
    .ok()
}

fn send(job: Job) {
    if let Some(tx) = worker() {
        let _ = tx.send(job);
    }
}

fn mono_ms() -> u64 {
    static T0: OnceLock<std::time::Instant> = OnceLock::new();
    u64::try_from(T0.get_or_init(std::time::Instant::now).elapsed().as_millis()).unwrap_or(0)
}

impl Win {
    /// Path of the open, file-backed document.
    fn journal_doc(&self) -> Option<String> {
        match &self.view {
            View::Doc(d) if !d.virt && !d.path.is_empty() => Some(d.path.clone()),
            _ => None,
        }
    }

    fn journal_path(path: &str) -> PathBuf {
        PathBuf::from(app::recovery_dir()).join(yj::file_name(path, 0))
    }

    /// After a buffer change (`after_edit`).
    pub(in crate::window) fn journal_edited(&mut self) {
        let (dirty, autosave) = self
            .edit()
            .map_or((false, true), |e| (e.ed.is_dirty(), e.opts.autosave));
        if !dirty {
            self.journal_clear();
            return;
        }
        if self.journal_doc().is_none() || !yj::wanted(autosave, self.plus.journal.save_failed)
        {
            return;
        }
        let now = mono_ms();
        self.plus.journal.sched.edit(now);
        self.journal_arm(now);
    }

    fn journal_arm(&self, now: u64) {
        if let Some(due) = self.plus.journal.sched.due() {
            let ms = u32::try_from(due.saturating_sub(now)).unwrap_or(u32::MAX).max(1);
            // SAFETY: valid hwnd; one-shot timer killed in `journal_tick`.
            unsafe { SetTimer(Some(self.hwnd), TIMER_JOURNAL, ms, None) };
        }
    }

    /// `TIMER_JOURNAL`: writes the journal when due.
    pub(in crate::window) fn journal_tick(&mut self) {
        // SAFETY: valid hwnd.
        let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_JOURNAL) };
        let now = mono_ms();
        match self.plus.journal.sched.due() {
            Some(due) if due <= now => {}
            Some(_) => return self.journal_arm(now),
            None => return,
        }
        self.plus.journal.sched.written(now);
        let Some(path) = self.journal_doc() else {
            return;
        };
        let (text, stamp, enc) = match &self.view {
            View::Doc(d) => match d.edit.as_deref() {
                Some(e) if e.ed.is_dirty() => (e.ed.text(), d.facts.stamp, d.facts.encoding),
                _ => return,
            },
            _ => return,
        };
        if text.len() > yj::MAX_TEXT {
            if !self.plus.journal.big_toasted {
                self.plus.journal.big_toasted = true;
                self.toast(Msg::JournalTooLarge);
            }
            return;
        }
        let h = yj::Header {
            path: path.clone(),
            mtime: stamp.map_or(0, |s| s.mtime),
            hash: stamp.map_or(0, |s| s.hash),
            encoding: enc.to_string(),
        };
        let Some(bytes) = yj::encode(&h, &text) else {
            return;
        };
        let p = Self::journal_path(&path);
        self.plus.journal.file = Some(p.clone());
        send(Job::Write(p, bytes));
    }

    /// Save succeeded, Discard, clean close: the journal goes away.
    pub(in crate::window) fn journal_clear(&mut self) {
        let j = &mut self.plus.journal;
        j.sched.clear();
        j.save_failed = false;
        if let Some(p) = j.file.take() {
            send(Job::Delete(p));
        }
        // SAFETY: valid hwnd.
        let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_JOURNAL) };
    }

    /// A save failed: journal from now on.
    pub(in crate::window) fn journal_save_failed(&mut self) {
        self.plus.journal.save_failed = true;
        self.journal_edited();
    }

    /// Newly shown document: true when an earlier-session journal of it is offered.
    pub(in crate::window) fn journal_on_open(&mut self) -> bool {
        let j = &mut self.plus.journal;
        j.offer = None;
        j.sched.clear();
        j.big_toasted = false;
        j.save_failed = false;
        let Some(path) = self.journal_doc() else {
            return false;
        };
        let p = Self::journal_path(&path);
        // Our own journal of a still-dirty buffer (reload) is not an earlier session.
        if self.plus.journal.file.as_ref() == Some(&p) {
            return false;
        }
        self.plus.journal.file = None;
        let ok = std::fs::metadata(&p).is_ok_and(|m| m.len() <= (yj::MAX_TEXT as u64) + 65_536);
        let Some((h, text)) = ok
            .then(|| std::fs::read(&p).ok())
            .flatten()
            .and_then(|b| yj::decode(&b))
        else {
            return false;
        };
        if !h.path.eq_ignore_ascii_case(&path) {
            return false;
        }
        self.plus.journal.offer = Some((p, h, text));
        true
    }

    /// File name shown in the recovery banner.
    pub(in crate::window) fn journal_name(&self) -> String {
        self.plus.journal.offer.as_ref().map_or_else(String::new, |(_, h, _)| {
            ypath::file_name(&ypath::display_form(&h.path)).to_string()
        })
    }

    /// [Restore] / [Discard] of the recovery banner.
    pub(in crate::window) fn journal_button(&mut self, sh: &Shared, id: u32) {
        let Some((p, h, text)) = self.plus.journal.offer.take() else {
            return;
        };
        if id != BTN_RESTORE {
            if id == BTN_JOURNAL_DISCARD {
                send(Job::Delete(p));
            }
            return;
        }
        if let View::Doc(d) = &mut self.view
            && d.edit.is_none()
        {
            d.edit = Some(Box::new(edit::Edit::new(d.content.source(), d.facts.eol)));
        }
        self.with_ed(sh, |ed| {
            ed.select_all();
            ed.paste(&text);
            ed.seal();
        });
        // The journal stays until the restored buffer is saved or discarded.
        self.plus.journal.file = Some(p);
        let long = ypath::long_form(&h.path);
        let now = sys::stat(&long).map_or(0, |(i, _)| edit::ticks(i.mtime));
        let hash = std::fs::read(&long).ok().map(|b| yamdv_core::encode::hash(&b));
        if yj::base_changed(&h, now, hash) {
            self.tz_banner(sh, BannerKind::Conflict, true);
        }
        self.invalidate();
    }
}
