# ADR-0011: Dependency `unicode-segmentation` (=1.13.3)

- Status: accepted (Phase 002)

Context/Decision: UAX #29 word bounds for double-click word selection (R-REN-16) and grapheme-safe estimates.

Alternatives: ICU via windows.globalization (heavy), hand-written tables.

Licence: MIT OR Apache-2.0. Supply chain: pinned in Cargo.lock, checked by `cargo deny` on CI.

Consequences: see the size and memory measurements in docs/reports/002_REPORT.md.
