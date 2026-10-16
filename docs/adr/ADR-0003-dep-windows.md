# ADR-0003: Dependency `windows` (=0.62.2), with `windows-core` and `windows-numerics`

- Status: accepted (Phase 001)

## Context
All Win32 and COM access goes through raw bindings. ARCHITECTURE §E requires Win32/COM with no UI framework.

## Decision
- Use `windows =0.62.2` with `default-features = false` and per-crate feature lists. It is only used by `yamdv-platform-win`, `xtask` (memprobe) and `spikes`.
- `windows-core =0.62.2` is a direct dependency only for the `#[implement]` macro, which needs the crate name in scope. Spike A uses it for the `IDWriteTextRenderer` implementation.
- `windows-numerics =0.3.1` provides `Vector2`/`Matrix3x2` for D2D and DWrite signatures. Both crates are already transitive dependencies of `windows`.
- Licences: MIT OR Apache-2.0. Maintained by Microsoft. No network or build-script downloads; the import libraries come from `windows-targets`/raw-dylib.

## Alternatives
- `windows-sys`: no COM interface wrappers, so DWrite, D2D and WIC would need hand-written vtables.
- `winapi`: unmaintained.

## Consequences
- Binary size depends on which features are used, and dead code is stripped.
- `windows-core` brings `combase.dll` and `oleaut32.dll` imports (error info and `BSTR`) even into the skeleton. Its PWS cost is visible in M0.

## Revisit
When a new `windows` release is needed. Pins are exact (`=`).
