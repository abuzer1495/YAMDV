# ADR-0005: Dependency `serde_json` (=1.0.154), xtask only

- Status: accepted (Phase 001)

Context: memprobe writes machine-readable results (schema `yamdv.memprobe/1`) for CI artifacts and later budget gates.

Decision: use `serde_json` with `std`, through the untyped `Value`/`json!` API, which needs no derive macros. It is a dev tool and never linked into the product. Licence: MIT OR Apache-2.0.

Alternatives: hand-rolled JSON writer (escaping bugs); `miniserde`.

Consequences: none on the product.
