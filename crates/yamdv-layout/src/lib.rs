//! TextShaper trait, block layout, virtualization, hit testing, display list.
//!
//! Platform-independent (pure) crate; see `docs/ARCHITECTURE.md` §E.2. The DirectWrite shaper
//! lives in `yamdv-platform-win`; tests use [`shaper::MockShaper`].
#![forbid(unsafe_code)]

pub mod block;
pub mod fenwick;
pub mod geom;
pub mod plain;
pub mod shaper;
pub mod viewer;
pub mod warm;

pub use geom::Rect;
