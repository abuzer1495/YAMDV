//! All Win32/COM code of YAMDV (ARCHITECTURE §E.8): process hardening, reading windows, the
//! DirectWrite shaper, backend A rendering, file intake, clipboard, console output and the
//! crash-note panic hook.
//!
//! `unsafe` is denied crate-wide and allowed only in the modules that call Win32; every block has a
//! `// SAFETY:` comment.
#![cfg_attr(not(windows), forbid(unsafe_code))]

#[cfg(windows)]
pub mod console;
#[cfg(windows)]
pub mod crash;
#[cfg(all(windows, feature = "dev-tools"))]
pub mod devtools;
#[cfg(windows)]
pub mod dialog_host;
#[cfg(windows)]
pub mod dwrite;
#[cfg(windows)]
pub mod export;
#[cfg(windows)]
pub mod hardening;
#[cfg(windows)]
pub mod images;
#[cfg(windows)]
pub mod install;
#[cfg(windows)]
pub mod job;
#[cfg(windows)]
pub mod log;
#[cfg(windows)]
pub mod perf;
#[cfg(windows)]
pub mod print;
#[cfg(windows)]
pub mod reentry;
#[cfg(windows)]
pub mod render;
#[cfg(windows)]
pub mod shell;
#[cfg(windows)]
pub mod snapshot;
#[cfg(windows)]
pub mod sys;
#[cfg(windows)]
pub mod theme;
#[cfg(windows)]
pub mod window;
