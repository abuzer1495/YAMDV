# ADR-0008: Dependency `pulldown-cmark` (=0.13.4)

- Status: accepted (Phase 002)

Context/Decision: CommonMark + GFM parser (R-MD-01). Pure Rust, no unsafe in the parsing path, actively maintained, supports every §4.3 option (footnotes, definition lists, math, wikilinks, super/subscript) and source offsets.

Alternatives: comrak (heavier AST, more memory); markdown-rs (slower, larger).

Licence: MIT. Supply chain: pinned in Cargo.lock, checked by `cargo deny` on CI.

Consequences: see the size and memory measurements in docs/reports/002_REPORT.md.
