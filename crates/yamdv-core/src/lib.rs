//! Source text, encoding, Markdown parsing adapter, document model (block index + lazy detail
//! trees), slugs, config model, INI parser, i18n tables, theme data.
//!
//! Platform-independent (pure) crate; see `docs/ARCHITECTURE.md` §E.2 and ADR-0007.
#![forbid(unsafe_code)]

pub mod cmdline;
pub mod decode;
pub mod devscript;
pub mod doc;
pub mod emoji_table;
pub mod encode;
pub mod find;
pub mod html;
pub mod i18n;
pub mod image;
pub mod ini;
pub mod ipc;
pub mod link;
pub mod log;
pub mod path;
pub mod reg;
pub mod settings;
mod settings_table;
pub mod state;
pub mod theme;
mod theme_table;
