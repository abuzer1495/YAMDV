# ADR-0004: Dependency `object` (=0.40.0), xtask only

- Status: accepted (Phase 001)

Context: `cargo xtask check-imports` must parse the PE import table, delay-import table, DllCharacteristics, debug directory (CET) and section flags (S-NET-03, S-TEST-03).

Decision: use `object` with features `read_core, pe, std` only. It is a dev tool and never linked into `yamdv.exe`. Licence: Apache-2.0 OR MIT. It is used by rustc and the gimli ecosystem.

Alternatives: hand-written PE parser (more code to trust); `goblin` (larger, less strict).

Consequences: none on the product binary.
