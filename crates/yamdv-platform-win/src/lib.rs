//! All Win32/COM code of YAMDV (ARCHITECTURE §E.8). Phase 001 contains only the window skeleton:
//! process hardening, the top-level window (DPI, theme, keyboard close), console output for the
//! CLI and the crash-note panic hook.
//!
//! `unsafe` is denied crate-wide and allowed only in the modules that call Win32; every block has a
//! `// SAFETY:` comment.
#![cfg_attr(not(windows), forbid(unsafe_code))]

#[cfg(windows)]
pub mod console;
#[cfg(windows)]
pub mod crash;
#[cfg(windows)]
pub mod hardening;
#[cfg(windows)]
pub mod log;
#[cfg(windows)]
pub mod theme;
#[cfg(windows)]
pub mod window;
