//! Editor core (prompt 006 §2.1): rope buffer, caret + selection, undo/redo, Markdown-aware
//! keys. Platform-independent (pure) crate; see `docs/ARCHITECTURE.md` §E.2. One [`Editor`] is
//! shared by F2 (block-level live edit) and F3 (split edit) for the same document.
#![forbid(unsafe_code)]

pub mod buffer;
pub mod history;
pub mod lines;
pub mod md;
pub mod pairs;
pub mod replace;
pub mod snippets;

pub use buffer::{Editor, Motion, normalize_eol};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_lines;
#[cfg(test)]
mod tests_replace;
#[cfg(test)]
mod tests_snippets;
