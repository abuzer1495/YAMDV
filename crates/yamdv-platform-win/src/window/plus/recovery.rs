//! Recovery journal (prompt 015 §3.6).

/// Journal state of one window.
#[derive(Debug, Default)]
pub(in crate::window) struct Journal {
    pub sched: yamdv_core::journal::Schedule,
}
