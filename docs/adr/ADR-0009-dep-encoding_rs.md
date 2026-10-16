# ADR-0009: Dependency `encoding_rs` (=0.8.42)

- Status: accepted (Phase 002)

Context/Decision: Legacy-encoding decoding (R-OPEN-06): the WHATWG encodings used by browsers, fast, used by Firefox.

Alternatives: Windows MultiByteToWideChar (no detection, platform-bound).

Licence: (Apache-2.0 OR MIT) AND BSD-3-Clause. Supply chain: pinned in Cargo.lock, checked by `cargo deny` on CI.

Consequences: see the size and memory measurements in docs/reports/002_REPORT.md.
