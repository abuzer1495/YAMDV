//! Native TeX math (R-MATH-01, FUT-17 core): parser, OpenType MATH table reader, math layout to
//! display ops.
//!
//! Platform-independent (pure) crate; see `docs/ARCHITECTURE.md` §E.2. The platform supplies a
//! [`font::MathFont`] (Cambria Math through DirectWrite) and draws the [`layout::Op`]s.
#![forbid(unsafe_code)]

pub mod extra;
pub mod font;
pub mod layout;
pub mod macros;
pub mod parse;
pub mod symbols;
pub mod table;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_cov;

pub use layout::{MathBox, Op, render};
pub use parse::MathError;
pub use table::MathTable;
