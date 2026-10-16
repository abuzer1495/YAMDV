# ADR-0010: Dependency `chardetng` (=1.0.0)

- Status: accepted (Phase 002)

Context/Decision: Encoding detection for non-UTF-8 files without a BOM (R-OPEN-06). Built for encoding_rs and used by Firefox. The API is used with ISO-2022-JP detection denied and UTF-8 handled separately.

Alternatives: uchardet (C), hand-written heuristics.

Licence: Apache-2.0 OR MIT. Supply chain: pinned in Cargo.lock, checked by `cargo deny` on CI.

Consequences: see the size and memory measurements in docs/reports/002_REPORT.md.
