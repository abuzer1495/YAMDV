# ADR-0012: Dependency `libfuzzer-sys` (0.4 (fuzz/ only))

- Status: accepted (Phase 002)

Context/Decision: cargo-fuzz harness for S-TEST-01. Lives in the separate fuzz/ workspace and is never part of the product.

Alternatives: afl.rs.

Licence: MIT OR Apache-2.0 OR NCSA. Supply chain: pinned in Cargo.lock, checked by `cargo deny` on CI.

Consequences: see the size and memory measurements in docs/reports/002_REPORT.md.
