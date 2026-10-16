//! HTML writer, PDF writer and CSV export.
//!
//! Platform-independent (pure) crate; see `docs/ARCHITECTURE.md` §E.2.
//! Phase 003: the HTML fragment writer for `CF_HTML` (§4.9), the start of the Phase 011
//! exporter, and "Copy as Markdown" range expansion.
#![forbid(unsafe_code)]

pub mod html;
