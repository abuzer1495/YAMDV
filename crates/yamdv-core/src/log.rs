//! Internal logger model (H14, S-PRIV-02): off by default, enabled by `privacy.diagnostic_log`,
//! ≤ 1 MiB × 3 rotating files. Messages are static categories plus numbers — **never document
//! text**; paths only when the diagnostic log is enabled. The file sink lives in the platform crate.

/// Severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Errors.
    Error,
    /// Warnings.
    Warn,
    /// Informational.
    Info,
    /// Debug (compiled out of release builds).
    Debug,
}

/// Most verbose level compiled in.
pub const MAX_LEVEL: Level = if cfg!(debug_assertions) {
    Level::Debug
} else {
    Level::Info
};
/// Maximum size of one log file.
pub const MAX_FILE_BYTES: u64 = 1024 * 1024;
/// Number of files kept (`yamdv.log`, `yamdv.1.log`, `yamdv.2.log`).
pub const FILES: usize = 3;

/// Whether a record at `level` is written given the runtime switch.
pub fn enabled(level: Level, diagnostic_log: bool) -> bool {
    diagnostic_log && level <= MAX_LEVEL
}

/// File names in rotation order (newest first).
pub fn rotation_names() -> [&'static str; FILES] {
    ["yamdv.log", "yamdv.1.log", "yamdv.2.log"]
}

/// Whether the current file must rotate before appending `incoming` bytes.
pub fn must_rotate(current_len: u64, incoming: u64) -> bool {
    current_len.saturating_add(incoming) > MAX_FILE_BYTES
}

/// Formats a record: `<unix_ms> <LEVEL> <category> <detail>`. `detail` must not contain
/// document text (callers pass numbers/static strings only).
pub fn format(unix_ms: u64, level: Level, category: &str, detail: &str) -> String {
    let l = match level {
        Level::Error => "ERROR",
        Level::Warn => "WARN",
        Level::Info => "INFO",
        Level::Debug => "DEBUG",
    };
    let clean: String = detail
        .chars()
        .filter(|c| !c.is_control())
        .take(512)
        .collect();
    format!("{unix_ms} {l} {category} {clean}\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s_priv_02_off_by_default_and_rotation() {
        assert!(!enabled(Level::Error, false));
        assert!(enabled(Level::Error, true));
        assert!(must_rotate(MAX_FILE_BYTES, 1));
        assert!(!must_rotate(10, 10));
        assert_eq!(format(1, Level::Warn, "open", "a\nb"), "1 WARN open ab\n");
        assert_eq!(rotation_names().len(), FILES);
    }
}
