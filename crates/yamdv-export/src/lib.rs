//! HTML writer, CSV export and print pagination.
//!
//! Platform-independent (pure) crate; see `docs/ARCHITECTURE.md` §E.2.
//! Phase 003: the HTML fragment writer for `CF_HTML` (§4.9) and "Copy as Markdown" range
//! expansion. Phase V1-D: the standalone static HTML document (R-EXP-02 per FUT-21), table CSV
//! (R-EXP-04) and pagination for print/PDF (R-EXP-03, FUT-39).
#![forbid(unsafe_code)]

pub mod csv;
pub mod html;
pub mod paginate;
