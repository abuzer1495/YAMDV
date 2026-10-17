# ADR-0016: Dependency `ropey` (=1.6.1)

- Status: accepted (Phase V1-B, prompt 006 §2.1; pre-approved in PROCESS.md)

Context/Decision: rope buffer of the shared editor component (`yamdv-editor`): O(log n) inserts, deletes and byte/line conversions on 1 MiB+ documents. `default-features = false` + `simd`: only LF counts as a line break, so CRLF lines keep their `\r` (S-FILE-10) and the editor trims it for line ends.

Alternatives: a `String` with gap buffer (O(n) moves on large files), `crop` (not pre-approved).

Licence: MIT. Dependencies: `smallvec`, `str_indices` (MIT/Apache-2.0). Supply chain: pinned in Cargo.lock, checked by `cargo deny` on CI.

Consequences: linked into `yamdv.exe`; size and memory are in the V1-B report (B-SIZE, M5).
