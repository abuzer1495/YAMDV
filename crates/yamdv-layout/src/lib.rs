//! TextShaper trait, block layout, virtualization, hit testing, display list.
//!
//! Platform-independent (pure) crate; see `docs/ARCHITECTURE.md` §E.2. The DirectWrite shaper
//! lives in `yamdv-platform-win`; tests use [`shaper::MockShaper`].
#![forbid(unsafe_code)]

pub mod block;
pub mod callout;
pub mod code;
pub mod diagram;
pub mod fenwick;
pub mod focus;
pub mod fold;
pub mod front;
pub mod geom;
pub mod img;
pub mod plain;
pub mod reveal;
pub mod rich;
pub mod shaper;
pub mod table;
pub mod table_edit;
pub mod viewer;
pub mod warm;

pub use geom::Rect;

/// Every syntax-highlighter language name and alias (F2 language picker).
pub fn highlight_names() -> &'static [&'static str] {
    yamdv_highlight::ALL_NAMES
}
