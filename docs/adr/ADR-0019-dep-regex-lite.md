# ADR-0019: Dependency `regex-lite` (=0.1.9) for find & replace regex

- Status: accepted (Phase V2-E, prompt 015 §2)

Context: R-EDIT-10 lets find & replace offer regex (MAY) only with a linear-time engine. Prompt 015 caps the
cost at +150 KB of `yamdv.exe` and asks for `regex-lite` or `regex` with minimal features.

Decision: `regex-lite` 0.1.9, no features beyond the defaults, used only by the pure find/replace engine in
`yamdv-editor` (`[.*]` toggle in the find bar, off by default). Replacement templates `$1` / `${name}`.
- Linear time: `regex-lite` documents a worst-case O(m·n) search (Pike VM, no unbounded
  backtracking); the compiled pattern is capped with `size_limit(1 MiB)` and nesting by the
  parser. On top of that the engine checks a **100 ms time guard** between matches and stops with
  "Regex too slow" instead of hanging the UI (unit test: `(a+)+$` on 100 KB stays under 50 ms).
- Case folding: regex case-insensitive matching is the engine's own (`regex-lite` folds ASCII only). The
  Turkish `i/İ/ı/I` equivalence (R-FIND) applies to **plain search only**; with `[.*]` on, `(?i)`/the case
  toggle follows the engine. Whole word wraps the pattern in `\b(?:…)\b`.

Measurement (cross-built Windows release profile `opt-level="s"`, fat LTO, stripped; probe binary using
`RegexBuilder::case_insensitive`, `replace_all` with `$1${name}`, `find`):
| Engine | Exe delta |
|---|---|
| `regex-lite` 0.1.9 | **+59.0 KB** (118 784 → 179 200 B) |
| `regex` 1.x, `default-features=false`, `std` + `unicode-case` | +329.5 KB (→ 456 192 B) — over budget |

Alternatives: `regex` minimal (over the budget, see table); a hand-written Thompson NFA (more code to audit
than a maintained crate); shipping without regex (the fallback, kept if the measured delta in `yamdv.exe`
ever exceeds +150 KB).

Licence: MIT OR Apache-2.0. Transitive dependencies: none. Maintenance: part of the `rust-lang/regex` repository
(maintained with `regex`). RAM: a compiled pattern exists only while the `[.*]` toggle is on (bounded by
`size_limit`); M0/M1 do not compile one, so their delta is 0 (V2-E report). Supply chain: pinned in Cargo.lock, checked by `cargo deny`.

Consequences: no network, no I/O; regex runs on the UI thread over the buffer only with the time guard.
Exe delta in `yamdv.exe` is in the V2-E report. Revisit when Unicode-aware regex case folding is requested.
