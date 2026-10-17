# ADR-0017: Dev dependency `proptest` (=1.12.0)

- Status: accepted (Phase V1-B, prompt 006 §2.1; pre-approved in PROCESS.md as dev/test-only)

Context/Decision: property tests of the editor invariants (random edit / undo / redo sequences restore the exact bytes). `default-features = false` + `std` (no regex strategies beyond the string generator, no fork/timeout).

Alternatives: hand-rolled LCG loops (already used for splice tests; no shrinking).

Licence: MIT OR Apache-2.0 (transitive: `rand`, `rand_xorshift`, `regex-syntax`, `bit-set`, `unarray`, `num-traits` — all on the S-SUP-04 allowlist). Never linked into `yamdv.exe` (dev-dependency of `yamdv-editor` only).

Consequences: `cargo test -p yamdv-editor` runs 2 × 400 cases (< 2 s).
