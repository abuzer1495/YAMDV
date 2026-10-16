//! Source text, encoding, Markdown parsing adapter, document model (block index + lazy detail
//! trees), slugs, config model, INI parser, i18n tables, theme data.
//!
//! Platform-independent (pure) crate; see `docs/ARCHITECTURE.md` §E.2 and ADR-0007.
#![forbid(unsafe_code)]

pub mod decode;
pub mod doc;
pub mod emoji_table;
pub mod html;
pub mod i18n;
pub mod ini;
pub mod ipc;
pub mod link;
pub mod log;
pub mod path;
pub mod settings;
mod settings_table;
pub mod theme;
mod theme_table;
