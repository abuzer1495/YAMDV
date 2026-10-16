//! Internal logger (ARCHITECTURE §E.11): compile-time levels, **off by default**, local file only.
//! PHASE-004: enabled by the `privacy.diagnostic_log` setting; until then nothing turns it on, so
//! log calls are no-ops in normal builds.

use std::sync::atomic::{AtomicBool, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(false);

/// Enable or disable logging at runtime.
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

/// `true` when logging is enabled.
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Record a warning. Never contains document text (S-PRIV-02).
pub fn warn(component: &'static str, message: &str) {
    if !enabled() {
        return;
    }
    use std::io::Write;
    if let Some(dir) = crate::crash::local_app_data().map(|d| d.join("YAMDV").join("logs"))
        && std::fs::create_dir_all(&dir).is_ok()
        && let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("yamdv.log"))
    {
        let _ = writeln!(f, "WARN {component}: {message}");
    }
}
