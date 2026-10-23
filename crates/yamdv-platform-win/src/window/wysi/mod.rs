//! V2-F WYSIWYG F2 (prompt 016, ADR-0020): inline reveal of the active block, typing transforms,
//! the link editor, table editing. Hooks are called from `edit.rs` / `window.rs`.

use super::*;

#[cfg(feature = "dev-tools")]
mod drive;
mod reveal;

pub(super) use reveal::RevealCache;
