//! SVG subset renderer (ADR-0021, R-IMG-01, S-RES-07): a bounded XML parser, presentation
//! attributes, shapes, paths, gradients, `use`, and text handed back as runs for DirectWrite.
//! Pure code (no Windows calls), so every limit is a unit test and the parser is fuzzable.

pub mod path;
pub mod render;
pub mod style;
pub mod xml;

#[cfg(test)]
mod tests;

pub use render::{Anchor, Image, TextRun, render, size};

/// Why an SVG cannot be shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Over the 10 MB input cap or the pixel limits.
    TooLarge,
    /// A DTD or entity declaration (rejected, never expanded).
    Dtd,
    /// Not well-formed.
    Malformed,
    /// Nesting deeper than 64.
    TooDeep,
    /// More than 50 000 elements.
    TooMany,
    /// The root element is not `<svg>`.
    NoSvg,
}
