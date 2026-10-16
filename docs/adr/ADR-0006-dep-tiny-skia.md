# ADR-0006: Dependency `tiny-skia` (=0.12.0)

- Status: accepted for xtask (icon generation) and spike A. Product use is decided by ADR-0002.

## Context
- The original icons (R-SYS-07, UI_SPEC §B.14) must be generated reproducibly from code.
- Spike A needs a CPU rasteriser for shapes on a GDI DIB.

## Decision
- Use `tiny-skia` with `std, simd`. Add `png-format` only in xtask, to write the preview PNGs.
- Licence: BSD-3-Clause, which is on the allowlist. Pure Rust, no unsafe-heavy C bindings, no network.
- Transitive dependencies are `arrayref`, `arrayvec`, `bytemuck`, `cfg-if`, `log`, `strict-num` and `tiny-skia-path`. `png` and its dependencies come in only through xtask.

## Alternatives
- Hand-written rasteriser: anti-aliasing quality costs too much time.
- D2D geometry: this is candidate B/C itself.

## Consequences
- Spike A grows by about 290 KB over B/C (see ADR-0002).
- No effect on `yamdv.exe` in Phase 001, which does not link it.
