# ADR-0001: Toolchain 1.99.0 and build hardening flags

- Status: accepted (Phase 001)

## Context
The spec requires a pinned stable toolchain, edition 2024, the `x86_64-pc-windows-msvc` target and build hardening (S-PROC-05, S-TEST-03).

## Decision
- `rust-toolchain.toml` pins **1.99.0** (current stable when Phase 001 started), with rustfmt, clippy and the MSVC target. The workspace sets `rust-version = "1.99"`.
- Release profile: `opt-level = "s"`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `overflow-checks = true`, `strip = "symbols"`. A `release-dbg` profile keeps symbols for profiling.
- `.cargo/config.toml` for the MSVC target:
  - `-C target-feature=+crt-static` (static CRT, no VC++ redistributable)
  - `-C control-flow-guard`
  - linker `/DYNAMICBASE /HIGHENTROPYVA /NXCOMPAT /CETCOMPAT /OPT:REF /OPT:ICF`
- `cargo xtask check-imports` verifies the result in CI: DllCharacteristics `DYNAMIC_BASE | HIGH_ENTROPY_VA | NX_COMPAT | GUARD_CF`, the CET-compat debug entry and no W+X sections.
- Cross-building from the Linux dev host uses `cargo-xwin` (lld-link). It accepted every flag above. Windows CI uses link.exe.
- **Deferred:** `ProcessDynamicCodePolicy` (S-PROC-04 SHOULD). DirectWrite and D2D behaviour under ProhibitDynamicCode must be verified on real hardware first, which is Phase 002 work.

## Alternatives
- Floating `stable`: not reproducible.
- `opt-level = "z"`: re-evaluate in Phase 013 with measurements.

## Consequences
- The static CRT adds roughly 100 KB.
- CFG adds a little code size.
- Cross-builds and native builds can differ slightly in size, because they use different linkers.

## Revisit
In Phase 013 (size tuning), or when a new stable release brings a needed feature.

## Amendment (Phase 002, F-06): ProcessDynamicCodePolicy

**Decision: on.** `hardening::apply` sets `ProcessDynamicCodePolicy { ProhibitDynamicCode }` (Arbitrary Code Guard, S-PROC-04) as its last step, after the image-load and strict-handle policies.

Verification on CI `windows-latest` (run 38932189793 and later; the release and dev-tools builds both apply the policy before anything else):

| Area | Result |
|---|---|
| DirectWrite text, font fallback (CJK, Arabic, Hebrew) and color emoji (`TranslateColorGlyphRun`) | works — all 32 goldens render, including `unicode-*` with emoji |
| WIC PNG **encode** (`--render-png`) | works |
| WIC **decode** | NOT VERIFIED — not used before Phase 003 (images); re-check then |
| `IFileOpenDialog` open + cancel (M6) | works, process exits cleanly |
| Dialog in a shell-extension-heavy folder | NOT VERIFIED on CI (no extensions installed) → owner kit step 2 |
| Drag & drop (`WM_DROPFILES`) | NOT VERIFIED on CI (no input injection) → owner kit step 2 |
| IME composition | NOT VERIFIED (no IME on the runner; the viewer has no text input yet) → owner kit step 2 |

Revisit if a user reports a shell extension failing to load in the Open dialog. Extensions that JIT (rare) cannot run under ACG, which is the intended protection.
